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

if [ ! -f "$SRC" ]; then
    echo "error: not found: $SRC" >&2
    exit 1
fi

echo "verifying submission candidate: $SRC" >&2
cargo build --release --quiet --offline --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"
echo "offline release build: OK ($BIN_NAME)" >&2
echo "submit this file: $SRC" >&2
