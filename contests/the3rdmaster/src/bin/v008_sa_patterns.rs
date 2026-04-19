// v008_sa_patterns.rs
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
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
const MAX_CANDIDATES: usize = 64;
const MAX_CANDIDATE_POOL: usize = 128;
const MAX_OCCURRENCES: usize = 192;
const MAX_RELATIONS: usize = 10;
const MAX_SEQUENCE_LEN: usize = 24;
const SA_START_TEMP: f64 = 20_000.0;
const SA_END_TEMP: f64 = 20.0;
const SOLVER_TIME_LIMIT_SEC: f64 = 1.92;
const EXTRACTION_LIMIT_SEC: f64 = 0.50;
const MAX_OCC_BAD: usize = 4;
const MAX_OCC_BAD_RATIO_NUM: usize = 1;
const MAX_OCC_BAD_RATIO_DEN: usize = 5;
const MIN_OCC_DELTA: isize = 2;

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
        let mut it = src.split_ascii_whitespace();
        let _: usize = read_value(&mut it);
        let k_layers = read_value(&mut it);
        let color_count = read_value(&mut it);
        let mut goal = [[0; N]; N];
        for row in &mut goal {
            for cell in row {
                *cell = read_value(&mut it);
            }
        }
        Self {
            k_layers,
            color_count,
            goal,
        }
    }

    fn nonzero_count(&self) -> usize {
        self.goal
            .iter()
            .flatten()
            .filter(|&&color| color != 0)
            .count()
    }
}

#[inline(always)]
fn read_value<T>(it: &mut SplitAsciiWhitespace<'_>) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    it.next().unwrap().parse().unwrap()
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
    fn minus_assign(&mut self, other: &Self) {
        for i in 0..WORDS {
            self.w[i] &= !other.w[i];
        }
    }

    #[inline]
    fn count(&self) -> usize {
        self.w.iter().map(|&x| x.count_ones() as usize).sum()
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
        let mut out = Vec::with_capacity(self.count());
        for (block, &bits0) in self.w.iter().enumerate() {
            let mut bits = bits0;
            while bits != 0 {
                let tz = bits.trailing_zeros() as usize;
                out.push((block << 6) + tz);
                bits &= bits - 1;
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
struct RotView {
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
    good_mask: Bits,
    bad_mask: Bits,
    good_count: usize,
    bad_count: usize,
    base_score: isize,
}

#[derive(Debug, Clone, Copy)]
struct Variant {
    seed: Option<usize>,
    limit: u8,
}

#[derive(Debug, Clone)]
struct Candidate {
    pattern: Pattern,
    rotations: Vec<RotView>,
    occurrences: Vec<Occurrence>,
    order: Vec<usize>,
    variants: Vec<Variant>,
    good_union: Bits,
    union_count: usize,
}

#[derive(Debug, Clone)]
struct CandidateStat {
    pattern: Pattern,
    hits: usize,
    total_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UseStep {
    candidate_idx: usize,
    variant_idx: usize,
}

#[derive(Debug, Clone)]
struct ExecutedStep {
    candidate_idx: usize,
    occurrence_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct EvalResult {
    score: i64,
    total_ops: usize,
    executed_steps: Vec<ExecutedStep>,
    residual: Bits,
}

#[derive(Debug, Clone)]
struct Relations {
    similar: Vec<Vec<usize>>,
    partner: Vec<Vec<usize>>,
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
}

fn main() {
    let input = Input::read();
    let ops = solve(&input);
    print!("{}", format_ops(&ops));
}

fn solve(input: &Input) -> Vec<Op> {
    let timer = Timer::new();
    let debug = std::env::var_os("V008_DEBUG").is_some();
    let baseline_ops = baseline_paint_ops(&input.goal);
    let baseline_score = score_from_ops(input.nonzero_count(), baseline_ops.len());

    let mut patterns = extract_patterns(input, &timer);
    patterns.sort_by_key(|pattern| Reverse((pattern.hint_score, pattern.cells.len())));
    if patterns.len() > MAX_CANDIDATE_POOL {
        patterns.truncate(MAX_CANDIDATE_POOL);
    }

    let mut candidates = patterns
        .into_iter()
        .map(|pattern| materialize_candidate(pattern, &input.goal))
        .filter(|cand| !cand.occurrences.is_empty() && !cand.variants.is_empty())
        .collect::<Vec<_>>();
    candidates.sort_by_key(|cand| {
        Reverse((
            cand.pattern.hint_score,
            cand.union_count,
            cand.occurrences.len(),
            cand.pattern.cells.len(),
        ))
    });
    if candidates.len() > MAX_CANDIDATES {
        candidates.truncate(MAX_CANDIDATES);
    }

    let relations = build_relations(&candidates);
    let mut rng = ChaCha20Rng::seed_from_u64(seed_from_goal(&input.goal));
    let mut current = greedy_initial_solution(input, &candidates, &timer);
    let mut current_eval = evaluate_sequence(&current, input, &candidates);
    let mut best_eval = current_eval.clone();

    if debug {
        eprintln!(
            "baseline={} candidates={} init_ops={} init_score={}",
            baseline_ops.len(),
            candidates.len(),
            current_eval.total_ops,
            current_eval.score
        );
    }

    while timer.elapsed() < SOLVER_TIME_LIMIT_SEC {
        let next = propose_neighbor(&current, &candidates, &relations, &mut rng);
        let next_eval = evaluate_sequence(&next, input, &candidates);
        let progress = ((timer.elapsed() - EXTRACTION_LIMIT_SEC).max(0.0)
            / (SOLVER_TIME_LIMIT_SEC - EXTRACTION_LIMIT_SEC).max(1e-9))
            .clamp(0.0, 1.0);
        let temp = SA_START_TEMP.powf(1.0 - progress) * SA_END_TEMP.powf(progress);
        let delta = next_eval.score - current_eval.score;
        let accept = delta >= 0 || rng.random::<f64>() < ((delta as f64) / temp).exp();
        if accept {
            current = next;
            current_eval = next_eval;
            if current_eval.score > best_eval.score
                || (current_eval.score == best_eval.score
                    && current_eval.total_ops < best_eval.total_ops)
            {
                best_eval = current_eval.clone();
                if debug {
                    eprintln!(
                        "improve score={} ops={} len={}",
                        best_eval.score,
                        best_eval.total_ops,
                        current.len()
                    );
                }
            }
        }
    }

    let final_eval = if best_eval.score >= current_eval.score {
        best_eval
    } else {
        current_eval
    };
    let ops = build_ops_from_eval(input, &candidates, &final_eval);
    if final_eval.score <= baseline_score || ops.len() > baseline_ops.len() || !validate_ops(input, &ops)
    {
        baseline_ops
    } else {
        ops
    }
}

fn validate_ops(input: &Input, ops: &[Op]) -> bool {
    if ops.len() > N * N {
        return false;
    }
    let mut state = State::new(input.k_layers);
    matches!(state.apply_all(ops), Ok(()) if state.layer0_matches(&input.goal))
}

fn score_from_ops(u: usize, t: usize) -> i64 {
    if t == 0 || t > N * N {
        return i64::MIN / 4;
    }
    let score = 1_000_000.0 * (1.0 + ((u as f64) / (t as f64)).log2());
    score.round() as i64
}

fn build_ops_from_eval(input: &Input, candidates: &[Candidate], eval: &EvalResult) -> Vec<Op> {
    let mut ops = Vec::with_capacity(eval.total_ops);
    let pattern_layer = 1usize.min(input.k_layers - 1);
    let mut loaded_candidate = None;

    for step in &eval.executed_steps {
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

fn greedy_initial_solution(input: &Input, candidates: &[Candidate], timer: &Timer) -> Vec<UseStep> {
    let mut seq = Vec::<UseStep>::new();
    let mut residual = goal_bits(&input.goal);
    let mut cost = 0usize;
    let mut loaded = None::<usize>;
    let mut residual_count = residual.count();

    while seq.len() < MAX_SEQUENCE_LEN && timer.elapsed() < EXTRACTION_LIMIT_SEC + 0.10 {
        let current_finish = cost + residual_count;
        let mut rough = candidates
            .iter()
            .enumerate()
            .map(|(idx, cand)| (residual.and_count(&cand.good_union), idx))
            .filter(|&(cover, _)| cover > 0)
            .collect::<Vec<_>>();
        rough.sort_by_key(|&(cover, idx)| Reverse((cover, idx)));
        if rough.len() > 24 {
            rough.truncate(24);
        }

        let mut best_use = None::<UseStep>;
        let mut best_occs = Vec::<usize>::new();
        let mut best_next_residual = residual;
        let mut best_next_count = residual_count;
        let mut best_finish = current_finish;

        for (_, candidate_idx) in rough {
            let candidate = &candidates[candidate_idx];
            let prep_cost = if loaded == Some(candidate_idx) {
                0
            } else {
                candidate.pattern.cells.len() + usize::from(loaded.is_some())
            };
            for variant_idx in 0..candidate.variants.len() {
                let Some((occurrence_indices, next_residual, next_count)) =
                    apply_variant(candidate, variant_idx, &residual)
                else {
                    continue;
                };
                let finish = cost + prep_cost + occurrence_indices.len() + next_count;
                if finish < best_finish {
                    best_finish = finish;
                    best_use = Some(UseStep {
                        candidate_idx,
                        variant_idx,
                    });
                    best_occs = occurrence_indices;
                    best_next_residual = next_residual;
                    best_next_count = next_count;
                }
            }
        }

        let Some(best_use) = best_use else {
            break;
        };
        if best_finish >= current_finish {
            break;
        }

        let candidate = &candidates[best_use.candidate_idx];
        let prep_cost = if loaded == Some(best_use.candidate_idx) {
            0
        } else {
            candidate.pattern.cells.len() + usize::from(loaded.is_some())
        };
        cost += prep_cost + best_occs.len();
        loaded = Some(best_use.candidate_idx);
        residual = best_next_residual;
        residual_count = best_next_count;
        seq.push(best_use);
    }
    seq
}

fn evaluate_sequence(seq: &[UseStep], input: &Input, candidates: &[Candidate]) -> EvalResult {
    let mut residual = goal_bits(&input.goal);
    let mut loaded = None::<usize>;
    let mut total_ops = 0usize;
    let mut executed_steps = Vec::<ExecutedStep>::with_capacity(seq.len());

    for step in seq {
        let candidate = &candidates[step.candidate_idx];
        let Some((occurrence_indices, next_residual, _next_count)) =
            apply_variant(candidate, step.variant_idx, &residual)
        else {
            continue;
        };
        let prep_cost = if loaded == Some(step.candidate_idx) {
            0
        } else {
            candidate.pattern.cells.len() + usize::from(loaded.is_some())
        };
        total_ops += prep_cost + occurrence_indices.len();
        if total_ops > N * N {
            return EvalResult {
                score: i64::MIN / 4,
                total_ops,
                executed_steps,
                residual,
            };
        }
        residual = next_residual;
        loaded = Some(step.candidate_idx);
        executed_steps.push(ExecutedStep {
            candidate_idx: step.candidate_idx,
            occurrence_indices,
        });
    }

    total_ops += residual.count();
    EvalResult {
        score: score_from_ops(input.nonzero_count(), total_ops),
        total_ops,
        executed_steps,
        residual,
    }
}

fn apply_variant(candidate: &Candidate, variant_idx: usize, residual: &Bits) -> Option<(Vec<usize>, Bits, usize)> {
    let variant = candidate.variants[variant_idx];
    let mut remaining = *residual;
    let mut selected = Vec::<usize>::new();
    let limit = if variant.limit == u8::MAX {
        usize::MAX
    } else {
        variant.limit as usize
    };

    if let Some(seed_idx) = variant.seed {
        let occ = &candidate.occurrences[seed_idx];
        let delta = occurrence_delta(&remaining, occ);
        if delta <= 0 {
            return None;
        }
        selected.push(seed_idx);
        apply_occurrence(&mut remaining, occ);
    }

    for &occ_idx in &candidate.order {
        if variant.seed == Some(occ_idx) {
            continue;
        }
        if selected.len() >= limit {
            break;
        }
        let occ = &candidate.occurrences[occ_idx];
        let delta = occurrence_delta(&remaining, occ);
        if delta <= 0 {
            continue;
        }
        selected.push(occ_idx);
        apply_occurrence(&mut remaining, occ);
    }

    if selected.is_empty() {
        None
    } else {
        let next_count = remaining.count();
        Some((selected, remaining, next_count))
    }
}

fn build_relations(candidates: &[Candidate]) -> Relations {
    let n = candidates.len();
    let mut similar = vec![Vec::<usize>::new(); n];
    let mut partner = vec![Vec::<usize>::new(); n];
    let sizes = candidates
        .iter()
        .map(|cand| cand.union_count as isize)
        .collect::<Vec<_>>();

    for i in 0..n {
        let mut similar_scored = Vec::<(isize, usize)>::new();
        let mut partner_scored = Vec::<(isize, usize)>::new();
        for j in 0..n {
            if i == j {
                continue;
            }
            let overlap = candidates[i].good_union.and_count(&candidates[j].good_union) as isize;
            let size_diff = (sizes[i] - sizes[j]).abs();
            let new_cover = sizes[j] - overlap;
            similar_scored.push((overlap * 4 - size_diff, j));
            partner_scored.push((new_cover * 3 - overlap, j));
        }
        similar_scored.sort_by_key(|&(score, idx)| Reverse((score, idx)));
        partner_scored.sort_by_key(|&(score, idx)| Reverse((score, idx)));
        similar[i] = similar_scored
            .into_iter()
            .take(MAX_RELATIONS)
            .map(|(_, idx)| idx)
            .collect();
        partner[i] = partner_scored
            .into_iter()
            .take(MAX_RELATIONS)
            .map(|(_, idx)| idx)
            .collect();
    }

    Relations { similar, partner }
}

fn propose_neighbor(
    current: &[UseStep],
    candidates: &[Candidate],
    relations: &Relations,
    rng: &mut ChaCha20Rng,
) -> Vec<UseStep> {
    let mut next = current.to_vec();
    let len = next.len();
    let can_insert = len < MAX_SEQUENCE_LEN;
    let can_delete = len > 0;
    let can_move = len >= 2;
    let can_duplicate = len > 0 && len < MAX_SEQUENCE_LEN;
    let r = rng.random_range(0..100);

    if can_insert && (len == 0 || r < 22) {
        let pos = rng.random_range(0..=len);
        let candidate_idx = choose_candidate_for_position(&next, pos, candidates, relations, rng);
        let variant_idx = rng.random_range(0..candidates[candidate_idx].variants.len());
        next.insert(
            pos,
            UseStep {
                candidate_idx,
                variant_idx,
            },
        );
        return next;
    }

    if can_delete && r < 36 {
        let pos = rng.random_range(0..len);
        next.remove(pos);
        return next;
    }

    if can_move && r < 54 {
        let from = rng.random_range(0..len);
        let step = next.remove(from);
        let to = rng.random_range(0..=next.len());
        next.insert(to, step);
        return next;
    }

    if can_delete && r < 72 {
        let pos = rng.random_range(0..len);
        let old = next[pos];
        let candidate_idx = choose_related_candidate(old.candidate_idx, candidates, relations, rng);
        next[pos] = UseStep {
            candidate_idx,
            variant_idx: rng.random_range(0..candidates[candidate_idx].variants.len()),
        };
        return next;
    }

    if can_delete && r < 86 {
        let pos = rng.random_range(0..len);
        let candidate_idx = next[pos].candidate_idx;
        let variants = &candidates[candidate_idx].variants;
        if variants.len() >= 2 {
            let mut variant_idx = rng.random_range(0..variants.len());
            if variant_idx == next[pos].variant_idx {
                variant_idx = (variant_idx + 1) % variants.len();
            }
            next[pos].variant_idx = variant_idx;
        }
        return next;
    }

    if can_duplicate {
        let from = rng.random_range(0..len);
        let insert_pos = rng.random_range(0..=len);
        next.insert(insert_pos, next[from]);
        return next;
    }

    next
}

fn choose_candidate_for_position(
    seq: &[UseStep],
    pos: usize,
    candidates: &[Candidate],
    relations: &Relations,
    rng: &mut ChaCha20Rng,
) -> usize {
    let mut pool = Vec::<usize>::new();
    if pos > 0 {
        let prev = seq[pos - 1].candidate_idx;
        pool.extend(relations.partner[prev].iter().copied());
        pool.extend(relations.similar[prev].iter().take(3).copied());
    }
    if pos < seq.len() {
        let next = seq[pos].candidate_idx;
        pool.extend(relations.partner[next].iter().take(5).copied());
        pool.extend(relations.similar[next].iter().take(3).copied());
    }
    if !pool.is_empty() && rng.random_bool(0.75) {
        let idx = rng.random_range(0..pool.len());
        return pool[idx];
    }
    weighted_random_candidate(candidates, rng)
}

fn choose_related_candidate(
    base: usize,
    candidates: &[Candidate],
    relations: &Relations,
    rng: &mut ChaCha20Rng,
) -> usize {
    let mut pool = Vec::<usize>::new();
    pool.extend(relations.partner[base].iter().copied());
    pool.extend(relations.similar[base].iter().copied());
    if !pool.is_empty() && rng.random_bool(0.85) {
        return pool[rng.random_range(0..pool.len())];
    }
    weighted_random_candidate(candidates, rng)
}

fn weighted_random_candidate(candidates: &[Candidate], rng: &mut ChaCha20Rng) -> usize {
    let total = candidates
        .iter()
        .map(|cand| cand.pattern.hint_score.max(1) + cand.union_count.max(1))
        .sum::<usize>();
    let mut x = rng.random_range(0..total);
    for (idx, cand) in candidates.iter().enumerate() {
        let w = cand.pattern.hint_score.max(1) + cand.union_count.max(1);
        if x < w {
            return idx;
        }
        x -= w;
    }
    candidates.len().saturating_sub(1)
}

fn extract_patterns(input: &Input, timer: &Timer) -> Vec<Pattern> {
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
                if timer.elapsed() > EXTRACTION_LIMIT_SEC {
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
                            total_size: 0,
                        });
                        entry.hits += 1;
                        entry.total_size += component.len();
                    }
                }
            }
        }
    }

    stats.into_values()
        .map(|stat| {
            let mut pattern = stat.pattern;
            pattern.hint_score = stat.total_size + stat.hits * pattern.cells.len();
            pattern
        })
        .collect()
}

fn materialize_candidate(pattern: Pattern, goal: &Grid) -> Candidate {
    let rotations = build_rotations(&pattern);
    let mut occurrences = Vec::<Occurrence>::new();
    let mut good_union = Bits::default();

    for view in &rotations {
        for top in 0..=N - view.height {
            for left in 0..=N - view.width {
                let mut good_mask = Bits::default();
                let mut bad_mask = Bits::default();
                let mut good_count = 0usize;
                let mut bad_count = 0usize;
                for cell in &pattern.cells {
                    let (ri, rj) =
                        rotate_local_coord((cell.i, cell.j), pattern.height, pattern.width, view.rot);
                    let bi = top + ri;
                    let bj = left + rj;
                    if goal[bi][bj] == cell.color {
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
                if bad_count * MAX_OCC_BAD_RATIO_DEN > pattern.cells.len() * MAX_OCC_BAD_RATIO_NUM {
                    continue;
                }
                let delta = good_count as isize - bad_count as isize;
                if delta < MIN_OCC_DELTA {
                    continue;
                }
                good_union.or_assign(&good_mask);
                occurrences.push(Occurrence {
                    rot: view.rot,
                    top,
                    left,
                    good_mask,
                    bad_mask,
                    good_count,
                    bad_count,
                    base_score: delta * 4 + good_count as isize - (bad_count as isize) * 2,
                });
            }
        }
    }

    occurrences.sort_by_key(|occ| {
        Reverse((occ.base_score, occ.good_count, usize::MAX - occ.bad_count))
    });
    if occurrences.len() > MAX_OCCURRENCES {
        occurrences.truncate(MAX_OCCURRENCES);
    }

    let mut order = (0..occurrences.len()).collect::<Vec<_>>();
    order.sort_by_key(|&idx| {
        let occ = &occurrences[idx];
        Reverse((occ.base_score, occ.good_count, usize::MAX - occ.bad_count, idx))
    });

    let variants = build_variants(&order);
    let union_count = good_union.count();
    Candidate {
        pattern,
        rotations,
        occurrences,
        order,
        variants,
        good_union,
        union_count,
    }
}

fn build_variants(order: &[usize]) -> Vec<Variant> {
    if order.is_empty() {
        return Vec::new();
    }
    let mut variants = Vec::<Variant>::new();
    let seeds = [None, Some(order[0]), order.get(1).copied()];
    let limits = [1_u8, 2, 4, 8, 16, u8::MAX];
    for seed in seeds {
        for &limit in &limits {
            if seed.is_none() && limit == 16 {
                continue;
            }
            let cand = Variant { seed, limit };
            if !variants
                .iter()
                .any(|v| v.seed == cand.seed && v.limit == cand.limit)
            {
                variants.push(cand);
            }
            if variants.len() >= 12 {
                return variants;
            }
        }
    }
    variants
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

fn occurrence_delta(residual: &Bits, occ: &Occurrence) -> isize {
    let fix = residual.and_count(&occ.good_mask) as isize;
    let damage = occ.bad_count as isize - residual.and_count(&occ.bad_mask) as isize;
    fix - damage
}

fn apply_occurrence(residual: &mut Bits, occ: &Occurrence) {
    residual.minus_assign(&occ.good_mask);
    residual.or_assign(&occ.bad_mask);
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
fn rotate_local_coord((i, j): Coord, height: usize, width: usize, rot: usize) -> Coord {
    match rot % 4 {
        0 => (i, j),
        1 => (j, height - 1 - i),
        2 => (height - 1 - i, width - 1 - j),
        3 => (width - 1 - j, i),
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

    let base_h = max_i - min_i + 1;
    let base_w = max_j - min_j + 1;
    let mut base_cells = component
        .iter()
        .map(|&(i, j)| Cell {
            i: i - min_i,
            j: j - min_j,
            color: goal[i][j],
        })
        .collect::<Vec<_>>();
    base_cells.sort();

    let mut best = rotate_pattern_cells(&base_cells, base_h, base_w, 0);
    let mut best_sig = pattern_signature(&best);
    for rot in 1..4 {
        let cand = rotate_pattern_cells(&base_cells, base_h, base_w, rot);
        let sig = pattern_signature(&cand);
        if sig < best_sig {
            best_sig = sig;
            best = cand;
        }
    }
    best
}

fn rotate_pattern_cells(cells: &[Cell], height: usize, width: usize, rot: usize) -> Pattern {
    let (rh, rw) = rotated_dims(height, width, rot);
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
        height: rh,
        width: rw,
        cells: rotated,
        hint_score: 0,
    }
}

fn pattern_signature(pattern: &Pattern) -> Vec<u16> {
    let mut sig = Vec::with_capacity(pattern.cells.len() + 2);
    sig.push(pattern.height as u16);
    sig.push(pattern.width as u16);
    for cell in &pattern.cells {
        sig.push(((cell.i as u16) << 10) | ((cell.j as u16) << 3) | cell.color as u16);
    }
    sig
}

fn build_rotations(pattern: &Pattern) -> Vec<RotView> {
    let mut views = Vec::with_capacity(4);
    for rot in 0..4 {
        let (height, width) = rotated_dims(pattern.height, pattern.width, rot);
        let mut min_board_i = N;
        let mut min_board_j = N;
        for cell in &pattern.cells {
            let (bi, bj) = rotate_coord((cell.i, cell.j), rot);
            min_board_i = min_board_i.min(bi);
            min_board_j = min_board_j.min(bj);
        }
        views.push(RotView {
            rot,
            height,
            width,
            min_board_i,
            min_board_j,
        });
    }
    views
}

fn seed_from_goal(goal: &Grid) -> u64 {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    for (i, row) in goal.iter().enumerate() {
        for (j, &color) in row.iter().enumerate() {
            let x = ((i * N + j) as u64) << 8 | color as u64;
            seed ^= x.wrapping_add(0x9e37_79b9_7f4a_7c15).rotate_left((x & 31) as u32);
            seed = seed.rotate_left(7).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        }
    }
    seed
}
