import tempfile
from pathlib import Path
import unittest

from prepare_release import prepare_release


MANIFEST = '''[package]
name = "zbase"
version = "1.0.13" # app version
edition = "2024"

[dependencies]
fixture = "1.0.13"
'''
LOCK = '''# Generated lockfile
version = 4

[[package]]
name = "fixture"
version = "1.0.13"
source = "registry+https://example.com"

[[package]]
name = "zbase"
version = "1.0.13"
dependencies = ["fixture"]
'''


class PrepareReleaseTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root / "Cargo.toml").write_text(MANIFEST)
        (self.root / "Cargo.lock").write_text(LOCK)

    def assert_unchanged(self, manifest=MANIFEST, lock=LOCK):
        self.assertEqual((self.root / "Cargo.toml").read_text(), manifest)
        self.assertEqual((self.root / "Cargo.lock").read_text(), lock)

    def test_bumps_only_app_versions_and_preserves_comments_and_dependencies(self):
        for version in ["1.0.14", "1.1.0", "2.0.0"]:
            with self.subTest(version=version):
                (self.root / "Cargo.toml").write_text(MANIFEST)
                (self.root / "Cargo.lock").write_text(LOCK)
                self.assertEqual(prepare_release(self.root, version), "1.0.13")
                self.assert_unchanged(
                    MANIFEST.replace('version = "1.0.13"', f'version = "{version}"'),
                    LOCK.replace('name = "zbase"\nversion = "1.0.13"', f'name = "zbase"\nversion = "{version}"'),
                )

    def test_rejects_invalid_equal_or_older_versions_without_writes(self):
        for version in ["1.0.13", "1.0.12", "0.9.99", "v1.0.14", "1.0", "01.0.14", "1.0.14-rc.1", "1.0.14\n", "1.0.$(id)", f"{2**64}.0.0"]:
            with self.subTest(version=version):
                with self.assertRaises(ValueError):
                    prepare_release(self.root, version)
                self.assert_unchanged()

    def test_rejects_missing_duplicate_or_mismatched_lock_entry_before_writes(self):
        app_entry = LOCK[LOCK.index('[[package]]\nname = "zbase"'):]
        for lock in [
            LOCK.replace('name = "zbase"', 'name = "other"'),
            LOCK + "\n" + app_entry,
            LOCK.replace('name = "zbase"\nversion = "1.0.13"', 'name = "zbase"\nversion = "1.0.12"'),
        ]:
            with self.subTest(lock=lock):
                (self.root / "Cargo.lock").write_text(lock)
                with self.assertRaises(ValueError):
                    prepare_release(self.root, "1.0.14")
                self.assert_unchanged(lock=lock)


if __name__ == "__main__":
    unittest.main()
