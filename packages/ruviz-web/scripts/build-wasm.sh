#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../.." && pwd)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
RUSTFLAGS_PREFIX="--remap-path-prefix=${CARGO_HOME_DIR}=/cargo --remap-path-prefix=${REPO_ROOT}=/workspace"

export RUSTFLAGS="${RUSTFLAGS_PREFIX}${RUSTFLAGS:+ ${RUSTFLAGS}}"
export PATH="${HOME}/.cargo/bin:${PATH}"

cd "${PACKAGE_DIR}"
wasm-pack build ../../crates/ruviz-web --target web --out-dir "${PACKAGE_DIR}/generated/raw" --out-name ruviz_web_raw
