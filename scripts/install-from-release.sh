#!/usr/bin/env bash
# Download rradar CLI from GitHub Releases (no Rust toolchain required).
set -euo pipefail
REPO="${RRADAR_REPO:-taipei49314/receiptradar}"
TAG="${1:-latest}"
PREFIX="${RRADAR_INSTALL_PREFIX:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64|Linux-amd64) ARTIFACT="rradar-x86_64-unknown-linux-gnu" ; EXT="tar.gz" ;;
  Darwin-x86_64) ARTIFACT="rradar-x86_64-apple-darwin" ; EXT="tar.gz" ;;
  Darwin-arm64) ARTIFACT="rradar-aarch64-apple-darwin" ; EXT="tar.gz" ;;
  *)
    echo "unsupported platform: $os $arch — build from source (docs/INSTALL.md)" >&2
    exit 1
    ;;
esac

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

if [[ "$TAG" == "latest" ]]; then
  BASE="https://github.com/${REPO}/releases/latest/download"
else
  BASE="https://github.com/${REPO}/releases/download/${TAG}"
fi

echo "fetch ${ARTIFACT}.${EXT} ($TAG)"
curl -fsSL -o "$TMP/pkg.${EXT}" "${BASE}/${ARTIFACT}.${EXT}"
curl -fsSL -o "$TMP/pkg.sha256" "${BASE}/${ARTIFACT}.sha256" || true
if [[ -f "$TMP/pkg.sha256" ]]; then
  (cd "$TMP" && sed "s|  ${ARTIFACT}.${EXT}|  pkg.${EXT}|" pkg.sha256 | sha256sum -c -) || {
    echo "checksum failed" >&2
    exit 1
  }
fi

mkdir -p "$TMP/out"
tar -xzf "$TMP/pkg.${EXT}" -C "$TMP/out"
BIN="$(find "$TMP/out" -type f -name rradar | head -n1)"
if [[ -z "$BIN" ]]; then
  echo "rradar binary not found in archive" >&2
  exit 1
fi
mkdir -p "$PREFIX"
install -m 755 "$BIN" "$PREFIX/rradar"
echo "installed $PREFIX/rradar"
"$PREFIX/rradar" version --long
