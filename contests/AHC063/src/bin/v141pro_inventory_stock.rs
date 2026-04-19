// v141pro_inventory_stock.rs
// これはあかんわ
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::ops::Deref;
use std::ops::Index;
use std::time::Instant;

const INTERNAL_COLOR_CAPACITY: usize = 16 * 16;
const INTERNAL_COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const INTERNAL_COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
const INTERNAL_POS_DEQUE_CAPACITY: usize = 16 * 16;
const INTERNAL_POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
const INTERNAL_POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const MAX_TURNS: usize = 100_000;
type Dir = u8; // 0 = U, 1 = D, 2 = L, 3 = R
type Ops = Vec<Dir>;
const DIR_U: Dir = 0;
const DIR_D: Dir = 1;
const DIR_L: Dir = 2;
const DIR_R: Dir = 3;
const DIR_COUNT: usize = 4;
const ALL_DIRS: [Dir; DIR_COUNT] = [DIR_U, DIR_D, DIR_L, DIR_R];
const DIR_CHARS: [char; DIR_COUNT] = ['U', 'D', 'L', 'R'];

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Cell(u16);

struct Grid;

impl Grid {
    #[inline]
    fn cell(n: usize, i: usize, j: usize) -> Cell {
        debug_assert!(i < n && j < n);
        Cell((i * n + j) as u16)
    }

    #[inline]
    fn index(cell: Cell) -> usize {
        cell.0 as usize
    }

    #[inline]
    fn i(n: usize, cell: Cell) -> usize {
        Self::index(cell) / n
    }

    #[inline]
    fn j(n: usize, cell: Cell) -> usize {
        Self::index(cell) % n
    }

    #[inline]
    fn ij(n: usize, cell: Cell) -> (usize, usize) {
        (Self::i(n, cell), Self::j(n, cell))
    }

    #[inline]
    fn can_move(n: usize, cell: Cell, dir: Dir) -> bool {
        if dir >= DIR_COUNT as Dir {
            return false;
        }
        let idx = Self::index(cell);
        match dir {
            0 => idx >= n,
            1 => idx + n < n * n,
            2 => idx % n != 0,
            3 => idx % n + 1 < n,
            _ => false,
        }
    }

    #[inline]
    fn next_cell(n: usize, cell: Cell, dir: Dir) -> Cell {
        debug_assert!(Self::can_move(n, cell, dir));
        let idx = Self::index(cell);
        match dir {
            0 => Cell((idx - n) as u16),
            1 => Cell((idx + n) as u16),
            2 => Cell((idx - 1) as u16),
            3 => Cell((idx + 1) as u16),
            _ => unreachable!("invalid dir: {dir}"),
        }
    }

    #[inline]
    fn dir_between_cells(n: usize, from: Cell, to: Cell) -> Dir {
        let from_idx = Self::index(from);
        let to_idx = Self::index(to);
        if to_idx + n == from_idx {
            0
        } else if from_idx + n == to_idx {
            1
        } else if to_idx + 1 == from_idx && from_idx / n == to_idx / n {
            2
        } else if from_idx + 1 == to_idx && from_idx / n == to_idx / n {
            3
        } else {
            unreachable!("cells are not adjacent: from={from_idx}, to={to_idx}");
        }
    }
}

#[derive(Debug, Clone)]
struct ManhattanTable {
    cell_count: usize,
    dist: Vec<u8>,
}

impl ManhattanTable {
    fn new(n: usize) -> Self {
        let cell_count = n * n;
        let mut dist = vec![0_u8; cell_count * cell_count];
        for a in 0..cell_count {
            let ai = a / n;
            let aj = a % n;
            for b in a..cell_count {
                let bi = b / n;
                let bj = b % n;
                let d = (ai.abs_diff(bi) + aj.abs_diff(bj)) as u8;
                dist[a * cell_count + b] = d;
                dist[b * cell_count + a] = d;
            }
        }
        Self { cell_count, dist }
    }

    #[inline]
    fn get(&self, a: Cell, b: Cell) -> usize {
        self.dist[Grid::index(a) * self.cell_count + Grid::index(b)] as usize
    }
}

const fn build_internal_pos_hash_pows(base: u64) -> [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] {
    let mut pows = [0_u64; INTERNAL_POS_DEQUE_CAPACITY + 1];
    pows[0] = 1;
    let mut i = 1;
    while i <= INTERNAL_POS_DEQUE_CAPACITY {
        pows[i] = pows[i - 1].wrapping_mul(base);
        i += 1;
    }
    pows
}

const INTERNAL_POS_HASH_POW1: [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] =
    build_internal_pos_hash_pows(INTERNAL_POS_HASH_BASE1);
const INTERNAL_POS_HASH_POW2: [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] =
    build_internal_pos_hash_pows(INTERNAL_POS_HASH_BASE2);

#[inline]
fn encode_internal_pos_hash(cell: Cell) -> u64 {
    cell.0 as u64 + 1
}

#[derive(Clone)]
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [Cell; INTERNAL_POS_DEQUE_CAPACITY],
    hash1: u64,
    hash2: u64,
}

impl InternalPosDeque {
    // hash化が遅いかもしれない
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [Cell(0); INTERNAL_POS_DEQUE_CAPACITY],
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(cells: &[Cell]) -> Self {
        debug_assert!(cells.len() <= INTERNAL_POS_DEQUE_CAPACITY);
        let mut deque = Self::new();
        deque.buf[..cells.len()].copy_from_slice(cells);
        deque.len = cells.len();
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &cell in cells {
            let x = encode_internal_pos_hash(cell);
            deque.hash1 = deque.hash1.wrapping_add(x.wrapping_mul(pow1));
            deque.hash2 = deque.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(INTERNAL_POS_HASH_BASE1);
            pow2 = pow2.wrapping_mul(INTERNAL_POS_HASH_BASE2);
        }
        deque
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn physical_index(&self, idx: usize) -> usize {
        debug_assert!(idx < self.len);
        let raw = self.head + idx;
        if raw < INTERNAL_POS_DEQUE_CAPACITY {
            raw
        } else {
            raw - INTERNAL_POS_DEQUE_CAPACITY
        }
    }

    #[inline]
    fn push_front(&mut self, cell: Cell) {
        debug_assert!(self.len < INTERNAL_POS_DEQUE_CAPACITY);
        let x = encode_internal_pos_hash(cell);
        self.head = (self.head + INTERNAL_POS_DEQUE_CAPACITY - 1) % INTERNAL_POS_DEQUE_CAPACITY;
        self.buf[self.head] = cell;
        self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(INTERNAL_POS_HASH_BASE1));
        self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(INTERNAL_POS_HASH_BASE2));
        self.len += 1;
    }

    #[inline]
    fn pop_back(&mut self) -> Option<Cell> {
        if self.is_empty() {
            return None;
        }
        let idx = self.physical_index(self.len - 1);
        let cell = self.buf[idx];
        let x = encode_internal_pos_hash(cell);
        self.hash1 = self
            .hash1
            .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW1[self.len - 1]));
        self.hash2 = self
            .hash2
            .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW2[self.len - 1]));
        self.len -= 1;
        Some(cell)
    }

    #[inline]
    fn iter(&self) -> InternalPosDequeIter<'_> {
        InternalPosDequeIter {
            deque: self,
            idx: 0,
        }
    }

    #[inline]
    fn back(&self) -> Option<Cell> {
        if self.is_empty() {
            None
        } else {
            Some(self[self.len - 1])
        }
    }

    #[inline]
    fn rolling_hash_pair(&self) -> (u64, u64) {
        (self.hash1, self.hash2)
    }
}

impl PartialEq for InternalPosDeque {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self.hash1 == other.hash1
            && self.hash2 == other.hash2
            && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for InternalPosDeque {}

impl Hash for InternalPosDeque {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        self.hash1.hash(state);
        self.hash2.hash(state);
    }
}

impl fmt::Debug for InternalPosDeque {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Index<usize> for InternalPosDeque {
    type Output = Cell;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        let physical_idx = self.physical_index(idx);
        &self.buf[physical_idx]
    }
}

struct InternalPosDequeIter<'a> {
    deque: &'a InternalPosDeque,
    idx: usize,
}

impl Iterator for InternalPosDequeIter<'_> {
    type Item = Cell;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.deque.len() {
            None
        } else {
            let cell = self.deque[self.idx];
            self.idx += 1;
            Some(cell)
        }
    }
}

const fn build_internal_color_hash_pows(base: u64) -> [u64; INTERNAL_COLOR_CAPACITY + 1] {
    let mut pows = [0_u64; INTERNAL_COLOR_CAPACITY + 1];
    pows[0] = 1;
    let mut i = 1;
    while i <= INTERNAL_COLOR_CAPACITY {
        pows[i] = pows[i - 1].wrapping_mul(base);
        i += 1;
    }
    pows
}

const INTERNAL_COLOR_HASH_POW1: [u64; INTERNAL_COLOR_CAPACITY + 1] =
    build_internal_color_hash_pows(INTERNAL_COLOR_HASH_BASE1);
const INTERNAL_COLOR_HASH_POW2: [u64; INTERNAL_COLOR_CAPACITY + 1] =
    build_internal_color_hash_pows(INTERNAL_COLOR_HASH_BASE2);

#[inline]
fn encode_internal_color_hash(color: u8) -> u64 {
    color as u64 + 1
}

#[derive(Clone)]
struct InternalColors {
    buf: [u8; INTERNAL_COLOR_CAPACITY],
    len: u16,
    hash1: u64,
    hash2: u64,
}

impl InternalColors {
    #[inline]
    fn new() -> Self {
        Self {
            buf: [0; INTERNAL_COLOR_CAPACITY],
            len: 0,
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(colors: &[u8]) -> Self {
        debug_assert!(colors.len() <= INTERNAL_COLOR_CAPACITY);
        let mut out = Self::new();
        out.buf[..colors.len()].copy_from_slice(colors);
        out.len = colors.len() as u16;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &color in colors {
            let x = encode_internal_color_hash(color);
            out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
            out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(INTERNAL_COLOR_HASH_BASE1);
            pow2 = pow2.wrapping_mul(INTERNAL_COLOR_HASH_BASE2);
        }
        out
    }

    #[inline]
    fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn push(&mut self, color: u8) {
        debug_assert!(self.len() < INTERNAL_COLOR_CAPACITY);
        let idx = self.len();
        self.buf[idx] = color;
        let x = encode_internal_color_hash(color);
        self.hash1 = self
            .hash1
            .wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW1[idx]));
        self.hash2 = self
            .hash2
            .wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW2[idx]));
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let idx = self.len() - 1;
        let color = self.buf[idx];
        let x = encode_internal_color_hash(color);
        self.hash1 = self
            .hash1
            .wrapping_sub(x.wrapping_mul(INTERNAL_COLOR_HASH_POW1[idx]));
        self.hash2 = self
            .hash2
            .wrapping_sub(x.wrapping_mul(INTERNAL_COLOR_HASH_POW2[idx]));
        self.len -= 1;
        Some(color)
    }

    #[inline]
    fn rolling_hash_pair(&self) -> (u64, u64) {
        (self.hash1, self.hash2)
    }
}

impl Deref for InternalColors {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buf[..self.len()]
    }
}

impl PartialEq for InternalColors {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self.hash1 == other.hash1
            && self.hash2 == other.hash2
            && &self[..] == &other[..]
    }
}

impl Eq for InternalColors {}

impl Hash for InternalColors {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        self.hash1.hash(state);
        self.hash2.hash(state);
    }
}

impl fmt::Debug for InternalColors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter().copied()).finish()
    }
}

#[inline]
fn find_internal_bite_idx(pos: &InternalPosDeque) -> Option<usize> {
    debug_assert!(!pos.is_empty());
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalPosOccupancy {
    cnt: [u8; INTERNAL_POS_DEQUE_CAPACITY],
}

impl InternalPosOccupancy {
    #[inline]
    fn new() -> Self {
        Self {
            cnt: [0; INTERNAL_POS_DEQUE_CAPACITY],
        }
    }

    #[inline]
    fn from_pos(pos: &InternalPosDeque) -> Self {
        let mut out = Self::new();
        for cell in pos.iter() {
            out.inc(cell);
        }
        out
    }

    #[inline]
    fn count(&self, cell: Cell) -> u8 {
        self.cnt[Grid::index(cell)]
    }

    #[inline]
    fn inc(&mut self, cell: Cell) {
        self.cnt[Grid::index(cell)] += 1;
    }

    #[inline]
    fn dec(&mut self, cell: Cell) {
        self.cnt[Grid::index(cell)] -= 1;
    }
}

// 高速化メモ:
// beam などで State::clone や state hash が支配的になった場合は、
// food / pos / colors を fixed-size 配列に寄せるとかなり速くなる可能性が高い。
// その場合はこの State 定義を起点に見直す。
#[derive(Debug, Clone)]
struct State {
    // 現在盤面の餌配置。0 は餌なし
    food: Vec<u8>,
    // 蛇の座標列。pos[0] が頭
    pos: InternalPosDeque,
    // 蛇の色列。colors[p] は pos[p] に対応する
    colors: InternalColors,
    // 各マスに何個 segment があるかを持つ占有数
    pos_occupancy: InternalPosOccupancy,
}

impl State {
    #[inline]
    fn initial(input: &Input) -> Self {
        let pos = InternalPosDeque::from_slice(&[
            Grid::cell(input.n, 4, 0),
            Grid::cell(input.n, 3, 0),
            Grid::cell(input.n, 2, 0),
            Grid::cell(input.n, 1, 0),
            Grid::cell(input.n, 0, 0),
        ]);
        Self {
            food: input.food.clone(),
            pos: pos.clone(),
            colors: InternalColors::from_slice(&[1; 5]),
            pos_occupancy: InternalPosOccupancy::from_pos(&pos),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.pos.len()
    }

    #[inline]
    fn head(&self) -> Cell {
        self.pos[0]
    }

    #[inline]
    fn tail_idx(&self) -> usize {
        self.pos.len() - 1
    }

    #[inline]
    fn tail(&self) -> Cell {
        self.pos[self.tail_idx()]
    }

    #[inline]
    fn is_legal_dir(&self, n: usize, dir: Dir) -> bool {
        if !Grid::can_move(n, self.head(), dir) {
            return false;
        }

        if self.len() >= 2 {
            let next = Grid::next_cell(n, self.head(), dir);
            if next == self.pos[1] {
                return false;
            }
        }

        true
    }

    #[inline]
    fn legal_dirs(&self, n: usize) -> Ops {
        let mut dirs = Ops::with_capacity(DIR_COUNT);
        for dir in ALL_DIRS {
            if self.is_legal_dir(n, dir) {
                dirs.push(dir);
            }
        }
        dirs
    }

    #[inline]
    fn legal_dir_count(&self, n: usize) -> usize {
        let mut count = 0;
        for dir in ALL_DIRS {
            if self.is_legal_dir(n, dir) {
                count += 1;
            }
        }
        count
    }
}

#[derive(Debug, Clone)]
struct CellSearchResult {
    // 探索の始点
    start: Cell,
    // 空マスへの最短到達手数。未到達は usize::MAX
    dist: Vec<usize>,
    // 最短路木での直前マス。始点や未到達は None
    prev: Vec<Option<Cell>>,
}

/// 初期 `pos` を固定しつつ、初期胴体マスが後ろから 1 手ごとに 1 マスずつ解放される
/// とみなした簡易 BFS を行い、空マスへの最短到達手数と復元用親ポインタを返す。
///
/// - `food` はすべて永久壁として扱う
/// - 初期胴体マスは、その Cell を占有する最前の segment が `excluded_tail`
///   になる時刻以降に通行可とみなす
///   - したがって旧 tail や `pos[len - 2]` は 1 手後に通れることがある
/// - 返り値 `dist[idx]` は head からその空マスへの最短到達手数で、未到達は `usize::MAX`
/// - `prev[idx]` はその最短路木における直前 Cell を表す
///
/// この探索は「Cell ごとの最短到着時刻」しか保持しない簡易化なので、
/// 同じ Cell に遅い時刻で再到達して release を待つ必要があるケースを落とすことがある
/// （false negative がある）。
///
/// したがって exact な到達可能性判定ではなく、軽量な到達可能性フィルタとして使う。
fn compute_body_release_dist(state: &State, n: usize) -> CellSearchResult {
    let cell_count = n * n;
    let inf = usize::MAX;

    // release[idx] = そのマスに最短で入ってよい時刻
    let mut front_idx = vec![None; cell_count];
    for j in 0..state.pos.len() {
        let cell = state.pos[j];
        let idx = Grid::index(cell);
        if front_idx[idx].is_none() {
            front_idx[idx] = Some(j);
        }
    }
    let mut release = vec![0usize; cell_count];
    for idx in 0..cell_count {
        if let Some(j) = front_idx[idx] {
            release[idx] = (state.pos.len() - 1 - j).max(1);
        }
    }

    let mut dist = vec![inf; cell_count];
    let mut prev = vec![None; cell_count];
    let mut q = VecDeque::new();

    let head = state.head();
    let head_idx = Grid::index(head);
    dist[head_idx] = 0;
    q.push_back(head);

    while let Some(cur) = q.pop_front() {
        let cur_idx = Grid::index(cur);
        let cur_dist = dist[cur_idx];

        for dir in ALL_DIRS {
            if !Grid::can_move(n, cur, dir) {
                continue;
            }

            let nxt = Grid::next_cell(n, cur, dir);
            let nxt_idx = Grid::index(nxt);
            let nd = cur_dist + 1;

            if dist[nxt_idx] != inf {
                continue;
            }
            if state.food[nxt_idx] != 0 {
                continue;
            }
            if nd < release[nxt_idx] {
                continue;
            }

            dist[nxt_idx] = nd;
            prev[nxt_idx] = Some(cur);
            q.push_back(nxt);
        }
    }

    CellSearchResult {
        start: head,
        dist,
        prev,
    }
}

/// `compute_body_release_dist` の結果を用いて、各 Cell について
/// 「その空マスへ到達する / その food を食べる」最短手数と復元用親ポインタを返す。
///
/// - 空マス `cell` については `bfs` と同じ `dist/prev` を返す
/// - food マス `cell` については、隣接 gate のうち到達可能なものから
///   `bfs.dist[gate] + 1` の最小値を `dist[cell]` に入れる
/// - 到達可能な food マス `cell` については `prev[cell]` に最後の gate を入れる
/// - 未到達は `usize::MAX` / `None`
///
/// この値も `compute_body_release_dist` と同じ簡易化に基づくため false negative がある。
fn body_release_eat_dist(bfs: &CellSearchResult, state: &State, n: usize) -> CellSearchResult {
    let inf = usize::MAX;
    let mut dist = bfs.dist.clone();
    let mut prev = bfs.prev.clone();

    for idx in 0..(n * n) {
        if state.food[idx] == 0 {
            continue;
        }

        let cell = Cell(idx as u16);
        let mut best = inf;
        let mut best_gate = None;
        for dir in ALL_DIRS {
            if !Grid::can_move(n, cell, dir) {
                continue;
            }
            let gate = Grid::next_cell(n, cell, dir);
            let gate_idx = Grid::index(gate);
            if bfs.dist[gate_idx] == inf {
                continue;
            }
            let cand = bfs.dist[gate_idx].saturating_add(1);
            if cand < best {
                best = cand;
                best_gate = Some(gate);
            }
        }
        dist[idx] = best;
        prev[idx] = best_gate;
    }

    CellSearchResult {
        start: bfs.start,
        dist,
        prev,
    }
}

/// `CellSearchResult` から `goal` までの Cell 列を復元する。
///
/// - `goal` が空マスならその空マスへの経路
/// - `goal` が food マスならその food を食べる経路
/// を同じ形式で返す。
/// 返り値は始点 `start` と終点 `goal` をともに含む。
fn reconstruct_cell_search_path(result: &CellSearchResult, goal: Cell) -> Option<Vec<Cell>> {
    let goal_idx = Grid::index(goal);
    if result.dist[goal_idx] == usize::MAX {
        return None;
    }

    let mut rev = Vec::with_capacity(result.dist[goal_idx].saturating_add(1));
    let mut cur = goal;
    loop {
        rev.push(cur);
        if cur == result.start {
            break;
        }
        cur = result.prev[Grid::index(cur)]?;
    }
    rev.reverse();
    Some(rev)
}

#[inline]
fn step(state: &State, n: usize, dir: Dir) -> StepResult {
    debug_assert!(
        state.is_legal_dir(n, dir),
        "illegal dir: dir={dir}, head={:?}",
        state.head()
    );

    let next_head = Grid::next_cell(n, state.head(), dir);

    let mut food = state.food.clone();
    let mut new_pos = state.pos.clone();
    let mut new_colors = state.colors.clone();
    let mut new_pos_occupancy = state.pos_occupancy.clone();
    let mut ate = None;

    let eat_idx = Grid::index(next_head);
    if food[eat_idx] != 0 {
        let food_color = food[eat_idx];
        food[eat_idx] = 0;
        new_colors.push(food_color);
        ate = Some(food_color);
    } else {
        let old_tail = new_pos.pop_back().unwrap();
        new_pos_occupancy.dec(old_tail);
    }

    let excluded_tail = new_pos.back();
    let tail_bias = u8::from(excluded_tail == Some(next_head));
    let bite = new_pos_occupancy.count(next_head) > tail_bias;

    new_pos_occupancy.inc(next_head);
    new_pos.push_front(next_head);
    let bite_idx = if bite {
        find_internal_bite_idx(&new_pos)
    } else {
        None
    };
    debug_assert!(
        ate.is_none() || bite_idx.is_none(),
        "eat and bite must not happen simultaneously"
    );

    let mut dropped = Vec::new();
    if let Some(h) = bite_idx {
        let drop_len = new_pos.len().saturating_sub(h + 1);
        dropped.reserve(drop_len);
        let mut dropped_rev = Vec::with_capacity(drop_len);
        while new_pos.len() > h + 1 {
            let cell = new_pos.pop_back().unwrap();
            new_pos_occupancy.dec(cell);
            let color = new_colors.pop().unwrap();
            food[Grid::index(cell)] = color;
            dropped_rev.push(Dropped { cell, color });
        }
        dropped_rev.reverse();
        dropped = dropped_rev;
    }

    StepResult {
        state: State {
            food,
            pos: new_pos,
            colors: new_colors,
            pos_occupancy: new_pos_occupancy,
        },
        ate,
        bite_idx,
        dropped,
    }
}

#[inline]
fn lcp(a: &[u8], b: &[u8]) -> usize {
    let lim = a.len().min(b.len());
    let mut i = 0;
    while i < lim && a[i] == b[i] {
        i += 1;
    }
    i
}

#[inline]
fn matches_prefix_len(a: &[u8], b: &[u8], len: usize) -> bool {
    if a.len() < len || b.len() < len {
        return false;
    }
    let mut i = 0;
    while i < len {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Debug, Clone, Copy)]
struct Dropped {
    // 噛みちぎりで盤面に落ちたマス
    cell: Cell,
    // そのマスに落ちた餌の色
    color: u8,
}

#[derive(Debug, Clone)]
struct StepResult {
    // 1 手進めた後の状態
    state: State,
    // その手で食べた色。食べていなければ None
    ate: Option<u8>,
    // 噛みちぎりが起きたときの接触先 index h
    bite_idx: Option<usize>,
    // 噛みちぎりで落ちた餌列
    dropped: Vec<Dropped>,
}

#[derive(Debug, Clone)]
struct PrefixRepairResult {
    // prefix 修復後の状態
    state: State,
    // bite 後に追加で必要だった復元操作列
    ops: Ops,
    // 実際に復元操作を行ったか。prefix が既に残っていたなら false
    repaired: bool,
}

/// 自己bite直後の状態から、保ちたい prefix を直接構成で復元する。
///
/// - `st_after` は自己bite後の state
/// - `prefix_target` は維持したい先頭一致列
/// - `dropped` はその bite で盤面に落ちた旧胴体列
///
/// `st_after.colors.len() < prefix_target.len()` のときは、`dropped` の先頭から
/// 必要個数だけを順に食べ直したのと等価な state を、`step` を回さずに直接組み立てる。
/// prefix が既に残っている場合は no-op としてそのまま返す。
///
/// 探索中に自己biteを許容しつつ、prefix を壊さずに次状態を軽量に評価したいときに使う。
fn repair_prefix_after_bite(
    st_after: &State,
    n: usize,
    prefix_target: &[u8],
    dropped: &[Dropped],
) -> PrefixRepairResult {
    let need = prefix_target.len().saturating_sub(st_after.colors.len());
    if need == 0 {
        return PrefixRepairResult {
            state: st_after.clone(),
            ops: Ops::new(),
            repaired: false,
        };
    }

    let mut food = st_after.food.clone();
    let mut pos = st_after.pos.clone();
    let mut pos_occupancy = st_after.pos_occupancy.clone();
    let mut ops = Ops::with_capacity(need);

    let mut prev = st_after.head();
    for ent in dropped.iter().take(need) {
        // 復元操作列は、現在 head 位置から次に回収すべき dropped cell への
        // 方向を並べればよい。ここでは step を回さず、隣接関係から直接求める。
        let dir = Grid::dir_between_cells(n, prev, ent.cell);
        ops.push(dir);

        // 復元後の盤面は「その dropped を食べ直した後」と等価なので、
        // その cell の food は 0 にする。
        food[Grid::index(ent.cell)] = 0;

        // 復元後の座標列は
        //   reverse(dropped[..need].cells) + st_after.pos
        // になる。したがって必要な dropped を順に push_front すればよい。
        pos.push_front(ent.cell);

        // 復元後はその cell が再び蛇の一部になるので、占有数も加算する。
        pos_occupancy.inc(ent.cell);

        prev = ent.cell;
    }

    PrefixRepairResult {
        state: State {
            food,
            pos,
            // 復元後の色列は prefix_target そのものになる。
            // ここも step を need 回回して append するのではなく、直接構成する。
            colors: InternalColors::from_slice(prefix_target),
            pos_occupancy,
        },
        ops,
        repaired: true,
    }
}

#[derive(Debug, Clone)]
struct Input {
    // 盤面サイズ N
    n: usize,
    // 目標色列の長さ M
    m: usize,
    // 色数 C
    color_count: usize,
    // 目標色列 d[0..M)
    d: Vec<u8>,
    // 初期盤面の餌配置。0 は餌なし
    food: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,

    iter: u64,
    check_mask: u64,

    elapsed_sec: f64,
    progress: f64,
    is_over: bool,
}

impl TimeKeeper {
    /// `check_interval_log2 = 8` なら 2^8 = 256 反復ごとに時計更新
    pub fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        assert!(time_limit_sec > 0.0);
        assert!(check_interval_log2 < 63);

        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };

        let mut tk = Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            elapsed_sec: 0.0,
            progress: 0.0,
            is_over: false,
        };
        tk.force_update();
        tk
    }

    /// ホットループではこれだけ呼ぶ
    /// true: 継続, false: 打ち切り
    #[inline(always)]
    pub fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    /// 明示的に時計を更新したいときに使う
    #[inline(always)]
    pub fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    /// batched な経過時間
    #[inline(always)]
    pub fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    /// batched な進捗率 [0, 1]
    #[inline(always)]
    pub fn progress(&self) -> f64 {
        self.progress
    }

    /// batched な時間切れ判定
    #[inline(always)]
    pub fn is_time_over(&self) -> bool {
        self.is_over
    }

    /// ログ用の正確な経過時間
    #[inline]
    pub fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// ログ用の正確な残り時間
    #[inline]
    pub fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let color_count: usize = it.next().unwrap().parse().unwrap();

    let mut d = vec![0_u8; m];
    for x in &mut d {
        *x = it.next().unwrap().parse::<u8>().unwrap();
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input {
        n,
        m,
        color_count,
        d,
        food,
    }
}

const INVENTORY_TIME_BUDGET_SEC: f64 = 1.86;
const GROW_BEAM_WIDTH: usize = 50;
const GROW_STAGE_SOLUTIONS: usize = 12;
const GROW_STAGE_NODE_LIMIT: usize = 20_000;
const GROW_STAGE_DEPTH_LIMIT: usize = 32;
const GROW_EXTRA_LEN_LIMIT: usize = 12;
const EXIT_NODE_LIMIT: usize = 10_000;
const EXIT_DEPTH_LIMIT: usize = 20;
const EXIT_EXTRA_LEN_LIMIT: usize = 8;
const ROUTE_NODE_LIMIT: usize = 40_000;
const ROUTE_DEPTH_LIMIT: usize = 120;
const ROUTE_EXTRA_LEN_LIMIT: usize = 12;
const INF_SCORE: usize = 1_000_000_000;

#[derive(Debug, Clone)]
struct BeamState {
    state: State,
    ops: Ops,
}

#[derive(Debug, Clone)]
struct SearchTransition {
    state: State,
    ops: Ops,
}

#[derive(Debug, Clone)]
struct SearchGraphNode {
    state: State,
    parent: Option<usize>,
    move_seg: Ops,
    depth: usize,
}

type StateSig = (usize, u64, u64, usize, u64, u64, u64);

#[inline]
fn food_hash(food: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for &x in food {
        h ^= x as u64 + 1;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

#[inline]
fn state_sig(state: &State) -> StateSig {
    let (pos_h1, pos_h2) = state.pos.rolling_hash_pair();
    let (col_h1, col_h2) = state.colors.rolling_hash_pair();
    (
        state.pos.len(),
        pos_h1,
        pos_h2,
        state.colors.len(),
        col_h1,
        col_h2,
        food_hash(&state.food),
    )
}

#[inline]
fn manhattan_cell(n: usize, a: Cell, b: Cell) -> usize {
    let (ai, aj) = Grid::ij(n, a);
    let (bi, bj) = Grid::ij(n, b);
    ai.abs_diff(bi) + aj.abs_diff(bj)
}

#[inline]
fn exact_prefix_state(state: &State, target: &[u8], len: usize) -> bool {
    matches_prefix_len(&state.colors, target, len)
}

#[inline]
fn all_positions_min_col(state: &State, n: usize, min_col: usize) -> bool {
    state.pos.iter().all(|cell| Grid::j(n, cell) >= min_col)
}

#[inline]
fn count_positions_below_col(state: &State, n: usize, min_col: usize) -> usize {
    state
        .pos
        .iter()
        .filter(|&cell| Grid::j(n, cell) < min_col)
        .count()
}

#[inline]
fn collect_food_cells_in_region(state: &State, n: usize, color: u8, min_col: usize) -> Vec<Cell> {
    let mut out = Vec::new();
    for idx in 0..(n * n) {
        if state.food[idx] == color && idx % n >= min_col {
            out.push(Cell(idx as u16));
        }
    }
    out
}

#[inline]
fn nearest_food_dist_in_region(state: &State, n: usize, color: u8, min_col: usize) -> usize {
    let head = state.head();
    let mut best = INF_SCORE;
    for idx in 0..(n * n) {
        if state.food[idx] == color && idx % n >= min_col {
            let dist = manhattan_cell(n, head, Cell(idx as u16));
            if dist < best {
                best = dist;
            }
        }
    }
    best
}

#[inline]
fn target_adjacent_in_region(state: &State, n: usize, color: u8, min_col: usize) -> bool {
    for dir in ALL_DIRS {
        if !state.is_legal_dir(n, dir) {
            continue;
        }
        let nxt = Grid::next_cell(n, state.head(), dir);
        if Grid::j(n, nxt) < min_col {
            continue;
        }
        if state.food[Grid::index(nxt)] == color {
            return true;
        }
    }
    false
}

fn apply_step_with_prefix_constraints(
    state: &State,
    n: usize,
    dir: Dir,
    prefix_target: &[u8],
    keep_len: usize,
    min_head_col: Option<usize>,
    min_drop_col: usize,
) -> Option<SearchTransition> {
    if !state.is_legal_dir(n, dir) {
        return None;
    }
    let next_head = Grid::next_cell(n, state.head(), dir);
    if let Some(min_col) = min_head_col {
        if Grid::j(n, next_head) < min_col {
            return None;
        }
    }

    let step_result = step(state, n, dir);

    for ent in &step_result.dropped {
        if Grid::j(n, ent.cell) < min_drop_col {
            return None;
        }
    }

    let mut ops = Ops::with_capacity(1 + keep_len);
    ops.push(dir);

    let next_state = if step_result.state.len() < keep_len {
        if let Some(min_col) = min_head_col {
            let need = keep_len.saturating_sub(step_result.state.len());
            for ent in step_result.dropped.iter().take(need) {
                if Grid::j(n, ent.cell) < min_col {
                    return None;
                }
            }
        }
        let repaired = repair_prefix_after_bite(
            &step_result.state,
            n,
            &prefix_target[..keep_len],
            &step_result.dropped,
        );
        ops.extend_from_slice(&repaired.ops);
        repaired.state
    } else {
        step_result.state
    };

    if !exact_prefix_state(&next_state, prefix_target, keep_len) {
        return None;
    }

    Some(SearchTransition {
        state: next_state,
        ops,
    })
}

fn append_reconstruct_search_plan(nodes: &[SearchGraphNode], mut idx: usize, out: &mut Ops) {
    let mut rev_idx = Vec::new();
    let mut total_len = 0usize;
    while let Some(parent) = nodes[idx].parent {
        rev_idx.push(idx);
        total_len += nodes[idx].move_seg.len();
        idx = parent;
    }
    out.reserve(total_len);
    for &node_idx in rev_idx.iter().rev() {
        out.extend_from_slice(&nodes[node_idx].move_seg);
    }
}

fn collect_goal_states<S, FScore, FGoal>(
    input: &Input,
    start_bs: &BeamState,
    prefix_target: &[u8],
    keep_len: usize,
    min_head_col: Option<usize>,
    min_drop_col: usize,
    extra_len_limit: usize,
    depth_limit: usize,
    node_limit: usize,
    max_solutions: usize,
    score_fn: FScore,
    goal_fn: FGoal,
) -> Vec<BeamState>
where
    S: Ord + Clone,
    FScore: Fn(&State) -> S,
    FGoal: Fn(&State) -> Option<(State, Ops)>,
{
    let start = start_bs.state.clone();
    if !exact_prefix_state(&start, prefix_target, keep_len.min(start.len())) {
        return Vec::new();
    }

    let mut nodes = Vec::with_capacity(node_limit.min(30_000) + 8);
    nodes.push(SearchGraphNode {
        state: start.clone(),
        parent: None,
        move_seg: Ops::new(),
        depth: 0,
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((score_fn(&start), 0usize, uid, 0usize)));
    uid += 1;

    let mut seen = HashMap::<StateSig, usize>::new();
    seen.insert(state_sig(&start), 0);

    let mut sols = Vec::new();
    let mut sol_seen = HashMap::<StateSig, usize>::new();
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= node_limit || sols.len() >= max_solutions {
            break;
        }
        expansions += 1;

        let st = nodes[idx].state.clone();

        if let Some((goal_state, goal_ops)) = goal_fn(&st) {
            let sig = state_sig(&goal_state);
            if !sol_seen.contains_key(&sig) {
                let mut ops = start_bs.ops.clone();
                append_reconstruct_search_plan(&nodes, idx, &mut ops);
                ops.extend_from_slice(&goal_ops);
                sols.push(BeamState {
                    state: goal_state.clone(),
                    ops,
                });
                sol_seen.insert(sig, 1);
                if sols.len() >= max_solutions {
                    break;
                }
            }
        }

        if depth >= depth_limit {
            continue;
        }

        for dir in st.legal_dirs(input.n) {
            let Some(trans) = apply_step_with_prefix_constraints(
                &st,
                input.n,
                dir,
                prefix_target,
                keep_len,
                min_head_col,
                min_drop_col,
            ) else {
                continue;
            };
            if trans.state.len() > keep_len + extra_len_limit {
                continue;
            }
            let nd = depth + trans.ops.len();
            if nd > depth_limit {
                continue;
            }
            let sig = state_sig(&trans.state);
            if seen.get(&sig).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(sig, nd);

            let child = nodes.len();
            nodes.push(SearchGraphNode {
                state: trans.state.clone(),
                parent: Some(idx),
                move_seg: trans.ops,
                depth: nd,
            });
            pq.push(Reverse((score_fn(&trans.state), nd, uid, child)));
            uid += 1;
        }
    }

    sols.sort_unstable_by_key(|bs| (score_fn(&bs.state), bs.ops.len()));
    sols
}

fn stage_growth_score(
    input: &Input,
    target: &[u8],
    ell: usize,
    min_col: usize,
    state: &State,
) -> (usize, usize, usize, usize, usize) {
    if exact_prefix_state(state, target, ell + 1) {
        return (0, 0, 0, 0, state.len().saturating_sub(ell + 1));
    }

    let next = target[ell];
    let dist = nearest_food_dist_in_region(state, input.n, next, min_col);
    let adj = usize::from(!target_adjacent_in_region(state, input.n, next, min_col));
    let food_cnt = collect_food_cells_in_region(state, input.n, next, min_col).len();
    (1, adj, dist, state.len().saturating_sub(ell), food_cnt)
}

fn next_growth_rank(
    input: &Input,
    target: &[u8],
    next_ell: usize,
    min_col: usize,
    state: &State,
) -> (usize, usize, usize) {
    if next_ell >= target.len() {
        return (0, 0, 0);
    }
    let next = target[next_ell];
    (
        nearest_food_dist_in_region(state, input.n, next, min_col),
        usize::from(!target_adjacent_in_region(state, input.n, next, min_col)),
        state.len().saturating_sub(next_ell),
    )
}

fn next_growth_rank_with_entry(
    input: &Input,
    target: &[u8],
    next_ell: usize,
    min_col: usize,
    entry_hint: Option<Cell>,
    state: &State,
) -> (usize, usize, usize, usize) {
    let base = next_growth_rank(input, target, next_ell, min_col, state);
    let entry_dist = entry_hint.map_or(0, |cell| manhattan_cell(input.n, state.head(), cell));
    (base.0, base.1, base.2, entry_dist)
}

fn growth_stage_search(
    input: &Input,
    start_bs: &BeamState,
    target: &[u8],
    ell: usize,
    min_col: usize,
) -> Vec<BeamState> {
    collect_goal_states(
        input,
        start_bs,
        &target[..(ell + 1)],
        ell,
        Some(min_col),
        min_col,
        GROW_EXTRA_LEN_LIMIT,
        GROW_STAGE_DEPTH_LIMIT,
        GROW_STAGE_NODE_LIMIT,
        GROW_STAGE_SOLUTIONS,
        |state| stage_growth_score(input, target, ell, min_col, state),
        |state| {
            if exact_prefix_state(state, target, ell + 1) {
                Some((state.clone(), Ops::new()))
            } else {
                None
            }
        },
    )
}

fn grow_to_target_prefix_restricted_beam(
    input: &Input,
    start_state: State,
    target: &[u8],
    min_col: usize,
    entry_hint: Option<Cell>,
) -> Option<Vec<BeamState>> {
    let start_match = lcp(&start_state.colors, target);
    if start_match < start_state.len().min(target.len()) {
        return None;
    }
    let start_ell = start_match.min(target.len());

    if start_ell == target.len() {
        return Some(vec![BeamState {
            state: start_state,
            ops: Ops::new(),
        }]);
    }

    let mut beam = vec![BeamState {
        state: start_state,
        ops: Ops::new(),
    }];

    for ell in start_ell..target.len() {
        let mut candidates = Vec::new();
        for bs in &beam {
            let mut sols = growth_stage_search(input, bs, target, ell, min_col);
            candidates.append(&mut sols);
        }
        if candidates.is_empty() {
            return None;
        }

        let stage_entry_hint = if ell + 4 >= target.len() {
            entry_hint
        } else {
            None
        };
        candidates.sort_unstable_by_key(|bs| {
            (
                next_growth_rank_with_entry(
                    input,
                    target,
                    ell + 1,
                    min_col,
                    stage_entry_hint,
                    &bs.state,
                ),
                bs.ops.len(),
            )
        });

        let mut next_beam = Vec::with_capacity(GROW_BEAM_WIDTH);
        let mut seen = HashMap::<StateSig, usize>::new();
        for cand in candidates {
            let sig = state_sig(&cand.state);
            if seen.contains_key(&sig) {
                continue;
            }
            seen.insert(sig, 1);
            next_beam.push(cand);
            if next_beam.len() >= GROW_BEAM_WIDTH {
                break;
            }
        }
        beam = next_beam;
    }

    beam.sort_unstable_by_key(|bs| {
        (
            entry_hint.map_or(0, |cell| manhattan_cell(input.n, bs.state.head(), cell)),
            bs.ops.len(),
            bs.state.len(),
        )
    });
    Some(beam)
}

fn deposit_ops(n: usize) -> Ops {
    let mut ops = Ops::with_capacity(2 * n + 2);
    for _ in 0..(n - 1) {
        ops.push(DIR_U);
    }
    ops.push(DIR_R);
    for _ in 0..(n - 2) {
        ops.push(DIR_D);
    }
    ops.push(DIR_D);
    ops.push(DIR_L);
    ops.push(DIR_U);
    ops.push(DIR_R);
    ops
}

fn deposit_cells_for_segment(n: usize, seg_idx: usize) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(2 * (n - 2));
    let odd_col = 2 * seg_idx + 1;
    let even_col = 2 * seg_idx;
    for r in (0..(n - 2)).rev() {
        cells.push(Grid::cell(n, r, odd_col));
    }
    for r in 0..(n - 2) {
        cells.push(Grid::cell(n, r, even_col));
    }
    cells
}

fn apply_deposit_if_valid(
    input: &Input,
    start: &State,
    seg_idx: usize,
    seg: &[u8],
    min_drop_col: usize,
) -> Option<(State, Ops)> {
    let ops = deposit_ops(input.n);
    let mut st = start.clone();

    let entry = Grid::cell(input.n, input.n - 1, 2 * seg_idx);
    if st.head() != entry {
        return None;
    }

    let target_cells = deposit_cells_for_segment(input.n, seg_idx);

    for (t, &dir) in ops.iter().enumerate() {
        if !st.is_legal_dir(input.n, dir) {
            return None;
        }
        let step_result = step(&st, input.n, dir);
        if t + 1 != ops.len() {
            if step_result.bite_idx.is_some() {
                return None;
            }
        } else {
            if step_result.bite_idx != Some(4) {
                return None;
            }
            if step_result.dropped.len() < seg.len() {
                return None;
            }
            for (idx, (&color, &cell)) in seg.iter().zip(target_cells.iter()).enumerate() {
                let ent = step_result.dropped[idx];
                if ent.color != color || ent.cell != cell {
                    return None;
                }
            }
            for ent in step_result.dropped.iter().skip(seg.len()) {
                if Grid::j(input.n, ent.cell) < min_drop_col {
                    return None;
                }
            }
            if step_result.state.len() != 5
                || !matches_prefix_len(&step_result.state.colors, &[1, 1, 1, 1, 1], 5)
            {
                return None;
            }
        }
        st = step_result.state;
    }

    Some((st, ops))
}

fn recovery_ops(n: usize, stock_cnt: usize) -> Ops {
    let mut ops = Ops::new();
    if stock_cnt == 0 {
        return ops;
    }

    ops.push(DIR_U);
    for idx in (0..stock_cnt).rev() {
        for _ in 0..(n - 3) {
            ops.push(DIR_U);
        }
        ops.push(DIR_L);
        for _ in 0..(n - 3) {
            ops.push(DIR_D);
        }
        if idx > 0 {
            ops.push(DIR_L);
        }
    }
    ops
}

fn apply_recovery_if_valid(input: &Input, start: &State, stock_cnt: usize) -> Option<(State, Ops)> {
    let ops = recovery_ops(input.n, stock_cnt);
    let mut st = start.clone();
    for &dir in &ops {
        if !st.is_legal_dir(input.n, dir) {
            return None;
        }
        let step_result = step(&st, input.n, dir);
        if step_result.bite_idx.is_some() {
            return None;
        }
        st = step_result.state;
    }
    if st.len() == input.m && exact_prefix_state(&st, &input.d, input.m) {
        Some((st, ops))
    } else {
        None
    }
}

fn exit_search_score(
    input: &Input,
    boundary: usize,
    state: &State,
) -> (usize, usize, usize, usize) {
    let bad = count_positions_below_col(state, input.n, boundary);
    let head_gap = boundary.saturating_sub(Grid::j(input.n, state.head()));
    let len_gap = state.len().abs_diff(5);
    let mobility_penalty = 4usize.saturating_sub(state.legal_dir_count(input.n));
    (bad, len_gap, head_gap, mobility_penalty)
}

fn exit_to_clean_prefix5(
    input: &Input,
    start_bs: &BeamState,
    boundary: usize,
) -> Option<BeamState> {
    let prefix = [1_u8, 1, 1, 1, 1];
    collect_goal_states(
        input,
        start_bs,
        &prefix,
        5,
        Some(boundary.saturating_sub(1)),
        boundary,
        EXIT_EXTRA_LEN_LIMIT,
        EXIT_DEPTH_LIMIT,
        EXIT_NODE_LIMIT,
        1,
        |state| exit_search_score(input, boundary, state),
        |state| {
            if state.len() == 5
                && exact_prefix_state(state, &prefix, 5)
                && all_positions_min_col(state, input.n, boundary)
            {
                Some((state.clone(), Ops::new()))
            } else {
                None
            }
        },
    )
    .into_iter()
    .next()
}

fn deposit_route_score(
    input: &Input,
    seg_idx: usize,
    keep_len: usize,
    state: &State,
) -> (usize, usize, usize, usize) {
    let entry = Grid::cell(input.n, input.n - 1, 2 * seg_idx);
    let head_dist = manhattan_cell(input.n, state.head(), entry);
    let mut neck_penalty = 0usize;
    if state.len() >= 2 && state.pos[1] == Grid::cell(input.n, input.n - 2, 2 * seg_idx) {
        neck_penalty = 1;
    }
    let (hr, hc) = Grid::ij(input.n, state.head());
    let bottom_dist = (input.n - 1).abs_diff(hr);
    (
        head_dist,
        neck_penalty,
        keep_len.abs_diff(state.len()),
        bottom_dist + hc.abs_diff(2 * seg_idx),
    )
}

fn route_to_depositable(
    input: &Input,
    start_bs: &BeamState,
    target: &[u8],
    seg_idx: usize,
    boundary: usize,
    seg: &[u8],
) -> Option<BeamState> {
    collect_goal_states(
        input,
        start_bs,
        target,
        target.len(),
        Some(boundary),
        boundary,
        ROUTE_EXTRA_LEN_LIMIT,
        ROUTE_DEPTH_LIMIT,
        ROUTE_NODE_LIMIT,
        1,
        |state| deposit_route_score(input, seg_idx, target.len(), state),
        |state| apply_deposit_if_valid(input, state, seg_idx, seg, boundary),
    )
    .into_iter()
    .next()
}

fn recovery_route_score(
    input: &Input,
    stock_cnt: usize,
    keep_len: usize,
    state: &State,
) -> (usize, usize, usize, usize) {
    if stock_cnt == 0 {
        return (0, 0, 0, 0);
    }
    let hi = Grid::cell(input.n, input.n - 2, 2 * (stock_cnt - 1) + 1);
    let dist = manhattan_cell(input.n, state.head(), hi);
    let (hr, _) = Grid::ij(input.n, state.head());
    (dist, keep_len.abs_diff(state.len()), hr, 0)
}

fn route_to_recoverable(
    input: &Input,
    start_bs: &BeamState,
    target: &[u8],
    stock_cnt: usize,
    boundary: usize,
) -> Option<BeamState> {
    collect_goal_states(
        input,
        start_bs,
        target,
        target.len(),
        Some(boundary),
        boundary,
        ROUTE_EXTRA_LEN_LIMIT,
        ROUTE_DEPTH_LIMIT,
        ROUTE_NODE_LIMIT,
        1,
        |state| recovery_route_score(input, stock_cnt, target.len(), state),
        |state| apply_recovery_if_valid(input, state, stock_cnt),
    )
    .into_iter()
    .next()
}

fn split_inventory_segments(input: &Input) -> (usize, Vec<Vec<u8>>, Vec<u8>) {
    let seg_len = 2 * (input.n - 2);
    let stock_cnt = (input.m - 5) / seg_len;
    let mut segments = Vec::with_capacity(stock_cnt);
    for i in 0..stock_cnt {
        let l = input.m - (i + 1) * seg_len;
        let r = input.m - i * seg_len;
        segments.push(input.d[l..r].to_vec());
    }
    let lead_right = input.m - stock_cnt * seg_len;
    let lead = input.d[5..lead_right].to_vec();
    (seg_len, segments, lead)
}

fn build_prefix_target(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(prefix.len() + suffix.len());
    out.extend_from_slice(prefix);
    out.extend_from_slice(suffix);
    out
}

fn validate_exact_solution(input: &Input, ops: &[Dir]) -> bool {
    let mut st = State::initial(input);
    for &dir in ops {
        if !st.is_legal_dir(input.n, dir) {
            return false;
        }
        let step_result = step(&st, input.n, dir);
        st = step_result.state;
    }
    st.len() == input.m && exact_prefix_state(&st, &input.d, input.m)
}

fn try_inventory_solve(input: &Input, timer: &TimeKeeper) -> Option<Ops> {
    let (_, segments, lead) = split_inventory_segments(input);
    let stock_cnt = segments.len();
    if stock_cnt == 0 {
        return None;
    }
    if 2 * stock_cnt + 2 > input.n {
        return None;
    }
    if timer.exact_elapsed_sec() > INVENTORY_TIME_BUDGET_SEC {
        return None;
    }

    let ones5 = [1_u8, 1, 1, 1, 1];

    let mut current = BeamState {
        state: State::initial(input),
        ops: Ops::new(),
    };

    for seg_idx in 0..stock_cnt {
        if timer.exact_elapsed_sec() > INVENTORY_TIME_BUDGET_SEC {
            return None;
        }

        let boundary = 2 * seg_idx;
        if seg_idx > 0 {
            current = exit_to_clean_prefix5(input, &current, boundary)?;
        }

        let target = build_prefix_target(&ones5, &segments[seg_idx]);
        let entry = Grid::cell(input.n, input.n - 1, 2 * seg_idx);
        let grown_beam = grow_to_target_prefix_restricted_beam(
            input,
            current.state.clone(),
            &target,
            boundary,
            Some(entry),
        )?;

        let mut next_current = None;
        for grown in grown_beam {
            let mut grown_bs = BeamState {
                state: grown.state,
                ops: {
                    let mut ops = current.ops.clone();
                    ops.extend_from_slice(&grown.ops);
                    ops
                },
            };

            if let Some(done) = apply_deposit_if_valid(
                input,
                &grown_bs.state,
                seg_idx,
                &segments[seg_idx],
                boundary,
            ) {
                grown_bs.state = done.0;
                grown_bs.ops.extend_from_slice(&done.1);
                next_current = Some(grown_bs);
                break;
            }

            if let Some(done) = route_to_depositable(
                input,
                &grown_bs,
                &target,
                seg_idx,
                boundary,
                &segments[seg_idx],
            ) {
                next_current = Some(done);
                break;
            }
        }
        current = next_current?;
    }

    if timer.exact_elapsed_sec() > INVENTORY_TIME_BUDGET_SEC {
        return None;
    }

    if lead.is_empty() {
        let final_bs = if let Some(done) = apply_recovery_if_valid(input, &current.state, stock_cnt)
        {
            let mut ops = current.ops.clone();
            ops.extend_from_slice(&done.1);
            BeamState { state: done.0, ops }
        } else {
            route_to_recoverable(input, &current, &ones5, stock_cnt, 2 * (stock_cnt - 1))?
        };
        return validate_exact_solution(input, &final_bs.ops).then_some(final_bs.ops);
    }

    let lead_boundary = 2 * stock_cnt;
    let staged = exit_to_clean_prefix5(input, &current, lead_boundary)?;
    let lead_target = build_prefix_target(&ones5, &lead);
    let hi_last = Grid::cell(input.n, input.n - 2, 2 * (stock_cnt - 1) + 1);
    let grown_lead_beam = grow_to_target_prefix_restricted_beam(
        input,
        staged.state.clone(),
        &lead_target,
        lead_boundary,
        Some(hi_last),
    )?;

    for grown_lead in grown_lead_beam {
        let grown_lead_bs = BeamState {
            state: grown_lead.state,
            ops: {
                let mut ops = staged.ops.clone();
                ops.extend_from_slice(&grown_lead.ops);
                ops
            },
        };

        if let Some(final_bs) = route_to_recoverable(
            input,
            &grown_lead_bs,
            &lead_target,
            stock_cnt,
            2 * (stock_cnt - 1),
        ) {
            if validate_exact_solution(input, &final_bs.ops) {
                return Some(final_bs.ops);
            }
        }
    }

    None
}

fn solve_naive(input: &Input) -> Ops {
    let mut st = State::initial(input);
    let mut ops = Ops::new();
    let mut ell = 5usize;

    while ell < input.m && ops.len() < MAX_TURNS {
        let target = input.d[ell];
        let bfs = compute_body_release_dist(&st, input.n);
        let eat = body_release_eat_dist(&bfs, &st, input.n);

        let mut best_cell = None;
        let mut best_dist = usize::MAX;
        for idx in 0..(input.n * input.n) {
            if st.food[idx] != target {
                continue;
            }
            if eat.dist[idx] < best_dist {
                best_dist = eat.dist[idx];
                best_cell = Some(Cell(idx as u16));
            }
        }
        let Some(goal) = best_cell else {
            break;
        };
        let Some(path) = reconstruct_cell_search_path(&eat, goal) else {
            break;
        };
        let mut ok = true;
        for w in path.windows(2) {
            let dir = Grid::dir_between_cells(input.n, w[0], w[1]);
            if !st.is_legal_dir(input.n, dir) {
                ok = false;
                break;
            }
            let step_result = step(&st, input.n, dir);
            if step_result.bite_idx.is_some() {
                ok = false;
                break;
            }
            st = step_result.state;
            ops.push(dir);
            if ops.len() >= MAX_TURNS {
                break;
            }
        }
        if !ok || st.len() <= ell || st.colors[ell] != target {
            break;
        }
        ell += 1;
    }

    ops
}

fn solve(input: &Input) -> Ops {
    let timer = TimeKeeper::new(1.90, 8);

    if let Some(ans) = try_inventory_solve(input, &timer) {
        if ans.len() <= MAX_TURNS {
            return ans;
        }
    }

    let naive = solve_naive(input);
    if naive.len() <= MAX_TURNS && validate_exact_solution(input, &naive) {
        return naive;
    }

    naive
}

fn main() {
    let input = read_input();
    let ans = solve(&input);

    let mut out = String::new();
    for dir in ans {
        out.push(DIR_CHARS[dir as usize]);
        out.push('\n');
    }
    print!("{out}");
}
