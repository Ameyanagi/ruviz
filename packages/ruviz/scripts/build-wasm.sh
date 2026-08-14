#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../.." && pwd)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
# Optional size/speed knob for the wasm module. Unset means the workspace
# release profile applies unchanged (opt-level 3). Setting `s` or `z` trades
# rasterization speed for module size; see docs/wasm-size.md for measurements.
#
# It has to be threaded through RUSTFLAGS here rather than through
# .cargo/config.toml, because a RUSTFLAGS *environment variable* — which this
# script exports for --remap-path-prefix — overrides the
# [target.wasm32-unknown-unknown] rustflags table wholesale.
RUSTFLAGS_OPT_LEVEL=""
if [[ -n "${RUVIZ_WASM_OPT_LEVEL:-}" ]]; then
  RUSTFLAGS_OPT_LEVEL="-C opt-level=${RUVIZ_WASM_OPT_LEVEL} "
fi
RUSTFLAGS_PREFIX="${RUSTFLAGS_OPT_LEVEL}--remap-path-prefix=${CARGO_HOME_DIR}=/cargo --remap-path-prefix=${REPO_ROOT}=/workspace"
WASM_PACK_BIN="$(bash "${SCRIPT_DIR}/ensure-wasm-pack.sh")"

export RUSTFLAGS="${RUSTFLAGS_PREFIX}${RUSTFLAGS:+ ${RUSTFLAGS}}"
export PATH="${CARGO_HOME_DIR}/bin:${PATH}"

cd "${PACKAGE_DIR}"
WASM_PACK_ARGS=(
  build
  ../../bindings/wasm
  --target
  web
  --out-dir
  "${PACKAGE_DIR}/generated/raw"
  --out-name
  ruviz_web_raw
)

# The notebook widget bundle is checked into the repo, so it needs a
# platform-independent wasm artifact. `wasm-opt` output can vary by host.
if [[ "${RUVIZ_WASM_PACK_NO_OPT:-0}" == "1" ]]; then
  WASM_PACK_ARGS+=(--no-opt)
fi

WASM_PACK_ARGS+=(--features 3d-gpu)

"${WASM_PACK_BIN}" "${WASM_PACK_ARGS[@]}"

cat >"${PACKAGE_DIR}/generated/raw/.npmignore" <<'EOF'
# Prevent npm from inheriting wasm-pack's generated .gitignore during package
# creation. The package.json "files" allowlist is the source of truth.
EOF
