#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/adhoc/analyze_v004_case.sh <case_id|case_file>

Example:
  ./scripts/adhoc/analyze_v004_case.sh 0047
EOF
}

if [ "$#" -ne 1 ]; then
    usage
    exit 1
fi

CASE_INPUT=$1
case "$CASE_INPUT" in
    *.txt) CASE_FILE=$CASE_INPUT ;;
    *) CASE_FILE="$CASE_INPUT.txt" ;;
esac

INPUT_PATH="$ROOT_DIR/tools/in/$CASE_FILE"
if [ ! -f "$INPUT_PATH" ]; then
    echo "error: input not found: $INPUT_PATH" >&2
    exit 1
fi

cargo build --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin v004_safe_first --bin v004_probe_log
cargo build --release --quiet --manifest-path "$ROOT_DIR/tools/Cargo.toml" --bin score

RUN_OUT_DIR="$ROOT_DIR/results/out/v004_safe_first"
PROBE_OUT_DIR="$ROOT_DIR/results/out/v004_probe_log"
mkdir -p "$RUN_OUT_DIR" "$PROBE_OUT_DIR"

RUN_OUT="$RUN_OUT_DIR/$CASE_FILE"
RUN_ERR="$RUN_OUT.err"
PROBE_OUT="$PROBE_OUT_DIR/$CASE_FILE"
PROBE_ERR="$PROBE_OUT.err"

"$ROOT_DIR/target/release/v004_safe_first" < "$INPUT_PATH" > "$RUN_OUT" 2> "$RUN_ERR"
"$ROOT_DIR/target/release/v004_probe_log" < "$INPUT_PATH" > "$PROBE_OUT" 2> "$PROBE_ERR"

TURNS=$(wc -l < "$RUN_OUT" | tr -d ' ')
SCORE=$("$ROOT_DIR/tools/target/release/score" "$INPUT_PATH" "$RUN_OUT" | awk 'NF{last=$NF} END{print last}')

printf 'case=%s\n' "$CASE_FILE"
printf 'v004_safe_first_turns=%s\n' "$TURNS"
printf 'v004_safe_first_score=%s\n' "$SCORE"
printf '%s\n' '--- v004_probe_log (key lines) ---'
grep -E 'extend_fail_detail|probe_summary' "$PROBE_ERR" || true
