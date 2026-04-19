// v000_template.rs
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::collections::VecDeque;
use std::ops::Index;
use std::ops::Deref;
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
        self.hash1 = self.hash1.wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW2[idx]));
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

fn main() {
    let input = read_input();
    let _state = State::initial(&input);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use std::collections::VecDeque;

    fn make_state(n: usize, food: Vec<u8>, pos_ij: &[(usize, usize)], colors: &[u8]) -> State {
        let pos = pos_ij
            .iter()
            .map(|&(i, j)| Grid::cell(n, i, j))
            .collect::<Vec<_>>();
        State {
            food,
            pos: InternalPosDeque::from_slice(&pos),
            colors: InternalColors::from_slice(colors),
            pos_occupancy: InternalPosOccupancy::from_pos(&InternalPosDeque::from_slice(&pos)),
        }
    }

    fn calc_internal_pos_hash_pair(cells: &[Cell]) -> (u64, u64) {
        let mut hash1 = 0_u64;
        let mut hash2 = 0_u64;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &cell in cells {
            let x = encode_internal_pos_hash(cell);
            hash1 = hash1.wrapping_add(x.wrapping_mul(pow1));
            hash2 = hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(INTERNAL_POS_HASH_BASE1);
            pow2 = pow2.wrapping_mul(INTERNAL_POS_HASH_BASE2);
        }
        (hash1, hash2)
    }

    fn calc_internal_color_hash_pair(colors: &[u8]) -> (u64, u64) {
        let mut hash1 = 0_u64;
        let mut hash2 = 0_u64;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &color in colors {
            let x = encode_internal_color_hash(color);
            hash1 = hash1.wrapping_add(x.wrapping_mul(pow1));
            hash2 = hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(INTERNAL_COLOR_HASH_BASE1);
            pow2 = pow2.wrapping_mul(INTERNAL_COLOR_HASH_BASE2);
        }
        (hash1, hash2)
    }

    #[test]
    fn lcp_works_for_vec_and_internal_colors() {
        assert_eq!(lcp(&[1, 1, 2, 3], &[1, 1, 2, 4, 5]), 3);
        assert_eq!(lcp(&[1, 2], &[1, 2]), 2);
        assert_eq!(lcp(&[1, 2], &[2, 1]), 0);
        assert_eq!(lcp(&[] as &[u8], &[1, 2, 3]), 0);

        let colors = InternalColors::from_slice(&[1, 1, 2, 5]);
        let target = vec![1, 1, 2, 4, 3];
        assert_eq!(lcp(&colors, &target), 3);
    }

    #[test]
    fn matches_prefix_len_works_for_vec_and_internal_colors() {
        assert!(matches_prefix_len(&[1, 1, 2, 3], &[1, 1, 2, 4, 5], 3));
        assert!(!matches_prefix_len(&[1, 1, 2, 3], &[1, 1, 2, 4, 5], 4));
        assert!(matches_prefix_len(&[1, 2], &[1, 2], 2));
        assert!(!matches_prefix_len(&[1, 2], &[1, 2], 3));

        let colors = InternalColors::from_slice(&[1, 1, 2, 5]);
        let target = vec![1, 1, 2, 4, 3];
        assert!(matches_prefix_len(&colors, &target, 3));
        assert!(!matches_prefix_len(&colors, &target, 4));
    }

    fn assert_state_consistent(state: &State) {
        assert_eq!(state.pos.len(), state.colors.len());
        let cells = state.pos.iter().collect::<Vec<_>>();
        assert_eq!(
            state.pos.rolling_hash_pair(),
            calc_internal_pos_hash_pair(&cells)
        );
        assert_eq!(
            state.colors.rolling_hash_pair(),
            calc_internal_color_hash_pair(&state.colors)
        );
        let mut cnt = [0_u8; INTERNAL_POS_DEQUE_CAPACITY];
        for &cell in &cells {
            cnt[Grid::index(cell)] += 1;
        }
        assert_eq!(state.pos_occupancy.cnt, cnt);
    }

    fn assert_step_result_eq(actual: &StepResult, expected: &StepResult) {
        assert_eq!(actual.ate, expected.ate);
        assert_eq!(actual.bite_idx, expected.bite_idx);
        assert_eq!(actual.dropped.len(), expected.dropped.len());
        for (a, e) in actual.dropped.iter().zip(expected.dropped.iter()) {
            assert_eq!(a.cell, e.cell);
            assert_eq!(a.color, e.color);
        }
        assert_eq!(actual.state.food, expected.state.food);
        assert_eq!(actual.state.colors, expected.state.colors);
        assert_eq!(actual.state.pos, expected.state.pos);
        assert_eq!(actual.state.pos_occupancy, expected.state.pos_occupancy);
        assert_state_consistent(&actual.state);
    }

    fn reference_step(state: &State, n: usize, dir: Dir) -> StepResult {
        assert!(state.is_legal_dir(n, dir));

        let next_head = Grid::next_cell(n, state.head(), dir);

        let mut food = state.food.clone();
        let mut pos = state.pos.iter().collect::<VecDeque<_>>();
        let mut colors = state.colors.clone();
        let mut ate = None;

        let eat_idx = Grid::index(next_head);
        if food[eat_idx] != 0 {
            let color = food[eat_idx];
            food[eat_idx] = 0;
            colors.push(color);
            ate = Some(color);
        } else {
            pos.pop_back();
        }
        pos.push_front(next_head);

        let mut bite_idx = None;
        for idx in 1..pos.len().saturating_sub(1) {
            if pos[idx] == next_head {
                bite_idx = Some(idx);
                break;
            }
        }

        let mut dropped = Vec::new();
        if let Some(h) = bite_idx {
            let mut dropped_rev = Vec::new();
            while pos.len() > h + 1 {
                let cell = pos.pop_back().unwrap();
                let color = colors.pop().unwrap();
                food[Grid::index(cell)] = color;
                dropped_rev.push(Dropped { cell, color });
            }
            dropped_rev.reverse();
            dropped = dropped_rev;
        }

        let pos = pos.iter().copied().collect::<Vec<_>>();
        StepResult {
            state: State {
                food,
                pos: InternalPosDeque::from_slice(&pos),
                colors,
                pos_occupancy: InternalPosOccupancy::from_pos(&InternalPosDeque::from_slice(&pos)),
            },
            ate,
            bite_idx,
            dropped,
        }
    }

    #[test]
    fn internal_pos_deque_push_front_and_pop_back() {
        let mut pos = InternalPosDeque::from_slice(&[
            Grid::cell(8, 4, 0),
            Grid::cell(8, 3, 0),
            Grid::cell(8, 2, 0),
        ]);
        assert_eq!(
            pos.rolling_hash_pair(),
            calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
        );
        pos.push_front(Grid::cell(8, 5, 0));
        assert_eq!(
            pos.iter().collect::<Vec<_>>(),
            vec![
                Grid::cell(8, 5, 0),
                Grid::cell(8, 4, 0),
                Grid::cell(8, 3, 0),
                Grid::cell(8, 2, 0)
            ]
        );
        assert_eq!(
            pos.rolling_hash_pair(),
            calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
        );
        assert_eq!(pos.pop_back(), Some(Grid::cell(8, 2, 0)));
        assert_eq!(
            pos.iter().collect::<Vec<_>>(),
            vec![
                Grid::cell(8, 5, 0),
                Grid::cell(8, 4, 0),
                Grid::cell(8, 3, 0)
            ]
        );
        assert_eq!(
            pos.rolling_hash_pair(),
            calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
        );
    }

    #[test]
    fn grid_methods_work() {
        let n = 8;
        let center = Grid::cell(n, 3, 4);
        assert!(Grid::can_move(n, center, 0));
        assert!(Grid::can_move(n, center, 1));
        assert!(Grid::can_move(n, center, 2));
        assert!(Grid::can_move(n, center, 3));
        assert!(!Grid::can_move(n, Grid::cell(n, 0, 0), 0));
        assert!(!Grid::can_move(n, Grid::cell(n, 0, 0), 2));

        assert_eq!(Grid::next_cell(n, center, 0), Grid::cell(n, 2, 4));
        assert_eq!(Grid::next_cell(n, center, 1), Grid::cell(n, 4, 4));
        assert_eq!(Grid::next_cell(n, center, 2), Grid::cell(n, 3, 3));
        assert_eq!(Grid::next_cell(n, center, 3), Grid::cell(n, 3, 5));

        assert_eq!(Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 2, 4)), 0);
        assert_eq!(Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 4, 4)), 1);
        assert_eq!(Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 3, 3)), 2);
        assert_eq!(Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 3, 5)), 3);
    }

    #[test]
    fn manhattan_table_works() {
        let n = 8;
        let table = ManhattanTable::new(n);
        let a = Grid::cell(n, 1, 2);
        let b = Grid::cell(n, 4, 6);
        let c = Grid::cell(n, 1, 2);
        assert_eq!(table.get(a, b), 7);
        assert_eq!(table.get(b, a), 7);
        assert_eq!(table.get(a, c), 0);
    }

    #[test]
    fn step_matches_reference_without_eat() {
        let n = 8;
        let input = Input {
            n,
            m: 5,
            color_count: 3,
            d: vec![1; 5],
            food: vec![0; n * n],
        };
        let state = State::initial(&input);
        let actual = step(&state, n, 3);
        let expected = reference_step(&state, n, 3);
        assert_step_result_eq(&actual, &expected);
    }

    #[test]
    fn step_matches_reference_with_eat() {
        let n = 8;
        let mut food = vec![0; n * n];
        food[Grid::index(Grid::cell(n, 4, 1))] = 2;
        let input = Input {
            n,
            m: 6,
            color_count: 3,
            d: vec![1; 6],
            food,
        };
        let state = State::initial(&input);
        let actual = step(&state, n, 3);
        let expected = reference_step(&state, n, 3);
        assert_step_result_eq(&actual, &expected);
        assert_eq!(actual.state.len(), 6);
        assert_eq!(actual.ate, Some(2));
    }

    #[test]
    fn step_matches_reference_with_bite() {
        let n = 6;
        let food = vec![0; n * n];
        let pos_ij = [
            (2, 2),
            (2, 1),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 3),
            (3, 3),
            (3, 2),
            (3, 1),
        ];
        let colors = [1, 2, 3, 4, 5, 6, 7, 1, 2];
        let state = make_state(n, food, &pos_ij, &colors);
        let actual = step(&state, n, 3);
        let expected = reference_step(&state, n, 3);
        assert_step_result_eq(&actual, &expected);
        assert_eq!(actual.bite_idx, Some(6));
        assert_eq!(actual.dropped.len(), 2);
    }

    #[test]
    fn repair_prefix_after_bite_is_noop_when_prefix_remains() {
        let n = 6;
        let food = vec![0; n * n];
        let pos_ij = [
            (2, 2),
            (2, 1),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 3),
            (3, 3),
            (3, 2),
            (3, 1),
        ];
        let colors = [1, 2, 3, 4, 5, 6, 7, 1, 2];
        let state = make_state(n, food, &pos_ij, &colors);
        let bite = step(&state, n, 3);

        let prefix_target = &state.colors[..6];
        let repaired = repair_prefix_after_bite(&bite.state, n, prefix_target, &bite.dropped);

        assert!(!repaired.repaired);
        assert!(repaired.ops.is_empty());
        assert_eq!(repaired.state.food, bite.state.food);
        assert_eq!(repaired.state.pos, bite.state.pos);
        assert_eq!(repaired.state.colors, bite.state.colors);
        assert_eq!(repaired.state.pos_occupancy, bite.state.pos_occupancy);
    }

    #[test]
    fn repair_prefix_after_bite_matches_step_replay() {
        let n = 6;
        let food = vec![0; n * n];
        let pos_ij = [
            (2, 2),
            (2, 1),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 3),
            (3, 3),
            (3, 2),
            (3, 1),
        ];
        let colors = [1, 2, 3, 4, 5, 6, 7, 1, 2];
        let state = make_state(n, food, &pos_ij, &colors);
        let bite = step(&state, n, 3);

        let prefix_target = &state.colors[..state.colors.len()];
        let actual = repair_prefix_after_bite(&bite.state, n, prefix_target, &bite.dropped);

        let mut expected_state = bite.state.clone();
        let mut expected_ops = Vec::new();
        for ent in bite
            .dropped
            .iter()
            .take(prefix_target.len() - bite.state.colors.len())
        {
            let dir = Grid::dir_between_cells(n, expected_state.head(), ent.cell);
            expected_ops.push(dir);
            expected_state = step(&expected_state, n, dir).state;
        }

        assert!(actual.repaired);
        assert_eq!(actual.ops, expected_ops);
        assert_eq!(actual.state.food, expected_state.food);
        assert_eq!(actual.state.pos, expected_state.pos);
        assert_eq!(actual.state.colors, expected_state.colors);
        assert_eq!(actual.state.pos_occupancy, expected_state.pos_occupancy);
        assert!(matches_prefix_len(
            &actual.state.colors,
            prefix_target,
            prefix_target.len()
        ));
    }

    #[test]
    fn body_release_dist_allows_late_opening_initial_body_cells() {
        let n = 6;
        let input = Input {
            n,
            m: 5,
            color_count: 3,
            d: vec![1; 5],
            food: vec![0; n * n],
        };
        let state = State::initial(&input);
        let bfs = compute_body_release_dist(&state, n);

        assert_eq!(bfs.start, Grid::cell(n, 4, 0));
        assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 2, 0))], 4);
        assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 3, 0))], 3);
        assert_eq!(
            bfs.prev[Grid::index(Grid::cell(n, 2, 0))],
            Some(Grid::cell(n, 2, 1))
        );
    }

    #[test]
    fn body_release_eat_dist_turns_adjacent_food_into_one_step_goal() {
        let n = 8;
        let mut food = vec![0; n * n];
        food[Grid::index(Grid::cell(n, 4, 1))] = 2;
        let input = Input {
            n,
            m: 6,
            color_count: 3,
            d: vec![1; 6],
            food,
        };
        let state = State::initial(&input);
        let bfs = compute_body_release_dist(&state, n);
        let eat_result = body_release_eat_dist(&bfs, &state, n);

        assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 4, 1))], usize::MAX);
        assert_eq!(eat_result.dist[Grid::index(Grid::cell(n, 4, 1))], 1);
        assert_eq!(
            eat_result.prev[Grid::index(Grid::cell(n, 4, 1))],
            Some(Grid::cell(n, 4, 0))
        );
        assert_eq!(
            reconstruct_cell_search_path(&eat_result, Grid::cell(n, 4, 1)),
            Some(vec![Grid::cell(n, 4, 0), Grid::cell(n, 4, 1)])
        );
    }

    #[test]
    fn random_walk_matches_reference() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
        let n = 8;
        let mut food = vec![0; n * n];
        for i in 0..n {
            for j in 0..n {
                if j == 0 && i <= 4 {
                    continue;
                }
                if rng.random_range(0..100) < 25 {
                    food[Grid::index(Grid::cell(n, i, j))] = rng.random_range(1..=3);
                }
            }
        }
        let input = Input {
            n,
            m: 40,
            color_count: 3,
            d: vec![1; 40],
            food,
        };
        let mut state = State::initial(&input);
        assert_state_consistent(&state);

        for _turn in 0..300 {
            let dirs = state.legal_dirs(n);
            let dir = dirs[rng.random_range(0..dirs.len())];
            let actual = step(&state, n, dir);
            let expected = reference_step(&state, n, dir);
            assert_eq!(
                actual.ate, expected.ate,
                "ate mismatch: dir={dir}, state={state:?}, actual={actual:?}, expected={expected:?}"
            );
            assert_eq!(
                actual.bite_idx, expected.bite_idx,
                "bite_idx mismatch: dir={dir}, state={state:?}, actual={actual:?}, expected={expected:?}"
            );
            assert_step_result_eq(&actual, &expected);
            state = actual.state;
        }
    }
}
