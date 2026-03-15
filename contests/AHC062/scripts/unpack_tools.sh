#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

detect_zip_path() {
    if [ -f "$ROOT_DIR/tools.zip" ]; then
        printf '%s\n' "$ROOT_DIR/tools.zip"
        return 0
    fi

    set -- "$ROOT_DIR"/tools/*.zip
    if [ "$#" -eq 1 ] && [ -f "$1" ]; then
        printf '%s\n' "$1"
        return 0
    fi

    if [ "$#" -gt 1 ] && [ -f "$1" ]; then
        echo "error: multiple zip files found under tools/. specify one explicitly." >&2
        exit 1
    fi

    echo "error: zip file not found. pass a path explicitly or place one zip under tools/." >&2
    exit 1
}

ZIP_PATH=${1:-$(detect_zip_path)}

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

if [ ! -f "$ZIP_PATH" ]; then
    echo "error: zip file not found: $ZIP_PATH" >&2
    exit 1
fi

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

unzip -oq "$ZIP_PATH" -d "$TMP_DIR"

SRC_DIR=$TMP_DIR
if [ -d "$TMP_DIR/tools" ]; then
    SRC_DIR="$TMP_DIR/tools"
fi

mkdir -p "$ROOT_DIR/tools"
find "$ROOT_DIR/tools" -mindepth 1 ! -name .gitkeep ! -name '*.zip' -exec rm -rf {} +
cp -R "$SRC_DIR"/. "$ROOT_DIR/tools"/

echo "unpacked: $ZIP_PATH -> $ROOT_DIR/tools" >&2
