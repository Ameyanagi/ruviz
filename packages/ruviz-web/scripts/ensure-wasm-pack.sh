#!/usr/bin/env bash
set -euo pipefail

WASM_PACK_VERSION="0.14.0"
WASM_PACK_REPO="https://github.com/wasm-bindgen/wasm-pack/releases/download"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/ruviz-tools/wasm-pack/${WASM_PACK_VERSION}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

detect_target() {
  local os
  local arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "${arch}" in
    x86_64 | x86-64 | amd64 | x64)
      arch="x86_64"
      ;;
    arm64 | aarch64)
      arch="aarch64"
      ;;
    *)
      echo "unsupported CPU architecture for wasm-pack bootstrap: ${arch}" >&2
      exit 1
      ;;
  esac

  case "${os}" in
    Linux | linux)
      printf '%s\n' "${arch}-unknown-linux-musl"
      ;;
    Darwin)
      printf '%s\n' "${arch}-apple-darwin"
      ;;
    *)
      echo "unsupported OS for wasm-pack bootstrap: ${os}" >&2
      exit 1
      ;;
  esac
}

need_cmd curl
need_cmd tar
need_cmd mktemp

TARGET="$(detect_target)"
INSTALL_DIR="${CACHE_ROOT}/${TARGET}"
BINARY_PATH="${INSTALL_DIR}/wasm-pack"
EXPECTED_VERSION="wasm-pack ${WASM_PACK_VERSION}"

if [[ -x "${BINARY_PATH}" ]] && [[ "$("${BINARY_PATH}" --version 2>/dev/null || true)" == "${EXPECTED_VERSION}" ]]; then
  printf '%s\n' "${BINARY_PATH}"
  exit 0
fi

ARCHIVE_NAME="wasm-pack-v${WASM_PACK_VERSION}-${TARGET}.tar.gz"
ARCHIVE_URL="${WASM_PACK_REPO}/v${WASM_PACK_VERSION}/${ARCHIVE_NAME}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "info: downloading wasm-pack ${WASM_PACK_VERSION} for ${TARGET}" >&2
curl -fsSL "${ARCHIVE_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}"
tar xf "${TMP_DIR}/${ARCHIVE_NAME}" --strip-components 1 -C "${TMP_DIR}"

mkdir -p "${INSTALL_DIR}"
cp "${TMP_DIR}/wasm-pack" "${BINARY_PATH}"
chmod 755 "${BINARY_PATH}"

if [[ "$("${BINARY_PATH}" --version 2>/dev/null || true)" != "${EXPECTED_VERSION}" ]]; then
  echo "downloaded wasm-pack does not report expected version ${WASM_PACK_VERSION}" >&2
  exit 1
fi

printf '%s\n' "${BINARY_PATH}"
