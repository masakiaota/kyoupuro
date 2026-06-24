// v997_repro_from_explanation.rs
use proconio::{input, marker::Bytes};
use std::fmt::Write as _;
use std::time::Instant;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

const N: usize = 20;
const M: usize = 50;
const K: usize = 10;
const CELL_COUNT: usize = N * N;
const H_DOOR_COUNT: usize = (N - 1) * N;
const V_DOOR_COUNT: usize = N * (N - 1);
const EDGE_COUNT: usize = H_DOOR_COUNT + V_DOOR_COUNT;
const MASK_COUNT: usize = 1 << K;
const HERO_STATE_COUNT: usize = MASK_COUNT * CELL_COUNT;
const START_ID: usize = 0;
const GOAL_ID: usize = CELL_COUNT - 1;
const NO_DOOR: u8 = u8::MAX;
const NO_SWITCH: u8 = u8::MAX;
const UNREACHED: i32 = -1;
const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[inline(always)]
fn id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn ij(id: usize) -> (usize, usize) {
    (id / N, id % N)
}

#[inline(always)]
fn h_edge_id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn v_edge_id(i: usize, j: usize) -> usize {
    H_DOOR_COUNT + i * (N - 1) + j
}

#[inline(always)]
fn is_open(g: u8, mask: usize) -> bool {
    g == NO_DOOR || ((mask >> (g as usize / 2)) & 1) == (g as usize & 1)
}

#[derive(Clone, Copy)]
struct Adj {
    to: u16,
    edge: u16,
}

impl Adj {
    const INVALID: Self = Self {
        to: u16::MAX,
        edge: u16::MAX,
    };
}

struct InputData {
    grid: [u8; CELL_COUNT],
    adj: [[Adj; 4]; CELL_COUNT],
    deg: [u8; CELL_COUNT],
    empty_ids: Vec<usize>,
}

impl InputData {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            k: usize,
            rows: [Bytes; N],
        }
        assert_eq!(n, N);
        assert_eq!(m, M);
        assert_eq!(k, K);

        let mut grid = [b'#'; CELL_COUNT];
        for i in 0..N {
            for j in 0..N {
                grid[id(i, j)] = rows[i][j];
            }
        }

        let mut adj = [[Adj::INVALID; 4]; CELL_COUNT];
        let mut deg = [0_u8; CELL_COUNT];
        let mut empty_ids = Vec::new();
        for v in 0..CELL_COUNT {
            if grid[v] != b'.' {
                continue;
            }
            empty_ids.push(v);
            let (i, j) = ij(v);
            for &(di, dj) in &DIRS {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                    continue;
                }
                let to = id(ni as usize, nj as usize);
                if grid[to] != b'.' {
                    continue;
                }
                let edge = match (di, dj) {
                    (1, 0) => h_edge_id(i, j),
                    (-1, 0) => h_edge_id(ni as usize, nj as usize),
                    (0, 1) => v_edge_id(i, j),
                    (0, -1) => v_edge_id(ni as usize, nj as usize),
                    _ => unreachable!(),
                };
                let p = deg[v] as usize;
                adj[v][p] = Adj {
                    to: to as u16,
                    edge: edge as u16,
                };
                deg[v] += 1;
            }
        }

        Self {
            grid,
            adj,
            deg,
            empty_ids,
        }
    }

    #[inline(always)]
    fn is_empty(&self, v: usize) -> bool {
        self.grid[v] == b'.'
    }

    #[inline(always)]
    fn adjs(&self, v: usize) -> &[Adj] {
        &self.adj[v][..self.deg[v] as usize]
    }
}

#[derive(Clone)]
struct Plan {
    door: [u8; EDGE_COUNT],
    switch: [u8; CELL_COUNT],
    door_count: u8,
}

impl Plan {
    fn empty() -> Self {
        Self {
            door: [NO_DOOR; EDGE_COUNT],
            switch: [NO_SWITCH; CELL_COUNT],
            door_count: 0,
        }
    }

    fn add_door(&mut self, edge: usize, g: u8) -> bool {
        if self.door[edge] == g {
            return true;
        }
        if self.door[edge] != NO_DOOR || self.door_count as usize >= M {
            return false;
        }
        self.door[edge] = g;
        self.door_count += 1;
        true
    }

    fn add_switch(&mut self, v: usize, s: u8) -> bool {
        if self.switch[v] != NO_SWITCH {
            return false;
        }
        self.switch[v] = s;
        true
    }

    fn output(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "{}", self.door_count).unwrap();
        for e in 0..H_DOOR_COUNT {
            let g = self.door[e];
            if g != NO_DOOR {
                writeln!(&mut out, "0 {} {} {}", e / N, e % N, g).unwrap();
            }
        }
        for e in H_DOOR_COUNT..EDGE_COUNT {
            let g = self.door[e];
            if g != NO_DOOR {
                let r = e - H_DOOR_COUNT;
                writeln!(&mut out, "1 {} {} {}", r / (N - 1), r % (N - 1), g).unwrap();
            }
        }

        let switches: Vec<_> = (0..CELL_COUNT)
            .filter(|&v| self.switch[v] != NO_SWITCH)
            .collect();
        writeln!(&mut out, "{}", switches.len()).unwrap();
        for v in switches {
            let (i, j) = ij(v);
            writeln!(&mut out, "{} {} {}", i, j, self.switch[v]).unwrap();
        }
        out
    }

    fn calc_t(&self, input: &InputData, scratch: &mut EvalScratch) -> usize {
        scratch.clear();
        scratch.dist[START_ID] = 0;
        scratch.queue.push(START_ID as u32);
        let mut head = 0;
        while head < scratch.queue.len() {
            let packed = scratch.queue[head];
            head += 1;
            let mask = (packed >> 16) as usize;
            let v = (packed & 0xffff) as usize;
            let d = scratch.dist[mask * CELL_COUNT + v];
            if v == GOAL_ID {
                return d as usize;
            }
            for &a in input.adjs(v) {
                let edge = a.edge as usize;
                if !is_open(self.door[edge], mask) {
                    continue;
                }
                let to = a.to as usize;
                let ni = mask * CELL_COUNT + to;
                if scratch.dist[ni] == UNREACHED {
                    scratch.dist[ni] = d + 1;
                    scratch.queue.push(((mask as u32) << 16) | to as u32);
                }
            }
            let s = self.switch[v];
            if s != NO_SWITCH {
                let nmask = mask ^ (1_usize << s as usize);
                let ni = nmask * CELL_COUNT + v;
                if scratch.dist[ni] == UNREACHED {
                    scratch.dist[ni] = d + 1;
                    scratch.queue.push(((nmask as u32) << 16) | v as u32);
                }
            }
        }
        0
    }
}

struct EvalScratch {
    dist: Vec<i32>,
    queue: Vec<u32>,
}

impl EvalScratch {
    fn new() -> Self {
        Self {
            dist: vec![UNREACHED; HERO_STATE_COUNT],
            queue: Vec::with_capacity(HERO_STATE_COUNT),
        }
    }

    fn clear(&mut self) {
        self.dist.fill(UNREACHED);
        self.queue.clear();
    }
}

struct FixedScratch {
    dist: [i16; CELL_COUNT],
    prev: [u16; CELL_COUNT],
    queue: [u16; CELL_COUNT],
}

impl FixedScratch {
    fn new() -> Self {
        Self {
            dist: [-1; CELL_COUNT],
            prev: [u16::MAX; CELL_COUNT],
            queue: [0; CELL_COUNT],
        }
    }

    fn bfs(&mut self, input: &InputData, plan: &Plan, src: usize, mask: usize) {
        self.dist.fill(-1);
        self.prev.fill(u16::MAX);
        let mut head = 0;
        let mut tail = 0;
        self.dist[src] = 0;
        self.queue[tail] = src as u16;
        tail += 1;
        while head < tail {
            let v = self.queue[head] as usize;
            head += 1;
            let d = self.dist[v];
            for &a in input.adjs(v) {
                let edge = a.edge as usize;
                if !is_open(plan.door[edge], mask) {
                    continue;
                }
                let to = a.to as usize;
                if self.dist[to] < 0 {
                    self.dist[to] = d + 1;
                    self.prev[to] = v as u16;
                    self.queue[tail] = to as u16;
                    tail += 1;
                }
            }
        }
    }
}

#[derive(Clone)]
struct Candidate {
    estimate: i64,
    plan: Plan,
}

#[derive(Clone)]
struct Rng {
    x: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { x: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    fn gen_usize(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n
    }
}

struct TimeKeeper {
    start: Instant,
    limit: f64,
    iter: u64,
    elapsed: f64,
}

impl TimeKeeper {
    fn new(limit: f64) -> Self {
        Self {
            start: Instant::now(),
            limit,
            iter: 0,
            elapsed: 0.0,
        }
    }

    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & 255) == 0 {
            self.elapsed = self.start.elapsed().as_secs_f64();
        }
        self.elapsed < self.limit
    }

    fn progress(&self) -> f64 {
        (self.elapsed / self.limit).clamp(0.0, 1.0)
    }

    fn over_exact(&self) -> bool {
        self.start.elapsed().as_secs_f64() >= self.limit
    }
}

fn random_path(input: &InputData, rng: &mut Rng, len_edges: usize) -> Option<Vec<usize>> {
    let mut path = Vec::with_capacity(len_edges + 1);
    let mut used = [false; CELL_COUNT];
    path.push(GOAL_ID);
    used[GOAL_ID] = true;

    while path.len() <= len_edges {
        let v = *path.last().unwrap();
        let mut choices = [(0_usize, 0_usize); 4];
        let mut count = 0;
        for &a in input.adjs(v) {
            let to = a.to as usize;
            if used[to] || to == START_ID {
                continue;
            }
            let mut touch = 0;
            let mut free = 0;
            for &b in input.adjs(to) {
                let u = b.to as usize;
                if used[u] {
                    touch += 1;
                } else {
                    free += 1;
                }
            }
            let w = 12 + 9 * touch + 3 * free + rng.gen_usize(7);
            choices[count] = (to, w);
            count += 1;
        }
        if count == 0 {
            return None;
        }
        let sum: usize = choices[..count].iter().map(|x| x.1).sum();
        let mut r = rng.gen_usize(sum);
        let mut picked = choices[0].0;
        for &(to, w) in &choices[..count] {
            if r < w {
                picked = to;
                break;
            }
            r -= w;
        }
        used[picked] = true;
        path.push(picked);
    }
    path.reverse();
    Some(path)
}

fn choose_branch(input: &InputData, path_used: &[bool; CELL_COUNT], v: usize, used: &[bool; CELL_COUNT]) -> Option<usize> {
    let mut best = None;
    let mut best_deg = 99_u8;
    for &a in input.adjs(v) {
        let to = a.to as usize;
        if path_used[to] || used[to] || to == START_ID || to == GOAL_ID {
            continue;
        }
        if input.deg[to] < best_deg {
            best_deg = input.deg[to];
            best = Some(to);
        }
    }
    best
}

fn make_candidate(
    input: &InputData,
    rng: &mut Rng,
    fixed: &mut FixedScratch,
    expand_branches: bool,
) -> Option<Candidate> {
    let len_edges = 10 + rng.gen_usize(18);
    let path = random_path(input, rng, len_edges)?;
    if path.len() < 11 {
        return None;
    }

    let mut path_used = [false; CELL_COUNT];
    for &v in &path {
        path_used[v] = true;
    }

    let mut branch_positions = Vec::with_capacity(8);
    let mut branch_cells = Vec::with_capacity(8);
    let mut used_side = path_used;
    for idx in 0..path.len() - 1 {
        if branch_positions.len() == 8 {
            break;
        }
        let v = path[idx];
        if let Some(b) = choose_branch(input, &path_used, v, &used_side) {
            branch_positions.push(idx);
            branch_cells.push(b);
            used_side[b] = true;
        }
    }
    if branch_positions.len() != 8 {
        return None;
    }
    let last_branch = *branch_positions.last().unwrap();
    let final_pos = (last_branch + 1..path.len() - 1).next()?;

    let mut protected = [false; CELL_COUNT];
    for &v in &path {
        protected[v] = true;
    }
    for &v in &branch_cells {
        protected[v] = true;
    }

    let mut region = protected;
    if expand_branches {
        let mut add = Vec::new();
        for &v in &branch_cells {
            for &a in input.adjs(v) {
                let to = a.to as usize;
                if to != START_ID && !region[to] {
                    add.push(to);
                }
            }
        }
        for v in add {
            region[v] = true;
        }
    }
    loop {
        let mut changed = false;
        for &v in &input.empty_ids {
            if region[v] || v == START_ID {
                continue;
            }
            let mut cnt = 0;
            for &a in input.adjs(v) {
                if region[a.to as usize] {
                    cnt += 1;
                }
            }
            if cnt >= 3 {
                region[v] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if region[START_ID] {
        return None;
    }

    let mut plan = Plan::empty();
    let mut allowed_core_edge = [false; EDGE_COUNT];
    for w in path.windows(2) {
        let e = input
            .adjs(w[0])
            .iter()
            .find(|a| a.to as usize == w[1])
            .map(|a| a.edge as usize)?;
        allowed_core_edge[e] = true;
    }
    for (t, &pos) in branch_positions.iter().enumerate() {
        let branch_edge = input
            .adjs(path[pos])
            .iter()
            .find(|a| a.to as usize == branch_cells[t])
            .map(|a| a.edge as usize)?;
        allowed_core_edge[branch_edge] = true;
        if !plan.add_switch(branch_cells[t], (t + 2) as u8) {
            return None;
        }
        if !plan.add_door(branch_edge, (2 * t + 3) as u8) {
            return None;
        }
        let forward_edge = input
            .adjs(path[pos])
            .iter()
            .find(|a| a.to as usize == path[pos + 1])
            .map(|a| a.edge as usize)?;
        if !plan.add_door(forward_edge, (2 * t + 2) as u8) {
            return None;
        }
    }

    let final_edge = input
        .adjs(path[final_pos])
        .iter()
        .find(|a| a.to as usize == path[final_pos + 1])
        .map(|a| a.edge as usize)?;
    if !plan.add_door(final_edge, 19) {
        return None;
    }

    for &v in &input.empty_ids {
        if !region[v] {
            continue;
        }
        for &a in input.adjs(v) {
            let to = a.to as usize;
            if region[to] {
                if allowed_core_edge[a.edge as usize] {
                    continue;
                }
                if protected[v] && protected[to] && !plan.add_door(a.edge as usize, 0) {
                    return None;
                }
                continue;
            } else if !plan.add_door(a.edge as usize, 0) {
                return None;
            }
        }
    }

    for v in 0..CELL_COUNT {
        if !protected[v] {
            continue;
        }
        for &a in input.adjs(v) {
            let to = a.to as usize;
            if !region[to] || protected[to] || allowed_core_edge[a.edge as usize] {
                continue;
            }
            if plan.door[a.edge as usize] == NO_DOOR && !plan.add_door(a.edge as usize, 1) {
                return None;
            }
        }
    }

    let mut accum = [0_i64; CELL_COUNT];
    let mut ok = [true; CELL_COUNT];
    fixed.bfs(input, &plan, START_ID, 0);
    for &v in &input.empty_ids {
        if fixed.dist[v] < 0 {
            ok[v] = false;
        } else {
            accum[v] += fixed.dist[v] as i64;
        }
    }
    for t in 0..8 {
        let mask = 1_usize << (t + 1);
        fixed.bfs(input, &plan, branch_cells[t], mask);
        for &v in &input.empty_ids {
            if fixed.dist[v] < 0 {
                ok[v] = false;
            } else {
                accum[v] += (256_i64 >> t) * fixed.dist[v] as i64;
            }
        }
    }
    fixed.bfs(input, &plan, GOAL_ID, 1 << 9);
    for &v in &input.empty_ids {
        if fixed.dist[v] < 0 {
            ok[v] = false;
        } else {
            accum[v] += fixed.dist[v] as i64;
        }
    }

    let mut s0 = None;
    let mut best_accum = -1_i64;
    for &v in &input.empty_ids {
        if v == START_ID || v == GOAL_ID || region[v] || protected[v] || !ok[v] {
            continue;
        }
        if accum[v] > best_accum {
            best_accum = accum[v];
            s0 = Some(v);
        }
    }
    let s0 = s0?;
    if !plan.add_switch(s0, 0) {
        return None;
    }

    let mut p1 = None;
    let mut best_d = -1_i16;
    fixed.bfs(input, &plan, s0, 1);
    for &v in &input.empty_ids {
        if v == START_ID || v == GOAL_ID || v == s0 || region[v] || protected[v] {
            continue;
        }
        let mut touches_r = false;
        let mut extra_doors = 0;
        for &a in input.adjs(v) {
            if region[a.to as usize] {
                touches_r = true;
            }
            if plan.door[a.edge as usize] == NO_DOOR {
                extra_doors += 1;
            }
        }
        if touches_r || plan.door_count as usize + extra_doors > M {
            continue;
        }
        let d = fixed.dist[v];
        if d > best_d {
            best_d = d;
            p1 = Some(v);
        }
    }
    let p1 = p1?;
    if best_d < 0 || !plan.add_switch(p1, 1) {
        return None;
    }
    for &a in input.adjs(p1) {
        if !plan.add_door(a.edge as usize, 1) {
            return None;
        }
    }

    let estimate = 1023_i64 + best_accum + 512 * best_d as i64;

    Some(Candidate { estimate, plan })
}

fn improve_with_type19(input: &InputData, cand: &mut Candidate, fixed: &mut FixedScratch) {
    while cand.plan.door_count as usize + 1 <= M {
        let mut score_by_edge = [0_i32; EDGE_COUNT];
        let routes = [
            (START_ID, 0_usize, 1_i32),
            (find_switch(&cand.plan, 1).unwrap_or(START_ID), 1_usize, 512_i32),
            (find_switch(&cand.plan, 2).unwrap_or(START_ID), 1_usize << 1, 256_i32),
            (find_switch(&cand.plan, 3).unwrap_or(START_ID), 1_usize << 2, 128_i32),
            (find_switch(&cand.plan, 4).unwrap_or(START_ID), 1_usize << 3, 64_i32),
            (find_switch(&cand.plan, 5).unwrap_or(START_ID), 1_usize << 4, 32_i32),
            (find_switch(&cand.plan, 6).unwrap_or(START_ID), 1_usize << 5, 16_i32),
            (find_switch(&cand.plan, 7).unwrap_or(START_ID), 1_usize << 6, 8_i32),
            (find_switch(&cand.plan, 8).unwrap_or(START_ID), 1_usize << 7, 4_i32),
            (find_switch(&cand.plan, 9).unwrap_or(START_ID), 1_usize << 8, 2_i32),
            (GOAL_ID, 1_usize << 9, 1_i32),
        ];
        let s0 = match find_switch(&cand.plan, 0) {
            Some(v) => v,
            None => return,
        };
        for &(src, mask, w) in &routes {
            fixed.bfs(input, &cand.plan, src, mask);
            if fixed.dist[s0] < 0 {
                return;
            }
            let mut v = s0;
            while v != src {
                let p = fixed.prev[v] as usize;
                if p >= CELL_COUNT {
                    break;
                }
                let edge = input
                    .adjs(v)
                    .iter()
                    .find(|a| a.to as usize == p)
                    .map(|a| a.edge as usize);
                if let Some(e) = edge {
                    if cand.plan.door[e] == NO_DOOR {
                        score_by_edge[e] += w;
                    }
                }
                v = p;
            }
        }
        let mut best = None;
        let mut best_w = 0;
        for (e, &w) in score_by_edge.iter().enumerate() {
            if w > best_w && cand.plan.door[e] == NO_DOOR {
                best_w = w;
                best = Some(e);
            }
        }
        if let Some(e) = best {
            if !cand.plan.add_door(e, 19) {
                return;
            }
        } else {
            return;
        }
    }
}

fn find_switch(plan: &Plan, s: u8) -> Option<usize> {
    (0..CELL_COUNT).find(|&v| plan.switch[v] == s)
}

fn main() {
    let input = InputData::read();
    let seed = input
        .empty_ids
        .iter()
        .fold(0x9e37_79b9_7f4a_7c15_u64, |acc, &v| {
            acc ^ ((v as u64 + 1).wrapping_mul(0xbf58_476d_1ce4_e5b9))
        });
    let mut rng = Rng::new(seed);
    let mut tk = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let mut fixed = FixedScratch::new();
    let mut candidates: Vec<Candidate> = Vec::new();

    while tk.step() && tk.progress() < 0.82 {
        if let Some(cand) = make_candidate(&input, &mut rng, &mut fixed, true) {
            candidates.push(cand);
            candidates.sort_by_key(|c| -c.estimate);
            if candidates.len() > 80 {
                candidates.pop();
            }
        }
    }

    // 型 19 の追加扉は解説では推定値を再計算しながら採用する。
    // ここでは基本構造の再現を優先し、無検証の追加で経路を壊さない。

    let mut eval = EvalScratch::new();
    let mut best_t = 0_usize;
    let mut best_plan = Plan::empty();
    for cand in &candidates {
        let t = cand.plan.calc_t(&input, &mut eval);
        if t > best_t {
            best_t = t;
            best_plan = cand.plan.clone();
        }
    }

    print!("{}", best_plan.output());
}
