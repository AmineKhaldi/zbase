#!/usr/bin/env bash
# Input must already be a self-contained app from bundle_dylibs.py.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP="${1:?Usage: package_release.sh path/to/zBase.app output.dmg}"
DMG="${2:?}"
: "${MACOS_SIGNING_IDENTITY:?}" "${SIGNING_KEYCHAIN:?}" "${APPLE_TEAM_ID:?}"
: "${APPLE_API_KEY_PATH:?}" "${APPLE_API_KEY_ID:?}" "${APPLE_API_ISSUER_ID:?}"
python3 "${ROOT_DIR}/scripts/macos/bundle_dylibs.py" "${APP}"
WORK="$(mktemp -d)"
MOUNTED=0
cleanup() {
  if [[ "${MOUNTED}" == 1 ]]; then hdiutil detach "${WORK}/mount" || true; fi
  rm -rf "${WORK}"
}
trap cleanup EXIT
sign() {
  codesign --force --options runtime --timestamp --keychain "${SIGNING_KEYCHAIN}" --sign "${MACOS_SIGNING_IDENTITY}" "$1"
}
notarize() {
  local file="$1" result="$2"
  # Preserve the JSON result, including a submission ID, even when submission fails.
  if ! xcrun notarytool submit "${file}" --key "${APPLE_API_KEY_PATH}" --key-id "${APPLE_API_KEY_ID}" \
    --issuer "${APPLE_API_ISSUER_ID}" --wait --timeout 30m --output-format json > "${result}"; then
    cat "${result}"
    return 1
  fi
  if ! python3 -c 'import json,sys; sys.exit(json.load(open(sys.argv[1])).get("status") != "Accepted")' "${result}"; then
    local submission_id
    submission_id="$(plutil -extract id raw -o - "${result}")"
    xcrun notarytool log "${submission_id}" --key "${APPLE_API_KEY_PATH}" --key-id "${APPLE_API_KEY_ID}" --issuer "${APPLE_API_ISSUER_ID}" || true
    cat "${result}"
    return 1
  fi
}
for library in "${APP}"/Contents/Frameworks/*.dylib; do sign "${library}"; done
sign "${APP}"
codesign --verify --deep --strict --verbose=2 "${APP}"
DETAILS="$(codesign --display --verbose=4 "${APP}" 2>&1)"
printf '%s\n' "${DETAILS}"
[[ "${DETAILS}" == *"Authority=Developer ID Application:"* ]]
printf '%s\n' "${DETAILS}" | grep -Fx "TeamIdentifier=${APPLE_TEAM_ID}"
mkdir -p "$(dirname "${DMG}")"
ditto -c -k --sequesterRsrc --keepParent "${APP}" "${WORK}/app.zip"
notarize "${WORK}/app.zip" "${WORK}/app-notary.json"
xcrun stapler staple "${APP}"
xcrun stapler validate "${APP}"
spctl --assess --type execute --verbose=2 "${APP}"
mkdir "${WORK}/volume"
ditto "${APP}" "${WORK}/volume/zBase.app"
ln -s /Applications "${WORK}/volume/Applications"
hdiutil create -srcfolder "${WORK}/volume" -volname zBase -format UDZO -ov "${DMG}"
sign "${DMG}"
notarize "${DMG}" "${WORK}/dmg-notary.json"
xcrun stapler staple "${DMG}"
xcrun stapler validate "${DMG}"
hdiutil verify "${DMG}"
mkdir "${WORK}/mount"
hdiutil attach "${DMG}" -readonly -nobrowse -mountpoint "${WORK}/mount"
MOUNTED=1
codesign --verify --deep --strict --verbose=2 "${WORK}/mount/zBase.app"
xcrun stapler validate "${WORK}/mount/zBase.app"
python3 "${ROOT_DIR}/scripts/macos/bundle_dylibs.py" "${WORK}/mount/zBase.app"
hdiutil detach "${WORK}/mount"
MOUNTED=0
spctl --assess --type open --context context:primary-signature --verbose=2 "${DMG}"
# Checksums must be computed after stapling changes the file.
(cd "$(dirname "${DMG}")" && shasum -a 256 "$(basename "${DMG}")" > "$(basename "${DMG}").sha256")
echo "Verified signed and notarized release: ${DMG}"
