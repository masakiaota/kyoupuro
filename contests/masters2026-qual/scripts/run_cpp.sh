#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
CSV_PATH="$ROOT_DIR/results/scores.csv"

# shellcheck source=scripts/cpp_common.sh
. "$SCRIPT_DIR/cpp_common.sh"

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
BIN_SRC="$ROOT_DIR/cpp/bin/$BIN_NAME.cpp"
BIN_LABEL="cpp/$BIN_NAME"
BUILD_DIR="$ROOT_DIR/target/cpp"
BIN_PATH="$BUILD_DIR/$BIN_NAME"

if [ ! -f "$BIN_SRC" ]; then
    echo "error: not found: $BIN_SRC" >&2
    exit 1
fi

if ! CXX_BIN=$(detect_cxx); then
    echo "error: C++ compiler not found. Install g++/clang++ or set CXX." >&2
    exit 1
fi

if ! STD_FLAG=$(detect_cpp_std_flag "$CXX_BIN"); then
    echo "error: compiler '$CXX_BIN' does not support C++23/C++2b." >&2
    exit 1
fi

mkdir -p "$BUILD_DIR" "$ROOT_DIR/results"
if [ ! -f "$CSV_PATH" ]; then
    printf "timestamp,bin,input,elapsed_sec,score\n" > "$CSV_PATH"
fi

echo "compile: $CXX_BIN $STD_FLAG $BIN_SRC" >&2
compile_cpp "$CXX_BIN" "$STD_FLAG" "$BIN_SRC" "$BIN_PATH"

START=$(date +%s)
if [ -n "$INPUT_FILE" ]; then
    if [ ! -f "$INPUT_FILE" ]; then
        echo "error: input file not found: $INPUT_FILE" >&2
        exit 1
    fi
    "$BIN_PATH" < "$INPUT_FILE"
    INPUT_LABEL=$INPUT_FILE
else
    "$BIN_PATH"
    INPUT_LABEL=-
fi
END=$(date +%s)
ELAPSED=$((END - START))
TS=$(date '+%Y-%m-%dT%H:%M:%S%z')

printf "%s,%s,%s,%s,%s\n" "$TS" "$BIN_LABEL" "$INPUT_LABEL" "$ELAPSED" "$SCORE" >> "$CSV_PATH"
echo "logged: $CSV_PATH" >&2
