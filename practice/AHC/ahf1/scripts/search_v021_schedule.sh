#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_NAME="v021_simulated_annealing"
BIN_EXEC="$ROOT_DIR/target/release/$BIN_NAME"
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
VIS_BIN="$ROOT_DIR/tools/target/release/vis"
RESULT_CSV="$ROOT_DIR/results/search_v021_schedule.csv"

START_TEMP="50"
END_TEMPS="0.1 1 5 10"
SCHEDULES="exp linear"

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

printf "start_temp,end_temp,schedule,total_avg,total_sum,total_min,total_max,total_cases\n" > "$RESULT_CSV"

PAIR_LIST=$(mktemp)
RESULTS=$(mktemp)
trap 'rm -f "$PAIR_LIST" "$RESULTS"' EXIT

run_cases() {
    OUTPUT_DIR=$1
    END_TEMP=$2
    SCHEDULE=$3
    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR"
    : > "$PAIR_LIST"

    find "$ROOT_DIR/tools/in" -type f | sort | while IFS= read -r INPUT_FILE; do
        BASE_NAME=$(basename "$INPUT_FILE")
        printf '%s\0%s\0' "$INPUT_FILE" "$OUTPUT_DIR/$BASE_NAME" >> "$PAIR_LIST"
    done

    xargs -0 -n 2 -P "$PARALLEL" sh -c '
        bin_exec=$1
        start_temp=$2
        end_temp=$3
        schedule=$4
        input_file=$5
        output_file=$6
        V021_START_TEMP=$start_temp \
        V021_END_TEMP=$end_temp \
        V021_SCHEDULE=$schedule \
        "$bin_exec" < "$input_file" > "$output_file"
    ' _ "$BIN_EXEC" "$START_TEMP" "$END_TEMP" "$SCHEDULE" < "$PAIR_LIST"
}

score_dir() {
    OUTPUT_DIR=$1
    : > "$RESULTS"
    find "$ROOT_DIR/tools/in" -type f | sort | while IFS= read -r INPUT_FILE; do
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
        printf "%s\n" "$SCORE" >> "$RESULTS"
    done

    awk '
        {
            score = $1 + 0
            count++
            total_sum += score
            if (count == 1 || score < total_min) {
                total_min = score
            }
            if (count == 1 || score > total_max) {
                total_max = score
            }
        }
        END {
            total_avg = total_sum / count
            printf "%.6f %.0f %.0f %.0f %d\n", total_avg, total_sum, total_min, total_max, count
        }
    ' "$RESULTS"
}

BEST_SUM=-1
BEST_END_TEMP=
BEST_SCHEDULE=

for END_TEMP in $END_TEMPS; do
    for SCHEDULE in $SCHEDULES; do
        TAG="v021_sa_t${START_TEMP}_e${END_TEMP}_${SCHEDULE}"
        OUTPUT_DIR="$ROOT_DIR/results/out/$TAG"
        run_cases "$OUTPUT_DIR" "$END_TEMP" "$SCHEDULE"
        set -- $(score_dir "$OUTPUT_DIR")
        TOTAL_AVG=$1
        TOTAL_SUM=$2
        TOTAL_MIN=$3
        TOTAL_MAX=$4
        TOTAL_CASES=$5
        printf "%s,%s,%s,%s,%s,%s,%s,%s\n" \
            "$START_TEMP" "$END_TEMP" "$SCHEDULE" "$TOTAL_AVG" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$TOTAL_CASES" >> "$RESULT_CSV"
        if [ "$TOTAL_SUM" -gt "$BEST_SUM" ]; then
            BEST_SUM=$TOTAL_SUM
            BEST_END_TEMP=$END_TEMP
            BEST_SCHEDULE=$SCHEDULE
        fi
    done
done

printf "best_start_temp=%s best_end_temp=%s best_schedule=%s total_sum=%s\n" \
    "$START_TEMP" "$BEST_END_TEMP" "$BEST_SCHEDULE" "$BEST_SUM"
