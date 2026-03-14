#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

usage() {
    echo "Usage: $0 <bin_name> [input_file] [score]" >&2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
INPUT_FILE=${2:-}
SCORE=${3:--}
BIN_SRC="$ROOT_DIR/src/bin/$BIN_NAME.rs"

if [ ! -f "$BIN_SRC" ]; then
    echo "error: not found: $BIN_SRC" >&2
    exit 1
fi

OUTPUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
mkdir -p "$OUTPUT_DIR"

START=$(date +%s)
if [ -n "$INPUT_FILE" ]; then
    if [ ! -f "$INPUT_FILE" ]; then
        echo "error: input file not found: $INPUT_FILE" >&2
        exit 1
    fi
    OUTPUT_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE")"
    cargo run --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME" < "$INPUT_FILE" > "$OUTPUT_FILE"
    cat "$OUTPUT_FILE"
    INPUT_LABEL=$INPUT_FILE
else
    cargo run --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"
    OUTPUT_FILE=-
    INPUT_LABEL=-
fi
END=$(date +%s)
ELAPSED=$((END - START))
echo "bin=$BIN_NAME input=$INPUT_LABEL elapsed=${ELAPSED}s score=$SCORE output=$OUTPUT_FILE"
