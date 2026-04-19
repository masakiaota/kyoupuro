// v201pro_sa.rs
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use rustc_hash::FxHashMap;
use std::fmt::Write as _;
use std::io::{self, Read};
use std::rc::Rc;
use std::str::{FromStr, SplitAsciiWhitespace};
use std::time::Instant;

const N: usize = 32;
const M: usize = N * N;
const WORDS: usize = M / 64;
const PATTERN_LAYER: usize = 1;
const DIR4: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

const MIN_PATTERN_SIZE: usize = 6;
const MAX_OCC_BAD: usize = 4;
const MAX_OCC_BAD_RATIO_NUM: usize = 1;
const MAX_OCC_BAD_RATIO_DEN: usize = 5;
const MIN_OCC_GAIN: isize = 2;
const MAX_OCC_STORE: usize = 256;
const MAX_SCORE_OCCS: usize = 128;
const MAX_SEED_OCCS: usize = 6;
const MAX_POOL_PATTERNS: usize = 160;
const MAX_ACTIVE_PATTERNS: usize = 12;
const MAX_INIT_SCAN: usize = 64;
const MAX_RANDOM_SIDE: usize = 12;
const TIME_LIMIT_SEC: f64 = 1.80;

type Color = u8;
type Grid = [[Color; N]; N];

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
struct Timer {
    start: Instant,
}

impl Timer {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[inline]
    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline]
    fn over(&self) -> bool {
        self.elapsed() >= TIME_LIMIT_SEC
    }
}

#[derive(Debug, Clone)]
struct FastRng {
    inner: Xoshiro256PlusPlus,
}

impl FastRng {
    fn new(seed: u64) -> Self {
        Self {
            inner: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    #[inline]
    fn next_double(&mut self) -> f64 {
        self.inner.random()
    }

    #[inline]
    fn chance(&mut self, p: f64) -> bool {
        self.next_double() < p
    }

    #[inline]
    fn next_index(&mut self, upper: usize) -> usize {
        self.inner.random_range(0..upper)
    }

    #[inline]
    fn next_usize_inclusive(&mut self, low: usize, high: usize) -> usize {
        self.inner.random_range(low..=high)
    }

    #[inline]
    fn next_i32_inclusive(&mut self, low: i32, high: i32) -> i32 {
        self.inner.random_range(low..=high)
    }
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
                let pos = (block << 6) + tz;
                if pos < M {
                    res.push(pos);
                }
                bits &= bits - 1;
            }
        }
        res
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
        rot: usize,
        di: isize,
        dj: isize,
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
            Self::Copy { k, h, rot, di, dj } => {
                let _ = writeln!(out, "1 {k} {h} {rot} {di} {dj}");
            }
            Self::Clear { k } => {
                let _ = writeln!(out, "2 {k}");
            }
        }
    }
}

#[derive(Debug, Clone)]
struct SimState {
    layers: Vec<Grid>,
    op_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cell {
    i: u8,
    j: u8,
    color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pattern {
    height: usize,
    width: usize,
    hint_score: i32,
    cells: Vec<Cell>,
    signature: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatternSpec {
    top: usize,
    left: usize,
    height: usize,
    width: usize,
    mask: Vec<u8>,
}

#[derive(Debug, Clone)]
struct RotView {
    rot: usize,
    height: usize,
    width: usize,
    min_board_i: usize,
    min_board_j: usize,
    local_coords: Vec<(u8, u8)>,
}

#[derive(Debug, Clone)]
struct Occurrence {
    rot: u8,
    top: u8,
    left: u8,
    good_mask: Bits,
    bad_mask: Bits,
    good_count: u16,
    bad_count: u16,
}

#[derive(Debug, Clone)]
struct Candidate {
    pattern: Pattern,
    rotations: Vec<RotView>,
    occs: Vec<Occurrence>,
    good_union: Bits,
}

#[derive(Debug, Clone)]
struct PatternData {
    spec: PatternSpec,
    cand: Candidate,
}

#[derive(Debug, Clone)]
struct SeedStat {
    spec: PatternSpec,
    canonical: Pattern,
    hits: i32,
    total_component_size: i32,
}

#[derive(Debug, Clone)]
struct BatchStep {
    pat_idx: usize,
    occ_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct EvalResult {
    total_cost: usize,
    residual: Bits,
    residual_count: usize,
    steps: Vec<BatchStep>,
}

#[derive(Debug, Clone)]
struct SearchState {
    pats: Vec<Rc<PatternData>>,
    cost: usize,
    residual: Bits,
    residual_count: usize,
}

#[derive(Debug, Clone)]
struct BatchChoice {
    valid: bool,
    gain: isize,
    transition_cost: usize,
    next_residual: Bits,
    next_residual_count: usize,
    occ_indices: Vec<usize>,
}

impl Default for BatchChoice {
    fn default() -> Self {
        Self {
            valid: false,
            gain: isize::MIN,
            transition_cost: 0,
            next_residual: Bits::default(),
            next_residual_count: usize::MAX,
            occ_indices: Vec::new(),
        }
    }
}

#[inline]
fn rotate_coord_board(i: usize, j: usize, rot: usize) -> (usize, usize) {
    match rot & 3 {
        0 => (i, j),
        1 => (j, N - 1 - i),
        2 => (N - 1 - i, N - 1 - j),
        _ => (N - 1 - j, i),
    }
}

#[inline]
fn rotated_dims(height: usize, width: usize, rot: usize) -> (usize, usize) {
    if (rot & 1) == 0 {
        (height, width)
    } else {
        (width, height)
    }
}

#[inline]
fn rotate_local_coord(
    i: usize,
    j: usize,
    height: usize,
    width: usize,
    rot: usize,
) -> (usize, usize) {
    match rot & 3 {
        0 => (i, j),
        1 => (j, height - 1 - i),
        2 => (height - 1 - i, width - 1 - j),
        _ => (width - 1 - j, i),
    }
}

#[inline]
fn transformed_coord(
    i: usize,
    j: usize,
    rot: usize,
    di: isize,
    dj: isize,
) -> Option<(usize, usize)> {
    let (ri, rj) = rotate_coord_board(i, j, rot);
    let ni = ri as isize + di;
    let nj = rj as isize + dj;
    if (0..N as isize).contains(&ni) && (0..N as isize).contains(&nj) {
        Some((ni as usize, nj as usize))
    } else {
        None
    }
}

fn pattern_signature(pattern: &Pattern) -> Vec<u16> {
    let mut sig = Vec::with_capacity(pattern.cells.len() + 2);
    sig.push(pattern.height as u16);
    sig.push(pattern.width as u16);
    for cell in &pattern.cells {
        let packed = ((cell.i as u16) << 11) | ((cell.j as u16) << 3) | (cell.color as u16);
        sig.push(packed);
    }
    sig
}

fn rotate_pattern_cells(cells: &[Cell], height: usize, width: usize, rot: usize) -> Pattern {
    let (rh, rw) = rotated_dims(height, width, rot);
    let mut tmp = Vec::with_capacity(cells.len());
    for cell in cells {
        let (ni, nj) = rotate_local_coord(cell.i as usize, cell.j as usize, height, width, rot);
        tmp.push(Cell {
            i: ni as u8,
            j: nj as u8,
            color: cell.color,
        });
    }
    tmp.sort();
    let mut pattern = Pattern {
        height: rh,
        width: rw,
        hint_score: 0,
        cells: tmp,
        signature: Vec::new(),
    };
    pattern.signature = pattern_signature(&pattern);
    pattern
}

fn canonicalize_from_trimmed_cells(base_cells: &[Cell], height: usize, width: usize) -> Pattern {
    let mut best = rotate_pattern_cells(base_cells, height, width, 0);
    for rot in 1..4 {
        let cand = rotate_pattern_cells(base_cells, height, width, rot);
        if cand.signature < best.signature {
            best = cand;
        }
    }
    best
}

fn canonicalize_component(goal: &Grid, comp: &[(usize, usize)]) -> Pattern {
    let mut min_i = N;
    let mut min_j = N;
    let mut max_i = 0usize;
    let mut max_j = 0usize;
    for &(i, j) in comp {
        min_i = min_i.min(i);
        min_j = min_j.min(j);
        max_i = max_i.max(i);
        max_j = max_j.max(j);
    }
    let height = max_i - min_i + 1;
    let width = max_j - min_j + 1;
    let mut cells = Vec::with_capacity(comp.len());
    for &(i, j) in comp {
        cells.push(Cell {
            i: (i - min_i) as u8,
            j: (j - min_j) as u8,
            color: goal[i][j],
        });
    }
    cells.sort();
    canonicalize_from_trimmed_cells(&cells, height, width)
}

fn component_to_spec(comp: &[(usize, usize)]) -> PatternSpec {
    let mut min_i = N;
    let mut min_j = N;
    let mut max_i = 0usize;
    let mut max_j = 0usize;
    for &(i, j) in comp {
        min_i = min_i.min(i);
        min_j = min_j.min(j);
        max_i = max_i.max(i);
        max_j = max_j.max(j);
    }
    let height = max_i - min_i + 1;
    let width = max_j - min_j + 1;
    let mut mask = vec![0_u8; height * width];
    for &(i, j) in comp {
        mask[(i - min_i) * width + (j - min_j)] = 1;
    }
    PatternSpec {
        top: min_i,
        left: min_j,
        height,
        width,
        mask,
    }
}

#[inline]
fn non_residual_damage_count(residual: &Bits, bad_mask: &Bits, bad_count: usize) -> usize {
    bad_count - residual.and_count(bad_mask)
}

#[inline]
fn residual_delta(residual: &Bits, occ: &Occurrence) -> isize {
    let fix = residual.and_count(&occ.good_mask) as isize;
    let damage =
        non_residual_damage_count(residual, &occ.bad_mask, occ.bad_count as usize) as isize;
    fix - damage
}

#[inline]
fn apply_occurrence_to_residual(residual: &mut Bits, occ: &Occurrence) {
    residual.minus_assign(&occ.good_mask);
    residual.or_assign(&occ.bad_mask);
}

fn goal_nonzero_bits(goal: &Grid) -> Bits {
    let mut bits = Bits::default();
    for (i, row) in goal.iter().enumerate() {
        for (j, &color) in row.iter().enumerate() {
            if color != 0 {
                bits.set(i * N + j);
            }
        }
    }
    bits
}

#[inline]
fn same_pattern_signature(a: &Candidate, b: &Candidate) -> bool {
    a.pattern.signature == b.pattern.signature
}

fn sort_patterns(pats: &mut [Rc<PatternData>]) {
    pats.sort_by(|a, b| a.cand.pattern.signature.cmp(&b.cand.pattern.signature));
}

struct Solver {
    input: Input,
    timer: Timer,
    rng: FastRng,
    goal_nonzero: Bits,
    nonzero_positions: Vec<usize>,
    pool: Vec<Rc<PatternData>>,
    start_temp: f64,
    end_temp: f64,
}

impl Solver {
    fn new(input: Input) -> Self {
        let goal_nonzero = goal_nonzero_bits(&input.goal);
        let mut nonzero_positions = Vec::new();
        for i in 0..N {
            for j in 0..N {
                if input.goal[i][j] != 0 {
                    nonzero_positions.push(i * N + j);
                }
            }
        }
        let seed = seed_from_input(&input);
        Self {
            input,
            timer: Timer::new(),
            rng: FastRng::new(seed),
            goal_nonzero,
            nonzero_positions,
            pool: Vec::new(),
            start_temp: 6.0,
            end_temp: 0.12,
        }
    }

    fn solve(&mut self) -> Vec<Op> {
        let baseline = self.baseline_ops();
        let baseline_cost = baseline.len();
        if self.input.k_layers <= PATTERN_LAYER {
            return baseline;
        }

        self.build_seed_pool();
        let mut current = self.greedy_initial_state(baseline_cost);
        let mut best = current.clone();

        let mut stagnation = 0usize;
        while !self.timer.over() {
            let Some(next) = self.propose_state(&current) else {
                continue;
            };

            let progress = (self.timer.elapsed() / TIME_LIMIT_SEC).min(1.0);
            let temp = self.start_temp * (self.end_temp / self.start_temp).powf(progress);
            let accept = if Self::better(&next, &current) {
                true
            } else {
                let ecur = Self::energy(&current);
                let enxt = Self::energy(&next);
                let prob = ((ecur - enxt) / temp.max(1e-9)).exp();
                self.rng.next_double() < prob
            };
            if accept {
                current = next;
            }

            if Self::better(&current, &best) {
                best = current.clone();
                stagnation = 0;
            } else {
                stagnation += 1;
            }

            if stagnation > 200 && self.rng.chance(0.08) {
                current = best.clone();
                stagnation = 0;
            }
        }

        let plan = self.evaluate_state(&best.pats, true);
        let ops = self.state_to_ops(&best.pats, &plan);
        if ops.len() > baseline_cost || ops.len() > N * N || !self.verify_ops(&ops) {
            baseline
        } else {
            ops
        }
    }

    fn baseline_ops(&self) -> Vec<Op> {
        let mut ops = Vec::with_capacity(M);
        for i in 0..N {
            for j in 0..N {
                if self.input.goal[i][j] != 0 {
                    ops.push(Op::Paint {
                        k: 0,
                        i,
                        j,
                        color: self.input.goal[i][j],
                    });
                }
            }
        }
        ops
    }

    fn better(a: &SearchState, b: &SearchState) -> bool {
        if a.cost != b.cost {
            return a.cost < b.cost;
        }
        if a.residual_count != b.residual_count {
            return a.residual_count < b.residual_count;
        }
        a.pats.len() < b.pats.len()
    }

    fn energy(state: &SearchState) -> f64 {
        state.cost as f64 + 1e-3 * state.residual_count as f64 + 1e-4 * state.pats.len() as f64
    }

    fn build_seed_pool(&mut self) {
        let mut stats: FxHashMap<Vec<u16>, SeedStat> = FxHashMap::default();
        let mut matching = [[false; N]; N];
        let mut visited = [[false; N]; N];
        let mut q = Vec::with_capacity(M);
        let mut comp = Vec::with_capacity(M);
        let mut iter = 0usize;

        'extract: for rot in 0..4 {
            for di in -(N as isize) + 1..=(N as isize) - 1 {
                for dj in -(N as isize) + 1..=(N as isize) - 1 {
                    if rot == 0 && di == 0 && dj == 0 {
                        continue;
                    }
                    iter += 1;
                    if (iter & 255) == 0 && self.timer.elapsed() > TIME_LIMIT_SEC * 0.18 {
                        break 'extract;
                    }

                    let mut match_count = 0usize;
                    for i in 0..N {
                        for j in 0..N {
                            let ok = self.input.goal[i][j] != 0
                                && transformed_coord(i, j, rot, di, dj)
                                    .map(|(ni, nj)| {
                                        self.input.goal[ni][nj] == self.input.goal[i][j]
                                    })
                                    .unwrap_or(false);
                            matching[i][j] = ok;
                            visited[i][j] = false;
                            match_count += usize::from(ok);
                        }
                    }
                    if match_count < MIN_PATTERN_SIZE {
                        continue;
                    }

                    for si in 0..N {
                        for sj in 0..N {
                            if !matching[si][sj] || visited[si][sj] {
                                continue;
                            }
                            q.clear();
                            comp.clear();
                            q.push((si, sj));
                            visited[si][sj] = true;
                            let mut head = 0usize;
                            while head < q.len() {
                                let (i, j) = q[head];
                                head += 1;
                                comp.push((i, j));
                                for &(di2, dj2) in &DIR4 {
                                    let ni = i as isize + di2;
                                    let nj = j as isize + dj2;
                                    if !(0..N as isize).contains(&ni)
                                        || !(0..N as isize).contains(&nj)
                                    {
                                        continue;
                                    }
                                    let ni = ni as usize;
                                    let nj = nj as usize;
                                    if matching[ni][nj] && !visited[ni][nj] {
                                        visited[ni][nj] = true;
                                        q.push((ni, nj));
                                    }
                                }
                            }
                            if comp.len() < MIN_PATTERN_SIZE {
                                continue;
                            }

                            let pat = canonicalize_component(&self.input.goal, &comp);
                            let key = pat.signature.clone();
                            match stats.entry(key) {
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(SeedStat {
                                        spec: component_to_spec(&comp),
                                        canonical: pat,
                                        hits: 1,
                                        total_component_size: comp.len() as i32,
                                    });
                                }
                                std::collections::hash_map::Entry::Occupied(mut entry) => {
                                    let st = entry.get_mut();
                                    st.hits += 1;
                                    st.total_component_size += comp.len() as i32;
                                    if comp.len() > st.spec.mask.len() {
                                        st.spec = component_to_spec(&comp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut seeds = Vec::with_capacity(stats.len());
        for (_, st) in stats {
            let hint = st.total_component_size + st.hits * st.canonical.cells.len() as i32;
            seeds.push((hint, st.spec));
        }
        seeds.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.mask.len().cmp(&a.1.mask.len()))
        });

        self.pool.clear();
        self.pool.reserve(MAX_POOL_PATTERNS);
        for (hint, spec) in seeds {
            let Some(data) = self.build_pattern_data_from_spec(&spec, hint) else {
                continue;
            };
            if self
                .pool
                .iter()
                .any(|p| same_pattern_signature(&p.cand, &data.cand))
            {
                continue;
            }
            self.pool.push(Rc::new(data));
            if self.pool.len() >= MAX_POOL_PATTERNS {
                break;
            }
        }
    }

    fn build_pattern_data_from_spec(
        &self,
        orig: &PatternSpec,
        hint_score: i32,
    ) -> Option<PatternData> {
        let (normalized, selected) = self.normalize_spec(orig)?;
        if selected.len() < MIN_PATTERN_SIZE {
            return None;
        }

        let mut pattern =
            canonicalize_from_trimmed_cells(&selected, normalized.height, normalized.width);
        pattern.hint_score = hint_score;
        pattern.signature = pattern_signature(&pattern);

        let candidate = self.materialize_candidate(&pattern);
        if candidate.occs.len() < 2 {
            return None;
        }

        Some(PatternData {
            spec: normalized,
            cand: candidate,
        })
    }

    fn normalize_spec(&self, input_spec: &PatternSpec) -> Option<(PatternSpec, Vec<Cell>)> {
        let mut selected = Vec::new();
        let mut min_r = usize::MAX;
        let mut min_c = usize::MAX;
        let mut max_r = 0usize;
        let mut max_c = 0usize;

        for r in 0..input_spec.height {
            for c in 0..input_spec.width {
                if input_spec.mask[r * input_spec.width + c] == 0 {
                    continue;
                }
                let bi = input_spec.top + r;
                let bj = input_spec.left + c;
                if bi >= N || bj >= N {
                    continue;
                }
                let color = self.input.goal[bi][bj];
                if color == 0 {
                    continue;
                }
                min_r = min_r.min(r);
                min_c = min_c.min(c);
                max_r = max_r.max(r);
                max_c = max_c.max(c);
                selected.push(Cell {
                    i: r as u8,
                    j: c as u8,
                    color,
                });
            }
        }

        if selected.is_empty() {
            return None;
        }

        let nh = max_r - min_r + 1;
        let nw = max_c - min_c + 1;
        let mut mask = vec![0_u8; nh * nw];
        for cell in &mut selected {
            let rr = cell.i as usize - min_r;
            let cc = cell.j as usize - min_c;
            mask[rr * nw + cc] = 1;
            cell.i = rr as u8;
            cell.j = cc as u8;
        }
        selected.sort();
        Some((
            PatternSpec {
                top: input_spec.top + min_r,
                left: input_spec.left + min_c,
                height: nh,
                width: nw,
                mask,
            },
            selected,
        ))
    }

    fn materialize_candidate(&self, pattern: &Pattern) -> Candidate {
        let rotations = self.build_rotations(pattern);
        let mut occs = Vec::with_capacity(512);
        let mut good_union = Bits::default();

        for view in &rotations {
            for top in 0..=N - view.height {
                for left in 0..=N - view.width {
                    let mut good_mask = Bits::default();
                    let mut bad_mask = Bits::default();
                    let mut good_count = 0usize;
                    let mut bad_count = 0usize;
                    for (idx, cell) in pattern.cells.iter().enumerate() {
                        let (ri, rj) = view.local_coords[idx];
                        let bi = top + ri as usize;
                        let bj = left + rj as usize;
                        if self.input.goal[bi][bj] == cell.color {
                            good_mask.set(bi * N + bj);
                            good_count += 1;
                        } else {
                            bad_mask.set(bi * N + bj);
                            bad_count += 1;
                        }
                    }
                    if good_count < MIN_PATTERN_SIZE {
                        continue;
                    }
                    if bad_count > MAX_OCC_BAD {
                        continue;
                    }
                    if bad_count * MAX_OCC_BAD_RATIO_DEN
                        > pattern.cells.len() * MAX_OCC_BAD_RATIO_NUM
                    {
                        continue;
                    }
                    if good_count <= bad_count + 1 {
                        continue;
                    }
                    if (good_count as isize - bad_count as isize) < MIN_OCC_GAIN {
                        continue;
                    }

                    occs.push(Occurrence {
                        rot: view.rot as u8,
                        top: top as u8,
                        left: left as u8,
                        good_mask,
                        bad_mask,
                        good_count: good_count as u16,
                        bad_count: bad_count as u16,
                    });
                }
            }
        }

        occs.sort_by(|a, b| {
            let da = a.good_count as i32 - a.bad_count as i32;
            let db = b.good_count as i32 - b.bad_count as i32;
            db.cmp(&da)
                .then_with(|| b.good_count.cmp(&a.good_count))
                .then_with(|| a.bad_count.cmp(&b.bad_count))
                .then_with(|| a.rot.cmp(&b.rot))
                .then_with(|| a.top.cmp(&b.top))
                .then_with(|| a.left.cmp(&b.left))
        });
        if occs.len() > MAX_OCC_STORE {
            occs.truncate(MAX_OCC_STORE);
        }
        for occ in &occs {
            good_union.or_assign(&occ.good_mask);
        }

        Candidate {
            pattern: pattern.clone(),
            rotations,
            occs,
            good_union,
        }
    }

    fn build_rotations(&self, pattern: &Pattern) -> Vec<RotView> {
        let mut res = Vec::with_capacity(4);
        for rot in 0..4 {
            let (height, width) = rotated_dims(pattern.height, pattern.width, rot);
            let mut min_board_i = N;
            let mut min_board_j = N;
            let mut local_coords = Vec::with_capacity(pattern.cells.len());
            for cell in &pattern.cells {
                let (bi, bj) = rotate_coord_board(cell.i as usize, cell.j as usize, rot);
                min_board_i = min_board_i.min(bi);
                min_board_j = min_board_j.min(bj);
                let (ri, rj) = rotate_local_coord(
                    cell.i as usize,
                    cell.j as usize,
                    pattern.height,
                    pattern.width,
                    rot,
                );
                local_coords.push((ri as u8, rj as u8));
            }
            res.push(RotView {
                rot,
                height,
                width,
                min_board_i,
                min_board_j,
                local_coords,
            });
        }
        res
    }

    fn greedy_initial_state(&mut self, baseline_cost: usize) -> SearchState {
        let mut current = SearchState {
            pats: Vec::new(),
            cost: baseline_cost,
            residual: self.goal_nonzero,
            residual_count: baseline_cost,
        };

        let mut used = vec![false; self.pool.len()];
        for _ in 0..MAX_ACTIVE_PATTERNS {
            let mut best_idx = None;
            let mut best_trial = current.clone();
            let scan = self.pool.len().min(MAX_INIT_SCAN);
            for i in 0..scan {
                if used[i] {
                    continue;
                }
                let mut trial_pats = current.pats.clone();
                if self.contains_duplicate(&trial_pats, &self.pool[i].cand.pattern.signature, None)
                {
                    continue;
                }
                trial_pats.push(self.pool[i].clone());
                sort_patterns(&mut trial_pats);
                let ev = self.evaluate_state(&trial_pats, false);
                let st = SearchState {
                    pats: trial_pats,
                    cost: ev.total_cost,
                    residual: ev.residual,
                    residual_count: ev.residual_count,
                };
                if Self::better(&st, &best_trial) {
                    best_trial = st;
                    best_idx = Some(i);
                }
                if self.timer.elapsed() > TIME_LIMIT_SEC * 0.35 {
                    break;
                }
            }
            let Some(best_idx) = best_idx else {
                break;
            };
            used[best_idx] = true;
            current = best_trial;
        }

        current
    }

    fn propose_state(&mut self, current: &SearchState) -> Option<SearchState> {
        if self.timer.over() {
            return None;
        }
        let mut next = SearchState {
            pats: current.pats.clone(),
            cost: current.cost,
            residual: current.residual,
            residual_count: current.residual_count,
        };

        let move_type = self.choose_move_type(current);
        let changed = match move_type {
            0 => self.add_from_pool(&mut next.pats, &current.residual),
            1 => self.add_random_pattern(&mut next.pats, &current.residual),
            2 => self.drop_pattern(&mut next.pats),
            3 => self.replace_with_pool(&mut next.pats, &current.residual),
            _ => self.mutate_pattern(&mut next.pats, &current.residual),
        };
        if !changed {
            return None;
        }

        sort_patterns(&mut next.pats);
        let ev = self.evaluate_state(&next.pats, false);
        next.cost = ev.total_cost;
        next.residual = ev.residual;
        next.residual_count = ev.residual_count;
        Some(next)
    }

    fn choose_move_type(&mut self, current: &SearchState) -> usize {
        if current.pats.is_empty() {
            return if self.rng.chance(0.65) { 0 } else { 1 };
        }
        let r = self.rng.next_double();
        let can_add = current.pats.len() < MAX_ACTIVE_PATTERNS;
        if can_add && r < 0.20 {
            0
        } else if can_add && r < 0.38 {
            1
        } else if r < 0.50 {
            2
        } else if r < 0.68 {
            3
        } else {
            4
        }
    }

    fn contains_duplicate(
        &self,
        pats: &[Rc<PatternData>],
        sig: &[u16],
        ignore_idx: Option<usize>,
    ) -> bool {
        pats.iter().enumerate().any(|(i, pat)| {
            if Some(i) == ignore_idx {
                false
            } else {
                pat.cand.pattern.signature == sig
            }
        })
    }

    fn add_from_pool(&mut self, pats: &mut Vec<Rc<PatternData>>, residual: &Bits) -> bool {
        if pats.len() >= MAX_ACTIVE_PATTERNS || self.pool.is_empty() {
            return false;
        }
        let mut best_idx = None;
        let mut best_score = isize::MIN;
        for _ in 0..16 {
            let idx = self.rng.next_index(self.pool.len());
            if self.contains_duplicate(pats, &self.pool[idx].cand.pattern.signature, None) {
                continue;
            }
            let score = residual.and_count(&self.pool[idx].cand.good_union) as isize
                - self.pool[idx].cand.pattern.cells.len() as isize
                + (self.pool[idx].cand.pattern.hint_score / 8) as isize;
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }
        let Some(best_idx) = best_idx else {
            return false;
        };
        pats.push(self.pool[best_idx].clone());
        true
    }

    fn add_random_pattern(&mut self, pats: &mut Vec<Rc<PatternData>>, residual: &Bits) -> bool {
        if pats.len() >= MAX_ACTIVE_PATTERNS {
            return false;
        }
        for _ in 0..8 {
            let spec = self.random_window_spec(residual);
            let Some(data) = self.build_pattern_data_from_spec(&spec, 0) else {
                continue;
            };
            if self.contains_duplicate(pats, &data.cand.pattern.signature, None) {
                continue;
            }
            pats.push(Rc::new(data));
            return true;
        }
        false
    }

    fn drop_pattern(&mut self, pats: &mut Vec<Rc<PatternData>>) -> bool {
        if pats.is_empty() {
            return false;
        }
        let idx = self.rng.next_index(pats.len());
        pats.remove(idx);
        true
    }

    fn replace_with_pool(&mut self, pats: &mut Vec<Rc<PatternData>>, residual: &Bits) -> bool {
        if pats.is_empty() || self.pool.is_empty() {
            return false;
        }
        let target = self.rng.next_index(pats.len());
        let mut best_idx = None;
        let mut best_score = isize::MIN;
        for _ in 0..16 {
            let idx = self.rng.next_index(self.pool.len());
            if self.contains_duplicate(pats, &self.pool[idx].cand.pattern.signature, Some(target)) {
                continue;
            }
            let score = residual.and_count(&self.pool[idx].cand.good_union) as isize
                - self.pool[idx].cand.pattern.cells.len() as isize
                + (self.pool[idx].cand.pattern.hint_score / 8) as isize;
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }
        let Some(best_idx) = best_idx else {
            return false;
        };
        pats[target] = self.pool[best_idx].clone();
        true
    }

    fn mutate_pattern(&mut self, pats: &mut Vec<Rc<PatternData>>, residual: &Bits) -> bool {
        if pats.is_empty() {
            return false;
        }
        let idx = self.rng.next_index(pats.len());
        let mut spec = pats[idx].spec.clone();
        let r = self.rng.next_double();
        if r < 0.42 {
            self.flip_mask_cell(&mut spec);
        } else if r < 0.67 {
            self.shift_window(&mut spec);
        } else if r < 0.87 {
            self.resize_side(&mut spec);
        } else if r < 0.95 {
            self.flip_rect(&mut spec);
        } else {
            spec = self.random_window_spec(residual);
        }
        let Some(data) =
            self.build_pattern_data_from_spec(&spec, pats[idx].cand.pattern.hint_score)
        else {
            return false;
        };
        if self.contains_duplicate(pats, &data.cand.pattern.signature, Some(idx)) {
            return false;
        }
        pats[idx] = Rc::new(data);
        true
    }

    fn random_window_spec(&mut self, residual: &Bits) -> PatternSpec {
        let mut seed_p = None;
        if self.rng.chance(0.85) {
            for _ in 0..24 {
                let p = self.nonzero_positions[self.rng.next_index(self.nonzero_positions.len())];
                if residual.contains(p) {
                    seed_p = Some(p);
                    break;
                }
            }
        }
        let seed_p = seed_p.unwrap_or_else(|| {
            self.nonzero_positions[self.rng.next_index(self.nonzero_positions.len())]
        });

        let si = seed_p / N;
        let sj = seed_p % N;
        let rr = self.rng.next_double();
        let (area_l, area_r) = if rr < 0.70 {
            (4usize, 16usize)
        } else if rr < 0.95 {
            (17usize, 36usize)
        } else {
            (37usize, 64usize)
        };

        let mut dims = Vec::new();
        for height in 1..=MAX_RANDOM_SIDE {
            for width in 1..=MAX_RANDOM_SIDE {
                let area = height * width;
                if area_l <= area && area <= area_r {
                    dims.push((height, width));
                }
            }
        }
        let (height, width) = dims[self.rng.next_index(dims.len())];
        let oi = self.rng.next_usize_inclusive(0, height - 1);
        let oj = self.rng.next_usize_inclusive(0, width - 1);
        let top = (si as isize - oi as isize).clamp(0, (N - height) as isize) as usize;
        let left = (sj as isize - oj as isize).clamp(0, (N - width) as isize) as usize;

        PatternSpec {
            top,
            left,
            height,
            width,
            mask: vec![1_u8; height * width],
        }
    }

    fn flip_mask_cell(&mut self, spec: &mut PatternSpec) {
        if spec.height == 0 || spec.width == 0 {
            return;
        }
        let r = self.rng.next_usize_inclusive(0, spec.height - 1);
        let c = self.rng.next_usize_inclusive(0, spec.width - 1);
        let idx = r * spec.width + c;
        spec.mask[idx] ^= 1;
    }

    fn shift_window(&mut self, spec: &mut PatternSpec) {
        let di = self.rng.next_i32_inclusive(1, 8) * if self.rng.chance(0.5) { 1 } else { -1 };
        let dj = self.rng.next_i32_inclusive(1, 8) * if self.rng.chance(0.5) { 1 } else { -1 };
        spec.top = (spec.top as isize + di as isize).clamp(0, (N - spec.height) as isize) as usize;
        spec.left = (spec.left as isize + dj as isize).clamp(0, (N - spec.width) as isize) as usize;
    }

    fn resize_side(&mut self, spec: &mut PatternSpec) {
        let side = self.rng.next_usize_inclusive(0, 3);
        let expand = self.rng.chance(0.5);
        let h = spec.height;
        let w = spec.width;
        let get = |mask: &[u8], r: usize, c: usize, width: usize| -> u8 { mask[r * width + c] };

        match side {
            0 => {
                if expand && spec.top > 0 {
                    let mut new_mask = vec![1_u8; (h + 1) * w];
                    for r in 0..h {
                        for c in 0..w {
                            new_mask[(r + 1) * w + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.top -= 1;
                    spec.height += 1;
                    spec.mask = new_mask;
                } else if !expand && h > 1 {
                    let mut new_mask = vec![0_u8; (h - 1) * w];
                    for r in 1..h {
                        for c in 0..w {
                            new_mask[(r - 1) * w + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.top += 1;
                    spec.height -= 1;
                    spec.mask = new_mask;
                }
            }
            1 => {
                if expand && spec.top + h < N {
                    let mut new_mask = vec![1_u8; (h + 1) * w];
                    for r in 0..h {
                        for c in 0..w {
                            new_mask[r * w + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.height += 1;
                    spec.mask = new_mask;
                } else if !expand && h > 1 {
                    let mut new_mask = vec![0_u8; (h - 1) * w];
                    for r in 0..(h - 1) {
                        for c in 0..w {
                            new_mask[r * w + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.height -= 1;
                    spec.mask = new_mask;
                }
            }
            2 => {
                if expand && spec.left > 0 {
                    let mut new_mask = vec![1_u8; h * (w + 1)];
                    for r in 0..h {
                        for c in 0..w {
                            new_mask[r * (w + 1) + (c + 1)] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.left -= 1;
                    spec.width += 1;
                    spec.mask = new_mask;
                } else if !expand && w > 1 {
                    let mut new_mask = vec![0_u8; h * (w - 1)];
                    for r in 0..h {
                        for c in 1..w {
                            new_mask[r * (w - 1) + (c - 1)] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.left += 1;
                    spec.width -= 1;
                    spec.mask = new_mask;
                }
            }
            _ => {
                if expand && spec.left + w < N {
                    let mut new_mask = vec![1_u8; h * (w + 1)];
                    for r in 0..h {
                        for c in 0..w {
                            new_mask[r * (w + 1) + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.width += 1;
                    spec.mask = new_mask;
                } else if !expand && w > 1 {
                    let mut new_mask = vec![0_u8; h * (w - 1)];
                    for r in 0..h {
                        for c in 0..(w - 1) {
                            new_mask[r * (w - 1) + c] = get(&spec.mask, r, c, w);
                        }
                    }
                    spec.width -= 1;
                    spec.mask = new_mask;
                }
            }
        }
    }

    fn flip_rect(&mut self, spec: &mut PatternSpec) {
        if spec.height == 0 || spec.width == 0 {
            return;
        }
        let rh = self.rng.next_usize_inclusive(1, 3.min(spec.height));
        let rw = self.rng.next_usize_inclusive(1, 3.min(spec.width));
        let r0 = self.rng.next_usize_inclusive(0, spec.height - rh);
        let c0 = self.rng.next_usize_inclusive(0, spec.width - rw);
        let mode = self.rng.next_usize_inclusive(0, 2);
        for r in 0..rh {
            for c in 0..rw {
                let v = &mut spec.mask[(r0 + r) * spec.width + (c0 + c)];
                match mode {
                    0 => *v = 0,
                    1 => *v = 1,
                    _ => *v ^= 1,
                }
            }
        }
    }

    fn evaluate_state(&self, pats: &[Rc<PatternData>], record_steps: bool) -> EvalResult {
        let mut result = EvalResult {
            total_cost: 0,
            residual: self.goal_nonzero,
            residual_count: self.goal_nonzero.count(),
            steps: Vec::new(),
        };
        let mut loaded: Option<usize> = None;
        let mut cost = 0usize;

        loop {
            let mut best_idx = None;
            let mut best_batch = BatchChoice::default();
            for (idx, pat) in pats.iter().enumerate() {
                let cand = &pat.cand;
                let load_cost = if loaded.is_none() {
                    cand.pattern.cells.len()
                } else if loaded == Some(idx) {
                    0
                } else {
                    1 + cand.pattern.cells.len()
                };
                if result.residual.and_count(&cand.good_union) <= load_cost + 1 {
                    continue;
                }
                let batch = self.best_batch_for_candidate(cand, &result.residual, load_cost);
                if !batch.valid {
                    continue;
                }
                let take = match best_idx {
                    None => true,
                    Some(_) => {
                        batch.gain > best_batch.gain
                            || (batch.gain == best_batch.gain
                                && batch.next_residual_count < best_batch.next_residual_count)
                            || (batch.gain == best_batch.gain
                                && batch.next_residual_count == best_batch.next_residual_count
                                && batch.transition_cost < best_batch.transition_cost)
                    }
                };
                if take {
                    best_idx = Some(idx);
                    best_batch = batch;
                }
            }

            let Some(best_idx) = best_idx else {
                break;
            };
            if best_batch.gain <= 0 {
                break;
            }

            loaded = Some(best_idx);
            cost += best_batch.transition_cost;
            result.residual = best_batch.next_residual;
            result.residual_count = best_batch.next_residual_count;
            if record_steps {
                result.steps.push(BatchStep {
                    pat_idx: best_idx,
                    occ_indices: best_batch.occ_indices,
                });
            }
            if cost >= N * N {
                break;
            }
        }

        result.total_cost = cost + result.residual_count;
        result
    }

    fn best_batch_for_candidate(
        &self,
        cand: &Candidate,
        residual: &Bits,
        load_cost: usize,
    ) -> BatchChoice {
        #[derive(Debug, Clone, Copy)]
        struct ScoredOcc {
            delta: isize,
            good: usize,
            bad: usize,
            idx: usize,
        }

        let mut scored = Vec::with_capacity(cand.occs.len());
        for (idx, occ) in cand.occs.iter().enumerate() {
            let delta = residual_delta(residual, occ);
            if delta >= MIN_OCC_GAIN {
                scored.push(ScoredOcc {
                    delta,
                    good: occ.good_count as usize,
                    bad: occ.bad_count as usize,
                    idx,
                });
            }
        }
        if scored.is_empty() {
            return BatchChoice::default();
        }

        scored.sort_by(|a, b| {
            b.delta
                .cmp(&a.delta)
                .then_with(|| b.good.cmp(&a.good))
                .then_with(|| a.bad.cmp(&b.bad))
                .then_with(|| a.idx.cmp(&b.idx))
        });
        if scored.len() > MAX_SCORE_OCCS {
            scored.truncate(MAX_SCORE_OCCS);
        }

        let mut seed_list = Vec::with_capacity(1 + MAX_SEED_OCCS);
        seed_list.push(None);
        for scored_occ in scored.iter().take(MAX_SEED_OCCS) {
            seed_list.push(Some(scored_occ.idx));
        }

        let mut best = BatchChoice::default();
        let initial_cnt = residual.count();
        for seed_idx in seed_list {
            let mut rem = *residual;
            let mut selected = Vec::new();

            let consider = |selected: &[usize], rem: &Bits, best: &mut BatchChoice| {
                let transition_cost = load_cost + selected.len();
                let next_cnt = rem.count();
                let gain = initial_cnt as isize - next_cnt as isize - transition_cost as isize;
                if gain <= 0 {
                    return;
                }
                let take = !best.valid
                    || gain > best.gain
                    || (gain == best.gain && next_cnt < best.next_residual_count)
                    || (gain == best.gain
                        && next_cnt == best.next_residual_count
                        && transition_cost < best.transition_cost);
                if take {
                    best.valid = true;
                    best.gain = gain;
                    best.transition_cost = transition_cost;
                    best.next_residual = *rem;
                    best.next_residual_count = next_cnt;
                    best.occ_indices = selected.to_vec();
                }
            };

            if let Some(seed_idx) = seed_idx {
                let occ = &cand.occs[seed_idx];
                let delta = residual_delta(&rem, occ);
                if delta < MIN_OCC_GAIN {
                    continue;
                }
                selected.push(seed_idx);
                apply_occurrence_to_residual(&mut rem, occ);
                consider(&selected, &rem, &mut best);
            }

            for scored_occ in &scored {
                if Some(scored_occ.idx) == seed_idx {
                    continue;
                }
                let occ = &cand.occs[scored_occ.idx];
                let delta = residual_delta(&rem, occ);
                if delta <= 0 {
                    continue;
                }
                selected.push(scored_occ.idx);
                apply_occurrence_to_residual(&mut rem, occ);
                consider(&selected, &rem, &mut best);
            }
        }

        best
    }

    fn state_to_ops(&self, pats: &[Rc<PatternData>], plan: &EvalResult) -> Vec<Op> {
        let mut ops = Vec::with_capacity(plan.total_cost + 8);
        let mut loaded: Option<usize> = None;

        for step in &plan.steps {
            let cand = &pats[step.pat_idx].cand;
            if loaded != Some(step.pat_idx) {
                if loaded.is_some() {
                    ops.push(Op::Clear { k: PATTERN_LAYER });
                }
                for cell in &cand.pattern.cells {
                    ops.push(Op::Paint {
                        k: PATTERN_LAYER,
                        i: cell.i as usize,
                        j: cell.j as usize,
                        color: cell.color,
                    });
                }
                loaded = Some(step.pat_idx);
            }
            for &occ_idx in &step.occ_indices {
                let occ = &cand.occs[occ_idx];
                let view = &cand.rotations[occ.rot as usize];
                let di = occ.top as isize - view.min_board_i as isize;
                let dj = occ.left as isize - view.min_board_j as isize;
                ops.push(Op::Copy {
                    k: 0,
                    h: PATTERN_LAYER,
                    rot: occ.rot as usize,
                    di,
                    dj,
                });
            }
        }

        for p in plan.residual.positions() {
            let i = p / N;
            let j = p % N;
            ops.push(Op::Paint {
                k: 0,
                i,
                j,
                color: self.input.goal[i][j],
            });
        }
        ops
    }

    fn verify_ops(&self, ops: &[Op]) -> bool {
        if ops.len() > N * N {
            return false;
        }

        let mut state = SimState {
            layers: vec![[[0; N]; N]; self.input.k_layers],
            op_count: 0,
        };

        for &op in ops {
            if state.op_count >= N * N {
                return false;
            }
            match op {
                Op::Paint { k, i, j, color } => {
                    if k >= self.input.k_layers
                        || i >= N
                        || j >= N
                        || color as usize > self.input.color_count
                    {
                        return false;
                    }
                    state.layers[k][i][j] = color;
                }
                Op::Copy { k, h, rot, di, dj } => {
                    if k >= self.input.k_layers || h >= self.input.k_layers || rot >= 4 {
                        return false;
                    }
                    let src = state.layers[h];
                    let mut dst = state.layers[k];
                    for (i, row) in src.iter().enumerate() {
                        for (j, &color) in row.iter().enumerate() {
                            if color == 0 {
                                continue;
                            }
                            let (ri, rj) = rotate_coord_board(i, j, rot);
                            let ni = ri as isize + di;
                            let nj = rj as isize + dj;
                            if !(0..N as isize).contains(&ni) || !(0..N as isize).contains(&nj) {
                                return false;
                            }
                            dst[ni as usize][nj as usize] = color;
                        }
                    }
                    state.layers[k] = dst;
                }
                Op::Clear { k } => {
                    if k >= self.input.k_layers {
                        return false;
                    }
                    state.layers[k] = [[0; N]; N];
                }
            }
            state.op_count += 1;
        }

        state.layers[0] == self.input.goal
    }
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

fn format_ops(ops: &[Op]) -> String {
    let mut out = String::new();
    for &op in ops {
        op.write_to(&mut out);
    }
    out
}

fn main() {
    let input = Input::read();
    let mut solver = Solver::new(input);
    let ops = solver.solve();
    print!("{}", format_ops(&ops));
}
