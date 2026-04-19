#!/bin/sh
set -eu

cd "$(dirname "$0")/../.."

cargo run --release --bin tmp_prefix_repair_bench -- "$@"
