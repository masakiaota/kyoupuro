// v103_sa_fastbuild.rs
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Write as _;
use std::io::{self, Read};
use std::rc::Rc;
use std::str::{FromStr, SplitAsciiWhitespace};
use std::time::Instant;

const N: usize = 32;
const M: usize = N * N;
const WORDS: usize = M / 64;
const PATTERN_LAYER: usize = 1;

const SOLVER_TIME_LIMIT_SEC: f64 = 1.92;
const SEARCH_WRAPUP_MARGIN_SEC: f64 = 0.12;
const BUILD_MARGIN_SEC: f64 = 0.05;
const INNER_LOOP_MARGIN_SEC: f64 = 0.02;
const SA_START_TEMP: f64 = 12.0;
const SA_END_TEMP: f64 = 0.05;
const MAX_PATTERNS: usize = 24;
const MAX_OCCURRENCES_PER_PATTERN: usize = 768;
const MAX_BATCH_OCCURRENCES: usize = 96;
const MAX_BATCH_STEPS: usize = 64;
const MAX_WINDOW_SIDE: usize = 12;
const MAX_COLOR: usize = 4;

type Color = u8;
type Grid = [[Color; N]; N];
type Coord = (usize, usize);
type GoalRows = [[u32; N]; MAX_COLOR + 1];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Input {
    k_layers: usize,
    color_count: usize,
    goal: Grid,
    goal_rows: GoalRows,
}

impl Input {
    fn read() -> Self {
        let mut src = String::new();
        io::stdin()
            .read_to_string(&mut src)
            .expect("failed to read stdin");
        Self::from_str(&src)
    }

    fn from_str(src: &str) -> Self {
        let mut tokens = src.split_ascii_whitespace();

        let _: usize = read_value(&mut tokens);
        let k_layers = read_value(&mut tokens);
        let color_count = read_value(&mut tokens);

        let mut goal = [[0; N]; N];
        for row in &mut goal {
            for cell in row {
                *cell = read_value(&mut tokens);
            }
        }
        let goal_rows = build_goal_rows(&goal);

        Self {
            k_layers,
            color_count,
            goal,
            goal_rows,
        }
    }

    fn nonzero_goal_count(&self) -> usize {
        self.goal
            .iter()
            .flatten()
            .filter(|&&color| color != 0)
            .count()
    }

    fn nonzero_goal_positions(&self) -> Vec<usize> {
        let mut positions = Vec::with_capacity(self.nonzero_goal_count());
        for i in 0..N {
            for j in 0..N {
                if self.goal[i][j] != 0 {
                    positions.push(i * N + j);
                }
            }
        }
        positions
    }
}

fn build_goal_rows(goal: &Grid) -> GoalRows {
    let mut goal_rows = [[0_u32; N]; MAX_COLOR + 1];
    for (i, row) in goal.iter().enumerate() {
        for (j, &color) in row.iter().enumerate() {
            if color != 0 {
                goal_rows[color as usize][i] |= 1_u32 << j;
            }
        }
    }
    goal_rows
}

#[inline(always)]
fn read_value<T>(tokens: &mut SplitAsciiWhitespace<'_>) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    tokens.next().unwrap().parse().unwrap()
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
    iter: u64,
    check_mask: u64,
    is_over: bool,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };
        Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            is_over: false,
        }
    }

    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.is_over = self.start.elapsed().as_secs_f64() >= self.time_limit_sec;
        }
        !self.is_over
    }

    #[inline]
    fn is_over(&mut self) -> bool {
        if !self.is_over {
            self.is_over = self.start.elapsed().as_secs_f64() >= self.time_limit_sec;
        }
        self.is_over
    }

    #[inline]
    fn remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.start.elapsed().as_secs_f64()).max(0.0)
    }

    #[inline]
    fn has_time_left(&self, margin_sec: f64) -> bool {
        self.remaining_sec() > margin_sec
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Transform {
    rot: usize,
    di: isize,
    dj: isize,
}

impl Transform {
    fn new(rot: usize, di: isize, dj: isize) -> Self {
        Self {
            rot: rot % 4,
            di,
            dj,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Paint {
        k: usize,
        i: usize,
        j: usize,
        color: Color,
    },
    Copy {
        k: usize,
        h: usize,
        transform: Transform,
    },
    Clear {
        k: usize,
    },
}

impl Op {
    fn write_to(self, out: &mut String) {
        match self {
            Self::Paint { k, i, j, color } => {
                let _ = writeln!(out, "0 {k} {i} {j} {color}");
            }
            Self::Copy { k, h, transform } => {
                let _ = writeln!(
                    out,
                    "1 {k} {h} {} {} {}",
                    transform.rot, transform.di, transform.dj
                );
            }
            Self::Clear { k } => {
                let _ = writeln!(out, "2 {k}");
            }
        }
    }
}

fn format_ops(ops: &[Op]) -> String {
    let mut out = String::new();
    for &op in ops {
        op.write_to(&mut out);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Bits {
    w: [u64; WORDS],
}

impl Bits {
    #[inline]
    fn set(&mut self, p: usize) {
        self.w[p >> 6] |= 1_u64 << (p & 63);
    }

    #[inline]
    fn contains(&self, p: usize) -> bool {
        ((self.w[p >> 6] >> (p & 63)) & 1) != 0
    }

    #[inline]
    fn or_assign(&mut self, other: &Self) {
        for i in 0..WORDS {
            self.w[i] |= other.w[i];
        }
    }

    #[inline]
    fn minus_assign(&mut self, other: &Self) {
        for i in 0..WORDS {
            self.w[i] &= !other.w[i];
        }
    }

    #[inline]
    fn count(&self) -> usize {
        self.w.iter().map(|x| x.count_ones() as usize).sum()
    }

    #[inline]
    fn and_count(&self, other: &Self) -> usize {
        self.w
            .iter()
            .zip(other.w.iter())
            .map(|(&a, &b)| (a & b).count_ones() as usize)
            .sum()
    }

    fn positions(&self) -> Vec<usize> {
        let mut res = Vec::with_capacity(self.count());
        for (block, &mut_bits) in self.w.iter().enumerate() {
            let mut bits = mut_bits;
            while bits != 0 {
                let tz = bits.trailing_zeros() as usize;
                res.push((block << 6) + tz);
                bits &= bits - 1;
            }
        }
        res
    }
}

#[inline]
fn rotate_coord((i, j): Coord, rot: usize) -> Coord {
    match rot % 4 {
        0 => (i, j),
        1 => (j, N - 1 - i),
        2 => (N - 1 - i, N - 1 - j),
        3 => (N - 1 - j, i),
        _ => unreachable!(),
    }
}

#[inline]
fn rotated_dims(height: usize, width: usize, rot: usize) -> (usize, usize) {
    match rot % 4 {
        0 | 2 => (height, width),
        1 | 3 => (width, height),
        _ => unreachable!(),
    }
}

#[inline]
fn rotate_local_coord((i, j): Coord, height: usize, width: usize, rot: usize) -> Coord {
    match rot % 4 {
        0 => (i, j),
        1 => (j, height - 1 - i),
        2 => (height - 1 - i, width - 1 - j),
        3 => (width - 1 - j, i),
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EditablePattern {
    top: isize,
    left: isize,
    height: usize,
    width: usize,
    mask: Vec<u8>,
}

impl EditablePattern {
    fn idx(&self, i: usize, j: usize) -> usize {
        i * self.width + j
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cell {
    i: usize,
    j: usize,
    color: Color,
}

#[derive(Debug, Clone)]
struct Pattern {
    height: usize,
    width: usize,
    cells: Vec<Cell>,
}

#[derive(Debug, Clone)]
struct RotatedView {
    rot: usize,
    height: usize,
    width: usize,
    min_board_i: usize,
    min_board_j: usize,
    cells: Vec<Cell>,
    row_any_masks: Vec<u32>,
    row_color_masks: Vec<[u32; MAX_COLOR + 1]>,
}

#[derive(Debug, Clone)]
struct Occurrence {
    rot: usize,
    top: usize,
    left: usize,
    fix_mask: Bits,
    break_mask: Bits,
    break_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct OccurrenceSeed {
    rot: usize,
    top: usize,
    left: usize,
    fix_count: usize,
    break_count: usize,
}

#[derive(Debug, Clone)]
struct CompiledPattern {
    pattern: Pattern,
    rotations: Vec<RotatedView>,
    occurrences: Vec<Occurrence>,
    build_cost: usize,
}

#[derive(Debug, Clone)]
struct BatchPlan {
    pattern_idx: usize,
    occurrence_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct EvalResult {
    cost: usize,
    residual: Bits,
    steps: Vec<BatchPlan>,
    total_pattern_cells: usize,
}

#[derive(Debug, Clone)]
struct SearchState {
    patterns: Vec<EditablePattern>,
    compiled: Vec<Rc<CompiledPattern>>,
    eval: EvalResult,
}

#[derive(Debug, Default, Clone)]
struct ProbeStats {
    baseline_cost: usize,
    initial_build_calls: usize,
    initial_build_successes: usize,
    search_iters: usize,
    candidate_build_attempts: usize,
    candidate_build_successes: usize,
    candidate_build_failures: usize,
    build_patterns_total: usize,
    cache_hits: usize,
    cache_misses: usize,
    accepted_total: usize,
    accepted_better: usize,
    accepted_equal: usize,
    accepted_worse: usize,
    best_updates: usize,
    final_current_cost: usize,
    final_best_cost: usize,
    final_best_patterns: usize,
    elapsed_ms: usize,
    build_state_total_sec: f64,
    build_state_calls: usize,
    build_state_max_sec: f64,
    apply_edit_total_sec: f64,
    apply_edit_calls: usize,
    apply_edit_max_sec: f64,
    compile_total_sec: f64,
    compile_calls: usize,
    compile_max_sec: f64,
    enumerate_total_sec: f64,
    enumerate_calls: usize,
    enumerate_max_sec: f64,
    plan_total_sec: f64,
    plan_calls: usize,
    plan_max_sec: f64,
}

impl ProbeStats {
    fn add(total: &mut f64, calls: &mut usize, max_sec: &mut f64, elapsed_sec: f64) {
        *total += elapsed_sec;
        *calls += 1;
        if elapsed_sec > *max_sec {
            *max_sec = elapsed_sec;
        }
    }

    fn report(&self) -> String {
        let total_build_successes = self.initial_build_successes + self.candidate_build_successes;
        let avg_patterns_per_eval = if total_build_successes == 0 {
            0.0
        } else {
            self.build_patterns_total as f64 / total_build_successes as f64
        };
        format!(
            "baseline_cost={} initial_build_calls={} initial_build_successes={} search_iters={} candidate_build_attempts={} candidate_evals={} candidate_build_failures={} avg_patterns_per_eval={:.2} cache_hits={} cache_misses={} accepted_total={} accepted_better={} accepted_equal={} accepted_worse={} best_updates={} final_current_cost={} final_best_cost={} final_best_patterns={} elapsed_ms={} build_state_ms={:.1} build_state_calls={} build_state_max_ms={:.3} apply_edit_ms={:.1} apply_edit_calls={} apply_edit_max_ms={:.3} compile_ms={:.1} compile_calls={} compile_max_ms={:.3} enumerate_ms={:.1} enumerate_calls={} enumerate_max_ms={:.3} plan_ms={:.1} plan_calls={} plan_max_ms={:.3}",
            self.baseline_cost,
            self.initial_build_calls,
            self.initial_build_successes,
            self.search_iters,
            self.candidate_build_attempts,
            self.candidate_build_successes,
            self.candidate_build_failures,
            avg_patterns_per_eval,
            self.cache_hits,
            self.cache_misses,
            self.accepted_total,
            self.accepted_better,
            self.accepted_equal,
            self.accepted_worse,
            self.best_updates,
            self.final_current_cost,
            self.final_best_cost,
            self.final_best_patterns,
            self.elapsed_ms,
            self.build_state_total_sec * 1000.0,
            self.build_state_calls,
            self.build_state_max_sec * 1000.0,
            self.apply_edit_total_sec * 1000.0,
            self.apply_edit_calls,
            self.apply_edit_max_sec * 1000.0,
            self.compile_total_sec * 1000.0,
            self.compile_calls,
            self.compile_max_sec * 1000.0,
            self.enumerate_total_sec * 1000.0,
            self.enumerate_calls,
            self.enumerate_max_sec * 1000.0,
            self.plan_total_sec * 1000.0,
            self.plan_calls,
            self.plan_max_sec * 1000.0,
        )
    }
}

#[derive(Debug, Clone)]
struct BatchEval {
    occurrence_indices: Vec<usize>,
    residual_after: Bits,
    residual_count: usize,
    total_gain: isize,
}

#[derive(Debug, Clone)]
enum NeighborEdit {
    None,
    Add(EditablePattern),
    Drop { idx: usize },
    Replace { idx: usize, pattern: EditablePattern },
}

fn main() {
    let input = Input::read();
    let stats = solve(&input);
    println!("{}", stats.report());
}

fn solve(input: &Input) -> ProbeStats {
    let baseline_cost = baseline_paint_ops(&input.goal).len();
    let mut stats = ProbeStats {
        baseline_cost,
        ..ProbeStats::default()
    };
    if input.k_layers < 2 {
        return stats;
    }

    let nonzero_positions = input.nonzero_goal_positions();
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_input(input));
    let mut time_keeper = TimeKeeper::new(SOLVER_TIME_LIMIT_SEC, 6);
    let mut compiled_cache = FxHashMap::default();

    stats.initial_build_calls += 1;
    let Some(mut current) =
        build_search_state_full(Vec::new(), input, &mut compiled_cache, &mut time_keeper, &mut stats)
    else {
        stats.elapsed_ms = time_keeper.start.elapsed().as_millis() as usize;
        return stats;
    };
    stats.initial_build_successes += 1;
    let mut best = current.clone();
    let search_budget_sec = (SOLVER_TIME_LIMIT_SEC - SEARCH_WRAPUP_MARGIN_SEC).max(0.0);
    while time_keeper.step() {
        stats.search_iters += 1;
        if !time_keeper.has_time_left(SEARCH_WRAPUP_MARGIN_SEC) {
            break;
        }
        let edit = propose_neighbor(&current, input, &nonzero_positions, &mut rng);
        stats.candidate_build_attempts += 1;
        let Some(candidate) =
            apply_neighbor_edit(
                &current,
                edit,
                input,
                &mut compiled_cache,
                &mut time_keeper,
                &mut stats,
            )
        else {
            stats.candidate_build_failures += 1;
            break;
        };
        stats.candidate_build_successes += 1;
        let delta = candidate.eval.cost as isize - current.eval.cost as isize;
        let temp = temperature(&time_keeper, search_budget_sec);
        if should_accept_sa(&candidate, &current, temp, &mut rng) {
            stats.accepted_total += 1;
            if delta < 0 {
                stats.accepted_better += 1;
            } else if delta == 0 {
                stats.accepted_equal += 1;
            } else {
                stats.accepted_worse += 1;
            }
            current = candidate;
        }
        if is_better_state(&current, &best) {
            best = current.clone();
            stats.best_updates += 1;
        }
    }
    let _ = time_keeper.is_over();

    stats.final_current_cost = current.eval.cost;
    stats.final_best_cost = best.eval.cost;
    stats.final_best_patterns = best.patterns.len();
    stats.elapsed_ms = time_keeper.start.elapsed().as_millis() as usize;
    stats
}

fn temperature(time_keeper: &TimeKeeper, search_budget_sec: f64) -> f64 {
    if search_budget_sec <= 0.0 {
        return SA_END_TEMP;
    }
    let elapsed = time_keeper.start.elapsed().as_secs_f64();
    let progress = (elapsed / search_budget_sec).clamp(0.0, 1.0);
    SA_START_TEMP.powf(1.0 - progress) * SA_END_TEMP.powf(progress)
}

fn seed_from_input(input: &Input) -> u64 {
    let mut seed = 1469598103934665603_u64;
    seed ^= input.k_layers as u64;
    seed = seed.wrapping_mul(1099511628211_u64);
    seed ^= input.color_count as u64;
    seed = seed.wrapping_mul(1099511628211_u64);
    for row in &input.goal {
        for &color in row {
            seed ^= color as u64 + 1;
            seed = seed.wrapping_mul(1099511628211_u64);
        }
    }
    seed
}

fn is_better_state(candidate: &SearchState, current: &SearchState) -> bool {
    if candidate.eval.cost != current.eval.cost {
        return candidate.eval.cost < current.eval.cost;
    }
    if candidate.patterns.len() != current.patterns.len() {
        return candidate.patterns.len() < current.patterns.len();
    }
    candidate.eval.total_pattern_cells < current.eval.total_pattern_cells
}

fn should_accept_sa(
    candidate: &SearchState,
    current: &SearchState,
    temp: f64,
    rng: &mut Xoshiro256PlusPlus,
) -> bool {
    let delta = candidate.eval.cost as isize - current.eval.cost as isize;
    if delta <= 0 {
        return true;
    }
    rng.random::<f64>() < (-(delta as f64) / temp).exp()
}

fn build_search_state_full(
    patterns: Vec<EditablePattern>,
    input: &Input,
    compiled_cache: &mut FxHashMap<EditablePattern, Rc<CompiledPattern>>,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<SearchState> {
    let started = Instant::now();
    stats.build_patterns_total += patterns.len();
    let mut normalized_patterns = Vec::new();
    let mut compiled = Vec::new();
    for pattern in patterns {
        if !time_keeper.step() || !time_keeper.has_time_left(BUILD_MARGIN_SEC) {
            return None;
        }
        if let Some(normalized) = normalize_editable_pattern(&pattern, &input.goal) {
            let compiled_pattern =
                get_or_compile_pattern(&normalized, input, compiled_cache, time_keeper, stats)?;
            normalized_patterns.push(normalized);
            compiled.push(compiled_pattern);
        }
    }
    let eval = plan_patterns(input, &compiled, time_keeper, stats)?;
    let result = Some(SearchState {
        patterns: normalized_patterns,
        compiled,
        eval,
    });
    ProbeStats::add(
        &mut stats.build_state_total_sec,
        &mut stats.build_state_calls,
        &mut stats.build_state_max_sec,
        started.elapsed().as_secs_f64(),
    );
    result
}

fn apply_neighbor_edit(
    current: &SearchState,
    edit: NeighborEdit,
    input: &Input,
    compiled_cache: &mut FxHashMap<EditablePattern, Rc<CompiledPattern>>,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<SearchState> {
    let started = Instant::now();
    if !time_keeper.has_time_left(BUILD_MARGIN_SEC) {
        return None;
    }

    let result = match edit {
        NeighborEdit::None => Some(current.clone()),
        NeighborEdit::Add(pattern) => {
            let Some(normalized) = normalize_editable_pattern(&pattern, &input.goal) else {
                return Some(current.clone());
            };
            let compiled_pattern =
                get_or_compile_pattern(&normalized, input, compiled_cache, time_keeper, stats)?;
            let mut patterns = current.patterns.clone();
            let mut compiled = current.compiled.clone();
            patterns.push(normalized);
            compiled.push(compiled_pattern);
            stats.build_patterns_total += patterns.len();
            let eval = plan_patterns(input, &compiled, time_keeper, stats)?;
            Some(SearchState {
                patterns,
                compiled,
                eval,
            })
        }
        NeighborEdit::Drop { idx } => {
            if idx >= current.patterns.len() {
                return Some(current.clone());
            }
            let mut patterns = current.patterns.clone();
            let mut compiled = current.compiled.clone();
            patterns.swap_remove(idx);
            compiled.swap_remove(idx);
            stats.build_patterns_total += patterns.len();
            let eval = plan_patterns(input, &compiled, time_keeper, stats)?;
            Some(SearchState {
                patterns,
                compiled,
                eval,
            })
        }
        NeighborEdit::Replace { idx, pattern } => {
            if idx >= current.patterns.len() {
                return Some(current.clone());
            }
            let Some(normalized) = normalize_editable_pattern(&pattern, &input.goal) else {
                let mut patterns = current.patterns.clone();
                let mut compiled = current.compiled.clone();
                patterns.swap_remove(idx);
                compiled.swap_remove(idx);
                let eval = plan_patterns(input, &compiled, time_keeper, stats)?;
                return Some(SearchState {
                    patterns,
                    compiled,
                    eval,
                });
            };
            if normalized == current.patterns[idx] {
                return Some(current.clone());
            }
            let compiled_pattern =
                get_or_compile_pattern(&normalized, input, compiled_cache, time_keeper, stats)?;
            let mut patterns = current.patterns.clone();
            let mut compiled = current.compiled.clone();
            patterns[idx] = normalized;
            compiled[idx] = compiled_pattern;
            stats.build_patterns_total += patterns.len();
            let eval = plan_patterns(input, &compiled, time_keeper, stats)?;
            Some(SearchState {
                patterns,
                compiled,
                eval,
            })
        }
    };
    ProbeStats::add(
        &mut stats.apply_edit_total_sec,
        &mut stats.apply_edit_calls,
        &mut stats.apply_edit_max_sec,
        started.elapsed().as_secs_f64(),
    );
    result
}

fn get_or_compile_pattern(
    normalized: &EditablePattern,
    input: &Input,
    compiled_cache: &mut FxHashMap<EditablePattern, Rc<CompiledPattern>>,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<Rc<CompiledPattern>> {
    if let Some(compiled) = compiled_cache.get(normalized) {
        stats.cache_hits += 1;
        return Some(compiled.clone());
    }
    stats.cache_misses += 1;
    let compiled = Rc::new(compile_pattern(normalized, input, time_keeper, stats)?);
    compiled_cache.insert(normalized.clone(), compiled.clone());
    Some(compiled)
}

fn normalize_editable_pattern(pattern: &EditablePattern, goal: &Grid) -> Option<EditablePattern> {
    let mut min_i = pattern.height;
    let mut max_i = 0usize;
    let mut min_j = pattern.width;
    let mut max_j = 0usize;
    let mut found = false;

    for i in 0..pattern.height {
        for j in 0..pattern.width {
            if pattern.mask[pattern.idx(i, j)] == 0 {
                continue;
            }
            let bi = pattern.top + i as isize;
            let bj = pattern.left + j as isize;
            if !(0..N as isize).contains(&bi) || !(0..N as isize).contains(&bj) {
                continue;
            }
            if goal[bi as usize][bj as usize] == 0 {
                continue;
            }
            min_i = min_i.min(i);
            max_i = max_i.max(i);
            min_j = min_j.min(j);
            max_j = max_j.max(j);
            found = true;
        }
    }

    if !found {
        return None;
    }

    let new_height = max_i - min_i + 1;
    let new_width = max_j - min_j + 1;
    let mut mask = vec![0_u8; new_height * new_width];
    for i in 0..new_height {
        for j in 0..new_width {
            mask[i * new_width + j] = pattern.mask[pattern.idx(min_i + i, min_j + j)];
        }
    }

    Some(EditablePattern {
        top: pattern.top + min_i as isize,
        left: pattern.left + min_j as isize,
        height: new_height,
        width: new_width,
        mask,
    })
}

fn compile_pattern(
    editable: &EditablePattern,
    input: &Input,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<CompiledPattern> {
    let started = Instant::now();
    if !time_keeper.has_time_left(BUILD_MARGIN_SEC) {
        return None;
    }
    let mut cells = Vec::new();
    for i in 0..editable.height {
        if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
            return None;
        }
        for j in 0..editable.width {
            if editable.mask[editable.idx(i, j)] == 0 {
                continue;
            }
            let bi = editable.top + i as isize;
            let bj = editable.left + j as isize;
            if !(0..N as isize).contains(&bi) || !(0..N as isize).contains(&bj) {
                continue;
            }
            let color = input.goal[bi as usize][bj as usize];
            if color == 0 {
                continue;
            }
            cells.push(Cell { i, j, color });
        }
    }
    cells.sort();

    let pattern = Pattern {
        height: editable.height,
        width: editable.width,
        cells,
    };
    let rotations = build_rotations(&pattern);
    let occurrences = enumerate_occurrences(input, &rotations, time_keeper, stats)?;
    let build_cost = pattern.cells.len();

    let result = Some(CompiledPattern {
        pattern,
        rotations,
        occurrences,
        build_cost,
    });
    ProbeStats::add(
        &mut stats.compile_total_sec,
        &mut stats.compile_calls,
        &mut stats.compile_max_sec,
        started.elapsed().as_secs_f64(),
    );
    result
}

fn build_rotations(pattern: &Pattern) -> Vec<RotatedView> {
    let mut rotations = Vec::with_capacity(4);
    for rot in 0..4 {
        let (height, width) = rotated_dims(pattern.height, pattern.width, rot);
        let mut min_board_i = N;
        let mut min_board_j = N;
        let mut cells = Vec::with_capacity(pattern.cells.len());
        let mut row_any_masks = vec![0_u32; height];
        let mut row_color_masks = vec![[0_u32; MAX_COLOR + 1]; height];
        for cell in &pattern.cells {
            let (ri, rj) = rotate_local_coord((cell.i, cell.j), pattern.height, pattern.width, rot);
            cells.push(Cell {
                i: ri,
                j: rj,
                color: cell.color,
            });
            row_any_masks[ri] |= 1_u32 << rj;
            row_color_masks[ri][cell.color as usize] |= 1_u32 << rj;
            let (bi, bj) = rotate_coord((cell.i, cell.j), rot);
            min_board_i = min_board_i.min(bi);
            min_board_j = min_board_j.min(bj);
        }
        cells.sort();
        rotations.push(RotatedView {
            rot,
            height,
            width,
            min_board_i,
            min_board_j,
            cells,
            row_any_masks,
            row_color_masks,
        });
    }
    rotations
}

fn enumerate_occurrences(
    input: &Input,
    rotations: &[RotatedView],
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<Vec<Occurrence>> {
    let started = Instant::now();
    let mut top_occurrences =
        BinaryHeap::<Reverse<((isize, usize, usize, usize, usize, usize), OccurrenceSeed)>>::new();
    for view in rotations {
        if view.height > N || view.width > N {
            continue;
        }
        for top in 0..=N - view.height {
            for left in 0..=N - view.width {
                if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
                    return None;
                }
                let mut fix_count = 0;
                let mut break_count = 0;
                for row in 0..view.height {
                    let placed_any = view.row_any_masks[row] << left;
                    let board_row = top + row;
                    let mut fixed_bits = 0_u32;
                    for color in 1..=input.color_count {
                        fixed_bits |= (view.row_color_masks[row][color] << left)
                            & input.goal_rows[color][board_row];
                    }
                    fix_count += fixed_bits.count_ones() as usize;
                    break_count += (placed_any & !fixed_bits).count_ones() as usize;
                }
                if fix_count <= break_count {
                    continue;
                }
                let seed = OccurrenceSeed {
                    rot: view.rot,
                    top,
                    left,
                    fix_count,
                    break_count,
                };
                let rank = (
                    fix_count as isize - break_count as isize,
                    fix_count,
                    usize::MAX - break_count,
                    usize::MAX - view.rot,
                    usize::MAX - top,
                    usize::MAX - left,
                );
                let entry = Reverse((rank, seed));
                if top_occurrences.len() < MAX_OCCURRENCES_PER_PATTERN {
                    top_occurrences.push(entry);
                } else if let Some(worst) = top_occurrences.peek() {
                    if (rank, seed) > worst.0 {
                        top_occurrences.pop();
                        top_occurrences.push(entry);
                    }
                }
            }
        }
    }

    let mut ranked = top_occurrences.into_iter().map(|entry| entry.0).collect::<Vec<_>>();
    ranked.sort_by(|lhs, rhs| rhs.cmp(lhs));

    let mut occurrences = Vec::with_capacity(ranked.len());
    for (_, seed) in ranked {
        if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
            return None;
        }
        let view = &rotations[seed.rot];
        let mut fix_mask = Bits::default();
        let mut break_mask = Bits::default();
        for cell in &view.cells {
            let bi = seed.top + cell.i;
            let bj = seed.left + cell.j;
            if input.goal[bi][bj] == cell.color {
                fix_mask.set(bi * N + bj);
            } else {
                break_mask.set(bi * N + bj);
            }
        }
        occurrences.push(Occurrence {
            rot: seed.rot,
            top: seed.top,
            left: seed.left,
            fix_mask,
            break_mask,
            break_count: seed.break_count,
        });
    }
    let result = Some(occurrences);
    ProbeStats::add(
        &mut stats.enumerate_total_sec,
        &mut stats.enumerate_calls,
        &mut stats.enumerate_max_sec,
        started.elapsed().as_secs_f64(),
    );
    result
}

fn plan_patterns(
    input: &Input,
    patterns: &[Rc<CompiledPattern>],
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<EvalResult> {
    let started = Instant::now();
    if !time_keeper.has_time_left(BUILD_MARGIN_SEC) {
        return None;
    }
    let mut residual = initial_mismatch_bits(&input.goal);
    let mut residual_count = residual.count();
    let mut loaded = None;
    let mut steps = Vec::new();
    let mut action_cost = 0usize;

    for _ in 0..MAX_BATCH_STEPS {
        if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
            return None;
        }
        let mut best_pattern = None;
        let mut best_batch = None::<BatchEval>;
        for (idx, pattern) in patterns.iter().enumerate() {
            let Some(batch) =
                best_batch_for_pattern(pattern, idx, &residual, loaded, time_keeper)
            else {
                if time_keeper.is_over() {
                    return None;
                }
                continue;
            };
            let take = match &best_batch {
                None => true,
                Some(prev) => {
                    batch.total_gain > prev.total_gain
                        || (batch.total_gain == prev.total_gain
                            && batch.residual_count < prev.residual_count)
                        || (batch.total_gain == prev.total_gain
                            && batch.residual_count == prev.residual_count
                            && batch.occurrence_indices.len() < prev.occurrence_indices.len())
                }
            };
            if take {
                best_pattern = Some(idx);
                best_batch = Some(batch);
            }
        }

        let (pattern_idx, batch) = match (best_pattern, best_batch) {
            (Some(idx), Some(batch)) => (idx, batch),
            _ => break,
        };

        let load_cost = if loaded == Some(pattern_idx) {
            0
        } else {
            patterns[pattern_idx].build_cost + usize::from(loaded.is_some())
        };
        action_cost += load_cost + batch.occurrence_indices.len();
        residual = batch.residual_after;
        residual_count = batch.residual_count;
        steps.push(BatchPlan {
            pattern_idx,
            occurrence_indices: batch.occurrence_indices,
        });
        loaded = Some(pattern_idx);
    }

    let result = Some(EvalResult {
        cost: action_cost + residual_count,
        residual,
        steps,
        total_pattern_cells: patterns.iter().map(|pattern| pattern.build_cost).sum(),
    });
    ProbeStats::add(
        &mut stats.plan_total_sec,
        &mut stats.plan_calls,
        &mut stats.plan_max_sec,
        started.elapsed().as_secs_f64(),
    );
    result
}

fn best_batch_for_pattern(
    pattern: &CompiledPattern,
    pattern_idx: usize,
    residual: &Bits,
    loaded: Option<usize>,
    time_keeper: &mut TimeKeeper,
) -> Option<BatchEval> {
    if pattern.build_cost == 0 || pattern.occurrences.is_empty() {
        return None;
    }
    if !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
        return None;
    }

    let load_cost = if loaded == Some(pattern_idx) {
        0
    } else {
        pattern.build_cost + usize::from(loaded.is_some())
    };

    let mut remaining = *residual;
    let initial_count = residual.count();
    let mut selected = Vec::new();
    for (idx, occ) in pattern.occurrences.iter().enumerate() {
        if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
            return None;
        }
        let delta = occurrence_gain(&remaining, occ);
        if delta <= 0 {
            continue;
        }
        selected.push(idx);
        apply_occurrence(&mut remaining, occ);
        if selected.len() >= MAX_BATCH_OCCURRENCES {
            break;
        }
    }

    if selected.is_empty() {
        return None;
    }

    let residual_count = remaining.count();
    let total_gain = initial_count as isize - residual_count as isize - load_cost as isize;
    if total_gain <= 0 {
        return None;
    }

    Some(BatchEval {
        occurrence_indices: selected,
        residual_after: remaining,
        residual_count,
        total_gain,
    })
}

fn occurrence_gain(residual: &Bits, occ: &Occurrence) -> isize {
    let fixed = residual.and_count(&occ.fix_mask) as isize;
    let broken = (occ.break_count - residual.and_count(&occ.break_mask)) as isize;
    fixed - broken - 1
}

fn apply_occurrence(residual: &mut Bits, occ: &Occurrence) {
    residual.minus_assign(&occ.fix_mask);
    residual.or_assign(&occ.break_mask);
}

fn initial_mismatch_bits(goal: &Grid) -> Bits {
    let mut bits = Bits::default();
    for i in 0..N {
        for j in 0..N {
            if goal[i][j] != 0 {
                bits.set(i * N + j);
            }
        }
    }
    bits
}

fn state_to_ops(input: &Input, patterns: &[Rc<CompiledPattern>], eval: &EvalResult) -> Vec<Op> {
    let mut ops = Vec::with_capacity(eval.cost);
    let mut loaded = None;

    for step in &eval.steps {
        if loaded != Some(step.pattern_idx) {
            if loaded.is_some() {
                ops.push(Op::Clear { k: PATTERN_LAYER });
            }
            for cell in &patterns[step.pattern_idx].pattern.cells {
                ops.push(Op::Paint {
                    k: PATTERN_LAYER,
                    i: cell.i,
                    j: cell.j,
                    color: cell.color,
                });
            }
            loaded = Some(step.pattern_idx);
        }

        for &occ_idx in &step.occurrence_indices {
            let occ = &patterns[step.pattern_idx].occurrences[occ_idx];
            let view = &patterns[step.pattern_idx].rotations[occ.rot];
            ops.push(Op::Copy {
                k: 0,
                h: PATTERN_LAYER,
                transform: Transform::new(
                    occ.rot,
                    occ.top as isize - view.min_board_i as isize,
                    occ.left as isize - view.min_board_j as isize,
                ),
            });
        }
    }

    for p in eval.residual.positions() {
        let i = p / N;
        let j = p % N;
        ops.push(Op::Paint {
            k: 0,
            i,
            j,
            color: input.goal[i][j],
        });
    }

    ops
}

fn baseline_paint_ops(goal: &Grid) -> Vec<Op> {
    let mut ops = Vec::new();
    for i in 0..N {
        for j in 0..N {
            if goal[i][j] != 0 {
                ops.push(Op::Paint {
                    k: 0,
                    i,
                    j,
                    color: goal[i][j],
                });
            }
        }
    }
    ops
}

fn propose_neighbor(
    current: &SearchState,
    input: &Input,
    nonzero_positions: &[usize],
    rng: &mut Xoshiro256PlusPlus,
) -> NeighborEdit {
    if current.patterns.is_empty() {
        return sample_add_pattern(&current.eval.residual, input, nonzero_positions, rng)
            .map_or(NeighborEdit::None, NeighborEdit::Add);
    }

    let roll = rng.random_range(0..100_u32);
    if roll < 35 {
        let idx = rng.random_range(0..current.patterns.len());
        let mut pattern = current.patterns[idx].clone();
        flip_mask_cell(&mut pattern, rng);
        postprocess_pattern(pattern, input).map_or(NeighborEdit::Drop { idx }, |pattern| {
            NeighborEdit::Replace { idx, pattern }
        })
    } else if roll < 60 {
        let idx = rng.random_range(0..current.patterns.len());
        let mut pattern = current.patterns[idx].clone();
        shift_window(&mut pattern, rng);
        postprocess_pattern(pattern, input).map_or(NeighborEdit::Drop { idx }, |pattern| {
            NeighborEdit::Replace { idx, pattern }
        })
    } else if roll < 75 {
        let idx = rng.random_range(0..current.patterns.len());
        let mut pattern = current.patterns[idx].clone();
        resize_side(&mut pattern, rng);
        postprocess_pattern(pattern, input).map_or(NeighborEdit::Drop { idx }, |pattern| {
            NeighborEdit::Replace { idx, pattern }
        })
    } else if roll < 95 {
        if current.patterns.len() < MAX_PATTERNS {
            if let Some(pattern) =
                sample_add_pattern(&current.eval.residual, input, nonzero_positions, rng)
            {
                NeighborEdit::Add(pattern)
            } else {
                NeighborEdit::None
            }
        } else {
            let idx = rng.random_range(0..current.patterns.len());
            let mut pattern = current.patterns[idx].clone();
            shift_window(&mut pattern, rng);
            postprocess_pattern(pattern, input).map_or(NeighborEdit::Drop { idx }, |pattern| {
                NeighborEdit::Replace { idx, pattern }
            })
        }
    } else {
        NeighborEdit::Drop {
            idx: rng.random_range(0..current.patterns.len()),
        }
    }
}

fn postprocess_pattern(pattern: EditablePattern, input: &Input) -> Option<EditablePattern> {
    normalize_editable_pattern(&pattern, &input.goal)
}

fn sample_add_pattern(
    residual: &Bits,
    input: &Input,
    nonzero_positions: &[usize],
    rng: &mut Xoshiro256PlusPlus,
) -> Option<EditablePattern> {
    let mut residual_nonzero = Vec::new();
    for &p in nonzero_positions {
        if residual.contains(p) {
            residual_nonzero.push(p);
        }
    }

    let seed_pool = if !residual_nonzero.is_empty() && rng.random_range(0..100_u32) < 85 {
        &residual_nonzero
    } else {
        nonzero_positions
    };
    if seed_pool.is_empty() {
        return None;
    }

    let p = seed_pool[rng.random_range(0..seed_pool.len())];
    let seed_i = p / N;
    let seed_j = p % N;
    let (height, width) = sample_window_size(rng);
    let offset_i = rng.random_range(0..height);
    let offset_j = rng.random_range(0..width);
    let pattern = EditablePattern {
        top: seed_i as isize - offset_i as isize,
        left: seed_j as isize - offset_j as isize,
        height,
        width,
        mask: vec![1_u8; height * width],
    };
    normalize_editable_pattern(&pattern, &input.goal)
}

fn sample_window_size(rng: &mut Xoshiro256PlusPlus) -> (usize, usize) {
    let range = match rng.random_range(0..100_u32) {
        0..=69 => (4, 16),
        70..=94 => (17, 36),
        _ => (37, 64),
    };
    for _ in 0..128 {
        let height = rng.random_range(1..=MAX_WINDOW_SIDE);
        let width = rng.random_range(1..=MAX_WINDOW_SIDE);
        let area = height * width;
        if range.0 <= area && area <= range.1 {
            return (height, width);
        }
    }
    (4, 4)
}

fn flip_mask_cell(pattern: &mut EditablePattern, rng: &mut Xoshiro256PlusPlus) {
    let i = rng.random_range(0..pattern.height);
    let j = rng.random_range(0..pattern.width);
    let idx = pattern.idx(i, j);
    pattern.mask[idx] ^= 1;
}

fn shift_window(pattern: &mut EditablePattern, rng: &mut Xoshiro256PlusPlus) {
    let dist = rng.random_range(1..=8) as isize;
    match rng.random_range(0..4_u32) {
        0 => pattern.top -= dist,
        1 => pattern.top += dist,
        2 => pattern.left -= dist,
        3 => pattern.left += dist,
        _ => unreachable!(),
    }
}

fn resize_side(pattern: &mut EditablePattern, rng: &mut Xoshiro256PlusPlus) {
    let side = rng.random_range(0..4_u32);
    let shrink = rng.random_bool(0.5);
    match side {
        0 => resize_top(pattern, shrink),
        1 => resize_bottom(pattern, shrink),
        2 => resize_left(pattern, shrink),
        3 => resize_right(pattern, shrink),
        _ => unreachable!(),
    }
}

fn resize_top(pattern: &mut EditablePattern, shrink: bool) {
    if shrink && pattern.height > 1 {
        pattern.top += 1;
        pattern.height -= 1;
        pattern.mask.drain(0..pattern.width);
        return;
    }

    let mut new_mask = vec![1_u8; pattern.width];
    new_mask.extend_from_slice(&pattern.mask);
    pattern.mask = new_mask;
    pattern.top -= 1;
    pattern.height += 1;
}

fn resize_bottom(pattern: &mut EditablePattern, shrink: bool) {
    if shrink && pattern.height > 1 {
        pattern.height -= 1;
        pattern.mask.truncate(pattern.height * pattern.width);
        return;
    }

    pattern.mask.extend(std::iter::repeat_n(1_u8, pattern.width));
    pattern.height += 1;
}

fn resize_left(pattern: &mut EditablePattern, shrink: bool) {
    if shrink && pattern.width > 1 {
        let mut new_mask = Vec::with_capacity(pattern.height * (pattern.width - 1));
        for i in 0..pattern.height {
            let row = &pattern.mask[i * pattern.width..(i + 1) * pattern.width];
            new_mask.extend_from_slice(&row[1..]);
        }
        pattern.mask = new_mask;
        pattern.left += 1;
        pattern.width -= 1;
        return;
    }

    let mut new_mask = Vec::with_capacity(pattern.height * (pattern.width + 1));
    for i in 0..pattern.height {
        new_mask.push(1);
        let row = &pattern.mask[i * pattern.width..(i + 1) * pattern.width];
        new_mask.extend_from_slice(row);
    }
    pattern.mask = new_mask;
    pattern.left -= 1;
    pattern.width += 1;
}

fn resize_right(pattern: &mut EditablePattern, shrink: bool) {
    if shrink && pattern.width > 1 {
        let mut new_mask = Vec::with_capacity(pattern.height * (pattern.width - 1));
        for i in 0..pattern.height {
            let row = &pattern.mask[i * pattern.width..(i + 1) * pattern.width];
            new_mask.extend_from_slice(&row[..pattern.width - 1]);
        }
        pattern.mask = new_mask;
        pattern.width -= 1;
        return;
    }

    let mut new_mask = Vec::with_capacity(pattern.height * (pattern.width + 1));
    for i in 0..pattern.height {
        let row = &pattern.mask[i * pattern.width..(i + 1) * pattern.width];
        new_mask.extend_from_slice(row);
        new_mask.push(1);
    }
    pattern.mask = new_mask;
    pattern.width += 1;
}
