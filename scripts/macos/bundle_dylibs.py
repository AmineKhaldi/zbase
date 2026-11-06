#!/usr/bin/env python3
"""Bundle only the explicitly built media prefix; reject external runtime dependencies."""
import argparse
import json
import plistlib
import re
import shutil
import subprocess
from pathlib import Path


def output(*args):
    return subprocess.check_output([str(arg) for arg in args], text=True)


def system(path):
    return path.startswith(("/usr/lib/", "/System/Library/"))


def dependencies(path):
    # otool includes a dylib's own install name as its first entry.
    lines = output("otool", "-L", path).splitlines()[1:]
    deps = [line.strip().split(" (compatibility version", 1)[0] for line in lines]
    if path.suffix == ".dylib":
        install_id = output("otool", "-D", path).splitlines()[1:]
        deps = [dep for dep in deps if dep not in install_id]
    return deps


def rpaths(path):
    return re.findall(r"cmd LC_RPATH\n\s+cmdsize \d+\n\s+path (.*?) \(offset", output("otool", "-l", path))


def resolve(dep, owner, executable):
    def expand(value):
        return value.replace("@loader_path", str(owner.parent)).replace("@executable_path", str(executable.parent))
    if dep.startswith("@rpath/"):
        candidates = [Path(expand(rpath)) / dep[len("@rpath/"):] for rpath in rpaths(owner) + rpaths(executable)]
    else:
        candidates = [Path(expand(dep))]
    for candidate in candidates:
        if candidate.is_absolute() and candidate.is_file():
            return candidate.resolve()
    raise RuntimeError(f"Unresolved dependency {dep} in {owner}")


def verify(app, arch):
    executable = app / "Contents/MacOS/zbase"
    with (app / "Contents/Info.plist").open("rb") as file:
        assert plistlib.load(file)["LSMinimumSystemVersion"] == "15.0", "Wrong app minimum OS"
    binaries = [executable, *sorted((app / "Contents/Frameworks").glob("*.dylib"))]
    for binary in binaries:
        assert output("lipo", "-archs", binary).strip() == arch, f"Wrong architecture: {binary}"
        minimums = re.findall(r"\bminos\s+(\S+)", output("vtool", "-show-build", binary))
        assert minimums and all(tuple(map(int, v.split("."))) <= (15, 0) for v in minimums), f"Unsupported minimum OS: {binary}: {minimums}"
        assert not rpaths(binary), f"Unexpected runtime search paths: {binary}"
        for dep in dependencies(binary):
            if system(dep):
                continue
            assert dep.startswith("@loader_path/"), f"Nonportable dependency: {binary}: {dep}"
            resolved = resolve(dep, binary, executable)
            assert resolved.is_relative_to(app), f"Dependency escapes app: {dep}"
    print(f"Verified {len(binaries)} self-contained {arch} binaries for macOS 15")


def bundle(app, prefix):
    executable = app / "Contents/MacOS/zbase"
    frameworks = app / "Contents/Frameworks"
    frameworks.mkdir(exist_ok=True)
    queue = [(executable, executable)]
    copied = {}
    for source, target in queue:
        rewrites = []
        for dep in dependencies(source):
            if system(dep):
                continue
            resolved = resolve(dep, source, executable)
            assert resolved.is_relative_to(prefix), f"Dependency outside release prefix: {resolved}"
            name = resolved.name
            assert name not in copied or copied[name] == resolved, f"Library name collision: {name}"
            if name not in copied:
                copied[name] = resolved
                destination = frameworks / name
                shutil.copy2(resolved, destination)
                destination.chmod(0o755)
                queue.append((resolved, destination))
            relative = f"@loader_path/../Frameworks/{name}" if target == executable else f"@loader_path/{name}"
            rewrites.extend(["-change", dep, relative])
        for rpath in rpaths(target):
            rewrites.extend(["-delete_rpath", rpath])
        if target != executable:
            rewrites.extend(["-id", f"@rpath/{target.name}"])
        if rewrites:
            # Keep existing signatures while editing. Removing them first can leave
            # __LINKEDIT padding that older install_name_tool versions reject.
            # Rewrites invalidate signatures; package_release.sh replaces them.
            subprocess.run(["install_name_tool", *rewrites, str(target)], check=True)
    licenses = app / "Contents/Resources/Licenses/Media"
    shutil.copytree(prefix / "share/zbase-licenses", licenses, dirs_exist_ok=True)
    (licenses / "bundled-libraries.json").write_text(json.dumps(sorted(copied), indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--prefix", type=Path)
    parser.add_argument("--arch", default="arm64")
    args = parser.parse_args()
    app = args.app.resolve()
    if args.prefix:
        bundle(app, args.prefix.resolve())
    verify(app, args.arch)
