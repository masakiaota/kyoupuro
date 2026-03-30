#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_DIR="$ROOT_DIR/tools"
TOOLS_MANIFEST="$TOOLS_DIR/Cargo.toml"

usage() {
    echo "Usage: $0 <args...>" >&2
    echo "example: $0 ./tools/seeds.txt" >&2
}

resolve_path() {
    case "$1" in
        /*) printf '%s\n' "$1" ;;
        *) printf '%s/%s\n' "$PWD" "$1" ;;
    esac
}

if [ "$#" -lt 1 ]; then
    usage
    exit 1
fi

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: official tools を tools/ に展開してから実行する" >&2
    exit 1
fi

SEEDS_FILE=$(resolve_path "$1")
shift

if [ ! -f "$SEEDS_FILE" ]; then
    echo "error: seeds file not found: $SEEDS_FILE" >&2
    exit 1
fi

(
    cd "$TOOLS_DIR"
    cargo run --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin gen -- "$SEEDS_FILE" "$@"
)
