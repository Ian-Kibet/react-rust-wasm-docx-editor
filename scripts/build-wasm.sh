#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$ROOT_DIR/crates/docx-wasm"
OUT_DIR="$ROOT_DIR/app/src/wasm/pkg"

echo "Building WASM from $CRATE_DIR..."

export PATH="$HOME/.cargo/bin:$PATH"

wasm-pack build "$CRATE_DIR" \
  --target web \
  --out-dir "$OUT_DIR" \
  --out-name docx_wasm

echo "WASM build complete -> $OUT_DIR"
