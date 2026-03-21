#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
VIS_BIN="$ROOT_DIR/tools/target/release/vis"

abs_file_path() {
    FILE_PATH=$1
    FILE_DIR=$(dirname "$FILE_PATH")
    FILE_NAME=$(basename "$FILE_PATH")
    FILE_DIR_ABS=$(CDPATH= cd -- "$FILE_DIR" && pwd)
    printf "%s/%s\n" "$FILE_DIR_ABS" "$FILE_NAME"
}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/vis_tools.sh <input_file> <output_file> [svg_file]

Example:
  ./scripts/vis_tools.sh ./tools/in/0000.txt ./results/out/v001_template/0000.txt
  ./scripts/vis_tools.sh ./tools/in/0000.txt ./results/out/v001_template/0000.txt ./results/vis/0000.svg

If svg_file is omitted:
  ./results/vis/<output_basename>.svg
EOF
}

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

INPUT_FILE=$1
OUTPUT_FILE=$2
if [ "$#" -eq 3 ]; then
    SVG_FILE=$3
else
    BASE_NAME=$(basename "$OUTPUT_FILE")
    BASE_STEM=${BASE_NAME%.*}
    if [ "$BASE_STEM" = "$BASE_NAME" ]; then
        BASE_STEM=$BASE_NAME
    fi
    SVG_FILE="$ROOT_DIR/results/vis/$BASE_STEM.svg"
fi

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

INPUT_FILE=$(abs_file_path "$INPUT_FILE")
OUTPUT_FILE=$(abs_file_path "$OUTPUT_FILE")
case "$SVG_FILE" in
    /*) ;;
    *) SVG_FILE="$(pwd)/$SVG_FILE" ;;
esac

cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis
if [ ! -x "$VIS_BIN" ]; then
    echo "error: vis binary not found: $VIS_BIN" >&2
    exit 1
fi

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if RESULT=$(cd "$TMP_DIR" && "$VIS_BIN" "$INPUT_FILE" "$OUTPUT_FILE" 2>&1); then
    :
else
    STATUS=$?
    printf "%s\n" "$RESULT" >&2
    exit "$STATUS"
fi

if [ ! -f "$TMP_DIR/out.svg" ]; then
    echo "error: vis did not produce out.svg" >&2
    printf "%s\n" "$RESULT" >&2
    exit 1
fi

SVG_DIR=$(dirname "$SVG_FILE")
mkdir -p "$SVG_DIR"
cp "$TMP_DIR/out.svg" "$SVG_FILE"

SCORE=$(printf "%s\n" "$RESULT" | awk '/^Score = / { print $3; exit }')
if [ -z "$SCORE" ]; then
    SCORE="-"
fi

printf "%s\n" "$RESULT" >&2
echo "score=$SCORE svg=$SVG_FILE"
