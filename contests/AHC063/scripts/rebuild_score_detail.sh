#!/usr/bin/env sh
set -eu

usage() {
    cat >&2 <<'EOF_USAGE'
Usage:
  ./scripts/rebuild_score_detail.sh <existing_csv> <new_row_file> <output_csv>

Rebuilds results/score_detail.csv by appending new rows and recomputing local_relative_score.
EOF_USAGE
}

build_detail_header() {
    printf 'bin,total_avg,local_relative_score'
    i=0
    while [ "$i" -lt 100 ]; do
        printf ',case%04d' "$i"
        i=$((i + 1))
    done
    printf ',max_elapsed,executed_at'
}

if [ "$#" -ne 3 ]; then
    usage
    exit 1
fi

EXISTING_CSV=$1
NEW_ROW_FILE=$2
OUTPUT_CSV=$3
DETAIL_HEADER=$(build_detail_header)
COMBINED_INPUT=$(mktemp)

cleanup() {
    rm -f "$COMBINED_INPUT"
}
trap cleanup EXIT INT TERM

if [ -f "$EXISTING_CSV" ] && [ -s "$EXISTING_CSV" ]; then
    IFS= read -r FIRST_LINE < "$EXISTING_CSV" || true
    if [ -n "$FIRST_LINE" ] && [ "$FIRST_LINE" != "$DETAIL_HEADER" ]; then
        echo "error: unexpected header in existing score_detail.csv" >&2
        exit 1
    fi
    sed '1d' "$EXISTING_CSV" >> "$COMBINED_INPUT"
fi

if [ -f "$NEW_ROW_FILE" ] && [ -s "$NEW_ROW_FILE" ]; then
    cat "$NEW_ROW_FILE" >> "$COMBINED_INPUT"
fi

{
    printf '%s\n' "$DETAIL_HEADER"
    awk -F, -v OFS=, '
        function round_nearest(value) {
            if (value >= 0) {
                return int(value + 0.5)
            }
            return -int(-value + 0.5)
        }

        BEGIN {
            case_count = 100
        }

        NF == 0 {
            next
        }

        {
            if (NF != 105) {
                printf "error: expected 105 columns in score_detail row, got %d on row %d\n", NF, NR > "/dev/stderr"
                exit 1
            }

            row_count++
            bin[row_count] = $1
            total_avg[row_count] = $2
            max_elapsed[row_count] = $104
            executed_at[row_count] = $105

            if (seen_executed_at[$105]++) {
                printf "error: duplicated executed_at in score_detail.csv: %s\n", $105 > "/dev/stderr"
                exit 1
            }

            for (i = 0; i < case_count; i++) {
                score[row_count, i] = $(i + 4)
            }
        }

        END {
            if (row_count == 0) {
                exit 0
            }

            for (row = 1; row <= row_count; row++) {
                relative_sum[row] = 0
            }

            for (case_index = 0; case_index < case_count; case_index++) {
                for (row = 1; row <= row_count; row++) {
                    lose = 0
                    tie = 0
                    current = score[row, case_index] + 0
                    for (other = 1; other <= row_count; other++) {
                        if (other == row) {
                            continue
                        }
                        candidate = score[other, case_index] + 0
                        if (candidate < current) {
                            lose++
                        } else if (candidate == current) {
                            tie++
                        }
                    }
                    rank_value = lose + 0.5 * tie
                    case_relative = round_nearest(1000000000 * (1 - rank_value / row_count))
                    relative_sum[row] += case_relative
                }
            }

            for (row = 1; row <= row_count; row++) {
                printf "%s,%d,%d", bin[row], round_nearest(total_avg[row] + 0), round_nearest(relative_sum[row] / case_count)
                for (case_index = 0; case_index < case_count; case_index++) {
                    printf ",%s", score[row, case_index]
                }
                printf ",%s,%s\n", max_elapsed[row], executed_at[row]
            }
        }
    ' "$COMBINED_INPUT"
} > "$OUTPUT_CSV"
