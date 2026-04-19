// v013_bite_beam.rs
// 考察メモ:
// - eval(all): total_avg=342929, avg_elapsed=0.280140, slowest_elapsed_ms=755。
// - いくつかのケースでは最短手が出た。
// - 一方で v012_simple_beam と比べると数手悪化するケースも多かった。
// - beam の種類を増やしたからと言って、単純な上位互換になるとは限らない。
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::ops::Deref;
use std::ops::Index;

const INTERNAL_COLOR_CAPACITY: usize = 16 * 16;
const INTERNAL_COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const INTERNAL_COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
const INTERNAL_POS_DEQUE_CAPACITY: usize = 16 * 16;
const INTERNAL_POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
const INTERNAL_POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const MAX_TURNS: usize = 100_000;
const BEAM_WIDTH: usize = 96;
const DIRECT_CANDIDATE_WIDTH: usize = 8;
// packed local path は u16 に 2bit/手で詰めるので、BITE_DEPTH_LIMIT は 8 以下に保つこと。
const BITE_DEPTH_LIMIT: usize = 6;
const BITE_CANDIDATE_WIDTH: usize = 4;
type Dir = u8;
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
        Self::try_dir_between_cells(n, from, to).unwrap_or_else(|| {
            unreachable!(
                "cells are not adjacent: from={}, to={}",
                Self::index(from),
                Self::index(to)
            )
        })
    }

    #[inline]
    fn try_dir_between_cells(n: usize, from: Cell, to: Cell) -> Option<Dir> {
        let from_idx = Self::index(from);
        let to_idx = Self::index(to);
        if to_idx + n == from_idx {
            Some(0)
        } else if from_idx + n == to_idx {
            Some(1)
        } else if to_idx + 1 == from_idx && from_idx / n == to_idx / n {
            Some(2)
        } else if from_idx + 1 == to_idx && from_idx / n == to_idx / n {
            Some(3)
        } else {
            None
        }
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
    fn back(&self) -> Option<Cell> {
        if self.is_empty() {
            None
        } else {
            Some(self[self.len - 1])
        }
    }
}

impl PartialEq for InternalPosDeque {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self.hash1 == other.hash1
            && self.hash2 == other.hash2
            && (0..self.len).all(|i| self[i] == other[i])
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
        f.debug_list()
            .entries((0..self.len).map(|i| self[i]))
            .finish()
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
        if self.len == 0 {
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
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        for i in 0..pos.len() {
            out.inc(pos[i]);
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: InternalColors,
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
}

#[derive(Debug, Clone)]
struct CellSearchResult {
    start: Cell,
    dist: Vec<usize>,
    prev: Vec<Option<Cell>>,
}

fn compute_body_release_dist(state: &State, n: usize) -> CellSearchResult {
    let cell_count = n * n;
    let inf = usize::MAX;

    // 初期胴体マスは、その Cell を占有する最前の segment が excluded_tail
    // になる時刻以降に通行可とみなす。旧 tail や pos[len - 2] は 1 手後に
    // 通れることがある。
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

        for &dir in &ALL_DIRS {
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
        for &dir in &ALL_DIRS {
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

#[derive(Debug, Clone, Copy)]
struct Dropped {
    cell: Cell,
    color: u8,
}

#[derive(Debug, Clone)]
struct StepResult {
    state: State,
    ate: Option<u8>,
    bite_idx: Option<usize>,
    dropped: Vec<Dropped>,
}

#[derive(Debug, Clone)]
struct PrefixRepairResult {
    state: State,
    ops: Ops,
    repaired: bool,
}

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
    if dropped.len() < need {
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
        let Some(dir) = Grid::try_dir_between_cells(n, prev, ent.cell) else {
            return PrefixRepairResult {
                state: st_after.clone(),
                ops: Ops::new(),
                repaired: false,
            };
        };
        ops.push(dir);

        debug_assert_eq!(food[Grid::index(ent.cell)], ent.color);
        food[Grid::index(ent.cell)] = 0;
        pos.push_front(ent.cell);
        pos_occupancy.inc(ent.cell);
        prev = ent.cell;
    }

    PrefixRepairResult {
        state: State {
            food,
            pos,
            colors: InternalColors::from_slice(prefix_target),
            pos_occupancy,
        },
        ops,
        repaired: true,
    }
}

fn step(state: &State, n: usize, dir: Dir) -> StepResult {
    debug_assert!(state.is_legal_dir(n, dir));

    let next_head = Grid::next_cell(n, state.head(), dir);

    let mut food = state.food.clone();
    let mut new_pos = state.pos.clone();
    let mut new_colors = state.colors.clone();
    let mut new_pos_occupancy = state.pos_occupancy.clone();

    let eat_idx = Grid::index(next_head);
    let ate = if food[eat_idx] != 0 {
        let food_color = food[eat_idx];
        food[eat_idx] = 0;
        new_colors.push(food_color);
        Some(food_color)
    } else {
        let old_tail = new_pos.pop_back().unwrap();
        new_pos_occupancy.dec(old_tail);
        None
    };

    let excluded_tail = new_pos.back();
    let tail_bias = u8::from(excluded_tail == Some(next_head));
    let bite = new_pos_occupancy.count(next_head) > tail_bias;

    new_pos_occupancy.inc(next_head);
    new_pos.push_front(next_head);
    let mut bite_idx = None;
    let mut dropped = Vec::new();
    if let Some(h) = if bite {
        find_internal_bite_idx(&new_pos)
    } else {
        None
    } {
        bite_idx = Some(h);
        while new_pos.len() > h + 1 {
            let cell = new_pos.pop_back().unwrap();
            new_pos_occupancy.dec(cell);
            let color = new_colors.pop().unwrap();
            food[Grid::index(cell)] = color;
            dropped.push(Dropped { cell, color });
        }
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

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    m: usize,
    d: Vec<u8>,
    food: Vec<u8>,
}

#[derive(Clone)]
struct BeamNode {
    state: State,
    ops: Ops,
}

type Score = (usize, usize, usize);

struct ScoredNode {
    node: BeamNode,
    score: Score,
}

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _color_count: usize = it.next().unwrap().parse().unwrap();

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

    Input { n, m, d, food }
}

fn solve_beam(input: &Input) -> Ops {
    let mut beam = vec![BeamNode {
        state: State::initial(input),
        ops: Ops::new(),
    }];
    let mut ell = 5;

    while ell < input.m {
        let mut all_children = Vec::new();
        for parent in &beam {
            all_children.extend(expand_one_node(parent, input, ell));
        }
        if all_children.is_empty() {
            break;
        }

        all_children.sort_by(|a, b| a.score.cmp(&b.score));
        if all_children.len() > BEAM_WIDTH {
            all_children.truncate(BEAM_WIDTH);
        }

        beam = all_children.into_iter().map(|scored| scored.node).collect();
        ell += 1;
    }

    select_best_node(&beam, input, ell).ops.clone()
}

fn collect_top_k_target_paths(
    state: &State,
    n: usize,
    target_color: u8,
    k: usize,
) -> Vec<Vec<Cell>> {
    let bfs = compute_body_release_dist(state, n);
    let eat_dist = body_release_eat_dist(&bfs, state, n);

    let mut foods = Vec::new();
    for idx in 0..(n * n) {
        if state.food[idx] != target_color {
            continue;
        }
        let dist = eat_dist.dist[idx];
        if dist == usize::MAX {
            continue;
        }
        foods.push((dist, idx));
    }

    foods.sort_by_key(|&(dist, idx)| (dist, idx));
    if foods.len() > k {
        foods.truncate(k);
    }

    let mut paths = Vec::with_capacity(foods.len());
    for (_, idx) in foods {
        if let Some(path) = reconstruct_cell_search_path(&eat_dist, Cell(idx as u16)) {
            paths.push(path);
        }
    }
    paths
}

fn collect_direct_children(parent: &BeamNode, input: &Input, ell: usize) -> Vec<BeamNode> {
    let target_color = input.d[ell];
    let paths =
        collect_top_k_target_paths(&parent.state, input.n, target_color, DIRECT_CANDIDATE_WIDTH);

    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let Some(child) = build_child_from_base(&parent.state, &parent.ops, input.n, &path) else {
            continue;
        };
        out.push(child);
    }
    out
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
    pos_len: usize,
    pos_hash1: u64,
    pos_hash2: u64,
    colors_len: usize,
    colors_hash1: u64,
    colors_hash2: u64,
}

#[inline]
fn local_visited_key(state: &State) -> LocalVisitedKey {
    LocalVisitedKey {
        pos_len: state.pos.len(),
        pos_hash1: state.pos.hash1,
        pos_hash2: state.pos.hash2,
        colors_len: state.colors.len(),
        colors_hash1: state.colors.hash1,
        colors_hash2: state.colors.hash2,
    }
}

#[inline]
fn append_local_dir(path_bits: u16, depth: u8, dir: Dir) -> u16 {
    // depth >= 8 は u16 に収まらない。release では debug_assert! で守れないので、定数変更時は型幅も見直す。
    debug_assert!(dir < DIR_COUNT as Dir);
    debug_assert!((depth as usize) < BITE_DEPTH_LIMIT);
    path_bits | ((dir as u16) << (2 * depth))
}

#[inline]
fn extend_ops_with_local_path(ops: &mut Ops, path_bits: u16, depth: u8) {
    // append_local_dir と同じ packed 形式を decode する。壊れた path_bits を渡すと不正手に直結する。
    for i in 0..depth {
        ops.push(((path_bits >> (2 * i)) & 3) as Dir);
    }
}

fn collect_first_bite_children(parent: &BeamNode, input: &Input, ell: usize) -> Vec<BeamNode> {
    let target_color = input.d[ell];
    let prefix_target = &input.d[..ell];

    let mut out = Vec::new();
    let mut q = VecDeque::new();
    let mut visited = HashSet::new();

    q.push_back(LocalBiteNode {
        state: parent.state.clone(),
        path_bits: 0,
        depth: 0,
    });
    visited.insert(local_visited_key(&parent.state));

    while let Some(node) = q.pop_front() {
        if out.len() >= BITE_CANDIDATE_WIDTH {
            break;
        }

        if usize::from(node.depth) == BITE_DEPTH_LIMIT {
            continue;
        }

        for &dir in &ALL_DIRS {
            if !node.state.is_legal_dir(input.n, dir) {
                continue;
            }

            let step_result = step(&node.state, input.n, dir);
            if step_result.ate == Some(target_color) {
                continue;
            }

            let next_depth = node.depth + 1;
            let next_path_bits = append_local_dir(node.path_bits, node.depth, dir);

            if step_result.bite_idx.is_none() {
                if visited.insert(local_visited_key(&step_result.state)) {
                    q.push_back(LocalBiteNode {
                        state: step_result.state,
                        path_bits: next_path_bits,
                        depth: next_depth,
                    });
                }
                continue;
            }

            if step_result.state.colors.len() > ell {
                continue;
            }
            if step_result.state.colors.len() + step_result.dropped.len() < ell {
                continue;
            }

            let repaired = repair_prefix_after_bite(
                &step_result.state,
                input.n,
                prefix_target,
                &step_result.dropped,
            );
            if repaired.state.colors.len() != ell {
                continue;
            }
            debug_assert_eq!(repaired.repaired, step_result.state.colors.len() < ell);

            let Some(path) =
                find_path_to_nearest_target_food(&repaired.state, input.n, target_color)
            else {
                continue;
            };

            let Some(child) = build_child_from_bite_base(
                parent,
                &repaired.state,
                next_path_bits,
                next_depth,
                &repaired.ops,
                input.n,
                &path,
            ) else {
                continue;
            };
            out.push(child);

            if out.len() >= BITE_CANDIDATE_WIDTH {
                break;
            }
        }
    }

    out
}

fn expand_one_node(parent: &BeamNode, input: &Input, ell: usize) -> Vec<ScoredNode> {
    let mut children = Vec::new();
    children.extend(collect_direct_children(parent, input, ell));
    children.extend(collect_first_bite_children(parent, input, ell));

    let mut out = Vec::with_capacity(children.len());
    for child in children {
        let score = evaluate_node(&child.state, input, ell + 1, child.ops.len());
        out.push(ScoredNode { node: child, score });
    }
    out
}

fn build_child_from_base(
    base_state: &State,
    base_ops: &[Dir],
    n: usize,
    path: &[Cell],
) -> Option<BeamNode> {
    let mut child_state = base_state.clone();
    let mut child_ops = base_ops.to_vec();
    if !apply_path_with_turn_cap(&mut child_state, &mut child_ops, n, path, MAX_TURNS) {
        return None;
    }

    Some(BeamNode {
        state: child_state,
        ops: child_ops,
    })
}

fn build_child_from_bite_base(
    parent: &BeamNode,
    repaired_state: &State,
    local_path_bits: u16,
    local_depth: u8,
    repaired_ops: &[Dir],
    n: usize,
    path: &[Cell],
) -> Option<BeamNode> {
    let mut child_state = repaired_state.clone();
    let mut child_ops = parent.ops.clone();
    extend_ops_with_local_path(&mut child_ops, local_path_bits, local_depth);
    child_ops.extend_from_slice(repaired_ops);
    if !apply_path_with_turn_cap(&mut child_state, &mut child_ops, n, path, MAX_TURNS) {
        return None;
    }

    Some(BeamNode {
        state: child_state,
        ops: child_ops,
    })
}

fn evaluate_node(state: &State, input: &Input, next_ell: usize, ops_len: usize) -> Score {
    if next_ell == input.m {
        return (0, ops_len, 0);
    }

    let next_color = input.d[next_ell];
    let Some(path) = find_path_to_nearest_target_food(state, input.n, next_color) else {
        return (1, ops_len, usize::MAX);
    };
    (0, ops_len, path.len().saturating_sub(1))
}

fn find_path_to_nearest_target_food(
    state: &State,
    n: usize,
    target_color: u8,
) -> Option<Vec<Cell>> {
    collect_top_k_target_paths(state, n, target_color, 1)
        .into_iter()
        .next()
}

fn select_best_node<'a>(beam: &'a [BeamNode], input: &Input, ell: usize) -> &'a BeamNode {
    debug_assert!(!beam.is_empty());

    let mut best_idx = 0;
    let mut best_score = evaluate_node(&beam[0].state, input, ell, beam[0].ops.len());
    for (idx, node) in beam.iter().enumerate().skip(1) {
        let score = evaluate_node(&node.state, input, ell, node.ops.len());
        if score < best_score {
            best_score = score;
            best_idx = idx;
        }
    }
    &beam[best_idx]
}

fn apply_path_with_turn_cap(
    state: &mut State,
    out: &mut Ops,
    n: usize,
    path: &[Cell],
    turn_cap: usize,
) -> bool {
    for pair in path.windows(2) {
        if out.len() == turn_cap {
            return false;
        }
        let dir = Grid::dir_between_cells(n, pair[0], pair[1]);
        let step_result = step(state, n, dir);
        *state = step_result.state;
        out.push(dir);
    }
    true
}

fn main() {
    let input = read_input();
    let out = solve_beam(&input);
    for dir in out {
        println!("{}", DIR_CHARS[dir as usize]);
    }
}
