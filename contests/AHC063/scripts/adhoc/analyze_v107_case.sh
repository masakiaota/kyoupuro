#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/adhoc/analyze_v107_case.sh <case_id|case_file> [analyze_v104_case.py extra args]

Examples:
  ./scripts/adhoc/analyze_v107_case.sh 0069
  ./scripts/adhoc/analyze_v107_case.sh 0069 --depth-limit 36 --cap-large 1200000
EOF
}

if [ "$#" -lt 1 ]; then
    usage
    exit 1
fi

CASE_INPUT=$1
shift || true
case "$CASE_INPUT" in
    *.txt) CASE_FILE=$CASE_INPUT ;;
    *) CASE_FILE="$CASE_INPUT.txt" ;;
esac

INPUT_PATH="$ROOT_DIR/tools/in/$CASE_FILE"
OUT_PATH="$ROOT_DIR/results/out/v107_pro_rescue_stage/$CASE_FILE"
ANALYSIS_DIR="$ROOT_DIR/results/analysis/v107_pro_rescue_stage"
mkdir -p "$ANALYSIS_DIR"
PROBE_ERR="$ANALYSIS_DIR/${CASE_FILE}.probe.err"

if [ ! -f "$INPUT_PATH" ]; then
    echo "error: input not found: $INPUT_PATH" >&2
    exit 1
fi
if [ ! -f "$OUT_PATH" ]; then
    echo "error: output not found: $OUT_PATH" >&2
    echo "hint: run ./scripts/eval.sh v107_pro_rescue_stage first" >&2
    exit 1
fi

cd "$ROOT_DIR"

"$SCRIPT_DIR/analyze_v104_case.py" "$CASE_FILE" --out-dir results/out/v107_pro_rescue_stage "$@"

cargo build --release --quiet --bin v107_probe_log
"$ROOT_DIR/target/release/v107_probe_log" < "$INPUT_PATH" > /dev/null 2> "$PROBE_ERR"

printf '%s\n' '--- v107_probe_log ---'
grep -E 'probe_stop reason=' "$PROBE_ERR" | tail -n 1 || true
grep -E 'probe_stage_end ell=' "$PROBE_ERR" | tail -n 3 || true
grep -E 'probe_stop_state ' "$PROBE_ERR" | sed -n '1,5p' || true
printf 'probe_err_path=%s\n' "$PROBE_ERR"
