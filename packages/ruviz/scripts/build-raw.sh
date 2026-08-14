#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../.." && pwd)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${REPO_ROOT}/target}"
RAW_OUT_DIR="${PACKAGE_DIR}/generated/raw"
INPUT_WASM="${CARGO_TARGET_DIR}/wasm32-unknown-unknown/release/ruviz_web.wasm"
# See build-wasm.sh: RUVIZ_WASM_OPT_LEVEL is the optional size/speed knob, and
# it must be threaded through RUSTFLAGS because the environment variable this
# script exports overrides any [target.*] rustflags table wholesale.
RUSTFLAGS_OPT_LEVEL=""
if [[ -n "${RUVIZ_WASM_OPT_LEVEL:-}" ]]; then
  RUSTFLAGS_OPT_LEVEL="-C opt-level=${RUVIZ_WASM_OPT_LEVEL} "
fi
RUSTFLAGS_PREFIX="${RUSTFLAGS_OPT_LEVEL}--remap-path-prefix=${CARGO_HOME_DIR}=/cargo --remap-path-prefix=${REPO_ROOT}=/workspace"
WASM_BINDGEN_BIN_DIR="$(bash "${SCRIPT_DIR}/ensure-wasm-bindgen.sh")"

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export RUSTFLAGS="${RUSTFLAGS_PREFIX}${RUSTFLAGS:+ ${RUSTFLAGS}}"
export PATH="${WASM_BINDGEN_BIN_DIR}:${CARGO_HOME_DIR}/bin:${PATH}"

mkdir -p "${RAW_OUT_DIR}"

# The checked-in Python widget compares consecutive bundle rebuilds, so avoid
# wasm-pack's managed bindgen bootstrap and invoke the pinned CLI directly.
cargo build \
  --locked \
  --manifest-path "${REPO_ROOT}/bindings/wasm/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --features 3d-gpu \
  --release

wasm-bindgen \
  "${INPUT_WASM}" \
  --target web \
  --out-dir "${RAW_OUT_DIR}" \
  --out-name ruviz_web_raw

cat >"${RAW_OUT_DIR}/.npmignore" <<'EOF'
# Prevent npm from inheriting wasm-bindgen's generated .gitignore during
# package creation. The package.json "files" allowlist is the source of truth.
EOF
