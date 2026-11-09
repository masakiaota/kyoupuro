#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/build_solver.sh [--no-local] [--profile NAME] [--time-ratio RATIO] <bin_name>

Build src/bin/<bin_name>.cpp or adhoc/bin/<bin_name>.cpp.
The LOCAL macro is defined unless --no-local is specified.
Local ratio comes from config/local_profiles.json (default: 1).
--time-ratio passes the value already selected by eval.py.
Set CXX to override the compiler (default: g++-15).
EOF
}

LOCAL_BUILD=1
PROFILE=
RATIO_OVERRIDE=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-local) LOCAL_BUILD=0; shift ;;
        --profile|--time-ratio)
            if [ "$#" -lt 2 ]; then usage; exit 1; fi
            if [ "$1" = "--profile" ]; then PROFILE=$2; else RATIO_OVERRIDE=$2; fi
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        --) shift; break ;;
        -*) usage; exit 1 ;;
        *) break ;;
    esac
done

if [ "$#" -ne 1 ]; then
    usage
    exit 1
fi

BIN_NAME=$1
SOLVER_SRC="$ROOT_DIR/src/bin/$BIN_NAME.cpp"
ADHOC_SRC="$ROOT_DIR/adhoc/bin/$BIN_NAME.cpp"

if [ -f "$SOLVER_SRC" ]; then
    SOURCE_FILE=$SOLVER_SRC
elif [ -f "$ADHOC_SRC" ]; then
    SOURCE_FILE=$ADHOC_SRC
else
    echo "error: not found: $SOLVER_SRC nor $ADHOC_SRC" >&2
    exit 1
fi

CXX_BIN=${CXX:-g++-15}
if ! command -v "$CXX_BIN" >/dev/null 2>&1; then
    echo "error: C++ compiler not found: $CXX_BIN" >&2
    echo "hint: install GCC 15 or set CXX to its executable" >&2
    exit 1
fi

OUTPUT_DIR="$ROOT_DIR/target/release"
OUTPUT_FILE="$OUTPUT_DIR/$BIN_NAME"
mkdir -p "$OUTPUT_DIR"

if [ "$LOCAL_BUILD" -eq 1 ]; then
    if [ -n "$RATIO_OVERRIDE" ]; then
        LOCAL_TIME_RATIO=$(python3 "$SCRIPT_DIR/local_config.py" --ratio "$RATIO_OVERRIDE")
    elif [ -n "$PROFILE" ]; then
        LOCAL_TIME_RATIO=$(python3 "$SCRIPT_DIR/local_config.py" --profile "$PROFILE")
    else
        LOCAL_TIME_RATIO=$(python3 "$SCRIPT_DIR/local_config.py")
    fi
    printf 'build: local=on ratio=%s compiler=%s\n' "$LOCAL_TIME_RATIO" "$CXX_BIN" >&2
    "$CXX_BIN" \
        -std=gnu++23 -O2 -Wall -Wextra -march=native -pthread \
        -ftrivial-auto-var-init=zero -fopenmp -DLOCAL "-DAHC_LOCAL_TIME_RATIO=$LOCAL_TIME_RATIO" \
        "$SOURCE_FILE" -o "$OUTPUT_FILE"
else
    printf 'build: local=off ratio=1 compiler=%s\n' "$CXX_BIN" >&2
    "$CXX_BIN" \
        -std=gnu++23 -O2 -Wall -Wextra -march=native -pthread \
        -ftrivial-auto-var-init=zero -fopenmp \
        -DATCODER -DONLINE_JUDGE -DNOMINMAX \
        "$SOURCE_FILE" -o "$OUTPUT_FILE"
fi

printf '%s\n' "$OUTPUT_FILE"
