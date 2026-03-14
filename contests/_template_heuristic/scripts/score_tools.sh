#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
SCORE_BIN="$ROOT_DIR/tools/target/release/score"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/score_tools.sh
  ./scripts/score_tools.sh <bin_name>
  ./scripts/score_tools.sh <input_file> <output_file>
  ./scripts/score_tools.sh <input_dir> <output_dir>
  ./scripts/score_tools.sh <bin_name> <input_file> <output_file>
  ./scripts/score_tools.sh <bin_name> <input_dir> <output_dir>

Without bin_name, default is `unknown`.

No args:
  tools/in と tools/out の対応ペアを自動で評価する（eval_set=all）
1 pair:
  1 つの input/output ファイルを評価する（eval_set=single）
2 args:
  2 つのディレクトリを評価する

Parallel:
  評価は CPU 数の半分（最低 1）で実行する。
EOF
}

if [ "$#" -eq 3 ]; then
    BIN_NAME=$1
    shift
fi

if [ "$#" -ne 0 ] && [ "$#" -ne 2 ]; then
    usage
    exit 1
fi

if [ -z "${BIN_NAME-}" ]; then
    BIN_NAME="unknown"
fi

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: official tools を tools/ に展開してから実行する" >&2
    exit 1
fi

if [ "$#" -eq 0 ]; then
    INPUT_DIR="$ROOT_DIR/tools/in"
    OUTPUT_DIR="$ROOT_DIR/tools/out"
    EVAL_SET="all"
elif [ -f "$1" ] && [ -f "$2" ]; then
    INPUT_FILE="$1"
    OUTPUT_FILE="$2"
    SINGLE_PAIR=1
    EVAL_SET="single"
else
    INPUT_DIR="$1"
    OUTPUT_DIR="$2"
    if [ ! -d "$INPUT_DIR" ] || [ ! -d "$OUTPUT_DIR" ]; then
        usage
        exit 1
    fi
    EVAL_SET=$(basename "$INPUT_DIR")
    if [ -z "$EVAL_SET" ] || [ "$EVAL_SET" = "in" ] || [ "$EVAL_SET" = "." ]; then
        EVAL_SET="all"
    fi
fi

if [ ! -d "$ROOT_DIR/tools" ]; then
    echo "error: tools directory not found: $ROOT_DIR/tools" >&2
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

cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score
if [ ! -x "$SCORE_BIN" ]; then
    echo "error: score binary not found: $SCORE_BIN" >&2
    exit 1
fi

PAIR_LIST=$(mktemp)
PAIR_COUNT=0
trap 'rm -f "$PAIR_LIST"' EXIT

if [ "${SINGLE_PAIR-0}" -eq 1 ]; then
    if [ ! -f "$OUTPUT_FILE" ]; then
        echo "error: output file not found: $OUTPUT_FILE" >&2
        exit 1
    fi
    printf '%s\0%s\0' "$INPUT_FILE" "$OUTPUT_FILE" >> "$PAIR_LIST"
    PAIR_COUNT=1
else
    if [ ! -d "$INPUT_DIR" ]; then
        echo "error: input directory not found: $INPUT_DIR" >&2
        exit 1
    fi
    if [ ! -d "$OUTPUT_DIR" ]; then
        echo "error: output directory not found: $OUTPUT_DIR" >&2
        exit 1
    fi

    for INPUT_FILE in $(find "$INPUT_DIR" -type f | sort); do
        BASE_NAME=$(basename "$INPUT_FILE")
        OUTPUT_CANDIDATE="$OUTPUT_DIR/$BASE_NAME"
        if [ ! -f "$OUTPUT_CANDIDATE" ]; then
            OUTPUT_CANDIDATE=$(find "$OUTPUT_DIR" -type f -name "$BASE_NAME" | head -n 1 || true)
        fi
        if [ ! -f "$OUTPUT_CANDIDATE" ]; then
            echo "warning: output not found for $INPUT_FILE (skip)" >&2
            continue
        fi
        printf '%s\0%s\0' "$INPUT_FILE" "$OUTPUT_CANDIDATE" >> "$PAIR_LIST"
        PAIR_COUNT=$((PAIR_COUNT + 1))
    done
fi

if [ "$PAIR_COUNT" -eq 0 ]; then
    echo "error: no input/output pairs found" >&2
    exit 1
fi

RESULTS=$(mktemp)
trap 'rm -f "$PAIR_LIST" "$RESULTS"' EXIT

printf "scoring pairs=%s parallel=%s eval_set=%s\n" "$PAIR_COUNT" "$PARALLEL" "$EVAL_SET" >&2

xargs -0 -n 2 -P "$PARALLEL" sh -c '
    score_bin=$1
    input_file=$2
    output_file=$3
    start=$(date +%s)
    result=$("$score_bin" "$input_file" "$output_file")
    elapsed=$(( $(date +%s) - start ))
    score=$(printf "%s\n" "$result" | awk "NF { print \$NF }" | head -n 1)
    if [ -z "$score" ]; then
        echo "[error] failed scoring $(basename "$input_file")" >&2
        exit 1
    fi
    echo "$score $elapsed"
' _ "$SCORE_BIN" < "$PAIR_LIST" > "$RESULTS"

STATUS=$?
if [ "$STATUS" -ne 0 ]; then
    echo "error: scoring failed" >&2
    exit "$STATUS"
fi

SUMMARY=$(awk '
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
' "$RESULTS")

if [ "$?" -ne 0 ]; then
    echo "error: summary failed" >&2
    exit 1
fi

set -- $SUMMARY
TOTAL_AVG=$1
AVG_ELAPSED=$2
TOTAL_SUM=$3
TOTAL_MIN=$4
TOTAL_MAX=$5
TOTAL_CASES=$6

if [ ! -f "$SUMMARY_CSV" ]; then
    printf "bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases\n" > "$SUMMARY_CSV"
fi

printf "%s,%s,%s,%s,%s,%s,%s,%s\n" "$BIN_NAME" "$TOTAL_AVG" "$AVG_ELAPSED" "$TOTAL_SUM" "$TOTAL_MIN" "$TOTAL_MAX" "$EVAL_SET" "$TOTAL_CASES" >> "$SUMMARY_CSV"
