#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

BASE_BIN="v013_middle_reroute_sa_fastest"
WORK_BIN="v013_sa_temp_grid_worker"
BASE_SRC="$ROOT_DIR/src/bin/$BASE_BIN.rs"
WORK_SRC="$ROOT_DIR/src/bin/$WORK_BIN.rs"

OUT_CSV="$ROOT_DIR/results/sa_temp_grid_v013.csv"
OUT_SORTED_CSV="$ROOT_DIR/results/sa_temp_grid_v013_sorted.csv"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"
SUMMARY_BACKUP=$(mktemp)

if [ ! -f "$BASE_SRC" ]; then
    echo "error: not found: $BASE_SRC" >&2
    exit 1
fi

if [ -f "$SUMMARY_CSV" ]; then
    cp "$SUMMARY_CSV" "$SUMMARY_BACKUP"
else
    : > "$SUMMARY_BACKUP"
fi

cleanup() {
    rm -f "$WORK_SRC"
    if [ -s "$SUMMARY_BACKUP" ]; then
        cp "$SUMMARY_BACKUP" "$SUMMARY_CSV"
    else
        rm -f "$SUMMARY_CSV"
    fi
    rm -f "$SUMMARY_BACKUP"
}
trap cleanup EXIT INT TERM

printf "sa_temp_start,sa_temp_end,total_avg,avg_elapsed,total_sum,total_min,total_max,success,failure,total_cases\n" > "$OUT_CSV"

TOTAL=25
IDX=0

for START in 800.0 500.0 400.0 300.0 220.0; do
    for END in 100.0 50.0 30.0 10.0 5.0; do
        IDX=$((IDX + 1))
        printf "[%02d/%02d] SA_TEMP_START=%s SA_TEMP_END=%s\n" "$IDX" "$TOTAL" "$START" "$END" >&2

        awk -v start="$START" -v end="$END" '
            /^const SA_TEMP_START: f64 = / { print "const SA_TEMP_START: f64 = " start ";"; next }
            /^const SA_TEMP_END: f64 = / { print "const SA_TEMP_END: f64 = " end ";"; next }
            { print }
        ' "$BASE_SRC" > "$WORK_SRC"

        LINE=$(./scripts/eval.sh "$WORK_BIN" 2>&1 >/dev/null | awk '/^eval: / { last = $0 } END { print last }')
        if [ -s "$SUMMARY_BACKUP" ]; then
            cp "$SUMMARY_BACKUP" "$SUMMARY_CSV"
        else
            rm -f "$SUMMARY_CSV"
        fi
        if [ -z "$LINE" ]; then
            echo "error: failed to parse eval output (start=$START end=$END)" >&2
            exit 1
        fi

        TOTAL_AVG=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="total_avg"{print $2}')
        AVG_ELAPSED=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="avg_elapsed"{print $2}')
        TOTAL_SUM=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="total_sum"{print $2}')
        TOTAL_MIN=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="total_min"{print $2}')
        TOTAL_MAX=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="total_max"{print $2}')
        SUCCESS=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="success"{print $2}')
        FAILURE=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="failure"{print $2}')
        TOTAL_CASES=$(printf '%s\n' "$LINE" | tr ' ' '\n' | awk -F= '$1=="total_cases"{print $2}')

        printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
            "$START" "$END" "$TOTAL_AVG" "$AVG_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$SUCCESS" "$FAILURE" "$TOTAL_CASES" \
            >> "$OUT_CSV"

        printf "  total_avg=%s avg_elapsed=%s\n" "$TOTAL_AVG" "$AVG_ELAPSED" >&2
    done
done

{
    head -n 1 "$OUT_CSV"
    tail -n +2 "$OUT_CSV" | sort -t, -k3,3nr
} > "$OUT_SORTED_CSV"

echo "done: $OUT_CSV" >&2
echo "done: $OUT_SORTED_CSV" >&2
