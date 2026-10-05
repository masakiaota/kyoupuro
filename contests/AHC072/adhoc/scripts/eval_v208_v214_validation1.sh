#!/bin/sh
# 指定された6版を、LOCAL・同時実行1ケースで順番に評価する。
set -eu

root_dir=$(CDPATH= cd "$(dirname "$0")/../.." && pwd)
run_dir=${1:?Usage: sh eval_v208_v214_validation1.sh <absolute_run_dir>}
label="validation1_j1_v208_v214_$(basename "$run_dir")"
mkdir -p "$run_dir"
cd "$root_dir"

# 応答終了後も、終了コードと最後に処理した版を確認できるようにする。
current_bin=initializing
finish() {
    result=$?
    trap - EXIT
    if [ "$result" -eq 0 ]; then
        printf 'completed\n' > "$run_dir/status.txt"
    else
        printf 'failed bin=%s exit_code=%s\n' "$current_bin" "$result" > "$run_dir/status.txt"
    fi
    printf '%s\n' "$result" > "$run_dir/exit_code.txt"
    TZ=Asia/Tokyo date '+%Y-%m-%dT%H:%M:%S%z' > "$run_dir/finished_at.txt"
    exit "$result"
}
trap finish EXIT

printf '%s\n' "$label" > "$run_dir/label.txt"
TZ=Asia/Tokyo date '+%Y-%m-%dT%H:%M:%S%z' > "$run_dir/started_at.txt"

for current_bin in \
    v210_collection_color_quotient \
    v214_local_color_runs \
    v209_color_quotient_reinsertion \
    v212_mono_packet_quotient \
    v211_jump_steiner_collection \
    v208_mono_collection
do
    printf 'running bin=%s\n' "$current_bin" > "$run_dir/status.txt"
    printf 'start: %s\n' "$current_bin"
    python3 scripts/eval.py "$current_bin" tools/validation1 \
        -j 1 --wait-lock --label "$label"
    printf 'finished: %s\n' "$current_bin"
done
