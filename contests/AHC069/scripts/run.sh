#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_PATH="$ROOT_DIR/target/release"
TOOLS_BIN_PATH="$ROOT_DIR/tools/target/release"
CPU_WRAPPER="$ROOT_DIR/scripts/measure_solver_cpu.py"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/run.sh [--no-local] <bin_name>
  ./scripts/run.sh [--no-local] <bin_name> <input_file>

Run one solver manually.
By default, the solver is built with --release --features local.
Use --no-local for a release build without the local feature.
Without input_file, stdin is used and stdout is left untouched.
With input_file, the official tester performs the interactive exchange.
EOF
}

LOCAL_FEATURE=1
if [ "${1:-}" = "--no-local" ]; then
    LOCAL_FEATURE=0
    shift
fi

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
INPUT_FILE=${2:-}
BIN_SRC="$ROOT_DIR/src/bin/$BIN_NAME.rs"
ADHOC_SRC="$ROOT_DIR/adhoc/src/bin/$BIN_NAME.rs"

# solver はルート、補助 bin は adhoc クレート。ソースの場所で manifest を切り替える。
if [ -f "$BIN_SRC" ]; then
    MANIFEST_PATH="$ROOT_DIR/Cargo.toml"
elif [ -f "$ADHOC_SRC" ]; then
    MANIFEST_PATH="$ROOT_DIR/adhoc/Cargo.toml"
else
    echo "error: not found: $BIN_SRC nor $ADHOC_SRC" >&2
    exit 1
fi

if [ -n "$INPUT_FILE" ] && [ ! -f "$INPUT_FILE" ]; then
    echo "error: input file not found: $INPUT_FILE" >&2
    exit 1
fi

OUTPUT_DIR="$ROOT_DIR/results/out/$BIN_NAME"
if [ -n "$INPUT_FILE" ]; then
    mkdir -p "$OUTPUT_DIR"
fi

START_ALL=$(date +%s)
if [ "$LOCAL_FEATURE" -eq 1 ]; then
    cargo build --release --features local --quiet --manifest-path "$MANIFEST_PATH" --bin "$BIN_NAME"
    LOCAL_LABEL=on
else
    cargo build --release --quiet --manifest-path "$MANIFEST_PATH" --bin "$BIN_NAME"
    LOCAL_LABEL=off
fi
BIN_EXEC="$BIN_PATH/$BIN_NAME"

if [ ! -x "$BIN_EXEC" ]; then
    echo "error: binary not found: $BIN_EXEC" >&2
    exit 1
fi

if [ -z "$INPUT_FILE" ]; then
    if "$BIN_EXEC"; then
        STATUS=0
    else
        STATUS=$?
    fi
    END_ALL=$(date +%s)
    ELAPSED_ALL=$((END_ALL - START_ALL))
    printf 'run: bin=%s local=%s input=stdin elapsed=%ss output=stdout\n' "$BIN_NAME" "$LOCAL_LABEL" "$ELAPSED_ALL" >&2
    exit "$STATUS"
fi

OUTPUT_FILE="$OUTPUT_DIR/$(basename "$INPUT_FILE")"
CPU_RESULT_FILE="$OUTPUT_FILE.cpu_result.$$"
trap 'rm -f "$CPU_RESULT_FILE"' EXIT HUP INT TERM

cargo build --release --quiet --manifest-path "$ROOT_DIR/tools/Cargo.toml" --bin tester --bin score
TESTER_EXEC="$TOOLS_BIN_PATH/tester"
SCORE_EXEC="$TOOLS_BIN_PATH/score"

if "$TESTER_EXEC" python3 "$CPU_WRAPPER" --result-file "$CPU_RESULT_FILE" "$BIN_EXEC" \
    < "$INPUT_FILE" > "$OUTPUT_FILE"; then
    STATUS=0
else
    STATUS=$?
fi

CPU_NS=$(sed -n 's/^cpu_elapsed_ns=//p' "$CPU_RESULT_FILE" 2>/dev/null || true)
SOLVER_EXIT_CODE=$(sed -n 's/^exit_code=//p' "$CPU_RESULT_FILE" 2>/dev/null || true)
SOLVER_TERM_SIGNAL=$(sed -n 's/^term_signal=//p' "$CPU_RESULT_FILE" 2>/dev/null || true)
if [ -z "$CPU_NS" ] || [ "$SOLVER_EXIT_CODE" != "0" ] || [ "$SOLVER_TERM_SIGNAL" != "0" ]; then
    STATUS=1
fi

if SCORE=$("$SCORE_EXEC" "$INPUT_FILE" "$OUTPUT_FILE"); then
    :
else
    STATUS=$?
    SCORE=invalid
fi
cat "$OUTPUT_FILE"
if [ -n "$CPU_NS" ]; then
    CPU_MS=$(((CPU_NS + 500000) / 1000000))
else
    CPU_MS=unknown
fi
printf 'run: bin=%s local=%s input=%s score=%s cpu_elapsed=%sms output=%s\n' \
    "$BIN_NAME" "$LOCAL_LABEL" "$INPUT_FILE" "$SCORE" "$CPU_MS" "$OUTPUT_FILE" >&2
exit "$STATUS"
