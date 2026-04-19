#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
VIS_BIN="$ROOT_DIR/tools/target/release/vis"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/vis_tools.sh <input_file> <output_file>
  ./scripts/vis_tools.sh <input_file> <output_file> <html_out>

Runs official visualizer and prints score.
If html_out is omitted, html is saved under results/vis/.
EOF
}

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

INPUT_FILE=$1
OUTPUT_FILE=$2
HTML_OUT=${3:-}

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: official tools を tools/ に展開してから実行する" >&2
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

if [ ! -x "$VIS_BIN" ]; then
    cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis
fi

if [ -z "$HTML_OUT" ]; then
    INPUT_BASE=$(basename "$INPUT_FILE")
    OUTPUT_BASE=$(basename "$OUTPUT_FILE")
    mkdir -p "$ROOT_DIR/results/vis"
    HTML_OUT="$ROOT_DIR/results/vis/${INPUT_BASE}_${OUTPUT_BASE}.html"
else
    mkdir -p "$(dirname "$HTML_OUT")"
fi

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if VIS_STDOUT=$(
    cd "$TMP_DIR"
    "$VIS_BIN" "$INPUT_FILE" "$OUTPUT_FILE"
); then
    STATUS=0
else
    STATUS=$?
fi

printf '%s\n' "$VIS_STDOUT"

if [ -f "$TMP_DIR/vis.html" ]; then
    cp "$TMP_DIR/vis.html" "$HTML_OUT"
    printf 'vis: html=%s\n' "$HTML_OUT" >&2
fi

exit "$STATUS"
