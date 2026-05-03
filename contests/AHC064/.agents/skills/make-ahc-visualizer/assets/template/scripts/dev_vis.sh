#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
WASM_ENTRY="$ROOT_DIR/src_vis/wasm/heuristic_contest_template_vis.js"

if ! command -v corepack >/dev/null 2>&1; then
    echo "error: corepack command not found" >&2
    exit 1
fi

if [ ! -d "$ROOT_DIR/node_modules" ]; then
    (
        cd "$ROOT_DIR"
        corepack yarn install
    )
fi

if [ ! -f "$WASM_ENTRY" ]; then
    "$SCRIPT_DIR/build_wasm.sh"
fi

cd "$ROOT_DIR"
exec corepack yarn dev "$@"
