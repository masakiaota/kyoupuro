// v002_hierarchical_shift.rs
use std::{
    cmp::Ordering,
    collections::BTreeMap,
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

fn best_scored_operation(
    state: &BoardState,
    candidates: &[Operation],
    axis: Direction,
    shift: usize,
    bound: usize,
) -> Option<ScoredOperation> {
    let prefixes = ScorePrefixes::build(state, axis, shift, bound);
    let mut best: Option<ScoredOperation> = None;
    for &op in candidates {
        let scored = ScoredOperation {
            op,
            stats: prefixes.stats(op),
        };
        match best {
            Some(current) if !scored_is_better(scored, current) => {}
            _ => best = Some(scored),
        }
    }
    best
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

#[derive(Debug, Default)]
struct StageRunStats {
    ops: usize,
    vertical_ops: usize,
    horizontal_ops: usize,
    good_moves: usize,
    harm_moves: usize,
    closer_moves: usize,
    farther_moves: usize,
    net_good_moves: i64,
    stalls: usize,
    axis_exhaustions: usize,
    terminal_vertical_gain: i32,
    terminal_horizontal_gain: i32,
    parallel_span_sum: usize,
    area_sum: usize,
    bad_before: usize,
    bad_after: usize,
}

fn run_stage(
    state: &mut BoardState,
    catalog: &StageCatalog,
    operations: &mut Vec<Operation>,
) -> StageRunStats {
    let mut stats = StageRunStats {
        bad_before: bad_count(state, Direction::Vertical, catalog.bound)
            + bad_count(state, Direction::Horizontal, catalog.bound),
        ..StageRunStats::default()
    };

    loop {
        let mut progressed = false;
        for axis in [Direction::Vertical, Direction::Horizontal] {
            loop {
                let best = best_scored_operation(
                    state,
                    catalog.candidates(axis),
                    axis,
                    catalog.shift,
                    catalog.bound,
                );
                let best_gain = best.map_or(i32::MIN, |best| best.stats.gain);
                if axis == Direction::Vertical {
                    stats.terminal_vertical_gain = best_gain;
                } else {
                    stats.terminal_horizontal_gain = best_gain;
                }
                let Some(best) = best.filter(|best| best.stats.gain > 0) else {
                    stats.axis_exhaustions += 1;
                    break;
                };

                let before = bad_count(state, axis, catalog.bound);
                assert_eq!(
                    operation_stats_direct(state, best.op, axis, catalog.bound),
                    best.stats
                );
                state.apply(best.op);
                let after = bad_count(state, axis, catalog.bound);
                assert_eq!(before - after, best.stats.gain as usize);

                operations.push(best.op);
                stats.ops += 1;
                stats.vertical_ops += (axis == Direction::Vertical) as usize;
                stats.horizontal_ops += (axis == Direction::Horizontal) as usize;
                stats.good_moves += best.stats.good;
                stats.harm_moves += best.stats.harm;
                stats.closer_moves += best.stats.closer;
                stats.farther_moves += best.stats.farther;
                stats.net_good_moves += best.stats.gain as i64;
                stats.parallel_span_sum += match axis {
                    Direction::Vertical => best.op.w,
                    Direction::Horizontal => best.op.h,
                };
                stats.area_sum += best.op.area();
                progressed = true;
            }
        }
        if !progressed {
            stats.stalls += 1;
            break;
        }
    }

    stats.bad_after = bad_count(state, Direction::Vertical, catalog.bound)
        + bad_count(state, Direction::Horizontal, catalog.bound);
    assert_eq!(
        stats.bad_before - stats.bad_after,
        stats.net_good_moves as usize
    );
    stats
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
    trace.add_time_ms("setup", setup_start.elapsed().as_secs_f64() * 1000.0);

    let mut state = BoardState::new(&input.initial_board);
    let mut operations = Vec::new();
    let solve_start = Instant::now();

    for catalog in &catalogs {
        trace.count_by(
            format!("stage_{}_vertical_candidates", catalog.shift),
            catalog.vertical.len() as i64,
        );
        trace.count_by(
            format!("stage_{}_horizontal_candidates", catalog.shift),
            catalog.horizontal.len() as i64,
        );

        let stats = run_stage(&mut state, catalog, &mut operations);
        let stage = catalog.shift;
        trace.count_by(format!("stage_{stage}_ops"), stats.ops as i64);
        trace.count_by(
            format!("stage_{stage}_vertical_ops"),
            stats.vertical_ops as i64,
        );
        trace.count_by(
            format!("stage_{stage}_horizontal_ops"),
            stats.horizontal_ops as i64,
        );
        trace.count_by(format!("stage_{stage}_good_moves"), stats.good_moves as i64);
        trace.count_by(format!("stage_{stage}_harm_moves"), stats.harm_moves as i64);
        trace.count_by(
            format!("stage_{stage}_closer_moves"),
            stats.closer_moves as i64,
        );
        trace.count_by(
            format!("stage_{stage}_farther_moves"),
            stats.farther_moves as i64,
        );
        trace.count_by(
            format!("stage_{stage}_net_good_moves"),
            stats.net_good_moves,
        );
        trace.count_by(format!("stage_{stage}_stalls"), stats.stalls as i64);
        trace.count_by(
            format!("stage_{stage}_axis_exhaustions"),
            stats.axis_exhaustions as i64,
        );
        trace.count_by(
            format!("stage_{stage}_terminal_vertical_gain"),
            stats.terminal_vertical_gain as i64,
        );
        trace.count_by(
            format!("stage_{stage}_terminal_horizontal_gain"),
            stats.terminal_horizontal_gain as i64,
        );
        trace.count_by(format!("stage_{stage}_bad_before"), stats.bad_before as i64);
        trace.count_by(format!("stage_{stage}_bad_after"), stats.bad_after as i64);
        trace.count_by(
            format!("stage_{stage}_parallel_span_sum"),
            stats.parallel_span_sum as i64,
        );
        trace.count_by(format!("stage_{stage}_area_sum"), stats.area_sum as i64);
    }
    trace.add_time_ms(
        "hierarchical_shift",
        solve_start.elapsed().as_secs_f64() * 1000.0,
    );

    assert!(operations.len() <= MAX_OPERATIONS);
    state.assert_consistent();

    let mut replay = BoardState::new(&input.initial_board);
    for &op in &operations {
        assert!(is_legal(&input, op));
        replay.apply(op);
    }
    replay.assert_consistent();
    assert_eq!(replay.board, state.board);

    trace.count_by("total_ops", operations.len() as i64);
    trace.count_by("final_e", state.misplaced_count as i64);
    trace.count_by("final_row_mismatch", state.row_mismatch_count as i64);
    trace.count_by("final_col_mismatch", state.col_mismatch_count as i64);
    trace.count_by("complete", (state.misplaced_count == 0) as i64);
    trace.count_by(
        "elapsed_micros",
        (timer.exact_elapsed_sec() * 1_000_000.0) as i64,
    );
    trace.count_by(
        "remaining_micros",
        (timer.exact_remaining_sec() * 1_000_000.0) as i64,
    );
    trace.summary();

    write_output(&operations);
}
