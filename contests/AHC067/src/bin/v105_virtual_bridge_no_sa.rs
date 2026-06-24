// v105_virtual_bridge_no_sa.rs
#![allow(dead_code)]

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
const K: usize = 10;
const M: usize = 50;
const C: usize = N * N;
const MASKS: usize = 1 << K;
const STATES: usize = MASKS * C;
const START: usize = 0;
const GOAL: usize = C - 1;

const CHINESE_LOOP_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.68 / JUDGE_TIME_LIMIT_SEC);
const GREEDY_SEARCH_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.74 / JUDGE_TIME_LIMIT_SEC);
const CELL_CHAIN_CAND_LIMIT: usize = 110;
const CELL_CHAIN_BEAM_WIDTH: usize = 240;
const INF: i32 = 1_000_000_000;

#[derive(Debug, Clone, Copy)]
struct Timer {
    st: Instant,
}

impl Timer {
    fn new() -> Self {
        Self { st: Instant::now() }
    }

    #[inline(always)]
    fn sec(&self) -> f64 {
        self.st.elapsed().as_secs_f64()
    }
}

#[derive(Debug, Clone, Copy)]
struct XorShift {
    x: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        Self { x: seed }
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn next_int(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    #[inline(always)]
    fn next_double(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct Edge {
    a: usize,
    b: usize,
    dir: usize,
    i: usize,
    j: usize,
}

#[derive(Debug, Clone)]
struct Candidate {
    comp: usize,
    cell: usize,
    tail: Vec<usize>,
    full: Vec<usize>,
    len: usize,
    depth: usize,
}

#[derive(Debug, Clone)]
struct Solution {
    t: usize,
    doors: Vec<(usize, usize)>,
    sw: [i32; C],
    n: usize,
}

impl Solution {
    fn new() -> Self {
        Self {
            t: 0,
            doors: Vec::new(),
            sw: [-1; C],
            n: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct BridgeTree {
    comp_of: Vec<usize>,
    comps: Vec<Vec<usize>>,
    tree: Vec<Vec<(usize, usize)>>,
}

#[derive(Debug, Clone, Copy)]
struct BeamState {
    score: i32,
    val: i32,
    last: usize,
    len: usize,
    seq: [i32; K],
}

impl BeamState {
    fn new(base_val: i32) -> Self {
        Self {
            score: 0,
            val: base_val,
            last: START,
            len: 0,
            seq: [-1; K],
        }
    }
}

#[derive(Debug, Clone)]
struct Solver {
    grid: [u8; C],
    edges: Vec<Edge>,
    adj: Vec<Vec<(usize, usize)>>,
    edge_mask: Vec<usize>,
    edge_open: Vec<usize>,
    seen: Vec<i32>,
    dist: Vec<i32>,
    queue: Vec<usize>,
    bfs_stamp: i32,
}

impl Solver {
    fn new(grid: [u8; C]) -> Self {
        let mut solver = Self {
            grid,
            edges: Vec::new(),
            adj: vec![Vec::new(); C],
            edge_mask: Vec::new(),
            edge_open: Vec::new(),
            seen: vec![0; STATES],
            dist: vec![0; STATES],
            queue: Vec::with_capacity(STATES),
            bfs_stamp: 1,
        };
        solver.build_graph();
        solver
    }

    #[inline(always)]
    fn id(i: usize, j: usize) -> usize {
        i * N + j
    }

    fn build_graph(&mut self) {
        self.edges.clear();
        for v in &mut self.adj {
            v.clear();
        }

        for i in 0..N {
            for j in 0..N {
                if self.grid[Self::id(i, j)] != b'.' {
                    continue;
                }

                if i + 1 < N && self.grid[Self::id(i + 1, j)] == b'.' {
                    let a = Self::id(i, j);
                    let b = Self::id(i + 1, j);
                    let e = self.edges.len();
                    self.edges.push(Edge { a, b, dir: 0, i, j });
                    self.adj[a].push((b, e));
                    self.adj[b].push((a, e));
                }

                if j + 1 < N && self.grid[Self::id(i, j + 1)] == b'.' {
                    let a = Self::id(i, j);
                    let b = Self::id(i, j + 1);
                    let e = self.edges.len();
                    self.edges.push(Edge { a, b, dir: 1, i, j });
                    self.adj[a].push((b, e));
                    self.adj[b].push((a, e));
                }
            }
        }

        self.edge_mask.resize(self.edges.len(), 0);
        self.edge_open.resize(self.edges.len(), 0);
    }

    fn calc_t(&mut self, doors: &[(usize, usize)], sw: &[i32; C]) -> usize {
        self.edge_mask.fill(0);
        self.edge_open.fill(0);

        for &(e, g) in doors {
            let bit = 1usize << (g >> 1);
            self.edge_mask[e] = bit;
            self.edge_open[e] = if (g & 1) == 1 { bit } else { 0 };
        }

        let stamp = self.bfs_stamp;
        self.bfs_stamp += 1;
        if self.bfs_stamp == i32::MAX {
            self.seen.fill(0);
            self.bfs_stamp = 2;
        }

        let mut head = 0usize;
        self.queue.clear();
        self.seen[START] = stamp;
        self.dist[START] = 0;
        self.queue.push(START);

        while head < self.queue.len() {
            let state = self.queue[head];
            head += 1;

            let mask = state / C;
            let v = state - mask * C;
            let d = self.dist[state];

            if v == GOAL {
                return d as usize;
            }

            for &(to, e) in &self.adj[v] {
                if (mask & self.edge_mask[e]) != self.edge_open[e] {
                    continue;
                }
                let ns = mask * C + to;
                if self.seen[ns] != stamp {
                    self.seen[ns] = stamp;
                    self.dist[ns] = d + 1;
                    self.queue.push(ns);
                }
            }

            let s = sw[v];
            if s >= 0 {
                let nm = mask ^ (1usize << s as usize);
                let ns = nm * C + v;
                if self.seen[ns] != stamp {
                    self.seen[ns] = stamp;
                    self.dist[ns] = d + 1;
                    self.queue.push(ns);
                }
            }
        }

        0
    }

    fn find_bridges(&self) -> Vec<bool> {
        fn dfs(
            solver: &Solver,
            v: usize,
            pe: usize,
            tin: &mut [i32],
            low: &mut [i32],
            timer: &mut i32,
            is_bridge: &mut [bool],
        ) {
            tin[v] = *timer;
            low[v] = *timer;
            *timer += 1;

            for &(to, e) in &solver.adj[v] {
                if e == pe {
                    continue;
                }
                if tin[to] >= 0 {
                    low[v] = low[v].min(tin[to]);
                } else {
                    dfs(solver, to, e, tin, low, timer, is_bridge);
                    low[v] = low[v].min(low[to]);
                    if low[to] > tin[v] {
                        is_bridge[e] = true;
                    }
                }
            }
        }

        let mut is_bridge = vec![false; self.edges.len()];
        let mut tin = [-1; C];
        let mut low = [0; C];
        let mut timer = 0;
        dfs(
            self,
            START,
            usize::MAX,
            &mut tin,
            &mut low,
            &mut timer,
            &mut is_bridge,
        );
        is_bridge
    }

    fn build_bridge_tree(&self, is_bridge: &[bool]) -> BridgeTree {
        let mut bt = BridgeTree {
            comp_of: vec![usize::MAX; C],
            comps: Vec::new(),
            tree: Vec::new(),
        };

        for s in 0..C {
            if self.grid[s] != b'.' || bt.comp_of[s] != usize::MAX {
                continue;
            }

            let cid = bt.comps.len();
            bt.comps.push(Vec::new());
            let mut st = vec![s];
            bt.comp_of[s] = cid;

            let mut qi = 0usize;
            while qi < st.len() {
                let v = st[qi];
                qi += 1;
                bt.comps[cid].push(v);

                for &(to, e) in &self.adj[v] {
                    if is_bridge[e] {
                        continue;
                    }
                    if bt.comp_of[to] == usize::MAX {
                        bt.comp_of[to] = cid;
                        st.push(to);
                    }
                }
            }
        }

        let b = bt.comps.len();
        bt.tree = vec![Vec::new(); b];

        for (e, edge) in self.edges.iter().enumerate() {
            if !is_bridge[e] {
                continue;
            }
            let ca = bt.comp_of[edge.a];
            let cb = bt.comp_of[edge.b];
            if ca == cb || ca == usize::MAX || cb == usize::MAX {
                continue;
            }
            bt.tree[ca].push((cb, e));
            bt.tree[cb].push((ca, e));
        }

        bt
    }

    fn farthest_cell_in_comp_from_parent(
        &self,
        bt: &BridgeTree,
        comp: usize,
        parent_edge: Option<usize>,
    ) -> usize {
        let Some(parent_edge) = parent_edge else {
            return bt.comps[comp][0];
        };

        let edge = self.edges[parent_edge];
        let src = if bt.comp_of[edge.a] == comp {
            edge.a
        } else {
            edge.b
        };

        let mut d = [-1; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        d[src] = 0;
        q[t] = src;
        t += 1;
        let mut best = src;

        while h < t {
            let v = q[h];
            h += 1;

            if d[v] > d[best] {
                best = v;
            }

            for &(to, _) in &self.adj[v] {
                if bt.comp_of[to] != comp {
                    continue;
                }
                if d[to] == -1 {
                    d[to] = d[v] + 1;
                    q[t] = to;
                    t += 1;
                }
            }
        }

        best
    }

    fn collect_candidates(
        &self,
        bt: &BridgeTree,
        parent: &[usize],
        pedge: &[usize],
        path: &[usize],
        t: usize,
    ) -> Vec<Candidate> {
        let b = bt.tree.len();
        let mut goal_sub = vec![false; b];

        let final_child = path[t];
        let mut st = vec![final_child];
        goal_sub[final_child] = true;

        let mut qi = 0usize;
        while qi < st.len() {
            let v = st[qi];
            qi += 1;
            for &(to, _) in &bt.tree[v] {
                if parent[to] == v {
                    goal_sub[to] = true;
                    st.push(to);
                }
            }
        }

        let mut prefix = vec![false; b];
        for v in 0..b {
            if !goal_sub[v] {
                prefix[v] = true;
            }
        }

        let mut main_edge = vec![false; self.edges.len()];
        for i in 1..t {
            if pedge[path[i]] != usize::MAX {
                main_edge[pedge[path[i]]] = true;
            }
        }

        let root = path[0];
        let mut cands = Vec::new();

        for v in 0..b {
            if !prefix[v] || v == root {
                continue;
            }

            let mut child_cnt = 0usize;
            for &(to, _) in &bt.tree[v] {
                if parent[to] == v && prefix[to] {
                    child_cnt += 1;
                }
            }
            if child_cnt != 0 {
                continue;
            }

            let mut tail = Vec::new();
            let mut x = v;
            while x != root {
                let e = pedge[x];
                if e == usize::MAX || main_edge[e] {
                    break;
                }
                tail.push(e);
                x = parent[x];
            }
            if tail.is_empty() {
                continue;
            }

            let mut full = Vec::new();
            x = v;
            while x != root {
                let e = pedge[x];
                if e == usize::MAX {
                    break;
                }
                full.push(e);
                x = parent[x];
            }

            let len = tail.len();
            let depth = full.len();
            let cell = self.farthest_cell_in_comp_from_parent(bt, v, Some(pedge[v]));

            cands.push(Candidate {
                comp: v,
                cell,
                tail,
                full,
                len,
                depth,
            });
        }

        cands
    }

    fn build_from_assignment(
        &mut self,
        cands: &[Candidate],
        assign: &[usize],
        n: usize,
        final_edge: usize,
    ) -> Solution {
        let mut sol = Solution::new();
        sol.n = n;
        let mut door_kind = vec![-1; self.edges.len()];

        for i in 0..n {
            let c = &cands[assign[i]];
            sol.sw[c.cell] = i as i32;

            if i > 0 {
                for j in 0..(i - 1) {
                    door_kind[c.tail[j]] = (2 * j) as i32;
                }
                door_kind[c.tail[i - 1]] = (2 * (i - 1) + 1) as i32;
            }
        }

        door_kind[final_edge] = (2 * (n - 1) + 1) as i32;

        for (e, &g) in door_kind.iter().enumerate() {
            if g >= 0 {
                sol.doors.push((e, g as usize));
            }
        }

        if sol.doors.len() > M {
            sol.t = 0;
        } else {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }

        sol
    }

    fn solve_chinese(&mut self, timer: &Timer) -> Solution {
        let mut best = Solution::new();

        let is_bridge = self.find_bridges();
        let bt = self.build_bridge_tree(&is_bridge);

        let root = bt.comp_of[START];
        let goal = bt.comp_of[GOAL];
        let b = bt.tree.len();
        if root == usize::MAX || goal == usize::MAX {
            return best;
        }

        let mut parent = vec![usize::MAX; b];
        let mut pedge = vec![usize::MAX; b];
        let mut order = vec![root];
        parent[root] = root;

        let mut qi = 0usize;
        while qi < order.len() {
            let v = order[qi];
            qi += 1;
            for &(to, e) in &bt.tree[v] {
                if parent[to] == usize::MAX {
                    parent[to] = v;
                    pedge[to] = e;
                    order.push(to);
                }
            }
        }

        if parent[goal] == usize::MAX || root == goal {
            return best;
        }

        let mut path = Vec::new();
        let mut v = goal;
        while v != root {
            path.push(v);
            v = parent[v];
        }
        path.push(root);
        path.reverse();

        let mut rng = XorShift::new(
            123_456_789u64 + self.edges.len() as u64 * 1009 + bt.comps.len() as u64 * 9176,
        );

        for t in 1..path.len() {
            if timer.sec() > CHINESE_LOOP_LIMIT_SEC {
                break;
            }

            let final_edge = pedge[path[t]];
            let cands = self.collect_candidates(&bt, &parent, &pedge, &path, t);
            if cands.is_empty() {
                continue;
            }

            for n in (2..=10).rev() {
                if n * (n - 1) / 2 + 1 > M {
                    continue;
                }

                let attempts = match n {
                    10 => 260,
                    9 => 220,
                    8 => 170,
                    7 => 110,
                    _ => 65,
                };

                for mode in 0..attempts {
                    if timer.sec() > GREEDY_SEARCH_LIMIT_SEC {
                        break;
                    }

                    let real_mode = if mode < 5 { mode } else { 4 };
                    let mut assign = Vec::new();
                    if !make_assignment(&self.edges, &cands, n, real_mode, &mut rng, &mut assign) {
                        continue;
                    }

                    let sol = self.build_from_assignment(&cands, &assign, n, final_edge);
                    if sol.t > best.t {
                        best = sol;
                    }
                }
            }
        }

        best
    }

    fn solve_layer_fallback(&mut self) -> Solution {
        let mut best = Solution::new();

        let mut ds = [-1_i32; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        ds[START] = 0;
        q[t] = START;
        t += 1;

        while h < t {
            let v = q[h];
            h += 1;
            for &(to, _) in &self.adj[v] {
                if ds[to] < 0 {
                    ds[to] = ds[v] + 1;
                    q[t] = to;
                    t += 1;
                }
            }
        }

        let dg = ds[GOAL];
        if dg <= 0 {
            return best;
        }

        let mut cut = vec![Vec::new(); dg as usize];
        let mut cost = vec![0usize; dg as usize];

        for (e, edge) in self.edges.iter().enumerate() {
            let da = ds[edge.a];
            let db = ds[edge.b];
            if da < 0 || db < 0 {
                continue;
            }

            if (da - db).abs() == 1 {
                let r = da.min(db);
                if 0 <= r && r < dg {
                    cut[r as usize].push(e);
                }
            }
        }

        for r in 0..dg as usize {
            cost[r] = cut[r].len();
        }

        let mut levels = Vec::new();
        let mut used = 0usize;
        for r in 0..dg as usize {
            if cost[r] > 0 && used + cost[r] <= M {
                levels.push(r);
                used += cost[r];
            }
        }

        let mut sol = Solution::new();
        let mut cnt = [0usize; K];
        let mut door_kind = vec![-1; self.edges.len()];
        let mut used_cell = [false; C];

        let mut prev = -1;
        let mut gnum = 0usize;

        for &r_usize in &levels {
            if gnum >= 50 {
                break;
            }

            let r = r_usize as i32;
            let k = gnum % K;
            cnt[k] += 1;
            let parity = cnt[k] & 1;

            for &e in &cut[r_usize] {
                door_kind[e] = (2 * k + parity) as i32;
            }

            let mut best_cell = usize::MAX;
            let mut best_val = i32::MIN;

            for c in 0..C {
                if ds[c] > prev && ds[c] <= r && !used_cell[c] {
                    let deg = self.adj[c].len() as i32;
                    let val = (4 - deg) * 20 + (ds[c] - prev).min(r - ds[c] + 1) * 2;
                    if val > best_val {
                        best_val = val;
                        best_cell = c;
                    }
                }
            }

            if best_cell == usize::MAX {
                for c in 0..C {
                    if ds[c] >= 0 && ds[c] <= r && !used_cell[c] {
                        best_cell = c;
                        break;
                    }
                }
            }

            if best_cell != usize::MAX {
                sol.sw[best_cell] = k as i32;
                used_cell[best_cell] = true;
            }

            prev = r;
            gnum += 1;
        }

        for (e, &g) in door_kind.iter().enumerate() {
            if g >= 0 {
                sol.doors.push((e, g as usize));
            }
        }

        if sol.doors.len() <= M {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }
        if sol.t > best.t {
            best = sol;
        }

        best
    }

    fn solution_from_door_kind(&mut self, door_kind: &[i32], sw: [i32; C]) -> Solution {
        let mut sol = Solution::new();
        sol.sw = sw;
        for (e, &g) in door_kind.iter().enumerate() {
            if g >= 0 {
                sol.doors.push((e, g as usize));
            }
        }
        if sol.doors.len() <= M {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }
        sol
    }

    fn all_pairs_dist(&self) -> Vec<[i32; C]> {
        let mut dist_all = vec![[INF; C]; C];
        let mut q = [0usize; C];

        for s in 0..C {
            if self.grid[s] != b'.' {
                continue;
            }
            let mut h = 0usize;
            let mut t = 0usize;
            dist_all[s][s] = 0;
            q[t] = s;
            t += 1;
            while h < t {
                let v = q[h];
                h += 1;
                let nd = dist_all[s][v] + 1;
                for &(to, _) in &self.adj[v] {
                    if dist_all[s][to] > nd {
                        dist_all[s][to] = nd;
                        q[t] = to;
                        t += 1;
                    }
                }
            }
        }

        dist_all
    }

    fn calc_articulation(&self) -> [bool; C] {
        fn dfs(
            solver: &Solver,
            v: usize,
            pe: usize,
            tin: &mut [i32; C],
            low: &mut [i32; C],
            timer: &mut i32,
            is_art: &mut [bool; C],
        ) {
            tin[v] = *timer;
            low[v] = *timer;
            *timer += 1;
            let mut child_count = 0usize;

            for &(to, e) in &solver.adj[v] {
                if e == pe {
                    continue;
                }
                if tin[to] >= 0 {
                    low[v] = low[v].min(tin[to]);
                } else {
                    dfs(solver, to, e, tin, low, timer, is_art);
                    low[v] = low[v].min(low[to]);
                    if pe != usize::MAX && low[to] >= tin[v] {
                        is_art[v] = true;
                    }
                    child_count += 1;
                }
            }

            if pe == usize::MAX && child_count > 1 {
                is_art[v] = true;
            }
        }

        let mut tin = [-1_i32; C];
        let mut low = [0_i32; C];
        let mut is_art = [false; C];
        let mut timer = 0_i32;
        dfs(
            self,
            START,
            usize::MAX,
            &mut tin,
            &mut low,
            &mut timer,
            &mut is_art,
        );
        is_art
    }

    fn calc_reach_no_goal(&self) -> [bool; C] {
        let mut reached = [false; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        reached[START] = true;
        q[t] = START;
        t += 1;

        while h < t {
            let v = q[h];
            h += 1;
            for &(to, _) in &self.adj[v] {
                if to == GOAL || reached[to] {
                    continue;
                }
                reached[to] = true;
                q[t] = to;
                t += 1;
            }
        }

        reached
    }

    fn trim_beam(beam: &mut Vec<BeamState>, width: usize) {
        if beam.len() > width {
            beam.select_nth_unstable_by(width, |a, b| b.val.cmp(&a.val));
            beam.truncate(width);
        }
        beam.sort_unstable_by(|a, b| b.val.cmp(&a.val));
    }

    fn beam_cell_sequence(
        &self,
        cand: &[usize],
        dist_all: &[[i32; C]],
        max_len: usize,
        beam_w: usize,
        avoid_adjacent: bool,
    ) -> Vec<usize> {
        let mut beam = vec![BeamState::new(dist_all[START][GOAL])];
        let mut next = Vec::new();
        let mut best_seq = Vec::new();
        let mut best_val = dist_all[START][GOAL];
        let limit = max_len.min(cand.len()).min(K);

        for _ in 0..limit {
            next.clear();
            for &bs in &beam {
                for ci in 0..cand.len() {
                    let u = cand[ci];
                    let mut bad = false;
                    for j in 0..bs.len {
                        let used_idx = bs.seq[j] as usize;
                        let v = cand[used_idx];
                        if used_idx == ci
                            || (avoid_adjacent && self.adj[v].iter().any(|&(to, _)| to == u))
                        {
                            bad = true;
                            break;
                        }
                    }
                    if bad {
                        continue;
                    }

                    let mut ns = bs;
                    ns.score = bs.score + dist_all[bs.last][u];
                    ns.last = u;
                    ns.seq[ns.len] = ci as i32;
                    ns.len += 1;
                    ns.val = ns.score + dist_all[u][GOAL];
                    next.push(ns);
                }
            }
            if next.is_empty() {
                break;
            }
            Self::trim_beam(&mut next, beam_w);
            std::mem::swap(&mut beam, &mut next);
            if beam[0].val > best_val {
                best_val = beam[0].val;
                best_seq.clear();
                for j in 0..beam[0].len {
                    best_seq.push(cand[beam[0].seq[j] as usize]);
                }
            }
        }

        best_seq
    }

    fn build_cell_chain_solution(&mut self, seq: &[usize], len: usize) -> Option<Solution> {
        if len == 0 || len > K {
            return None;
        }

        let mut sw = [-1_i32; C];
        let mut door_kind = vec![-1_i32; self.edges.len()];
        let mut door_count = 0usize;

        for idx in 0..len {
            let u = seq[idx];
            if sw[u] >= 0 {
                return None;
            }
            sw[u] = idx as i32;
            if idx >= 1 {
                let g = (2 * (idx - 1) + 1) as i32;
                for &(_, e) in &self.adj[u] {
                    if door_kind[e] >= 0 && door_kind[e] != g {
                        return None;
                    }
                    if door_kind[e] < 0 {
                        if door_count >= M {
                            return None;
                        }
                        door_kind[e] = g;
                        door_count += 1;
                    }
                }
            }
        }

        let g = (2 * (len - 1) + 1) as i32;
        for &(_, e) in &self.adj[GOAL] {
            if door_kind[e] >= 0 && door_kind[e] != g {
                return None;
            }
            if door_kind[e] < 0 {
                if door_count >= M {
                    return None;
                }
                door_kind[e] = g;
                door_count += 1;
            }
        }

        Some(self.solution_from_door_kind(&door_kind, sw))
    }

    fn construct_cell_chain(&mut self, mode: usize, beam_w: usize, cand_limit: usize) -> Solution {
        let dist_all = self.all_pairs_dist();
        let is_art = self.calc_articulation();
        let reach_no_goal = self.calc_reach_no_goal();

        let mut cand = Vec::new();
        for u in 0..C {
            if self.grid[u] != b'.' || u == START || u == GOAL || !reach_no_goal[u] {
                continue;
            }
            if self.adj[GOAL].iter().any(|&(to, _)| to == u) {
                continue;
            }
            if mode == 0 && is_art[u] {
                continue;
            }
            if mode == 1 && is_art[u] && self.adj[u].len() <= 2 {
                continue;
            }
            cand.push(u);
        }

        if cand.is_empty() {
            return Solution::new();
        }

        let mut ecc = [0_i32; C];
        for &u in &cand {
            let mut mx = 0_i32;
            for v in 0..C {
                if self.grid[v] == b'.' {
                    mx = mx.max(dist_all[u][v]);
                }
            }
            ecc[u] = mx;
        }

        cand.sort_unstable_by(|&a, &b| {
            let score = |u: usize| -> i32 {
                let deg = self.adj[u].len() as i32;
                if mode == 0 {
                    100 * ecc[u] + 45 * (dist_all[START][u] + dist_all[u][GOAL]) - 8 * deg
                } else if mode == 1 {
                    80 * ecc[u] + 70 * dist_all[START][u] + 30 * dist_all[u][GOAL] - 5 * deg
                } else {
                    50 * (dist_all[START][u] + dist_all[u][GOAL])
                        + 100 * dist_all[START][u].min(dist_all[u][GOAL])
                        - 4 * deg
                }
            };
            score(b).cmp(&score(a))
        });
        cand.truncate(cand_limit.min(cand.len()));

        let seq = self.beam_cell_sequence(&cand, &dist_all, K, beam_w, true);
        let mut best = Solution::new();
        for len in 1..=seq.len() {
            if let Some(sol) = self.build_cell_chain_solution(&seq, len) {
                if sol.t > best.t {
                    best = sol;
                }
            }
        }

        best
    }

    fn solve_cell_chain_candidates(&mut self) -> Solution {
        let mut best = Solution::new();
        for &(mode, beam_w, cand_limit) in &[
            (0usize, CELL_CHAIN_BEAM_WIDTH, CELL_CHAIN_CAND_LIMIT),
            (1usize, 220usize, 90usize),
            (2usize, 220usize, 90usize),
        ] {
            let sol = self.construct_cell_chain(mode, beam_w, cand_limit);
            if sol.t > best.t {
                best = sol;
            }
        }
        best
    }

    fn output_solution(&self, best: &Solution) -> String {
        if best.t == 0 {
            return "0\n0\n".to_string();
        }

        let mut door_kind = vec![-1; self.edges.len()];
        for &(e, g) in &best.doors {
            door_kind[e] = g as i32;
        }

        let mut out_doors = Vec::new();
        for (e, &g) in door_kind.iter().enumerate() {
            if g >= 0 {
                out_doors.push((e, g as usize));
            }
        }

        if out_doors.len() > M {
            out_doors.truncate(M);
        }

        let mut out = String::new();
        writeln!(&mut out, "{}", out_doors.len()).unwrap();
        for &(e, g) in &out_doors {
            let ed = self.edges[e];
            writeln!(&mut out, "{} {} {} {}", ed.dir, ed.i, ed.j, g).unwrap();
        }

        let mut out_sw = Vec::new();
        for c in 0..C {
            if best.sw[c] >= 0 {
                out_sw.push((c, best.sw[c] as usize));
            }
        }

        writeln!(&mut out, "{}", out_sw.len()).unwrap();
        for &(c, s) in &out_sw {
            writeln!(&mut out, "{} {} {}", c / N, c % N, s).unwrap();
        }

        out
    }
}

fn toggle_counts(n: usize) -> Vec<usize> {
    let mut cnt = vec![1; n];
    if n >= 2 {
        for i in 0..=(n - 2) {
            cnt[i] = 1usize << (n - 2 - i);
        }
    }
    cnt[n - 1] = 1;
    cnt
}

fn make_assignment(
    edges: &[Edge],
    cands: &[Candidate],
    n: usize,
    mode: usize,
    rng: &mut XorShift,
    assign: &mut Vec<usize>,
) -> bool {
    assign.clear();
    assign.resize(n, usize::MAX);

    let mut used_door = vec![false; edges.len()];
    let mut protected_edge = vec![false; edges.len()];
    let mut used_cand = vec![false; cands.len()];
    let cnt = toggle_counts(n);

    let mut order: Vec<usize> = (0..n).collect();
    if mode == 1 {
        order.reverse();
    } else if mode == 2 {
        order.sort_by(|&a, &b| cnt[b].cmp(&cnt[a]));
    } else if mode == 3 {
        order.clear();
        order.push(0);
        for i in (1..n).rev() {
            order.push(i);
        }
    } else if mode >= 4 {
        for i in (1..n).rev() {
            let j = rng.next_int(i + 1);
            order.swap(i, j);
        }
    }

    for &si in &order {
        let mut picks: Vec<(f64, usize)> = Vec::new();

        for (ci, c) in cands.iter().enumerate() {
            if used_cand[ci] {
                continue;
            }
            if c.len < si {
                continue;
            }

            let mut ok = true;
            for &e in &c.full {
                if used_door[e] {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }

            for p in 0..si {
                if protected_edge[c.tail[p]] {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }

            let len_score = c.depth as f64 + 1.5 * c.len as f64;
            let score = if mode == 1 {
                1000.0 * c.len as f64 + c.depth as f64 + rng.next_double()
            } else if mode == 3 {
                (si + 1) as f64 * 200.0 + c.len as f64 * 20.0 + rng.next_double()
            } else {
                cnt[si] as f64 * len_score + rng.next_double() * (1.0 + cnt[si] as f64) * 3.0
            };

            picks.push((score, ci));
        }

        if picks.is_empty() {
            return false;
        }

        picks.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut take = 0usize;
        if mode >= 4 && picks.len() >= 2 {
            let lim = 5usize.min(picks.len());
            take = rng.next_int(lim);
        }

        let ci = picks[take].1;
        assign[si] = ci;
        used_cand[ci] = true;

        for &e in &cands[ci].full {
            protected_edge[e] = true;
        }
        for p in 0..si {
            used_door[cands[ci].tail[p]] = true;
        }
    }

    true
}

fn valid_assignment(edges: &[Edge], cands: &[Candidate], n: usize, assign: &[usize]) -> bool {
    if assign.len() != n {
        return false;
    }

    let mut owner = vec![-1; edges.len()];
    let mut used_cand = vec![false; cands.len()];
    let mut used_cell = [false; C];

    for i in 0..n {
        let ci = assign[i];
        if ci >= cands.len() {
            return false;
        }
        if used_cand[ci] {
            return false;
        }
        used_cand[ci] = true;

        if cands[ci].len < i {
            return false;
        }
        if used_cell[cands[ci].cell] {
            return false;
        }
        used_cell[cands[ci].cell] = true;

        for p in 0..i {
            let e = cands[ci].tail[p];
            if owner[e] != -1 {
                return false;
            }
            owner[e] = i as i32;
        }
    }

    for i in 0..n {
        let c = &cands[assign[i]];
        for &e in &c.full {
            if owner[e] != -1 && owner[e] != i as i32 {
                return false;
            }
        }
    }

    true
}

fn read_grid() -> [u8; C] {
    input! {
        n: usize,
        m: usize,
        k: usize,
        rows: [Bytes; N],
    }

    assert_eq!(n, N);
    assert_eq!(m, M);
    assert_eq!(k, K);

    let mut grid = [b'#'; C];
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), N);
        for (j, &cell) in row.iter().enumerate() {
            assert!(cell == b'.' || cell == b'#');
            grid[Solver::id(i, j)] = cell;
        }
    }

    grid
}

fn main() {
    let timer = Timer::new();
    let grid = read_grid();
    let mut solver = Solver::new(grid);

    let mut best = solver.solve_chinese(&timer);
    let fallback = solver.solve_layer_fallback();
    if fallback.t > best.t {
        best = fallback;
    }
    let cell_chain = solver.solve_cell_chain_candidates();
    if cell_chain.t > best.t {
        best = cell_chain;
    }

    print!("{}", solver.output_solution(&best));
}
