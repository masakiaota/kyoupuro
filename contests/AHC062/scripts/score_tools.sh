#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"

usage() {
    echo "Usage: $0 <args...>" >&2
    echo "example: $0 ./tools/in/0000.txt ./out/0000.txt" >&2
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

if [ -f "$ROOT_DIR/tools/src/bin/score.rs" ]; then
    cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score
    exec "$ROOT_DIR/tools/target/release/score" "$@"
fi

if [ -f "$ROOT_DIR/tools/src/bin/vis.rs" ]; then
    exec "$SCRIPT_DIR/vis_tools.sh" "$@"
fi

echo "error: neither score nor vis binary was found under tools/src/bin" >&2
exit 1
