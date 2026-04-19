#!/bin/sh
set -eu

cd "$(dirname "$0")/../.."

cargo run --release --bin bench_internal_colors_beam -- "$@"
