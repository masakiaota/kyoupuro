// v995_repro_from_explanation_v3.rs
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
fn cell_id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn cell_ij(id: usize) -> (usize, usize) {
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

#[derive(Clone)]
struct Input {
    grid: [u8; CELL_COUNT],
    empty_ids: Vec<usize>,
    adj: [[Adj; 4]; CELL_COUNT],
    deg: [u8; CELL_COUNT],
}

impl Input {
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
                grid[cell_id(i, j)] = rows[i][j];
            }
        }

        let mut empty_ids = Vec::new();
        for id in 0..CELL_COUNT {
            if grid[id] == b'.' {
                empty_ids.push(id);
            }
        }

        let mut adj = [[Adj::INVALID; 4]; CELL_COUNT];
        let mut deg = [0_u8; CELL_COUNT];
        for id in 0..CELL_COUNT {
            if grid[id] != b'.' {
                continue;
            }
            let (i, j) = cell_ij(id);
            for &(di, dj) in &DIRS {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                    continue;
                }
                let nid = cell_id(ni as usize, nj as usize);
                if grid[nid] != b'.' {
                    continue;
                }
                let edge = match (di, dj) {
                    (1, 0) => h_edge_id(i, j),
                    (-1, 0) => h_edge_id(ni as usize, nj as usize),
                    (0, 1) => v_edge_id(i, j),
                    (0, -1) => v_edge_id(ni as usize, nj as usize),
                    _ => unreachable!(),
                };
                let p = deg[id] as usize;
                adj[id][p] = Adj {
                    to: nid as u16,
                    edge: edge as u16,
                };
                deg[id] += 1;
            }
        }

        Self {
            grid,
            empty_ids,
            adj,
            deg,
        }
    }

    #[inline(always)]
    fn is_empty(&self, id: usize) -> bool {
        self.grid[id] == b'.'
    }

    #[inline(always)]
    fn adjs(&self, id: usize) -> &[Adj] {
        &self.adj[id][..self.deg[id] as usize]
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
        let cur = self.door[edge];
        if cur == NO_DOOR {
            if self.door_count as usize >= M {
                return false;
            }
            self.door[edge] = g;
            self.door_count += 1;
            true
        } else {
            cur == g
        }
    }

    fn set_switch(&mut self, id: usize, s: u8) -> bool {
        if self.switch[id] != NO_SWITCH {
            return false;
        }
        self.switch[id] = s;
        true
    }

    #[inline(always)]
    fn edge_open(&self, edge: usize, mask: usize) -> bool {
        let g = self.door[edge];
        g == NO_DOOR || (((mask >> (g as usize / 2)) & 1) == (g as usize & 1))
    }

    fn output(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "{}", self.door_count).unwrap();
        for edge in 0..H_DOOR_COUNT {
            let g = self.door[edge];
            if g != NO_DOOR {
                writeln!(&mut out, "0 {} {} {}", edge / N, edge % N, g).unwrap();
            }
        }
        for edge in H_DOOR_COUNT..EDGE_COUNT {
            let g = self.door[edge];
            if g != NO_DOOR {
                let r = edge - H_DOOR_COUNT;
                writeln!(&mut out, "1 {} {} {}", r / (N - 1), r % (N - 1), g).unwrap();
            }
        }

        let switch_count = self.switch.iter().filter(|&&s| s != NO_SWITCH).count();
        writeln!(&mut out, "{}", switch_count).unwrap();
        for id in 0..CELL_COUNT {
            let s = self.switch[id];
            if s != NO_SWITCH {
                let (i, j) = cell_ij(id);
                writeln!(&mut out, "{} {} {}", i, j, s).unwrap();
            }
        }
        out
    }
}

#[derive(Clone)]
struct Candidate {
    plan: Plan,
    estimate: i64,
    s0: usize,
    p1: usize,
    branches: [usize; 8],
}

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

    fn usize(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }

    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            xs.swap(i, self.usize(i + 1));
        }
    }
}

struct TimeKeeper {
    start: Instant,
    limit: f64,
}

impl TimeKeeper {
    fn new(limit: f64) -> Self {
        Self {
            start: Instant::now(),
            limit,
        }
    }

    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn over_ratio(&self, ratio: f64) -> bool {
        self.elapsed() >= self.limit * ratio
    }
}

fn edge_between(a: usize, b: usize) -> Option<usize> {
    let (ai, aj) = cell_ij(a);
    let (bi, bj) = cell_ij(b);
    if ai == bi && aj + 1 == bj {
        Some(v_edge_id(ai, aj))
    } else if ai == bi && bj + 1 == aj {
        Some(v_edge_id(ai, bj))
    } else if aj == bj && ai + 1 == bi {
        Some(h_edge_id(ai, aj))
    } else if aj == bj && bi + 1 == ai {
        Some(h_edge_id(bi, aj))
    } else {
        None
    }
}

fn fixed_bfs(input: &Input, plan: &Plan, src: usize, mask: usize, dist: &mut [i16; CELL_COUNT]) {
    dist.fill(-1);
    let mut q = [0_usize; CELL_COUNT];
    let mut head = 0;
    let mut tail = 0;
    dist[src] = 0;
    q[tail] = src;
    tail += 1;
    while head < tail {
        let id = q[head];
        head += 1;
        let nd = dist[id] + 1;
        for &a in input.adjs(id) {
            let edge = a.edge as usize;
            if !plan.edge_open(edge, mask) {
                continue;
            }
            let to = a.to as usize;
            if dist[to] == -1 {
                dist[to] = nd;
                q[tail] = to;
                tail += 1;
            }
        }
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
}

fn calc_t(input: &Input, plan: &Plan, scratch: &mut EvalScratch) -> usize {
    scratch.dist.fill(UNREACHED);
    scratch.queue.clear();
    scratch.dist[START_ID] = 0;
    scratch.queue.push(START_ID as u32);

    let mut head = 0;
    while head < scratch.queue.len() {
        let packed = scratch.queue[head];
        head += 1;
        let mask = (packed >> 16) as usize;
        let id = (packed & 0xffff) as usize;
        let idx = mask * CELL_COUNT + id;
        let d = scratch.dist[idx];
        if id == GOAL_ID {
            return d as usize;
        }

        for &a in input.adjs(id) {
            let edge = a.edge as usize;
            if !plan.edge_open(edge, mask) {
                continue;
            }
            let to = a.to as usize;
            let nidx = mask * CELL_COUNT + to;
            if scratch.dist[nidx] == UNREACHED {
                scratch.dist[nidx] = d + 1;
                scratch.queue.push(((mask as u32) << 16) | to as u32);
            }
        }

        let s = plan.switch[id];
        if s != NO_SWITCH {
            let nmask = mask ^ (1_usize << s as usize);
            let nidx = nmask * CELL_COUNT + id;
            if scratch.dist[nidx] == UNREACHED {
                scratch.dist[nidx] = d + 1;
                scratch.queue.push(((nmask as u32) << 16) | id as u32);
            }
        }
    }
    0
}

fn generate_spine(input: &Input, rng: &mut Rng, target_len: usize) -> Option<Vec<usize>> {
    let mut used = [false; CELL_COUNT];
    let mut path = Vec::with_capacity(target_len);
    path.push(GOAL_ID);
    used[GOAL_ID] = true;

    while path.len() < target_len {
        let cur = *path.last().unwrap();
        let mut items = Vec::new();
        let mut total = 0_usize;
        for &a in input.adjs(cur) {
            let to = a.to as usize;
            if used[to] || to == START_ID {
                continue;
            }
            let mut contact = 0;
            let mut future = 0;
            for &b in input.adjs(to) {
                let v = b.to as usize;
                if used[v] {
                    contact += 1;
                } else if v != START_ID {
                    future += 1;
                }
            }
            let w = 2 + contact * 3 + future * 2 + rng.usize(4);
            items.push((to, w));
            total += w;
        }
        if items.is_empty() {
            break;
        }
        let mut r = rng.usize(total);
        let mut chosen = items[0].0;
        for (to, w) in items {
            if r < w {
                chosen = to;
                break;
            }
            r -= w;
        }
        used[chosen] = true;
        path.push(chosen);
    }

    if path.len() < 11 {
        return None;
    }
    path.reverse();
    Some(path)
}

fn branch_options(input: &Input, spine: &[usize]) -> Vec<Vec<usize>> {
    let mut on_spine = [false; CELL_COUNT];
    for &id in spine {
        on_spine[id] = true;
    }
    let mut opts = vec![Vec::new(); spine.len()];
    for (idx, &id) in spine.iter().enumerate() {
        for &a in input.adjs(id) {
            let to = a.to as usize;
            if !on_spine[to] {
                opts[idx].push(to);
            }
        }
    }
    opts
}

fn choose_controls(spine_len: usize, opts: &[Vec<usize>], rng: &mut Rng) -> Option<[usize; 9]> {
    if spine_len < 11 || opts[0].is_empty() {
        return None;
    }
    let mut controls = [0_usize; 9];
    controls[0] = 0;
    let mut prev = 0;
    for slot in 1..8 {
        let remaining = 7 - slot;
        let mut cand = Vec::new();
        for idx in (prev + 1)..(spine_len - 2) {
            if opts[idx].is_empty() {
                continue;
            }
            let after = ((idx + 1)..(spine_len - 2))
                .filter(|&j| !opts[j].is_empty())
                .count();
            if after >= remaining {
                cand.push(idx);
            }
        }
        if cand.is_empty() {
            return None;
        }
        cand.sort_by_key(|&idx| {
            let ideal = slot * (spine_len - 2) / 9;
            idx.abs_diff(ideal) + rng.usize(5)
        });
        let take = rng.usize(cand.len().min(4));
        controls[slot] = cand[take];
        prev = controls[slot];
    }

    if prev + 1 >= spine_len - 1 {
        return None;
    }
    let mut final_cand: Vec<usize> = ((prev + 1)..(spine_len - 1)).collect();
    final_cand.sort_by_key(|&idx| idx.abs_diff((prev + spine_len - 1) / 2) + rng.usize(6));
    controls[8] = final_cand[rng.usize(final_cand.len().min(5))];
    Some(controls)
}

fn assign_branches(opts: &[Vec<usize>], controls: &[usize; 9], rng: &mut Rng) -> Option<[usize; 8]> {
    let mut order: Vec<usize> = (0..8).collect();
    order.sort_by_key(|&i| opts[controls[i]].len());
    let mut branches = [usize::MAX; 8];
    let mut used = [false; CELL_COUNT];
    assign_branches_dfs(0, &order, opts, controls, rng, &mut used, &mut branches)
        .then_some(branches)
}

fn assign_branches_dfs(
    depth: usize,
    order: &[usize],
    opts: &[Vec<usize>],
    controls: &[usize; 9],
    rng: &mut Rng,
    used: &mut [bool; CELL_COUNT],
    branches: &mut [usize; 8],
) -> bool {
    if depth == order.len() {
        return true;
    }
    let k = order[depth];
    let mut cand = opts[controls[k]].clone();
    rng.shuffle(&mut cand);
    for id in cand {
        if used[id] {
            continue;
        }
        used[id] = true;
        branches[k] = id;
        if assign_branches_dfs(depth + 1, order, opts, controls, rng, used, branches) {
            return true;
        }
        branches[k] = usize::MAX;
        used[id] = false;
    }
    false
}

fn build_base(
    input: &Input,
    spine: &[usize],
    controls: &[usize; 9],
    branches: &[usize; 8],
) -> Option<(Plan, [bool; CELL_COUNT])> {
    let mut plan = Plan::empty();
    let mut free_edge = [false; EDGE_COUNT];
    for w in spine.windows(2) {
        free_edge[edge_between(w[0], w[1])?] = true;
    }
    for i in 0..8 {
        free_edge[edge_between(spine[controls[i]], branches[i])?] = true;
    }

    for i in 1..=8 {
        let edge = edge_between(spine[controls[i - 1]], spine[controls[i - 1] + 1])?;
        if !plan.add_door(edge, (2 * i) as u8) {
            return None;
        }
        let branch_edge = edge_between(spine[controls[i - 1]], branches[i - 1])?;
        if !plan.add_door(branch_edge, (2 * i + 1) as u8) {
            return None;
        }
        if !plan.set_switch(branches[i - 1], (i + 1) as u8) {
            return None;
        }
    }
    let final_edge = edge_between(spine[controls[8]], spine[controls[8] + 1])?;
    if !plan.add_door(final_edge, 19) {
        return None;
    }

    let mut in_u = [false; CELL_COUNT];
    for &id in &spine[1..] {
        in_u[id] = true;
    }
    for &id in branches {
        in_u[id] = true;
    }

    let mut in_r = in_u;
    in_r[spine[0]] = true;
    for id in 0..CELL_COUNT {
        if !in_u[id] {
            continue;
        }
        for &a in input.adjs(id) {
            in_r[a.to as usize] = true;
        }
    }
    loop {
        let mut add = Vec::new();
        for &id in &input.empty_ids {
            if in_r[id] {
                continue;
            }
            let mut cnt = 0;
            for &a in input.adjs(id) {
                if in_r[a.to as usize] {
                    cnt += 1;
                }
            }
            if cnt >= 3 {
                add.push(id);
            }
        }
        if add.is_empty() {
            break;
        }
        for id in add {
            in_r[id] = true;
        }
    }
    if in_r[START_ID] {
        return None;
    }

    for &id in &input.empty_ids {
        for &a in input.adjs(id) {
            let to = a.to as usize;
            if id < to && in_r[id] != in_r[to] && !plan.add_door(a.edge as usize, 0) {
                return None;
            }
        }
    }

    for &id in &input.empty_ids {
        for &a in input.adjs(id) {
            let to = a.to as usize;
            let edge = a.edge as usize;
            if id < to
                && (in_u[id] || in_u[to])
                && !free_edge[edge]
                && plan.door[edge] == NO_DOOR
                && !plan.add_door(edge, 1)
            {
                return None;
            }
        }
    }

    Some((plan, in_r))
}

fn add_p1_room(input: &Input, base: &Plan, in_r: &[bool; CELL_COUNT], p1: usize) -> Option<Plan> {
    if in_r[p1] || p1 == START_ID || p1 == GOAL_ID {
        return None;
    }
    for &a in input.adjs(p1) {
        if in_r[a.to as usize] {
            return None;
        }
    }

    let mut plan = base.clone();
    for &a in input.adjs(p1) {
        if !plan.add_door(a.edge as usize, 1) {
            return None;
        }
    }
    if !plan.set_switch(p1, 1) {
        return None;
    }
    Some(plan)
}

fn estimate_candidate(
    input: &Input,
    plan: &Plan,
    in_r: &[bool; CELL_COUNT],
    p1: usize,
    branches: &[usize; 8],
) -> Option<(i64, usize)> {
    let sources = make_sources(p1, branches);
    let mut dists = [[-1_i16; CELL_COUNT]; 11];
    for (idx, &(src, mask, _w)) in sources.iter().enumerate() {
        fixed_bfs(input, plan, src, mask, &mut dists[idx]);
    }

    let mut best_est = -1_i64;
    let mut best_s0 = usize::MAX;
    for &id in &input.empty_ids {
        if in_r[id] || id == START_ID || id == GOAL_ID || id == p1 {
            continue;
        }
        let mut est = 1023_i64;
        let mut ok = true;
        for (idx, &(_, _, w)) in sources.iter().enumerate() {
            let d = dists[idx][id];
            if d < 0 {
                ok = false;
                break;
            }
            est += w * d as i64;
        }
        if ok && est > best_est {
            best_est = est;
            best_s0 = id;
        }
    }
    (best_s0 != usize::MAX).then_some((best_est, best_s0))
}

fn make_sources(p1: usize, branches: &[usize; 8]) -> [(usize, usize, i64); 11] {
    [
        (START_ID, 0, 1),
        (p1, 1, 512),
        (branches[0], 2, 256),
        (branches[1], 4, 128),
        (branches[2], 8, 64),
        (branches[3], 16, 32),
        (branches[4], 32, 16),
        (branches[5], 64, 8),
        (branches[6], 128, 4),
        (branches[7], 256, 2),
        (GOAL_ID, 512, 1),
    ]
}

fn make_candidate(
    input: &Input,
    base: &Plan,
    in_r: &[bool; CELL_COUNT],
    p1: usize,
    branches: &[usize; 8],
) -> Option<Candidate> {
    let mut plan = add_p1_room(input, base, in_r, p1)?;
    let (estimate, s0) = estimate_candidate(input, &plan, in_r, p1, branches)?;
    if !plan.set_switch(s0, 0) {
        return None;
    }
    Some(Candidate {
        plan,
        estimate,
        s0,
        p1,
        branches: *branches,
    })
}

fn push_top(cands: &mut Vec<Candidate>, cand: Candidate, keep: usize) {
    cands.push(cand);
    if cands.len() > keep * 2 {
        cands.sort_by(|a, b| b.estimate.cmp(&a.estimate));
        cands.truncate(keep);
    }
}

fn collect_candidates(input: &Input, tk: &TimeKeeper, rng: &mut Rng) -> Vec<Candidate> {
    let mut cands = Vec::new();
    let mut iter = 0_usize;
    while !tk.over_ratio(0.84) {
        iter += 1;
        let target_len = 14 + rng.usize(23);
        let Some(spine) = generate_spine(input, rng, target_len) else {
            continue;
        };
        let opts = branch_options(input, &spine);
        let Some(controls) = choose_controls(spine.len(), &opts, rng) else {
            continue;
        };
        let Some(branches) = assign_branches(&opts, &controls, rng) else {
            continue;
        };
        let Some((base, in_r)) = build_base(input, &spine, &controls, &branches) else {
            continue;
        };

        let mut p1s = Vec::new();
        for &id in &input.empty_ids {
            if in_r[id] || id == START_ID || id == GOAL_ID {
                continue;
            }
            if input.adjs(id).iter().any(|a| in_r[a.to as usize]) {
                continue;
            }
            p1s.push(id);
        }
        if p1s.is_empty() {
            continue;
        }
        rng.shuffle(&mut p1s);
        let take = p1s.len().min(10 + rng.usize(8));
        for &p1 in &p1s[..take] {
            if let Some(cand) = make_candidate(input, &base, &in_r, p1, &branches) {
                push_top(&mut cands, cand, 80);
            }
            if tk.over_ratio(0.84) {
                break;
            }
        }
    }
    cands.sort_by(|a, b| b.estimate.cmp(&a.estimate));
    cands.truncate(80);
    local_log("generate_iter", iter as i64);
    cands
}

fn recompute_estimate(input: &Input, plan: &Plan, s0: usize, p1: usize, branches: &[usize; 8]) -> Option<i64> {
    let sources = make_sources(p1, branches);
    let mut dist = [-1_i16; CELL_COUNT];
    let mut est = 1023_i64;
    for &(src, mask, w) in &sources {
        fixed_bfs(input, plan, src, mask, &mut dist);
        let d = dist[s0];
        if d < 0 {
            return None;
        }
        est += w * d as i64;
    }
    Some(est)
}

fn shortest_edge_weights(input: &Input, cand: &Candidate) -> Vec<(i64, usize)> {
    let sources = make_sources(cand.p1, &cand.branches);
    let mut weight = [0_i64; EDGE_COUNT];
    let mut ds = [-1_i16; CELL_COUNT];
    let mut dt = [-1_i16; CELL_COUNT];
    for &(src, mask, w) in &sources {
        fixed_bfs(input, &cand.plan, src, mask, &mut ds);
        fixed_bfs(input, &cand.plan, cand.s0, mask, &mut dt);
        let d = ds[cand.s0];
        if d < 0 {
            continue;
        }
        for &id in &input.empty_ids {
            for &a in input.adjs(id) {
                let to = a.to as usize;
                if id >= to {
                    continue;
                }
                let edge = a.edge as usize;
                if cand.plan.door[edge] != NO_DOOR || !cand.plan.edge_open(edge, mask) {
                    continue;
                }
                if (ds[id] >= 0 && dt[to] >= 0 && ds[id] + 1 + dt[to] == d)
                    || (ds[to] >= 0 && dt[id] >= 0 && ds[to] + 1 + dt[id] == d)
                {
                    weight[edge] += w;
                }
            }
        }
    }
    let mut items = Vec::new();
    for (edge, &w) in weight.iter().enumerate() {
        if w > 0 {
            items.push((w, edge));
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items
}

fn improve_with_g19(input: &Input, original: &Candidate, tk: &TimeKeeper) -> Candidate {
    let mut cand = original.clone();
    while (cand.plan.door_count as usize) < M && !tk.over_ratio(0.95) {
        let edge_weights = shortest_edge_weights(input, &cand);
        if edge_weights.is_empty() {
            break;
        }
        let mut best: Option<(i64, usize)> = None;
        for &(_, edge) in edge_weights.iter().take(36) {
            let mut plan = cand.plan.clone();
            if !plan.add_door(edge, 19) {
                continue;
            }
            if let Some(est) = recompute_estimate(input, &plan, cand.s0, cand.p1, &cand.branches) {
                if est >= cand.estimate && best.map_or(true, |(b, _)| est > b) {
                    best = Some((est, edge));
                }
            }
            if tk.over_ratio(0.95) {
                break;
            }
        }
        let Some((est, edge)) = best else {
            break;
        };
        if !cand.plan.add_door(edge, 19) {
            break;
        }
        cand.estimate = est;
    }
    cand
}

fn local_log(_key: &str, _value: i64) {
    #[cfg(feature = "local")]
    {
        eprintln!("[trace] {}={}", _key, _value);
    }
}

fn solve(input: &Input) -> Plan {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64 ^ ((input.empty_ids.len() as u64) << 32);
    for &id in &input.empty_ids {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(id as u64 + 1442695040888963407);
    }
    let mut rng = Rng::new(seed);
    let tk = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);

    let mut candidates = collect_candidates(input, &tk, &mut rng);
    let base_count = candidates.len();
    if !candidates.is_empty() {
        let originals = candidates.clone();
        for cand in originals.iter().take(14) {
            if tk.over_ratio(0.95) {
                break;
            }
            let improved = improve_with_g19(input, cand, &tk);
            push_top(&mut candidates, improved, 100);
        }
    }
    candidates.sort_by(|a, b| b.estimate.cmp(&a.estimate));
    candidates.truncate(100);

    let mut scratch = EvalScratch::new();
    let mut best_t = 0_usize;
    let mut best_plan = Plan::empty();
    let mut evaluated = 0_i64;
    for cand in &candidates {
        if tk.over_ratio(0.995) && evaluated > 0 {
            break;
        }
        let t = calc_t(input, &cand.plan, &mut scratch);
        evaluated += 1;
        if t > best_t {
            best_t = t;
            best_plan = cand.plan.clone();
        }
    }
    local_log("base_candidates", base_count as i64);
    local_log("eval_candidates", evaluated);
    local_log("best_t", best_t as i64);
    best_plan
}

fn main() {
    let input = Input::read();
    let plan = solve(&input);
    print!("{}", plan.output());
}
