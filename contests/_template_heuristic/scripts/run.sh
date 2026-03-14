#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
CSV_PATH="$ROOT_DIR/results/scores.csv"

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

mkdir -p "$ROOT_DIR/results"
if [ ! -f "$CSV_PATH" ]; then
    printf "timestamp,bin,input,elapsed_sec,score\n" > "$CSV_PATH"
fi

START=$(date +%s)
if [ -n "$INPUT_FILE" ]; then
    if [ ! -f "$INPUT_FILE" ]; then
        echo "error: input file not found: $INPUT_FILE" >&2
        exit 1
    fi
    cargo run --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME" < "$INPUT_FILE"
    INPUT_LABEL=$INPUT_FILE
else
    cargo run --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"
    INPUT_LABEL=-
fi
END=$(date +%s)
ELAPSED=$((END - START))
TS=$(date '+%Y-%m-%dT%H:%M:%S%z')

printf "%s,%s,%s,%s,%s\n" "$TS" "$BIN_NAME" "$INPUT_LABEL" "$ELAPSED" "$SCORE" >> "$CSV_PATH"
echo "logged: $CSV_PATH" >&2
