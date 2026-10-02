#!/usr/bin/env bash
# Downloads the pinned PDFium build (bblanchon/pdfium-binaries) into vendor/pdfium/<platform>/.
#
# Usage: scripts/fetch-pdfium.sh [windows-x64|linux-x64|macos-arm64|macos-x64]
# Without an argument the current platform is detected. Safe to run repeatedly.
set -euo pipefail

PDFIUM_TAG="chromium/8066"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

detect_platform() {
  local os arch
  case "$(uname -s)" in
    Linux*) os=linux ;;
    Darwin*) os=macos ;;
    MINGW* | MSYS* | CYGWIN*) os=windows ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch=x64 ;;
    arm64 | aarch64) arch=arm64 ;;
    *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
  echo "$os-$arch"
}

PLATFORM="${1:-$(detect_platform)}"
case "$PLATFORM" in
  windows-x64) ARCHIVE=pdfium-win-x64.tgz; MEMBER=bin/pdfium.dll ;;
  linux-x64) ARCHIVE=pdfium-linux-x64.tgz; MEMBER=lib/libpdfium.so ;;
  macos-arm64) ARCHIVE=pdfium-mac-arm64.tgz; MEMBER=lib/libpdfium.dylib ;;
  macos-x64) ARCHIVE=pdfium-mac-x64.tgz; MEMBER=lib/libpdfium.dylib ;;
  *) echo "Unknown platform '$PLATFORM' (use windows-x64, linux-x64, macos-arm64 or macos-x64)" >&2; exit 1 ;;
esac
LIB="$(basename "$MEMBER")"
DEST="$ROOT/vendor/pdfium/$PLATFORM"

# TAG may have been written with CRLF by fetch-pdfium.ps1.
if [ -f "$DEST/$LIB" ] && [ "$(tr -d '\r\n' < "$DEST/TAG" 2>/dev/null)" = "$PDFIUM_TAG" ]; then
  echo "PDFium $PDFIUM_TAG already in $DEST"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
URL="https://github.com/bblanchon/pdfium-binaries/releases/download/$PDFIUM_TAG/$ARCHIVE"
echo "Downloading $URL"
curl -fL --retry 3 --silent --show-error -o "$TMP/$ARCHIVE" "$URL"
tar -xzf "$TMP/$ARCHIVE" -C "$TMP" "$MEMBER" LICENSE

mkdir -p "$DEST"
cp "$TMP/$MEMBER" "$DEST/$LIB"
cp "$TMP/LICENSE" "$DEST/LICENSE-pdfium.txt"
echo "$PDFIUM_TAG" > "$DEST/TAG"
echo "PDFium $PDFIUM_TAG installed in $DEST"
