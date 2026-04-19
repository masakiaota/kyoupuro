// v125_faster123.rs
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::fmt;
use std::io::{self, Read};
use std::ops::Index;
use std::time::Instant;

#[path = "../profile_counts.rs"]
mod profile_counts;

#[cfg(profile_counts)]
macro_rules! prof_inc {
    ($field:ident) => {{
        unsafe {
            (*profile_counts::counters_ptr()).$field += 1;
        }
    }};
    ($field:ident, $delta:expr) => {{
        unsafe {
            (*profile_counts::counters_ptr()).$field += ($delta) as u64;
        }
    }};
}

#[cfg(not(profile_counts))]
macro_rules! prof_inc {
    ($field:ident) => {};
    ($field:ident, $delta:expr) => {};
}

#[cfg(profile_counts)]
macro_rules! prof_set {
    ($field:ident, $value:expr) => {{
        unsafe {
            (*profile_counts::counters_ptr()).$field = ($value) as u64;
        }
    }};
}

#[cfg(not(profile_counts))]
macro_rules! prof_set {
    ($field:ident, $value:expr) => {};
}

const INTERNAL_POS_DEQUE_CAPACITY: usize = 16 * 16;
const INTERNAL_POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
const INTERNAL_POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
const STAGE_BEAM: usize = 5;
const MAX_TARGETS_PER_STAGE: usize = 10;
const MAX_TARGETS_ENDGAME: usize = 24;
const MAX_TARGETS_RESCUE: usize = 28;
const VISIT_REPEAT_LIMIT: usize = 12;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
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
fn encode_internal_pos_hash(cell: u16) -> u64 {
    cell as u64 + 1
}

#[derive(Clone)]
struct Input {
    n: usize,
    m: usize,
    d: Vec<u8>,
    food: Vec<u8>,
}

#[derive(Clone)]
struct State {
    n: usize,
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: Vec<u8>,
}

#[derive(Clone)]
struct BeamState {
    state: State,
    ops: String,
}

#[derive(Clone)]
struct Dropped {
    cell: u16,
    color: u8,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct Key {
    pos: InternalPosDeque,
    colors: Vec<u8>,
    food: Vec<(u16, u8)>,
}

#[derive(Hash, Eq, PartialEq)]
struct VisitKey {
    head: u16,
    neck: u16,
    len: u16,
    goal: u16,
    restore_len: u16,
}

struct Node {
    state: State,
    parent: Option<usize>,
    move_seg: String,
}

struct PosNode {
    pos: Vec<u16>,
    parent: Option<usize>,
    mv: char,
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
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [u16; INTERNAL_POS_DEQUE_CAPACITY],
    hash1: u64,
    hash2: u64,
}

impl InternalPosDeque {
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [0; INTERNAL_POS_DEQUE_CAPACITY],
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(cells: &[u16]) -> Self {
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
    fn push_front(&mut self, cell: u16) {
        debug_assert!(self.len < INTERNAL_POS_DEQUE_CAPACITY);
        let x = encode_internal_pos_hash(cell);
        self.head = (self.head + INTERNAL_POS_DEQUE_CAPACITY - 1) % INTERNAL_POS_DEQUE_CAPACITY;
        self.buf[self.head] = cell;
        self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(INTERNAL_POS_HASH_BASE1));
        self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(INTERNAL_POS_HASH_BASE2));
        self.len += 1;
    }

    #[inline]
    fn pop_back(&mut self) -> Option<u16> {
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

impl std::hash::Hash for InternalPosDeque {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
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
    type Output = u16;

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
    type Item = u16;

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

impl State {
    fn initial(input: &Input) -> Self {
        let n = input.n;
        let pos = InternalPosDeque::from_slice(&[
            cell_of(4, 0, n),
            cell_of(3, 0, n),
            cell_of(2, 0, n),
            cell_of(1, 0, n),
            cell_of(0, 0, n),
        ]);
        Self {
            n,
            food: input.food.clone(),
            pos: pos.clone(),
            colors: vec![1; 5],
        }
    }

    #[inline]
    fn head(&self) -> u16 {
        self.pos[0]
    }
}

#[inline]
fn find_internal_bite_idx(pos: &InternalPosDeque) -> Option<usize> {
    debug_assert!(!pos.is_empty());
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

#[inline]
fn cell_of(r: usize, c: usize, n: usize) -> u16 {
    (r * n + c) as u16
}

#[inline]
fn rc_of(cell: u16, n: usize) -> (usize, usize) {
    let x = cell as usize;
    (x / n, x % n)
}

#[inline]
fn manhattan(n: usize, a: u16, b: u16) -> usize {
    let (ar, ac) = rc_of(a, n);
    let (br, bc) = rc_of(b, n);
    ar.abs_diff(br) + ac.abs_diff(bc)
}

#[inline]
fn time_over(started: &Instant) -> bool {
    let over = started.elapsed().as_secs_f64() >= TIME_LIMIT_SEC;
    if over {
        prof_inc!(time_over_hits);
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

    let mut d = Vec::with_capacity(m);
    for _ in 0..m {
        d.push(it.next().unwrap().parse::<u8>().unwrap());
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input { n, m, d, food }
}

#[inline]
fn dir_between_cells(n: usize, a: u16, b: u16) -> Option<usize> {
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
fn next_head_cell(st: &State, dir: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(st.head(), st.n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= st.n as isize || nc < 0 || nc >= st.n as isize {
        return None;
    }
    Some(cell_of(nr as usize, nc as usize, st.n))
}

#[inline]
fn is_legal_dir(st: &State, dir: usize) -> bool {
    let Some(nh) = next_head_cell(st, dir) else {
        return false;
    };
    st.pos.len() < 2 || nh != st.pos[1]
}

fn legal_dirs(st: &State) -> Vec<usize> {
    let mut out = Vec::with_capacity(4);
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

fn step(st: &State, dir: usize) -> (State, u8, Option<usize>, Vec<Dropped>) {
    prof_inc!(step_calls);
    let nh = next_head_cell(st, dir).unwrap();
    let mut food = st.food.clone();
    let mut new_pos = st.pos.clone();
    let mut new_colors = st.colors.clone();
    let ate = food[nh as usize];
    if ate != 0 {
        prof_inc!(step_ate);
        food[nh as usize] = 0;
        new_colors.push(ate);
    } else {
        new_pos.pop_back().unwrap();
    }

    new_pos.push_front(nh);
    let bite_idx = find_internal_bite_idx(&new_pos);
    debug_assert!(
        ate == 0 || bite_idx.is_none(),
        "eat and bite must not happen simultaneously"
    );

    let mut dropped = Vec::new();
    if let Some(bi) = bite_idx {
        prof_inc!(step_bite);
        let drop_len = new_pos.len().saturating_sub(bi + 1);
        dropped.reserve(drop_len);
        let mut dropped_rev = Vec::with_capacity(drop_len);
        while new_pos.len() > bi + 1 {
            let cell = new_pos.pop_back().unwrap();
            let color = new_colors.pop().unwrap();
            food[cell as usize] = color;
            dropped_rev.push(Dropped { cell, color });
        }
        dropped_rev.reverse();
        dropped = dropped_rev;
    }

    (
        State {
            n: st.n,
            food,
            pos: new_pos,
            colors: new_colors,
        },
        ate,
        bite_idx,
        dropped,
    )
}

#[inline]
fn lcp(colors: &[u8], d: &[u8]) -> usize {
    let mut i = 0;
    let m = colors.len().min(d.len());
    while i < m && colors[i] == d[i] {
        i += 1;
    }
    i
}

#[inline]
fn prefix_ok(st: &State, d: &[u8], ell: usize) -> bool {
    let keep = st.colors.len().min(ell);
    st.colors[..keep] == d[..keep]
}

#[inline]
fn exact_prefix(st: &State, d: &[u8], ell: usize) -> bool {
    st.colors.len() == ell && st.colors[..ell] == d[..ell]
}

#[inline]
fn remaining_food_count(st: &State) -> usize {
    st.food.iter().filter(|&&c| c != 0).count()
}

fn encode_key(st: &State) -> Key {
    prof_inc!(encode_key_calls);
    let mut food = Vec::with_capacity(st.colors.len() + 16);
    for (idx, &col) in st.food.iter().enumerate() {
        if col != 0 {
            food.push((idx as u16, col));
        }
    }
    Key {
        pos: st.pos.clone(),
        colors: st.colors.clone(),
        food,
    }
}

fn nearest_food_dist(st: &State, color: u8) -> (usize, usize) {
    let head = st.head();
    let mut best = usize::MAX;
    let mut cnt = 0usize;
    for (idx, &col) in st.food.iter().enumerate() {
        if col == color {
            cnt += 1;
            let dist = manhattan(st.n, head, idx as u16);
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
    let neck = st.pos[1];
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

fn target_suffix_info(st: &State, ell: usize, target: u8) -> Option<(usize, usize)> {
    let head = st.head();
    let mut best: Option<(usize, usize)> = None;
    for idx in ell..st.colors.len() {
        if st.colors[idx] != target {
            continue;
        }
        let prev = st.pos[idx - 1];
        let cand = (manhattan(st.n, head, prev), idx);
        if best.is_none() || cand < best.unwrap() {
            best = Some(cand);
        }
    }
    best
}

fn local_score(st: &State, input: &Input, ell: usize) -> (usize, usize, usize, usize, usize) {
    let target = input.d[ell];
    if exact_prefix(st, &input.d, ell) {
        let (dist, _) = nearest_food_dist(st, target);
        let adj = target_adjacent(st, target).is_some();
        return (0, if adj { 0 } else { 1 }, dist, 0, st.colors.len() - ell);
    }

    if let Some((dist, idx)) = target_suffix_info(st, ell, target) {
        return (1, 0, dist, idx - ell, st.colors.len() - ell);
    }

    let (dist, _) = nearest_food_dist(st, target);
    (2, 0, dist, 0, st.colors.len().saturating_sub(ell))
}

fn next_stage_rank(st: &State, input: &Input, ellp1: usize) -> (usize, usize, usize) {
    if ellp1 >= input.m {
        return (0, 0, 0);
    }
    let (dist, _) = nearest_food_dist(st, input.d[ellp1]);
    let (hr, hc) = rc_of(st.head(), st.n);
    let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
    (dist, center, 0)
}

fn greedy_future_lb_from_cell(
    st: &State,
    input: &Input,
    start_cell: u16,
    start_ell: usize,
    horizon: usize,
    banned: Option<u16>,
) -> (usize, usize, usize) {
    let mut cur = start_cell;
    let end = (start_ell + horizon).min(input.m);
    let mut used = Vec::with_capacity(horizon + 1);
    if let Some(b) = banned {
        used.push(b);
    }

    let mut miss = 0usize;
    let mut first = 0usize;
    let mut total = 0usize;

    for idx in start_ell..end {
        let color = input.d[idx];
        let mut best: Option<(usize, u16)> = None;
        for (cell_idx, &col) in st.food.iter().enumerate() {
            if col != color {
                continue;
            }
            let cell = cell_idx as u16;
            if used.iter().any(|&x| x == cell) {
                continue;
            }
            let dist = manhattan(st.n, cur, cell);
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
    target: u16,
) -> (usize, usize, usize, usize, usize) {
    let head = st.head();
    let capture_lb = manhattan(st.n, head, target);
    let (miss, first, total) =
        greedy_future_lb_from_cell(st, input, target, ell + 1, LOOKAHEAD_HORIZON, Some(target));

    let mut goal_blocked = 1usize;
    for nb in neighbors(st.n, target) {
        if st.food[nb as usize] == 0 || nb == head {
            goal_blocked = 0;
            break;
        }
    }

    (miss, capture_lb + total, first, goal_blocked, capture_lb)
}

fn final_rank(bs: &BeamState, input: &Input) -> (usize, Reverse<usize>, Reverse<usize>) {
    (
        lcp(&bs.state.colors, &input.d),
        Reverse(remaining_food_count(&bs.state)),
        Reverse(bs.ops.len()),
    )
}

fn reconstruct_plan(nodes: &[Node], mut idx: usize) -> String {
    let mut rev = Vec::new();
    while let Some(parent) = nodes[idx].parent {
        rev.push(nodes[idx].move_seg.clone());
        idx = parent;
    }
    rev.reverse();

    let mut out = String::new();
    for seg in rev {
        out.push_str(&seg);
    }
    out
}

fn reconstruct_quick_plan(nodes: &[QuickSearchNode], mut idx: usize) -> String {
    let mut rev = Vec::new();
    while nodes[idx].parent != usize::MAX {
        rev.push(DIRS[nodes[idx].dir as usize].2);
        idx = nodes[idx].parent;
    }
    rev.reverse();
    rev.into_iter().collect()
}

fn state_hash64(st: &State) -> u64 {
    let mut h = 0xcbf29ce484222325_u64;
    let p = 0x100000001b3_u64;

    h ^= st.pos.len() as u64;
    h = h.wrapping_mul(p);
    h ^= st.colors.len() as u64;
    h = h.wrapping_mul(p);

    for &x in &st.food {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    for cell in st.pos.iter() {
        h ^= cell as u64;
        h = h.wrapping_mul(p);
    }
    for &x in &st.colors {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    h
}

fn plan_color_goal_quick(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target_color: u8,
    cfg: QuickPlanConfig,
    started: &Instant,
) -> Option<BeamState> {
    prof_inc!(plan_quick_calls);
    let root = QuickSearchNode {
        state: bs.state.clone(),
        parent: usize::MAX,
        dir: 0,
        depth: 0,
        non_target: 0,
        bite: 0,
    };
    let mut nodes = vec![root];
    let mut q = VecDeque::new();
    q.push_back(0usize);

    let mut seen = FxHashMap::<(u64, u8, u8), u8>::default();
    seen.insert((state_hash64(&nodes[0].state), 0, 0), 0);

    while let Some(cur_idx) = q.pop_front() {
        prof_inc!(plan_quick_expansions);
        if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
            return None;
        }
        let cur_depth = nodes[cur_idx].depth;
        if cur_depth >= cfg.depth_limit {
            continue;
        }

        for dir in legal_dirs(&nodes[cur_idx].state) {
            let (sim, ate, bite_idx, _) = step(&nodes[cur_idx].state, dir);

            let keep = sim.colors.len().min(ell);
            if sim.colors[..keep] != input.d[..keep] {
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

            if child.state.colors.len() >= ell + 1 && child.state.colors[..=ell] == input.d[..=ell]
            {
                nodes.push(child);
                let goal_idx = nodes.len() - 1;
                let mut ops = bs.ops.clone();
                ops.push_str(&reconstruct_quick_plan(&nodes, goal_idx));
                prof_inc!(plan_quick_success);
                return Some(BeamState {
                    state: nodes[goal_idx].state.clone(),
                    ops,
                });
            }

            if nodes.len() >= cfg.node_limit {
                return None;
            }

            let key = (state_hash64(&child.state), non_target, bite);
            if seen
                .get(&key)
                .is_some_and(|&best_depth| best_depth <= next_depth)
            {
                continue;
            }
            seen.insert(key, next_depth);
            nodes.push(child);
            q.push_back(nodes.len() - 1);
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
    prof_inc!(fastlane_calls);
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
        prof_inc!(fastlane_success);
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
        prof_inc!(fastlane_success);
        return Some(sol);
    }

    let sols =
        collect_exact_solutions(bs, input, ell, target_color, FAST_FALLBACK_TARGETS, started);
    let out = sols.into_iter().min_by_key(|cand| cand.ops.len());
    if out.is_some() {
        prof_inc!(fastlane_success);
    }
    out
}

fn try_recover_exact(
    st: &State,
    input: &Input,
    ell: usize,
    dropped: &[Dropped],
) -> Option<(State, String)> {
    prof_inc!(try_recover_exact_calls);
    let mut s = st.clone();
    let need_cnt = ell as isize - s.colors.len() as isize;
    if need_cnt < 0 || dropped.len() < need_cnt as usize {
        return None;
    }

    let mut ops = String::new();
    for ent in dropped.iter().take(need_cnt as usize) {
        let need = input.d[s.colors.len()];
        if ent.color != need {
            return None;
        }
        let dir = dir_between_cells(s.n, s.head(), ent.cell)?;
        if ent.cell == s.pos[1] {
            return None;
        }
        if s.food[ent.cell as usize] != need {
            return None;
        }

        let (ns, ate, bite_idx, _) = step(&s, dir);
        if ate != need || bite_idx.is_some() {
            return None;
        }
        s = ns;
        ops.push(DIRS[dir].2);
        prof_inc!(try_recover_exact_steps);
    }

    if exact_prefix(&s, &input.d, ell) {
        prof_inc!(try_recover_exact_success);
        Some((s, ops))
    } else {
        None
    }
}

fn stage_search_bestfirst(
    start_bs: &BeamState,
    input: &Input,
    ell: usize,
    budgets: &[(usize, usize)],
    keep_solutions: usize,
    started: &Instant,
) -> Vec<BeamState> {
    prof_inc!(stage_search_calls);
    if budgets.is_empty() {
        return Vec::new();
    }
    let start = start_bs.state.clone();

    let max_expansions = budgets[budgets.len() - 1].0;
    let mut nodes = Vec::with_capacity(max_expansions.min(30_000) + 8);
    nodes.push(Node {
        state: start.clone(),
        parent: None,
        move_seg: String::new(),
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

    let mut seen = FxHashMap::<Key, usize>::default();
    seen.insert(encode_key(&start), 0);

    let mut sols: Vec<BeamState> = Vec::new();
    let mut solkeys: FxHashSet<Key> = FxHashSet::default();
    let mut expansions = 0usize;
    let mut stage_idx = 0usize;
    let mut stage_limit = budgets[0].0;
    let mut extra_limit = budgets[0].1;
    let final_limit = budgets[budgets.len() - 1].0;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        prof_inc!(stage_search_expansions);
        if expansions >= final_limit || sols.len() >= keep_solutions || time_over(started) {
            break;
        }

        expansions += 1;
        let st = nodes[idx].state.clone();

        if exact_prefix(&st, &input.d, ell) {
            if let Some(dir2) = target_adjacent(&st, input.d[ell]) {
                let (ns2, _, bite2, _) = step(&st, dir2);
                if bite2.is_none() && exact_prefix(&ns2, &input.d, ell + 1) {
                    let key = encode_key(&ns2);
                    if solkeys.insert(key) {
                        let mut plan = reconstruct_plan(&nodes, idx);
                        plan.push(DIRS[dir2].2);

                        let mut ops = start_bs.ops.clone();
                        ops.push_str(&plan);
                        sols.push(BeamState { state: ns2, ops });
                        prof_inc!(stage_search_solution_hits);
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
            let mut prefix_plan: Option<String> = None;
            for dir1 in legal_dirs(&st) {
                let (ns1, _, bite1, dropped1) = step(&st, dir1);
                if bite1.is_none() || !prefix_ok(&ns1, &input.d, ell) {
                    continue;
                }

                let mut rs = ns1;
                let mut recover_ops = String::new();
                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, input, ell, &dropped1)
                    else {
                        continue;
                    };
                    rs = rec_state;
                    recover_ops = rec_ops;
                }

                if !exact_prefix(&rs, &input.d, ell) {
                    continue;
                }

                for dir2 in legal_dirs(&rs) {
                    let nh = next_head_cell(&rs, dir2).unwrap();
                    if rs.food[nh as usize] != input.d[ell] {
                        continue;
                    }

                    let (ns2, _, bite2, _) = step(&rs, dir2);
                    if bite2.is_some() || !exact_prefix(&ns2, &input.d, ell + 1) {
                        continue;
                    }

                    let key = encode_key(&ns2);
                    if !solkeys.insert(key) {
                        continue;
                    }

                    let mut plan = prefix_plan.clone().unwrap_or_else(|| {
                        let s = reconstruct_plan(&nodes, idx);
                        prefix_plan = Some(s.clone());
                        s
                    });
                    plan.push(DIRS[dir1].2);
                    plan.push_str(&recover_ops);
                    plan.push(DIRS[dir2].2);

                    let mut ops = start_bs.ops.clone();
                    ops.push_str(&plan);
                    sols.push(BeamState { state: ns2, ops });
                    prof_inc!(stage_search_solution_hits);
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

        for dir in legal_dirs(&st) {
            let (mut ns, _, bite_idx, dropped) = step(&st, dir);
            let mut seg = String::new();
            seg.push(DIRS[dir].2);

            if bite_idx.is_some() && ns.colors.len() < ell {
                if !prefix_ok(&ns, &input.d, ell) {
                    continue;
                }
                let Some((rec_state, rec_ops)) = try_recover_exact(&ns, input, ell, &dropped)
                else {
                    continue;
                };
                ns = rec_state;
                seg.push_str(&rec_ops);
            }

            if !prefix_ok(&ns, &input.d, ell) {
                continue;
            }
            if ns.colors.len() > ell + extra_limit {
                continue;
            }

            let nd = depth + seg.len();
            let key = encode_key(&ns);
            if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(key, nd);

            let child = nodes.len();
            nodes.push(Node {
                state: ns.clone(),
                parent: Some(idx),
                move_seg: seg,
            });
            pq.push(Reverse((local_score(&ns, input, ell), nd, uid, child)));
            uid += 1;
        }

        if !exact_prefix(&st, &input.d, ell) {
            for dir in legal_dirs(&st) {
                let (ns, _, bite_idx, dropped) = step(&st, dir);
                if bite_idx.is_none() || !prefix_ok(&ns, &input.d, ell) {
                    continue;
                }

                let mut rs = ns;
                let mut seg = String::new();
                seg.push(DIRS[dir].2);

                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, input, ell, &dropped)
                    else {
                        continue;
                    };
                    rs = rec_state;
                    seg.push_str(&rec_ops);
                }

                if !exact_prefix(&rs, &input.d, ell) {
                    continue;
                }

                let nd = depth + seg.len();
                let key = encode_key(&rs);
                if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(key, nd);

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
    let mut seen2: FxHashSet<Key> = FxHashSet::default();
    for bs in sols {
        if seen2.insert(encode_key(&bs.state)) {
            out.push(bs);
            if out.len() >= keep_solutions {
                break;
            }
        }
    }
    out
}

fn collect_food_cells(st: &State, color: u8) -> Vec<u16> {
    let mut out = Vec::new();
    for (idx, &col) in st.food.iter().enumerate() {
        if col == color {
            out.push(idx as u16);
        }
    }
    out
}

fn neighbors(n: usize, cid: u16) -> Vec<u16> {
    let (r, c) = rc_of(cid, n);
    let mut out = Vec::with_capacity(4);
    for (dr, dc, _) in DIRS {
        let nr = r as isize + dr;
        let nc = c as isize + dc;
        if nr >= 0 && nr < n as isize && nc >= 0 && nc < n as isize {
            out.push(cell_of(nr as usize, nc as usize, n));
        }
    }
    out
}

#[inline]
fn can_reach_target_next_pos(n: usize, pos: &[u16], target: u16) -> bool {
    dir_between_cells(n, pos[0], target).is_some() && (pos.len() < 2 || target != pos[1])
}

fn empty_step_pos(n: usize, food: &[u8], pos: &[u16], dir: usize, target: u16) -> Option<Vec<u16>> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(pos[0], n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
        return None;
    }
    let nh = cell_of(nr as usize, nc as usize, n);
    if pos.len() >= 2 && nh == pos[1] {
        return None;
    }
    if nh == target || food[nh as usize] != 0 {
        return None;
    }

    let mut new_pos = Vec::with_capacity(pos.len());
    new_pos.push(nh);
    new_pos.extend_from_slice(&pos[..pos.len() - 1]);
    for &cell in &new_pos[1..new_pos.len() - 1] {
        if cell == nh {
            return None;
        }
    }
    Some(new_pos)
}

fn reachable_goal_neighbor_count_pos(n: usize, pos: &[u16], target: u16) -> usize {
    let mut blocked = vec![false; n * n];
    if pos.len() >= 3 {
        for &cell in &pos[1..pos.len() - 1] {
            blocked[cell as usize] = true;
        }
    }

    let start = pos[0];
    let mut seen = vec![false; n * n];
    let mut q = VecDeque::new();
    seen[start as usize] = true;
    q.push_back(start);

    while let Some(cur) = q.pop_front() {
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
            q.push_back(nxt);
        }
    }

    let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
    let mut cnt = 0usize;
    for nb in neighbors(n, target) {
        if nb != neck && seen[nb as usize] {
            cnt += 1;
        }
    }
    cnt
}

fn legal_dir_count_pos(n: usize, pos: &[u16]) -> usize {
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
fn empty_path_rank(n: usize, pos: &[u16], target: u16) -> (usize, usize, usize, usize) {
    (
        usize::from(!can_reach_target_next_pos(n, pos, target)),
        manhattan(n, pos[0], target),
        4usize.saturating_sub(reachable_goal_neighbor_count_pos(n, pos, target)),
        4usize.saturating_sub(legal_dir_count_pos(n, pos)),
    )
}

#[inline]
fn can_reach_target_next(st: &State, target: u16) -> bool {
    let Some(_) = dir_between_cells(st.n, st.head(), target) else {
        return false;
    };
    target != st.pos[1]
}

fn bfs_next_dir(
    st: &State,
    goal: u16,
    target: u16,
    avoid_food: bool,
    strict_body: bool,
) -> Option<usize> {
    prof_inc!(bfs_calls);
    let n = st.n;
    let start = st.head();
    if start == goal {
        return None;
    }

    let mut blocked = vec![false; n * n];
    if avoid_food {
        for (idx, &col) in st.food.iter().enumerate() {
            let cell = idx as u16;
            if col != 0 && cell != goal && cell != target {
                blocked[idx] = true;
            }
        }
    }

    if strict_body && st.pos.len() >= 3 {
        for (idx, cell) in st.pos.iter().enumerate() {
            if idx == 0 || idx + 1 == st.pos.len() {
                continue;
            }
            blocked[cell as usize] = true;
        }
    }

    blocked[start as usize] = false;
    if blocked[goal as usize] {
        return None;
    }

    let mut dist = vec![-1_i16; n * n];
    let mut first = vec![None::<usize>; n * n];
    let mut q = VecDeque::new();

    for dir in legal_dirs(st) {
        let nid = next_head_cell(st, dir).unwrap();
        let idx = nid as usize;
        if blocked[idx] || dist[idx] != -1 {
            continue;
        }
        dist[idx] = 1;
        first[idx] = Some(dir);
        q.push_back(nid);
    }

    while let Some(cid) = q.pop_front() {
        prof_inc!(bfs_pops);
        if cid == goal {
            prof_inc!(bfs_success);
            return first[cid as usize];
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
            if blocked[idx] || dist[idx] != -1 {
                continue;
            }
            dist[idx] = dist[cid as usize] + 1;
            first[idx] = first[cid as usize];
            q.push_back(nid);
        }
    }

    None
}

#[inline]
fn make_visit_key(st: &State, goal: u16, restore_len: usize) -> VisitKey {
    let neck = if st.pos.len() >= 2 {
        st.pos[1]
    } else {
        st.head()
    };
    VisitKey {
        head: st.head(),
        neck,
        len: st.colors.len() as u16,
        goal,
        restore_len: restore_len as u16,
    }
}

fn advance_with_restore_queue(
    st: &State,
    dir: usize,
    target: u16,
    ell: usize,
    restore_queue: &mut VecDeque<Dropped>,
) -> Option<(State, Option<usize>)> {
    let (ns, _, bite_idx, dropped) = step(st, dir);
    if ns.food[target as usize] == 0 {
        return None;
    }

    if !restore_queue.is_empty() {
        restore_queue.pop_front();
        return Some((ns, bite_idx));
    }

    if bite_idx.is_some() && ns.colors.len() < ell {
        let need = ell - ns.colors.len();
        for ent in dropped.into_iter().take(need) {
            restore_queue.push_back(ent);
        }
    }
    Some((ns, bite_idx))
}

fn navigate_to_goal_safe(
    bs: &BeamState,
    goal: u16,
    target: u16,
    started: &Instant,
) -> Option<BeamState> {
    prof_inc!(navigate_safe_calls);
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
        let (ns, ate, bite_idx, _) = step(&st, dir);
        if ns.food[target as usize] == 0 {
            return None;
        }
        if bite_idx.is_some() || ate != 0 {
            return None;
        }

        st = ns;
        ops.push(DIRS[dir].2);
        prof_inc!(navigate_safe_steps);
    }

    prof_inc!(navigate_safe_success);
    Some(BeamState { state: st, ops })
}

fn navigate_to_goal_loose(
    bs: &BeamState,
    goal: u16,
    target: u16,
    ell: usize,
    started: &Instant,
) -> Option<BeamState> {
    prof_inc!(navigate_loose_calls);
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    let mut restore_queue: VecDeque<Dropped> = VecDeque::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 4;
    let mut guard = 0usize;

    while st.head() != goal || !restore_queue.is_empty() {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 80 {
            return None;
        }

        let key = make_visit_key(&st, goal, restore_queue.len());
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if front.cell == st.pos[1] {
                return None;
            }
            dir
        } else {
            if let Some(dir) = bfs_next_dir(&st, goal, target, true, false) {
                dir
            } else {
                bfs_next_dir(&st, goal, target, false, false)?
            }
        };

        let (ns, bite_idx) = advance_with_restore_queue(&st, dir, target, ell, &mut restore_queue)?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(DIRS[dir].2);
        prof_inc!(navigate_loose_steps);
    }

    if st.colors.len() < ell {
        return None;
    }

    prof_inc!(navigate_loose_success);
    Some(BeamState { state: st, ops })
}

fn choose_shrink_dir(st: &State, input: &Input, ell: usize, target: u16) -> Option<usize> {
    let anchor_idx = (ell.saturating_sub(1)).min(st.pos.len() - 1);
    let anchor = st.pos[anchor_idx];

    let mut best_bite: Option<((usize, usize, usize, usize), usize)> = None;
    let mut best_move: Option<((usize, usize, usize, usize), usize)> = None;

    for dir in legal_dirs(st) {
        let nh = next_head_cell(st, dir).unwrap();
        if nh == target {
            continue;
        }

        let (sim, ate, bite_idx, _) = step(st, dir);
        if sim.food[target as usize] == 0 {
            continue;
        }

        let keep = sim.colors.len().min(ell);
        if sim.colors[..keep] != input.d[..keep] {
            continue;
        }

        let target_dist = manhattan(sim.n, sim.head(), target);
        let anchor_dist = manhattan(sim.n, sim.head(), anchor);

        if bite_idx.is_some() {
            let under = usize::from(sim.colors.len() < ell);
            let dist_len = sim.colors.len().abs_diff(ell);
            let not_ready = usize::from(!can_reach_target_next(&sim, target));
            let key = (under, dist_len, not_ready, target_dist + anchor_dist);
            if best_bite.as_ref().is_none_or(|(k, _)| key < *k) {
                best_bite = Some((key, dir));
            }
        } else {
            let len_gap = sim.colors.len().abs_diff(ell);
            let not_ready = usize::from(!can_reach_target_next(&sim, target));
            let ate_penalty = usize::from(ate != 0);
            let key = (len_gap, not_ready, target_dist + anchor_dist, ate_penalty);
            if best_move.as_ref().is_none_or(|(k, _)| key < *k) {
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
    target: u16,
    target_color: u8,
    started: &Instant,
) -> Option<BeamState> {
    prof_inc!(shrink_calls);
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();

    if st.colors.len() == ell {
        return can_reach_target_next(&st, target).then_some(BeamState { state: st, ops });
    }

    let mut restore_queue: VecDeque<Dropped> = VecDeque::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 3;
    let mut guard = 0usize;

    while st.colors.len() != ell || !restore_queue.is_empty() || !can_reach_target_next(&st, target)
    {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 60 {
            return None;
        }

        let key = make_visit_key(&st, target, restore_queue.len());
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if front.cell == st.pos[1] {
                return None;
            }
            dir
        } else {
            choose_shrink_dir(&st, input, ell, target)?
        };

        let (ns, bite_idx) = advance_with_restore_queue(&st, dir, target, ell, &mut restore_queue)?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(DIRS[dir].2);
        prof_inc!(shrink_steps);
    }

    if st.colors.len() == ell
        && st.food[target as usize] == target_color
        && can_reach_target_next(&st, target)
    {
        prof_inc!(shrink_success);
        Some(BeamState { state: st, ops })
    } else {
        None
    }
}

fn finish_eat_target(bs: &BeamState, input: &Input, ell: usize, target: u16) -> Option<BeamState> {
    let st = &bs.state;
    let mut ops = bs.ops.clone();
    let dir = dir_between_cells(st.n, st.head(), target)?;
    if target == st.pos[1] {
        return None;
    }

    let (ns, _, bite_idx, _) = step(st, dir);
    if bite_idx.is_some() {
        return None;
    }
    if ns.colors.len() >= ell + 1 && ns.colors[..=ell] == input.d[..=ell] {
        ops.push(DIRS[dir].2);
        Some(BeamState { state: ns, ops })
    } else {
        None
    }
}

fn try_target_empty_path(
    bs: &BeamState,
    input: &Input,
    ell: usize,
    target: u16,
    started: &Instant,
) -> Option<BeamState> {
    prof_inc!(try_target_empty_path_calls);
    let st = &bs.state;
    if !exact_prefix(st, &input.d, ell) {
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
    let start_pos: Vec<u16> = st.pos.iter().collect();
    if reachable_goal_neighbor_count_pos(st.n, &start_pos, target) > 0 {
        return None;
    }
    if collect_food_cells(st, input.d[ell]).len() != 1 {
        return None;
    }

    let mut nodes = Vec::with_capacity(EMPTY_PATH_EXPANSION_CAP.min(120_000) + 8);
    nodes.push(PosNode {
        pos: start_pos.clone(),
        parent: None,
        mv: '\0',
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((
        empty_path_rank(st.n, &start_pos, target),
        0usize,
        uid,
        0usize,
    )));
    uid += 1;

    let mut seen = FxHashMap::<Vec<u16>, usize>::default();
    seen.insert(start_pos, 0);
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        prof_inc!(try_target_empty_path_expansions);
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
            for ch in rev {
                let dir = dir_of_char(ch as u8).unwrap();
                let (ns, ate, bite_idx, _) = step(&state, dir);
                if ate != 0 || bite_idx.is_some() {
                    return None;
                }
                state = ns;
                ops.push(ch);
            }
            let gate_bs = BeamState { state, ops };
            let out = finish_eat_target(&gate_bs, input, ell, target);
            if out.is_some() {
                prof_inc!(try_target_empty_path_success);
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
                mv: DIRS[dir].2,
            });
            pq.push(Reverse((
                empty_path_rank(st.n, &next_pos, target),
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
    target: u16,
    target_color: u8,
    started: &Instant,
) -> Vec<BeamState> {
    prof_inc!(try_target_exact_calls);
    let head = bs.state.head();
    let mut cand = neighbors(bs.state.n, target);

    cand.sort_unstable_by_key(|&cid| {
        (
            usize::from(bs.state.food[cid as usize] > 0),
            manhattan(bs.state.n, head, cid),
        )
    });

    let mut sols = Vec::new();

    for goal in cand {
        if time_over(started) {
            break;
        }

        if let Some(b1) = navigate_to_goal_safe(bs, goal, target, started)
            && let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started)
            && let Some(b3) = finish_eat_target(&b2, input, ell, target)
        {
            sols.push(b3);
        }

        if let Some(b1) = navigate_to_goal_loose(bs, goal, target, ell, started)
            && let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started)
            && let Some(b3) = finish_eat_target(&b2, input, ell, target)
        {
            sols.push(b3);
        }
    }

    if sols.is_empty()
        && is_endgame_mode(&bs.state, input, ell)
        && let Some(sol) = try_target_empty_path(bs, input, ell, target, started)
    {
        sols.push(sol);
    }

    sols.sort_unstable_by_key(|x| x.ops.len());
    let mut out = Vec::new();
    let mut seen: FxHashSet<Key> = FxHashSet::default();
    for s in sols {
        if seen.insert(encode_key(&s.state)) {
            out.push(s);
        }
    }
    prof_inc!(try_target_exact_success, out.len());
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
    prof_inc!(collect_exact_calls);
    let mut sols = Vec::new();
    let mut targets = collect_food_cells(&bs.state, target_color);
    targets.sort_unstable_by_key(|&cid| manhattan(bs.state.n, bs.state.head(), cid));
    if targets.len() > max_targets {
        targets.truncate(max_targets);
    }
    prof_inc!(collect_exact_targets, targets.len());
    for target in targets {
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
    prof_inc!(collect_exact_returned, sols.len());
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
    prof_inc!(collect_exact_turn_calls);
    let mut sols = Vec::new();
    let mut targets = collect_food_cells(&bs.state, target_color);
    targets.sort_unstable_by_key(|&cid| target_candidate_rank(&bs.state, input, ell, cid));
    if targets.len() > max_targets {
        targets.truncate(max_targets);
    }
    prof_inc!(collect_exact_turn_targets, targets.len());

    for target in targets {
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
    let mut seen: FxHashSet<Key> = FxHashSet::default();
    for bs in sols {
        if seen.insert(encode_key(&bs.state)) {
            out.push(bs);
            if out.len() >= SUFFIX_STAGE_BEAM {
                break;
            }
        }
    }
    prof_inc!(collect_exact_turn_returned, out.len());
    out
}

fn rescue_stage(
    beam: &[BeamState],
    input: &Input,
    ell: usize,
    target_color: u8,
    started: &Instant,
) -> Vec<BeamState> {
    prof_inc!(rescue_stage_calls);
    prof_inc!(rescue_stage_beam_inputs, beam.len());
    let mut order: Vec<usize> = (0..beam.len()).collect();
    order.sort_unstable_by_key(|&idx| {
        (
            local_score(&beam[idx].state, input, ell),
            beam[idx].ops.len(),
        )
    });

    let mut rescue_map: FxHashMap<Key, BeamState> = FxHashMap::default();
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
            let key = encode_key(&s.state);
            match rescue_map.get_mut(&key) {
                Some(prev) => {
                    if s.ops.len() < prev.ops.len() {
                        *prev = s;
                    }
                }
                None => {
                    rescue_map.insert(key, s);
                }
            }
        }

        if rescue_map.len() >= STAGE_BEAM * 2 {
            break;
        }
    }

    let mut out: Vec<BeamState> = rescue_map.into_values().collect();
    out.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, input, ell + 1), bs.ops.len()));
    if out.len() > STAGE_BEAM {
        out.truncate(STAGE_BEAM);
    }
    prof_inc!(rescue_stage_returned, out.len());
    out
}

fn trim_stage_beam(
    cands: Vec<BeamState>,
    input: &Input,
    next_ell: usize,
    short_lane: Option<&BeamState>,
) -> Vec<BeamState> {
    prof_inc!(trim_stage_beam_calls);
    prof_inc!(trim_stage_beam_inputs, cands.len());
    let mut strategic = cands.clone();
    strategic
        .sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, input, next_ell), bs.ops.len()));

    let mut out = Vec::with_capacity(STAGE_BEAM);
    let mut seen: FxHashSet<Key> = FxHashSet::default();

    if let Some(bs) = short_lane {
        let key = encode_key(&bs.state);
        if seen.insert(key) {
            out.push(bs.clone());
        }
    }

    if let Some(best_short) = cands.iter().min_by_key(|bs| bs.ops.len()) {
        let key = encode_key(&best_short.state);
        if seen.insert(key) {
            out.push(best_short.clone());
        }
    }

    for bs in strategic {
        let key = encode_key(&bs.state);
        if seen.insert(key) {
            out.push(bs);
            if out.len() >= STAGE_BEAM {
                break;
            }
        }
    }

    prof_inc!(trim_stage_beam_returned, out.len());
    out
}

fn solve_base(input: &Input, started: &Instant) -> BeamState {
    let init = BeamState {
        state: State::initial(input),
        ops: String::new(),
    };
    let mut beam = vec![init];

    for ell in 5..input.m {
        prof_inc!(solve_base_iters);
        prof_inc!(solve_base_beam_inputs, beam.len());
        if time_over(&started) {
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

        let mut new_map: FxHashMap<Key, BeamState> = FxHashMap::default();

        if let Some(sol) = quick_short.clone() {
            let key = encode_key(&sol.state);
            new_map.insert(key, sol);
        }

        for bs in &beam {
            if time_over(&started) {
                break;
            }

            let endgame_mode = is_endgame_mode(&bs.state, input, ell);
            let mut sols = Vec::new();

            if !time_over(&started) {
                if endgame_mode {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        ell,
                        target_color,
                        MAX_TARGETS_ENDGAME,
                        &started,
                    );
                } else {
                    sols = stage_search_bestfirst(bs, input, ell, budgets, STAGE_BEAM, &started);
                }
            }

            if sols.is_empty() && !time_over(&started) {
                if endgame_mode {
                    sols = stage_search_bestfirst(
                        bs,
                        input,
                        ell,
                        &BUDGETS_ENDGAME_LIGHT,
                        STAGE_BEAM,
                        &started,
                    );
                } else {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        ell,
                        target_color,
                        MAX_TARGETS_PER_STAGE,
                        &started,
                    );
                }
            }

            for s in sols {
                if s.ops.len() > MAX_TURNS {
                    continue;
                }
                let key = encode_key(&s.state);
                match new_map.get_mut(&key) {
                    Some(prev) => {
                        if s.ops.len() < prev.ops.len() {
                            *prev = s;
                        }
                    }
                    None => {
                        new_map.insert(key, s);
                    }
                }
            }
        }

        if new_map.is_empty() && !time_over(&started) {
            prof_inc!(solve_base_rescue_calls);
            let rescue = rescue_stage(&beam, input, ell, target_color, &started);
            for s in rescue {
                let key = encode_key(&s.state);
                match new_map.get_mut(&key) {
                    Some(prev) => {
                        if s.ops.len() < prev.ops.len() {
                            *prev = s;
                        }
                    }
                    None => {
                        new_map.insert(key, s);
                    }
                }
            }
        }

        if new_map.is_empty() {
            break;
        }
        prof_inc!(solve_base_new_map_size_sum, new_map.len());

        let new_beam = trim_stage_beam(
            new_map.into_values().collect(),
            input,
            ell + 1,
            quick_short.as_ref(),
        );
        beam = new_beam;
    }

    if beam.is_empty() {
        return BeamState {
            state: State::initial(input),
            ops: String::new(),
        };
    }

    beam.sort_unstable_by_key(|bs| final_rank(bs, input));
    let mut best = beam.pop().unwrap();
    if best.ops.len() > MAX_TURNS {
        best.ops.truncate(MAX_TURNS);
    }
    best
}

fn reconstruct_exact_checkpoints(input: &Input, ops: &str) -> Vec<Option<(usize, State)>> {
    let mut checkpoints = vec![None; input.m + 1];
    let mut st = State::initial(input);
    checkpoints[5] = Some((0, st.clone()));
    let mut ell = 5usize;

    for (t, ch) in ops.bytes().enumerate() {
        let Some(dir) = dir_of_char(ch) else {
            break;
        };
        if !is_legal_dir(&st, dir) {
            break;
        }
        let (ns, _, _, _) = step(&st, dir);
        st = ns;
        if ell < input.m && exact_prefix(&st, &input.d, ell + 1) {
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
    bs.state.colors.len() == input.m
        && bs.state.colors == input.d
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
        prof_inc!(solve_suffix_iters);
        prof_inc!(solve_suffix_beam_inputs, beam.len());
        if time_over(started) || time_left(started) < 0.02 {
            break;
        }

        let target_color = input.d[ell];
        let mut new_map: FxHashMap<Key, BeamState> = FxHashMap::default();

        for bs in &beam {
            if time_over(started) {
                break;
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
                let key = encode_key(&s.state);
                match new_map.get_mut(&key) {
                    Some(prev) => {
                        if s.ops.len() < prev.ops.len() {
                            *prev = s;
                        }
                    }
                    None => {
                        new_map.insert(key, s);
                    }
                }
            }
        }

        if new_map.is_empty() {
            break;
        }
        prof_inc!(solve_suffix_new_map_size_sum, new_map.len());

        let mut new_beam: Vec<BeamState> = new_map.into_values().collect();
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

    beam.sort_unstable_by_key(|bs| final_rank(bs, input));
    beam.pop().unwrap()
}

fn optimize_exact_suffix(input: &Input, base: BeamState, started: &Instant) -> BeamState {
    if !is_complete_exact(&base, input) {
        return base;
    }

    let mut best = base;

    for &window in &SUFFIX_OPT_WINDOWS {
        prof_inc!(optimize_suffix_windows);
        if time_left(started) < SUFFIX_OPT_MIN_LEFT_SEC {
            break;
        }

        let checkpoints = reconstruct_exact_checkpoints(input, &best.ops);
        let start_ell = input.m.saturating_sub(window).max(5);
        let Some((prefix_turns, st)) = checkpoints[start_ell].clone() else {
            continue;
        };

        let prefix_ops = best.ops[..prefix_turns].to_string();
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

fn solve(input: &Input) -> String {
    let started = Instant::now();
    let base = solve_base(input, &started);
    let mut best = optimize_exact_suffix(input, base, &started);
    if best.ops.len() > MAX_TURNS {
        best.ops.truncate(MAX_TURNS);
    }
    prof_set!(final_turns, best.ops.len());
    best.ops
}

fn main() {
    let input = read_input();
    let ans = solve(&input);

    let mut out = String::new();
    for ch in ans.chars() {
        out.push(ch);
        out.push('\n');
    }
    print!("{out}");
    profile_counts::dump(env!("CARGO_BIN_NAME"));
}
