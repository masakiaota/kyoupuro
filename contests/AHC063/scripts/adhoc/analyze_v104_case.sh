#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

if [ "$#" -lt 1 ]; then
    cat >&2 <<'EOF'
Usage:
  ./scripts/adhoc/analyze_v104_case.sh <case_id|case_file> [extra_args...]

Examples:
  ./scripts/adhoc/analyze_v104_case.sh 0062
  ./scripts/adhoc/analyze_v104_case.sh 0062 --rerun
EOF
    exit 1
fi

cd "$ROOT_DIR"
exec "$SCRIPT_DIR/analyze_v104_case.py" "$@"
