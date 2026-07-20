// v003_timed_complete.rs
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_OPERATIONS: usize = 100_000;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

const SEARCH_END_RATIO: f64 = 0.92;
const INITIAL_BEAM_WIDTH: usize = 8;
const INITIAL_BRANCHES: usize = 3;

/// 各段階は、この段階を終えたときに残差を `bound` 以下へ収めることを狙う。
const STAGE_SPECS: [(usize, usize); 5] = [(10, 9), (5, 4), (3, 2), (2, 1), (1, 0)];

struct Input {
    initial_board: [usize; CELLS],
    vertical_walls: [[bool; N - 1]; N],
    horizontal_walls: [[bool; N]; N - 1],
}

impl Input {
    fn read() -> Self {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap();
        let mut tokens = source.split_whitespace();

        let n: usize = tokens.next().unwrap().parse().unwrap();
        assert_eq!(n, N);
        let initial_board: [usize; CELLS] =
            std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            assert_eq!(row.len(), N - 1);
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            assert_eq!(row.len(), N);
            std::array::from_fn(|j| row[j] == b'1')
        });

        let mut seen = [false; CELLS];
        for &card in &initial_board {
            assert!(card < CELLS);
            assert!(!seen[card], "カード番号が重複している");
            seen[card] = true;
        }

        Self {
            initial_board,
            vertical_walls,
            horizontal_walls,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Direction {
    fn as_char(self) -> char {
        match self {
            Self::Vertical => 'V',
            Self::Horizontal => 'H',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

impl Operation {
    fn area(self) -> usize {
        self.h * self.w
    }
}

fn write_output(operations: &[Operation]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for op in operations {
        writeln!(
            writer,
            "{} {} {} {} {}",
            op.direction.as_char(),
            op.r,
            op.c,
            op.h,
            op.w,
        )
        .unwrap();
    }
}

/// `v000_template.rs` から持ち込んだ、候補評価用の軽量状態。
#[derive(Clone)]
struct BoardState {
    board: [u16; CELLS],
    misplaced_count: u16,
    row_mismatch_count: u16,
    col_mismatch_count: u16,
}

impl BoardState {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        let board = std::array::from_fn(|cell| initial_board[cell] as u16);
        let mut state = Self {
            board,
            misplaced_count: 0,
            row_mismatch_count: 0,
            col_mismatch_count: 0,
        };
        state.recompute_counts();
        state
    }

    #[inline]
    fn card_at(&self, cell: usize) -> usize {
        self.board[cell] as usize
    }

    #[inline]
    fn row_delta_at(&self, cell: usize) -> i32 {
        (self.card_at(cell) / N) as i32 - (cell / N) as i32
    }

    #[inline]
    fn col_delta_at(&self, cell: usize) -> i32 {
        (self.card_at(cell) % N) as i32 - (cell % N) as i32
    }

    fn recompute_counts(&mut self) {
        let mut misplaced = 0_u16;
        let mut row_mismatch = 0_u16;
        let mut col_mismatch = 0_u16;
        for cell in 0..CELLS {
            let card = self.card_at(cell);
            misplaced += (card != cell) as u16;
            row_mismatch += (card / N != cell / N) as u16;
            col_mismatch += (card % N != cell % N) as u16;
        }
        self.misplaced_count = misplaced;
        self.row_mismatch_count = row_mismatch;
        self.col_mismatch_count = col_mismatch;
    }

    fn remove_contribution(&mut self, cell: usize) {
        let card = self.card_at(cell);
        self.misplaced_count -= (card != cell) as u16;
        self.row_mismatch_count -= (card / N != cell / N) as u16;
        self.col_mismatch_count -= (card % N != cell % N) as u16;
    }

    fn add_contribution(&mut self, cell: usize) {
        let card = self.card_at(cell);
        self.misplaced_count += (card != cell) as u16;
        self.row_mismatch_count += (card / N != cell / N) as u16;
        self.col_mismatch_count += (card % N != cell % N) as u16;
    }

    fn swap_cells(&mut self, a: usize, b: usize) {
        self.remove_contribution(a);
        self.remove_contribution(b);
        self.board.swap(a, b);
        self.add_contribution(a);
        self.add_contribution(b);
    }

    /// 長方形操作は対合なので、同じ操作を再適用すると undo になる。
    fn apply(&mut self, op: Operation) {
        match op.direction {
            Direction::Vertical => {
                let half = op.h / 2;
                for dr in 0..half {
                    for dc in 0..op.w {
                        let a = (op.r + dr) * N + op.c + dc;
                        let b = (op.r + half + dr) * N + op.c + dc;
                        self.swap_cells(a, b);
                    }
                }
            }
            Direction::Horizontal => {
                let half = op.w / 2;
                for dr in 0..op.h {
                    for dc in 0..half {
                        let a = (op.r + dr) * N + op.c + dc;
                        let b = (op.r + dr) * N + op.c + half + dc;
                        self.swap_cells(a, b);
                    }
                }
            }
        }
    }

    fn assert_consistent(&self) {
        let mut recomputed = self.clone();
        recomputed.recompute_counts();
        assert_eq!(self.misplaced_count, recomputed.misplaced_count);
        assert_eq!(self.row_mismatch_count, recomputed.row_mismatch_count);
        assert_eq!(self.col_mismatch_count, recomputed.col_mismatch_count);

        let mut seen = [false; CELLS];
        for &card in &self.board {
            let card = card as usize;
            assert!(card < CELLS);
            assert!(!seen[card]);
            seen[card] = true;
        }
    }
}

#[derive(Debug, Default)]
struct TraceStats {
    counts: BTreeMap<String, i64>,
    times_ms: BTreeMap<String, f64>,
}

impl TraceStats {
    fn count_by(&mut self, key: impl Into<String>, delta: i64) {
        if cfg!(feature = "local") {
            *self.counts.entry(key.into()).or_insert(0) += delta;
        }
    }

    fn add_time_ms(&mut self, key: impl Into<String>, ms: f64) {
        if cfg!(feature = "local") {
            *self.times_ms.entry(key.into()).or_insert(0.0) += ms;
        }
    }

    fn summary(&self) {
        if cfg!(feature = "local") {
            for (key, value) in &self.counts {
                eprintln!("[summary.count] {key}={value}");
            }
            for (key, value) in &self.times_ms {
                eprintln!("[summary.time_ms] {key}={value:.3}");
            }
        }
    }
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        assert!(time_limit_sec > 0.0);
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn is_legal(input: &Input, op: Operation) -> bool {
    if op.h == 0 || op.w == 0 || op.r + op.h > N || op.c + op.w > N {
        return false;
    }
    match op.direction {
        Direction::Vertical if op.h % 2 != 0 => return false,
        Direction::Horizontal if op.w % 2 != 0 => return false,
        _ => {}
    }

    for r in op.r..op.r + op.h {
        for c in op.c..op.c + op.w - 1 {
            if input.vertical_walls[r][c] {
                return false;
            }
        }
    }
    for r in op.r..op.r + op.h - 1 {
        for c in op.c..op.c + op.w {
            if input.horizontal_walls[r][c] {
                return false;
            }
        }
    }
    true
}

fn visit_swap_pairs(op: Operation, mut f: impl FnMut(usize, usize)) {
    match op.direction {
        Direction::Vertical => {
            let half = op.h / 2;
            for dr in 0..half {
                for dc in 0..op.w {
                    let a = (op.r + dr) * N + op.c + dc;
                    let b = (op.r + half + dr) * N + op.c + dc;
                    f(a, b);
                }
            }
        }
        Direction::Horizontal => {
            let half = op.w / 2;
            for dr in 0..op.h {
                for dc in 0..half {
                    let a = (op.r + dr) * N + op.c + dc;
                    let b = (op.r + dr) * N + op.c + half + dc;
                    f(a, b);
                }
            }
        }
    }
}

fn is_bad(state: &BoardState, cell: usize, axis: Direction, bound: usize) -> bool {
    let delta = match axis {
        Direction::Vertical => state.row_delta_at(cell),
        Direction::Horizontal => state.col_delta_at(cell),
    };
    delta.unsigned_abs() as usize > bound
}

fn bad_count(state: &BoardState, axis: Direction, bound: usize) -> usize {
    (0..CELLS)
        .filter(|&cell| is_bad(state, cell, axis, bound))
        .count()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MoveStats {
    gain: i32,
    good: usize,
    harm: usize,
    closer: usize,
    farther: usize,
}

/// gain は、この段階の範囲外カード数が一手で何枚純減するかを表す。
fn operation_stats_direct(
    state: &BoardState,
    op: Operation,
    axis: Direction,
    bound: usize,
) -> MoveStats {
    let mut stats = MoveStats::default();
    visit_swap_pairs(op, |a, b| {
        for (source, destination) in [(a, b), (b, a)] {
            let card = state.card_at(source);
            let before_delta = match axis {
                Direction::Vertical => (card / N) as i32 - (source / N) as i32,
                Direction::Horizontal => (card % N) as i32 - (source % N) as i32,
            };
            let after_delta = match axis {
                Direction::Vertical => (card / N) as i32 - (destination / N) as i32,
                Direction::Horizontal => (card % N) as i32 - (destination % N) as i32,
            };
            let before_bad = before_delta.unsigned_abs() as usize > bound;
            let after_bad = after_delta.unsigned_abs() as usize > bound;
            match after_delta.unsigned_abs().cmp(&before_delta.unsigned_abs()) {
                Ordering::Less => stats.closer += 1,
                Ordering::Greater => stats.farther += 1,
                Ordering::Equal => {}
            }
            match (before_bad, after_bad) {
                (true, false) => stats.good += 1,
                (false, true) => stats.harm += 1,
                _ => {}
            }
        }
    });
    stats.gain = stats.good as i32 - stats.harm as i32;
    stats
}

/// 固定シフトの各帯について、相乗り方向の区間和を O(1) で引くための累積和。
struct ScorePrefixes {
    good: [[u16; N + 1]; N + 1],
    harm: [[u16; N + 1]; N + 1],
    closer: [[u16; N + 1]; N + 1],
    farther: [[u16; N + 1]; N + 1],
}

impl ScorePrefixes {
    fn build(state: &BoardState, axis: Direction, shift: usize, bound: usize) -> Self {
        let mut prefixes = Self {
            good: [[0; N + 1]; N + 1],
            harm: [[0; N + 1]; N + 1],
            closer: [[0; N + 1]; N + 1],
            farther: [[0; N + 1]; N + 1],
        };

        match axis {
            Direction::Vertical => {
                for r in 0..=N - 2 * shift {
                    for c in 0..N {
                        let unit = Operation {
                            direction: axis,
                            r,
                            c,
                            h: 2 * shift,
                            w: 1,
                        };
                        let stats = operation_stats_direct(state, unit, axis, bound);
                        prefixes.good[r][c + 1] = prefixes.good[r][c] + stats.good as u16;
                        prefixes.harm[r][c + 1] = prefixes.harm[r][c] + stats.harm as u16;
                        prefixes.closer[r][c + 1] = prefixes.closer[r][c] + stats.closer as u16;
                        prefixes.farther[r][c + 1] = prefixes.farther[r][c] + stats.farther as u16;
                    }
                }
            }
            Direction::Horizontal => {
                for c in 0..=N - 2 * shift {
                    for r in 0..N {
                        let unit = Operation {
                            direction: axis,
                            r,
                            c,
                            h: 1,
                            w: 2 * shift,
                        };
                        let stats = operation_stats_direct(state, unit, axis, bound);
                        prefixes.good[c][r + 1] = prefixes.good[c][r] + stats.good as u16;
                        prefixes.harm[c][r + 1] = prefixes.harm[c][r] + stats.harm as u16;
                        prefixes.closer[c][r + 1] = prefixes.closer[c][r] + stats.closer as u16;
                        prefixes.farther[c][r + 1] = prefixes.farther[c][r] + stats.farther as u16;
                    }
                }
            }
        }
        prefixes
    }

    fn stats(&self, op: Operation) -> MoveStats {
        let (band, from, to) = match op.direction {
            Direction::Vertical => (op.r, op.c, op.c + op.w),
            Direction::Horizontal => (op.c, op.r, op.r + op.h),
        };
        let good = (self.good[band][to] - self.good[band][from]) as usize;
        let harm = (self.harm[band][to] - self.harm[band][from]) as usize;
        MoveStats {
            gain: good as i32 - harm as i32,
            good,
            harm,
            closer: (self.closer[band][to] - self.closer[band][from]) as usize,
            farther: (self.farther[band][to] - self.farther[band][from]) as usize,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScoredOperation {
    op: Operation,
    stats: MoveStats,
}

fn operation_lex_cmp(a: Operation, b: Operation) -> Ordering {
    (a.r, a.c, a.h, a.w).cmp(&(b.r, b.c, b.h, b.w))
}

fn scored_is_better(candidate: ScoredOperation, current: ScoredOperation) -> bool {
    candidate.stats.gain > current.stats.gain
        || (candidate.stats.gain == current.stats.gain
            && (candidate.stats.good > current.stats.good
                || (candidate.stats.good == current.stats.good
                    && (candidate.stats.harm < current.stats.harm
                        || (candidate.stats.harm == current.stats.harm
                            && (candidate.op.area() > current.op.area()
                                || (candidate.op.area() == current.op.area()
                                    && operation_lex_cmp(candidate.op, current.op)
                                        == Ordering::Less)))))))
}

fn top_scored_operations(
    state: &BoardState,
    candidates: &[Operation],
    axis: Direction,
    shift: usize,
    bound: usize,
    limit: usize,
) -> Vec<ScoredOperation> {
    let prefixes = ScorePrefixes::build(state, axis, shift, bound);
    let mut top = Vec::with_capacity(limit + 1);
    for &op in candidates {
        let scored = ScoredOperation {
            op,
            stats: prefixes.stats(op),
        };
        if scored.stats.gain <= 0 {
            continue;
        }
        let at = top
            .iter()
            .position(|&current| scored_is_better(scored, current))
            .unwrap_or(top.len());
        if at < limit {
            top.insert(at, scored);
            top.truncate(limit);
        }
    }
    top
}

fn generate_candidates(input: &Input, direction: Direction, shift: usize) -> Vec<Operation> {
    let mut operations = Vec::new();
    match direction {
        Direction::Vertical => {
            let h = 2 * shift;
            for r in 0..=N - h {
                for c in 0..N {
                    for w in 1..=N - c {
                        let op = Operation {
                            direction,
                            r,
                            c,
                            h,
                            w,
                        };
                        if is_legal(input, op) {
                            operations.push(op);
                        }
                    }
                }
            }
        }
        Direction::Horizontal => {
            let w = 2 * shift;
            for r in 0..N {
                for h in 1..=N - r {
                    for c in 0..=N - w {
                        let op = Operation {
                            direction,
                            r,
                            c,
                            h,
                            w,
                        };
                        if is_legal(input, op) {
                            operations.push(op);
                        }
                    }
                }
            }
        }
    }
    operations
}

struct StageCatalog {
    shift: usize,
    bound: usize,
    vertical: Vec<Operation>,
    horizontal: Vec<Operation>,
}

impl StageCatalog {
    fn candidates(&self, direction: Direction) -> &[Operation] {
        match direction {
            Direction::Vertical => &self.vertical,
            Direction::Horizontal => &self.horizontal,
        }
    }
}

#[derive(Clone)]
struct BeamNode {
    state: BoardState,
    operations: Vec<Operation>,
    closer: usize,
    farther: usize,
    tie_break: u64,
    stalled_axes: u8,
    stage_ops: usize,
    stage_net_gain: usize,
    stage_bad_before: usize,
}

#[derive(Debug, Default)]
struct StageRunStats {
    ops: usize,
    net_gain: usize,
    bad_before: usize,
    bad_after: usize,
}

#[derive(Debug, Default)]
struct BeamRunStats {
    expanded: usize,
    kept: usize,
    multi_branch_nodes: usize,
}

#[derive(Clone, Copy)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.state = x;
        x
    }

    fn index(&mut self, len: usize) -> usize {
        assert!(len > 0);
        (self.next_u64() % len as u64) as usize
    }
}

fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn board_hash(state: &BoardState) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for (cell, &card) in state.board.iter().enumerate() {
        hash ^= (card as u64) | ((cell as u64) << 16);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

fn total_bad(state: &BoardState, bound: usize) -> usize {
    bad_count(state, Direction::Vertical, bound) + bad_count(state, Direction::Horizontal, bound)
}

fn beam_node_cmp(a: &BeamNode, b: &BeamNode, bound: usize) -> Ordering {
    total_bad(&a.state, bound)
        .cmp(&total_bad(&b.state, bound))
        .then_with(|| a.farther.cmp(&b.farther))
        .then_with(|| b.closer.cmp(&a.closer))
        .then_with(|| a.operations.len().cmp(&b.operations.len()))
        .then_with(|| a.tie_break.cmp(&b.tie_break))
}

fn prune_beam(mut nodes: Vec<BeamNode>, bound: usize, width: usize) -> Vec<BeamNode> {
    nodes.sort_by(|a, b| beam_node_cmp(a, b, bound));
    let mut seen = HashSet::with_capacity(nodes.len());
    nodes.retain(|node| seen.insert(board_hash(&node.state)));
    nodes.truncate(width);
    nodes
}

fn run_stage_beam(
    mut beam: Vec<BeamNode>,
    catalog: &StageCatalog,
    width: usize,
    branches: usize,
    round_seed: u64,
    timer: &TimeKeeper,
    deadline_sec: Option<f64>,
    run_stats: &mut BeamRunStats,
) -> (Vec<BeamNode>, StageRunStats) {
    for node in &mut beam {
        node.stalled_axes = 0;
        node.stage_ops = 0;
        node.stage_net_gain = 0;
        node.stage_bad_before = total_bad(&node.state, catalog.bound);
    }

    let mut depth = 0_usize;
    loop {
        if beam.iter().all(|node| node.stalled_axes >= 2) {
            break;
        }
        if deadline_sec.is_some_and(|deadline| timer.exact_elapsed_sec() >= deadline) {
            break;
        }

        let axis = if depth % 2 == 0 {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };
        let mut next = Vec::with_capacity(beam.len() * branches);
        for node in beam {
            if node.stalled_axes >= 2 {
                next.push(node);
                continue;
            }
            let top = top_scored_operations(
                &node.state,
                catalog.candidates(axis),
                axis,
                catalog.shift,
                catalog.bound,
                branches,
            );
            if top.is_empty() {
                let mut carried = node;
                carried.stalled_axes += 1;
                next.push(carried);
                continue;
            }

            run_stats.expanded += top.len();
            run_stats.multi_branch_nodes += (top.len() > 1) as usize;
            for scored in top {
                let mut child = node.clone();
                let before = bad_count(&child.state, axis, catalog.bound);
                assert_eq!(
                    operation_stats_direct(&child.state, scored.op, axis, catalog.bound),
                    scored.stats
                );
                child.state.apply(scored.op);
                let after = bad_count(&child.state, axis, catalog.bound);
                assert_eq!(before - after, scored.stats.gain as usize);
                child.operations.push(scored.op);
                child.closer += scored.stats.closer;
                child.farther += scored.stats.farther;
                child.stalled_axes = 0;
                child.stage_ops += 1;
                child.stage_net_gain += scored.stats.gain as usize;
                child.tie_break = mix64(
                    board_hash(&child.state)
                        ^ round_seed
                        ^ (child.operations.len() as u64).rotate_left(21),
                );
                next.push(child);
            }
        }
        beam = prune_beam(next, catalog.bound, width);
        run_stats.kept += beam.len();
        assert!(!beam.is_empty());
        depth += 1;
    }

    beam = prune_beam(beam, catalog.bound, width);
    let leader = &beam[0];
    let stats = StageRunStats {
        ops: leader.stage_ops,
        net_gain: leader.stage_net_gain,
        bad_before: leader.stage_bad_before,
        bad_after: total_bad(&leader.state, catalog.bound),
    };
    assert_eq!(stats.bad_before - stats.bad_after, stats.net_gain);
    (beam, stats)
}

fn run_beam_round(
    initial_state: &BoardState,
    catalogs: &[StageCatalog],
    width: usize,
    branches: usize,
    round_seed: u64,
    timer: &TimeKeeper,
    deadline_sec: Option<f64>,
    trace: &mut TraceStats,
) -> Vec<BeamNode> {
    let mut beam = vec![BeamNode {
        state: initial_state.clone(),
        operations: Vec::new(),
        closer: 0,
        farther: 0,
        tie_break: round_seed,
        stalled_axes: 0,
        stage_ops: 0,
        stage_net_gain: 0,
        stage_bad_before: 0,
    }];
    let mut run_stats = BeamRunStats::default();
    for catalog in catalogs {
        let (next, stats) = run_stage_beam(
            beam,
            catalog,
            width,
            branches,
            round_seed,
            timer,
            deadline_sec,
            &mut run_stats,
        );
        beam = next;
        let stage = catalog.shift;
        trace.count_by(format!("stage_{stage}_ops"), stats.ops as i64);
        trace.count_by(format!("stage_{stage}_net_gain"), stats.net_gain as i64);
        trace.count_by(format!("stage_{stage}_bad_before"), stats.bad_before as i64);
        trace.count_by(format!("stage_{stage}_bad_after"), stats.bad_after as i64);
    }
    trace.count_by("beam_expanded", run_stats.expanded as i64);
    trace.count_by("beam_kept", run_stats.kept as i64);
    trace.count_by(
        "beam_multi_branch_nodes",
        run_stats.multi_branch_nodes as i64,
    );
    beam
}

fn add_edge(graph: &mut [Vec<usize>], u: usize, v: usize) {
    graph[u].push(v);
    graph[v].push(u);
}

fn build_graph(input: &Input) -> Vec<Vec<usize>> {
    let mut graph = vec![Vec::new(); CELLS];
    for r in 0..N {
        for c in 0..N - 1 {
            if !input.vertical_walls[r][c] {
                add_edge(&mut graph, r * N + c, r * N + c + 1);
            }
        }
    }
    for r in 0..N - 1 {
        for c in 0..N {
            if !input.horizontal_walls[r][c] {
                add_edge(&mut graph, r * N + c, (r + 1) * N + c);
            }
        }
    }
    graph
}

fn build_spanning_tree(
    graph: &[Vec<usize>],
    root: usize,
    rng: &mut Rng,
    randomized: bool,
) -> (Vec<usize>, Vec<usize>) {
    let mut parent = vec![usize::MAX; CELLS];
    let mut order = Vec::with_capacity(CELLS);
    parent[root] = root;
    order.push(root);
    let mut head = 0;
    while head < order.len() {
        let v = order[head];
        head += 1;
        let mut neighbors = graph[v].clone();
        if randomized {
            for i in (1..neighbors.len()).rev() {
                let j = rng.index(i + 1);
                neighbors.swap(i, j);
            }
        }
        for to in neighbors {
            if parent[to] == usize::MAX {
                parent[to] = v;
                order.push(to);
            }
        }
    }
    assert_eq!(order.len(), CELLS, "盤面の隣接グラフが連結でない");
    for v in 0..CELLS {
        assert!(v == root || graph[v].contains(&parent[v]));
    }
    (parent, order)
}

fn path_in_tree(start: usize, goal: usize, parent: &[usize]) -> Vec<usize> {
    let mut from_chain = Vec::new();
    let mut from_index = vec![usize::MAX; CELLS];
    let mut v = start;
    loop {
        from_index[v] = from_chain.len();
        from_chain.push(v);
        if parent[v] == v {
            break;
        }
        v = parent[v];
    }
    let mut to_chain = Vec::new();
    let mut v = goal;
    while from_index[v] == usize::MAX {
        to_chain.push(v);
        v = parent[v];
    }
    let mut path = from_chain[..=from_index[v]].to_vec();
    to_chain.reverse();
    path.extend(to_chain);
    path
}

fn adjacent_swap_operation(u: usize, v: usize) -> Operation {
    let (ru, cu) = (u / N, u % N);
    let (rv, cv) = (v / N, v % N);
    if ru == rv {
        assert_eq!(cu.abs_diff(cv), 1);
        Operation {
            direction: Direction::Horizontal,
            r: ru,
            c: cu.min(cv),
            h: 1,
            w: 2,
        }
    } else {
        assert_eq!(cu, cv);
        assert_eq!(ru.abs_diff(rv), 1);
        Operation {
            direction: Direction::Vertical,
            r: ru.min(rv),
            c: cu,
            h: 2,
            w: 1,
        }
    }
}

fn complete_tree_tail(
    state: &BoardState,
    input: &Input,
    parent: &[usize],
    order: &[usize],
) -> Vec<Operation> {
    let root = order[0];
    let mut board: Vec<usize> = state.board.iter().map(|&card| card as usize).collect();
    let mut position = vec![usize::MAX; CELLS];
    for (cell, &card) in board.iter().enumerate() {
        position[card] = cell;
    }
    let mut fixed = vec![false; CELLS];
    let mut operations = Vec::new();
    for target in order.iter().rev().copied().filter(|&v| v != root) {
        let source = position[target];
        assert!(!fixed[source]);
        let path = path_in_tree(source, target, parent);
        for edge in path.windows(2) {
            assert!(!fixed[edge[0]] && !fixed[edge[1]]);
            let op = adjacent_swap_operation(edge[0], edge[1]);
            assert!(is_legal(input, op));
            let card_u = board[edge[0]];
            let card_v = board[edge[1]];
            board.swap(edge[0], edge[1]);
            position[card_u] = edge[1];
            position[card_v] = edge[0];
            operations.push(op);
        }
        assert_eq!(board[target], target);
        fixed[target] = true;
    }
    assert_eq!(board[root], root);
    assert!(board.iter().enumerate().all(|(cell, &card)| cell == card));
    operations
}

fn input_seed(input: &Input) -> u64 {
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    for (cell, &card) in input.initial_board.iter().enumerate() {
        seed = mix64(seed ^ card as u64 ^ (cell as u64).rotate_left(17));
    }
    for r in 0..N {
        for c in 0..N - 1 {
            seed = mix64(seed ^ ((input.vertical_walls[r][c] as u64) << ((r + c) % 61)));
        }
    }
    for r in 0..N - 1 {
        for c in 0..N {
            seed = mix64(seed ^ ((input.horizontal_walls[r][c] as u64) << ((r + c + 7) % 61)));
        }
    }
    seed
}

fn candidate_from_node(
    node: &BeamNode,
    input: &Input,
    graph: &[Vec<usize>],
    rng: &mut Rng,
    randomized_tree: bool,
) -> (Vec<Operation>, usize) {
    let root = if randomized_tree { rng.index(CELLS) } else { 0 };
    let (parent, order) = build_spanning_tree(graph, root, rng, randomized_tree);
    let tail = complete_tree_tail(&node.state, input, &parent, &order);
    let tail_len = tail.len();
    let mut operations = node.operations.clone();
    operations.extend(tail);
    assert!(operations.len() <= MAX_OPERATIONS);
    (operations, tail_len)
}

fn main() {
    let timer = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let input = Input::read();
    let mut trace = TraceStats::default();
    let setup_start = Instant::now();
    let catalogs: Vec<_> = STAGE_SPECS
        .into_iter()
        .map(|(shift, bound)| StageCatalog {
            shift,
            bound,
            vertical: generate_candidates(&input, Direction::Vertical, shift),
            horizontal: generate_candidates(&input, Direction::Horizontal, shift),
        })
        .collect();
    let graph = build_graph(&input);
    let seed = input_seed(&input);
    trace.add_time_ms("setup", setup_start.elapsed().as_secs_f64() * 1000.0);
    for catalog in &catalogs {
        trace.count_by(
            format!("stage_{}_vertical_candidates", catalog.shift),
            catalog.vertical.len() as i64,
        );
        trace.count_by(
            format!("stage_{}_horizontal_candidates", catalog.shift),
            catalog.horizontal.len() as i64,
        );
    }

    let initial_state = BoardState::new(&input.initial_board);
    let search_start = Instant::now();
    let first_beam = run_beam_round(
        &initial_state,
        &catalogs,
        INITIAL_BEAM_WIDTH,
        INITIAL_BRANCHES,
        0,
        &timer,
        None,
        &mut trace,
    );
    trace.count_by("beam_rounds", 1);
    trace.count_by("first_beam_states", first_beam.len() as i64);

    // 最初の incumbent は必ず、階層 beam の先頭状態と決定的 BFS 木から作る。
    let mut deterministic_rng = Rng::new(seed);
    let (mut best_operations, initial_tail_ops) = candidate_from_node(
        &first_beam[0],
        &input,
        &graph,
        &mut deterministic_rng,
        false,
    );
    let mut best_prefix_ops = first_beam[0].operations.len();
    let mut best_tail_ops = initial_tail_ops;
    let mut complete_candidates = 1_usize;
    let mut best_updates = 0_usize;

    for (index, node) in first_beam.iter().enumerate().skip(1) {
        let mut rng = Rng::new(mix64(seed ^ index as u64));
        let (candidate, tail_ops) = candidate_from_node(node, &input, &graph, &mut rng, false);
        complete_candidates += 1;
        if candidate.len() < best_operations.len() {
            best_prefix_ops = node.operations.len();
            best_tail_ops = tail_ops;
            best_operations = candidate;
            best_updates += 1;
        }
    }

    let search_deadline = PROGRAM_TIME_LIMIT_SEC * SEARCH_END_RATIO;
    let mut round = 1_u64;
    while timer.exact_elapsed_sec() < search_deadline {
        let remaining_ratio = (search_deadline - timer.exact_elapsed_sec()) / search_deadline;
        let width = if remaining_ratio > 0.45 {
            12
        } else {
            INITIAL_BEAM_WIDTH
        };
        let branches = if remaining_ratio > 0.25 {
            4
        } else {
            INITIAL_BRANCHES
        };
        let round_seed = mix64(seed ^ round.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let beam = run_beam_round(
            &initial_state,
            &catalogs,
            width,
            branches,
            round_seed,
            &timer,
            Some(search_deadline),
            &mut trace,
        );
        trace.count_by("beam_rounds", 1);
        for (index, node) in beam.iter().enumerate() {
            let mut rng = Rng::new(mix64(round_seed ^ index as u64));
            let (candidate, tail_ops) = candidate_from_node(node, &input, &graph, &mut rng, true);
            complete_candidates += 1;
            if candidate.len() < best_operations.len() {
                best_prefix_ops = node.operations.len();
                best_tail_ops = tail_ops;
                best_operations = candidate;
                best_updates += 1;
            }
        }
        round += 1;
    }
    trace.add_time_ms("search", search_start.elapsed().as_secs_f64() * 1000.0);

    let mut replay = BoardState::new(&input.initial_board);
    for &op in &best_operations {
        assert!(is_legal(&input, op));
        replay.apply(op);
    }
    replay.assert_consistent();
    assert_eq!(replay.misplaced_count, 0);
    assert!(best_operations.len() <= MAX_OPERATIONS);

    trace.count_by("prefix_ops", best_prefix_ops as i64);
    trace.count_by("initial_tail_ops", initial_tail_ops as i64);
    trace.count_by("best_tail_ops", best_tail_ops as i64);
    trace.count_by("total_ops", best_operations.len() as i64);
    trace.count_by("iterations", round as i64);
    trace.count_by("complete_candidates", complete_candidates as i64);
    trace.count_by("best_updates", best_updates as i64);
    trace.count_by("final_e", 0);
    trace.count_by(
        "elapsed_micros",
        (timer.exact_elapsed_sec() * 1_000_000.0) as i64,
    );
    trace.count_by(
        "remaining_micros",
        (timer.exact_remaining_sec() * 1_000_000.0) as i64,
    );
    trace.summary();
    write_output(&best_operations);
}
