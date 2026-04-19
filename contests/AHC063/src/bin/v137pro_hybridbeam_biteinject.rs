// v137pro_hybridbeam_biteinject.rs
// メモ:
// - 一時的に誤食してから自分で断ち切り、順番通りに食べ直す高度なテクが入っている。
//   そのおかげか case0022 は結構強い。
// - ヘビが長くなったときには弱い。途中で運び屋戦略のような中継手を挟む余地がある。
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::io::{self, Read};
use std::time::Instant;

const MAX_N: usize = 16;
const MAX_CELLS: usize = MAX_N * MAX_N;
const MAX_LEN: usize = MAX_CELLS;
const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.88;
const STAGE_BEAM: usize = 5;
const MAX_TARGETS_PER_STAGE: usize = 10;
const MAX_TARGETS_ENDGAME: usize = 24;
const MAX_TARGETS_RESCUE: usize = 28;
const VISIT_REPEAT_LIMIT: usize = 12;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const DIR_CHARS: [char; 4] = ['U', 'D', 'L', 'R'];
const ALL_DIRS: [Dir; 4] = [0, 1, 2, 3];
const BUDGETS_NORMAL: [(usize, usize); 3] = [(2_000, 20), (8_000, 20), (25_000, 24)];
const BUDGETS_LATE: [(usize, usize); 3] = [(4_000, 20), (12_000, 24), (40_000, 28)];
const BUDGETS_ENDGAME_LIGHT: [(usize, usize); 2] = [(800, 16), (2_500, 20)];
const BUDGETS_RESCUE: [(usize, usize); 2] = [(16_000, 24), (60_000, 32)];
const ENDGAME_REMAINING_FOOD: usize = 18;
const ENDGAME_ELL_LEFT: usize = 24;
const LOOKAHEAD_HORIZON: usize = 6;
const SUFFIX_OPT_WINDOWS: [usize; 4] = [8, 12, 16, 20];
const SUFFIX_STAGE_BEAM: usize = 5;
const SUFFIX_OPT_TARGETS: usize = 12;
const SUFFIX_OPT_MIN_LEFT_SEC: f64 = 0.18;
const EMPTY_PATH_DEPTH_LIMIT: usize = 64;
const EMPTY_PATH_EXPANSION_CAP: usize = 140_000;
const EMPTY_PATH_REMAINING_LIMIT: usize = 12;
const EMPTY_PATH_MIN_LEFT_SEC: f64 = 0.10;
const FASTLANE_MIN_LEFT_SEC: f64 = 0.28;
const FAST_SAFE_DEPTH_LIMIT: u8 = 10;
const FAST_SAFE_NODE_LIMIT: usize = 2_500;
const FAST_RESCUE_DEPTH_LIMIT: u8 = 16;
const FAST_RESCUE_NODE_LIMIT: usize = 9_000;
const FAST_FALLBACK_TARGETS: usize = 8;
const SIMPLE_STAGE_BEAM: usize = 48;
const SIMPLE_CANDIDATE_WIDTH: usize = 6;
const SIMPLE_INJECT_PER_STATE: usize = 4;
// packed local path は u16 に 2bit/手で詰めるので、BITE_DEPTH_LIMIT は 8 以下に保つこと。
const BITE_DEPTH_LIMIT: usize = 6;
const BITE_CANDIDATE_WIDTH: usize = 4;
const SIMPLE_SUFFIX_WINDOWS: [usize; 4] = [8, 12, 16, 24];
const SIMPLE_SUFFIX_MIN_LEFT_SEC: f64 = 0.12;

type Cell = u16;
type Dir = u8;
type Ops = Vec<Dir>;

type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
type FxHashSet<T> = HashSet<T, BuildHasherDefault<FxHasher>>;

#[derive(Default)]
struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn mix(&mut self, x: u64) {
        self.hash ^= x;
        self.hash = self.hash.rotate_left(5).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut idx = 0usize;
        while idx + 8 <= bytes.len() {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[idx..idx + 8]);
            self.mix(u64::from_le_bytes(chunk));
            idx += 8;
        }
        if idx < bytes.len() {
            let mut tail = [0u8; 8];
            tail[..bytes.len() - idx].copy_from_slice(&bytes[idx..]);
            self.mix(u64::from_le_bytes(tail));
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.mix(i as u64);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.mix(i as u64);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.mix(i as u64);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.mix(i);
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.mix(i as u64);
        self.mix((i >> 64) as u64);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.mix(i as u64);
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.mix(i as i64 as u64);
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.mix(i as i64 as u64);
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.mix(i as i64 as u64);
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.mix(i as u64);
    }

    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.mix(i as u64);
        self.mix((i >> 64) as u64);
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.mix(i as i64 as u64);
    }
}

const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

const INTERNAL_POS_HASH_BASE1: u64 = 0x94d0_49bb_1331_11eb;
const INTERNAL_POS_HASH_BASE2: u64 = 0xbf58_476d_1ce4_e5b9;
const COLOR_HASH_BASE1: u64 = 0x9e37_79b9_7f4a_7c15;
const COLOR_HASH_BASE2: u64 = 0x517c_c1b7_2722_0a95;

const fn build_pow(base: u64) -> [u64; MAX_LEN + 1] {
    let mut out = [0_u64; MAX_LEN + 1];
    let mut idx = 0usize;
    let mut cur = 1_u64;
    while idx <= MAX_LEN {
        out[idx] = cur;
        cur = cur.wrapping_mul(base);
        idx += 1;
    }
    out
}

const fn build_food_hash_table() -> [[u64; 8]; MAX_CELLS] {
    let mut out = [[0_u64; 8]; MAX_CELLS];
    let mut cell = 0usize;
    while cell < MAX_CELLS {
        let mut color = 1usize;
        while color < 8 {
            out[cell][color] =
                splitmix64(((cell as u64) << 8) ^ color as u64 ^ 0x1234_5678_9abc_def0);
            color += 1;
        }
        cell += 1;
    }
    out
}

const INTERNAL_POS_HASH_POW1: [u64; MAX_LEN + 1] = build_pow(INTERNAL_POS_HASH_BASE1);
const INTERNAL_POS_HASH_POW2: [u64; MAX_LEN + 1] = build_pow(INTERNAL_POS_HASH_BASE2);
const COLOR_HASH_POW1: [u64; MAX_LEN + 1] = build_pow(COLOR_HASH_BASE1);
const COLOR_HASH_POW2: [u64; MAX_LEN + 1] = build_pow(COLOR_HASH_BASE2);
const FOOD_HASH_TABLE: [[u64; 8]; MAX_CELLS] = build_food_hash_table();

#[inline]
const fn encode_internal_pos_hash(cell: Cell) -> u64 {
    splitmix64(cell as u64 ^ 0x243f_6a88_85a3_08d3)
}

#[inline]
const fn encode_color_hash(color: u8) -> u64 {
    splitmix64(color as u64 ^ 0x1319_8a2e_0370_7344)
}

#[derive(Clone)]
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
        self.dist[a as usize * self.cell_count + b as usize] as usize
    }
}

#[derive(Clone)]
struct Input {
    n: usize,
    m: usize,
    d: [u8; MAX_LEN],
    food: [u8; MAX_CELLS],
    manhattan: ManhattanTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalPosOccupancy {
    cnt: [u8; MAX_CELLS],
}

impl InternalPosOccupancy {
    #[inline]
    fn new() -> Self {
        Self {
            cnt: [0; MAX_CELLS],
        }
    }

    #[inline]
    fn from_pos(pos: &InternalPosDeque) -> Self {
        let mut out = Self::new();
        for idx in 0..pos.len() {
            out.inc(pos[idx]);
        }
        out
    }

    #[inline]
    fn count(&self, cell: Cell) -> u8 {
        self.cnt[cell as usize]
    }

    #[inline]
    fn inc(&mut self, cell: Cell) {
        self.cnt[cell as usize] += 1;
    }

    #[inline]
    fn dec(&mut self, cell: Cell) {
        self.cnt[cell as usize] -= 1;
    }
}

#[derive(Clone)]
struct State {
    n: usize,
    food: [u8; MAX_CELLS],
    remaining_food: usize,
    food_hash: u64,
    pos: InternalPosDeque,
    colors: [u8; MAX_LEN],
    color_hash1: u64,
    color_hash2: u64,
    pos_occupancy: InternalPosOccupancy,
}

#[derive(Clone)]
struct BeamState {
    state: State,
    ops: Ops,
}

#[derive(Clone)]
struct LocalBiteNode {
    state: State,
    // packed local path: u16 に 2bit/手で格納するので最大 8 手まで。深さを増やすなら型幅も広げる。
    path_bits: u16,
    depth: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct LocalVisitedKey {
    pos_len: u16,
    pos_hash1: u64,
    pos_hash2: u64,
    colors_len: u16,
    colors_hash1: u64,
    colors_hash2: u64,
}

#[derive(Clone)]
struct CellSearchResult {
    start: Cell,
    dist: [u16; MAX_CELLS],
    prev: [Cell; MAX_CELLS],
}

#[derive(Clone, Copy, Default)]
struct Dropped {
    cell: Cell,
    color: u8,
}

struct PrefixRepairResult {
    state: State,
    ops: Ops,
    repaired: bool,
}

#[derive(Clone)]
struct DroppedBuf {
    len: usize,
    entries: [Dropped; MAX_LEN],
}

#[derive(Clone)]
struct DroppedQueue {
    head: usize,
    len: usize,
    buf: [Dropped; MAX_LEN],
}

#[derive(Hash, Eq, PartialEq)]
struct VisitKey {
    head: Cell,
    neck: Cell,
    len: u16,
    goal: Cell,
    restore_len: u16,
}

struct Node {
    state: State,
    parent: Option<usize>,
    move_seg: Ops,
}

struct PosNode {
    pos: InternalPosDeque,
    parent: Option<usize>,
    mv: u8,
}

#[derive(Clone, Copy)]
struct QuickPlanConfig {
    depth_limit: u8,
    node_limit: usize,
    non_target_limit: u8,
    bite_limit: u8,
}

#[derive(Clone)]
struct QuickSearchNode {
    state: State,
    parent: usize,
    dir: u8,
    depth: u8,
    non_target: u8,
    bite: u8,
}

#[derive(Clone)]
struct QuickSeenKey {
    state: State,
    non_target: u8,
    bite: u8,
}

#[derive(Clone)]
struct CellList {
    len: usize,
    buf: [Cell; MAX_CELLS],
}

#[derive(Clone, Copy)]
struct SmallDirList {
    len: usize,
    dirs: [u8; 4],
}

#[derive(Clone, Copy)]
struct SmallCellList {
    len: usize,
    cells: [Cell; 4],
}

#[derive(Clone)]
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [Cell; MAX_LEN],
    hash1: u64,
    hash2: u64,
}

impl DroppedBuf {
    #[inline]
    fn new() -> Self {
        Self {
            len: 0,
            entries: [Dropped::default(); MAX_LEN],
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    fn push(&mut self, cell: Cell, color: u8) {
        self.entries[self.len] = Dropped { cell, color };
        self.len += 1;
    }

    #[inline]
    fn as_slice(&self) -> &[Dropped] {
        &self.entries[..self.len]
    }
}

impl DroppedQueue {
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [Dropped::default(); MAX_LEN],
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn push_back(&mut self, x: Dropped) {
        let idx = self.head + self.len;
        let idx = if idx < MAX_LEN { idx } else { idx - MAX_LEN };
        self.buf[idx] = x;
        self.len += 1;
    }

    #[inline]
    fn pop_front(&mut self) -> Option<Dropped> {
        if self.len == 0 {
            return None;
        }
        let out = self.buf[self.head];
        self.head += 1;
        if self.head == MAX_LEN {
            self.head = 0;
        }
        self.len -= 1;
        Some(out)
    }

    #[inline]
    fn front(&self) -> Option<&Dropped> {
        if self.len == 0 {
            None
        } else {
            Some(&self.buf[self.head])
        }
    }
}

impl CellList {
    #[inline]
    fn new() -> Self {
        Self {
            len: 0,
            buf: [0; MAX_CELLS],
        }
    }

    #[inline]
    fn push(&mut self, cell: Cell) {
        self.buf[self.len] = cell;
        self.len += 1;
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn truncate(&mut self, new_len: usize) {
        self.len = self.len.min(new_len);
    }

    #[inline]
    fn as_slice(&self) -> &[Cell] {
        &self.buf[..self.len]
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Cell] {
        &mut self.buf[..self.len]
    }
}

impl SmallDirList {
    #[inline]
    fn new() -> Self {
        Self {
            len: 0,
            dirs: [0; 4],
        }
    }

    #[inline]
    fn push(&mut self, dir: usize) {
        self.dirs[self.len] = dir as u8;
        self.len += 1;
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        &self.dirs[..self.len]
    }
}

impl SmallCellList {
    #[inline]
    fn new() -> Self {
        Self {
            len: 0,
            cells: [0; 4],
        }
    }

    #[inline]
    fn push(&mut self, cell: Cell) {
        self.cells[self.len] = cell;
        self.len += 1;
    }

    #[inline]
    fn as_slice(&self) -> &[Cell] {
        &self.cells[..self.len]
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Cell] {
        &mut self.cells[..self.len]
    }
}

impl InternalPosDeque {
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [0; MAX_LEN],
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(cells: &[Cell]) -> Self {
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
    fn physical_index(&self, idx: usize) -> usize {
        let raw = self.head + idx;
        if raw < MAX_LEN { raw } else { raw - MAX_LEN }
    }

    #[inline]
    fn head(&self) -> Cell {
        self.buf[self.head]
    }

    #[inline]
    fn push_front_grow(&mut self, cell: Cell) {
        let x = encode_internal_pos_hash(cell);
        self.head = if self.head == 0 {
            MAX_LEN - 1
        } else {
            self.head - 1
        };
        self.buf[self.head] = cell;
        self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(INTERNAL_POS_HASH_BASE1));
        self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(INTERNAL_POS_HASH_BASE2));
        self.len += 1;
    }

    #[inline]
    fn push_front_pop_back(&mut self, cell: Cell) {
        let len = self.len;
        let tail = self[len - 1];
        let tail_hash = encode_internal_pos_hash(tail);
        let x = encode_internal_pos_hash(cell);

        self.head = if self.head == 0 {
            MAX_LEN - 1
        } else {
            self.head - 1
        };
        self.buf[self.head] = cell;

        self.hash1 = x.wrapping_add(
            self.hash1
                .wrapping_sub(tail_hash.wrapping_mul(INTERNAL_POS_HASH_POW1[len - 1]))
                .wrapping_mul(INTERNAL_POS_HASH_BASE1),
        );
        self.hash2 = x.wrapping_add(
            self.hash2
                .wrapping_sub(tail_hash.wrapping_mul(INTERNAL_POS_HASH_POW2[len - 1]))
                .wrapping_mul(INTERNAL_POS_HASH_BASE2),
        );
    }

    #[inline]
    fn truncate(&mut self, new_len: usize) {
        if new_len >= self.len {
            return;
        }
        for idx in new_len..self.len {
            let x = encode_internal_pos_hash(self[idx]);
            self.hash1 = self
                .hash1
                .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW1[idx]));
            self.hash2 = self
                .hash2
                .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW2[idx]));
        }
        self.len = new_len;
    }
}

impl std::ops::Index<usize> for InternalPosDeque {
    type Output = Cell;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.buf[self.physical_index(idx)]
    }
}

impl PartialEq for InternalPosDeque {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len || self.hash1 != other.hash1 || self.hash2 != other.hash2 {
            return false;
        }
        for idx in 0..self.len {
            if self[idx] != other[idx] {
                return false;
            }
        }
        true
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

impl std::fmt::Debug for InternalPosDeque {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for idx in 0..self.len {
            list.entry(&self[idx]);
        }
        list.finish()
    }
}

impl State {
    fn initial(input: &Input) -> Self {
        let food = input.food;
        let mut food_hash = 0_u64;
        let mut remaining_food = 0usize;
        for idx in 0..input.n * input.n {
            let color = food[idx];
            if color != 0 {
                food_hash ^= FOOD_HASH_TABLE[idx][color as usize];
                remaining_food += 1;
            }
        }

        let pos = InternalPosDeque::from_slice(&[
            cell_of(4, 0, input.n),
            cell_of(3, 0, input.n),
            cell_of(2, 0, input.n),
            cell_of(1, 0, input.n),
            cell_of(0, 0, input.n),
        ]);

        let mut colors = [0_u8; MAX_LEN];
        let mut color_hash1 = 0_u64;
        let mut color_hash2 = 0_u64;
        for idx in 0..5 {
            colors[idx] = 1;
            let x = encode_color_hash(1);
            color_hash1 = color_hash1.wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
            color_hash2 = color_hash2.wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        }

        Self {
            n: input.n,
            food,
            remaining_food,
            food_hash,
            pos: pos.clone(),
            colors,
            color_hash1,
            color_hash2,
            pos_occupancy: InternalPosOccupancy::from_pos(&pos),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.pos.len()
    }

    #[inline]
    fn head(&self) -> Cell {
        self.pos.head()
    }

    #[inline]
    fn neck(&self) -> Cell {
        if self.len() >= 2 {
            self.pos[1]
        } else {
            self.head()
        }
    }

    #[inline]
    fn set_food(&mut self, cell: Cell, new_color: u8) {
        let idx = cell as usize;
        let old_color = self.food[idx];
        if old_color == new_color {
            return;
        }
        if old_color != 0 {
            self.food_hash ^= FOOD_HASH_TABLE[idx][old_color as usize];
            self.remaining_food -= 1;
        }
        if new_color != 0 {
            self.food_hash ^= FOOD_HASH_TABLE[idx][new_color as usize];
            self.remaining_food += 1;
        }
        self.food[idx] = new_color;
    }

    #[inline]
    fn append_color_at(&mut self, idx: usize, color: u8) {
        self.colors[idx] = color;
        let x = encode_color_hash(color);
        self.color_hash1 = self
            .color_hash1
            .wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.color_hash2 = self
            .color_hash2
            .wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
    }

    #[inline]
    fn truncate_colors(&mut self, new_len: usize, old_len: usize) {
        for idx in new_len..old_len {
            let x = encode_color_hash(self.colors[idx]);
            self.color_hash1 = self
                .color_hash1
                .wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
            self.color_hash2 = self
                .color_hash2
                .wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        }
    }
}

impl PartialEq for State {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.n != other.n
            || self.remaining_food != other.remaining_food
            || self.food_hash != other.food_hash
            || self.pos != other.pos
            || self.color_hash1 != other.color_hash1
            || self.color_hash2 != other.color_hash2
        {
            return false;
        }
        if self.food != other.food {
            return false;
        }
        let len = self.len();
        for idx in 0..len {
            if self.colors[idx] != other.colors[idx] {
                return false;
            }
        }
        true
    }
}

impl Eq for State {}

impl Hash for State {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.n.hash(state);
        self.remaining_food.hash(state);
        self.food_hash.hash(state);
        self.pos.hash(state);
        self.color_hash1.hash(state);
        self.color_hash2.hash(state);
    }
}

impl PartialEq for QuickSeenKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.non_target == other.non_target && self.bite == other.bite && self.state == other.state
    }
}

impl Eq for QuickSeenKey {}

impl Hash for QuickSeenKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state);
        self.non_target.hash(state);
        self.bite.hash(state);
    }
}

#[inline]
fn cell_of(r: usize, c: usize, n: usize) -> Cell {
    (r * n + c) as Cell
}

#[inline]
fn rc_of(cell: Cell, n: usize) -> (usize, usize) {
    let x = cell as usize;
    (x / n, x % n)
}

#[inline]
fn manhattan(table: &ManhattanTable, a: Cell, b: Cell) -> usize {
    table.get(a, b)
}

#[inline]
fn time_over(started: &Instant) -> bool {
    let over = started.elapsed().as_secs_f64() >= TIME_LIMIT_SEC;
    if over {
    }
    over
}

#[inline]
fn time_left(started: &Instant) -> f64 {
    (TIME_LIMIT_SEC - started.elapsed().as_secs_f64()).max(0.0)
}

#[inline]
fn dir_of_char(ch: u8) -> Option<usize> {
    match ch {
        b'U' => Some(0),
        b'D' => Some(1),
        b'L' => Some(2),
        b'R' => Some(3),
        _ => None,
    }
}

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut d = [0_u8; MAX_LEN];
    for x in d.iter_mut().take(m) {
        *x = it.next().unwrap().parse::<u8>().unwrap();
    }

    let mut food = [0_u8; MAX_CELLS];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input {
        n,
        m,
        d,
        food,
        manhattan: ManhattanTable::new(n),
    }
}

#[inline]
fn dir_between_cells(n: usize, a: Cell, b: Cell) -> Option<usize> {
    let (ar, ac) = rc_of(a, n);
    let (br, bc) = rc_of(b, n);
    if ar == br + 1 && ac == bc {
        return Some(0);
    }
    if ar + 1 == br && ac == bc {
        return Some(1);
    }
    if ar == br && ac == bc + 1 {
        return Some(2);
    }
    if ar == br && ac + 1 == bc {
        return Some(3);
    }
    None
}

#[inline]
fn next_head_cell(st: &State, dir: usize) -> Option<Cell> {
    let head = st.head() as usize;
    match dir {
        0 => (head >= st.n).then_some((head - st.n) as Cell),
        1 => (head + st.n < st.n * st.n).then_some((head + st.n) as Cell),
        2 => (head % st.n != 0).then_some((head - 1) as Cell),
        3 => (head % st.n + 1 < st.n).then_some((head + 1) as Cell),
        _ => None,
    }
}

#[inline]
fn next_head_cell_pos(n: usize, pos: &InternalPosDeque, dir: usize) -> Option<Cell> {
    let head = pos.head() as usize;
    match dir {
        0 => (head >= n).then_some((head - n) as Cell),
        1 => (head + n < n * n).then_some((head + n) as Cell),
        2 => (head % n != 0).then_some((head - 1) as Cell),
        3 => (head % n + 1 < n).then_some((head + 1) as Cell),
        _ => None,
    }
}

#[inline]
fn is_legal_dir(st: &State, dir: usize) -> bool {
    let Some(nh) = next_head_cell(st, dir) else {
        return false;
    };
    st.len() < 2 || nh != st.neck()
}

#[inline]
fn legal_dirs(st: &State) -> SmallDirList {
    let mut out = SmallDirList::new();
    for dir in 0..4 {
        if is_legal_dir(st, dir) {
            out.push(dir);
        }
    }
    out
}

#[inline]
fn legal_dir_count(st: &State) -> usize {
    let mut cnt = 0usize;
    for dir in 0..4 {
        if is_legal_dir(st, dir) {
            cnt += 1;
        }
    }
    cnt
}

#[inline]
fn find_internal_bite_idx(pos: &InternalPosDeque) -> Option<usize> {
    if pos.len() <= 2 {
        return None;
    }
    let head = pos[0];
    for idx in 1..pos.len() - 1 {
        if pos[idx] == head {
            return Some(idx);
        }
    }
    None
}

fn step_impl(
    st: &State,
    dir: usize,
    mut dropped: Option<&mut DroppedBuf>,
) -> (State, u8, Option<usize>) {
    let nh = next_head_cell(st, dir).unwrap();
    let old_len = st.len();
    let mut ns = st.clone();

    let ate = ns.food[nh as usize];
    let mut excluded_tail = if old_len > 0 {
        Some(st.pos[old_len - 1])
    } else {
        None
    };
    if ate != 0 {
        ns.set_food(nh, 0);
        ns.append_color_at(old_len, ate);
    } else {
        let old_tail = st.pos[old_len - 1];
        ns.pos_occupancy.dec(old_tail);
        ns.pos.push_front_pop_back(nh);
        excluded_tail = if old_len >= 2 {
            Some(st.pos[old_len - 2])
        } else {
            None
        };
    }

    let tail_bias = u8::from(excluded_tail == Some(nh));
    let bite = ns.pos_occupancy.count(nh) > tail_bias;
    ns.pos_occupancy.inc(nh);
    if ate != 0 {
        ns.pos.push_front_grow(nh);
    }

    let bite_idx = if bite {
        find_internal_bite_idx(&ns.pos)
    } else {
        None
    };
    debug_assert!(ate == 0 || bite_idx.is_none());
    if let Some(bi) = bite_idx {
        if let Some(buf) = &mut dropped {
            buf.clear();
        }
        let cur_len = ns.len();
        for p in bi + 1..cur_len {
            let cell = ns.pos[p];
            let color = ns.colors[p];
            ns.pos_occupancy.dec(cell);
            ns.set_food(cell, color);
            if let Some(buf) = &mut dropped {
                buf.push(cell, color);
            }
        }
        ns.truncate_colors(bi + 1, cur_len);
        ns.pos.truncate(bi + 1);
    } else if let Some(buf) = &mut dropped {
        buf.clear();
    }

    (ns, ate, bite_idx)
}

#[inline]
fn step(st: &State, dir: usize) -> (State, u8, Option<usize>) {
    step_impl(st, dir, None)
}

#[inline]
fn step_with_dropped(
    st: &State,
    dir: usize,
    dropped: &mut DroppedBuf,
) -> (State, u8, Option<usize>) {
    step_impl(st, dir, Some(dropped))
}

fn repair_prefix_after_bite(
    st_after: &State,
    input: &Input,
    ell: usize,
    dropped: &DroppedBuf,
) -> Option<PrefixRepairResult> {
    let need = ell.checked_sub(st_after.len())?;
    if need == 0 {
        return Some(PrefixRepairResult {
            state: st_after.clone(),
            ops: Ops::new(),
            repaired: false,
        });
    }
    if dropped.len < need {
        return None;
    }

    let mut state = st_after.clone();
    let mut ops = Ops::with_capacity(need);
    let mut cur_len = state.len();
    let mut prev = state.head();

    for ent in dropped.as_slice().iter().take(need) {
        let need_color = input.d[cur_len];
        if ent.color != need_color {
            return None;
        }
        let dir = dir_between_cells(state.n, prev, ent.cell)?;
        if state.len() >= 2 && ent.cell == state.neck() {
            return None;
        }
        if state.food[ent.cell as usize] != need_color {
            return None;
        }

        state.set_food(ent.cell, 0);
        state.pos.push_front_grow(ent.cell);
        state.pos_occupancy.inc(ent.cell);
        state.append_color_at(cur_len, need_color);
        ops.push(dir as Dir);

        prev = ent.cell;
        cur_len += 1;
    }

    if exact_prefix(&state, input, ell) {
        Some(PrefixRepairResult {
            state,
            ops,
            repaired: true,
        })
    } else {
        None
    }
}

#[inline]
fn lcp_state(st: &State, input: &Input) -> usize {
    let mut i = 0usize;
    let m = st.len().min(input.m);
    while i < m && st.colors[i] == input.d[i] {
        i += 1;
    }
    i
}

#[inline]
fn prefix_ok(st: &State, input: &Input, ell: usize) -> bool {
    let keep = st.len().min(ell);
    for idx in 0..keep {
        if st.colors[idx] != input.d[idx] {
            return false;
        }
    }
    true
}

#[inline]
fn exact_prefix(st: &State, input: &Input, ell: usize) -> bool {
    st.len() == ell && prefix_ok(st, input, ell)
}

#[inline]
fn remaining_food_count(st: &State) -> usize {
    st.remaining_food
}

fn nearest_food_dist(st: &State, input: &Input, color: u8) -> (usize, usize) {
    let head = st.head();
    let mut best = usize::MAX;
    let mut cnt = 0usize;
    for idx in 0..st.n * st.n {
        if st.food[idx] == color {
            cnt += 1;
            let dist = manhattan(&input.manhattan, head, idx as Cell);
            if dist < best {
                best = dist;
            }
        }
    }
    if best == usize::MAX {
        (1_000_000_000, cnt)
    } else {
        (best, cnt)
    }
}

fn target_adjacent(st: &State, target: u8) -> Option<usize> {
    let neck = st.neck();
    for dir in 0..4 {
        let Some(nh) = next_head_cell(st, dir) else {
            continue;
        };
        if nh == neck {
            continue;
        }
        if st.food[nh as usize] == target {
            return Some(dir);
        }
    }
    None
}

fn target_suffix_info(st: &State, input: &Input, ell: usize, target: u8) -> Option<(usize, usize)> {
    let head = st.head();
    let len = st.len();
    let mut best: Option<(usize, usize)> = None;
    for idx in ell..len {
        if st.colors[idx] != target {
            continue;
        }
        let prev = st.pos[idx - 1];
        let cand = (manhattan(&input.manhattan, head, prev), idx);
        if best.is_none() || cand < best.unwrap() {
            best = Some(cand);
        }
    }
    best
}

fn local_score(st: &State, input: &Input, ell: usize) -> (usize, usize, usize, usize, usize) {
    let target = input.d[ell];
    if exact_prefix(st, input, ell) {
        let (dist, _) = nearest_food_dist(st, input, target);
        let adj = target_adjacent(st, target).is_some();
        return (0, if adj { 0 } else { 1 }, dist, 0, st.len() - ell);
    }

    if let Some((dist, idx)) = target_suffix_info(st, input, ell, target) {
        return (1, 0, dist, idx - ell, st.len() - ell);
    }

    let (dist, _) = nearest_food_dist(st, input, target);
    (2, 0, dist, 0, st.len().saturating_sub(ell))
}

fn next_stage_rank(st: &State, input: &Input, ellp1: usize) -> (usize, usize, usize) {
    if ellp1 >= input.m {
        return (0, 0, 0);
    }
    let (dist, _) = nearest_food_dist(st, input, input.d[ellp1]);
    let (hr, hc) = rc_of(st.head(), st.n);
    let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
    (dist, center, 0)
}

fn greedy_future_lb_from_cell(
    st: &State,
    input: &Input,
    start_cell: Cell,
    start_ell: usize,
    horizon: usize,
    banned: Option<Cell>,
) -> (usize, usize, usize) {
    let mut cur = start_cell;
    let end = (start_ell + horizon).min(input.m);
    let mut used = CellList::new();
    if let Some(b) = banned {
        used.push(b);
    }

    let mut miss = 0usize;
    let mut first = 0usize;
    let mut total = 0usize;

    for idx in start_ell..end {
        let color = input.d[idx];
        let mut best: Option<(usize, Cell)> = None;
        for cell_idx in 0..st.n * st.n {
            if st.food[cell_idx] != color {
                continue;
            }
            let cell = cell_idx as Cell;
            if used.as_slice().iter().any(|&x| x == cell) {
                continue;
            }
            let dist = manhattan(&input.manhattan, cur, cell);
            if best.is_none() || dist < best.unwrap().0 {
                best = Some((dist, cell));
            }
        }

        if let Some((dist, cell)) = best {
            if idx == start_ell {
                first = dist;
            }
            total += dist;
            cur = cell;
            used.push(cell);
        } else {
            miss += 1;
            let penalty = st.n * st.n;
            if idx == start_ell {
                first = penalty;
            }
            total += penalty;
        }
    }

    (miss, first, total)
}

fn turn_focus_next_stage_rank(
    st: &State,
    input: &Input,
    ellp1: usize,
) -> (usize, usize, usize, usize, usize) {
    if ellp1 >= input.m {
        return (0, 0, 0, 0, 0);
    }
    let (miss, first, total) =
        greedy_future_lb_from_cell(st, input, st.head(), ellp1, LOOKAHEAD_HORIZON, None);
    let adj = target_adjacent(st, input.d[ellp1]).is_some();
    let (hr, hc) = rc_of(st.head(), st.n);
    let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
    let mobility_penalty = 4usize.saturating_sub(legal_dir_count(st));
    (
        miss,
        usize::from(!adj),
        total,
        first,
        center + mobility_penalty,
    )
}

fn target_candidate_rank(
    st: &State,
    input: &Input,
    ell: usize,
    target: Cell,
) -> (usize, usize, usize, usize, usize) {
    let head = st.head();
    let capture_lb = manhattan(&input.manhattan, head, target);
    let (miss, first, total) =
        greedy_future_lb_from_cell(st, input, target, ell + 1, LOOKAHEAD_HORIZON, Some(target));

    let mut goal_blocked = 1usize;
    for &nb in neighbors(st.n, target).as_slice() {
        if st.food[nb as usize] == 0 || nb == head {
            goal_blocked = 0;
            break;
        }
    }

    (miss, capture_lb + total, first, goal_blocked, capture_lb)
}

fn absolute_score_state(st: &State, input: &Input, ops_len: usize) -> usize {
    let k = st.len().min(input.m);
    let mut mismatch = 0usize;
    for idx in 0..k {
        if st.colors[idx] != input.d[idx] {
            mismatch += 1;
        }
    }
    ops_len + 10_000 * (mismatch + 2 * (input.m - k))
}

fn beam_score_key(bs: &BeamState, input: &Input) -> (usize, usize, Reverse<usize>, usize) {
    (
        absolute_score_state(&bs.state, input, bs.ops.len()),
        bs.ops.len(),
        Reverse(lcp_state(&bs.state, input)),
        remaining_food_count(&bs.state),
    )
}

fn choose_best_beamstate(cands: Vec<BeamState>, input: &Input) -> BeamState {
    cands
        .into_iter()
        .min_by_key(|bs| beam_score_key(bs, input))
        .unwrap()
}

fn append_reconstruct_plan(nodes: &[Node], mut idx: usize, out: &mut Ops) {
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

fn reconstruct_plan(nodes: &[Node], idx: usize) -> Ops {
    let mut out = Ops::new();
    append_reconstruct_plan(nodes, idx, &mut out);
    out
}

fn append_reconstruct_quick_plan(nodes: &[QuickSearchNode], mut idx: usize, out: &mut Ops) {
    let mut rev = Vec::new();
    while nodes[idx].parent != usize::MAX {
        rev.push(nodes[idx].dir);
        idx = nodes[idx].parent;
    }
    rev.reverse();
    out.reserve(rev.len());
    out.extend_from_slice(&rev);
}

fn plan_color_goal_quick(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target_color: u8,
    cfg: QuickPlanConfig,
    started: &Instant,
) -> Option<BeamState> {
    let root = QuickSearchNode {
        state: bs.state.clone(),
        parent: usize::MAX,
        dir: 0,
        depth: 0,
        non_target: 0,
        bite: 0,
    };
    let mut nodes = vec![root];
    let mut q = Vec::with_capacity(cfg.node_limit.min(16_384));
    let mut q_head = 0usize;
    q.push(0usize);

    let mut seen = FxHashMap::<QuickSeenKey, u8>::default();
    seen.insert(
        QuickSeenKey {
            state: nodes[0].state.clone(),
            non_target: 0,
            bite: 0,
        },
        0,
    );

    while q_head < q.len() {
        let cur_idx = q[q_head];
        q_head += 1;
        if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
            return None;
        }
        let cur_depth = nodes[cur_idx].depth;
        if cur_depth >= cfg.depth_limit {
            continue;
        }

        let dirs = legal_dirs(&nodes[cur_idx].state);
        for &dir_u8 in dirs.as_slice() {
            let dir = dir_u8 as usize;
            let (sim, ate, bite_idx) = step(&nodes[cur_idx].state, dir);

            let keep = sim.len().min(ell);
            let mut ok = true;
            for idx in 0..keep {
                if sim.colors[idx] != input.d[idx] {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }

            let mut non_target = nodes[cur_idx].non_target;
            if ate != 0 && ate != target_color {
                if non_target >= cfg.non_target_limit {
                    continue;
                }
                non_target += 1;
            }

            let mut bite = nodes[cur_idx].bite;
            if bite_idx.is_some() {
                if bite >= cfg.bite_limit {
                    continue;
                }
                bite += 1;
            }

            let next_depth = cur_depth + 1;
            let child = QuickSearchNode {
                state: sim,
                parent: cur_idx,
                dir: dir as u8,
                depth: next_depth,
                non_target,
                bite,
            };

            if child.state.len() >= ell + 1 {
                let mut good = true;
                for idx in 0..=ell {
                    if child.state.colors[idx] != input.d[idx] {
                        good = false;
                        break;
                    }
                }
                if good {
                    nodes.push(child);
                    let goal_idx = nodes.len() - 1;
                    let mut ops = bs.ops.clone();
                    append_reconstruct_quick_plan(&nodes, goal_idx, &mut ops);
                    return Some(BeamState {
                        state: nodes[goal_idx].state.clone(),
                        ops,
                    });
                }
            }

            if nodes.len() >= cfg.node_limit {
                return None;
            }

            let key = QuickSeenKey {
                state: child.state.clone(),
                non_target,
                bite,
            };
            if seen
                .get(&key)
                .is_some_and(|&best_depth| best_depth <= next_depth)
            {
                continue;
            }
            seen.insert(key, next_depth);
            nodes.push(child);
            q.push(nodes.len() - 1);
        }
    }

    None
}

fn extend_fastlane_one(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    started: &Instant,
) -> Option<BeamState> {
    if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
        return None;
    }

    let target_color = input.d[ell];
    if collect_food_cells(&bs.state, target_color).is_empty() {
        return None;
    }

    let safe_cfg = QuickPlanConfig {
        depth_limit: FAST_SAFE_DEPTH_LIMIT,
        node_limit: FAST_SAFE_NODE_LIMIT,
        non_target_limit: 0,
        bite_limit: 0,
    };
    if let Some(sol) = plan_color_goal_quick(bs, input, ell, target_color, safe_cfg, started) {
        return Some(sol);
    }

    if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
        return None;
    }

    let rescue_cfg = QuickPlanConfig {
        depth_limit: FAST_RESCUE_DEPTH_LIMIT,
        node_limit: FAST_RESCUE_NODE_LIMIT,
        non_target_limit: 6,
        bite_limit: 2,
    };
    if let Some(sol) = plan_color_goal_quick(bs, input, ell, target_color, rescue_cfg, started) {
        return Some(sol);
    }

    let sols =
        collect_exact_solutions(bs, input, ell, target_color, FAST_FALLBACK_TARGETS, started);
    let out = sols.into_iter().min_by_key(|cand| cand.ops.len());
    if out.is_some() {
    }
    out
}

fn try_recover_exact(
    st: &State,
    input: &Input,
    ell: usize,
    dropped: &DroppedBuf,
) -> Option<(State, Ops)> {
    let repaired = repair_prefix_after_bite(st, input, ell, dropped)?;
    if !exact_prefix(&repaired.state, input, ell) {
        return None;
    }
    let ops = if repaired.repaired {
        repaired.ops
    } else {
        Ops::new()
    };
    Some((repaired.state, ops))
}

fn stage_search_bestfirst(
    start_bs: &BeamState,
    input: &Input,
    ell: usize,
    budgets: &[(usize, usize)],
    keep_solutions: usize,
    started: &Instant,
) -> Vec<BeamState> {
    if budgets.is_empty() {
        return Vec::new();
    }
    let start = start_bs.state.clone();

    let max_expansions = budgets[budgets.len() - 1].0;
    let mut nodes = Vec::with_capacity(max_expansions.min(30_000) + 8);
    nodes.push(Node {
        state: start.clone(),
        parent: None,
        move_seg: Ops::new(),
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((
        local_score(&start, input, ell),
        0usize,
        uid,
        0usize,
    )));
    uid += 1;

    let mut seen = FxHashMap::<State, usize>::default();
    seen.insert(start, 0);

    let mut sols: Vec<BeamState> = Vec::new();
    let mut solkeys: FxHashSet<State> = FxHashSet::default();
    let mut expansions = 0usize;
    let mut stage_idx = 0usize;
    let mut stage_limit = budgets[0].0;
    let mut extra_limit = budgets[0].1;
    let final_limit = budgets[budgets.len() - 1].0;
    let mut dropped1 = DroppedBuf::new();
    let mut dropped2 = DroppedBuf::new();

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= final_limit || sols.len() >= keep_solutions || time_over(started) {
            break;
        }

        expansions += 1;
        let st = nodes[idx].state.clone();

        if exact_prefix(&st, input, ell) {
            if let Some(dir2) = target_adjacent(&st, input.d[ell]) {
                let (ns2, _, bite2) = step(&st, dir2);
                if bite2.is_none() && exact_prefix(&ns2, input, ell + 1) {
                    if solkeys.insert(ns2.clone()) {
                        let mut ops = start_bs.ops.clone();
                        append_reconstruct_plan(&nodes, idx, &mut ops);
                        ops.push(dir2 as Dir);
                        sols.push(BeamState { state: ns2, ops });
                        if sols.len() >= keep_solutions {
                            break;
                        }
                    }
                }
            }
        }
        if sols.len() >= keep_solutions {
            break;
        }

        {
            let mut prefix_plan: Option<Ops> = None;
            let dirs1 = legal_dirs(&st);
            for &dir1_u8 in dirs1.as_slice() {
                let dir1 = dir1_u8 as usize;
                let (ns1, _, bite1) = step_with_dropped(&st, dir1, &mut dropped1);
                if bite1.is_none() || !prefix_ok(&ns1, input, ell) {
                    continue;
                }

                let mut rs = ns1;
                let mut recover_ops = Ops::new();
                if rs.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, input, ell, &dropped1)
                    else {
                        continue;
                    };
                    rs = rec_state;
                    recover_ops = rec_ops;
                }

                if !exact_prefix(&rs, input, ell) {
                    continue;
                }

                let dirs2 = legal_dirs(&rs);
                for &dir2_u8 in dirs2.as_slice() {
                    let dir2 = dir2_u8 as usize;
                    let nh = next_head_cell(&rs, dir2).unwrap();
                    if rs.food[nh as usize] != input.d[ell] {
                        continue;
                    }

                    let (ns2, _, bite2) = step(&rs, dir2);
                    if bite2.is_some() || !exact_prefix(&ns2, input, ell + 1) {
                        continue;
                    }

                    if !solkeys.insert(ns2.clone()) {
                        continue;
                    }

                    let mut ops = start_bs.ops.clone();
                    let prefix = prefix_plan.get_or_insert_with(|| reconstruct_plan(&nodes, idx));
                    ops.reserve(prefix.len() + recover_ops.len() + 2);
                    ops.extend_from_slice(prefix);
                    ops.push(dir1 as Dir);
                    ops.extend_from_slice(&recover_ops);
                    ops.push(dir2 as Dir);
                    sols.push(BeamState { state: ns2, ops });
                    if sols.len() >= keep_solutions {
                        break;
                    }
                }
                if sols.len() >= keep_solutions {
                    break;
                }
            }
        }
        if sols.len() >= keep_solutions {
            break;
        }

        let dirs = legal_dirs(&st);
        for &dir_u8 in dirs.as_slice() {
            let dir = dir_u8 as usize;
            let (mut ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped2);
            let mut seg = Ops::with_capacity(1 + ell);
            seg.push(dir as Dir);

            if bite_idx.is_some() && ns.len() < ell {
                if !prefix_ok(&ns, input, ell) {
                    continue;
                }
                let Some((rec_state, rec_ops)) = try_recover_exact(&ns, input, ell, &dropped2)
                else {
                    continue;
                };
                ns = rec_state;
                seg.extend_from_slice(&rec_ops);
            }

            if !prefix_ok(&ns, input, ell) {
                continue;
            }
            if ns.len() > ell + extra_limit {
                continue;
            }

            let nd = depth + seg.len();
            if seen.get(&ns).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(ns.clone(), nd);

            let child = nodes.len();
            nodes.push(Node {
                state: ns.clone(),
                parent: Some(idx),
                move_seg: seg,
            });
            pq.push(Reverse((local_score(&ns, input, ell), nd, uid, child)));
            uid += 1;
        }

        if !exact_prefix(&st, input, ell) {
            let dirs = legal_dirs(&st);
            for &dir_u8 in dirs.as_slice() {
                let dir = dir_u8 as usize;
                let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped1);
                if bite_idx.is_none() || !prefix_ok(&ns, input, ell) {
                    continue;
                }

                let mut rs = ns;
                let mut seg = Ops::with_capacity(1 + ell);
                seg.push(dir as Dir);

                if rs.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, input, ell, &dropped1)
                    else {
                        continue;
                    };
                    rs = rec_state;
                    seg.extend_from_slice(&rec_ops);
                }

                if !exact_prefix(&rs, input, ell) {
                    continue;
                }

                let nd = depth + seg.len();
                if seen.get(&rs).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(rs.clone(), nd);

                let child = nodes.len();
                nodes.push(Node {
                    state: rs.clone(),
                    parent: Some(idx),
                    move_seg: seg,
                });
                pq.push(Reverse((local_score(&rs, input, ell), nd, uid, child)));
                uid += 1;
            }
        }

        if expansions >= stage_limit {
            if !sols.is_empty() {
                break;
            }
            if stage_idx + 1 < budgets.len() {
                stage_idx += 1;
                stage_limit = budgets[stage_idx].0;
                extra_limit = budgets[stage_idx].1;
            }
        }
    }

    sols.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, input, ell + 1), bs.ops.len()));

    let mut out = Vec::with_capacity(keep_solutions);
    let mut seen2: FxHashSet<State> = FxHashSet::default();
    for bs in sols {
        if seen2.insert(bs.state.clone()) {
            out.push(bs);
            if out.len() >= keep_solutions {
                break;
            }
        }
    }
    out
}

fn collect_food_cells(st: &State, color: u8) -> CellList {
    let mut out = CellList::new();
    for idx in 0..st.n * st.n {
        if st.food[idx] == color {
            out.push(idx as Cell);
        }
    }
    out
}

fn neighbors(n: usize, cid: Cell) -> SmallCellList {
    let (r, c) = rc_of(cid, n);
    let mut out = SmallCellList::new();
    for (dr, dc, _) in DIRS {
        let nr = r as isize + dr;
        let nc = c as isize + dc;
        if nr >= 0 && nr < n as isize && nc >= 0 && nc < n as isize {
            out.push(cell_of(nr as usize, nc as usize, n));
        }
    }
    out
}

fn compute_body_release_dist(st: &State) -> CellSearchResult {
    let mut front_idx = [u16::MAX; MAX_CELLS];
    let len = st.len();
    for idx in 0..len {
        let cell_idx = st.pos[idx] as usize;
        if front_idx[cell_idx] == u16::MAX {
            front_idx[cell_idx] = idx as u16;
        }
    }

    let mut release = [0_u16; MAX_CELLS];
    for cell_idx in 0..st.n * st.n {
        let j = front_idx[cell_idx];
        if j != u16::MAX {
            let t = (len - 1 - j as usize).max(1) as u16;
            release[cell_idx] = t;
        }
    }

    let mut dist = [u16::MAX; MAX_CELLS];
    let mut prev = [u16::MAX; MAX_CELLS];
    let mut q = [0_u16; MAX_CELLS];
    let mut q_head = 0usize;
    let mut q_tail = 0usize;

    let head = st.head();
    dist[head as usize] = 0;
    q[q_tail] = head;
    q_tail += 1;

    while q_head < q_tail {
        let cur = q[q_head];
        q_head += 1;
        let cur_dist = dist[cur as usize];

        for &nxt in neighbors(st.n, cur).as_slice() {
            let nxt_idx = nxt as usize;
            let nd = cur_dist.saturating_add(1);
            if dist[nxt_idx] != u16::MAX {
                continue;
            }
            if st.food[nxt_idx] != 0 {
                continue;
            }
            if nd < release[nxt_idx] {
                continue;
            }
            dist[nxt_idx] = nd;
            prev[nxt_idx] = cur;
            q[q_tail] = nxt;
            q_tail += 1;
        }
    }

    CellSearchResult {
        start: head,
        dist,
        prev,
    }
}

fn body_release_eat_dist(bfs: &CellSearchResult, st: &State) -> CellSearchResult {
    let mut dist = bfs.dist;
    let mut prev = bfs.prev;

    for idx in 0..st.n * st.n {
        if st.food[idx] == 0 {
            continue;
        }

        let cell = idx as Cell;
        let mut best = u16::MAX;
        let mut best_gate = u16::MAX;
        for &gate in neighbors(st.n, cell).as_slice() {
            let gate_dist = bfs.dist[gate as usize];
            if gate_dist == u16::MAX {
                continue;
            }
            let cand = gate_dist.saturating_add(1);
            if cand < best {
                best = cand;
                best_gate = gate;
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

fn reconstruct_cell_search_path(result: &CellSearchResult, goal: Cell) -> Option<CellList> {
    if result.dist[goal as usize] == u16::MAX {
        return None;
    }

    let mut rev = CellList::new();
    let mut cur = goal;
    loop {
        rev.push(cur);
        if cur == result.start {
            break;
        }
        let p = result.prev[cur as usize];
        if p == u16::MAX {
            return None;
        }
        cur = p;
    }

    let mut out = CellList::new();
    let mut idx = rev.len();
    while idx > 0 {
        idx -= 1;
        out.push(rev.buf[idx]);
    }
    Some(out)
}

fn collect_top_k_target_paths(st: &State, target_color: u8, k: usize) -> Vec<CellList> {
    let bfs = compute_body_release_dist(st);
    let eat_dist = body_release_eat_dist(&bfs, st);

    let mut foods = Vec::<(u16, Cell)>::new();
    for idx in 0..st.n * st.n {
        if st.food[idx] != target_color {
            continue;
        }
        let dist = eat_dist.dist[idx];
        if dist == u16::MAX {
            continue;
        }
        foods.push((dist, idx as Cell));
    }

    foods.sort_unstable();
    if foods.len() > k {
        foods.truncate(k);
    }

    let mut paths = Vec::with_capacity(foods.len());
    for (_, cell) in foods {
        if let Some(path) = reconstruct_cell_search_path(&eat_dist, cell) {
            paths.push(path);
        }
    }
    paths
}

fn build_simple_child(
    parent: &BeamState,
    input: &Input,
    ell: usize,
    path: &CellList,
) -> Option<BeamState> {
    if !exact_prefix(&parent.state, input, ell) {
        return None;
    }

    let slice = path.as_slice();
    if slice.first().copied() != Some(parent.state.head()) {
        return None;
    }

    let mut child_state = parent.state.clone();
    let mut child_ops = parent.ops.clone();

    for step_idx in 1..slice.len() {
        let dir = dir_between_cells(child_state.n, slice[step_idx - 1], slice[step_idx])?;
        let (ns, ate, bite_idx) = step(&child_state, dir);
        if bite_idx.is_some() {
            return None;
        }
        if step_idx + 1 != slice.len() && ate != 0 {
            return None;
        }
        child_state = ns;
        child_ops.push(dir as Dir);
        if child_ops.len() > MAX_TURNS {
            return None;
        }
    }

    exact_prefix(&child_state, input, ell + 1).then_some(BeamState {
        state: child_state,
        ops: child_ops,
    })
}

#[inline]
fn local_visited_key(state: &State) -> LocalVisitedKey {
    LocalVisitedKey {
        pos_len: state.pos.len() as u16,
        pos_hash1: state.pos.hash1,
        pos_hash2: state.pos.hash2,
        colors_len: state.len() as u16,
        colors_hash1: state.color_hash1,
        colors_hash2: state.color_hash2,
    }
}

#[inline]
fn append_local_dir(path_bits: u16, depth: u8, dir: Dir) -> u16 {
    // depth >= 8 は u16 に収まらない。release では guard が無いので、定数変更時は型幅も必ず見直す。
    path_bits | ((dir as u16) << (2 * depth))
}

#[inline]
fn extend_ops_with_local_path(ops: &mut Ops, path_bits: u16, depth: u8) {
    // append_local_dir と同じ packed 形式を decode する。壊れた path_bits を渡すと不正手に直結する。
    ops.reserve(depth as usize);
    for i in 0..depth {
        ops.push(((path_bits >> (2 * i)) & 3) as Dir);
    }
}

fn build_bite_child_from_base(
    parent: &BeamState,
    repaired_state: &State,
    local_path_bits: u16,
    local_depth: u8,
    repaired_ops: &[Dir],
    input: &Input,
    ell: usize,
    path: &CellList,
) -> Option<BeamState> {
    if !exact_prefix(repaired_state, input, ell) {
        return None;
    }

    let slice = path.as_slice();
    if slice.first().copied() != Some(repaired_state.head()) {
        return None;
    }

    let mut child_state = repaired_state.clone();
    let mut child_ops = parent.ops.clone();
    extend_ops_with_local_path(&mut child_ops, local_path_bits, local_depth);
    child_ops.extend_from_slice(repaired_ops);
    if child_ops.len() > MAX_TURNS {
        return None;
    }

    for step_idx in 1..slice.len() {
        let dir = dir_between_cells(child_state.n, slice[step_idx - 1], slice[step_idx])?;
        let (ns, ate, bite_idx) = step(&child_state, dir);
        if bite_idx.is_some() {
            return None;
        }
        if step_idx + 1 != slice.len() && ate != 0 {
            return None;
        }
        child_state = ns;
        child_ops.push(dir as Dir);
        if child_ops.len() > MAX_TURNS {
            return None;
        }
    }

    exact_prefix(&child_state, input, ell + 1).then_some(BeamState {
        state: child_state,
        ops: child_ops,
    })
}

fn expand_bite_children(
    parent: &BeamState,
    input: &Input,
    ell: usize,
    candidate_width: usize,
) -> Vec<BeamState> {
    if !exact_prefix(&parent.state, input, ell) {
        return Vec::new();
    }

    let target_color = input.d[ell];
    let mut out = Vec::new();
    let mut q = Vec::new();
    let mut q_head = 0usize;
    let mut visited: FxHashSet<LocalVisitedKey> = FxHashSet::default();
    let mut dropped = DroppedBuf::new();

    q.push(LocalBiteNode {
        state: parent.state.clone(),
        path_bits: 0,
        depth: 0,
    });
    visited.insert(local_visited_key(&parent.state));

    while q_head < q.len() {
        if out.len() >= candidate_width {
            break;
        }
        let node = q[q_head].clone();
        q_head += 1;

        if node.depth as usize >= BITE_DEPTH_LIMIT {
            continue;
        }

        let dirs = legal_dirs(&node.state);
        for &dir_u8 in dirs.as_slice() {
            let dir = dir_u8 as usize;
            let (next_state, ate, bite_idx) = step_with_dropped(&node.state, dir, &mut dropped);
            if ate == target_color {
                continue;
            }

            let next_depth = node.depth + 1;
            let next_path_bits = append_local_dir(node.path_bits, node.depth, dir_u8 as Dir);

            if bite_idx.is_none() {
                let key = local_visited_key(&next_state);
                if visited.insert(key) {
                    q.push(LocalBiteNode {
                        state: next_state,
                        path_bits: next_path_bits,
                        depth: next_depth,
                    });
                }
                continue;
            }

            if next_state.len() > ell || next_state.len() + dropped.len < ell {
                continue;
            }

            let Some(repaired) = repair_prefix_after_bite(&next_state, input, ell, &dropped) else {
                continue;
            };
            if !exact_prefix(&repaired.state, input, ell) {
                continue;
            }

            let paths = collect_top_k_target_paths(&repaired.state, target_color, 1);
            let Some(path) = paths.first() else {
                continue;
            };

            let Some(child) = build_bite_child_from_base(
                parent,
                &repaired.state,
                next_path_bits,
                next_depth,
                &repaired.ops,
                input,
                ell,
                path,
            ) else {
                continue;
            };
            out.push(child);
            if out.len() >= candidate_width {
                break;
            }
        }
    }

    out
}

fn expand_simple_children(
    parent: &BeamState,
    input: &Input,
    ell: usize,
    candidate_width: usize,
) -> Vec<BeamState> {
    if !exact_prefix(&parent.state, input, ell) {
        return Vec::new();
    }

    let paths = collect_top_k_target_paths(&parent.state, input.d[ell], candidate_width);
    let mut out = Vec::with_capacity(paths.len());
    let mut seen: FxHashSet<State> = FxHashSet::default();
    for path in &paths {
        if let Some(child) = build_simple_child(parent, input, ell, path) {
            if seen.insert(child.state.clone()) {
                out.push(child);
            }
        }
    }
    out
}

fn simple_next_target_path_len(st: &State, input: &Input, next_ell: usize) -> Option<usize> {
    if next_ell >= input.m {
        return Some(0);
    }
    collect_top_k_target_paths(st, input.d[next_ell], 1)
        .into_iter()
        .next()
        .map(|path| path.len().saturating_sub(1))
}

fn simple_beam_rank(
    st: &State,
    input: &Input,
    next_ell: usize,
    ops_len: usize,
) -> (usize, usize, usize, usize) {
    if next_ell >= input.m {
        return (0, ops_len, 0, 0);
    }
    if let Some(path_len) = simple_next_target_path_len(st, input, next_ell) {
        (0, ops_len, path_len, remaining_food_count(st))
    } else {
        (1, ops_len, usize::MAX, remaining_food_count(st))
    }
}

fn solve_simple_beam_from(
    input: &Input,
    start_bs: BeamState,
    start_ell: usize,
    beam_width: usize,
    candidate_width: usize,
    started: &Instant,
    min_left_sec: f64,
) -> BeamState {
    let mut beam = vec![start_bs.clone()];
    let mut ell = start_ell;

    while ell < input.m {
        if time_over(started) || time_left(started) < min_left_sec {
            break;
        }

        let mut all_children = Vec::new();
        for parent in &beam {
            if time_over(started) || time_left(started) < min_left_sec {
                break;
            }
            let mut children = expand_simple_children(parent, input, ell, candidate_width);
            all_children.append(&mut children);
        }

        if all_children.is_empty() {
            break;
        }

        all_children
            .sort_unstable_by_key(|bs| simple_beam_rank(&bs.state, input, ell + 1, bs.ops.len()));

        let mut next_beam = Vec::with_capacity(beam_width);
        let mut seen: FxHashSet<State> = FxHashSet::default();
        for child in all_children {
            if seen.insert(child.state.clone()) {
                next_beam.push(child);
                if next_beam.len() >= beam_width {
                    break;
                }
            }
        }
        beam = next_beam;
        ell += 1;
    }

    if beam.is_empty() {
        start_bs
    } else {
        choose_best_beamstate(beam, input)
    }
}

#[inline]
fn can_reach_target_next_pos(n: usize, pos: &InternalPosDeque, target: Cell) -> bool {
    dir_between_cells(n, pos[0], target).is_some() && (pos.len() < 2 || target != pos[1])
}

fn empty_step_pos(
    n: usize,
    food: &[u8; MAX_CELLS],
    pos: &InternalPosDeque,
    dir: usize,
    target: Cell,
) -> Option<InternalPosDeque> {
    let nh = next_head_cell_pos(n, pos, dir)?;
    if pos.len() >= 2 && nh == pos[1] {
        return None;
    }
    if nh == target || food[nh as usize] != 0 {
        return None;
    }

    let mut new_pos = pos.clone();
    new_pos.push_front_pop_back(nh);
    if find_internal_bite_idx(&new_pos).is_some() {
        return None;
    }
    Some(new_pos)
}

fn reachable_goal_neighbor_count_pos(n: usize, pos: &InternalPosDeque, target: Cell) -> usize {
    let mut blocked = [false; MAX_CELLS];
    if pos.len() >= 3 {
        for idx in 1..pos.len() - 1 {
            blocked[pos[idx] as usize] = true;
        }
    }

    let start = pos[0];
    let mut seen = [false; MAX_CELLS];
    let mut q = [0_u16; MAX_CELLS];
    let mut q_head = 0usize;
    let mut q_tail = 0usize;

    seen[start as usize] = true;
    q[q_tail] = start;
    q_tail += 1;

    while q_head < q_tail {
        let cur = q[q_head];
        q_head += 1;
        let (r, c) = rc_of(cur, n);
        for (dr, dc, _) in DIRS {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                continue;
            }
            let nxt = cell_of(nr as usize, nc as usize, n);
            let idx = nxt as usize;
            if blocked[idx] || seen[idx] {
                continue;
            }
            seen[idx] = true;
            q[q_tail] = nxt;
            q_tail += 1;
        }
    }

    let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
    let mut cnt = 0usize;
    for &nb in neighbors(n, target).as_slice() {
        if nb != neck && seen[nb as usize] {
            cnt += 1;
        }
    }
    cnt
}

fn legal_dir_count_pos(n: usize, pos: &InternalPosDeque) -> usize {
    let (hr, hc) = rc_of(pos[0], n);
    let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
    let mut cnt = 0usize;
    for (dr, dc, _) in DIRS {
        let nr = hr as isize + dr;
        let nc = hc as isize + dc;
        if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
            continue;
        }
        let nh = cell_of(nr as usize, nc as usize, n);
        if nh != neck {
            cnt += 1;
        }
    }
    cnt
}

#[inline]
fn empty_path_rank(
    n: usize,
    pos: &InternalPosDeque,
    target: Cell,
    manhattan_table: &ManhattanTable,
) -> (usize, usize, usize, usize) {
    (
        usize::from(!can_reach_target_next_pos(n, pos, target)),
        manhattan(manhattan_table, pos[0], target),
        4usize.saturating_sub(reachable_goal_neighbor_count_pos(n, pos, target)),
        4usize.saturating_sub(legal_dir_count_pos(n, pos)),
    )
}

#[inline]
fn can_reach_target_next(st: &State, target: Cell) -> bool {
    let Some(_) = dir_between_cells(st.n, st.head(), target) else {
        return false;
    };
    st.len() < 2 || target != st.neck()
}

fn bfs_next_dir(
    st: &State,
    goal: Cell,
    target: Cell,
    avoid_food: bool,
    strict_body: bool,
) -> Option<usize> {
    let n = st.n;
    let cell_count = n * n;
    let start = st.head();
    if start == goal {
        return None;
    }

    let mut blocked = [false; MAX_CELLS];
    if avoid_food {
        for idx in 0..cell_count {
            let cell = idx as Cell;
            if st.food[idx] != 0 && cell != goal && cell != target {
                blocked[idx] = true;
            }
        }
    }

    if strict_body && st.len() >= 3 {
        for idx in 1..st.len() - 1 {
            blocked[st.pos[idx] as usize] = true;
        }
    }

    blocked[start as usize] = false;
    if blocked[goal as usize] {
        return None;
    }

    let mut dist = [u16::MAX; MAX_CELLS];
    let mut first = [u8::MAX; MAX_CELLS];
    let mut q = [0_u16; MAX_CELLS];
    let mut q_head = 0usize;
    let mut q_tail = 0usize;

    let dirs = legal_dirs(st);
    for &dir_u8 in dirs.as_slice() {
        let dir = dir_u8 as usize;
        let nid = next_head_cell(st, dir).unwrap();
        let idx = nid as usize;
        if blocked[idx] || dist[idx] != u16::MAX {
            continue;
        }
        dist[idx] = 1;
        first[idx] = dir as u8;
        q[q_tail] = nid;
        q_tail += 1;
    }

    while q_head < q_tail {
        let cid = q[q_head];
        q_head += 1;
        if cid == goal {
            return Some(first[cid as usize] as usize);
        }
        let (r, c) = rc_of(cid, n);
        for (dr, dc, _) in DIRS {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                continue;
            }
            let nid = cell_of(nr as usize, nc as usize, n);
            let idx = nid as usize;
            if blocked[idx] || dist[idx] != u16::MAX {
                continue;
            }
            dist[idx] = dist[cid as usize] + 1;
            first[idx] = first[cid as usize];
            q[q_tail] = nid;
            q_tail += 1;
        }
    }

    None
}

#[inline]
fn make_visit_key(st: &State, goal: Cell, restore_len: usize) -> VisitKey {
    let neck = if st.len() >= 2 { st.neck() } else { st.head() };
    VisitKey {
        head: st.head(),
        neck,
        len: st.len() as u16,
        goal,
        restore_len: restore_len as u16,
    }
}

fn advance_with_restore_queue(
    st: &State,
    input: &Input,
    dir: usize,
    target: Cell,
    ell: usize,
    restore_queue: &mut DroppedQueue,
    dropped: &mut DroppedBuf,
) -> Option<(State, Ops, Option<usize>)> {
    let (ns, _, bite_idx) = step_with_dropped(st, dir, dropped);
    if ns.food[target as usize] == 0 {
        return None;
    }

    *restore_queue = DroppedQueue::new();

    if bite_idx.is_some() && ns.len() < ell {
        let repaired = repair_prefix_after_bite(&ns, input, ell, dropped)?;
        if repaired.state.food[target as usize] == 0 {
            return None;
        }
        return Some((repaired.state, repaired.ops, bite_idx));
    }
    Some((ns, Ops::new(), bite_idx))
}

fn navigate_to_goal_safe(
    bs: &BeamState,
    goal: Cell,
    target: Cell,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut guard = 0usize;

    while st.head() != goal {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 30 {
            return None;
        }

        let key = make_visit_key(&st, goal, 0);
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = bfs_next_dir(&st, goal, target, true, true)?;
        let (ns, ate, bite_idx) = step(&st, dir);
        if ns.food[target as usize] == 0 {
            return None;
        }
        if bite_idx.is_some() || ate != 0 {
            return None;
        }

        st = ns;
        ops.push(dir as Dir);
    }
    Some(BeamState { state: st, ops })
}

fn navigate_to_goal_loose(
    bs: &BeamState,
    input: &Input,
    goal: Cell,
    target: Cell,
    ell: usize,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    let mut restore_queue = DroppedQueue::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 4;
    let mut guard = 0usize;
    let mut dropped = DroppedBuf::new();

    while st.head() != goal || !restore_queue.is_empty() {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 80 {
            return None;
        }

        let key = make_visit_key(&st, goal, restore_queue.len);
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if st.len() >= 2 && front.cell == st.neck() {
                return None;
            }
            dir
        } else if let Some(dir) = bfs_next_dir(&st, goal, target, true, false) {
            dir
        } else {
            bfs_next_dir(&st, goal, target, false, false)?
        };

        let (ns, recover_ops, bite_idx) = advance_with_restore_queue(
            &st,
            input,
            dir,
            target,
            ell,
            &mut restore_queue,
            &mut dropped,
        )?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(dir as Dir);
        ops.extend_from_slice(&recover_ops);
    }

    if st.len() < ell {
        return None;
    }
    Some(BeamState { state: st, ops })
}

fn choose_shrink_dir(st: &State, input: &Input, ell: usize, target: Cell) -> Option<usize> {
    let anchor_idx = (ell.saturating_sub(1)).min(st.len() - 1);
    let anchor = st.pos[anchor_idx];

    let mut best_bite: Option<((usize, usize, usize, usize), usize)> = None;
    let mut best_move: Option<((usize, usize, usize, usize), usize)> = None;
    let mut dropped = DroppedBuf::new();

    let dirs = legal_dirs(st);
    for &dir_u8 in dirs.as_slice() {
        let dir = dir_u8 as usize;
        let nh = next_head_cell(st, dir).unwrap();
        if nh == target {
            continue;
        }

        let (sim, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
        if sim.food[target as usize] == 0 {
            continue;
        }

        if !prefix_ok(&sim, input, ell) {
            continue;
        }

        let target_dist = manhattan(&input.manhattan, sim.head(), target);
        let anchor_dist = manhattan(&input.manhattan, sim.head(), anchor);

        if bite_idx.is_some() {
            let under = usize::from(sim.len() < ell);
            let dist_len = sim.len().abs_diff(ell);
            let not_ready = usize::from(!can_reach_target_next(&sim, target));
            let key = (under, dist_len, not_ready, target_dist + anchor_dist);
            if best_bite.as_ref().map_or(true, |(k, _)| key < *k) {
                best_bite = Some((key, dir));
            }
        } else {
            let len_gap = sim.len().abs_diff(ell);
            let not_ready = usize::from(!can_reach_target_next(&sim, target));
            let ate_penalty = usize::from(ate != 0);
            let key = (len_gap, not_ready, target_dist + anchor_dist, ate_penalty);
            if best_move.as_ref().map_or(true, |(k, _)| key < *k) {
                best_move = Some((key, dir));
            }
        }
    }

    if let Some((_, dir)) = best_bite {
        return Some(dir);
    }
    if let Some((_, dir)) = best_move {
        return Some(dir);
    }
    None
}

fn shrink_to_ell(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target: Cell,
    target_color: u8,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();

    if st.len() == ell {
        return can_reach_target_next(&st, target).then_some(BeamState { state: st, ops });
    }

    let mut restore_queue = DroppedQueue::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 3;
    let mut guard = 0usize;
    let mut dropped = DroppedBuf::new();

    while st.len() != ell || !restore_queue.is_empty() || !can_reach_target_next(&st, target) {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 60 {
            return None;
        }

        let key = make_visit_key(&st, target, restore_queue.len);
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if st.len() >= 2 && front.cell == st.neck() {
                return None;
            }
            dir
        } else {
            choose_shrink_dir(&st, input, ell, target)?
        };

        let (ns, recover_ops, bite_idx) = advance_with_restore_queue(
            &st,
            input,
            dir,
            target,
            ell,
            &mut restore_queue,
            &mut dropped,
        )?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(dir as Dir);
        ops.extend_from_slice(&recover_ops);
    }

    if st.len() == ell
        && st.food[target as usize] == target_color
        && can_reach_target_next(&st, target)
    {
        Some(BeamState { state: st, ops })
    } else {
        None
    }
}

fn finish_eat_target(bs: &BeamState, input: &Input, ell: usize, target: Cell) -> Option<BeamState> {
    let st = &bs.state;
    let mut ops = bs.ops.clone();
    let dir = dir_between_cells(st.n, st.head(), target)?;
    if st.len() >= 2 && target == st.neck() {
        return None;
    }

    let (ns, _, bite_idx) = step(st, dir);
    if bite_idx.is_some() {
        return None;
    }
    if ns.len() >= ell + 1 && exact_prefix(&ns, input, ell + 1) {
        ops.push(dir as Dir);
        Some(BeamState { state: ns, ops })
    } else {
        None
    }
}

fn try_target_empty_path(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target: Cell,
    started: &Instant,
) -> Option<BeamState> {
    let st = &bs.state;
    if !exact_prefix(st, input, ell) {
        return None;
    }
    if st.food[target as usize] != input.d[ell] {
        return None;
    }
    if remaining_food_count(st) > EMPTY_PATH_REMAINING_LIMIT
        || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
    {
        return None;
    }

    if can_reach_target_next(st, target) {
        return finish_eat_target(bs, input, ell, target);
    }
    if reachable_goal_neighbor_count_pos(st.n, &st.pos, target) > 0 {
        return None;
    }
    if collect_food_cells(st, input.d[ell]).len() != 1 {
        return None;
    }

    let mut nodes = Vec::with_capacity(EMPTY_PATH_EXPANSION_CAP.min(120_000) + 8);
    nodes.push(PosNode {
        pos: st.pos.clone(),
        parent: None,
        mv: 0,
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((
        empty_path_rank(st.n, &st.pos, target, &input.manhattan),
        0usize,
        uid,
        0usize,
    )));
    uid += 1;

    let mut seen = FxHashMap::<InternalPosDeque, usize>::default();
    seen.insert(st.pos.clone(), 0);
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= EMPTY_PATH_EXPANSION_CAP
            || time_over(started)
            || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
        {
            break;
        }
        expansions += 1;
        let pos = nodes[idx].pos.clone();

        if can_reach_target_next_pos(st.n, &pos, target) {
            let mut rev = Vec::new();
            let mut cur = idx;
            while let Some(parent) = nodes[cur].parent {
                rev.push(nodes[cur].mv);
                cur = parent;
            }
            rev.reverse();

            let mut state = st.clone();
            let mut ops = bs.ops.clone();
            for dir_u8 in rev {
                let dir = dir_u8 as usize;
                let (ns, ate, bite_idx) = step(&state, dir);
                if ate != 0 || bite_idx.is_some() {
                    return None;
                }
                state = ns;
                ops.push(dir as Dir);
            }
            let gate_bs = BeamState { state, ops };
            let out = finish_eat_target(&gate_bs, input, ell, target);
            if out.is_some() {
            }
            return out;
        }

        if depth >= EMPTY_PATH_DEPTH_LIMIT {
            continue;
        }

        for dir in 0..4 {
            let Some(next_pos) = empty_step_pos(st.n, &st.food, &pos, dir, target) else {
                continue;
            };
            let nd = depth + 1;
            if seen.get(&next_pos).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(next_pos.clone(), nd);
            let child = nodes.len();
            nodes.push(PosNode {
                pos: next_pos.clone(),
                parent: Some(idx),
                mv: dir as u8,
            });
            pq.push(Reverse((
                empty_path_rank(st.n, &next_pos, target, &input.manhattan),
                nd,
                uid,
                child,
            )));
            uid += 1;
        }
    }

    None
}

fn try_target_exact(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target: Cell,
    target_color: u8,
    started: &Instant,
) -> Vec<BeamState> {
    let head = bs.state.head();
    let mut cand = neighbors(bs.state.n, target);
    cand.as_mut_slice().sort_unstable_by_key(|&cid| {
        (
            usize::from(bs.state.food[cid as usize] > 0),
            manhattan(&input.manhattan, head, cid),
        )
    });

    let mut sols = Vec::new();

    for &goal in cand.as_slice() {
        if time_over(started) {
            break;
        }

        if let Some(b1) = navigate_to_goal_safe(bs, goal, target, started) {
            if let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started) {
                if let Some(b3) = finish_eat_target(&b2, input, ell, target) {
                    sols.push(b3);
                }
            }
        }

        if let Some(b1) = navigate_to_goal_loose(bs, input, goal, target, ell, started) {
            if let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started) {
                if let Some(b3) = finish_eat_target(&b2, input, ell, target) {
                    sols.push(b3);
                }
            }
        }
    }

    if sols.is_empty() && is_endgame_mode(&bs.state, input, ell) {
        if let Some(sol) = try_target_empty_path(bs, input, ell, target, started) {
            sols.push(sol);
        }
    }

    sols.sort_unstable_by_key(|x| x.ops.len());
    let mut out = Vec::new();
    let mut seen: FxHashSet<State> = FxHashSet::default();
    for s in sols {
        if seen.insert(s.state.clone()) {
            out.push(s);
        }
    }
    out
}

#[inline]
fn is_endgame_mode(st: &State, input: &Input, ell: usize) -> bool {
    input.m - ell <= ENDGAME_ELL_LEFT && remaining_food_count(st) <= ENDGAME_REMAINING_FOOD
}

fn collect_exact_solutions(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target_color: u8,
    max_targets: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let mut sols = Vec::new();
    let mut targets = collect_food_cells(&bs.state, target_color);
    targets
        .as_mut_slice()
        .sort_unstable_by_key(|&cid| manhattan(&input.manhattan, bs.state.head(), cid));
    if targets.len() > max_targets {
        targets.truncate(max_targets);
    }
    for &target in targets.as_slice() {
        if time_over(started) {
            break;
        }
        let cand = try_target_exact(bs, input, ell, target, target_color, started);
        for s in cand {
            sols.push(s);
        }
        if sols.len() >= STAGE_BEAM {
            break;
        }
    }
    sols
}

fn collect_exact_solutions_turn_focused(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target_color: u8,
    max_targets: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let mut sols = Vec::new();
    let mut targets = collect_food_cells(&bs.state, target_color);
    targets
        .as_mut_slice()
        .sort_unstable_by_key(|&cid| target_candidate_rank(&bs.state, input, ell, cid));
    if targets.len() > max_targets {
        targets.truncate(max_targets);
    }

    for &target in targets.as_slice() {
        if time_over(started) {
            break;
        }
        let cand = try_target_exact(bs, input, ell, target, target_color, started);
        for s in cand {
            sols.push(s);
        }
        if sols.len() >= SUFFIX_STAGE_BEAM * 2 {
            break;
        }
    }

    sols.sort_unstable_by_key(|bs| {
        (
            turn_focus_next_stage_rank(&bs.state, input, ell + 1),
            bs.ops.len(),
        )
    });

    let mut out = Vec::with_capacity(SUFFIX_STAGE_BEAM);
    let mut seen: FxHashSet<State> = FxHashSet::default();
    for bs in sols {
        if seen.insert(bs.state.clone()) {
            out.push(bs);
            if out.len() >= SUFFIX_STAGE_BEAM {
                break;
            }
        }
    }
    out
}

#[inline]
fn insert_best_plan(map: &mut FxHashMap<State, Ops>, state: State, ops: Ops) {
    if let Some(prev_ops) = map.get_mut(&state) {
        if ops.len() < prev_ops.len() {
            *prev_ops = ops;
        }
    } else {
        map.insert(state, ops);
    }
}

#[inline]
fn map_into_beamstates(map: FxHashMap<State, Ops>) -> Vec<BeamState> {
    map.into_iter()
        .map(|(state, ops)| BeamState { state, ops })
        .collect()
}

fn rescue_stage(
    beam: &[BeamState],
    input: &Input,
    ell: usize,
    target_color: u8,
    started: &Instant,
) -> Vec<BeamState> {
    let mut order: Vec<usize> = (0..beam.len()).collect();
    order.sort_unstable_by_key(|&idx| {
        (
            local_score(&beam[idx].state, input, ell),
            beam[idx].ops.len(),
        )
    });

    let mut rescue_map: FxHashMap<State, Ops> = FxHashMap::default();
    for &idx in &order {
        if time_over(started) {
            break;
        }

        let bs = &beam[idx];
        let endgame_mode = is_endgame_mode(&bs.state, input, ell);

        let mut sols = if endgame_mode {
            collect_exact_solutions(bs, input, ell, target_color, MAX_TARGETS_RESCUE, started)
        } else {
            stage_search_bestfirst(bs, input, ell, &BUDGETS_RESCUE, STAGE_BEAM, started)
        };

        if sols.is_empty() && !time_over(started) {
            if endgame_mode {
                sols = stage_search_bestfirst(
                    bs,
                    input,
                    ell,
                    &BUDGETS_ENDGAME_LIGHT,
                    STAGE_BEAM,
                    started,
                );
            } else {
                sols = collect_exact_solutions(
                    bs,
                    input,
                    ell,
                    target_color,
                    MAX_TARGETS_RESCUE,
                    started,
                );
            }
        }

        for s in sols {
            if s.ops.len() > MAX_TURNS {
                continue;
            }
            insert_best_plan(&mut rescue_map, s.state, s.ops);
        }

        if rescue_map.len() >= STAGE_BEAM * 2 {
            break;
        }
    }

    let mut out = map_into_beamstates(rescue_map);
    out.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, input, ell + 1), bs.ops.len()));
    if out.len() > STAGE_BEAM {
        out.truncate(STAGE_BEAM);
    }
    out
}

fn trim_stage_beam(
    cands: Vec<BeamState>,
    input: &Input,
    next_ell: usize,
    short_lane: Option<&BeamState>,
) -> Vec<BeamState> {

    let mut strategic_order: Vec<usize> = (0..cands.len()).collect();
    strategic_order.sort_unstable_by_key(|&idx| {
        (
            next_stage_rank(&cands[idx].state, input, next_ell),
            cands[idx].ops.len(),
        )
    });

    let mut turn_order: Vec<usize> = (0..cands.len()).collect();
    turn_order.sort_unstable_by_key(|&idx| {
        (
            turn_focus_next_stage_rank(&cands[idx].state, input, next_ell),
            cands[idx].ops.len(),
        )
    });

    let best_short = cands.iter().min_by_key(|bs| bs.ops.len()).cloned();
    let best_turn = cands
        .iter()
        .min_by_key(|bs| {
            (
                turn_focus_next_stage_rank(&bs.state, input, next_ell),
                bs.ops.len(),
            )
        })
        .cloned();
    let best_simple = cands
        .iter()
        .min_by_key(|bs| simple_beam_rank(&bs.state, input, next_ell, bs.ops.len()))
        .cloned();

    let mut out = Vec::with_capacity(STAGE_BEAM);
    let mut seen: FxHashSet<State> = FxHashSet::default();

    if let Some(bs) = short_lane {
        if seen.insert(bs.state.clone()) {
            out.push(bs.clone());
        }
    }

    if let Some(bs) = best_short {
        if seen.insert(bs.state.clone()) {
            out.push(bs);
        }
    }

    if let Some(bs) = best_turn {
        if seen.insert(bs.state.clone()) {
            out.push(bs);
        }
    }

    if let Some(bs) = best_simple {
        if seen.insert(bs.state.clone()) {
            out.push(bs);
        }
    }

    for idx in strategic_order {
        let bs = &cands[idx];
        if seen.insert(bs.state.clone()) {
            out.push(bs.clone());
            if out.len() >= STAGE_BEAM {
                break;
            }
        }
    }

    if out.len() < STAGE_BEAM {
        for idx in turn_order {
            let bs = &cands[idx];
            if seen.insert(bs.state.clone()) {
                out.push(bs.clone());
                if out.len() >= STAGE_BEAM {
                    break;
                }
            }
        }
    }
    out
}

fn solve_base(input: &Input, started: &Instant) -> BeamState {
    let init = BeamState {
        state: State::initial(input),
        ops: Ops::new(),
    };
    let mut beam = vec![init];

    for ell in 5..input.m {
        if time_over(started) {
            break;
        }

        let target_color = input.d[ell];
        let budgets: &[(usize, usize)] = if input.m - ell < 10 {
            &BUDGETS_LATE
        } else {
            &BUDGETS_NORMAL
        };

        let short_seed = beam.iter().min_by_key(|bs| bs.ops.len()).cloned();
        let quick_short = short_seed
            .as_ref()
            .and_then(|bs| extend_fastlane_one(bs, input, ell, started));

        let mut new_map: FxHashMap<State, Ops> = FxHashMap::default();
        if let Some(sol) = quick_short.clone() {
            insert_best_plan(&mut new_map, sol.state, sol.ops);
        }

        for bs in &beam {
            if time_over(started) {
                break;
            }

            if !time_over(started) {
                let simple_children =
                    expand_simple_children(bs, input, ell, SIMPLE_INJECT_PER_STATE);
                for s in simple_children {
                    if s.ops.len() > MAX_TURNS {
                        continue;
                    }
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }

                let bite_children = expand_bite_children(bs, input, ell, BITE_CANDIDATE_WIDTH);
                for s in bite_children {
                    if s.ops.len() > MAX_TURNS {
                        continue;
                    }
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }
            }

            let endgame_mode = is_endgame_mode(&bs.state, input, ell);
            let mut sols = Vec::new();

            if !time_over(started) {
                if endgame_mode {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        ell,
                        target_color,
                        MAX_TARGETS_ENDGAME,
                        started,
                    );
                } else {
                    sols = stage_search_bestfirst(bs, input, ell, budgets, STAGE_BEAM, started);
                }
            }

            if sols.is_empty() && !time_over(started) {
                if endgame_mode {
                    sols = stage_search_bestfirst(
                        bs,
                        input,
                        ell,
                        &BUDGETS_ENDGAME_LIGHT,
                        STAGE_BEAM,
                        started,
                    );
                } else {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        ell,
                        target_color,
                        MAX_TARGETS_PER_STAGE,
                        started,
                    );
                }
            }

            for s in sols {
                if s.ops.len() > MAX_TURNS {
                    continue;
                }
                insert_best_plan(&mut new_map, s.state, s.ops);
            }
        }

        if new_map.is_empty() && !time_over(started) {
            let rescue = rescue_stage(&beam, input, ell, target_color, started);
            for s in rescue {
                insert_best_plan(&mut new_map, s.state, s.ops);
            }
        }

        if new_map.is_empty() {
            break;
        }

        let new_beam = trim_stage_beam(
            map_into_beamstates(new_map),
            input,
            ell + 1,
            quick_short.as_ref(),
        );
        beam = new_beam;
    }

    if beam.is_empty() {
        return BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
    }

    let mut best = choose_best_beamstate(beam, input);
    if best.ops.len() > MAX_TURNS {
        best.ops.truncate(MAX_TURNS);
    }
    best
}

fn reconstruct_exact_checkpoints(input: &Input, ops: &[Dir]) -> Vec<Option<(usize, State)>> {
    let mut checkpoints = vec![None; input.m + 1];
    let mut st = State::initial(input);
    checkpoints[5] = Some((0, st.clone()));
    let mut ell = 5usize;

    for (t, &dir_u8) in ops.iter().enumerate() {
        let dir = dir_u8 as usize;
        if !is_legal_dir(&st, dir) {
            break;
        }
        let (ns, _, _) = step(&st, dir);
        st = ns;
        if ell < input.m && exact_prefix(&st, input, ell + 1) {
            ell += 1;
            checkpoints[ell] = Some((t + 1, st.clone()));
            if ell == input.m {
                break;
            }
        }
    }

    checkpoints
}

fn is_complete_exact(bs: &BeamState, input: &Input) -> bool {
    bs.state.len() == input.m
        && exact_prefix(&bs.state, input, input.m)
        && remaining_food_count(&bs.state) == 0
}

fn solve_suffix_turn_focused(
    input: &Input,
    start_bs: BeamState,
    start_ell: usize,
    started: &Instant,
) -> BeamState {
    let mut beam = vec![start_bs.clone()];

    for ell in start_ell..input.m {
        if time_over(started) || time_left(started) < 0.02 {
            break;
        }

        let target_color = input.d[ell];
        let mut new_map: FxHashMap<State, Ops> = FxHashMap::default();

        for bs in &beam {
            if time_over(started) {
                break;
            }

            if !time_over(started) {
                let simple_children =
                    expand_simple_children(bs, input, ell, SIMPLE_INJECT_PER_STATE);
                for s in simple_children {
                    if s.ops.len() > MAX_TURNS {
                        continue;
                    }
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }

                let bite_children = expand_bite_children(bs, input, ell, BITE_CANDIDATE_WIDTH);
                for s in bite_children {
                    if s.ops.len() > MAX_TURNS {
                        continue;
                    }
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }
            }

            let mut sols = collect_exact_solutions_turn_focused(
                bs,
                input,
                ell,
                target_color,
                SUFFIX_OPT_TARGETS,
                started,
            );

            if sols.is_empty() && !time_over(started) {
                sols = stage_search_bestfirst(
                    bs,
                    input,
                    ell,
                    &BUDGETS_ENDGAME_LIGHT,
                    SUFFIX_STAGE_BEAM,
                    started,
                );
            }
            if sols.is_empty() && !time_over(started) {
                sols = stage_search_bestfirst(
                    bs,
                    input,
                    ell,
                    &BUDGETS_LATE,
                    SUFFIX_STAGE_BEAM,
                    started,
                );
            }
            if sols.is_empty() && !time_over(started) {
                sols = collect_exact_solutions(
                    bs,
                    input,
                    ell,
                    target_color,
                    MAX_TARGETS_ENDGAME,
                    started,
                );
            }

            for s in sols {
                if s.ops.len() > MAX_TURNS {
                    continue;
                }
                insert_best_plan(&mut new_map, s.state, s.ops);
            }
        }

        if new_map.is_empty() {
            break;
        }

        let mut new_beam = map_into_beamstates(new_map);
        new_beam.sort_unstable_by_key(|bs| {
            (
                turn_focus_next_stage_rank(&bs.state, input, ell + 1),
                bs.ops.len(),
            )
        });
        if new_beam.len() > SUFFIX_STAGE_BEAM {
            new_beam.truncate(SUFFIX_STAGE_BEAM);
        }
        beam = new_beam;
    }

    if beam.is_empty() {
        return start_bs;
    }

    choose_best_beamstate(beam, input)
}

fn optimize_exact_suffix(input: &Input, base: BeamState, started: &Instant) -> BeamState {
    if !is_complete_exact(&base, input) {
        return base;
    }

    let mut best = base;

    for &window in &SUFFIX_OPT_WINDOWS {
        if time_left(started) < SUFFIX_OPT_MIN_LEFT_SEC {
            break;
        }

        let checkpoints = reconstruct_exact_checkpoints(input, &best.ops);
        let start_ell = input.m.saturating_sub(window).max(5);
        let Some((prefix_turns, st)) = checkpoints[start_ell].clone() else {
            continue;
        };

        let prefix_ops = best.ops[..prefix_turns].to_vec();
        let start_bs = BeamState {
            state: st,
            ops: prefix_ops,
        };

        let cand = solve_suffix_turn_focused(input, start_bs, start_ell, started);
        if is_complete_exact(&cand, input) && cand.ops.len() < best.ops.len() {
            best = cand;
        }
    }

    best
}

fn optimize_exact_suffix_with_simple(
    input: &Input,
    base: BeamState,
    started: &Instant,
) -> BeamState {
    if !is_complete_exact(&base, input) {
        return base;
    }

    let mut best = base;

    for &window in &SIMPLE_SUFFIX_WINDOWS {
        if time_left(started) < SIMPLE_SUFFIX_MIN_LEFT_SEC {
            break;
        }

        let checkpoints = reconstruct_exact_checkpoints(input, &best.ops);
        let start_ell = input.m.saturating_sub(window).max(5);
        let Some((prefix_turns, st)) = checkpoints[start_ell].clone() else {
            continue;
        };

        let start_bs = BeamState {
            state: st,
            ops: best.ops[..prefix_turns].to_vec(),
        };

        let cand = solve_simple_beam_from(
            input,
            start_bs,
            start_ell,
            SIMPLE_STAGE_BEAM,
            SIMPLE_CANDIDATE_WIDTH,
            started,
            SIMPLE_SUFFIX_MIN_LEFT_SEC,
        );
        if is_complete_exact(&cand, input) && cand.ops.len() < best.ops.len() {
            best = cand;
        }
    }

    best
}

fn solve(input: &Input) -> Ops {
    let started = Instant::now();
    let base = solve_base(input, &started);
    let turn_opt = optimize_exact_suffix(input, base.clone(), &started);
    let simple_opt = optimize_exact_suffix_with_simple(input, turn_opt.clone(), &started);
    let mut best = choose_best_beamstate(vec![base, turn_opt, simple_opt], input);
    if best.ops.len() > MAX_TURNS {
        best.ops.truncate(MAX_TURNS);
    }
    best.ops
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
