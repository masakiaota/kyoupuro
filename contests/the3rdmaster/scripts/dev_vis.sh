#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

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

cd "$ROOT_DIR"
exec corepack yarn dev "$@"
