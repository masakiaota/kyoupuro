// v023_reverse_portfolio.rs
// v022_cost_weight を初期盤面(前向き)と逆置換(逆向き)の両方へ半分ずつの時間で
// 適用し、完成列が短い方を出力する時間反転 portfolio(IDEA-D20 の v022 ベース版)。
// 操作は対合なので、逆置換を解いた操作列を逆順にするだけで元問題の解になる。
// 探索は各方向とも「幅 1 pass(完成保証、deadline 無視で完走)+ 固定幅 64 の単一 pass」
// とし、反復幅拡大は行わない。状態・遷移・評価・木は v022 と同一。
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
/// 手数と木距離和の換算重み。実測の距離消化率(約 4.4 距離/手、v021 の 0000)に
/// 合わせて 4 とする(旧値は最大 shift の 10 で、手数を過大に重視していた)。
const EVAL_COST_WEIGHT: u64 = 4;
/// 本命 pass の固定ビーム幅。反復幅拡大はせず、方向ごとにこの幅 1 本だけ回す。
/// 半分時間(local 約 0.69 秒)で 399 深を完走できる上限から 16 とする
/// (1 親展開 ≈ 70μs の実測より、幅 64 は両方向とも中断して機構が発動しなかった)。
const FIXED_BEAM_WIDTH: usize = 16;
const NO_CELL: u16 = u16::MAX;

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
    initial_degree: [u8; CELLS],
    distance: Vec<u16>,
    next: Vec<u16>,
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
    let mut next_step = vec![NO_CELL; CELLS * CELLS];
    for source in 0..CELLS {
        let base = source * CELLS;
        distance[base + source] = 0;
        next_step[base + source] = source as u16;
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
                next_step[base + to] = if cell == source {
                    to as u16
                } else {
                    next_step[base + cell]
                };
                bfs.push_back(to);
            }
        }
    }

    TreeData {
        neighbors: tree_neighbors,
        neighbor_count,
        initial_degree: neighbor_count,
        distance,
        next: next_step,
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
        if value {
            self.active[cell >> 6] |= bit;
        } else {
            self.active[cell >> 6] &= !bit;
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

/// 帯位置選択のための 1 本プローブ: 帯の両隣 1 本ずつを拡大した場合の木距離和差分の
/// 負部分だけを返す。判定・差分式は expand_support と同一で、適用時の min-sum 拡大の
/// 1 次近似として「拡大しやすい帯位置」を候補比較へ反映する。
fn expand_probe(input: &Input, state: &SearchState, tree: &TreeData, op: Operation) -> i32 {
    let mut probe = 0;
    match op.direction {
        Direction::Horizontal => {
            let half = op.w / 2;
            let row_delta = |i: usize| -> i32 {
                let mut delta = 0;
                for y in 0..half {
                    let a = i * N + op.c + y;
                    let b = a + half;
                    let card_a = state.board[a] as usize;
                    let card_b = state.board[b] as usize;
                    delta += tree.distance[b * CELLS + card_a] as i32
                        + tree.distance[a * CELLS + card_b] as i32
                        - tree.distance[a * CELLS + card_a] as i32
                        - tree.distance[b * CELLS + card_b] as i32;
                }
                delta
            };
            let row_ok = |i: usize| -> bool {
                (op.c..op.c + op.w).all(|j| state.is_active(i * N + j))
                    && (op.c..op.c + op.w - 1).all(|j| !input.vertical_walls[i][j])
            };
            let boundary_ok = |i: usize| -> bool {
                (op.c..op.c + op.w).all(|j| !input.horizontal_walls[i][j])
            };
            if op.r > 0 && boundary_ok(op.r - 1) && row_ok(op.r - 1) {
                probe += row_delta(op.r - 1).min(0);
            }
            let bottom = op.r + op.h - 1;
            if bottom + 1 < N && boundary_ok(bottom) && row_ok(bottom + 1) {
                probe += row_delta(bottom + 1).min(0);
            }
        }
        Direction::Vertical => {
            let half = op.h / 2;
            let col_delta = |j: usize| -> i32 {
                let mut delta = 0;
                for x in 0..half {
                    let a = (op.r + x) * N + j;
                    let b = a + half * N;
                    let card_a = state.board[a] as usize;
                    let card_b = state.board[b] as usize;
                    delta += tree.distance[b * CELLS + card_a] as i32
                        + tree.distance[a * CELLS + card_b] as i32
                        - tree.distance[a * CELLS + card_a] as i32
                        - tree.distance[b * CELLS + card_b] as i32;
                }
                delta
            };
            let col_ok = |j: usize| -> bool {
                (op.r..op.r + op.h).all(|i| state.is_active(i * N + j))
                    && (op.r..op.r + op.h - 1).all(|i| !input.horizontal_walls[i][j])
            };
            let boundary_ok = |j: usize| -> bool {
                (op.r..op.r + op.h).all(|i| !input.vertical_walls[i][j])
            };
            if op.c > 0 && boundary_ok(op.c - 1) && col_ok(op.c - 1) {
                probe += col_delta(op.c - 1).min(0);
            }
            let right = op.c + op.w - 1;
            if right + 1 < N && boundary_ok(right) && col_ok(right + 1) {
                probe += col_delta(right + 1).min(0);
            }
        }
    }
    probe
}

/// `from` を `to` へ写す最小支持の一次元操作から、巻き添え後の木距離和が最小のものを選ぶ。
/// 比較値には隣 1 本ぶんの拡大見込み益(expand_probe)を加え、拡大余地も帯位置選択に効かせる。
fn best_mapping_operation(
    input: &Input,
    state: &SearchState,
    tree: &TreeData,
    from: usize,
    to: usize,
) -> Option<Operation> {
    let from_i = from / N;
    let from_j = from % N;
    let to_i = to / N;
    let to_j = to % N;
    let mut best: Option<(i32, usize, Operation)> = None;

    if from_i == to_i {
        let shift = from_j.abs_diff(to_j);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let width = 2 * shift;
        for c in 0..=N - width {
            let op = Operation {
                direction: Direction::Horizontal,
                r: from_i,
                c,
                h: 1,
                w: width,
            };
            if mapped_cell(op, from) != Some(to) || !is_active_legal_operation(input, state, op) {
                continue;
            }
            let delta = operation_delta_potential(state, tree, op) + expand_probe(input, state, tree, op);
            let key = (delta, c);
            let is_better = match best {
                Some((best_delta, best_pos, _)) => key < (best_delta, best_pos),
                None => true,
            };
            if is_better {
                best = Some((delta, c, op));
            }
        }
    } else if from_j == to_j {
        let shift = from_i.abs_diff(to_i);
        if shift == 0 || shift > MAX_SINGLE_AXIS_SHIFT {
            return None;
        }
        let height = 2 * shift;
        for r in 0..=N - height {
            let op = Operation {
                direction: Direction::Vertical,
                r,
                c: from_j,
                h: height,
                w: 1,
            };
            if mapped_cell(op, from) != Some(to) || !is_active_legal_operation(input, state, op) {
                continue;
            }
            let delta = operation_delta_potential(state, tree, op) + expand_probe(input, state, tree, op);
            let key = (delta, r);
            let is_better = match best {
                Some((best_delta, best_pos, _)) => key < (best_delta, best_pos),
                None => true,
            };
            if is_better {
                best = Some((delta, r, op));
            }
        }
    }
    best.map(|(_, _, op)| op)
}

/// 選択済みの細長運搬帯を、from 行(列)を含む直交方向の連続区間へ貪欲に拡大する。
/// 追加行(列)の木距離和差分を 1 本ずつ累積し、累積が負である限り伸ばす。
/// 戻り値は (拡大後の操作, 差分合計, 追加した本数)。拡大なしなら元の操作を返す。
fn expand_support(
    input: &Input,
    state: &SearchState,
    tree: &TreeData,
    op: Operation,
) -> (Operation, i32, usize) {
    match op.direction {
        Direction::Horizontal => {
            let half = op.w / 2;
            // 行 i を帯へ追加したときの木距離和差分。
            let row_delta = |i: usize| -> i32 {
                let mut delta = 0;
                for y in 0..half {
                    let a = i * N + op.c + y;
                    let b = a + half;
                    let card_a = state.board[a] as usize;
                    let card_b = state.board[b] as usize;
                    delta += tree.distance[b * CELLS + card_a] as i32
                        + tree.distance[a * CELLS + card_b] as i32
                        - tree.distance[a * CELLS + card_a] as i32
                        - tree.distance[b * CELLS + card_b] as i32;
                }
                delta
            };
            // 行 i が梁として使えるか(全セル active かつ区間内 V 壁なし)。
            let row_ok = |i: usize| -> bool {
                (op.c..op.c + op.w).all(|j| state.is_active(i * N + j))
                    && (op.c..op.c + op.w - 1).all(|j| !input.vertical_walls[i][j])
            };
            // 行境界 (i, i+1) が帯として繋がるか(H 壁なし)。
            let boundary_ok = |i: usize| -> bool {
                (op.c..op.c + op.w).all(|j| !input.horizontal_walls[i][j])
            };

            // 固定端 min-sum: 合法な限り累積し、累積が最小(負)になる位置まで採用する。
            // 途中に正の行があっても、その先の大きな負を取りに行ける。
            let mut top = op.r;
            let mut gain = 0;
            {
                let mut cum = 0;
                let mut scan = op.r;
                while scan > 0 && boundary_ok(scan - 1) && row_ok(scan - 1) {
                    scan -= 1;
                    cum += row_delta(scan);
                    if cum < gain {
                        gain = cum;
                        top = scan;
                    }
                }
            }
            let mut bottom = op.r + op.h - 1;
            let mut lower_gain = 0;
            {
                let mut cum = 0;
                let mut scan = bottom;
                while scan + 1 < N && boundary_ok(scan) && row_ok(scan + 1) {
                    scan += 1;
                    cum += row_delta(scan);
                    if cum < lower_gain {
                        lower_gain = cum;
                        bottom = scan;
                    }
                }
            }
            gain += lower_gain;
            let rows_added = (op.r - top) + (bottom - (op.r + op.h - 1));
            if rows_added == 0 {
                return (op, 0, 0);
            }
            let expanded = Operation {
                direction: Direction::Horizontal,
                r: top,
                c: op.c,
                h: bottom - top + 1,
                w: op.w,
            };
            debug_assert!(is_active_legal_operation(input, state, expanded));
            (expanded, gain, rows_added)
        }
        Direction::Vertical => {
            let half = op.h / 2;
            let col_delta = |j: usize| -> i32 {
                let mut delta = 0;
                for x in 0..half {
                    let a = (op.r + x) * N + j;
                    let b = a + half * N;
                    let card_a = state.board[a] as usize;
                    let card_b = state.board[b] as usize;
                    delta += tree.distance[b * CELLS + card_a] as i32
                        + tree.distance[a * CELLS + card_b] as i32
                        - tree.distance[a * CELLS + card_a] as i32
                        - tree.distance[b * CELLS + card_b] as i32;
                }
                delta
            };
            let col_ok = |j: usize| -> bool {
                (op.r..op.r + op.h).all(|i| state.is_active(i * N + j))
                    && (op.r..op.r + op.h - 1).all(|i| !input.horizontal_walls[i][j])
            };
            let boundary_ok = |j: usize| -> bool {
                (op.r..op.r + op.h).all(|i| !input.vertical_walls[i][j])
            };

            // 固定端 min-sum(行方向と対称)。
            let mut left = op.c;
            let mut gain = 0;
            {
                let mut cum = 0;
                let mut scan = op.c;
                while scan > 0 && boundary_ok(scan - 1) && col_ok(scan - 1) {
                    scan -= 1;
                    cum += col_delta(scan);
                    if cum < gain {
                        gain = cum;
                        left = scan;
                    }
                }
            }
            let mut right = op.c + op.w - 1;
            let mut right_gain = 0;
            {
                let mut cum = 0;
                let mut scan = right;
                while scan + 1 < N && boundary_ok(scan) && col_ok(scan + 1) {
                    scan += 1;
                    cum += col_delta(scan);
                    if cum < right_gain {
                        right_gain = cum;
                        right = scan;
                    }
                }
            }
            gain += right_gain;
            let cols_added = (op.c - left) + (right - (op.c + op.w - 1));
            if cols_added == 0 {
                return (op, 0, 0);
            }
            let expanded = Operation {
                direction: Direction::Vertical,
                r: op.r,
                c: left,
                h: op.h,
                w: right - left + 1,
            };
            debug_assert!(is_active_legal_operation(input, state, expanded));
            (expanded, gain, cols_added)
        }
    }
}

/// 葉の正解カードを全域木経路に沿って運ぶ。各直線部分では最長の合法 shift を使う。
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

    while current != target {
        let first = tree.next[current * CELLS + target] as usize;
        assert_ne!(first, current);
        let step_i = first as i32 / N as i32 - current as i32 / N as i32;
        let step_j = first as i32 % N as i32 - current as i32 % N as i32;
        let mut straight_nodes = [0_usize; MAX_SINGLE_AXIS_SHIFT + 1];
        straight_nodes[0] = current;
        straight_nodes[1] = first;
        let mut straight_len = 1;
        while straight_len < MAX_SINGLE_AXIS_SHIFT && straight_nodes[straight_len] != target {
            let at = straight_nodes[straight_len];
            let next = tree.next[at * CELLS + target] as usize;
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
            if let Some(op) = best_mapping_operation(input, state, tree, current, next_position) {
                selected = Some((next_position, op));
                break;
            }
        }
        let (next_position, op) = selected.expect("tree edge must provide an adjacent operation");
        // 支持拡大: 巻き添えの木距離和が減るなら帯を直交方向へ広げる。
        // 直交幅は注目カードの写像に影響しないため、搬送経路は変わらない。
        let (op, expand_gain, added) = expand_support(input, state, tree, op);
        if added > 0 {
            stats.support_expansions += 1;
            stats.support_rows_added += added as u64;
            stats.support_gain += expand_gain as i64;
        }
        touched_pairs += apply_operation(state, tree, zobrist, op);
        scratch.push(op);
        current = state.position[target] as usize;
        assert_eq!(current, next_position);
    }
    touched_pairs
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
    // 支持拡大の計測。
    support_expansions: u64,
    support_rows_added: u64,
    support_gain: i64,
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
        trace.count_by("support_expansions", self.support_expansions as i64);
        trace.count_by("support_rows_added", self.support_rows_added as i64);
        trace.count_by("support_gain", self.support_gain);
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
        leaf: NO_CELL,
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
                    verify_state(parent, tree, zobrist);
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
                a.base_score,
                a.cost,
                a.potential,
                a.tie_key,
                a.hash_a,
                a.hash_b,
            );
            let key_b = (
                b.base_score,
                b.cost,
                b.potential,
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
    let mut scratch = Vec::with_capacity(CELLS);
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
        operations.extend_from_slice(&scratch);
        remove_leaf(&mut state, tree, zobrist, leaf as usize);
    }
    (state, operations)
}

/// 一方向ぶんの探索: 幅 1 pass を完成保証のため deadline 無視で完走させた後、
/// 固定幅 FIXED_BEAM_WIDTH の pass を 1 本だけ回す(反復幅拡大はしない)。
/// 戻り値は (最良の完成計画, 幅 1 の T)。
#[allow(clippy::too_many_arguments)]
fn solve_direction(
    input: &Input,
    tree: &TreeData,
    zobrist: &Zobrist,
    deadline_sec: f64,
    seed_base: u64,
    time_keeper: &mut TimeKeeper,
    stats: &mut MechanismStats,
) -> (PassResult, u32) {
    let initial = SearchState::new(input, tree, zobrist);
    #[cfg(feature = "local")]
    verify_state(&initial, tree, zobrist);

    let mut best = beam_pass(
        input,
        &initial,
        tree,
        zobrist,
        1,
        seed_base,
        false,
        deadline_sec,
        time_keeper,
        stats,
    )
    .unwrap();
    let greedy_cost = best.cost;

    time_keeper.force_update();
    if time_keeper.elapsed_sec() < deadline_sec {
        if let Some(result) = beam_pass(
            input,
            &initial,
            tree,
            zobrist,
            FIXED_BEAM_WIDTH,
            seed_base + 1,
            true,
            deadline_sec,
            time_keeper,
            stats,
        ) {
            if result.cost < best.cost {
                best = result;
                stats.best_updates += 1;
            }
        }
    }
    (best, greedy_cost)
}

fn main() {
    // V000 と同様、入力読み込みより前を基準時刻にする。
    let mut time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
    let mut trace = TraceStats::default();
    let input = Input::read();
    let tree = local_time!(trace, "tree_build", { build_spanning_tree(&input) });
    let zobrist = local_time!(trace, "zobrist_build", { Zobrist::new() });

    let search_started = Instant::now();
    let full_deadline = PROGRAM_TIME_LIMIT_SEC * SEARCH_TIME_RATIO;
    let half_deadline = full_deadline * 0.5;
    let mut stats = MechanismStats::default();

    // 前向き(探索時間の前半)。
    let (forward_best, forward_greedy) =
        solve_direction(&input, &tree, &zobrist, half_deadline, 1, &mut time_keeper, &mut stats);

    // 逆向き(残り時間): 逆置換 position を初期盤面とみなした同一問題を解く。
    // 対合性より、その解列を逆順にしたものが元問題の解になる。
    let mut inverse_board = [0usize; CELLS];
    for cell in 0..CELLS {
        inverse_board[input.initial_board[cell]] = cell;
    }
    let reverse_input = Input {
        initial_board: inverse_board,
        vertical_walls: input.vertical_walls,
        horizontal_walls: input.horizontal_walls,
    };
    let (reverse_best, reverse_greedy) = solve_direction(
        &reverse_input,
        &tree,
        &zobrist,
        full_deadline,
        1001,
        &mut time_keeper,
        &mut stats,
    );

    time_keeper.force_update();
    trace.add_time_ms(
        "beam_search",
        search_started.elapsed().as_secs_f64() * 1000.0,
    );

    let use_reverse = reverse_best.cost < forward_best.cost;
    let forward_cost = forward_best.cost;
    let reverse_cost = reverse_best.cost;
    let (chosen_input, chosen) = if use_reverse {
        (&reverse_input, reverse_best)
    } else {
        (&input, forward_best)
    };

    let best_width = chosen.width;
    let best_cost = chosen.cost;
    let chosen_initial = SearchState::new(chosen_input, &tree, &zobrist);
    let (final_state, mut operations) = local_time!(trace, "final_replay", {
        replay_order(
            chosen_input,
            &chosen_initial,
            &tree,
            &zobrist,
            &chosen.order,
            &mut stats,
        )
    });

    assert_eq!(operations.len(), final_state.operations_used as usize);
    assert_eq!(operations.len(), best_cost as usize);
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
    verify_state(&final_state, &tree, &zobrist);

    if use_reverse {
        // 逆向きの解列は逆順に読み替えると元問題の解になる(各操作は自己逆)。
        operations.reverse();
    }

    // 選ばれた向きに関わらず、出力列を元の初期盤面へ再生して完成を最終検証する。
    #[cfg(feature = "local")]
    {
        let mut replay: [u16; CELLS] = std::array::from_fn(|cell| input.initial_board[cell] as u16);
        for op in &operations {
            match op.direction {
                Direction::Vertical => {
                    let half = op.h / 2;
                    for x in 0..half {
                        for j in op.c..op.c + op.w {
                            let a = (op.r + x) * N + j;
                            replay.swap(a, a + half * N);
                        }
                    }
                }
                Direction::Horizontal => {
                    let half = op.w / 2;
                    for i in op.r..op.r + op.h {
                        for y in 0..half {
                            let a = i * N + op.c + y;
                            replay.swap(a, a + half);
                        }
                    }
                }
            }
        }
        assert!(
            (0..CELLS).all(|cell| replay[cell] as usize == cell),
            "逆順出力の再生が完成配置にならない"
        );
    }

    let adjacent_ops = operations
        .iter()
        .filter(|op| match op.direction {
            Direction::Vertical => op.h / 2 == 1,
            Direction::Horizontal => op.w / 2 == 1,
        })
        .count();
    let jump_ops = operations.len() - adjacent_ops;

    stats.flush(&mut trace);
    trace.count_by("greedy_t", forward_greedy as i64);
    trace.count_by("reverse_greedy_t", reverse_greedy as i64);
    trace.count_by("forward_t", forward_cost as i64);
    trace.count_by("reverse_t", reverse_cost as i64);
    trace.count_by("reverse_selected", use_reverse as i64);
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
