// v996_repro_from_explanation_v2.rs
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

const EST_MASKS: [usize; 11] = [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512];
const EST_WEIGHTS: [i32; 11] = [1, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1];

#[inline(always)]
fn id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn ij(v: usize) -> (usize, usize) {
    (v / N, v % N)
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
struct AdjEdge {
    to: u16,
    edge_id: u16,
}

impl AdjEdge {
    const INVALID: Self = Self {
        to: u16::MAX,
        edge_id: u16::MAX,
    };
}

#[derive(Clone)]
struct Input {
    grid: [u8; CELL_COUNT],
    empty_ids: Vec<usize>,
    adj: [[AdjEdge; 4]; CELL_COUNT],
    deg: [u8; CELL_COUNT],
    edge_ends: [(u16, u16); EDGE_COUNT],
    empty_edges: Vec<usize>,
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
                grid[id(i, j)] = rows[i][j];
            }
        }

        let mut empty_ids = Vec::new();
        for v in 0..CELL_COUNT {
            if grid[v] == b'.' {
                empty_ids.push(v);
            }
        }

        let mut edge_ends = [(u16::MAX, u16::MAX); EDGE_COUNT];
        for i in 0..N - 1 {
            for j in 0..N {
                let a = id(i, j);
                let b = id(i + 1, j);
                edge_ends[h_edge_id(i, j)] = (a as u16, b as u16);
            }
        }
        for i in 0..N {
            for j in 0..N - 1 {
                let a = id(i, j);
                let b = id(i, j + 1);
                edge_ends[v_edge_id(i, j)] = (a as u16, b as u16);
            }
        }

        let mut adj = [[AdjEdge::INVALID; 4]; CELL_COUNT];
        let mut deg = [0_u8; CELL_COUNT];
        let mut empty_edges = Vec::new();
        for i in 0..N {
            for j in 0..N {
                let a = id(i, j);
                if grid[a] != b'.' {
                    continue;
                }
                for &(di, dj) in &DIRS {
                    let ni = i as isize + di;
                    let nj = j as isize + dj;
                    if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                        continue;
                    }
                    let b = id(ni as usize, nj as usize);
                    if grid[b] != b'.' {
                        continue;
                    }
                    let edge_id = match (di, dj) {
                        (1, 0) => h_edge_id(i, j),
                        (-1, 0) => h_edge_id(ni as usize, nj as usize),
                        (0, 1) => v_edge_id(i, j),
                        (0, -1) => v_edge_id(ni as usize, nj as usize),
                        _ => unreachable!(),
                    };
                    let p = deg[a] as usize;
                    adj[a][p] = AdjEdge {
                        to: b as u16,
                        edge_id: edge_id as u16,
                    };
                    deg[a] += 1;
                    if a < b {
                        empty_edges.push(edge_id);
                    }
                }
            }
        }

        Self {
            grid,
            empty_ids,
            adj,
            deg,
            edge_ends,
            empty_edges,
        }
    }

    #[inline(always)]
    fn adj_edges(&self, v: usize) -> &[AdjEdge] {
        &self.adj[v][..self.deg[v] as usize]
    }

    fn edge_between(&self, a: usize, b: usize) -> Option<usize> {
        for e in self.adj_edges(a) {
            if e.to as usize == b {
                return Some(e.edge_id as usize);
            }
        }
        None
    }
}

#[derive(Clone)]
struct Plan {
    doors: [u8; EDGE_COUNT],
    switches: [u8; CELL_COUNT],
    door_count: u8,
}

impl Plan {
    fn empty() -> Self {
        Self {
            doors: [NO_DOOR; EDGE_COUNT],
            switches: [NO_SWITCH; CELL_COUNT],
            door_count: 0,
        }
    }

    fn set_door(&mut self, edge_id: usize, g: u8) -> bool {
        if self.doors[edge_id] == g {
            return true;
        }
        if self.doors[edge_id] != NO_DOOR || self.door_count as usize >= M {
            return false;
        }
        self.doors[edge_id] = g;
        self.door_count += 1;
        true
    }

    fn set_switch(&mut self, cell: usize, s: u8) -> bool {
        if self.switches[cell] != NO_SWITCH {
            return false;
        }
        self.switches[cell] = s;
        true
    }

    fn to_output_string(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "{}", self.door_count).unwrap();
        for e in 0..H_DOOR_COUNT {
            let g = self.doors[e];
            if g != NO_DOOR {
                writeln!(&mut out, "0 {} {} {}", e / N, e % N, g).unwrap();
            }
        }
        for e in H_DOOR_COUNT..EDGE_COUNT {
            let g = self.doors[e];
            if g != NO_DOOR {
                let r = e - H_DOOR_COUNT;
                writeln!(&mut out, "1 {} {} {}", r / (N - 1), r % (N - 1), g).unwrap();
            }
        }

        let switch_count = self.switches.iter().filter(|&&s| s != NO_SWITCH).count();
        writeln!(&mut out, "{}", switch_count).unwrap();
        for v in 0..CELL_COUNT {
            let s = self.switches[v];
            if s != NO_SWITCH {
                let (i, j) = ij(v);
                writeln!(&mut out, "{} {} {}", i, j, s).unwrap();
            }
        }
        out
    }

    #[inline(always)]
    fn can_pass(&self, edge_id: usize, mask: usize) -> bool {
        let g = self.doors[edge_id];
        g == NO_DOOR || ((mask >> (g as usize / 2)) & 1) == (g as usize & 1)
    }
}

struct Rng {
    x: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { x: seed | 1 }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    #[inline(always)]
    fn gen_usize(&mut self, n: usize) -> usize {
        if n <= 1 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    #[inline(always)]
    fn gen_range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.gen_usize(hi - lo)
    }

    fn shuffle<T>(&mut self, a: &mut [T]) {
        for i in (1..a.len()).rev() {
            let j = self.gen_usize(i + 1);
            a.swap(i, j);
        }
    }
}

struct TimeKeeper {
    start: Instant,
    limit: f64,
    iter: u64,
    mask: u64,
    progress: f64,
    over: bool,
}

impl TimeKeeper {
    fn new(limit: f64, check_interval_log2: u32) -> Self {
        let mut t = Self {
            start: Instant::now(),
            limit,
            iter: 0,
            mask: (1_u64 << check_interval_log2) - 1,
            progress: 0.0,
            over: false,
        };
        t.force_update();
        t
    }

    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.mask) == 0 {
            self.force_update();
        }
        !self.over
    }

    #[inline(always)]
    fn force_update(&mut self) {
        let e = self.start.elapsed().as_secs_f64();
        self.progress = (e / self.limit).clamp(0.0, 1.0);
        self.over = e >= self.limit;
    }
}

#[derive(Clone)]
struct Candidate {
    plan: Plan,
    estimate: i32,
    s0: usize,
    p1: usize,
    branches: [usize; 8],
    in_r: [bool; CELL_COUNT],
}

struct FixedBfs {
    dist: [i16; CELL_COUNT],
    que: [usize; CELL_COUNT],
}

impl FixedBfs {
    fn new() -> Self {
        Self {
            dist: [-1; CELL_COUNT],
            que: [0; CELL_COUNT],
        }
    }

    fn run(&mut self, input: &Input, plan: &Plan, src: usize, mask: usize) -> [i16; CELL_COUNT] {
        self.dist.fill(-1);
        let mut head = 0;
        let mut tail = 0;
        self.dist[src] = 0;
        self.que[tail] = src;
        tail += 1;
        while head < tail {
            let v = self.que[head];
            head += 1;
            let nd = self.dist[v] + 1;
            for &adj in input.adj_edges(v) {
                let e = adj.edge_id as usize;
                if !plan.can_pass(e, mask) {
                    continue;
                }
                let to = adj.to as usize;
                if self.dist[to] == -1 {
                    self.dist[to] = nd;
                    self.que[tail] = to;
                    tail += 1;
                }
            }
        }
        self.dist
    }
}

struct FullBfs {
    edge_mask: [u16; EDGE_COUNT],
    edge_open: [u16; EDGE_COUNT],
    dist: Vec<i32>,
    que: Vec<u32>,
}

impl FullBfs {
    fn new() -> Self {
        Self {
            edge_mask: [0; EDGE_COUNT],
            edge_open: [0; EDGE_COUNT],
            dist: vec![UNREACHED; HERO_STATE_COUNT],
            que: Vec::with_capacity(HERO_STATE_COUNT),
        }
    }

    fn calc_t(&mut self, input: &Input, plan: &Plan) -> usize {
        for e in 0..EDGE_COUNT {
            let g = plan.doors[e];
            if g == NO_DOOR {
                self.edge_mask[e] = 0;
                self.edge_open[e] = 0;
            } else {
                let bit = 1_u16 << (g as usize / 2);
                self.edge_mask[e] = bit;
                self.edge_open[e] = if (g & 1) == 1 { bit } else { 0 };
            }
        }
        self.dist.fill(UNREACHED);
        self.que.clear();
        self.dist[START_ID] = 0;
        self.que.push(START_ID as u32);
        let mut head = 0;
        while head < self.que.len() {
            let p = self.que[head];
            head += 1;
            let mask = (p >> 16) as usize;
            let v = (p & 0xffff) as usize;
            let idx = mask * CELL_COUNT + v;
            let d = self.dist[idx];
            if v == GOAL_ID {
                return d as usize;
            }
            for &adj in input.adj_edges(v) {
                let e = adj.edge_id as usize;
                if ((mask as u16) & self.edge_mask[e]) != self.edge_open[e] {
                    continue;
                }
                let to = adj.to as usize;
                let ni = mask * CELL_COUNT + to;
                if self.dist[ni] == UNREACHED {
                    self.dist[ni] = d + 1;
                    self.que.push(((mask as u32) << 16) | to as u32);
                }
            }
            let s = plan.switches[v];
            if s != NO_SWITCH {
                let nm = mask ^ (1_usize << s as usize);
                let ni = nm * CELL_COUNT + v;
                if self.dist[ni] == UNREACHED {
                    self.dist[ni] = d + 1;
                    self.que.push(((nm as u32) << 16) | v as u32);
                }
            }
        }
        0
    }
}

fn make_seed(input: &Input) -> u64 {
    let mut h = 0x9e37_79b9_7f4a_7c15_u64;
    for v in 0..CELL_COUNT {
        h ^= (input.grid[v] as u64).wrapping_mul((v as u64 + 3) * 0x1000_0000_01b3);
        h = h.rotate_left(11);
    }
    h
}

fn make_spine(input: &Input, rng: &mut Rng, len: usize) -> Option<Vec<usize>> {
    let mut path = vec![GOAL_ID];
    let mut used = [false; CELL_COUNT];
    used[GOAL_ID] = true;
    while path.len() < len {
        let v = *path.last().unwrap();
        let mut cand = Vec::new();
        for &adj in input.adj_edges(v) {
            let to = adj.to as usize;
            if used[to] || to == START_ID {
                continue;
            }
            let mut touch = 0_i32;
            let mut free = 0_i32;
            for &adj2 in input.adj_edges(to) {
                let u = adj2.to as usize;
                if used[u] {
                    touch += 1;
                } else {
                    free += 1;
                }
            }
            if free == 0 && path.len() + 1 < len {
                continue;
            }
            let w = 2 + touch * 2 + free + rng.gen_usize(5) as i32;
            cand.push((to, w.max(1) as usize));
        }
        if cand.is_empty() {
            return None;
        }
        let sum: usize = cand.iter().map(|&(_, w)| w).sum();
        let mut r = rng.gen_usize(sum);
        let mut chosen = cand[0].0;
        for (to, w) in cand {
            if r < w {
                chosen = to;
                break;
            }
            r -= w;
        }
        used[chosen] = true;
        path.push(chosen);
    }
    path.reverse();
    Some(path)
}

fn branch_candidates(input: &Input, spine: &[usize]) -> Vec<Vec<usize>> {
    let mut on_spine = [false; CELL_COUNT];
    for &v in spine {
        on_spine[v] = true;
    }
    let mut out = vec![Vec::new(); spine.len()];
    for (idx, &v) in spine.iter().enumerate() {
        for &adj in input.adj_edges(v) {
            let to = adj.to as usize;
            if !on_spine[to] && to != START_ID && to != GOAL_ID {
                out[idx].push(to);
            }
        }
    }
    out
}

fn choose_positions(
    spine_len: usize,
    cand: &[Vec<usize>],
    rng: &mut Rng,
) -> Option<[usize; 9]> {
    if spine_len < 10 || cand[0].is_empty() {
        return None;
    }
    let mut branchable = Vec::new();
    for i in 1..spine_len - 2 {
        if !cand[i].is_empty() {
            branchable.push(i);
        }
    }
    if branchable.len() < 7 {
        return None;
    }

    let mut pos = [0_usize; 9];
    pos[0] = 0;
    let mut prev = 0;
    for slot in 1..=7 {
        let remain = 7 - slot;
        let target = slot * (spine_len - 2) / 9;
        let mut best: Option<(i32, usize)> = None;
        for &x in &branchable {
            if x <= prev || x >= spine_len - 2 {
                continue;
            }
            let after = branchable.iter().filter(|&&y| y > x).count();
            if after < remain {
                continue;
            }
            let key = (x as i32 - target as i32).abs() + rng.gen_usize(7) as i32;
            if best.map_or(true, |(bk, _)| key < bk) {
                best = Some((key, x));
            }
        }
        let x = best?.1;
        pos[slot] = x;
        prev = x;
    }
    if prev + 1 >= spine_len - 1 {
        return None;
    }
    pos[8] = rng.gen_range(prev + 1, spine_len - 1);
    Some(pos)
}

fn assign_branches(
    pos: &[usize; 9],
    cand: &[Vec<usize>],
    rng: &mut Rng,
) -> Option<[usize; 8]> {
    let mut order: Vec<usize> = (0..8).collect();
    order.sort_by_key(|&t| cand[pos[t]].len());
    let mut branches = [usize::MAX; 8];
    let mut used = [false; CELL_COUNT];

    fn dfs(
        at: usize,
        order: &[usize],
        pos: &[usize; 9],
        cand: &[Vec<usize>],
        used: &mut [bool; CELL_COUNT],
        branches: &mut [usize; 8],
        rng: &mut Rng,
    ) -> bool {
        if at == order.len() {
            return true;
        }
        let t = order[at];
        let mut choices = cand[pos[t]].clone();
        rng.shuffle(&mut choices);
        for b in choices {
            if used[b] {
                continue;
            }
            used[b] = true;
            branches[t] = b;
            if dfs(at + 1, order, pos, cand, used, branches, rng) {
                return true;
            }
            used[b] = false;
            branches[t] = usize::MAX;
        }
        false
    }

    if dfs(0, &order, pos, cand, &mut used, &mut branches, rng) {
        Some(branches)
    } else {
        None
    }
}

fn build_regions(
    input: &Input,
    spine: &[usize],
    branches: &[usize; 8],
) -> Option<([bool; CELL_COUNT], [bool; CELL_COUNT])> {
    let mut in_u = [false; CELL_COUNT];
    for &v in spine.iter().skip(1) {
        in_u[v] = true;
    }
    for &b in branches {
        in_u[b] = true;
    }
    if in_u[START_ID] {
        return None;
    }

    let mut in_r = in_u;
    in_r[spine[0]] = true;
    for v in 0..CELL_COUNT {
        if !in_u[v] {
            continue;
        }
        for &adj in input.adj_edges(v) {
            in_r[adj.to as usize] = true;
        }
    }
    loop {
        let mut changed = false;
        for &v in &input.empty_ids {
            if in_r[v] {
                continue;
            }
            let mut c = 0;
            for &adj in input.adj_edges(v) {
                if in_r[adj.to as usize] {
                    c += 1;
                }
            }
            if c >= 3 {
                in_r[v] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if in_r[START_ID] {
        return None;
    }
    Some((in_u, in_r))
}

fn add_structure_doors(
    input: &Input,
    spine: &[usize],
    pos: &[usize; 9],
    branches: &[usize; 8],
    in_u: &[bool; CELL_COUNT],
    in_r: &[bool; CELL_COUNT],
) -> Option<(Plan, [bool; EDGE_COUNT])> {
    let mut plan = Plan::empty();
    let mut protected = [false; EDGE_COUNT];
    for w in spine.windows(2) {
        let e = input.edge_between(w[0], w[1])?;
        protected[e] = true;
    }
    for t in 0..8 {
        let e = input.edge_between(spine[pos[t]], branches[t])?;
        protected[e] = true;
    }

    for t in 0..8 {
        let e1 = input.edge_between(spine[pos[t]], spine[pos[t] + 1])?;
        if !plan.set_door(e1, (2 * (t + 1)) as u8) {
            return None;
        }
        let e2 = input.edge_between(spine[pos[t]], branches[t])?;
        if !plan.set_door(e2, (2 * (t + 1) + 1) as u8) {
            return None;
        }
    }
    let last = input.edge_between(spine[pos[8]], spine[pos[8] + 1])?;
    if !plan.set_door(last, 19) {
        return None;
    }

    for &e in &input.empty_edges {
        let (a, b) = input.edge_ends[e];
        let a = a as usize;
        let b = b as usize;
        if in_r[a] ^ in_r[b] {
            if !plan.set_door(e, 0) {
                return None;
            }
        }
    }

    let mut extras = Vec::new();
    for &e in &input.empty_edges {
        if plan.doors[e] != NO_DOOR || protected[e] {
            continue;
        }
        let (a, b) = input.edge_ends[e];
        if in_u[a as usize] || in_u[b as usize] {
            extras.push(e);
        }
    }
    for e in extras {
        if !plan.set_door(e, 1) {
            return None;
        }
    }

    Some((plan, protected))
}

fn p1_candidates(input: &Input, in_r: &[bool; CELL_COUNT]) -> Vec<usize> {
    let mut out = Vec::new();
    'cell: for &v in &input.empty_ids {
        if v == START_ID || v == GOAL_ID || in_r[v] {
            continue;
        }
        for &adj in input.adj_edges(v) {
            if in_r[adj.to as usize] {
                continue 'cell;
            }
        }
        out.push(v);
    }
    out
}

fn add_p1_doors(input: &Input, base: &Plan, p1: usize) -> Option<Plan> {
    let mut plan = base.clone();
    for &adj in input.adj_edges(p1) {
        if !plan.set_door(adj.edge_id as usize, 1) {
            return None;
        }
    }
    Some(plan)
}

fn estimate_and_choose_s0(
    input: &Input,
    plan: &Plan,
    in_r: &[bool; CELL_COUNT],
    p1: usize,
    branches: &[usize; 8],
    fbfs: &mut FixedBfs,
) -> Option<(i32, usize)> {
    let mut sources = [0_usize; 11];
    sources[0] = START_ID;
    sources[1] = p1;
    sources[2..10].copy_from_slice(branches);
    sources[10] = GOAL_ID;

    let mut dists = [[-1_i16; CELL_COUNT]; 11];
    for t in 0..11 {
        dists[t] = fbfs.run(input, plan, sources[t], EST_MASKS[t]);
    }

    let mut best_est = i32::MIN;
    let mut best_s0 = usize::MAX;
    'cell: for &s0 in &input.empty_ids {
        if in_r[s0] || s0 == START_ID || s0 == GOAL_ID || s0 == p1 {
            continue;
        }
        for &b in branches {
            if s0 == b {
                continue 'cell;
            }
        }
        let mut est = 1023_i32;
        for t in 0..11 {
            let d = dists[t][s0];
            if d < 0 {
                continue 'cell;
            }
            est += EST_WEIGHTS[t] * d as i32;
        }
        if est > best_est {
            best_est = est;
            best_s0 = s0;
        }
    }
    if best_s0 == usize::MAX {
        None
    } else {
        Some((best_est, best_s0))
    }
}

fn complete_candidate(
    input: &Input,
    base: &Plan,
    in_r: &[bool; CELL_COUNT],
    p1: usize,
    branches: &[usize; 8],
    fbfs: &mut FixedBfs,
) -> Option<Candidate> {
    let mut plan = add_p1_doors(input, base, p1)?;
    let (estimate, s0) = estimate_and_choose_s0(input, &plan, in_r, p1, branches, fbfs)?;
    if !plan.set_switch(s0, 0) || !plan.set_switch(p1, 1) {
        return None;
    }
    for t in 0..8 {
        if !plan.set_switch(branches[t], (t + 2) as u8) {
            return None;
        }
    }
    Some(Candidate {
        plan,
        estimate,
        s0,
        p1,
        branches: *branches,
        in_r: *in_r,
    })
}

fn build_candidate(input: &Input, rng: &mut Rng, fbfs: &mut FixedBfs) -> Option<Candidate> {
    let len = rng.gen_range(12, 31);
    let spine = make_spine(input, rng, len)?;
    let bc = branch_candidates(input, &spine);
    let pos = choose_positions(spine.len(), &bc, rng)?;
    let branches = assign_branches(&pos, &bc, rng)?;
    let (in_u, in_r) = build_regions(input, &spine, &branches)?;
    let (base, _) = add_structure_doors(input, &spine, &pos, &branches, &in_u, &in_r)?;
    if base.door_count as usize >= M - 1 {
        return None;
    }

    let mut p1s = p1_candidates(input, &in_r);
    if p1s.is_empty() {
        return None;
    }
    rng.shuffle(&mut p1s);
    if p1s.len() > 48 {
        p1s.truncate(48);
    }

    let mut best: Option<Candidate> = None;
    for p1 in p1s {
        if let Some(cand) = complete_candidate(input, &base, &in_r, p1, &branches, fbfs) {
            if best.as_ref().map_or(true, |b| cand.estimate > b.estimate) {
                best = Some(cand);
            }
        }
    }
    best
}

fn recompute_estimate(input: &Input, cand: &Candidate, fbfs: &mut FixedBfs) -> Option<i32> {
    estimate_and_choose_s0(
        input,
        &cand.plan,
        &cand.in_r,
        cand.p1,
        &cand.branches,
        fbfs,
    )
    .and_then(|(est, s0)| if s0 == cand.s0 { Some(est) } else { Some(est) })
}

fn add_type19_extra(input: &Input, cand: &mut Candidate, fbfs: &mut FixedBfs, tk: &mut TimeKeeper) {
    while (cand.plan.door_count as usize) < M && tk.progress < 0.95 {
        tk.force_update();
        if tk.over {
            break;
        }
        let mut sources = [0_usize; 11];
        sources[0] = START_ID;
        sources[1] = cand.p1;
        sources[2..10].copy_from_slice(&cand.branches);
        sources[10] = GOAL_ID;

        let mut edge_score = [0_i32; EDGE_COUNT];
        for t in 0..11 {
            let ds = fbfs.run(input, &cand.plan, sources[t], EST_MASKS[t]);
            let d = ds[cand.s0];
            if d < 0 {
                continue;
            }
            let dt = fbfs.run(input, &cand.plan, cand.s0, EST_MASKS[t]);
            for &e in &input.empty_edges {
                if cand.plan.doors[e] != NO_DOOR {
                    continue;
                }
                let (a, b) = input.edge_ends[e];
                let a = a as usize;
                let b = b as usize;
                let on_path = (ds[a] >= 0 && dt[b] >= 0 && ds[a] + 1 + dt[b] == d)
                    || (ds[b] >= 0 && dt[a] >= 0 && ds[b] + 1 + dt[a] == d);
                if on_path {
                    edge_score[e] += EST_WEIGHTS[t];
                }
            }
        }

        let mut edges: Vec<usize> = input
            .empty_edges
            .iter()
            .copied()
            .filter(|&e| cand.plan.doors[e] == NO_DOOR && edge_score[e] > 0)
            .collect();
        if edges.is_empty() {
            break;
        }
        edges.sort_by_key(|&e| -edge_score[e]);
        edges.truncate(40);

        let old_est = cand.estimate;
        let mut best_plan: Option<(Plan, i32)> = None;
        for e in edges {
            let mut trial = cand.plan.clone();
            if !trial.set_door(e, 19) {
                continue;
            }
            let trial_cand = Candidate {
                plan: trial,
                estimate: cand.estimate,
                s0: cand.s0,
                p1: cand.p1,
                branches: cand.branches,
                in_r: cand.in_r,
            };
            if let Some(est) = recompute_estimate(input, &trial_cand, fbfs) {
                if est >= old_est && best_plan.as_ref().map_or(true, |(_, be)| est > *be) {
                    best_plan = Some((trial_cand.plan, est));
                }
            }
        }
        if let Some((plan, est)) = best_plan {
            cand.plan = plan;
            cand.estimate = est;
        } else {
            break;
        }
    }
}

fn push_pool(pool: &mut Vec<Candidate>, cand: Candidate) {
    pool.push(cand);
    if pool.len() > 160 {
        pool.sort_by_key(|c| -c.estimate);
        pool.truncate(128);
    }
}

fn solve(input: &Input) -> Plan {
    let mut rng = Rng::new(make_seed(input) ^ 0x243f_6a88_85a3_08d3);
    let mut tk = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 10);
    let mut fbfs = FixedBfs::new();
    let mut pool: Vec<Candidate> = Vec::new();

    while tk.step() && tk.progress < 0.86 {
        if let Some(cand) = build_candidate(input, &mut rng, &mut fbfs) {
            push_pool(&mut pool, cand);
        }
    }

    pool.sort_by_key(|c| -c.estimate);
    pool.truncate(128);
    let mut eval_pool = pool.clone();
    let mut extra_pool: Vec<Candidate> = pool.iter().take(32).cloned().collect();
    for cand in &mut extra_pool {
        if tk.progress >= 0.95 {
            break;
        }
        add_type19_extra(input, cand, &mut fbfs, &mut tk);
    }

    eval_pool.extend(extra_pool);
    eval_pool.sort_by_key(|c| -c.estimate);
    eval_pool.truncate(144);
    let mut full = FullBfs::new();
    let mut best_t = 0_usize;
    let mut best_plan = Plan::empty();
    for cand in &eval_pool {
        let t = full.calc_t(input, &cand.plan);
        if t > best_t {
            best_t = t;
            best_plan = cand.plan.clone();
        }
    }

    #[cfg(feature = "local")]
    eprintln!(
        "[summary] candidates={} best_t={} elapsed={:.3}",
        eval_pool.len(),
        best_t,
        tk.start.elapsed().as_secs_f64()
    );

    best_plan
}

fn main() {
    let input = Input::read();
    let plan = solve(&input);
    print!("{}", plan.to_output_string());
}
