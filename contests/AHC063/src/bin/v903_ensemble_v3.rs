// v903_ensemble_v3.rs
use std::io::{self, Read};
use std::time::Instant;

mod v170_impl {
    #![allow(dead_code)]
    include!("v170_stock6_hybrid.rs");

    pub fn solve_from_common(common: &crate::CommonInput, limit_sec: f64) -> (Vec<u8>, bool) {
        if limit_sec <= 0.0 {
            return (Vec::new(), false);
        }
        let mut d = [0_u8; MAX_LEN];
        for (dst, &src) in d.iter_mut().zip(common.goal_colors.iter()) {
            *dst = src;
        }
        let mut food = [0_u8; MAX_CELLS];
        for (dst, &src) in food.iter_mut().zip(common.food.iter()) {
            *dst = src;
        }
        let input = Input {
            n: common.n,
            m: common.m,
            d,
            food,
            manhattan: ManhattanTable::new(common.n),
        };

        let seg_len = 2 * (input.n - 2);
        let stock_cnt = if input.m <= 5 || seg_len == 0 {
            0
        } else {
            (input.m - 5) / seg_len
        };
        if stock_cnt >= 6 {
            return (
                v161_stock6::solve_from_parts(
                    input.n,
                    input.m,
                    &input.d[..input.m],
                    &input.food[..input.n * input.n],
                ),
                true,
            );
        }

        let timer = TimeKeeper::new(limit_sec.max(1e-6).min(TIME_LIMIT_SEC), 8);
        let _global_budget = push_search_budget(&timer, limit_sec.max(0.0), 0.0);
        let mut best = solve_inventory_stock(&input, &timer);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        let finished = timer.exact_remaining_sec() > 1e-9;
        (best.ops, finished)
    }
}

const TOTAL_LIMIT_SEC: f64 = 1.88;
const SKIP_V149_THRESHOLD_SEC: f64 = 0.8;
const MAX_TURNS: usize = 100_000;
const INVALID_SCORE: u64 = 1_u64 << 60;
const DIR_CHARS: [char; 4] = ['U', 'D', 'L', 'R'];

#[derive(Debug, Clone)]
struct CommonInput {
    n: usize,
    m: usize,
    goal_colors: Vec<u8>,
    food: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Density3 {
    L,
    M,
    H,
}

#[derive(Debug, Clone)]
struct Candidate {
    ops: Vec<u8>,
    abs_score: u64,
    finished: bool,
    elapsed_sec: f64,
}

#[derive(Debug, Clone, Copy)]
struct ScoreResult {
    abs_score: u64,
}

fn read_common_input() -> CommonInput {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut goal_colors = vec![0_u8; m];
    for x in &mut goal_colors {
        *x = it.next().unwrap().parse::<u8>().unwrap();
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for col in 0..n {
            food[r * n + col] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    CommonInput {
        n,
        m,
        goal_colors,
        food,
    }
}

fn classify_density3(input: &CommonInput) -> Density3 {
    let density = input.m as f64 / (input.n * input.n) as f64;
    if density < 0.40 {
        Density3::L
    } else if density < 0.55 {
        Density3::M
    } else {
        Density3::H
    }
}

fn invalid_score() -> ScoreResult {
    ScoreResult { abs_score: INVALID_SCORE }
}

fn next_cell(n: usize, head: usize, dir: u8) -> Option<usize> {
    let r = head / n;
    let c = head % n;
    match dir {
        0 if r > 0 => Some((r - 1) * n + c),
        1 if r + 1 < n => Some((r + 1) * n + c),
        2 if c > 0 => Some(r * n + (c - 1)),
        3 if c + 1 < n => Some(r * n + (c + 1)),
        _ => None,
    }
}

fn score_ops(input: &CommonInput, ops: &[u8]) -> ScoreResult {
    if ops.len() > MAX_TURNS {
        return invalid_score();
    }

    let n = input.n;
    let mut food = input.food.clone();
    let mut pos = vec![4 * n, 3 * n, 2 * n, n, 0];
    let mut colors = vec![1_u8; 5];

    for &dir in ops {
        if dir > 3 {
            return invalid_score();
        }
        let Some(next_head) = next_cell(n, pos[0], dir) else {
            return invalid_score();
        };
        if pos.len() >= 2 && next_head == pos[1] {
            return invalid_score();
        }

        let ate_color = food[next_head];
        if ate_color != 0 {
            pos.insert(0, next_head);
            food[next_head] = 0;
            colors.push(ate_color);
        } else {
            pos.pop();
            pos.insert(0, next_head);

            if pos.len() >= 3 {
                let mut bite_idx = None;
                for idx in 1..(pos.len() - 1) {
                    if pos[idx] == next_head {
                        bite_idx = Some(idx);
                        break;
                    }
                }
                if let Some(h) = bite_idx {
                    while pos.len() > h + 1 {
                        let cell = pos.pop().unwrap();
                        let color = colors.pop().unwrap();
                        food[cell] = color;
                    }
                }
            }
        }
    }

    let k = colors.len().min(input.m);
    let mut mismatch = 0usize;
    for idx in 0..k {
        if colors[idx] != input.goal_colors[idx] {
            mismatch += 1;
        }
    }
    ScoreResult {
        abs_score: ops.len() as u64 + 10_000_u64 * (mismatch as u64 + 2 * (input.m - k) as u64),
    }
}

fn remaining_limit_sec(start: &Instant) -> f64 {
    (TOTAL_LIMIT_SEC - start.elapsed().as_secs_f64()).max(0.0)
}

fn update_best(best: &mut Candidate, cand: Candidate) {
    if cand.abs_score < best.abs_score {
        *best = cand;
    }
}

fn run_v012(input: &CommonInput) -> Candidate {
    let started = Instant::now();
    let ops = v012_impl::solve_from_common(input);
    let elapsed_sec = started.elapsed().as_secs_f64();
    let score = score_ops(input, &ops);
    Candidate {
        ops,
        abs_score: score.abs_score,
        finished: true,
        elapsed_sec,
    }
}

fn run_v139(input: &CommonInput, limit_sec: f64) -> Candidate {
    let started = Instant::now();
    let (ops, finished) = v139_impl::solve_from_common(input, limit_sec);
    let elapsed_sec = started.elapsed().as_secs_f64();
    let score = score_ops(input, &ops);
    Candidate {
        ops,
        abs_score: score.abs_score,
        finished,
        elapsed_sec,
    }
}

fn run_v149(input: &CommonInput, limit_sec: f64) -> Candidate {
    let started = Instant::now();
    let (ops, finished) = v170_impl::solve_from_common(input, limit_sec);
    let elapsed_sec = started.elapsed().as_secs_f64();
    let score = score_ops(input, &ops);
    Candidate {
        ops,
        abs_score: score.abs_score,
        finished,
        elapsed_sec,
    }
}

fn run_v001_sample_fallback(input: &CommonInput) -> Candidate {
    let started = Instant::now();
    let mut ops = Vec::new();

    for _ in 4..(input.n - 1) {
        ops.push(1);
    }

    for col in 1..input.n {
        ops.push(3);
        let vertical = if col % 2 == 1 { 0 } else { 1 };
        for _ in 0..(input.n - 1) {
            ops.push(vertical);
        }
    }

    let elapsed_sec = started.elapsed().as_secs_f64();
    let score = score_ops(input, &ops);
    Candidate {
        ops,
        abs_score: score.abs_score,
        finished: true,
        elapsed_sec,
    }
}

#[allow(dead_code, unused_imports, unused_variables)]
mod v012_impl {
    // v012_simple_beam.rs
    // 考察メモ:
    // - max でも 0.3 秒台で十分速い。
    // - これで解けるケースではかなりスコアがよいことが多い。
    // - 戦略の一つとして有力である。
    // - TODO: どういう input のときにこれでほぼ解けるのかを調査できると、
    //   使うタイミングを切り分けられるかもしれない。
    use std::collections::VecDeque;
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
    const CANDIDATE_WIDTH: usize = 8;
    const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];

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
        fn can_move(n: usize, cell: Cell, dir: usize) -> bool {
            if dir >= DIRS.len() {
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
        fn next_cell(n: usize, cell: Cell, dir: usize) -> Cell {
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
        fn dir_between_cells(n: usize, from: Cell, to: Cell) -> usize {
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

    #[derive(Debug, Clone)]
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
        fn is_legal_dir(&self, n: usize, dir: usize) -> bool {
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

            for dir in 0..DIRS.len() {
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
            for dir in 0..DIRS.len() {
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

    #[derive(Debug, Clone)]
    struct StepResult {
        state: State,
    }

    fn step(state: &State, n: usize, dir: usize) -> StepResult {
        debug_assert!(state.is_legal_dir(n, dir));

        let next_head = Grid::next_cell(n, state.head(), dir);

        let mut food = state.food.clone();
        let mut new_pos = state.pos.clone();
        let mut new_colors = state.colors.clone();
        let mut new_pos_occupancy = state.pos_occupancy.clone();

        let eat_idx = Grid::index(next_head);
        if food[eat_idx] != 0 {
            let food_color = food[eat_idx];
            food[eat_idx] = 0;
            new_colors.push(food_color);
        } else {
            let old_tail = new_pos.pop_back().unwrap();
            new_pos_occupancy.dec(old_tail);
        }

        let excluded_tail = new_pos.back();
        let tail_bias = u8::from(excluded_tail == Some(next_head));
        let bite = new_pos_occupancy.count(next_head) > tail_bias;

        new_pos_occupancy.inc(next_head);
        new_pos.push_front(next_head);
        if let Some(h) = if bite {
            find_internal_bite_idx(&new_pos)
        } else {
            None
        } {
            while new_pos.len() > h + 1 {
                let cell = new_pos.pop_back().unwrap();
                new_pos_occupancy.dec(cell);
                let color = new_colors.pop().unwrap();
                food[Grid::index(cell)] = color;
            }
        }

        StepResult {
            state: State {
                food,
                pos: new_pos,
                colors: new_colors,
                pos_occupancy: new_pos_occupancy,
            },
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
        ops: Vec<char>,
    }

    type Score = (usize, usize, usize);

    struct ScoredNode {
        node: BeamNode,
        score: Score,
    }

    fn solve_beam(input: &Input) -> Vec<char> {
        let mut beam = vec![BeamNode {
            state: State::initial(input),
            ops: Vec::new(),
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

    fn expand_one_node(parent: &BeamNode, input: &Input, ell: usize) -> Vec<ScoredNode> {
        let target_color = input.d[ell];
        let paths = collect_top_k_target_paths(&parent.state, input.n, target_color, CANDIDATE_WIDTH);

        let mut children = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(child) = build_child_node(parent, input.n, &path) else {
                continue;
            };
            let score = evaluate_node(&child.state, input, ell + 1, child.ops.len());
            children.push(ScoredNode { node: child, score });
        }
        children
    }

    fn build_child_node(parent: &BeamNode, n: usize, path: &[Cell]) -> Option<BeamNode> {
        let mut child_state = parent.state.clone();
        let mut child_ops = parent.ops.clone();
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
        out: &mut Vec<char>,
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
            out.push(DIRS[dir].2);
        }
        true
    }

    pub fn solve_from_common(common: &crate::CommonInput) -> Vec<u8> {
        let input = from_common_input(common);
        solve_beam(&input)
            .into_iter()
            .map(|ch| match ch {
                'U' => 0,
                'D' => 1,
                'L' => 2,
                'R' => 3,
                _ => unreachable!("invalid direction char: {ch}"),
            })
            .collect()
    }

    fn from_common_input(common: &crate::CommonInput) -> Input {
        Input {
            n: common.n,
            m: common.m,
            d: common.goal_colors.clone(),
            food: common.food.clone(),
        }
    }
}

#[allow(dead_code, unused_imports, unused_variables)]
mod v139_impl {
    // v139_refactor_v137.rs
    // メモ:
    // - 一時的に誤食してから自分で断ち切り、順番通りに食べ直す高度なテクが入っている。
    //   そのおかげか case0022 は結構強い。
    // - ヘビが長くなったときには弱い。途中で運び屋戦略のような中継手を挟む余地がある。
    use std::cell::RefCell;
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap, HashSet};
    use std::hash::{BuildHasherDefault, Hash, Hasher};
    use std::io::{self, Read};
    use std::thread_local;
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

    #[derive(Debug, Clone)]
    struct TimeKeeper {
        start: Instant,
        time_limit_sec: f64,
        iter: u64,
        check_mask: u64,
        elapsed_sec: f64,
        progress: f64,
        is_over: bool,
    }

    #[allow(dead_code)]
    impl TimeKeeper {
        fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
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

        #[inline(always)]
        fn step(&mut self) -> bool {
            self.iter += 1;
            if (self.iter & self.check_mask) == 0 {
                self.force_update();
            }
            !self.is_over
        }

        #[inline(always)]
        fn force_update(&mut self) {
            let elapsed = self.start.elapsed().as_secs_f64();
            self.elapsed_sec = elapsed;
            self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
            self.is_over = elapsed >= self.time_limit_sec;
        }

        #[inline(always)]
        fn elapsed_sec(&self) -> f64 {
            self.elapsed_sec
        }

        #[inline(always)]
        fn progress(&self) -> f64 {
            self.progress
        }

        #[inline(always)]
        fn is_time_over(&self) -> bool {
            self.is_over
        }

        #[inline]
        fn exact_elapsed_sec(&self) -> f64 {
            self.start.elapsed().as_secs_f64()
        }

        #[inline]
        fn exact_remaining_sec(&self) -> f64 {
            (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
        }
    }

    #[derive(Clone)]
    struct SearchBudget {
        global_start: Instant,
        global_limit_sec: f64,
        local_limit_sec: f64,
        min_remaining_sec: f64,
    }

    thread_local! {
        static ACTIVE_SEARCH_BUDGET: RefCell<Option<SearchBudget>> = const { RefCell::new(None) };
    }

    struct ScopedSearchBudget {
        prev: Option<SearchBudget>,
    }

    impl Drop for ScopedSearchBudget {
        fn drop(&mut self) {
            ACTIVE_SEARCH_BUDGET.with(|slot| {
                *slot.borrow_mut() = self.prev.take();
            });
        }
    }

    #[inline]
    fn push_search_budget(
        timer: &TimeKeeper,
        local_limit_sec: f64,
        min_remaining_sec: f64,
    ) -> ScopedSearchBudget {
        let next = SearchBudget {
            global_start: timer.start,
            global_limit_sec: timer.time_limit_sec,
            local_limit_sec,
            min_remaining_sec,
        };
        let prev = ACTIVE_SEARCH_BUDGET.with(|slot| {
            let prev = slot.borrow().clone();
            *slot.borrow_mut() = Some(next);
            prev
        });
        ScopedSearchBudget { prev }
    }

    #[inline]
    fn current_time_left(started: &Instant) -> f64 {
        let local_elapsed = started.elapsed().as_secs_f64();
        ACTIVE_SEARCH_BUDGET.with(|slot| {
            if let Some(budget) = slot.borrow().as_ref() {
                let global_elapsed = budget.global_start.elapsed().as_secs_f64();
                let global_left =
                    (budget.global_limit_sec - budget.min_remaining_sec - global_elapsed).max(0.0);
                let local_left = (budget.local_limit_sec - local_elapsed).max(0.0);
                global_left.min(local_left)
            } else {
                (TIME_LIMIT_SEC - local_elapsed).max(0.0)
            }
        })
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
        let over = current_time_left(started) <= 0.0;
        if over {
        }
        over
    }

    #[inline]
    fn time_left(started: &Instant) -> f64 {
        current_time_left(started)
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
        _input: &Input,
        goal_colors: &[u8],
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
            let need_color = goal_colors[cur_len];
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

        if exact_prefix(&state, goal_colors, ell) {
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
    fn lcp_state(st: &State, goal_colors: &[u8]) -> usize {
        let mut i = 0usize;
        let m = st.len().min(goal_colors.len());
        while i < m && st.colors[i] == goal_colors[i] {
            i += 1;
        }
        i
    }

    #[inline]
    fn prefix_ok(st: &State, goal_colors: &[u8], ell: usize) -> bool {
        let keep = st.len().min(ell);
        for idx in 0..keep {
            if st.colors[idx] != goal_colors[idx] {
                return false;
            }
        }
        true
    }

    #[inline]
    fn exact_prefix(st: &State, goal_colors: &[u8], ell: usize) -> bool {
        st.len() == ell && prefix_ok(st, goal_colors, ell)
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

    fn local_score(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
    ) -> (usize, usize, usize, usize, usize) {
        let target = goal_colors[ell];
        if exact_prefix(st, goal_colors, ell) {
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

    fn next_stage_rank(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ellp1: usize,
    ) -> (usize, usize, usize) {
        if ellp1 >= goal_colors.len() {
            return (0, 0, 0);
        }
        let (dist, _) = nearest_food_dist(st, input, goal_colors[ellp1]);
        let (hr, hc) = rc_of(st.head(), st.n);
        let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
        (dist, center, 0)
    }

    fn greedy_future_lb_from_cell(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        start_cell: Cell,
        start_ell: usize,
        horizon: usize,
        banned: Option<Cell>,
    ) -> (usize, usize, usize) {
        let mut cur = start_cell;
        let end = (start_ell + horizon).min(goal_colors.len());
        let mut used = CellList::new();
        if let Some(b) = banned {
            used.push(b);
        }

        let mut miss = 0usize;
        let mut first = 0usize;
        let mut total = 0usize;

        for idx in start_ell..end {
            let color = goal_colors[idx];
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
        goal_colors: &[u8],
        ellp1: usize,
    ) -> (usize, usize, usize, usize, usize) {
        if ellp1 >= goal_colors.len() {
            return (0, 0, 0, 0, 0);
        }
        let (miss, first, total) =
            greedy_future_lb_from_cell(st, input, goal_colors, st.head(), ellp1, LOOKAHEAD_HORIZON, None);
        let adj = target_adjacent(st, goal_colors[ellp1]).is_some();
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
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
    ) -> (usize, usize, usize, usize, usize) {
        let head = st.head();
        let capture_lb = manhattan(&input.manhattan, head, target);
        let (miss, first, total) = greedy_future_lb_from_cell(
            st,
            input,
            goal_colors,
            target,
            ell + 1,
            LOOKAHEAD_HORIZON,
            Some(target),
        );

        let mut goal_blocked = 1usize;
        for &nb in neighbors(st.n, target).as_slice() {
            if st.food[nb as usize] == 0 || nb == head {
                goal_blocked = 0;
                break;
            }
        }

        (miss, capture_lb + total, first, goal_blocked, capture_lb)
    }

    fn absolute_score_state(st: &State, goal_colors: &[u8], ops_len: usize) -> usize {
        let k = st.len().min(goal_colors.len());
        let mut mismatch = 0usize;
        for idx in 0..k {
            if st.colors[idx] != goal_colors[idx] {
                mismatch += 1;
            }
        }
        ops_len + 10_000 * (mismatch + 2 * (goal_colors.len() - k))
    }

    fn beam_score_key(
        bs: &BeamState,
        goal_colors: &[u8],
    ) -> (usize, usize, Reverse<usize>, usize) {
        (
            absolute_score_state(&bs.state, goal_colors, bs.ops.len()),
            bs.ops.len(),
            Reverse(lcp_state(&bs.state, goal_colors)),
            remaining_food_count(&bs.state),
        )
    }

    fn choose_best_beamstate(cands: Vec<BeamState>, goal_colors: &[u8]) -> BeamState {
        cands
            .into_iter()
            .min_by_key(|bs| beam_score_key(bs, goal_colors))
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
        _input: &Input,
        goal_colors: &[u8],
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
                    if sim.colors[idx] != goal_colors[idx] {
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
                        if child.state.colors[idx] != goal_colors[idx] {
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
        goal_colors: &[u8],
        ell: usize,
        started: &Instant,
    ) -> Option<BeamState> {
        if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
            return None;
        }

        let target_color = goal_colors[ell];
        if collect_food_cells(&bs.state, target_color).is_empty() {
            return None;
        }

        let safe_cfg = QuickPlanConfig {
            depth_limit: FAST_SAFE_DEPTH_LIMIT,
            node_limit: FAST_SAFE_NODE_LIMIT,
            non_target_limit: 0,
            bite_limit: 0,
        };
        if let Some(sol) =
            plan_color_goal_quick(bs, input, goal_colors, ell, target_color, safe_cfg, started)
        {
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
        if let Some(sol) = plan_color_goal_quick(
            bs,
            input,
            goal_colors,
            ell,
            target_color,
            rescue_cfg,
            started,
        ) {
            return Some(sol);
        }

        let sols = collect_exact_solutions(
            bs,
            input,
            goal_colors,
            ell,
            target_color,
            FAST_FALLBACK_TARGETS,
            started,
        );
        let out = sols.into_iter().min_by_key(|cand| cand.ops.len());
        if out.is_some() {
        }
        out
    }

    fn try_recover_exact(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        dropped: &DroppedBuf,
    ) -> Option<(State, Ops)> {
        let repaired = repair_prefix_after_bite(st, input, goal_colors, ell, dropped)?;
        if !exact_prefix(&repaired.state, goal_colors, ell) {
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
        goal_colors: &[u8],
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
            local_score(&start, input, goal_colors, ell),
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

            if exact_prefix(&st, goal_colors, ell) {
                if let Some(dir2) = target_adjacent(&st, goal_colors[ell]) {
                    let (ns2, _, bite2) = step(&st, dir2);
                    if bite2.is_none() && exact_prefix(&ns2, goal_colors, ell + 1) {
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
                    if bite1.is_none() || !prefix_ok(&ns1, goal_colors, ell) {
                        continue;
                    }

                    let mut rs = ns1;
                    let mut recover_ops = Ops::new();
                    if rs.len() < ell {
                        let Some((rec_state, rec_ops)) =
                            try_recover_exact(&rs, input, goal_colors, ell, &dropped1)
                        else {
                            continue;
                        };
                        rs = rec_state;
                        recover_ops = rec_ops;
                    }

                    if !exact_prefix(&rs, goal_colors, ell) {
                        continue;
                    }

                    let dirs2 = legal_dirs(&rs);
                    for &dir2_u8 in dirs2.as_slice() {
                        let dir2 = dir2_u8 as usize;
                        let nh = next_head_cell(&rs, dir2).unwrap();
                        if rs.food[nh as usize] != goal_colors[ell] {
                            continue;
                        }

                        let (ns2, _, bite2) = step(&rs, dir2);
                        if bite2.is_some() || !exact_prefix(&ns2, goal_colors, ell + 1) {
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
                    if !prefix_ok(&ns, goal_colors, ell) {
                        continue;
                    }
                    let Some((rec_state, rec_ops)) =
                        try_recover_exact(&ns, input, goal_colors, ell, &dropped2)
                    else {
                        continue;
                    };
                    ns = rec_state;
                    seg.extend_from_slice(&rec_ops);
                }

                if !prefix_ok(&ns, goal_colors, ell) {
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
                pq.push(Reverse((local_score(&ns, input, goal_colors, ell), nd, uid, child)));
                uid += 1;
            }

            if !exact_prefix(&st, goal_colors, ell) {
                let dirs = legal_dirs(&st);
                for &dir_u8 in dirs.as_slice() {
                    let dir = dir_u8 as usize;
                    let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped1);
                    if bite_idx.is_none() || !prefix_ok(&ns, goal_colors, ell) {
                        continue;
                    }

                    let mut rs = ns;
                    let mut seg = Ops::with_capacity(1 + ell);
                    seg.push(dir as Dir);

                    if rs.len() < ell {
                        let Some((rec_state, rec_ops)) =
                            try_recover_exact(&rs, input, goal_colors, ell, &dropped1)
                        else {
                            continue;
                        };
                        rs = rec_state;
                        seg.extend_from_slice(&rec_ops);
                    }

                    if !exact_prefix(&rs, goal_colors, ell) {
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
                    pq.push(Reverse((local_score(&rs, input, goal_colors, ell), nd, uid, child)));
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

        sols.sort_unstable_by_key(|bs| {
            (
                next_stage_rank(&bs.state, input, goal_colors, ell + 1),
                bs.ops.len(),
            )
        });

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
        goal_colors: &[u8],
        ell: usize,
        path: &CellList,
    ) -> Option<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
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

        exact_prefix(&child_state, goal_colors, ell + 1).then_some(BeamState {
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
        goal_colors: &[u8],
        ell: usize,
        path: &CellList,
    ) -> Option<BeamState> {
        if !exact_prefix(repaired_state, goal_colors, ell) {
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

        exact_prefix(&child_state, goal_colors, ell + 1).then_some(BeamState {
            state: child_state,
            ops: child_ops,
        })
    }

    fn expand_bite_children(
        parent: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        candidate_width: usize,
    ) -> Vec<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
            return Vec::new();
        }

        let target_color = goal_colors[ell];
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

                let Some(repaired) =
                    repair_prefix_after_bite(&next_state, input, goal_colors, ell, &dropped)
                else {
                    continue;
                };
                if !exact_prefix(&repaired.state, goal_colors, ell) {
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
                    goal_colors,
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
        _input: &Input,
        goal_colors: &[u8],
        ell: usize,
        candidate_width: usize,
    ) -> Vec<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
            return Vec::new();
        }

        let paths = collect_top_k_target_paths(&parent.state, goal_colors[ell], candidate_width);
        let mut out = Vec::with_capacity(paths.len());
        let mut seen: FxHashSet<State> = FxHashSet::default();
        for path in &paths {
            if let Some(child) = build_simple_child(parent, goal_colors, ell, path) {
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
                let mut children =
                    expand_simple_children(parent, input, &input.d[..input.m], ell, candidate_width);
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
            choose_best_beamstate(beam, &input.d[..input.m])
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
        goal_colors: &[u8],
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
            let repaired = repair_prefix_after_bite(&ns, input, goal_colors, ell, dropped)?;
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
        goal_colors: &[u8],
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
                goal_colors,
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

    fn choose_shrink_dir(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
    ) -> Option<usize> {
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

            if !prefix_ok(&sim, goal_colors, ell) {
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
        goal_colors: &[u8],
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
                choose_shrink_dir(&st, input, goal_colors, ell, target)?
            };

            let (ns, recover_ops, bite_idx) = advance_with_restore_queue(
                &st,
                input,
                goal_colors,
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

    fn finish_eat_target(
        bs: &BeamState,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
    ) -> Option<BeamState> {
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
        if ns.len() >= ell + 1 && exact_prefix(&ns, goal_colors, ell + 1) {
            ops.push(dir as Dir);
            Some(BeamState { state: ns, ops })
        } else {
            None
        }
    }

    fn try_target_empty_path(
        bs: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
        started: &Instant,
    ) -> Option<BeamState> {
        let st = &bs.state;
        if !exact_prefix(st, goal_colors, ell) {
            return None;
        }
        if st.food[target as usize] != goal_colors[ell] {
            return None;
        }
        if remaining_food_count(st) > EMPTY_PATH_REMAINING_LIMIT
            || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
        {
            return None;
        }

        if can_reach_target_next(st, target) {
            return finish_eat_target(bs, goal_colors, ell, target);
        }
        if reachable_goal_neighbor_count_pos(st.n, &st.pos, target) > 0 {
            return None;
        }
        if collect_food_cells(st, goal_colors[ell]).len() != 1 {
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
                let out = finish_eat_target(&gate_bs, goal_colors, ell, target);
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
        goal_colors: &[u8],
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
                if let Some(b2) =
                    shrink_to_ell(&b1, input, goal_colors, ell, target, target_color, started)
                {
                    if let Some(b3) = finish_eat_target(&b2, goal_colors, ell, target) {
                        sols.push(b3);
                    }
                }
            }

            if let Some(b1) = navigate_to_goal_loose(bs, input, goal_colors, goal, target, ell, started)
            {
                if let Some(b2) =
                    shrink_to_ell(&b1, input, goal_colors, ell, target, target_color, started)
                {
                    if let Some(b3) = finish_eat_target(&b2, goal_colors, ell, target) {
                        sols.push(b3);
                    }
                }
            }
        }

        if sols.is_empty() && is_endgame_mode(&bs.state, goal_colors.len(), ell) {
            if let Some(sol) = try_target_empty_path(bs, input, goal_colors, ell, target, started) {
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
    fn is_endgame_mode(st: &State, goal_len: usize, ell: usize) -> bool {
        goal_len - ell <= ENDGAME_ELL_LEFT && remaining_food_count(st) <= ENDGAME_REMAINING_FOOD
    }

    fn collect_exact_solutions(
        bs: &BeamState,
        input: &Input,
        goal_colors: &[u8],
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
            let cand = try_target_exact(bs, input, goal_colors, ell, target, target_color, started);
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
        goal_colors: &[u8],
        ell: usize,
        target_color: u8,
        max_targets: usize,
        started: &Instant,
    ) -> Vec<BeamState> {
        let mut sols = Vec::new();
        let mut targets = collect_food_cells(&bs.state, target_color);
        targets
            .as_mut_slice()
            .sort_unstable_by_key(|&cid| target_candidate_rank(&bs.state, input, goal_colors, ell, cid));
        if targets.len() > max_targets {
            targets.truncate(max_targets);
        }

        for &target in targets.as_slice() {
            if time_over(started) {
                break;
            }
            let cand = try_target_exact(bs, input, goal_colors, ell, target, target_color, started);
            for s in cand {
                sols.push(s);
            }
            if sols.len() >= SUFFIX_STAGE_BEAM * 2 {
                break;
            }
        }

        sols.sort_unstable_by_key(|bs| {
            (
                turn_focus_next_stage_rank(&bs.state, input, goal_colors, ell + 1),
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
        goal_colors: &[u8],
        ell: usize,
        target_color: u8,
        started: &Instant,
    ) -> Vec<BeamState> {
        let mut order: Vec<usize> = (0..beam.len()).collect();
        order.sort_unstable_by_key(|&idx| {
            (
                local_score(&beam[idx].state, input, goal_colors, ell),
                beam[idx].ops.len(),
            )
        });

        let mut rescue_map: FxHashMap<State, Ops> = FxHashMap::default();
        for &idx in &order {
            if time_over(started) {
                break;
            }

            let bs = &beam[idx];
            let endgame_mode = is_endgame_mode(&bs.state, goal_colors.len(), ell);

            let mut sols = if endgame_mode {
                collect_exact_solutions(
                    bs,
                    input,
                    goal_colors,
                    ell,
                    target_color,
                    MAX_TARGETS_RESCUE,
                    started,
                )
            } else {
                stage_search_bestfirst(
                    bs,
                    input,
                    goal_colors,
                    ell,
                    &BUDGETS_RESCUE,
                    STAGE_BEAM,
                    started,
                )
            };

            if sols.is_empty() && !time_over(started) {
                if endgame_mode {
                    sols = stage_search_bestfirst(
                        bs,
                        input,
                        goal_colors,
                        ell,
                        &BUDGETS_ENDGAME_LIGHT,
                        STAGE_BEAM,
                        started,
                    );
                } else {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        goal_colors,
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
        out.sort_unstable_by_key(|bs| {
            (
                next_stage_rank(&bs.state, input, goal_colors, ell + 1),
                bs.ops.len(),
            )
        });
        if out.len() > STAGE_BEAM {
            out.truncate(STAGE_BEAM);
        }
        out
    }

    fn trim_stage_beam(
        cands: Vec<BeamState>,
        input: &Input,
        goal_colors: &[u8],
        next_ell: usize,
        short_lane: Option<&BeamState>,
    ) -> Vec<BeamState> {

        let mut strategic_order: Vec<usize> = (0..cands.len()).collect();
        strategic_order.sort_unstable_by_key(|&idx| {
            (
                next_stage_rank(&cands[idx].state, input, goal_colors, next_ell),
                cands[idx].ops.len(),
            )
        });

        let mut turn_order: Vec<usize> = (0..cands.len()).collect();
        turn_order.sort_unstable_by_key(|&idx| {
            (
                turn_focus_next_stage_rank(&cands[idx].state, input, goal_colors, next_ell),
                cands[idx].ops.len(),
            )
        });

        let best_short = cands.iter().min_by_key(|bs| bs.ops.len()).cloned();
        let best_turn = cands
            .iter()
            .min_by_key(|bs| {
                (
                    turn_focus_next_stage_rank(&bs.state, input, goal_colors, next_ell),
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

    #[derive(Clone, Copy)]
    enum GrowthMode {
        Base,
        SuffixTurnFocused,
    }

    fn grow_to_target_prefix(
        input: &Input,
        start_state: State,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        local_time_limit_sec: f64,
    ) -> Option<BeamState> {
        if start_state.len() > goal_colors.len() {
            return None;
        }
        for idx in 0..start_state.len() {
            if start_state.colors[idx] != goal_colors[idx] {
                return None;
            }
        }

        let start_ell = start_state.len();
        let start_bs = BeamState {
            state: start_state,
            ops: Ops::new(),
        };
        Some(grow_to_target_prefix_impl(
            input,
            start_bs,
            goal_colors,
            start_ell,
            timer,
            local_time_limit_sec,
            GrowthMode::Base,
        ))
    }

    fn grow_to_target_prefix_impl(
        input: &Input,
        start_bs: BeamState,
        goal_colors: &[u8],
        start_ell: usize,
        timer: &TimeKeeper,
        local_time_limit_sec: f64,
        mode: GrowthMode,
    ) -> BeamState {
        let _budget = push_search_budget(timer, local_time_limit_sec.max(0.0), 0.0);
        let started = Instant::now();
        let mut beam = vec![start_bs.clone()];
        let goal_len = goal_colors.len();

        match mode {
            GrowthMode::Base => {
                for ell in start_ell..goal_len {
                    if time_over(&started) {
                        break;
                    }

                    let target_color = goal_colors[ell];
                    let budgets: &[(usize, usize)] = if goal_len - ell < 10 {
                        &BUDGETS_LATE
                    } else {
                        &BUDGETS_NORMAL
                    };

                    let short_seed = beam.iter().min_by_key(|bs| bs.ops.len()).cloned();
                    let quick_short = short_seed
                        .as_ref()
                        .and_then(|bs| extend_fastlane_one(bs, input, goal_colors, ell, &started));

                    let mut new_map: FxHashMap<State, Ops> = FxHashMap::default();
                    if let Some(sol) = quick_short.clone() {
                        insert_best_plan(&mut new_map, sol.state, sol.ops);
                    }

                    for bs in &beam {
                        if time_over(&started) {
                            break;
                        }

                        if !time_over(&started) {
                            let simple_children = expand_simple_children(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                SIMPLE_INJECT_PER_STATE,
                            );
                            for s in simple_children {
                                if s.ops.len() > MAX_TURNS {
                                    continue;
                                }
                                insert_best_plan(&mut new_map, s.state, s.ops);
                            }

                            let bite_children = expand_bite_children(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                BITE_CANDIDATE_WIDTH,
                            );
                            for s in bite_children {
                                if s.ops.len() > MAX_TURNS {
                                    continue;
                                }
                                insert_best_plan(&mut new_map, s.state, s.ops);
                            }
                        }

                        let endgame_mode = is_endgame_mode(&bs.state, goal_len, ell);
                        let mut sols = Vec::new();

                        if !time_over(&started) {
                            if endgame_mode {
                                sols = collect_exact_solutions(
                                    bs,
                                    input,
                                    goal_colors,
                                    ell,
                                    target_color,
                                    MAX_TARGETS_ENDGAME,
                                    &started,
                                );
                            } else {
                                sols = stage_search_bestfirst(
                                    bs,
                                    input,
                                    goal_colors,
                                    ell,
                                    budgets,
                                    STAGE_BEAM,
                                    &started,
                                );
                            }
                        }

                        if sols.is_empty() && !time_over(&started) {
                            if endgame_mode {
                                sols = stage_search_bestfirst(
                                    bs,
                                    input,
                                    goal_colors,
                                    ell,
                                    &BUDGETS_ENDGAME_LIGHT,
                                    STAGE_BEAM,
                                    &started,
                                );
                            } else {
                                sols = collect_exact_solutions(
                                    bs,
                                    input,
                                    goal_colors,
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
                            insert_best_plan(&mut new_map, s.state, s.ops);
                        }
                    }

                    if new_map.is_empty() && !time_over(&started) {
                        let rescue =
                            rescue_stage(&beam, input, goal_colors, ell, target_color, &started);
                        for s in rescue {
                            insert_best_plan(&mut new_map, s.state, s.ops);
                        }
                    }

                    if new_map.is_empty() {
                        break;
                    }

                    beam = trim_stage_beam(
                        map_into_beamstates(new_map),
                        input,
                        goal_colors,
                        ell + 1,
                        quick_short.as_ref(),
                    );
                }
            }
            GrowthMode::SuffixTurnFocused => {
                for ell in start_ell..goal_len {
                    if time_over(&started) || time_left(&started) < 0.02 {
                        break;
                    }

                    let target_color = goal_colors[ell];
                    let mut new_map: FxHashMap<State, Ops> = FxHashMap::default();

                    for bs in &beam {
                        if time_over(&started) {
                            break;
                        }

                        if !time_over(&started) {
                            let simple_children = expand_simple_children(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                SIMPLE_INJECT_PER_STATE,
                            );
                            for s in simple_children {
                                if s.ops.len() > MAX_TURNS {
                                    continue;
                                }
                                insert_best_plan(&mut new_map, s.state, s.ops);
                            }

                            let bite_children = expand_bite_children(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                BITE_CANDIDATE_WIDTH,
                            );
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
                            goal_colors,
                            ell,
                            target_color,
                            SUFFIX_OPT_TARGETS,
                            &started,
                        );

                        if sols.is_empty() && !time_over(&started) {
                            sols = stage_search_bestfirst(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                &BUDGETS_ENDGAME_LIGHT,
                                SUFFIX_STAGE_BEAM,
                                &started,
                            );
                        }
                        if sols.is_empty() && !time_over(&started) {
                            sols = stage_search_bestfirst(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                &BUDGETS_LATE,
                                SUFFIX_STAGE_BEAM,
                                &started,
                            );
                        }
                        if sols.is_empty() && !time_over(&started) {
                            sols = collect_exact_solutions(
                                bs,
                                input,
                                goal_colors,
                                ell,
                                target_color,
                                MAX_TARGETS_ENDGAME,
                                &started,
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
                            turn_focus_next_stage_rank(&bs.state, input, goal_colors, ell + 1),
                            bs.ops.len(),
                        )
                    });
                    if new_beam.len() > SUFFIX_STAGE_BEAM {
                        new_beam.truncate(SUFFIX_STAGE_BEAM);
                    }
                    beam = new_beam;
                }
            }
        }

        let mut best = choose_best_beamstate(beam, goal_colors);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        best
    }

    fn solve_base(input: &Input, timer: &TimeKeeper) -> BeamState {
        let init_state = State::initial(input);
        grow_to_target_prefix(
            input,
            init_state.clone(),
            &input.d[..input.m],
            timer,
            timer.exact_remaining_sec(),
        )
        .unwrap_or(BeamState {
            state: init_state,
            ops: Ops::new(),
        })
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
            if ell < input.m && exact_prefix(&st, &input.d[..input.m], ell + 1) {
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
            && exact_prefix(&bs.state, &input.d[..input.m], input.m)
            && remaining_food_count(&bs.state) == 0
    }

    fn solve_suffix_turn_focused(
        input: &Input,
        start_bs: BeamState,
        start_ell: usize,
        timer: &TimeKeeper,
    ) -> BeamState {
        grow_to_target_prefix_impl(
            input,
            start_bs,
            &input.d[..input.m],
            start_ell,
            timer,
            timer.exact_remaining_sec(),
            GrowthMode::SuffixTurnFocused,
        )
    }

    fn optimize_exact_suffix(input: &Input, base: BeamState, timer: &TimeKeeper) -> BeamState {
        if !is_complete_exact(&base, input) {
            return base;
        }

        let mut best = base;

        for &window in &SUFFIX_OPT_WINDOWS {
            if timer.exact_remaining_sec() < SUFFIX_OPT_MIN_LEFT_SEC {
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

            let cand = solve_suffix_turn_focused(input, start_bs, start_ell, timer);
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

    pub fn solve_from_common(common: &crate::CommonInput, limit_sec: f64) -> (Vec<u8>, bool) {
        if limit_sec <= 0.0 {
            return (Vec::new(), false);
        }
        let input = from_common_input(common);
        let timer = TimeKeeper::new(limit_sec.max(1e-6), 8);
        let _global_budget = push_search_budget(&timer, limit_sec.max(0.0), 0.0);
        let base = solve_base(&input, &timer);
        let turn_opt = optimize_exact_suffix(&input, base.clone(), &timer);
        let simple_opt = optimize_exact_suffix_with_simple(&input, turn_opt.clone(), &timer.start);
        let mut best = choose_best_beamstate(vec![base, turn_opt, simple_opt], &input.d[..input.m]);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        let finished = timer.exact_remaining_sec() > 1e-9;
        (best.ops, finished)
    }

    fn from_common_input(common: &crate::CommonInput) -> Input {
        let mut d = [0_u8; MAX_LEN];
        for (dst, &src) in d.iter_mut().zip(common.goal_colors.iter()) {
            *dst = src;
        }
        let mut food = [0_u8; MAX_CELLS];
        for (dst, &src) in food.iter_mut().zip(common.food.iter()) {
            *dst = src;
        }
        Input {
            n: common.n,
            m: common.m,
            d,
            food,
            manhattan: ManhattanTable::new(common.n),
        }
    }
}

#[allow(dead_code, unused_imports, unused_variables)]
mod v149_impl {
    // v149_no_logs.rs
    // メモ:
    // - 在庫戦略成功
    use std::cell::RefCell;
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
    use std::hash::{BuildHasherDefault, Hash, Hasher};
    use std::io::{self, Read};
    use std::thread_local;
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
    const ALL_DIRS: [Dir; 4] = [0, 1, 2, 3];
    const BUDGETS_NORMAL: [(usize, usize); 3] = [(2_000, 20), (8_000, 20), (25_000, 24)];
    const BUDGETS_LATE: [(usize, usize); 3] = [(4_000, 20), (12_000, 24), (40_000, 28)];
    const BUDGETS_ENDGAME_LIGHT: [(usize, usize); 2] = [(800, 16), (2_500, 20)];
    const BUDGETS_RESCUE: [(usize, usize); 2] = [(16_000, 24), (60_000, 32)];
    const ENDGAME_REMAINING_FOOD: usize = 18;
    const ENDGAME_ELL_LEFT: usize = 24;
    const LOOKAHEAD_HORIZON: usize = 6;
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
    const SIMPLE_INJECT_PER_STATE: usize = 4;
    // packed local path は u16 に 2bit/手で詰めるので、BITE_DEPTH_LIMIT は 8 以下に保つこと。
    const BITE_DEPTH_LIMIT: usize = 6;
    const BITE_CANDIDATE_WIDTH: usize = 4;

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

    #[derive(Debug, Clone)]
    struct TimeKeeper {
        start: Instant,
        time_limit_sec: f64,
        iter: u64,
        check_mask: u64,
        elapsed_sec: f64,
        progress: f64,
        is_over: bool,
    }

    #[allow(dead_code)]
    impl TimeKeeper {
        fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
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

        #[inline(always)]
        fn step(&mut self) -> bool {
            self.iter += 1;
            if (self.iter & self.check_mask) == 0 {
                self.force_update();
            }
            !self.is_over
        }

        #[inline(always)]
        fn force_update(&mut self) {
            let elapsed = self.start.elapsed().as_secs_f64();
            self.elapsed_sec = elapsed;
            self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
            self.is_over = elapsed >= self.time_limit_sec;
        }

        #[inline(always)]
        fn elapsed_sec(&self) -> f64 {
            self.elapsed_sec
        }

        #[inline(always)]
        fn progress(&self) -> f64 {
            self.progress
        }

        #[inline(always)]
        fn is_time_over(&self) -> bool {
            self.is_over
        }

        #[inline]
        fn exact_elapsed_sec(&self) -> f64 {
            self.start.elapsed().as_secs_f64()
        }

        #[inline]
        fn exact_remaining_sec(&self) -> f64 {
            (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
        }
    }

    #[derive(Clone)]
    struct SearchBudget {
        global_start: Instant,
        global_limit_sec: f64,
        local_limit_sec: f64,
        min_remaining_sec: f64,
    }

    thread_local! {
        static ACTIVE_SEARCH_BUDGET: RefCell<Option<SearchBudget>> = const { RefCell::new(None) };
    }

    #[derive(Clone, Copy)]
    struct MovementConstraint {
        min_allowed_col: u8,
        allow_special_strip: bool,
        special_col: u8,
        special_row_min: u8,
    }

    thread_local! {
        static ACTIVE_MOVEMENT_CONSTRAINT: RefCell<MovementConstraint> = const {
            RefCell::new(MovementConstraint {
                min_allowed_col: 0,
                allow_special_strip: false,
                special_col: 0,
                special_row_min: 0,
            })
        };
    }

    #[derive(Clone)]
    struct GoalPrefixHashCache {
        ptr: usize,
        len: usize,
        prefix_hash1: [u64; MAX_LEN + 1],
        prefix_hash2: [u64; MAX_LEN + 1],
    }

    thread_local! {
        static ACTIVE_GOAL_PREFIX_HASH: RefCell<Option<GoalPrefixHashCache>> = const { RefCell::new(None) };
    }

    struct ScopedSearchBudget {
        prev: Option<SearchBudget>,
    }

    impl Drop for ScopedSearchBudget {
        fn drop(&mut self) {
            ACTIVE_SEARCH_BUDGET.with(|slot| {
                *slot.borrow_mut() = self.prev.take();
            });
        }
    }

    #[inline]
    fn push_search_budget(
        timer: &TimeKeeper,
        local_limit_sec: f64,
        min_remaining_sec: f64,
    ) -> ScopedSearchBudget {
        let next = SearchBudget {
            global_start: timer.start,
            global_limit_sec: timer.time_limit_sec,
            local_limit_sec,
            min_remaining_sec,
        };
        let prev = ACTIVE_SEARCH_BUDGET.with(|slot| {
            let prev = slot.borrow().clone();
            *slot.borrow_mut() = Some(next);
            prev
        });
        ScopedSearchBudget { prev }
    }

    #[inline]
    fn current_time_left(started: &Instant) -> f64 {
        let local_elapsed = started.elapsed().as_secs_f64();
        ACTIVE_SEARCH_BUDGET.with(|slot| {
            if let Some(budget) = slot.borrow().as_ref() {
                let global_elapsed = budget.global_start.elapsed().as_secs_f64();
                let global_left =
                    (budget.global_limit_sec - budget.min_remaining_sec - global_elapsed).max(0.0);
                let local_left = (budget.local_limit_sec - local_elapsed).max(0.0);
                global_left.min(local_left)
            } else {
                (TIME_LIMIT_SEC - local_elapsed).max(0.0)
            }
        })
    }

    struct ScopedMovementConstraint {
        prev: MovementConstraint,
    }

    struct ScopedGoalPrefixHash {
        prev: Option<GoalPrefixHashCache>,
    }

    impl Drop for ScopedMovementConstraint {
        fn drop(&mut self) {
            ACTIVE_MOVEMENT_CONSTRAINT.with(|slot| {
                *slot.borrow_mut() = self.prev;
            });
        }
    }

    impl Drop for ScopedGoalPrefixHash {
        fn drop(&mut self) {
            ACTIVE_GOAL_PREFIX_HASH.with(|slot| {
                *slot.borrow_mut() = self.prev.take();
            });
        }
    }

    #[inline]
    fn push_movement_constraint(next: MovementConstraint) -> ScopedMovementConstraint {
        let prev = ACTIVE_MOVEMENT_CONSTRAINT.with(|slot| {
            let prev = *slot.borrow();
            *slot.borrow_mut() = next;
            prev
        });
        ScopedMovementConstraint { prev }
    }

    #[inline]
    fn current_movement_constraint() -> MovementConstraint {
        ACTIVE_MOVEMENT_CONSTRAINT.with(|slot| *slot.borrow())
    }

    #[inline]
    fn build_goal_prefix_hash_cache(goal_colors: &[u8]) -> GoalPrefixHashCache {
        let mut prefix_hash1 = [0_u64; MAX_LEN + 1];
        let mut prefix_hash2 = [0_u64; MAX_LEN + 1];
        for (idx, &color) in goal_colors.iter().enumerate() {
            let x = encode_color_hash(color);
            prefix_hash1[idx + 1] =
                prefix_hash1[idx].wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
            prefix_hash2[idx + 1] =
                prefix_hash2[idx].wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        }
        GoalPrefixHashCache {
            ptr: goal_colors.as_ptr() as usize,
            len: goal_colors.len(),
            prefix_hash1,
            prefix_hash2,
        }
    }

    #[inline]
    fn push_goal_prefix_hash(goal_colors: &[u8]) -> ScopedGoalPrefixHash {
        let next = build_goal_prefix_hash_cache(goal_colors);
        let prev = ACTIVE_GOAL_PREFIX_HASH.with(|slot| {
            let prev = slot.borrow().clone();
            *slot.borrow_mut() = Some(next);
            prev
        });
        ScopedGoalPrefixHash { prev }
    }

    #[inline]
    fn current_goal_prefix_hash(goal_colors: &[u8], ell: usize) -> Option<(u64, u64)> {
        ACTIVE_GOAL_PREFIX_HASH.with(|slot| {
            let cache = slot.borrow();
            let cache = cache.as_ref()?;
            if cache.ptr != goal_colors.as_ptr() as usize
                || cache.len != goal_colors.len()
                || ell > cache.len
            {
                return None;
            }
            Some((cache.prefix_hash1[ell], cache.prefix_hash2[ell]))
        })
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

    struct InventoryStateNode {
        state: State,
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
    fn is_cell_allowed(n: usize, cell: Cell) -> bool {
        let (row, col) = rc_of(cell, n);
        let constraint = current_movement_constraint();
        if col >= constraint.min_allowed_col as usize {
            return true;
        }
        constraint.allow_special_strip
            && col == constraint.special_col as usize
            && row >= constraint.special_row_min as usize
    }

    #[inline]
    fn manhattan(table: &ManhattanTable, a: Cell, b: Cell) -> usize {
        table.get(a, b)
    }

    #[inline]
    fn time_over(started: &Instant) -> bool {
        let over = current_time_left(started) <= 0.0;
        if over {}
        over
    }

    #[inline]
    fn time_left(started: &Instant) -> f64 {
        current_time_left(started)
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
            0 => {
                if head >= st.n {
                    Some((head - st.n) as Cell)
                } else {
                    None
                }
            }
            1 => {
                if head + st.n < st.n * st.n {
                    Some((head + st.n) as Cell)
                } else {
                    None
                }
            }
            2 => {
                if head % st.n != 0 {
                    Some((head - 1) as Cell)
                } else {
                    None
                }
            }
            3 => {
                if head % st.n + 1 < st.n {
                    Some((head + 1) as Cell)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[inline]
    fn next_head_cell_pos(n: usize, pos: &InternalPosDeque, dir: usize) -> Option<Cell> {
        let head = pos.head() as usize;
        match dir {
            0 => {
                if head >= n {
                    Some((head - n) as Cell)
                } else {
                    None
                }
            }
            1 => {
                if head + n < n * n {
                    Some((head + n) as Cell)
                } else {
                    None
                }
            }
            2 => {
                if head % n != 0 {
                    Some((head - 1) as Cell)
                } else {
                    None
                }
            }
            3 => {
                if head % n + 1 < n {
                    Some((head + 1) as Cell)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[inline]
    fn is_legal_dir(st: &State, dir: usize) -> bool {
        let Some(nh) = next_head_cell(st, dir) else {
            return false;
        };
        if !is_cell_allowed(st.n, nh) {
            return false;
        }
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

    #[inline]
    fn dropped_respects_active_constraint(n: usize, dropped: &DroppedBuf) -> bool {
        dropped
            .as_slice()
            .iter()
            .all(|ent| is_cell_allowed(n, ent.cell))
    }

    fn repair_prefix_after_bite(
        st_after: &State,
        _input: &Input,
        goal_colors: &[u8],
        ell: usize,
        dropped: &DroppedBuf,
    ) -> Option<PrefixRepairResult> {
        if !dropped_respects_active_constraint(st_after.n, dropped) {
            return None;
        }
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
            let need_color = goal_colors[cur_len];
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

        if exact_prefix(&state, goal_colors, ell) {
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
    fn lcp_state(st: &State, goal_colors: &[u8]) -> usize {
        let m = st.len().min(goal_colors.len());
        if m == 0 {
            return 0;
        }
        if m == st.len() {
            if let Some((h1, h2)) = current_goal_prefix_hash(goal_colors, m) {
                if st.color_hash1 == h1 && st.color_hash2 == h2 {
                    return m;
                }
            }
        }
        let lhs = &st.colors[..m];
        let rhs = &goal_colors[..m];
        let mut idx = 0usize;
        let mut lhs_chunks = lhs.chunks_exact(8);
        let mut rhs_chunks = rhs.chunks_exact(8);
        for (la, ra) in lhs_chunks.by_ref().zip(rhs_chunks.by_ref()) {
            if la != ra {
                for off in 0..8 {
                    if la[off] != ra[off] {
                        return idx + off;
                    }
                }
            }
            idx += 8;
        }
        for (&lc, &rc) in lhs_chunks
            .remainder()
            .iter()
            .zip(rhs_chunks.remainder().iter())
        {
            if lc != rc {
                return idx;
            }
            idx += 1;
        }
        m
    }

    #[inline]
    fn prefix_ok(st: &State, goal_colors: &[u8], ell: usize) -> bool {
        let keep = st.len().min(ell);
        if keep == 0 {
            return true;
        }
        if keep == st.len() {
            if let Some((h1, h2)) = current_goal_prefix_hash(goal_colors, keep) {
                if st.color_hash1 == h1 && st.color_hash2 == h2 {
                    return true;
                }
            }
        }
        st.colors[..keep] == goal_colors[..keep]
    }

    #[inline]
    fn exact_prefix(st: &State, goal_colors: &[u8], ell: usize) -> bool {
        if st.len() != ell {
            return false;
        }
        if let Some((h1, h2)) = current_goal_prefix_hash(goal_colors, ell) {
            return st.color_hash1 == h1 && st.color_hash2 == h2;
        }
        st.colors[..ell] == goal_colors[..ell]
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
            let cell = idx as Cell;
            if st.food[idx] == color && is_cell_allowed(st.n, cell) {
                cnt += 1;
                let dist = manhattan(&input.manhattan, head, cell);
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
            if !is_cell_allowed(st.n, nh) || nh == neck {
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

    fn local_score(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
    ) -> (usize, usize, usize, usize, usize) {
        let target = goal_colors[ell];
        if exact_prefix(st, goal_colors, ell) {
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

    fn next_stage_rank(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ellp1: usize,
    ) -> (usize, usize, usize) {
        if ellp1 >= goal_colors.len() {
            return (0, 0, 0);
        }
        let (dist, _) = nearest_food_dist(st, input, goal_colors[ellp1]);
        let (hr, hc) = rc_of(st.head(), st.n);
        let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
        (dist, center, 0)
    }

    fn greedy_future_lb_from_cell(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        start_cell: Cell,
        start_ell: usize,
        horizon: usize,
        banned: Option<Cell>,
    ) -> (usize, usize, usize) {
        let mut cur = start_cell;
        let end = (start_ell + horizon).min(goal_colors.len());
        let mut used = CellList::new();
        if let Some(b) = banned {
            used.push(b);
        }

        let mut miss = 0usize;
        let mut first = 0usize;
        let mut total = 0usize;

        for idx in start_ell..end {
            let color = goal_colors[idx];
            let mut best: Option<(usize, Cell)> = None;
            for cell_idx in 0..st.n * st.n {
                if st.food[cell_idx] != color {
                    continue;
                }
                let cell = cell_idx as Cell;
                if !is_cell_allowed(st.n, cell) {
                    continue;
                }
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
        goal_colors: &[u8],
        ellp1: usize,
    ) -> (usize, usize, usize, usize, usize) {
        if ellp1 >= goal_colors.len() {
            return (0, 0, 0, 0, 0);
        }
        let (miss, first, total) = greedy_future_lb_from_cell(
            st,
            input,
            goal_colors,
            st.head(),
            ellp1,
            LOOKAHEAD_HORIZON,
            None,
        );
        let adj = target_adjacent(st, goal_colors[ellp1]).is_some();
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

    fn absolute_score_state(st: &State, goal_colors: &[u8], ops_len: usize) -> usize {
        let k = st.len().min(goal_colors.len());
        let mut mismatch = 0usize;
        for idx in 0..k {
            if st.colors[idx] != goal_colors[idx] {
                mismatch += 1;
            }
        }
        ops_len + 10_000 * (mismatch + 2 * (goal_colors.len() - k))
    }

    fn beam_score_key(bs: &BeamState, goal_colors: &[u8]) -> (usize, usize, Reverse<usize>, usize) {
        (
            absolute_score_state(&bs.state, goal_colors, bs.ops.len()),
            bs.ops.len(),
            Reverse(lcp_state(&bs.state, goal_colors)),
            remaining_food_count(&bs.state),
        )
    }

    fn choose_best_beamstate(cands: Vec<BeamState>, goal_colors: &[u8]) -> BeamState {
        cands
            .into_iter()
            .min_by_key(|bs| beam_score_key(bs, goal_colors))
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
        _input: &Input,
        goal_colors: &[u8],
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
                    if sim.colors[idx] != goal_colors[idx] {
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
                        if child.state.colors[idx] != goal_colors[idx] {
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
        goal_colors: &[u8],
        ell: usize,
        started: &Instant,
    ) -> Option<BeamState> {
        if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
            return None;
        }

        let target_color = goal_colors[ell];
        if collect_food_cells(&bs.state, target_color).is_empty() {
            return None;
        }

        let safe_cfg = QuickPlanConfig {
            depth_limit: FAST_SAFE_DEPTH_LIMIT,
            node_limit: FAST_SAFE_NODE_LIMIT,
            non_target_limit: 0,
            bite_limit: 0,
        };
        if let Some(sol) =
            plan_color_goal_quick(bs, input, goal_colors, ell, target_color, safe_cfg, started)
        {
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
        if let Some(sol) = plan_color_goal_quick(
            bs,
            input,
            goal_colors,
            ell,
            target_color,
            rescue_cfg,
            started,
        ) {
            return Some(sol);
        }

        let sols = collect_exact_solutions(
            bs,
            input,
            goal_colors,
            ell,
            target_color,
            FAST_FALLBACK_TARGETS,
            started,
        );
        let out = sols.into_iter().min_by_key(|cand| cand.ops.len());
        if out.is_some() {}
        out
    }

    fn try_recover_exact(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        dropped: &DroppedBuf,
    ) -> Option<(State, Ops)> {
        let repaired = repair_prefix_after_bite(st, input, goal_colors, ell, dropped)?;
        if !exact_prefix(&repaired.state, goal_colors, ell) {
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
        goal_colors: &[u8],
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
            local_score(&start, input, goal_colors, ell),
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

            if exact_prefix(&st, goal_colors, ell) {
                if let Some(dir2) = target_adjacent(&st, goal_colors[ell]) {
                    let (ns2, _, bite2) = step(&st, dir2);
                    if bite2.is_none() && exact_prefix(&ns2, goal_colors, ell + 1) {
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
                    if bite1.is_some() && !dropped_respects_active_constraint(st.n, &dropped1) {
                        continue;
                    }
                    if bite1.is_none() || !prefix_ok(&ns1, goal_colors, ell) {
                        continue;
                    }

                    let mut rs = ns1;
                    let mut recover_ops = Ops::new();
                    if rs.len() < ell {
                        let Some((rec_state, rec_ops)) =
                            try_recover_exact(&rs, input, goal_colors, ell, &dropped1)
                        else {
                            continue;
                        };
                        rs = rec_state;
                        recover_ops = rec_ops;
                    }

                    if !exact_prefix(&rs, goal_colors, ell) {
                        continue;
                    }

                    let dirs2 = legal_dirs(&rs);
                    for &dir2_u8 in dirs2.as_slice() {
                        let dir2 = dir2_u8 as usize;
                        let nh = next_head_cell(&rs, dir2).unwrap();
                        if rs.food[nh as usize] != goal_colors[ell] {
                            continue;
                        }

                        let (ns2, _, bite2) = step(&rs, dir2);
                        if bite2.is_some() || !exact_prefix(&ns2, goal_colors, ell + 1) {
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
                if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped2) {
                    continue;
                }
                let mut seg = Ops::with_capacity(1 + ell);
                seg.push(dir as Dir);

                if bite_idx.is_some() && ns.len() < ell {
                    if !prefix_ok(&ns, goal_colors, ell) {
                        continue;
                    }
                    let Some((rec_state, rec_ops)) =
                        try_recover_exact(&ns, input, goal_colors, ell, &dropped2)
                    else {
                        continue;
                    };
                    ns = rec_state;
                    seg.extend_from_slice(&rec_ops);
                }

                if !prefix_ok(&ns, goal_colors, ell) {
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
                pq.push(Reverse((
                    local_score(&ns, input, goal_colors, ell),
                    nd,
                    uid,
                    child,
                )));
                uid += 1;
            }

            if !exact_prefix(&st, goal_colors, ell) {
                let dirs = legal_dirs(&st);
                for &dir_u8 in dirs.as_slice() {
                    let dir = dir_u8 as usize;
                    let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped1);
                    if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped1) {
                        continue;
                    }
                    if bite_idx.is_none() || !prefix_ok(&ns, goal_colors, ell) {
                        continue;
                    }

                    let mut rs = ns;
                    let mut seg = Ops::with_capacity(1 + ell);
                    seg.push(dir as Dir);

                    if rs.len() < ell {
                        let Some((rec_state, rec_ops)) =
                            try_recover_exact(&rs, input, goal_colors, ell, &dropped1)
                        else {
                            continue;
                        };
                        rs = rec_state;
                        seg.extend_from_slice(&rec_ops);
                    }

                    if !exact_prefix(&rs, goal_colors, ell) {
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
                    pq.push(Reverse((
                        local_score(&rs, input, goal_colors, ell),
                        nd,
                        uid,
                        child,
                    )));
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

        sols.sort_unstable_by_key(|bs| {
            (
                next_stage_rank(&bs.state, input, goal_colors, ell + 1),
                bs.ops.len(),
            )
        });

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
            if st.food[idx] == color && is_cell_allowed(st.n, idx as Cell) {
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
                if !is_cell_allowed(st.n, nxt) {
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
                if !is_cell_allowed(st.n, gate) {
                    continue;
                }
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
            if !is_cell_allowed(st.n, idx as Cell) {
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
        goal_colors: &[u8],
        ell: usize,
        path: &CellList,
    ) -> Option<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
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

        exact_prefix(&child_state, goal_colors, ell + 1).then_some(BeamState {
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
        goal_colors: &[u8],
        ell: usize,
        path: &CellList,
    ) -> Option<BeamState> {
        if !exact_prefix(repaired_state, goal_colors, ell) {
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

        exact_prefix(&child_state, goal_colors, ell + 1).then_some(BeamState {
            state: child_state,
            ops: child_ops,
        })
    }

    fn expand_bite_children(
        parent: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        candidate_width: usize,
    ) -> Vec<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
            return Vec::new();
        }

        let target_color = goal_colors[ell];
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
                if bite_idx.is_some() && !dropped_respects_active_constraint(node.state.n, &dropped) {
                    continue;
                }
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

                let Some(repaired) =
                    repair_prefix_after_bite(&next_state, input, goal_colors, ell, &dropped)
                else {
                    continue;
                };
                if !exact_prefix(&repaired.state, goal_colors, ell) {
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
                    goal_colors,
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
        _input: &Input,
        goal_colors: &[u8],
        ell: usize,
        candidate_width: usize,
    ) -> Vec<BeamState> {
        if !exact_prefix(&parent.state, goal_colors, ell) {
            return Vec::new();
        }

        let paths = collect_top_k_target_paths(&parent.state, goal_colors[ell], candidate_width);
        let mut out = Vec::with_capacity(paths.len());
        let mut seen: FxHashSet<State> = FxHashSet::default();
        for path in &paths {
            if let Some(child) = build_simple_child(parent, goal_colors, ell, path) {
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

    #[inline]
    fn can_reach_target_next_pos(n: usize, pos: &InternalPosDeque, target: Cell) -> bool {
        is_cell_allowed(n, target)
            && dir_between_cells(n, pos[0], target).is_some()
            && (pos.len() < 2 || target != pos[1])
    }

    fn empty_step_pos(
        n: usize,
        food: &[u8; MAX_CELLS],
        pos: &InternalPosDeque,
        dir: usize,
        target: Cell,
    ) -> Option<InternalPosDeque> {
        let nh = next_head_cell_pos(n, pos, dir)?;
        if !is_cell_allowed(n, nh) {
            return None;
        }
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
                if !is_cell_allowed(n, nxt) || blocked[idx] || seen[idx] {
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
            if is_cell_allowed(n, nb) && nb != neck && seen[nb as usize] {
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
        if !is_cell_allowed(st.n, target) {
            return false;
        }
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
        if !is_cell_allowed(n, goal) {
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
            if !is_cell_allowed(n, nid) || blocked[idx] || dist[idx] != u16::MAX {
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
                if !is_cell_allowed(n, nid) || blocked[idx] || dist[idx] != u16::MAX {
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
        goal_colors: &[u8],
        dir: usize,
        target: Cell,
        ell: usize,
        restore_queue: &mut DroppedQueue,
        dropped: &mut DroppedBuf,
    ) -> Option<(State, Ops, Option<usize>)> {
        let (ns, _, bite_idx) = step_with_dropped(st, dir, dropped);
        if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, dropped) {
            return None;
        }
        if ns.food[target as usize] == 0 {
            return None;
        }

        *restore_queue = DroppedQueue::new();

        if bite_idx.is_some() && ns.len() < ell {
            let repaired = repair_prefix_after_bite(&ns, input, goal_colors, ell, dropped)?;
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
        goal_colors: &[u8],
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
                goal_colors,
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

    fn choose_shrink_dir(
        st: &State,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
    ) -> Option<usize> {
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
            if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped) {
                continue;
            }
            if sim.food[target as usize] == 0 {
                continue;
            }

            if !prefix_ok(&sim, goal_colors, ell) {
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
        goal_colors: &[u8],
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
                choose_shrink_dir(&st, input, goal_colors, ell, target)?
            };

            let (ns, recover_ops, bite_idx) = advance_with_restore_queue(
                &st,
                input,
                goal_colors,
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

    fn finish_eat_target(
        bs: &BeamState,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
    ) -> Option<BeamState> {
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
        if ns.len() >= ell + 1 && exact_prefix(&ns, goal_colors, ell + 1) {
            ops.push(dir as Dir);
            Some(BeamState { state: ns, ops })
        } else {
            None
        }
    }

    fn try_target_empty_path(
        bs: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
        started: &Instant,
    ) -> Option<BeamState> {
        let st = &bs.state;
        if !exact_prefix(st, goal_colors, ell) {
            return None;
        }
        if st.food[target as usize] != goal_colors[ell] {
            return None;
        }
        if remaining_food_count(st) > EMPTY_PATH_REMAINING_LIMIT
            || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
        {
            return None;
        }

        if can_reach_target_next(st, target) {
            return finish_eat_target(bs, goal_colors, ell, target);
        }
        if reachable_goal_neighbor_count_pos(st.n, &st.pos, target) > 0 {
            return None;
        }
        if collect_food_cells(st, goal_colors[ell]).len() != 1 {
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
                let out = finish_eat_target(&gate_bs, goal_colors, ell, target);
                if out.is_some() {}
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
        goal_colors: &[u8],
        ell: usize,
        target: Cell,
        target_color: u8,
        started: &Instant,
    ) -> Vec<BeamState> {
        let head = bs.state.head();
        let mut cand = Vec::new();
        for &cid in neighbors(bs.state.n, target).as_slice() {
            if is_cell_allowed(bs.state.n, cid) {
                cand.push(cid);
            }
        }
        cand.sort_unstable_by_key(|&cid| {
            (
                usize::from(bs.state.food[cid as usize] > 0),
                manhattan(&input.manhattan, head, cid),
            )
        });

        let mut sols = Vec::new();

        for &goal in &cand {
            if time_over(started) {
                break;
            }

            if let Some(b1) = navigate_to_goal_safe(bs, goal, target, started) {
                if let Some(b2) =
                    shrink_to_ell(&b1, input, goal_colors, ell, target, target_color, started)
                {
                    if let Some(b3) = finish_eat_target(&b2, goal_colors, ell, target) {
                        sols.push(b3);
                    }
                }
            }

            if let Some(b1) = navigate_to_goal_loose(bs, input, goal_colors, goal, target, ell, started)
            {
                if let Some(b2) =
                    shrink_to_ell(&b1, input, goal_colors, ell, target, target_color, started)
                {
                    if let Some(b3) = finish_eat_target(&b2, goal_colors, ell, target) {
                        sols.push(b3);
                    }
                }
            }
        }

        if sols.is_empty() && is_endgame_mode(&bs.state, goal_colors.len(), ell) {
            if let Some(sol) = try_target_empty_path(bs, input, goal_colors, ell, target, started) {
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
    fn is_endgame_mode(st: &State, goal_len: usize, ell: usize) -> bool {
        goal_len - ell <= ENDGAME_ELL_LEFT && remaining_food_count(st) <= ENDGAME_REMAINING_FOOD
    }

    fn collect_exact_solutions(
        bs: &BeamState,
        input: &Input,
        goal_colors: &[u8],
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
            let cand = try_target_exact(bs, input, goal_colors, ell, target, target_color, started);
            for s in cand {
                sols.push(s);
            }
            if sols.len() >= STAGE_BEAM {
                break;
            }
        }
        sols
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
        goal_colors: &[u8],
        ell: usize,
        target_color: u8,
        started: &Instant,
    ) -> Vec<BeamState> {
        let mut order: Vec<usize> = (0..beam.len()).collect();
        order.sort_unstable_by_key(|&idx| {
            (
                local_score(&beam[idx].state, input, goal_colors, ell),
                beam[idx].ops.len(),
            )
        });

        let mut rescue_map: FxHashMap<State, Ops> = FxHashMap::default();
        for &idx in &order {
            if time_over(started) {
                break;
            }

            let bs = &beam[idx];
            let endgame_mode = is_endgame_mode(&bs.state, goal_colors.len(), ell);

            let mut sols = if endgame_mode {
                collect_exact_solutions(
                    bs,
                    input,
                    goal_colors,
                    ell,
                    target_color,
                    MAX_TARGETS_RESCUE,
                    started,
                )
            } else {
                stage_search_bestfirst(
                    bs,
                    input,
                    goal_colors,
                    ell,
                    &BUDGETS_RESCUE,
                    STAGE_BEAM,
                    started,
                )
            };

            if sols.is_empty() && !time_over(started) {
                if endgame_mode {
                    sols = stage_search_bestfirst(
                        bs,
                        input,
                        goal_colors,
                        ell,
                        &BUDGETS_ENDGAME_LIGHT,
                        STAGE_BEAM,
                        started,
                    );
                } else {
                    sols = collect_exact_solutions(
                        bs,
                        input,
                        goal_colors,
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
        out.sort_unstable_by_key(|bs| {
            (
                next_stage_rank(&bs.state, input, goal_colors, ell + 1),
                bs.ops.len(),
            )
        });
        if out.len() > STAGE_BEAM {
            out.truncate(STAGE_BEAM);
        }
        out
    }

    fn trim_stage_beam(
        cands: Vec<BeamState>,
        input: &Input,
        goal_colors: &[u8],
        next_ell: usize,
        short_lane: Option<&BeamState>,
    ) -> Vec<BeamState> {
        let mut strategic_order: Vec<usize> = (0..cands.len()).collect();
        strategic_order.sort_unstable_by_key(|&idx| {
            (
                next_stage_rank(&cands[idx].state, input, goal_colors, next_ell),
                cands[idx].ops.len(),
            )
        });

        let mut turn_order: Vec<usize> = (0..cands.len()).collect();
        turn_order.sort_unstable_by_key(|&idx| {
            (
                turn_focus_next_stage_rank(&cands[idx].state, input, goal_colors, next_ell),
                cands[idx].ops.len(),
            )
        });

        let best_short = cands.iter().min_by_key(|bs| bs.ops.len()).cloned();
        let best_turn = cands
            .iter()
            .min_by_key(|bs| {
                (
                    turn_focus_next_stage_rank(&bs.state, input, goal_colors, next_ell),
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

    // ===== Core Search =====

    fn grow_to_target_prefix(
        input: &Input,
        start_state: State,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        local_time_limit_sec: f64,
    ) -> Option<BeamState> {
        let _goal_hash = push_goal_prefix_hash(goal_colors);
        if start_state.len() > goal_colors.len() {
            return None;
        }
        for idx in 0..start_state.len() {
            if start_state.colors[idx] != goal_colors[idx] {
                return None;
            }
        }

        let start_ell = start_state.len();
        let start_bs = BeamState {
            state: start_state,
            ops: Ops::new(),
        };
        Some(grow_to_target_prefix_impl(
            input,
            start_bs,
            goal_colors,
            start_ell,
            timer,
            local_time_limit_sec,
        ))
    }

    fn grow_to_target_prefix_impl(
        input: &Input,
        start_bs: BeamState,
        goal_colors: &[u8],
        start_ell: usize,
        timer: &TimeKeeper,
        local_time_limit_sec: f64,
    ) -> BeamState {
        let _budget = push_search_budget(timer, local_time_limit_sec.max(0.0), 0.0);
        let started = Instant::now();
        let mut beam = vec![start_bs.clone()];
        let goal_len = goal_colors.len();

        for ell in start_ell..goal_len {
            if time_over(&started) {
                break;
            }

            let target_color = goal_colors[ell];
            let budgets: &[(usize, usize)] = if goal_len - ell < 10 {
                &BUDGETS_LATE
            } else {
                &BUDGETS_NORMAL
            };

            let short_seed = beam.iter().min_by_key(|bs| bs.ops.len()).cloned();
            let quick_short = short_seed
                .as_ref()
                .and_then(|bs| extend_fastlane_one(bs, input, goal_colors, ell, &started));

            let mut new_map: FxHashMap<State, Ops> = FxHashMap::default();
            if let Some(sol) = quick_short.clone() {
                insert_best_plan(&mut new_map, sol.state, sol.ops);
            }

            for bs in &beam {
                if time_over(&started) {
                    break;
                }

                if !time_over(&started) {
                    let simple_children =
                        expand_simple_children(bs, input, goal_colors, ell, SIMPLE_INJECT_PER_STATE);
                    for s in simple_children {
                        if s.ops.len() > MAX_TURNS {
                            continue;
                        }
                        insert_best_plan(&mut new_map, s.state, s.ops);
                    }

                    let bite_children =
                        expand_bite_children(bs, input, goal_colors, ell, BITE_CANDIDATE_WIDTH);
                    for s in bite_children {
                        if s.ops.len() > MAX_TURNS {
                            continue;
                        }
                        insert_best_plan(&mut new_map, s.state, s.ops);
                    }
                }

                let endgame_mode = is_endgame_mode(&bs.state, goal_len, ell);
                let mut sols = Vec::new();

                if !time_over(&started) {
                    if endgame_mode {
                        sols = collect_exact_solutions(
                            bs,
                            input,
                            goal_colors,
                            ell,
                            target_color,
                            MAX_TARGETS_ENDGAME,
                            &started,
                        );
                    } else {
                        sols = stage_search_bestfirst(
                            bs,
                            input,
                            goal_colors,
                            ell,
                            budgets,
                            STAGE_BEAM,
                            &started,
                        );
                    }
                }

                if sols.is_empty() && !time_over(&started) {
                    if endgame_mode {
                        sols = stage_search_bestfirst(
                            bs,
                            input,
                            goal_colors,
                            ell,
                            &BUDGETS_ENDGAME_LIGHT,
                            STAGE_BEAM,
                            &started,
                        );
                    } else {
                        sols = collect_exact_solutions(
                            bs,
                            input,
                            goal_colors,
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
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }
            }

            if new_map.is_empty() && !time_over(&started) {
                let rescue = rescue_stage(&beam, input, goal_colors, ell, target_color, &started);
                for s in rescue {
                    insert_best_plan(&mut new_map, s.state, s.ops);
                }
            }

            if new_map.is_empty() {
                break;
            }

            beam = trim_stage_beam(
                map_into_beamstates(new_map),
                input,
                goal_colors,
                ell + 1,
                quick_short.as_ref(),
            );
        }

        let mut best = choose_best_beamstate(beam, goal_colors);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        best
    }

    // ===== Inventory Build =====

    const INVENTORY_MIN_LEFT_SEC: f64 = 0.18;
    const INVENTORY_BUILD_TIME_CAP_SEC: f64 = 0.28;
    const INVENTORY_RESCUE_MIN_LEFT_SEC: f64 = 0.05;
    const INVENTORY_TRANSPORT_DEPTH_LIMIT: usize = 48;
    const INVENTORY_TRANSPORT_NODE_LIMIT: usize = 60_000;
    const INVENTORY_TRANSPORT_EAT_LIMIT: u8 = 8;
    const INVENTORY_TRANSPORT_BITE_LIMIT: u8 = 3;
    const INVENTORY_TRANSPORT_HARD_DEPTH_LIMIT: usize = 80;
    const INVENTORY_TRANSPORT_HARD_NODE_LIMIT: usize = 180_000;
    const INVENTORY_TRANSPORT_HARD_EAT_LIMIT: u8 = 12;
    const INVENTORY_TRANSPORT_HARD_BITE_LIMIT: u8 = 6;
    const INVENTORY_ENTRY_REPAIR_DEPTH_LIMIT: usize = 16;
    const INVENTORY_ENTRY_REPAIR_NODE_LIMIT: usize = 8_000;
    const INVENTORY_ENTRY_REPAIR_EAT_LIMIT: u8 = 2;
    const INVENTORY_ENTRY_REPAIR_BITE_LIMIT: u8 = 2;
    const INVENTORY_DIRECT_BUILD_CAP_SEC: f64 = 0.05;
    const INVENTORY_LAUNCH_DIRECT_DEPTH_LIMIT: usize = 12;
    const INVENTORY_LAUNCH_DIRECT_NODE_LIMIT: usize = 18_000;
    const INVENTORY_DEPART_DEPTH_LIMIT: usize = 18;
    const INVENTORY_DEPART_NODE_LIMIT: usize = 60_000;
    const INVENTORY_DEPART_EAT_LIMIT: u8 = 8;
    const INVENTORY_DEPART_BITE_LIMIT: u8 = 3;
    const INVENTORY_DEPART_MAX_LEN: usize = 15;
    const INVENTORY_DEPART_HARD_DEPTH_LIMIT: usize = 24;
    const INVENTORY_DEPART_HARD_NODE_LIMIT: usize = 160_000;
    const INVENTORY_DEPART_HARD_EAT_LIMIT: u8 = 12;
    const INVENTORY_DEPART_HARD_BITE_LIMIT: u8 = 4;
    const INVENTORY_DEPART_HARD_MAX_LEN: usize = 17;
    const INVENTORY_DEPART_EMPTY_GOAL: u8 = 4;
    const INVENTORY_DEPART_STATIC_DEPTH_LIMIT: usize = 24;
    const INVENTORY_DEPART_STATIC_NODE_LIMIT: usize = 50_000;
    const INVENTORY_DEPART_NORMALIZE_DEPTH_LIMIT: usize = 24;
    const INVENTORY_DEPART_NORMALIZE_NODE_LIMIT: usize = 160_000;
    const INVENTORY_DEPART_NORMALIZE_EAT_LIMIT: u8 = 12;
    const INVENTORY_DEPART_NORMALIZE_BITE_LIMIT: u8 = 8;
    const INVENTORY_LAUNCH_DEPTH_LIMIT: usize = 10;
    const INVENTORY_LAUNCH_NODE_LIMIT: usize = 30_000;
    const INVENTORY_PARKED_LAUNCH_PROBE_NODE_LIMIT: usize = 1024;
    const INVENTORY_LAUNCH_EXTRA_BUILD_CAP_SEC: f64 = 1.20;
    const INVENTORY_PARKED_EXTRA_BUILD_CAP_SEC: f64 = 1.70;
    const INVENTORY_LAUNCH_FIRST_PER_DEPTH: usize = 10;
    const INVENTORY_LAUNCH_BEST_PER_DEPTH: usize = 10;
    const INVENTORY_GREEDY_LAUNCH_STEPS: usize = 8;

    #[derive(Clone, Copy)]
    struct InventoryCtx<'a> {
        input: &'a Input,
        timer: &'a TimeKeeper,
        seg_len: usize,
        stock_cnt: usize,
    }

    #[derive(Clone, Copy)]
    struct InventoryTarget<'a> {
        colors: &'a [u8],
        protect_len: usize,
    }

    enum InventoryBuildOutcome {
        Built(BeamState),
        GrowFailed,
        ExactFailed,
    }

    #[inline]
    fn movement_constraint_with_min_col(min_allowed_col: usize) -> MovementConstraint {
        MovementConstraint {
            min_allowed_col: min_allowed_col as u8,
            allow_special_strip: false,
            special_col: 0,
            special_row_min: 0,
        }
    }

    #[inline]
    fn movement_constraint_for_harvest_entry(stock_cnt: usize, n: usize) -> MovementConstraint {
        debug_assert!(stock_cnt >= 1);
        MovementConstraint {
            min_allowed_col: (2 * stock_cnt) as u8,
            allow_special_strip: true,
            special_col: (2 * stock_cnt - 1) as u8,
            special_row_min: (n - 3) as u8,
        }
    }

    #[inline]
    fn append_incremental_beam(base: &BeamState, inc: BeamState) -> Option<BeamState> {
        let mut ops = base.ops.clone();
        ops.extend_from_slice(&inc.ops);
        if ops.len() > MAX_TURNS {
            return None;
        }
        Some(BeamState {
            state: inc.state,
            ops,
        })
    }

    fn inventory_segment_target(input: &Input, seg_idx: usize, seg_len: usize) -> Vec<u8> {
        let seg_end = input.m - seg_idx * seg_len;
        let seg_begin = seg_end - seg_len;
        let mut out = Vec::with_capacity(5 + seg_len);
        out.extend_from_slice(&input.d[..5]);
        out.extend_from_slice(&input.d[seg_begin..seg_end]);
        out
    }

    fn inventory_tail_target(input: &Input, stock_cnt: usize, seg_len: usize) -> Vec<u8> {
        let tail_end = input.m - stock_cnt * seg_len;
        input.d[..tail_end].to_vec()
    }

    #[inline]
    fn inventory_build_time_limit(timer: &TimeKeeper, phases_left: usize) -> f64 {
        let free = (timer.exact_remaining_sec() - INVENTORY_MIN_LEFT_SEC).max(0.0);
        (free / (phases_left.max(1) as f64 + 1.0)).min(INVENTORY_BUILD_TIME_CAP_SEC)
    }

    #[inline]
    fn inventory_launch_extra_build_limit(timer: &TimeKeeper, base_limit: f64) -> f64 {
        let free = (timer.exact_remaining_sec() - INVENTORY_MIN_LEFT_SEC).max(0.0);
        (base_limit * 4.0)
            .max(0.60)
            .min(INVENTORY_LAUNCH_EXTRA_BUILD_CAP_SEC)
            .min(free)
    }

    #[inline]
    fn inventory_initial_build_limit(_st: &State, build_limit: f64) -> f64 {
        if _st.len() <= 6 {
            (build_limit * 3.0)
                .max(0.90)
                .min(INVENTORY_LAUNCH_EXTRA_BUILD_CAP_SEC)
        } else {
            build_limit
        }
    }

    #[inline(always)]
    fn inventory_depart_tail_cells(n: usize, min_allowed_col: usize) -> Option<(Cell, Cell)> {
        (min_allowed_col > 0).then(|| {
            let c = min_allowed_col - 1;
            (cell_of(n - 2, c, n), cell_of(n - 1, c, n))
        })
    }

    #[inline(always)]
    fn inventory_depart_bad_count(st: &State, min_allowed_col: usize) -> usize {
        if min_allowed_col == 0 {
            return 0;
        }
        let tail_idx = st.len().saturating_sub(1);
        let tail_cells = inventory_depart_tail_cells(st.n, min_allowed_col);
        let mut bad = 0usize;
        for idx in 0..st.len() {
            let cell = st.pos[idx];
            let col = cell as usize % st.n;
            if col >= min_allowed_col {
                continue;
            }
            let tail_ok = idx == tail_idx
                && tail_cells
                    .as_ref()
                    .is_some_and(|&(a, b)| cell == a || cell == b);
            if !tail_ok {
                bad += 1;
            }
        }
        bad
    }

    #[inline(always)]
    fn inventory_is_launch_ready(st: &State, min_allowed_col: usize) -> bool {
        inventory_depart_bad_count(st, min_allowed_col) == 0
    }

    #[inline(always)]
    fn inventory_launch_prefix_ok(st: &State, goal_colors: &[u8]) -> bool {
        prefix_ok(st, goal_colors, 5)
    }

    fn inventory_try_depart_empty4_static(
        base: &BeamState,
        _input: &Input,
        goal_colors: &[u8],
        min_allowed_col: usize,
    ) -> Option<BeamState> {
        let st = &base.state;
        let n = st.n;
        let mut blocked = [false; MAX_CELLS];
        for idx in 0..st.len() {
            blocked[st.pos[idx] as usize] = true;
        }
        blocked[st.head() as usize] = false;

        let mut q = VecDeque::new();
        let mut cells = Vec::<(Cell, [Cell; 4], u8, u8, Option<usize>, u8)>::new();
        let mut seen = FxHashSet::<(Cell, [Cell; 4], u8, u8)>::default();
        cells.push((st.head(), [0; 4], 0, 0, None, 0));
        q.push_back(0usize);
        seen.insert((st.head(), [0; 4], 0, 0));
        let mut expansions = 0usize;

        while let Some(idx) = q.pop_front() {
            if expansions >= INVENTORY_DEPART_STATIC_NODE_LIMIT {
                break;
            }
            expansions += 1;

            let (cur, used, used_len, depth, _parent, _mv) = cells[idx];
            if used_len == INVENTORY_DEPART_EMPTY_GOAL {
                let mut rev = Vec::new();
                let mut cur_idx = idx;
                while let Some(parent) = cells[cur_idx].4 {
                    rev.push(cells[cur_idx].5);
                    cur_idx = parent;
                }
                rev.reverse();

                let mut sim = base.state.clone();
                let mut ops = base.ops.clone();
                let mut ok = true;
                for &dir_u8 in &rev {
                    let (ns, _, bite_idx) = step(&sim, dir_u8 as usize);
                    if bite_idx.is_some() || !inventory_launch_prefix_ok(&ns, goal_colors) {
                        ok = false;
                        break;
                    }
                    sim = ns;
                    ops.push(dir_u8);
                }
                if ok && inventory_is_launch_ready(&sim, min_allowed_col) {
                    return Some(BeamState { state: sim, ops });
                }
                continue;
            }
            if depth as usize >= INVENTORY_DEPART_STATIC_DEPTH_LIMIT {
                continue;
            }

            let mut nexts = SmallCellList::new();
            for &nxt in neighbors(n, cur).as_slice() {
                if !is_cell_allowed(n, nxt)
                    || nxt as usize % n < min_allowed_col
                    || blocked[nxt as usize]
                {
                    continue;
                }
                nexts.push(nxt);
            }

            let mut next_vec = nexts.as_slice().to_vec();
            next_vec.sort_unstable_by_key(|&nxt| {
                let (r, c) = rc_of(nxt, n);
                (usize::from(st.food[nxt as usize] != 0), usize::MAX - c, r)
            });

            for nxt in next_vec {
                let mut next_used = used;
                let mut next_used_len = used_len;
                if st.food[nxt as usize] == 0 && !used[..used_len as usize].contains(&nxt) {
                    next_used[next_used_len as usize] = nxt;
                    next_used_len += 1;
                }
                let dir = dir_between_cells(n, cur, nxt)? as u8;
                let node = (nxt, next_used, next_used_len, depth + 1, Some(idx), dir);
                if !seen.insert((node.0, node.1, node.2, node.3)) {
                    continue;
                }
                let child = cells.len();
                cells.push(node);
                q.push_back(child);
            }
        }

        None
    }

    fn inventory_depart_from_parked_search(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
        max_len: usize,
    ) -> Option<BeamState> {
        if inventory_is_launch_ready(&base.state, min_allowed_col) {
            return Some(base.clone());
        }

        let empty4 = inventory_try_depart_empty4_static(base, input, goal_colors, min_allowed_col);
        if let Some(empty4) = empty4.as_ref() {
            if inventory_is_launch_ready(&empty4.state, min_allowed_col) {
                return Some(empty4.clone());
            }
        }

        if let Some(direct) =
            inventory_try_launch_direct(base, input, goal_colors, timer, min_allowed_col)
        {
            return Some(direct);
        }

        let seed = empty4.unwrap_or_else(|| base.clone());

        let mut nodes = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        nodes.push(InventoryStateNode {
            state: seed.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            (
                inventory_depart_bad_count(&seed.state, min_allowed_col),
                usize::from(seed.state.len() != 5),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&seed.state)),
            ),
            0usize,
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(seed.state.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let cur = nodes[idx].state.clone();
            if inventory_is_launch_ready(&cur, min_allowed_col) {
                return inventory_append_launch_and_build(
                    base,
                    &nodes,
                    idx,
                    BeamState {
                        state: cur,
                        ops: Ops::new(),
                    },
                );
            }
            if depth >= depth_limit {
                continue;
            }

            let mut cands = Vec::with_capacity(4);
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < 5 || ns.len() > max_len || !inventory_launch_prefix_ok(&ns, goal_colors) {
                    continue;
                }
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }

                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }

                let nd = depth + 1;
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        inventory_depart_bad_count(&ns, min_allowed_col),
                        usize::from(ns.len() != 5),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);

            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, depth + 1, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_depart_from_parked(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
    ) -> Option<BeamState> {
        inventory_depart_from_parked_search(
            base,
            input,
            goal_colors,
            timer,
            min_allowed_col,
            INVENTORY_DEPART_DEPTH_LIMIT,
            INVENTORY_DEPART_NODE_LIMIT,
            INVENTORY_DEPART_EAT_LIMIT,
            INVENTORY_DEPART_BITE_LIMIT,
            INVENTORY_DEPART_MAX_LEN,
        )
        .or_else(|| {
            inventory_depart_from_parked_search(
                base,
                input,
                goal_colors,
                timer,
                min_allowed_col,
                INVENTORY_DEPART_HARD_DEPTH_LIMIT,
                INVENTORY_DEPART_HARD_NODE_LIMIT,
                INVENTORY_DEPART_HARD_EAT_LIMIT,
                INVENTORY_DEPART_HARD_BITE_LIMIT,
                INVENTORY_DEPART_HARD_MAX_LEN,
            )
        })
    }

    fn inventory_normalize_departed_len5(
        base: &BeamState,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
    ) -> Option<BeamState> {
        if base.state.len() == 5 {
            return Some(base.clone());
        }
        if !inventory_is_launch_ready(&base.state, min_allowed_col) {
            return None;
        }

        let mut nodes = Vec::with_capacity(INVENTORY_DEPART_NORMALIZE_NODE_LIMIT.min(8192) + 8);
        let mut depths = Vec::with_capacity(INVENTORY_DEPART_NORMALIZE_NODE_LIMIT.min(8192) + 8);
        let mut eat_counts = Vec::with_capacity(INVENTORY_DEPART_NORMALIZE_NODE_LIMIT.min(8192) + 8);
        let mut bite_counts = Vec::with_capacity(INVENTORY_DEPART_NORMALIZE_NODE_LIMIT.min(8192) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            (
                base.state.len().saturating_sub(5),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&base.state)),
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= INVENTORY_DEPART_NORMALIZE_NODE_LIMIT
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let cur = nodes[idx].state.clone();
            if cur.len() == 5 && inventory_is_launch_ready(&cur, min_allowed_col) {
                let mut ops = base.ops.clone();
                ops.extend_from_slice(&reconstruct_inventory_state_path(&nodes, idx));
                if ops.len() > MAX_TURNS {
                    return None;
                }
                return Some(BeamState { state: cur, ops });
            }

            let depth = depths[idx] as usize;
            if depth >= INVENTORY_DEPART_NORMALIZE_DEPTH_LIMIT {
                continue;
            }

            let mut cands = Vec::with_capacity(4);
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < 5
                    || !inventory_launch_prefix_ok(&ns, goal_colors)
                    || !inventory_is_launch_ready(&ns, min_allowed_col)
                {
                    continue;
                }
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }

                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > INVENTORY_DEPART_NORMALIZE_EAT_LIMIT
                    || next_bite > INVENTORY_DEPART_NORMALIZE_BITE_LIMIT
                {
                    continue;
                }

                let nd = depth + 1;
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        ns.len().saturating_sub(5),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_try_launch_direct(
        base: &BeamState,
        _input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(INVENTORY_LAUNCH_DIRECT_NODE_LIMIT.min(8192) + 8);
        let mut depths = Vec::with_capacity(INVENTORY_LAUNCH_DIRECT_NODE_LIMIT.min(8192) + 8);
        let mut eat_counts = Vec::with_capacity(INVENTORY_LAUNCH_DIRECT_NODE_LIMIT.min(8192) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        let root_residue = inventory_depart_bad_count(&base.state, min_allowed_col);
        let root_gap = goal_colors
            .len()
            .saturating_sub(lcp_state(&base.state, goal_colors));
        pq.push(Reverse((
            (root_residue, root_gap, 0usize, 0usize, 0usize),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0));
        let mut expansions = 0usize;
        let mut best_ready_idx: Option<(usize, (usize, usize, usize, usize, usize))> = None;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= INVENTORY_LAUNCH_DIRECT_NODE_LIMIT
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let depth = depths[idx] as usize;
            let cur = nodes[idx].state.clone();
            if inventory_is_launch_ready(&cur, min_allowed_col) {
                let gap = goal_colors
                    .len()
                    .saturating_sub(lcp_state(&cur, goal_colors));
                let key = (
                    gap,
                    4usize.saturating_sub(legal_dir_count(&cur)),
                    eat_counts[idx] as usize,
                    depth,
                    idx,
                );
                if best_ready_idx
                    .as_ref()
                    .map_or(true, |(_, best_key)| key < *best_key)
                {
                    best_ready_idx = Some((idx, key));
                }
            }
            if depth >= INVENTORY_LAUNCH_DIRECT_DEPTH_LIMIT {
                continue;
            }

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let (ns, ate, bite_idx) = step(&cur, dir);
                if bite_idx.is_some()
                    || ns.len() > goal_colors.len()
                    || !inventory_launch_prefix_ok(&ns, goal_colors)
                {
                    continue;
                }
                let nd = depth + 1;
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                if seen.get(&ns).is_some_and(|&(best_depth, best_eat)| {
                    best_depth <= nd as u8 && best_eat <= next_eat
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat));
                let residue = inventory_depart_bad_count(&ns, min_allowed_col);
                let gap = goal_colors
                    .len()
                    .saturating_sub(lcp_state(&ns, goal_colors));
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns.clone(),
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push(nd as u8);
                eat_counts.push(next_eat);
                pq.push(Reverse((
                    (
                        residue,
                        gap,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        next_eat as usize,
                        nd,
                    ),
                    uid,
                    child,
                )));
                uid += 1;
            }
        }

        best_ready_idx.and_then(|(idx, _)| {
            inventory_append_launch_and_build(
                base,
                &nodes,
                idx,
                BeamState {
                    state: nodes[idx].state.clone(),
                    ops: Ops::new(),
                },
            )
        })
    }

    fn inventory_prepare_build_start(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        goal_colors: &[u8],
        _phases_left: usize,
        min_allowed_col: usize,
    ) -> Option<BeamState> {
        if base.state.len() != 5 || min_allowed_col == 0 {
            return Some(base.clone());
        }
        let departed =
            inventory_depart_from_parked(base, ctx.input, goal_colors, ctx.timer, min_allowed_col)?;
        if departed.state.len() == 5 {
            return Some(departed);
        }
        inventory_normalize_departed_len5(&departed, goal_colors, ctx.timer, min_allowed_col)
            .or(Some(departed))
    }

    #[inline(always)]
    fn inventory_build_target_exact(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        target: &InventoryTarget<'_>,
        phases_left: usize,
    ) -> InventoryBuildOutcome {
        let build_limit = inventory_build_time_limit(ctx.timer, phases_left);
        let grown = grow_to_target_prefix(
            ctx.input,
            base.state.clone(),
            target.colors,
            ctx.timer,
            inventory_initial_build_limit(&base.state, build_limit),
        );

        let built = if let Some(grown) = grown {
            if exact_prefix(&grown.state, target.colors, target.protect_len) {
                append_incremental_beam(base, grown)
            } else {
                inventory_try_launch_rescue_build(
                    base,
                    ctx.input,
                    target.colors,
                    ctx.timer,
                    build_limit,
                    true,
                )
            }
        } else {
            inventory_try_launch_rescue_build(
                base,
                ctx.input,
                target.colors,
                ctx.timer,
                build_limit,
                true,
            )
        };

        match built {
            Some(next_bs) => InventoryBuildOutcome::Built(next_bs),
            None => InventoryBuildOutcome::ExactFailed,
        }
    }

    fn inventory_build_target_exact_legacy_parked(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        target: &InventoryTarget<'_>,
        phases_left: usize,
    ) -> InventoryBuildOutcome {
        let build_limit = inventory_build_time_limit(ctx.timer, phases_left);
        let Some(grown) = grow_to_target_prefix(
            ctx.input,
            base.state.clone(),
            target.colors,
            ctx.timer,
            inventory_initial_build_limit(&base.state, build_limit),
        ) else {
            return InventoryBuildOutcome::GrowFailed;
        };

        let built = if exact_prefix(&grown.state, target.colors, target.protect_len) {
            append_incremental_beam(base, grown)
        } else {
            let gap = target
                .protect_len
                .saturating_sub(lcp_state(&grown.state, target.colors));
            if gap >= 6 {
                inventory_try_extra_build(base, ctx.input, target.colors, ctx.timer, build_limit)
                    .or_else(|| {
                        inventory_try_greedy_launch_build(
                            base,
                            ctx.input,
                            target.colors,
                            ctx.timer,
                            build_limit,
                        )
                    })
                    .or_else(|| {
                        inventory_try_launch_rescue_build(
                            base,
                            ctx.input,
                            target.colors,
                            ctx.timer,
                            build_limit,
                            false,
                        )
                    })
            } else {
                inventory_try_greedy_launch_build(
                    base,
                    ctx.input,
                    target.colors,
                    ctx.timer,
                    build_limit,
                )
                .or_else(|| {
                    inventory_try_launch_rescue_build(
                        base,
                        ctx.input,
                        target.colors,
                        ctx.timer,
                        build_limit,
                        false,
                    )
                })
                .or_else(|| {
                    inventory_try_extra_build(base, ctx.input, target.colors, ctx.timer, build_limit)
                })
            }
        };

        match built {
            Some(next_bs) => InventoryBuildOutcome::Built(next_bs),
            None => InventoryBuildOutcome::ExactFailed,
        }
    }

    fn inventory_try_direct_build_from_current(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        target: &InventoryTarget<'_>,
        phases_left: usize,
    ) -> Option<BeamState> {
        let build_limit = inventory_build_time_limit(ctx.timer, phases_left)
            .min(INVENTORY_DIRECT_BUILD_CAP_SEC)
            .min((ctx.timer.exact_remaining_sec() - INVENTORY_MIN_LEFT_SEC).max(0.0));
        if build_limit <= 0.0 {
            return None;
        }
        let grown = grow_to_target_prefix(
            ctx.input,
            base.state.clone(),
            target.colors,
            ctx.timer,
            build_limit,
        )?;
        if !exact_prefix(&grown.state, target.colors, target.protect_len) {
            return None;
        }
        append_incremental_beam(base, grown)
    }

    fn inventory_try_transportable_build_from_parked(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        target: &InventoryTarget<'_>,
        phases_left: usize,
        seg_idx: usize,
    ) -> Option<BeamState> {
        if base.state.len() != 5 {
            return None;
        }
        let geom = inventory_entry_geometry(ctx.input.n, seg_idx)?;
        let build_limit = inventory_build_time_limit(ctx.timer, phases_left);
        let rescue_build_limit = inventory_launch_extra_build_limit(ctx.timer, build_limit);
        inventory_try_launch_rescue_build_with_validator(
            base,
            ctx.input,
            target.colors,
            ctx.timer,
            rescue_build_limit,
            true,
            |candidate| {
                inventory_transport_oracle_direct_plan(
                    &candidate.state,
                    target.colors,
                    target.protect_len,
                    geom,
                )
                .is_some()
                    || inventory_transport_plan_with_limits(
                        &candidate.state,
                        ctx.input,
                        target.colors,
                        target.protect_len,
                        geom,
                        &ctx.timer.start,
                        48,
                        60_000,
                        8,
                        3,
                    )
                    .is_some()
            },
        )
    }

    // ===== Inventory Transport =====

    #[derive(Clone, Copy)]
    struct InventoryEntryGeometry {
        entry_hi: Cell,
        entry_lo: Cell,
        pre_hi: Cell,
        pre_lo: Cell,
    }

    #[inline]
    fn inventory_entry_geometry(n: usize, seg_idx: usize) -> Option<InventoryEntryGeometry> {
        let entry_col = 2 * seg_idx + 1;
        let pre_col = entry_col + 1;
        if pre_col >= n {
            return None;
        }
        Some(InventoryEntryGeometry {
            entry_hi: cell_of(n - 2, entry_col, n),
            entry_lo: cell_of(n - 1, entry_col, n),
            pre_hi: cell_of(n - 2, pre_col, n),
            pre_lo: cell_of(n - 1, pre_col, n),
        })
    }

    #[inline]
    fn inventory_is_entry_cell(geom: InventoryEntryGeometry, cell: Cell) -> bool {
        cell == geom.entry_hi || cell == geom.entry_lo
    }

    #[inline]
    fn can_commit_inventory_entry_from_right_state(st: &State, geom: InventoryEntryGeometry) -> bool {
        if st.head() != geom.pre_hi && st.head() != geom.pre_lo {
            return false;
        }
        let Some(nh) = next_head_cell(st, 2) else {
            return false;
        };
        inventory_is_entry_cell(geom, nh) && (st.len() < 2 || nh != st.neck())
    }

    // ===== Inventory Placement =====

    fn inventory_place_from_entry(
        st: &State,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        n: usize,
    ) -> Option<(State, Ops)> {
        let mut cur = st.clone();
        let mut ops = Ops::with_capacity(2 * n + 6);
        let entry_ok = |cell: Cell| inventory_is_entry_cell(geom, cell);

        if cur.head() == geom.entry_hi {
            ops.extend_from_slice(&[1, 2]);
        } else if cur.head() == geom.entry_lo {
            ops.push(2);
        } else {
            return None;
        }

        for _ in 0..n - 1 {
            ops.push(0);
        }
        ops.push(3);
        for _ in 0..n - 2 {
            ops.push(1);
        }
        ops.extend_from_slice(&[1, 2, 0, 3]);

        let mut sim_ops = Ops::with_capacity(ops.len());
        for (step_idx, &dir_u8) in ops.iter().enumerate() {
            let dir = dir_u8 as usize;
            if !is_legal_dir(&cur, dir) {
                return None;
            }
            let (ns, _, bite_idx) = step(&cur, dir);
            let is_last = step_idx + 1 == ops.len();
            if !is_last {
                if ns.len() < protect_len || !prefix_ok(&ns, protect_colors, protect_len) {
                    return None;
                }
                if bite_idx.is_some() && !entry_ok(ns.head()) {
                    return None;
                }
            } else if bite_idx.is_none() || ns.len() != 5 {
                return None;
            }
            cur = ns;
            sim_ops.push(dir_u8);
        }

        Some((cur, sim_ops))
    }

    fn inventory_append_launch_and_build(
        base: &BeamState,
        nodes: &[InventoryStateNode],
        launch_idx: usize,
        grown: BeamState,
    ) -> Option<BeamState> {
        let mut ops = base.ops.clone();
        let mut rev = Vec::new();
        let mut cur = launch_idx;
        while let Some(parent) = nodes[cur].parent {
            rev.push(nodes[cur].mv);
            cur = parent;
        }
        rev.reverse();
        ops.extend_from_slice(&rev);
        ops.extend_from_slice(&grown.ops);
        if ops.len() > MAX_TURNS {
            return None;
        }
        Some(BeamState {
            state: grown.state,
            ops,
        })
    }

    fn inventory_try_greedy_launch_build(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        build_limit_sec: f64,
    ) -> Option<BeamState> {
        let mut st = base.state.clone();
        let mut launch_ops = Ops::new();
        let keep_prefix_len = lcp_state(&base.state, goal_colors).max(5);

        for _ in 0..INVENTORY_GREEDY_LAUNCH_STEPS {
            let mut best: Option<((usize, usize, usize, usize, usize, usize), State, u8)> = None;
            for &dir_u8 in legal_dirs(&st).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped);
                if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped) {
                    continue;
                }
                if ns.len() > goal_colors.len() || !prefix_ok(&ns, goal_colors, keep_prefix_len) {
                    continue;
                }
                let (hr, hc) = rc_of(ns.head(), ns.n);
                let center = hr.abs_diff(ns.n / 2) + hc.abs_diff(ns.n / 2);
                let key = (
                    goal_colors
                        .len()
                        .saturating_sub(lcp_state(&ns, goal_colors)),
                    4usize.saturating_sub(legal_dir_count(&ns)),
                    center,
                    hr,
                    usize::MAX - hc,
                    usize::from(dir == 1),
                );
                if best
                    .as_ref()
                    .map_or(true, |(best_key, _, _)| key < *best_key)
                {
                    best = Some((key, ns, dir_u8));
                }
            }
            let Some((_, ns, dir_u8)) = best else {
                break;
            };
            st = ns;
            launch_ops.push(dir_u8);
            if let Some(grown) =
                grow_to_target_prefix(input, st.clone(), goal_colors, timer, build_limit_sec)
            {
                if exact_prefix(&grown.state, goal_colors, goal_colors.len()) {
                    let mut ops = base.ops.clone();
                    ops.extend_from_slice(&launch_ops);
                    ops.extend_from_slice(&grown.ops);
                    if ops.len() > MAX_TURNS {
                        return None;
                    }
                    return Some(BeamState {
                        state: grown.state,
                        ops,
                    });
                }
            }
        }

        None
    }

    fn inventory_try_extra_build(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        build_limit_sec: f64,
    ) -> Option<BeamState> {
        let extra_build_limit = if base.state.len() == 5 {
            (timer.exact_remaining_sec() - INVENTORY_MIN_LEFT_SEC)
                .max(0.0)
                .min(INVENTORY_PARKED_EXTRA_BUILD_CAP_SEC)
        } else {
            inventory_launch_extra_build_limit(timer, build_limit_sec)
        };
        if extra_build_limit <= build_limit_sec {
            return None;
        }
        let grown = grow_to_target_prefix(
            input,
            base.state.clone(),
            goal_colors,
            timer,
            extra_build_limit,
        )?;
        exact_prefix(&grown.state, goal_colors, goal_colors.len())
            .then(|| append_incremental_beam(base, grown))
            .flatten()
    }

    fn inventory_try_launch_rescue_build_with_validator<F>(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        build_limit_sec: f64,
        allow_extra_build: bool,
        mut accept: F,
    ) -> Option<BeamState>
    where
        F: FnMut(&BeamState) -> bool,
    {
        let mut nodes = Vec::with_capacity(INVENTORY_LAUNCH_NODE_LIMIT.min(8192) + 8);
        let mut depths = Vec::with_capacity(INVENTORY_LAUNCH_NODE_LIMIT.min(8192) + 8);
        let mut q = VecDeque::new();
        let mut seen = FxHashSet::<State>::default();
        let mut all_by_depth = vec![Vec::<usize>::new(); INVENTORY_LAUNCH_DEPTH_LIMIT + 1];
        let mut first_by_depth = vec![Vec::<usize>::new(); INVENTORY_LAUNCH_DEPTH_LIMIT + 1];
        let mut ranked_by_depth = vec![Vec::<usize>::new(); INVENTORY_LAUNCH_DEPTH_LIMIT + 1];
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        q.push_back(0usize);
        seen.insert(base.state.clone());
        let mut expansions = 0usize;
        let parked_probe_limit = inventory_initial_build_limit(&base.state, build_limit_sec);
        let keep_prefix_len = lcp_state(&base.state, goal_colors).max(5);

        while let Some(idx) = q.pop_front() {
            if expansions >= INVENTORY_LAUNCH_NODE_LIMIT
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let depth = depths[idx] as usize;
            let cur = nodes[idx].state.clone();
            if depth != 0 {
                all_by_depth[depth].push(idx);
                if first_by_depth[depth].len() < INVENTORY_LAUNCH_FIRST_PER_DEPTH {
                    first_by_depth[depth].push(idx);
                }
                ranked_by_depth[depth].push(idx);
            }
            if depth >= INVENTORY_LAUNCH_DEPTH_LIMIT {
                continue;
            }

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, _, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }
                if ns.len() > goal_colors.len() || !prefix_ok(&ns, goal_colors, keep_prefix_len) {
                    continue;
                }
                if !seen.insert(ns.clone()) {
                    continue;
                }
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                q.push_back(child);
            }
        }

        if base.state.len() == 5 {
            let mut probed = 0usize;
            'outer: for depth in (1..=INVENTORY_LAUNCH_DEPTH_LIMIT).rev() {
                for &idx in &all_by_depth[depth] {
                    if probed >= INVENTORY_PARKED_LAUNCH_PROBE_NODE_LIMIT
                        || time_over(&timer.start)
                        || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
                    {
                        break 'outer;
                    }
                    probed += 1;
                    if let Some(grown) = grow_to_target_prefix(
                        input,
                        nodes[idx].state.clone(),
                        goal_colors,
                        timer,
                        parked_probe_limit,
                    ) {
                        if exact_prefix(&grown.state, goal_colors, goal_colors.len()) {
                            if let Some(next_bs) =
                                inventory_append_launch_and_build(base, &nodes, idx, grown)
                            {
                                if accept(&next_bs) {
                                    return Some(next_bs);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut eval_list = Vec::new();
        let mut used = FxHashSet::<usize>::default();
        for depth in 1..=INVENTORY_LAUNCH_DEPTH_LIMIT {
            for &idx in &first_by_depth[depth] {
                if used.insert(idx) {
                    eval_list.push(idx);
                }
            }

            ranked_by_depth[depth].sort_unstable_by_key(|&idx| {
                let st = &nodes[idx].state;
                let (hr, hc) = rc_of(st.head(), st.n);
                let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
                (
                    goal_colors.len().saturating_sub(lcp_state(st, goal_colors)),
                    4usize.saturating_sub(legal_dir_count(st)),
                    center,
                    hr,
                    usize::MAX - hc,
                )
            });
            let take = ranked_by_depth[depth]
                .len()
                .min(INVENTORY_LAUNCH_BEST_PER_DEPTH);
            for &idx in ranked_by_depth[depth].iter().take(take) {
                if used.insert(idx) {
                    eval_list.push(idx);
                }
            }
        }

        for idx in eval_list {
            if time_over(&timer.start) || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC {
                break;
            }
            let cur = nodes[idx].state.clone();
            if let Some(grown) = grow_to_target_prefix(input, cur, goal_colors, timer, build_limit_sec)
            {
                if exact_prefix(&grown.state, goal_colors, goal_colors.len()) {
                    if let Some(next_bs) = inventory_append_launch_and_build(base, &nodes, idx, grown) {
                        if accept(&next_bs) {
                            return Some(next_bs);
                        }
                    }
                }
            }
        }

        if allow_extra_build {
            if let Some(next_bs) =
                inventory_try_extra_build(base, input, goal_colors, timer, build_limit_sec)
            {
                if accept(&next_bs) {
                    return Some(next_bs);
                }
            }
        }

        None
    }

    fn inventory_try_launch_rescue_build(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        build_limit_sec: f64,
        allow_extra_build: bool,
    ) -> Option<BeamState> {
        inventory_try_launch_rescue_build_with_validator(
            base,
            input,
            goal_colors,
            timer,
            build_limit_sec,
            allow_extra_build,
            |_| true,
        )
    }

    #[inline]
    fn try_commit_inventory_entry_and_validate_place(
        st: &State,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<State> {
        if !can_commit_inventory_entry_from_right_state(st, geom) {
            return None;
        }
        let (ns, _, bite_idx) = step(st, 2);
        if bite_idx.is_some() && !inventory_is_entry_cell(geom, ns.head()) {
            return None;
        }
        if !inventory_is_entry_cell(geom, ns.head())
            || ns.len() < protect_len
            || !prefix_ok(&ns, protect_colors, protect_len)
        {
            return None;
        }
        inventory_place_from_entry(&ns, protect_colors, protect_len, geom, st.n)?;
        Some(ns)
    }

    #[inline]
    fn inventory_transport_finish_kind(
        st: &State,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<(State, Option<Dir>)> {
        if inventory_is_entry_cell(geom, st.head())
            && inventory_place_from_entry(st, protect_colors, protect_len, geom, st.n).is_some()
        {
            return Some((st.clone(), None));
        }
        try_commit_inventory_entry_and_validate_place(st, protect_colors, protect_len, geom)
            .map(|ns| (ns, Some(2)))
    }

    #[inline]
    fn reconstruct_inventory_state_path(nodes: &[InventoryStateNode], mut idx: usize) -> Ops {
        let mut rev = Vec::new();
        while let Some(parent) = nodes[idx].parent {
            rev.push(nodes[idx].mv);
            idx = parent;
        }
        rev.reverse();
        rev
    }

    fn inventory_transport_finish_from_node(
        nodes: &[InventoryStateNode],
        idx: usize,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<BeamState> {
        let finish =
            inventory_transport_finish_kind(&nodes[idx].state, protect_colors, protect_len, geom)?;
        let mut ops = reconstruct_inventory_state_path(nodes, idx);
        if let Some(dir) = finish.1 {
            ops.push(dir);
        }
        Some(BeamState {
            state: finish.0,
            ops,
        })
    }

    fn inventory_transport_finish_from_node_with_suffix(
        nodes: &[InventoryStateNode],
        idx: usize,
        suffix: &BeamState,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<BeamState> {
        let finish = inventory_transport_finish_kind(&suffix.state, protect_colors, protect_len, geom)?;
        let mut ops = reconstruct_inventory_state_path(nodes, idx);
        ops.extend_from_slice(&suffix.ops);
        if let Some(dir) = finish.1 {
            ops.push(dir);
        }
        Some(BeamState {
            state: finish.0,
            ops,
        })
    }

    #[inline]
    fn inventory_transport_plan_rank(
        st: &State,
        input: &Input,
        protect_len: usize,
        geom: InventoryEntryGeometry,
        eat_count: u8,
        bite_count: u8,
        depth: usize,
    ) -> (usize, usize, usize, usize, usize, usize, usize) {
        let goal_dist = inventory_ready_rescue_goal_dist(st, geom, &input.manhattan);
        let entry_dist = [geom.entry_hi, geom.entry_lo]
            .iter()
            .map(|&goal| manhattan(&input.manhattan, st.head(), goal))
            .min()
            .unwrap_or(usize::MAX);
        (
            goal_dist,
            entry_dist,
            st.len().saturating_sub(protect_len),
            bite_count as usize,
            eat_count as usize,
            4usize.saturating_sub(legal_dir_count(st)),
            depth,
        )
    }

    fn inventory_transport_plan_with_limits(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        started: &Instant,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(InventoryStateNode {
            state: st.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            inventory_transport_plan_rank(st, input, protect_len, geom, 0, 0, 0),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(started)
                || time_left(started) < INVENTORY_RESCUE_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            if let Some(done) =
                inventory_transport_finish_from_node(&nodes, idx, protect_colors, protect_len, geom)
            {
                return Some(done);
            }

            let cur = nodes[idx].state.clone();
            if let Some(suffix) =
                inventory_transport_oracle_direct_plan(&cur, protect_colors, protect_len, geom)
            {
                if let Some(done) = inventory_transport_finish_from_node_with_suffix(
                    &nodes,
                    idx,
                    &suffix,
                    protect_colors,
                    protect_len,
                    geom,
                ) {
                    return Some(done);
                }
            }

            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }
            let mut cands: Vec<(
                (usize, usize, usize, usize, usize, usize, usize),
                State,
                u8,
                u8,
                u8,
            )> = Vec::with_capacity(4);

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let Some((ns, ate, bit)) =
                    inventory_transport_step_state(&cur, dir, protect_colors, protect_len)
                else {
                    continue;
                };

                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bit));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }

                if inventory_is_entry_cell(geom, ns.head()) {
                    if inventory_transport_finish_kind(&ns, protect_colors, protect_len, geom).is_some()
                    {
                        let child = nodes.len();
                        nodes.push(InventoryStateNode {
                            state: ns,
                            parent: Some(idx),
                            mv: dir_u8,
                        });
                        depths.push((depth + 1) as u8);
                        eat_counts.push(next_eat);
                        bite_counts.push(next_bite);
                        return inventory_transport_finish_from_node(
                            &nodes,
                            child,
                            protect_colors,
                            protect_len,
                            geom,
                        );
                    }
                    continue;
                }

                let nd = depth + 1;
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    inventory_transport_plan_rank(
                        &ns,
                        input,
                        protect_len,
                        geom,
                        next_eat,
                        next_bite,
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }

            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_transport_plan(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        started: &Instant,
    ) -> Option<BeamState> {
        inventory_transport_plan_with_limits(
            st,
            input,
            protect_colors,
            protect_len,
            geom,
            started,
            INVENTORY_TRANSPORT_DEPTH_LIMIT,
            INVENTORY_TRANSPORT_NODE_LIMIT,
            INVENTORY_TRANSPORT_EAT_LIMIT,
            INVENTORY_TRANSPORT_BITE_LIMIT,
        )
    }

    #[inline]
    fn inventory_ready_rescue_goal_dist(
        st: &State,
        geom: InventoryEntryGeometry,
        manhattan_table: &ManhattanTable,
    ) -> usize {
        [geom.pre_hi, geom.pre_lo]
            .iter()
            .map(|&goal| manhattan(manhattan_table, st.head(), goal))
            .min()
            .unwrap_or(usize::MAX)
    }

    fn inventory_transport_oracle_direct_plan(
        st: &State,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<BeamState> {
        let n = st.n;
        let cell_count = n * n;
        let suffix_bite_idx = protect_len.saturating_sub(1);
        let mut dropped = DroppedBuf::new();

        let mut front_idx = [u16::MAX; MAX_CELLS];
        for idx in 0..st.pos.len() {
            let cell_idx = st.pos[idx] as usize;
            if front_idx[cell_idx] == u16::MAX {
                front_idx[cell_idx] = idx as u16;
            }
        }

        let mut release = [0_u16; MAX_CELLS];
        for cell_idx in 0..cell_count {
            let idx = front_idx[cell_idx];
            if idx == u16::MAX {
                continue;
            }
            let front = idx as usize;
            if front < suffix_bite_idx {
                release[cell_idx] = (st.len() - 1 - front).max(1) as u16;
            }
        }

        let start = st.head();
        let mut dist = [u16::MAX; MAX_CELLS];
        let mut prev = [u16::MAX; MAX_CELLS];
        let mut prev_dir = [u8::MAX; MAX_CELLS];
        let mut q = [0_u16; MAX_CELLS];
        let mut q_head = 0usize;
        let mut q_tail = 0usize;
        dist[start as usize] = 0;
        q[q_tail] = start;
        q_tail += 1;

        while q_head < q_tail {
            let cur = q[q_head];
            q_head += 1;
            let cur_dist = dist[cur as usize];
            for &dir_u8 in ALL_DIRS.as_slice() {
                let dir = dir_u8 as usize;
                let Some(nxt) = next_head_cell_pos(n, &InternalPosDeque::from_slice(&[cur]), dir)
                else {
                    continue;
                };
                if !is_cell_allowed(n, nxt) {
                    continue;
                }
                if cur == start && st.len() >= 2 && nxt == st.neck() {
                    continue;
                }
                let nd = cur_dist.saturating_add(1);
                let nxt_idx = nxt as usize;
                if dist[nxt_idx] != u16::MAX || nd < release[nxt_idx] {
                    continue;
                }
                dist[nxt_idx] = nd;
                prev[nxt_idx] = cur;
                prev_dir[nxt_idx] = dir as u8;
                q[q_tail] = nxt;
                q_tail += 1;
            }
        }

        let mut candidates = [
            (dist[geom.pre_hi as usize], geom.pre_hi),
            (dist[geom.pre_lo as usize], geom.pre_lo),
            (dist[geom.entry_hi as usize], geom.entry_hi),
            (dist[geom.entry_lo as usize], geom.entry_lo),
        ];
        candidates.sort_unstable_by_key(|&(d, _)| d);

        for &(goal_dist, goal) in &candidates {
            if goal_dist == u16::MAX {
                continue;
            }
            let mut rev = Vec::new();
            let mut cur = goal;
            while cur != start {
                let dir_u8 = prev_dir[cur as usize];
                if dir_u8 == u8::MAX {
                    rev.clear();
                    break;
                }
                rev.push(dir_u8);
                cur = prev[cur as usize];
            }
            if rev.is_empty() && goal != start {
                continue;
            }
            rev.reverse();

            let mut cur_state = st.clone();
            let mut ops = Ops::with_capacity(rev.len());
            let mut ok = true;
            for (step_idx, &dir_u8) in rev.iter().enumerate() {
                let (ns, _, bite_idx) = step_with_dropped(&cur_state, dir_u8 as usize, &mut dropped);
                let is_last = step_idx + 1 == rev.len();
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur_state.n, &dropped) {
                    ok = false;
                    break;
                }
                if ns.len() < protect_len
                    || !prefix_ok(&ns, protect_colors, protect_len)
                    || (!is_last && inventory_is_entry_cell(geom, ns.head()))
                {
                    ok = false;
                    break;
                }
                cur_state = ns;
                ops.push(dir_u8);
            }
            if ok {
                if inventory_is_entry_cell(geom, cur_state.head()) {
                    inventory_place_from_entry(&cur_state, protect_colors, protect_len, geom, n)?;
                    return Some(BeamState {
                        state: cur_state,
                        ops,
                    });
                }
                if try_commit_inventory_entry_and_validate_place(
                    &cur_state,
                    protect_colors,
                    protect_len,
                    geom,
                )
                .is_some()
                {
                    return Some(BeamState {
                        state: cur_state,
                        ops,
                    });
                }
            }
        }

        None
    }

    #[inline]
    fn compute_body_release_dist_general(st: &State, block_food: bool) -> CellSearchResult {
        let mut front_idx = [u16::MAX; MAX_CELLS];
        for idx in 0..st.pos.len() {
            let cell_idx = st.pos[idx] as usize;
            if front_idx[cell_idx] == u16::MAX {
                front_idx[cell_idx] = idx as u16;
            }
        }

        let mut release = [0_u16; MAX_CELLS];
        for cell_idx in 0..st.n * st.n {
            if front_idx[cell_idx] != u16::MAX {
                let t = (st.pos.len() - 1 - front_idx[cell_idx] as usize).max(1) as u16;
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
                if !is_cell_allowed(st.n, nxt) {
                    continue;
                }
                if block_food && st.food[nxt_idx] != 0 {
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

    fn choose_inventory_transport_dir(
        st: &State,
        goal: Cell,
        protect_colors: &[u8],
        protect_len: usize,
        input: &Input,
    ) -> Option<usize> {
        let bfs = compute_body_release_dist_general(st, false);
        if let Some(path) = reconstruct_cell_search_path(&bfs, goal) {
            let slice = path.as_slice();
            if slice.len() >= 2 {
                if let Some(dir) = dir_between_cells(st.n, slice[0], slice[1]) {
                    if inventory_transport_step_state(st, dir, protect_colors, protect_len).is_some() {
                        return Some(dir);
                    }
                }
            }
        }

        let mut best: Option<((usize, usize, usize, usize), usize)> = None;
        for &dir_u8 in legal_dirs(st).as_slice() {
            let dir = dir_u8 as usize;
            let Some((ns, ate, bit)) =
                inventory_transport_step_state(st, dir, protect_colors, protect_len)
            else {
                continue;
            };
            let key = (
                manhattan(&input.manhattan, ns.head(), goal),
                usize::from(ate != 0 && ns.head() != goal) + usize::from(bit),
                4usize.saturating_sub(legal_dir_count(&ns)),
                ns.len().saturating_sub(protect_len),
            );
            if best.as_ref().map_or(true, |(best_key, _)| key < *best_key) {
                best = Some((key, dir));
            }
        }
        best.map(|(_, dir)| dir)
    }

    fn inventory_transport_to_goal_oracle_plan(
        st: &State,
        goal: Cell,
        protect_colors: &[u8],
        protect_len: usize,
    ) -> Option<BeamState> {
        let bfs = compute_body_release_dist_general(st, false);
        let path = reconstruct_cell_search_path(&bfs, goal)?;
        let slice = path.as_slice();
        if slice.len() <= 1 {
            return (st.head() == goal).then(|| BeamState {
                state: st.clone(),
                ops: Ops::new(),
            });
        }

        let mut cur = st.clone();
        let mut ops = Ops::with_capacity(slice.len().saturating_sub(1));
        for win in slice.windows(2) {
            let dir = dir_between_cells(st.n, win[0], win[1])?;
            let Some((ns, _, _)) =
                inventory_transport_step_state(&cur, dir, protect_colors, protect_len)
            else {
                return None;
            };
            cur = ns;
            ops.push(dir as u8);
        }
        (cur.head() == goal).then_some(BeamState { state: cur, ops })
    }

    fn inventory_transport_to_goal_plan_with_limits(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        goal: Cell,
        started: &Instant,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        if let Some(plan) =
            inventory_transport_to_goal_oracle_plan(st, goal, protect_colors, protect_len)
        {
            return Some(plan);
        }

        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(InventoryStateNode {
            state: st.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            (
                manhattan(&input.manhattan, st.head(), goal),
                st.len().saturating_sub(protect_len),
                4usize.saturating_sub(legal_dir_count(st)),
                0usize,
                0usize,
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(started)
                || time_left(started) < INVENTORY_RESCUE_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let cur = nodes[idx].state.clone();
            if cur.head() == goal {
                return Some(BeamState {
                    state: cur,
                    ops: reconstruct_inventory_state_path(&nodes, idx),
                });
            }
            if let Some(suffix) =
                inventory_transport_to_goal_oracle_plan(&cur, goal, protect_colors, protect_len)
            {
                let mut ops = reconstruct_inventory_state_path(&nodes, idx);
                ops.extend_from_slice(&suffix.ops);
                return Some(BeamState {
                    state: suffix.state,
                    ops,
                });
            }

            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }

            let mut cands: Vec<(
                (usize, usize, usize, usize, usize, usize),
                State,
                u8,
                u8,
                u8,
            )> = Vec::with_capacity(4);
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let Some((ns, ate, bit)) =
                    inventory_transport_step_state(&cur, dir_u8 as usize, protect_colors, protect_len)
                else {
                    continue;
                };
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bit));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }

                let nd = depth + 1;
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        manhattan(&input.manhattan, ns.head(), goal),
                        ns.len().saturating_sub(protect_len),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }

            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_transport_to_goal_plan(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        goal: Cell,
        started: &Instant,
    ) -> Option<BeamState> {
        inventory_transport_to_goal_plan_with_limits(
            st,
            input,
            protect_colors,
            protect_len,
            goal,
            started,
            INVENTORY_TRANSPORT_DEPTH_LIMIT,
            INVENTORY_TRANSPORT_NODE_LIMIT,
            INVENTORY_TRANSPORT_EAT_LIMIT,
            INVENTORY_TRANSPORT_BITE_LIMIT,
        )
    }

    #[inline]
    fn inventory_transport_step_state(
        st: &State,
        dir: usize,
        protect_colors: &[u8],
        protect_len: usize,
    ) -> Option<(State, u8, bool)> {
        let mut dropped = DroppedBuf::new();
        let (ns, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
        if ns.len() < protect_len || !prefix_ok(&ns, protect_colors, protect_len) {
            return None;
        }
        if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped) {
            return None;
        }
        Some((ns, ate, bite_idx.is_some()))
    }

    #[inline]
    fn inventory_transport_step_state_with_repair_for_entry(
        st: &State,
        input: &Input,
        dir: usize,
        protect_colors: &[u8],
        protect_len: usize,
    ) -> Option<(State, Ops, u8, bool)> {
        let mut dropped = DroppedBuf::new();
        let (ns, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
        if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped) {
            return None;
        }
        if ns.len() >= protect_len && prefix_ok(&ns, protect_colors, protect_len) {
            return Some((ns, vec![dir as Dir], ate, bite_idx.is_some()));
        }
        if bite_idx.is_none() {
            return None;
        }
        let repaired = repair_prefix_after_bite(&ns, input, protect_colors, protect_len, &dropped)?;
        if !exact_prefix(&repaired.state, protect_colors, protect_len) {
            return None;
        }
        let mut seg = Ops::with_capacity(1 + repaired.ops.len());
        seg.push(dir as Dir);
        seg.extend_from_slice(&repaired.ops);
        Some((repaired.state, seg, ate, true))
    }

    fn inventory_transport_finish_from_repair_node(
        nodes: &[Node],
        idx: usize,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<BeamState> {
        let finish =
            inventory_transport_finish_kind(&nodes[idx].state, protect_colors, protect_len, geom)?;
        let mut ops = reconstruct_plan(nodes, idx);
        if let Some(dir) = finish.1 {
            ops.push(dir);
        }
        Some(BeamState {
            state: finish.0,
            ops,
        })
    }

    fn inventory_transport_finish_from_repair_node_with_suffix(
        nodes: &[Node],
        idx: usize,
        suffix: &BeamState,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
    ) -> Option<BeamState> {
        let finish = inventory_transport_finish_kind(&suffix.state, protect_colors, protect_len, geom)?;
        let mut ops = reconstruct_plan(nodes, idx);
        ops.extend_from_slice(&suffix.ops);
        if let Some(dir) = finish.1 {
            ops.push(dir);
        }
        Some(BeamState {
            state: finish.0,
            ops,
        })
    }

    fn inventory_transport_plan_with_repair_limits(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        started: &Instant,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(Node {
            state: st.clone(),
            parent: None,
            move_seg: Ops::new(),
        });
        depths.push(0u16);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            inventory_transport_plan_rank(st, input, protect_len, geom, 0, 0, 0),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u16, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(started)
                || time_left(started) < INVENTORY_RESCUE_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            if let Some(done) = inventory_transport_finish_from_repair_node(
                &nodes,
                idx,
                protect_colors,
                protect_len,
                geom,
            ) {
                return Some(done);
            }

            let cur = nodes[idx].state.clone();
            if let Some(suffix) =
                inventory_transport_oracle_direct_plan(&cur, protect_colors, protect_len, geom)
            {
                if let Some(done) = inventory_transport_finish_from_repair_node_with_suffix(
                    &nodes,
                    idx,
                    &suffix,
                    protect_colors,
                    protect_len,
                    geom,
                ) {
                    return Some(done);
                }
            }
            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }

            let mut cands: Vec<(
                (usize, usize, usize, usize, usize, usize, usize),
                State,
                Ops,
                u8,
                u8,
                usize,
            )> = Vec::with_capacity(4);

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let Some((ns, seg, ate, bit)) = inventory_transport_step_state_with_repair_for_entry(
                    &cur,
                    input,
                    dir,
                    protect_colors,
                    protect_len,
                ) else {
                    continue;
                };
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bit));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + seg.len();
                if nd > depth_limit {
                    continue;
                }
                if inventory_is_entry_cell(geom, ns.head()) {
                    if let Some(finish) =
                        inventory_transport_finish_kind(&ns, protect_colors, protect_len, geom)
                    {
                        let child = nodes.len();
                        nodes.push(Node {
                            state: ns,
                            parent: Some(idx),
                            move_seg: seg,
                        });
                        depths.push(nd as u16);
                        eat_counts.push(next_eat);
                        bite_counts.push(next_bite);
                        let mut ops = reconstruct_plan(&nodes, child);
                        if let Some(dir) = finish.1 {
                            ops.push(dir);
                        }
                        return Some(BeamState {
                            state: finish.0,
                            ops,
                        });
                    }
                    continue;
                }
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u16 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u16, next_eat, next_bite));
                cands.push((
                    inventory_transport_plan_rank(
                        &ns,
                        input,
                        protect_len,
                        geom,
                        next_eat,
                        next_bite,
                        nd,
                    ),
                    ns,
                    seg,
                    next_eat,
                    next_bite,
                    nd,
                ));
            }

            cands.sort_unstable_by_key(|(key, _, _, _, _, _)| *key);
            for (key, ns, seg, next_eat, next_bite, nd) in cands {
                let child = nodes.len();
                nodes.push(Node {
                    state: ns,
                    parent: Some(idx),
                    move_seg: seg,
                });
                depths.push(nd as u16);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_transport_plan_with_repair(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        started: &Instant,
    ) -> Option<BeamState> {
        inventory_transport_plan_with_repair_limits(
            st,
            input,
            protect_colors,
            protect_len,
            geom,
            started,
            INVENTORY_ENTRY_REPAIR_DEPTH_LIMIT,
            INVENTORY_ENTRY_REPAIR_NODE_LIMIT,
            INVENTORY_ENTRY_REPAIR_EAT_LIMIT,
            INVENTORY_ENTRY_REPAIR_BITE_LIMIT,
        )
    }

    fn transport_to_cell_preserving_prefix(
        bs: &BeamState,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        goal: Cell,
        started: &Instant,
    ) -> Option<BeamState> {
        let mut st = bs.state.clone();
        let mut ops = bs.ops.clone();
        let mut seen = FxHashMap::<VisitKey, usize>::default();
        let mut guard = 0usize;

        while st.head() != goal {
            if time_over(started) || time_left(started) < INVENTORY_MIN_LEFT_SEC {
                return None;
            }
            guard += 1;
            if guard > st.n * st.n * 80 {
                break;
            }

            let key = make_visit_key(&st, goal, 0);
            let cnt = seen.entry(key).or_insert(0);
            *cnt += 1;
            if *cnt > VISIT_REPEAT_LIMIT * 2 {
                break;
            }

            let Some(dir) =
                choose_inventory_transport_dir(&st, goal, protect_colors, protect_len, input)
            else {
                break;
            };
            let Some((ns, _, _)) =
                inventory_transport_step_state(&st, dir, protect_colors, protect_len)
            else {
                break;
            };

            st = ns;
            ops.push(dir as Dir);
            if ops.len() > MAX_TURNS {
                return None;
            }
        }

        if st.head() == goal {
            return Some(BeamState { state: st, ops });
        }

        let suffix =
            inventory_transport_to_goal_plan(&st, input, protect_colors, protect_len, goal, started)
                .or_else(|| {
                    inventory_transport_to_goal_plan_with_limits(
                        &st,
                        input,
                        protect_colors,
                        protect_len,
                        goal,
                        started,
                        INVENTORY_TRANSPORT_HARD_DEPTH_LIMIT,
                        INVENTORY_TRANSPORT_HARD_NODE_LIMIT,
                        INVENTORY_TRANSPORT_HARD_EAT_LIMIT,
                        INVENTORY_TRANSPORT_HARD_BITE_LIMIT,
                    )
                })?;
        ops.extend_from_slice(&suffix.ops);
        if ops.len() > MAX_TURNS {
            return None;
        }
        Some(BeamState {
            state: suffix.state,
            ops,
        })
    }

    fn transport_to_entry_from_right(
        bs: &BeamState,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        seg_idx: usize,
        started: &Instant,
    ) -> Option<BeamState> {
        let geom = inventory_entry_geometry(input.n, seg_idx)?;
        if let Some((state, last_dir)) =
            inventory_transport_finish_kind(&bs.state, protect_colors, protect_len, geom)
        {
            let mut ops = bs.ops.clone();
            if let Some(dir) = last_dir {
                ops.push(dir);
            }
            if ops.len() <= MAX_TURNS {
                return Some(BeamState { state, ops });
            }
            return None;
        }

        // Lightweight oracle: assume suffix cells release over time and try a direct cell-BFS path first.
        if let Some(plan) =
            inventory_transport_oracle_direct_plan(&bs.state, protect_colors, protect_len, geom)
        {
            if let Some((state, last_dir)) =
                inventory_transport_finish_kind(&plan.state, protect_colors, protect_len, geom)
            {
                let mut ops = bs.ops.clone();
                ops.extend_from_slice(&plan.ops);
                if let Some(dir) = last_dir {
                    ops.push(dir);
                }
                if ops.len() <= MAX_TURNS {
                    return Some(BeamState { state, ops });
                }
                return None;
            }
        }

        if let Some(plan) = inventory_transport_plan_with_repair(
            &bs.state,
            input,
            protect_colors,
            protect_len,
            geom,
            started,
        ) {
            return append_incremental_beam(bs, plan);
        }

        let plan =
            inventory_transport_plan(&bs.state, input, protect_colors, protect_len, geom, started)?;
        append_incremental_beam(bs, plan)
    }

    fn place_inventory_segment(
        bs: &BeamState,
        seg_idx: usize,
        n: usize,
        protect_colors: &[u8],
        protect_len: usize,
    ) -> Option<BeamState> {
        let geom = inventory_entry_geometry(n, seg_idx)?;
        let (st, seg_ops) =
            inventory_place_from_entry(&bs.state, protect_colors, protect_len, geom, n)?;
        let mut ops = bs.ops.clone();
        ops.extend_from_slice(&seg_ops);
        if ops.len() > MAX_TURNS {
            return None;
        }
        Some(BeamState { state: st, ops })
    }

    // ===== Inventory Harvest =====

    fn harvest_inventory_impl(
        bs: &BeamState,
        input: &Input,
        stock_cnt: usize,
    ) -> Option<BeamState> {
        let mut st = bs.state.clone();
        let mut ops = bs.ops.clone();

        for seg_idx in (0..stock_cnt).rev() {
            if seg_idx + 1 != stock_cnt {
                if !is_legal_dir(&st, 2) {
                    return None;
                }
                let (ns, _, bite_idx) = step(&st, 2);
                if bite_idx.is_some() {
                    return None;
                }
                st = ns;
                ops.push(2);
            }

            let up_steps = input.n - 3;
            for _ in 0..up_steps {
                if !is_legal_dir(&st, 0) {
                    return None;
                }
                let (ns, _, bite_idx) = step(&st, 0);
                if bite_idx.is_some() {
                    return None;
                }
                st = ns;
                ops.push(0);
            }

            if !is_legal_dir(&st, 2) {
                return None;
            }
            let (ns, _, bite_idx) = step(&st, 2);
            if bite_idx.is_some() {
                return None;
            }
            st = ns;
            ops.push(2);

            for _ in 0..input.n - 3 {
                if !is_legal_dir(&st, 1) {
                    return None;
                }
                let (ns, _, bite_idx) = step(&st, 1);
                if bite_idx.is_some() {
                    return None;
                }
                st = ns;
                ops.push(1);
            }

            if ops.len() > MAX_TURNS {
                return None;
            }
        }

        Some(BeamState { state: st, ops })
    }

    fn harvest_inventory(bs: &BeamState, input: &Input, stock_cnt: usize) -> Option<BeamState> {
        harvest_inventory_impl(bs, input, stock_cnt)
    }

    #[inline]
    fn inventory_harvest_goal_cell(n: usize, stock_cnt: usize) -> Option<Cell> {
        (stock_cnt > 0).then(|| cell_of(n - 3, 2 * stock_cnt - 1, n))
    }

    fn inventory_finish_harvest_from_state(
        st: &State,
        input: &Input,
        stock_cnt: usize,
    ) -> Option<BeamState> {
        let _clear = push_movement_constraint(movement_constraint_with_min_col(0));
        let start = BeamState {
            state: st.clone(),
            ops: Ops::new(),
        };
        let out = harvest_inventory_impl(&start, input, stock_cnt)?;
        exact_prefix(&out.state, &input.d[..input.m], input.m).then_some(out)
    }

    fn inventory_transport_step_state_with_repair(
        st: &State,
        input: &Input,
        dir: usize,
        protect_colors: &[u8],
        protect_len: usize,
    ) -> Option<(State, Ops, u8, bool)> {
        let mut dropped = DroppedBuf::new();
        let (ns, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
        if bite_idx.is_some() && !dropped_respects_active_constraint(st.n, &dropped) {
            return None;
        }
        if ns.len() >= protect_len && prefix_ok(&ns, protect_colors, protect_len) {
            return Some((ns, vec![dir as Dir], ate, bite_idx.is_some()));
        }
        if bite_idx.is_none() {
            return None;
        }
        let repaired = repair_prefix_after_bite(&ns, input, protect_colors, protect_len, &dropped)?;
        if !exact_prefix(&repaired.state, protect_colors, protect_len) {
            return None;
        }
        let mut seg = Ops::with_capacity(1 + repaired.ops.len());
        seg.push(dir as Dir);
        seg.extend_from_slice(&repaired.ops);
        Some((repaired.state, seg, ate, true))
    }

    #[inline]
    fn inventory_harvest_transport_rank(
        st: &State,
        input: &Input,
        stock_cnt: usize,
        protect_len: usize,
        eat_count: u8,
        bite_count: u8,
        depth: usize,
    ) -> (usize, usize, usize, usize, usize, usize, usize) {
        let goal_dist = inventory_harvest_goal_cell(input.n, stock_cnt)
            .map(|goal| manhattan(&input.manhattan, st.head(), goal))
            .unwrap_or(0);
        let (hr, hc) = rc_of(st.head(), st.n);
        let bottom_dist = hr.abs_diff(input.n - 3);
        let right_col = 2 * stock_cnt - 1;
        let col_dist = hc.abs_diff(right_col);
        (
            goal_dist,
            bottom_dist + col_dist,
            st.len().saturating_sub(protect_len),
            bite_count as usize,
            eat_count as usize,
            4usize.saturating_sub(legal_dir_count(st)),
            depth,
        )
    }

    fn inventory_transport_to_harvestable_plan_with_limits(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        stock_cnt: usize,
        started: &Instant,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(Node {
            state: st.clone(),
            parent: None,
            move_seg: Ops::new(),
        });
        depths.push(0u16);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            inventory_harvest_transport_rank(st, input, stock_cnt, protect_len, 0, 0, 0),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u16, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(started)
                || time_left(started) < INVENTORY_RESCUE_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let cur = nodes[idx].state.clone();
            if inventory_finish_harvest_from_state(&cur, input, stock_cnt).is_some() {
                return Some(BeamState {
                    state: cur,
                    ops: reconstruct_plan(&nodes, idx),
                });
            }

            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }

            let mut cands: Vec<(
                (usize, usize, usize, usize, usize, usize, usize),
                State,
                Ops,
                u8,
                u8,
                usize,
            )> = Vec::with_capacity(4);

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let Some((ns, seg, ate, bit)) = inventory_transport_step_state_with_repair(
                    &cur,
                    input,
                    dir,
                    protect_colors,
                    protect_len,
                ) else {
                    continue;
                };
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bit));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + seg.len();
                if nd > depth_limit {
                    continue;
                }
                if seen.get(&ns).is_some_and(|&(best_d, best_e, best_b)| {
                    best_d <= nd as u16 && best_e <= next_eat && best_b <= next_bite
                }) {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u16, next_eat, next_bite));
                cands.push((
                    inventory_harvest_transport_rank(
                        &ns,
                        input,
                        stock_cnt,
                        protect_len,
                        next_eat,
                        next_bite,
                        nd,
                    ),
                    ns,
                    seg,
                    next_eat,
                    next_bite,
                    nd,
                ));
            }

            cands.sort_unstable_by_key(|(key, _, _, _, _, _)| *key);
            for (key, ns, seg, next_eat, next_bite, nd) in cands {
                let child = nodes.len();
                nodes.push(Node {
                    state: ns,
                    parent: Some(idx),
                    move_seg: seg,
                });
                depths.push(nd as u16);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn inventory_transport_to_harvestable_plan(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        stock_cnt: usize,
        started: &Instant,
    ) -> Option<BeamState> {
        inventory_transport_to_harvestable_plan_with_limits(
            st,
            input,
            protect_colors,
            protect_len,
            stock_cnt,
            started,
            INVENTORY_TRANSPORT_HARD_DEPTH_LIMIT * 2,
            INVENTORY_TRANSPORT_HARD_NODE_LIMIT,
            INVENTORY_TRANSPORT_HARD_EAT_LIMIT + 4,
            INVENTORY_TRANSPORT_HARD_BITE_LIMIT + 4,
        )
    }

    fn inventory_transport_harvest_entry(
        bs: &BeamState,
        input: &Input,
        tail_colors: &[u8],
        stock_cnt: usize,
        started: &Instant,
    ) -> Option<BeamState> {
        if stock_cnt == 0 {
            return Some(bs.clone());
        }
        let protect_len = tail_colors.len();
        let goal = inventory_harvest_goal_cell(input.n, stock_cnt)?;

        {
            let _constraint =
                push_movement_constraint(movement_constraint_for_harvest_entry(stock_cnt, input.n));
            if let Some(next_bs) =
                transport_to_cell_preserving_prefix(bs, input, tail_colors, protect_len, goal, started)
            {
                if inventory_finish_harvest_from_state(&next_bs.state, input, stock_cnt).is_some() {
                    return Some(next_bs);
                }
            }
            if let Some(plan) = inventory_transport_to_harvestable_plan(
                &bs.state,
                input,
                tail_colors,
                protect_len,
                stock_cnt,
                started,
            ) {
                if let Some(next_bs) = append_incremental_beam(bs, plan) {
                    return Some(next_bs);
                }
            }
        }

        let min_allowed_col = 2 * stock_cnt - 2;
        let _relaxed = push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));
        let plan = inventory_transport_to_harvestable_plan(
            &bs.state,
            input,
            tail_colors,
            protect_len,
            stock_cnt,
            started,
        )?;
        append_incremental_beam(bs, plan)
    }

    // ===== Inventory Orchestration =====

    #[inline(always)]
    fn inventory_run_segment_phase(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        seg_idx: usize,
    ) -> Option<BeamState> {
        let target_colors = inventory_segment_target(ctx.input, seg_idx, ctx.seg_len);
        let target = InventoryTarget {
            colors: &target_colors,
            protect_len: target_colors.len(),
        };
        let _goal_hash = push_goal_prefix_hash(target.colors);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * seg_idx));

        let phases_left = ctx.stock_cnt - seg_idx + 1;
        let split_built = if base.state.len() == 5 && seg_idx > 0 {
            if let Some(next_bs) =
                inventory_try_direct_build_from_current(ctx, base, &target, phases_left)
            {
                next_bs
            } else {
                let prepared = match inventory_prepare_build_start(
                    ctx,
                    base,
                    target.colors,
                    phases_left,
                    2 * seg_idx,
                ) {
                    Some(next_bs) => next_bs,
                    None => return None,
                };

                if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                    prepared
                } else {
                    match inventory_build_target_exact(ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => next_bs,
                        _ => match inventory_build_target_exact_legacy_parked(
                            ctx,
                            base,
                            &target,
                            phases_left,
                        ) {
                            InventoryBuildOutcome::Built(next_bs) => next_bs,
                            InventoryBuildOutcome::GrowFailed => {
                                match inventory_try_transportable_build_from_parked(
                                    ctx,
                                    base,
                                    &target,
                                    phases_left,
                                    seg_idx,
                                ) {
                                    Some(next_bs) => next_bs,
                                    None => return None,
                                }
                            }
                            InventoryBuildOutcome::ExactFailed => {
                                match inventory_try_transportable_build_from_parked(
                                    ctx,
                                    base,
                                    &target,
                                    phases_left,
                                    seg_idx,
                                ) {
                                    Some(next_bs) => next_bs,
                                    None => return None,
                                }
                            }
                        },
                    }
                }
            }
        } else {
            let prepared =
                match inventory_prepare_build_start(ctx, base, target.colors, phases_left, 2 * seg_idx)
                {
                    Some(next_bs) => next_bs,
                    None => return None,
                };

            if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                prepared
            } else {
                match inventory_build_target_exact(ctx, &prepared, &target, phases_left) {
                    InventoryBuildOutcome::Built(next_bs) => next_bs,
                    _ => {
                        if base.state.len() == 5 {
                            match inventory_build_target_exact_legacy_parked(
                                ctx,
                                base,
                                &target,
                                phases_left,
                            ) {
                                InventoryBuildOutcome::Built(next_bs) => next_bs,
                                InventoryBuildOutcome::GrowFailed => {
                                    match inventory_try_transportable_build_from_parked(
                                        ctx,
                                        base,
                                        &target,
                                        phases_left,
                                        seg_idx,
                                    ) {
                                        Some(next_bs) => next_bs,
                                        None => return None,
                                    }
                                }
                                InventoryBuildOutcome::ExactFailed => {
                                    match inventory_try_transportable_build_from_parked(
                                        ctx,
                                        base,
                                        &target,
                                        phases_left,
                                        seg_idx,
                                    ) {
                                        Some(next_bs) => next_bs,
                                        None => return None,
                                    }
                                }
                            }
                        } else {
                            return None;
                        }
                    }
                }
            }
        };
        let mut bs = split_built;

        bs = match transport_to_entry_from_right(
            &bs,
            ctx.input,
            target.colors,
            target.protect_len,
            seg_idx,
            &ctx.timer.start,
        ) {
            Some(next_bs) => next_bs,
            None => {
                if let Some(alt_built) = inventory_try_transportable_build_from_parked(
                    ctx,
                    base,
                    &target,
                    phases_left,
                    seg_idx,
                ) {
                    if let Some(next_bs) = transport_to_entry_from_right(
                        &alt_built,
                        ctx.input,
                        target.colors,
                        target.protect_len,
                        seg_idx,
                        &ctx.timer.start,
                    ) {
                        next_bs
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
        };

        bs = match place_inventory_segment(&bs, seg_idx, ctx.input.n, target.colors, target.protect_len)
        {
            Some(next_bs) => next_bs,
            None => return None,
        };
        Some(bs)
    }

    #[inline(always)]
    fn inventory_build_tail_exact(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
    ) -> Option<(BeamState, Vec<u8>)> {
        let tail_colors = inventory_tail_target(ctx.input, ctx.stock_cnt, ctx.seg_len);
        let target = InventoryTarget {
            colors: &tail_colors,
            protect_len: tail_colors.len(),
        };
        let _goal_hash = push_goal_prefix_hash(target.colors);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * ctx.stock_cnt));

        let prepared =
            match inventory_prepare_build_start(ctx, base, target.colors, 1, 2 * ctx.stock_cnt) {
                Some(next_bs) => next_bs,
                None => return None,
            };

        let next_bs = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
            prepared
        } else {
            match inventory_build_target_exact(ctx, &prepared, &target, 1) {
                InventoryBuildOutcome::Built(next_bs) => next_bs,
                _ => {
                    if base.state.len() == 5 {
                        match inventory_build_target_exact_legacy_parked(ctx, base, &target, 1) {
                            InventoryBuildOutcome::Built(next_bs) => next_bs,
                            InventoryBuildOutcome::GrowFailed => return None,
                            InventoryBuildOutcome::ExactFailed => return None,
                        }
                    } else {
                        return None;
                    }
                }
            }
        };

        Some((next_bs, tail_colors))
    }

    #[inline(always)]
    fn inventory_finish_harvest(
        ctx: &InventoryCtx<'_>,
        base: &BeamState,
        tail_colors: &[u8],
    ) -> Option<BeamState> {
        let _goal_hash = push_goal_prefix_hash(tail_colors);
        let mut bs = base.clone();

        if ctx.stock_cnt > 0 {
            bs = match inventory_transport_harvest_entry(
                &bs,
                ctx.input,
                tail_colors,
                ctx.stock_cnt,
                &ctx.timer.start,
            ) {
                Some(next_bs) => next_bs,
                None => return None,
            };
        }

        let _constraint = push_movement_constraint(movement_constraint_with_min_col(0));
        if ctx.stock_cnt == 0 {
            return Some(bs);
        }

        let next_bs = match harvest_inventory(&bs, ctx.input, ctx.stock_cnt) {
            Some(next_bs) => next_bs,
            None => return None,
        };
        Some(next_bs)
    }

    fn solve_inventory_stock(input: &Input, timer: &TimeKeeper) -> BeamState {
        let seg_len = 2 * (input.n - 2);
        debug_assert!(seg_len > 0);
        let stock_cnt = (input.m - 5) / seg_len;
        debug_assert!(2 * stock_cnt <= input.n);
        let ctx = InventoryCtx {
            input,
            timer,
            seg_len,
            stock_cnt,
        };

        let mut bs = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };

        for seg_idx in 0..ctx.stock_cnt {
            let Some(next_bs) = inventory_run_segment_phase(&ctx, &bs, seg_idx) else {
                return bs;
            };
            bs = next_bs;
        }

        let Some((next_bs, tail_colors)) = inventory_build_tail_exact(&ctx, &bs) else {
            return bs;
        };
        bs = next_bs;

        let Some(next_bs) = inventory_finish_harvest(&ctx, &bs, &tail_colors) else {
            return bs;
        };
        next_bs
    }

    pub fn solve_from_common(common: &crate::CommonInput, limit_sec: f64) -> (Vec<u8>, bool) {
        if limit_sec <= 0.0 {
            return (Vec::new(), false);
        }
        let input = from_common_input(common);
        let timer = TimeKeeper::new(limit_sec.max(1e-6), 8);
        let _global_budget = push_search_budget(&timer, limit_sec.max(0.0), 0.0);
        let mut best = solve_inventory_stock(&input, &timer);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        let finished = timer.exact_remaining_sec() > 1e-9;
        (best.ops, finished)
    }

    fn from_common_input(common: &crate::CommonInput) -> Input {
        let mut d = [0_u8; MAX_LEN];
        for (dst, &src) in d.iter_mut().zip(common.goal_colors.iter()) {
            *dst = src;
        }
        let mut food = [0_u8; MAX_CELLS];
        for (dst, &src) in food.iter_mut().zip(common.food.iter()) {
            *dst = src;
        }
        Input {
            n: common.n,
            m: common.m,
            d,
            food,
            manhattan: ManhattanTable::new(common.n),
        }
    }
}

fn select_ensemble_candidate(input: &CommonInput) -> Candidate {
    let ensemble_start = Instant::now();
    let density3 = classify_density3(input);
    let three_way = matches!(input.n, 8 | 9) || (density3 == Density3::L && input.n <= 12);

    if three_way {
        let mut best = run_v012(input);

        let v139_limit = remaining_limit_sec(&ensemble_start);
        if v139_limit > 0.0 {
            let v139 = run_v139(input, v139_limit);
            if v139.finished {
                update_best(&mut best, v139);
            }
        }

        if ensemble_start.elapsed().as_secs_f64() <= SKIP_V149_THRESHOLD_SEC {
            let v149_limit = remaining_limit_sec(&ensemble_start);
            if v149_limit > 0.0 {
                let v149 = run_v149(input, v149_limit);
                if v149.finished {
                    update_best(&mut best, v149);
                }
            }
        }

        return best;
    }

    if matches!(input.n, 10 | 11) {
        let v139_limit = remaining_limit_sec(&ensemble_start);
        let mut best = run_v139(input, v139_limit);
        if best.elapsed_sec <= SKIP_V149_THRESHOLD_SEC {
            let v149_limit = remaining_limit_sec(&ensemble_start);
            if v149_limit > 0.0 {
                let v149 = run_v149(input, v149_limit);
                if v149.finished {
                    update_best(&mut best, v149);
                }
            }
        }
        return best;
    }

    if input.n >= 12 {
        return run_v149(input, remaining_limit_sec(&ensemble_start));
    }

    run_v139(input, remaining_limit_sec(&ensemble_start))
}

fn solve_ensemble(input: &CommonInput) -> Vec<u8> {
    let best = select_ensemble_candidate(input);
    let fallback = run_v001_sample_fallback(input);
    if fallback.abs_score < best.abs_score {
        fallback.ops
    } else {
        best.ops
    }
}

fn main() {
    let input = read_common_input();
    let ans = solve_ensemble(&input);
    let mut out = String::new();
    for dir in ans {
        out.push(DIR_CHARS[dir as usize]);
        out.push('\n');
    }
    print!("{out}");
}
