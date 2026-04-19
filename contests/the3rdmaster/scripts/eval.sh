#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SOLVER_MANIFEST="$ROOT_DIR/Cargo.toml"
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
SOLVER_BIN_DIR="$ROOT_DIR/target/release"
TOOLS_BIN_DIR="$ROOT_DIR/tools/target/release"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"
DEFAULT_EVAL_SETS="in inB"
BIN_OUTPUT_ROOT=

canonical_dir() {
    (cd -- "$1" 2>/dev/null && pwd)
}

now_ms() {
    if command -v perl >/dev/null 2>&1; then
        perl -MTime::HiRes=time -e "print int(time() * 1000)" 2>/dev/null && return 0
    fi
    expr "$(date +%s)" \* 1000
}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/eval.sh <bin_name> [input_dir]
  ./scripts/eval.sh -v <bin_name> [input_dir]
  ./scripts/eval.sh -j <jobs> <bin_name> [input_dir]
  ./scripts/eval.sh --serial <bin_name> [input_dir]

Without input_dir, all existing default datasets are evaluated in order:
  tools/in, tools/inB

The evaluation pipeline builds once, then runs and scores each dataset/case.
Outputs are stored under `results/out/<bin_name>/<eval_set>/`.
By default, jobs=`cpu//2`. Use `-j 1` or `--serial` for strict serial evaluation.
EOF
}

VERBOSE=0
JOBS=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -j|--jobs)
            if [ "$#" -lt 2 ]; then
                usage
                exit 1
            fi
            JOBS=$2
            shift 2
            ;;
        --jobs=*)
            JOBS=${1#--jobs=}
            shift
            ;;
        --serial)
            JOBS=1
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
INPUT_DIR_ARG=${2:-}
BIN_OUTPUT_ROOT="$ROOT_DIR/results/out/$BIN_NAME"
WORK_ROOT=$(mktemp -d)
TARGETS_FILE="$WORK_ROOT/targets.txt"

cleanup() {
    rm -rf "$WORK_ROOT"
}
trap cleanup EXIT INT TERM

if [ ! -f "$ROOT_DIR/src/bin/$BIN_NAME.rs" ]; then
    echo "error: not found: $ROOT_DIR/src/bin/$BIN_NAME.rs" >&2
    exit 1
fi

if [ -n "$INPUT_DIR_ARG" ]; then
    printf '%s\n' "$INPUT_DIR_ARG" > "$TARGETS_FILE"
else
    for DATASET in $DEFAULT_EVAL_SETS; do
        INPUT_PATH="$ROOT_DIR/tools/$DATASET"
        if [ -d "$INPUT_PATH" ]; then
            printf '%s\n' "$INPUT_PATH" >> "$TARGETS_FILE"
        fi
    done
fi

if [ ! -s "$TARGETS_FILE" ]; then
    echo "error: no evaluation dataset found" >&2
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
if [ -n "$JOBS" ]; then
    case "$JOBS" in
        ''|*[!0-9]*)
            echo "error: jobs must be a positive integer: $JOBS" >&2
            exit 1
            ;;
        0)
            echo "error: jobs must be >= 1: $JOBS" >&2
            exit 1
            ;;
    esac
    PARALLEL=$JOBS
fi

if [ "$VERBOSE" -ge 1 ]; then
    printf 'eval: bin=%s parallel=%s output_root=%s\n' "$BIN_NAME" "$PARALLEL" "$BIN_OUTPUT_ROOT" >&2
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

mkdir -p "$BIN_OUTPUT_ROOT"

append_summary_csv() {
    if [ ! -f "$SUMMARY_CSV" ]; then
        printf "bin,total_avg,avg_elapsed,max_elapsed,total_sum,total_min,total_max,eval_set,total_cases\n" > "$SUMMARY_CSV"
    fi
    printf "%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
        "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9" >> "$SUMMARY_CSV"
}

evaluate_dataset() {
    INPUT_DIR_RAW=$1
    DATASET_WORK_DIR=$2
    INPUT_LIST="$DATASET_WORK_DIR/inputs.txt"
    BASE_LIST="$DATASET_WORK_DIR/basenames.txt"
    DUPLICATES="$DATASET_WORK_DIR/duplicates.txt"
    META_DIR="$DATASET_WORK_DIR/meta"
    ROWS_FILE="$DATASET_WORK_DIR/rows.txt"

    rm -rf "$DATASET_WORK_DIR"
    mkdir -p "$META_DIR"

    INPUT_DIR_ABS=$(canonical_dir "$INPUT_DIR_RAW") || {
        echo "error: input directory not found: $INPUT_DIR_RAW" >&2
        return 1
    }
    EVAL_SET=$(basename "$INPUT_DIR_ABS")
    if [ -z "$EVAL_SET" ] || [ "$EVAL_SET" = "." ] || [ "$EVAL_SET" = "/" ]; then
        echo "error: invalid eval_set derived from input_dir: $INPUT_DIR_ABS" >&2
        return 1
    fi

    OUTPUT_DIR="$BIN_OUTPUT_ROOT/$EVAL_SET"

    find "$INPUT_DIR_ABS" -type f | sort > "$INPUT_LIST"
    INPUT_COUNT=$(wc -l < "$INPUT_LIST" | tr -d ' ')
    if [ "$INPUT_COUNT" -eq 0 ]; then
        echo "error: input directory is empty: $INPUT_DIR_ABS" >&2
        return 1
    fi

    : > "$BASE_LIST"
    while IFS= read -r INPUT_FILE; do
        basename "$INPUT_FILE" >> "$BASE_LIST"
    done < "$INPUT_LIST"

    sort "$BASE_LIST" | uniq -d > "$DUPLICATES"
    if [ -s "$DUPLICATES" ]; then
        echo "error: input directory contains duplicate basenames; results would collide" >&2
        echo "files with duplicated basename:" >&2
        cat "$DUPLICATES" >&2
        return 1
    fi

    mkdir -p "$OUTPUT_DIR"
    find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -exec rm -rf {} +

    export OUTPUT_DIR META_DIR SOLVER_BIN SCORE_BIN VERBOSE

    if [ "$VERBOSE" -ge 1 ]; then
        printf 'eval: start run -> score pipeline eval_set=%s input_dir=%s output=%s\n' "$EVAL_SET" "$INPUT_DIR_ABS" "$OUTPUT_DIR" >&2
    fi

    if tr '\n' '\0' < "$INPUT_LIST" | xargs -0 -n 1 -P "$PARALLEL" sh -c '
        set -eu

        now_ms() {
            if command -v perl >/dev/null 2>&1; then
                perl -MTime::HiRes=time -e "print int(time() * 1000)" 2>/dev/null && return 0
            fi
            expr "$(date +%s)" \* 1000
        }

        append_score_stdout() {
            if [ -n "$1" ]; then
                {
                    printf "%s\n" "[score stdout]"
                    printf "%s\n" "$1"
                } >> "$2"
            fi
        }

        input_file=$1
        base=$(basename "$input_file")
        output_file="$OUTPUT_DIR/$base"
        meta_file="$META_DIR/$base.meta"
        err_file="$output_file.err"

        if [ "$VERBOSE" -ge 1 ]; then
            printf "start: %s\n" "$base" >&2
        fi

        run_start=$(now_ms)
        if "$SOLVER_BIN" < "$input_file" > "$output_file" 2> "$err_file"; then
            run_status=0
        else
            run_status=$?
        fi
        run_elapsed=$(( $(now_ms) - run_start ))

        if [ "$run_status" -ne 0 ]; then
            {
                printf "status=run_fail\n"
                printf "score=\n"
                printf "elapsed=%s\n" "$run_elapsed"
                printf "input=%s\n" "$input_file"
                printf "output=%s\n" "$output_file"
            } > "$meta_file"
            if [ "$VERBOSE" -ge 1 ]; then
                printf "fail(run): %s elapsed=%sms\n" "$base" "$run_elapsed" >&2
            fi
            exit 0
        fi

        score_start=$(now_ms)
        if score_stdout=$("$SCORE_BIN" "$input_file" "$output_file" 2>> "$err_file"); then
            score_status=0
        else
            score_status=$?
        fi
        score_elapsed=$(( $(now_ms) - score_start ))
        total_elapsed=$((run_elapsed + score_elapsed))

        if [ "$score_status" -ne 0 ]; then
            append_score_stdout "$score_stdout" "$err_file"
            {
                printf "status=score_fail\n"
                printf "score=\n"
                printf "elapsed=%s\n" "$total_elapsed"
                printf "input=%s\n" "$input_file"
                printf "output=%s\n" "$output_file"
            } > "$meta_file"
            if [ "$VERBOSE" -ge 1 ]; then
                printf "fail(score): %s elapsed=%sms\n" "$base" "$total_elapsed" >&2
            fi
            exit 0
        fi

        score_line_count=$(printf "%s\n" "$score_stdout" | awk "NF { count++ } END { print count + 0 }")
        last_score_line=$(printf "%s\n" "$score_stdout" | awk "NF { last = \$0 } END { print last }")
        score=$(printf "%s\n" "$last_score_line" | sed -n "s/^Score = \\([0-9][0-9]*\\)$/\\1/p")

        if [ "$score_line_count" -eq 0 ] || [ -z "$score" ]; then
            append_score_stdout "$score_stdout" "$err_file"
            {
                printf "status=score_parse_fail\n"
                printf "score=\n"
                printf "elapsed=%s\n" "$total_elapsed"
                printf "input=%s\n" "$input_file"
                printf "output=%s\n" "$output_file"
            } > "$meta_file"
            if [ "$VERBOSE" -ge 1 ]; then
                printf "fail(parse): %s elapsed=%sms\n" "$base" "$total_elapsed" >&2
            fi
            exit 0
        fi

        if [ "$score_line_count" -gt 1 ]; then
            append_score_stdout "$score_stdout" "$err_file"
            {
                printf "status=score_invalid\n"
                printf "score=\n"
                printf "elapsed=%s\n" "$total_elapsed"
                printf "input=%s\n" "$input_file"
                printf "output=%s\n" "$output_file"
            } > "$meta_file"
            if [ "$VERBOSE" -ge 1 ]; then
                printf "fail(invalid): %s elapsed=%sms\n" "$base" "$total_elapsed" >&2
            fi
            exit 0
        fi

        {
            printf "status=ok\n"
            printf "score=%s\n" "$score"
            printf "elapsed=%s\n" "$total_elapsed"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"

        if [ "$VERBOSE" -ge 1 ]; then
            printf "done: %s score=%s elapsed=%sms output=%s\n" "$base" "$score" "$total_elapsed" "$output_file" >&2
        fi
    ' _
    then
        XARGS_STATUS=0
    else
        XARGS_STATUS=$?
    fi

    SUCCESS=0
    FAILURE=0
    TOTAL_CASES=0
    : > "$ROWS_FILE"
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
        return 1
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
                if (count == 1 || elapsed > max_elapsed) {
                    max_elapsed = elapsed
                }
            }
            END {
                if (count == 0) {
                    exit 1
                }
                total_avg = total_sum / count
                avg_elapsed = total_elapsed / count
                printf "%.0f %.0f %.0f %.0f %.0f %.0f %d", total_avg, avg_elapsed, max_elapsed, total_sum, total_min, total_max, count
            }
        ' "$ROWS_FILE"); then
            :
        else
            echo "error: summary failed" >&2
            return 1
        fi

        set -- $SUMMARY
        TOTAL_AVG=$1
        AVG_ELAPSED=$2
        MAX_ELAPSED=$3
        TOTAL_SUM=$4
        TOTAL_MIN=$5
        TOTAL_MAX=$6
    else
        TOTAL_AVG=0
        AVG_ELAPSED=0
        MAX_ELAPSED=0
        TOTAL_SUM=0
        TOTAL_MIN=0
        TOTAL_MAX=0
    fi

    printf 'eval: bin=%s eval_set=%s success=%s failure=%s total_avg=%s avg_elapsed=%sms max_elapsed=%sms total_sum=%s total_min=%s total_max=%s total_cases=%s output=%s\n' \
        "$BIN_NAME" "$EVAL_SET" "$SUCCESS" "$FAILURE" "$TOTAL_AVG" "$AVG_ELAPSED" "$MAX_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$INPUT_COUNT" "$OUTPUT_DIR" >&2

    if [ "$FAILURE" -ne 0 ] || [ "$XARGS_STATUS" -ne 0 ]; then
        return 1
    fi

    append_summary_csv "$BIN_NAME" "$TOTAL_AVG" "$AVG_ELAPSED" "$MAX_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$EVAL_SET" "$INPUT_COUNT"
    return 0
}

OVERALL_STATUS=0
INDEX=0
while IFS= read -r TARGET_INPUT_DIR; do
    [ -n "$TARGET_INPUT_DIR" ] || continue
    INDEX=$((INDEX + 1))
    if ! evaluate_dataset "$TARGET_INPUT_DIR" "$WORK_ROOT/set_$INDEX"; then
        OVERALL_STATUS=1
    fi
done < "$TARGETS_FILE"

exit "$OVERALL_STATUS"
