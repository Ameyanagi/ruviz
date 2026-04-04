#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../.." && pwd)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/ruviz-tools/wasm-bindgen"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

need_cmd cargo

WASM_BINDGEN_VERSION="$(
  awk '
    $0 == "[[package]]" { in_package = 0 }
    $0 == "name = \"wasm-bindgen\"" { in_package = 1; next }
    in_package && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "${REPO_ROOT}/Cargo.lock"
)"

if [[ -z "${WASM_BINDGEN_VERSION}" ]]; then
  echo "failed to determine wasm-bindgen version from Cargo.lock" >&2
  exit 1
fi

INSTALL_DIR="${CACHE_ROOT}/${WASM_BINDGEN_VERSION}"
BIN_DIR="${INSTALL_DIR}/bin"
BINARY_PATH="${BIN_DIR}/wasm-bindgen"
EXPECTED_VERSION="wasm-bindgen ${WASM_BINDGEN_VERSION}"

if [[ -x "${BINARY_PATH}" ]] && [[ "$("${BINARY_PATH}" --version 2>/dev/null || true)" == "${EXPECTED_VERSION}" ]]; then
  printf '%s\n' "${BIN_DIR}"
  exit 0
fi

mkdir -p "${BIN_DIR}"
export PATH="${CARGO_HOME_DIR}/bin:${PATH}"

echo "info: installing wasm-bindgen-cli ${WASM_BINDGEN_VERSION}" >&2
cargo install \
  wasm-bindgen-cli \
  --version "${WASM_BINDGEN_VERSION}" \
  --locked \
  --root "${INSTALL_DIR}"

if [[ "$("${BINARY_PATH}" --version 2>/dev/null || true)" != "${EXPECTED_VERSION}" ]]; then
  echo "installed wasm-bindgen does not report expected version ${WASM_BINDGEN_VERSION}" >&2
  exit 1
fi

printf '%s\n' "${BIN_DIR}"
