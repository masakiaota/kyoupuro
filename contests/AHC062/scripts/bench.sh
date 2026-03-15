#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
ROOT_MANIFEST="$ROOT_DIR/Cargo.toml"

default_jobs() {
    JOBS=$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)
    case "${JOBS:-}" in
        ''|*[!0-9]*)
            JOBS=4
            ;;
    esac
    if [ "$JOBS" -lt 1 ]; then
        JOBS=1
    fi
    printf "%s\n" "$JOBS"
}

usage() {
    cat >&2 <<'EOF'
Usage: ./scripts/bench.sh [--jobs N] [--out-dir DIR] <bin_name> [input_dir]

example:
  ./scripts/bench.sh v400
  ./scripts/bench.sh --jobs 8 v400 ./tools/in
EOF
}

worker_main() {
    BIN_EXE=$1
    SCORE_EXE=$2
    INPUT_FILE=$3
    OUT_DIR=$4
    TMP_RESULTS_DIR=$5

    CASE_NAME=$(basename -- "$INPUT_FILE")
    STEM=${CASE_NAME%.txt}
    OUT_FILE="$OUT_DIR/$CASE_NAME"
    STDERR_FILE="$OUT_DIR/$STEM.stderr"

    START=$(date +%s)
    if "$BIN_EXE" < "$INPUT_FILE" > "$OUT_FILE" 2> "$STDERR_FILE"; then
        SCORE_OUTPUT=$("$SCORE_EXE" "$INPUT_FILE" "$OUT_FILE" 2>> "$STDERR_FILE" || true)
        SCORE=$(printf "%s\n" "$SCORE_OUTPUT" | awk '/^Score = / { print $3 }' | tail -n 1)
        if [ -z "${SCORE:-}" ]; then
            SCORE=0
        fi
    else
        SCORE=0
    fi
    END=$(date +%s)
    ELAPSED=$((END - START))

    if [ ! -s "$STDERR_FILE" ]; then
        rm -f "$STDERR_FILE"
    fi

    printf "%s,%s,%s\n" "$CASE_NAME" "$ELAPSED" "$SCORE" > "$TMP_RESULTS_DIR/$CASE_NAME.csv"
}

if [ "${1:-}" = "__worker" ]; then
    shift
    worker_main "$@"
    exit 0
fi

JOBS=$(default_jobs)
CUSTOM_OUT_DIR=

while [ "$#" -gt 0 ]; do
    case "$1" in
        -j|--jobs)
            if [ "$#" -lt 2 ]; then
                usage
                exit 1
            fi
            JOBS=$2
            shift 2
            ;;
        --out-dir)
            if [ "$#" -lt 2 ]; then
                usage
                exit 1
            fi
            CUSTOM_OUT_DIR=$2
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "error: unknown option: $1" >&2
            usage
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    usage
    exit 1
fi

case "$JOBS" in
    ''|*[!0-9]*)
        echo "error: jobs must be a positive integer" >&2
        exit 1
        ;;
esac
if [ "$JOBS" -lt 1 ]; then
    echo "error: jobs must be >= 1" >&2
    exit 1
fi

BIN_NAME=$1
INPUT_DIR=${2:-$ROOT_DIR/tools/in}
BIN_SRC="$ROOT_DIR/src/bin/$BIN_NAME.rs"
BIN_EXE="$ROOT_DIR/target/release/$BIN_NAME"

if [ ! -f "$BIN_SRC" ]; then
    echo "error: not found: $BIN_SRC" >&2
    exit 1
fi

if [ ! -d "$INPUT_DIR" ]; then
    echo "error: input directory not found: $INPUT_DIR" >&2
    exit 1
fi
INPUT_DIR=$(CDPATH= cd -- "$INPUT_DIR" && pwd)

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    exit 1
fi

mkdir -p "$ROOT_DIR/results/bench" "$ROOT_DIR/results/out"

INPUT_SET=$INPUT_DIR
case "$INPUT_DIR" in
    "$ROOT_DIR"/*)
        INPUT_SET=${INPUT_DIR#"$ROOT_DIR"/}
        ;;
esac
INPUT_TAG=$(printf "%s" "$INPUT_SET" | tr '/' '_' | tr ' ' '_' | tr -cd 'A-Za-z0-9_.-')
if [ -z "$INPUT_TAG" ]; then
    INPUT_TAG=input
fi

OUT_DIR=${CUSTOM_OUT_DIR:-$ROOT_DIR/results/out/${BIN_NAME}_bench}
BENCH_CSV="$ROOT_DIR/results/bench/${BIN_NAME}_${INPUT_TAG}.csv"
SUMMARY_TXT="$ROOT_DIR/results/bench/${BIN_NAME}_${INPUT_TAG}_summary.txt"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"

TMP_DIR=$(mktemp -d)
TMP_RESULTS_DIR="$TMP_DIR/results"
INPUT_LIST="$TMP_DIR/inputs.txt"
mkdir -p "$TMP_RESULTS_DIR" "$OUT_DIR"
OUT_DIR=$(CDPATH= cd -- "$OUT_DIR" && pwd)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

find "$INPUT_DIR" -maxdepth 1 -type f -name '*.txt' | sort > "$INPUT_LIST"
CASES=$(wc -l < "$INPUT_LIST" | tr -d ' ')
if [ "$CASES" -eq 0 ]; then
    echo "error: no input files found under $INPUT_DIR" >&2
    exit 1
fi

echo "building solver: $BIN_NAME" >&2
cargo build --release --quiet --manifest-path "$ROOT_MANIFEST" --bin "$BIN_NAME"
echo "building scorer" >&2
cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score
SCORE_EXE="$ROOT_DIR/tools/target/release/score"

START_TS=$(date +%s)
cat "$INPUT_LIST" | xargs -I{} -P "$JOBS" sh "$0" __worker "$BIN_EXE" "$SCORE_EXE" "{}" "$OUT_DIR" "$TMP_RESULTS_DIR"
END_TS=$(date +%s)
TOTAL_ELAPSED=$((END_TS - START_TS))

printf "input,elapsed_sec,score\n" > "$BENCH_CSV"
find "$TMP_RESULTS_DIR" -maxdepth 1 -type f -name '*.csv' | sort | while IFS= read -r path; do
    cat "$path" >> "$BENCH_CSV"
done

SUMMARY_VALUES=$(tail -n +2 "$BENCH_CSV" | awk -F, '
BEGIN {
    total = 0;
    count = 0;
    min_score = -1;
    max_score = -1;
}
{
    score = $3 + 0;
    total += score;
    count += 1;
    if (min_score < 0 || score < min_score) {
        min_score = score;
        min_input = $1;
    }
    if (max_score < 0 || score > max_score) {
        max_score = score;
        max_input = $1;
    }
}
END {
    average = 0;
    if (count > 0) {
        average = int(total / count);
    }
    printf "%d,%d,%d,%s,%d,%s\n", total, average, min_score, min_input, max_score, max_input;
}')

OLD_IFS=$IFS
IFS=,
set -- $SUMMARY_VALUES
TOTAL_SCORE=$1
AVERAGE_SCORE=$2
MIN_SCORE=$3
MIN_INPUT=$4
MAX_SCORE=$5
MAX_INPUT=$6
IFS=$OLD_IFS

UPDATED_AT=$(date '+%Y-%m-%dT%H:%M:%S%z')
EXPERIMENT_ID="${BIN_NAME}_${INPUT_TAG}_$(date '+%Y%m%d_%H%M%S')"
BENCH_CSV_REL=${BENCH_CSV#"$ROOT_DIR"/}
SUMMARY_TXT_REL=${SUMMARY_TXT#"$ROOT_DIR"/}

cat > "$SUMMARY_TXT" <<EOF
bin=$BIN_NAME
input_dir=$INPUT_SET
jobs=$JOBS
cases=$CASES
wall_time_sec=$TOTAL_ELAPSED
total_score=$TOTAL_SCORE
average_score=$AVERAGE_SCORE
min_score=$MIN_SCORE
min_input=$MIN_INPUT
max_score=$MAX_SCORE
max_input=$MAX_INPUT
bench_csv=$BENCH_CSV_REL
out_dir=${OUT_DIR#"$ROOT_DIR"/}
updated_at=$UPDATED_AT
EOF

if [ ! -f "$SUMMARY_CSV" ]; then
    printf "experiment_id,bin,input_set,cases,total_score,average_score,min_score,min_input,max_score,max_input,bench_csv,summary_txt,last_updated\n" > "$SUMMARY_CSV"
fi
printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
    "$EXPERIMENT_ID" \
    "$BIN_NAME" \
    "$INPUT_SET" \
    "$CASES" \
    "$TOTAL_SCORE" \
    "$AVERAGE_SCORE" \
    "$MIN_SCORE" \
    "$MIN_INPUT" \
    "$MAX_SCORE" \
    "$MAX_INPUT" \
    "$BENCH_CSV_REL" \
    "$SUMMARY_TXT_REL" \
    "$UPDATED_AT" >> "$SUMMARY_CSV"

echo "bench csv: $BENCH_CSV_REL" >&2
echo "summary : $SUMMARY_TXT_REL" >&2
cat "$SUMMARY_TXT"
