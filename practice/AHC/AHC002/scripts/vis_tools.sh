#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_DIR="$ROOT_DIR/tools"
TOOLS_MANIFEST="$TOOLS_DIR/Cargo.toml"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/vis_tools.sh <input_file> <output_file> [svg_out]

If svg_out is omitted, tools/out.svg is overwritten.
EOF
}

resolve_path() {
    case "$1" in
        /*) printf '%s\n' "$1" ;;
        *) printf '%s/%s\n' "$PWD" "$1" ;;
    esac
}

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: ./scripts/unpack_tools.sh を先に実行する" >&2
    exit 1
fi

INPUT_FILE=$(resolve_path "$1")
OUTPUT_FILE=$(resolve_path "$2")
SVG_OUT=${3:-"$TOOLS_DIR/out.svg"}
SVG_OUT=$(resolve_path "$SVG_OUT")

if [ ! -f "$INPUT_FILE" ]; then
    echo "error: input file not found: $INPUT_FILE" >&2
    exit 1
fi
if [ ! -f "$OUTPUT_FILE" ]; then
    echo "error: output file not found: $OUTPUT_FILE" >&2
    exit 1
fi

SVG_DIR=$(dirname -- "$SVG_OUT")
mkdir -p "$SVG_DIR"

(
    cd "$TOOLS_DIR"
    cargo run --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis -- "$INPUT_FILE" "$OUTPUT_FILE"
)

if [ "$TOOLS_DIR/out.svg" != "$SVG_OUT" ]; then
    cp "$TOOLS_DIR/out.svg" "$SVG_OUT"
fi

printf 'svg: %s\n' "$SVG_OUT" >&2
