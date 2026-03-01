#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SCORES_CSV="$ROOT_DIR/results/scores.csv"
SUMMARY_CSV="$ROOT_DIR/results/summary.csv"

usage() {
    cat >&2 <<'EOF'
Usage: ./scripts/eval_stride4_25.sh <version> <a_bin> <b_bin> <c_bin>

Optional environment variables:
  A_SEARCH_TIME_MS
  B_SEARCH_TIME_MS
  C_SEARCH_TIME_MS
EOF
}

if [ "$#" -ne 4 ]; then
    usage
    exit 1
fi

VERSION=$1
A_BIN=$2
B_BIN=$3
C_BIN=$4

for bin_name in "$A_BIN" "$B_BIN" "$C_BIN"; do
    if [ ! -f "$ROOT_DIR/src/bin/$bin_name.rs" ]; then
        echo "error: not found: $ROOT_DIR/src/bin/$bin_name.rs" >&2
        exit 1
    fi
done

mkdir -p "$ROOT_DIR/results" "$ROOT_DIR/out_eval/$VERSION"

if [ ! -f "$SCORES_CSV" ]; then
    printf "timestamp,bin,input,elapsed_sec,score\n" > "$SCORES_CSV"
fi
if [ ! -f "$SUMMARY_CSV" ]; then
    printf "eval_set,version,a_bin,a_cases,a_avg,a_min,a_max,b_bin,b_cases,b_avg,b_min,b_max,c_bin,c_cases,c_avg,c_min,c_max,total_cases,total_sum,total_avg\n" > "$SUMMARY_CSV"
fi

cargo build --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$A_BIN" --bin "$B_BIN" --bin "$C_BIN"
cargo build --release --quiet --manifest-path "$ROOT_DIR/tools/Cargo.toml" --bin score
SCORE_BIN="$ROOT_DIR/tools/target/release/score"
if [ ! -x "$SCORE_BIN" ]; then
    echo "error: score binary not found: $SCORE_BIN" >&2
    exit 1
fi

run_problem() {
    PROB=$1
    BIN_NAME=$2
    SEARCH_MS=$3

    CASES=0
    SUM=0
    MIN=999999999999
    MAX=0

    seed=0
    while [ "$seed" -le 96 ]; do
        id=$(printf "%04d" "$seed")
        in_path="$ROOT_DIR/tools/in$PROB/$id.txt"
        out_path="$ROOT_DIR/out_eval/$VERSION/${PROB}_${id}.txt"

        start=$(date +%s)
        if [ -n "$SEARCH_MS" ]; then
            SEARCH_TIME_MS=$SEARCH_MS "$ROOT_DIR/target/release/$BIN_NAME" < "$in_path" > "$out_path"
        else
            "$ROOT_DIR/target/release/$BIN_NAME" < "$in_path" > "$out_path"
        fi
        end=$(date +%s)
        elapsed=$((end - start))

        score_line=$("$SCORE_BIN" "$in_path" "$out_path")
        score=$(printf "%s\n" "$score_line" | awk '{print $3}')

        ts=$(date '+%Y-%m-%dT%H:%M:%S%z')
        printf "%s,%s,tools/in%s/%s.txt,%s,%s\n" "$ts" "$BIN_NAME" "$PROB" "$id" "$elapsed" "$score" >> "$SCORES_CSV"

        CASES=$((CASES + 1))
        SUM=$((SUM + score))
        if [ "$score" -lt "$MIN" ]; then
            MIN=$score
        fi
        if [ "$score" -gt "$MAX" ]; then
            MAX=$score
        fi

        seed=$((seed + 4))
    done

    AVG=$(awk -v s="$SUM" -v c="$CASES" 'BEGIN { printf "%.2f", s / c }')

    RESULT_CASES=$CASES
    RESULT_SUM=$SUM
    RESULT_MIN=$MIN
    RESULT_MAX=$MAX
    RESULT_AVG=$AVG
}

run_problem A "$A_BIN" "${A_SEARCH_TIME_MS:-}"
A_CASES=$RESULT_CASES
A_SUM=$RESULT_SUM
A_MIN=$RESULT_MIN
A_MAX=$RESULT_MAX
A_AVG=$RESULT_AVG

run_problem B "$B_BIN" "${B_SEARCH_TIME_MS:-}"
B_CASES=$RESULT_CASES
B_SUM=$RESULT_SUM
B_MIN=$RESULT_MIN
B_MAX=$RESULT_MAX
B_AVG=$RESULT_AVG

run_problem C "$C_BIN" "${C_SEARCH_TIME_MS:-}"
C_CASES=$RESULT_CASES
C_SUM=$RESULT_SUM
C_MIN=$RESULT_MIN
C_MAX=$RESULT_MAX
C_AVG=$RESULT_AVG

TOTAL_CASES=$((A_CASES + B_CASES + C_CASES))
TOTAL_SUM=$((A_SUM + B_SUM + C_SUM))
TOTAL_AVG=$(awk -v s="$TOTAL_SUM" -v c="$TOTAL_CASES" 'BEGIN { printf "%.2f", s / c }')

printf "stride4_25,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
    "$VERSION" \
    "$A_BIN" "$A_CASES" "$A_AVG" "$A_MIN" "$A_MAX" \
    "$B_BIN" "$B_CASES" "$B_AVG" "$B_MIN" "$B_MAX" \
    "$C_BIN" "$C_CASES" "$C_AVG" "$C_MIN" "$C_MAX" \
    "$TOTAL_CASES" "$TOTAL_SUM" "$TOTAL_AVG" >> "$SUMMARY_CSV"

echo "done: $VERSION"
echo "A avg=$A_AVG sum=$A_SUM"
echo "B avg=$B_AVG sum=$B_SUM"
echo "C avg=$C_AVG sum=$C_SUM"
echo "TOTAL avg=$TOTAL_AVG sum=$TOTAL_SUM"
