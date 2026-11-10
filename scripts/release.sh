#!/usr/bin/env bash
# Start the same one-step release flow available in GitHub Actions.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
VERSION="${1:?Usage: scripts/release.sh <version>}"
[[ "${VERSION}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || { echo "Expected a stable x.y.z version" >&2; exit 1; }
gh workflow run release.yml --ref main -f "version=$VERSION"
echo "Release workflow started. Follow progress in GitHub Actions."
