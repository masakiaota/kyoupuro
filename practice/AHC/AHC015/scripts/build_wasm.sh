#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
WASM_DIR="$ROOT_DIR/wasm"
OUT_DIR="$ROOT_DIR/public/wasm"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "error: wasm-pack command not found" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

(
    cd "$WASM_DIR"
    cargo check
    wasm-pack build --target web --out-dir "$OUT_DIR"
)

echo "built wasm: $OUT_DIR" >&2
