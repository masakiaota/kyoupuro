#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_NAME="v011_hill_climb"
BIN_EXEC="$ROOT_DIR/target/release/$BIN_NAME"
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
VIS_BIN="$ROOT_DIR/tools/target/release/vis"
SEARCH_CSV="$ROOT_DIR/results/search_v011_weights.csv"

STEP=${1:-20}
SUBSET_CASES=${2:-12}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/search_v011_weights.sh [step] [subset_cases]

Search weight ratios for v011_hill_climb on a subset of tools/in.
Defaults:
  step=20
  subset_cases=12

The script:
  1. searches all triples (reinsert, replace1, replace2) summing to 100
  2. evaluates them on the first subset_cases inputs under tools/in
  3. writes all subset results to results/search_v011_weights.csv
  4. runs the best triple on all tools/in and appends only that full result to score_summary.csv
EOF
}

case "$STEP" in
    ''|*[!0-9]*)
        usage
        exit 1
        ;;
esac
case "$SUBSET_CASES" in
    ''|*[!0-9]*)
        usage
        exit 1
        ;;
esac

if [ "$STEP" -le 0 ] || [ "$STEP" -gt 100 ]; then
    echo "error: step must be in 1..100" >&2
    exit 1
fi
if [ $((100 % STEP)) -ne 0 ]; then
    echo "error: step must divide 100" >&2
    exit 1
fi
if [ "$SUBSET_CASES" -le 0 ]; then
    echo "error: subset_cases must be positive" >&2
    exit 1
fi

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

mkdir -p "$ROOT_DIR/results"
mkdir -p "$ROOT_DIR/results/out"

cargo build --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"
cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis

SUBSET_DIR=$(mktemp -d)
OUT_DIR=$(mktemp -d)
PAIR_LIST=$(mktemp)
trap 'rm -rf "$SUBSET_DIR" "$OUT_DIR"; rm -f "$PAIR_LIST"' EXIT

find "$ROOT_DIR/tools/in" -type f | sort | head -n "$SUBSET_CASES" | while IFS= read -r INPUT_FILE; do
    BASE_NAME=$(basename "$INPUT_FILE")
    ln -s "$INPUT_FILE" "$SUBSET_DIR/$BASE_NAME"
done

if [ -z "$(find "$SUBSET_DIR" -type f -o -type l | head -n 1)" ]; then
    echo "error: failed to prepare subset inputs" >&2
    exit 1
fi

printf "reinsert,replace1,replace2,total_sum,total_avg,total_cases\n" > "$SEARCH_CSV"

run_cases() {
    INPUT_DIR=$1
    OUTPUT_DIR=$2
    REINSERT=$3
    REPLACE1=$4
    REPLACE2=$5

    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR"
    : > "$PAIR_LIST"

    find "$INPUT_DIR" -type f -o -type l | sort | while IFS= read -r INPUT_FILE; do
        BASE_NAME=$(basename "$INPUT_FILE")
        printf '%s\0%s\0' "$INPUT_FILE" "$OUTPUT_DIR/$BASE_NAME" >> "$PAIR_LIST"
    done

    xargs -0 -n 2 -P "$PARALLEL" sh -c '
        bin_exec=$1
        reinsert=$2
        replace1=$3
        replace2=$4
        input_file=$5
        output_file=$6
        V011_REINSERT_WEIGHT=$reinsert \
        V011_REPLACE1_WEIGHT=$replace1 \
        V011_REPLACE2_WEIGHT=$replace2 \
        "$bin_exec" < "$input_file" > "$output_file"
    ' _ "$BIN_EXEC" "$REINSERT" "$REPLACE1" "$REPLACE2" < "$PAIR_LIST"
}

score_dir() {
    INPUT_DIR=$1
    OUTPUT_DIR=$2
    TOTAL_SUM=0
    TOTAL_CASES=0

    find "$INPUT_DIR" -type f -o -type l | sort | while IFS= read -r INPUT_FILE; do
        BASE_NAME=$(basename "$INPUT_FILE")
        OUTPUT_FILE="$OUTPUT_DIR/$BASE_NAME"
        TMP_DIR=$(mktemp -d)
        RESULT=$(cd "$TMP_DIR" && "$VIS_BIN" "$INPUT_FILE" "$OUTPUT_FILE" 2>/dev/null)
        rm -rf "$TMP_DIR"
        SCORE=$(printf "%s\n" "$RESULT" | awk '/^Score = / { print $3; exit }')
        if [ -z "$SCORE" ]; then
            echo "error: failed to parse score for $BASE_NAME" >&2
            exit 1
        fi
        TOTAL_SUM=$((TOTAL_SUM + SCORE))
        TOTAL_CASES=$((TOTAL_CASES + 1))
        printf "%s %s\n" "$TOTAL_SUM" "$TOTAL_CASES" > "$OUTPUT_DIR/.partial_score"
    done

    cat "$OUTPUT_DIR/.partial_score"
    rm -f "$OUTPUT_DIR/.partial_score"
}

BEST_SUM=-1
BEST_REINSERT=0
BEST_REPLACE1=0
BEST_REPLACE2=0

REINSERT=0
while [ "$REINSERT" -le 100 ]; do
    REPLACE1=0
    while [ $((REINSERT + REPLACE1)) -le 100 ]; do
        REPLACE2=$((100 - REINSERT - REPLACE1))
        TAG="r${REINSERT}_a${REPLACE1}_b${REPLACE2}"
        CANDIDATE_OUT="$OUT_DIR/$TAG"

        run_cases "$SUBSET_DIR" "$CANDIDATE_OUT" "$REINSERT" "$REPLACE1" "$REPLACE2"
        set -- $(score_dir "$SUBSET_DIR" "$CANDIDATE_OUT")
        TOTAL_SUM=$1
        TOTAL_CASES=$2
        TOTAL_AVG=$(awk "BEGIN { printf \"%.6f\", $TOTAL_SUM / $TOTAL_CASES }")
        printf "%s,%s,%s,%s,%s,%s\n" \
            "$REINSERT" "$REPLACE1" "$REPLACE2" "$TOTAL_SUM" "$TOTAL_AVG" "$TOTAL_CASES" >> "$SEARCH_CSV"

        if [ "$TOTAL_SUM" -gt "$BEST_SUM" ]; then
            BEST_SUM=$TOTAL_SUM
            BEST_REINSERT=$REINSERT
            BEST_REPLACE1=$REPLACE1
            BEST_REPLACE2=$REPLACE2
        fi

        REPLACE1=$((REPLACE1 + STEP))
    done
    REINSERT=$((REINSERT + STEP))
done

BEST_TAG="v011_hill_climb_r${BEST_REINSERT}_a${BEST_REPLACE1}_b${BEST_REPLACE2}"
BEST_OUTPUT_DIR="$ROOT_DIR/results/out/$BEST_TAG"
run_cases "$ROOT_DIR/tools/in" "$BEST_OUTPUT_DIR" "$BEST_REINSERT" "$BEST_REPLACE1" "$BEST_REPLACE2"
"$ROOT_DIR/scripts/score_tools.sh" "$BEST_TAG" "$ROOT_DIR/tools/in" "$BEST_OUTPUT_DIR"

printf "best_ratio=%s/%s/%s subset_sum=%s full_tag=%s\n" \
    "$BEST_REINSERT" "$BEST_REPLACE1" "$BEST_REPLACE2" "$BEST_SUM" "$BEST_TAG"
