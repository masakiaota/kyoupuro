#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BIN_RESOLVER="$ROOT_DIR/scripts/lib/resolve_bin_src.py"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/adhoc/investigate_case.sh <case_id|case_file> [bin_name]

Examples:
  ./scripts/adhoc/investigate_case.sh 0056
  ./scripts/adhoc/investigate_case.sh 0056.txt v002_probe_log
EOF
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    usage
    exit 1
fi

CASE_INPUT=$1
BIN_NAME=${2:-v002_probe_log}

case "$CASE_INPUT" in
    *.txt) CASE_FILE=$CASE_INPUT ;;
    *) CASE_FILE="$CASE_INPUT.txt" ;;
esac

INPUT_PATH="$ROOT_DIR/tools/in/$CASE_FILE"
if [ ! -f "$INPUT_PATH" ]; then
    echo "error: input not found: $INPUT_PATH" >&2
    exit 1
fi

if ! SRC_PATH=$("$BIN_RESOLVER" "$ROOT_DIR" "$BIN_NAME"); then
    echo "error: failed to resolve solver source: $BIN_NAME" >&2
    exit 1
fi
if [ ! -f "$SRC_PATH" ]; then
    echo "error: solver source not found: $SRC_PATH" >&2
    exit 1
fi

cargo build --release --quiet --manifest-path "$ROOT_DIR/Cargo.toml" --bin "$BIN_NAME"

SOLVER_BIN="$ROOT_DIR/target/release/$BIN_NAME"
OUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
mkdir -p "$OUT_DIR"

OUT_PATH="$OUT_DIR/$CASE_FILE"
ERR_PATH="$OUT_PATH.err"

"$SOLVER_BIN" < "$INPUT_PATH" > "$OUT_PATH" 2> "$ERR_PATH"

TURNS=$(wc -l < "$OUT_PATH" | tr -d ' ')
printf 'case=%s bin=%s turns=%s\n' "$CASE_FILE" "$BIN_NAME" "$TURNS"
grep -E 'extend_fail_detail|probe_summary' "$ERR_PATH" || true
