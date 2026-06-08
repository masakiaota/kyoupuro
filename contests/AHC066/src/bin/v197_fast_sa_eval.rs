// v197_fast_sa_eval.rs
use proconio::{input, marker::Chars};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::{Ordering, max, min};
use std::collections::BTreeMap;
#[cfg(feature = "local")]
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Instant;

const MAX_N: usize = 20;
const MAX_M: usize = 40;
const MAX_CELLS: usize = MAX_N * MAX_N;
const MAX_ORIENTED_STATES: usize = MAX_CELLS * 4;
const INF_DIST: u16 = 30000;
const INF_COST: u32 = 1_000_000_000;
const NONE: u8 = 255;

const OP_F: u8 = 0;
const OP_R: u8 = 1;
const OP_L: u8 = 2;
const OP_S: u8 = 3;
const OP_M: u8 = 4;
const OP_P: u8 = 5;
const OP_REGISTER: u8 = 6;
const OP_DOUBLE: u8 = 7;
const OP_REGISTER_MOM: u8 = 8;
const OP_NONE: u8 = 255;

const Q_BEAM_POOL_SIZE: usize = 256;
const Q_POOL_EXTRA_LIMIT: usize = 110;
const Q_SEARCH_TIME_RATIO: f64 = 0.40;
const SA_EARLY_PRUNE_CHECK_RATIO: f64 = 1.0 / 3.0;
const SA_EARLY_PRUNE_SLACK_PER_ITEM: usize = 3;
const SA_EARLY_PRUNE_SLACK_BASE: usize = 15;
const TWO_STAGE_PROBE_FRACTION: f64 = 0.75;
const TWO_STAGE_PROBE_SLICE_RATIO: f64 = 0.000625;
const TWO_STAGE_FULL_SLICE_RATIO: f64 = 0.0066875;
const TWO_STAGE_FULL_KEEP: usize = 96;
const POWER_RECOMPRESS_RESERVE_RATIO: f64 = 0.075;
const DEADLINE_MARGIN_RATIO: f64 = 0.00375;
const SA_PROBE_STOP_MARGIN_RATIO: f64 = 0.0025;
const POST_RECOMPRESS_MIN_REMAIN_RATIO: f64 = 0.01125;
const POWER_RECOMPRESS_MIN_REMAIN_RATIO: f64 = 0.01875;
const MOM_MIN_TOTAL_GAIN: usize = 2;
const PRIORITY_FRONT_KEEP: usize = 256;
const LONG_FREQ_PROMOTE_AFTER_PRIORITY: usize = 96;
const LONG_FREQ_PROMOTE_KEEP: usize = 16;
const PAIR_ROUTE_PREFIX_KEEP: usize = 700;

#[cfg(feature = "local")]
static SA_EARLY_PRUNE_COUNT: AtomicUsize = AtomicUsize::new(0);

const CAND_KIND_SHORT: u8 = 0;
const CAND_KIND_FREQ: u8 = 1;
const CAND_KIND_PRIORITY: u8 = 2;
const CAND_KIND_PAIR_ROUTE: u8 = 3;
const CAND_KIND_COUNT: usize = 4;
const MAX_MACRO_Q_LEN: usize = 42;
#[cfg(feature = "local")]
const BEST_SOURCE_INIT: u8 = 250;
#[cfg(feature = "local")]
const BEST_SOURCE_RECOMPRESS: u8 = 251;
#[cfg(feature = "local")]
const BEST_SOURCE_FALLBACK: u8 = 252;
#[cfg(feature = "local")]
const BEST_SOURCE_SAFE_FALLBACK: u8 = 253;

#[cfg(feature = "local")]
const DEFAULT_PROGRAM_TIME_LIMIT_SEC: f64 = 1.6;
#[cfg(not(feature = "local"))]
const DEFAULT_PROGRAM_TIME_LIMIT_SEC: f64 = 1.985;

fn program_time_limit_sec() -> f64 {
    #[cfg(feature = "local")]
    {
        if let Ok(s) = std::env::var("AHC066_TIME_LIMIT_SEC") {
            if let Ok(v) = s.parse::<f64>() {
                if v.is_finite() && v > 0.1 {
                    return v;
                }
            }
        }
    }
    DEFAULT_PROGRAM_TIME_LIMIT_SEC
}

fn q_search_time_limit_sec(program_time_limit_sec: f64) -> f64 {
    time_ratio_sec(program_time_limit_sec, Q_SEARCH_TIME_RATIO)
}

#[inline(always)]
fn time_ratio_sec(program_time_limit_sec: f64, ratio: f64) -> f64 {
    program_time_limit_sec * ratio
}

fn sa_early_prune_slack(input: &Input) -> u32 {
    (SA_EARLY_PRUNE_SLACK_PER_ITEM * input.m + SA_EARLY_PRUNE_SLACK_BASE) as u32
}

#[cfg(feature = "local")]
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

struct Timer {
    start: Instant,
}

impl Timer {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[inline(always)]
    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            x: if seed == 0 { 88172645463325252 } else { seed },
        }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() & 0xffff_ffff) as u32
    }

    #[inline(always)]
    fn randint(&mut self, l: usize, r: usize) -> usize {
        l + (self.next_u32() as usize % (r - l + 1))
    }

    #[inline(always)]
    fn uniform01(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / 9007199254740992.0)
    }

    fn shuffle_vec<T>(&mut self, a: &mut [T]) {
        if a.len() <= 1 {
            return;
        }
        for i in (1..a.len()).rev() {
            let j = self.randint(0, i);
            a.swap(i, j);
        }
    }
}

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    m: usize,
    t_limit: usize,
    wall_mask: [u8; MAX_CELLS],
    init_ball_at: [u8; MAX_CELLS],
    basket_at: [u8; MAX_CELLS],
    ball_pos: [u16; MAX_M],
    basket_pos: [u16; MAX_M],
}

impl Input {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            t_limit: usize,
            wall_v_raw: [Chars; n],
            wall_h_raw: [Chars; n - 1],
            bcde: [(usize, usize, usize, usize); m],
        }

        let cell = |i: usize, j: usize| -> usize { i * n + j };

        let mut wall_mask = [0u8; MAX_CELLS];
        for i in 0..n {
            for j in 0..n - 1 {
                if wall_v_raw[i][j] == '1' {
                    let left = cell(i, j);
                    let right = cell(i, j + 1);
                    wall_mask[left] |= 1 << 1;
                    wall_mask[right] |= 1 << 3;
                }
            }
        }
        for i in 0..n - 1 {
            for j in 0..n {
                if wall_h_raw[i][j] == '1' {
                    let up = cell(i, j);
                    let down = cell(i + 1, j);
                    wall_mask[up] |= 1 << 2;
                    wall_mask[down] |= 1 << 0;
                }
            }
        }

        let mut init_ball_at = [NONE; MAX_CELLS];
        let mut basket_at = [NONE; MAX_CELLS];
        let mut ball_pos = [0u16; MAX_M];
        let mut basket_pos = [0u16; MAX_M];
        for (k, &(b, c, d, e)) in bcde.iter().enumerate() {
            let ball_cell = cell(b, c);
            let basket_cell = cell(d, e);
            init_ball_at[ball_cell] = k as u8;
            basket_at[basket_cell] = k as u8;
            ball_pos[k] = ball_cell as u16;
            basket_pos[k] = basket_cell as u16;
        }

        Self {
            n,
            m,
            t_limit,
            wall_mask,
            init_ball_at,
            basket_at,
            ball_pos,
            basket_pos,
        }
    }
}

#[derive(Debug, Clone)]
struct Grid {
    cell_count: usize,
    move_mask: [u8; MAX_CELLS],
    next_cell: [[u16; 4]; MAX_CELLS],
}

impl Grid {
    fn new(input: &Input) -> Self {
        let n = input.n;
        let cell_count = n * n;
        let dir_delta = [-(n as i16), 1, n as i16, -1];

        let mut move_mask = [0u8; MAX_CELLS];
        let mut next_cell = [[0u16; 4]; MAX_CELLS];

        for cell in 0..cell_count {
            let i = cell / n;
            let j = cell % n;
            let mut edge = 0u8;
            if i > 0 {
                edge |= 1 << 0;
            }
            if j + 1 < n {
                edge |= 1 << 1;
            }
            if i + 1 < n {
                edge |= 1 << 2;
            }
            if j > 0 {
                edge |= 1 << 3;
            }
            move_mask[cell] = edge & !input.wall_mask[cell];
            for dir in 0..4 {
                next_cell[cell][dir] = if move_mask[cell] & (1 << dir) != 0 {
                    (cell as i16 + dir_delta[dir]) as u16
                } else {
                    cell as u16
                };
            }
        }

        Self {
            cell_count,
            move_mask,
            next_cell,
        }
    }

    #[inline(always)]
    fn can_move(&self, cell: usize, dir: usize) -> bool {
        unsafe { *self.move_mask.get_unchecked(cell) & (1 << dir) != 0 }
    }

    #[inline(always)]
    fn next(&self, cell: usize, dir: usize) -> usize {
        unsafe { *self.next_cell.get_unchecked(cell).get_unchecked(dir) as usize }
    }
}

#[derive(Debug, Clone)]
struct State {
    pos: u16,
    dir: u8,
    held: u8,
    matched: u8,
    cell_ball: [u8; MAX_CELLS],
    basic_count: usize,
    recording: bool,
    last_macro: Vec<u8>,
    cur_macro: Vec<u8>,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut matched = 0u8;
        for k in 0..input.m {
            let cell = input.basket_pos[k] as usize;
            if input.init_ball_at[cell] == k as u8 {
                matched += 1;
            }
        }

        Self {
            pos: 0,
            dir: 1,
            held: NONE,
            matched,
            cell_ball: input.init_ball_at,
            basic_count: 0,
            recording: false,
            last_macro: Vec::new(),
            cur_macro: Vec::new(),
        }
    }

    fn press_button(&mut self, input: &Input, grid: &Grid, button: u8) {
        if self.basic_count >= input.t_limit {
            return;
        }
        match button {
            OP_F | OP_R | OP_L | OP_S => {
                self.execute_basic(input, grid, button);
            }
            OP_M => self.toggle_recording(),
            OP_P => self.replay_last_macro(input, grid),
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn toggle_recording(&mut self) {
        if self.recording {
            std::mem::swap(&mut self.last_macro, &mut self.cur_macro);
            self.cur_macro.clear();
            self.recording = false;
        } else {
            self.cur_macro.clear();
            self.recording = true;
        }
    }

    fn replay_last_macro(&mut self, input: &Input, grid: &Grid) {
        let mut idx = 0;
        while idx < self.last_macro.len() {
            let op = self.last_macro[idx];
            if !self.execute_basic(input, grid, op) {
                break;
            }
            idx += 1;
        }
    }

    #[inline(always)]
    fn execute_basic(&mut self, input: &Input, grid: &Grid, op: u8) -> bool {
        if self.basic_count >= input.t_limit {
            return false;
        }
        self.apply_basic(input, grid, op);
        self.basic_count += 1;
        if self.recording {
            self.cur_macro.push(op);
        }
        true
    }

    #[inline(always)]
    fn apply_basic(&mut self, input: &Input, grid: &Grid, op: u8) {
        match op {
            OP_F => {
                self.pos = grid.next(self.pos as usize, self.dir as usize) as u16;
            }
            OP_R => {
                self.dir = (self.dir + 1) & 3;
            }
            OP_L => {
                self.dir = (self.dir + 3) & 3;
            }
            OP_S => self.apply_swap(input),
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn apply_swap(&mut self, input: &Input) {
        let cell = self.pos as usize;
        let old = self.cell_ball[cell];
        let new = self.held;
        if is_correct_at(input, cell, old) {
            self.matched -= 1;
        }
        self.cell_ball[cell] = new;
        self.held = old;
        if is_correct_at(input, cell, new) {
            self.matched += 1;
        }
    }
}

#[inline(always)]
fn is_correct_at(input: &Input, cell: usize, ball: u8) -> bool {
    ball != NONE && input.basket_at[cell] == ball
}

type DistMatrix = Vec<[u16; MAX_ORIENTED_STATES]>;

#[inline(always)]
fn oriented_state(cell: usize, dir: u8) -> usize {
    (cell << 2) | dir as usize
}

fn all_pairs_dist(grid: &Grid) -> DistMatrix {
    let mut dist = vec![[INF_DIST; MAX_ORIENTED_STATES]; grid.cell_count];
    for target in 0..grid.cell_count {
        let mut q = [0usize; MAX_ORIENTED_STATES];
        let mut head = 0usize;
        let mut tail = 0usize;
        for dir in 0..4u8 {
            let state = oriented_state(target, dir);
            dist[target][state] = 0;
            q[tail] = state;
            tail += 1;
        }
        while head < tail {
            let state = q[head];
            head += 1;
            let cell = state >> 2;
            let dir = (state & 3) as u8;
            let nd = dist[target][state] + 1;

            let prev_r = oriented_state(cell, (dir + 3) & 3);
            if dist[target][prev_r] == INF_DIST {
                dist[target][prev_r] = nd;
                q[tail] = prev_r;
                tail += 1;
            }

            let prev_l = oriented_state(cell, (dir + 1) & 3);
            if dist[target][prev_l] == INF_DIST {
                dist[target][prev_l] = nd;
                q[tail] = prev_l;
                tail += 1;
            }

            let back_dir = (dir + 2) & 3;
            if grid.can_move(cell, back_dir as usize) {
                let prev_cell = grid.next(cell, back_dir as usize);
                let prev_f = oriented_state(prev_cell, dir);
                if dist[target][prev_f] == INF_DIST {
                    dist[target][prev_f] = nd;
                    q[tail] = prev_f;
                    tail += 1;
                }
            }
        }
    }
    dist
}

fn move_to(
    grid: &Grid,
    dist: &DistMatrix,
    pos: &mut usize,
    dir: &mut u8,
    target: usize,
    ops: &mut Vec<u8>,
) {
    while *pos != target {
        let cur_state = oriented_state(*pos, *dir);
        let cd = dist[target][cur_state];
        if cd == INF_DIST {
            return;
        }

        let fpos = grid.next(*pos, *dir as usize);
        let fstate = oriented_state(fpos, *dir);
        if fpos != *pos && dist[target][fstate] + 1 == cd {
            ops.push(OP_F);
            *pos = fpos;
            continue;
        }

        let rdir = (*dir + 1) & 3;
        let rstate = oriented_state(*pos, rdir);
        if dist[target][rstate] + 1 == cd {
            ops.push(OP_R);
            *dir = rdir;
            continue;
        }

        let ldir = (*dir + 3) & 3;
        let lstate = oriented_state(*pos, ldir);
        if dist[target][lstate] + 1 == cd {
            ops.push(OP_L);
            *dir = ldir;
            continue;
        }

        return;
    }
}

fn trace_move_end_dir(grid: &Grid, dist: &DistMatrix, pos: usize, dir: u8, target: usize) -> u8 {
    let mut pos = pos;
    let mut dir = dir;
    while pos != target {
        let cur_state = oriented_state(pos, dir);
        let cd = dist[target][cur_state];
        if cd == INF_DIST {
            break;
        }

        let fpos = grid.next(pos, dir as usize);
        let fstate = oriented_state(fpos, dir);
        if fpos != pos && dist[target][fstate] + 1 == cd {
            pos = fpos;
            continue;
        }

        let rdir = (dir + 1) & 3;
        let rstate = oriented_state(pos, rdir);
        if dist[target][rstate] + 1 == cd {
            dir = rdir;
            continue;
        }

        let ldir = (dir + 3) & 3;
        let lstate = oriented_state(pos, ldir);
        if dist[target][lstate] + 1 == cd {
            dir = ldir;
            continue;
        }

        break;
    }
    dir
}

/// マクロを一切使わない素朴貪欲。 「今の位置から取りに行って、そのまま対応かごへ運ぶ距離」が一番短いものを貪欲に選ぶ。
fn build_basic_ops(input: &Input, grid: &Grid, dist: &DistMatrix) -> Vec<u8> {
    let mut ops = Vec::with_capacity(4096);
    let mut done = [false; MAX_M];
    let mut done_count = 0usize;
    let mut pos = 0usize;
    let mut dir = 1u8;
    while done_count < input.m {
        let mut best_k = 0usize;
        let mut best_score = INF_COST;
        let mut best_raw_cost = INF_COST;
        let mut best_to_ball = INF_DIST;
        for k in 0..input.m {
            if done[k] {
                continue;
            }
            let b = input.ball_pos[k] as usize;
            let t = input.basket_pos[k] as usize;
            let tb = dist[b][oriented_state(pos, dir)];
            if tb == INF_DIST {
                continue;
            }
            let ball_dir = trace_move_end_dir(grid, dist, pos, dir, b);
            let bt = dist[t][oriented_state(b, ball_dir)];
            if bt == INF_DIST {
                continue;
            }
            let basket_dir = trace_move_end_dir(grid, dist, b, ball_dir, t);
            let mut next_min = 0u16;
            if done_count + 1 < input.m {
                next_min = INF_DIST;
                for j in 0..input.m {
                    if done[j] || j == k {
                        continue;
                    }
                    let nb = input.ball_pos[j] as usize;
                    next_min = min(next_min, dist[nb][oriented_state(t, basket_dir)]);
                }
            }
            let raw_cost = tb as u32 + bt as u32;
            let score = tb as u32 * 80 + bt as u32 * 100 + next_min as u32 * 20;
            if score < best_score
                || (score == best_score
                    && (raw_cost < best_raw_cost || (raw_cost == best_raw_cost && tb < best_to_ball)))
            {
                best_score = score;
                best_raw_cost = raw_cost;
                best_to_ball = tb;
                best_k = k;
            }
        }
        let b = input.ball_pos[best_k] as usize;
        let t = input.basket_pos[best_k] as usize;
        move_to(grid, dist, &mut pos, &mut dir, b, &mut ops);
        ops.push(OP_S);
        move_to(grid, dist, &mut pos, &mut dir, t, &mut ops);
        ops.push(OP_S);
        done[best_k] = true;
        done_count += 1;
    }
    ops
}

#[derive(Clone, Default)]
struct Choice {
    typ: u8,
    len: usize,
    future: Vec<usize>,
}

struct RollingHash {
    pref: Vec<u64>,
    pw: Vec<u64>,
}

impl RollingHash {
    const BASE: u64 = 911382323;

    fn new(s: &[u8]) -> Self {
        let n = s.len();
        let mut pref = vec![0u64; n + 1];
        let mut pw = vec![1u64; n + 1];
        for i in 0..n {
            pref[i + 1] = pref[i]
                .wrapping_mul(Self::BASE)
                .wrapping_add(s[i] as u64 + 1);
            pw[i + 1] = pw[i].wrapping_mul(Self::BASE);
        }
        Self { pref, pw }
    }

    #[inline(always)]
    fn get(&self, l: usize, r: usize) -> u64 {
        self.pref[r].wrapping_sub(self.pref[l].wrapping_mul(self.pw[r - l]))
    }
}

#[inline(always)]
fn lower_bound_vec(xs: &[usize], value: usize) -> usize {
    let mut l = 0usize;
    let mut r = xs.len();
    while l < r {
        let m = (l + r) >> 1;
        if xs[m] < value {
            l = m + 1;
        } else {
            r = m;
        }
    }
    l
}

fn build_positions_by_len(
    ops: &[u8],
    hash: &RollingHash,
    max_len: usize,
) -> Vec<FxHashMap<u64, Vec<usize>>> {
    let n = ops.len();
    let mut mp: Vec<FxHashMap<u64, Vec<usize>>> =
        (0..=max_len).map(|_| FxHashMap::default()).collect();
    for len in 2..=min(max_len, n) {
        mp[len].reserve((n - len + 1) * 2);
        for st in 0..=n - len {
            mp[len].entry(hash.get(st, st + len)).or_default().push(st);
        }
    }
    mp
}

fn collect_future_occurrences(
    ops: &[u8],
    hash: &RollingHash,
    pos_by_len: &[FxHashMap<u64, Vec<usize>>],
    start: usize,
    len: usize,
) -> Vec<usize> {
    let key = hash.get(start, start + len);
    let Some(positions) = pos_by_len[len].get(&key) else {
        return Vec::new();
    };
    let first = lower_bound_vec(positions, start + len);
    let mut occ = Vec::with_capacity(positions.len().saturating_sub(first));
    'outer: for &p in &positions[first..] {
        for z in 0..len {
            if ops[p + z] != ops[start + z] {
                continue 'outer;
            }
        }
        occ.push(p);
    }
    occ
}

fn evaluate_macro_candidate_dp(
    start: usize,
    len: usize,
    occ: &[usize],
    dp: &[usize],
) -> Option<(usize, Vec<usize>)> {
    if occ.is_empty() {
        return None;
    }
    let m = occ.len();
    let iinf = usize::MAX / 4;
    let mut choose_after = vec![usize::MAX; m];
    let mut suffix_value = vec![iinf; m + 1];
    let mut suffix_index = vec![usize::MAX; m + 1];
    for idx in (0..m).rev() {
        let cur = occ[idx] + len;
        let next_idx = lower_bound_vec(occ, cur);
        let mut best = dp[cur];
        let mut choose = usize::MAX;
        if next_idx < m {
            let use_cost = suffix_value[next_idx].saturating_sub(cur);
            if use_cost < best {
                best = use_cost;
                choose = suffix_index[next_idx];
            }
        }
        choose_after[idx] = choose;
        let value = occ[idx] + 1 + best;
        if value <= suffix_value[idx + 1] {
            suffix_value[idx] = value;
            suffix_index[idx] = idx;
        } else {
            suffix_value[idx] = suffix_value[idx + 1];
            suffix_index[idx] = suffix_index[idx + 1];
        }
    }

    let start_cur = start + len;
    let mut active_cost = dp[start_cur];
    let mut first_choice = usize::MAX;
    if suffix_value[0] < iinf {
        let use_cost = suffix_value[0] - start_cur;
        if use_cost < active_cost {
            active_cost = use_cost;
            first_choice = suffix_index[0];
        }
    }
    if first_choice == usize::MAX {
        return None;
    }

    let mut future = Vec::new();
    let mut idx = first_choice;
    while idx != usize::MAX {
        future.push(occ[idx]);
        idx = choose_after[idx];
    }
    Some((len + 2 + active_cost, future))
}

/// 展開済みの基本操作列から重複部分列を探し、`M ... M` と `P` で短くする。
///
/// 入力 `ops` は基本操作（`F`, `R`, `L`, `S`）のみの列を想定する。
/// 返り値は `M` と `P` を含む圧縮済み操作列。展開すると元の `ops` と同じ基本操作列になる。
fn compress_with_multiple_macros(ops: &[u8]) -> Vec<u8> {
    let n = ops.len();
    if n < 4 {
        return ops.to_vec();
    }
    let max_len = min(96, n / 2);
    let hash = RollingHash::new(ops);
    let positions_by_len = build_positions_by_len(ops, &hash, max_len);
    let mut dp = vec![0usize; n + 1];
    let mut choice = vec![Choice::default(); n + 1];

    for start in (0..n).rev() {
        let mut best_cost = 1 + dp[start + 1];
        let mut best_choice = Choice::default();
        let max_len_at_start = min(max_len, (n - start) / 2);
        for len in 2..=max_len_at_start {
            let occ = collect_future_occurrences(ops, &hash, &positions_by_len, start, len);
            let Some((cost, future)) = evaluate_macro_candidate_dp(start, len, &occ, &dp) else {
                continue;
            };
            if cost < best_cost {
                best_cost = cost;
                best_choice = Choice {
                    typ: 1,
                    len,
                    future,
                };
            }
        }
        dp[start] = best_cost;
        choice[start] = best_choice;
    }

    let mut compressed = Vec::with_capacity(dp[0]);
    let mut pos = 0usize;
    while pos < n {
        if choice[pos].typ == 0 {
            compressed.push(ops[pos]);
            pos += 1;
        } else {
            let len = choice[pos].len;
            compressed.push(OP_M);
            compressed.extend_from_slice(&ops[pos..pos + len]);
            compressed.push(OP_M);
            let mut cur = pos + len;
            for &np in &choice[pos].future {
                compressed.extend_from_slice(&ops[cur..np]);
                compressed.push(OP_P);
                cur = np + len;
            }
            pos = cur;
        }
    }
    compressed
}

#[inline(always)]
fn starts_with_at(haystack: &[u8], pos: usize, needle: &[u8]) -> bool {
    pos + needle.len() <= haystack.len() && &haystack[pos..pos + needle.len()] == needle
}

#[derive(Clone)]
struct RegisteredMacro {
    ops: Vec<u8>,
    used_p_while_recording: bool,
}

#[derive(Clone, Copy)]
struct PowerPrev {
    prev: u32,
    action: u16,
}

const POWER_RECOMPRESS_REGISTER_BASE: u16 = 256;
const POWER_RECOMPRESS_MAX_CANDIDATES: usize = 384;
const POWER_RECOMPRESS_MAX_EXPANDED: usize = 2600;
const POWER_RECOMPRESS_MAX_MACRO_LEN: usize = 126;
const POWER_RECOMPRESS_FREQ_KEEP: usize = 260;
const POWER_RECOMPRESS_FREQ_MAX_LEN: usize = 72;

fn collect_registered_macros(buttons: &[u8]) -> Vec<RegisteredMacro> {
    let mut out = Vec::new();
    let mut last_macro = Vec::<u8>::new();
    let mut cur_macro = Vec::<u8>::new();
    let mut recording = false;
    let mut cur_used_p = false;
    for &button in buttons {
        if button == OP_F || button == OP_R || button == OP_L || button == OP_S {
            if recording {
                cur_macro.push(button);
            }
        } else if button == OP_M {
            if recording {
                last_macro = cur_macro.clone();
                if !last_macro.is_empty() {
                    out.push(RegisteredMacro {
                        ops: last_macro.clone(),
                        used_p_while_recording: cur_used_p,
                    });
                }
                cur_macro.clear();
                recording = false;
                cur_used_p = false;
            } else {
                cur_macro.clear();
                recording = true;
                cur_used_p = false;
            }
        } else if button == OP_P && recording {
            cur_macro.extend_from_slice(&last_macro);
            cur_used_p = true;
        }
    }
    out
}

fn push_power_candidate(
    out: &mut Vec<Vec<u8>>,
    used: &mut FxHashMap<Vec<u8>, ()>,
    q: Vec<u8>,
    basic_len: usize,
) {
    if q.len() < 2 || q.len() > POWER_RECOMPRESS_MAX_MACRO_LEN || q.len() > basic_len {
        return;
    }
    if q.iter().any(|&op| op > OP_S) {
        return;
    }
    if used.insert(q.clone(), ()).is_none() {
        out.push(q);
    }
}

fn build_power_recompress_candidates(basic: &[u8], seed_buttons: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::<Vec<u8>>::new();
    let mut used = FxHashMap::<Vec<u8>, ()>::default();
    out.push(Vec::new());
    used.insert(Vec::new(), ());
    let registered = collect_registered_macros(seed_buttons);
    for item in &registered {
        push_power_candidate(&mut out, &mut used, item.ops.clone(), basic.len());
        if !item.used_p_while_recording {
            let mut q2 = Vec::with_capacity(item.ops.len() * 2);
            q2.extend_from_slice(&item.ops);
            q2.extend_from_slice(&item.ops);
            push_power_candidate(&mut out, &mut used, q2, basic.len());

            let mut q3 = Vec::with_capacity(item.ops.len() * 3);
            q3.extend_from_slice(&item.ops);
            q3.extend_from_slice(&item.ops);
            q3.extend_from_slice(&item.ops);
            push_power_candidate(&mut out, &mut used, q3, basic.len());
        }
        if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
            break;
        }
    }

    let snapshot: Vec<Vec<u8>> = out.iter().skip(1).cloned().collect();
    for q in snapshot {
        if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
            break;
        }
        let len = q.len();
        if len == 0 || len > basic.len() {
            continue;
        }
        for st in 0..=basic.len() - len {
            if !starts_with_at(basic, st, &q) {
                continue;
            }
            for extra in 1..=3 {
                if st >= extra {
                    push_power_candidate(
                        &mut out,
                        &mut used,
                        basic[st - extra..st + len].to_vec(),
                        basic.len(),
                    );
                    if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
                        break;
                    }
                }
                if st + len + extra <= basic.len() {
                    push_power_candidate(
                        &mut out,
                        &mut used,
                        basic[st..st + len + extra].to_vec(),
                        basic.len(),
                    );
                    if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
                        break;
                    }
                }
            }
            if st > 0 && st + len < basic.len() {
                push_power_candidate(
                    &mut out,
                    &mut used,
                    basic[st - 1..st + len + 1].to_vec(),
                    basic.len(),
                );
            }
            if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
                break;
            }
        }
    }

    let mut freq_items = Vec::<(i64, Vec<u8>)>::new();
    let freq_max_len = POWER_RECOMPRESS_FREQ_MAX_LEN
        .min(POWER_RECOMPRESS_MAX_MACRO_LEN)
        .min(basic.len());
    for len in 2..=freq_max_len {
        let mut counts = FxHashMap::<Vec<u8>, u16>::default();
        counts.reserve(basic.len().saturating_sub(len) + 1);
        for st in 0..=basic.len() - len {
            let q = basic[st..st + len].to_vec();
            let entry = counts.entry(q).or_insert(0);
            if *entry < u16::MAX {
                *entry += 1;
            }
        }
        for (q, count) in counts {
            if count < 2 {
                continue;
            }
            let turns = q.iter().filter(|&&op| op == OP_R || op == OP_L).count() as i64;
            let has_s = q.iter().any(|&op| op == OP_S);
            let mut score = (count as i64) * (len as i64 - 1) * 120 + turns * 350 + len as i64 * 20;
            if has_s {
                score += 1800 + count as i64 * 250;
            }
            if len >= 10 && turns >= 2 {
                score += 1200;
            }
            freq_items.push((score, q));
        }
    }
    freq_items.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.len().cmp(&a.1.len()))
            .then_with(|| a.1.cmp(&b.1))
    });
    for (_, q) in freq_items.into_iter().take(POWER_RECOMPRESS_FREQ_KEEP) {
        push_power_candidate(&mut out, &mut used, q, basic.len());
        if out.len() >= POWER_RECOMPRESS_MAX_CANDIDATES {
            break;
        }
    }
    out.truncate(POWER_RECOMPRESS_MAX_CANDIDATES);
    out
}

fn parse_power_cost(target: &[u8], cur: &[u8]) -> usize {
    let n = target.len();
    let m = cur.len();
    let mut dp = vec![0usize; n + 1];
    for i in (0..n).rev() {
        let mut best = 1 + dp[i + 1];
        if m > 0 && i + m <= n && target[i..i + m] == cur[..] {
            best = best.min(1 + dp[i + m]);
        }
        dp[i] = best;
    }
    dp[0]
}

fn parse_power_inside(target: &[u8], cur: &[u8]) -> Vec<u8> {
    let n = target.len();
    let m = cur.len();
    let mut dp = vec![0usize; n + 1];
    let mut use_p = vec![false; n];
    for i in (0..n).rev() {
        let mut best = 1 + dp[i + 1];
        if m > 0 && i + m <= n && target[i..i + m] == cur[..] {
            let cost = 1 + dp[i + m];
            if cost < best {
                best = cost;
                use_p[i] = true;
            }
        }
        dp[i] = best;
    }
    let mut out = Vec::with_capacity(dp[0]);
    let mut i = 0usize;
    while i < n {
        if use_p[i] {
            out.push(OP_P);
            i += m;
        } else {
            out.push(target[i]);
            i += 1;
        }
    }
    out
}

fn power_recompress_limited(
    basic: &[u8],
    seed_buttons: &[u8],
    best_limit: usize,
    timer: &Timer,
    deadline: f64,
) -> Option<(Vec<u8>, usize)> {
    let n = basic.len();
    if n == 0 || n > POWER_RECOMPRESS_MAX_EXPANDED || timer.elapsed() >= deadline {
        return None;
    }
    let cands = build_power_recompress_candidates(basic, seed_buttons);
    let cand_count = cands.len();
    if cand_count <= 1 || best_limit == 0 {
        return None;
    }

    let mut starts = vec![Vec::<usize>::new(); n + 1];
    for id in 1..cand_count {
        if (id & 15) == 0 && timer.elapsed() >= deadline {
            return None;
        }
        let len = cands[id].len();
        if len == 0 || len > n {
            continue;
        }
        for pos in 0..=n - len {
            if starts_with_at(basic, pos, &cands[id]) {
                starts[pos].push(id);
            }
        }
    }

    let mut reg_cost = vec![vec![u16::MAX; cand_count]; cand_count];
    for old in 0..cand_count {
        if (old & 7) == 0 && timer.elapsed() >= deadline {
            return None;
        }
        for new in 1..cand_count {
            if old == new {
                continue;
            }
            let cost = parse_power_cost(&cands[new], &cands[old]) + 2;
            if cost <= best_limit {
                reg_cost[old][new] = cost as u16;
            }
        }
    }

    let state_count = (n + 1) * cand_count;
    let state_id = |pos: usize, mid: usize| -> usize { pos * cand_count + mid };
    let mut dist = vec![u16::MAX; state_count];
    let mut prev = vec![
        PowerPrev {
            prev: u32::MAX,
            action: u16::MAX,
        };
        state_count
    ];
    let mut buckets = vec![Vec::<u32>::new(); best_limit + 1];
    let start = state_id(0, 0);
    dist[start] = 0;
    buckets[0].push(start as u32);
    let mut goal = None;
    let mut popped = 0usize;

    for d in 0..=best_limit {
        while let Some(su) = buckets[d].pop() {
            popped += 1;
            if (popped & 4095) == 0 && timer.elapsed() >= deadline {
                return None;
            }
            let sid = su as usize;
            if dist[sid] as usize != d {
                continue;
            }
            let pos = sid / cand_count;
            let mid = sid - pos * cand_count;
            if pos == n {
                goal = Some(sid);
                break;
            }

            let nd = d + 1;
            if nd <= best_limit {
                let nk = state_id(pos + 1, mid);
                if nd < dist[nk] as usize {
                    dist[nk] = nd as u16;
                    prev[nk] = PowerPrev {
                        prev: su,
                        action: basic[pos] as u16,
                    };
                    buckets[nd].push(nk as u32);
                }
            }

            if mid != 0 && starts_with_at(basic, pos, &cands[mid]) {
                let np = pos + cands[mid].len();
                let nk = state_id(np, mid);
                if nd <= best_limit && nd < dist[nk] as usize {
                    dist[nk] = nd as u16;
                    prev[nk] = PowerPrev {
                        prev: su,
                        action: OP_P as u16,
                    };
                    buckets[nd].push(nk as u32);
                }
            }

            for &new_mid in &starts[pos] {
                if new_mid == mid {
                    continue;
                }
                let rc = reg_cost[mid][new_mid];
                if rc == u16::MAX {
                    continue;
                }
                let nd = d + rc as usize;
                if nd > best_limit {
                    continue;
                }
                let nk = state_id(pos + cands[new_mid].len(), new_mid);
                if nd < dist[nk] as usize {
                    dist[nk] = nd as u16;
                    prev[nk] = PowerPrev {
                        prev: su,
                        action: POWER_RECOMPRESS_REGISTER_BASE + new_mid as u16,
                    };
                    buckets[nd].push(nk as u32);
                }
            }
        }
        if goal.is_some() {
            break;
        }
    }

    let mut key = goal?;
    let mut chunks = Vec::<Vec<u8>>::new();
    while key != start {
        let p = prev[key];
        if p.action >= POWER_RECOMPRESS_REGISTER_BASE {
            let old_mid = (p.prev as usize) % cand_count;
            let new_mid = (p.action - POWER_RECOMPRESS_REGISTER_BASE) as usize;
            let mut chunk = Vec::new();
            chunk.push(OP_M);
            chunk.extend(parse_power_inside(&cands[new_mid], &cands[old_mid]));
            chunk.push(OP_M);
            chunks.push(chunk);
        } else {
            chunks.push(vec![p.action as u8]);
        }
        key = p.prev as usize;
    }
    chunks.reverse();
    let mut answer = Vec::new();
    for chunk in chunks {
        answer.extend(chunk);
    }
    if answer.len() <= best_limit {
        Some((answer, cand_count))
    } else {
        None
    }
}

#[inline(always)]
fn max_registered_level_for_len(len: usize, allow_power: bool) -> usize {
    if !allow_power {
        return 1;
    }
    let mut level = 1usize;
    let mut expanded = len;
    while level < 5 && expanded * 2 <= 32 {
        expanded *= 2;
        level += 1;
    }
    level
}

fn matches_ops_at(ops: &[u8], pat: &[u8], pos: usize) -> bool {
    pos + pat.len() <= ops.len() && &ops[pos..pos + pat.len()] == pat
}

fn encode_macro_with_seed(ops: &[u8], seed: &[u8]) -> (usize, usize, Vec<u8>) {
    let n = ops.len();
    let seed_len = seed.len();
    let mut dp = vec![0usize; n + 1];
    let mut p_count = vec![0usize; n + 1];
    let mut take_p = vec![false; n + 1];
    for i in (0..n).rev() {
        let mut best = 1 + dp[i + 1];
        let mut best_p_count = p_count[i + 1];
        if matches_ops_at(ops, seed, i) {
            let cand = 1 + dp[i + seed_len];
            let cand_p_count = 1 + p_count[i + seed_len];
            if cand < best || (cand == best && cand_p_count > best_p_count) {
                best = cand;
                best_p_count = cand_p_count;
                take_p[i] = true;
            }
        }
        dp[i] = best;
        p_count[i] = best_p_count;
    }

    let mut encoded = Vec::with_capacity(dp[0]);
    let mut pos = 0usize;
    while pos < n {
        if take_p[pos] {
            encoded.push(OP_P);
            pos += seed_len;
        } else {
            encoded.push(ops[pos]);
            pos += 1;
        }
    }
    (dp[0], p_count[0], encoded)
}

#[derive(Clone)]
struct MacroOfMacro {
    helper: Vec<u8>,
    encoded_q: Vec<u8>,
    total_cost: usize,
}

fn get_macro_of_macro(q: &[u8]) -> Option<MacroOfMacro> {
    if q.len() < 4 {
        return None;
    }
    let direct_cost = q.len() + 2;
    let max_helper_len = min(32, q.len() / 2);
    let mut best: Option<MacroOfMacro> = None;
    for helper_len in 2..=max_helper_len {
        for start in 0..=q.len() - helper_len {
            let helper = &q[start..start + helper_len];
            let (encoded_len, uses, encoded_q) = encode_macro_with_seed(q, helper);
            if uses < 2 {
                continue;
            }
            let total_cost = helper_len + 2 + encoded_len + 2;
            if total_cost + MOM_MIN_TOTAL_GAIN > direct_cost {
                continue;
            }
            let better = best.as_ref().map_or(true, |cur| {
                total_cost < cur.total_cost
                    || (total_cost == cur.total_cost && helper_len < cur.helper.len())
            });
            if better {
                best = Some(MacroOfMacro {
                    helper: helper.to_vec(),
                    encoded_q,
                    total_cost,
                });
            }
        }
    }
    best
}

#[derive(Clone, Copy, Default)]
struct Edge {
    to: u16,
    cost: u16,
    op: u8,
}

struct MacroGraph {
    cell_count: usize,
    max_level: usize,
    state_count: usize,
    max_edge_cost: usize,
    edges: Vec<[Edge; 6]>,
    edge_cnt: Vec<u8>,
    rev_start: Vec<u32>,
    rev_packed: Vec<u32>,
    cell_of_state: Vec<u16>,
    macro_of_macro: Option<MacroOfMacro>,
    initial_state: usize,
    initial_buttons: Vec<u8>,
}

impl MacroGraph {
    fn new(grid: &Grid, q: &[u8], allow_power: bool) -> Self {
        let mut graph = Self {
            cell_count: 0,
            max_level: 1,
            state_count: 0,
            max_edge_cost: 1,
            edges: Vec::new(),
            edge_cnt: Vec::new(),
            rev_start: Vec::new(),
            rev_packed: Vec::new(),
            cell_of_state: Vec::new(),
            macro_of_macro: None,
            initial_state: 0,
            initial_buttons: Vec::new(),
        };
        graph.build(grid, q, allow_power);
        graph
    }

    #[inline(always)]
    fn state_id(&self, cell: usize, dir: usize, level: usize) -> usize {
        ((level * self.cell_count + cell) << 2) + dir
    }

    #[inline(always)]
    fn decode(&self, state: usize) -> (usize, usize, usize) {
        let dir = state & 3;
        let x = state >> 2;
        let level = x / self.cell_count;
        let cell = x - level * self.cell_count;
        (cell, dir, level)
    }

    #[inline(always)]
    fn start_state(&self) -> usize {
        self.initial_state
    }

    #[inline(always)]
    fn initial_cost(&self) -> usize {
        self.initial_buttons.len()
    }

    #[inline(always)]
    #[cfg(feature = "local")]
    fn has_macro_of_macro(&self) -> bool {
        self.macro_of_macro.is_some()
    }

    fn apply_ops(
        &self,
        grid: &Grid,
        ops: &[u8],
        mut cell: usize,
        mut dir: usize,
    ) -> (usize, usize) {
        for &op in ops {
            if op == OP_F {
                cell = grid.next(cell, dir);
            } else if op == OP_R {
                dir = (dir + 1) & 3;
            } else if op == OP_L {
                dir = (dir + 3) & 3;
            }
        }
        (cell, dir)
    }

    #[inline(always)]
    fn rev_edges_packed(&self, state: usize) -> &[u32] {
        let l = self.rev_start[state] as usize;
        let r = self.rev_start[state + 1] as usize;
        &self.rev_packed[l..r]
    }

    #[inline(always)]
    fn add_edge(&mut self, from: usize, to: usize, cost: usize, op: u8) {
        let idx = self.edge_cnt[from] as usize;
        self.edge_cnt[from] += 1;
        let e = Edge {
            to: to as u16,
            cost: cost as u16,
            op,
        };
        self.edges[from][idx] = e;
    }

    fn build(&mut self, grid: &Grid, q: &[u8], allow_power: bool) {
        self.cell_count = grid.cell_count;
        self.max_level = max_registered_level_for_len(q.len(), allow_power);
        self.state_count = self.cell_count * 4 * (self.max_level + 1);
        self.macro_of_macro = get_macro_of_macro(q);
        self.initial_buttons.clear();
        self.initial_state = self.state_id(0, 1, 0);
        let mom_register_cost = self
            .macro_of_macro
            .as_ref()
            .map_or(0, |mom| mom.encoded_q.len() + 2);
        self.max_edge_cost = max(max(q.len() + 2, 4), mom_register_cost);
        self.edges = vec![[Edge::default(); 6]; self.state_count];
        self.edge_cnt = vec![0u8; self.state_count];
        self.rev_start.clear();
        self.rev_packed.clear();
        self.cell_of_state = vec![0u16; self.state_count];

        let base = macro_effect_base(grid, q);
        let effect = build_macro_effects(&base, self.max_level);

        let helper_effect = self.macro_of_macro.as_ref().map(|mom| {
            let mut eff = vec![0u16; self.cell_count * 4];
            for cell in 0..self.cell_count {
                for dir in 0..4 {
                    let (c1, d1) = self.apply_ops(grid, &mom.helper, cell, dir);
                    eff[cell * 4 + dir] = (c1 * 4 + d1) as u16;
                }
            }
            eff
        });
        if let Some(mom) = self.macro_of_macro.as_ref() {
            self.initial_buttons.push(OP_M);
            self.initial_buttons.extend_from_slice(&mom.helper);
            self.initial_buttons.push(OP_M);
            let s0 = helper_effect.as_ref().unwrap()[1] as usize;
            self.initial_state = self.state_id(s0 >> 2, s0 & 3, 0);
        }
        let mom_encoded_len = self.macro_of_macro.as_ref().map(|mom| mom.encoded_q.len());

        for state in 0..self.state_count {
            let (cell, dir, level) = self.decode(state);
            self.cell_of_state[state] = cell as u16;
            self.add_edge(
                state,
                self.state_id(grid.next(cell, dir), dir, level),
                1,
                OP_F,
            );
            self.add_edge(state, self.state_id(cell, (dir + 1) & 3, level), 1, OP_R);
            self.add_edge(state, self.state_id(cell, (dir + 3) & 3, level), 1, OP_L);
            let e1 = effect[1][cell * 4 + dir] as usize;
            self.add_edge(
                state,
                self.state_id(e1 >> 2, e1 & 3, 1),
                q.len() + 2,
                OP_REGISTER,
            );
            if level == 0 {
                if let Some(encoded_len) = mom_encoded_len {
                    self.add_edge(
                        state,
                        self.state_id(e1 >> 2, e1 & 3, 1),
                        encoded_len + 2,
                        OP_REGISTER_MOM,
                    );
                    let eh = helper_effect.as_ref().unwrap()[cell * 4 + dir] as usize;
                    self.add_edge(state, self.state_id(eh >> 2, eh & 3, 0), 1, OP_P);
                }
            }
            if level >= 1 {
                let ep = effect[level][cell * 4 + dir] as usize;
                self.add_edge(state, self.state_id(ep >> 2, ep & 3, level), 1, OP_P);
                if level < self.max_level {
                    let ed = effect[level + 1][cell * 4 + dir] as usize;
                    self.add_edge(
                        state,
                        self.state_id(ed >> 2, ed & 3, level + 1),
                        4,
                        OP_DOUBLE,
                    );
                }
            }
        }

        self.rev_start = vec![0u32; self.state_count + 1];
        for from in 0..self.state_count {
            for ei in 0..self.edge_cnt[from] as usize {
                let to = self.edges[from][ei].to as usize;
                self.rev_start[to + 1] += 1;
            }
        }
        for state in 0..self.state_count {
            self.rev_start[state + 1] += self.rev_start[state];
        }
        self.rev_packed = vec![0u32; self.rev_start[self.state_count] as usize];
        let mut cursor = self.rev_start[..self.state_count].to_vec();
        for from in 0..self.state_count {
            for ei in 0..self.edge_cnt[from] as usize {
                let e = self.edges[from][ei];
                let to = e.to as usize;
                let pos = cursor[to] as usize;
                cursor[to] += 1;
                self.rev_packed[pos] =
                    from as u32 | ((e.cost as u32) << 16) | ((e.op as u32) << 24);
            }
        }
    }
}

struct ForwardScratch {
    dist: Vec<u32>,
    prev_state: Vec<u16>,
    prev_op: Vec<u8>,
    touched_states: Vec<usize>,
    buckets: RouteScratch,
}

impl ForwardScratch {
    fn new(state_count: usize, bucket_count: usize) -> Self {
        Self {
            dist: vec![INF_COST; state_count],
            prev_state: vec![u16::MAX; state_count],
            prev_op: vec![OP_NONE; state_count],
            touched_states: Vec::with_capacity(state_count.min(1024)),
            buckets: RouteScratch::new(bucket_count),
        }
    }

    fn begin(&mut self) {
        for &state in &self.touched_states {
            self.dist[state] = INF_COST;
            self.prev_state[state] = u16::MAX;
            self.prev_op[state] = OP_NONE;
        }
        self.touched_states.clear();
        self.buckets.begin();
    }

    #[inline(always)]
    fn set_state(&mut self, state: usize, dist: u32, prev_state: u16, prev_op: u8) {
        if self.dist[state] == INF_COST {
            self.touched_states.push(state);
        }
        self.dist[state] = dist;
        self.prev_state[state] = prev_state;
        self.prev_op[state] = prev_op;
    }
}

fn forward_best_ball(
    graph: &MacroGraph,
    routes: &RouteTables,
    ball_at: &[usize],
    done: &[bool],
    start: usize,
    limit: usize,
    scratch: &mut ForwardScratch,
) -> Option<(usize, usize)> {
    scratch.begin();
    scratch.set_state(start, 0, u16::MAX, OP_NONE);
    scratch.buckets.push(0, start as u16);

    let mut best_k = None;
    let mut best_bs = 0usize;
    let mut best_total = INF_COST;
    let mut best_first = INF_COST;

    for cur in 0..=limit {
        if best_total < INF_COST && cur + 2 > best_total as usize {
            break;
        }
        while let Some(su) = scratch.buckets.pop(cur) {
            let state = su as usize;
            if scratch.dist[state] != cur as u32 {
                continue;
            }

            let cell = graph.cell_of_state[state] as usize;
            let k = ball_at[cell];
            if k != usize::MAX && !done[k] {
                let d2 = routes.dist_at(k, state);
                if d2 < INF_COST {
                    let total = cur as u32 + 1 + d2 + 1;
                    let better = total < best_total
                        || (total == best_total
                            && ((cur as u32) < best_first
                                || ((cur as u32) == best_first
                                    && (best_k.map_or(true, |bk| k < bk)
                                        || (best_k == Some(k) && state < best_bs)))));
                    if better {
                        best_total = total;
                        best_first = cur as u32;
                        best_k = Some(k);
                        best_bs = state;
                    }
                }
            }

            if best_total < INF_COST && cur + 2 >= best_total as usize {
                continue;
            }

            for ei in 0..graph.edge_cnt[state] as usize {
                let e = graph.edges[state][ei];
                let nc = cur + e.cost as usize;
                let to = e.to as usize;
                if nc <= limit && (nc as u32) < scratch.dist[to] {
                    scratch.set_state(to, nc as u32, su, e.op);
                    scratch.buckets.push(nc, to as u16);
                }
            }
        }
    }
    best_k.map(|k| (k, best_bs))
}

struct RouteTables {
    state_count: usize,
    dist: Vec<u32>,
    next_state: Vec<u16>,
    next_op: Vec<u8>,
    end_state: Vec<u16>,
}

impl RouteTables {
    fn new(target_count: usize, state_count: usize) -> Self {
        let total = target_count * state_count;
        Self {
            state_count,
            dist: vec![INF_COST; total],
            next_state: vec![u16::MAX; total],
            next_op: vec![OP_NONE; total],
            end_state: vec![u16::MAX; total],
        }
    }

    #[inline(always)]
    fn idx(&self, target_idx: usize, state: usize) -> usize {
        target_idx * self.state_count + state
    }

    #[inline(always)]
    fn dist_at(&self, target_idx: usize, state: usize) -> u32 {
        unsafe { *self.dist.get_unchecked(self.idx(target_idx, state)) }
    }

    #[inline(always)]
    fn next_state_at(&self, target_idx: usize, state: usize) -> u16 {
        unsafe { *self.next_state.get_unchecked(self.idx(target_idx, state)) }
    }

    #[inline(always)]
    fn next_op_at(&self, target_idx: usize, state: usize) -> u8 {
        unsafe { *self.next_op.get_unchecked(self.idx(target_idx, state)) }
    }

    #[inline(always)]
    fn end_state_at(&self, target_idx: usize, state: usize) -> u16 {
        unsafe { *self.end_state.get_unchecked(self.idx(target_idx, state)) }
    }
}

struct RouteScratch {
    buckets: Vec<Vec<u16>>,
    mark: Vec<u32>,
    touched: Vec<usize>,
    epoch: u32,
}

impl RouteScratch {
    fn new(bucket_count: usize) -> Self {
        Self {
            buckets: vec![Vec::new(); bucket_count],
            mark: vec![0; bucket_count],
            touched: Vec::with_capacity(bucket_count.min(1024)),
            epoch: 0,
        }
    }

    fn begin(&mut self) {
        for &idx in &self.touched {
            self.buckets[idx].clear();
        }
        self.touched.clear();
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.mark.fill(0);
            self.epoch = 1;
        }
    }

    #[inline(always)]
    fn push(&mut self, bucket_idx: usize, state: u16) {
        if self.mark[bucket_idx] != self.epoch {
            self.mark[bucket_idx] = self.epoch;
            self.touched.push(bucket_idx);
        }
        self.buckets[bucket_idx].push(state);
    }

    #[inline(always)]
    fn pop(&mut self, bucket_idx: usize) -> Option<u16> {
        self.buckets[bucket_idx].pop()
    }
}

fn build_routes_to_targets(
    graph: &MacroGraph,
    target_cells: &[u16],
    source_cells: &[u16],
    limit: usize,
) -> RouteTables {
    let mut routes = RouteTables::new(target_cells.len(), graph.state_count);
    let mut scratch = RouteScratch::new(limit + graph.max_edge_cost + 1);
    for (target_idx, (&target_cell, &source_cell)) in
        target_cells.iter().zip(source_cells.iter()).enumerate()
    {
        scratch.begin();
        let offset = target_idx * graph.state_count;
        let dist = &mut routes.dist[offset..offset + graph.state_count];
        let next_state = &mut routes.next_state[offset..offset + graph.state_count];
        let next_op = &mut routes.next_op[offset..offset + graph.state_count];
        let end_state = &mut routes.end_state[offset..offset + graph.state_count];
        let mut remaining_source_states = 4 * (graph.max_level + 1);
        for level in 0..=graph.max_level {
            for dir in 0..4 {
                let state = graph.state_id(target_cell as usize, dir, level);
                dist[state] = 0;
                end_state[state] = state as u16;
                scratch.push(0, state as u16);
            }
        }
        let mut done = false;
        for cur in 0..=limit {
            if done {
                break;
            }
            while let Some(su) = scratch.pop(cur) {
                let state = su as usize;
                if dist[state] != cur as u32 {
                    continue;
                }
                if graph.cell_of_state[state] == source_cell {
                    remaining_source_states -= 1;
                    if remaining_source_states == 0 {
                        done = true;
                        break;
                    }
                }
                for &rev in graph.rev_edges_packed(state) {
                    let prev = (rev & 0xffff) as usize;
                    let nc = cur + ((rev >> 16) & 0xff) as usize;
                    if nc <= limit && (nc as u32) < dist[prev] {
                        dist[prev] = nc as u32;
                        next_state[prev] = su;
                        next_op[prev] = (rev >> 24) as u8;
                        end_state[prev] = end_state[state];
                        scratch.push(nc, prev as u16);
                    }
                }
            }
        }
    }
    routes
}

fn emit_graph_op(buttons: &mut Vec<u8>, op: u8, q: &[u8], mom: Option<&MacroOfMacro>) {
    if op == OP_F || op == OP_R || op == OP_L || op == OP_P {
        buttons.push(op);
    } else if op == OP_REGISTER {
        buttons.push(OP_M);
        buttons.extend_from_slice(q);
        buttons.push(OP_M);
    } else if op == OP_REGISTER_MOM {
        let Some(mom) = mom else {
            return;
        };
        buttons.push(OP_M);
        buttons.extend_from_slice(&mom.encoded_q);
        buttons.push(OP_M);
    } else if op == OP_DOUBLE {
        buttons.push(OP_M);
        buttons.push(OP_P);
        buttons.push(OP_P);
        buttons.push(OP_M);
    }
}

fn append_forward_path(
    search: &ForwardScratch,
    start: usize,
    target: usize,
    q: &[u8],
    mom: Option<&MacroOfMacro>,
    buttons: &mut Vec<u8>,
) -> bool {
    if search.dist[target] >= INF_COST {
        return false;
    }
    let mut ops = Vec::new();
    let mut cur = target;
    while cur != start {
        let op = search.prev_op[cur];
        let prev = search.prev_state[cur];
        if op == OP_NONE || prev == u16::MAX {
            return false;
        }
        ops.push(op);
        cur = prev as usize;
    }
    for &op in ops.iter().rev() {
        emit_graph_op(buttons, op, q, mom);
    }
    true
}

fn append_route_to_target(
    routes: &RouteTables,
    target_idx: usize,
    mut state: usize,
    q: &[u8],
    mom: Option<&MacroOfMacro>,
    buttons: &mut Vec<u8>,
) -> Option<usize> {
    while routes.dist_at(target_idx, state) != 0 {
        let op = routes.next_op_at(target_idx, state);
        let ns = routes.next_state_at(target_idx, state);
        if op == OP_NONE || ns == u16::MAX {
            return None;
        }
        emit_graph_op(buttons, op, q, mom);
        state = ns as usize;
    }
    Some(state)
}

fn route_end_state(routes: &RouteTables, target_idx: usize, state: usize) -> Option<usize> {
    let es = routes.end_state_at(target_idx, state);
    if es == u16::MAX {
        None
    } else {
        Some(es as usize)
    }
}

struct MacroSolveResult {
    buttons: Vec<u8>,
    q: SmallQ,
    order: Vec<usize>,
    prepared: Option<GreedyPrepared>,
    #[cfg(feature = "local")]
    mom_used: bool,
}

struct GreedyPrepared {
    graph: Box<MacroGraph>,
    routes: Box<RouteTables>,
}

fn solve_with_macro_candidate_greedy(
    input: &Input,
    grid: &Grid,
    q: SmallQ,
    best_limit: usize,
    allow_power: bool,
) -> Option<MacroSolveResult> {
    let qs = q.as_slice();
    let graph = MacroGraph::new(grid, qs, allow_power);
    let mut basket_targets = vec![0u16; input.m];
    basket_targets[..input.m].copy_from_slice(&input.basket_pos[..input.m]);
    let mut ball_sources = vec![0u16; input.m];
    ball_sources[..input.m].copy_from_slice(&input.ball_pos[..input.m]);
    let routes = build_routes_to_targets(&graph, &basket_targets, &ball_sources, best_limit);
    let mut done = vec![false; input.m];
    let mut done_count = 0usize;
    let mut state = graph.start_state();
    let mut buttons = Vec::with_capacity(min(best_limit, 4096));
    buttons.extend_from_slice(&graph.initial_buttons);
    if buttons.len() >= best_limit || buttons.len() > input.t_limit {
        return None;
    }
    let mut order = Vec::with_capacity(input.m);
    let mut ball_at = vec![usize::MAX; graph.cell_count];
    for k in 0..input.m {
        ball_at[input.ball_pos[k] as usize] = k;
    }
    let mut forward_scratch =
        ForwardScratch::new(graph.state_count, best_limit + graph.max_edge_cost + 1);

    while done_count < input.m {
        let remaining = best_limit.saturating_sub(buttons.len());
        let (k, best_bs) = forward_best_ball(
            &graph,
            &routes,
            &ball_at,
            &done,
            state,
            remaining,
            &mut forward_scratch,
        )?;
        if !append_forward_path(
            &forward_scratch,
            state,
            best_bs,
            qs,
            graph.macro_of_macro.as_ref(),
            &mut buttons,
        ) {
            return None;
        }
        state = best_bs;
        buttons.push(OP_S);
        state = append_route_to_target(
            &routes,
            k,
            state,
            qs,
            graph.macro_of_macro.as_ref(),
            &mut buttons,
        )?;
        buttons.push(OP_S);
        done[k] = true;
        order.push(k);
        done_count += 1;
        if buttons.len() >= best_limit || buttons.len() > input.t_limit {
            return None;
        }
    }

    #[cfg(feature = "local")]
    let mom_used = graph.has_macro_of_macro();
    Some(MacroSolveResult {
        buttons,
        q,
        order,
        prepared: Some(GreedyPrepared {
            graph: Box::new(graph),
            routes: Box::new(routes),
        }),
        #[cfg(feature = "local")]
        mom_used,
    })
}

struct DeliveryPrecomp {
    state_count: usize,
    cost: Vec<u32>,
    end_state: Vec<u16>,
    next_state: Vec<u16>,
    next_op: Vec<u8>,
}

impl DeliveryPrecomp {
    #[inline(always)]
    fn idx(&self, k: usize, state: usize) -> usize {
        k * self.state_count + state
    }

    #[inline(always)]
    unsafe fn cost_unchecked(&self, k: usize, state: usize) -> u32 {
        unsafe { *self.cost.get_unchecked(self.idx(k, state)) }
    }

    #[inline(always)]
    unsafe fn end_state_unchecked(&self, k: usize, state: usize) -> u16 {
        unsafe { *self.end_state.get_unchecked(self.idx(k, state)) }
    }

    #[inline(always)]
    unsafe fn next_state_unchecked(&self, k: usize, state: usize) -> u16 {
        unsafe { *self.next_state.get_unchecked(self.idx(k, state)) }
    }

    #[inline(always)]
    unsafe fn next_op_unchecked(&self, k: usize, state: usize) -> u8 {
        unsafe { *self.next_op.get_unchecked(self.idx(k, state)) }
    }
}

fn build_delivery_precomp(
    input: &Input,
    graph: &MacroGraph,
    routes: &RouteTables,
    limit: usize,
) -> DeliveryPrecomp {
    let total = input.m * graph.state_count;
    let mut pc = DeliveryPrecomp {
        state_count: graph.state_count,
        cost: vec![INF_COST; total],
        end_state: vec![u16::MAX; total],
        next_state: vec![u16::MAX; total],
        next_op: vec![OP_NONE; total],
    };
    let mut scratch = RouteScratch::new(limit + graph.max_edge_cost + 1);
    for k in 0..input.m {
        scratch.begin();
        let offset = k * graph.state_count;
        let cost = &mut pc.cost[offset..offset + graph.state_count];
        let end_state = &mut pc.end_state[offset..offset + graph.state_count];
        let next_state = &mut pc.next_state[offset..offset + graph.state_count];
        let next_op = &mut pc.next_op[offset..offset + graph.state_count];
        let ball = input.ball_pos[k] as usize;
        for level in 0..=graph.max_level {
            for dir in 0..4 {
                let bs = graph.state_id(ball, dir, level);
                let d2 = routes.dist_at(k, bs);
                if d2 >= INF_COST {
                    continue;
                }
                let Some(es) = route_end_state(routes, k, bs) else {
                    continue;
                };
                let seed_cost = d2 + 2;
                if seed_cost <= limit as u32 && seed_cost < cost[bs] {
                    cost[bs] = seed_cost;
                    end_state[bs] = es as u16;
                    scratch.push(seed_cost as usize, bs as u16);
                }
            }
        }
        for cur in 0..=limit {
            loop {
                let Some(su) = scratch.pop(cur) else {
                    break;
                };
                let state = su as usize;
                if cost[state] != cur as u32 {
                    continue;
                }
                for &rev in graph.rev_edges_packed(state) {
                    let prev = (rev & 0xffff) as usize;
                    let nc = cur + ((rev >> 16) & 0xff) as usize;
                    if nc <= limit && (nc as u32) < cost[prev] {
                        cost[prev] = nc as u32;
                        end_state[prev] = end_state[state];
                        next_state[prev] = state as u16;
                        next_op[prev] = (rev >> 24) as u8;
                        scratch.push(nc, prev as u16);
                    }
                }
            }
        }
    }
    pc
}

fn eval_order_with_precomp(
    pc: &DeliveryPrecomp,
    graph: &MacroGraph,
    order: &[usize],
) -> (u32, usize) {
    let mut state = graph.start_state();
    let mut total = graph.initial_cost() as u32;
    for &k in order {
        let c = unsafe { pc.cost_unchecked(k, state) };
        let es = unsafe { pc.end_state_unchecked(k, state) };
        if c >= INF_COST || es == u16::MAX {
            return (INF_COST, state);
        }
        if total >= INF_COST - c {
            return (INF_COST, state);
        }
        total += c;
        state = es as usize;
    }
    (total, state)
}

fn fill_order_prefix(
    pc: &DeliveryPrecomp,
    graph: &MacroGraph,
    order: &[usize],
    prefix_state: &mut [u16],
    prefix_cost: &mut [u32],
) -> u32 {
    let mut state = graph.start_state();
    let mut total = graph.initial_cost() as u32;
    prefix_state[0] = state as u16;
    prefix_cost[0] = total;
    for (i, &k) in order.iter().enumerate() {
        let c = unsafe { pc.cost_unchecked(k, state) };
        let es = unsafe { pc.end_state_unchecked(k, state) };
        if c >= INF_COST || es == u16::MAX || total >= INF_COST - c {
            return INF_COST;
        }
        total += c;
        state = es as usize;
        prefix_state[i + 1] = es;
        prefix_cost[i + 1] = total;
    }
    total
}

fn eval_order_suffix_with_precomp(
    pc: &DeliveryPrecomp,
    order: &[usize],
    start: usize,
    cur_prefix_state: &[u16],
    cur_prefix_cost: &[u32],
    scratch_prefix_state: &mut [u16],
    scratch_prefix_cost: &mut [u32],
) -> u32 {
    let mut state = cur_prefix_state[start] as usize;
    let mut total = cur_prefix_cost[start];
    for (i, &k) in order.iter().enumerate().skip(start) {
        let c = unsafe { pc.cost_unchecked(k, state) };
        let es = unsafe { pc.end_state_unchecked(k, state) };
        if c >= INF_COST || es == u16::MAX || total >= INF_COST - c {
            return INF_COST;
        }
        total += c;
        state = es as usize;
        scratch_prefix_state[i + 1] = es;
        scratch_prefix_cost[i + 1] = total;
    }
    total
}

fn greedy_order_precomp(input: &Input, graph: &MacroGraph, pc: &DeliveryPrecomp) -> Vec<usize> {
    let mut order = Vec::with_capacity(input.m);
    let mut done = vec![false; input.m];
    let mut state = graph.start_state();
    for _ in 0..input.m {
        let mut best_k = None;
        let mut best_c = INF_COST;
        for k in 0..input.m {
            if done[k] {
                continue;
            }
            let c = unsafe { pc.cost_unchecked(k, state) };
            if c < best_c {
                best_c = c;
                best_k = Some(k);
            }
        }
        let Some(k) = best_k else {
            break;
        };
        order.push(k);
        done[k] = true;
        let es = unsafe { pc.end_state_unchecked(k, state) };
        if es == u16::MAX {
            break;
        }
        state = es as usize;
    }
    if order.len() != input.m {
        let mut used = vec![false; input.m];
        for &k in &order {
            used[k] = true;
        }
        for (k, used_k) in used.iter().enumerate().take(input.m) {
            if !*used_k {
                order.push(k);
            }
        }
    }
    order
}

fn hash_order(order: &[usize]) -> u64 {
    let mut h = 1469598103934665603u64;
    for &x in order {
        h ^= x as u64 + 1;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn mutate_order(order: &mut Vec<usize>, rng: &mut XorShift64) {
    let n = order.len();
    if n <= 1 {
        return;
    }
    let typ = rng.randint(0, 6);
    if typ == 0 {
        let a = rng.randint(0, n - 1);
        let b = rng.randint(0, n - 1);
        if a != b {
            order.swap(a, b);
        }
    } else if typ == 1 {
        let mut a = rng.randint(0, n - 1);
        let mut b = rng.randint(0, n - 1);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        if a < b {
            order[a..=b].reverse();
        }
    } else if typ == 2 {
        let a = rng.randint(0, n - 1);
        let b = rng.randint(0, n - 1);
        if a == b {
            return;
        }
        let x = order.remove(a);
        let pos = min(b, order.len());
        order.insert(pos, x);
    } else if typ == 3 {
        let a = rng.randint(0, n - 1);
        let len = rng.randint(2, min(n, 8));
        let b = min(n, a + len);
        if b - a >= 2 {
            order[a..b].rotate_left(1);
        }
    } else if typ == 4 {
        let a = rng.randint(0, n - 2);
        order.swap(a, a + 1);
    } else if typ == 5 {
        let mut a = rng.randint(0, n - 1);
        let mut b = rng.randint(0, n - 1);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        if b - a >= 3 {
            let c = rng.randint(a + 1, b - 1);
            order[a..=b].rotate_left(c - a);
        }
    } else {
        let a = rng.randint(0, n - 1);
        let len = rng.randint(1, min(n - a, 5));
        let b = a + len;
        let seg: Vec<usize> = order.drain(a..b).collect();
        let pos = rng.randint(0, order.len());
        order.splice(pos..pos, seg);
    }
}

enum MutateRecord {
    Swap(usize, usize),
    Reverse { l: usize, r: usize },
    MoveOne { from: usize, to: usize },
    RotateLeft { l: usize, r: usize, by: usize },
    MoveSegment { from: usize, to: usize, len: usize },
}

impl MutateRecord {
    #[inline(always)]
    fn dirty_start(&self) -> usize {
        match *self {
            MutateRecord::Swap(a, b) => min(a, b),
            MutateRecord::Reverse { l, .. } => l,
            MutateRecord::MoveOne { from, to } => min(from, to),
            MutateRecord::RotateLeft { l, .. } => l,
            MutateRecord::MoveSegment { from, to, .. } => min(from, to),
        }
    }
}

#[inline(always)]
fn move_segment_in_place(order: &mut [usize], from: usize, len: usize, to: usize) {
    if len == 0 || from == to {
        return;
    }
    if to < from {
        order[to..from + len].rotate_right(len);
    } else {
        order[from..to + len].rotate_left(len);
    }
}

fn mutate_order_in_place(order: &mut Vec<usize>, rng: &mut XorShift64) -> Option<MutateRecord> {
    let n = order.len();
    if n <= 1 {
        return None;
    }
    let typ = rng.randint(0, 6);
    if typ == 0 {
        let a = rng.randint(0, n - 1);
        let b = rng.randint(0, n - 1);
        if a == b {
            return None;
        }
        order.swap(a, b);
        Some(MutateRecord::Swap(a, b))
    } else if typ == 1 {
        let mut a = rng.randint(0, n - 1);
        let mut b = rng.randint(0, n - 1);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        if a >= b {
            return None;
        }
        order[a..=b].reverse();
        Some(MutateRecord::Reverse { l: a, r: b })
    } else if typ == 2 {
        let a = rng.randint(0, n - 1);
        let b = rng.randint(0, n - 1);
        if a == b {
            return None;
        }
        let pos = min(b, n - 1);
        move_segment_in_place(order, a, 1, pos);
        Some(MutateRecord::MoveOne { from: a, to: pos })
    } else if typ == 3 {
        let a = rng.randint(0, n - 1);
        let len = rng.randint(2, min(n, 8));
        let b = min(n, a + len);
        if b - a < 2 {
            return None;
        }
        order[a..b].rotate_left(1);
        Some(MutateRecord::RotateLeft { l: a, r: b, by: 1 })
    } else if typ == 4 {
        let a = rng.randint(0, n - 2);
        order.swap(a, a + 1);
        Some(MutateRecord::Swap(a, a + 1))
    } else if typ == 5 {
        let mut a = rng.randint(0, n - 1);
        let mut b = rng.randint(0, n - 1);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        if b - a < 3 {
            return None;
        }
        let c = rng.randint(a + 1, b - 1);
        let by = c - a;
        order[a..=b].rotate_left(by);
        Some(MutateRecord::RotateLeft { l: a, r: b + 1, by })
    } else {
        let a = rng.randint(0, n - 1);
        let len = rng.randint(1, min(n - a, 5));
        let pos = rng.randint(0, n - len);
        move_segment_in_place(order, a, len, pos);
        Some(MutateRecord::MoveSegment {
            from: a,
            to: pos,
            len,
        })
    }
}

fn rollback_order_mutation(order: &mut Vec<usize>, record: MutateRecord) {
    match record {
        MutateRecord::Swap(a, b) => {
            order.swap(a, b);
        }
        MutateRecord::Reverse { l, r } => {
            order[l..=r].reverse();
        }
        MutateRecord::MoveOne { from, to } => {
            move_segment_in_place(order, to, 1, from);
        }
        MutateRecord::RotateLeft { l, r, by } => {
            order[l..r].rotate_right(by % (r - l));
        }
        MutateRecord::MoveSegment { from, to, len } => {
            move_segment_in_place(order, to, len, from);
        }
    }
}

fn build_buttons_for_order(
    input: &Input,
    graph: &MacroGraph,
    routes: &RouteTables,
    pc: &DeliveryPrecomp,
    q: &[u8],
    order: &[usize],
    best_limit: usize,
) -> Vec<u8> {
    let mut state = graph.start_state();
    let mut buttons = Vec::with_capacity(min(best_limit, 4096));
    buttons.extend_from_slice(&graph.initial_buttons);
    if buttons.len() > best_limit || buttons.len() > input.t_limit {
        return Vec::new();
    }
    for &k in order {
        if unsafe { pc.cost_unchecked(k, state) } >= INF_COST {
            return Vec::new();
        }
        loop {
            let op = unsafe { pc.next_op_unchecked(k, state) };
            if op == OP_NONE {
                break;
            }
            let ns = unsafe { pc.next_state_unchecked(k, state) };
            if ns == u16::MAX {
                return Vec::new();
            }
            emit_graph_op(&mut buttons, op, q, graph.macro_of_macro.as_ref());
            state = ns as usize;
            if buttons.len() > best_limit || buttons.len() > input.t_limit {
                return Vec::new();
            }
        }
        buttons.push(OP_S);
        let Some(ns) = append_route_to_target(
            routes,
            k,
            state,
            q,
            graph.macro_of_macro.as_ref(),
            &mut buttons,
        ) else {
            return Vec::new();
        };
        state = ns;
        buttons.push(OP_S);
        if buttons.len() > best_limit || buttons.len() > input.t_limit {
            return Vec::new();
        }
    }
    buttons
}

struct SaContext {
    graph: MacroGraph,
    routes: RouteTables,
    pc: DeliveryPrecomp,
    q: SmallQ,
    #[cfg(feature = "local")]
    kind: u8,
    #[cfg(feature = "local")]
    mom_used: bool,
    best_order: Vec<usize>,
    best_cost: u32,
    cur_order: Vec<usize>,
    cur_cost: u32,
    cur_prefix_state: Vec<u16>,
    cur_prefix_cost: Vec<u32>,
    scratch_prefix_state: Vec<u16>,
    scratch_prefix_cost: Vec<u32>,
    rng: XorShift64,
    stagnant: usize,
}

struct ProbeSaContext {
    pool_idx: usize,
    greedy_len: usize,
    ctx: SaContext,
}

fn probe_context_cmp(
    a: &ProbeSaContext,
    b: &ProbeSaContext,
    q_pool: &[GreedyQCandidate],
) -> Ordering {
    a.ctx
        .best_cost
        .cmp(&b.ctx.best_cost)
        .then_with(|| a.greedy_len.cmp(&b.greedy_len))
        .then_with(|| q_pool_cmp(&q_pool[a.pool_idx], &q_pool[b.pool_idx]))
}

fn probe_context_bucket_limit(item: &GreedyQCandidate) -> usize {
    sa_shape_bucket_limit(item) * 3
}

fn probe_context_coverage_bucket_limit(item: &GreedyQCandidate) -> usize {
    max(1, (probe_context_bucket_limit(item) + 1) / 2)
}

fn diversify_probe_contexts(
    contexts: &mut Vec<ProbeSaContext>,
    q_pool: &[GreedyQCandidate],
    keep: usize,
) {
    contexts.sort_by(|a, b| probe_context_cmp(a, b, q_pool));
    if contexts.len() <= 1 {
        return;
    }

    let mut selected = Vec::with_capacity(min(keep, contexts.len()));
    let mut deferred = Vec::with_capacity(contexts.len());
    let mut shape_counts: BTreeMap<(usize, usize, usize), usize> = BTreeMap::new();
    let mut coverage_counts: BTreeMap<((usize, usize, usize), u8), usize> = BTreeMap::new();

    for item in contexts.drain(..) {
        let pool_item = &q_pool[item.pool_idx];
        let shape_bucket = sa_shape_bucket(pool_item.q.as_slice());
        let coverage_bucket = (shape_bucket, pool_item.coverage_class);
        let shape_cnt = *shape_counts.get(&shape_bucket).unwrap_or(&0);
        let coverage_cnt = *coverage_counts.get(&coverage_bucket).unwrap_or(&0);
        if selected.len() < keep
            && shape_cnt < probe_context_bucket_limit(pool_item)
            && coverage_cnt < probe_context_coverage_bucket_limit(pool_item)
        {
            shape_counts.insert(shape_bucket, shape_cnt + 1);
            coverage_counts.insert(coverage_bucket, coverage_cnt + 1);
            selected.push(item);
        } else {
            deferred.push(item);
        }
    }

    for item in deferred {
        if selected.len() >= keep {
            break;
        }
        selected.push(item);
    }
    *contexts = selected;
}

fn prepare_sa_context(
    input: &Input,
    grid: &Grid,
    item: &mut GreedyQCandidate,
    _kind: u8,
    best_limit: usize,
    timer: &Timer,
    deadline: f64,
    seed_salt: u64,
    allow_power: bool,
) -> Option<SaContext> {
    if timer.elapsed() >= deadline {
        return None;
    }
    let (graph, routes) = if let Some(prepared) = item.prepared.take() {
        (*prepared.graph, *prepared.routes)
    } else {
        let graph = MacroGraph::new(grid, item.q.as_slice(), allow_power);
        let mut basket_targets = vec![0u16; input.m];
        basket_targets[..input.m].copy_from_slice(&input.basket_pos[..input.m]);
        let mut ball_sources = vec![0u16; input.m];
        ball_sources[..input.m].copy_from_slice(&input.ball_pos[..input.m]);
        let routes = build_routes_to_targets(&graph, &basket_targets, &ball_sources, best_limit);
        (graph, routes)
    };
    if timer.elapsed() >= deadline {
        return None;
    }
    let pc = build_delivery_precomp(input, &graph, &routes, best_limit);
    if timer.elapsed() >= deadline {
        return None;
    }

    let natural: Vec<usize> = (0..input.m).collect();
    let greedy = greedy_order_precomp(input, &graph, &pc);
    let mut initials = Vec::new();
    initials.push(natural.clone());
    initials.push(greedy.clone());
    if item.order.len() == input.m {
        initials.push(item.order.clone());
    }
    let mut rev = natural.clone();
    rev.reverse();
    initials.push(rev);

    let mut rng = XorShift64::new(
        123456789
            ^ ((input.n as u64) << 48)
            ^ ((input.m as u64) << 32)
            ^ ((input.t_limit as u64) << 8)
            ^ seed_salt,
    );
    for r in 0..4 {
        let mut p = greedy.clone();
        for _ in 0..=r {
            mutate_order(&mut p, &mut rng);
        }
        initials.push(p);
    }
    for _ in 0..5 {
        let mut p = natural.clone();
        rng.shuffle_vec(&mut p);
        initials.push(p);
    }

    let mut best_cost = INF_COST;
    let mut best_order = greedy;
    let mut seen = FxHashSet::default();
    seen.reserve(initials.len() * 2 + 10);
    for p in initials {
        if p.len() != input.m {
            continue;
        }
        let h = hash_order(&p);
        if !seen.insert(h) {
            continue;
        }
        let cost = eval_order_with_precomp(&pc, &graph, &p).0;
        if cost < best_cost {
            best_cost = cost;
            best_order = p;
        }
    }
    if best_cost >= INF_COST {
        return None;
    }
    let mut cur_prefix_state = vec![0u16; input.m + 1];
    let mut cur_prefix_cost = vec![INF_COST; input.m + 1];
    let prefix_cost = fill_order_prefix(
        &pc,
        &graph,
        &best_order,
        &mut cur_prefix_state,
        &mut cur_prefix_cost,
    );
    if prefix_cost >= INF_COST {
        return None;
    }

    Some(SaContext {
        #[cfg(feature = "local")]
        mom_used: graph.has_macro_of_macro(),
        graph,
        routes,
        pc,
        q: item.q,
        #[cfg(feature = "local")]
        kind: _kind,
        best_order: best_order.clone(),
        best_cost: prefix_cost,
        cur_order: best_order,
        cur_cost: prefix_cost,
        cur_prefix_state,
        cur_prefix_cost,
        scratch_prefix_state: vec![0u16; input.m + 1],
        scratch_prefix_cost: vec![INF_COST; input.m + 1],
        rng,
        stagnant: 0,
    })
}

fn run_order_sa_context(
    _input: &Input,
    ctx: &mut SaContext,
    timer: &Timer,
    deadline: f64,
    prune_limit: Option<u32>,
) -> bool {
    let sa_start = timer.elapsed();
    let sa_end = deadline;
    let sa_checkpoint = sa_start + (sa_end - sa_start).max(0.0) * SA_EARLY_PRUNE_CHECK_RATIO;
    let mut early_prune_checked = false;
    let temp0 = (ctx.best_cost as f64 * 0.06).max(8.0);
    let temp1 = 0.02;
    let mut iter = 0usize;
    let mut temp = temp0;
    loop {
        if (iter & 255) == 0 {
            let now = timer.elapsed();
            if now >= sa_end {
                break;
            }
            if let Some(limit) = prune_limit {
                if !early_prune_checked && now >= sa_checkpoint {
                    early_prune_checked = true;
                    if ctx.best_cost > limit {
                        #[cfg(feature = "local")]
                        SA_EARLY_PRUNE_COUNT.fetch_add(1, AtomicOrdering::Relaxed);
                        return true;
                    }
                }
            }
            let mut prog = (now - sa_start) / (sa_end - sa_start).max(1e-9);
            prog = prog.clamp(0.0, 1.0);
            temp = temp0 * (temp1 / temp0).powf(prog);
        }
        let Some(record) = mutate_order_in_place(&mut ctx.cur_order, &mut ctx.rng) else {
            iter += 1;
            continue;
        };
        let dirty_start = record.dirty_start();
        let next_cost = eval_order_suffix_with_precomp(
            &ctx.pc,
            &ctx.cur_order,
            dirty_start,
            &ctx.cur_prefix_state,
            &ctx.cur_prefix_cost,
            &mut ctx.scratch_prefix_state,
            &mut ctx.scratch_prefix_cost,
        );
        let delta = next_cost as i64 - ctx.cur_cost as i64;
        let accept = delta <= 0
            || (next_cost < INF_COST && ctx.rng.uniform01() < (-(delta as f64) / temp).exp());
        if accept {
            ctx.cur_cost = next_cost;
            ctx.cur_prefix_state[dirty_start + 1..=_input.m]
                .copy_from_slice(&ctx.scratch_prefix_state[dirty_start + 1..=_input.m]);
            ctx.cur_prefix_cost[dirty_start + 1..=_input.m]
                .copy_from_slice(&ctx.scratch_prefix_cost[dirty_start + 1..=_input.m]);
            if ctx.cur_cost < ctx.best_cost {
                ctx.best_cost = ctx.cur_cost;
                ctx.best_order = ctx.cur_order.clone();
                ctx.stagnant = 0;
            } else {
                ctx.stagnant += 1;
            }
        } else {
            rollback_order_mutation(&mut ctx.cur_order, record);
            ctx.stagnant += 1;
        }
        if ctx.stagnant > 12000 {
            ctx.cur_order = ctx.best_order.clone();
            ctx.cur_cost = ctx.best_cost;
            fill_order_prefix(
                &ctx.pc,
                &ctx.graph,
                &ctx.cur_order,
                &mut ctx.cur_prefix_state,
                &mut ctx.cur_prefix_cost,
            );
            ctx.stagnant = 0;
        }
        iter += 1;
    }
    false
}

fn result_from_sa_context(
    input: &Input,
    ctx: &SaContext,
    best_limit: usize,
) -> Option<MacroSolveResult> {
    if ctx.best_cost >= best_limit as u32 {
        return None;
    }
    let buttons = build_buttons_for_order(
        input,
        &ctx.graph,
        &ctx.routes,
        &ctx.pc,
        ctx.q.as_slice(),
        &ctx.best_order,
        best_limit,
    );
    if buttons.is_empty() || buttons.len() >= best_limit {
        return None;
    }
    Some(MacroSolveResult {
        buttons,
        q: ctx.q,
        order: ctx.best_order.clone(),
        prepared: None,
        #[cfg(feature = "local")]
        mom_used: ctx.mom_used,
    })
}

fn gen_raw_macro_candidates(
    min_len: usize,
    max_len: usize,
    excluded: &FxHashSet<SmallQ>,
) -> Vec<SmallQ> {
    let mut out = Vec::new();
    let max_len = min(max_len, MAX_MACRO_Q_LEN);
    if min_len > max_len {
        return out;
    }
    let mut ops = [0u8; MAX_MACRO_Q_LEN];
    for len in min_len..=max_len {
        gen_structured_raw_candidates_of_len(len, &mut ops, excluded, &mut out);
    }
    out
}

fn gen_structured_raw_candidates_of_len(
    len: usize,
    ops: &mut [u8; MAX_MACRO_Q_LEN],
    excluded: &FxHashSet<SmallQ>,
    out: &mut Vec<SmallQ>,
) {
    for prefix in 0..len {
        for op in ops.iter_mut().take(prefix) {
            *op = OP_F;
        }

        ops[prefix] = OP_R;
        dfs_structured_raw_after_turn(len, prefix + 1, ops, excluded, out);

        ops[prefix] = OP_L;
        dfs_structured_raw_after_turn(len, prefix + 1, ops, excluded, out);

        if prefix + 2 <= len {
            ops[prefix] = OP_L;
            ops[prefix + 1] = OP_L;
            dfs_structured_raw_after_turn(len, prefix + 2, ops, excluded, out);
        }
    }
}

fn dfs_structured_raw_after_turn(
    len: usize,
    pos: usize,
    ops: &mut [u8; MAX_MACRO_Q_LEN],
    excluded: &FxHashSet<SmallQ>,
    out: &mut Vec<SmallQ>,
) {
    for op in ops.iter_mut().take(len).skip(pos) {
        *op = OP_F;
    }
    emit_structured_raw_candidate(len, ops, excluded, out);

    let rem = len - pos;
    for run in 1..rem {
        for op in ops.iter_mut().skip(pos).take(run) {
            *op = OP_F;
        }
        ops[pos + run] = OP_R;
        dfs_structured_raw_after_turn(len, pos + run + 1, ops, excluded, out);

        ops[pos + run] = OP_L;
        dfs_structured_raw_after_turn(len, pos + run + 1, ops, excluded, out);
    }
}

fn emit_structured_raw_candidate(
    len: usize,
    ops: &[u8; MAX_MACRO_Q_LEN],
    excluded: &FxHashSet<SmallQ>,
    out: &mut Vec<SmallQ>,
) {
    let Some(q) = SmallQ::from_slice(&ops[..len]) else {
        return;
    };
    if excluded.contains(&q) {
        return;
    }
    let qs = q.as_slice();
    if is_sa_decile_bad_macro(qs)
        || is_extra_sa_decile_bad_macro(qs)
        || is_sa_bad_only_long_tail_macro(qs)
    {
        return;
    }
    out.push(q);
}

fn gen_frequent_route_candidates(
    basic_ops: &[u8],
    min_len: usize,
    max_len: usize,
    keep: usize,
) -> Vec<SmallQ> {
    struct Item {
        q: SmallQ,
        score: i32,
    }

    let n = basic_ops.len();
    let upper = min(max_len, n);
    if upper < min_len {
        return Vec::new();
    }

    let mut s_prefix = vec![0usize; n + 1];
    for (i, &op) in basic_ops.iter().enumerate() {
        s_prefix[i + 1] = s_prefix[i] + (op == OP_S) as usize;
    }

    let mut counts: FxHashMap<SmallQ, i32> = FxHashMap::default();
    counts.reserve(n.saturating_mul(upper - min_len + 1));
    for len in min_len..=upper {
        for st in 0..=n - len {
            if s_prefix[st + len] != s_prefix[st] {
                continue;
            }
            let q = &basic_ops[st..st + len];
            if !is_structured_regex_macro(q) {
                continue;
            }
            if is_sa_decile_bad_macro(q)
                || is_extra_sa_decile_bad_macro(q)
                || is_sa_bad_only_long_tail_macro(q)
            {
                continue;
            }
            let Some(key) = SmallQ::from_slice(q) else {
                continue;
            };
            *counts.entry(key).or_insert(0) += 1;
        }
    }

    let mut items = Vec::new();
    for (q, cnt) in counts {
        if cnt < 2 {
            continue;
        }
        let qs = q.as_slice();
        let len = q.len() as i32;
        let turns = qs.iter().filter(|&&op| op != OP_F).count() as i32;
        let mut score = cnt * (len - 1) - (len + 2) + turns * 45 + len * 2;
        if qs[0] != OP_F {
            score += 180;
        }
        if *qs.last().unwrap() != OP_F {
            score += 60;
        }
        if turns >= 2 {
            score += 80;
        }
        if score > 0 {
            items.push(Item { q, score });
        }
    }
    items.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.q.len().cmp(&a.q.len()))
            .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
    });
    if items.len() > keep {
        items.truncate(keep);
    }
    items.into_iter().map(|item| item.q).collect()
}

fn reconstruct_oriented_route_prefix(
    grid: &Grid,
    costs: &[u32],
    start_state: usize,
    max_len: usize,
) -> Vec<u8> {
    let mut state = start_state;
    let mut out = Vec::with_capacity(max_len);
    while costs[state] > 0 && costs[state] < INF_COST && out.len() < max_len {
        let cur = costs[state];
        let cell = state >> 2;
        let dir = state & 3;

        let fcell = grid.next(cell, dir);
        let fs = fcell * 4 + dir;
        if fcell != cell && costs[fs].saturating_add(1) == cur {
            out.push(OP_F);
            state = fs;
            continue;
        }

        let rs = cell * 4 + ((dir + 1) & 3);
        let ls = cell * 4 + ((dir + 3) & 3);
        if costs[rs].saturating_add(1) == cur {
            out.push(OP_R);
            state = rs;
        } else if costs[ls].saturating_add(1) == cur {
            out.push(OP_L);
            state = ls;
        } else {
            break;
        }
    }
    out
}

#[derive(Clone, Copy)]
struct RouteCoverageItem {
    state: u16,
    target_idx: u16,
    weight: u8,
    base: u32,
}

#[derive(Clone, Copy)]
struct MacroScore {
    score: i64,
    coverage_class: u8,
}

const COVERAGE_CLASS_DELIVER: u8 = 0;
const COVERAGE_CLASS_SWITCH: u8 = 1;
const COVERAGE_CLASS_ROUTE: u8 = 2;
const COVERAGE_CLASS_PRIORITY: u8 = 3;

fn macro_score_min() -> MacroScore {
    MacroScore {
        score: i64::MIN / 4,
        coverage_class: COVERAGE_CLASS_DELIVER,
    }
}

fn build_route_target_coverage_profile(
    input: &Input,
    grid: &Grid,
    oriented_costs: &[Vec<u32>],
) -> Vec<RouteCoverageItem> {
    let mut counts = FxHashMap::<(u16, u16), u16>::default();
    counts.reserve(input.m * 32);
    for k in 0..input.m {
        let ball = input.ball_pos[k] as usize;
        let costs = &oriented_costs[k];
        for dir in 0..4 {
            let start = ball * 4 + dir;
            let base = costs[start];
            if base >= INF_COST || base < 6 {
                continue;
            }
            let path = reconstruct_oriented_route_prefix(grid, costs, start, 48);
            if path.len() < 6 {
                continue;
            }
            let mut state = start;
            for (step, &op) in path.iter().enumerate() {
                let cell = state >> 2;
                let dir = state & 3;
                state = match op {
                    OP_F => grid.next(cell, dir) * 4 + dir,
                    OP_R => cell * 4 + ((dir + 1) & 3),
                    OP_L => cell * 4 + ((dir + 3) & 3),
                    _ => state,
                };
                let rem = costs[state];
                if rem >= INF_COST {
                    continue;
                }
                if step + 2 >= path.len() || step < 2 || rem <= 2 {
                    continue;
                }
                let key = (state as u16, k as u16);
                let entry = counts.entry(key).or_insert(0);
                if *entry < u16::MAX {
                    *entry += 1;
                }
            }
        }
    }

    let mut items = Vec::<RouteCoverageItem>::with_capacity(counts.len());
    for ((state, target_idx), count) in counts {
        let base = oriented_costs[target_idx as usize][state as usize];
        if base < INF_COST {
            items.push(RouteCoverageItem {
                state,
                target_idx,
                weight: min(count, 8) as u8,
                base,
            });
        }
    }
    items.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then_with(|| a.base.cmp(&b.base))
            .then_with(|| a.target_idx.cmp(&b.target_idx))
            .then_with(|| a.state.cmp(&b.state))
    });
    if items.len() > 160 {
        items.truncate(160);
    }
    items
}

fn add_pair_route_prefix_item(
    mp: &mut FxHashMap<SmallQ, i32>,
    path: &[u8],
    len: usize,
    full_route: bool,
) {
    if len == 0 || len > path.len() {
        return;
    }
    let q = &path[..len];
    if !is_structured_regex_macro(q) {
        return;
    }
    if is_sa_decile_bad_macro(q)
        || is_extra_sa_decile_bad_macro(q)
        || is_sa_bad_only_long_tail_macro(q)
    {
        return;
    }
    let Some(sq) = SmallQ::from_slice(q) else {
        return;
    };
    let feat = macro_features(q);
    let mut score = (len as i32 - 1) * 12 + feat.turn_count as i32 * 45;
    if full_route {
        score += 180;
    }
    if q[0] != OP_F {
        score += 70;
    }
    if *q.last().unwrap() != OP_F {
        score += 45;
    }
    if feat.turn_count >= 2 {
        score += 80;
    }
    *mp.entry(sq).or_insert(0) += score;
}

fn gen_pair_route_prefix_candidates(
    input: &Input,
    grid: &Grid,
    oriented_costs: &[Vec<u32>],
    min_len: usize,
    max_len: usize,
    keep: usize,
) -> Vec<SmallQ> {
    struct Item {
        q: SmallQ,
        score: i32,
    }

    let mut mp: FxHashMap<SmallQ, i32> = FxHashMap::default();
    mp.reserve(input.m * 64);
    for (k, costs) in oriented_costs.iter().enumerate().take(input.m) {
        let ball = input.ball_pos[k] as usize;
        for dir in 0..4 {
            let start_state = ball * 4 + dir;
            if costs[start_state] < min_len as u32 || costs[start_state] >= INF_COST {
                continue;
            }
            let path = reconstruct_oriented_route_prefix(grid, costs, start_state, max_len);
            if path.len() < min_len {
                continue;
            }

            for len in [13usize, 14, 15, 16, 18, 20, 24, 28, 32, 36, 42] {
                if (min_len..=path.len()).contains(&len) {
                    add_pair_route_prefix_item(&mut mp, &path, len, false);
                }
            }
            for len in min_len..=path.len() {
                let turn_boundary = len == path.len() || path[len] != OP_F || path[len - 1] != OP_F;
                if turn_boundary {
                    add_pair_route_prefix_item(&mut mp, &path, len, len == path.len());
                }
            }
        }
    }

    let mut items: Vec<Item> = mp.into_iter().map(|(q, score)| Item { q, score }).collect();
    items.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.q.len().cmp(&a.q.len()))
            .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
    });
    if items.len() > keep {
        items.truncate(keep);
    }
    items.into_iter().map(|item| item.q).collect()
}

#[derive(Clone, Copy)]
struct MacroBuilder {
    ops: [u8; MAX_MACRO_Q_LEN],
    len: usize,
}

impl MacroBuilder {
    fn new() -> Self {
        Self {
            ops: [OP_NONE; MAX_MACRO_Q_LEN],
            len: 0,
        }
    }

    fn push_op(&mut self, op: u8) -> bool {
        if self.len >= MAX_MACRO_Q_LEN {
            return false;
        }
        self.ops[self.len] = op;
        self.len += 1;
        true
    }

    fn push_f_run(&mut self, run: usize) -> bool {
        if self.len + run > MAX_MACRO_Q_LEN {
            return false;
        }
        for _ in 0..run {
            self.ops[self.len] = OP_F;
            self.len += 1;
        }
        true
    }

    fn finish(self) -> Option<SmallQ> {
        if self.len == 0 {
            return None;
        }
        Some(SmallQ {
            len: self.len as u8,
            ops: self.ops,
        })
    }
}

fn add_priority_candidate(
    priority: &mut Vec<SmallQ>,
    used: &mut FxHashSet<SmallQ>,
    q: SmallQ,
) {
    let qs = q.as_slice();
    if qs.is_empty()
        || !is_structured_regex_macro(qs)
        || is_sa_decile_bad_macro(qs)
        || is_extra_sa_decile_bad_macro(qs)
        || is_sa_bad_only_long_tail_macro(qs)
        || !used.insert(q)
    {
        return;
    }
    priority.push(q);
}

fn mirror_turn(op: u8) -> u8 {
    if op == OP_R { OP_L } else { OP_R }
}

fn turns_match_or_mirror(actual: &[u8], pattern: &[u8]) -> bool {
    if actual.len() != pattern.len() {
        return false;
    }
    let same = actual.iter().zip(pattern).all(|(&a, &b)| a == b);
    let mirrored = actual
        .iter()
        .zip(pattern)
        .all(|(&a, &b)| a == mirror_turn(b));
    same || mirrored
}

fn decompose_start_f(
    q: &[u8],
    runs: &mut [usize; 8],
    turns: &mut [u8; 8],
) -> Option<(usize, usize)> {
    let mut i = 0usize;
    let mut r = 0usize;
    let mut t = 0usize;
    while i < q.len() {
        if q[i] != OP_F || r >= runs.len() {
            return None;
        }
        let st = i;
        while i < q.len() && q[i] == OP_F {
            i += 1;
        }
        runs[r] = i - st;
        r += 1;
        if i == q.len() {
            break;
        }
        if (q[i] != OP_R && q[i] != OP_L) || t >= turns.len() {
            return None;
        }
        turns[t] = q[i];
        t += 1;
        i += 1;
    }
    Some((r, t))
}

fn decompose_start_turn(
    q: &[u8],
    turns: &mut [u8; 8],
    runs: &mut [usize; 8],
) -> Option<(usize, usize)> {
    let mut i = 0usize;
    let mut t = 0usize;
    let mut r = 0usize;
    while i < q.len() {
        if (q[i] != OP_R && q[i] != OP_L) || t >= turns.len() {
            return None;
        }
        turns[t] = q[i];
        t += 1;
        i += 1;
        if i == q.len() {
            break;
        }
        if q[i] != OP_F || r >= runs.len() {
            return None;
        }
        let st = i;
        while i < q.len() && q[i] == OP_F {
            i += 1;
        }
        runs[r] = i - st;
        r += 1;
    }
    Some((t, r))
}

fn build_run_turn_pattern(runs: &[usize], turns: &[u8], mirror: bool) -> Option<SmallQ> {
    if turns.len() > runs.len() {
        return None;
    }
    let mut q = MacroBuilder::new();
    for (i, &run) in runs.iter().enumerate() {
        if !q.push_f_run(run) {
            return None;
        }
        if i < turns.len() {
            let turn = if mirror { mirror_turn(turns[i]) } else { turns[i] };
            if !q.push_op(turn) {
                return None;
            }
        }
    }
    q.finish()
}

fn add_run_turn_pattern(
    priority: &mut Vec<SmallQ>,
    used: &mut FxHashSet<SmallQ>,
    runs: &[usize],
    turns: &[u8],
) {
    if let Some(q) = build_run_turn_pattern(runs, turns, false) {
        add_priority_candidate(priority, used, q);
    }
    if let Some(q) = build_run_turn_pattern(runs, turns, true) {
        add_priority_candidate(priority, used, q);
    }
}

fn build_start_turn_pattern(runs: &[usize], turns: &[u8], mirror: bool) -> Option<SmallQ> {
    let mut q = MacroBuilder::new();
    for (i, &turn) in turns.iter().enumerate() {
        let turn = if mirror { mirror_turn(turn) } else { turn };
        if !q.push_op(turn) {
            return None;
        }
        if i < runs.len() {
            if !q.push_f_run(runs[i]) {
                return None;
            }
        }
    }
    q.finish()
}

fn add_start_turn_pattern(
    priority: &mut Vec<SmallQ>,
    used: &mut FxHashSet<SmallQ>,
    runs: &[usize],
    turns: &[u8],
) {
    if let Some(q) = build_start_turn_pattern(runs, turns, false) {
        add_priority_candidate(priority, used, q);
    }
    if let Some(q) = build_start_turn_pattern(runs, turns, true) {
        add_priority_candidate(priority, used, q);
    }
}

fn add_mined_macro_priority(priority: &mut Vec<SmallQ>, used: &mut FxHashSet<SmallQ>) {
    for a in 1..=3 {
        add_run_turn_pattern(priority, used, &[a, 5, 5], &[OP_L, OP_R, OP_R]);
    }
    for b in 6..=8 {
        add_run_turn_pattern(priority, used, &[1, b, 5], &[OP_L, OP_R, OP_R]);
        add_run_turn_pattern(priority, used, &[1, 5, b], &[OP_L, OP_R, OP_R]);
    }
    for a in 3..=12 {
        for b in 3..=8 {
            add_run_turn_pattern(priority, used, &[a, b, 1], &[OP_L, OP_R, OP_R]);
        }
    }
    for a in 3..=4 {
        for b in 9..=12 {
            add_run_turn_pattern(priority, used, &[a, b, 1], &[OP_L, OP_R, OP_R]);
        }
    }
    for a in 6..=8 {
        add_run_turn_pattern(priority, used, &[a, 3, 2], &[OP_L, OP_R, OP_R]);
        add_run_turn_pattern(priority, used, &[a, 3, 3], &[OP_L, OP_R, OP_R]);
        add_run_turn_pattern(priority, used, &[a, 4, 2], &[OP_L, OP_R, OP_R]);
    }

    add_run_turn_pattern(priority, used, &[1, 2, 2, 2, 1], &[OP_L, OP_R, OP_L, OP_R]);
    add_run_turn_pattern(priority, used, &[2, 2, 2, 2], &[OP_L, OP_R, OP_L, OP_R]);
    add_run_turn_pattern(priority, used, &[1, 3, 1, 3], &[OP_L, OP_R, OP_L, OP_R]);
    add_run_turn_pattern(
        priority,
        used,
        &[1, 1, 1, 1, 1, 1],
        &[OP_L, OP_R, OP_L, OP_R, OP_L, OP_R],
    );
    add_run_turn_pattern(priority, used, &[5, 5], &[OP_L, OP_L]);
    add_run_turn_pattern(priority, used, &[1, 5, 4], &[OP_L, OP_L]);
    add_run_turn_pattern(priority, used, &[2, 5, 3], &[OP_L, OP_L]);
    for a in 6..=8 {
        add_start_turn_pattern(priority, used, &[a, 1], &[OP_L, OP_L, OP_L]);
        add_start_turn_pattern(priority, used, &[a, 2], &[OP_L, OP_L, OP_L]);
    }
    add_start_turn_pattern(priority, used, &[2, 2, 2, 2], &[OP_L, OP_R, OP_L, OP_R]);
}

fn mined_lane_arc_class(q: &[u8]) -> u8 {
    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    let Some((r, t)) = decompose_start_f(q, &mut runs, &mut turns) else {
        return 0;
    };
    if r != 3 || t != 3 || !turns_match_or_mirror(&turns[..3], &[OP_L, OP_R, OP_R]) {
        return 0;
    }
    let (a, b, c) = (runs[0], runs[1], runs[2]);
    if (1..=3).contains(&a) && b == 5 && c == 5 {
        return 1;
    }
    if a == 1 && ((6..=8).contains(&b) && c == 5 || b == 5 && (6..=8).contains(&c)) {
        return 1;
    }
    if c == 1 && (3..=12).contains(&a) && (3..=8).contains(&b) {
        return 2;
    }
    if c == 1 && (3..=4).contains(&a) && (9..=12).contains(&b) {
        return 2;
    }
    if (6..=8).contains(&a) && b == 3 && (2..=3).contains(&c) {
        return 2;
    }
    if (6..=8).contains(&a) && b == 4 && c == 2 {
        return 2;
    }
    0
}

fn is_mined_lane_arc(q: &[u8]) -> bool {
    mined_lane_arc_class(q) != 0
}

fn sa_mined_lane_arc_bonus(q: &[u8]) -> i32 {
    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    let Some((r, t)) = decompose_start_f(q, &mut runs, &mut turns) else {
        return 0;
    };
    if r != 3 || t != 3 || !turns_match_or_mirror(&turns[..3], &[OP_L, OP_R, OP_R]) {
        return 0;
    }
    let (a, b, c) = (runs[0], runs[1], runs[2]);
    if c == 1 {
        if (6..=8).contains(&a) && b == 4 {
            return 48;
        }
        if a == 4 && (6..=8).contains(&b) {
            return 45;
        }
        if a == 3 && (6..=8).contains(&b) {
            return 42;
        }
        if (6..=8).contains(&a) && b == 3 {
            return 40;
        }
        if (6..=8).contains(&a) && (5..=8).contains(&b) {
            return 36;
        }
        if (2..=5).contains(&a) && (6..=12).contains(&b) {
            return 34;
        }
        if (3..=5).contains(&a) && (3..=5).contains(&b) {
            return 32;
        }
        if (6..=8).contains(&a) && (9..=12).contains(&b) {
            return 26;
        }
        if (1..=12).contains(&a) && (3..=12).contains(&b) {
            return 20;
        }
    } else if c == 2 {
        if (6..=8).contains(&a) && (3..=5).contains(&b) {
            return 34;
        }
        if (1..=5).contains(&a) && (6..=12).contains(&b) {
            return 30;
        }
        if (3..=8).contains(&a) && (3..=8).contains(&b) {
            return 24;
        }
    } else if c == 3 && (3..=8).contains(&a) && (3..=9).contains(&b) {
        return 18;
    }
    if mined_lane_arc_class(q) != 0 {
        return 12;
    }
    0
}

fn sa_mined_turn_start_bonus(q: &[u8]) -> i32 {
    let mut turns = [0u8; 8];
    let mut runs = [0usize; 8];
    let Some((t, r)) = decompose_start_turn(q, &mut turns, &mut runs) else {
        return 0;
    };
    if t == 3
        && r == 2
        && (6..=8).contains(&runs[0])
        && (1..=2).contains(&runs[1])
        && turns_match_or_mirror(&turns[..3], &[OP_L, OP_L, OP_L])
    {
        return 24;
    }
    if t == 3
        && r == 2
        && (3..=8).contains(&runs[0])
        && (1..=3).contains(&runs[1])
        && turns_match_or_mirror(&turns[..3], &[OP_L, OP_L, OP_L])
    {
        return 14;
    }
    0
}

fn sa_mined_same_turn_arc_bonus(q: &[u8]) -> i32 {
    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    let Some((r, t)) = decompose_start_f(q, &mut runs, &mut turns) else {
        return 0;
    };
    if r == 3
        && t == 3
        && turns_match_or_mirror(&turns[..3], &[OP_L, OP_L, OP_L])
        && (3..=8).contains(&runs[0])
        && (1..=8).contains(&runs[1])
        && (1..=3).contains(&runs[2])
    {
        return 14;
    }
    0
}

fn sa_mined_q_bonus(kind: u8, q: &[u8]) -> i32 {
    let feat = macro_features(q);
    let mut bonus = 0;
    bonus += sa_mined_lane_arc_bonus(q);
    bonus += sa_mined_turn_start_bonus(q);
    bonus += sa_mined_same_turn_arc_bonus(q);

    if kind == CAND_KIND_PRIORITY {
        bonus += 8;
    } else if kind == CAND_KIND_SHORT && feat.turn_count == 3 && (8..=12).contains(&q.len()) {
        bonus += 5;
    }
    if feat.turn_count == 3 && (9..=17).contains(&q.len()) && (3..=8).contains(&feat.max_run) {
        bonus += 8;
    }
    if feat.turn_count == 4 && (9..=16).contains(&q.len()) && feat.max_run <= 5 {
        bonus += 4;
    }
    if kind == CAND_KIND_FREQ || kind == CAND_KIND_PAIR_ROUTE {
        bonus -= 4;
    }
    if q.len() >= 25 {
        bonus -= 18;
    }
    if feat.max_run >= 13 {
        bonus -= 12;
    }
    if feat.turn_count <= 2 {
        bonus -= 6;
    }
    bonus
}

fn mined_lane_arc_input_bonus(input: &Input, q: &[u8], lane_class: u8) -> i64 {
    if lane_class == 0 {
        return 0;
    }

    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    let Some((r, t)) = decompose_start_f(q, &mut runs, &mut turns) else {
        return 0;
    };
    if r != 3 || t != 3 {
        return 0;
    }

    let n2 = input.n * input.n;
    let dense = input.m * 100 >= n2 * 12;
    let mid_dense = input.m * 100 >= n2 * 8;
    let sparse = input.m * 100 < n2 * 8;
    let short_tpm = input.t_limit < input.m * 100;
    let mid_tpm = input.t_limit < input.m * 200;
    let very_long_tpm = input.t_limit >= input.m * 400;
    let compact_time = input.t_limit < input.n * input.m * 6;
    let long_area_time = input.t_limit >= input.n * input.m * 12;
    let high_mpn = input.m * 2 >= input.n * 3;
    let low_mpn = input.m * 10 < input.n * 7;
    let (a, b, c) = (runs[0], runs[1], runs[2]);

    if short_tpm {
        return 0;
    }
    if input.m >= 28 && input.t_limit > 5000 {
        return 0;
    }

    if lane_class == 1 {
        let mut bonus = 0i64;
        if input.n <= 12 {
            bonus += 250_000;
        } else if input.n <= 15 {
            bonus += 170_000;
        } else if input.n >= 18 {
            bonus -= 120_000;
        }
        if dense {
            bonus += 180_000;
        } else if mid_dense {
            bonus += 80_000;
        } else if sparse {
            bonus -= 60_000;
        }
        if !mid_tpm {
            bonus -= 60_000;
        }
        if compact_time {
            bonus += 100_000;
        }
        if high_mpn {
            bonus += 90_000;
        }
        if (1..=3).contains(&a) && b == 5 && c == 5 {
            bonus += 110_000;
            if a == 1 {
                bonus += 30_000;
            }
        }
        if a == 1 && ((6..=8).contains(&b) && c == 5 || b == 5 && (6..=8).contains(&c)) {
            bonus += 130_000;
        }
        if input.n >= 16 && sparse {
            bonus -= 120_000;
        }
        bonus
    } else {
        let mut bonus = 0i64;
        if input.n >= 18 {
            bonus += 220_000;
        } else if input.n >= 16 {
            bonus += 150_000;
        } else if input.n <= 12 {
            bonus -= 100_000;
        }
        if sparse {
            bonus += 160_000;
        } else if dense {
            bonus -= 110_000;
        } else if !mid_dense {
            bonus += 80_000;
        }
        bonus += 120_000;
        if very_long_tpm {
            bonus += 120_000;
        }
        if long_area_time {
            bonus += 60_000;
        }
        if low_mpn {
            bonus += 60_000;
        }
        if c == 1 && (6..=8).contains(&a) && b == 4 {
            bonus += 180_000;
        } else if c == 1 && (6..=8).contains(&a) && b == 3 {
            bonus += 150_000;
        } else if c == 1
            && (((6..=8).contains(&a) && b == 5) || ((4..=5).contains(&a) && (6..=8).contains(&b)))
        {
            bonus += 110_000;
        } else if (2..=3).contains(&c) && (6..=8).contains(&a) && (3..=4).contains(&b) {
            bonus += 70_000;
        }
        bonus
    }
}

fn is_mined_short_pattern(q: &[u8]) -> bool {
    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    if let Some((r, t)) = decompose_start_f(q, &mut runs, &mut turns) {
        if r == 5
            && t == 4
            && runs[..5] == [1, 2, 2, 2, 1]
            && turns_match_or_mirror(&turns[..4], &[OP_L, OP_R, OP_L, OP_R])
        {
            return true;
        }
        if r == 4
            && t == 4
            && runs[..4] == [2, 2, 2, 2]
            && turns_match_or_mirror(&turns[..4], &[OP_L, OP_R, OP_L, OP_R])
        {
            return true;
        }
        if r == 4
            && t == 4
            && runs[..4] == [1, 3, 1, 3]
            && turns_match_or_mirror(&turns[..4], &[OP_L, OP_R, OP_L, OP_R])
        {
            return true;
        }
        if r == 6
            && t == 6
            && runs[..6] == [1, 1, 1, 1, 1, 1]
            && turns_match_or_mirror(&turns[..6], &[OP_L, OP_R, OP_L, OP_R, OP_L, OP_R])
        {
            return true;
        }
        if r == 2
            && t == 2
            && runs[..2] == [5, 5]
            && turns_match_or_mirror(&turns[..2], &[OP_L, OP_L])
        {
            return true;
        }
        if r == 3
            && t == 2
            && (runs[..3] == [1, 5, 4] || runs[..3] == [2, 5, 3])
            && turns_match_or_mirror(&turns[..2], &[OP_L, OP_L])
        {
            return true;
        }
    }

    let mut runs = [0usize; 8];
    let mut turns = [0u8; 8];
    if let Some((t, r)) = decompose_start_turn(q, &mut turns, &mut runs) {
        if t == 3
            && r == 2
            && (6..=8).contains(&runs[0])
            && (1..=2).contains(&runs[1])
            && turns_match_or_mirror(&turns[..3], &[OP_L, OP_L, OP_L])
        {
            return true;
        }
        if t == 4
            && r == 4
            && runs[..4] == [2, 2, 2, 2]
            && turns_match_or_mirror(&turns[..4], &[OP_L, OP_R, OP_L, OP_R])
        {
            return true;
        }
    }
    false
}

fn is_mined_bad_late_q(kind: u8, q: &[u8]) -> bool {
    let feat = macro_features(q);
    if kind == CAND_KIND_SHORT && q.len() >= 9 && q.len() <= 12 && feat.turn_count >= 5 {
        return true;
    }
    (kind == CAND_KIND_PAIR_ROUTE || kind == CAND_KIND_FREQ)
        && q.len() >= 25
        && feat.turn_count >= 5
}

fn is_mined_bad_late_candidate(item: &CandItem) -> bool {
    is_mined_bad_late_q(item.kind, item.q.as_slice())
}

fn mined_priority_promote_keep(_input: &Input) -> usize {
    96
}

fn mined_short_promote_keep(_input: &Input) -> usize {
    if _input.m <= 15 && _input.t_limit < _input.m * 300 {
        64
    } else {
        0
    }
}

fn should_use_mined_short_bias(input: &Input) -> bool {
    input.m <= 15 && input.t_limit < input.m * 300
}

fn add_three_turn_zigzag(
    priority: &mut Vec<SmallQ>,
    used: &mut FxHashSet<SmallQ>,
    runs: [usize; 3],
    turns: [u8; 3],
) {
    if let Some(q) = build_run_turn_pattern(&runs, &turns, false) {
        add_priority_candidate(priority, used, q);
    }
}

fn add_long_zigzag_priority(priority: &mut Vec<SmallQ>, used: &mut FxHashSet<SmallQ>) {
    for total_len in [20usize, 18, 16] {
        let sum_f = total_len - 3;
        for a in 1..=5 {
            for c in 1..=3 {
                let b = sum_f as isize - a as isize - c as isize;
                if b < 6 {
                    continue;
                }
                for turns in [[OP_R, OP_L, OP_L], [OP_L, OP_R, OP_R]] {
                    add_three_turn_zigzag(priority, used, [a, b as usize, c], turns);
                }
            }
        }
    }
    for total_len in 12usize..=14 {
        let sum_f = total_len - 3;
        for b in 3..=min(8, sum_f - 2) {
            for a in 1..=sum_f - b - 1 {
                let c = sum_f - a - b;
                if c < 1 {
                    continue;
                }
                for turns in [[OP_L, OP_R, OP_R], [OP_R, OP_L, OP_L]] {
                    add_three_turn_zigzag(priority, used, [a, b, c], turns);
                }
            }
        }
    }
}

fn add_short_zigzag_priority(priority: &mut Vec<SmallQ>, used: &mut FxHashSet<SmallQ>) {
    for total_len in 12usize..=16 {
        let sum_f = total_len - 3;
        for b in 3..=min(9, sum_f - 2) {
            for a in 1..=sum_f - b - 1 {
                let c = sum_f - a - b;
                if c < 1 {
                    continue;
                }
                for turns in [[OP_L, OP_R, OP_R], [OP_R, OP_L, OP_L]] {
                    add_three_turn_zigzag(priority, used, [a, b, c], turns);
                }
            }
        }
    }
    for total_len in [20usize, 18] {
        let sum_f = total_len - 3;
        for a in 1..=5 {
            for c in 1..=3 {
                let Some(b) = sum_f.checked_sub(a + c) else {
                    continue;
                };
                if b < 6 {
                    continue;
                }
                for turns in [[OP_R, OP_L, OP_L], [OP_L, OP_R, OP_R]] {
                    add_three_turn_zigzag(priority, used, [a, b, c], turns);
                }
            }
        }
    }
}

fn add_selected_zigzag_priority(
    priority: &mut Vec<SmallQ>,
    used: &mut FxHashSet<SmallQ>,
    short_priority: bool,
) {
    if short_priority {
        add_short_zigzag_priority(priority, used);
    } else {
        add_long_zigzag_priority(priority, used);
    }
}

fn macro_effect_base(grid: &Grid, q: &[u8]) -> Vec<u16> {
    let mut eff = vec![0u16; grid.cell_count * 4];
    for cell in 0..grid.cell_count {
        for dir in 0..4 {
            let mut c = cell;
            let mut d = dir;
            for &op in q {
                if op == OP_F {
                    c = grid.next(c, d);
                } else if op == OP_R {
                    d = (d + 1) & 3;
                } else if op == OP_L {
                    d = (d + 3) & 3;
                }
            }
            eff[cell * 4 + dir] = (c * 4 + d) as u16;
        }
    }
    eff
}

fn build_macro_effects(eff: &[u16], max_level: usize) -> Vec<Vec<u16>> {
    let state_count = eff.len();
    let mut effects = Vec::with_capacity(max_level + 1);
    effects.push(vec![0u16; state_count]);
    effects.push(eff.to_vec());
    for level in 2..=max_level {
        let prev = &effects[level - 1];
        let mut cur = vec![0u16; state_count];
        for state in 0..state_count {
            let mid = prev[state] as usize;
            cur[state] = prev[mid];
        }
        effects.push(cur);
    }
    effects
}

fn oriented_costs_to_targets(grid: &Grid, targets: &[usize]) -> Vec<Vec<u32>> {
    let state_count = grid.cell_count * 4;
    let mut all = Vec::with_capacity(targets.len());
    for &target in targets {
        let mut dist = vec![INF_COST; state_count];
        let mut q = [0usize; MAX_CELLS * 4];
        let mut head = 0usize;
        let mut tail = 0usize;
        for dir in 0..4 {
            let state = target * 4 + dir;
            dist[state] = 0;
            q[tail] = state;
            tail += 1;
        }
        while head < tail {
            let state = q[head];
            head += 1;
            let cell = state >> 2;
            let dir = state & 3;
            let nd = dist[state] + 1;

            let pr = cell * 4 + ((dir + 3) & 3);
            if dist[pr] == INF_COST {
                dist[pr] = nd;
                q[tail] = pr;
                tail += 1;
            }
            let pl = cell * 4 + ((dir + 1) & 3);
            if dist[pl] == INF_COST {
                dist[pl] = nd;
                q[tail] = pl;
                tail += 1;
            }
            if grid.next(cell, dir) == cell {
                let ps = cell * 4 + dir;
                if dist[ps] == INF_COST {
                    dist[ps] = nd;
                    q[tail] = ps;
                    tail += 1;
                }
            }
            let back_dir = (dir + 2) & 3;
            if grid.can_move(cell, back_dir) {
                let prev_cell = grid.next(cell, back_dir);
                let pf = prev_cell * 4 + dir;
                if dist[pf] == INF_COST {
                    dist[pf] = nd;
                    q[tail] = pf;
                    tail += 1;
                }
            }
        }
        all.push(dist);
    }
    all
}

fn build_switch_costs_excluding_self(input: &Input, ball_costs: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let state_count = ball_costs.first().map_or(0, Vec::len);
    let mut out = Vec::with_capacity(input.m);
    for i in 0..input.m {
        let mut costs = vec![INF_COST; state_count];
        for (j, ball_cost) in ball_costs.iter().enumerate().take(input.m) {
            if i == j {
                continue;
            }
            for state in 0..state_count {
                if ball_cost[state] < costs[state] {
                    costs[state] = ball_cost[state];
                }
            }
        }
        out.push(costs);
    }
    out
}

fn count_occurrences(ops: &[u8], q: &[u8]) -> usize {
    let mut cnt = 0usize;
    let n = ops.len();
    let len = q.len();
    if len == 0 {
        return 0;
    }
    let mut i = 0usize;
    while i + len <= n {
        let mut ok = true;
        for z in 0..len {
            if ops[i + z] != q[z] {
                ok = false;
                break;
            }
        }
        if ok {
            cnt += 1;
            i += len;
        } else {
            i += 1;
        }
    }
    cnt
}

#[derive(Default)]
struct MacroFeat {
    f_count: usize,
    max_run: usize,
    turn_count: usize,
}

fn macro_features(q: &[u8]) -> MacroFeat {
    let mut feat = MacroFeat::default();
    let mut cur = 0usize;
    for &op in q {
        if op == OP_F {
            feat.f_count += 1;
            cur += 1;
            feat.max_run = max(feat.max_run, cur);
        } else {
            feat.turn_count += 1;
            cur = 0;
        }
    }
    feat
}

fn score_macro_candidate(
    input: &Input,
    q: &[u8],
    eff: &[u16],
    oriented_costs: &[Vec<u32>],
    switch_costs: &[Vec<u32>],
    route_coverage: &[RouteCoverageItem],
    allow_power: bool,
) -> MacroScore {
    let max_level = min(max_registered_level_for_len(q.len(), allow_power), 3);
    let effects = build_macro_effects(eff, max_level);

    let mut gain_sum = 0i64;
    let mut required_expanded = 0usize;
    let q_len = q.len();
    for (k, costs) in oriented_costs.iter().enumerate().take(input.m) {
        let ball = input.ball_pos[k] as usize;
        let mut best_diff = i64::MIN / 4;
        let mut best_level = 0usize;
        for dir in 0..4 {
            let state = ball * 4 + dir;
            let base = costs[state];
            if base >= INF_COST {
                continue;
            }
            for (level, peff) in effects.iter().enumerate().take(max_level + 1).skip(1) {
                let to = peff[state] as usize;
                let after = costs[to].saturating_add(level as u32);
                if after < INF_COST {
                    let diff = base as i64 - after as i64;
                    if diff > best_diff {
                        best_diff = diff;
                        best_level = level;
                    }
                }
            }
        }
        if best_diff > 0 {
            required_expanded += best_level * q_len;
            if required_expanded > input.t_limit {
                return macro_score_min();
            }
            gain_sum += best_diff;
        }
    }

    let mut switch_gain_sum = 0i64;
    for (i, costs) in switch_costs.iter().enumerate().take(input.m) {
        let basket = input.basket_pos[i] as usize;
        let mut best_diff = i64::MIN / 4;
        let mut best_level = 0usize;
        for dir in 0..4 {
            let state = basket * 4 + dir;
            let base = costs[state];
            if base >= INF_COST {
                continue;
            }
            for (level, peff) in effects.iter().enumerate().take(max_level + 1).skip(1) {
                let to = peff[state] as usize;
                let after = costs[to].saturating_add(level as u32);
                if after < INF_COST {
                    let diff = base as i64 - after as i64;
                    if diff > best_diff {
                        best_diff = diff;
                        best_level = level;
                    }
                }
            }
        }
        if best_diff > 0 {
            required_expanded += best_level * q_len;
            if required_expanded > input.t_limit {
                return macro_score_min();
            }
            switch_gain_sum += best_diff;
        }
    }

    let mut route_cov_score = 0i64;
    for item in route_coverage {
        let state = item.state as usize;
        let costs = &oriented_costs[item.target_idx as usize];
        let base = item.base;
        if base >= INF_COST {
            continue;
        }
        let mut best_diff = i64::MIN / 4;
        for (level, peff) in effects.iter().enumerate().take(max_level + 1).skip(1) {
            let to = peff[state] as usize;
            let after = costs[to].saturating_add(level as u32);
            if after < INF_COST {
                best_diff = max(best_diff, base as i64 - after as i64);
            }
        }
        let w = item.weight as i64;
        if best_diff > 0 {
            route_cov_score += min(best_diff, 10) * w * 18 + w * 4;
        } else if best_diff < 0 && best_diff > i64::MIN / 8 {
            route_cov_score -= min(-best_diff, 6) * w * 8;
        }
    }

    let deliver_score = gain_sum * 1000;
    let switch_score = switch_gain_sum * 1000;
    let coverage_class = if route_cov_score > 0
        && route_cov_score >= deliver_score
        && route_cov_score >= switch_score
    {
        COVERAGE_CLASS_ROUTE
    } else if switch_score > deliver_score {
        COVERAGE_CLASS_SWITCH
    } else {
        COVERAGE_CLASS_DELIVER
    };
    MacroScore {
        score: deliver_score + switch_score + route_cov_score,
        coverage_class,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SmallQ {
    len: u8,
    ops: [u8; MAX_MACRO_Q_LEN],
}

impl SmallQ {
    fn from_slice(q: &[u8]) -> Option<Self> {
        if q.len() > MAX_MACRO_Q_LEN {
            return None;
        }
        let mut ops = [OP_NONE; MAX_MACRO_Q_LEN];
        ops[..q.len()].copy_from_slice(q);
        Some(Self {
            len: q.len() as u8,
            ops,
        })
    }

    fn len(self) -> usize {
        self.len as usize
    }

    fn as_slice(&self) -> &[u8] {
        &self.ops[..self.len as usize]
    }

}

struct CandItem {
    q: SmallQ,
    score: i64,
    kind: u8,
    coverage_class: u8,
}

struct MacroCandidate {
    q: SmallQ,
    kind: u8,
    coverage_class: u8,
}

fn candidate_coverage_class(kind: u8, q: &[u8], score_class: u8) -> u8 {
    if kind == CAND_KIND_PRIORITY || is_mined_lane_arc(q) || is_mined_short_pattern(q) {
        COVERAGE_CLASS_PRIORITY
    } else if kind == CAND_KIND_PAIR_ROUTE {
        COVERAGE_CLASS_ROUTE
    } else {
        score_class
    }
}

fn make_cand_item(q: SmallQ, kind: u8, detail: MacroScore) -> CandItem {
    CandItem {
        q,
        score: detail.score,
        kind,
        coverage_class: candidate_coverage_class(kind, q.as_slice(), detail.coverage_class),
    }
}

fn non_priority_order_score(input: &Input, item: &CandItem) -> i64 {
    let mut score = item.score;
    let q = item.q.as_slice();
    if item.kind == CAND_KIND_FREQ && input.m >= 28 && q.len() >= 20 {
        let feat = macro_features(q);
        score += q.len() as i64 * 900 + feat.turn_count as i64 * 1200;
    }
    if should_use_mined_short_bias(input)
        && item.kind == CAND_KIND_SHORT
        && is_mined_short_pattern(q)
    {
        score += 180_000;
    }
    if is_mined_bad_late_candidate(item) {
        score -= 240_000;
    }
    score
}

fn mined_priority_order_score(_input: &Input, item: &CandItem) -> i64 {
    let mut score = item.score;
    let q = item.q.as_slice();
    let lane_class = mined_lane_arc_class(q);
    if lane_class != 0 {
        score += 400_000;
        score += mined_lane_arc_input_bonus(_input, q, lane_class);
    }
    if should_use_mined_short_bias(_input) && is_mined_short_pattern(q) {
        score += 260_000;
    }
    score
}

fn push_macro_candidate(
    out: &mut Vec<MacroCandidate>,
    used: &mut FxHashSet<SmallQ>,
    item: &CandItem,
) {
    if used.insert(item.q) {
        out.push(MacroCandidate {
            q: item.q,
            kind: item.kind,
            coverage_class: item.coverage_class,
        });
    }
}

fn is_turn_op(op: u8) -> bool {
    op == OP_R || op == OP_L
}

fn is_structured_regex_macro(q: &[u8]) -> bool {
    let mut i = 0usize;
    while i < q.len() && q[i] == OP_F {
        i += 1;
    }
    if i >= q.len() {
        return false;
    }

    if q[i] == OP_L && i + 1 < q.len() && q[i + 1] == OP_L {
        i += 2;
    } else if is_turn_op(q[i]) {
        i += 1;
    } else {
        return false;
    }

    while i < q.len() {
        let run_start = i;
        while i < q.len() && q[i] == OP_F {
            i += 1;
        }
        if i == q.len() {
            return true;
        }
        if i == run_start || !is_turn_op(q[i]) {
            return false;
        }
        i += 1;
    }
    true
}

fn opposite_turn(op: u8) -> u8 {
    if op == OP_R { OP_L } else { OP_R }
}

fn take_f_run(q: &[u8], i: &mut usize) -> usize {
    let st = *i;
    while *i < q.len() && q[*i] == OP_F {
        *i += 1;
    }
    *i - st
}

#[derive(Default)]
struct MacroRunShape {
    run_count: usize,
    turn_count: usize,
    tail_run: usize,
}

fn macro_run_shape(q: &[u8]) -> MacroRunShape {
    let mut shape = MacroRunShape::default();
    let mut cur_run = 0usize;
    for &op in q {
        if op == OP_F {
            cur_run += 1;
        } else {
            if cur_run > 0 {
                shape.run_count += 1;
                cur_run = 0;
            }
            shape.turn_count += 1;
        }
    }
    if cur_run > 0 {
        shape.run_count += 1;
    }
    shape.tail_run = cur_run;
    shape
}

fn is_sa_decile_bad_macro(q: &[u8]) -> bool {
    if q.len() == 2 && q[0] == OP_F && is_turn_op(q[1]) {
        return true;
    }
    if q.len() == 3 && q[0] == OP_F && is_turn_op(q[1]) && q[2] == OP_F {
        return true;
    }
    if q.len() == 8 && q.iter().all(|&op| op == OP_F) {
        return true;
    }
    if (12..=14).contains(&q.len())
        && is_turn_op(q[0])
        && q[1] == OP_F
        && q[2] == q[0]
        && q[3..q.len() - 3].iter().all(|&op| op == OP_F)
        && q[q.len() - 3] == q[0]
        && q[q.len() - 2] == OP_F
        && q[q.len() - 1] == q[0]
    {
        let middle_run = q.len() - 6;
        return (6..=8).contains(&middle_run);
    }
    false
}

fn is_extra_sa_decile_bad_macro(q: &[u8]) -> bool {
    if (6..=7).contains(&q.len()) && q.iter().all(|&op| op == OP_F) {
        return true;
    }

    if (10..=13).contains(&q.len()) && is_turn_op(q[0]) && q[1..].iter().all(|&op| op == OP_F) {
        return true;
    }

    if q.len() >= 7 && is_turn_op(q[0]) {
        let turn = q[0];
        let mut i = 1usize;
        if take_f_run(q, &mut i) != 1 || i >= q.len() || q[i] != turn {
            return false;
        }
        i += 1;
        let mid = take_f_run(q, &mut i);
        if !(2..=5).contains(&mid) || i >= q.len() || q[i] != turn {
            return false;
        }
        i += 1;
        if take_f_run(q, &mut i) != 1 || i >= q.len() || q[i] != turn {
            return false;
        }
        i += 1;
        if i == q.len() {
            return true;
        }
    }

    if q.len() >= 9 && q[0] == OP_F && q[1] == OP_F && is_turn_op(q[2]) {
        let turn = q[2];
        let mut i = 3usize;
        let first = take_f_run(q, &mut i);
        if !(3..=5).contains(&first) || i >= q.len() || q[i] != turn {
            return false;
        }
        i += 1;
        let second = take_f_run(q, &mut i);
        if !(2..=4).contains(&second) || i >= q.len() || q[i] != opposite_turn(turn) {
            return false;
        }
        i += 1;
        let tail = take_f_run(q, &mut i);
        if tail <= 2 && i == q.len() {
            return true;
        }
    }

    false
}

fn is_sa_bad_only_long_tail_macro(q: &[u8]) -> bool {
    let shape = macro_run_shape(q);
    let len = q.len();
    (shape.run_count == 6
        && ((len == 24 && shape.tail_run == 10) || (len == 22 && shape.tail_run >= 9)))
        || (shape.turn_count == 6
            && ((len == 23 && shape.tail_run >= 9) || (len == 24 && shape.tail_run == 10)))
        || (shape.turn_count == 5 && len == 22 && shape.tail_run == 10)
}

fn should_promote_long_freq(input: &Input, basic_ops: &[u8], item: &CandItem) -> bool {
    let q = item.q.as_slice();
    if item.kind != CAND_KIND_FREQ || input.m < 28 || input.t_limit <= 5000 || q.len() < 20 {
        return false;
    }
    if count_occurrences(basic_ops, q) < 3 {
        return false;
    }
    let feat = macro_features(q);
    feat.turn_count >= 2
}

#[cfg(feature = "local")]
fn cand_kind_name(kind: u8) -> &'static str {
    match kind {
        CAND_KIND_SHORT => "short",
        CAND_KIND_FREQ => "freq",
        CAND_KIND_PRIORITY => "priority",
        CAND_KIND_PAIR_ROUTE => "pair_route",
        BEST_SOURCE_INIT => "init",
        BEST_SOURCE_RECOMPRESS => "recompress",
        BEST_SOURCE_FALLBACK => "fallback",
        BEST_SOURCE_SAFE_FALLBACK => "safe_fallback",
        _ => "unknown",
    }
}

fn gen_macro_candidates(
    input: &Input,
    grid: &Grid,
    _dist: &DistMatrix,
    basic_ops: &[u8],
    allow_power: bool,
) -> Vec<MacroCandidate> {
    let mut priority = Vec::new();
    let mut priority_used = FxHashSet::default();
    let short_priority = input.m >= 10 && input.m <= 29 && input.t_limit <= 10000;
    if input.t_limit > 5000 {
        add_selected_zigzag_priority(&mut priority, &mut priority_used, short_priority);
    }
    add_mined_macro_priority(&mut priority, &mut priority_used);
    for a in 1..=5 {
        add_run_turn_pattern(&mut priority, &mut priority_used, &[a], &[OP_R]);
        add_run_turn_pattern(&mut priority, &mut priority_used, &[a, a], &[OP_R]);
    }
    if input.t_limit <= 5000 {
        add_selected_zigzag_priority(&mut priority, &mut priority_used, short_priority);
    }

    let raw_short = gen_raw_macro_candidates(6, 12, &priority_used);
    let extra = gen_frequent_route_candidates(basic_ops, 13, 42, 700);

    let basket_targets: Vec<usize> = (0..input.m).map(|k| input.basket_pos[k] as usize).collect();
    let oriented_costs = oriented_costs_to_targets(grid, &basket_targets);
    let ball_targets: Vec<usize> = (0..input.m).map(|k| input.ball_pos[k] as usize).collect();
    let ball_costs = oriented_costs_to_targets(grid, &ball_targets);
    let switch_costs = build_switch_costs_excluding_self(input, &ball_costs);
    let route_coverage = build_route_target_coverage_profile(input, grid, &oriented_costs);
    let pair_routes = gen_pair_route_prefix_candidates(
        input,
        grid,
        &oriented_costs,
        6,
        32,
        PAIR_ROUTE_PREFIX_KEEP,
    );
    let mut priority_scored = Vec::with_capacity(priority.len());
    let mut scored = Vec::with_capacity(raw_short.len() + extra.len() + pair_routes.len());
    let mut seen = FxHashSet::default();
    seen.reserve((raw_short.len() + extra.len() + pair_routes.len() + priority.len()) * 2 + 100);
    for q in priority {
        let qs = q.as_slice();
        if !seen.insert(q) {
            continue;
        }
        let eff = macro_effect_base(grid, qs);
        let detail = score_macro_candidate(
            input,
            qs,
            &eff,
            &oriented_costs,
            &switch_costs,
            &route_coverage,
            allow_power,
        );
        priority_scored.push(make_cand_item(q, CAND_KIND_PRIORITY, detail));
    }
    for q in raw_short {
        let qs = q.as_slice();
        if !seen.insert(q) {
            continue;
        }
        let eff = macro_effect_base(grid, qs);
        let detail = score_macro_candidate(
            input,
            qs,
            &eff,
            &oriented_costs,
            &switch_costs,
            &route_coverage,
            allow_power,
        );
        scored.push(make_cand_item(q, CAND_KIND_SHORT, detail));
    }
    for q in extra {
        let qs = q.as_slice();
        if !seen.insert(q) {
            continue;
        }
        let eff = macro_effect_base(grid, qs);
        let detail = score_macro_candidate(
            input,
            qs,
            &eff,
            &oriented_costs,
            &switch_costs,
            &route_coverage,
            allow_power,
        );
        scored.push(make_cand_item(q, CAND_KIND_FREQ, detail));
    }
    for q in pair_routes {
        let qs = q.as_slice();
        if !seen.insert(q) {
            continue;
        }
        let eff = macro_effect_base(grid, qs);
        let detail = score_macro_candidate(
            input,
            qs,
            &eff,
            &oriented_costs,
            &switch_costs,
            &route_coverage,
            allow_power,
        );
        scored.push(make_cand_item(q, CAND_KIND_PAIR_ROUTE, detail));
    }
    scored.sort_by(|a, b| {
        non_priority_order_score(input, b)
            .cmp(&non_priority_order_score(input, a))
            .then_with(|| a.q.len().cmp(&b.q.len()))
            .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
    });
    priority_scored.sort_by(|a, b| {
        mined_priority_order_score(input, b)
            .cmp(&mined_priority_order_score(input, a))
            .then_with(|| a.q.len().cmp(&b.q.len()))
            .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
    });

    let mut out = Vec::with_capacity(priority_scored.len() + scored.len());
    let mut used = FxHashSet::default();
    used.reserve(priority_scored.len() + scored.len() + 100);
    let mined_priority_keep = mined_priority_promote_keep(input);
    let mined_short_keep = mined_short_promote_keep(input);
    let mut mined_promoted = 0usize;
    for item in &priority_scored {
        if mined_promoted >= mined_priority_keep {
            break;
        }
        let q = item.q.as_slice();
        if is_mined_lane_arc(q) || is_mined_short_pattern(q) {
            let before = out.len();
            push_macro_candidate(&mut out, &mut used, item);
            if out.len() != before {
                mined_promoted += 1;
            }
        }
    }
    let mut short_promoted = 0usize;
    for item in &scored {
        if short_promoted >= mined_short_keep {
            break;
        }
        if item.kind == CAND_KIND_SHORT && is_mined_short_pattern(item.q.as_slice()) {
            let before = out.len();
            push_macro_candidate(&mut out, &mut used, item);
            if out.len() != before {
                short_promoted += 1;
            }
        }
    }
    let front_keep = min(PRIORITY_FRONT_KEEP, priority_scored.len());
    let promote_long_freq = input.m >= 28 && input.t_limit > 5000;
    let priority_lead = if promote_long_freq {
        min(LONG_FREQ_PROMOTE_AFTER_PRIORITY, front_keep)
    } else {
        front_keep
    };
    for item in priority_scored.iter().take(priority_lead) {
        push_macro_candidate(&mut out, &mut used, item);
    }
    if priority_lead < front_keep {
        if promote_long_freq {
            let mut promoted = 0usize;
            for item in &scored {
                if promoted >= LONG_FREQ_PROMOTE_KEEP {
                    break;
                }
                if should_promote_long_freq(input, basic_ops, item) {
                    let before = out.len();
                    push_macro_candidate(&mut out, &mut used, item);
                    if out.len() != before {
                        promoted += 1;
                    }
                }
            }
        }
    }
    for item in priority_scored.iter().take(front_keep).skip(priority_lead) {
        push_macro_candidate(&mut out, &mut used, item);
    }

    let mut rest: Vec<&CandItem> =
        Vec::with_capacity(scored.len() + priority_scored.len().saturating_sub(front_keep));
    rest.extend(scored.iter());
    rest.extend(priority_scored.iter().skip(front_keep));
    rest.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.q.len().cmp(&b.q.len()))
            .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
    });

    let take_n = min(rest.len(), 2600);
    for item in rest.into_iter().take(take_n) {
        push_macro_candidate(&mut out, &mut used, item);
    }
    out
}

struct GreedyQCandidate {
    len: usize,
    q: SmallQ,
    order: Vec<usize>,
    prepared: Option<GreedyPrepared>,
    kind: u8,
    coverage_class: u8,
    #[cfg(feature = "local")]
    mom_used: bool,
}

fn q_pool_cmp(a: &GreedyQCandidate, b: &GreedyQCandidate) -> Ordering {
    a.len
        .cmp(&b.len)
        .then_with(|| a.q.len().cmp(&b.q.len()))
        .then_with(|| a.q.as_slice().cmp(b.q.as_slice()))
}

fn sort_q_pool(pool: &mut [GreedyQCandidate]) {
    pool.sort_by(q_pool_cmp);
}

fn insert_greedy_q_candidate(
    pool: &mut Vec<GreedyQCandidate>,
    res: MacroSolveResult,
    kind: u8,
    coverage_class: u8,
) {
    let buttons_len = res.buttons.len();
    #[cfg(feature = "local")]
    let mom_used = res.mom_used;
    let prepared = res.prepared;
    if let Some(pos) = pool.iter().position(|item| item.q == res.q) {
        if buttons_len < pool[pos].len {
            pool[pos].len = buttons_len;
            pool[pos].order = res.order;
            if prepared.is_some() {
                pool[pos].prepared = prepared;
            }
            pool[pos].kind = kind;
            pool[pos].coverage_class = coverage_class;
            #[cfg(feature = "local")]
            {
                pool[pos].mom_used = mom_used;
            }
        }
        sort_q_pool(pool);
        return;
    }
    pool.push(GreedyQCandidate {
        len: buttons_len,
        q: res.q,
        order: res.order,
        prepared,
        kind,
        coverage_class,
        #[cfg(feature = "local")]
        mom_used,
    });
    sort_q_pool(pool);
    if pool.len() > Q_BEAM_POOL_SIZE {
        pool.truncate(Q_BEAM_POOL_SIZE);
    }
}

fn sa_candidate_score(item: &GreedyQCandidate, allow_power: bool) -> i32 {
    let feat = macro_features(item.q.as_slice());
    let qlen = item.q.len();
    let mut score = item.len as i32;
    score -= sa_mined_q_bonus(item.kind, item.q.as_slice());
    if feat.turn_count == 3 && qlen >= 12 {
        score -= 10;
    }
    if (12..=20).contains(&qlen) {
        score -= 4;
    }
    if max_registered_level_for_len(qlen, allow_power) >= 3 {
        score -= 7;
    }
    if feat.turn_count == 0 {
        score += 2;
    }
    if feat.max_run <= 2 {
        score += 4;
    }
    if qlen >= 25 {
        score += 14;
    }
    if item.kind == CAND_KIND_FREQ || item.kind == CAND_KIND_PAIR_ROUTE {
        score += 2;
    }
    score
}

fn sa_shape_bucket(q: &[u8]) -> (usize, usize, usize) {
    let feat = macro_features(q);
    let len_bucket = if q.len() <= 4 {
        0
    } else if q.len() <= 8 {
        1
    } else if q.len() <= 11 {
        2
    } else if q.len() <= 14 {
        3
    } else if q.len() <= 20 {
        4
    } else {
        5
    };
    let run_bucket = if feat.max_run <= 2 {
        0
    } else if feat.max_run <= 4 {
        1
    } else if feat.max_run <= 6 {
        2
    } else {
        3
    };
    (min(feat.turn_count, 7), len_bucket, run_bucket)
}

fn sa_shape_bucket_limit(item: &GreedyQCandidate) -> usize {
    let bonus = sa_mined_q_bonus(item.kind, item.q.as_slice());
    if bonus >= 44 {
        6
    } else if bonus >= 30 {
        5
    } else if bonus >= 18 {
        3
    } else {
        2
    }
}

fn select_sa_candidate_indices(pool: &[GreedyQCandidate], allow_power: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pool.len()).collect();
    order.sort_by(|&a, &b| {
        let sa = sa_candidate_score(&pool[a], allow_power);
        let sb = sa_candidate_score(&pool[b], allow_power);
        sa.cmp(&sb).then_with(|| q_pool_cmp(&pool[a], &pool[b]))
    });
    let mut selected = Vec::with_capacity(order.len());
    let mut used = vec![false; pool.len()];
    let mut kind_seed_counts = [0usize; CAND_KIND_COUNT];
    for &idx in &order {
        let kind = pool[idx].kind as usize;
        let cap = match pool[idx].kind {
            CAND_KIND_FREQ => 4,
            CAND_KIND_PAIR_ROUTE => 4,
            _ => 0,
        };
        if cap == 0 || kind_seed_counts[kind] >= cap {
            continue;
        }
        kind_seed_counts[kind] += 1;
        used[idx] = true;
        selected.push(idx);
    }
    let mut bucket_counts: BTreeMap<(usize, usize, usize), usize> = BTreeMap::new();
    for &idx in &order {
        if used[idx] {
            continue;
        }
        let bucket = sa_shape_bucket(pool[idx].q.as_slice());
        let cnt = *bucket_counts.get(&bucket).unwrap_or(&0);
        if cnt >= sa_shape_bucket_limit(&pool[idx]) {
            continue;
        }
        bucket_counts.insert(bucket, cnt + 1);
        used[idx] = true;
        selected.push(idx);
    }
    for idx in order {
        if !used[idx] {
            selected.push(idx);
        }
    }
    selected
}

fn simulate_buttons(input: &Input, grid: &Grid, buttons: &[u8]) -> bool {
    if buttons.len() > input.t_limit {
        return false;
    }
    let mut state = State::new(input);
    for &button in buttons {
        state.press_button(input, grid, button);
    }
    for k in 0..input.m {
        if state.cell_ball[input.basket_pos[k] as usize] != k as u8 {
            return false;
        }
    }
    true
}

fn expanded_basic_count_limited(buttons: &[u8], limit: usize) -> usize {
    let mut count = 0usize;
    let mut last_macro_len = 0usize;
    let mut cur_macro_len = 0usize;
    let mut recording = false;
    for &button in buttons {
        if count > limit {
            return count;
        }
        if button == OP_F || button == OP_R || button == OP_L || button == OP_S {
            count += 1;
            if recording {
                cur_macro_len += 1;
            }
        } else if button == OP_M {
            if recording {
                last_macro_len = cur_macro_len;
                cur_macro_len = 0;
                recording = false;
            } else {
                cur_macro_len = 0;
                recording = true;
            }
        } else if button == OP_P {
            count += last_macro_len;
            if recording {
                cur_macro_len += last_macro_len;
            }
        }
    }
    count
}

fn buttons_fit_limits(input: &Input, buttons: &[u8]) -> bool {
    buttons.len() <= input.t_limit
        && expanded_basic_count_limited(buttons, input.t_limit) <= input.t_limit
}

fn expand_buttons_to_basic(input: &Input, buttons: &[u8]) -> Vec<u8> {
    let mut expanded_ops = Vec::with_capacity(input.t_limit);
    let mut last_macro = Vec::new();
    let mut cur_macro = Vec::new();
    let mut recording = false;
    for &button in buttons {
        if expanded_ops.len() >= input.t_limit {
            break;
        }
        if button == OP_F || button == OP_R || button == OP_L || button == OP_S {
            expanded_ops.push(button);
            if recording {
                cur_macro.push(button);
            }
        } else if button == OP_M {
            if recording {
                last_macro = cur_macro;
                cur_macro = Vec::new();
                recording = false;
            } else {
                cur_macro.clear();
                recording = true;
            }
        } else if button == OP_P {
            let macro_ops = last_macro.clone();
            for op in macro_ops {
                if expanded_ops.len() >= input.t_limit {
                    break;
                }
                expanded_ops.push(op);
                if recording {
                    cur_macro.push(op);
                }
            }
        }
    }
    expanded_ops
}

#[inline(always)]
fn button_to_char(button: u8) -> char {
    match button {
        OP_F => 'F',
        OP_R => 'R',
        OP_L => 'L',
        OP_S => 'S',
        OP_M => 'M',
        OP_P => 'P',
        _ => unreachable!(),
    }
}

fn main() {
    let timer = Timer::new();
    let program_time_limit_sec = program_time_limit_sec();
    let input = Input::read();
    let allow_power = true;
    let grid = Grid::new(&input);
    let dist = all_pairs_dist(&grid); // dist[target][cell*4+dir] は (cell, dir) から target への最短手数(F/R/L=1)
    let basic_ops = build_basic_ops(&input, &grid, &dist);
    let mut best_answer = compress_with_multiple_macros(&basic_ops);
    let safe_answer = best_answer.clone();
    #[cfg(feature = "local")]
    let mut best_source = BEST_SOURCE_INIT;
    #[cfg(feature = "local")]
    let mut greedy_tried = 0usize;
    #[cfg(feature = "local")]
    let mut greedy_success = 0usize;
    #[cfg(feature = "local")]
    let mut greedy_best_update = 0usize;
    #[cfg(feature = "local")]
    let mut sa_best_update = 0usize;
    #[cfg(feature = "local")]
    let mut sa_probe_tried = 0usize;
    #[cfg(feature = "local")]
    let sa_probe_kept: usize;
    #[cfg(feature = "local")]
    let mut sa_probe_best_update = 0usize;
    #[cfg(feature = "local")]
    let mut sa_full_tried = 0usize;
    #[cfg(feature = "local")]
    let mut mom_greedy_success = 0usize;
    #[cfg(feature = "local")]
    let mut mom_greedy_best_update = 0usize;
    #[cfg(feature = "local")]
    let mut mom_sa_best_update = 0usize;
    #[cfg(feature = "local")]
    let mut post_recompress_tried = false;
    #[cfg(feature = "local")]
    let mut post_recompress_compressed = false;
    #[cfg(feature = "local")]
    let mut post_recompress_improved = false;
    #[cfg(feature = "local")]
    let mut post_recompress_gain = 0usize;
    #[cfg(feature = "local")]
    let mut post_recompress_expand_len = 0usize;
    #[cfg(feature = "local")]
    let mut post_recompress_ms = 0.0f64;
    #[cfg(feature = "local")]
    let mut generated_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut greedy_tried_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut greedy_success_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut greedy_best_update_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut sa_tried_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut sa_success_by_kind = [0usize; CAND_KIND_COUNT];
    #[cfg(feature = "local")]
    let mut sa_best_update_by_kind = [0usize; CAND_KIND_COUNT];

    let mut q_pool = Vec::with_capacity(Q_BEAM_POOL_SIZE);
    let candidates = gen_macro_candidates(&input, &grid, &dist, &basic_ops, allow_power);
    // gen_macro_candidates では
    // 1. 短い一般候補
    // 2. basic_ops 由来の長め頻出候補
    // 3. ball->basket の最短路 prefix 候補
    // 4. 手作りの優先候補
    // の四系統で候補を生成する。

    #[cfg(feature = "local")]
    let candidate_count = candidates.len();
    #[cfg(feature = "local")]
    {
        for cand in &candidates {
            generated_by_kind[cand.kind as usize] += 1;
        }
    }
    // ここが主力。 macroの候補に対して貪欲に解いてみて、良いものをQプールに入れていく。
    for cand in &candidates {
        if timer.elapsed() >= q_search_time_limit_sec(program_time_limit_sec) {
            // 全体の40%ぐらいの時間をここで使用する。
            break;
        }
        #[cfg(feature = "local")]
        {
            greedy_tried += 1;
            greedy_tried_by_kind[cand.kind as usize] += 1;
        }
        let candidate_limit = min(input.t_limit + 1, best_answer.len() + Q_POOL_EXTRA_LIMIT);
        let Some(res) = solve_with_macro_candidate_greedy(
            &input,
            &grid,
            cand.q,
            candidate_limit,
            allow_power,
        )
        else {
            continue;
        };
        #[cfg(feature = "local")]
        {
            greedy_success += 1;
            greedy_success_by_kind[cand.kind as usize] += 1;
            if res.mom_used {
                mom_greedy_success += 1;
            }
        }
        let shorter = res.buttons.len() < best_answer.len();
        if shorter && buttons_fit_limits(&input, &res.buttons) {
            #[cfg(feature = "local")]
            {
                greedy_best_update += 1;
                greedy_best_update_by_kind[cand.kind as usize] += 1;
                if res.mom_used {
                    mom_greedy_best_update += 1;
                }
                best_source = cand.kind;
            }
            best_answer = res.buttons.clone();
        }
        insert_greedy_q_candidate(&mut q_pool, res, cand.kind, cand.coverage_class); // SA用に少数残しておく
    }

    let sa_order = select_sa_candidate_indices(&q_pool, allow_power);
    let deadline_margin_sec = time_ratio_sec(program_time_limit_sec, DEADLINE_MARGIN_RATIO);
    let sa_probe_stop_margin_sec =
        time_ratio_sec(program_time_limit_sec, SA_PROBE_STOP_MARGIN_RATIO);
    let two_stage_probe_slice_sec =
        time_ratio_sec(program_time_limit_sec, TWO_STAGE_PROBE_SLICE_RATIO);
    let two_stage_full_slice_sec =
        time_ratio_sec(program_time_limit_sec, TWO_STAGE_FULL_SLICE_RATIO);
    let search_deadline =
        program_time_limit_sec - time_ratio_sec(program_time_limit_sec, POWER_RECOMPRESS_RESERVE_RATIO);
    let sa_hard_deadline = search_deadline - deadline_margin_sec;
    let probe_phase_deadline =
        timer.elapsed() + (sa_hard_deadline - timer.elapsed()).max(0.0) * TWO_STAGE_PROBE_FRACTION;
    let mut probe_contexts: Vec<ProbeSaContext> = Vec::with_capacity(TWO_STAGE_FULL_KEEP);
    for &idx in &sa_order {
        if timer.elapsed() + sa_probe_stop_margin_sec >= probe_phase_deadline {
            break;
        }
        let limit = min(input.t_limit + 1, best_answer.len()); //現在bestより短くない解は不要
        #[cfg(feature = "local")]
        {
            sa_probe_tried += 1;
            sa_tried_by_kind[q_pool[idx].kind as usize] += 1;
        }
        let kind = q_pool[idx].kind;
        let Some(mut ctx) = prepare_sa_context(
            &input,
            &grid,
            &mut q_pool[idx],
            kind,
            limit,
            &timer,
            probe_phase_deadline,
            2000 + idx as u64,
            allow_power,
        ) else {
            continue;
        };
        let probe_deadline = probe_phase_deadline.min(timer.elapsed() + two_stage_probe_slice_sec);
        run_order_sa_context(&input, &mut ctx, &timer, probe_deadline, None);
        if let Some(res) = result_from_sa_context(&input, &ctx, limit) {
            #[cfg(feature = "local")]
            {
                sa_success_by_kind[q_pool[idx].kind as usize] += 1;
            }
            if res.buttons.len() < best_answer.len() && buttons_fit_limits(&input, &res.buttons) {
                #[cfg(feature = "local")]
                {
                    sa_best_update += 1;
                    sa_probe_best_update += 1;
                    sa_best_update_by_kind[q_pool[idx].kind as usize] += 1;
                    if res.mom_used {
                        mom_sa_best_update += 1;
                    }
                    best_source = q_pool[idx].kind;
                }
                best_answer = res.buttons;
            }
        }
        probe_contexts.push(ProbeSaContext {
            pool_idx: idx,
            greedy_len: q_pool[idx].len,
            ctx,
        });
        diversify_probe_contexts(&mut probe_contexts, &q_pool, TWO_STAGE_FULL_KEEP);
    }
    #[cfg(feature = "local")]
    {
        sa_probe_kept = probe_contexts.len();
    }

    diversify_probe_contexts(&mut probe_contexts, &q_pool, TWO_STAGE_FULL_KEEP);
    let full_len = probe_contexts.len();
    for (ordpos, item) in probe_contexts.iter_mut().enumerate() {
        if timer.elapsed() + two_stage_full_slice_sec >= search_deadline {
            break;
        }
        let remain = search_deadline - timer.elapsed() - deadline_margin_sec;
        let left = max(1, full_len - ordpos) as f64;
        let slice = two_stage_full_slice_sec.max(remain / left);
        let deadline = sa_hard_deadline.min(timer.elapsed() + slice);
        let limit = min(input.t_limit + 1, best_answer.len());
        #[cfg(feature = "local")]
        {
            sa_full_tried += 1;
        }
        let prune_limit = (limit as u32).saturating_add(sa_early_prune_slack(&input));
        let pruned =
            run_order_sa_context(&input, &mut item.ctx, &timer, deadline, Some(prune_limit));
        if pruned {
            continue;
        }
        if let Some(res) = result_from_sa_context(&input, &item.ctx, limit) {
            #[cfg(feature = "local")]
            {
                sa_success_by_kind[item.ctx.kind as usize] += 1;
            }
            if res.buttons.len() < best_answer.len() && buttons_fit_limits(&input, &res.buttons) {
                #[cfg(feature = "local")]
                {
                    sa_best_update += 1;
                    sa_best_update_by_kind[item.ctx.kind as usize] += 1;
                    if res.mom_used {
                        mom_sa_best_update += 1;
                    }
                    best_source = item.ctx.kind;
                }
                best_answer = res.buttons;
            }
        }
    }

    let post_recompress_min_remain_sec =
        time_ratio_sec(program_time_limit_sec, POST_RECOMPRESS_MIN_REMAIN_RATIO);
    let power_recompress_min_remain_sec =
        time_ratio_sec(program_time_limit_sec, POWER_RECOMPRESS_MIN_REMAIN_RATIO);
    if timer.elapsed() + post_recompress_min_remain_sec < program_time_limit_sec {
        #[cfg(feature = "local")]
        let post_start = timer.elapsed();
        #[cfg(feature = "local")]
        {
            post_recompress_tried = true;
        }
        let expanded_basic = expand_buttons_to_basic(&input, &best_answer);
        #[cfg(feature = "local")]
        {
            post_recompress_expand_len = expanded_basic.len();
        }
        if expanded_basic.len() <= input.t_limit && expanded_basic.len() <= 1200 {
            #[cfg(feature = "local")]
            {
                post_recompress_compressed = true;
            }
            let rec = compress_with_multiple_macros(&expanded_basic);
            if rec.len() < best_answer.len() && buttons_fit_limits(&input, &rec) {
                #[cfg(feature = "local")]
                {
                    post_recompress_improved = true;
                    post_recompress_gain = best_answer.len() - rec.len();
                    best_source = BEST_SOURCE_RECOMPRESS;
                }
                best_answer = rec;
            }
        }
        if timer.elapsed() + power_recompress_min_remain_sec < program_time_limit_sec {
            let deadline = program_time_limit_sec - deadline_margin_sec;
            let best_limit = best_answer.len().saturating_sub(1);
            if best_limit > 0 {
                if let Some((rec, _cand_count)) = power_recompress_limited(
                    &expanded_basic,
                    &best_answer,
                    best_limit,
                    &timer,
                    deadline,
                ) {
                    #[cfg(feature = "local")]
                    {
                        post_recompress_compressed = true;
                    }
                    if rec.len() < best_answer.len() && buttons_fit_limits(&input, &rec) {
                        #[cfg(feature = "local")]
                        {
                            post_recompress_improved = true;
                            post_recompress_gain = best_answer.len() - rec.len();
                            best_source = BEST_SOURCE_RECOMPRESS;
                        }
                        best_answer = rec;
                    }
                }
            }
        }
        #[cfg(feature = "local")]
        {
            post_recompress_ms = (timer.elapsed() - post_start) * 1000.0;
        }
    }

    if best_answer.len() > input.t_limit || !simulate_buttons(&input, &grid, &best_answer) {
        best_answer = safe_answer;
        #[cfg(feature = "local")]
        {
            best_source = BEST_SOURCE_SAFE_FALLBACK;
        }
        if best_answer.len() > input.t_limit || !simulate_buttons(&input, &grid, &best_answer) {
            best_answer = basic_ops;
            #[cfg(feature = "local")]
            {
                best_source = BEST_SOURCE_FALLBACK;
            }
        }
    }

    local! {
        let mut q_pool_by_kind = [0usize; CAND_KIND_COUNT];
        let mut mom_q_pool = 0usize;
        for item in &q_pool {
            q_pool_by_kind[item.kind as usize] += 1;
            if item.mom_used {
                mom_q_pool += 1;
            }
        }
        let sa_early_prune = SA_EARLY_PRUNE_COUNT.load(AtomicOrdering::Relaxed);
        eprintln!(
            "[summary] time_limit_sec={:.3} elapsed_sec={:.3} answer_len={} candidates={} q_pool={} long_priority_first={} greedy_tried={} greedy_success={} greedy_best_update={} sa_best_update={} sa_probe_tried={} sa_probe_kept={} sa_probe_best_update={} sa_full_tried={} sa_early_prune={} mom_greedy_success={} mom_greedy_best_update={} mom_q_pool={} mom_sa_best_update={} best_source={} post_recompress_tried={} post_recompress_compressed={} post_recompress_improved={} post_recompress_gain={} post_recompress_expand_len={} post_recompress_ms={:.3}",
            program_time_limit_sec,
            timer.elapsed(),
            best_answer.len(),
            candidate_count,
            q_pool.len(),
            (input.t_limit > 5000) as usize,
            greedy_tried,
            greedy_success,
            greedy_best_update,
            sa_best_update,
            sa_probe_tried,
            sa_probe_kept,
            sa_probe_best_update,
            sa_full_tried,
            sa_early_prune,
            mom_greedy_success,
            mom_greedy_best_update,
            mom_q_pool,
            mom_sa_best_update,
            cand_kind_name(best_source),
            post_recompress_tried as usize,
            post_recompress_compressed as usize,
            post_recompress_improved as usize,
            post_recompress_gain,
            post_recompress_expand_len,
            post_recompress_ms
        );
        for kind in 0..CAND_KIND_COUNT {
            eprintln!(
                "[candidate_kind] kind={} generated={} greedy_tried={} greedy_success={} greedy_best_update={} q_pool={} sa_tried={} sa_success={} sa_best_update={}",
                cand_kind_name(kind as u8),
                generated_by_kind[kind],
                greedy_tried_by_kind[kind],
                greedy_success_by_kind[kind],
                greedy_best_update_by_kind[kind],
                q_pool_by_kind[kind],
                sa_tried_by_kind[kind],
                sa_success_by_kind[kind],
                sa_best_update_by_kind[kind],
            );
        }
    }

    let mut out = String::with_capacity(best_answer.len() * 2);
    for &op in &best_answer {
        out.push(button_to_char(op));
        out.push('\n');
    }
    print!("{out}");
}
