#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
DEFAULT_HTML="$ROOT_DIR/results/vis.html"

usage() {
    echo "Usage: $0 <input_file> <output_file> [html_output_path]" >&2
    echo "example: $0 ./tools/in/0000.txt ./results/out/0000.txt" >&2
}

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: official tools を tools/ に展開してから実行する" >&2
    exit 1
fi

INPUT_FILE=$1
OUTPUT_FILE=$2
HTML_PATH=${3:-$DEFAULT_HTML}

if [ ! -f "$INPUT_FILE" ]; then
    echo "error: input file not found: $INPUT_FILE" >&2
    exit 1
fi

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "error: output file not found: $OUTPUT_FILE" >&2
    exit 1
fi

mkdir -p "$ROOT_DIR/results"
mkdir -p "$(dirname -- "$HTML_PATH")"
ABS_INPUT=$(CDPATH= cd -- "$(dirname -- "$INPUT_FILE")" && pwd)/$(basename -- "$INPUT_FILE")
ABS_OUTPUT=$(CDPATH= cd -- "$(dirname -- "$OUTPUT_FILE")" && pwd)/$(basename -- "$OUTPUT_FILE")
ABS_HTML_DIR=$(CDPATH= cd -- "$(dirname -- "$HTML_PATH")" && pwd)
ABS_HTML=$ABS_HTML_DIR/$(basename -- "$HTML_PATH")

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

(
    cd "$TMP_DIR"
    cargo run --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis -- "$ABS_INPUT" "$ABS_OUTPUT"
)

if [ ! -f "$TMP_DIR/vis.html" ]; then
    echo "error: vis.html was not generated" >&2
    exit 1
fi

mv "$TMP_DIR/vis.html" "$ABS_HTML"
echo "saved html: $ABS_HTML" >&2
