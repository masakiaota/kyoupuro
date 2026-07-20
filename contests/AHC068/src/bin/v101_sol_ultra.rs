// v101_sol_ultra.rs
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_SINGLE_AXIS_SHIFT: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;
#[allow(clippy::manual_div_ceil)]
const MASK_WORDS: usize = (CELLS + 63) / 64;
const INF: u16 = u16::MAX;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 完成済みの最良列を保持したまま新しい完全計画を始める期限。
const NEW_PLAN_DEADLINE_RATIO: f64 = 0.82;
/// 途中の計画を破棄して出力準備へ移る期限。
const ACTIVE_PLAN_DEADLINE_RATIO: f64 = 0.90;
const ROUTE_SCORE: i64 = 4096;

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
        let initial_board = std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });

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
    #[inline]
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
        let mut misplaced_count = 0;
        let mut row_mismatch_count = 0;
        let mut col_mismatch_count = 0;
        for (cell, &card16) in board.iter().enumerate() {
            let card = card16 as usize;
            misplaced_count += (card != cell) as u16;
            row_mismatch_count += (card / N != cell / N) as u16;
            col_mismatch_count += (card % N != cell % N) as u16;
        }
        Self {
            board,
            misplaced_count,
            row_mismatch_count,
            col_mismatch_count,
        }
    }
}

#[derive(Clone)]
struct State {
    board_state: BoardState,
    position: [u16; CELLS],
}

impl State {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        let board_state = BoardState::new(initial_board);
        let mut position = [0; CELLS];
        for cell in 0..CELLS {
            position[board_state.board[cell] as usize] = cell as u16;
        }
        Self {
            board_state,
            position,
        }
    }

    #[inline]
    fn card_at(&self, cell: usize) -> usize {
        self.board_state.board[cell] as usize
    }

    #[inline]
    fn position_of(&self, card: usize) -> usize {
        self.position[card] as usize
    }

    #[inline]
    fn is_complete(&self) -> bool {
        self.board_state.misplaced_count == 0
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }

    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }

    fn summary(&self) {
        eprintln!("[summary] fallback_count={}", self.fallback_count);
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {}={}", key, value);
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {}={:.3}", key, value);
        }
    }
}

#[cfg(feature = "local")]
macro_rules! local {
    ($($body:tt)*) => {{ $($body)* }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline]
    fn ratio(&self) -> f64 {
        self.exact_elapsed_sec() / self.time_limit_sec
    }
}

#[derive(Clone, Copy)]
struct CellMask([u64; MASK_WORDS]);

impl CellMask {
    fn full() -> Self {
        let mut words = [u64::MAX; MASK_WORDS];
        let unused = MASK_WORDS * 64 - CELLS;
        if unused > 0 {
            words[MASK_WORDS - 1] = u64::MAX >> unused;
        }
        Self(words)
    }

    fn empty() -> Self {
        Self([0; MASK_WORDS])
    }

    #[inline]
    fn contains(self, cell: usize) -> bool {
        (self.0[cell >> 6] >> (cell & 63)) & 1 != 0
    }

    #[inline]
    fn insert(&mut self, cell: usize) {
        self.0[cell >> 6] |= 1_u64 << (cell & 63);
    }

    #[inline]
    fn remove(&mut self, cell: usize) {
        self.0[cell >> 6] &= !(1_u64 << (cell & 63));
    }

    #[inline]
    fn is_subset_of(self, superset: Self) -> bool {
        (0..MASK_WORDS).all(|i| self.0[i] & !superset.0[i] == 0)
    }
}

#[derive(Clone, Copy)]
struct LineOption {
    op: Operation,
    mask: CellMask,
}

struct Catalog {
    unit_neighbors: Vec<Vec<u16>>,
    line_options: Vec<LineOption>,
    pair_options: Vec<Vec<u16>>,
    macro_pairs: Vec<(u16, u16)>,
    cell_line_options: Vec<Vec<u16>>,
    full_unit_dist: Vec<[u16; CELLS]>,
    full_macro_dist: Vec<[u16; CELLS]>,
}

impl Catalog {
    fn build(input: &Input) -> Self {
        let mut unit_neighbors = vec![Vec::new(); CELLS];
        for r in 0..N {
            for c in 0..N {
                let p = r * N + c;
                if c + 1 < N && !input.vertical_walls[r][c] {
                    let q = p + 1;
                    unit_neighbors[p].push(q as u16);
                    unit_neighbors[q].push(p as u16);
                }
                if r + 1 < N && !input.horizontal_walls[r][c] {
                    let q = p + N;
                    unit_neighbors[p].push(q as u16);
                    unit_neighbors[q].push(p as u16);
                }
            }
        }

        let mut line_options = Vec::new();
        let mut pair_options = vec![Vec::new(); CELLS * CELLS];
        let mut cell_line_options = vec![Vec::new(); CELLS];

        for r in 0..N {
            for d in 1..=MAX_SINGLE_AXIS_SHIFT {
                let width = 2 * d;
                for c in 0..=N - width {
                    if (c..c + width - 1).any(|j| input.vertical_walls[r][j]) {
                        continue;
                    }
                    let op = Operation {
                        direction: Direction::Horizontal,
                        r,
                        c,
                        h: 1,
                        w: width,
                    };
                    Self::add_line_option(
                        op,
                        &mut line_options,
                        &mut pair_options,
                        &mut cell_line_options,
                    );
                }
            }
        }

        for c in 0..N {
            for d in 1..=MAX_SINGLE_AXIS_SHIFT {
                let height = 2 * d;
                for r in 0..=N - height {
                    if (r..r + height - 1).any(|i| input.horizontal_walls[i][c]) {
                        continue;
                    }
                    let op = Operation {
                        direction: Direction::Vertical,
                        r,
                        c,
                        h: height,
                        w: 1,
                    };
                    Self::add_line_option(
                        op,
                        &mut line_options,
                        &mut pair_options,
                        &mut cell_line_options,
                    );
                }
            }
        }

        assert!(line_options.len() < u16::MAX as usize);
        let mut macro_pairs = Vec::new();
        let mut full_macro_neighbors = vec![Vec::new(); CELLS];
        for p in 0..CELLS {
            for q in p + 1..CELLS {
                if !pair_options[pair_key(p, q)].is_empty() {
                    macro_pairs.push((p as u16, q as u16));
                    full_macro_neighbors[p].push(q as u16);
                    full_macro_neighbors[q].push(p as u16);
                }
            }
        }

        let full_unit_dist = all_pairs_dist(&unit_neighbors);
        let full_macro_dist = all_pairs_dist(&full_macro_neighbors);
        for p in 0..CELLS {
            assert!(full_unit_dist[p].iter().all(|&d| d != INF));
            assert!(full_macro_dist[p].iter().all(|&d| d != INF));
        }

        Self {
            unit_neighbors,
            line_options,
            pair_options,
            macro_pairs,
            cell_line_options,
            full_unit_dist,
            full_macro_dist,
        }
    }

    fn add_line_option(
        op: Operation,
        line_options: &mut Vec<LineOption>,
        pair_options: &mut [Vec<u16>],
        cell_line_options: &mut [Vec<u16>],
    ) {
        let mut mask = CellMask::empty();
        for r in op.r..op.r + op.h {
            for c in op.c..op.c + op.w {
                mask.insert(r * N + c);
            }
        }
        let id = line_options.len() as u16;
        line_options.push(LineOption { op, mask });
        for r in op.r..op.r + op.h {
            for c in op.c..op.c + op.w {
                cell_line_options[r * N + c].push(id);
            }
        }
        match op.direction {
            Direction::Horizontal => {
                let d = op.w / 2;
                for x in 0..op.h {
                    for y in 0..d {
                        let p = (op.r + x) * N + op.c + y;
                        let q = p + d;
                        pair_options[pair_key(p, q)].push(id);
                    }
                }
            }
            Direction::Vertical => {
                let d = op.h / 2;
                for x in 0..d {
                    for y in 0..op.w {
                        let p = (op.r + x) * N + op.c + y;
                        let q = p + d * N;
                        pair_options[pair_key(p, q)].push(id);
                    }
                }
            }
        }
    }

    fn build_active_macro_graph(&self, active: CellMask) -> Vec<Vec<u16>> {
        let mut graph = vec![Vec::new(); CELLS];
        for &(p16, q16) in &self.macro_pairs {
            let p = p16 as usize;
            let q = q16 as usize;
            if !active.contains(p) || !active.contains(q) {
                continue;
            }
            let usable = self.pair_options[pair_key(p, q)]
                .iter()
                .any(|&id| self.line_options[id as usize].mask.is_subset_of(active));
            if usable {
                graph[p].push(q16);
                graph[q].push(p16);
            }
        }
        graph
    }

    #[inline]
    fn cell_cost(&self, cell: usize, card: usize) -> i32 {
        let exact = if cell == card { 0 } else { 256 };
        exact
            + 64 * self.full_macro_dist[cell][card] as i32
            + self.full_unit_dist[cell][card] as i32
    }

    fn active_support_count(&self, cell: usize, active: CellMask) -> usize {
        self.cell_line_options[cell]
            .iter()
            .filter(|&&id| self.line_options[id as usize].mask.is_subset_of(active))
            .count()
    }
}

#[inline]
fn pair_key(p: usize, q: usize) -> usize {
    let (a, b) = if p < q { (p, q) } else { (q, p) };
    a * CELLS + b
}

fn all_pairs_dist(graph: &[Vec<u16>]) -> Vec<[u16; CELLS]> {
    let mut result = vec![[INF; CELLS]; CELLS];
    for (start, dist) in result.iter_mut().enumerate() {
        let mut queue = VecDeque::new();
        dist[start] = 0;
        queue.push_back(start);
        while let Some(p) = queue.pop_front() {
            let next_dist = dist[p] + 1;
            for &q16 in &graph[p] {
                let q = q16 as usize;
                if dist[q] == INF {
                    dist[q] = next_dist;
                    queue.push_back(q);
                }
            }
        }
    }
    result
}

fn bfs_dist(graph: &[Vec<u16>], start: usize) -> [u16; CELLS] {
    let mut dist = [INF; CELLS];
    let mut queue = VecDeque::new();
    dist[start] = 0;
    queue.push_back(start);
    while let Some(p) = queue.pop_front() {
        let next_dist = dist[p] + 1;
        for &q16 in &graph[p] {
            let q = q16 as usize;
            if dist[q] == INF {
                dist[q] = next_dist;
                queue.push_back(q);
            }
        }
    }
    dist
}

#[inline]
fn mismatch_tuple(cell: usize, card: usize) -> (i32, i32, i32) {
    (
        (cell != card) as i32,
        (cell / N != card / N) as i32,
        (cell % N != card % N) as i32,
    )
}

fn swap_cells(state: &mut State, p: usize, q: usize) {
    let a = state.card_at(p);
    let b = state.card_at(q);
    let p_before = mismatch_tuple(p, a);
    let q_before = mismatch_tuple(q, b);
    let p_after = mismatch_tuple(p, b);
    let q_after = mismatch_tuple(q, a);

    state.board_state.board.swap(p, q);
    state.position[a] = q as u16;
    state.position[b] = p as u16;

    let dm = p_after.0 + q_after.0 - p_before.0 - q_before.0;
    let dr = p_after.1 + q_after.1 - p_before.1 - q_before.1;
    let dc = p_after.2 + q_after.2 - p_before.2 - q_before.2;
    state.board_state.misplaced_count = (state.board_state.misplaced_count as i32 + dm) as u16;
    state.board_state.row_mismatch_count =
        (state.board_state.row_mismatch_count as i32 + dr) as u16;
    state.board_state.col_mismatch_count =
        (state.board_state.col_mismatch_count as i32 + dc) as u16;
}

fn apply_operation(state: &mut State, op: Operation) {
    match op.direction {
        Direction::Horizontal => {
            let d = op.w / 2;
            for x in 0..op.h {
                for y in 0..d {
                    let p = (op.r + x) * N + op.c + y;
                    swap_cells(state, p, p + d);
                }
            }
        }
        Direction::Vertical => {
            let d = op.h / 2;
            for x in 0..d {
                for y in 0..op.w {
                    let p = (op.r + x) * N + op.c + y;
                    swap_cells(state, p, p + d * N);
                }
            }
        }
    }
}

fn operation_gain(state: &State, op: Operation, catalog: &Catalog) -> i32 {
    let mut gain = 0;
    match op.direction {
        Direction::Horizontal => {
            let d = op.w / 2;
            for x in 0..op.h {
                for y in 0..d {
                    let p = (op.r + x) * N + op.c + y;
                    let q = p + d;
                    let a = state.card_at(p);
                    let b = state.card_at(q);
                    gain += catalog.cell_cost(p, a) + catalog.cell_cost(q, b)
                        - catalog.cell_cost(p, b)
                        - catalog.cell_cost(q, a);
                }
            }
        }
        Direction::Vertical => {
            let d = op.h / 2;
            for x in 0..d {
                for y in 0..op.w {
                    let p = (op.r + x) * N + op.c + y;
                    let q = p + d * N;
                    let a = state.card_at(p);
                    let b = state.card_at(q);
                    gain += catalog.cell_cost(p, a) + catalog.cell_cost(q, b)
                        - catalog.cell_cost(p, b)
                        - catalog.cell_cost(q, a);
                }
            }
        }
    }
    gain
}

fn operation_is_legal(input: &Input, op: Operation) -> bool {
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

fn horizontal_row_usable(input: &Input, active: CellMask, r: usize, c: usize, w: usize) -> bool {
    (c..c + w).all(|x| active.contains(r * N + x))
        && (c..c + w - 1).all(|x| !input.vertical_walls[r][x])
}

fn horizontal_boundary_usable(input: &Input, r: usize, c: usize, w: usize) -> bool {
    (c..c + w).all(|x| !input.horizontal_walls[r][x])
}

fn vertical_col_usable(input: &Input, active: CellMask, r: usize, c: usize, h: usize) -> bool {
    (r..r + h).all(|x| active.contains(x * N + c))
        && (r..r + h - 1).all(|x| !input.horizontal_walls[x][c])
}

fn vertical_boundary_usable(input: &Input, r: usize, c: usize, h: usize) -> bool {
    (r..r + h).all(|x| !input.vertical_walls[x][c])
}

/// 注目カードを同じ p->q へ動かしたまま、直交方向の正益 strip を無料で束ねる。
fn best_expanded_operation(
    state: &State,
    p: usize,
    q: usize,
    active: CellMask,
    input: &Input,
    catalog: &Catalog,
) -> Option<(Operation, i32)> {
    let mut best: Option<(Operation, i32)> = None;
    for &id in &catalog.pair_options[pair_key(p, q)] {
        let line = catalog.line_options[id as usize];
        if !line.mask.is_subset_of(active) {
            continue;
        }
        let op = match line.op.direction {
            Direction::Horizontal => {
                let base = line.op.r;
                let c = line.op.c;
                let w = line.op.w;
                let mut first = base;
                while first > 0
                    && horizontal_row_usable(input, active, first - 1, c, w)
                    && horizontal_boundary_usable(input, first - 1, c, w)
                {
                    first -= 1;
                }
                let mut end = base + 1;
                while end < N
                    && horizontal_row_usable(input, active, end, c, w)
                    && horizontal_boundary_usable(input, end - 1, c, w)
                {
                    end += 1;
                }

                let mut best_first = base;
                let mut sum = 0;
                let mut best_sum = 0;
                for r in (first..base).rev() {
                    sum += operation_gain(
                        state,
                        Operation {
                            direction: Direction::Horizontal,
                            r,
                            c,
                            h: 1,
                            w,
                        },
                        catalog,
                    );
                    if sum > best_sum {
                        best_sum = sum;
                        best_first = r;
                    }
                }

                let mut best_end = base + 1;
                sum = 0;
                best_sum = 0;
                for r in base + 1..end {
                    sum += operation_gain(
                        state,
                        Operation {
                            direction: Direction::Horizontal,
                            r,
                            c,
                            h: 1,
                            w,
                        },
                        catalog,
                    );
                    if sum > best_sum {
                        best_sum = sum;
                        best_end = r + 1;
                    }
                }
                Operation {
                    direction: Direction::Horizontal,
                    r: best_first,
                    c,
                    h: best_end - best_first,
                    w,
                }
            }
            Direction::Vertical => {
                let base = line.op.c;
                let r = line.op.r;
                let h = line.op.h;
                let mut first = base;
                while first > 0
                    && vertical_col_usable(input, active, r, first - 1, h)
                    && vertical_boundary_usable(input, r, first - 1, h)
                {
                    first -= 1;
                }
                let mut end = base + 1;
                while end < N
                    && vertical_col_usable(input, active, r, end, h)
                    && vertical_boundary_usable(input, r, end - 1, h)
                {
                    end += 1;
                }

                let mut best_first = base;
                let mut sum = 0;
                let mut best_sum = 0;
                for c in (first..base).rev() {
                    sum += operation_gain(
                        state,
                        Operation {
                            direction: Direction::Vertical,
                            r,
                            c,
                            h,
                            w: 1,
                        },
                        catalog,
                    );
                    if sum > best_sum {
                        best_sum = sum;
                        best_first = c;
                    }
                }

                let mut best_end = base + 1;
                sum = 0;
                best_sum = 0;
                for c in base + 1..end {
                    sum += operation_gain(
                        state,
                        Operation {
                            direction: Direction::Vertical,
                            r,
                            c,
                            h,
                            w: 1,
                        },
                        catalog,
                    );
                    if sum > best_sum {
                        best_sum = sum;
                        best_end = c + 1;
                    }
                }
                Operation {
                    direction: Direction::Vertical,
                    r,
                    c: best_first,
                    h,
                    w: best_end - best_first,
                }
            }
        };
        debug_assert!(operation_is_legal(input, op));
        let gain = operation_gain(state, op, catalog);
        let replace = match best {
            None => true,
            Some((old, old_gain)) => {
                gain > old_gain || (gain == old_gain && op.area() < old.area())
            }
        };
        if replace {
            best = Some((op, gain));
        }
    }
    best
}

/// 最初の保証解では直交拡張を省き、最小支持だけを評価して計算量を抑える。
fn best_mapping_operation(
    state: &State,
    p: usize,
    q: usize,
    active: CellMask,
    input: &Input,
    catalog: &Catalog,
    expand: bool,
) -> Option<(Operation, i32)> {
    if expand {
        return best_expanded_operation(state, p, q, active, input, catalog);
    }
    let mut best: Option<(Operation, i32)> = None;
    for &id in &catalog.pair_options[pair_key(p, q)] {
        let line = catalog.line_options[id as usize];
        if !line.mask.is_subset_of(active) {
            continue;
        }
        let gain = operation_gain(state, line.op, catalog);
        let replace = match best {
            None => true,
            Some((old, old_gain)) => {
                gain > old_gain || (gain == old_gain && line.op.area() < old.area())
            }
        };
        if replace {
            best = Some((line.op, gain));
        }
    }
    best
}

fn articulation_points(catalog: &Catalog, active: CellMask) -> [bool; CELLS] {
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        u: usize,
        parent: usize,
        catalog: &Catalog,
        active: CellMask,
        timer: &mut usize,
        disc: &mut [usize; CELLS],
        low: &mut [usize; CELLS],
        articulation: &mut [bool; CELLS],
    ) {
        disc[u] = *timer;
        low[u] = *timer;
        *timer += 1;
        let mut children = 0;
        for &v16 in &catalog.unit_neighbors[u] {
            let v = v16 as usize;
            if !active.contains(v) {
                continue;
            }
            if disc[v] == usize::MAX {
                children += 1;
                dfs(v, u, catalog, active, timer, disc, low, articulation);
                low[u] = low[u].min(low[v]);
                if parent != usize::MAX && low[v] >= disc[u] {
                    articulation[u] = true;
                }
            } else if v != parent {
                low[u] = low[u].min(disc[v]);
            }
        }
        if parent == usize::MAX && children > 1 {
            articulation[u] = true;
        }
    }

    let mut articulation = [false; CELLS];
    let mut disc = [usize::MAX; CELLS];
    let mut low = [0; CELLS];
    let mut timer = 0;
    if let Some(start) = (0..CELLS).find(|&p| active.contains(p)) {
        dfs(
            start,
            usize::MAX,
            catalog,
            active,
            &mut timer,
            &mut disc,
            &mut low,
            &mut articulation,
        );
        debug_assert_eq!(timer, (0..CELLS).filter(|&p| active.contains(p)).count());
    }
    articulation
}

#[derive(Clone, Copy)]
struct TrialPolicy {
    capacity_weight: i64,
    top_k: usize,
    target_noise: i64,
    hop_noise: i64,
    expand: bool,
}

#[derive(Default, Clone)]
struct PlanStats {
    macro_hops: i64,
    long_hops: i64,
    expanded_hops: i64,
    capacity_scored_targets: i64,
    merged_ops: i64,
    cancelled_ops: i64,
}

struct PlanResult {
    operations: Vec<Operation>,
    #[cfg_attr(not(feature = "local"), allow(dead_code))]
    stats: PlanStats,
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.state = x;
        x
    }

    fn bounded(&mut self, bound: i64) -> i64 {
        if bound <= 0 {
            0
        } else {
            (self.next() % bound as u64) as i64
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn choose_target(
    state: &State,
    active: CellMask,
    articulation: &[bool; CELLS],
    macro_graph: &[Vec<u16>],
    catalog: &Catalog,
    policy: TrialPolicy,
    rng: &mut XorShift64,
    stats: &mut PlanStats,
) -> (usize, [u16; CELLS]) {
    let mut ranked = Vec::new();
    for (target, &is_articulation) in articulation.iter().enumerate() {
        if !active.contains(target) || is_articulation {
            continue;
        }
        let source = state.position_of(target);
        let capacity = catalog.active_support_count(target, active) as i64;
        let estimate = catalog.full_macro_dist[target][source] as i64 * ROUTE_SCORE
            + capacity * policy.capacity_weight
            + rng.bounded(policy.target_noise + 1);
        ranked.push((estimate, target, capacity));
    }
    assert!(!ranked.is_empty());
    ranked.sort_unstable_by_key(|&(score, target, _)| (score, target));

    let mut best: Option<(i64, usize, [u16; CELLS])> = None;
    for &(_, target, capacity) in ranked.iter().take(policy.top_k.max(1)) {
        stats.capacity_scored_targets += 1;
        let dist = bfs_dist(macro_graph, target);
        let route = dist[state.position_of(target)];
        assert_ne!(route, INF);
        let score = route as i64 * ROUTE_SCORE
            + capacity * policy.capacity_weight
            + rng.bounded(policy.target_noise + 1);
        let replace = match &best {
            None => true,
            Some((old_score, old_target, _)) => {
                score < *old_score || (score == *old_score && target < *old_target)
            }
        };
        if replace {
            best = Some((score, target, dist));
        }
    }
    let (_, target, dist) = best.unwrap();
    (target, dist)
}

fn try_merge_operations(input: &Input, a: Operation, b: Operation) -> Option<Operation> {
    let merged = match (a.direction, b.direction) {
        (Direction::Horizontal, Direction::Horizontal) if a.c == b.c && a.w == b.w => {
            if a.r + a.h == b.r {
                Operation {
                    direction: Direction::Horizontal,
                    r: a.r,
                    c: a.c,
                    h: a.h + b.h,
                    w: a.w,
                }
            } else if b.r + b.h == a.r {
                Operation {
                    direction: Direction::Horizontal,
                    r: b.r,
                    c: a.c,
                    h: a.h + b.h,
                    w: a.w,
                }
            } else {
                return None;
            }
        }
        (Direction::Vertical, Direction::Vertical) if a.r == b.r && a.h == b.h => {
            if a.c + a.w == b.c {
                Operation {
                    direction: Direction::Vertical,
                    r: a.r,
                    c: a.c,
                    h: a.h,
                    w: a.w + b.w,
                }
            } else if b.c + b.w == a.c {
                Operation {
                    direction: Direction::Vertical,
                    r: a.r,
                    c: b.c,
                    h: a.h,
                    w: a.w + b.w,
                }
            } else {
                return None;
            }
        }
        _ => return None,
    };
    operation_is_legal(input, merged).then_some(merged)
}

fn record_operation(
    operations: &mut Vec<Operation>,
    mut op: Operation,
    input: &Input,
    stats: &mut PlanStats,
) {
    loop {
        let Some(&last) = operations.last() else {
            operations.push(op);
            return;
        };
        if last == op {
            operations.pop();
            stats.cancelled_ops += 1;
            return;
        }
        if let Some(merged) = try_merge_operations(input, last, op) {
            operations.pop();
            op = merged;
            stats.merged_ops += 1;
            continue;
        }
        operations.push(op);
        return;
    }
}

fn build_plan(
    input: &Input,
    catalog: &Catalog,
    policy: TrialPolicy,
    rng: &mut XorShift64,
    time_keeper: &TimeKeeper,
    deadline_sec: Option<f64>,
) -> Option<PlanResult> {
    let mut state = State::new(&input.initial_board);
    let mut active = CellMask::full();
    let mut remaining = CELLS;
    let mut operations = Vec::new();
    let mut stats = PlanStats::default();

    while remaining > 1 {
        if deadline_sec.is_some_and(|deadline| time_keeper.exact_elapsed_sec() >= deadline) {
            return None;
        }
        let articulation = articulation_points(catalog, active);
        let macro_graph = catalog.build_active_macro_graph(active);
        let (target, target_dist) = choose_target(
            &state,
            active,
            &articulation,
            &macro_graph,
            catalog,
            policy,
            rng,
            &mut stats,
        );

        while state.position_of(target) != target {
            if deadline_sec.is_some_and(|deadline| time_keeper.exact_elapsed_sec() >= deadline) {
                return None;
            }
            let p = state.position_of(target);
            let mut best: Option<(i64, Operation, usize, i32)> = None;
            for &q16 in &macro_graph[p] {
                let q = q16 as usize;
                if target_dist[q] == INF || target_dist[q] + 1 != target_dist[p] {
                    continue;
                }
                let Some((op, gain)) =
                    best_mapping_operation(&state, p, q, active, input, catalog, policy.expand)
                else {
                    continue;
                };
                let jitter = rng.bounded(2 * policy.hop_noise + 1) - policy.hop_noise;
                let score = gain as i64 + jitter;
                let replace = match best {
                    None => true,
                    Some((old_score, old_op, old_q, _)) => {
                        score > old_score
                            || (score == old_score && (op.area(), q) < (old_op.area(), old_q))
                    }
                };
                if replace {
                    best = Some((score, op, q, gain));
                }
            }
            let (_, op, q, _) = best.expect("macro shortest path must have a usable next hop");
            apply_operation(&mut state, op);
            record_operation(&mut operations, op, input, &mut stats);
            stats.macro_hops += 1;
            let shift = if p / N == q / N {
                p.abs_diff(q)
            } else {
                (p / N).abs_diff(q / N)
            };
            if shift > 1 {
                stats.long_hops += 1;
            }
            if (op.direction == Direction::Horizontal && op.h > 1)
                || (op.direction == Direction::Vertical && op.w > 1)
            {
                stats.expanded_hops += 1;
            }
            assert_eq!(state.position_of(target), q);
        }

        assert_eq!(state.card_at(target), target);
        active.remove(target);
        remaining -= 1;
        local! {
            if remaining % 32 == 0 {
                verify_active_invariants(&state, active, catalog);
            }
        }
    }

    assert!(state.is_complete());
    assert!(operations.len() <= MAX_OPERATIONS);
    local! {
        verify_active_invariants(&state, active, catalog);
    }
    Some(PlanResult { operations, stats })
}

#[cfg(feature = "local")]
fn verify_active_invariants(state: &State, active: CellMask, catalog: &Catalog) {
    let mut misplaced = 0;
    let mut row_mismatch = 0;
    let mut col_mismatch = 0;
    for cell in 0..CELLS {
        let card = state.card_at(cell);
        assert_eq!(state.position_of(card), cell);
        misplaced += (card != cell) as u16;
        row_mismatch += (card / N != cell / N) as u16;
        col_mismatch += (card % N != cell % N) as u16;
        if !active.contains(cell) {
            assert_eq!(card, cell);
        } else {
            assert!(active.contains(card));
        }
    }
    assert_eq!(misplaced, state.board_state.misplaced_count);
    assert_eq!(row_mismatch, state.board_state.row_mismatch_count);
    assert_eq!(col_mismatch, state.board_state.col_mismatch_count);

    let active_count = (0..CELLS).filter(|&p| active.contains(p)).count();
    if active_count > 0 {
        let start = (0..CELLS).find(|&p| active.contains(p)).unwrap();
        let mut seen = [false; CELLS];
        let mut queue = VecDeque::new();
        seen[start] = true;
        queue.push_back(start);
        let mut count = 0;
        while let Some(p) = queue.pop_front() {
            count += 1;
            for &q16 in &catalog.unit_neighbors[p] {
                let q = q16 as usize;
                if active.contains(q) && !seen[q] {
                    seen[q] = true;
                    queue.push_back(q);
                }
            }
        }
        assert_eq!(count, active_count);
    }
}

fn input_seed(input: &Input) -> u64 {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    for (cell, &card) in input.initial_board.iter().enumerate() {
        seed ^= (card as u64 + 0x9e37_79b9 + cell as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        seed = seed.rotate_left(17).wrapping_mul(0x94d0_49bb_1331_11eb);
    }
    seed
}

fn policy_for_trial(trial: usize) -> TrialPolicy {
    const CAPACITY_WEIGHTS: [i64; 7] = [0, 2, 4, 8, 12, 20, 32];
    if trial == 0 {
        return TrialPolicy {
            capacity_weight: 12,
            top_k: 1,
            target_noise: 0,
            hop_noise: 0,
            expand: false,
        };
    }
    let variant = trial - 1;
    let round = variant / CAPACITY_WEIGHTS.len();
    TrialPolicy {
        capacity_weight: CAPACITY_WEIGHTS[variant % CAPACITY_WEIGHTS.len()],
        top_k: 16,
        target_noise: if round == 0 {
            0
        } else {
            (round.min(3) as i64) * (ROUTE_SCORE / 2)
        },
        hop_noise: if round <= 1 {
            0
        } else {
            64 * (round.min(4) as i64 - 1)
        },
        expand: true,
    }
}

fn main() {
    // 探索時間は入力処理も含むため、timer を main の先頭で作る。
    let time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let input = Input::read();
    let catalog = Catalog::build(&input);
    let mut rng = XorShift64::new(input_seed(&input));

    // 最初の一本は期限を設けず、完全解の存在を構造的に保証する。
    let mut best = build_plan(
        &input,
        &catalog,
        policy_for_trial(0),
        &mut rng,
        &time_keeper,
        None,
    )
    .unwrap();
    let mut _complete_plans = 1_i64;
    let mut _aborted_plans = 0_i64;
    let mut _best_updates = 0_i64;
    let mut trial = 1;

    while time_keeper.ratio() < NEW_PLAN_DEADLINE_RATIO {
        let deadline = PROGRAM_TIME_LIMIT_SEC * ACTIVE_PLAN_DEADLINE_RATIO;
        let result = build_plan(
            &input,
            &catalog,
            policy_for_trial(trial),
            &mut rng,
            &time_keeper,
            Some(deadline),
        );
        trial += 1;
        let Some(candidate) = result else {
            _aborted_plans += 1;
            break;
        };
        _complete_plans += 1;
        if candidate.operations.len() < best.operations.len() {
            best = candidate;
            _best_updates += 1;
        }
    }

    assert!(best.operations.len() <= MAX_OPERATIONS);
    local! {
        let mut replay = State::new(&input.initial_board);
        for &op in &best.operations {
            assert!(operation_is_legal(&input, op));
            apply_operation(&mut replay, op);
        }
        assert!(replay.is_complete());

        let mut trace = TraceStats::default();
        trace.count_by("complete_plans", _complete_plans);
        trace.count_by("aborted_plans", _aborted_plans);
        trace.count_by("best_updates", _best_updates);
        trace.count_by("macro_hops", best.stats.macro_hops);
        trace.count_by("long_hops", best.stats.long_hops);
        trace.count_by("expanded_hops", best.stats.expanded_hops);
        trace.count_by("capacity_scored_targets", best.stats.capacity_scored_targets);
        trace.count_by("merged_ops", best.stats.merged_ops);
        trace.count_by("cancelled_ops", best.stats.cancelled_ops);
        trace.count_by("final_t", best.operations.len() as i64);
        trace.add_time_ms("search", time_keeper.exact_elapsed_sec() * 1000.0);
        trace.summary();
    }
    write_output(&best.operations);
}
