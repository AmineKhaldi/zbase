"""macOS integration checks for portable dependency packaging and rejection paths."""
import platform
import plistlib
import subprocess
import tempfile
import unittest
from pathlib import Path

from bundle_dylibs import bundle, output, verify


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="zbase-bundle-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.prefix = self.root / "prefix"
        self.prefix.mkdir()
        licenses = self.prefix / "share/zbase-licenses"
        licenses.mkdir(parents=True)
        (licenses / "license.txt").write_text("Fixture only")
        self.app = self.root / "zBase.app"
        (self.app / "Contents/MacOS").mkdir(parents=True)
        (self.app / "Contents/Resources").mkdir()
        with (self.app / "Contents/Info.plist").open("wb") as file:
            plistlib.dump({"LSMinimumSystemVersion": "15.0"}, file)
        self.library = self.prefix / "libfixture.1.dylib"
        self.compile("int answer(void) { return 42; }", self.library, "-dynamiclib", "-install_name", str(self.library))
        # A second library exercises recursive copying and loader-relative rewrites.
        outer = self.prefix / "libouter.1.dylib"
        self.compile("int answer(void); int outer(void) { return answer(); }", outer, "-dynamiclib", "-install_name", str(outer), str(self.library))
        self.compile("int outer(void); int main(void) { return outer() == 42 ? 0 : 1; }", self.app / "Contents/MacOS/zbase", str(outer), "-Wl,-rpath,/unused-build-path")

    def compile(self, source, destination, *args):
        source_file = self.root / "fixture.c"
        source_file.write_text(source)
        subprocess.run(["clang", "-mmacosx-version-min=15.0", str(source_file), "-o", str(destination), *args], check=True)

    def test_recursive_bundle_runs_after_original_libraries_disappear(self):
        # Exercise signed inputs explicitly, independent of linker defaults.
        # Their signatures must be replaceable after dependency and rpath edits.
        for binary in [self.app / "Contents/MacOS/zbase", *self.prefix.glob("*.dylib")]:
            output("codesign", "--force", "--sign", "-", binary)
            output("codesign", "--verify", "--strict", binary)
        bundle(self.app, self.prefix)
        for library in self.prefix.glob("*.dylib"):
            library.unlink()
        verify(self.app, platform.machine())
        # Mach-O edits invalidate signatures; use ad-hoc signing for this fixture.
        for library in (self.app / "Contents/Frameworks").glob("*.dylib"):
            output("codesign", "--force", "--sign", "-", library)
        output("codesign", "--force", "--sign", "-", self.app / "Contents/MacOS/zbase")
        for binary in [self.app / "Contents/MacOS/zbase", *(self.app / "Contents/Frameworks").glob("*.dylib")]:
            output("codesign", "--verify", "--strict", binary)
        subprocess.run([str(self.app / "Contents/MacOS/zbase")], check=True)

    def test_rejects_dependency_outside_explicit_prefix(self):
        allowed = self.root / "different-prefix"
        allowed.mkdir()
        with self.assertRaisesRegex(AssertionError, "outside release prefix"):
            bundle(self.app, allowed)

    def test_verify_rejects_unbundled_app(self):
        with self.assertRaisesRegex(AssertionError, "runtime search paths"):
            verify(self.app, platform.machine())


if __name__ == "__main__":
    unittest.main()
