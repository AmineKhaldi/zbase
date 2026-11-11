"""Exercise release preparation against a local Git remote, without GitHub writes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from test_prepare_release import MANIFEST, LOCK


class ReleaseWorkflowTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.remote = self.root / 'remote.git'
        self.repo = self.root / 'repo'
        subprocess.run(['git', 'init', '--bare', str(self.remote)], check=True, capture_output=True)
        self.repo.mkdir()
        self.git('init', '-b', 'main')
        self.git('config', 'user.name', 'Test')
        self.git('config', 'user.email', 'test@example.com')
        scripts = self.repo / 'scripts'
        scripts.mkdir()
        for name in ['prepare_release.py', 'prepare_release_workflow.sh']:
            shutil.copy(Path(__file__).parent / name, scripts / name)
        (self.repo / 'Cargo.toml').write_text(MANIFEST)
        (self.repo / 'Cargo.lock').write_text(LOCK)
        self.git('add', '.')
        self.git('commit', '-m', 'Initial')
        self.initial = self.git('rev-parse', 'HEAD')
        self.git('remote', 'add', 'origin', str(self.remote))
        self.git('push', '-u', 'origin', 'main')
        self.state = dict(id=123, tag_name='v1.0.14', draft=False, immutable=False,
                          prerelease=False, assets=[])
        binary = self.root / 'bin'
        binary.mkdir()
        gh = binary / 'gh'
        gh.write_text('#!/bin/sh\nif [ "$1" = api ]; then\n'
                      '  case "$*" in *--jq*) echo 123;; *) cat "$RELEASE_STATE";; esac\nfi\n')
        gh.chmod(0o755)
        self.env = dict(os.environ, PATH=f'{binary}:{os.environ["PATH"]}',
                        GH_REPO='test/repo', RELEASE_ID='123', RELEASE_VERSION='1.0.14',
                        GITHUB_REF='refs/heads/main', GITHUB_SHA=self.initial,
                        GITHUB_OUTPUT=str(self.root / 'output'),
                        GITHUB_STEP_SUMMARY=str(self.root / 'summary'),
                        RELEASE_STATE=str(self.root / 'state.json'))

    def git(self, *args):
        return subprocess.run(['git', *args], cwd=self.repo, check=True,
                              capture_output=True, text=True).stdout.strip()

    def publish_tag(self):
        self.git('tag', '-a', 'v1.0.14', '-m', 'Release')
        self.git('push', 'origin', 'refs/tags/v1.0.14')

    def run_prepare(self, event='release', success=True):
        (self.root / 'state.json').write_text(json.dumps(self.state))
        result = subprocess.run(['bash', 'scripts/prepare_release_workflow.sh'], cwd=self.repo,
                                env=dict(self.env, EVENT_NAME=event,
                                         RELEASE_ID='123' if event == 'release' else ''),
                                capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return (self.root / 'output').read_text() if success else result.stderr

    def assert_release(self):
        head = self.git('rev-parse', 'HEAD')
        self.assertNotEqual(head, self.initial)
        self.assertEqual(self.git('rev-parse', 'refs/tags/v1.0.14^{commit}'), head)
        self.assertEqual(self.git('rev-parse', 'refs/remotes/origin/main'), head)
        self.assertIn('version = "1.0.14"', (self.repo / 'Cargo.toml').read_text())
        self.assertIn(f'ref={head}', (self.root / 'output').read_text())
        self.assertIn('release-id=123', (self.root / 'output').read_text())

    def test_dispatch_bumps_and_publishes(self):
        self.run_prepare('workflow_dispatch')
        self.assert_release()

    def test_published_release_bumps_and_retry_uses_current_tag(self):
        self.publish_tag()
        self.run_prepare()
        self.assert_release()
        head = self.git('rev-parse', 'HEAD')
        # GitHub replays the original checkout and event on a full rerun.
        self.git('checkout', '--detach', self.initial)
        self.run_prepare()
        self.assertEqual(self.git('rev-parse', 'HEAD'), head)

    def test_refuses_to_move_tag_with_assets_or_immutable_release(self):
        self.publish_tag()
        for changes in [dict(assets=[dict(id=1)]), dict(immutable=True)]:
            with self.subTest(changes=changes):
                previous = self.state.copy()
                self.state.update(changes)
                self.run_prepare(success=False)
                self.assertEqual(self.git('rev-parse', 'HEAD'), self.initial)
                self.state = previous

    def test_refuses_to_bump_old_main_commit(self):
        self.publish_tag()
        self.git('commit', '--allow-empty', '-m', 'Later work')
        self.git('push', 'origin', 'main')
        self.run_prepare(success=False)
        self.assertEqual(self.git('rev-parse', 'refs/tags/v1.0.14^{commit}'), self.initial)

    def test_blank_dispatch_only_builds(self):
        self.env['RELEASE_VERSION'] = ''
        output = self.run_prepare('workflow_dispatch')
        self.assertIn('release-id=\n', output)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.initial)

    def test_rejected_main_push_does_not_move_remote_tag(self):
        self.publish_tag()
        hook = self.remote / 'hooks' / 'pre-receive'
        hook.write_text('#!/bin/sh\nexit 1\n')
        hook.chmod(0o755)
        self.run_prepare(success=False)
        remote_main = self.git('ls-remote', 'origin', 'refs/heads/main').split()[0]
        remote_tag = self.git('ls-remote', 'origin', 'refs/tags/v1.0.14^{}').split()[0]
        self.assertEqual(remote_main, self.initial)
        self.assertEqual(remote_tag, self.initial)

    def test_preversioned_release_preserves_tag_and_prerelease_setting(self):
        self.git('tag', '-a', 'v1.0.13', '-m', 'Existing version')
        self.git('push', 'origin', 'refs/tags/v1.0.13')
        self.state.update(tag_name='v1.0.13', prerelease=True, assets=[dict(id=1)])
        tag_oid = self.git('rev-parse', 'refs/tags/v1.0.13')
        output = self.run_prepare()
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.initial)
        self.assertEqual(self.git('rev-parse', 'refs/tags/v1.0.13'), tag_oid)
        self.assertIn('stable=false', output)


if __name__ == '__main__':
    unittest.main()
