// v004_beam_parts.rs
use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::fmt::Write as _;
use std::io::{self, Read};
use std::str::{FromStr, SplitAsciiWhitespace};
use std::time::Instant;

const N: usize = 32;
const M: usize = N * N;
const WORDS: usize = M / 64;
const DIR4: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

const MIN_PATTERN_SIZE: usize = 6;
const MAX_CANDIDATES: usize = 160;
const MAX_SCAN_CANDIDATES: usize = 64;
const MAX_TOP_OCCS: usize = 256;
const MAX_SEED_OCCS: usize = 10;
const MAX_VARIANTS_PER_CANDIDATE: usize = 12;
const MAX_BRANCHES_PER_STATE: usize = 32;
const MAX_BEAM_WIDTH: usize = 192;
const MAX_BEAM_DEPTH: usize = 16;
const MIN_OCC_GAIN: usize = 2;
const SOLVER_TIME_LIMIT_SEC: f64 = 1.98;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Bits {
    w: [u64; WORDS],
}

impl Bits {
    #[inline]
    fn set(&mut self, p: usize) {
        self.w[p >> 6] |= 1_u64 << (p & 63);
    }

    #[inline]
    fn or_assign(&mut self, other: &Self) {
        for i in 0..WORDS {
            self.w[i] |= other.w[i];
        }
    }

    #[inline]
    fn difference(&self, other: &Self) -> Self {
        let mut res = Self::default();
        for i in 0..WORDS {
            res.w[i] = self.w[i] & !other.w[i];
        }
        res
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    hint_score: usize,
}

#[derive(Debug, Clone)]
struct RotatedPattern {
    rot: usize,
    height: usize,
    width: usize,
    min_board_i: usize,
    min_board_j: usize,
}

#[derive(Debug, Clone)]
struct Occurrence {
    rot: usize,
    top: usize,
    left: usize,
    mask: Bits,
}

#[derive(Debug, Clone)]
struct Candidate {
    pattern: Pattern,
    rotations: Vec<RotatedPattern>,
    occurrences: Vec<Occurrence>,
    union_mask: Bits,
}

#[derive(Debug, Clone)]
struct CandidateStat {
    pattern: Pattern,
    hits: usize,
    total_component_size: usize,
}

#[derive(Debug, Clone)]
struct StepBatch {
    candidate_idx: usize,
    occurrence_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct BeamState {
    residual: Bits,
    residual_count: usize,
    loaded_candidate: Option<usize>,
    cost: usize,
    steps: Vec<StepBatch>,
}

#[derive(Debug, Clone)]
struct Transition {
    candidate_idx: usize,
    occurrence_indices: Vec<usize>,
    covered_mask: Bits,
    covered_count: usize,
    transition_cost: usize,
    gain: isize,
}

fn main() {
    let input = Input::read();
    let ops = solve(&input);
    print!("{}", format_ops(&ops));
}

fn solve(input: &Input) -> Vec<Op> {
    let baseline_ops = baseline_paint_ops(&input.goal);
    let baseline_cost = input.nonzero_goal_count();
    let debug = std::env::var_os("V004_DEBUG").is_some();

    let mut time_keeper = TimeKeeper::new(SOLVER_TIME_LIMIT_SEC, 10);
    let mut patterns = extract_patterns(input, &mut time_keeper);
    patterns.sort_by_key(|pattern| Reverse((pattern.hint_score, pattern.cells.len())));
    if patterns.len() > MAX_CANDIDATES * 2 {
        patterns.truncate(MAX_CANDIDATES * 2);
    }

    let mut candidates = patterns
        .into_iter()
        .map(|pattern| materialize_candidate(pattern, &input.goal))
        .filter(|candidate| candidate.occurrences.len() >= 2)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        Reverse((
            candidate.pattern.hint_score,
            candidate.pattern.cells.len(),
            candidate.occurrences.len(),
        ))
    });
    if candidates.len() > MAX_CANDIDATES {
        candidates.truncate(MAX_CANDIDATES);
    }

    if debug {
        eprintln!(
            "baseline={} candidates={} limit={:.2}",
            baseline_cost,
            candidates.len(),
            SOLVER_TIME_LIMIT_SEC
        );
    }

    let ops = build_plan_beam(input, &candidates, baseline_cost, &mut time_keeper, debug);
    if ops.len() > baseline_cost {
        return baseline_ops;
    }

    let mut state = State::new(input.k_layers);
    match state.apply_all(&ops) {
        Ok(()) if state.layer0_matches(&input.goal) => ops,
        Ok(()) => baseline_ops,
        Err(_) => baseline_ops,
    }
}

fn build_plan_beam(
    input: &Input,
    candidates: &[Candidate],
    baseline_cost: usize,
    time_keeper: &mut TimeKeeper,
    debug: bool,
) -> Vec<Op> {
    let target = goal_bits(&input.goal);
    let initial = BeamState {
        residual: target,
        residual_count: baseline_cost,
        loaded_candidate: None,
        cost: 0,
        steps: Vec::new(),
    };

    let mut best_state = initial.clone();
    let mut best_total_cost = baseline_cost;
    let mut beam = vec![initial];

    for depth in 0..MAX_BEAM_DEPTH {
        if beam.is_empty() || time_keeper.is_over() {
            break;
        }

        let mut next = Vec::<BeamState>::new();
        let mut seen = FxHashMap::<(Bits, i16), usize>::default();

        for state in &beam {
            let finish_cost = state.cost + state.residual_count;
            if finish_cost < best_total_cost {
                best_total_cost = finish_cost;
                best_state = state.clone();
            }
        }

        for state in beam.into_iter() {
            if time_keeper.is_over() {
                break;
            }

            let transitions = enumerate_transitions(&state, candidates);
            for transition in transitions.into_iter().take(MAX_BRANCHES_PER_STATE) {
                let next_cost = state.cost + transition.transition_cost;
                let next_total_bound = next_cost + state.residual_count - transition.covered_count;
                if next_total_bound >= best_total_cost || next_cost > N * N {
                    continue;
                }

                let next_residual = state.residual.difference(&transition.covered_mask);
                let next_residual_count = state.residual_count - transition.covered_count;
                let next_loaded = Some(transition.candidate_idx);
                let seen_key = (next_residual, next_loaded.map(|idx| idx as i16).unwrap_or(-1));

                let keep = match seen.get(&seen_key) {
                    None => true,
                    Some(&best_cost) => next_cost < best_cost,
                };
                if !keep {
                    continue;
                }
                seen.insert(seen_key, next_cost);

                let mut next_steps = state.steps.clone();
                next_steps.push(StepBatch {
                    candidate_idx: transition.candidate_idx,
                    occurrence_indices: transition.occurrence_indices,
                });
                next.push(BeamState {
                    residual: next_residual,
                    residual_count: next_residual_count,
                    loaded_candidate: next_loaded,
                    cost: next_cost,
                    steps: next_steps,
                });
            }
        }

        next.sort_by_key(|state| {
            (
                state.cost + state.residual_count,
                state.cost,
                state.residual_count,
                state.steps.len(),
            )
        });
        if next.len() > MAX_BEAM_WIDTH {
            next.truncate(MAX_BEAM_WIDTH);
        }

        if debug {
            eprintln!(
                "depth={} beam={} best={}",
                depth + 1,
                next.len(),
                best_total_cost
            );
        }
        beam = next;
    }

    for state in &beam {
        let finish_cost = state.cost + state.residual_count;
        if finish_cost < best_total_cost {
            best_total_cost = finish_cost;
            best_state = state.clone();
        }
    }

    if debug {
        eprintln!(
            "final best_total={} batches={}",
            best_total_cost,
            best_state.steps.len()
        );
    }

    state_to_ops(input, candidates, &best_state)
}

fn enumerate_transitions(state: &BeamState, candidates: &[Candidate]) -> Vec<Transition> {
    let loaded = state.loaded_candidate;
    let mut rough = Vec::<(usize, usize)>::new();
    for (idx, candidate) in candidates.iter().enumerate() {
        let base_cost = match loaded {
            Some(current) if current == idx => 0,
            Some(_) => 1 + candidate.pattern.cells.len(),
            None => candidate.pattern.cells.len(),
        };
        let potential_cover = state.residual.and_count(&candidate.union_mask);
        if potential_cover <= base_cost + 1 {
            continue;
        }
        rough.push((potential_cover - base_cost, idx));
    }
    rough.sort_by_key(|&(score, idx)| Reverse((score, idx)));
    if rough.len() > MAX_SCAN_CANDIDATES {
        rough.truncate(MAX_SCAN_CANDIDATES);
    }

    let mut transitions = Vec::new();
    for (_, idx) in rough {
        let variants = build_candidate_variants(
            idx,
            &candidates[idx],
            &state.residual,
            match loaded {
                Some(current) if current == idx => 0,
                Some(_) => 1 + candidates[idx].pattern.cells.len(),
                None => candidates[idx].pattern.cells.len(),
            },
        );
        transitions.extend(variants);
    }
    transitions.sort_by_key(|tr| {
        Reverse((
            tr.gain,
            tr.covered_count,
            usize::MAX - tr.transition_cost,
            tr.candidate_idx,
        ))
    });
    transitions
}

fn build_candidate_variants(
    candidate_idx: usize,
    candidate: &Candidate,
    residual: &Bits,
    base_cost: usize,
) -> Vec<Transition> {
    let mut scored = candidate
        .occurrences
        .iter()
        .enumerate()
        .map(|(idx, occ)| (residual.and_count(&occ.mask), idx))
        .filter(|&(gain, _)| gain >= MIN_OCC_GAIN)
        .collect::<Vec<_>>();
    scored.sort_by_key(|&(gain, idx)| Reverse((gain, idx)));
    if scored.len() > MAX_TOP_OCCS {
        scored.truncate(MAX_TOP_OCCS);
    }
    if scored.is_empty() {
        return Vec::new();
    }

    let mut seed_list = Vec::with_capacity(MAX_SEED_OCCS + 1);
    seed_list.push(None);
    for &(_, idx) in scored.iter().take(MAX_SEED_OCCS) {
        seed_list.push(Some(idx));
    }

    let mut variants = Vec::<Transition>::new();
    let mut seen = FxHashMap::<Bits, usize>::default();

    for seed in seed_list {
        for (occurrence_indices, covered_mask, covered_count) in
            greedy_occurrence_prefixes(candidate, residual, &scored, seed)
        {
            let transition_cost = base_cost + occurrence_indices.len();
            let gain = covered_count as isize - transition_cost as isize;
            if gain <= 0 {
                continue;
            }

            let copies = occurrence_indices.len();
            let keep = match seen.get(&covered_mask) {
                None => true,
                Some(&best_copies) => copies < best_copies,
            };
            if !keep {
                continue;
            }
            seen.insert(covered_mask, copies);

            variants.push(Transition {
                candidate_idx,
                occurrence_indices,
                covered_mask,
                covered_count,
                transition_cost,
                gain,
            });
        }
    }

    variants.sort_by_key(|tr| {
        Reverse((
            tr.gain,
            tr.covered_count,
            usize::MAX - tr.transition_cost,
        ))
    });
    if variants.len() > MAX_VARIANTS_PER_CANDIDATE {
        variants.truncate(MAX_VARIANTS_PER_CANDIDATE);
    }
    variants
}

fn greedy_occurrence_prefixes(
    candidate: &Candidate,
    residual: &Bits,
    scored: &[(usize, usize)],
    seed: Option<usize>,
) -> Vec<(Vec<usize>, Bits, usize)> {
    let mut remaining = *residual;
    let mut covered_mask = Bits::default();
    let mut selected = Vec::new();
    let mut covered_count = 0usize;
    let mut variants = Vec::new();
    let mut next_checkpoint_idx = 0usize;
    const CHECKPOINTS: [usize; 12] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64];

    if let Some(seed_idx) = seed {
        let gain = remaining.and_count(&candidate.occurrences[seed_idx].mask);
        if gain < MIN_OCC_GAIN {
            return Vec::new();
        }
        selected.push(seed_idx);
        let occ_mask = candidate.occurrences[seed_idx].mask;
        remaining.minus_assign(&occ_mask);
        covered_mask.or_assign(&occ_mask);
        covered_count += gain;
        variants.push((selected.clone(), covered_mask, covered_count));
        while next_checkpoint_idx < CHECKPOINTS.len()
            && CHECKPOINTS[next_checkpoint_idx] <= selected.len()
        {
            next_checkpoint_idx += 1;
        }
    }

    for &(_, occ_idx) in scored {
        if seed == Some(occ_idx) {
            continue;
        }
        let gain = remaining.and_count(&candidate.occurrences[occ_idx].mask);
        if gain < MIN_OCC_GAIN {
            continue;
        }
        selected.push(occ_idx);
        let occ_mask = candidate.occurrences[occ_idx].mask;
        remaining.minus_assign(&occ_mask);
        covered_mask.or_assign(&occ_mask);
        covered_count += gain;

        if next_checkpoint_idx < CHECKPOINTS.len()
            && CHECKPOINTS[next_checkpoint_idx] == selected.len()
        {
            variants.push((selected.clone(), covered_mask, covered_count));
            next_checkpoint_idx += 1;
        }
    }

    if !selected.is_empty() {
        let needs_final = variants
            .last()
            .map(|(occurrence_indices, _, _)| occurrence_indices.len() != selected.len())
            .unwrap_or(true);
        if needs_final {
            variants.push((selected, covered_mask, covered_count));
        }
    }

    variants
}

fn state_to_ops(input: &Input, candidates: &[Candidate], state: &BeamState) -> Vec<Op> {
    let mut ops = Vec::with_capacity(state.cost + state.residual_count);
    let pattern_layer = 1usize;
    let mut loaded_candidate = None;

    for step in &state.steps {
        let candidate = &candidates[step.candidate_idx];
        if loaded_candidate != Some(step.candidate_idx) {
            if loaded_candidate.is_some() {
                ops.push(Op::Clear { k: pattern_layer });
            }
            for cell in &candidate.pattern.cells {
                ops.push(Op::Paint {
                    k: pattern_layer,
                    i: cell.i,
                    j: cell.j,
                    color: cell.color,
                });
            }
            loaded_candidate = Some(step.candidate_idx);
        }
        for &occ_idx in &step.occurrence_indices {
            let occ = &candidate.occurrences[occ_idx];
            let view = &candidate.rotations[occ.rot];
            ops.push(Op::Copy {
                k: 0,
                h: pattern_layer,
                transform: Transform::new(
                    occ.rot,
                    occ.top as isize - view.min_board_i as isize,
                    occ.left as isize - view.min_board_j as isize,
                ),
            });
        }
    }

    for p in state.residual.positions() {
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

fn goal_bits(goal: &Grid) -> Bits {
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

fn baseline_paint_ops(goal: &Grid) -> Vec<Op> {
    let mut ops = Vec::new();
    for (i, row) in goal.iter().enumerate() {
        for (j, &color) in row.iter().enumerate() {
            if color != 0 {
                ops.push(Op::Paint { k: 0, i, j, color });
            }
        }
    }
    ops
}

fn extract_patterns(input: &Input, time_keeper: &mut TimeKeeper) -> Vec<Pattern> {
    let goal = &input.goal;
    let mut matching = [[false; N]; N];
    let mut visited = [[false; N]; N];
    let mut queue = Vec::<Coord>::with_capacity(M);
    let mut component = Vec::<Coord>::with_capacity(M);
    let mut stats = FxHashMap::<Vec<u16>, CandidateStat>::default();

    'outer: for rot in 0..4 {
        for di in -(N as isize) + 1..=(N as isize) - 1 {
            for dj in -(N as isize) + 1..=(N as isize) - 1 {
                if rot == 0 && di == 0 && dj == 0 {
                    continue;
                }
                if !time_keeper.step() {
                    break 'outer;
                }

                let mut match_count = 0usize;
                for i in 0..N {
                    for j in 0..N {
                        let ok = goal[i][j] != 0
                            && transformed_coord((i, j), rot, di, dj)
                                .map(|(ni, nj)| goal[ni][nj] == goal[i][j])
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
                        queue.clear();
                        component.clear();
                        queue.push((si, sj));
                        visited[si][sj] = true;

                        let mut head = 0usize;
                        while head < queue.len() {
                            let (i, j) = queue[head];
                            head += 1;
                            component.push((i, j));
                            for &(di4, dj4) in &DIR4 {
                                let ni = i as isize + di4;
                                let nj = j as isize + dj4;
                                if !(0..N as isize).contains(&ni) || !(0..N as isize).contains(&nj)
                                {
                                    continue;
                                }
                                let ni = ni as usize;
                                let nj = nj as usize;
                                if matching[ni][nj] && !visited[ni][nj] {
                                    visited[ni][nj] = true;
                                    queue.push((ni, nj));
                                }
                            }
                        }

                        if component.len() < MIN_PATTERN_SIZE {
                            continue;
                        }

                        let pattern = canonicalize_component(goal, &component);
                        let key = pattern_signature(&pattern);
                        let entry = stats.entry(key).or_insert_with(|| CandidateStat {
                            pattern: pattern.clone(),
                            hits: 0,
                            total_component_size: 0,
                        });
                        entry.hits += 1;
                        entry.total_component_size += component.len();
                    }
                }
            }
        }
    }

    stats.into_values()
        .map(|stat| {
            let mut pattern = stat.pattern;
            pattern.hint_score = stat.total_component_size + stat.hits * pattern.cells.len();
            pattern
        })
        .collect()
}

fn transformed_coord((i, j): Coord, rot: usize, di: isize, dj: isize) -> Option<Coord> {
    let (ri, rj) = rotate_coord((i, j), rot);
    let ni = ri as isize + di;
    let nj = rj as isize + dj;
    if (0..N as isize).contains(&ni) && (0..N as isize).contains(&nj) {
        Some((ni as usize, nj as usize))
    } else {
        None
    }
}

fn canonicalize_component(goal: &Grid, component: &[Coord]) -> Pattern {
    let mut min_i = N;
    let mut max_i = 0usize;
    let mut min_j = N;
    let mut max_j = 0usize;
    for &(i, j) in component {
        min_i = min_i.min(i);
        max_i = max_i.max(i);
        min_j = min_j.min(j);
        max_j = max_j.max(j);
    }

    let base_height = max_i - min_i + 1;
    let base_width = max_j - min_j + 1;
    let mut base_cells = component
        .iter()
        .map(|&(i, j)| Cell {
            i: i - min_i,
            j: j - min_j,
            color: goal[i][j],
        })
        .collect::<Vec<_>>();
    base_cells.sort();

    let mut best_pattern = rotate_pattern_cells(&base_cells, base_height, base_width, 0);
    let mut best_signature = pattern_signature(&best_pattern);
    for rot in 1..4 {
        let candidate = rotate_pattern_cells(&base_cells, base_height, base_width, rot);
        let signature = pattern_signature(&candidate);
        if signature < best_signature {
            best_signature = signature;
            best_pattern = candidate;
        }
    }
    best_pattern
}

fn rotate_pattern_cells(cells: &[Cell], height: usize, width: usize, rot: usize) -> Pattern {
    let (rot_h, rot_w) = rotated_dims(height, width, rot);
    let mut rotated = cells
        .iter()
        .map(|cell| {
            let (i, j) = rotate_local_coord((cell.i, cell.j), height, width, rot);
            Cell {
                i,
                j,
                color: cell.color,
            }
        })
        .collect::<Vec<_>>();
    rotated.sort();
    Pattern {
        height: rot_h,
        width: rot_w,
        cells: rotated,
        hint_score: 0,
    }
}

fn pattern_signature(pattern: &Pattern) -> Vec<u16> {
    let mut signature = Vec::with_capacity(pattern.cells.len() + 2);
    signature.push(pattern.height as u16);
    signature.push(pattern.width as u16);
    for cell in &pattern.cells {
        signature.push(((cell.i as u16) << 11) | ((cell.j as u16) << 3) | cell.color as u16);
    }
    signature
}

fn materialize_candidate(pattern: Pattern, goal: &Grid) -> Candidate {
    let rotations = build_rotations(&pattern);
    let mut occurrences = Vec::new();
    let mut union_mask = Bits::default();

    for view in &rotations {
        for top in 0..=N - view.height {
            for left in 0..=N - view.width {
                let mut mask = Bits::default();
                let mut ok = true;
                for cell in &pattern.cells {
                    let (ri, rj) =
                        rotate_local_coord((cell.i, cell.j), pattern.height, pattern.width, view.rot);
                    let bi = top + ri;
                    let bj = left + rj;
                    if goal[bi][bj] != cell.color {
                        ok = false;
                        break;
                    }
                    mask.set(bi * N + bj);
                }
                if ok {
                    union_mask.or_assign(&mask);
                    occurrences.push(Occurrence {
                        rot: view.rot,
                        top,
                        left,
                        mask,
                    });
                }
            }
        }
    }

    Candidate {
        pattern,
        rotations,
        occurrences,
        union_mask,
    }
}

fn build_rotations(pattern: &Pattern) -> Vec<RotatedPattern> {
    let mut rotations = Vec::with_capacity(4);
    for rot in 0..4 {
        let (height, width) = rotated_dims(pattern.height, pattern.width, rot);
        let mut min_board_i = N;
        let mut min_board_j = N;
        for cell in &pattern.cells {
            let (bi, bj) = rotate_coord((cell.i, cell.j), rot);
            min_board_i = min_board_i.min(bi);
            min_board_j = min_board_j.min(bj);
        }
        rotations.push(RotatedPattern {
            rot,
            height,
            width,
            min_board_i,
            min_board_j,
        });
    }
    rotations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_difference_and_count() {
        let mut a = Bits::default();
        let mut b = Bits::default();
        a.set(1);
        a.set(65);
        b.set(65);
        assert_eq!(a.count(), 2);
        assert_eq!(a.and_count(&b), 1);
        let c = a.difference(&b);
        assert_eq!(c.count(), 1);
    }

    #[test]
    fn rotate_coord_matches_problem_definition() {
        assert_eq!(rotate_coord((1, 2), 0), (1, 2));
        assert_eq!(rotate_coord((1, 2), 1), (2, N - 1 - 1));
        assert_eq!(rotate_coord((1, 2), 2), (N - 1 - 1, N - 1 - 2));
        assert_eq!(rotate_coord((1, 2), 3), (N - 1 - 2, 1));
    }
}
