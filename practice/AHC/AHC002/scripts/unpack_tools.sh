#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOLS_DIR="$ROOT_DIR/tools"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/unpack_tools.sh
  ./scripts/unpack_tools.sh <zip_path>...

Without arguments, zip files under tools/*.zip are unpacked and merged into tools/.
EOF
}

if ! command -v unzip >/dev/null 2>&1; then
    echo "error: unzip command not found" >&2
    exit 1
fi

collect_default_zips() {
    found=0
    for path in "$TOOLS_DIR"/*.zip; do
        if [ -f "$path" ]; then
            printf '%s\n' "$path"
            found=1
        fi
    done
    if [ "$found" -eq 0 ] && [ -f "$ROOT_DIR/tools.zip" ]; then
        printf '%s\n' "$ROOT_DIR/tools.zip"
    fi
}

TMP_DIR=$(mktemp -d)
MERGE_DIR="$TMP_DIR/merged"
mkdir -p "$MERGE_DIR"

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

ZIP_LIST_FILE="$TMP_DIR/zips.txt"
if [ "$#" -eq 0 ]; then
    collect_default_zips > "$ZIP_LIST_FILE"
else
    : > "$ZIP_LIST_FILE"
    for path in "$@"; do
        printf '%s\n' "$path" >> "$ZIP_LIST_FILE"
    done
fi

if [ ! -s "$ZIP_LIST_FILE" ]; then
    usage
    echo "error: no zip files found" >&2
    exit 1
fi

idx=0
while IFS= read -r ZIP_PATH; do
    if [ ! -f "$ZIP_PATH" ]; then
        echo "error: zip file not found: $ZIP_PATH" >&2
        exit 1
    fi

    EXTRACT_DIR="$TMP_DIR/extract_$idx"
    mkdir -p "$EXTRACT_DIR"
    unzip -oq "$ZIP_PATH" -d "$EXTRACT_DIR"

    COPY_SRC="$EXTRACT_DIR"
    if [ -d "$EXTRACT_DIR/tools" ]; then
        COPY_SRC="$EXTRACT_DIR/tools"
    fi

    cp -R "$COPY_SRC"/. "$MERGE_DIR"/
    idx=$((idx + 1))
done < "$ZIP_LIST_FILE"

mkdir -p "$TOOLS_DIR"
find "$TOOLS_DIR" -mindepth 1 ! -name '.gitkeep' ! -name '*.zip' -exec rm -rf {} +
cp -R "$MERGE_DIR"/. "$TOOLS_DIR"/

printf 'unpacked into %s from:\n' "$TOOLS_DIR" >&2
sed 's/^/  - /' "$ZIP_LIST_FILE" >&2
