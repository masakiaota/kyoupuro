#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SOLVER_MANIFEST="$ROOT_DIR/Cargo.toml"
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
SOLVER_BIN_DIR="$ROOT_DIR/target/release"
TOOLS_BIN_DIR="$ROOT_DIR/tools/target/release"
TOOLS_SRC_BIN_DIR="$ROOT_DIR/tools/src/bin"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"
DETAIL_CSV="$ROOT_DIR/results/score_detail.csv"
REBUILD_DETAIL_SH="$ROOT_DIR/scripts/rebuild_score_detail.sh"
BIN_RESOLVER="$ROOT_DIR/scripts/lib/resolve_bin_src.py"

canonical_dir() {
    (cd -- "$1" 2>/dev/null && pwd)
}

now_iso8601() {
    case "$MS_CLOCK" in
        python3)
            python3 -c 'from datetime import datetime; print(datetime.now().astimezone().isoformat(timespec="milliseconds"))'
            ;;
        perl)
            perl -MPOSIX=strftime -MTime::HiRes=gettimeofday -e '
                ($sec, $usec) = gettimeofday();
                @t = localtime($sec);
                $offset = strftime("%z", @t);
                $offset =~ s/(..)$/:$1/;
                print strftime("%Y-%m-%dT%H:%M:%S", @t), sprintf(".%03d", int($usec / 1000)), $offset, qq(\n);
            '
            ;;
        *)
            date '+%Y-%m-%dT%H:%M:%S%z'
            ;;
    esac
}

usage() {
    cat >&2 <<'EOF_USAGE'
Usage:
  ./scripts/eval.sh <bin_name> [input_dir]
  ./scripts/eval.sh -v <bin_name> [input_dir]
  ./scripts/eval.sh --serial <bin_name> [input_dir]

Without input_dir, `tools/in` is used.
The evaluation pipeline builds once, then runs and scores cases in parallel.
EOF_USAGE
}

VERBOSE=0
SERIAL=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -s|--serial)
            SERIAL=1
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
DETAIL_ROWS_FILE="$WORK_DIR/detail_rows.txt"
DETAIL_RUN_ROW="$WORK_DIR/detail_run_row.csv"
SUMMARY_TMP="$WORK_DIR/score_summary.csv"
DETAIL_TMP="$WORK_DIR/score_detail.csv"
INPUT_COUNT=0
SUCCESS=0
FAILURE=0
TOTAL_CASES=0
EVAL_SET=all
SLOWEST_ELAPSED=-1
SLOWEST_CASE=
DETAIL_ELIGIBLE=0
DETAIL_REASON=
EXECUTED_AT=

if command -v python3 >/dev/null 2>&1; then
    MS_CLOCK=python3
elif command -v perl >/dev/null 2>&1; then
    MS_CLOCK=perl
else
    echo "error: requires python3 or perl for millisecond timing" >&2
    exit 1
fi

cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT INT TERM

if ! BIN_SRC=$("$BIN_RESOLVER" "$ROOT_DIR" "$BIN_NAME"); then
    echo "error: failed to resolve bin source: $BIN_NAME" >&2
    exit 1
fi
if [ ! -f "$BIN_SRC" ]; then
    echo "error: not found: $BIN_SRC" >&2
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
EXECUTED_AT=$(now_iso8601)

CPU_COUNT=$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)
if [ -z "$CPU_COUNT" ] && command -v sysctl >/dev/null 2>&1; then
    CPU_COUNT=$(sysctl -n hw.ncpu 2>/dev/null || true)
fi
if [ -z "$CPU_COUNT" ] || [ "$CPU_COUNT" -le 0 ]; then
    CPU_COUNT=2
fi
PARALLEL=$((CPU_COUNT / 2 - 1))
if [ "$PARALLEL" -lt 1 ]; then
    PARALLEL=1
fi
if [ "$SERIAL" -eq 1 ]; then
    PARALLEL=1
fi

if [ "$VERBOSE" -ge 1 ]; then
    printf 'eval: bin=%s input_dir=%s parallel=%s output=%s\n' "$BIN_NAME" "$INPUT_DIR" "$PARALLEL" "$OUTPUT_DIR" >&2
fi

if [ ! -f "$TOOLS_SRC_BIN_DIR/score.rs" ]; then
    echo "error: scorer source not found: $TOOLS_SRC_BIN_DIR/score.rs" >&2
    exit 1
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
    echo "error: scoring binary not found: $SCORE_BIN" >&2
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

DETAIL_ELIGIBLE=1
DETAIL_REASON=
if [ "$INPUT_DIR" != "$DEFAULT_INPUT_DIR_ABS" ]; then
    DETAIL_ELIGIBLE=0
    DETAIL_REASON="input_dir is not tools/in"
fi
if [ "$DETAIL_ELIGIBLE" -eq 1 ] && [ "$INPUT_COUNT" -ne 100 ]; then
    DETAIL_ELIGIBLE=0
    DETAIL_REASON="score_detail.csv requires exactly 100 cases"
fi
if [ "$DETAIL_ELIGIBLE" -eq 1 ]; then
    EXPECTED_INDEX=0
    while IFS= read -r BASE_NAME; do
        EXPECTED_BASE=$(printf '%04d.txt' "$EXPECTED_INDEX")
        if [ "$BASE_NAME" != "$EXPECTED_BASE" ]; then
            DETAIL_ELIGIBLE=0
            DETAIL_REASON="score_detail.csv requires basenames 0000.txt..0099.txt"
            break
        fi
        EXPECTED_INDEX=$((EXPECTED_INDEX + 1))
    done < "$BASE_LIST"
fi

mkdir -p "$OUTPUT_DIR" "$META_DIR"
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -exec rm -rf {} +

export OUTPUT_DIR META_DIR SOLVER_BIN SCORE_BIN VERBOSE MS_CLOCK

if [ "$VERBOSE" -ge 1 ]; then
    printf 'eval: start run -> score pipeline\n' >&2
fi

if tr '\n' '\0' < "$INPUT_LIST" | xargs -0 -n 1 -P "$PARALLEL" sh -c '
    set -eu

    now_ms() {
        case "$MS_CLOCK" in
            python3)
                python3 -c "import time; print(int(time.time() * 1000))"
                ;;
            perl)
                perl -MTime::HiRes=time -e "print int(time() * 1000), qq(\n)"
                ;;
            *)
                date +%s
                ;;
        esac
    }

    input_file=$1
    base=$(basename "$input_file")
    output_file="$OUTPUT_DIR/$base"
    meta_file="$META_DIR/$base.meta"
    err_file="$output_file.err"

    if [ "$VERBOSE" -ge 1 ]; then
        printf "start: %s\n" "$base" >&2
    fi

    run_start_ms=$(now_ms)
    if "$SOLVER_BIN" < "$input_file" > "$output_file" 2> "$err_file"; then
        run_status=0
    else
        run_status=$?
    fi
    run_elapsed_ms=$(( $(now_ms) - run_start_ms ))

    if [ "$run_status" -ne 0 ]; then
        {
            printf "status=run_fail\n"
            printf "score=99999999\n"
            printf "elapsed=%s\n" "$run_elapsed_ms"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(run): %s\n" "$base" >&2
        fi
        exit 0
    fi

    if score_result=$("$SCORE_BIN" "$input_file" "$output_file" 2>> "$err_file"); then
        score_status=0
    else
        score_status=$?
    fi

    if [ "$score_status" -ne 0 ]; then
        {
            printf "status=score_fail\n"
            printf "score=99999999\n"
            printf "elapsed=%s\n" "$run_elapsed_ms"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(score): %s\n" "$base" >&2
        fi
        exit 0
    fi

    score=$(printf "%s\n" "$score_result" | awk "NF { value = \$NF } END { print value }")
    if [ -z "$score" ]; then
        {
            printf "status=score_parse_fail\n"
            printf "score=99999999\n"
            printf "elapsed=%s\n" "$run_elapsed_ms"
            printf "input=%s\n" "$input_file"
            printf "output=%s\n" "$output_file"
        } > "$meta_file"
        if [ "$VERBOSE" -ge 1 ]; then
            printf "fail(parse): %s\n" "$base" >&2
        fi
        exit 0
    fi

    if [ "$score" -eq 0 ] 2>/dev/null; then
        score=99999999
        status=score_zero
    else
        status=ok
    fi

    {
        printf "status=%s\n" "$status"
        printf "score=%s\n" "$score"
        printf "elapsed=%s\n" "$run_elapsed_ms"
        printf "input=%s\n" "$input_file"
        printf "output=%s\n" "$output_file"
    } > "$meta_file"

    if [ "$VERBOSE" -ge 1 ]; then
        printf "done: %s score=%s run_elapsed=%sms output=%s\n" "$base" "$score" "$run_elapsed_ms" "$output_file" >&2
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
    INPUT_PATH=
    while IFS= read -r LINE; do
        case $LINE in
            status=*) STATUS=${LINE#status=} ;;
            score=*) SCORE=${LINE#score=} ;;
            elapsed=*) ELAPSED=${LINE#elapsed=} ;;
            input=*) INPUT_PATH=${LINE#input=} ;;
        esac
    done < "$META_FILE"

    CASE_NAME=$(basename "$META_FILE" .meta)
    if [ -n "$INPUT_PATH" ]; then
        CASE_NAME=$(basename "$INPUT_PATH")
    fi

    case "$STATUS" in
        ok)
            SUCCESS=$((SUCCESS + 1))
            ;;
        *)
            FAILURE=$((FAILURE + 1))
            ;;
    esac

    if [ -z "$SCORE" ]; then
        echo "error: missing score in meta file: $META_FILE" >&2
        exit 1
    fi
    if [ -z "$ELAPSED" ]; then
        echo "error: missing elapsed in meta file: $META_FILE" >&2
        exit 1
    fi

    printf '%s %s\n' "$SCORE" "$ELAPSED" >> "$ROWS_FILE"
    printf '%s %s %s\n' "$CASE_NAME" "$SCORE" "$ELAPSED" >> "$DETAIL_ROWS_FILE"
    if [ "$ELAPSED" -gt "$SLOWEST_ELAPSED" ]; then
        SLOWEST_ELAPSED=$ELAPSED
        SLOWEST_CASE=$CASE_NAME
    fi
done

if [ "$TOTAL_CASES" -ne "$INPUT_COUNT" ]; then
    echo "error: evaluated cases mismatch: input=$INPUT_COUNT meta=$TOTAL_CASES" >&2
    exit 1
fi

if [ "$SUCCESS" -gt 0 ]; then
    if SUMMARY=$(awk '
        function round_nearest(value) {
            if (value >= 0) {
                return int(value + 0.5)
            }
            return -int(-value + 0.5)
        }

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
            total_avg = round_nearest(total_sum / count)
            avg_elapsed = (total_elapsed / count) / 1000.0
            printf "%d %.6f %.0f %.0f %.0f %d", total_avg, avg_elapsed, total_sum, total_min, total_max, count
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

if [ "$SLOWEST_ELAPSED" -ge 0 ]; then
    printf 'eval: slowest_case=%s slowest_elapsed_ms=%s\n' "$SLOWEST_CASE" "$SLOWEST_ELAPSED" >&2
fi

if [ "$DETAIL_ELIGIBLE" -eq 1 ]; then
    MAX_ELAPSED=$(awk -v elapsed_ms="$SLOWEST_ELAPSED" 'BEGIN { printf "%.6f", elapsed_ms / 1000.0 }')
    awk -v bin="$BIN_NAME" -v total_avg="$TOTAL_AVG" -v max_elapsed="$MAX_ELAPSED" -v executed_at="$EXECUTED_AT" '
        BEGIN {
            for (i = 0; i < 100; i++) {
                key = sprintf("%04d.txt", i)
                seen[key] = 0
            }
        }
        {
            if (NF != 3) {
                printf "error: malformed detail row input: %s\n", $0 > "/dev/stderr"
                exit 1
            }
            key = $1
            if (!(key in seen)) {
                printf "error: unexpected case name for score_detail.csv: %s\n", key > "/dev/stderr"
                exit 1
            }
            if (seen[key]) {
                printf "error: duplicate case name for score_detail.csv: %s\n", key > "/dev/stderr"
                exit 1
            }
            seen[key] = 1
            score[key] = $2
        }
        END {
            printf "%s,%s,0", bin, total_avg
            for (i = 0; i < 100; i++) {
                key = sprintf("%04d.txt", i)
                if (!seen[key]) {
                    printf "error: missing case for score_detail.csv: %s\n", key > "/dev/stderr"
                    exit 1
                }
                printf ",%s", score[key]
            }
            printf ",%s,%s\n", max_elapsed, executed_at
        }
    ' "$DETAIL_ROWS_FILE" > "$DETAIL_RUN_ROW"

    if [ ! -f "$REBUILD_DETAIL_SH" ]; then
        echo "error: score detail rebuild script not found: $REBUILD_DETAIL_SH" >&2
        exit 1
    fi
    sh "$REBUILD_DETAIL_SH" "$DETAIL_CSV" "$DETAIL_RUN_ROW" "$DETAIL_TMP"
else
    printf 'eval: skip score_detail.csv update: %s\n' "$DETAIL_REASON" >&2
fi

if [ -f "$SUMMARY_CSV" ]; then
    cat "$SUMMARY_CSV" > "$SUMMARY_TMP"
else
    printf 'bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases\n' > "$SUMMARY_TMP"
fi
printf '%s,%s,%s,%s,%s,%s,%s,%s\n' "$BIN_NAME" "$TOTAL_AVG" "$AVG_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$EVAL_SET" "$INPUT_COUNT" >> "$SUMMARY_TMP"

if [ "$DETAIL_ELIGIBLE" -eq 1 ]; then
    mv "$DETAIL_TMP" "$DETAIL_CSV"
fi
mv "$SUMMARY_TMP" "$SUMMARY_CSV"

if [ "$FAILURE" -ne 0 ] || [ "$XARGS_STATUS" -ne 0 ]; then
    exit 1
fi
