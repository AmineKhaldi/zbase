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

Merge the workflow to the default branch first so GitHub exposes its manual Run workflow control. Run **Release** from the desired branch with **publish unchecked**. This performs the full signing and notarization pipeline and uploads a `macos-release` artifact, without creating a GitHub release or changing the Homebrew tap. Download the DMG from that workflow run and test it on a Mac without development dependencies installed.

## Publish

1. Update the package version in `Cargo.toml` and `Cargo.lock`, and merge the release commit to `main`.
2. From that committed revision, run `scripts/release.sh 1.0.13` (substitute its actual version). The script checks the version and pushes the annotated `v1.0.13` tag.
3. The tag starts the Release workflow. Successful builds upload the signed/notarized DMG, its SHA-256 checksum, media source archives, and build information, then publish the draft release. Only then is the Homebrew source formula updated.

The workflow no longer publishes on every push to `main`, creates its own tags, or bumps versions automatically. The local release script does not create an unsigned GitHub release.

If building/notarization fails, no public release is created. Rerun the failed workflow after correcting the cause. A publishing failure can leave a draft; rerunning resumes that draft. An already-public release is never overwritten. If only the Homebrew update failed after publication, rerun that step/job with the publish step skipped, or run `scripts/update_homebrew.py vVERSION` with an appropriately scoped `GH_TOKEN`; do not republish the existing release.

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
