// v203_pairwise_rescue.rs
#![allow(dead_code)]

use proconio::{input, marker::Bytes};
use std::fmt::Write as _;
use std::time::Instant;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.95;
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
const EDGE_BIT_WORDS: usize = (2 * N * (N - 1) + 63) / 64;

const ORIGINAL_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (0.72 / JUDGE_TIME_LIMIT_SEC);
const GOAL_CAGE_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.03 / JUDGE_TIME_LIMIT_SEC);
const AUGMENTED_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.58 / JUDGE_TIME_LIMIT_SEC);
const CELL_CHAIN_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.68 / JUDGE_TIME_LIMIT_SEC);
const VIRTUAL_CALL_LIMIT_SEC: f64 = 0.0;
const VIRTUAL_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.75 / JUDGE_TIME_LIMIT_SEC);
const PRUNE_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.82 / JUDGE_TIME_LIMIT_SEC);
const POST_IMPROVE_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.93 / JUDGE_TIME_LIMIT_SEC);
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
    cells: Vec<usize>,
    tail: Vec<usize>,
    full: Vec<usize>,
    len: usize,
    depth: usize,
    cell_depth: i32,
    trip_cost: i32,
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
struct AssignPairBeamState {
    score: f64,
    assign: [usize; K],
    used_door: [u64; EDGE_BIT_WORDS],
    protected_edge: [u64; EDGE_BIT_WORDS],
}

impl AssignPairBeamState {
    fn new() -> Self {
        Self {
            score: 0.0,
            assign: [usize::MAX; K],
            used_door: [0; EDGE_BIT_WORDS],
            protected_edge: [0; EDGE_BIT_WORDS],
        }
    }
}

#[derive(Debug, Clone)]
struct VTree {
    parent: [i32; C],
    pedge: [usize; C],
    depth: [i32; C],
    tin: [usize; C],
    tout: [usize; C],
    child_of_edge: Vec<usize>,
    children: Vec<Vec<usize>>,
    order: Vec<usize>,
    cross: Vec<Vec<usize>>,
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
    prev: Vec<usize>,
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
            prev: vec![usize::MAX; STATES],
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

    fn prepare_eval_edges(&mut self, doors: &[(usize, usize)]) {
        self.edge_mask.fill(0);
        self.edge_open.fill(0);

        for &(e, g) in doors {
            let bit = 1usize << (g >> 1);
            self.edge_mask[e] = bit;
            self.edge_open[e] = if (g & 1) == 1 { bit } else { 0 };
        }
    }

    fn next_bfs_stamp(&mut self) -> i32 {
        let stamp = self.bfs_stamp;
        self.bfs_stamp += 1;
        if self.bfs_stamp == i32::MAX {
            self.seen.fill(0);
            self.bfs_stamp = 2;
        }
        stamp
    }

    fn calc_t(&mut self, doors: &[(usize, usize)], sw: &[i32; C]) -> usize {
        self.calc_t_internal(doors, sw, None, None).unwrap_or(0)
    }

    fn calc_t_with_cutoff(
        &mut self,
        doors: &[(usize, usize)],
        sw: &[i32; C],
        cutoff: usize,
    ) -> Option<usize> {
        self.calc_t_internal(doors, sw, Some(cutoff), None)
    }

    fn calc_t_with_path(
        &mut self,
        doors: &[(usize, usize)],
        sw: &[i32; C],
        path: &mut Vec<(usize, usize)>,
    ) -> usize {
        self.calc_t_internal(doors, sw, None, Some(path))
            .unwrap_or(0)
    }

    fn calc_t_internal(
        &mut self,
        doors: &[(usize, usize)],
        sw: &[i32; C],
        cutoff: Option<usize>,
        mut path: Option<&mut Vec<(usize, usize)>>,
    ) -> Option<usize> {
        self.prepare_eval_edges(doors);
        let stamp = self.next_bfs_stamp();

        if let Some(path) = path.as_mut() {
            path.clear();
        }

        let cutoff = cutoff.map(|x| x as i32);
        let mut head = 0usize;
        self.queue.clear();
        self.seen[START] = stamp;
        self.dist[START] = 0;
        if path.is_some() {
            self.prev[START] = usize::MAX;
        }
        self.queue.push(START);

        while head < self.queue.len() {
            let state = self.queue[head];
            head += 1;

            let mask = state / C;
            let v = state - mask * C;
            let d = self.dist[state];

            if v == GOAL {
                if let Some(path) = path.as_mut() {
                    let mut cur = state;
                    while self.prev[cur] != usize::MAX {
                        path.push((self.prev[cur], cur));
                        cur = self.prev[cur];
                    }
                    path.reverse();
                }
                return Some(d as usize);
            }

            if cutoff.is_some_and(|limit| d >= limit) {
                continue;
            }

            for &(to, e) in &self.adj[v] {
                if (mask & self.edge_mask[e]) != self.edge_open[e] {
                    continue;
                }
                let ns = mask * C + to;
                if self.seen[ns] != stamp {
                    self.seen[ns] = stamp;
                    self.dist[ns] = d + 1;
                    if path.is_some() {
                        self.prev[ns] = state;
                    }
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
                    if path.is_some() {
                        self.prev[ns] = state;
                    }
                    self.queue.push(ns);
                }
            }
        }

        None
    }

    fn active_all(&self) -> Vec<bool> {
        vec![true; self.edges.len()]
    }

    fn find_bridges_active(&self, active: &[bool], exclude_goal: bool) -> Vec<bool> {
        fn dfs(
            solver: &Solver,
            v: usize,
            pe: usize,
            active: &[bool],
            exclude_goal: bool,
            tin: &mut [i32],
            low: &mut [i32],
            timer: &mut i32,
            is_bridge: &mut [bool],
        ) {
            tin[v] = *timer;
            low[v] = *timer;
            *timer += 1;

            for &(to, e) in &solver.adj[v] {
                if !active[e] {
                    continue;
                }
                if exclude_goal && to == GOAL {
                    continue;
                }
                if e == pe {
                    continue;
                }
                if tin[to] >= 0 {
                    low[v] = low[v].min(tin[to]);
                } else {
                    dfs(
                        solver,
                        to,
                        e,
                        active,
                        exclude_goal,
                        tin,
                        low,
                        timer,
                        is_bridge,
                    );
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
        if self.grid[START] == b'.' {
            dfs(
                self,
                START,
                usize::MAX,
                active,
                exclude_goal,
                &mut tin,
                &mut low,
                &mut timer,
                &mut is_bridge,
            );
        }
        is_bridge
    }

    fn build_bridge_tree_active(
        &self,
        active: &[bool],
        is_bridge: &[bool],
        exclude_goal: bool,
    ) -> BridgeTree {
        let mut bt = BridgeTree {
            comp_of: vec![usize::MAX; C],
            comps: Vec::new(),
            tree: Vec::new(),
        };

        for s in 0..C {
            if self.grid[s] != b'.' || (exclude_goal && s == GOAL) || bt.comp_of[s] != usize::MAX {
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
                    if !active[e] || is_bridge[e] {
                        continue;
                    }
                    if exclude_goal && to == GOAL {
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
            if !active[e] || !is_bridge[e] {
                continue;
            }
            if exclude_goal && (edge.a == GOAL || edge.b == GOAL) {
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
        active: &[bool],
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

            for &(to, e) in &self.adj[v] {
                if !active[e] || bt.comp_of[to] != comp {
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

    fn candidate_cells_in_comp_from_parent(
        &self,
        bt: &BridgeTree,
        active: &[bool],
        comp: usize,
        parent_edge: Option<usize>,
        limit: usize,
    ) -> Vec<(usize, i32)> {
        let Some(parent_edge) = parent_edge else {
            return bt.comps[comp]
                .iter()
                .copied()
                .take(limit)
                .map(|cell| (cell, 0))
                .collect();
        };

        let edge = self.edges[parent_edge];
        let src = if bt.comp_of[edge.a] == comp {
            edge.a
        } else {
            edge.b
        };

        let mut d = [-1_i32; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        d[src] = 0;
        q[t] = src;
        t += 1;

        while h < t {
            let v = q[h];
            h += 1;
            for &(to, e) in &self.adj[v] {
                if !active[e] || bt.comp_of[to] != comp || d[to] >= 0 {
                    continue;
                }
                d[to] = d[v] + 1;
                q[t] = to;
                t += 1;
            }
        }

        let mut scored = Vec::new();
        for &cell in &bt.comps[comp] {
            if cell == START || cell == GOAL || d[cell] < 0 {
                continue;
            }
            let deg = self.adj[cell].len() as i32;
            let score = d[cell] * 100 - deg * 7 + ((cell / N + cell % N) as i32 & 3);
            scored.push((score, d[cell], cell));
        }
        if scored.is_empty() {
            scored.push((0, 0, src));
        }
        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
        let mut ret = Vec::new();
        for (_, depth, cell) in scored {
            if !ret.iter().any(|&(x, _)| x == cell) {
                ret.push((cell, depth));
            }
            if ret.len() >= limit {
                break;
            }
        }
        ret
    }

    fn collect_candidates_original(
        &self,
        bt: &BridgeTree,
        active: &[bool],
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
            let cell = self.farthest_cell_in_comp_from_parent(bt, active, v, Some(pedge[v]));
            let mut cell_depths =
                self.candidate_cells_in_comp_from_parent(bt, active, v, Some(pedge[v]), 8);
            let cell_depth = cell_depths
                .iter()
                .find_map(|&(x, d)| if x == cell { Some(d) } else { None })
                .unwrap_or(0);
            let mut cells = cell_depths
                .drain(..)
                .map(|(cell, _)| cell)
                .collect::<Vec<_>>();
            cells.retain(|&x| x != cell);
            cells.insert(0, cell);
            let trip_cost = 3 * cell_depth + 5 * len as i32 + 2 * depth as i32;

            cands.push(Candidate {
                comp: v,
                cell,
                cells,
                tail,
                full,
                len,
                depth,
                cell_depth,
                trip_cost,
            });
        }

        cands
    }

    fn collect_leaf_candidates(
        &self,
        bt: &BridgeTree,
        active: &[bool],
        root: usize,
        parent: &[usize],
        pedge: &[usize],
    ) -> Vec<Candidate> {
        let mut cands = Vec::new();
        for v in 0..bt.tree.len() {
            if v == root || parent[v] == usize::MAX {
                continue;
            }

            let mut has_child = false;
            for &(to, _) in &bt.tree[v] {
                if parent[to] == v {
                    has_child = true;
                    break;
                }
            }
            if has_child {
                continue;
            }

            let mut full = Vec::new();
            let mut x = v;
            while x != root {
                let e = pedge[x];
                if e == usize::MAX {
                    break;
                }
                full.push(e);
                x = parent[x];
            }
            if full.is_empty() {
                continue;
            }

            let len = full.len();
            let depth = full.len();
            let cell = self.farthest_cell_in_comp_from_parent(bt, active, v, Some(pedge[v]));
            let mut cell_depths =
                self.candidate_cells_in_comp_from_parent(bt, active, v, Some(pedge[v]), 8);
            let cell_depth = cell_depths
                .iter()
                .find_map(|&(x, d)| if x == cell { Some(d) } else { None })
                .unwrap_or(0);
            let mut cells = cell_depths
                .drain(..)
                .map(|(cell, _)| cell)
                .collect::<Vec<_>>();
            cells.retain(|&x| x != cell);
            cells.insert(0, cell);
            let trip_cost = 3 * cell_depth + 5 * len as i32 + 2 * depth as i32;

            cands.push(Candidate {
                comp: v,
                cell,
                cells,
                tail: full.clone(),
                full,
                len,
                depth,
                cell_depth,
                trip_cost,
            });
        }
        cands
    }

    fn build_from_assignment(
        &mut self,
        cands: &[Candidate],
        assign: &[usize],
        n: usize,
        final_edges: &[usize],
        extra_edges: &[usize],
    ) -> Solution {
        let mut door_kind = vec![-1; self.edges.len()];
        let final_g = (2 * (n - 1) + 1) as i32;
        for &e in extra_edges {
            if e < self.edges.len() {
                door_kind[e] = final_g;
            }
        }

        let mut sw_primary = [-1_i32; C];
        let mut dup = false;

        for i in 0..n {
            let c = &cands[assign[i]];
            if sw_primary[c.cell] != -1 {
                dup = true;
            }
            sw_primary[c.cell] = i as i32;

            if i > 0 {
                for j in 0..(i - 1) {
                    door_kind[c.tail[j]] = (2 * j) as i32;
                }
                door_kind[c.tail[i - 1]] = (2 * (i - 1) + 1) as i32;
            }
        }

        for &e in final_edges {
            if e < self.edges.len() {
                door_kind[e] = final_g;
            }
        }

        let mut primary = Solution::new();
        if !dup {
            primary = self.solution_from_door_kind(&door_kind, sw_primary);
            primary.n = n;
        }

        let mut sw_alt = [-1_i32; C];
        let mut used_cell = [false; C];
        let mut ok_alt = true;
        for i in 0..n {
            let c = &cands[assign[i]];
            let cell = c.cells.iter().copied().find(|&cell| !used_cell[cell]);
            let Some(cell) = cell else {
                ok_alt = false;
                break;
            };
            sw_alt[cell] = i as i32;
            used_cell[cell] = true;
        }

        if !ok_alt || sw_alt == sw_primary {
            return primary;
        }

        let mut alt = self.solution_from_door_kind(&door_kind, sw_alt);
        alt.n = n;

        if alt.t > primary.t { alt } else { primary }
    }

    fn set_edge_bit(bits: &mut [u64; EDGE_BIT_WORDS], edge: usize) {
        bits[edge >> 6] |= 1_u64 << (edge & 63);
    }

    fn edge_bits(edge_ids: &[usize]) -> [u64; EDGE_BIT_WORDS] {
        let mut bits = [0_u64; EDGE_BIT_WORDS];
        for &edge in edge_ids {
            Self::set_edge_bit(&mut bits, edge);
        }
        bits
    }

    fn bits_disjoint(a: &[u64; EDGE_BIT_WORDS], b: &[u64; EDGE_BIT_WORDS]) -> bool {
        for i in 0..EDGE_BIT_WORDS {
            if (a[i] & b[i]) != 0 {
                return false;
            }
        }
        true
    }

    fn bits_or_assign(dst: &mut [u64; EDGE_BIT_WORDS], src: &[u64; EDGE_BIT_WORDS]) {
        for i in 0..EDGE_BIT_WORDS {
            dst[i] |= src[i];
        }
    }

    fn assignment_pairwise_beam(
        &self,
        cands: &[Candidate],
        n: usize,
        order: &[usize],
        beam_width: usize,
        keep: usize,
        mode: usize,
    ) -> Vec<Vec<usize>> {
        if n != K || cands.len() < n || order.len() != n {
            return Vec::new();
        }

        let cnt = toggle_counts(n);
        let dist_start = self.calc_plain_dist(START);
        let dist_goal = self.calc_plain_dist(GOAL);
        let mut pair_dist = vec![vec![0_i32; cands.len()]; cands.len()];
        for (ci, c) in cands.iter().enumerate() {
            let dist = self.calc_plain_dist(c.cell);
            for (cj, d) in pair_dist[ci].iter_mut().enumerate() {
                *d = dist[cands[cj].cell].max(0);
            }
        }

        let full_bits = cands
            .iter()
            .map(|c| Self::edge_bits(&c.full))
            .collect::<Vec<_>>();
        let mut tail_bits = vec![[[0_u64; EDGE_BIT_WORDS]; K]; cands.len()];
        for (ci, c) in cands.iter().enumerate() {
            for si in 1..K {
                let upto = si.min(c.tail.len());
                tail_bits[ci][si] = Self::edge_bits(&c.tail[..upto]);
            }
        }

        let mut candidate_value = vec![0.0; cands.len()];
        for (ci, c) in cands.iter().enumerate() {
            let ds = dist_start[c.cell].max(0) as f64;
            let dg = dist_goal[c.cell].max(0) as f64;
            candidate_value[ci] = c.trip_cost as f64
                + 0.65 * (ds + dg)
                + 3.0 * c.len as f64
                + 0.7 * c.depth as f64
                + 1.2 * c.cell_depth.max(0) as f64;
        }

        let (pair_scale, base_scale, spread_scale) = match mode % 4 {
            0 => (7.0, 1.00, 0.10),
            1 => (11.0, 0.55, 0.22),
            2 => (5.0, 1.45, 0.05),
            _ => (15.0, 0.25, 0.35),
        };

        let mut beam = vec![AssignPairBeamState::new()];
        for &si in order {
            let mut next = Vec::new();
            for state in &beam {
                for (ci, c) in cands.iter().enumerate() {
                    if c.len < si {
                        continue;
                    }
                    if state.assign.iter().any(|&x| x == ci) {
                        continue;
                    }
                    if !Self::bits_disjoint(&full_bits[ci], &state.used_door) {
                        continue;
                    }
                    if !Self::bits_disjoint(&tail_bits[ci][si], &state.protected_edge) {
                        continue;
                    }

                    let mut pair_bonus = 0.0;
                    if si > 0 {
                        let prev = state.assign[si - 1];
                        if prev != usize::MAX {
                            pair_bonus += cnt[si - 1] as f64 * pair_dist[ci][prev] as f64;
                        }
                    } else {
                        pair_bonus += 0.35 * cnt[0] as f64 * dist_start[c.cell].max(0) as f64;
                    }
                    if si + 1 < n {
                        let next_ci = state.assign[si + 1];
                        if next_ci != usize::MAX {
                            pair_bonus += cnt[si] as f64 * pair_dist[ci][next_ci] as f64;
                        }
                    } else {
                        pair_bonus += 2.0 * dist_goal[c.cell].max(0) as f64;
                    }

                    let spread = (dist_start[c.cell].max(0) + dist_goal[c.cell].max(0)) as f64;
                    let mut ns = state.clone();
                    ns.assign[si] = ci;
                    ns.score += base_scale * cnt[si] as f64 * candidate_value[ci]
                        + pair_scale * pair_bonus
                        + spread_scale * cnt[si] as f64 * spread
                        + 0.35 * (si + 1) as f64 * c.len as f64
                        + c.depth as f64 * 0.2;
                    Self::bits_or_assign(&mut ns.protected_edge, &full_bits[ci]);
                    Self::bits_or_assign(&mut ns.used_door, &tail_bits[ci][si]);
                    next.push(ns);
                }
            }

            if next.is_empty() {
                return Vec::new();
            }
            next.sort_by(|a, b| b.score.total_cmp(&a.score));
            next.truncate(beam_width);
            beam = next;
        }

        beam.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut ret = Vec::new();
        for state in beam {
            if state.assign.iter().any(|&x| x == usize::MAX) {
                continue;
            }
            let assign = state.assign.to_vec();
            if valid_assignment(&self.edges, cands, n, &assign) && !ret.iter().any(|x| x == &assign)
            {
                ret.push(assign);
            }
            if ret.len() >= keep {
                break;
            }
        }
        ret
    }

    fn best_pairwise_assignment_solution(
        &mut self,
        cands: &[Candidate],
        n: usize,
        final_edges: &[usize],
        extra_edges: &[usize],
        timer: &Timer,
        deadline: f64,
        beam_width: usize,
        exact_cap: usize,
    ) -> Option<(Vec<usize>, Solution)> {
        if n != K || n * (n - 1) / 2 + final_edges.len() + extra_edges.len() > M {
            return None;
        }

        let orders: [[usize; K]; 4] = [
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            [0, 9, 1, 8, 2, 7, 3, 6, 4, 5],
            [0, 1, 9, 2, 8, 3, 7, 4, 6, 5],
        ];
        let per_run_keep = (exact_cap / 4).clamp(8, 20);

        let mut assignments = Vec::new();
        for mode in 0..4 {
            for order in &orders {
                if timer.sec() >= deadline {
                    break;
                }
                let mut got =
                    self.assignment_pairwise_beam(cands, n, order, beam_width, per_run_keep, mode);
                for assign in got.drain(..) {
                    if !assignments.iter().any(|x| x == &assign) {
                        assignments.push(assign);
                    }
                }
                if assignments.len() >= exact_cap * 2 {
                    break;
                }
            }
            if assignments.len() >= exact_cap * 2 || timer.sec() >= deadline {
                break;
            }
        }

        let mut best_pair = None;
        for assign in assignments.into_iter().take(exact_cap) {
            if timer.sec() >= deadline {
                break;
            }
            let sol = self.build_from_assignment(cands, &assign, n, final_edges, extra_edges);
            if sol.t == 0 {
                continue;
            }
            if best_pair
                .as_ref()
                .is_none_or(|(_, best_sol): &(Vec<usize>, Solution)| sol.t > best_sol.t)
            {
                best_pair = Some((assign, sol));
            }
        }
        best_pair
    }

    fn solve_chinese_original(&mut self, timer: &Timer, deadline: f64) -> Solution {
        let mut best = Solution::new();

        let active = self.active_all();
        let is_bridge = self.find_bridges_active(&active, false);
        let bt = self.build_bridge_tree_active(&active, &is_bridge, false);

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

        let mut best_cands = Vec::new();
        let mut best_assign = Vec::new();
        let mut best_final = Vec::new();
        let mut best_extra = Vec::new();
        let mut best_n = 0usize;

        for t in 1..path.len() {
            if timer.sec() >= deadline {
                break;
            }

            let final_edge = pedge[path[t]];
            let cands = self.collect_candidates_original(&bt, &active, &parent, &pedge, &path, t);
            if cands.is_empty() {
                continue;
            }

            for n in (2..=10).rev() {
                if timer.sec() >= deadline {
                    break;
                }
                if n * (n - 1) / 2 + 1 > M {
                    continue;
                }

                let mut feasible_once = false;
                let attempts = match n {
                    10 => 36,
                    9 => 28,
                    8 => 22,
                    _ => 9,
                };

                for mode in 0..attempts {
                    if timer.sec() >= deadline {
                        break;
                    }

                    let real_mode = mode % 15;
                    let mut assign = Vec::new();
                    if !make_assignment(&self.edges, &cands, n, real_mode, &mut rng, &mut assign) {
                        continue;
                    }
                    feasible_once = true;

                    let sol = self.build_from_assignment(&cands, &assign, n, &[final_edge], &[]);
                    if sol.t > best.t {
                        best = sol;
                        best_cands = cands.clone();
                        best_assign = assign;
                        best_final = vec![final_edge];
                        best_extra.clear();
                        best_n = n;
                    }
                }

                if feasible_once && n >= 8 {
                    break;
                }
            }
        }

        if !best_assign.is_empty() && timer.sec() < deadline {
            self.anneal_assignment(
                &best_cands,
                best_n,
                &best_final,
                &best_extra,
                best_assign,
                &mut best,
                timer,
                deadline,
                &mut rng,
            );
        }

        best
    }

    fn goal_incident_edges(&self) -> Vec<usize> {
        self.adj[GOAL].iter().map(|&(_, e)| e).collect()
    }

    fn build_tree_rooted(
        &self,
        bt: &BridgeTree,
        root: usize,
        parent: &mut Vec<usize>,
        pedge: &mut Vec<usize>,
    ) -> bool {
        let b = bt.tree.len();
        parent.clear();
        parent.resize(b, usize::MAX);
        pedge.clear();
        pedge.resize(b, usize::MAX);
        if root >= b {
            return false;
        }

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
        true
    }

    fn solve_goal_cage_chinese(&mut self, timer: &Timer, deadline: f64) -> Solution {
        let mut best = Solution::new();
        let final_edges = self.goal_incident_edges();
        if final_edges.is_empty() {
            return best;
        }

        let active = self.active_all();
        let is_bridge = self.find_bridges_active(&active, true);
        let bt = self.build_bridge_tree_active(&active, &is_bridge, true);
        let root = bt.comp_of[START];
        let mut parent = Vec::new();
        let mut pedge = Vec::new();
        if !self.build_tree_rooted(&bt, root, &mut parent, &mut pedge) {
            return best;
        }

        let cands = self.collect_leaf_candidates(&bt, &active, root, &parent, &pedge);
        if cands.is_empty() {
            return best;
        }

        let mut rng = XorShift::new(
            998_244_353u64 + self.edges.len() as u64 * 17 + cands.len() as u64 * 1231,
        );
        let mut best_cands = Vec::new();
        let mut best_assign = Vec::new();
        let mut best_n = 0usize;

        for n in (2..=10).rev() {
            if timer.sec() >= deadline {
                break;
            }
            if n * (n - 1) / 2 + final_edges.len() > M {
                continue;
            }

            let mut feasible_once = false;
            let attempts = match n {
                10 => 70,
                9 => 54,
                8 => 36,
                _ => 15,
            };
            for mode in 0..attempts {
                if timer.sec() >= deadline {
                    break;
                }
                let real_mode = mode % 15;
                let mut assign = Vec::new();
                if !make_assignment(&self.edges, &cands, n, real_mode, &mut rng, &mut assign) {
                    continue;
                }
                feasible_once = true;
                let sol = self.build_from_assignment(&cands, &assign, n, &final_edges, &[]);
                if sol.t > best.t {
                    best = sol;
                    best_cands = cands.clone();
                    best_assign = assign;
                    best_n = n;
                }
            }
            if n == K && best.t < 25_000 && timer.sec() < deadline {
                if let Some((assign, sol)) = self.best_pairwise_assignment_solution(
                    &cands,
                    n,
                    &final_edges,
                    &[],
                    timer,
                    deadline,
                    620,
                    52,
                ) {
                    feasible_once = true;
                    if sol.t > best.t {
                        best = sol;
                        best_cands = cands.clone();
                        best_assign = assign;
                        best_n = n;
                    }
                }
            }
            if feasible_once && n >= 8 {
                break;
            }
        }

        if !best_assign.is_empty() && timer.sec() < deadline {
            self.anneal_assignment(
                &best_cands,
                best_n,
                &final_edges,
                &[],
                best_assign,
                &mut best,
                timer,
                deadline,
                &mut rng,
            );
        }
        best
    }

    fn make_random_non_tree_edges(&self, dfs_mode: bool, rng: &mut XorShift) -> Vec<usize> {
        let mut seen_cell = [false; C];
        let mut is_tree_edge = vec![false; self.edges.len()];
        let mut parent = [usize::MAX; C];
        let mut depth = [0_i32; C];
        seen_cell[START] = true;
        parent[START] = START;

        if !dfs_mode {
            let mut q = vec![START];
            let mut qi = 0usize;
            while qi < q.len() {
                let v = q[qi];
                qi += 1;
                let mut nb = Vec::new();
                for &(to, e) in &self.adj[v] {
                    if to != GOAL {
                        nb.push((to, e));
                    }
                }
                for i in (1..nb.len()).rev() {
                    let j = rng.next_int(i + 1);
                    nb.swap(i, j);
                }
                for (to, e) in nb {
                    if !seen_cell[to] {
                        seen_cell[to] = true;
                        is_tree_edge[e] = true;
                        parent[to] = v;
                        depth[to] = depth[v] + 1;
                        q.push(to);
                    }
                }
            }
        } else {
            let mut st = vec![START];
            while let Some(&v) = st.last() {
                let mut opts = Vec::new();
                for &(to, e) in &self.adj[v] {
                    if to != GOAL && !seen_cell[to] {
                        opts.push((to, e));
                    }
                }
                if opts.is_empty() {
                    st.pop();
                } else {
                    let (to, e) = opts[rng.next_int(opts.len())];
                    seen_cell[to] = true;
                    is_tree_edge[e] = true;
                    parent[to] = v;
                    depth[to] = depth[v] + 1;
                    st.push(to);
                }
            }
        }

        let tree_dist = |a: usize, b: usize| -> i32 {
            let mut d = 0_i32;
            let mut x = a;
            let mut y = b;
            while x != y && x != usize::MAX && y != usize::MAX {
                if depth[x] >= depth[y] {
                    x = parent[x];
                } else {
                    y = parent[y];
                }
                d += 1;
            }
            d
        };

        let mut scored = Vec::new();
        for (e, edge) in self.edges.iter().enumerate() {
            if is_tree_edge[e] || edge.a == GOAL || edge.b == GOAL {
                continue;
            }
            if seen_cell[edge.a] && seen_cell[edge.b] {
                let cyc = tree_dist(edge.a, edge.b);
                scored.push((cyc * 100 + rng.next_int(100) as i32, e));
            }
        }
        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, e)| e).collect()
    }

    fn solve_augmented_goal_cage(&mut self, timer: &Timer, deadline: f64) -> Solution {
        let mut best = Solution::new();
        let final_edges = self.goal_incident_edges();
        if final_edges.is_empty() {
            return best;
        }

        let mut rng =
            XorShift::new(1_469_598_103_934_665_603u64 ^ self.edges.len() as u64 * 1_315_423_911);
        let mut trial = 0usize;
        while timer.sec() < deadline {
            let phase = trial % 10;
            let n = if phase < 4 {
                10
            } else if phase < 7 {
                9
            } else if phase < 9 {
                8
            } else {
                7
            };
            let base_cost = n * (n - 1) / 2 + final_edges.len();
            if base_cost > M {
                trial += 1;
                continue;
            }

            let remain = M - base_cost;
            let mut non = self.make_random_non_tree_edges((trial & 1) != 0, &mut rng);
            if !non.is_empty() {
                match trial % 3 {
                    0 => {
                        for i in (1..non.len()).rev() {
                            let j = rng.next_int(i + 1);
                            non.swap(i, j);
                        }
                    }
                    1 => {
                        let dist = self.calc_plain_dist(START);
                        let mut scored = Vec::with_capacity(non.len());
                        for e in non {
                            let edge = self.edges[e];
                            let score = dist[edge.a].min(dist[edge.b]) + rng.next_int(7) as i32;
                            scored.push((score, e));
                        }
                        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
                        non = scored.into_iter().map(|(_, e)| e).collect();
                    }
                    _ => {
                        let mut scored = Vec::with_capacity(non.len());
                        for e in non {
                            let edge = self.edges[e];
                            let score = (edge.a / N).abs_diff(10)
                                + (edge.a % N).abs_diff(10)
                                + rng.next_int(5);
                            scored.push((score, e));
                        }
                        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
                        non = scored.into_iter().map(|(_, e)| e).collect();
                    }
                }
            }

            let extra_limit = match (trial / 10) % 5 {
                0 => remain,
                1 => remain.min(6),
                2 => remain / 2,
                3 => remain.min(3),
                _ => 0,
            };
            let extra = non.into_iter().take(extra_limit).collect::<Vec<_>>();
            let mut active = self.active_all();
            for &e in &extra {
                active[e] = false;
            }

            let is_bridge = self.find_bridges_active(&active, true);
            let bt = self.build_bridge_tree_active(&active, &is_bridge, true);
            let root = bt.comp_of[START];
            let mut parent = Vec::new();
            let mut pedge = Vec::new();
            if !self.build_tree_rooted(&bt, root, &mut parent, &mut pedge) {
                trial += 1;
                continue;
            }
            let cands = self.collect_leaf_candidates(&bt, &active, root, &parent, &pedge);
            if cands.is_empty() {
                trial += 1;
                continue;
            }

            let attempts = if n >= 9 { 12 } else { 8 };
            for mode in 0..attempts {
                if timer.sec() >= deadline {
                    break;
                }
                let mut assign = Vec::new();
                if !make_assignment(&self.edges, &cands, n, mode % 15, &mut rng, &mut assign) {
                    continue;
                }
                let sol = self.build_from_assignment(&cands, &assign, n, &final_edges, &extra);
                if sol.t > best.t {
                    best = sol;
                }
            }
            if n == K && trial < 4 && best.t < 18_000 && timer.sec() < deadline {
                if let Some((_assign, sol)) = self.best_pairwise_assignment_solution(
                    &cands,
                    n,
                    &final_edges,
                    &extra,
                    timer,
                    deadline,
                    160,
                    12,
                ) {
                    if sol.t > best.t {
                        best = sol;
                    }
                }
            }
            trial += 1;
        }
        best
    }

    fn anneal_assignment(
        &mut self,
        cands: &[Candidate],
        n: usize,
        final_edges: &[usize],
        extra_edges: &[usize],
        init_assign: Vec<usize>,
        global_best: &mut Solution,
        timer: &Timer,
        deadline: f64,
        rng: &mut XorShift,
    ) {
        if !valid_assignment(&self.edges, cands, n, &init_assign) {
            return;
        }

        let cur_sol = self.build_from_assignment(cands, &init_assign, n, final_edges, extra_edges);
        if cur_sol.t == 0 {
            return;
        }

        let mut cur = init_assign;
        let mut cur_t = cur_sol.t;

        if cur_t > global_best.t {
            *global_best = cur_sol;
        }

        let mut by_switch = vec![Vec::new(); n];
        for i in 0..n {
            for ci in 0..cands.len() {
                if cands[ci].len >= i {
                    by_switch[i].push(ci);
                }
            }
        }

        while timer.sec() < deadline {
            let si = if (rng.next() & 7) == 0 {
                rng.next_int(n)
            } else {
                let r = rng.next_int(100);
                (n - 1).min(((r + 1) as f64).log2() as usize)
            };

            if by_switch[si].is_empty() {
                continue;
            }

            let old = cur[si];
            let ni = by_switch[si][rng.next_int(by_switch[si].len())];
            if ni == old {
                continue;
            }

            cur[si] = ni;
            if !valid_assignment(&self.edges, cands, n, &cur) {
                cur[si] = old;
                continue;
            }

            let nxt = self.build_from_assignment(cands, &cur, n, final_edges, extra_edges);
            let nt = nxt.t;
            if nt == 0 {
                cur[si] = old;
                continue;
            }

            let progress = 1.0_f64.min(timer.sec() / deadline.max(0.001));
            let temp = 2500.0 * (1.0 - progress) + 30.0 * progress;

            let mut accept = nt >= cur_t;
            if !accept {
                let prob = ((nt as f64 - cur_t as f64) / temp).exp();
                accept = rng.next_double() < prob;
            }

            if accept {
                cur_t = nt;
                if cur_t > global_best.t {
                    *global_best = nxt;
                }
            } else {
                cur[si] = old;
            }
        }
    }

    fn edge_between_cells(&self, u: usize, v: usize) -> Option<usize> {
        self.adj[u]
            .iter()
            .find_map(|&(to, e)| if to == v { Some(e) } else { None })
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

    fn door_kind_from_solution(&self, sol: &Solution) -> Vec<i32> {
        let mut door_kind = vec![-1; self.edges.len()];
        for &(e, g) in &sol.doors {
            door_kind[e] = g as i32;
        }
        door_kind
    }

    fn eval_candidate_solution(&mut self, sol: &mut Solution, cutoff: usize) -> usize {
        match self.calc_t_with_cutoff(&sol.doors, &sol.sw, cutoff) {
            Some(t) => {
                sol.t = t;
                t
            }
            None => {
                let t = self.calc_t(&sol.doors, &sol.sw);
                sol.t = t;
                t
            }
        }
    }

    fn improve_path_doors(
        &mut self,
        sol: &mut Solution,
        timer: &Timer,
        deadline: f64,
        rng: &mut XorShift,
    ) {
        if sol.t == 0 {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }

        let mut path = Vec::new();
        let mut seen_code = vec![0_i32; self.edges.len() * 2 * K];
        let mut stamp = 1_i32;

        while sol.doors.len() < M && timer.sec() < deadline {
            let current_t = self.calc_t_with_path(&sol.doors, &sol.sw, &mut path);
            if current_t == 0 || path.is_empty() {
                break;
            }
            if current_t > sol.t {
                sol.t = current_t;
            }

            stamp += 1;
            if stamp == i32::MAX {
                seen_code.fill(0);
                stamp = 1;
            }

            let door_kind = self.door_kind_from_solution(sol);
            let mut candidates = Vec::new();
            for &(a, b) in &path {
                let mask = a / C;
                let u = a - mask * C;
                let next_mask = b / C;
                let v = b - next_mask * C;
                if u == v || mask != next_mask {
                    continue;
                }
                let Some(e) = self.edge_between_cells(u, v) else {
                    continue;
                };
                if door_kind[e] >= 0 {
                    continue;
                }
                for k in 0..K {
                    let bit = (mask >> k) & 1;
                    let g = 2 * k + (bit ^ 1);
                    let code = e * 2 * K + g;
                    if seen_code[code] != stamp {
                        seen_code[code] = stamp;
                        candidates.push((e, g));
                    }
                }
            }
            if candidates.is_empty() {
                break;
            }
            for i in (1..candidates.len()).rev() {
                let j = rng.next_int(i + 1);
                candidates.swap(i, j);
            }

            let remain_ratio = ((deadline - timer.sec()) / deadline).clamp(0.0, 1.0);
            let eval_cap = candidates.len().min((24.0 + 96.0 * remain_ratio) as usize);

            let mut best_t = sol.t;
            let mut best_pair = None;
            for (idx, &(e, g)) in candidates.iter().enumerate() {
                if idx >= eval_cap && best_pair.is_some() {
                    break;
                }
                if idx >= eval_cap * 5 || timer.sec() >= deadline {
                    break;
                }

                let mut cand = sol.clone();
                cand.doors.push((e, g));
                let nt = self.eval_candidate_solution(&mut cand, best_t);
                if nt > best_t || (nt == best_t && nt > 0 && best_pair.is_none()) {
                    best_t = nt;
                    best_pair = Some((e, g));
                }
            }

            let Some((e, g)) = best_pair else {
                break;
            };
            sol.doors.push((e, g));
            sol.t = best_t;
        }
    }

    fn free_slots_for_path_improve(
        &mut self,
        sol: &mut Solution,
        timer: &Timer,
        deadline: f64,
        target_slots: usize,
    ) {
        if sol.doors.len() < M || target_slots == 0 {
            return;
        }
        if sol.t == 0 {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }

        let mut goal_edge = vec![false; self.edges.len()];
        for &(_, e) in &self.adj[GOAL] {
            goal_edge[e] = true;
        }

        let mut removed = 0usize;
        while removed < target_slots && sol.doors.len() > M.saturating_sub(target_slots) {
            if timer.sec() >= deadline {
                break;
            }

            let current_t = sol.t;
            let mut kind_count = [0usize; 2 * K];
            for &(_, g) in &sol.doors {
                if g < 2 * K {
                    kind_count[g] += 1;
                }
            }

            let mut candidates = Vec::new();
            for (idx, &(e, g)) in sol.doors.iter().enumerate() {
                if e >= self.edges.len() || goal_edge[e] {
                    continue;
                }
                let count_score = kind_count[g.min(2 * K - 1)] as i32;
                let odd_bonus = if (g & 1) == 1 { 30 } else { 0 };
                let high_kind_bonus = g as i32;
                candidates.push((count_score * 100 + odd_bonus + high_kind_bonus, idx));
            }
            if candidates.is_empty() {
                break;
            }
            candidates.sort_unstable_by(|a, b| b.0.cmp(&a.0));

            let mut best_idx = None;
            let mut best_t = 0usize;
            let eval_limit = candidates.len().min(20);
            for &(_, idx) in candidates.iter().take(eval_limit) {
                if timer.sec() >= deadline {
                    break;
                }
                let mut tmp = sol.clone();
                tmp.doors.remove(idx);
                let t = self.calc_t(&tmp.doors, &tmp.sw);
                if t > best_t {
                    best_t = t;
                    best_idx = Some(idx);
                }
            }

            let Some(idx) = best_idx else {
                break;
            };
            if best_t == 0 || best_t * 100 < current_t * 55 {
                break;
            }
            sol.doors.remove(idx);
            sol.t = best_t;
            removed += 1;
        }
    }

    fn rewire_path_doors(
        &mut self,
        sol: &mut Solution,
        timer: &Timer,
        prune_deadline: f64,
        improve_deadline: f64,
        rng: &mut XorShift,
    ) {
        if sol.t == 0 {
            sol.t = self.calc_t(&sol.doors, &sol.sw);
        }
        let original = sol.clone();
        let original_t = sol.t;

        self.free_slots_for_path_improve(sol, timer, prune_deadline, 6);
        self.improve_path_doors(sol, timer, improve_deadline, rng);

        if sol.t < original_t {
            *sol = original;
        }
    }

    fn calc_plain_dist(&self, start: usize) -> [i32; C] {
        let mut dist = [-1_i32; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        dist[start] = 0;
        q[t] = start;
        t += 1;

        while h < t {
            let v = q[h];
            h += 1;
            for &(to, _) in &self.adj[v] {
                if dist[to] < 0 {
                    dist[to] = dist[v] + 1;
                    q[t] = to;
                    t += 1;
                }
            }
        }

        dist
    }

    fn boundary_dist(&self, metric: &[i32; C], cut_edges: &[usize], level: usize) -> [i32; C] {
        let mut dist = [-1_i32; C];
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        let level_i = level as i32;

        for &e in cut_edges {
            let edge = self.edges[e];
            let inside = if metric[edge.a] <= level_i {
                edge.a
            } else {
                edge.b
            };
            if metric[inside] >= 0 && dist[inside] < 0 {
                dist[inside] = 0;
                q[t] = inside;
                t += 1;
            }
        }

        while h < t {
            let v = q[h];
            h += 1;
            for &(to, _) in &self.adj[v] {
                if metric[to] >= 0 && metric[to] <= level_i && dist[to] < 0 {
                    dist[to] = dist[v] + 1;
                    q[t] = to;
                    t += 1;
                }
            }
        }

        dist
    }

    fn choose_level_switch(
        &self,
        metric: &[i32; C],
        cut_edges: &[usize],
        level: usize,
        previous_level: i32,
        used_cell: &[bool; C],
        switch_mode: usize,
    ) -> Option<usize> {
        let boundary_dist = self.boundary_dist(metric, cut_edges, level);
        let level_i = level as i32;
        let mut best = None;

        for cell in 0..C {
            if self.grid[cell] != b'.' || used_cell[cell] {
                continue;
            }
            let cell_level = metric[cell];
            if cell_level <= previous_level || cell_level > level_i || boundary_dist[cell] < 0 {
                continue;
            }
            let depth_from_previous = cell_level - previous_level - 1;
            let deg = self.adj[cell].len() as i32;
            let score = match switch_mode {
                0 => boundary_dist[cell] + depth_from_previous.min(20),
                1 => boundary_dist[cell],
                2 => cell_level,
                3 => boundary_dist[cell] + depth_from_previous.min(level_i - cell_level + 1),
                4 => boundary_dist[cell] * 2 + cell_level,
                5 => boundary_dist[cell] * 2 - deg * 5,
                _ => boundary_dist[cell] - cell_level,
            };
            if best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, cell));
            }
        }

        best.map(|(_, cell)| cell)
    }

    fn select_level_cuts(&self, cuts: &[Vec<usize>], mode: usize) -> Vec<usize> {
        let mut order: Vec<usize> = (0..cuts.len()).filter(|&r| !cuts[r].is_empty()).collect();
        let mid = (cuts.len() / 2) as i32;
        order.sort_by(|&a, &b| {
            let cost_a = cuts[a].len();
            let cost_b = cuts[b].len();
            match mode {
                0 => (cost_a, usize::MAX - a).cmp(&(cost_b, usize::MAX - b)),
                1 => (cost_a, a).cmp(&(cost_b, b)),
                2 => a.cmp(&b),
                3 => {
                    let da = (a as i32 - mid).abs();
                    let db = (b as i32 - mid).abs();
                    (cost_a, da, usize::MAX - a).cmp(&(cost_b, db, usize::MAX - b))
                }
                4 => {
                    let va = ((a.min(cuts.len() - a) + 2) * 1024) / cost_a.max(1);
                    let vb = ((b.min(cuts.len() - b) + 2) * 1024) / cost_b.max(1);
                    vb.cmp(&va)
                        .then_with(|| cost_a.cmp(&cost_b))
                        .then_with(|| b.cmp(&a))
                }
                _ => (cost_a * 2, usize::MAX - a).cmp(&(cost_b * 2, usize::MAX - b)),
            }
        });

        let mut selected = Vec::new();
        let mut used = 0usize;
        for r in order {
            let cost = cuts[r].len();
            if used + cost <= M {
                selected.push(r);
                used += cost;
            }
        }
        selected.sort_unstable();
        selected
    }

    fn make_level_cut_solution(
        &mut self,
        metric: &[i32; C],
        cuts: &[Vec<usize>],
        selected: &[usize],
        switch_mode: usize,
    ) -> Solution {
        let mut sw = [-1_i32; C];
        let mut door_kind = vec![-1_i32; self.edges.len()];
        let mut used_cell = [false; C];
        let mut mask = 0usize;
        let mut previous_level = -1_i32;
        let mut door_count = 0usize;

        for (stage, &level) in selected.iter().enumerate() {
            let cost = cuts[level].len();
            if cost == 0 || door_count + cost > M {
                continue;
            }
            let Some(cell) = self.choose_level_switch(
                metric,
                &cuts[level],
                level,
                previous_level,
                &used_cell,
                switch_mode,
            ) else {
                continue;
            };

            let switch_kind = stage % K;
            let bit = (mask >> switch_kind) & 1;
            let g = (2 * switch_kind + (1 - bit)) as i32;
            for &e in &cuts[level] {
                if door_kind[e] < 0 {
                    door_kind[e] = g;
                    door_count += 1;
                }
            }
            if sw[cell] < 0 {
                sw[cell] = switch_kind as i32;
                used_cell[cell] = true;
            }
            mask ^= 1usize << switch_kind;
            previous_level = level as i32;
        }

        self.solution_from_door_kind(&door_kind, sw)
    }

    fn solve_layer_fallback(&mut self) -> Solution {
        let metric = self.calc_plain_dist(START);
        let dg = metric[GOAL];
        if dg <= 0 {
            return Solution::new();
        }

        let mut cuts = vec![Vec::new(); dg as usize];
        for (e, edge) in self.edges.iter().enumerate() {
            let da = metric[edge.a];
            let db = metric[edge.b];
            if da < 0 || db < 0 || da == db {
                continue;
            }
            let r = da.min(db);
            if 0 <= r && r < dg {
                cuts[r as usize].push(e);
            }
        }

        let mut best = Solution::new();
        for mode in 0..6 {
            let selected = self.select_level_cuts(&cuts, mode);
            if selected.is_empty() {
                continue;
            }
            for switch_mode in 0..7 {
                let sol = self.make_level_cut_solution(&metric, &cuts, &selected, switch_mode);
                if sol.t > best.t {
                    best = sol;
                }
            }
        }

        best
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

    fn construct_cell_chain(
        &mut self,
        mode: usize,
        beam_w: usize,
        cand_limit: usize,
        dist_all: &[[i32; C]],
        is_art: &[bool; C],
        reach_no_goal: &[bool; C],
    ) -> Solution {
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
        let dist_all = self.all_pairs_dist();
        let is_art = self.calc_articulation();
        let reach_no_goal = self.calc_reach_no_goal();
        let mut best = Solution::new();
        for &(mode, beam_w, cand_limit) in &[
            (0usize, CELL_CHAIN_BEAM_WIDTH, CELL_CHAIN_CAND_LIMIT),
            (1usize, 220usize, 90usize),
            (2usize, 220usize, 90usize),
        ] {
            let sol = self.construct_cell_chain(
                mode,
                beam_w,
                cand_limit,
                &dist_all,
                &is_art,
                &reach_no_goal,
            );
            if sol.t > best.t {
                best = sol;
            }
        }
        best
    }

    fn spanning_tree_neighbor_order(
        &self,
        mode: usize,
        v: usize,
        rng: &mut XorShift,
    ) -> Vec<(usize, usize)> {
        let mut ns = self.adj[v].clone();
        match mode {
            0 => ns.sort_unstable_by(|a, b| {
                let sa = a.0 / N + a.0 % N;
                let sb = b.0 / N + b.0 % N;
                sa.cmp(&sb).then_with(|| a.0.cmp(&b.0))
            }),
            1 => ns.sort_unstable_by(|a, b| {
                let sa = a.0 / N + a.0 % N;
                let sb = b.0 / N + b.0 % N;
                sb.cmp(&sa).then_with(|| b.0.cmp(&a.0))
            }),
            2 => ns.sort_unstable_by(|a, b| a.0.cmp(&b.0)),
            3 => ns.sort_unstable_by(|a, b| b.0.cmp(&a.0)),
            _ => {
                for i in (1..ns.len()).rev() {
                    let j = rng.next_int(i + 1);
                    ns.swap(i, j);
                }
            }
        }
        ns
    }

    fn build_spanning_tree(&self, mode: usize, rng: &mut XorShift) -> VTree {
        let mut vt = VTree {
            parent: [-2; C],
            pedge: [usize::MAX; C],
            depth: [0; C],
            tin: [0; C],
            tout: [0; C],
            child_of_edge: vec![usize::MAX; self.edges.len()],
            children: vec![Vec::new(); C],
            order: Vec::new(),
            cross: vec![Vec::new(); C],
        };
        vt.parent[START] = -1;

        if mode == 0 {
            let mut q = [0usize; C];
            let mut h = 0usize;
            let mut t = 0usize;
            q[t] = START;
            t += 1;
            while h < t {
                let v = q[h];
                h += 1;
                vt.order.push(v);
                for (to, e) in self.spanning_tree_neighbor_order(mode, v, rng) {
                    if vt.parent[to] == -2 {
                        vt.parent[to] = v as i32;
                        vt.pedge[to] = e;
                        vt.depth[to] = vt.depth[v] + 1;
                        vt.children[v].push(to);
                        q[t] = to;
                        t += 1;
                    }
                }
            }
        } else {
            let mut st = vec![START];
            while let Some(v) = st.pop() {
                vt.order.push(v);
                for (to, e) in self.spanning_tree_neighbor_order(mode, v, rng) {
                    if vt.parent[to] == -2 {
                        vt.parent[to] = v as i32;
                        vt.pedge[to] = e;
                        vt.depth[to] = vt.depth[v] + 1;
                        vt.children[v].push(to);
                        st.push(to);
                    }
                }
            }
        }

        fn dfs_tin(vt: &mut VTree, v: usize, timer: &mut usize) {
            vt.tin[v] = *timer;
            *timer += 1;
            let child_count = vt.children[v].len();
            for idx in 0..child_count {
                let to = vt.children[v][idx];
                dfs_tin(vt, to, timer);
            }
            vt.tout[v] = *timer;
        }
        let mut timer = 0usize;
        dfs_tin(&mut vt, START, &mut timer);

        let mut is_tree = vec![false; self.edges.len()];
        for v in 0..C {
            if vt.pedge[v] != usize::MAX {
                is_tree[vt.pedge[v]] = true;
                vt.child_of_edge[vt.pedge[v]] = v;
            }
        }

        for (e, edge) in self.edges.iter().enumerate() {
            if is_tree[e] {
                continue;
            }
            let mut u = edge.a;
            let mut v = edge.b;
            if vt.parent[u] == -2 || vt.parent[v] == -2 {
                continue;
            }
            while vt.depth[u] > vt.depth[v] {
                vt.cross[u].push(e);
                u = vt.parent[u] as usize;
            }
            while vt.depth[v] > vt.depth[u] {
                vt.cross[v].push(e);
                v = vt.parent[v] as usize;
            }
            while u != v {
                vt.cross[u].push(e);
                vt.cross[v].push(e);
                u = vt.parent[u] as usize;
                v = vt.parent[v] as usize;
            }
        }

        for cross in &mut vt.cross {
            cross.sort_unstable();
            cross.dedup();
        }

        vt
    }

    fn collect_virtual_candidates(&self, vt: &VTree, path: &[usize], tt: usize) -> Vec<Candidate> {
        let final_child = path[tt];
        let mut prefix = [false; C];
        let mut main_edge = vec![false; self.edges.len()];

        for v in 0..C {
            if self.grid[v] != b'.' || vt.parent[v] == -2 {
                continue;
            }
            let in_sub = vt.tin[final_child] <= vt.tin[v] && vt.tin[v] < vt.tout[final_child];
            prefix[v] = !in_sub;
        }
        for i in 1..tt {
            if vt.pedge[path[i]] != usize::MAX {
                main_edge[vt.pedge[path[i]]] = true;
            }
        }

        let mut cands = Vec::new();
        for v in 0..C {
            if self.grid[v] != b'.' || !prefix[v] || v == START || v == GOAL {
                continue;
            }
            let child_cnt = vt.children[v].iter().filter(|&&to| prefix[to]).count();
            if child_cnt != 0 {
                continue;
            }

            let mut tail = Vec::new();
            let mut x = v;
            while x != START && vt.parent[x] >= 0 {
                let e = vt.pedge[x];
                if e == usize::MAX || main_edge[e] {
                    break;
                }
                tail.push(e);
                x = vt.parent[x] as usize;
            }
            if tail.is_empty() {
                continue;
            }

            let mut full = Vec::new();
            x = v;
            while x != START && vt.parent[x] >= 0 {
                let e = vt.pedge[x];
                if e == usize::MAX {
                    break;
                }
                full.push(e);
                x = vt.parent[x] as usize;
            }

            cands.push(Candidate {
                comp: v,
                cell: v,
                cells: vec![v],
                len: tail.len(),
                depth: full.len(),
                tail,
                full,
                cell_depth: 0,
                trip_cost: 0,
            });
        }

        cands
    }

    fn virtual_gate_increment(
        &self,
        vt: &VTree,
        e: usize,
        gate_used: &[bool],
        blocked: &[bool],
    ) -> usize {
        let mut inc = if gate_used[e] { 0 } else { 1 };
        let child = vt.child_of_edge[e];
        if child != usize::MAX {
            for &be in &vt.cross[child] {
                if !blocked[be] {
                    inc += 1;
                }
            }
        }
        inc
    }

    fn virtual_add_gate_cost(
        &self,
        vt: &VTree,
        e: usize,
        gate_used: &mut [bool],
        blocked: &mut [bool],
        cost: &mut usize,
    ) {
        if !gate_used[e] {
            gate_used[e] = true;
            *cost += 1;
        }
        let child = vt.child_of_edge[e];
        if child != usize::MAX {
            for &be in &vt.cross[child] {
                if !blocked[be] {
                    blocked[be] = true;
                    *cost += 1;
                }
            }
        }
    }

    fn make_virtual_assignment(
        &self,
        vt: &VTree,
        cands: &[Candidate],
        n: usize,
        mode: usize,
        final_edge: usize,
        rng: &mut XorShift,
        assign: &mut Vec<usize>,
    ) -> bool {
        assign.clear();
        assign.resize(n, usize::MAX);

        let mut used_door = vec![false; self.edges.len()];
        let mut protected_edge = vec![false; self.edges.len()];
        let mut used_cand = vec![false; cands.len()];
        let mut gate_used = vec![false; self.edges.len()];
        let mut blocked = vec![false; self.edges.len()];
        let mut cost = 0usize;
        self.virtual_add_gate_cost(vt, final_edge, &mut gate_used, &mut blocked, &mut cost);
        if cost > M {
            return false;
        }

        let cnt = toggle_counts(n);
        let mut order = (0..n).collect::<Vec<_>>();
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
            let mut picks = Vec::new();
            for (ci, c) in cands.iter().enumerate() {
                if used_cand[ci] || c.len < si {
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

                let mut inc = 0usize;
                for p in 0..si {
                    let e = c.tail[p];
                    if protected_edge[e] {
                        ok = false;
                        break;
                    }
                    inc += self.virtual_gate_increment(vt, e, &gate_used, &blocked);
                    if cost + inc > M {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }

                let first_child = vt.child_of_edge[c.tail[0]];
                let cross_count = if first_child == usize::MAX {
                    0
                } else {
                    vt.cross[first_child].len()
                };
                let len_score = c.depth as f64 + 1.5 * c.len as f64;
                let penalty =
                    if mode >= 4 { 100.0 } else { 70.0 } * inc as f64 + 5.0 * cross_count as f64;
                let score = if mode == 1 {
                    1000.0 * c.len as f64 + c.depth as f64 - penalty + rng.next_double()
                } else if mode == 3 {
                    (si + 1) as f64 * 200.0 + c.len as f64 * 20.0 - penalty + rng.next_double()
                } else {
                    cnt[si] as f64 * len_score - penalty
                        + rng.next_double() * (1.0 + cnt[si] as f64) * 3.0
                };
                picks.push((score, ci));
            }

            if picks.is_empty() {
                return false;
            }
            picks.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut take = 0usize;
            if mode >= 4 && picks.len() >= 2 {
                take = rng.next_int(5usize.min(picks.len()));
            }
            let ci = picks[take].1;
            assign[si] = ci;
            used_cand[ci] = true;
            for &e in &cands[ci].full {
                protected_edge[e] = true;
            }
            for p in 0..si {
                let e = cands[ci].tail[p];
                used_door[e] = true;
                self.virtual_add_gate_cost(vt, e, &mut gate_used, &mut blocked, &mut cost);
            }
        }

        cost <= M
    }

    fn add_virtual_gate(door_kind: &mut [i32], gate_edges: &mut [bool], e: usize, g: i32) -> bool {
        if door_kind[e] >= 0 && door_kind[e] != g {
            return false;
        }
        door_kind[e] = g;
        gate_edges[e] = true;
        true
    }

    fn add_blockers_of_virtual_gate(vt: &VTree, door_kind: &mut [i32], e: usize) -> bool {
        let child = vt.child_of_edge[e];
        if child == usize::MAX {
            return true;
        }
        let permanent_g = (2 * (K - 1) + 1) as i32;
        for &be in &vt.cross[child] {
            if door_kind[be] >= 0 && door_kind[be] != permanent_g {
                return false;
            }
            door_kind[be] = permanent_g;
        }
        true
    }

    fn build_virtual_from_assignment(
        &mut self,
        vt: &VTree,
        cands: &[Candidate],
        assign: &[usize],
        n: usize,
        final_edge: usize,
    ) -> Solution {
        let mut door_kind = vec![-1_i32; self.edges.len()];
        let mut sw = [-1_i32; C];
        let mut gate_edges = vec![false; self.edges.len()];
        let mut ok = true;

        for i in 0..n {
            let c = &cands[assign[i]];
            if sw[c.cell] >= 0 {
                ok = false;
                break;
            }
            sw[c.cell] = i as i32;
            if i > 0 {
                for j in 0..(i - 1) {
                    ok &= Self::add_virtual_gate(
                        &mut door_kind,
                        &mut gate_edges,
                        c.tail[j],
                        (2 * j) as i32,
                    );
                }
                ok &= Self::add_virtual_gate(
                    &mut door_kind,
                    &mut gate_edges,
                    c.tail[i - 1],
                    (2 * (i - 1) + 1) as i32,
                );
            }
            if !ok {
                break;
            }
        }
        ok &= Self::add_virtual_gate(
            &mut door_kind,
            &mut gate_edges,
            final_edge,
            (2 * (n - 1) + 1) as i32,
        );

        if ok {
            for e in 0..self.edges.len() {
                if gate_edges[e] {
                    ok &= Self::add_blockers_of_virtual_gate(vt, &mut door_kind, e);
                    if !ok {
                        break;
                    }
                }
            }
        }

        if !ok {
            return Solution::new();
        }

        let mut sol = self.solution_from_door_kind(&door_kind, sw);
        sol.n = n;
        sol
    }

    fn solve_virtual_tree(&mut self, timer: &Timer, limit_sec: f64) -> Solution {
        let mut best = Solution::new();
        let mut rng = XorShift::new(
            0x9e37_79b9_7f4a_7c15u64
                ^ (self.edges.len() as u64).wrapping_mul(11995408973635179863u64),
        );

        for mode_tree in 0..5 {
            if timer.sec() > limit_sec {
                break;
            }
            let vt = self.build_spanning_tree(mode_tree, &mut rng);
            if vt.parent[GOAL] == -2 {
                continue;
            }

            let mut path = Vec::new();
            let mut v = GOAL;
            while v != START {
                path.push(v);
                v = vt.parent[v] as usize;
            }
            path.push(START);
            path.reverse();

            for tt in 1..path.len() {
                if timer.sec() > limit_sec {
                    break;
                }
                let final_edge = vt.pedge[path[tt]];
                let mut cands = self.collect_virtual_candidates(&vt, &path, tt);
                if cands.is_empty() {
                    continue;
                }
                cands.sort_unstable_by(|a, b| {
                    let score = |c: &Candidate| -> i32 {
                        let child = vt.child_of_edge[c.tail[0]];
                        let cross_count = if child == usize::MAX {
                            0
                        } else {
                            vt.cross[child].len() as i32
                        };
                        c.depth as i32 * 30 + c.len as i32 * 80 - cross_count * 160
                    };
                    score(b).cmp(&score(a))
                });
                cands.truncate(cands.len().min(80));

                for n in (2..=9).rev() {
                    let base_gates = n * (n - 1) / 2 + 1;
                    if base_gates > M {
                        continue;
                    }
                    let attempts = if n >= 8 {
                        18
                    } else if n >= 6 {
                        14
                    } else {
                        8
                    };
                    let mut feasible = false;
                    for mode in 0..attempts {
                        if timer.sec() > limit_sec {
                            break;
                        }
                        let real_mode = if mode < 5 { mode } else { 4 };
                        let mut assign = Vec::new();
                        if !self.make_virtual_assignment(
                            &vt,
                            &cands,
                            n,
                            real_mode,
                            final_edge,
                            &mut rng,
                            &mut assign,
                        ) {
                            continue;
                        }
                        feasible = true;
                        let sol =
                            self.build_virtual_from_assignment(&vt, &cands, &assign, n, final_edge);
                        if sol.t > best.t {
                            best = sol;
                        }
                    }
                    if feasible && n >= 7 {
                        break;
                    }
                }
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
    let order_mode = mode % 5;
    let score_mode = mode / 5;

    let mut order: Vec<usize> = (0..n).collect();
    if order_mode == 1 {
        order.reverse();
    } else if order_mode == 2 {
        order.sort_by(|&a, &b| cnt[b].cmp(&cnt[a]));
    } else if order_mode == 3 {
        order.clear();
        order.push(0);
        for i in (1..n).rev() {
            order.push(i);
        }
    } else if order_mode >= 4 {
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

            let trip_score = c.trip_cost as f64;
            let len_score = c.depth as f64 + 1.5 * c.len as f64;
            let deep_score = trip_score + 0.8 * c.len as f64 + 0.35 * c.depth as f64;
            let cell_score = c.cell_depth as f64;
            let score = match (score_mode, order_mode) {
                (0, 1) => 1000.0 * c.len as f64 + c.depth as f64 + rng.next_double(),
                (0, 3) => (si + 1) as f64 * 200.0 + c.len as f64 * 20.0 + rng.next_double(),
                (0, _) => {
                    cnt[si] as f64 * len_score + rng.next_double() * (1.0 + cnt[si] as f64) * 3.0
                }
                (1, 1) => {
                    1000.0 * c.len as f64 + 8.0 * trip_score + c.depth as f64 + rng.next_double()
                }
                (1, 3) => {
                    (si + 1) as f64 * 200.0
                        + c.len as f64 * 20.0
                        + 3.0 * trip_score
                        + rng.next_double()
                }
                (1, _) => {
                    cnt[si] as f64 * deep_score + rng.next_double() * (1.0 + cnt[si] as f64) * 4.0
                }
                _ => {
                    cnt[si] as f64 * (deep_score + 0.7 * cell_score)
                        + c.depth as f64 * 4.0
                        + rng.next_double() * (1.0 + cnt[si] as f64) * 5.0
                }
            };

            picks.push((score, ci));
        }

        if picks.is_empty() {
            return false;
        }

        picks.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut take = 0usize;
        if order_mode >= 4 && picks.len() >= 2 {
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

    let mut best = Solution::new();

    let mut sol = solver.solve_chinese_original(&timer, ORIGINAL_LIMIT_SEC);
    if sol.t > best.t {
        best = sol;
    }
    if timer.sec() < GOAL_CAGE_LIMIT_SEC {
        sol = solver.solve_goal_cage_chinese(&timer, GOAL_CAGE_LIMIT_SEC);
        if sol.t > best.t {
            best = sol;
        }
    }
    if timer.sec() < AUGMENTED_LIMIT_SEC {
        sol = solver.solve_augmented_goal_cage(&timer, AUGMENTED_LIMIT_SEC);
        if sol.t > best.t {
            best = sol;
        }
    }

    let fallback = solver.solve_layer_fallback();
    if fallback.t > best.t {
        best = fallback;
    }
    if timer.sec() < CELL_CHAIN_LIMIT_SEC {
        let cell_chain = solver.solve_cell_chain_candidates();
        if cell_chain.t > best.t {
            best = cell_chain;
        }
    }
    if timer.sec() < VIRTUAL_CALL_LIMIT_SEC {
        let virt = solver.solve_virtual_tree(&timer, VIRTUAL_LIMIT_SEC);
        if virt.t > best.t {
            best = virt;
        }
    }

    let mut rng = XorShift::new(9_146_959_810_393_466_560_u64 ^ solver.edges.len() as u64);
    solver.rewire_path_doors(
        &mut best,
        &timer,
        PRUNE_LIMIT_SEC,
        POST_IMPROVE_LIMIT_SEC,
        &mut rng,
    );

    print!("{}", solver.output_solution(&best));
}
