#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
ZIP_PATH=${1:-"$ROOT_DIR/tools.zip"}
ZIP_DIR=$(dirname -- "$ZIP_PATH")
ZIP_BASE=$(basename -- "$ZIP_PATH")
ZIP_PATH_ABS=$(CDPATH= cd -- "$ZIP_DIR" && pwd)/$ZIP_BASE

usage() {
    echo "Usage: $0 [tools_zip_path]" >&2
}

if [ "$#" -gt 1 ]; then
    usage
    exit 1
fi

if ! command -v unzip >/dev/null 2>&1; then
    echo "error: unzip command not found" >&2
    exit 1
fi

if [ ! -f "$ZIP_PATH_ABS" ]; then
    echo "error: zip file not found: $ZIP_PATH_ABS" >&2
    exit 1
fi

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

unzip -o "$ZIP_PATH_ABS" -d "$TMP_DIR"

SRC_DIR=$TMP_DIR
if [ -d "$TMP_DIR/tools" ]; then
    SRC_DIR="$TMP_DIR/tools"
fi

mkdir -p "$ROOT_DIR/tools"
KEEP_NAME=
case "$ZIP_PATH_ABS" in
    "$ROOT_DIR/tools"/*)
        KEEP_NAME=$(basename "$ZIP_PATH_ABS")
        ;;
esac

if [ -n "$KEEP_NAME" ]; then
    find "$ROOT_DIR/tools" -mindepth 1 ! -name .gitkeep ! -name "$KEEP_NAME" -exec rm -rf {} +
else
    find "$ROOT_DIR/tools" -mindepth 1 ! -name .gitkeep -exec rm -rf {} +
fi
cp -R "$SRC_DIR"/. "$ROOT_DIR/tools"/

echo "unpacked: $ZIP_PATH_ABS -> $ROOT_DIR/tools" >&2
