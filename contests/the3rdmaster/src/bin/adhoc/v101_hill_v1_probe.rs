// v101_hill_v1.rs
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::fmt::Write as _;
use std::io::{self, Read};
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
const MAX_PATTERNS: usize = 24;
const MAX_OCCURRENCES_PER_PATTERN: usize = 768;
const MAX_BATCH_OCCURRENCES: usize = 96;
const MAX_BATCH_STEPS: usize = 64;
const MAX_WINDOW_SIDE: usize = 12;

type Color = u8;
type Grid = [[Color; N]; N];
type Coord = (usize, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Input {
    k_layers: usize,
    color_count: usize,
    goal: Grid,
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

        Self {
            k_layers,
            color_count,
            goal,
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

#[derive(Debug, Default, Clone)]
struct ProbeStats {
    search_iters: usize,
    neighbor_total_sec: f64,
    neighbor_max_sec: f64,
    neighbor_calls: usize,
    build_total_sec: f64,
    build_max_sec: f64,
    build_calls: usize,
    compile_total_sec: f64,
    compile_max_sec: f64,
    compile_calls: usize,
    enumerate_total_sec: f64,
    enumerate_max_sec: f64,
    enumerate_calls: usize,
    plan_total_sec: f64,
    plan_max_sec: f64,
    plan_calls: usize,
    batch_total_sec: f64,
    batch_max_sec: f64,
    batch_calls: usize,
    finalize_total_sec: f64,
    finalize_max_sec: f64,
    finalize_calls: usize,
    validate_total_sec: f64,
    validate_max_sec: f64,
    validate_calls: usize,
}

impl ProbeStats {
    fn add(total: &mut f64, max_value: &mut f64, calls: &mut usize, elapsed_sec: f64) {
        *total += elapsed_sec;
        if elapsed_sec > *max_value {
            *max_value = elapsed_sec;
        }
        *calls += 1;
    }

    fn record_neighbor(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.neighbor_total_sec,
            &mut self.neighbor_max_sec,
            &mut self.neighbor_calls,
            elapsed_sec,
        );
    }

    fn record_build(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.build_total_sec,
            &mut self.build_max_sec,
            &mut self.build_calls,
            elapsed_sec,
        );
    }

    fn record_compile(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.compile_total_sec,
            &mut self.compile_max_sec,
            &mut self.compile_calls,
            elapsed_sec,
        );
    }

    fn record_enumerate(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.enumerate_total_sec,
            &mut self.enumerate_max_sec,
            &mut self.enumerate_calls,
            elapsed_sec,
        );
    }

    fn record_plan(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.plan_total_sec,
            &mut self.plan_max_sec,
            &mut self.plan_calls,
            elapsed_sec,
        );
    }

    fn record_batch(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.batch_total_sec,
            &mut self.batch_max_sec,
            &mut self.batch_calls,
            elapsed_sec,
        );
    }

    fn record_finalize(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.finalize_total_sec,
            &mut self.finalize_max_sec,
            &mut self.finalize_calls,
            elapsed_sec,
        );
    }

    fn record_validate(&mut self, elapsed_sec: f64) {
        Self::add(
            &mut self.validate_total_sec,
            &mut self.validate_max_sec,
            &mut self.validate_calls,
            elapsed_sec,
        );
    }

    fn report(&self) -> String {
        format!(
            concat!(
                "probe search_iters={}\n",
                "neighbor: calls={} total_ms={:.1} max_ms={:.1}\n",
                "build_state: calls={} total_ms={:.1} max_ms={:.1}\n",
                "compile_pattern: calls={} total_ms={:.1} max_ms={:.1}\n",
                "enumerate_occ: calls={} total_ms={:.1} max_ms={:.1}\n",
                "plan_patterns: calls={} total_ms={:.1} max_ms={:.1}\n",
                "best_batch: calls={} total_ms={:.1} max_ms={:.1}\n",
                "finalize_ops: calls={} total_ms={:.1} max_ms={:.1}\n",
                "validate: calls={} total_ms={:.1} max_ms={:.1}"
            ),
            self.search_iters,
            self.neighbor_calls,
            self.neighbor_total_sec * 1000.0,
            self.neighbor_max_sec * 1000.0,
            self.build_calls,
            self.build_total_sec * 1000.0,
            self.build_max_sec * 1000.0,
            self.compile_calls,
            self.compile_total_sec * 1000.0,
            self.compile_max_sec * 1000.0,
            self.enumerate_calls,
            self.enumerate_total_sec * 1000.0,
            self.enumerate_max_sec * 1000.0,
            self.plan_calls,
            self.plan_total_sec * 1000.0,
            self.plan_max_sec * 1000.0,
            self.batch_calls,
            self.batch_total_sec * 1000.0,
            self.batch_max_sec * 1000.0,
            self.finalize_calls,
            self.finalize_total_sec * 1000.0,
            self.finalize_max_sec * 1000.0,
            self.validate_calls,
            self.validate_total_sec * 1000.0,
            self.validate_max_sec * 1000.0,
        )
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

#[derive(Debug, Clone)]
struct State {
    layers: Vec<Grid>,
    op_count: usize,
}

impl State {
    fn new(k_layers: usize) -> Self {
        Self {
            layers: vec![[[0; N]; N]; k_layers],
            op_count: 0,
        }
    }

    fn apply_all(&mut self, ops: &[Op]) -> Result<(), String> {
        for &op in ops {
            self.apply(op)?;
        }
        Ok(())
    }

    fn apply(&mut self, op: Op) -> Result<(), String> {
        if self.op_count >= N * N {
            return Err("too many operations".to_owned());
        }

        match op {
            Op::Paint { k, i, j, color } => {
                self.ensure_layer(k)?;
                self.layers[k][i][j] = color;
            }
            Op::Copy { k, h, transform } => {
                self.ensure_layer(k)?;
                self.ensure_layer(h)?;
                let src = self.layers[h];
                let mut dst = self.layers[k];
                for (i, row) in src.iter().enumerate() {
                    for (j, &color) in row.iter().enumerate() {
                        if color == 0 {
                            continue;
                        }
                        let (ri, rj) = rotate_coord((i, j), transform.rot);
                        let ni = ri as isize + transform.di;
                        let nj = rj as isize + transform.dj;
                        if !(0..N as isize).contains(&ni) || !(0..N as isize).contains(&nj) {
                            return Err(format!("copy out of bounds: ({ni}, {nj})"));
                        }
                        dst[ni as usize][nj as usize] = color;
                    }
                }
                self.layers[k] = dst;
            }
            Op::Clear { k } => {
                self.ensure_layer(k)?;
                self.layers[k] = [[0; N]; N];
            }
        }

        self.op_count += 1;
        Ok(())
    }

    fn ensure_layer(&self, k: usize) -> Result<(), String> {
        if k < self.layers.len() {
            Ok(())
        } else {
            Err(format!("invalid layer index: {k}"))
        }
    }

    fn layer0_matches(&self, goal: &Grid) -> bool {
        self.layers[0] == *goal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

#[derive(Debug, Clone)]
struct Occurrence {
    rot: usize,
    top: usize,
    left: usize,
    fix_mask: Bits,
    break_mask: Bits,
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
    compiled: Vec<CompiledPattern>,
    eval: EvalResult,
}

#[derive(Debug, Clone)]
struct BatchEval {
    occurrence_indices: Vec<usize>,
    residual_after: Bits,
    residual_count: usize,
    total_gain: isize,
}

fn main() {
    let input = Input::read();
    let (ops, stats) = solve(&input);
    eprintln!("{}", stats.report());
    print!("{}", format_ops(&ops));
}

fn solve(input: &Input) -> (Vec<Op>, ProbeStats) {
    let baseline_ops = baseline_paint_ops(&input.goal);
    let baseline_cost = baseline_ops.len();
    if input.k_layers < 2 {
        return (baseline_ops, ProbeStats::default());
    }

    let nonzero_positions = input.nonzero_goal_positions();
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_input(input));
    let mut time_keeper = TimeKeeper::new(SOLVER_TIME_LIMIT_SEC, 6);
    let mut stats = ProbeStats::default();

    let mut current = build_search_state(Vec::new(), input, &mut time_keeper, &mut stats)
        .expect("initial empty state should always fit in time");
    while time_keeper.step() {
        stats.search_iters += 1;
        if !time_keeper.has_time_left(SEARCH_WRAPUP_MARGIN_SEC) {
            break;
        }
        let neighbor_started = Instant::now();
        let next_patterns = propose_neighbor(&current, input, &nonzero_positions, &mut rng);
        stats.record_neighbor(neighbor_started.elapsed().as_secs_f64());
        let Some(candidate) = build_search_state(next_patterns, input, &mut time_keeper, &mut stats) else {
            break;
        };
        if is_better_state(&candidate, &current) {
            current = candidate;
        }
    }
    let _ = time_keeper.is_over();

    if current.eval.cost >= baseline_cost {
        return (baseline_ops, stats);
    }

    let finalize_started = Instant::now();
    let ops = state_to_ops(input, &current.compiled, &current.eval);
    stats.record_finalize(finalize_started.elapsed().as_secs_f64());
    if ops.len() >= baseline_cost || ops.len() > N * N {
        return (baseline_ops, stats);
    }

    let validate_started = Instant::now();
    let mut state = State::new(input.k_layers);
    let result = match state.apply_all(&ops) {
        Ok(()) if state.layer0_matches(&input.goal) => ops,
        _ => baseline_ops,
    };
    stats.record_validate(validate_started.elapsed().as_secs_f64());
    (result, stats)
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

fn build_search_state(
    patterns: Vec<EditablePattern>,
    input: &Input,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<SearchState> {
    let started = Instant::now();
    let mut normalized_patterns = Vec::new();
    let mut compiled = Vec::new();
    for pattern in patterns {
        if !time_keeper.step() || !time_keeper.has_time_left(BUILD_MARGIN_SEC) {
            return None;
        }
        if let Some(normalized) = normalize_editable_pattern(&pattern, &input.goal) {
            let compiled_pattern = compile_pattern(&normalized, &input.goal, time_keeper, stats)?;
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
    stats.record_build(started.elapsed().as_secs_f64());
    result
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
    goal: &Grid,
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
            let color = goal[bi as usize][bj as usize];
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
    let occurrences = enumerate_occurrences(goal, &pattern, &rotations, time_keeper, stats)?;
    let build_cost = pattern.cells.len();

    let result = Some(CompiledPattern {
        pattern,
        rotations,
        occurrences,
        build_cost,
    });
    stats.record_compile(started.elapsed().as_secs_f64());
    result
}

fn build_rotations(pattern: &Pattern) -> Vec<RotatedView> {
    let mut rotations = Vec::with_capacity(4);
    for rot in 0..4 {
        let (height, width) = rotated_dims(pattern.height, pattern.width, rot);
        let mut min_board_i = N;
        let mut min_board_j = N;
        let mut cells = Vec::with_capacity(pattern.cells.len());
        for cell in &pattern.cells {
            let (ri, rj) = rotate_local_coord((cell.i, cell.j), pattern.height, pattern.width, rot);
            cells.push(Cell {
                i: ri,
                j: rj,
                color: cell.color,
            });
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
        });
    }
    rotations
}

fn enumerate_occurrences(
    goal: &Grid,
    pattern: &Pattern,
    rotations: &[RotatedView],
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<Vec<Occurrence>> {
    let started = Instant::now();
    let mut occurrences = Vec::new();
    for view in rotations {
        if view.height > N || view.width > N {
            continue;
        }
        for top in 0..=N - view.height {
            for left in 0..=N - view.width {
                if !time_keeper.step() || !time_keeper.has_time_left(INNER_LOOP_MARGIN_SEC) {
                    return None;
                }
                let mut fix_mask = Bits::default();
                let mut break_mask = Bits::default();
                let mut fix_count = 0usize;
                let mut break_count = 0usize;
                for cell in &view.cells {
                    let bi = top + cell.i;
                    let bj = left + cell.j;
                    if goal[bi][bj] == cell.color {
                        fix_mask.set(bi * N + bj);
                        fix_count += 1;
                    } else {
                        break_mask.set(bi * N + bj);
                        break_count += 1;
                    }
                }
                if fix_count <= break_count {
                    continue;
                }
                occurrences.push(Occurrence {
                    rot: view.rot,
                    top,
                    left,
                    fix_mask,
                    break_mask,
                    fix_count,
                    break_count,
                });
            }
        }
    }

    occurrences.sort_by_key(|occ| {
        std::cmp::Reverse((
            occ.fix_count as isize - occ.break_count as isize,
            occ.fix_count,
            usize::MAX - occ.break_count,
        ))
    });
    if occurrences.len() > MAX_OCCURRENCES_PER_PATTERN {
        occurrences.truncate(MAX_OCCURRENCES_PER_PATTERN);
    }
    let _ = pattern;
    let result = Some(occurrences);
    stats.record_enumerate(started.elapsed().as_secs_f64());
    result
}

fn plan_patterns(
    input: &Input,
    patterns: &[CompiledPattern],
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
                best_batch_for_pattern(pattern, idx, &residual, loaded, time_keeper, stats)
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
    stats.record_plan(started.elapsed().as_secs_f64());
    result
}

fn best_batch_for_pattern(
    pattern: &CompiledPattern,
    pattern_idx: usize,
    residual: &Bits,
    loaded: Option<usize>,
    time_keeper: &mut TimeKeeper,
    stats: &mut ProbeStats,
) -> Option<BatchEval> {
    let started = Instant::now();
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

    let result = Some(BatchEval {
        occurrence_indices: selected,
        residual_after: remaining,
        residual_count,
        total_gain,
    });
    stats.record_batch(started.elapsed().as_secs_f64());
    result
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

fn state_to_ops(input: &Input, patterns: &[CompiledPattern], eval: &EvalResult) -> Vec<Op> {
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
) -> Vec<EditablePattern> {
    if current.patterns.is_empty() {
        let mut patterns = Vec::new();
        if let Some(pattern) = sample_add_pattern(&current.eval.residual, input, nonzero_positions, rng) {
            patterns.push(pattern);
        }
        return patterns;
    }

    let mut patterns = current.patterns.clone();
    let roll = rng.random_range(0..100_u32);
    if roll < 35 {
        let idx = rng.random_range(0..patterns.len());
        flip_mask_cell(&mut patterns[idx], rng);
        postprocess_pattern(&mut patterns, idx, input);
    } else if roll < 60 {
        let idx = rng.random_range(0..patterns.len());
        shift_window(&mut patterns[idx], rng);
        postprocess_pattern(&mut patterns, idx, input);
    } else if roll < 75 {
        let idx = rng.random_range(0..patterns.len());
        resize_side(&mut patterns[idx], rng);
        postprocess_pattern(&mut patterns, idx, input);
    } else if roll < 95 {
        if patterns.len() < MAX_PATTERNS {
            if let Some(pattern) =
                sample_add_pattern(&current.eval.residual, input, nonzero_positions, rng)
            {
                patterns.push(pattern);
            }
        } else {
            let idx = rng.random_range(0..patterns.len());
            shift_window(&mut patterns[idx], rng);
            postprocess_pattern(&mut patterns, idx, input);
        }
    } else {
        let idx = rng.random_range(0..patterns.len());
        patterns.swap_remove(idx);
    }
    patterns
}

fn postprocess_pattern(patterns: &mut Vec<EditablePattern>, idx: usize, input: &Input) {
    if idx >= patterns.len() {
        return;
    }
    if let Some(normalized) = normalize_editable_pattern(&patterns[idx], &input.goal) {
        patterns[idx] = normalized;
    } else {
        patterns.swap_remove(idx);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_empty_pattern() {
        let goal = [[0_u8; N]; N];
        let pattern = EditablePattern {
            top: 0,
            left: 0,
            height: 2,
            width: 2,
            mask: vec![1, 1, 1, 1],
        };
        assert!(normalize_editable_pattern(&pattern, &goal).is_none());
    }

    #[test]
    fn resize_left_expand_then_shrink_round_trip() {
        let mut pattern = EditablePattern {
            top: 3,
            left: 4,
            height: 2,
            width: 2,
            mask: vec![1, 0, 0, 1],
        };
        resize_left(&mut pattern, false);
        assert_eq!(pattern.left, 3);
        assert_eq!(pattern.width, 3);
        resize_left(&mut pattern, true);
        assert_eq!(pattern.left, 4);
        assert_eq!(pattern.width, 2);
    }

    #[test]
    fn planner_never_beats_baseline_on_empty_patterns() {
        let mut goal = [[0_u8; N]; N];
        goal[0][0] = 1;
        goal[0][1] = 1;
        let input = Input {
            k_layers: 2,
            color_count: 1,
            goal,
        };
        let mut time_keeper = TimeKeeper::new(1.0, 0);
        let eval = plan_patterns(&input, &[], &mut time_keeper).unwrap();
        assert_eq!(eval.cost, 2);
        assert_eq!(eval.residual.count(), 2);
    }
}
