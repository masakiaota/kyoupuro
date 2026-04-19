#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
SRC_FILE="$ROOT_DIR/src/bin/v109_pro_suffix_opt.rs"
TOOLS_DIR="$ROOT_DIR/tools/in"
BUILD_DIR="$ROOT_DIR/target/v109_investigation"
PROBE_DIR="$BUILD_DIR/probe"
BIN_NAME=v109_pro_suffix_opt
REAL_BIN="$ROOT_DIR/target/release/$BIN_NAME"

usage() {
    cat >&2 <<'EOF'
Usage:
  ./scripts/adhoc/investigate_v109.sh scan
  ./scripts/adhoc/investigate_v109.sh profile <case_id|case_file> [more cases...]
EOF
}

now_ms() {
    python3 -c 'import time; print(int(time.time() * 1000))'
}

resolve_case() {
    case "$1" in
        *.txt) printf '%s\n' "$TOOLS_DIR/$1" ;;
        *) printf '%s\n' "$TOOLS_DIR/$1.txt" ;;
    esac
}

build_real() {
    cargo build --release --quiet --bin "$BIN_NAME" >/dev/null
}

prepare_probe_source() {
    mkdir -p "$PROBE_DIR/src"
    cat > "$PROBE_DIR/Cargo.toml" <<'EOF'
[package]
name = "v109_probe"
version = "0.0.0"
edition = "2024"
publish = false

[profile.release]
lto = true

[dependencies]
rustc-hash = "=2.1.1"
EOF

    python3 - "$SRC_FILE" "$PROBE_DIR/src/main.rs" <<'PY'
from pathlib import Path
import sys
src = Path(sys.argv[1]).read_text()
out = Path(sys.argv[2])
insert_after = 'use std::time::Instant;\n'
profiler = '''use std::cell::RefCell;\nuse std::collections::BTreeMap;\nuse std::time::Duration;\n\nthread_local! {\n    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::default());\n}\n\n#[derive(Default)]\nstruct Profiler {\n    stats: BTreeMap<&\'static str, ProfileStat>,\n}\n\n#[derive(Default, Clone, Copy)]\nstruct ProfileStat {\n    calls: u64,\n    nanos: u128,\n}\n\nstruct ProfileScope {\n    name: &\'static str,\n    started: Instant,\n}\n\nimpl Drop for ProfileScope {\n    fn drop(&mut self) {\n        let elapsed = self.started.elapsed();\n        PROFILER.with(|prof| {\n            let mut prof = prof.borrow_mut();\n            let ent = prof.stats.entry(self.name).or_default();\n            ent.calls += 1;\n            ent.nanos += elapsed.as_nanos();\n        });\n    }\n}\n\n#[inline]\nfn start_profile_scope(name: &\'static str) -> ProfileScope {\n    ProfileScope {\n        name,\n        started: Instant::now(),\n    }\n}\n\nmacro_rules! profile_scope {\n    ($name:literal) => {\n        let _profile_scope_guard = start_profile_scope($name);\n    };\n}\n\nfn dump_profile() {\n    PROFILER.with(|prof| {\n        for (name, stat) in prof.borrow().stats.iter() {\n            let ms = Duration::from_nanos(stat.nanos.min(u64::MAX as u128) as u64).as_secs_f64() * 1000.0;\n            eprintln!("profile,{name},calls={},ms={ms:.3}", stat.calls);\n        }\n    });\n}\n\n'''
if insert_after not in src:
    raise SystemExit('failed to find import anchor')
src = src.replace(insert_after, insert_after + profiler, 1)
replacements = {
    'fn step(st: &State, dir: usize) -> (State, u8, Option<usize>, Vec<Dropped>) {\n':
        'fn step(st: &State, dir: usize) -> (State, u8, Option<usize>, Vec<Dropped>) {\n    profile_scope!("step");\n',
    'fn encode_key(st: &State) -> Key {\n':
        'fn encode_key(st: &State) -> Key {\n    profile_scope!("encode_key");\n',
    'fn nearest_food_dist(st: &State, color: u8) -> (usize, usize) {\n':
        'fn nearest_food_dist(st: &State, color: u8) -> (usize, usize) {\n    profile_scope!("nearest_food_dist");\n',
    'fn greedy_future_lb_from_cell(\n    st: &State,\n    input: &Input,\n    start_cell: u16,\n    start_ell: usize,\n    horizon: usize,\n    banned: Option<u16>,\n) -> (usize, usize, usize) {\n':
        'fn greedy_future_lb_from_cell(\n    st: &State,\n    input: &Input,\n    start_cell: u16,\n    start_ell: usize,\n    horizon: usize,\n    banned: Option<u16>,\n) -> (usize, usize, usize) {\n    profile_scope!("greedy_future_lb_from_cell");\n',
    'fn stage_search_bestfirst(\n    start_bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    budgets: &[(usize, usize)],\n    keep_solutions: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n':
        'fn stage_search_bestfirst(\n    start_bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    budgets: &[(usize, usize)],\n    keep_solutions: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n    profile_scope!("stage_search_bestfirst");\n',
    'fn collect_food_cells(st: &State, color: u8) -> Vec<u16> {\n':
        'fn collect_food_cells(st: &State, color: u8) -> Vec<u16> {\n    profile_scope!("collect_food_cells");\n',
    'fn bfs_next_dir(st: &State, goal: u16, target: u16, avoid_food: bool, strict_body: bool) -> Option<usize> {\n':
        'fn bfs_next_dir(st: &State, goal: u16, target: u16, avoid_food: bool, strict_body: bool) -> Option<usize> {\n    profile_scope!("bfs_next_dir");\n',
    'fn navigate_to_goal_safe(bs: &BeamState, goal: u16, target: u16, started: &Instant) -> Option<BeamState> {\n':
        'fn navigate_to_goal_safe(bs: &BeamState, goal: u16, target: u16, started: &Instant) -> Option<BeamState> {\n    profile_scope!("navigate_to_goal_safe");\n',
    'fn navigate_to_goal_loose(\n    bs: &BeamState,\n    goal: u16,\n    target: u16,\n    ell: usize,\n    started: &Instant,\n) -> Option<BeamState> {\n':
        'fn navigate_to_goal_loose(\n    bs: &BeamState,\n    goal: u16,\n    target: u16,\n    ell: usize,\n    started: &Instant,\n) -> Option<BeamState> {\n    profile_scope!("navigate_to_goal_loose");\n',
    'fn shrink_to_ell(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target: u16,\n    target_color: u8,\n    started: &Instant,\n) -> Option<BeamState> {\n':
        'fn shrink_to_ell(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target: u16,\n    target_color: u8,\n    started: &Instant,\n) -> Option<BeamState> {\n    profile_scope!("shrink_to_ell");\n',
    'fn try_target_exact(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target: u16,\n    target_color: u8,\n    started: &Instant,\n) -> Vec<BeamState> {\n':
        'fn try_target_exact(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target: u16,\n    target_color: u8,\n    started: &Instant,\n) -> Vec<BeamState> {\n    profile_scope!("try_target_exact");\n',
    'fn collect_exact_solutions(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    max_targets: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n':
        'fn collect_exact_solutions(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    max_targets: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n    profile_scope!("collect_exact_solutions");\n',
    'fn collect_exact_solutions_turn_focused(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    max_targets: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n':
        'fn collect_exact_solutions_turn_focused(\n    bs: &BeamState,\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    max_targets: usize,\n    started: &Instant,\n) -> Vec<BeamState> {\n    profile_scope!("collect_exact_solutions_turn_focused");\n',
    'fn rescue_stage(\n    beam: &[BeamState],\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    started: &Instant,\n) -> Vec<BeamState> {\n':
        'fn rescue_stage(\n    beam: &[BeamState],\n    input: &Input,\n    ell: usize,\n    target_color: u8,\n    started: &Instant,\n) -> Vec<BeamState> {\n    profile_scope!("rescue_stage");\n',
    'fn solve_base(input: &Input, started: &Instant) -> BeamState {\n':
        'fn solve_base(input: &Input, started: &Instant) -> BeamState {\n    profile_scope!("solve_base");\n',
    'fn reconstruct_exact_checkpoints(input: &Input, ops: &str) -> Vec<Option<(usize, State)>> {\n':
        'fn reconstruct_exact_checkpoints(input: &Input, ops: &str) -> Vec<Option<(usize, State)>> {\n    profile_scope!("reconstruct_exact_checkpoints");\n',
    'fn solve_suffix_turn_focused(\n    input: &Input,\n    start_bs: BeamState,\n    start_ell: usize,\n    started: &Instant,\n) -> BeamState {\n':
        'fn solve_suffix_turn_focused(\n    input: &Input,\n    start_bs: BeamState,\n    start_ell: usize,\n    started: &Instant,\n) -> BeamState {\n    profile_scope!("solve_suffix_turn_focused");\n',
    'fn optimize_exact_suffix(input: &Input, base: BeamState, started: &Instant) -> BeamState {\n':
        'fn optimize_exact_suffix(input: &Input, base: BeamState, started: &Instant) -> BeamState {\n    profile_scope!("optimize_exact_suffix");\n',
    'fn solve(input: &Input) -> String {\n':
        'fn solve(input: &Input) -> String {\n    profile_scope!("solve_total");\n',
}
for old, new in replacements.items():
    if old not in src:
        raise SystemExit(f'failed to patch signature: {old.splitlines()[0]}')
    src = src.replace(old, new, 1)
if 'print!("{out}");\n}' not in src:
    raise SystemExit('failed to patch main print')
src = src.replace('print!("{out}");\n}', 'print!("{out}");\n    dump_profile();\n}', 1)
out.write_text(src)
PY
}

build_probe() {
    prepare_probe_source
    cargo build --release --quiet --manifest-path "$PROBE_DIR/Cargo.toml" >/dev/null
}

run_scan() {
    build_real
    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT INT TERM
    for input in "$TOOLS_DIR"/*.txt; do
        base=$(basename "$input")
        start=$(now_ms)
        "$REAL_BIN" < "$input" > /dev/null 2>/dev/null
        elapsed=$(( $(now_ms) - start ))
        printf '%s\t%s\n' "$elapsed" "$base" >> "$tmp"
    done
    sort -rn "$tmp"
}

run_profile() {
    if [ "$#" -lt 1 ]; then
        usage
        exit 1
    fi
    build_probe
    probe_bin="$PROBE_DIR/target/release/v109_probe"
    for case_arg in "$@"; do
        input=$(resolve_case "$case_arg")
        if [ ! -f "$input" ]; then
            echo "error: input not found: $input" >&2
            exit 1
        fi
        base=$(basename "$input")
        printf '=== %s ===\n' "$base"
        /usr/bin/time -lp "$probe_bin" < "$input" > /dev/null
        printf '\n'
    done
}

if [ "$#" -lt 1 ]; then
    usage
    exit 1
fi

cmd=$1
shift
case "$cmd" in
    scan) run_scan ;;
    profile) run_profile "$@" ;;
    *) usage; exit 1 ;;
esac
