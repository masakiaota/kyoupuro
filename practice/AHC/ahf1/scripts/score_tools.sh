#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_MANIFEST="$ROOT_DIR/tools/Cargo.toml"
SCORE_BIN="$ROOT_DIR/tools/target/release/score"
VIS_BIN="$ROOT_DIR/tools/target/release/vis"
SUMMARY_CSV="$ROOT_DIR/results/score_summary.csv"

abs_file_path() {
    FILE_PATH=$1
    FILE_DIR=$(dirname "$FILE_PATH")
    FILE_NAME=$(basename "$FILE_PATH")
    FILE_DIR_ABS=$(CDPATH= cd -- "$FILE_DIR" && pwd)
    printf "%s/%s\n" "$FILE_DIR_ABS" "$FILE_NAME"
}

abs_dir_path() {
    DIR_PATH=$1
    DIR_ABS=$(CDPATH= cd -- "$DIR_PATH" && pwd)
    printf "%s\n" "$DIR_ABS"
}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/score_tools.sh
  ./scripts/score_tools.sh <input_file> <output_file>
  ./scripts/score_tools.sh <input_dir> <output_dir>
  ./scripts/score_tools.sh <bin_name>
  ./scripts/score_tools.sh <bin_name> <input_file> <output_file>
  ./scripts/score_tools.sh <bin_name> <input_dir> <output_dir>

Without bin_name, default is `unknown`.

No args:
  tools/in と tools/out の対応ペアを自動で評価する（eval_set=all）
1 arg:
  bin 名を指定し、tools/in と results/out/<bin_name> の対応ペアを評価する（eval_set=results_out/<bin_name>）
1 pair:
  1 つの input/output ファイルを評価する（eval_set=single）
2 args:
  2 つのディレクトリを評価する
3 args:
  1 つ目を bin 名として扱い、input/output を評価する

Parallel:
  評価は CPU 数の半分（最低 1）で実行する。

Evaluator:
  tools 側に score があれば score を使う。
  score が無く vis だけある場合は vis の "Score = ..." を使って評価する。
EOF
}

if [ "$#" -eq 0 ]; then
    BIN_NAME="unknown"
    INPUT_DIR="$ROOT_DIR/tools/in"
    OUTPUT_DIR="$ROOT_DIR/tools/out"
    EVAL_SET="all"
elif [ "$#" -eq 1 ]; then
    BIN_NAME="$1"
    INPUT_DIR="$ROOT_DIR/tools/in"
    OUTPUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
    EVAL_SET="results_out/$BIN_NAME"
elif [ "$#" -eq 2 ]; then
    if [ -f "$1" ] && [ -f "$2" ]; then
        INPUT_FILE="$1"
        OUTPUT_FILE="$2"
        SINGLE_PAIR=1
        BIN_NAME="unknown"
        EVAL_SET="single"
    elif [ -d "$1" ] && [ -d "$2" ]; then
        INPUT_DIR="$1"
        OUTPUT_DIR="$2"
        EVAL_SET=$(basename "$INPUT_DIR")
        if [ -z "$EVAL_SET" ] || [ "$EVAL_SET" = "in" ] || [ "$EVAL_SET" = "." ]; then
            EVAL_SET="all"
        fi
        BIN_NAME="unknown"
    else
        usage
        exit 1
    fi
elif [ "$#" -eq 3 ]; then
    BIN_NAME=$1
    shift
    if [ -f "$1" ] && [ -f "$2" ]; then
        INPUT_FILE="$1"
        OUTPUT_FILE="$2"
        SINGLE_PAIR=1
        EVAL_SET="single"
    elif [ -d "$1" ] && [ -d "$2" ]; then
        INPUT_DIR="$1"
        OUTPUT_DIR="$2"
        EVAL_SET=$(basename "$INPUT_DIR")
        if [ -z "$EVAL_SET" ] || [ "$EVAL_SET" = "in" ] || [ "$EVAL_SET" = "." ]; then
            EVAL_SET="all"
        fi
    else
        usage
        exit 1
    fi
else
    usage
    exit 1
fi

if [ ! -f "$TOOLS_MANIFEST" ]; then
    echo "error: tools manifest not found: $TOOLS_MANIFEST" >&2
    echo "hint: official tools を tools/ に展開してから実行する" >&2
    exit 1
fi

if [ ! -d "$ROOT_DIR/tools" ]; then
    echo "error: tools directory not found: $ROOT_DIR/tools" >&2
    exit 1
fi

EVALUATOR_MODE=
EVALUATOR_BIN=
if cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin score >/dev/null 2>&1; then
    EVALUATOR_MODE="score"
    EVALUATOR_BIN="$SCORE_BIN"
elif cargo build --release --quiet --manifest-path "$TOOLS_MANIFEST" --bin vis >/dev/null 2>&1; then
    EVALUATOR_MODE="vis"
    EVALUATOR_BIN="$VIS_BIN"
else
    echo "error: neither 'score' nor 'vis' bin target exists in tools" >&2
    exit 1
fi

if [ ! -x "$EVALUATOR_BIN" ]; then
    echo "error: evaluator binary not found: $EVALUATOR_BIN" >&2
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

PAIR_LIST=$(mktemp)
PAIR_COUNT=0
trap 'rm -f "$PAIR_LIST"' EXIT

if [ "${SINGLE_PAIR-0}" -eq 1 ]; then
    if [ ! -f "$OUTPUT_FILE" ]; then
        echo "error: output file not found: $OUTPUT_FILE" >&2
        exit 1
    fi
    INPUT_FILE=$(abs_file_path "$INPUT_FILE")
    OUTPUT_FILE=$(abs_file_path "$OUTPUT_FILE")
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

    INPUT_DIR=$(abs_dir_path "$INPUT_DIR")
    OUTPUT_DIR=$(abs_dir_path "$OUTPUT_DIR")

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

printf "scoring pairs=%s parallel=%s eval_set=%s evaluator=%s\n" "$PAIR_COUNT" "$PARALLEL" "$EVAL_SET" "$EVALUATOR_MODE" >&2

if [ "$EVALUATOR_MODE" = "score" ]; then
    xargs -0 -n 2 -P "$PARALLEL" sh -c '
        score_bin=$1
        input_file=$2
        output_file=$3
        start=$(date +%s)
        if result=$("$score_bin" "$input_file" "$output_file" 2>&1); then
            :
        else
            status=$?
            echo "[error] scoring failed: $(basename "$input_file")" >&2
            printf "%s\n" "$result" >&2
            exit "$status"
        fi
        elapsed=$(( $(date +%s) - start ))
        score=$(printf "%s\n" "$result" | awk "
            {
                for (i = 1; i <= NF; i++) {
                    if (\$i ~ /^-?[0-9]+([.][0-9]+)?$/) {
                        last = \$i
                    }
                }
            }
            END {
                if (last != \"\") {
                    print last
                }
            }
        ")
        if [ -z "$score" ]; then
            echo "[error] failed parsing score for $(basename "$input_file")" >&2
            printf "%s\n" "$result" >&2
            exit 1
        fi
        echo "$score $elapsed"
    ' _ "$EVALUATOR_BIN" < "$PAIR_LIST" > "$RESULTS"
else
    xargs -0 -n 2 -P "$PARALLEL" sh -c '
        vis_bin=$1
        input_file=$2
        output_file=$3
        start=$(date +%s)
        tmp_dir=$(mktemp -d)
        if result=$(cd "$tmp_dir" && "$vis_bin" "$input_file" "$output_file" 2>&1); then
            :
        else
            status=$?
            rm -rf "$tmp_dir"
            echo "[error] visualization failed: $(basename "$input_file")" >&2
            printf "%s\n" "$result" >&2
            exit "$status"
        fi
        rm -rf "$tmp_dir"
        elapsed=$(( $(date +%s) - start ))
        score=$(printf "%s\n" "$result" | awk "/^Score = / { print \$3; exit }")
        if [ -z "$score" ]; then
            echo "[error] failed parsing score from vis output: $(basename "$input_file")" >&2
            printf "%s\n" "$result" >&2
            exit 1
        fi
        echo "$score $elapsed"
    ' _ "$EVALUATOR_BIN" < "$PAIR_LIST" > "$RESULTS"
fi

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
