#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

# shellcheck source=scripts/cpp_common.sh
. "$SCRIPT_DIR/cpp_common.sh"

usage() {
    echo "Usage: $0 <bin_name>" >&2
}

if [ "$#" -ne 1 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
SRC="$ROOT_DIR/cpp/bin/$BIN_NAME.cpp"
DST="$ROOT_DIR/cpp/main.cpp"
BUILD_DIR="$ROOT_DIR/target/cpp"
MAIN_BIN="$BUILD_DIR/main"

if [ ! -f "$SRC" ]; then
    echo "error: not found: $SRC" >&2
    exit 1
fi

if ! CXX_BIN=$(detect_cxx); then
    echo "error: C++ compiler not found. Install g++/clang++ or set CXX." >&2
    exit 1
fi

if ! STD_FLAG=$(detect_cpp_std_flag "$CXX_BIN"); then
    echo "error: compiler '$CXX_BIN' does not support C++23/C++2b." >&2
    exit 1
fi

mkdir -p "$BUILD_DIR"
cp "$SRC" "$DST"

echo "promoted: cpp/bin/$BIN_NAME.cpp -> cpp/main.cpp" >&2
echo "compile: $CXX_BIN $STD_FLAG $DST" >&2
compile_cpp "$CXX_BIN" "$STD_FLAG" "$DST" "$MAIN_BIN"
echo "release-like compile: OK" >&2
