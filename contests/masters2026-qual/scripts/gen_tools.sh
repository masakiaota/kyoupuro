#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_DIR="$ROOT_DIR/tools"
TOOLS_MANIFEST="$TOOLS_DIR/Cargo.toml"

usage() {
    echo "Usage: $0 <problem_id> [seeds_file] [output_dir]" >&2
    echo "  problem_id: A | B | C" >&2
    echo "  seeds_file: default tools/seeds.txt" >&2
    echo "  output_dir: default tools/in" >&2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

PROBLEM=$(printf '%s' "$1" | tr '[:lower:]' '[:upper:]')
SEEDS_FILE=${2:-"$TOOLS_DIR/seeds.txt"}
OUTPUT_DIR=${3:-"$TOOLS_DIR/in"}

case "$PROBLEM" in
    A|B|C) ;;
    *)
        echo "error: problem_id must be one of A, B, C: $PROBLEM" >&2
        exit 1
        ;;
esac

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    exit 1
fi

if [ ! -f "$SEEDS_FILE" ]; then
    echo "error: seeds file not found: $SEEDS_FILE" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

cargo run --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin gen -- "$SEEDS_FILE" "$PROBLEM" --dir "$OUTPUT_DIR"

echo "generated: $OUTPUT_DIR (problem=$PROBLEM)" >&2
