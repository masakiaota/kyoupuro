// v998_simple_champion.rs
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

const HARD_LIMIT_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (1.88 / JUDGE_TIME_LIMIT_SEC);
const TIMER_GUARD_SEC: f64 = PROGRAM_TIME_LIMIT_SEC * (0.015 / JUDGE_TIME_LIMIT_SEC);
const SEARCH_PHASE_RATIO: f64 = 0.86;
const CANDIDATE_PHASE_RATIO: f64 = 0.90;
const IMPROVE_PHASE_RATIO: f64 = 0.97;

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

    #[inline(always)]
    fn expired(&self, deadline: f64) -> bool {
        self.sec() + TIMER_GUARD_SEC >= deadline
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
struct NestedCandidate {
    estimated_t: i64,
    door_kind: Vec<i32>,
    sw: [i32; C],
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
        self.prepare_eval_edges(doors);
        let stamp = self.next_bfs_stamp();

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

    fn fixed_mask_dist(&self, src: usize, mask: usize, door_kind: &[i32]) -> [i32; C] {
        let mut dist = [-1_i32; C];
        if self.grid[src] != b'.' {
            return dist;
        }
        let mut q = [0usize; C];
        let mut h = 0usize;
        let mut t = 0usize;
        dist[src] = 0;
        q[t] = src;
        t += 1;
        while h < t {
            let v = q[h];
            h += 1;
            for &(to, e) in &self.adj[v] {
                let g = door_kind[e];
                if g >= 0 {
                    let bit = (mask >> (g as usize / 2)) & 1;
                    if bit != (g as usize & 1) {
                        continue;
                    }
                }
                if dist[to] == -1 {
                    dist[to] = dist[v] + 1;
                    q[t] = to;
                    t += 1;
                }
            }
        }
        dist
    }

    fn nested_weighted_score(
        &self,
        door_kind: &[i32],
        sw: &[i32; C],
        timer: &Timer,
        deadline: f64,
    ) -> Option<i64> {
        if timer.expired(deadline) {
            return None;
        }
        let mut pos = [usize::MAX; K];
        for (v, &s) in sw.iter().enumerate() {
            if s >= 0 {
                pos[s as usize] = v;
            }
        }
        if pos.iter().any(|&p| p == usize::MAX) {
            return None;
        }

        let d_start = self.fixed_mask_dist(START, 0, door_kind);
        if d_start[pos[0]] < 0 {
            return None;
        }
        let mut ret = 1023_i64 + d_start[pos[0]] as i64;

        if timer.expired(deadline) {
            return None;
        }
        let d1 = self.fixed_mask_dist(pos[1], 1, door_kind);
        if d1[pos[0]] < 0 {
            return None;
        }
        ret += 512_i64 * d1[pos[0]] as i64;

        for i in 2..=9 {
            if timer.expired(deadline) {
                return None;
            }
            let d = self.fixed_mask_dist(pos[i], 1usize << (i - 1), door_kind);
            if d[pos[0]] < 0 {
                return None;
            }
            ret += (1_i64 << (10 - i)) * d[pos[0]] as i64;
        }

        if timer.expired(deadline) {
            return None;
        }
        let d_goal = self.fixed_mask_dist(GOAL, 1usize << 9, door_kind);
        if d_goal[pos[0]] < 0 {
            return None;
        }
        ret += d_goal[pos[0]] as i64;
        Some(ret)
    }

    fn keep_nested_candidate(top: &mut Vec<NestedCandidate>, cand: NestedCandidate, cap: usize) {
        if cand.estimated_t < 0 {
            return;
        }
        top.push(cand);
        if top.len() > cap * 2 {
            top.select_nth_unstable_by(cap, |a, b| b.estimated_t.cmp(&a.estimated_t));
            top.truncate(cap);
        }
    }

    fn nested_g19_wall_candidates(
        &self,
        door_kind: &[i32],
        sw: &[i32; C],
        timer: &Timer,
        deadline: f64,
    ) -> Vec<(i64, usize)> {
        let mut pos = [usize::MAX; K];
        for (v, &s) in sw.iter().enumerate() {
            if s >= 0 {
                pos[s as usize] = v;
            }
        }
        if pos.iter().any(|&p| p == usize::MAX) {
            return Vec::new();
        }

        let mut pairs = Vec::new();
        pairs.push((START, pos[0], 0usize, 1_i64));
        pairs.push((pos[1], pos[0], 1usize, 512_i64));
        for i in 2..=9 {
            pairs.push((pos[i], pos[0], 1usize << (i - 1), 1_i64 << (10 - i)));
        }

        let mut score = vec![0_i64; self.edges.len()];
        for (src, dst, mask, weight) in pairs {
            if timer.expired(deadline) {
                break;
            }
            let ds = self.fixed_mask_dist(src, mask, door_kind);
            if timer.expired(deadline) {
                break;
            }
            let dt = self.fixed_mask_dist(dst, mask, door_kind);
            let d = ds[dst];
            if d < 0 {
                continue;
            }
            for (e, edge) in self.edges.iter().enumerate() {
                if door_kind[e] >= 0 {
                    continue;
                }
                let on_path =
                    (ds[edge.a] >= 0 && dt[edge.b] >= 0 && ds[edge.a] + 1 + dt[edge.b] == d)
                        || (ds[edge.b] >= 0 && dt[edge.a] >= 0 && ds[edge.b] + 1 + dt[edge.a] == d);
                if on_path {
                    score[e] += weight * 10_000 + d as i64 * weight;
                }
            }
        }

        let mut candidates = score
            .into_iter()
            .enumerate()
            .filter_map(|(e, s)| if s > 0 { Some((s, e)) } else { None })
            .collect::<Vec<_>>();
        candidates.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        candidates
    }

    fn improve_nested_g19_walls(&self, cand: &mut NestedCandidate, timer: &Timer, deadline: f64) {
        let mut door_count = cand.door_kind.iter().filter(|&&g| g >= 0).count();
        if door_count >= M {
            return;
        }
        let Some(mut current_score) =
            self.nested_weighted_score(&cand.door_kind, &cand.sw, timer, deadline)
        else {
            return;
        };

        while door_count < M && !timer.expired(deadline) {
            let candidates =
                self.nested_g19_wall_candidates(&cand.door_kind, &cand.sw, timer, deadline);
            if candidates.is_empty() {
                break;
            }
            let remain = (deadline - timer.sec()).max(0.0);
            let eval_cap = if remain > 0.20 {
                96
            } else if remain > 0.08 {
                48
            } else {
                16
            }
            .min(candidates.len());

            let mut best_e = None;
            let mut best_score = current_score;
            for &(_, e) in candidates.iter().take(eval_cap) {
                if timer.expired(deadline) {
                    break;
                }
                cand.door_kind[e] = 19;
                if let Some(score) =
                    self.nested_weighted_score(&cand.door_kind, &cand.sw, timer, deadline)
                {
                    if score >= best_score {
                        best_score = score;
                        best_e = Some(e);
                    }
                }
                cand.door_kind[e] = -1;
            }

            let Some(e) = best_e else {
                break;
            };
            cand.door_kind[e] = 19;
            current_score = best_score;
            cand.estimated_t = best_score;
            door_count += 1;
        }
    }

    fn random_spine_backbone(
        &self,
        length: usize,
        rng: &mut XorShift,
        timer: &Timer,
        deadline: f64,
    ) -> Option<Vec<usize>> {
        let mut rev = Vec::with_capacity(length + 1);
        let mut used = [false; C];
        rev.push(GOAL);
        used[GOAL] = true;

        fn dfs(
            solver: &Solver,
            length: usize,
            rev: &mut Vec<usize>,
            used: &mut [bool; C],
            rng: &mut XorShift,
            timer: &Timer,
            deadline: f64,
        ) -> bool {
            if timer.sec() >= deadline {
                return false;
            }
            let depth = rev.len() - 1;
            if depth == length {
                return true;
            }
            let v = *rev.last().unwrap();
            let mut cand = Vec::new();
            for &(to, _) in &solver.adj[v] {
                if used[to] || (to == START && depth + 1 < length) {
                    continue;
                }
                let mut touch = 0_i32;
                let mut free_deg = 0_i32;
                for &(w, _) in &solver.adj[to] {
                    if used[w] {
                        touch += 1;
                    } else {
                        free_deg += 1;
                    }
                }
                let ti = GOAL / N;
                let tj = GOAL % N;
                let ii = to / N;
                let jj = to % N;
                let md = ii.abs_diff(ti) + jj.abs_diff(tj);
                let key = touch as f64 * 2.2 + free_deg as f64 * 0.25 - md as f64 * 0.08
                    + rng.next_double() * 3.0;
                cand.push((key, to));
            }
            cand.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for &(_, to) in &cand {
                used[to] = true;
                rev.push(to);
                if dfs(solver, length, rev, used, rng, timer, deadline) {
                    return true;
                }
                rev.pop();
                used[to] = false;
            }
            false
        }

        if !dfs(self, length, &mut rev, &mut used, rng, timer, deadline) {
            return None;
        }
        rev.reverse();
        Some(rev)
    }

    fn random_spine_branch_matching(
        &self,
        spine: &[usize],
        spine_pos: &[usize],
        l: usize,
        rng: &mut XorShift,
        timer: &Timer,
        deadline: f64,
    ) -> Option<Vec<usize>> {
        let mut in_spine = [false; C];
        for &v in spine {
            in_spine[v] = true;
        }
        let mut opts = vec![Vec::<usize>::new(); l - 1];
        for j in 0..l - 1 {
            for &(to, _) in &self.adj[spine[spine_pos[j]]] {
                if !in_spine[to] && to != START {
                    opts[j].push(to);
                }
            }
            if opts[j].is_empty() {
                return None;
            }
        }

        let mut order = (0..l - 1).collect::<Vec<_>>();
        for i in (1..order.len()).rev() {
            let j = rng.next_int(i + 1);
            order.swap(i, j);
        }
        order.sort_by_key(|&i| opts[i].len());

        fn dfs_assign(
            solver: &Solver,
            opts: &[Vec<usize>],
            order: &[usize],
            in_spine: &[bool; C],
            used: &mut [bool; C],
            assign: &mut [usize],
            p: usize,
            rng: &mut XorShift,
            timer: &Timer,
            deadline: f64,
        ) -> bool {
            if timer.expired(deadline) {
                return false;
            }
            if p == order.len() {
                return true;
            }
            let idx = order[p];
            let mut cand = opts[idx].clone();
            for i in (1..cand.len()).rev() {
                let j = rng.next_int(i + 1);
                cand.swap(i, j);
            }
            cand.sort_by_key(|&a| {
                let mut c = 0usize;
                for &(w, _) in &solver.adj[a] {
                    if in_spine[w] || used[w] {
                        c += 1;
                    }
                }
                usize::MAX - c
            });
            for v in cand {
                if used[v] {
                    continue;
                }
                used[v] = true;
                assign[idx] = v;
                if dfs_assign(
                    solver,
                    opts,
                    order,
                    in_spine,
                    used,
                    assign,
                    p + 1,
                    rng,
                    timer,
                    deadline,
                ) {
                    return true;
                }
                assign[idx] = usize::MAX;
                used[v] = false;
            }
            false
        }

        let mut used = [false; C];
        let mut assign = vec![usize::MAX; l - 1];
        if !dfs_assign(
            self,
            &opts,
            &order,
            &in_spine,
            &mut used,
            &mut assign,
            0,
            rng,
            timer,
            deadline,
        ) {
            return None;
        }
        let mut branch = vec![usize::MAX; l];
        for j in 0..l - 1 {
            branch[j + 1] = assign[j];
        }
        Some(branch)
    }

    fn build_nested_chamber_candidate(
        &self,
        spine: &[usize],
        spine_pos: &[usize],
        branch: &[usize],
        top: &mut Vec<NestedCandidate>,
        timer: &Timer,
        deadline: f64,
    ) -> bool {
        if timer.expired(deadline) {
            return false;
        }
        let l = 9usize;
        let mut in_u = vec![false; C];
        let mut dyn_edge = vec![false; self.edges.len()];
        let mut free_edge = vec![false; self.edges.len()];
        for &v in spine.iter().skip(1) {
            in_u[v] = true;
        }
        for &v in branch.iter().take(l).skip(1) {
            in_u[v] = true;
        }
        if in_u[START] {
            return false;
        }

        let mut base_door = vec![-1_i32; self.edges.len()];
        let mut base_sw = [-1_i32; C];
        for i in 0..l - 1 {
            let Some(e) = self.edge_between_cells(spine[spine_pos[i]], spine[spine_pos[i] + 1])
            else {
                return false;
            };
            if dyn_edge[e] {
                return false;
            }
            dyn_edge[e] = true;
            base_door[e] = (2 * (i + 1)) as i32;
        }
        let Some(final_e) =
            self.edge_between_cells(spine[spine_pos[l - 1]], spine[spine_pos[l - 1] + 1])
        else {
            return false;
        };
        if dyn_edge[final_e] {
            return false;
        }
        dyn_edge[final_e] = true;
        base_door[final_e] = 19;

        for i in 1..l {
            let Some(e) = self.edge_between_cells(spine[spine_pos[i - 1]], branch[i]) else {
                return false;
            };
            if dyn_edge[e] {
                return false;
            }
            dyn_edge[e] = true;
            base_door[e] = (2 * i + 1) as i32;
            base_sw[branch[i]] = (i + 1) as i32;
        }

        let mut controlled_start = vec![false; spine.len()];
        for &p in spine_pos {
            controlled_start[p] = true;
        }
        for j in 0..spine.len() - 1 {
            let Some(e) = self.edge_between_cells(spine[j], spine[j + 1]) else {
                return false;
            };
            if !controlled_start[j] {
                free_edge[e] = true;
            }
        }

        for (e, edge) in self.edges.iter().enumerate() {
            if !dyn_edge[e] && (in_u[edge.a] || in_u[edge.b]) && !free_edge[e] {
                base_door[e] = 1;
            }
        }

        let mut in_r = in_u.clone();
        in_r[spine[0]] = true;
        for v in 0..C {
            if in_u[v] {
                for &(to, _) in &self.adj[v] {
                    in_r[to] = true;
                }
            }
        }
        loop {
            if timer.expired(deadline) {
                return false;
            }
            let mut add = Vec::new();
            for v in 0..C {
                if in_r[v] || v == START || self.grid[v] != b'.' {
                    continue;
                }
                let cnt = self.adj[v].iter().filter(|&&(to, _)| in_r[to]).count();
                if cnt >= 3 {
                    add.push(v);
                }
            }
            if add.is_empty() {
                break;
            }
            for v in add {
                in_r[v] = true;
            }
        }
        if in_r[START] {
            return false;
        }

        for (e, edge) in self.edges.iter().enumerate() {
            if in_r[edge.a] != in_r[edge.b] {
                if base_door[e] >= 0 {
                    return false;
                }
                base_door[e] = 0;
            }
        }

        let base_d = base_door.iter().filter(|&&g| g >= 0).count();
        if base_d + 1 > M {
            return false;
        }

        let mut pockets = Vec::new();
        for v in 0..C {
            if in_r[v] || v == START || self.grid[v] != b'.' {
                continue;
            }
            let touch_r = self.adj[v].iter().any(|&(to, _)| in_r[to]);
            if touch_r || base_d + self.adj[v].len() > M {
                continue;
            }
            pockets.push(v);
        }
        if pockets.is_empty() {
            return false;
        }
        pockets.sort_by_key(|&v| {
            let deg = self.adj[v].len();
            let dist_goal = (v / N).abs_diff(N - 1) + (v % N).abs_diff(N - 1);
            (deg, usize::MAX - dist_goal)
        });

        let mut sampled = Vec::new();
        for &v in pockets.iter().take(10) {
            sampled.push(v);
        }
        // Deterministic spread over the remaining candidates keeps this cheap.
        if pockets.len() > 10 {
            let step = (pockets.len() / 16).max(1);
            let mut idx = 10usize;
            while sampled.len() < 26 && idx < pockets.len() {
                sampled.push(pockets[idx]);
                idx += step;
            }
        }
        sampled.sort_unstable();
        sampled.dedup();

        for p1 in sampled {
            if timer.expired(deadline) {
                return false;
            }
            let mut door = base_door.clone();
            let mut sw = base_sw;
            let mut bad = false;
            for &(_, e) in &self.adj[p1] {
                if door[e] >= 0 && door[e] != 1 {
                    bad = true;
                    break;
                }
                door[e] = 1;
            }
            if bad || door.iter().filter(|&&g| g >= 0).count() > M {
                continue;
            }
            sw[p1] = 1;

            let mut target = [usize::MAX; K];
            target[1] = p1;
            for (v, &s) in sw.iter().enumerate() {
                if s >= 2 {
                    target[s as usize] = v;
                }
            }
            if target[2..=9].iter().any(|&v| v == usize::MAX) {
                continue;
            }

            if timer.expired(deadline) {
                return false;
            }
            let dstart = self.fixed_mask_dist(START, 0, &door);
            if timer.expired(deadline) {
                return false;
            }
            let d1 = self.fixed_mask_dist(p1, 1, &door);
            if timer.expired(deadline) {
                return false;
            }
            let dgoal = self.fixed_mask_dist(GOAL, 1usize << 9, &door);
            let mut di = Vec::with_capacity(K);
            di.resize(K, [-1_i32; C]);
            for i in 2..=9 {
                if timer.expired(deadline) {
                    return false;
                }
                di[i] = self.fixed_mask_dist(target[i], 1usize << (i - 1), &door);
            }

            let mut best = -1_i64;
            let mut best_s0 = usize::MAX;
            for s0 in 0..C {
                if (s0 & 31) == 0 && timer.expired(deadline) {
                    return false;
                }
                if in_r[s0] || s0 == p1 || self.grid[s0] != b'.' {
                    continue;
                }
                if dstart[s0] < 0 || d1[s0] < 0 || dgoal[s0] < 0 {
                    continue;
                }
                let mut val =
                    1023_i64 + dstart[s0] as i64 + 512_i64 * d1[s0] as i64 + dgoal[s0] as i64;
                let mut ok = true;
                for i in 2..=9 {
                    if di[i][s0] < 0 {
                        ok = false;
                        break;
                    }
                    val += (1_i64 << (10 - i)) * di[i][s0] as i64;
                }
                if ok && val > best {
                    best = val;
                    best_s0 = s0;
                }
            }
            if best_s0 == usize::MAX {
                continue;
            }
            sw[best_s0] = 0;
            Self::keep_nested_candidate(
                top,
                NestedCandidate {
                    estimated_t: best,
                    door_kind: door,
                    sw,
                },
                24,
            );
        }
        true
    }

    fn solve_nested_chamber(&mut self, timer: &Timer, deadline: f64) -> Solution {
        let mut rng = XorShift::new(
            0x9060_0000_0000_0001u64
                ^ (self.edges.len() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        );
        let mut top = Vec::<NestedCandidate>::new();
        let l = 9usize;
        let search_deadline = deadline * SEARCH_PHASE_RATIO;
        let candidate_deadline = deadline * CANDIDATE_PHASE_RATIO;
        let improve_deadline = deadline * IMPROVE_PHASE_RATIO;
        while !timer.expired(search_deadline) {
            let total_depth = l + rng.next_int(20);
            let Some(spine) =
                self.random_spine_backbone(total_depth, &mut rng, timer, search_deadline)
            else {
                continue;
            };
            let mut in_path = [false; C];
            for &v in &spine {
                in_path[v] = true;
            }
            let mut eligible = Vec::new();
            for j in 0..total_depth {
                let has = self.adj[spine[j]]
                    .iter()
                    .any(|&(to, _)| !in_path[to] && to != START);
                if has {
                    eligible.push(j);
                }
            }
            if eligible.is_empty() || eligible[0] != 0 || eligible.len() < l - 1 {
                continue;
            }

            for _ in 0..4 {
                if timer.expired(search_deadline) {
                    break;
                }
                let mut pos = vec![0usize];
                let mut last = 0usize;
                let mut ok = true;
                for need in 1..l - 1 {
                    let rem = (l - 1) - (need + 1);
                    let cand = eligible
                        .iter()
                        .copied()
                        .filter(|&x| x > last && x + rem + 1 < total_depth)
                        .collect::<Vec<_>>();
                    if cand.is_empty() {
                        ok = false;
                        break;
                    }
                    let lim = cand.len().min(5);
                    let x = if rng.next_double() < 0.70 {
                        cand[0]
                    } else {
                        cand[rng.next_int(lim)]
                    };
                    pos.push(x);
                    last = x;
                }
                if !ok || last + 1 >= total_depth {
                    continue;
                }
                let room = total_depth - last - 1;
                pos.push(last + 1 + rng.next_int(room.min(4).max(1)));
                let Some(branch) = self.random_spine_branch_matching(
                    &spine,
                    &pos,
                    l,
                    &mut rng,
                    timer,
                    candidate_deadline,
                ) else {
                    continue;
                };
                self.build_nested_chamber_candidate(
                    &spine,
                    &pos,
                    &branch,
                    &mut top,
                    timer,
                    candidate_deadline,
                );
            }
        }

        top.sort_by(|a, b| b.estimated_t.cmp(&a.estimated_t));
        top.truncate(12);
        for cand in top.iter_mut().take(4) {
            if timer.expired(improve_deadline) {
                break;
            }
            self.improve_nested_g19_walls(cand, timer, improve_deadline);
        }
        top.sort_by(|a, b| b.estimated_t.cmp(&a.estimated_t));
        top.truncate(12);
        let mut best = Solution::new();
        for cand in top {
            if timer.expired(deadline) {
                break;
            }
            let mut sol = self.solution_from_door_kind(&cand.door_kind, cand.sw);
            sol.n = 10;
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

    let best = solver.solve_nested_chamber(&timer, HARD_LIMIT_SEC);

    print!("{}", solver.output_solution(&best));
}
