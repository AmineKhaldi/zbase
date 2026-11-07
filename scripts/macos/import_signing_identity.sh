#!/usr/bin/env bash
# CI only. Credentials live outside the checkout and are deleted by an always() step.
set -euo pipefail
umask 077
: "${RUNNER_TEMP:?}" "${GITHUB_ENV:?}" "${MACOS_CERTIFICATE_P12_BASE64:?}" "${MACOS_CERTIFICATE_PASSWORD:?}" "${APPLE_API_PRIVATE_KEY:?}" "${MACOS_SIGNING_IDENTITY:?}" "${APPLE_TEAM_ID:?}"
SIGNING_DIR="${RUNNER_TEMP}/zbase-signing"
mkdir -p "${SIGNING_DIR}"
SIGNING_KEYCHAIN="${SIGNING_DIR}/release.keychain-db"
KEYCHAIN_PASSWORD="$(openssl rand -hex 32)"
echo "::add-mask::${KEYCHAIN_PASSWORD}"
printf '%s' "${MACOS_CERTIFICATE_P12_BASE64}" | base64 --decode > "${SIGNING_DIR}/certificate.p12"
printf '%s\n' "${APPLE_API_PRIVATE_KEY}" > "${SIGNING_DIR}/AuthKey.p8"
security create-keychain -p "${KEYCHAIN_PASSWORD}" "${SIGNING_KEYCHAIN}"
security set-keychain-settings -lut 21600 "${SIGNING_KEYCHAIN}"
security unlock-keychain -p "${KEYCHAIN_PASSWORD}" "${SIGNING_KEYCHAIN}"
security import "${SIGNING_DIR}/certificate.p12" -k "${SIGNING_KEYCHAIN}" -P "${MACOS_CERTIFICATE_PASSWORD}" -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "${KEYCHAIN_PASSWORD}" "${SIGNING_KEYCHAIN}" >/dev/null
rm "${SIGNING_DIR}/certificate.p12"
# codesign's private-key lookup also needs the keychain in the user search list.
# Keep the runner's existing keychains available for certificate trust lookup.
python3 - "${SIGNING_KEYCHAIN}" <<'PYKEYCHAIN'
import os, re, shlex, subprocess, sys
keychain = sys.argv[1]
existing = shlex.split(subprocess.check_output(
    ["security", "list-keychains", "-d", "user"], text=True))
subprocess.run(["security", "list-keychains", "-d", "user", "-s", keychain,
                *[path for path in existing if path != keychain]], check=True)
identities = subprocess.check_output(
    ["security", "find-identity", "-v", "-p", "codesigning", keychain], text=True)
configured = os.environ["MACOS_SIGNING_IDENTITY"]
for fingerprint, name in re.findall(r'([0-9A-Fa-f]{40}) "([^"\n]+)"', identities):
    if configured.upper() == fingerprint.upper() or configured == name:
        if not name.startswith("Developer ID Application:") or not name.endswith(
                " (" + os.environ["APPLE_TEAM_ID"] + ")"):
            sys.exit("Signing identity must be Developer ID Application for APPLE_TEAM_ID")
        break
else:
    print(identities, file=sys.stderr)
    sys.exit("MACOS_SIGNING_IDENTITY does not match a valid identity in the imported "
             "P12. Check the certificate/private key, expiry, trust, and Actions variable.")
PYKEYCHAIN
# Exercise the same private-key access and timestamp service as release signing
# before spending time compiling the app. Do not modify a runner system binary.
PROBE="${SIGNING_DIR}/signing-probe"
cp /usr/bin/true "${PROBE}"
codesign --force --options runtime --timestamp --keychain "${SIGNING_KEYCHAIN}" --sign "${MACOS_SIGNING_IDENTITY}" "${PROBE}"
codesign --verify --strict "${PROBE}"
rm "${PROBE}"
printf 'SIGNING_KEYCHAIN=%s\nAPPLE_API_KEY_PATH=%s\n' "${SIGNING_KEYCHAIN}" "${SIGNING_DIR}/AuthKey.p8" >> "${GITHUB_ENV}"
