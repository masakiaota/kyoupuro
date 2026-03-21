#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_PATH="$ROOT_DIR/target/release"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/run.sh <bin_name>
  ./scripts/run.sh <bin_name> <input_file|input_dir>
  ./scripts/run.sh <bin_name> <input_file|input_dir> [score]

Run binary for one input, or for all files under input_dir in parallel (cpu//2 workers).
Default score tag is `-` when omitted.
EOF
}

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
TARGET=${2:-}
SCORE=${3:--}
BIN_SRC="$ROOT_DIR/src/bin/$BIN_NAME.rs"

if [ ! -f "$BIN_SRC" ]; then
    echo "error: not found: $BIN_SRC" >&2
    exit 1
fi

OUTPUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
mkdir -p "$OUTPUT_DIR"

CPU_COUNT=$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)
if [ -z "$CPU_COUNT" ] && command -v sysctl >/dev/null 2>&1; then
    CPU_COUNT=$(sysctl -n hw.ncpu 2>/dev/null || true)
fi
if [ -z "$CPU_COUNT" ] || [ "$CPU_COUNT" -le 0 ]; then
    CPU_COUNT=2
fi
PARALLEL=$((CPU_COUNT / 2))
if [ "$PARALLEL" -lt 1 ]; then
    PARALLEL=1
fi

START_ALL=$(date +%s)
cargo build --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"
BIN_EXEC="$BIN_PATH/$BIN_NAME"
if [ ! -x "$BIN_EXEC" ]; then
    echo "error: binary not found: $BIN_EXEC" >&2
    exit 1
fi

if [ -z "${TARGET:-}" ]; then
    "$BIN_EXEC"
    INPUT_LABEL=-
    OUTPUT_FILE=-
elif [ -f "$TARGET" ]; then
    INPUT_FILE=$TARGET
    OUTPUT_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE")"
    START=$(date +%s)
    "$BIN_EXEC" < "$INPUT_FILE" > "$OUTPUT_FILE"
    cat "$OUTPUT_FILE"
    ELAPSED=$(( $(date +%s) - START ))
    echo "bin=$BIN_NAME input=$INPUT_FILE elapsed=${ELAPSED}s score=$SCORE output=$OUTPUT_FILE"
    exit 0
elif [ -d "$TARGET" ]; then
    INPUT_DIR="$TARGET"
    INPUT_LIST=$(mktemp)
    BASE_LIST=$(mktemp)
    PAIR_LIST=$(mktemp)
    RESULTS=$(mktemp)
    trap 'rm -f "$INPUT_LIST" "$BASE_LIST" "$PAIR_LIST" "$RESULTS"' EXIT

    find "$INPUT_DIR" -type f | sort > "$INPUT_LIST"
    INPUT_COUNT=$(wc -l < "$INPUT_LIST" | tr -d ' ')
    if [ "$INPUT_COUNT" -eq 0 ]; then
        echo "error: input directory is empty: $INPUT_DIR" >&2
        exit 1
    fi

    while IFS= read -r INPUT_FILE; do
        basename "$INPUT_FILE" >> "$BASE_LIST"
    done < "$INPUT_LIST"

    DUPLICATES=$(mktemp)
    trap 'rm -f "$INPUT_LIST" "$BASE_LIST" "$DUPLICATES" "$PAIR_LIST" "$RESULTS"' EXIT
    sort "$BASE_LIST" | uniq -d > "$DUPLICATES"
    if [ -s "$DUPLICATES" ]; then
        echo "error: input directory contains duplicate basenames; results would collide" >&2
        echo "files with duplicated basename:" >&2
        cat "$DUPLICATES" >&2
        exit 1
    fi

    while IFS= read -r INPUT_FILE; do
        BASE_NAME=$(basename "$INPUT_FILE")
        OUTPUT_FILE="$OUTPUT_DIR/$BASE_NAME"
        printf '%s\0%s\0' "$INPUT_FILE" "$OUTPUT_FILE" >> "$PAIR_LIST"
    done < "$INPUT_LIST"

    printf "run: bin=%s input_dir=%s parallel=%s\n" "$BIN_NAME" "$INPUT_DIR" "$PARALLEL" >&2

    xargs -0 -n 2 -P "$PARALLEL" sh -c '
        bin_path=$1
        input_file=$2
        output_file=$3
        start=$(date +%s)
        if "$bin_path" < "$input_file" > "$output_file" 2> "${output_file}.err"; then
            status=0
        else
            status=$?
        fi
        elapsed=$(( $(date +%s) - start ))
        echo "$input_file" "$status" "$elapsed" "$output_file"
    ' _ "$BIN_EXEC" < "$PAIR_LIST" > "$RESULTS"

    STATUS=$?
    if [ "$STATUS" -ne 0 ]; then
        echo "error: run failed (some cases might have failed)" >&2
    fi

    SUCCESS=0
    FAILURE=0
    TOTAL_ELAPSED=0
    while IFS=' ' read -r INPUT_FILE_CASE STATUS_CASE ELAPSED_CASE OUTPUT_CASE; do
        TOTAL_ELAPSED=$((TOTAL_ELAPSED + ELAPSED_CASE))
        if [ "$STATUS_CASE" -eq 0 ]; then
            SUCCESS=$((SUCCESS + 1))
        else
            FAILURE=$((FAILURE + 1))
            echo "[warn] failed: $INPUT_FILE_CASE (output: $OUTPUT_CASE.err)" >&2
        fi
    done < "$RESULTS"

    END_ALL=$(date +%s)
    ELAPSED_ALL=$((END_ALL - START_ALL))
    echo "bin=$BIN_NAME input=$INPUT_DIR cases=$INPUT_COUNT success=$SUCCESS failure=$FAILURE elapsed=${ELAPSED_ALL}s score=$SCORE output=$OUTPUT_DIR"
    if [ "$FAILURE" -ne 0 ]; then
        exit 1
    fi
    exit 0
else
    echo "error: input not found: $TARGET" >&2
    usage
    exit 1
fi

END_ALL=$(date +%s)
ELAPSED_ALL=$((END_ALL - START_ALL))
echo "bin=$BIN_NAME input=$INPUT_LABEL elapsed=${ELAPSED_ALL}s score=$SCORE output=$OUTPUT_FILE"
