#!/usr/bin/env bash
# Shared preparation for workflow_dispatch and release.published.
set -euo pipefail

output() {
  {
    echo "ref=$(git rev-parse HEAD)"
    echo "release-id=${RELEASE_ID:-}"
    echo "stable=${STABLE:-false}"
  } >> "$GITHUB_OUTPUT"
}

if [[ "$EVENT_NAME" == release ]]; then
  # Read current state: on a retry the original event SHA may predate our bump.
  state=$(gh api "repos/$GH_REPO/releases/$RELEASE_ID")
  jq -e '.draft == false and .immutable != true' <<< "$state" >/dev/null
  tag=$(jq -r .tag_name <<< "$state")
  [[ "$tag" == v* ]] || { echo 'Release tags must start with v' >&2; exit 1; }
  RELEASE_VERSION=${tag#v}
  STABLE=$(jq -r '.prerelease == false' <<< "$state")
  # Resolve only an actual tag ref, never a branch supplied through a tag name.
  git check-ref-format "refs/tags/$tag"
  tag_oid=$(git rev-parse "refs/tags/$tag")
  git checkout --detach "refs/tags/$tag"
  version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')
  if [[ "$version" == "$RELEASE_VERSION" ]]; then
    output
    exit 0
  fi
  jq -e '.assets | length == 0' <<< "$state" >/dev/null || {
    echo 'Cannot move a release tag after assets have been uploaded' >&2; exit 1;
  }
  [[ "$(git rev-parse HEAD)" == "$(git rev-parse refs/remotes/origin/main)" ]] || {
    echo 'For an automatic version bump, publish a tag at the current main commit' >&2; exit 1;
  }
else
  if [[ -z "$RELEASE_VERSION" ]]; then
    output
    exit 0
  fi
  [[ "$GITHUB_REF" == refs/heads/main ]] || {
    echo 'Run releases from main; leave version blank for branch build tests.' >&2; exit 1;
  }
  tag="v$RELEASE_VERSION"
  if git show-ref --verify --quiet "refs/tags/$tag"; then
    echo "Tag $tag already exists. Retry failed jobs in the original run or choose a new version." >&2
    exit 1
  fi
fi

python3 scripts/prepare_release.py "$RELEASE_VERSION"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add Cargo.toml Cargo.lock
git commit -m "Release $tag"
if [[ "$EVENT_NAME" == release ]]; then
  git tag -f -a "$tag" -m "zBase $tag"
  # Update main and the tag together; reject concurrent changes to either ref.
  git push --atomic --force-with-lease="refs/tags/$tag:$tag_oid" origin HEAD:refs/heads/main "refs/tags/$tag"
else
  git tag -a "$tag" -m "zBase $tag"
  git push --atomic origin HEAD:refs/heads/main "refs/tags/$tag"
  gh release create "$tag" --verify-tag --title "zBase $tag" --generate-notes
  RELEASE_ID=$(gh api "repos/$GH_REPO/releases/tags/$tag" --jq .id)
  STABLE=true
fi
output
printf 'Prepared [%s](https://github.com/%s/releases/tag/%s). Assets will appear as builds finish.\n' "$tag" "$GH_REPO" "$tag" >> "$GITHUB_STEP_SUMMARY"
