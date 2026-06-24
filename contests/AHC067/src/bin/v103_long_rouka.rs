// v103_long_rouka.rs
#![allow(dead_code)]

use proconio::{input, marker::Bytes};
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const N: usize = 20;
const M: usize = 50;
const K: usize = 10;
const C: usize = N * N;
const HEDGE: usize = (N - 1) * N;
const VEDGE: usize = N * (N - 1);
const ECOUNT: usize = HEDGE + VEDGE;
const MASKS: usize = 1 << K;
const STATES: usize = MASKS * C;
const START: usize = 0;
const GOAL: usize = C - 1;
const INF: i32 = 1_000_000_000;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};
const RANDOM_START_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.55 / JUDGE_TIME_LIMIT_SEC);
const RANDOM_END_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.82 / JUDGE_TIME_LIMIT_SEC);

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
        let x = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        };
        Self { x }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[derive(Debug, Clone, Copy)]
struct Adj {
    to: usize,
    eid: usize,
}

#[derive(Debug, Clone, Copy)]
struct DoorOut {
    d: usize,
    i: usize,
    j: usize,
    g: i8,
}

#[derive(Debug, Clone, Copy)]
struct SwitchOut {
    i: usize,
    j: usize,
    s: i8,
}

#[derive(Debug, Clone)]
struct State {
    door: [i8; ECOUNT],
    sw: [i8; C],
}

impl State {
    fn new() -> Self {
        Self {
            door: [-1; ECOUNT],
            sw: [-1; C],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BeamState {
    val: i32,
    score: i32,
    last: usize,
    len: usize,
    seq: [i32; 10],
}

impl BeamState {
    fn new(initial_val: i32) -> Self {
        Self {
            val: initial_val,
            score: 0,
            last: START,
            len: 0,
            seq: [-1; 10],
        }
    }
}

#[derive(Debug, Clone)]
struct OffBranch {
    path_idx: usize,
    parent_cell: usize,
    child_cell: usize,
    edge: usize,
    verts: Vec<usize>,
    leaf: Option<usize>,
    depth: i32,
}

#[derive(Debug, Clone, Copy)]
struct TEdge {
    to: usize,
    u: usize,
    v: usize,
    e: usize,
}

#[derive(Debug, Clone, Copy)]
struct BranchCand {
    edge: usize,
    leaf: usize,
    score: i32,
}

#[derive(Debug, Clone, Copy)]
struct Chosen {
    p: usize,
    u: usize,
    fe: usize,
    bi: usize,
    be: usize,
    leaf: usize,
    sc: i32,
}

#[derive(Debug)]
struct Solver {
    timer: Timer,
    grid: [u8; C],
    empties: Vec<usize>,
    adj: Vec<Vec<Adj>>,
    edges: Vec<(usize, usize)>,
    dist_all: [[i32; C]; C],
    is_empty: [bool; C],
    is_bridge: [bool; ECOUNT],
    is_art: [bool; C],
    adj_bool: [[bool; C]; C],
    rng: XorShift,

    comp_of: Vec<i32>,
    comps: Vec<Vec<usize>>,
    tree: Vec<Vec<TEdge>>,
    path_nodes: Vec<usize>,
    path_bridges: Vec<(usize, usize, usize)>,
    off_branches: Vec<OffBranch>,
    reach_no_goal: [bool; C],

    best: State,
    best_t: i32,

    edge_mask: [u16; ECOUNT],
    edge_open: [u16; ECOUNT],
    dist: Vec<i32>,
    que: Vec<u32>,
}

impl Solver {
    fn new() -> Self {
        Self {
            timer: Timer::new(),
            grid: [b'#'; C],
            empties: Vec::new(),
            adj: vec![Vec::new(); C],
            edges: Vec::new(),
            dist_all: [[9999; C]; C],
            is_empty: [false; C],
            is_bridge: [false; ECOUNT],
            is_art: [false; C],
            adj_bool: [[false; C]; C],
            rng: XorShift::new(1_234_567),
            comp_of: Vec::new(),
            comps: Vec::new(),
            tree: Vec::new(),
            path_nodes: Vec::new(),
            path_bridges: Vec::new(),
            off_branches: Vec::new(),
            reach_no_goal: [false; C],
            best: State::new(),
            best_t: -1,
            edge_mask: [0; ECOUNT],
            edge_open: [0; ECOUNT],
            dist: vec![0; STATES],
            que: Vec::with_capacity(STATES),
        }
    }

    #[inline(always)]
    fn id(i: usize, j: usize) -> usize {
        i * N + j
    }

    #[inline(always)]
    fn ij(x: usize) -> (usize, usize) {
        (x / N, x % N)
    }

    #[inline(always)]
    fn h_edge(i: usize, j: usize) -> usize {
        i * N + j
    }

    #[inline(always)]
    fn v_edge(i: usize, j: usize) -> usize {
        HEDGE + i * (N - 1) + j
    }

    fn edge_between(mut a: usize, mut b: usize) -> Option<usize> {
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let ai = a / N;
        let aj = a % N;
        let bi = b / N;
        let bj = b % N;
        if bi == ai + 1 && bj == aj {
            return Some(Self::h_edge(ai, aj));
        }
        if bi == ai && bj == aj + 1 {
            return Some(Self::v_edge(ai, aj));
        }
        None
    }

    fn read_input(&mut self) {
        input! {
            n: usize,
            m: usize,
            k: usize,
            rows: [Bytes; N],
        }
        assert_eq!(n, N);
        assert_eq!(m, M);
        assert_eq!(k, K);

        self.empties.clear();
        self.is_empty.fill(false);
        for (i, row) in rows.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                let x = Self::id(i, j);
                self.grid[x] = cell;
                self.is_empty[x] = cell == b'.';
                if self.is_empty[x] {
                    self.empties.push(x);
                }
            }
        }
        self.build_graph();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1_234_567);
        self.rng = XorShift::new(now ^ (self.empties.len() as u64 * 1_000_003));
    }

    fn build_graph(&mut self) {
        for v in &mut self.adj {
            v.clear();
        }
        self.edges.clear();
        self.adj_bool = [[false; C]; C];

        for i in 0..N {
            for j in 0..N {
                let u = Self::id(i, j);
                if !self.is_empty[u] {
                    continue;
                }

                if i + 1 < N {
                    let v = Self::id(i + 1, j);
                    if self.is_empty[v] {
                        let e = Self::h_edge(i, j);
                        self.adj[u].push(Adj { to: v, eid: e });
                        self.adj[v].push(Adj { to: u, eid: e });
                        self.edges.push((u, v));
                        self.adj_bool[u][v] = true;
                        self.adj_bool[v][u] = true;
                    }
                }

                if j + 1 < N {
                    let v = Self::id(i, j + 1);
                    if self.is_empty[v] {
                        let e = Self::v_edge(i, j);
                        self.adj[u].push(Adj { to: v, eid: e });
                        self.adj[v].push(Adj { to: u, eid: e });
                        self.edges.push((u, v));
                        self.adj_bool[u][v] = true;
                        self.adj_bool[v][u] = true;
                    }
                }
            }
        }
    }

    fn all_pairs_bfs(&mut self) {
        self.dist_all = [[9999; C]; C];
        let mut q = [0usize; C];
        for &s in &self.empties {
            let mut head = 0usize;
            let mut tail = 0usize;
            q[tail] = s;
            tail += 1;
            self.dist_all[s][s] = 0;
            while head < tail {
                let u = q[head];
                head += 1;
                let nd = self.dist_all[s][u] + 1;
                for &a in &self.adj[u] {
                    if self.dist_all[s][a.to] > nd {
                        self.dist_all[s][a.to] = nd;
                        q[tail] = a.to;
                        tail += 1;
                    }
                }
            }
        }
    }

    fn calc_bridges(&mut self) {
        self.is_bridge.fill(false);
        let mut tin = [-1i32; C];
        let mut low = [0i32; C];
        let mut timer = 0i32;
        Self::dfs_bridge(
            START,
            usize::MAX,
            &self.adj,
            &mut tin,
            &mut low,
            &mut timer,
            &mut self.is_bridge,
        );
    }

    fn dfs_bridge(
        u: usize,
        pe: usize,
        adj: &[Vec<Adj>],
        tin: &mut [i32; C],
        low: &mut [i32; C],
        timer: &mut i32,
        is_bridge: &mut [bool; ECOUNT],
    ) {
        tin[u] = *timer;
        low[u] = *timer;
        *timer += 1;
        for &a in &adj[u] {
            let v = a.to;
            let e = a.eid;
            if e == pe {
                continue;
            }
            if tin[v] != -1 {
                low[u] = min(low[u], tin[v]);
            } else {
                Self::dfs_bridge(v, e, adj, tin, low, timer, is_bridge);
                low[u] = min(low[u], low[v]);
                if low[v] > tin[u] {
                    is_bridge[e] = true;
                }
            }
        }
    }

    fn calc_articulation(&mut self) {
        self.is_art.fill(false);
        let mut tin = [-1i32; C];
        let mut low = [0i32; C];
        let mut timer = 0i32;
        Self::dfs_articulation(
            START,
            usize::MAX,
            &self.adj,
            &mut tin,
            &mut low,
            &mut timer,
            &mut self.is_art,
        );
    }

    fn dfs_articulation(
        u: usize,
        pe: usize,
        adj: &[Vec<Adj>],
        tin: &mut [i32; C],
        low: &mut [i32; C],
        timer: &mut i32,
        is_art: &mut [bool; C],
    ) {
        tin[u] = *timer;
        low[u] = *timer;
        *timer += 1;
        let mut ch = 0usize;
        for &a in &adj[u] {
            let v = a.to;
            let e = a.eid;
            if e == pe {
                continue;
            }
            if tin[v] != -1 {
                low[u] = min(low[u], tin[v]);
            } else {
                Self::dfs_articulation(v, e, adj, tin, low, timer, is_art);
                low[u] = min(low[u], low[v]);
                if pe != usize::MAX && low[v] >= tin[u] {
                    is_art[u] = true;
                }
                ch += 1;
            }
        }
        if pe == usize::MAX && ch > 1 {
            is_art[u] = true;
        }
    }

    fn calc_reach_no_goal(&mut self) {
        self.reach_no_goal.fill(false);
        if START == GOAL {
            return;
        }

        let mut q = [0usize; C];
        let mut head = 0usize;
        let mut tail = 0usize;
        self.reach_no_goal[START] = true;
        q[tail] = START;
        tail += 1;
        while head < tail {
            let u = q[head];
            head += 1;
            for &a in &self.adj[u] {
                let v = a.to;
                if v == GOAL || self.reach_no_goal[v] {
                    continue;
                }
                self.reach_no_goal[v] = true;
                q[tail] = v;
                tail += 1;
            }
        }
    }

    fn build_bridge_tree(&mut self) {
        self.calc_bridges();
        self.comp_of = vec![-1; C];
        self.comps.clear();

        for &s in &self.empties {
            if self.comp_of[s] != -1 {
                continue;
            }
            let cid = self.comps.len();
            self.comps.push(Vec::new());
            let mut q = [0usize; C];
            let mut head = 0usize;
            let mut tail = 0usize;
            q[tail] = s;
            tail += 1;
            self.comp_of[s] = cid as i32;
            while head < tail {
                let u = q[head];
                head += 1;
                self.comps[cid].push(u);
                for &a in &self.adj[u] {
                    if !self.is_bridge[a.eid] && self.comp_of[a.to] == -1 {
                        self.comp_of[a.to] = cid as i32;
                        q[tail] = a.to;
                        tail += 1;
                    }
                }
            }
        }

        self.tree = vec![Vec::new(); self.comps.len()];
        for &(u, v) in &self.edges {
            let Some(e) = Self::edge_between(u, v) else {
                continue;
            };
            if self.is_bridge[e] {
                let cu = self.comp_of[u] as usize;
                let cv = self.comp_of[v] as usize;
                self.tree[cu].push(TEdge { to: cv, u, v, e });
                self.tree[cv].push(TEdge {
                    to: cu,
                    u: v,
                    v: u,
                    e,
                });
            }
        }

        self.path_nodes.clear();
        self.path_bridges.clear();
        self.off_branches.clear();
        let sc = self.comp_of[START] as usize;
        let gc = self.comp_of[GOAL] as usize;
        let mut par = vec![-2isize; self.comps.len()];
        let mut paru = vec![usize::MAX; self.comps.len()];
        let mut parv = vec![usize::MAX; self.comps.len()];
        let mut pare = vec![usize::MAX; self.comps.len()];
        let mut qq = VecDeque::new();
        qq.push_back(sc);
        par[sc] = sc as isize;
        while let Some(c) = qq.pop_front() {
            if c == gc {
                break;
            }
            for &te in &self.tree[c] {
                if par[te.to] == -2 {
                    par[te.to] = c as isize;
                    paru[te.to] = te.u;
                    parv[te.to] = te.v;
                    pare[te.to] = te.e;
                    qq.push_back(te.to);
                }
            }
        }
        if par[gc] == -2 {
            return;
        }

        let mut cur = gc;
        while cur != sc {
            self.path_nodes.push(cur);
            cur = par[cur] as usize;
        }
        self.path_nodes.push(sc);
        self.path_nodes.reverse();

        for i in 0..self.path_nodes.len().saturating_sub(1) {
            let child = self.path_nodes[i + 1];
            self.path_bridges
                .push((paru[child], parv[child], pare[child]));
        }

        let mut on_path = vec![false; self.comps.len()];
        for &c in &self.path_nodes {
            on_path[c] = true;
        }

        for (i, &c) in self.path_nodes.iter().enumerate() {
            for &te in &self.tree[c] {
                if on_path[te.to] {
                    continue;
                }
                let mut ob = OffBranch {
                    path_idx: i,
                    parent_cell: te.u,
                    child_cell: te.v,
                    edge: te.e,
                    verts: Vec::new(),
                    leaf: None,
                    depth: -1,
                };
                let mut stack = vec![te.to];
                let mut parent = vec![usize::MAX; self.comps.len()];
                parent[te.to] = c;
                let mut si = 0usize;
                while si < stack.len() {
                    let x = stack[si];
                    si += 1;
                    ob.verts.extend(self.comps[x].iter().copied());
                    for &ne in &self.tree[x] {
                        if ne.to != parent[x] && !on_path[ne.to] {
                            parent[ne.to] = x;
                            stack.push(ne.to);
                        }
                    }
                }

                for &vtx in &ob.verts {
                    if vtx == START || vtx == GOAL {
                        continue;
                    }
                    let d = self.dist_all[te.u][vtx];
                    if d > ob.depth {
                        ob.depth = d;
                        ob.leaf = Some(vtx);
                    }
                }
                if ob.leaf.is_some() {
                    self.off_branches.push(ob);
                }
            }
        }
    }

    fn calc_t(&mut self, s: &State) -> i32 {
        for e in 0..ECOUNT {
            let g = s.door[e];
            if g < 0 {
                self.edge_mask[e] = 0;
                self.edge_open[e] = 0;
            } else {
                let bit = 1u16 << ((g as usize) / 2);
                self.edge_mask[e] = bit;
                self.edge_open[e] = if (g & 1) != 0 { bit } else { 0 };
            }
        }

        self.dist.fill(-1);
        self.que.clear();
        self.dist[START] = 0;
        self.que.push(START as u32);
        let mut head = 0usize;
        while head < self.que.len() {
            let pack = self.que[head];
            head += 1;
            let mask = (pack >> 9) as usize;
            let u = (pack & 511) as usize;
            let di = self.dist[mask * C + u];
            if u == GOAL {
                return di;
            }

            for &a in &self.adj[u] {
                let e = a.eid;
                if ((mask as u16) & self.edge_mask[e]) != self.edge_open[e] {
                    continue;
                }
                let ni = mask * C + a.to;
                if self.dist[ni] < 0 {
                    self.dist[ni] = di + 1;
                    self.que.push(((mask as u32) << 9) | a.to as u32);
                }
            }

            let sw = s.sw[u];
            if sw >= 0 {
                let nm = mask ^ (1usize << sw as usize);
                let ni = nm * C + u;
                if self.dist[ni] < 0 {
                    self.dist[ni] = di + 1;
                    self.que.push(((nm as u32) << 9) | u as u32);
                }
            }
        }
        0
    }

    fn add_door(s: &mut State, edge: usize, g: usize) -> bool {
        if edge >= ECOUNT || g >= 2 * K {
            return false;
        }
        if s.door[edge] != -1 {
            return s.door[edge] == g as i8;
        }
        if Self::door_count(s) >= M {
            return false;
        }
        s.door[edge] = g as i8;
        true
    }

    fn add_switch(s: &mut State, cell: usize, k: usize) -> bool {
        if cell >= C || k >= K {
            return false;
        }
        if s.sw[cell] != -1 {
            return s.sw[cell] == k as i8;
        }
        s.sw[cell] = k as i8;
        true
    }

    fn door_count(s: &State) -> usize {
        s.door.iter().filter(|&&d| d != -1).count()
    }

    fn valid_basic(s: &State) -> bool {
        Self::door_count(s) <= M
    }

    fn consider(&mut self, s: &State) {
        if !Self::valid_basic(s) {
            return;
        }
        let t = self.calc_t(s);
        if t > self.best_t {
            self.best_t = t;
            self.best = s.clone();
        }
    }

    fn used_in_seq(bs: &BeamState, cand_idx: usize) -> bool {
        for i in 0..bs.len {
            if bs.seq[i] == cand_idx as i32 {
                return true;
            }
        }
        false
    }

    fn trim_sort_beam(beam: &mut Vec<BeamState>, beam_w: usize) {
        if beam.len() > beam_w {
            beam.select_nth_unstable_by(beam_w, |a, b| b.val.cmp(&a.val));
            beam.truncate(beam_w);
        }
        beam.sort_unstable_by(|a, b| b.val.cmp(&a.val));
    }

    fn beam_longest_sequence(
        &self,
        nodes: &[usize],
        max_len: usize,
        beam_w: usize,
        avoid_adjacent: bool,
    ) -> Vec<usize> {
        let mut beam = Vec::new();
        let mut nb = Vec::new();
        let init = BeamState::new(self.dist_all[START][GOAL]);
        beam.push(init);
        let mut best_seq = Vec::new();
        let mut best_val = self.dist_all[START][GOAL];
        let limit = min(max_len, nodes.len());

        for _step in 0..limit {
            nb.clear();
            nb.reserve(beam.len() * nodes.len());
            for &bs in &beam {
                for ci in 0..nodes.len() {
                    if Self::used_in_seq(&bs, ci) {
                        continue;
                    }
                    let u = nodes[ci];
                    let mut bad = false;
                    if avoid_adjacent {
                        for j in 0..bs.len {
                            let v = nodes[bs.seq[j] as usize];
                            if u == v || self.adj_bool[u][v] {
                                bad = true;
                                break;
                            }
                        }
                    }
                    if bad {
                        continue;
                    }

                    let mut ns = bs;
                    ns.score = bs.score + self.dist_all[bs.last][u];
                    ns.last = u;
                    ns.seq[ns.len] = ci as i32;
                    ns.len += 1;
                    ns.val = ns.score + self.dist_all[u][GOAL];
                    nb.push(ns);
                }
            }
            if nb.is_empty() {
                break;
            }
            Self::trim_sort_beam(&mut nb, beam_w);
            std::mem::swap(&mut beam, &mut nb);
            if beam[0].val > best_val {
                best_val = beam[0].val;
                best_seq.clear();
                for j in 0..beam[0].len {
                    best_seq.push(nodes[beam[0].seq[j] as usize]);
                }
            }
        }
        best_seq
    }

    fn construct_open_first_branch_chain(&mut self, beam_w: usize) {
        let mut cands = Vec::new();
        for ob in &self.off_branches {
            if !self.reach_no_goal[ob.parent_cell] {
                continue;
            }
            let Some(leaf) = ob.leaf else {
                continue;
            };
            if leaf == START || leaf == GOAL {
                continue;
            }
            let e = ob.edge;
            let mut goal_incident = false;
            for &a in &self.adj[GOAL] {
                if a.eid == e {
                    goal_incident = true;
                    break;
                }
            }
            if goal_incident {
                continue;
            }
            let pot = ob.depth + max(self.dist_all[START][leaf], self.dist_all[leaf][GOAL]);
            cands.push(BranchCand {
                edge: e,
                leaf,
                score: pot,
            });
        }
        if cands.is_empty() {
            return;
        }
        cands.sort_unstable_by(|a, b| b.score.cmp(&a.score));
        if cands.len() > 120 {
            cands.truncate(120);
        }

        let mut beam = Vec::new();
        let mut nb = Vec::new();
        let init = BeamState::new(self.dist_all[START][GOAL]);
        beam.push(init);
        let mut best_bs = init;
        let mut best_val = init.val;
        let max_len = min(10, cands.len());
        for _step in 0..max_len {
            nb.clear();
            nb.reserve(beam.len() * cands.len());
            for &bs in &beam {
                for ci in 0..cands.len() {
                    if Self::used_in_seq(&bs, ci) {
                        continue;
                    }
                    let leaf = cands[ci].leaf;
                    let mut cell_used = false;
                    for j in 0..bs.len {
                        if cands[bs.seq[j] as usize].leaf == leaf {
                            cell_used = true;
                            break;
                        }
                    }
                    if cell_used {
                        continue;
                    }

                    let mut ns = bs;
                    ns.score = bs.score + self.dist_all[bs.last][leaf];
                    ns.last = leaf;
                    ns.seq[ns.len] = ci as i32;
                    ns.len += 1;
                    ns.val = ns.score + self.dist_all[leaf][GOAL];
                    nb.push(ns);
                }
            }
            if nb.is_empty() {
                break;
            }
            Self::trim_sort_beam(&mut nb, beam_w);
            std::mem::swap(&mut beam, &mut nb);
            if beam[0].val > best_val {
                best_val = beam[0].val;
                best_bs = beam[0];
            }
        }

        if best_bs.len == 0 {
            return;
        }
        let mut s = State::new();
        for idx in 0..best_bs.len {
            let c = cands[best_bs.seq[idx] as usize];
            if !Self::add_switch(&mut s, c.leaf, idx) {
                return;
            }
            if idx >= 1 && !Self::add_door(&mut s, c.edge, 2 * (idx - 1) + 1) {
                return;
            }
        }
        let ctrl = best_bs.len - 1;
        for &a in &self.adj[GOAL] {
            if !Self::add_door(&mut s, a.eid, 2 * ctrl + 1) {
                return;
            }
        }
        self.consider(&s);
    }

    fn build_cell_chain_state(&self, seq: &[usize], len: usize) -> State {
        let mut s = State::new();
        if len == 0 {
            return s;
        }
        for idx in 0..len {
            let u = seq[idx];
            Self::add_switch(&mut s, u, idx);
            if idx >= 1 {
                let g = 2 * (idx - 1) + 1;
                for &a in &self.adj[u] {
                    Self::add_door(&mut s, a.eid, g);
                }
            }
        }
        let g = 2 * (len - 1) + 1;
        for &a in &self.adj[GOAL] {
            Self::add_door(&mut s, a.eid, g);
        }
        s
    }

    fn construct_cell_chain(&mut self, mode: usize, beam_w: usize, cand_limit: usize) {
        let mut cand = Vec::new();
        for &u in &self.empties {
            if u == START || u == GOAL {
                continue;
            }
            if self.adj_bool[u][GOAL] {
                continue;
            }
            if !self.reach_no_goal[u] {
                continue;
            }
            if mode == 0 && self.is_art[u] {
                continue;
            }
            if mode == 1 && self.is_art[u] && self.adj[u].len() <= 2 {
                continue;
            }
            cand.push(u);
        }
        if cand.is_empty() {
            return;
        }

        let mut ecc = [0i32; C];
        for &u in &cand {
            let mut mx = 0;
            for &v in &self.empties {
                mx = max(mx, self.dist_all[u][v]);
            }
            ecc[u] = mx;
        }

        cand.sort_unstable_by(|&a, &b| {
            let score_cell = |u: usize| -> i32 {
                let deg = self.adj[u].len() as i32;
                if mode == 0 {
                    return 100 * ecc[u] + 45 * (self.dist_all[START][u] + self.dist_all[u][GOAL])
                        - 8 * deg;
                }
                if mode == 1 {
                    return 80 * ecc[u]
                        + 70 * self.dist_all[START][u]
                        + 30 * self.dist_all[u][GOAL]
                        - 5 * deg;
                }
                50 * (self.dist_all[START][u] + self.dist_all[u][GOAL])
                    + 100 * min(self.dist_all[START][u], self.dist_all[u][GOAL])
                    - 4 * deg
            };
            score_cell(b).cmp(&score_cell(a))
        });
        if cand.len() > cand_limit {
            cand.truncate(cand_limit);
        }

        let seq = self.beam_longest_sequence(&cand, 10, beam_w, true);
        for len in 1..=seq.len() {
            let s = self.build_cell_chain_state(&seq, len);
            self.consider(&s);
        }
    }

    fn construct_bridge_return(&mut self, variant: usize) {
        let s_count = 5usize;
        if self.path_bridges.is_empty() || self.off_branches.is_empty() {
            return;
        }

        let b_count = self.path_bridges.len();
        let mut ps = Vec::new();
        if variant == 0 {
            for p in b_count.saturating_sub(s_count)..b_count {
                ps.push(p);
            }
        } else if variant == 1 {
            for i in 0..s_count {
                let p =
                    ((i + 1) as f64 * b_count as f64 / (s_count + 1) as f64).round() as isize - 1;
                let p = max(0isize, min(b_count as isize - 1, p)) as usize;
                ps.push(p);
            }
            ps.sort_unstable();
            ps.dedup();
            for p in (0..b_count).rev() {
                if ps.len() >= s_count {
                    break;
                }
                if ps.binary_search(&p).is_err() {
                    ps.push(p);
                    ps.sort_unstable();
                }
            }
        } else {
            let step = max(1, b_count / s_count);
            let mut p = 0usize;
            while p < b_count && ps.len() < s_count {
                ps.push(p);
                p += step;
            }
            for p in (0..b_count).rev() {
                if ps.len() >= s_count {
                    break;
                }
                ps.push(p);
            }
            ps.sort_unstable();
            ps.dedup();
        }

        let mut used = vec![false; self.off_branches.len()];
        let mut used_cell = Vec::new();
        let mut ch = Vec::new();
        for &p in &ps {
            let (u, _v, fe) = self.path_bridges[p];
            let mut best = -1;
            let mut best_bi = None;
            let mut best_leaf = None;
            for (bi, ob) in self.off_branches.iter().enumerate() {
                if used[bi] || ob.path_idx > p || !self.reach_no_goal[ob.parent_cell] {
                    continue;
                }
                let mut leaf = None;
                let mut bd = -1;
                for &x in &ob.verts {
                    if x == START || x == GOAL || x == u {
                        continue;
                    }
                    if used_cell.iter().any(|&y| y == x) {
                        continue;
                    }
                    let d = self.dist_all[u][x];
                    if d > bd {
                        bd = d;
                        leaf = Some(x);
                    }
                }
                if let Some(lf) = leaf {
                    if bd > best {
                        best = bd;
                        best_bi = Some(bi);
                        best_leaf = Some(lf);
                    }
                }
            }
            let Some(best_bi) = best_bi else {
                continue;
            };
            let best_leaf = best_leaf.unwrap();
            used[best_bi] = true;
            used_cell.push(u);
            used_cell.push(best_leaf);
            let ob = &self.off_branches[best_bi];
            ch.push(Chosen {
                p,
                u,
                fe,
                bi: best_bi,
                be: ob.edge,
                leaf: best_leaf,
                sc: best,
            });
        }

        let len = min(5, ch.len());
        if len == 0 {
            return;
        }
        let mut s = State::new();
        for (t, chosen) in ch.iter().take(len).enumerate() {
            let a = t;
            let bt = len + t;
            if !Self::add_switch(&mut s, chosen.u, a) {
                return;
            }
            if !Self::add_switch(&mut s, chosen.leaf, bt) {
                return;
            }
            if !Self::add_door(&mut s, chosen.be, 2 * a + 1) {
                return;
            }
            if !Self::add_door(&mut s, chosen.fe, 2 * bt + 1) {
                return;
            }
        }
        self.consider(&s);
    }

    fn random_cell_chains(&mut self, end_sec: f64) {
        let mut cand = Vec::new();
        for &u in &self.empties {
            if u == START || u == GOAL || self.adj_bool[u][GOAL] || !self.reach_no_goal[u] {
                continue;
            }
            if self.is_art[u] && self.adj[u].len() <= 2 {
                continue;
            }
            cand.push(u);
        }
        if cand.len() < 2 {
            return;
        }

        let mut ecc = [0i32; C];
        for &u in &cand {
            let mut mx = 0;
            for &v in &self.empties {
                mx = max(mx, self.dist_all[u][v]);
            }
            ecc[u] = mx;
        }

        let mut iter = 0usize;
        while self.timer.sec() < end_sec {
            iter += 1;
            let mut seq = Vec::new();
            let mut last = START;
            for _step in 0..10 {
                let mut best_u = None;
                let mut best_score = -INF;
                let samples = if iter % 8 == 0 {
                    cand.len()
                } else {
                    min(cand.len(), 35)
                };
                for si in 0..samples {
                    let u = if samples == cand.len() {
                        cand[si]
                    } else {
                        cand[self.rng.next_usize(1_000_000) % cand.len()]
                    };
                    let mut bad = false;
                    for &v in &seq {
                        if u == v || self.adj_bool[u][v] {
                            bad = true;
                            break;
                        }
                    }
                    if bad {
                        continue;
                    }
                    let noise = self.rng.next_usize(1_000_000) as i32 % 50;
                    let sc =
                        100 * self.dist_all[last][u] + 25 * self.dist_all[u][GOAL] + 35 * ecc[u]
                            - 3 * self.adj[u].len() as i32
                            + noise;
                    if sc > best_score {
                        best_score = sc;
                        best_u = Some(u);
                    }
                }
                let Some(u) = best_u else {
                    break;
                };
                seq.push(u);
                last = u;
            }
            if seq.is_empty() {
                continue;
            }

            let from = max(1, seq.len().saturating_sub(3));
            for len in from..=seq.len() {
                let s = self.build_cell_chain_state(&seq, len);
                self.consider(&s);
                if self.timer.sec() >= end_sec {
                    break;
                }
            }
        }
    }

    fn solve(&mut self) {
        self.all_pairs_bfs();
        self.calc_articulation();
        self.calc_reach_no_goal();
        self.build_bridge_tree();

        let empty = State::new();
        self.consider(&empty);
        self.construct_open_first_branch_chain(800);
        self.construct_cell_chain(0, 260, 95);
        self.construct_cell_chain(1, 220, 85);
        self.construct_cell_chain(2, 220, 85);
        self.construct_bridge_return(0);
        self.construct_bridge_return(1);
        self.construct_bridge_return(2);
        if self.timer.sec() < RANDOM_START_LIMIT_SEC {
            self.random_cell_chains(RANDOM_END_SEC);
        }
    }

    fn print_answer(&self) {
        let mut ds = Vec::new();
        for e in 0..ECOUNT {
            if self.best.door[e] == -1 {
                continue;
            }
            if e < HEDGE {
                ds.push(DoorOut {
                    d: 0,
                    i: e / N,
                    j: e % N,
                    g: self.best.door[e],
                });
            } else {
                let r = e - HEDGE;
                ds.push(DoorOut {
                    d: 1,
                    i: r / (N - 1),
                    j: r % (N - 1),
                    g: self.best.door[e],
                });
            }
        }

        let mut ss = Vec::new();
        for x in 0..C {
            if self.best.sw[x] != -1 {
                let (i, j) = Self::ij(x);
                ss.push(SwitchOut {
                    i,
                    j,
                    s: self.best.sw[x],
                });
            }
        }

        if ds.len() > M {
            ds.clear();
            ss.clear();
        }

        let mut out = String::new();
        writeln!(&mut out, "{}", ds.len()).unwrap();
        for d in &ds {
            writeln!(&mut out, "{} {} {} {}", d.d, d.i, d.j, d.g).unwrap();
        }
        writeln!(&mut out, "{}", ss.len()).unwrap();
        for s in &ss {
            writeln!(&mut out, "{} {} {}", s.i, s.j, s.s).unwrap();
        }
        print!("{out}");
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.read_input();
    solver.solve();
    solver.print_answer();
}
