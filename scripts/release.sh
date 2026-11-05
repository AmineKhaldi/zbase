#!/usr/bin/env bash
# The tag-triggered workflow owns building, signing, notarization, and publishing.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
VERSION="${1:?Usage: scripts/release.sh <version>}"
[[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Expected a stable x.y.z version" >&2; exit 1; }
CURRENT="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
[[ "${VERSION}" == "${CURRENT}" ]] || { echo "Cargo version is ${CURRENT}; update Cargo.toml and Cargo.lock first" >&2; exit 1; }
[[ -z "$(git status --porcelain --untracked-files=no)" ]] || { echo "Commit tracked changes before releasing" >&2; exit 1; }
git fetch origin main
git merge-base --is-ancestor HEAD origin/main || { echo "Release commit must already be on origin/main" >&2; exit 1; }
git tag -a "v${VERSION}" -m "zBase v${VERSION}"
git push origin "refs/tags/v${VERSION}"
echo "Release workflow started for v${VERSION}. It publishes only after signing and notarization succeed."
