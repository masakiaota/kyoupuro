// v030_connected_peel.rs
// active 未完成セルを壁なし隣接グラフとして扱い、非関節点のうち開辺次数が
// 最小のセルから確定する。袋小路・細通路を先に処理し、中央の高次数な連結コアと
// 大きな長方形による一括輸送の余地を終盤まで残す。
use std::{
    collections::{HashSet, VecDeque},
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
#[allow(clippy::manual_div_ceil)] // Rust 1.70 互換を保つ。
const ACTIVE_WORDS: usize = (CELLS + 63) / 64;
const MAX_SINGLE_AXIS_SHIFT: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 最終 replay と出力の余白を残しつつ、探索には制限時間の大半を使う。
const SEARCH_TIME_RATIO: f64 = 0.95;
/// v022_cost_weight で実測較正された手数と木距離和の換算重み。
const EVAL_COST_WEIGHT: u64 = 4;
const MAX_BEAM_WIDTH: usize = 8192;
const NO_CELL: u16 = u16::MAX;

#[inline]
fn range_mask(start: usize, len: usize) -> u32 {
    debug_assert!(start + len <= N);
    if len == 0 {
        0
    } else {
        ((1_u32 << len) - 1) << start
    }
}

struct Input {
    initial_board: [usize; CELLS],
    vertical_walls: [[bool; N - 1]; N],
    horizontal_walls: [[bool; N]; N - 1],
    // 矩形の壁判定を定数時間にする直交二方向のbit mask。
    vertical_wall_rows: [u32; N],
    vertical_wall_boundaries: [u32; N - 1],
    horizontal_wall_boundaries: [u32; N - 1],
    horizontal_wall_cols: [u32; N],
}

impl Input {
    fn read() -> Self {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap();
        let mut tokens = source.split_whitespace();

        let input_n: usize = tokens.next().unwrap().parse().unwrap();
        assert_eq!(input_n, N);
        let initial_board = std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        let vertical_wall_rows = std::array::from_fn(|i| {
            (0..N - 1).fold(0_u32, |mask, j| mask | ((vertical_walls[i][j] as u32) << j))
        });
        let vertical_wall_boundaries = std::array::from_fn(|j| {
            (0..N).fold(0_u32, |mask, i| mask | ((vertical_walls[i][j] as u32) << i))
        });
        let horizontal_wall_boundaries = std::array::from_fn(|i| {
            (0..N).fold(0_u32, |mask, j| {
                mask | ((horizontal_walls[i][j] as u32) << j)
            })
        });
        let horizontal_wall_cols = std::array::from_fn(|j| {
            (0..N - 1).fold(0_u32, |mask, i| {
                mask | ((horizontal_walls[i][j] as u32) << i)
            })
        });

        Self {
            initial_board,
            vertical_walls,
            horizontal_walls,
            vertical_wall_rows,
            vertical_wall_boundaries,
            horizontal_wall_boundaries,
            horizontal_wall_cols,
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

#[cfg(feature = "local")]
#[derive(Debug, Default)]
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

#[cfg(not(feature = "local"))]
#[derive(Debug, Default)]
struct TraceStats;

#[cfg(not(feature = "local"))]
impl TraceStats {
    #[inline]
    fn count_by(&mut self, _key: &'static str, _delta: i64) {}

    #[inline]
    fn add_time_ms(&mut self, _key: &'static str, _ms: f64) {}

    #[inline]
    fn summary(&self) {}
}

#[cfg(feature = "local")]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let start = Instant::now();
        let result = { $body };
        $trace.add_time_ms($key, start.elapsed().as_secs_f64() * 1000.0);
        result
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        $body
    }};
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
    iter: u64,
    check_mask: u64,
    elapsed_sec: f64,
    is_over: bool,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        assert!(time_limit_sec > 0.0);
        assert!(check_interval_log2 < 63);
        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };
        let mut result = Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            elapsed_sec: 0.0,
            is_over: false,
        };
        result.force_update();
        result
    }

    #[inline(always)]
    fn step(&mut self) {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
    }

    #[inline(always)]
    fn force_update(&mut self) {
        self.elapsed_sec = self.start.elapsed().as_secs_f64();
        self.is_over = self.elapsed_sec >= self.time_limit_sec;
    }

    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

/// 固定木は従来と同じ残り手数 proxy にだけ使う。確定候補と搬送経路は、
/// その時点の active 開グラフから動的に決める。
struct TreeData {
    distance: Vec<u16>,
    freedom: [u32; CELLS],
}

fn open_neighbors(input: &Input, cell: usize) -> ([usize; 4], usize) {
    let i = cell / N;
    let j = cell % N;
    let mut result = [CELLS; 4];
    let mut len = 0;

    // 親の同点選択を固定するため、順序も固定する。
    if i > 0 && !input.horizontal_walls[i - 1][j] {
        result[len] = cell - N;
        len += 1;
    }
    if j > 0 && !input.vertical_walls[i][j - 1] {
        result[len] = cell - 1;
        len += 1;
    }
    if j + 1 < N && !input.vertical_walls[i][j] {
        result[len] = cell + 1;
        len += 1;
    }
    if i + 1 < N && !input.horizontal_walls[i][j] {
        result[len] = cell + N;
        len += 1;
    }
    (result, len)
}

/// そのセルを動かす合法操作の総数。値が小さいほど細通路・袋小路寄りである。
fn count_cell_freedom(input: &Input) -> [u32; CELLS] {
    let mut freedom = [0_u32; CELLS];
    for r in 0..N {
        for c in 0..N {
            for h in 1..=N - r {
                for w in 1..=N - c {
                    if h % 2 == 0 {
                        let op = Operation {
                            direction: Direction::Vertical,
                            r,
                            c,
                            h,
                            w,
                        };
                        if is_legal_operation(input, op) {
                            for i in r..r + h {
                                for j in c..c + w {
                                    freedom[i * N + j] += 1;
                                }
                            }
                        }
                    }
                    if w % 2 == 0 {
                        let op = Operation {
                            direction: Direction::Horizontal,
                            r,
                            c,
                            h,
                            w,
                        };
                        if is_legal_operation(input, op) {
                            for i in r..r + h {
                                for j in c..c + w {
                                    freedom[i * N + j] += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(freedom.iter().all(|&value| value > 0));
    freedom
}

fn build_spanning_tree(input: &Input) -> TreeData {
    let freedom = count_cell_freedom(input);
    let root = (N / 2) * N + N / 2;
    let mut parent = [NO_CELL; CELLS];
    parent[root] = root as u16;
    let mut queue = VecDeque::with_capacity(CELLS);
    queue.push_back(root);

    while let Some(cell) = queue.pop_front() {
        let (neighbors, len) = open_neighbors(input, cell);
        for &next in &neighbors[..len] {
            if parent[next] == NO_CELL {
                parent[next] = cell as u16;
                queue.push_back(next);
            }
        }
    }
    assert!(parent.iter().all(|&p| p != NO_CELL));

    let mut tree_neighbors = [[NO_CELL; 4]; CELLS];
    let mut neighbor_count = [0_u8; CELLS];
    for cell in 0..CELLS {
        if cell == root {
            continue;
        }
        let p = parent[cell] as usize;
        let a = neighbor_count[cell] as usize;
        let b = neighbor_count[p] as usize;
        tree_neighbors[cell][a] = p as u16;
        tree_neighbors[p][b] = cell as u16;
        neighbor_count[cell] += 1;
        neighbor_count[p] += 1;
    }

    let mut distance = vec![u16::MAX; CELLS * CELLS];
    for source in 0..CELLS {
        let base = source * CELLS;
        distance[base + source] = 0;
        let mut bfs = VecDeque::with_capacity(CELLS);
        bfs.push_back(source);
        while let Some(cell) = bfs.pop_front() {
            let current_distance = distance[base + cell];
            for k in 0..neighbor_count[cell] as usize {
                let to = tree_neighbors[cell][k] as usize;
                if distance[base + to] != u16::MAX {
                    continue;
                }
                distance[base + to] = current_distance + 1;
                bfs.push_back(to);
            }
        }
    }

    TreeData { distance, freedom }
}

#[derive(Clone)]
struct Zobrist {
    board_a: Vec<u64>,
    board_b: Vec<u64>,
    active_a: [u64; CELLS],
    active_b: [u64; CELLS],
}

#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

impl Zobrist {
    fn new() -> Self {
        let mut seed = 0x41ac_6800_5eed_f00d;
        let mut board_a = Vec::with_capacity(CELLS * CELLS);
        let mut board_b = Vec::with_capacity(CELLS * CELLS);
        for _ in 0..CELLS * CELLS {
            board_a.push(splitmix64(&mut seed));
            board_b.push(splitmix64(&mut seed));
        }
        let active_a = std::array::from_fn(|_| splitmix64(&mut seed));
        let active_b = std::array::from_fn(|_| splitmix64(&mut seed));
        Self {
            board_a,
            board_b,
            active_a,
            active_b,
        }
    }

    #[inline]
    fn board_index(cell: usize, card: usize) -> usize {
        cell * CELLS + card
    }
}

#[derive(Clone, PartialEq, Eq)]
struct SearchState {
    board: [u16; CELLS],
    position: [u16; CELLS],
    active: [u64; ACTIVE_WORDS],
    // 支持laneの全active判定を定数時間にする転置mask。
    active_rows: [u32; N],
    active_cols: [u32; N],
    // active 開グラフ上の動的次数。
    degree: [u8; CELLS],
    active_count: u16,
    // 除去時の動的開辺次数を累積し、低次数の連結端を先にした履歴を優先する。
    degree_prefix: u16,
    freedom_prefix: u64,
    operations_used: u32,
    potential: i32,
    misplaced_count: u16,
    hash_a: u64,
    hash_b: u64,
    history_id: u32,
}

impl SearchState {
    fn new(input: &Input, tree: &TreeData, zobrist: &Zobrist) -> Self {
        let board = std::array::from_fn(|cell| input.initial_board[cell] as u16);
        let mut position = [0_u16; CELLS];
        let mut active = [0_u64; ACTIVE_WORDS];
        let mut potential = 0_i32;
        let mut misplaced_count = 0_u16;
        let mut hash_a = 0_u64;
        let mut hash_b = 0_u64;

        for cell in 0..CELLS {
            let card = board[cell] as usize;
            position[card] = cell as u16;
            active[cell >> 6] |= 1_u64 << (cell & 63);
            potential += tree.distance[cell * CELLS + card] as i32;
            misplaced_count += (cell != card) as u16;
            let z = Zobrist::board_index(cell, card);
            hash_a ^= zobrist.board_a[z] ^ zobrist.active_a[cell];
            hash_b ^= zobrist.board_b[z] ^ zobrist.active_b[cell];
        }

        let full_mask = range_mask(0, N);

        Self {
            board,
            position,
            active,
            active_rows: [full_mask; N],
            active_cols: [full_mask; N],
            degree: std::array::from_fn(|cell| open_neighbors(input, cell).1 as u8),
            active_count: CELLS as u16,
            degree_prefix: 0,
            freedom_prefix: 0,
            operations_used: 0,
            potential,
            misplaced_count,
            hash_a,
            hash_b,
            history_id: 0,
        }
    }

    #[inline]
    fn is_active(&self, cell: usize) -> bool {
        ((self.active[cell >> 6] >> (cell & 63)) & 1) != 0
    }

    #[inline]
    fn set_active(&mut self, cell: usize, value: bool) {
        let bit = 1_u64 << (cell & 63);
        let row = cell / N;
        let col = cell % N;
        let row_bit = 1_u32 << col;
        let col_bit = 1_u32 << row;
        if value {
            self.active[cell >> 6] |= bit;
            self.active_rows[row] |= row_bit;
            self.active_cols[col] |= col_bit;
        } else {
            self.active[cell >> 6] &= !bit;
            self.active_rows[row] &= !row_bit;
            self.active_cols[col] &= !col_bit;
        }
    }

    #[inline]
    fn score(&self) -> u64 {
        self.operations_used as u64 * EVAL_COST_WEIGHT + self.potential as u64
    }
}

/// active 開グラフの関節点を Tarjan 法で求める。関節点を確定すると未完成領域が
/// 分断されるため候補から除外する。
fn articulation_points(input: &Input, state: &SearchState) -> [bool; CELLS] {
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        input: &Input,
        state: &SearchState,
        cell: usize,
        parent: usize,
        timer: &mut usize,
        discovery: &mut [usize; CELLS],
        low: &mut [usize; CELLS],
        articulation: &mut [bool; CELLS],
    ) {
        discovery[cell] = *timer;
        low[cell] = *timer;
        *timer += 1;
        let mut child_count = 0;
        let (neighbors, len) = open_neighbors(input, cell);
        for &next in &neighbors[..len] {
            if !state.is_active(next) {
                continue;
            }
            if discovery[next] == usize::MAX {
                child_count += 1;
                dfs(
                    input,
                    state,
                    next,
                    cell,
                    timer,
                    discovery,
                    low,
                    articulation,
                );
                low[cell] = low[cell].min(low[next]);
                if parent != CELLS && low[next] >= discovery[cell] {
                    articulation[cell] = true;
                }
            } else if next != parent {
                low[cell] = low[cell].min(discovery[next]);
            }
        }
        if parent == CELLS && child_count > 1 {
            articulation[cell] = true;
        }
    }

    let mut articulation = [false; CELLS];
    let mut discovery = [usize::MAX; CELLS];
    let mut low = [0_usize; CELLS];
    let mut timer = 0;
    let start = (0..CELLS).find(|&cell| state.is_active(cell)).unwrap();
    dfs(
        input,
        state,
        start,
        CELLS,
        &mut timer,
        &mut discovery,
        &mut low,
        &mut articulation,
    );
    assert_eq!(timer, state.active_count as usize);
    articulation
}

/// target から active 開グラフを BFS し、各セルから target へ一歩近づく隣接セルを返す。
fn active_next_to_target(input: &Input, state: &SearchState, target: usize) -> [u16; CELLS] {
    assert!(state.is_active(target));
    let mut next_step = [NO_CELL; CELLS];
    next_step[target] = target as u16;
    let mut queue = VecDeque::with_capacity(state.active_count as usize);
    queue.push_back(target);
    let mut seen_count = 0;
    while let Some(cell) = queue.pop_front() {
        seen_count += 1;
        let (neighbors, len) = open_neighbors(input, cell);
        for &next in &neighbors[..len] {
            if state.is_active(next) && next_step[next] == NO_CELL {
                next_step[next] = cell as u16;
                queue.push_back(next);
            }
        }
    }
    assert_eq!(seen_count, state.active_count as usize);
    next_step
}

#[inline]
fn swap_cells(
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    first: usize,
    second: usize,
) {
    let card_first = state.board[first] as usize;
    let card_second = state.board[second] as usize;

    let old_potential = tree.distance[first * CELLS + card_first] as i32
        + tree.distance[second * CELLS + card_second] as i32;
    let new_potential = tree.distance[second * CELLS + card_first] as i32
        + tree.distance[first * CELLS + card_second] as i32;
    state.potential += new_potential - old_potential;

    let old_misplaced = (card_first != first) as i32 + (card_second != second) as i32;
    let new_misplaced = (card_first != second) as i32 + (card_second != first) as i32;
    state.misplaced_count = (state.misplaced_count as i32 + new_misplaced - old_misplaced) as u16;

    let ff = Zobrist::board_index(first, card_first);
    let ss = Zobrist::board_index(second, card_second);
    let fs = Zobrist::board_index(first, card_second);
    let sf = Zobrist::board_index(second, card_first);
    state.hash_a ^=
        zobrist.board_a[ff] ^ zobrist.board_a[ss] ^ zobrist.board_a[fs] ^ zobrist.board_a[sf];
    state.hash_b ^=
        zobrist.board_b[ff] ^ zobrist.board_b[ss] ^ zobrist.board_b[fs] ^ zobrist.board_b[sf];

    state.board.swap(first, second);
    state.position[card_first] = second as u16;
    state.position[card_second] = first as u16;
}

fn operation_delta_potential(state: &SearchState, tree: &TreeData, op: Operation) -> i32 {
    let mut delta = 0_i32;
    match op.direction {
        Direction::Vertical => {
            let half = op.h / 2;
            for x in 0..half {
                for y in 0..op.w {
                    let first = (op.r + x) * N + op.c + y;
                    let second = (op.r + half + x) * N + op.c + y;
                    let a = state.board[first] as usize;
                    let b = state.board[second] as usize;
                    delta += tree.distance[second * CELLS + a] as i32
                        + tree.distance[first * CELLS + b] as i32
                        - tree.distance[first * CELLS + a] as i32
                        - tree.distance[second * CELLS + b] as i32;
                }
            }
        }
        Direction::Horizontal => {
            let half = op.w / 2;
            for x in 0..op.h {
                for y in 0..half {
                    let first = (op.r + x) * N + op.c + y;
                    let second = (op.r + x) * N + op.c + half + y;
                    let a = state.board[first] as usize;
                    let b = state.board[second] as usize;
                    delta += tree.distance[second * CELLS + a] as i32
                        + tree.distance[first * CELLS + b] as i32
                        - tree.distance[first * CELLS + a] as i32
                        - tree.distance[second * CELLS + b] as i32;
                }
            }
        }
    }
    delta
}

fn apply_operation(
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    op: Operation,
) -> usize {
    let mut pair_count = 0;
    match op.direction {
        Direction::Vertical => {
            let half = op.h / 2;
            for x in 0..half {
                for y in 0..op.w {
                    let first = (op.r + x) * N + op.c + y;
                    let second = (op.r + half + x) * N + op.c + y;
                    swap_cells(state, tree, zobrist, first, second);
                    pair_count += 1;
                }
            }
        }
        Direction::Horizontal => {
            let half = op.w / 2;
            for x in 0..op.h {
                for y in 0..half {
                    let first = (op.r + x) * N + op.c + y;
                    let second = (op.r + x) * N + op.c + half + y;
                    swap_cells(state, tree, zobrist, first, second);
                    pair_count += 1;
                }
            }
        }
    }
    pair_count
}

fn is_legal_operation(input: &Input, op: Operation) -> bool {
    if op.h == 0 || op.w == 0 || op.r + op.h > N || op.c + op.w > N {
        return false;
    }
    match op.direction {
        Direction::Vertical if op.h % 2 != 0 => return false,
        Direction::Horizontal if op.w % 2 != 0 => return false,
        _ => {}
    }
    for i in op.r..op.r + op.h {
        for j in op.c..op.c + op.w.saturating_sub(1) {
            if input.vertical_walls[i][j] {
                return false;
            }
        }
    }
    for i in op.r..op.r + op.h.saturating_sub(1) {
        for j in op.c..op.c + op.w {
            if input.horizontal_walls[i][j] {
                return false;
            }
        }
    }
    true
}

fn is_active_legal_operation(input: &Input, state: &SearchState, op: Operation) -> bool {
    if !is_legal_operation(input, op) {
        return false;
    }
    for i in op.r..op.r + op.h {
        for j in op.c..op.c + op.w {
            if !state.is_active(i * N + j) {
                return false;
            }
        }
    }
    true
}

fn mapped_cell(op: Operation, cell: usize) -> Option<usize> {
    let i = cell / N;
    let j = cell % N;
    if i < op.r || i >= op.r + op.h || j < op.c || j >= op.c + op.w {
        return None;
    }
    match op.direction {
        Direction::Vertical => {
            let half = op.h / 2;
            let local_i = i - op.r;
            let mapped_i = if local_i < half { i + half } else { i - half };
            Some(mapped_i * N + j)
        }
        Direction::Horizontal => {
            let half = op.w / 2;
            let local_j = j - op.c;
            let mapped_j = if local_j < half { j + half } else { j - half };
            Some(i * N + mapped_j)
        }
    }
}

#[inline]
fn horizontal_lane_usable(
    input: &Input,
    state: &SearchState,
    row: usize,
    c: usize,
    width: usize,
) -> bool {
    let cells = range_mask(c, width);
    let inner_boundaries = range_mask(c, width - 1);
    state.active_rows[row] & cells == cells && input.vertical_wall_rows[row] & inner_boundaries == 0
}

#[inline]
fn horizontal_boundary_open(input: &Input, boundary: usize, c: usize, width: usize) -> bool {
    input.horizontal_wall_boundaries[boundary] & range_mask(c, width) == 0
}

#[inline]
fn vertical_lane_usable(
    input: &Input,
    state: &SearchState,
    col: usize,
    r: usize,
    height: usize,
) -> bool {
    let cells = range_mask(r, height);
    let inner_boundaries = range_mask(r, height - 1);
    state.active_cols[col] & cells == cells
        && input.horizontal_wall_cols[col] & inner_boundaries == 0
}

#[inline]
fn vertical_boundary_open(input: &Input, boundary: usize, r: usize, height: usize) -> bool {
    input.vertical_wall_boundaries[boundary] & range_mask(r, height) == 0
}

#[inline]
fn swap_pair_delta(state: &SearchState, tree: &TreeData, first: usize, second: usize) -> i32 {
    let first_card = state.board[first] as usize;
    let second_card = state.board[second] as usize;
    tree.distance[second * CELLS + first_card] as i32
        + tree.distance[first * CELLS + second_card] as i32
        - tree.distance[first * CELLS + first_card] as i32
        - tree.distance[second * CELLS + second_card] as i32
}

fn opposing_pair_count(state: &SearchState, op: Operation) -> usize {
    let mut count = 0;
    match op.direction {
        Direction::Horizontal => {
            let shift = op.w / 2;
            for x in 0..op.h {
                for y in 0..shift {
                    let first = (op.r + x) * N + op.c + y;
                    let second = first + shift;
                    let first_card = state.board[first] as usize;
                    let second_card = state.board[second] as usize;
                    let first_target = first_card % N;
                    let second_target = second_card % N;
                    let first_before = first_target.abs_diff(first % N);
                    let first_after = first_target.abs_diff(second % N);
                    let second_before = second_target.abs_diff(second % N);
                    let second_after = second_target.abs_diff(first % N);
                    count +=
                        usize::from(first_after < first_before && second_after < second_before);
                }
            }
        }
        Direction::Vertical => {
            let shift = op.h / 2;
            for x in 0..shift {
                for y in 0..op.w {
                    let first = (op.r + x) * N + op.c + y;
                    let second = first + shift * N;
                    let first_card = state.board[first] as usize;
                    let second_card = state.board[second] as usize;
                    let first_target = first_card / N;
                    let second_target = second_card / N;
                    let first_before = first_target.abs_diff(first / N);
                    let first_after = first_target.abs_diff(second / N);
                    let second_before = second_target.abs_diff(second / N);
                    let second_after = second_target.abs_diff(first / N);
                    count +=
                        usize::from(first_after < first_before && second_after < second_before);
                }
            }
        }
    }
    count
}

#[derive(Clone, Copy)]
struct JointChoice {
    op: Operation,
    delta: i32,
    long_position: usize,
    limit_start: usize,
    limit_end: usize,
    crosses_greedy_barrier: bool,
}

/// 同じ from -> to を実現する全長手位置と、anchor を含む全合法支持区間を共同選択する。
/// lane delta は加法的なので、anchor の両側を最後まで走査し、累積和が最小の端点を
/// 独立に選べば、anchor を含む厳密 min-sum 区間になる。
fn best_joint_mapping_operation(
    input: &Input,
    state: &SearchState,
    tree: &TreeData,
    from: usize,
    to: usize,
    stats: &mut MechanismStats,
) -> Option<Operation> {
    stats.joint_mapping_calls += 1;
    let from_i = from / N;
    let from_j = from % N;
    let to_i = to / N;
    let to_j = to % N;

    let choice = if from_i == to_i {
        let shift = from_j.abs_diff(to_j);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let width = 2 * shift;
        let anchor = from_i;
        let mut positions = [0_usize; N];
        let mut position_count = 0;
        for c in 0..=N - width {
            let thin = Operation {
                direction: Direction::Horizontal,
                r: anchor,
                c,
                h: 1,
                w: width,
            };
            if mapped_cell(thin, from) == Some(to)
                && horizontal_lane_usable(input, state, anchor, c, width)
            {
                positions[position_count] = c;
                position_count += 1;
            }
        }
        if position_count == 0 {
            return None;
        }
        stats.joint_positions_evaluated += position_count as u64;

        let mut support_start = [anchor; N];
        let mut support_end = [anchor; N];
        let mut needed_lanes = 1_u32 << anchor;
        for &c in &positions[..position_count] {
            let mut top = anchor;
            while top > 0
                && horizontal_boundary_open(input, top - 1, c, width)
                && horizontal_lane_usable(input, state, top - 1, c, width)
            {
                top -= 1;
            }
            let mut bottom = anchor;
            while bottom + 1 < N
                && horizontal_boundary_open(input, bottom, c, width)
                && horizontal_lane_usable(input, state, bottom + 1, c, width)
            {
                bottom += 1;
            }
            support_start[c] = top;
            support_end[c] = bottom;
            needed_lanes |= range_mask(top, bottom - top + 1);
        }

        // lane_delta[row][c]。候補窓を直接足す場合と、全pairのprefixを作る場合の
        // 小さい方を選び、合法支持になり得るlaneだけを O(N^2) pair 評価以内で埋める。
        let mut lane_delta = [[0_i32; N]; N];
        for (row, row_delta) in lane_delta.iter_mut().enumerate() {
            if (needed_lanes >> row) & 1 == 0 {
                continue;
            }
            if position_count * shift <= N - shift {
                for &c in &positions[..position_count] {
                    let mut delta = 0;
                    for y in 0..shift {
                        let first = row * N + c + y;
                        delta += swap_pair_delta(state, tree, first, first + shift);
                    }
                    row_delta[c] = delta;
                }
            } else {
                let mut prefix = [0_i32; N + 1];
                for j in 0..N - shift {
                    let first = row * N + j;
                    prefix[j + 1] = prefix[j] + swap_pair_delta(state, tree, first, first + shift);
                }
                for &c in &positions[..position_count] {
                    row_delta[c] = prefix[c + shift] - prefix[c];
                }
            }
        }

        let mut thin_key = (i32::MAX, N);
        for &c in &positions[..position_count] {
            thin_key = thin_key.min((lane_delta[anchor][c], c));
        }

        let mut best_key: Option<(i32, usize, usize, usize)> = None;
        let mut best_choice = None;
        for &c in &positions[..position_count] {
            let mut top = anchor;
            let mut upper_sum = 0;
            let mut best_upper_sum = 0;
            for scan in (support_start[c]..anchor).rev() {
                upper_sum += lane_delta[scan][c];
                if upper_sum < best_upper_sum {
                    best_upper_sum = upper_sum;
                    top = scan;
                }
            }

            let mut bottom = anchor;
            let mut lower_sum = 0;
            let mut best_lower_sum = 0;
            for (scan, row_delta) in lane_delta
                .iter()
                .enumerate()
                .take(support_end[c] + 1)
                .skip(anchor + 1)
            {
                lower_sum += row_delta[c];
                if lower_sum < best_lower_sum {
                    best_lower_sum = lower_sum;
                    bottom = scan;
                }
            }

            let delta = lane_delta[anchor][c] + best_upper_sum + best_lower_sum;
            let support_len = bottom - top + 1;
            let key = (delta, c, support_len, top);
            let is_better = match best_key {
                Some(current) => key < current,
                None => true,
            };
            if is_better {
                let mut upper_prefix = 0;
                let upper_barrier = (top..anchor).rev().any(|row| {
                    upper_prefix += lane_delta[row][c];
                    upper_prefix >= 0
                });
                let mut lower_prefix = 0;
                let lower_barrier = (anchor + 1..=bottom).any(|row| {
                    lower_prefix += lane_delta[row][c];
                    lower_prefix >= 0
                });
                best_key = Some(key);
                best_choice = Some(JointChoice {
                    op: Operation {
                        direction: Direction::Horizontal,
                        r: top,
                        c,
                        h: support_len,
                        w: width,
                    },
                    delta,
                    long_position: c,
                    limit_start: support_start[c],
                    limit_end: support_end[c],
                    crosses_greedy_barrier: upper_barrier || lower_barrier,
                });
            }
        }
        let selected = best_choice.unwrap();
        debug_assert!(selected.delta <= thin_key.0);
        if selected.long_position != thin_key.1 {
            stats.joint_position_changed += 1;
        }
        stats.joint_potential_saved += (thin_key.0 - selected.delta) as i64;
        selected
    } else if from_j == to_j {
        let shift = from_i.abs_diff(to_i);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let height = 2 * shift;
        let anchor = from_j;
        let mut positions = [0_usize; N];
        let mut position_count = 0;
        for r in 0..=N - height {
            let thin = Operation {
                direction: Direction::Vertical,
                r,
                c: anchor,
                h: height,
                w: 1,
            };
            if mapped_cell(thin, from) == Some(to)
                && vertical_lane_usable(input, state, anchor, r, height)
            {
                positions[position_count] = r;
                position_count += 1;
            }
        }
        if position_count == 0 {
            return None;
        }
        stats.joint_positions_evaluated += position_count as u64;

        let mut support_start = [anchor; N];
        let mut support_end = [anchor; N];
        let mut needed_lanes = 1_u32 << anchor;
        for &r in &positions[..position_count] {
            let mut left = anchor;
            while left > 0
                && vertical_boundary_open(input, left - 1, r, height)
                && vertical_lane_usable(input, state, left - 1, r, height)
            {
                left -= 1;
            }
            let mut right = anchor;
            while right + 1 < N
                && vertical_boundary_open(input, right, r, height)
                && vertical_lane_usable(input, state, right + 1, r, height)
            {
                right += 1;
            }
            support_start[r] = left;
            support_end[r] = right;
            needed_lanes |= range_mask(left, right - left + 1);
        }

        // lane_delta[col][r]。横操作と同じくdirect/prefixの安い方で埋める。
        let mut lane_delta = [[0_i32; N]; N];
        for (col, col_delta) in lane_delta.iter_mut().enumerate() {
            if (needed_lanes >> col) & 1 == 0 {
                continue;
            }
            if position_count * shift <= N - shift {
                for &r in &positions[..position_count] {
                    let mut delta = 0;
                    for x in 0..shift {
                        let first = (r + x) * N + col;
                        delta += swap_pair_delta(state, tree, first, first + shift * N);
                    }
                    col_delta[r] = delta;
                }
            } else {
                let mut prefix = [0_i32; N + 1];
                for i in 0..N - shift {
                    let first = i * N + col;
                    prefix[i + 1] =
                        prefix[i] + swap_pair_delta(state, tree, first, first + shift * N);
                }
                for &r in &positions[..position_count] {
                    col_delta[r] = prefix[r + shift] - prefix[r];
                }
            }
        }

        let mut thin_key = (i32::MAX, N);
        for &r in &positions[..position_count] {
            thin_key = thin_key.min((lane_delta[anchor][r], r));
        }

        let mut best_key: Option<(i32, usize, usize, usize)> = None;
        let mut best_choice = None;
        for &r in &positions[..position_count] {
            let mut left = anchor;
            let mut left_sum = 0;
            let mut best_left_sum = 0;
            for scan in (support_start[r]..anchor).rev() {
                left_sum += lane_delta[scan][r];
                if left_sum < best_left_sum {
                    best_left_sum = left_sum;
                    left = scan;
                }
            }

            let mut right = anchor;
            let mut right_sum = 0;
            let mut best_right_sum = 0;
            for (scan, col_delta) in lane_delta
                .iter()
                .enumerate()
                .take(support_end[r] + 1)
                .skip(anchor + 1)
            {
                right_sum += col_delta[r];
                if right_sum < best_right_sum {
                    best_right_sum = right_sum;
                    right = scan;
                }
            }

            let delta = lane_delta[anchor][r] + best_left_sum + best_right_sum;
            let support_len = right - left + 1;
            let key = (delta, r, support_len, left);
            let is_better = match best_key {
                Some(current) => key < current,
                None => true,
            };
            if is_better {
                let mut left_prefix = 0;
                let left_barrier = (left..anchor).rev().any(|col| {
                    left_prefix += lane_delta[col][r];
                    left_prefix >= 0
                });
                let mut right_prefix = 0;
                let right_barrier = (anchor + 1..=right).any(|col| {
                    right_prefix += lane_delta[col][r];
                    right_prefix >= 0
                });
                best_key = Some(key);
                best_choice = Some(JointChoice {
                    op: Operation {
                        direction: Direction::Vertical,
                        r,
                        c: left,
                        h: height,
                        w: support_len,
                    },
                    delta,
                    long_position: r,
                    limit_start: support_start[r],
                    limit_end: support_end[r],
                    crosses_greedy_barrier: left_barrier || right_barrier,
                });
            }
        }
        let selected = best_choice.unwrap();
        debug_assert!(selected.delta <= thin_key.0);
        if selected.long_position != thin_key.1 {
            stats.joint_position_changed += 1;
        }
        stats.joint_potential_saved += (thin_key.0 - selected.delta) as i64;
        selected
    } else {
        return None;
    };

    let support_len = match choice.op.direction {
        Direction::Horizontal => choice.op.h,
        Direction::Vertical => choice.op.w,
    };
    if support_len > 1 {
        stats.joint_support_expansions += 1;
        stats.joint_support_lanes_added += (support_len - 1) as u64;
        let uses_full_legal_support = match choice.op.direction {
            Direction::Horizontal => {
                choice.op.r == choice.limit_start
                    && choice.op.r + choice.op.h - 1 == choice.limit_end
            }
            Direction::Vertical => {
                choice.op.c == choice.limit_start
                    && choice.op.c + choice.op.w - 1 == choice.limit_end
            }
        };
        if uses_full_legal_support {
            stats.full_support_selections += 1;
        }
        if support_len == N {
            stats.full_axis_support_selections += 1;
        }
    }
    if choice.crosses_greedy_barrier {
        stats.minsum_bridge_selections += 1;
    }
    if stats.collect_output_metrics {
        stats.output_joint_operations += 1;
        stats.opposing_pairs_selected += opposing_pair_count(state, choice.op) as u64;
    }

    #[cfg(feature = "local")]
    if stats.collect_output_metrics || stats.joint_mapping_calls & 4095 == 0 {
        assert_eq!(mapped_cell(choice.op, from), Some(to));
        assert!(is_active_legal_operation(input, state, choice.op));
        assert_eq!(
            operation_delta_potential(state, tree, choice.op),
            choice.delta
        );
    }
    debug_assert_eq!(mapped_cell(choice.op, from), Some(to));
    debug_assert!(is_active_legal_operation(input, state, choice.op));
    debug_assert_eq!(
        operation_delta_potential(state, tree, choice.op),
        choice.delta
    );
    Some(choice.op)
}

/// target の正解カードを active 開グラフの最短路に沿って運ぶ。
/// 各直線部分では最長の合法 shift を使い、隣接開辺の shift=1 は常に残る。
fn apply_transport(
    input: &Input,
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    target: usize,
    scratch: &mut Vec<Operation>,
    stats: &mut MechanismStats,
) -> usize {
    scratch.clear();
    let mut touched_pairs = 0;
    let mut current = state.position[target] as usize;
    assert!(state.is_active(current));
    let next_to_target = active_next_to_target(input, state, target);

    while current != target {
        let first = next_to_target[current] as usize;
        assert_ne!(first, current);
        let step_i = first as i32 / N as i32 - current as i32 / N as i32;
        let step_j = first as i32 % N as i32 - current as i32 % N as i32;
        let mut straight_nodes = [0_usize; MAX_SINGLE_AXIS_SHIFT + 1];
        straight_nodes[0] = current;
        straight_nodes[1] = first;
        let mut straight_len = 1;
        while straight_len < MAX_SINGLE_AXIS_SHIFT && straight_nodes[straight_len] != target {
            let at = straight_nodes[straight_len];
            let next = next_to_target[at] as usize;
            let next_step_i = next as i32 / N as i32 - at as i32 / N as i32;
            let next_step_j = next as i32 % N as i32 - at as i32 % N as i32;
            if next_step_i != step_i || next_step_j != step_j {
                break;
            }
            straight_len += 1;
            straight_nodes[straight_len] = next;
        }

        let mut selected = None;
        for shift in (1..=straight_len).rev() {
            let next_position = straight_nodes[shift];
            if let Some(op) =
                best_joint_mapping_operation(input, state, tree, current, next_position, stats)
            {
                selected = Some((next_position, op));
                break;
            }
        }
        let (next_position, op) = selected.expect("open edge must provide an adjacent operation");
        touched_pairs += apply_operation(state, tree, zobrist, op);
        scratch.push(op);
        current = state.position[target] as usize;
        assert_eq!(current, next_position);
    }
    touched_pairs
}

fn remove_target(
    input: &Input,
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    target: usize,
) -> u8 {
    assert!(state.active_count > 1);
    assert!(state.is_active(target));
    assert_eq!(state.board[target] as usize, target);

    let removed_degree = state.degree[target];
    let (neighbors, len) = open_neighbors(input, target);
    for &next in &neighbors[..len] {
        if state.is_active(next) {
            assert!(state.degree[next] > 0);
            state.degree[next] -= 1;
        }
    }
    state.degree_prefix += removed_degree as u16;
    state.freedom_prefix += tree.freedom[target] as u64;
    state.set_active(target, false);
    state.degree[target] = 0;
    state.active_count -= 1;
    state.hash_a ^= zobrist.active_a[target];
    state.hash_b ^= zobrist.active_b[target];
    removed_degree
}

fn restore_target(
    input: &Input,
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    target: usize,
    removed_degree: u8,
) {
    state.set_active(target, true);
    let (neighbors, len) = open_neighbors(input, target);
    let mut restored_degree = 0_u8;
    for &next in &neighbors[..len] {
        if state.is_active(next) && next != target {
            state.degree[next] += 1;
            restored_degree += 1;
        }
    }
    assert_eq!(restored_degree, removed_degree);
    state.degree[target] = restored_degree;
    state.active_count += 1;
    state.hash_a ^= zobrist.active_a[target];
    state.hash_b ^= zobrist.active_b[target];
    state.degree_prefix -= removed_degree as u16;
    state.freedom_prefix -= tree.freedom[target] as u64;
}

#[cfg(feature = "local")]
fn verify_state(input: &Input, state: &SearchState, tree: &TreeData, zobrist: &Zobrist) {
    let mut seen = [false; CELLS];
    let mut potential = 0_i32;
    let mut misplaced = 0_u16;
    let mut hash_a = 0_u64;
    let mut hash_b = 0_u64;
    let mut freedom_prefix = 0_u64;
    for cell in 0..CELLS {
        let card = state.board[cell] as usize;
        assert!(!seen[card]);
        seen[card] = true;
        assert_eq!(state.position[card] as usize, cell);
        potential += tree.distance[cell * CELLS + card] as i32;
        misplaced += (cell != card) as u16;
        let z = Zobrist::board_index(cell, card);
        hash_a ^= zobrist.board_a[z];
        hash_b ^= zobrist.board_b[z];
        if state.is_active(cell) {
            hash_a ^= zobrist.active_a[cell];
            hash_b ^= zobrist.active_b[cell];
        } else {
            assert_eq!(card, cell);
            freedom_prefix += tree.freedom[cell] as u64;
        }
        let row = cell / N;
        let col = cell % N;
        assert_eq!(
            ((state.active_rows[row] >> col) & 1) != 0,
            state.is_active(cell)
        );
        assert_eq!(
            ((state.active_cols[col] >> row) & 1) != 0,
            state.is_active(cell)
        );
    }
    assert_eq!(potential, state.potential);
    assert_eq!(misplaced, state.misplaced_count);
    assert_eq!(hash_a, state.hash_a);
    assert_eq!(hash_b, state.hash_b);
    assert_eq!(freedom_prefix, state.freedom_prefix);
    assert!(state.degree_prefix <= 4 * (CELLS as u16 - state.active_count));
    assert_eq!(
        state
            .active
            .iter()
            .map(|word| word.count_ones())
            .sum::<u32>(),
        state.active_count as u32
    );
    assert_eq!(
        state
            .active_rows
            .iter()
            .map(|mask| mask.count_ones())
            .sum::<u32>(),
        state.active_count as u32
    );
    assert_eq!(
        state
            .active_cols
            .iter()
            .map(|mask| mask.count_ones())
            .sum::<u32>(),
        state.active_count as u32
    );
    for cell in 0..CELLS {
        let expected_degree = if state.is_active(cell) {
            let (neighbors, len) = open_neighbors(input, cell);
            neighbors[..len]
                .iter()
                .filter(|&&next| state.is_active(next))
                .count() as u8
        } else {
            0
        };
        assert_eq!(state.degree[cell], expected_degree);
    }
    articulation_points(input, state);
}

#[derive(Default)]
struct MechanismStats {
    passes_started: u64,
    passes_completed: u64,
    passes_aborted: u64,
    max_completed_width: u64,
    beam_parents: u64,
    candidates: u64,
    children_kept: u64,
    duplicate_drops: u64,
    candidate_apply_ops: u64,
    candidate_undo_ops: u64,
    materialize_ops: u64,
    swap_pairs_touched: u64,
    rollback_checks: u64,
    invariant_checks: u64,
    best_updates: u64,
    topological_candidate_seen: u64,
    topological_candidate_eligible: u64,
    articulation_excluded: u64,
    output_articulation_excluded: u64,
    output_degree_one_two: u64,
    output_degree_sum: u64,
    topological_gate_violations: u64,
    joint_mapping_calls: u64,
    joint_positions_evaluated: u64,
    joint_position_changed: u64,
    joint_support_expansions: u64,
    joint_support_lanes_added: u64,
    joint_potential_saved: i64,
    minsum_bridge_selections: u64,
    // 最大合法連結帯を使い切った回数と、盤面全高/全幅に達した回数を分ける。
    full_support_selections: u64,
    full_axis_support_selections: u64,
    output_joint_operations: u64,
    opposing_pairs_selected: u64,
    // 対向需要の走査は最終出力replayだけに限定し、探索量を変えない。
    collect_output_metrics: bool,
}

impl MechanismStats {
    fn flush(&self, trace: &mut TraceStats) {
        trace.count_by("beam_passes_started", self.passes_started as i64);
        trace.count_by("beam_passes_completed", self.passes_completed as i64);
        trace.count_by("beam_passes_aborted", self.passes_aborted as i64);
        trace.count_by("max_completed_width", self.max_completed_width as i64);
        trace.count_by("beam_parents", self.beam_parents as i64);
        trace.count_by("beam_candidates", self.candidates as i64);
        trace.count_by("beam_children_kept", self.children_kept as i64);
        trace.count_by("duplicate_drops", self.duplicate_drops as i64);
        trace.count_by("candidate_apply_ops", self.candidate_apply_ops as i64);
        trace.count_by("candidate_undo_ops", self.candidate_undo_ops as i64);
        trace.count_by("materialize_ops", self.materialize_ops as i64);
        trace.count_by("swap_pairs_touched", self.swap_pairs_touched as i64);
        trace.count_by("rollback_checks", self.rollback_checks as i64);
        trace.count_by("rollback_check_fail", 0);
        trace.count_by("invariant_checks", self.invariant_checks as i64);
        trace.count_by("invariant_check_fail", 0);
        trace.count_by("best_updates", self.best_updates as i64);
        trace.count_by(
            "topological_candidate_seen",
            self.topological_candidate_seen as i64,
        );
        trace.count_by(
            "topological_candidate_eligible",
            self.topological_candidate_eligible as i64,
        );
        trace.count_by("articulation_excluded", self.articulation_excluded as i64);
        trace.count_by(
            "output_articulation_excluded",
            self.output_articulation_excluded as i64,
        );
        trace.count_by("output_degree_one_two", self.output_degree_one_two as i64);
        trace.count_by("output_degree_sum", self.output_degree_sum as i64);
        trace.count_by(
            "topological_gate_violations",
            self.topological_gate_violations as i64,
        );
        trace.count_by("joint_mapping_calls", self.joint_mapping_calls as i64);
        trace.count_by(
            "joint_positions_evaluated",
            self.joint_positions_evaluated as i64,
        );
        trace.count_by("joint_position_changed", self.joint_position_changed as i64);
        trace.count_by(
            "joint_support_expansions",
            self.joint_support_expansions as i64,
        );
        trace.count_by(
            "joint_support_lanes_added",
            self.joint_support_lanes_added as i64,
        );
        trace.count_by("joint_potential_saved", self.joint_potential_saved);
        trace.count_by(
            "minsum_bridge_selections",
            self.minsum_bridge_selections as i64,
        );
        trace.count_by(
            "full_support_selections",
            self.full_support_selections as i64,
        );
        trace.count_by(
            "full_axis_support_selections",
            self.full_axis_support_selections as i64,
        );
        trace.count_by(
            "output_joint_operations",
            self.output_joint_operations as i64,
        );
        trace.count_by(
            "opposing_pairs_selected",
            self.opposing_pairs_selected as i64,
        );
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    degree_prefix: u16,
    freedom_prefix: u64,
    base_score: u64,
    cost: u32,
    potential: i32,
    tie_key: u64,
    hash_a: u64,
    hash_b: u64,
    parent_index: u32,
    target: u16,
}

#[derive(Clone, Copy)]
struct HistoryNode {
    parent: u32,
    target: u16,
}

struct PassResult {
    order: Vec<u16>,
    cost: u32,
    degree_prefix: u16,
    freedom_prefix: u64,
    width: usize,
}

#[inline]
fn tie_hash(value: u64, seed: u64) -> u64 {
    let mut state = value ^ seed;
    splitmix64(&mut state)
}

#[allow(clippy::too_many_arguments)]
fn beam_pass(
    input: &Input,
    initial: &SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    width: usize,
    pass_seed: u64,
    abort_on_deadline: bool,
    search_deadline_sec: f64,
    time_keeper: &mut TimeKeeper,
    stats: &mut MechanismStats,
) -> Option<PassResult> {
    stats.passes_started += 1;
    let mut root = initial.clone();
    root.history_id = 0;
    let mut beam = vec![root];
    let mut history = vec![HistoryNode {
        parent: u32::MAX,
        target: NO_CELL,
    }];
    let mut scratch = Vec::with_capacity(CELLS);

    for _depth in 0..CELLS - 1 {
        if abort_on_deadline {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.passes_aborted += 1;
                return None;
            }
        }
        let mut candidates = Vec::with_capacity(beam.len().saturating_mul(8));
        for (parent_index, parent) in beam.iter_mut().enumerate() {
            stats.beam_parents += 1;
            let articulation = articulation_points(input, parent);
            let mut min_degree = u8::MAX;
            let mut non_articulation_count = 0_usize;
            for (target, &is_articulation) in articulation.iter().enumerate() {
                if !parent.is_active(target) {
                    continue;
                }
                if is_articulation {
                    stats.articulation_excluded += 1;
                } else {
                    non_articulation_count += 1;
                    min_degree = min_degree.min(parent.degree[target]);
                }
            }
            assert!(non_articulation_count > 0);
            stats.topological_candidate_seen += non_articulation_count as u64;

            for (target, &is_articulation) in articulation.iter().enumerate() {
                if !parent.is_active(target)
                    || is_articulation
                    || parent.degree[target] != min_degree
                {
                    continue;
                }
                stats.topological_candidate_eligible += 1;

                #[cfg(feature = "local")]
                let rollback_snapshot = if stats.candidates & 255 == 0 {
                    Some(parent.clone())
                } else {
                    None
                };

                let old_cost = parent.operations_used;
                let touched =
                    apply_transport(input, parent, tree, zobrist, target, &mut scratch, stats);
                let route_len = scratch.len() as u32;
                parent.operations_used += route_len;
                let removed_degree = remove_target(input, parent, tree, zobrist, target);
                let candidate = Candidate {
                    degree_prefix: parent.degree_prefix,
                    freedom_prefix: parent.freedom_prefix,
                    base_score: parent.score(),
                    cost: parent.operations_used,
                    potential: parent.potential,
                    tie_key: tie_hash(parent.hash_a ^ parent.hash_b, pass_seed),
                    hash_a: parent.hash_a,
                    hash_b: parent.hash_b,
                    parent_index: parent_index as u32,
                    target: target as u16,
                };

                restore_target(input, parent, tree, zobrist, target, removed_degree);
                parent.operations_used = old_cost;
                let mut undo_touched = 0;
                for &op in scratch.iter().rev() {
                    undo_touched += apply_operation(parent, tree, zobrist, op);
                }
                assert_eq!(touched, undo_touched);

                stats.candidates += 1;
                stats.candidate_apply_ops += route_len as u64;
                stats.candidate_undo_ops += route_len as u64;
                stats.swap_pairs_touched += (touched + undo_touched) as u64;

                #[cfg(feature = "local")]
                if let Some(snapshot) = rollback_snapshot {
                    assert!(*parent == snapshot);
                    verify_state(input, parent, tree, zobrist);
                    stats.rollback_checks += 1;
                }

                candidates.push(candidate);
                time_keeper.step();
                if abort_on_deadline && time_keeper.elapsed_sec() >= search_deadline_sec {
                    stats.passes_aborted += 1;
                    return None;
                }
            }
        }
        assert!(!candidates.is_empty());

        if abort_on_deadline {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.passes_aborted += 1;
                return None;
            }
        }

        let compare_candidates = |a: &Candidate, b: &Candidate| {
            let key_a = (
                a.degree_prefix,
                a.base_score,
                a.cost,
                a.potential,
                a.freedom_prefix,
                a.tie_key,
                a.hash_a,
                a.hash_b,
            );
            let key_b = (
                b.degree_prefix,
                b.base_score,
                b.cost,
                b.potential,
                b.freedom_prefix,
                b.tie_key,
                b.hash_a,
                b.hash_b,
            );
            key_a.cmp(&key_b)
        };

        // 全候補の sort を避け、重複を見込んだ上位集合だけを線形選択して整列する。
        let selection_cap = candidates.len().min(width.saturating_mul(4).max(64));
        if candidates.len() > selection_cap {
            candidates.select_nth_unstable_by(selection_cap, &compare_candidates);
            candidates.truncate(selection_cap);
        }
        if abort_on_deadline {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.passes_aborted += 1;
                return None;
            }
        }
        candidates.sort_unstable_by(&compare_candidates);

        if abort_on_deadline {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.passes_aborted += 1;
                return None;
            }
        }

        let keep = width.min(candidates.len());
        let mut selected = Vec::with_capacity(keep);
        let mut seen = HashSet::with_capacity(keep.saturating_mul(2));
        for candidate in candidates {
            if !seen.insert((candidate.hash_a, candidate.hash_b)) {
                stats.duplicate_drops += 1;
                continue;
            }
            selected.push(candidate);
            if selected.len() == keep {
                break;
            }
        }
        assert!(!selected.is_empty());

        let mut children = Vec::with_capacity(selected.len());
        for (selected_index, candidate) in selected.into_iter().enumerate() {
            let mut child = beam[candidate.parent_index as usize].clone();
            let touched = apply_transport(
                input,
                &mut child,
                tree,
                zobrist,
                candidate.target as usize,
                &mut scratch,
                stats,
            );
            child.operations_used += scratch.len() as u32;
            remove_target(input, &mut child, tree, zobrist, candidate.target as usize);
            assert_eq!(child.degree_prefix, candidate.degree_prefix);
            assert_eq!(child.freedom_prefix, candidate.freedom_prefix);
            assert_eq!(child.operations_used, candidate.cost);
            assert_eq!(child.potential, candidate.potential);
            assert_eq!(child.hash_a, candidate.hash_a);
            assert_eq!(child.hash_b, candidate.hash_b);

            history.push(HistoryNode {
                parent: child.history_id,
                target: candidate.target,
            });
            child.history_id = (history.len() - 1) as u32;
            stats.materialize_ops += scratch.len() as u64;
            stats.swap_pairs_touched += touched as u64;
            children.push(child);

            if abort_on_deadline && selected_index & 63 == 63 {
                time_keeper.force_update();
                if time_keeper.elapsed_sec() >= search_deadline_sec {
                    stats.passes_aborted += 1;
                    return None;
                }
            }
        }
        stats.children_kept += children.len() as u64;
        beam = children;

        #[cfg(feature = "local")]
        if _depth % 64 == 0 {
            verify_state(input, &beam[0], tree, zobrist);
            stats.invariant_checks += 1;
        }
    }

    let best_index = (0..beam.len())
        .min_by_key(|&index| {
            (
                beam[index].degree_prefix,
                beam[index].operations_used,
                beam[index].freedom_prefix,
                beam[index].hash_a,
                beam[index].hash_b,
            )
        })
        .unwrap();
    let best = &beam[best_index];
    assert_eq!(best.active_count, 1);
    assert_eq!(best.misplaced_count, 0);
    assert!(best
        .board
        .iter()
        .enumerate()
        .all(|(cell, &card)| cell == card as usize));

    let mut order = Vec::with_capacity(CELLS - 1);
    let mut history_id = best.history_id;
    while history_id != 0 {
        let node = history[history_id as usize];
        order.push(node.target);
        history_id = node.parent;
    }
    order.reverse();
    assert_eq!(order.len(), CELLS - 1);

    stats.passes_completed += 1;
    stats.max_completed_width = stats.max_completed_width.max(width as u64);
    Some(PassResult {
        order,
        cost: best.operations_used,
        degree_prefix: best.degree_prefix,
        freedom_prefix: best.freedom_prefix,
        width,
    })
}

fn replay_order(
    input: &Input,
    initial: &SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    order: &[u16],
    stats: &mut MechanismStats,
) -> (SearchState, Vec<Operation>) {
    let mut state = initial.clone();
    state.history_id = 0;
    let mut operations = Vec::new();
    let mut scratch = Vec::with_capacity(CELLS);
    for &target_u16 in order {
        let target = target_u16 as usize;
        let articulation = articulation_points(input, &state);
        let mut min_degree = u8::MAX;
        let mut excluded = 0_u64;
        for (cell, &is_articulation) in articulation.iter().enumerate() {
            if !state.is_active(cell) {
                continue;
            }
            if is_articulation {
                excluded += 1;
            } else {
                min_degree = min_degree.min(state.degree[cell]);
            }
        }
        let gate_violation = articulation[target] || state.degree[target] != min_degree;
        stats.output_articulation_excluded += excluded;
        stats.output_degree_one_two += u64::from(state.degree[target] <= 2);
        stats.output_degree_sum += state.degree[target] as u64;
        stats.topological_gate_violations += u64::from(gate_violation);
        assert!(!gate_violation);
        apply_transport(
            input,
            &mut state,
            tree,
            zobrist,
            target,
            &mut scratch,
            stats,
        );
        state.operations_used += scratch.len() as u32;
        operations.extend_from_slice(&scratch);
        remove_target(input, &mut state, tree, zobrist, target);
    }
    (state, operations)
}

fn record_freedom_profile(order: &[u16], tree: &TreeData, trace: &mut TraceStats) {
    const PROFILE_SIZE: usize = 100;
    assert!(order.len() >= PROFILE_SIZE);
    let early = &order[..PROFILE_SIZE];
    let late = &order[order.len() - PROFILE_SIZE..];
    let early_freedom_sum: u64 = early
        .iter()
        .map(|&cell| tree.freedom[cell as usize] as u64)
        .sum();
    let late_freedom_sum: u64 = late
        .iter()
        .map(|&cell| tree.freedom[cell as usize] as u64)
        .sum();
    trace.count_by("early100_freedom_sum", early_freedom_sum as i64);
    trace.count_by("late100_freedom_sum", late_freedom_sum as i64);
}

fn main() {
    // V000 と同様、入力読み込みより前を基準時刻にする。
    let mut time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();
    #[cfg(not(feature = "local"))]
    let mut trace = TraceStats;
    let input = Input::read();
    let tree = local_time!(trace, "tree_build", { build_spanning_tree(&input) });
    let zobrist = local_time!(trace, "zobrist_build", { Zobrist::new() });
    let initial = SearchState::new(&input, &tree, &zobrist);
    #[cfg(feature = "local")]
    verify_state(&input, &initial, &tree, &zobrist);

    let search_started = Instant::now();
    let search_deadline_sec = PROGRAM_TIME_LIMIT_SEC * SEARCH_TIME_RATIO;
    let mut stats = MechanismStats::default();

    // 幅1と幅2も同じビーム機構で最後まで回し、常に完成解を保持する。
    let greedy = beam_pass(
        &input,
        &initial,
        &tree,
        &zobrist,
        1,
        1,
        false,
        search_deadline_sec,
        &mut time_keeper,
        &mut stats,
    )
    .unwrap();
    let greedy_cost = greedy.cost;
    let greedy_degree_prefix = greedy.degree_prefix;
    let mut best = greedy;

    let width_two = beam_pass(
        &input,
        &initial,
        &tree,
        &zobrist,
        2,
        2,
        true,
        search_deadline_sec,
        &mut time_keeper,
        &mut stats,
    );
    match width_two {
        Some(width_two)
            if (
                width_two.degree_prefix,
                width_two.cost,
                width_two.freedom_prefix,
            ) < (best.degree_prefix, best.cost, best.freedom_prefix) =>
        {
            best = width_two;
            stats.best_updates += 1;
        }
        _ => {}
    }

    let mut width = 8_usize;
    let mut pass_seed = 3_u64;
    loop {
        time_keeper.force_update();
        if time_keeper.elapsed_sec() >= search_deadline_sec {
            break;
        }
        let result = beam_pass(
            &input,
            &initial,
            &tree,
            &zobrist,
            width,
            pass_seed,
            true,
            search_deadline_sec,
            &mut time_keeper,
            &mut stats,
        );
        let Some(result) = result else {
            break;
        };
        if (result.degree_prefix, result.cost, result.freedom_prefix)
            < (best.degree_prefix, best.cost, best.freedom_prefix)
        {
            best = result;
            stats.best_updates += 1;
        }
        width = (width * 4).min(MAX_BEAM_WIDTH);
        pass_seed += 1;
    }
    time_keeper.force_update();
    trace.add_time_ms(
        "beam_search",
        search_started.elapsed().as_secs_f64() * 1000.0,
    );

    let best_width = best.width;
    let best_cost = best.cost;
    let best_degree_prefix = best.degree_prefix;
    let best_freedom_prefix = best.freedom_prefix;
    let best_order = best.order;
    record_freedom_profile(&best_order, &tree, &mut trace);
    stats.collect_output_metrics = true;
    let (final_state, operations) = local_time!(trace, "final_replay", {
        replay_order(&input, &initial, &tree, &zobrist, &best_order, &mut stats)
    });
    stats.collect_output_metrics = false;

    assert_eq!(operations.len(), final_state.operations_used as usize);
    assert_eq!(operations.len(), best_cost as usize);
    assert_eq!(final_state.degree_prefix, best_degree_prefix);
    assert_eq!(final_state.freedom_prefix, best_freedom_prefix);
    assert_eq!(stats.output_joint_operations, operations.len() as u64);
    assert_eq!(stats.topological_gate_violations, 0);
    assert!(operations.len() <= MAX_OPERATIONS);
    assert_eq!(final_state.active_count, 1);
    assert_eq!(final_state.misplaced_count, 0);
    assert!(final_state
        .board
        .iter()
        .enumerate()
        .all(|(cell, &card)| cell == card as usize));
    assert!(operations.iter().all(|&op| is_legal_operation(&input, op)));
    #[cfg(feature = "local")]
    verify_state(&input, &final_state, &tree, &zobrist);

    let adjacent_ops = operations
        .iter()
        .filter(|op| match op.direction {
            Direction::Vertical => op.h / 2 == 1,
            Direction::Horizontal => op.w / 2 == 1,
        })
        .count();
    let jump_ops = operations.len() - adjacent_ops;

    stats.flush(&mut trace);
    trace.count_by("greedy_t", greedy_cost as i64);
    trace.count_by("greedy_degree_prefix", greedy_degree_prefix as i64);
    trace.count_by("final_t", operations.len() as i64);
    trace.count_by("final_degree_prefix", final_state.degree_prefix as i64);
    trace.count_by("best_width", best_width as i64);
    trace.count_by("adjacent_ops", adjacent_ops as i64);
    trace.count_by("jump_ops", jump_ops as i64);
    trace.count_by("final_e", final_state.misplaced_count as i64);
    trace.count_by("final_replay_complete", 1);
    trace.count_by(
        "search_elapsed_us",
        (time_keeper.exact_elapsed_sec() * 1_000_000.0) as i64,
    );
    trace.summary();

    write_output(&operations);
}
