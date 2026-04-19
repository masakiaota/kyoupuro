// bench_bite_detection.rs
use std::env;
use std::hash::Hash;
use std::hint::black_box;
use std::ops::{Deref, Index};
use std::time::{Duration, Instant};

use rustc_hash::FxHashMap;

const CAP: usize = 16 * 16;
const BRANCHES: usize = 3;
const DEFAULT_INPUT_COUNT: usize = 2;
const DEFAULT_MICRO_MS: u64 = 90;
const DEFAULT_MACRO_MS: u64 = 120;
const DEFAULT_ROUNDS: usize = 18;
const LENGTHS: [usize; 4] = [32, 64, 128, 192];
const MICRO_PARENT_COUNTS: [usize; 2] = [256, 4096];
const BEAM_WIDTHS: [usize; 4] = [64, 256, 1024, 4096];

const COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
const POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
const POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;

const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Cell(u16);

#[inline]
fn pos_hash_token(cell: Cell) -> u64 {
    cell.0 as u64 + 1
}

#[inline]
fn color_hash_token(color: u8) -> u64 {
    color as u64 + 1
}

const fn build_hash_pows(base: u64) -> [u64; CAP + 1] {
    let mut pows = [0_u64; CAP + 1];
    pows[0] = 1;
    let mut i = 1;
    while i <= CAP {
        pows[i] = pows[i - 1].wrapping_mul(base);
        i += 1;
    }
    pows
}

const POS_HASH_POW1: [u64; CAP + 1] = build_hash_pows(POS_HASH_BASE1);
const POS_HASH_POW2: [u64; CAP + 1] = build_hash_pows(POS_HASH_BASE2);
const COLOR_HASH_POW1: [u64; CAP + 1] = build_hash_pows(COLOR_HASH_BASE1);
const COLOR_HASH_POW2: [u64; CAP + 1] = build_hash_pows(COLOR_HASH_BASE2);

#[derive(Clone)]
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [Cell; CAP],
    hash1: u64,
    hash2: u64,
}

impl InternalPosDeque {
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [Cell(0); CAP],
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
            let x = pos_hash_token(cell);
            deque.hash1 = deque.hash1.wrapping_add(x.wrapping_mul(pow1));
            deque.hash2 = deque.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(POS_HASH_BASE1);
            pow2 = pow2.wrapping_mul(POS_HASH_BASE2);
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
        if raw < CAP { raw } else { raw - CAP }
    }

    #[inline]
    fn push_front(&mut self, cell: Cell) {
        self.head = (self.head + CAP - 1) % CAP;
        self.buf[self.head] = cell;
        let x = pos_hash_token(cell);
        self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(POS_HASH_BASE1));
        self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(POS_HASH_BASE2));
        self.len += 1;
    }

    #[inline]
    fn pop_back(&mut self) -> Option<Cell> {
        if self.len == 0 {
            return None;
        }
        let idx = self.physical_index(self.len - 1);
        let cell = self.buf[idx];
        let x = pos_hash_token(cell);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(POS_HASH_POW1[self.len - 1]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(POS_HASH_POW2[self.len - 1]));
        self.len -= 1;
        Some(cell)
    }

    #[inline]
    fn iter(&self) -> InternalPosDequeIter<'_> {
        InternalPosDequeIter { deque: self, idx: 0 }
    }
}

impl Index<usize> for InternalPosDeque {
    type Output = Cell;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.buf[self.physical_index(idx)]
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
        if self.idx == self.deque.len {
            None
        } else {
            let cell = self.deque[self.idx];
            self.idx += 1;
            Some(cell)
        }
    }
}

#[derive(Clone)]
struct InternalColors {
    buf: [u8; CAP],
    len: u16,
    hash1: u64,
    hash2: u64,
}

impl InternalColors {
    #[inline]
    fn new() -> Self {
        Self {
            buf: [0; CAP],
            len: 0,
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(colors: &[u8]) -> Self {
        let mut out = Self::new();
        out.buf[..colors.len()].copy_from_slice(colors);
        out.len = colors.len() as u16;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &color in colors {
            let x = color_hash_token(color);
            out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
            out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(COLOR_HASH_BASE1);
            pow2 = pow2.wrapping_mul(COLOR_HASH_BASE2);
        }
        out
    }

    #[inline]
    fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    fn push(&mut self, color: u8) {
        let idx = self.len();
        self.buf[idx] = color;
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let idx = self.len() - 1;
        let color = self.buf[idx];
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
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

#[derive(Clone)]
struct Snapshot {
    food: Vec<u8>,
    pos: Vec<Cell>,
    colors: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Action {
    next_head: Cell,
    eat_color: u8,
}

#[derive(Clone)]
struct RoundDataset {
    parents: Vec<Snapshot>,
    actions: Vec<[Action; BRANCHES]>,
}

#[derive(Clone)]
struct Dataset {
    rounds: Vec<RoundDataset>,
}

#[derive(Clone)]
struct LinearDetectState {
    pos: InternalPosDeque,
}

#[derive(Clone)]
struct OccupancyDetectState {
    pos: InternalPosDeque,
    occ: [u8; CAP],
}

#[derive(Clone)]
struct MultiDetectState {
    pos: InternalPosDeque,
    occ: FxHashMap<u16, u8>,
}

#[derive(Clone)]
struct LinearState {
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: InternalColors,
}

#[derive(Clone)]
struct OccupancyState {
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: InternalColors,
    occ: [u8; CAP],
}

#[derive(Clone)]
struct MultiState {
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: InternalColors,
    occ: FxHashMap<u16, u8>,
}

#[derive(Default, Clone, Copy)]
struct BenchAccum {
    elapsed_ns: f64,
    ops: u64,
}

impl BenchAccum {
    fn add(&mut self, other: BenchAccum) {
        self.elapsed_ns += other.elapsed_ns;
        self.ops += other.ops;
    }

    fn ns_per_op(self) -> f64 {
        self.elapsed_ns / self.ops as f64
    }
}

fn build_occ_counts(cells: &[Cell]) -> [u8; CAP] {
    let mut occ = [0_u8; CAP];
    for &cell in cells {
        occ[cell.0 as usize] += 1;
    }
    occ
}

fn build_occ_multiset(cells: &[Cell]) -> FxHashMap<u16, u8> {
    let mut occ = FxHashMap::default();
    for &cell in cells {
        *occ.entry(cell.0).or_insert(0) += 1;
    }
    occ
}

#[inline]
fn multiset_dec(occ: &mut FxHashMap<u16, u8>, cell: Cell) {
    if let Some(cnt) = occ.get_mut(&cell.0) {
        if *cnt == 1 {
            occ.remove(&cell.0);
        } else {
            *cnt -= 1;
        }
    }
}

#[inline]
fn multiset_inc(occ: &mut FxHashMap<u16, u8>, cell: Cell) {
    *occ.entry(cell.0).or_insert(0) += 1;
}

#[inline]
fn find_bite_idx_linear(pos: &InternalPosDeque) -> Option<usize> {
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

fn build_snapshot(len: usize, seed: u64) -> Snapshot {
    let mut cells = [Cell(0); CAP];
    let mut i = 0;
    while i < CAP {
        cells[i] = Cell(i as u16);
        i += 1;
    }
    let mut k = 0;
    while k < len {
        let j = k + (splitmix64(seed ^ (k as u64) ^ 0xA5A5_5A5A) as usize % (CAP - k));
        cells.swap(k, j);
        k += 1;
    }
    let pos = cells[..len].to_vec();
    let mut colors = Vec::with_capacity(len);
    for idx in 0..len {
        colors.push(((splitmix64(seed ^ 0x1234_5678 ^ idx as u64) % 7) as u8) + 1);
    }
    let mut food = vec![0_u8; CAP];
    let occupied = build_occ_counts(&pos);
    for idx in 0..CAP {
        if occupied[idx] == 0 {
            let x = splitmix64(seed ^ 0xCAFEBABE ^ idx as u64);
            if x % 100 < 28 {
                food[idx] = ((x >> 8) % 7) as u8 + 1;
            }
        }
    }
    Snapshot { food, pos, colors }
}

fn choose_empty_cell(snapshot: &Snapshot, require_food: bool, salt: u64) -> Option<Cell> {
    let occ = build_occ_counts(&snapshot.pos);
    let start = splitmix64(salt) as usize % CAP;
    for step in 0..CAP {
        let idx = (start + step) % CAP;
        if occ[idx] == 0 {
            let has_food = snapshot.food[idx] != 0;
            if has_food == require_food {
                return Some(Cell(idx as u16));
            }
        }
    }
    None
}

fn choose_actions(snapshot: &Snapshot, round: usize, parent_idx: usize, seed: u64) -> [Action; BRANCHES] {
    let mut out = [Action {
        next_head: snapshot.pos[0],
        eat_color: 0,
    }; BRANCHES];
    for branch_idx in 0..BRANCHES {
        let x = splitmix64(
            seed ^ ((round as u64) << 32) ^ ((parent_idx as u64) << 8) ^ branch_idx as u64,
        );
        let bucket = (x % 100) as u8;
        let len = snapshot.pos.len();
        if bucket < 20 && len >= 3 {
            let bite_span = len - 2;
            let idx = 1 + ((x >> 8) as usize % bite_span);
            out[branch_idx] = Action {
                next_head: snapshot.pos[idx],
                eat_color: 0,
            };
        } else if bucket < 30 && len >= 1 {
            out[branch_idx] = Action {
                next_head: snapshot.pos[len - 1],
                eat_color: 0,
            };
        } else if bucket < 58 {
            if let Some(cell) = choose_empty_cell(snapshot, true, x ^ 0xEA70_EA70) {
                out[branch_idx] = Action {
                    next_head: cell,
                    eat_color: snapshot.food[cell.0 as usize],
                };
            } else if let Some(cell) = choose_empty_cell(snapshot, false, x ^ 0xBEEF_BEEF) {
                out[branch_idx] = Action {
                    next_head: cell,
                    eat_color: 0,
                };
            }
        } else if let Some(cell) = choose_empty_cell(snapshot, false, x ^ 0xFACE_FACE) {
            out[branch_idx] = Action {
                next_head: cell,
                eat_color: 0,
            };
        } else {
            let idx = 1 + ((x >> 16) as usize % len.saturating_sub(2).max(1));
            out[branch_idx] = Action {
                next_head: snapshot.pos[idx.min(len - 2)],
                eat_color: 0,
            };
        }
    }
    out
}

fn reference_step(snapshot: &Snapshot, action: Action) -> Snapshot {
    let mut food = snapshot.food.clone();
    let mut pos = snapshot.pos.clone();
    let mut colors = snapshot.colors.clone();

    if action.eat_color != 0 {
        debug_assert_eq!(food[action.next_head.0 as usize], action.eat_color);
        food[action.next_head.0 as usize] = 0;
        colors.push(action.eat_color);
    } else {
        let _ = pos.pop();
    }
    pos.insert(0, action.next_head);

    let mut bite_idx = None;
    for idx in 1..pos.len().saturating_sub(1) {
        if pos[idx] == action.next_head {
            bite_idx = Some(idx);
            break;
        }
    }
    if let Some(h) = bite_idx {
        while pos.len() > h + 1 {
            let cell = pos.pop().unwrap();
            let color = colors.pop().unwrap();
            food[cell.0 as usize] = color;
        }
    }

    Snapshot { food, pos, colors }
}

fn build_dataset(len: usize, beam_width: usize, rounds: usize, seed: u64) -> Dataset {
    let mut beam = (0..beam_width)
        .map(|parent_idx| build_snapshot(len, seed ^ parent_idx as u64))
        .collect::<Vec<_>>();
    let mut round_data = Vec::with_capacity(rounds);

    for round in 0..rounds {
        let parents = beam.clone();
        let mut actions = Vec::with_capacity(beam_width);
        let mut children = Vec::with_capacity(beam_width * BRANCHES);
        for (parent_idx, parent) in parents.iter().enumerate() {
            let act = choose_actions(parent, round, parent_idx, seed ^ 0xDEAD_BEEF_CAFE_BABE);
            for &action in &act {
                children.push(reference_step(parent, action));
            }
            actions.push(act);
        }
        round_data.push(RoundDataset { parents, actions });
        let offset = (round * 5 + 1) % BRANCHES;
        beam = children
            .into_iter()
            .skip(offset)
            .step_by(BRANCHES)
            .take(beam_width)
            .collect();
    }

    Dataset { rounds: round_data }
}

fn convert_linear_detect(parents: &[Snapshot]) -> Vec<LinearDetectState> {
    parents
        .iter()
        .map(|s| LinearDetectState {
            pos: InternalPosDeque::from_slice(&s.pos),
        })
        .collect()
}

fn convert_occupancy_detect(parents: &[Snapshot]) -> Vec<OccupancyDetectState> {
    parents
        .iter()
        .map(|s| OccupancyDetectState {
            pos: InternalPosDeque::from_slice(&s.pos),
            occ: build_occ_counts(&s.pos),
        })
        .collect()
}

fn convert_multi_detect(parents: &[Snapshot]) -> Vec<MultiDetectState> {
    parents
        .iter()
        .map(|s| MultiDetectState {
            pos: InternalPosDeque::from_slice(&s.pos),
            occ: build_occ_multiset(&s.pos),
        })
        .collect()
}

fn convert_linear_step(parents: &[Snapshot]) -> Vec<LinearState> {
    parents
        .iter()
        .map(|s| LinearState {
            food: s.food.clone(),
            pos: InternalPosDeque::from_slice(&s.pos),
            colors: InternalColors::from_slice(&s.colors),
        })
        .collect()
}

fn convert_occupancy_step(parents: &[Snapshot]) -> Vec<OccupancyState> {
    parents
        .iter()
        .map(|s| OccupancyState {
            food: s.food.clone(),
            pos: InternalPosDeque::from_slice(&s.pos),
            colors: InternalColors::from_slice(&s.colors),
            occ: build_occ_counts(&s.pos),
        })
        .collect()
}

fn convert_multi_step(parents: &[Snapshot]) -> Vec<MultiState> {
    parents
        .iter()
        .map(|s| MultiState {
            food: s.food.clone(),
            pos: InternalPosDeque::from_slice(&s.pos),
            colors: InternalColors::from_slice(&s.colors),
            occ: build_occ_multiset(&s.pos),
        })
        .collect()
}

fn micro_check_linear(parent: &LinearDetectState, action: Action) -> bool {
    let mut pos = parent.pos.clone();
    if action.eat_color == 0 {
        let _ = pos.pop_back();
    }
    pos.push_front(action.next_head);
    find_bite_idx_linear(&pos).is_some()
}

fn micro_check_occupancy(parent: &OccupancyDetectState, action: Action) -> bool {
    let mut pos = parent.pos.clone();
    let mut occ = parent.occ;
    let excluded_tail = if action.eat_color != 0 {
        Some(pos[pos.len() - 1])
    } else if pos.len() >= 2 {
        Some(pos[pos.len() - 2])
    } else {
        None
    };
    if action.eat_color == 0 {
        let removed_tail = pos.pop_back().unwrap();
        occ[removed_tail.0 as usize] -= 1;
    }
    let tail_bias = usize::from(excluded_tail == Some(action.next_head)) as u8;
    let bite = occ[action.next_head.0 as usize] > tail_bias;
    occ[action.next_head.0 as usize] += 1;
    pos.push_front(action.next_head);
    black_box(pos);
    bite
}

fn micro_check_multi(parent: &MultiDetectState, action: Action) -> bool {
    let mut pos = parent.pos.clone();
    let mut occ = parent.occ.clone();
    let excluded_tail = if action.eat_color != 0 {
        Some(pos[pos.len() - 1])
    } else if pos.len() >= 2 {
        Some(pos[pos.len() - 2])
    } else {
        None
    };
    if action.eat_color == 0 {
        let removed_tail = pos.pop_back().unwrap();
        multiset_dec(&mut occ, removed_tail);
    }
    let tail_bias = usize::from(excluded_tail == Some(action.next_head)) as u8;
    let bite = occ.get(&action.next_head.0).copied().unwrap_or(0) > tail_bias;
    multiset_inc(&mut occ, action.next_head);
    pos.push_front(action.next_head);
    black_box(pos);
    bite
}

fn step_like_linear(parent: &LinearState, action: Action) -> u64 {
    let mut food = parent.food.clone();
    let mut pos = parent.pos.clone();
    let mut colors = parent.colors.clone();

    if action.eat_color != 0 {
        food[action.next_head.0 as usize] = 0;
        colors.push(action.eat_color);
    } else {
        let _ = pos.pop_back();
    }
    pos.push_front(action.next_head);

    if let Some(h) = find_bite_idx_linear(&pos) {
        while pos.len() > h + 1 {
            let cell = pos.pop_back().unwrap();
            let color = colors.pop().unwrap();
            food[cell.0 as usize] = color;
        }
    }
    (pos.len() as u64) ^ ((colors.len() as u64) << 16) ^ food[action.next_head.0 as usize] as u64
}

fn step_like_occupancy(parent: &OccupancyState, action: Action) -> u64 {
    let mut food = parent.food.clone();
    let mut pos = parent.pos.clone();
    let mut colors = parent.colors.clone();
    let mut occ = parent.occ;
    let excluded_tail = if action.eat_color != 0 {
        Some(pos[pos.len() - 1])
    } else if pos.len() >= 2 {
        Some(pos[pos.len() - 2])
    } else {
        None
    };

    if action.eat_color != 0 {
        food[action.next_head.0 as usize] = 0;
        colors.push(action.eat_color);
    } else {
        let removed_tail = pos.pop_back().unwrap();
        occ[removed_tail.0 as usize] -= 1;
    }
    let tail_bias = usize::from(excluded_tail == Some(action.next_head)) as u8;
    let bite = occ[action.next_head.0 as usize] > tail_bias;
    occ[action.next_head.0 as usize] += 1;
    pos.push_front(action.next_head);

    if bite {
        let h = find_bite_idx_linear(&pos).unwrap_or_else(|| {
            panic!(
                "occupancy false positive: eat_color={}, next_head={}, excluded_tail={:?}, tail_bias={}, occ_count={}, len={}, pos={:?}",
                action.eat_color,
                action.next_head.0,
                excluded_tail.map(|c| c.0),
                tail_bias,
                occ[action.next_head.0 as usize],
                pos.len(),
                pos.iter().map(|c| c.0).collect::<Vec<_>>()
            )
        });
        while pos.len() > h + 1 {
            let cell = pos.pop_back().unwrap();
            occ[cell.0 as usize] -= 1;
            let color = colors.pop().unwrap();
            food[cell.0 as usize] = color;
        }
    }
    (pos.len() as u64) ^ ((colors.len() as u64) << 16) ^ food[action.next_head.0 as usize] as u64
}

fn step_like_multi(parent: &MultiState, action: Action) -> u64 {
    let mut food = parent.food.clone();
    let mut pos = parent.pos.clone();
    let mut colors = parent.colors.clone();
    let mut occ = parent.occ.clone();
    let excluded_tail = if action.eat_color != 0 {
        Some(pos[pos.len() - 1])
    } else if pos.len() >= 2 {
        Some(pos[pos.len() - 2])
    } else {
        None
    };

    if action.eat_color != 0 {
        food[action.next_head.0 as usize] = 0;
        colors.push(action.eat_color);
    } else {
        let removed_tail = pos.pop_back().unwrap();
        multiset_dec(&mut occ, removed_tail);
    }
    let tail_bias = usize::from(excluded_tail == Some(action.next_head)) as u8;
    let bite = occ.get(&action.next_head.0).copied().unwrap_or(0) > tail_bias;
    multiset_inc(&mut occ, action.next_head);
    pos.push_front(action.next_head);

    if bite {
        let h = find_bite_idx_linear(&pos).unwrap_or_else(|| {
            panic!(
                "multiset false positive: eat_color={}, next_head={}, excluded_tail={:?}, tail_bias={}, occ_count={}, len={}, pos={:?}",
                action.eat_color,
                action.next_head.0,
                excluded_tail.map(|c| c.0),
                tail_bias,
                occ.get(&action.next_head.0).copied().unwrap_or(0),
                pos.len(),
                pos.iter().map(|c| c.0).collect::<Vec<_>>()
            )
        });
        while pos.len() > h + 1 {
            let cell = pos.pop_back().unwrap();
            multiset_dec(&mut occ, cell);
            let color = colors.pop().unwrap();
            food[cell.0 as usize] = color;
        }
    }
    (pos.len() as u64) ^ ((colors.len() as u64) << 16) ^ food[action.next_head.0 as usize] as u64
}

fn measure_micro_linear(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let first = &dataset.rounds[0];
    let parents = convert_linear_detect(&first.parents);
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parent, actions) in parents.iter().zip(first.actions.iter()) {
            for &action in actions {
                acc ^= micro_check_linear(parent, action) as u64;
                ops += 1;
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_micro_occupancy(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let first = &dataset.rounds[0];
    let parents = convert_occupancy_detect(&first.parents);
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parent, actions) in parents.iter().zip(first.actions.iter()) {
            for &action in actions {
                acc ^= micro_check_occupancy(parent, action) as u64;
                ops += 1;
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_micro_multi(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let first = &dataset.rounds[0];
    let parents = convert_multi_detect(&first.parents);
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parent, actions) in parents.iter().zip(first.actions.iter()) {
            for &action in actions {
                acc ^= micro_check_multi(parent, action) as u64;
                ops += 1;
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_macro_linear(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let rounds = dataset
        .rounds
        .iter()
        .map(|round| (convert_linear_step(&round.parents), round.actions.clone()))
        .collect::<Vec<_>>();
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parents, actions) in &rounds {
            for (parent, branch_actions) in parents.iter().zip(actions.iter()) {
                for &action in branch_actions {
                    acc ^= step_like_linear(parent, action);
                    ops += 1;
                }
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_macro_occupancy(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let rounds = dataset
        .rounds
        .iter()
        .map(|round| (convert_occupancy_step(&round.parents), round.actions.clone()))
        .collect::<Vec<_>>();
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parents, actions) in &rounds {
            for (parent, branch_actions) in parents.iter().zip(actions.iter()) {
                for &action in branch_actions {
                    acc ^= step_like_occupancy(parent, action);
                    ops += 1;
                }
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_macro_multi(dataset: &Dataset, min_duration: Duration) -> BenchAccum {
    let rounds = dataset
        .rounds
        .iter()
        .map(|round| (convert_multi_step(&round.parents), round.actions.clone()))
        .collect::<Vec<_>>();
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for (parents, actions) in &rounds {
            for (parent, branch_actions) in parents.iter().zip(actions.iter()) {
                for &action in branch_actions {
                    acc ^= step_like_multi(parent, action);
                    ops += 1;
                }
            }
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn sanity_check() {
    let dataset = build_dataset(32, 64, 3, 1);
    let round = &dataset.rounds[0];
    let lin = convert_linear_step(&round.parents);
    let occ = convert_occupancy_step(&round.parents);
    let mul = convert_multi_step(&round.parents);
    for parent_idx in 0..4 {
        for branch_idx in 0..BRANCHES {
            let action = round.actions[parent_idx][branch_idx];
            let a = step_like_linear(&lin[parent_idx], action);
            let b = step_like_occupancy(&occ[parent_idx], action);
            let c = step_like_multi(&mul[parent_idx], action);
            assert_eq!(a, b);
            assert_eq!(a, c);
        }
    }
}

fn getenv_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn getenv_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
}

fn print_micro_header(input_count: usize, micro_ms: u64) {
    println!(
        "# Self-bite micro benchmark\n\
         # current=linear scan, occupancy=[u8;256], multiset=FxHashMap<Cell,u8>\n\
         # lengths={:?}, parent_counts={:?}, inputs_per_config={}, measure_ms={}\n",
        LENGTHS, MICRO_PARENT_COUNTS, input_count, micro_ms
    );
    println!(
        "{:<12} {:>6} {:>10} {:>16} {:>16} {:>16} {:>14} {:>14}",
        "section",
        "len",
        "parents",
        "linear ns/op",
        "occupancy",
        "multiset",
        "occ/linear",
        "multi/linear"
    );
}

fn print_macro_header(input_count: usize, rounds: usize, macro_ms: u64) {
    println!(
        "# Self-bite practical benchmark\n\
         # step-like workload with food/pos/colors clone, bite resolution, and recorded beam-style parent states\n\
         # lengths={:?}, beam_widths={:?}, rounds={}, inputs_per_config={}, measure_ms={}\n",
        LENGTHS, BEAM_WIDTHS, rounds, input_count, macro_ms
    );
    println!(
        "{:<12} {:>6} {:>10} {:>16} {:>16} {:>16} {:>14} {:>14}",
        "section",
        "len",
        "beam",
        "linear ns/op",
        "occupancy",
        "multiset",
        "occ/linear",
        "multi/linear"
    );
}

fn main() {
    sanity_check();

    let input_count = getenv_usize("BENCH_INPUT_COUNT", DEFAULT_INPUT_COUNT);
    let micro_ms = getenv_u64("BENCH_MICRO_MS", DEFAULT_MICRO_MS);
    let macro_ms = getenv_u64("BENCH_MACRO_MS", DEFAULT_MACRO_MS);
    let rounds = getenv_usize("BENCH_ROUNDS", DEFAULT_ROUNDS);

    print_micro_header(input_count, micro_ms);
    for &len in &LENGTHS {
        for &parent_count in &MICRO_PARENT_COUNTS {
            let mut linear = BenchAccum::default();
            let mut occ = BenchAccum::default();
            let mut multi = BenchAccum::default();
            for input_idx in 0..input_count {
                let seed = 0x1234_5678_9ABC_DEF0
                    ^ (len as u64) << 20
                    ^ (parent_count as u64) << 4
                    ^ input_idx as u64;
                let dataset = build_dataset(len, parent_count, 1, seed);
                let min_duration = Duration::from_millis(micro_ms);
                linear.add(measure_micro_linear(&dataset, min_duration));
                occ.add(measure_micro_occupancy(&dataset, min_duration));
                multi.add(measure_micro_multi(&dataset, min_duration));
            }
            let linear_ns = linear.ns_per_op();
            let occ_ns = occ.ns_per_op();
            let multi_ns = multi.ns_per_op();
            println!(
                "{:<12} {:>6} {:>10} {:>16.3} {:>16.3} {:>16.3} {:>14.3} {:>14.3}",
                "micro",
                len,
                parent_count,
                linear_ns,
                occ_ns,
                multi_ns,
                occ_ns / linear_ns,
                multi_ns / linear_ns
            );
        }
        println!();
    }

    print_macro_header(input_count, rounds, macro_ms);
    for &len in &LENGTHS {
        for &beam_width in &BEAM_WIDTHS {
            let mut linear = BenchAccum::default();
            let mut occ = BenchAccum::default();
            let mut multi = BenchAccum::default();
            for input_idx in 0..input_count {
                let seed = 0x0F0F_F0F0_AAAA_5555
                    ^ (len as u64) << 20
                    ^ (beam_width as u64) << 4
                    ^ input_idx as u64;
                let dataset = build_dataset(len, beam_width, rounds, seed);
                let min_duration = Duration::from_millis(macro_ms);
                linear.add(measure_macro_linear(&dataset, min_duration));
                occ.add(measure_macro_occupancy(&dataset, min_duration));
                multi.add(measure_macro_multi(&dataset, min_duration));
            }
            let linear_ns = linear.ns_per_op();
            let occ_ns = occ.ns_per_op();
            let multi_ns = multi.ns_per_op();
            println!(
                "{:<12} {:>6} {:>10} {:>16.3} {:>16.3} {:>16.3} {:>14.3} {:>14.3}",
                "practical",
                len,
                beam_width,
                linear_ns,
                occ_ns,
                multi_ns,
                occ_ns / linear_ns,
                multi_ns / linear_ns
            );
        }
        println!();
    }
}
