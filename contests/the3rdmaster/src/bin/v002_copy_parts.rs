// v002_copy_parts.rs
use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::fmt::Write as _;
use std::io::{self, Read};
use std::str::{FromStr, SplitAsciiWhitespace};
use std::time::Instant;

const N: usize = 32;
const DIR4: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const MIN_PATTERN_SIZE: usize = 6;
const MAX_CANDIDATES: usize = 72;
const SOLVER_TIME_LIMIT_SEC: f64 = 1.80;

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
    cells: Vec<Cell>,
    min_board_i: usize,
    min_board_j: usize,
}

#[derive(Debug, Clone)]
struct Occurrence {
    rot: usize,
    top: usize,
    left: usize,
    cells: Vec<Coord>,
}

#[derive(Debug, Clone)]
struct Candidate {
    pattern: Pattern,
    rotations: Vec<RotatedPattern>,
    occurrences: Vec<Occurrence>,
}

#[derive(Debug, Clone)]
struct CandidateStat {
    pattern: Pattern,
    hits: usize,
    total_component_size: usize,
}

fn main() {
    let input = Input::read();
    let ops = solve(&input);
    print!("{}", format_ops(&ops));
}

fn solve(input: &Input) -> Vec<Op> {
    let baseline_ops = baseline_paint_ops(&input.goal);
    let baseline_cost = input.nonzero_goal_count();
    let debug = std::env::var_os("V002_DEBUG").is_some();

    let mut time_keeper = TimeKeeper::new(SOLVER_TIME_LIMIT_SEC, 10);
    let mut patterns = extract_patterns(input, &mut time_keeper);
    patterns.sort_by_key(|pattern| Reverse((pattern.hint_score, pattern.cells.len())));
    if patterns.len() > MAX_CANDIDATES * 2 {
        patterns.truncate(MAX_CANDIDATES * 2);
    }
    let pattern_count = patterns.len();

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
            "patterns={} candidates={} baseline_cost={}",
            pattern_count,
            candidates.len(),
            baseline_cost
        );
    }

    let ops = build_plan(input, &candidates, baseline_cost);
    if debug {
        eprintln!("planned_ops={}", ops.len());
    }
    if ops.len() > baseline_cost {
        return baseline_ops;
    }

    let mut state = State::new(input.k_layers);
    match state.apply_all(&ops) {
        Ok(()) if state.layer0_matches(&input.goal) => ops,
        Ok(()) => {
            if debug {
                eprintln!("fallback: final board mismatch");
            }
            baseline_ops
        }
        Err(err) => {
            if debug {
                eprintln!("fallback: simulation error: {err}");
            }
            baseline_ops
        }
    }
}

fn extract_patterns(input: &Input, time_keeper: &mut TimeKeeper) -> Vec<Pattern> {
    let goal = &input.goal;
    let mut matching = [[false; N]; N];
    let mut visited = [[false; N]; N];
    let mut queue = Vec::<Coord>::with_capacity(N * N);
    let mut component = Vec::<Coord>::with_capacity(N * N);
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
    for view in &rotations {
        for top in 0..=N - view.height {
            for left in 0..=N - view.width {
                let mut cells = Vec::with_capacity(view.cells.len());
                let mut ok = true;
                for cell in &view.cells {
                    let bi = top + cell.i;
                    let bj = left + cell.j;
                    if goal[bi][bj] != cell.color {
                        ok = false;
                        break;
                    }
                    cells.push((bi, bj));
                }
                if ok {
                    occurrences.push(Occurrence {
                        rot: view.rot,
                        top,
                        left,
                        cells,
                    });
                }
            }
        }
    }
    Candidate {
        pattern,
        rotations,
        occurrences,
    }
}

fn build_rotations(pattern: &Pattern) -> Vec<RotatedPattern> {
    let mut rotations = Vec::with_capacity(4);
    for rot in 0..4 {
        let rotated = rotate_pattern_cells(&pattern.cells, pattern.height, pattern.width, rot);
        let mut min_board_i = N;
        let mut min_board_j = N;
        for cell in &pattern.cells {
            let (bi, bj) = rotate_coord((cell.i, cell.j), rot);
            min_board_i = min_board_i.min(bi);
            min_board_j = min_board_j.min(bj);
        }
        rotations.push(RotatedPattern {
            rot,
            height: rotated.height,
            width: rotated.width,
            cells: rotated.cells,
            min_board_i,
            min_board_j,
        });
    }
    rotations
}

fn build_plan(input: &Input, candidates: &[Candidate], baseline_cost: usize) -> Vec<Op> {
    let debug = std::env::var_os("V002_DEBUG").is_some();
    let mut residual = [[false; N]; N];
    for i in 0..N {
        for j in 0..N {
            residual[i][j] = input.goal[i][j] != 0;
        }
    }

    let mut ops = Vec::<Op>::new();
    let mut used = vec![false; candidates.len()];
    let mut built_any_pattern = false;
    let pattern_layer = 1usize;

    loop {
        let mut best: Option<(usize, Vec<usize>, isize, usize, usize)> = None;

        for (idx, candidate) in candidates.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let (selected, cover_count) = select_occurrences(candidate, &residual);
            if selected.is_empty() {
                continue;
            }

            let cost =
                candidate.pattern.cells.len() + selected.len() + usize::from(built_any_pattern);
            let gain = cover_count as isize - cost as isize;
            let score_tuple = (gain, cover_count, candidate.pattern.cells.len());

            let replace = match &best {
                None => true,
                Some((best_idx, _, best_gain, best_cover, _)) => {
                    let best_tuple =
                        (*best_gain, *best_cover, candidates[*best_idx].pattern.cells.len());
                    score_tuple > best_tuple
                }
            };
            if replace {
                best = Some((idx, selected, gain, cover_count, cost));
            }
        }

        let Some((idx, selected, gain, _, _)) = best else {
            break;
        };
        if gain <= 0 {
            break;
        }

        let candidate = &candidates[idx];
        if debug {
            eprintln!(
                "pick idx={} size={} occ={} gain={}",
                idx,
                candidate.pattern.cells.len(),
                selected.len(),
                gain
            );
        }
        if built_any_pattern {
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
        for occ_index in selected {
            let occ = &candidate.occurrences[occ_index];
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
            for &(i, j) in &occ.cells {
                residual[i][j] = false;
            }
        }
        used[idx] = true;
        built_any_pattern = true;

        if ops.len() + residual_count(&residual) >= baseline_cost {
            break;
        }
    }

    for i in 0..N {
        for j in 0..N {
            if residual[i][j] {
                ops.push(Op::Paint {
                    k: 0,
                    i,
                    j,
                    color: input.goal[i][j],
                });
            }
        }
    }

    ops
}

fn select_occurrences(candidate: &Candidate, residual: &[[bool; N]; N]) -> (Vec<usize>, usize) {
    let mut order = candidate
        .occurrences
        .iter()
        .enumerate()
        .map(|(idx, occ)| (count_residual_cells(&occ.cells, residual), idx))
        .filter(|&(count, _)| count >= 2)
        .collect::<Vec<_>>();
    order.sort_by_key(|&(count, idx)| Reverse((count, idx)));

    let mut remaining = *residual;
    let mut selected = Vec::new();
    let mut covered = 0usize;
    for (_, idx) in order {
        let occ = &candidate.occurrences[idx];
        let gain = count_residual_cells(&occ.cells, &remaining);
        if gain < 2 {
            continue;
        }
        covered += gain;
        selected.push(idx);
        for &(i, j) in &occ.cells {
            remaining[i][j] = false;
        }
    }
    (selected, covered)
}

fn count_residual_cells(cells: &[Coord], residual: &[[bool; N]; N]) -> usize {
    cells.iter().filter(|&&(i, j)| residual[i][j]).count()
}

fn residual_count(residual: &[[bool; N]; N]) -> usize {
    residual
        .iter()
        .flatten()
        .filter(|&&needed| needed)
        .count()
}
