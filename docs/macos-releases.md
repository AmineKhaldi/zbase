# macOS releases

The Release workflow builds a self-contained **Apple Silicon** DMG for **macOS 15 or newer**. It signs the app and disk image with Developer ID, submits both to Apple's notary service, staples their tickets, and checks signatures and Gatekeeper assessment before uploading assets. Intel Macs are not supported by this initial release job.

## Repository configuration

Under Settings → Secrets and variables → Actions, configure:

| Kind | Name | Value |
| --- | --- | --- |
| Secret | `MACOS_CERTIFICATE_P12_BASE64` | Base64-encoded Developer ID Application certificate **and private key**, exported as a password-protected `.p12` |
| Secret | `MACOS_CERTIFICATE_PASSWORD` | Export password for that `.p12` |
| Secret | `APPLE_API_PRIVATE_KEY` | Complete App Store Connect team API `.p8` private key, including PEM header/footer |
| Variable | `MACOS_SIGNING_IDENTITY` | Developer ID Application identity name or SHA-1 fingerprint |
| Variable | `APPLE_TEAM_ID` | Apple Developer team ID |
| Variable | `APPLE_API_KEY_ID` | App Store Connect API key ID |
| Variable | `APPLE_API_ISSUER_ID` | Issuer ID for the team API key |
| Secret | `TAP_TOKEN` | Token with contents-write permission on `cameroncooper/homebrew-tap` |

The signing certificate must be Developer ID **Application**, not Apple Development or Developer ID Installer. The notary API key must have permission to submit notarization requests. Credentials are imported after compilation into a temporary keychain; an unconditional cleanup step removes the keychain and private key file. Artifacts contain no credential files.

## Test before publishing

Merge the workflow to the default branch first so GitHub exposes its manual Run workflow control. Run **Release** from the desired branch. Manual runs only build artifacts; they never attach assets to a release. This performs the full signing and notarization pipeline and uploads a `macos-release` artifact, without creating a GitHub release or changing the Homebrew tap. Download the DMG from that workflow run and test it on a Mac without development dependencies installed.

## Publish

1. Merge these workflows to `main` first. In **Actions → Prepare release → Run workflow**, select `main` and enter a new stable version without `v` (for example, `1.0.14`).
2. Open the version-bump PR linked in the workflow summary. It updates only the app version in `Cargo.toml` and `Cargo.lock`, preserving dependency versions. CI starts automatically on its branch. Review the changes, wait for checks, and merge the PR.
3. In GitHub, open **Releases → Draft a new release**. Choose **Create new tag**, enter the matching tag (for example, `v1.0.14`), and target the merged release-preparation commit on `main`.
4. Write your title and release notes. Save the draft while editing; click **Publish release** when ready.
5. The `release: published` event starts the workflow. It builds, signs and notarizes the app and DMG, then attaches the DMG, checksum, corresponding media sources and build information to that same release. Your title, notes, latest selection and prerelease status are preserved. The Homebrew source formula is updated after attachment for stable releases only.

Prepare release accepts patch, minor, or major version increases. It rejects malformed, unchanged, older, or already-tagged versions. It creates neither a tag nor a release, and does not merge its PR. A rerun reuses an existing open preparation PR for that version without overwriting its branch. If a branch exists without an open PR, inspect it and open the PR manually or delete the abandoned branch before retrying.

The repository setting **Settings → Actions → General → Workflow permissions → Allow GitHub Actions to create and approve pull requests** must be enabled. The workflow uses the built-in `GITHUB_TOKEN`; no additional secret is required. GitHub can require approval for PR-event workflows created by this token, so Prepare release explicitly dispatches `CI` on the preparation branch. See [GitHub's token-trigger rules](https://docs.github.com/en/actions/concepts/security/github_token#when-github_token-triggers-workflow-runs). It does not approve or merge PRs.

You can also update the two Cargo files in an ordinary PR instead of using Prepare release. Either way, the published tag must match the version committed at its target.

The release is public immediately; its DMG becomes available once the workflow succeeds. Saving a draft, editing notes, pushing a tag, or merging to `main` does not start a release build. There is no automatic tagging or version bump on merge. Manual workflow runs remain available for testing without changing any release.

This process requires **immutable releases to be disabled**, since assets are attached after publication. The workflow rejects an immutable release before compiling.

The optional CLI helper `scripts/release.sh VERSION` validates the committed version, pushes its tag, and creates a draft with generated notes. Edit and publish that draft in GitHub to start the build. The script is not required for the browser-based process.

If building or notarization fails, the public release remains available with its notes, but the new DMG is not attached. Use **Re-run failed jobs** on that release's workflow run to retry; do not create a replacement release or move its tag. Asset upload retries replace matching generated asset names and preserve your release metadata and other assets. If only the Homebrew step fails, rerunning the attachment job is safe. Manual workflow runs do not attach assets, so use the original release-triggered run when retrying attachment.

## Packaging and media libraries

`scripts/macos/build_media_deps.sh` builds pinned, checksum-verified FFmpeg and dav1d sources with a macOS 15 deployment target. FFmpeg uses shared libraries, disables GPL/nonfree components and automatic discovery of installed third-party libraries, and includes dav1d for AV1 decoding. Source archives accompany the release; licenses, configuration and the bundled library list are included in the app's Resources/Licenses/Media directory.

`scripts/macos/bundle_dylibs.py` copies the recursive library graph from that explicit build prefix into Contents/Frameworks, rewrites install names, and rejects unresolved dependencies, external non-system paths, incorrect architectures, or libraries requiring an OS newer than macOS 15. It can be rerun without `--prefix` to verify a prepared bundle. Ordinary local/source Homebrew builds continue to use the developer's installed media libraries; the official DMG is self-contained.

To reproduce the unsigned packaging locally (no installed app is replaced):

```sh
scripts/macos/build_media_deps.sh
export PKG_CONFIG_PATH="$PWD/dist/macos-deps/prefix/lib/pkgconfig"
export PKG_CONFIG_LIBDIR="$PKG_CONFIG_PATH"
scripts/macos/build_app_bundle.sh release
python3 scripts/macos/bundle_dylibs.py dist/macos/zBase.app --prefix "$PWD/dist/macos-deps/prefix"
```

The unsigned output is for development only. `scripts/macos/package_release.sh` requires the signing/notary environment described above and produces the distributable DMG.
