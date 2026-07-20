#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_PATH="$ROOT_DIR/target/release"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/run.sh [--no-local] <bin_name>
  ./scripts/run.sh [--no-local] <bin_name> <input_file>

Run one solver manually.
By default, the solver is built with --release --features local.
Use --no-local for a release build without the local feature.
Without input_file, stdin is used and stdout is left untouched.
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
if "$BIN_EXEC" < "$INPUT_FILE" > "$OUTPUT_FILE"; then
    STATUS=0
else
    STATUS=$?
fi
cat "$OUTPUT_FILE"
END_ALL=$(date +%s)
ELAPSED_ALL=$((END_ALL - START_ALL))
printf 'run: bin=%s local=%s input=%s elapsed=%ss output=%s\n' "$BIN_NAME" "$LOCAL_LABEL" "$INPUT_FILE" "$ELAPSED_ALL" "$OUTPUT_FILE" >&2
exit "$STATUS"
