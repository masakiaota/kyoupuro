// v007_multilayer_prep_beam.rs
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
const MAX_CANDIDATES: usize = 256;
const MAX_TOP_OCCS: usize = 384;
const MAX_SEED_OCCS: usize = 16;
const MAX_VARIANTS_PER_CANDIDATE: usize = 20;
const MIN_OCC_GAIN: usize = 2;
const SOLVER_TIME_LIMIT_SEC: f64 = 1.92;
const MAX_OCC_BAD: usize = 4;
const MAX_OCC_BAD_RATIO_NUM: usize = 1;
const MAX_OCC_BAD_RATIO_DEN: usize = 5;
const MAX_PREP_ROOT_PATTERNS: usize = 48;
const MAX_PREP_CHILD_PATTERNS: usize = 24;
const MAX_PREP_PATTERNS_PER_PARENT: usize = 8;
const MAX_REPO_PATTERNS: usize = 384;
const MAX_PREP_SCAN_PATTERNS: usize = 64;
const MAX_PREP_RECIPES_PER_PATTERN: usize = 4;
const MAX_LOAD_PLANS_PER_PATTERN: usize = 6;
const MAX_MULTI_SCAN_CANDIDATES: usize = 48;
const MAX_MULTI_BRANCHES_PER_STATE: usize = 40;
const MAX_MULTI_BEAM_WIDTH: usize = 192;
const MAX_MULTI_BEAM_DEPTH: usize = 14;
const MIN_PREP_GAIN: usize = 2;

type Color = u8;
type Grid = [[Color; N]; N];
type Coord = (usize, usize);
type PatternId = usize;

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

    #[inline]
    fn contains(&self, p: usize) -> bool {
        ((self.w[p >> 6] >> (p & 63)) & 1) != 0
    }

    #[cfg(test)]
    #[inline]
    fn difference(&self, other: &Self) -> Self {
        let mut res = *self;
        res.minus_assign(other);
        res
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
    good_mask: Bits,
    bad_mask: Bits,
    good_count: usize,
    bad_count: usize,
}

#[derive(Debug, Clone)]
struct Candidate {
    pattern: Pattern,
    rotations: Vec<RotatedPattern>,
    occurrences: Vec<Occurrence>,
    good_union_mask: Bits,
}

#[derive(Debug, Clone)]
struct CandidateStat {
    pattern: Pattern,
    hits: usize,
    total_component_size: usize,
}

#[derive(Debug, Clone)]
struct Transition {
    occurrence_indices: Vec<usize>,
    next_residual: Bits,
    next_residual_count: usize,
    transition_cost: usize,
    gain: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternKind {
    Sprayable { candidate_idx: usize },
    PrepOnly,
}

#[derive(Debug, Clone)]
struct LocalOccurrence {
    rot: usize,
    top: usize,
    left: usize,
    mask: Bits,
}

#[derive(Debug, Clone)]
struct PrepRecipe {
    src_pattern_id: PatternId,
    occurrences: Vec<LocalOccurrence>,
    residual_cells: Vec<Cell>,
    build_cost: usize,
    gain: usize,
}

#[derive(Debug, Clone)]
struct PatternRepoEntry {
    pattern: Pattern,
    rotations: Vec<RotatedPattern>,
    kind: PatternKind,
    prep_recipes: Vec<PrepRecipe>,
}

#[derive(Debug, Clone)]
struct PatternRepo {
    entries: Vec<PatternRepoEntry>,
    candidate_to_pattern_id: Vec<PatternId>,
}

#[derive(Debug, Clone)]
enum BeamAction {
    Load {
        dst_layer: usize,
        pattern_id: PatternId,
        recipe_idx: Option<usize>,
        src_layer: Option<usize>,
    },
    Spray {
        src_layer: usize,
        candidate_idx: usize,
        occurrence_indices: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
struct MultiBeamState {
    residual: Bits,
    residual_count: usize,
    work_layers: Vec<Option<PatternId>>,
    cost: usize,
    actions: Vec<BeamAction>,
}

#[derive(Debug, Clone)]
struct LoadPlan {
    actions: Vec<BeamAction>,
    next_work_layers: Vec<Option<PatternId>>,
    src_layer: usize,
    prep_cost: usize,
}

#[derive(Debug, Clone)]
struct MultiTransition {
    actions: Vec<BeamAction>,
    next_work_layers: Vec<Option<PatternId>>,
    next_residual: Bits,
    next_residual_count: usize,
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
    let debug = std::env::var_os("V007_DEBUG").is_some();

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

    let repo = build_pattern_repo(&candidates, &mut time_keeper, debug);
    let ops = build_multilayer_plan(
        input,
        &candidates,
        &repo,
        baseline_cost,
        &mut time_keeper,
        debug,
    );
    if ops.len() <= baseline_cost && validate_ops(input, &ops) {
        ops
    } else {
        baseline_ops
    }
}

fn validate_ops(input: &Input, ops: &[Op]) -> bool {
    if ops.len() > N * N {
        return false;
    }
    let mut state = State::new(input.k_layers);
    matches!(state.apply_all(ops), Ok(()) if state.layer0_matches(&input.goal))
}

fn build_multilayer_plan(
    input: &Input,
    candidates: &[Candidate],
    repo: &PatternRepo,
    baseline_cost: usize,
    time_keeper: &mut TimeKeeper,
    debug: bool,
) -> Vec<Op> {
    let initial = MultiBeamState {
        residual: goal_bits(&input.goal),
        residual_count: baseline_cost,
        work_layers: vec![None; input.k_layers.saturating_sub(1)],
        cost: 0,
        actions: Vec::new(),
    };

    let mut best_state = initial.clone();
    let mut best_total_cost = baseline_cost;
    let mut beam = vec![initial];

    for depth in 0..MAX_MULTI_BEAM_DEPTH {
        if beam.is_empty() || time_keeper.is_over() {
            break;
        }

        let mut next = Vec::<MultiBeamState>::new();
        let mut seen = FxHashMap::<(Bits, [i16; 4]), usize>::default();

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
            let transitions =
                enumerate_multilayer_transitions(&state, candidates, repo, time_keeper);
            for tr in transitions.into_iter().take(MAX_MULTI_BRANCHES_PER_STATE) {
                let next_cost = state.cost + tr.transition_cost;
                let next_total_bound = next_cost + tr.next_residual_count;
                if next_total_bound >= best_total_cost || next_cost > N * N {
                    continue;
                }
                let seen_key = (tr.next_residual, work_layers_key(&tr.next_work_layers));
                let keep = match seen.get(&seen_key) {
                    None => true,
                    Some(&best_cost) => next_cost < best_cost,
                };
                if !keep {
                    continue;
                }
                seen.insert(seen_key, next_cost);

                let mut next_actions = state.actions.clone();
                next_actions.extend(tr.actions);
                next.push(MultiBeamState {
                    residual: tr.next_residual,
                    residual_count: tr.next_residual_count,
                    work_layers: tr.next_work_layers,
                    cost: next_cost,
                    actions: next_actions,
                });
            }
        }

        next.sort_by_key(|state| {
            (
                state.cost + state.residual_count,
                state.cost,
                state.residual_count,
                state.actions.len(),
            )
        });
        if next.len() > MAX_MULTI_BEAM_WIDTH {
            next.truncate(MAX_MULTI_BEAM_WIDTH);
        }

        if debug {
            eprintln!(
                "multi depth={} beam={} best={}",
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
            "multi final best_total={} actions={}",
            best_total_cost,
            best_state.actions.len()
        );
    }

    state_to_ops_multilayer(input, candidates, repo, &best_state)
}

fn enumerate_multilayer_transitions(
    state: &MultiBeamState,
    candidates: &[Candidate],
    repo: &PatternRepo,
    time_keeper: &mut TimeKeeper,
) -> Vec<MultiTransition> {
    let mut rough = Vec::<(usize, usize)>::new();
    for (candidate_idx, candidate) in candidates.iter().enumerate() {
        let potential_cover = state.residual.and_count(&candidate.good_union_mask);
        if potential_cover <= 1 {
            continue;
        }
        let pattern_id = repo.candidate_to_pattern_id[candidate_idx];
        let prep_cost_lb = estimate_prep_cost(state, repo, pattern_id);
        if potential_cover <= prep_cost_lb + 1 {
            continue;
        }
        rough.push((potential_cover - prep_cost_lb, candidate_idx));
    }
    rough.sort_by_key(|&(score, candidate_idx)| Reverse((score, candidate_idx)));
    if rough.len() > MAX_MULTI_SCAN_CANDIDATES {
        rough.truncate(MAX_MULTI_SCAN_CANDIDATES);
    }

    let mut transitions = Vec::<MultiTransition>::new();
    let mut seen = FxHashMap::<(Bits, [i16; 4]), usize>::default();

    for (_, candidate_idx) in rough {
        if time_keeper.is_over() {
            break;
        }
        let candidate = &candidates[candidate_idx];
        let load_plans = enumerate_candidate_use_plans(state, repo, candidate_idx);
        if load_plans.is_empty() {
            continue;
        }
        let spray_variants = build_candidate_variants(candidate_idx, candidate, &state.residual, 0);
        for load_plan in &load_plans {
            for variant in &spray_variants {
                let transition_cost = load_plan.prep_cost + variant.transition_cost;
                let improvement = state.residual_count as isize - variant.next_residual_count as isize;
                let gain = improvement - transition_cost as isize;
                if gain <= 0 {
                    continue;
                }

                let mut actions = load_plan.actions.clone();
                actions.push(BeamAction::Spray {
                    src_layer: load_plan.src_layer,
                    candidate_idx,
                    occurrence_indices: variant.occurrence_indices.clone(),
                });
                let seen_key = (variant.next_residual, work_layers_key(&load_plan.next_work_layers));
                let keep = match seen.get(&seen_key) {
                    None => true,
                    Some(&best_cost) => transition_cost < best_cost,
                };
                if !keep {
                    continue;
                }
                seen.insert(seen_key, transition_cost);

                transitions.push(MultiTransition {
                    actions,
                    next_work_layers: load_plan.next_work_layers.clone(),
                    next_residual: variant.next_residual,
                    next_residual_count: variant.next_residual_count,
                    transition_cost,
                    gain,
                });
            }
        }
    }

    transitions.sort_by_key(|tr| {
        Reverse((
            tr.gain,
            usize::MAX - tr.next_residual_count,
            usize::MAX - tr.transition_cost,
            tr.actions.len(),
        ))
    });
    transitions
}

fn estimate_prep_cost(state: &MultiBeamState, repo: &PatternRepo, pattern_id: PatternId) -> usize {
    if state.work_layers.iter().any(|&loaded| loaded == Some(pattern_id)) {
        return 0;
    }
    let mut best = repo.entries[pattern_id].pattern.cells.len();
    for recipe in &repo.entries[pattern_id].prep_recipes {
        if state
            .work_layers
            .iter()
            .any(|&loaded| loaded == Some(recipe.src_pattern_id))
        {
            best = best.min(recipe.build_cost);
            continue;
        }
        best = best.min(recipe.build_cost + repo.entries[recipe.src_pattern_id].pattern.cells.len());
        for src_recipe in &repo.entries[recipe.src_pattern_id].prep_recipes {
            if state
                .work_layers
                .iter()
                .any(|&loaded| loaded == Some(src_recipe.src_pattern_id))
            {
                best = best.min(recipe.build_cost + src_recipe.build_cost);
            }
        }
    }
    best
}

fn enumerate_candidate_use_plans(
    state: &MultiBeamState,
    repo: &PatternRepo,
    candidate_idx: usize,
) -> Vec<LoadPlan> {
    let pattern_id = repo.candidate_to_pattern_id[candidate_idx];
    if let Some(src_layer) = state
        .work_layers
        .iter()
        .position(|&loaded| loaded == Some(pattern_id))
    {
        return vec![LoadPlan {
            actions: Vec::new(),
            next_work_layers: state.work_layers.clone(),
            src_layer,
            prep_cost: 0,
        }];
    }

    let mut plans = Vec::<LoadPlan>::new();
    let entry = &repo.entries[pattern_id];

    for dst_layer in 0..state.work_layers.len() {
        let prep_cost = clear_cost(state.work_layers[dst_layer]) + entry.pattern.cells.len();
        let mut next_work_layers = state.work_layers.clone();
        next_work_layers[dst_layer] = Some(pattern_id);
        plans.push(LoadPlan {
            actions: vec![BeamAction::Load {
                dst_layer,
                pattern_id,
                recipe_idx: None,
                src_layer: None,
            }],
            next_work_layers,
            src_layer: dst_layer,
            prep_cost,
        });
    }

    plans.extend(enumerate_load_plans_from_loaded_sources(
        state,
        repo,
        pattern_id,
        &state.work_layers,
        Vec::new(),
        0,
    ));

    if state.work_layers.len() >= 2 {
        for (recipe_idx, recipe) in entry.prep_recipes.iter().enumerate() {
            if state
                .work_layers
                .iter()
                .any(|&loaded| loaded == Some(recipe.src_pattern_id))
            {
                continue;
            }
            let src_plans = enumerate_load_plans_depth1(state, repo, recipe.src_pattern_id);
            for src_plan in src_plans {
                for dst_layer in 0..src_plan.next_work_layers.len() {
                    if dst_layer == src_plan.src_layer {
                        continue;
                    }
                    let mut actions = src_plan.actions.clone();
                    actions.push(BeamAction::Load {
                        dst_layer,
                        pattern_id,
                        recipe_idx: Some(recipe_idx),
                        src_layer: Some(src_plan.src_layer),
                    });
                    let mut next_work_layers = src_plan.next_work_layers.clone();
                    let prep_cost = src_plan.prep_cost
                        + clear_cost(next_work_layers[dst_layer])
                        + recipe.build_cost;
                    next_work_layers[dst_layer] = Some(pattern_id);
                    plans.push(LoadPlan {
                        actions,
                        next_work_layers,
                        src_layer: dst_layer,
                        prep_cost,
                    });
                }
            }
        }
    }

    dedup_and_truncate_load_plans(plans)
}

fn enumerate_load_plans_depth1(
    state: &MultiBeamState,
    repo: &PatternRepo,
    pattern_id: PatternId,
) -> Vec<LoadPlan> {
    if let Some(src_layer) = state
        .work_layers
        .iter()
        .position(|&loaded| loaded == Some(pattern_id))
    {
        return vec![LoadPlan {
            actions: Vec::new(),
            next_work_layers: state.work_layers.clone(),
            src_layer,
            prep_cost: 0,
        }];
    }

    let entry = &repo.entries[pattern_id];
    let mut plans = Vec::<LoadPlan>::new();
    for dst_layer in 0..state.work_layers.len() {
        let prep_cost = clear_cost(state.work_layers[dst_layer]) + entry.pattern.cells.len();
        let mut next_work_layers = state.work_layers.clone();
        next_work_layers[dst_layer] = Some(pattern_id);
        plans.push(LoadPlan {
            actions: vec![BeamAction::Load {
                dst_layer,
                pattern_id,
                recipe_idx: None,
                src_layer: None,
            }],
            next_work_layers,
            src_layer: dst_layer,
            prep_cost,
        });
    }
    plans.extend(enumerate_load_plans_from_loaded_sources(
        state,
        repo,
        pattern_id,
        &state.work_layers,
        Vec::new(),
        0,
    ));
    dedup_and_truncate_load_plans(plans)
}

fn enumerate_load_plans_from_loaded_sources(
    state: &MultiBeamState,
    repo: &PatternRepo,
    pattern_id: PatternId,
    work_layers: &[Option<PatternId>],
    prefix_actions: Vec<BeamAction>,
    prefix_cost: usize,
) -> Vec<LoadPlan> {
    let mut plans = Vec::new();
    for (recipe_idx, recipe) in repo.entries[pattern_id].prep_recipes.iter().enumerate() {
        for (src_layer, &loaded) in work_layers.iter().enumerate() {
            if loaded != Some(recipe.src_pattern_id) {
                continue;
            }
            for dst_layer in 0..work_layers.len() {
                if dst_layer == src_layer {
                    continue;
                }
                let mut actions = prefix_actions.clone();
                actions.push(BeamAction::Load {
                    dst_layer,
                    pattern_id,
                    recipe_idx: Some(recipe_idx),
                    src_layer: Some(src_layer),
                });
                let mut next_work_layers = work_layers.to_vec();
                let prep_cost =
                    prefix_cost + clear_cost(next_work_layers[dst_layer]) + recipe.build_cost;
                next_work_layers[dst_layer] = Some(pattern_id);
                plans.push(LoadPlan {
                    actions,
                    next_work_layers,
                    src_layer: dst_layer,
                    prep_cost,
                });
            }
        }
    }
    let _ = state;
    plans
}

fn dedup_and_truncate_load_plans(plans: Vec<LoadPlan>) -> Vec<LoadPlan> {
    let mut best = FxHashMap::<([i16; 4], i16), usize>::default();
    let mut deduped = Vec::<LoadPlan>::new();
    for plan in plans {
        let key = (
            work_layers_key(&plan.next_work_layers),
            plan.src_layer as i16,
        );
        let keep = match best.get(&key) {
            None => true,
            Some(&best_cost) => plan.prep_cost < best_cost,
        };
        if !keep {
            continue;
        }
        best.insert(key, plan.prep_cost);
        deduped.push(plan);
    }
    deduped.sort_by_key(|plan| (plan.prep_cost, plan.actions.len(), plan.src_layer));
    if deduped.len() > MAX_LOAD_PLANS_PER_PATTERN {
        deduped.truncate(MAX_LOAD_PLANS_PER_PATTERN);
    }
    deduped
}

fn clear_cost(loaded: Option<PatternId>) -> usize {
    usize::from(loaded.is_some())
}

fn work_layers_key(work_layers: &[Option<PatternId>]) -> [i16; 4] {
    let mut key = [-1_i16; 4];
    for (idx, &loaded) in work_layers.iter().enumerate() {
        key[idx] = loaded.map(|pattern_id| pattern_id as i16).unwrap_or(-1);
    }
    key
}

fn build_pattern_repo(
    candidates: &[Candidate],
    time_keeper: &mut TimeKeeper,
    debug: bool,
) -> PatternRepo {
    let mut entries = Vec::<PatternRepoEntry>::new();
    let mut signature_to_id = FxHashMap::<Vec<u16>, PatternId>::default();
    let mut candidate_to_pattern_id = vec![0; candidates.len()];

    for (candidate_idx, candidate) in candidates.iter().enumerate() {
        let pattern_id = add_repo_entry(
            &mut entries,
            &mut signature_to_id,
            candidate.pattern.clone(),
            PatternKind::Sprayable { candidate_idx },
        );
        candidate_to_pattern_id[candidate_idx] = pattern_id;
    }

    let root_pattern_ids = candidate_to_pattern_id
        .iter()
        .copied()
        .take(MAX_PREP_ROOT_PATTERNS)
        .collect::<Vec<_>>();
    let mut first_level_new = Vec::<PatternId>::new();
    for &pattern_id in &root_pattern_ids {
        if time_keeper.is_over() || entries.len() >= MAX_REPO_PATTERNS {
            break;
        }
        let mut subs = extract_local_subpatterns(&entries[pattern_id].pattern, time_keeper);
        subs.sort_by_key(|pattern| Reverse((pattern.hint_score, pattern.cells.len())));
        for pattern in subs.into_iter().take(MAX_PREP_PATTERNS_PER_PARENT) {
            if pattern.cells.len() + MIN_PREP_GAIN > entries[pattern_id].pattern.cells.len() {
                continue;
            }
            let old_len = entries.len();
            let next_id = add_repo_entry(
                &mut entries,
                &mut signature_to_id,
                pattern,
                PatternKind::PrepOnly,
            );
            if entries.len() > old_len {
                first_level_new.push(next_id);
            }
            if entries.len() >= MAX_REPO_PATTERNS {
                break;
            }
        }
    }

    for &pattern_id in first_level_new.iter().take(MAX_PREP_CHILD_PATTERNS) {
        if time_keeper.is_over() || entries.len() >= MAX_REPO_PATTERNS {
            break;
        }
        let mut subs = extract_local_subpatterns(&entries[pattern_id].pattern, time_keeper);
        subs.sort_by_key(|pattern| Reverse((pattern.hint_score, pattern.cells.len())));
        for pattern in subs.into_iter().take(MAX_PREP_PATTERNS_PER_PARENT / 2 + 1) {
            if pattern.cells.len() + MIN_PREP_GAIN > entries[pattern_id].pattern.cells.len() {
                continue;
            }
            add_repo_entry(
                &mut entries,
                &mut signature_to_id,
                pattern,
                PatternKind::PrepOnly,
            );
            if entries.len() >= MAX_REPO_PATTERNS {
                break;
            }
        }
    }

    let mut pattern_order = (0..entries.len()).collect::<Vec<_>>();
    pattern_order.sort_by_key(|&pattern_id| {
        (
            entries[pattern_id].pattern.cells.len(),
            entries[pattern_id].pattern.hint_score,
        )
    });

    for &target_id in &pattern_order {
        if time_keeper.is_over() {
            break;
        }
        let target_size = entries[target_id].pattern.cells.len();
        let mut source_ids = pattern_order
            .iter()
            .copied()
            .filter(|&source_id| {
                source_id != target_id
                    && entries[source_id].pattern.cells.len() + MIN_PREP_GAIN <= target_size
            })
            .collect::<Vec<_>>();
        source_ids.sort_by_key(|&source_id| {
            Reverse((
                entries[source_id].pattern.cells.len(),
                entries[source_id].pattern.hint_score,
            ))
        });
        if source_ids.len() > MAX_PREP_SCAN_PATTERNS {
            source_ids.truncate(MAX_PREP_SCAN_PATTERNS);
        }

        let mut recipes = Vec::<PrepRecipe>::new();
        for source_id in source_ids {
            if time_keeper.is_over() {
                break;
            }
            if let Some(recipe) =
                build_prep_recipe(source_id, &entries[source_id].pattern, &entries[target_id].pattern)
            {
                recipes.push(recipe);
            }
        }
        recipes.sort_by_key(|recipe| {
            Reverse((
                recipe.gain,
                entries[recipe.src_pattern_id].pattern.cells.len(),
                usize::MAX - recipe.build_cost,
            ))
        });
        if recipes.len() > MAX_PREP_RECIPES_PER_PATTERN {
            recipes.truncate(MAX_PREP_RECIPES_PER_PATTERN);
        }
        entries[target_id].prep_recipes = recipes;
    }

    if debug {
        let prep_only = entries
            .iter()
            .filter(|entry| matches!(entry.kind, PatternKind::PrepOnly))
            .count();
        eprintln!(
            "repo entries={} prep_only={}",
            entries.len(),
            prep_only
        );
    }

    PatternRepo {
        entries,
        candidate_to_pattern_id,
    }
}

fn add_repo_entry(
    entries: &mut Vec<PatternRepoEntry>,
    signature_to_id: &mut FxHashMap<Vec<u16>, PatternId>,
    pattern: Pattern,
    kind: PatternKind,
) -> PatternId {
    let signature = pattern_signature(&pattern);
    if let Some(&pattern_id) = signature_to_id.get(&signature) {
        if let PatternKind::Sprayable { candidate_idx } = kind {
            entries[pattern_id].kind = PatternKind::Sprayable { candidate_idx };
        }
        return pattern_id;
    }

    let pattern_id = entries.len();
    entries.push(PatternRepoEntry {
        rotations: build_rotations(&pattern),
        pattern,
        kind,
        prep_recipes: Vec::new(),
    });
    signature_to_id.insert(signature, pattern_id);
    pattern_id
}

fn extract_local_subpatterns(pattern: &Pattern, time_keeper: &mut TimeKeeper) -> Vec<Pattern> {
    let h = pattern.height;
    let w = pattern.width;
    if pattern.cells.len() < MIN_PATTERN_SIZE * 2 || h == 0 || w == 0 {
        return Vec::new();
    }

    let grid = pattern_to_local_grid(pattern);
    let mut matching = vec![vec![false; w]; h];
    let mut visited = vec![vec![false; w]; h];
    let mut queue = Vec::<Coord>::with_capacity(h * w);
    let mut component = Vec::<Coord>::with_capacity(h * w);
    let mut stats = FxHashMap::<Vec<u16>, CandidateStat>::default();

    'outer: for rot in 0..4 {
        let (rh, rw) = rotated_dims(h, w, rot);
        for di in -(rh as isize) + 1..=(h as isize) - 1 {
            for dj in -(rw as isize) + 1..=(w as isize) - 1 {
                if rot == 0 && di == 0 && dj == 0 {
                    continue;
                }
                if !time_keeper.step() {
                    break 'outer;
                }

                let mut match_count = 0usize;
                for i in 0..h {
                    for j in 0..w {
                        let ok = grid[i][j] != 0
                            && transformed_local_coord((i, j), h, w, rot, di, dj, h, w)
                                .map(|(ni, nj)| grid[ni][nj] == grid[i][j])
                                .unwrap_or(false);
                        matching[i][j] = ok;
                        visited[i][j] = false;
                        match_count += usize::from(ok);
                    }
                }
                if match_count < MIN_PATTERN_SIZE {
                    continue;
                }

                for si in 0..h {
                    for sj in 0..w {
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
                                if !(0..h as isize).contains(&ni) || !(0..w as isize).contains(&nj)
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

                        if component.len() < MIN_PATTERN_SIZE || component.len() >= pattern.cells.len()
                        {
                            continue;
                        }

                        let local_pattern = canonicalize_local_component(&grid, &component);
                        let key = pattern_signature(&local_pattern);
                        let entry = stats.entry(key).or_insert_with(|| CandidateStat {
                            pattern: local_pattern.clone(),
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

fn build_prep_recipe(
    src_pattern_id: PatternId,
    source: &Pattern,
    target: &Pattern,
) -> Option<PrepRecipe> {
    let occurrences = find_exact_local_occurrences(source, target);
    if occurrences.len() < 2 {
        return None;
    }

    let target_mask = pattern_local_mask(target);
    let mut covered = Bits::default();
    let mut selected = Vec::<LocalOccurrence>::new();

    loop {
        let mut best_idx = None;
        let mut best_gain = 0usize;
        for (idx, occurrence) in occurrences.iter().enumerate() {
            let gain = uncovered_count(&occurrence.mask, &covered);
            if gain > best_gain {
                best_gain = gain;
                best_idx = Some(idx);
            }
        }
        if best_gain <= 1 {
            break;
        }
        let idx = best_idx.unwrap();
        covered.or_assign(&occurrences[idx].mask);
        selected.push(occurrences[idx].clone());
    }

    if selected.len() < 2 {
        return None;
    }

    let mut residual_cells = Vec::<Cell>::new();
    for cell in &target.cells {
        let p = cell.i * N + cell.j;
        if target_mask.contains(p) && !covered.contains(p) {
            residual_cells.push(*cell);
        }
    }
    let build_cost = selected.len() + residual_cells.len();
    let gain = target.cells.len().saturating_sub(build_cost);
    if gain < MIN_PREP_GAIN {
        return None;
    }

    Some(PrepRecipe {
        src_pattern_id,
        occurrences: selected,
        residual_cells,
        build_cost,
        gain,
    })
}

fn pattern_to_local_grid(pattern: &Pattern) -> Vec<Vec<Color>> {
    let mut grid = vec![vec![0; pattern.width]; pattern.height];
    for cell in &pattern.cells {
        grid[cell.i][cell.j] = cell.color;
    }
    grid
}

fn canonicalize_local_component(grid: &[Vec<Color>], component: &[Coord]) -> Pattern {
    let mut min_i = usize::MAX;
    let mut max_i = 0usize;
    let mut min_j = usize::MAX;
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
            color: grid[i][j],
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

fn transformed_local_coord(
    (i, j): Coord,
    src_h: usize,
    src_w: usize,
    rot: usize,
    di: isize,
    dj: isize,
    dst_h: usize,
    dst_w: usize,
) -> Option<Coord> {
    let (ri, rj) = rotate_local_coord((i, j), src_h, src_w, rot);
    let ni = ri as isize + di;
    let nj = rj as isize + dj;
    if (0..dst_h as isize).contains(&ni) && (0..dst_w as isize).contains(&nj) {
        Some((ni as usize, nj as usize))
    } else {
        None
    }
}

fn find_exact_local_occurrences(source: &Pattern, target: &Pattern) -> Vec<LocalOccurrence> {
    if source.cells.len() + MIN_PREP_GAIN > target.cells.len() {
        return Vec::new();
    }
    let target_grid = pattern_to_local_grid(target);
    let mut occurrences = Vec::<LocalOccurrence>::new();
    for rot in 0..4 {
        let (rh, rw) = rotated_dims(source.height, source.width, rot);
        if rh > target.height || rw > target.width {
            continue;
        }
        for top in 0..=target.height - rh {
            for left in 0..=target.width - rw {
                let mut mask = Bits::default();
                let mut ok = true;
                for cell in &source.cells {
                    let (ri, rj) =
                        rotate_local_coord((cell.i, cell.j), source.height, source.width, rot);
                    let ti = top + ri;
                    let tj = left + rj;
                    if target_grid[ti][tj] != cell.color {
                        ok = false;
                        break;
                    }
                    mask.set(ti * N + tj);
                }
                if ok {
                    occurrences.push(LocalOccurrence { rot, top, left, mask });
                }
            }
        }
    }
    occurrences
}

fn pattern_local_mask(pattern: &Pattern) -> Bits {
    let mut mask = Bits::default();
    for cell in &pattern.cells {
        mask.set(cell.i * N + cell.j);
    }
    mask
}

fn uncovered_count(mask: &Bits, covered: &Bits) -> usize {
    let mut count = 0usize;
    for i in 0..WORDS {
        count += (mask.w[i] & !covered.w[i]).count_ones() as usize;
    }
    count
}

fn build_candidate_variants(
    _candidate_idx: usize,
    candidate: &Candidate,
    residual: &Bits,
    base_cost: usize,
) -> Vec<Transition> {
    let mut scored = candidate
        .occurrences
        .iter()
        .enumerate()
        .map(|(idx, occ)| {
            let fix = residual.and_count(&occ.good_mask);
            let damage = non_residual_damage_count(residual, &occ.bad_mask);
            let delta = fix as isize - damage as isize;
            (delta, occ.good_count, occ.bad_count, idx)
        })
        .filter(|&(delta, _, _, _)| delta >= MIN_OCC_GAIN as isize)
        .collect::<Vec<_>>();
    scored.sort_by_key(|&(delta, good_count, bad_count, idx)| {
        Reverse((delta, good_count, usize::MAX - bad_count, idx))
    });
    if scored.len() > MAX_TOP_OCCS {
        scored.truncate(MAX_TOP_OCCS);
    }
    if scored.is_empty() {
        return Vec::new();
    }

    let mut seed_list = Vec::with_capacity(MAX_SEED_OCCS + 1);
    seed_list.push(None);
    for &(_, _, _, idx) in scored.iter().take(MAX_SEED_OCCS) {
        seed_list.push(Some(idx));
    }

    let mut variants = Vec::<Transition>::new();
    let mut seen = FxHashMap::<Bits, usize>::default();

    for seed in seed_list {
        for (occurrence_indices, next_residual, next_residual_count) in
            greedy_occurrence_prefixes(candidate, residual, &scored, seed)
        {
            let transition_cost = base_cost + occurrence_indices.len();
            let improvement = residual.count() as isize - next_residual_count as isize;
            let gain = improvement - transition_cost as isize;
            if gain <= 0 {
                continue;
            }

            let copies = occurrence_indices.len();
            let keep = match seen.get(&next_residual) {
                None => true,
                Some(&best_copies) => copies < best_copies,
            };
            if !keep {
                continue;
            }
            seen.insert(next_residual, copies);

            variants.push(Transition {
                occurrence_indices,
                next_residual,
                next_residual_count,
                transition_cost,
                gain,
            });
        }
    }

    variants.sort_by_key(|tr| {
        Reverse((
            tr.gain,
            usize::MAX - tr.next_residual_count,
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
    scored: &[(isize, usize, usize, usize)],
    seed: Option<usize>,
) -> Vec<(Vec<usize>, Bits, usize)> {
    let mut remaining = *residual;
    let mut selected = Vec::new();
    let mut variants = Vec::new();
    let mut next_checkpoint_idx = 0usize;
    const CHECKPOINTS: [usize; 16] = [
        1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 80, 96, 128, 160,
    ];

    if let Some(seed_idx) = seed {
        let occ = &candidate.occurrences[seed_idx];
        let delta = residual_delta(&remaining, occ);
        if delta < MIN_OCC_GAIN as isize {
            return Vec::new();
        }
        selected.push(seed_idx);
        apply_occurrence_to_residual(&mut remaining, occ);
        variants.push((selected.clone(), remaining, remaining.count()));
        while next_checkpoint_idx < CHECKPOINTS.len()
            && CHECKPOINTS[next_checkpoint_idx] <= selected.len()
        {
            next_checkpoint_idx += 1;
        }
    }

    for &(_, _, _, occ_idx) in scored {
        if seed == Some(occ_idx) {
            continue;
        }
        let occ = &candidate.occurrences[occ_idx];
        let delta = residual_delta(&remaining, occ);
        if delta <= 0 {
            continue;
        }
        selected.push(occ_idx);
        apply_occurrence_to_residual(&mut remaining, occ);

        if next_checkpoint_idx < CHECKPOINTS.len()
            && CHECKPOINTS[next_checkpoint_idx] == selected.len()
        {
            variants.push((selected.clone(), remaining, remaining.count()));
            next_checkpoint_idx += 1;
        }
    }

    if !selected.is_empty() {
        let needs_final = variants
            .last()
            .map(|(occurrence_indices, _, _)| occurrence_indices.len() != selected.len())
            .unwrap_or(true);
        if needs_final {
            variants.push((selected, remaining, remaining.count()));
        }
    }

    variants
}

fn state_to_ops_multilayer(
    input: &Input,
    candidates: &[Candidate],
    repo: &PatternRepo,
    state: &MultiBeamState,
) -> Vec<Op> {
    let mut ops = Vec::with_capacity(state.cost + state.residual_count);
    let mut work_layers = vec![None; input.k_layers.saturating_sub(1)];

    for action in &state.actions {
        match action {
            BeamAction::Load {
                dst_layer,
                pattern_id,
                recipe_idx,
                src_layer,
            } => {
                let actual_dst = dst_layer + 1;
                if work_layers[*dst_layer].is_some() {
                    ops.push(Op::Clear { k: actual_dst });
                }
                let entry = &repo.entries[*pattern_id];
                match recipe_idx {
                    None => {
                        for cell in &entry.pattern.cells {
                            ops.push(Op::Paint {
                                k: actual_dst,
                                i: cell.i,
                                j: cell.j,
                                color: cell.color,
                            });
                        }
                    }
                    Some(recipe_idx) => {
                        let recipe = &entry.prep_recipes[*recipe_idx];
                        let actual_src = src_layer.unwrap() + 1;
                        let src_entry = &repo.entries[recipe.src_pattern_id];
                        for occurrence in &recipe.occurrences {
                            let view = &src_entry.rotations[occurrence.rot];
                            ops.push(Op::Copy {
                                k: actual_dst,
                                h: actual_src,
                                transform: Transform::new(
                                    occurrence.rot,
                                    occurrence.top as isize - view.min_board_i as isize,
                                    occurrence.left as isize - view.min_board_j as isize,
                                ),
                            });
                        }
                        for cell in &recipe.residual_cells {
                            ops.push(Op::Paint {
                                k: actual_dst,
                                i: cell.i,
                                j: cell.j,
                                color: cell.color,
                            });
                        }
                    }
                }
                work_layers[*dst_layer] = Some(*pattern_id);
            }
            BeamAction::Spray {
                src_layer,
                candidate_idx,
                occurrence_indices,
            } => {
                let actual_src = src_layer + 1;
                let candidate = &candidates[*candidate_idx];
                for &occ_idx in occurrence_indices {
                    let occ = &candidate.occurrences[occ_idx];
                    let view = &candidate.rotations[occ.rot];
                    ops.push(Op::Copy {
                        k: 0,
                        h: actual_src,
                        transform: Transform::new(
                            occ.rot,
                            occ.top as isize - view.min_board_i as isize,
                            occ.left as isize - view.min_board_j as isize,
                        ),
                    });
                }
            }
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

fn residual_delta(residual: &Bits, occ: &Occurrence) -> isize {
    let fix = residual.and_count(&occ.good_mask) as isize;
    let damage = non_residual_damage_count(residual, &occ.bad_mask) as isize;
    fix - damage
}

fn non_residual_damage_count(residual: &Bits, bad_mask: &Bits) -> usize {
    bad_mask.count() - residual.and_count(bad_mask)
}

fn apply_occurrence_to_residual(residual: &mut Bits, occ: &Occurrence) {
    residual.minus_assign(&occ.good_mask);
    residual.or_assign(&occ.bad_mask);
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
    let mut good_union_mask = Bits::default();

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
                if good_count <= bad_count + 1 {
                    continue;
                }

                let optimistic_delta = good_count as isize - bad_count as isize;
                if optimistic_delta < MIN_OCC_GAIN as isize {
                    continue;
                }

                if good_count > 0 {
                    good_union_mask.or_assign(&good_mask);
                    occurrences.push(Occurrence {
                        rot: view.rot,
                        top,
                        left,
                        good_mask,
                        bad_mask,
                        good_count,
                        bad_count,
                    });
                }
            }
        }
    }

    Candidate {
        pattern,
        rotations,
        occurrences,
        good_union_mask,
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
