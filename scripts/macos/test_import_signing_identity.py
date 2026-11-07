"""Signing preflight tests using fake credentials and macOS tools."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("import_signing_identity.sh")
FINGERPRINT = "A" * 40
IDENTITY = "Developer ID Application: Example Company (EXAMPLETEAM)"


class SigningImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="zbase-signing-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.env = dict(os.environ, PATH=f"{self.bin}{os.pathsep}{os.environ['PATH']}",
                        RUNNER_TEMP=str(self.root), GITHUB_ENV=str(self.root / "env"),
                        MACOS_CERTIFICATE_P12_BASE64="Zml4dHVyZQ==",
                        MACOS_CERTIFICATE_PASSWORD="fixture-password",
                        APPLE_API_PRIVATE_KEY="fixture-notary-key",
                        MACOS_SIGNING_IDENTITY=FINGERPRINT, APPLE_TEAM_ID="EXAMPLETEAM",
                        TEST_IDENTITY=IDENTITY, TEST_FINGERPRINT=FINGERPRINT,
                        TEST_SIGN_FAILURE="0")
        mock = '''import json, os, pathlib, sys
root = pathlib.Path(os.environ["RUNNER_TEMP"])
command = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with (root / "calls").open("a") as log:
    log.write(json.dumps([command, *args]) + "\\n")
if command == "security":
    if args[:4] == ["list-keychains", "-d", "user", "-s"]:
        (root / "search-list").write_text(json.dumps(args[4:]))
    elif args == ["list-keychains", "-d", "user"]:
        print('    "/Library/Keychains/System.keychain"')
        print('    "/Users/runner/Library/Keychains/login with spaces.keychain-db"')
    elif args[0] == "find-identity":
        print('  1) ' + os.environ["TEST_FINGERPRINT"] + ' "' + os.environ["TEST_IDENTITY"] + '"')
elif command == "codesign":
    keychain = str(root / "zbase-signing/release.keychain-db")
    if keychain not in json.loads((root / "search-list").read_text()):
        sys.exit("Signing keychain is absent from search list")
    if os.environ["TEST_SIGN_FAILURE"] == "1":
        sys.exit("Fixture private key unavailable")
'''
        for name in ("security", "codesign"):
            path = self.bin / name
            path.write_text(f"#!{sys.executable}\n" + mock)
            path.chmod(0o755)

    def run_import(self):
        return subprocess.run(["bash", str(SCRIPT)], env=self.env, text=True,
                              capture_output=True)

    def calls(self):
        return [json.loads(line) for line in (self.root / "calls").read_text().splitlines()]

    def test_import_registers_keychain_preserves_search_paths_and_signs_probe(self):
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stderr)
        paths = json.loads((self.root / "search-list").read_text())
        keychain = str(self.root / "zbase-signing/release.keychain-db")
        self.assertEqual(paths, [keychain, "/Library/Keychains/System.keychain",
                                "/Users/runner/Library/Keychains/login with spaces.keychain-db"])
        signs = [call for call in self.calls() if call[0] == "codesign"]
        self.assertIn("--timestamp", signs[0])
        self.assertEqual(signs[0][signs[0].index("--keychain") + 1], keychain)
        self.assertEqual(signs[1][1:3], ["--verify", "--strict"])
        self.assertIn(f"SIGNING_KEYCHAIN={keychain}\n", (self.root / "env").read_text())
        self.assertFalse((self.root / "zbase-signing/certificate.p12").exists())
        self.assertFalse((self.root / "zbase-signing/signing-probe").exists())

    def test_identity_name_is_supported(self):
        self.env["MACOS_SIGNING_IDENTITY"] = IDENTITY
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_mismatched_or_invalid_certificate_fails_before_signing(self):
        self.env["TEST_FINGERPRINT"] = "B" * 40
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match a valid identity", result.stderr)
        self.assertFalse(any(call[0] == "codesign" for call in self.calls()))
        self.assertFalse((self.root / "env").exists())

    def test_wrong_team_is_rejected(self):
        self.env["APPLE_TEAM_ID"] = "OTHERTEAM"
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must be Developer ID Application for APPLE_TEAM_ID", result.stderr)

    def test_development_identity_is_rejected(self):
        self.env["TEST_IDENTITY"] = "Apple Development: Example Company (EXAMPLETEAM)"
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must be Developer ID Application", result.stderr)

    def test_private_key_access_failure_does_not_export_credentials(self):
        self.env["TEST_SIGN_FAILURE"] = "1"
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Fixture private key unavailable", result.stderr)
        self.assertFalse((self.root / "env").exists())


if __name__ == "__main__":
    unittest.main()
