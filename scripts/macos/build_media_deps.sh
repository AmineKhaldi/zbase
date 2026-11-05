#!/usr/bin/env bash
# Build the release's shared LGPL FFmpeg libraries, with BSD dav1d for AV1.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPS_DIR="${DEPS_DIR:-${ROOT_DIR}/dist/macos-deps}"
PREFIX="${DEPS_DIR}/prefix"
DOWNLOADS="${DEPS_DIR}/downloads"
export MACOSX_DEPLOYMENT_TARGET=15.0
export CFLAGS="-O2 -mmacosx-version-min=15.0"
export LDFLAGS="-mmacosx-version-min=15.0 -Wl,-headerpad_max_install_names"
export PKG_CONFIG_PATH="${PREFIX}/lib/pkgconfig"
FFMPEG_VERSION=8.1
FFMPEG_SHA256=b072aed6871998cce9b36e7774033105ca29e33632be5b6347f3206898e0756a
DAV1D_VERSION=1.5.3
DAV1D_SHA256=e099f53253f6c247580c554d53a13f1040638f2066edc3c740e4c2f15174ce22
mkdir -p "${DOWNLOADS}" "${DEPS_DIR}/build"
fetch() {
  local url="$1" archive="$2" expected="$3"
  if [[ ! -f "${archive}" ]]; then
    curl --fail --location --retry 3 --connect-timeout 30 "${url}" -o "${archive}.download"
    mv "${archive}.download" "${archive}"
  fi
  echo "${expected}  ${archive}" | shasum -a 256 -c -
}
FFMPEG_ARCHIVE="${DOWNLOADS}/ffmpeg-${FFMPEG_VERSION}.tar.xz"
DAV1D_ARCHIVE="${DOWNLOADS}/dav1d-${DAV1D_VERSION}.tar.bz2"
fetch "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.xz" "${FFMPEG_ARCHIVE}" "${FFMPEG_SHA256}"
fetch "https://code.videolan.org/videolan/dav1d/-/archive/${DAV1D_VERSION}/dav1d-${DAV1D_VERSION}.tar.bz2" "${DAV1D_ARCHIVE}" "${DAV1D_SHA256}"
RECIPE="$(shasum -a 256 "$0" | awk '{print $1}')-$(uname -m)-${PREFIX}"
if [[ -f "${PREFIX}/share/zbase-media-recipe" ]] && [[ "$(cat "${PREFIX}/share/zbase-media-recipe")" == "${RECIPE}" ]]; then
  echo "Using cached release media libraries: ${PREFIX}"
  exit 0
fi
# Only remove this script's dedicated build outputs, never a system prefix.
rm -rf "${DEPS_DIR}/build/dav1d-${DAV1D_VERSION}" "${DEPS_DIR}/build/ffmpeg-${FFMPEG_VERSION}" "${PREFIX}"
tar -xf "${DAV1D_ARCHIVE}" -C "${DEPS_DIR}/build"
tar -xf "${FFMPEG_ARCHIVE}" -C "${DEPS_DIR}/build"
DAV1D_SOURCE="${DEPS_DIR}/build/dav1d-${DAV1D_VERSION}"
FFMPEG_SOURCE="${DEPS_DIR}/build/ffmpeg-${FFMPEG_VERSION}"
meson setup "${DAV1D_SOURCE}/build" "${DAV1D_SOURCE}" --prefix "${PREFIX}" \
  --libdir lib --buildtype release --default-library shared -Denable_tools=false -Denable_tests=false
ninja -C "${DAV1D_SOURCE}/build" install
(
  cd "${FFMPEG_SOURCE}"
  ./configure --prefix="${PREFIX}" --cc=clang --enable-shared --disable-static \
    --disable-autodetect --disable-gpl --disable-nonfree --disable-version3 \
    --disable-programs --disable-doc --disable-debug --enable-pic --enable-zlib \
    --enable-libdav1d --enable-videotoolbox --enable-audiotoolbox \
    --extra-cflags="${CFLAGS}" --extra-ldflags="${LDFLAGS}"
  make -j "${MEDIA_BUILD_JOBS:-$(sysctl -n hw.logicalcpu)}"
  make install
)
mkdir -p "${PREFIX}/share/zbase-licenses/ffmpeg" "${PREFIX}/share/zbase-licenses/dav1d"
cp "${FFMPEG_SOURCE}/COPYING.LGPLv2.1" "${PREFIX}/share/zbase-licenses/ffmpeg/"
cp "${FFMPEG_SOURCE}/ffbuild/config.log" "${PREFIX}/share/zbase-licenses/ffmpeg/build-config.txt"
cp "${DAV1D_SOURCE}/COPYING" "${PREFIX}/share/zbase-licenses/dav1d/"
printf '%s\n' "FFmpeg ${FFMPEG_VERSION} (${FFMPEG_SHA256})" "dav1d ${DAV1D_VERSION} (${DAV1D_SHA256})" \
  "Corresponding unmodified source archives accompany the GitHub release." > "${PREFIX}/share/zbase-licenses/README.txt"
printf '%s\n' "${RECIPE}" > "${PREFIX}/share/zbase-media-recipe"
echo "Built release media libraries: ${PREFIX}"
