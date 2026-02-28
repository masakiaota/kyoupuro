#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

usage() {
    echo "Usage: $0 <bin_name>" >&2
}

if [ "$#" -ne 1 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
SRC="$ROOT_DIR/src/bin/$BIN_NAME.rs"
DST="$ROOT_DIR/src/main.rs"

if [ ! -f "$SRC" ]; then
    echo "error: not found: $SRC" >&2
    exit 1
fi

cp "$SRC" "$DST"

echo "promoted: $BIN_NAME -> src/main.rs" >&2
cargo build --release --quiet --offline --manifest-path "$ROOT_DIR/Cargo.toml"
echo "offline release build: OK" >&2
