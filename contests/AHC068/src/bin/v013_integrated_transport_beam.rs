// v013_integrated_transport_beam.rs
// v011 の固定木葉ビームに、active 開グラフ最短路、exact 支持矩形、C18 二手 macro、
// 疎な実完成 rollout を統合する。候補の搬送方針は常に決定的で、時刻は pass と
// rollout の中断だけに使う。
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
const PROVEN_MAX_OPERATIONS: usize = CELLS * (CELLS - 1) / 2;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 最終 replay と出力の余白を残しつつ、探索には制限時間の大半を使う。
const SEARCH_TIME_RATIO: f64 = 0.95;
/// 実測の消化率に合わせた proxy の手数重み。
const EVAL_COST_WEIGHT: u64 = 4;
const MAX_BEAM_WIDTH: usize = 8192;
const MAX_COMPLETED_ROLLOUTS: u64 = 10;
const ROLLOUT_DEPTHS: [usize; 5] = [0, 50, 100, 200, 300];
/// rollout 専用 pass をここで打ち切り、通常 beam と終了処理へ時間を残す。
const ROLLOUT_PASS_TIME_RATIO: f64 = 0.45;
const NO_CELL: u16 = u16::MAX;

struct Input {
    initial_board: [usize; CELLS],
    vertical_walls: [[bool; N - 1]; N],
    horizontal_walls: [[bool; N]; N - 1],
    vertical_wall_rows: [u32; N],
    vertical_wall_cols: [u32; N - 1],
    horizontal_wall_rows: [u32; N - 1],
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
        let vertical_wall_cols = std::array::from_fn(|j| {
            (0..N).fold(0_u32, |mask, i| mask | ((vertical_walls[i][j] as u32) << i))
        });
        let horizontal_wall_rows = std::array::from_fn(|i| {
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
            vertical_wall_cols,
            horizontal_wall_rows,
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

/// 探索の各遷移は、この木の active な葉へ正解カードを運んでから葉を除く。
/// active 部分は常に連結で、shift=1 の操作が必ず候補に残るため、どのビーム枝も
/// 最後まで進める。各段階の最大搬送長の総和は 399+...+1=79,800 である。
struct TreeData {
    neighbors: [[u16; 4]; CELLS],
    neighbor_count: [u8; CELLS],
    open_neighbors: [[u16; 4]; CELLS],
    open_neighbor_count: [u8; CELLS],
    initial_degree: [u8; CELLS],
    distance: Vec<u16>,
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

fn build_spanning_tree(input: &Input) -> TreeData {
    let root = (N / 2) * N + N / 2;
    let mut fixed_open_neighbors = [[NO_CELL; 4]; CELLS];
    let mut open_neighbor_count = [0_u8; CELLS];
    for cell in 0..CELLS {
        let (neighbors, len) = open_neighbors(input, cell);
        open_neighbor_count[cell] = len as u8;
        for (k, &to) in neighbors[..len].iter().enumerate() {
            fixed_open_neighbors[cell][k] = to as u16;
        }
    }
    let mut parent = [NO_CELL; CELLS];
    parent[root] = root as u16;
    let mut queue = VecDeque::with_capacity(CELLS);
    queue.push_back(root);

    while let Some(cell) = queue.pop_front() {
        for k in 0..open_neighbor_count[cell] as usize {
            let next = fixed_open_neighbors[cell][k] as usize;
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

    TreeData {
        neighbors: tree_neighbors,
        neighbor_count,
        open_neighbors: fixed_open_neighbors,
        open_neighbor_count,
        initial_degree: neighbor_count,
        distance,
    }
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
    active_rows: [u32; N],
    active_cols: [u32; N],
    degree: [u8; CELLS],
    active_count: u16,
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
        let active_rows = [(1_u32 << N) - 1; N];
        let active_cols = [(1_u32 << N) - 1; N];
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

        Self {
            board,
            position,
            active,
            active_rows,
            active_cols,
            degree: tree.initial_degree,
            active_count: CELLS as u16,
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
        let i = cell / N;
        let j = cell % N;
        if value {
            self.active[cell >> 6] |= bit;
            self.active_rows[i] |= 1_u32 << j;
            self.active_cols[j] |= 1_u32 << i;
        } else {
            self.active[cell >> 6] &= !bit;
            self.active_rows[i] &= !(1_u32 << j);
            self.active_cols[j] &= !(1_u32 << i);
        }
    }

    #[inline]
    fn score(&self) -> u64 {
        self.operations_used as u64 * EVAL_COST_WEIGHT + self.potential as u64
    }
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
fn bit_range(start: usize, len: usize) -> u32 {
    debug_assert!(len > 0 && start + len <= N);
    ((1_u32 << len) - 1) << start
}

#[derive(Clone, Copy)]
struct SupportChoice {
    op: Operation,
    delta: i32,
    thin_delta: i32,
    placement: usize,
    thin_placement: usize,
    minsum_bridge: bool,
}

#[inline]
fn support_key(choice: &SupportChoice) -> (i32, usize, usize, usize, usize, usize, usize, u8) {
    (
        choice.delta,
        choice.op.h * choice.op.w,
        choice.placement,
        choice.op.r,
        choice.op.c,
        choice.op.h,
        choice.op.w,
        match choice.op.direction {
            Direction::Vertical => 0,
            Direction::Horizontal => 1,
        },
    )
}

/// `from -> to` を実現する全長手位置と、anchor を含む全合法連続支持区間を同時に比較する。
/// 各線の pair delta は移動軸 prefix で O(1) に取り出し、直交方向は両端まで走査する。
fn exact_support_operation(
    input: &Input,
    state: &SearchState,
    tree: &TreeData,
    from: usize,
    to: usize,
) -> Option<SupportChoice> {
    let from_i = from / N;
    let from_j = from % N;
    let to_i = to / N;
    let to_j = to % N;
    let mut best: Option<SupportChoice> = None;
    let mut best_thin: Option<(i32, usize)> = None;

    if from_i == to_i {
        let shift = from_j.abs_diff(to_j);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let width = 2 * shift;
        let mut pair_prefix = [[0_i32; N + 1]; N];
        for i in 0..N {
            for j in 0..N - shift {
                let a = i * N + j;
                let b = a + shift;
                let card_a = state.board[a] as usize;
                let card_b = state.board[b] as usize;
                let pair_delta = tree.distance[b * CELLS + card_a] as i32
                    + tree.distance[a * CELLS + card_b] as i32
                    - tree.distance[a * CELLS + card_a] as i32
                    - tree.distance[b * CELLS + card_b] as i32;
                pair_prefix[i][j + 1] = pair_prefix[i][j] + pair_delta;
            }
        }
        for c in 0..=N - width {
            let thin = Operation {
                direction: Direction::Horizontal,
                r: from_i,
                c,
                h: 1,
                w: width,
            };
            if mapped_cell(thin, from) != Some(to) {
                continue;
            }
            let cell_mask = bit_range(c, width);
            let inner_mask = bit_range(c, width - 1);
            let row_ok = |i: usize| {
                state.active_rows[i] & cell_mask == cell_mask
                    && input.vertical_wall_rows[i] & inner_mask == 0
            };
            let boundary_ok = |i: usize| input.horizontal_wall_rows[i] & cell_mask == 0;
            if !row_ok(from_i) {
                continue;
            }
            let mut top_limit = from_i;
            while top_limit > 0 && row_ok(top_limit - 1) && boundary_ok(top_limit - 1) {
                top_limit -= 1;
            }
            let mut bottom_limit = from_i;
            while bottom_limit + 1 < N && row_ok(bottom_limit + 1) && boundary_ok(bottom_limit) {
                bottom_limit += 1;
            }
            let line_delta = |i: usize| pair_prefix[i][c + shift] - pair_prefix[i][c];
            let thin_delta = line_delta(from_i);
            if best_thin.map_or(true, |key| (thin_delta, c) < key) {
                best_thin = Some((thin_delta, c));
            }

            let mut best_top = from_i;
            let mut best_above = 0_i32;
            let mut running = 0_i32;
            let mut blocked_above = false;
            let mut bridge_above = false;
            for top in (top_limit..from_i).rev() {
                running += line_delta(top);
                blocked_above |= running >= 0;
                if (running, from_i - top, top) < (best_above, from_i - best_top, best_top) {
                    best_above = running;
                    best_top = top;
                    bridge_above = blocked_above && top < from_i;
                }
            }
            let mut best_bottom = from_i;
            let mut best_below = 0_i32;
            running = 0;
            let mut blocked_below = false;
            let mut bridge_below = false;
            for bottom in from_i + 1..=bottom_limit {
                running += line_delta(bottom);
                blocked_below |= running >= 0;
                if (running, bottom - from_i, bottom)
                    < (best_below, best_bottom - from_i, best_bottom)
                {
                    best_below = running;
                    best_bottom = bottom;
                    bridge_below = blocked_below && bottom > from_i;
                }
            }
            let choice = SupportChoice {
                op: Operation {
                    direction: Direction::Horizontal,
                    r: best_top,
                    c,
                    h: best_bottom - best_top + 1,
                    w: width,
                },
                delta: thin_delta + best_above + best_below,
                thin_delta: 0,
                placement: c,
                thin_placement: 0,
                minsum_bridge: bridge_above || bridge_below,
            };
            debug_assert!(is_active_legal_operation(input, state, choice.op));
            debug_assert_eq!(
                choice.delta,
                operation_delta_potential(state, tree, choice.op)
            );
            if best
                .as_ref()
                .map_or(true, |old| support_key(&choice) < support_key(old))
            {
                best = Some(choice);
            }
        }
    } else if from_j == to_j {
        let shift = from_i.abs_diff(to_i);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let height = 2 * shift;
        let mut pair_prefix = [[0_i32; N + 1]; N];
        for j in 0..N {
            for i in 0..N - shift {
                let a = i * N + j;
                let b = a + shift * N;
                let card_a = state.board[a] as usize;
                let card_b = state.board[b] as usize;
                let pair_delta = tree.distance[b * CELLS + card_a] as i32
                    + tree.distance[a * CELLS + card_b] as i32
                    - tree.distance[a * CELLS + card_a] as i32
                    - tree.distance[b * CELLS + card_b] as i32;
                pair_prefix[j][i + 1] = pair_prefix[j][i] + pair_delta;
            }
        }
        for r in 0..=N - height {
            let thin = Operation {
                direction: Direction::Vertical,
                r,
                c: from_j,
                h: height,
                w: 1,
            };
            if mapped_cell(thin, from) != Some(to) {
                continue;
            }
            let cell_mask = bit_range(r, height);
            let inner_mask = bit_range(r, height - 1);
            let col_ok = |j: usize| {
                state.active_cols[j] & cell_mask == cell_mask
                    && input.horizontal_wall_cols[j] & inner_mask == 0
            };
            let boundary_ok = |j: usize| input.vertical_wall_cols[j] & cell_mask == 0;
            if !col_ok(from_j) {
                continue;
            }
            let mut left_limit = from_j;
            while left_limit > 0 && col_ok(left_limit - 1) && boundary_ok(left_limit - 1) {
                left_limit -= 1;
            }
            let mut right_limit = from_j;
            while right_limit + 1 < N && col_ok(right_limit + 1) && boundary_ok(right_limit) {
                right_limit += 1;
            }
            let line_delta = |j: usize| pair_prefix[j][r + shift] - pair_prefix[j][r];
            let thin_delta = line_delta(from_j);
            if best_thin.map_or(true, |key| (thin_delta, r) < key) {
                best_thin = Some((thin_delta, r));
            }

            let mut best_left = from_j;
            let mut best_before = 0_i32;
            let mut running = 0_i32;
            let mut blocked_left = false;
            let mut bridge_left = false;
            for left in (left_limit..from_j).rev() {
                running += line_delta(left);
                blocked_left |= running >= 0;
                if (running, from_j - left, left) < (best_before, from_j - best_left, best_left) {
                    best_before = running;
                    best_left = left;
                    bridge_left = blocked_left && left < from_j;
                }
            }
            let mut best_right = from_j;
            let mut best_after = 0_i32;
            running = 0;
            let mut blocked_right = false;
            let mut bridge_right = false;
            for right in from_j + 1..=right_limit {
                running += line_delta(right);
                blocked_right |= running >= 0;
                if (running, right - from_j, right) < (best_after, best_right - from_j, best_right)
                {
                    best_after = running;
                    best_right = right;
                    bridge_right = blocked_right && right > from_j;
                }
            }
            let choice = SupportChoice {
                op: Operation {
                    direction: Direction::Vertical,
                    r,
                    c: best_left,
                    h: height,
                    w: best_right - best_left + 1,
                },
                delta: thin_delta + best_before + best_after,
                thin_delta: 0,
                placement: r,
                thin_placement: 0,
                minsum_bridge: bridge_left || bridge_right,
            };
            debug_assert!(is_active_legal_operation(input, state, choice.op));
            debug_assert_eq!(
                choice.delta,
                operation_delta_potential(state, tree, choice.op)
            );
            if best
                .as_ref()
                .map_or(true, |old| support_key(&choice) < support_key(old))
            {
                best = Some(choice);
            }
        }
    }

    let (thin_delta, thin_placement) = best_thin?;
    best.map(|mut choice| {
        choice.thin_delta = thin_delta;
        choice.thin_placement = thin_placement;
        choice
    })
}

fn active_bfs_path(
    state: &SearchState,
    tree: &TreeData,
    start: usize,
    target: usize,
) -> ([u16; CELLS], usize) {
    assert!(state.is_active(start) && state.is_active(target));
    let mut parent = [NO_CELL; CELLS];
    let mut queue = [NO_CELL; CELLS];
    let mut head = 0;
    let mut tail = 0;
    parent[start] = start as u16;
    queue[tail] = start as u16;
    tail += 1;
    while head < tail && parent[target] == NO_CELL {
        let cell = queue[head] as usize;
        head += 1;
        for k in 0..tree.open_neighbor_count[cell] as usize {
            let next = tree.open_neighbors[cell][k] as usize;
            if state.is_active(next) && parent[next] == NO_CELL {
                parent[next] = cell as u16;
                queue[tail] = next as u16;
                tail += 1;
            }
        }
    }
    assert_ne!(parent[target], NO_CELL);
    let mut reverse = [NO_CELL; CELLS];
    let mut len = 0;
    let mut cell = target;
    loop {
        reverse[len] = cell as u16;
        len += 1;
        if cell == start {
            break;
        }
        cell = parent[cell] as usize;
    }
    let mut path = [NO_CELL; CELLS];
    for k in 0..len {
        path[k] = reverse[len - 1 - k];
    }
    (path, len)
}

#[derive(Default)]
struct TransportScratch {
    operations: Vec<Operation>,
    /// 0=標準、1=atomic pair の先頭、2=atomic pair の末尾。
    kinds: Vec<u8>,
}

impl TransportScratch {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            operations: Vec::with_capacity(capacity),
            kinds: Vec::with_capacity(capacity),
        }
    }

    fn clear(&mut self) {
        self.operations.clear();
        self.kinds.clear();
    }

    fn len(&self) -> usize {
        self.operations.len()
    }

    fn push_standard(&mut self, op: Operation) {
        self.operations.push(op);
        self.kinds.push(0);
    }

    fn push_atomic(&mut self, first: Operation, second: Operation) {
        self.operations.push(first);
        self.kinds.push(1);
        self.operations.push(second);
        self.kinds.push(2);
    }
}

#[cfg(feature = "local")]
fn verify_inactive_correct(state: &SearchState) {
    for cell in 0..CELLS {
        if !state.is_active(cell) {
            assert_eq!(state.board[cell] as usize, cell);
        }
    }
}

fn compressed_segment_ops(path: &[u16; CELLS], start: usize, end: usize) -> (usize, usize) {
    let mut edge = start;
    let mut runs = 0;
    let mut operations = 0;
    while edge < end {
        let a = path[edge] as usize;
        let b = path[edge + 1] as usize;
        let di = b as i32 / N as i32 - a as i32 / N as i32;
        let dj = b as i32 % N as i32 - a as i32 % N as i32;
        let mut run_len = 1;
        edge += 1;
        while edge < end {
            let x = path[edge] as usize;
            let y = path[edge + 1] as usize;
            let next_di = y as i32 / N as i32 - x as i32 / N as i32;
            let next_dj = y as i32 % N as i32 - x as i32 % N as i32;
            if (next_di, next_dj) != (di, dj) {
                break;
            }
            run_len += 1;
            edge += 1;
        }
        runs += 1;
        operations += (run_len + MAX_SINGLE_AXIS_SHIFT - 1) / MAX_SINGLE_AXIS_SHIFT;
    }
    (runs, operations)
}

#[derive(Clone, Copy)]
struct OverlapChoice {
    end_index: usize,
    first: Operation,
    second: Operation,
    saved: usize,
}

fn best_overlap_choice(
    input: &Input,
    state: &SearchState,
    path: &[u16; CELLS],
    path_len: usize,
    start_index: usize,
    stats: &mut MechanismStats,
) -> Option<OverlapChoice> {
    let start = path[start_index] as usize;
    let si = start / N;
    let sj = start % N;
    let mut best: Option<OverlapChoice> = None;
    for end_index in start_index + 1..path_len {
        let end = path[end_index] as usize;
        let ei = end / N;
        let ej = end % N;
        if si != ei && sj != ej {
            continue;
        }
        let distance = si.abs_diff(ei) + sj.abs_diff(ej);
        if distance < 2 || distance % 2 != 0 || distance / 2 > 9 {
            continue;
        }
        let d = distance / 2;
        let mi = (si + ei) / 2;
        let mj = (sj + ej) / 2;
        let midpoint = mi * N + mj;
        if !state.is_active(end) || !state.is_active(midpoint) {
            continue;
        }
        let (a, b) = if sj == ej {
            let r = si.min(ei);
            (
                Operation {
                    direction: Direction::Vertical,
                    r,
                    c: sj,
                    h: 2 * d,
                    w: 1,
                },
                Operation {
                    direction: Direction::Vertical,
                    r: r + 1,
                    c: sj,
                    h: 2 * d,
                    w: 1,
                },
            )
        } else {
            let c = sj.min(ej);
            (
                Operation {
                    direction: Direction::Horizontal,
                    r: si,
                    c,
                    h: 1,
                    w: 2 * d,
                },
                Operation {
                    direction: Direction::Horizontal,
                    r: si,
                    c: c + 1,
                    h: 1,
                    w: 2 * d,
                },
            )
        };
        if !is_legal_operation(input, a) || !is_legal_operation(input, b) {
            continue;
        }
        let (runs, route_ops) = compressed_segment_ops(path, start_index, end_index);
        if runs < 3 {
            continue;
        }
        debug_assert!(route_ops >= 3);
        stats.overlap_candidates += 1;
        let (first, second) = if ei > si || ej > sj { (a, b) } else { (b, a) };
        let choice = OverlapChoice {
            end_index,
            first,
            second,
            saved: route_ops - 2,
        };
        if best.map_or(true, |old| {
            (choice.saved, choice.end_index) > (old.saved, old.end_index)
        }) {
            best = Some(choice);
        }
    }
    best
}

/// active 開グラフの単純最短路を一度固定し、標準直線圧縮または C18 atomic pair で辿る。
fn apply_transport(
    input: &Input,
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    target: usize,
    scratch: &mut TransportScratch,
    stats: &mut MechanismStats,
) -> usize {
    scratch.clear();
    let start = state.position[target] as usize;
    let (path, path_len) = active_bfs_path(state, tree, start, target);
    stats.active_route_calls += 1;
    let bfs_distance = path_len - 1;
    let tree_distance = tree.distance[start * CELLS + target] as usize;
    stats.tree_edges_saved += tree_distance.saturating_sub(bfs_distance) as u64;

    let mut touched_pairs = 0;
    let mut path_index = 0;
    while path_index + 1 < path_len {
        if let Some(overlap) = best_overlap_choice(input, state, &path, path_len, path_index, stats)
        {
            touched_pairs += apply_operation(state, tree, zobrist, overlap.first);
            touched_pairs += apply_operation(state, tree, zobrist, overlap.second);
            scratch.push_atomic(overlap.first, overlap.second);
            path_index = overlap.end_index;
            assert_eq!(state.position[target] as usize, path[path_index] as usize);
            stats.overlap_selected += 1;
            stats.overlap_path_ops_saved += overlap.saved as u64;
            #[cfg(feature = "local")]
            {
                verify_inactive_correct(state);
                stats.overlap_restored_checks += 1;
            }
            continue;
        }

        let current = path[path_index] as usize;
        let first = path[path_index + 1] as usize;
        let step_i = first as i32 / N as i32 - current as i32 / N as i32;
        let step_j = first as i32 % N as i32 - current as i32 % N as i32;
        let mut straight_len = 1;
        while straight_len < MAX_SINGLE_AXIS_SHIFT && path_index + straight_len + 1 < path_len {
            let a = path[path_index + straight_len] as usize;
            let b = path[path_index + straight_len + 1] as usize;
            let next_i = b as i32 / N as i32 - a as i32 / N as i32;
            let next_j = b as i32 % N as i32 - a as i32 % N as i32;
            if (next_i, next_j) != (step_i, step_j) {
                break;
            }
            straight_len += 1;
        }

        let mut selected = None;
        for shift in (1..=straight_len).rev() {
            let next_position = path[path_index + shift] as usize;
            if let Some(choice) =
                exact_support_operation(input, state, tree, current, next_position)
            {
                selected = Some((shift, next_position, choice));
                break;
            }
        }
        let (shift, next_position, choice) =
            selected.expect("an active open edge must provide a legal shift-1 operation");
        stats.exact_support_selected += 1;
        stats.exact_gain_vs_thin += (choice.thin_delta - choice.delta) as i64;
        stats.joint_position_changed += (choice.placement != choice.thin_placement) as u64;
        stats.minsum_bridge += choice.minsum_bridge as u64;
        stats.full_axis_support += match choice.op.direction {
            Direction::Horizontal => (choice.op.h == N) as u64,
            Direction::Vertical => (choice.op.w == N) as u64,
        };
        touched_pairs += apply_operation(state, tree, zobrist, choice.op);
        scratch.push_standard(choice.op);
        path_index += shift;
        assert_eq!(state.position[target] as usize, next_position);
    }
    assert_eq!(state.position[target] as usize, target);
    assert!(scratch.len() <= bfs_distance);
    touched_pairs
}

fn undo_transport(
    state: &mut SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    scratch: &TransportScratch,
    stats: &mut MechanismStats,
) -> usize {
    #[cfg(not(feature = "local"))]
    let _ = stats;
    let mut touched = 0;
    for index in (0..scratch.operations.len()).rev() {
        touched += apply_operation(state, tree, zobrist, scratch.operations[index]);
        #[cfg(feature = "local")]
        if scratch.kinds[index] == 1 {
            verify_inactive_correct(state);
            stats.overlap_restored_checks += 1;
        }
    }
    touched
}

fn remove_leaf(state: &mut SearchState, tree: &TreeData, zobrist: &Zobrist, leaf: usize) -> usize {
    assert!(state.active_count > 1);
    assert!(state.is_active(leaf));
    assert_eq!(state.degree[leaf], 1);
    assert_eq!(state.board[leaf] as usize, leaf);

    let mut neighbor = CELLS;
    for k in 0..tree.neighbor_count[leaf] as usize {
        let to = tree.neighbors[leaf][k] as usize;
        if state.is_active(to) {
            neighbor = to;
            break;
        }
    }
    assert!(neighbor < CELLS);
    state.set_active(leaf, false);
    state.degree[leaf] = 0;
    state.degree[neighbor] -= 1;
    state.active_count -= 1;
    state.hash_a ^= zobrist.active_a[leaf];
    state.hash_b ^= zobrist.active_b[leaf];
    neighbor
}

fn restore_leaf(state: &mut SearchState, zobrist: &Zobrist, leaf: usize, neighbor: usize) {
    state.set_active(leaf, true);
    state.degree[leaf] = 1;
    state.degree[neighbor] += 1;
    state.active_count += 1;
    state.hash_a ^= zobrist.active_a[leaf];
    state.hash_b ^= zobrist.active_b[leaf];
}

#[cfg(feature = "local")]
fn verify_state(state: &SearchState, tree: &TreeData, zobrist: &Zobrist) {
    let mut seen = [false; CELLS];
    let mut expected_rows = [0_u32; N];
    let mut expected_cols = [0_u32; N];
    let mut potential = 0_i32;
    let mut misplaced = 0_u16;
    let mut hash_a = 0_u64;
    let mut hash_b = 0_u64;
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
            expected_rows[cell / N] |= 1_u32 << (cell % N);
            expected_cols[cell % N] |= 1_u32 << (cell / N);
            hash_a ^= zobrist.active_a[cell];
            hash_b ^= zobrist.active_b[cell];
        } else {
            assert_eq!(card, cell);
        }
    }
    assert_eq!(potential, state.potential);
    assert_eq!(misplaced, state.misplaced_count);
    assert_eq!(hash_a, state.hash_a);
    assert_eq!(hash_b, state.hash_b);
    assert_eq!(expected_rows, state.active_rows);
    assert_eq!(expected_cols, state.active_cols);
    assert_eq!(
        state
            .active
            .iter()
            .map(|word| word.count_ones())
            .sum::<u32>(),
        state.active_count as u32
    );
    for cell in 0..CELLS {
        let expected_degree = if state.is_active(cell) {
            (0..tree.neighbor_count[cell] as usize)
                .filter(|&k| state.is_active(tree.neighbors[cell][k] as usize))
                .count() as u8
        } else {
            0
        };
        assert_eq!(state.degree[cell], expected_degree);
    }

    let start = (0..CELLS).find(|&cell| state.is_active(cell)).unwrap();
    let mut reached = [false; CELLS];
    let mut queue = [NO_CELL; CELLS];
    let mut head = 0;
    let mut tail = 1;
    queue[0] = start as u16;
    reached[start] = true;
    while head < tail {
        let cell = queue[head] as usize;
        head += 1;
        for k in 0..tree.open_neighbor_count[cell] as usize {
            let next = tree.open_neighbors[cell][k] as usize;
            if state.is_active(next) && !reached[next] {
                reached[next] = true;
                queue[tail] = next as u16;
                tail += 1;
            }
        }
    }
    assert_eq!(tail, state.active_count as usize);
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
    active_route_calls: u64,
    tree_edges_saved: u64,
    exact_support_selected: u64,
    exact_gain_vs_thin: i64,
    joint_position_changed: u64,
    minsum_bridge: u64,
    full_axis_support: u64,
    overlap_candidates: u64,
    overlap_selected: u64,
    overlap_path_ops_saved: u64,
    overlap_restored_checks: u64,
    rollout_runs: u64,
    rollout_completed: u64,
    rollout_aborted: u64,
    rollout_rank_changes: u64,
    rollout_best_updates: u64,
    rollout_time_ms: f64,
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
        trace.count_by("active_route_calls", self.active_route_calls as i64);
        trace.count_by("tree_edges_saved", self.tree_edges_saved as i64);
        trace.count_by("exact_support_selected", self.exact_support_selected as i64);
        trace.count_by("exact_gain_vs_thin", self.exact_gain_vs_thin);
        trace.count_by("joint_position_changed", self.joint_position_changed as i64);
        trace.count_by("minsum_bridge", self.minsum_bridge as i64);
        trace.count_by("full_axis_support", self.full_axis_support as i64);
        trace.count_by("overlap_candidates", self.overlap_candidates as i64);
        trace.count_by("overlap_selected", self.overlap_selected as i64);
        trace.count_by("overlap_path_ops_saved", self.overlap_path_ops_saved as i64);
        trace.count_by(
            "overlap_restored_checks",
            self.overlap_restored_checks as i64,
        );
        trace.count_by("rollout_runs", self.rollout_runs as i64);
        trace.count_by("rollout_completed", self.rollout_completed as i64);
        trace.count_by("rollout_aborted", self.rollout_aborted as i64);
        trace.count_by("rollout_rank_changes", self.rollout_rank_changes as i64);
        trace.count_by("rollout_best_updates", self.rollout_best_updates as i64);
        trace.add_time_ms("rollout", self.rollout_time_ms);
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    base_score: u64,
    cost: u32,
    potential: i32,
    tie_key: u64,
    hash_a: u64,
    hash_b: u64,
    parent_index: u32,
    leaf: u16,
}

#[derive(Clone, Copy)]
struct HistoryNode {
    parent: u32,
    leaf: u16,
}

struct PassResult {
    order: Vec<u16>,
    cost: u32,
    width: usize,
}

#[inline]
fn tie_hash(value: u64, seed: u64) -> u64 {
    let mut state = value ^ seed;
    splitmix64(&mut state)
}

#[inline]
fn candidate_key(candidate: &Candidate) -> (u64, u32, i32, u64, u64, u64) {
    (
        candidate.base_score,
        candidate.cost,
        candidate.potential,
        candidate.tie_key,
        candidate.hash_a,
        candidate.hash_b,
    )
}

fn history_prefix(history: &[HistoryNode], mut history_id: u32) -> Vec<u16> {
    let mut order = Vec::new();
    while history_id != 0 {
        let node = history[history_id as usize];
        order.push(node.leaf);
        history_id = node.parent;
    }
    order.reverse();
    order
}

/// rollout 自身から rollout は起動しない。同じ搬送と proxy 比較で width 1 を完走する。
#[allow(clippy::too_many_arguments)]
fn complete_width_one(
    input: &Input,
    mut state: SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    pass_seed: u64,
    search_deadline_sec: f64,
    time_keeper: &mut TimeKeeper,
    stats: &mut MechanismStats,
) -> Option<(Vec<u16>, u32)> {
    let started = Instant::now();
    stats.rollout_runs += 1;
    let mut suffix = Vec::with_capacity(state.active_count as usize - 1);
    let mut scratch = TransportScratch::with_capacity(CELLS);

    while state.active_count > 1 {
        time_keeper.force_update();
        if time_keeper.elapsed_sec() >= search_deadline_sec {
            stats.rollout_aborted += 1;
            stats.rollout_time_ms += started.elapsed().as_secs_f64() * 1000.0;
            return None;
        }
        let mut best: Option<Candidate> = None;
        for leaf in 0..CELLS {
            // 非葉も含めて一定間隔で確認し、疎な葉集合でも deadline を見失わない。
            time_keeper.step();
            if leaf & 7 == 7 {
                time_keeper.force_update();
                if time_keeper.elapsed_sec() >= search_deadline_sec {
                    stats.rollout_aborted += 1;
                    stats.rollout_time_ms += started.elapsed().as_secs_f64() * 1000.0;
                    return None;
                }
            }
            if !state.is_active(leaf) || state.degree[leaf] != 1 {
                continue;
            }
            let old_cost = state.operations_used;
            let touched =
                apply_transport(input, &mut state, tree, zobrist, leaf, &mut scratch, stats);
            let route_len = scratch.len() as u32;
            state.operations_used += route_len;
            let neighbor = remove_leaf(&mut state, tree, zobrist, leaf);
            let candidate = Candidate {
                base_score: state.score(),
                cost: state.operations_used,
                potential: state.potential,
                tie_key: tie_hash(state.hash_a ^ state.hash_b, pass_seed),
                hash_a: state.hash_a,
                hash_b: state.hash_b,
                parent_index: 0,
                leaf: leaf as u16,
            };
            restore_leaf(&mut state, zobrist, leaf, neighbor);
            state.operations_used = old_cost;
            let undo_touched = undo_transport(&mut state, tree, zobrist, &scratch, stats);
            assert_eq!(touched, undo_touched);
            stats.candidates += 1;
            stats.candidate_apply_ops += route_len as u64;
            stats.candidate_undo_ops += route_len as u64;
            stats.swap_pairs_touched += (touched + undo_touched) as u64;
            if best
                .as_ref()
                .map_or(true, |old| candidate_key(&candidate) < candidate_key(old))
            {
                best = Some(candidate);
            }
        }
        time_keeper.force_update();
        if time_keeper.elapsed_sec() >= search_deadline_sec {
            stats.rollout_aborted += 1;
            stats.rollout_time_ms += started.elapsed().as_secs_f64() * 1000.0;
            return None;
        }
        let selected = best.expect("an active tree with at least two cells has a leaf");
        let touched = apply_transport(
            input,
            &mut state,
            tree,
            zobrist,
            selected.leaf as usize,
            &mut scratch,
            stats,
        );
        state.operations_used += scratch.len() as u32;
        stats.materialize_ops += scratch.len() as u64;
        stats.swap_pairs_touched += touched as u64;
        remove_leaf(&mut state, tree, zobrist, selected.leaf as usize);
        suffix.push(selected.leaf);

        if state.active_count > 1 {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.rollout_aborted += 1;
                stats.rollout_time_ms += started.elapsed().as_secs_f64() * 1000.0;
                return None;
            }
        }
    }

    assert_eq!(state.misplaced_count, 0);
    assert!(state
        .board
        .iter()
        .enumerate()
        .all(|(cell, &card)| cell == card as usize));
    stats.rollout_completed += 1;
    stats.rollout_time_ms += started.elapsed().as_secs_f64() * 1000.0;
    Some((suffix, state.operations_used))
}

fn update_incumbent(
    incumbent: &mut Option<PassResult>,
    order: Vec<u16>,
    cost: u32,
    width: usize,
    from_rollout: bool,
    stats: &mut MechanismStats,
) {
    if incumbent.as_ref().map_or(true, |old| cost < old.cost) {
        *incumbent = Some(PassResult { order, cost, width });
        stats.best_updates += 1;
        stats.rollout_best_updates += from_rollout as u64;
    }
}

#[allow(clippy::too_many_arguments)]
fn beam_pass(
    input: &Input,
    initial: &SearchState,
    tree: &TreeData,
    zobrist: &Zobrist,
    width: usize,
    pass_seed: u64,
    enable_rollout: bool,
    abort_on_deadline: bool,
    search_deadline_sec: f64,
    time_keeper: &mut TimeKeeper,
    incumbent: &mut Option<PassResult>,
    stats: &mut MechanismStats,
) -> Option<PassResult> {
    stats.passes_started += 1;
    let mut root = initial.clone();
    root.history_id = 0;
    let mut beam = vec![root];
    let mut history = vec![HistoryNode {
        parent: u32::MAX,
        leaf: NO_CELL,
    }];
    let mut scratch = TransportScratch::with_capacity(CELLS);
    let mut deadline_work = 0_u64;

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
            for leaf in 0..CELLS {
                if !parent.is_active(leaf) || parent.degree[leaf] != 1 {
                    continue;
                }

                #[cfg(feature = "local")]
                let rollback_snapshot = if stats.candidates & 255 == 0 {
                    Some(parent.clone())
                } else {
                    None
                };

                let old_cost = parent.operations_used;
                let touched =
                    apply_transport(input, parent, tree, zobrist, leaf, &mut scratch, stats);
                let route_len = scratch.len() as u32;
                parent.operations_used += route_len;
                let neighbor = remove_leaf(parent, tree, zobrist, leaf);
                let candidate = Candidate {
                    base_score: parent.score(),
                    cost: parent.operations_used,
                    potential: parent.potential,
                    tie_key: tie_hash(parent.hash_a ^ parent.hash_b, pass_seed),
                    hash_a: parent.hash_a,
                    hash_b: parent.hash_b,
                    parent_index: parent_index as u32,
                    leaf: leaf as u16,
                };

                restore_leaf(parent, zobrist, leaf, neighbor);
                parent.operations_used = old_cost;
                let undo_touched = undo_transport(parent, tree, zobrist, &scratch, stats);
                assert_eq!(touched, undo_touched);

                stats.candidates += 1;
                stats.candidate_apply_ops += route_len as u64;
                stats.candidate_undo_ops += route_len as u64;
                stats.swap_pairs_touched += (touched + undo_touched) as u64;

                #[cfg(feature = "local")]
                if let Some(snapshot) = rollback_snapshot {
                    assert!(*parent == snapshot);
                    verify_state(parent, tree, zobrist);
                    stats.rollback_checks += 1;
                }

                candidates.push(candidate);
                time_keeper.step();
                if abort_on_deadline {
                    deadline_work += 1;
                    if deadline_work & 7 == 0 {
                        time_keeper.force_update();
                        if time_keeper.elapsed_sec() >= search_deadline_sec {
                            stats.passes_aborted += 1;
                            return None;
                        }
                    }
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

        let compare_candidates =
            |a: &Candidate, b: &Candidate| candidate_key(a).cmp(&candidate_key(b));

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

        let rollout_here = enable_rollout
            && ROLLOUT_DEPTHS.contains(&_depth)
            && stats.rollout_completed < MAX_COMPLETED_ROLLOUTS;
        let shortlist_limit = width.max(if rollout_here { 2 } else { width });
        let mut selected = Vec::with_capacity(shortlist_limit.min(candidates.len()));
        let mut seen = HashSet::with_capacity(shortlist_limit.saturating_mul(2));
        for candidate in candidates {
            if !seen.insert((candidate.hash_a, candidate.hash_b)) {
                stats.duplicate_drops += 1;
                continue;
            }
            selected.push(candidate);
            if selected.len() == shortlist_limit {
                break;
            }
        }
        assert!(!selected.is_empty());

        if rollout_here {
            // 幅1でも上位2つの相異なる状態を実完成させ、exact cost で残す1つを選ぶ。
            let rollout_count = selected.len().min(2);
            let mut exact_cost = [None; 2];
            for candidate_index in 0..rollout_count {
                if stats.rollout_completed >= MAX_COMPLETED_ROLLOUTS {
                    break;
                }
                time_keeper.force_update();
                if time_keeper.elapsed_sec() >= search_deadline_sec {
                    break;
                }
                let candidate = selected[candidate_index];
                let mut child = beam[candidate.parent_index as usize].clone();
                let touched = apply_transport(
                    input,
                    &mut child,
                    tree,
                    zobrist,
                    candidate.leaf as usize,
                    &mut scratch,
                    stats,
                );
                child.operations_used += scratch.len() as u32;
                stats.materialize_ops += scratch.len() as u64;
                stats.swap_pairs_touched += touched as u64;
                remove_leaf(&mut child, tree, zobrist, candidate.leaf as usize);
                assert_eq!(child.operations_used, candidate.cost);
                assert_eq!(child.hash_a, candidate.hash_a);
                assert_eq!(child.hash_b, candidate.hash_b);

                let mut complete_order = history_prefix(&history, child.history_id);
                complete_order.push(candidate.leaf);
                if let Some((suffix, exact_total_cost)) = complete_width_one(
                    input,
                    child,
                    tree,
                    zobrist,
                    pass_seed,
                    search_deadline_sec,
                    time_keeper,
                    stats,
                ) {
                    exact_cost[candidate_index] = Some(exact_total_cost);
                    complete_order.extend_from_slice(&suffix);
                    assert_eq!(complete_order.len(), CELLS - 1);
                    update_incumbent(
                        incumbent,
                        complete_order,
                        exact_total_cost,
                        width,
                        true,
                        stats,
                    );
                } else {
                    time_keeper.force_update();
                    if time_keeper.elapsed_sec() >= search_deadline_sec {
                        break;
                    }
                }
            }
            if rollout_count == 2 {
                if let (Some(first), Some(second)) = (exact_cost[0], exact_cost[1]) {
                    if second < first {
                        selected.swap(0, 1);
                        stats.rollout_rank_changes += 1;
                    }
                }
            }
        }

        if abort_on_deadline {
            time_keeper.force_update();
            if time_keeper.elapsed_sec() >= search_deadline_sec {
                stats.passes_aborted += 1;
                return None;
            }
        }

        selected.truncate(width.min(selected.len()));

        let mut children = Vec::with_capacity(selected.len());
        for (selected_index, candidate) in selected.into_iter().enumerate() {
            let mut child = beam[candidate.parent_index as usize].clone();
            let touched = apply_transport(
                input,
                &mut child,
                tree,
                zobrist,
                candidate.leaf as usize,
                &mut scratch,
                stats,
            );
            child.operations_used += scratch.len() as u32;
            remove_leaf(&mut child, tree, zobrist, candidate.leaf as usize);
            assert_eq!(child.operations_used, candidate.cost);
            assert_eq!(child.potential, candidate.potential);
            assert_eq!(child.hash_a, candidate.hash_a);
            assert_eq!(child.hash_b, candidate.hash_b);

            history.push(HistoryNode {
                parent: child.history_id,
                leaf: candidate.leaf,
            });
            child.history_id = (history.len() - 1) as u32;
            stats.materialize_ops += scratch.len() as u64;
            stats.swap_pairs_touched += touched as u64;
            children.push(child);

            if abort_on_deadline && selected_index & 7 == 7 {
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
            verify_state(&beam[0], tree, zobrist);
            stats.invariant_checks += 1;
        }
    }

    let best_index = (0..beam.len())
        .min_by_key(|&index| {
            (
                beam[index].operations_used,
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
        order.push(node.leaf);
        history_id = node.parent;
    }
    order.reverse();
    assert_eq!(order.len(), CELLS - 1);

    stats.passes_completed += 1;
    stats.max_completed_width = stats.max_completed_width.max(width as u64);
    Some(PassResult {
        order,
        cost: best.operations_used,
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
    let mut scratch = TransportScratch::with_capacity(CELLS);
    for &leaf in order {
        apply_transport(
            input,
            &mut state,
            tree,
            zobrist,
            leaf as usize,
            &mut scratch,
            stats,
        );
        state.operations_used += scratch.len() as u32;
        operations.extend_from_slice(&scratch.operations);
        remove_leaf(&mut state, tree, zobrist, leaf as usize);
    }
    (state, operations)
}

fn main() {
    // V000 と同様、入力読み込みより前を基準時刻にする。
    let mut time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
    let mut trace = TraceStats::default();
    let input = Input::read();
    let tree = local_time!(trace, "tree_build", { build_spanning_tree(&input) });
    let zobrist = local_time!(trace, "zobrist_build", { Zobrist::new() });
    let initial = SearchState::new(&input, &tree, &zobrist);
    #[cfg(feature = "local")]
    verify_state(&initial, &tree, &zobrist);

    let search_started = Instant::now();
    let search_deadline_sec = PROGRAM_TIME_LIMIT_SEC * SEARCH_TIME_RATIO;
    let mut stats = MechanismStats::default();
    let mut incumbent = None;

    // rollout を含まない幅1を完走し、時刻に依存しない完成解を先に保持する。
    let greedy = beam_pass(
        &input,
        &initial,
        &tree,
        &zobrist,
        1,
        1,
        false,
        false,
        search_deadline_sec,
        &mut time_keeper,
        &mut incumbent,
        &mut stats,
    )
    .unwrap();
    let greedy_cost = greedy.cost;
    update_incumbent(
        &mut incumbent,
        greedy.order,
        greedy.cost,
        greedy.width,
        false,
        &mut stats,
    );

    // exact rollout は専用の幅1 pass に隔離し、通常 beam の時間を先に予約する。
    let rollout_deadline_sec = PROGRAM_TIME_LIMIT_SEC * ROLLOUT_PASS_TIME_RATIO;
    let rollout_width_one = beam_pass(
        &input,
        &initial,
        &tree,
        &zobrist,
        1,
        2,
        true,
        true,
        rollout_deadline_sec,
        &mut time_keeper,
        &mut incumbent,
        &mut stats,
    );
    if let Some(rollout_width_one) = rollout_width_one {
        update_incumbent(
            &mut incumbent,
            rollout_width_one.order,
            rollout_width_one.cost,
            rollout_width_one.width,
            false,
            &mut stats,
        );
    }

    let width_two = beam_pass(
        &input,
        &initial,
        &tree,
        &zobrist,
        2,
        3,
        false,
        true,
        search_deadline_sec,
        &mut time_keeper,
        &mut incumbent,
        &mut stats,
    );
    if let Some(width_two) = width_two {
        update_incumbent(
            &mut incumbent,
            width_two.order,
            width_two.cost,
            width_two.width,
            false,
            &mut stats,
        );
    }

    let mut width = 8_usize;
    let mut pass_seed = 4_u64;
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
            false,
            true,
            search_deadline_sec,
            &mut time_keeper,
            &mut incumbent,
            &mut stats,
        );
        let Some(result) = result else {
            break;
        };
        update_incumbent(
            &mut incumbent,
            result.order,
            result.cost,
            result.width,
            false,
            &mut stats,
        );
        width = (width * 4).min(MAX_BEAM_WIDTH);
        pass_seed += 1;
    }
    time_keeper.force_update();
    trace.add_time_ms(
        "beam_search",
        search_started.elapsed().as_secs_f64() * 1000.0,
    );

    let best = incumbent.expect("width-1 pass always supplies a completed incumbent");
    let best_width = best.width;
    let best_cost = best.cost;
    let best_order = best.order;
    let (final_state, operations) = local_time!(trace, "final_replay", {
        replay_order(&input, &initial, &tree, &zobrist, &best_order, &mut stats)
    });

    assert_eq!(operations.len(), final_state.operations_used as usize);
    assert_eq!(operations.len(), best_cost as usize);
    assert!(operations.len() <= MAX_OPERATIONS);
    assert!(operations.len() <= PROVEN_MAX_OPERATIONS);
    assert_eq!(final_state.active_count, 1);
    assert_eq!(final_state.misplaced_count, 0);
    assert!(final_state
        .board
        .iter()
        .enumerate()
        .all(|(cell, &card)| cell == card as usize));
    assert!(operations.iter().all(|&op| is_legal_operation(&input, op)));
    #[cfg(feature = "local")]
    verify_state(&final_state, &tree, &zobrist);

    let adjacent_ops = operations
        .iter()
        .filter(|op| match op.direction {
            Direction::Vertical => op.h / 2 == 1,
            Direction::Horizontal => op.w / 2 == 1,
        })
        .count();
    let jump_ops = operations.len() - adjacent_ops;

    stats.flush(&mut trace);
    #[cfg(feature = "local")]
    assert_eq!(trace.fallback_count, 0);
    trace.count_by("greedy_t", greedy_cost as i64);
    trace.count_by("final_t", operations.len() as i64);
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
