#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if [ ! -d "$ROOT_DIR/node_modules" ]; then
    (
        cd "$ROOT_DIR"
        if command -v yarn >/dev/null 2>&1; then
            yarn install
        elif command -v corepack >/dev/null 2>&1; then
            corepack yarn install
        else
            echo "error: neither yarn nor corepack is available" >&2
            exit 1
        fi
    )
fi

cd "$ROOT_DIR"
if command -v yarn >/dev/null 2>&1; then
    exec yarn dev "$@"
fi
if command -v corepack >/dev/null 2>&1; then
    exec corepack yarn dev "$@"
fi
echo "error: neither yarn nor corepack is available" >&2
exit 1
