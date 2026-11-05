#!/usr/bin/env python3
"""Prepare a stable version bump without changing dependency versions."""
import argparse
from pathlib import Path
import re
import tomllib


def stable_version(value):
    if not re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", value):
        raise ValueError("Expected a stable x.y.z version without a v prefix")
    parts = tuple(map(int, value.split(".")))
    if any(part > 2**64 - 1 for part in parts):
        raise ValueError("Version components must fit Cargo's unsigned 64-bit integers")
    return parts


def replace_version(block, previous, version):
    updated, count = re.subn(
        rf'(?m)^(version\s*=\s*"){re.escape(previous)}("[^\n]*)$',
        lambda match: match[1] + version + match[2],
        block,
    )
    if count != 1:
        raise ValueError("Expected exactly one package version in its TOML table")
    return updated


def prepare_release(root, version):
    requested = stable_version(version)
    manifest_path = root / "Cargo.toml"
    lock_path = root / "Cargo.lock"
    manifest = manifest_path.read_text()
    lock = lock_path.read_text()
    package = tomllib.loads(manifest)["package"]
    previous = package["version"]
    if requested <= stable_version(previous):
        raise ValueError(f"Release version must be newer than {previous}")

    manifest_table = re.search(r"(?ms)^\[package\]\s*\n.*?(?=^\[|\Z)", manifest)
    if manifest_table is None:
        raise ValueError("Missing [package] table")
    lock_tables = [
        match
        for match in re.finditer(r"(?ms)^\[\[package\]\]\s*\n.*?(?=^\[\[package\]\]|\Z)", lock)
        if tomllib.loads(match[0])["package"][0].get("name") == package["name"]
        and "source" not in tomllib.loads(match[0])["package"][0]
    ]
    if len(lock_tables) != 1:
        raise ValueError("Expected exactly one local app package in Cargo.lock")
    lock_table = lock_tables[0]
    if tomllib.loads(lock_table[0])["package"][0]["version"] != previous:
        raise ValueError("Cargo.toml and Cargo.lock versions do not match")

    # Validate both replacements before writing either file. Preserve dependency
    # entries, comments and formatting rather than resolving the lockfile anew.
    manifest_updated = (
        manifest[:manifest_table.start()]
        + replace_version(manifest_table[0], previous, version)
        + manifest[manifest_table.end():]
    )
    lock_updated = (
        lock[:lock_table.start()]
        + replace_version(lock_table[0], previous, version)
        + lock[lock_table.end():]
    )
    tomllib.loads(manifest_updated)
    tomllib.loads(lock_updated)
    manifest_path.write_text(manifest_updated)
    lock_path.write_text(lock_updated)
    return previous


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="New stable version, e.g. 1.0.14")
    args = parser.parse_args()
    try:
        old = prepare_release(Path(__file__).resolve().parents[1], args.version)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"Cannot prepare release: {error}\n")
    print(f"Prepared version bump: {old} → {args.version}")
