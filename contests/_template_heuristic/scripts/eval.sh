#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SOLVER_MANIFEST="$ROOT_DIR/Cargo.toml"
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
SOLVER_BIN_DIR="$ROOT_DIR/target/release"
TOOLS_BIN_DIR="$ROOT_DIR/tools/target/release"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"

canonical_dir() {
    (cd -- "$1" 2>/dev/null && pwd)
}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/eval.sh <bin_name> [input_dir]
  ./scripts/eval.sh -v <bin_name> [input_dir]

Without input_dir, `tools/in` is used.
The evaluation pipeline builds once, then runs and scores cases in parallel.
EOF
}

VERBOSE=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        --)
            shift
            break
            ;;
        -*)
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

BIN_NAME=$1
INPUT_DIR=${2:-"$ROOT_DIR/tools/in"}
OUTPUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
WORK_DIR=$(mktemp -d)
INPUT_LIST="$WORK_DIR/inputs.txt"
BASE_LIST="$WORK_DIR/basenames.txt"
META_DIR="$WORK_DIR/meta"
ROWS_FILE="$WORK_DIR/rows.txt"
INPUT_COUNT=0
SUCCESS=0
FAILURE=0
TOTAL_CASES=0
EVAL_SET=all

cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT INT TERM

if [ ! -f "$ROOT_DIR/src/bin/$BIN_NAME.rs" ]; then
    echo "error: not found: $ROOT_DIR/src/bin/$BIN_NAME.rs" >&2
    exit 1
fi

INPUT_DIR_ABS=$(canonical_dir "$INPUT_DIR") || {
    echo "error: input directory not found: $INPUT_DIR" >&2
    exit 1
}
DEFAULT_INPUT_DIR_ABS=$(canonical_dir "$ROOT_DIR/tools/in")
if [ "$INPUT_DIR_ABS" = "$DEFAULT_INPUT_DIR_ABS" ]; then
    EVAL_SET="all"
else
    EVAL_SET=$(basename "$INPUT_DIR_ABS")
    if [ -z "$EVAL_SET" ] || [ "$EVAL_SET" = "." ] || [ "$EVAL_SET" = "/" ]; then
        EVAL_SET="all"
    fi
fi
INPUT_DIR="$INPUT_DIR_ABS"

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

if [ "$VERBOSE" -ge 1 ]; then
    printf 'eval: bin=%s input_dir=%s parallel=%s output=%s\n' "$BIN_NAME" "$INPUT_DIR" "$PARALLEL" "$OUTPUT_DIR" >&2
fi

cargo build --release --quiet --manifest-path "$SOLVER_MANIFEST" --bin "$BIN_NAME"
cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score

SOLVER_BIN="$SOLVER_BIN_DIR/$BIN_NAME"
SCORE_BIN="$TOOLS_BIN_DIR/score"

if [ ! -x "$SOLVER_BIN" ]; then
    echo "error: solver binary not found: $SOLVER_BIN" >&2
    exit 1
fi
if [ ! -x "$SCORE_BIN" ]; then
    echo "error: score binary not found: $SCORE_BIN" >&2
    exit 1
fi

find "$INPUT_DIR" -type f | sort > "$INPUT_LIST"
INPUT_COUNT=$(wc -l < "$INPUT_LIST" | tr -d ' ')
if [ "$INPUT_COUNT" -eq 0 ]; then
    echo "error: input directory is empty: $INPUT_DIR" >&2
    exit 1
fi

while IFS= read -r INPUT_FILE; do
    basename "$INPUT_FILE" >> "$BASE_LIST"
done < "$INPUT_LIST"

DUPLICATES="$WORK_DIR/duplicates.txt"
sort "$BASE_LIST" | uniq -d > "$DUPLICATES"
if [ -s "$DUPLICATES" ]; then
    echo "error: input directory contains duplicate basenames; results would collide" >&2
    echo "files with duplicated basename:" >&2
    cat "$DUPLICATES" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR" "$META_DIR"
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -exec rm -rf {} +

export OUTPUT_DIR META_DIR SOLVER_BIN SCORE_BIN VERBOSE

if [ "$VERBOSE" -ge 1 ]; then
    printf 'eval: start run -> score pipeline\n' >&2
fi

if tr '\n' '\0' < "$INPUT_LIST" | xargs -0 -n 1 -P "$PARALLEL" sh -c '
    set -eu

    input_file=$1
    base=$(basename "$input_file")
    output_file="$OUTPUT_DIR/$base"
    meta_file="$META_DIR/$base.meta"
    err_file="$output_file.err"

    if [ "$VERBOSE" -ge 1 ]; then
        printf "start: %s\n" "$base" >&2
    fi

    run_start=$(date +%s)
    if "$SOLVER_BIN" < "$input_file" > "$output_file" 2> "$err_file"; then
        run_status=0
    else
        run_status=$?
    fi
    run_elapsed=$(( $(date +%s) - run_start ))

    if [ "$run_status" -ne 0 ]; then
        {
            printf "status=run_fail\n"
            printf "score=\n"
            printf "elapsed=%s\n" "$run_elapsed"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(run): %s\n" "$base" >&2
        fi
        exit 0
    fi

    score_start=$(date +%s)
    if score_result=$("$SCORE_BIN" "$input_file" "$output_file" 2>> "$err_file"); then
        score_status=0
    else
        score_status=$?
    fi
    score_elapsed=$(( $(date +%s) - score_start ))

    if [ "$score_status" -ne 0 ]; then
        {
            printf "status=score_fail\n"
            printf "score=\n"
            printf "elapsed=%s\n" "$((run_elapsed + score_elapsed))"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(score): %s\n" "$base" >&2
        fi
        exit 0
    fi

    score=$(printf '%s\n' "$score_result" | awk 'NF { value = $NF } END { print value }')
    if [ -z "$score" ]; then
        {
            printf "status=score_parse_fail\n"
            printf "score=\n"
            printf "elapsed=%s\n" "$((run_elapsed + score_elapsed))"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(parse): %s\n" "$base" >&2
        fi
        exit 0
    fi

    total_elapsed=$((run_elapsed + score_elapsed))
    {
        printf "status=ok\n"
        printf "score=%s\n" "$score"
        printf "elapsed=%s\n" "$total_elapsed"
        printf "input=%s\n" "$input_file"
        printf "output=%s\n" "$output_file"
    } > "$meta_file"

    if [ "$VERBOSE" -ge 1 ]; then
        printf "done: %s score=%s elapsed=%ss output=%s\n" "$base" "$score" "$total_elapsed" "$output_file" >&2
    fi
' _
then
    XARGS_STATUS=0
else
    XARGS_STATUS=$?
fi

for META_FILE in "$META_DIR"/*.meta; do
    [ -f "$META_FILE" ] || continue
    TOTAL_CASES=$((TOTAL_CASES + 1))
    STATUS=
    SCORE=
    ELAPSED=
    while IFS= read -r LINE; do
        case $LINE in
            status=*) STATUS=${LINE#status=} ;;
            score=*) SCORE=${LINE#score=} ;;
            elapsed=*) ELAPSED=${LINE#elapsed=} ;;
        esac
    done < "$META_FILE"

    case "$STATUS" in
        ok)
            SUCCESS=$((SUCCESS + 1))
            printf '%s %s\n' "$SCORE" "$ELAPSED" >> "$ROWS_FILE"
            ;;
        *)
            FAILURE=$((FAILURE + 1))
            ;;
    esac
done

if [ "$TOTAL_CASES" -ne "$INPUT_COUNT" ]; then
    echo "error: evaluated cases mismatch: input=$INPUT_COUNT meta=$TOTAL_CASES" >&2
    exit 1
fi

if [ "$SUCCESS" -gt 0 ]; then
    if SUMMARY=$(awk '
        {
            score = $1 + 0
            elapsed = $2 + 0
            count++
            total_sum += score
            total_elapsed += elapsed
            if (count == 1 || score < total_min) {
                total_min = score
            }
            if (count == 1 || score > total_max) {
                total_max = score
            }
        }
        END {
            if (count == 0) {
                exit 1
            }
            total_avg = total_sum / count
            avg_elapsed = total_elapsed / count
            printf "%.6f %.6f %.0f %.0f %.0f %d", total_avg, avg_elapsed, total_sum, total_min, total_max, count
        }
    ' "$ROWS_FILE"); then
        :
    else
        echo "error: summary failed" >&2
        exit 1
    fi

    set -- $SUMMARY
    TOTAL_AVG=$1
    AVG_ELAPSED=$2
    TOTAL_SUM=$3
    TOTAL_MIN=$4
    TOTAL_MAX=$5
else
    TOTAL_AVG=0
    AVG_ELAPSED=0
    TOTAL_SUM=0
    TOTAL_MIN=0
    TOTAL_MAX=0
fi

printf 'eval: bin=%s eval_set=%s success=%s failure=%s total_avg=%s avg_elapsed=%s total_sum=%s total_min=%s total_max=%s total_cases=%s output=%s\n' \
    "$BIN_NAME" "$EVAL_SET" "$SUCCESS" "$FAILURE" "$TOTAL_AVG" "$AVG_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$INPUT_COUNT" "$OUTPUT_DIR" >&2

if [ "$FAILURE" -ne 0 ] || [ "$XARGS_STATUS" -ne 0 ]; then
    exit 1
fi

if [ ! -f "$SUMMARY_CSV" ]; then
    printf "bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases\n" > "$SUMMARY_CSV"
fi
printf "%s,%s,%s,%s,%s,%s,%s,%s\n" "$BIN_NAME" "$TOTAL_AVG" "$AVG_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$EVAL_SET" "$INPUT_COUNT" >> "$SUMMARY_CSV"
