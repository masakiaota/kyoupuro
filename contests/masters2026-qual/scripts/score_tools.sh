#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_DIR="$ROOT_DIR/tools"
TOOLS_MANIFEST="$TOOLS_DIR/Cargo.toml"

usage() {
    echo "Usage: $0 <input_file> <output_file>" >&2
}

if [ "$#" -ne 2 ]; then
    usage
    exit 1
fi

INPUT_FILE=$1
OUTPUT_FILE=$2

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    exit 1
fi

if [ ! -f "$INPUT_FILE" ]; then
    echo "error: input file not found: $INPUT_FILE" >&2
    exit 1
fi

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "error: output file not found: $OUTPUT_FILE" >&2
    exit 1
fi

cargo run --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score -- "$INPUT_FILE" "$OUTPUT_FILE"
