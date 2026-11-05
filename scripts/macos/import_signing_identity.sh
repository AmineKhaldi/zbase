#!/usr/bin/env bash
# CI only. Credentials live outside the checkout and are deleted by an always() step.
set -euo pipefail
umask 077
: "${RUNNER_TEMP:?}" "${GITHUB_ENV:?}" "${MACOS_CERTIFICATE_P12_BASE64:?}" "${MACOS_CERTIFICATE_PASSWORD:?}" "${APPLE_API_PRIVATE_KEY:?}"
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
printf 'SIGNING_KEYCHAIN=%s\nAPPLE_API_KEY_PATH=%s\n' "${SIGNING_KEYCHAIN}" "${SIGNING_DIR}/AuthKey.p8" >> "${GITHUB_ENV}"
