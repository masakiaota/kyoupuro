// v012_segment_reconnect_sa.rs
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const E: (usize, usize) = (0, N / 2);
const EXIT_P: usize = E.0 * N + E.1;
const EMPTY: usize = NN;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;
const LOC_FIXED: usize = MAX_M;
const LOC_GONE: usize = MAX_M + 1;

const LOOK: usize = 80;
const BEAM_WIDTH: usize = 32;
const MAX_PATH: usize = 96;
const INF_DIST: i16 = 30_000;
const NO_POS: u16 = u16::MAX;
const BAD: u16 = u16::MAX;

const SA_TIME_SEC: f64 = 0.50;
const EVAL_PREFIX: usize = 120;
const H_BLOCK: usize = 4;
const V_BLOCK: usize = 4;

#[inline(always)]
fn to_p(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn to_ij(p: usize) -> (usize, usize) {
    (p / N, p % N)
}

#[inline(always)]
fn is_adjacent(p: usize, q: usize) -> bool {
    let (pi, pj) = to_ij(p);
    let (qi, qj) = to_ij(q);
    pi.abs_diff(qi) + pj.abs_diff(qj) == 1
}

#[inline(always)]
fn manhattan_to_exit(p: usize) -> i16 {
    let (i, j) = to_ij(p);
    (i + j.abs_diff(N / 2)) as i16
}

#[derive(Debug, Clone)]
struct Input {
    a: [usize; NN],
    pos_of_box: [usize; NN],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n_in: usize = it.next().unwrap().parse().unwrap();
        debug_assert_eq!(n_in, N);

        let mut a = [0usize; NN];
        let mut pos_of_box = [0usize; NN];

        for p in 0..NN {
            let k: usize = it.next().unwrap().parse().unwrap();
            a[p] = k;
            pos_of_box[k] = p;
        }

        Self { a, pos_of_box }
    }
}

#[derive(Debug, Clone, Copy)]
struct Operation {
    m: usize,
    d: i8,
}

#[derive(Debug, Clone)]
struct Conveyor {
    len: usize,
    cells: [usize; NN],
}

impl Conveyor {
    fn new() -> Self {
        Self {
            len: 0,
            cells: [0; NN],
        }
    }

    fn from_slice(cells: &[usize]) -> Self {
        debug_assert!(2 <= cells.len() && cells.len() <= NN);

        let mut conveyor = Self::new();
        conveyor.len = cells.len();
        conveyor.cells[..cells.len()].copy_from_slice(cells);
        conveyor
    }

    #[inline(always)]
    fn as_slice(&self) -> &[usize] {
        &self.cells[..self.len]
    }
}

#[derive(Debug, Clone)]
struct Solution {
    conveyors: Vec<Conveyor>,
    ops: Vec<Operation>,
}

type Output = Solution;

impl Solution {
    fn new() -> Self {
        Self {
            conveyors: Vec::with_capacity(MAX_M),
            ops: Vec::with_capacity(MAX_T),
        }
    }

    fn add_conveyor(&mut self, cells: &[usize]) -> usize {
        debug_assert!(self.conveyors.len() < MAX_M);

        let m = self.conveyors.len();
        self.conveyors.push(Conveyor::from_slice(cells));
        m
    }

    #[inline(always)]
    fn add_op(&mut self, m: usize, d: i8) {
        debug_assert!(self.ops.len() < MAX_T);
        debug_assert!(m < self.conveyors.len());
        debug_assert!(d == -1 || d == 1);

        self.ops.push(Operation { m, d });
    }

    fn print(&self) {
        let mut out = String::new();

        writeln!(&mut out, "{}", self.conveyors.len()).unwrap();
        for conveyor in &self.conveyors {
            write!(&mut out, "{}", conveyor.len).unwrap();
            for &p in conveyor.as_slice() {
                write!(&mut out, " {} {}", p / N, p % N).unwrap();
            }
            out.push('\n');
        }

        writeln!(&mut out, "{}", self.ops.len()).unwrap();
        for op in &self.ops {
            writeln!(&mut out, "{} {}", op.m, op.d).unwrap();
        }

        io::stdout().write_all(out.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone)]
struct ConveyorState {
    offset: usize,
    items: [usize; NN],
}

impl ConveyorState {
    fn new() -> Self {
        Self {
            offset: 0,
            items: [EMPTY; NN],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CellRef {
    m: usize,
    x: usize,
}

const NO_CELL_REF: CellRef = CellRef {
    m: usize::MAX,
    x: usize::MAX,
};

#[derive(Debug, Clone, Copy)]
struct CellRefs {
    len: usize,
    refs: [CellRef; 2],
}

const EMPTY_CELL_REFS: CellRefs = CellRefs {
    len: 0,
    refs: [NO_CELL_REF; 2],
};

#[derive(Debug, Clone, Copy)]
struct SharedRef {
    x: usize,
    other_m: usize,
    other_x: usize,
}

#[derive(Debug, Clone)]
struct State {
    conv_states: Vec<ConveyorState>,
    fixed: [usize; NN],
    cell_refs: [CellRefs; NN],
    shared_refs: Vec<Vec<SharedRef>>,
    exit_refs: CellRefs,
    loc_m: [usize; NN],
    loc_y: [usize; NN],
    delivered: usize,
}

impl State {
    fn new(input: &Input, conveyors: &[Conveyor]) -> Self {
        let mut conv_states = Vec::with_capacity(conveyors.len());
        for _ in 0..conveyors.len() {
            conv_states.push(ConveyorState::new());
        }

        let mut cell_refs = [EMPTY_CELL_REFS; NN];
        for (m, conveyor) in conveyors.iter().enumerate() {
            debug_assert!(m < MAX_M);
            debug_assert!(2 <= conveyor.len && conveyor.len <= NN);
            for x in 0..conveyor.len {
                let p = conveyor.cells[x];
                debug_assert!(p < NN);
                debug_assert!(cell_refs[p].len < 2);
                let idx = cell_refs[p].len;
                cell_refs[p].refs[idx] = CellRef { m, x };
                cell_refs[p].len += 1;
            }
        }

        let mut shared_refs = vec![Vec::new(); conveyors.len()];
        for refs in &cell_refs {
            if refs.len == 2 {
                let r0 = refs.refs[0];
                let r1 = refs.refs[1];
                shared_refs[r0.m].push(SharedRef {
                    x: r0.x,
                    other_m: r1.m,
                    other_x: r1.x,
                });
                shared_refs[r1.m].push(SharedRef {
                    x: r1.x,
                    other_m: r0.m,
                    other_x: r0.x,
                });
            }
        }

        let mut fixed = [EMPTY; NN];
        let mut loc_m = [LOC_GONE; NN];
        let mut loc_y = [EMPTY; NN];

        for p in 0..NN {
            let k = input.a[p];
            let refs = cell_refs[p];
            if refs.len == 0 {
                fixed[p] = k;
                loc_m[k] = LOC_FIXED;
                loc_y[k] = p;
            } else {
                for idx in 0..refs.len {
                    let r = refs.refs[idx];
                    conv_states[r.m].items[r.x] = k;
                }
                let r = refs.refs[0];
                loc_m[k] = r.m;
                loc_y[k] = r.x;
            }
        }

        let mut state = Self {
            conv_states,
            fixed,
            cell_refs,
            shared_refs,
            exit_refs: cell_refs[EXIT_P],
            loc_m,
            loc_y,
            delivered: 0,
        };
        state.deliver_if_possible(conveyors);
        state
    }

    #[inline(always)]
    fn logical_index(&self, conveyors: &[Conveyor], m: usize, x: usize) -> usize {
        let len = conveyors[m].len;
        (x + len - self.conv_states[m].offset) % len
    }

    #[inline(always)]
    fn physical_index(&self, conveyors: &[Conveyor], m: usize, y: usize) -> usize {
        let len = conveyors[m].len;
        (y + self.conv_states[m].offset) % len
    }

    #[inline(always)]
    fn get_at_ref(&self, conveyors: &[Conveyor], r: CellRef) -> usize {
        let y = self.logical_index(conveyors, r.m, r.x);
        self.conv_states[r.m].items[y]
    }

    #[inline(always)]
    fn write_at_ref(&mut self, conveyors: &[Conveyor], r: CellRef, k: usize) {
        let y = self.logical_index(conveyors, r.m, r.x);
        self.conv_states[r.m].items[y] = k;
    }

    #[inline(always)]
    fn pos_p(&self, conveyors: &[Conveyor], k: usize) -> Option<usize> {
        debug_assert!(k < NN);

        let m = self.loc_m[k];
        if m == LOC_GONE {
            None
        } else if m == LOC_FIXED {
            Some(self.loc_y[k])
        } else {
            let y = self.loc_y[k];
            let x = self.physical_index(conveyors, m, y);
            Some(conveyors[m].cells[x])
        }
    }

    #[inline(always)]
    fn exit_value(&self, conveyors: &[Conveyor]) -> usize {
        if self.exit_refs.len == 0 {
            self.fixed[EXIT_P]
        } else {
            self.get_at_ref(conveyors, self.exit_refs.refs[0])
        }
    }

    #[inline(always)]
    fn deliver_if_possible(&mut self, conveyors: &[Conveyor]) -> bool {
        if self.delivered >= NN || self.exit_value(conveyors) != self.delivered {
            return false;
        }

        let k = self.delivered;
        if self.exit_refs.len == 0 {
            self.fixed[EXIT_P] = EMPTY;
        } else {
            for idx in 0..self.exit_refs.len {
                self.write_at_ref(conveyors, self.exit_refs.refs[idx], EMPTY);
            }
        }

        self.loc_m[k] = LOC_GONE;
        self.loc_y[k] = EMPTY;
        self.delivered += 1;
        true
    }

    fn apply_op(&mut self, conveyors: &[Conveyor], op: Operation) {
        debug_assert!(op.m < conveyors.len());
        debug_assert!(op.d == -1 || op.d == 1);

        let m = op.m;
        let len = conveyors[m].len;
        let shared_len = self.shared_refs[m].len();

        for idx in 0..shared_len {
            let sh = self.shared_refs[m][idx];
            let y = self.logical_index(conveyors, m, sh.x);
            let k = self.conv_states[m].items[y];
            if k != EMPTY {
                self.loc_m[k] = m;
                self.loc_y[k] = y;
            }
        }

        if op.d == 1 {
            self.conv_states[m].offset += 1;
            if self.conv_states[m].offset == len {
                self.conv_states[m].offset = 0;
            }
        } else if self.conv_states[m].offset == 0 {
            self.conv_states[m].offset = len - 1;
        } else {
            self.conv_states[m].offset -= 1;
        }

        for idx in 0..shared_len {
            let sh = self.shared_refs[m][idx];
            let k = self.get_at_ref(conveyors, CellRef { m, x: sh.x });
            self.write_at_ref(
                conveyors,
                CellRef {
                    m: sh.other_m,
                    x: sh.other_x,
                },
                k,
            );
        }

        self.deliver_if_possible(conveyors);
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionList {
    len: usize,
    idx: [u16; 8],
}

impl ActionList {
    const fn new() -> Self {
        Self {
            len: 0,
            idx: [0; 8],
        }
    }

    fn push(&mut self, action_idx: usize) {
        if self.len < self.idx.len() {
            self.idx[self.len] = action_idx as u16;
            self.len += 1;
        }
    }

    #[inline(always)]
    fn as_slice(&self) -> &[u16] {
        &self.idx[..self.len]
    }
}

#[derive(Debug, Clone)]
struct Layout {
    action_ops: Vec<Operation>,
    next_pos: Vec<[u16; NN]>,
    actions_from: [ActionList; NN],
    dist_to_exit: [i16; NN],
    weight: [i32; LOOK],
    dummy_action: Option<usize>,
}

impl Layout {
    fn new(conveyors: &[Conveyor]) -> Self {
        let mut action_ops = Vec::with_capacity(conveyors.len() * 2);
        let mut next_pos = Vec::with_capacity(conveyors.len() * 2);

        for (m, conveyor) in conveyors.iter().enumerate() {
            for d in [-1i8, 1i8] {
                action_ops.push(Operation { m, d });

                let mut next = [0u16; NN];
                for (p, v) in next.iter_mut().enumerate() {
                    *v = p as u16;
                }

                let c = conveyor.as_slice();
                let len = c.len();
                for x in 0..len {
                    let to = if d == 1 {
                        c[(x + 1) % len]
                    } else {
                        c[(x + len - 1) % len]
                    };
                    next[c[x]] = to as u16;
                }
                next_pos.push(next);
            }
        }

        let mut actions_from = [ActionList::new(); NN];
        for (action_idx, next) in next_pos.iter().enumerate() {
            for p in 0..NN {
                if next[p] as usize != p {
                    actions_from[p].push(action_idx);
                }
            }
        }

        let dist_to_exit = Self::bfs_dist_to_exit(&actions_from, &next_pos);
        let mut weight = [0i32; LOOK];
        for (i, w) in weight.iter_mut().enumerate() {
            *w = (10_000.0 / ((i + 1) as f64).sqrt()) as i32;
        }

        let mut dummy_action = None;
        for (idx, next) in next_pos.iter().enumerate() {
            if next[EXIT_P] as usize == EXIT_P {
                dummy_action = Some(idx);
                break;
            }
        }

        Self {
            action_ops,
            next_pos,
            actions_from,
            dist_to_exit,
            weight,
            dummy_action,
        }
    }

    fn bfs_dist_to_exit(
        actions_from: &[ActionList; NN],
        next_pos: &[[u16; NN]],
    ) -> [i16; NN] {
        let mut dist = [INF_DIST; NN];
        let mut que = VecDeque::new();
        dist[EXIT_P] = 0;
        que.push_back(EXIT_P);

        while let Some(p) = que.pop_front() {
            let nd = dist[p] + 1;
            for &a in actions_from[p].as_slice() {
                let q = next_pos[a as usize][p] as usize;
                if dist[q] == INF_DIST {
                    dist[q] = nd;
                    que.push_back(q);
                }
            }
        }

        dist
    }

    fn all_reachable(&self) -> bool {
        self.dist_to_exit.iter().all(|&d| d < INF_DIST)
    }

    fn static_penalty(&self) -> f64 {
        if !self.all_reachable() || self.dummy_action.is_none() {
            return 1.0e18;
        }

        let mut penalty = 0.0;
        for p in 0..NN {
            let extra = self.dist_to_exit[p] - manhattan_to_exit(p);
            if extra > 0 {
                penalty += f64::from(extra) * 2.0;
            }
        }
        penalty
    }
}

#[derive(Debug, Clone, Copy)]
struct BeamState {
    target_pos: u16,
    future_pos: [u16; LOOK],
    score: i64,
    len: usize,
    seq: [u16; MAX_PATH],
}

impl BeamState {
    fn new(target_pos: usize) -> Self {
        Self {
            target_pos: target_pos as u16,
            future_pos: [NO_POS; LOOK],
            score: 0,
            len: 0,
            seq: [0; MAX_PATH],
        }
    }
}

#[derive(Debug, Clone)]
struct RunResult {
    delivered: usize,
    steps: usize,
    ops: Vec<Operation>,
    failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Layer {
    adj: [[u16; 2]; NN],
}

impl Layer {
    fn empty() -> Self {
        Self {
            adj: [[BAD; 2]; NN],
        }
    }

    #[inline(always)]
    fn has_edge(&self, u: usize, v: usize) -> bool {
        self.adj[u][0] as usize == v || self.adj[u][1] as usize == v
    }

    #[inline(always)]
    fn add_directed(&mut self, u: usize, v: usize) {
        let vv = v as u16;
        if self.adj[u][0] == BAD {
            self.adj[u][0] = vv;
        } else if self.adj[u][1] == BAD {
            debug_assert_ne!(self.adj[u][0], vv);
            self.adj[u][1] = vv;
        } else {
            debug_assert!(false, "degree overflow");
        }
    }

    #[inline(always)]
    fn remove_directed(&mut self, u: usize, v: usize) {
        let vv = v as u16;
        if self.adj[u][0] == vv {
            self.adj[u][0] = self.adj[u][1];
            self.adj[u][1] = BAD;
        } else if self.adj[u][1] == vv {
            self.adj[u][1] = BAD;
        } else {
            debug_assert!(false, "edge not found");
        }
    }

    #[inline(always)]
    fn add_edge(&mut self, u: usize, v: usize) {
        debug_assert!(is_adjacent(u, v));
        debug_assert!(!self.has_edge(u, v));
        self.add_directed(u, v);
        self.add_directed(v, u);
    }

    #[inline(always)]
    fn remove_edge(&mut self, u: usize, v: usize) {
        debug_assert!(self.has_edge(u, v));
        self.remove_directed(u, v);
        self.remove_directed(v, u);
    }

    fn try_flip_square(&mut self, i: usize, j: usize) -> bool {
        debug_assert!(i + 1 < N && j + 1 < N);

        let a = to_p(i, j);
        let b = to_p(i, j + 1);
        let c = to_p(i + 1, j + 1);
        let d = to_p(i + 1, j);

        let ab = self.has_edge(a, b);
        let bc = self.has_edge(b, c);
        let cd = self.has_edge(c, d);
        let da = self.has_edge(d, a);

        if ab && cd && !bc && !da {
            self.remove_edge(a, b);
            self.remove_edge(c, d);
            self.add_edge(a, d);
            self.add_edge(b, c);
            true
        } else if bc && da && !ab && !cd {
            self.remove_edge(b, c);
            self.remove_edge(d, a);
            self.add_edge(a, b);
            self.add_edge(c, d);
            true
        } else {
            false
        }
    }

    fn try_toggle_cycle(&mut self, cycle: &[usize]) -> bool {
        let len = cycle.len();
        if len < 4 || (len & 1) == 1 {
            return false;
        }

        let mut used = [false; NN];
        for &p in cycle {
            if p >= NN || used[p] {
                return false;
            }
            used[p] = true;
        }

        let mut edge_on = [false; 80];
        debug_assert!(len <= edge_on.len());
        for x in 0..len {
            let u = cycle[x];
            let v = cycle[(x + 1) % len];
            if !is_adjacent(u, v) {
                return false;
            }
            edge_on[x] = self.has_edge(u, v);
        }

        // Toggling a simple cycle preserves degree exactly when the layer
        // edges alternate on that cycle. This is the large-cycle analogue of
        // a 2x2 plaquette flip.
        for x in 0..len {
            if edge_on[x] == edge_on[(x + len - 1) % len] {
                return false;
            }
        }

        for x in 0..len {
            let u = cycle[x];
            let v = cycle[(x + 1) % len];
            if edge_on[x] {
                self.remove_edge(u, v);
            }
        }
        for x in 0..len {
            let u = cycle[x];
            let v = cycle[(x + 1) % len];
            if !edge_on[x] {
                self.add_edge(u, v);
            }
        }

        true
    }

    fn try_toggle_rect(&mut self, r0: usize, c0: usize, h: usize, w: usize) -> bool {
        if h == 0 || w == 0 || r0 + h >= N || c0 + w >= N {
            return false;
        }

        let mut cycle = Vec::with_capacity(2 * (h + w));
        for c in c0..=c0 + w {
            cycle.push(to_p(r0, c));
        }
        for r in r0 + 1..=r0 + h {
            cycle.push(to_p(r, c0 + w));
        }
        for c in (c0..c0 + w).rev() {
            cycle.push(to_p(r0 + h, c));
        }
        for r in (r0 + 1..r0 + h).rev() {
            cycle.push(to_p(r, c0));
        }

        self.try_toggle_cycle(&cycle)
    }
}

#[derive(Debug, Clone, Copy)]
struct XorShift {
    x: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        Self { x: seed | 1 }
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn randint(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (self.next() % n as u64) as usize
    }

    #[inline(always)]
    fn uniform01(&mut self) -> f64 {
        ((self.next() >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
    }
}

#[derive(Debug, Clone, Copy)]
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

fn add_cycle_edges(layer: &mut Layer, cells: &[usize]) {
    for x in 0..cells.len() {
        let u = cells[x];
        let v = cells[(x + 1) % cells.len()];
        layer.add_edge(u, v);
    }
}

fn rect_cycle(r0: usize, h: usize, c0: usize, w: usize) -> Vec<usize> {
    let mut c = Vec::with_capacity(h * w);

    for jj in 0..w {
        c.push(to_p(r0, c0 + jj));
    }

    let mut first = true;
    let mut high = w - 1;
    while high >= 1 {
        let start = if first { 1 } else { 2 };
        first = false;
        let ch = c0 + high;

        for ii in start..h {
            c.push(to_p(r0 + ii, ch));
        }
        c.push(to_p(r0 + h - 1, ch - 1));
        for ii in (1..=(h - 2)).rev() {
            c.push(to_p(r0 + ii, ch - 1));
        }
        if high - 1 > 0 {
            c.push(to_p(r0 + 1, ch - 2));
        }

        if high < 2 {
            break;
        }
        high -= 2;
    }

    debug_assert_eq!(c.len(), h * w);
    c
}

fn horizontal_block_cycle(r0: usize, h: usize) -> Vec<usize> {
    let t = rect_cycle(0, N, 0, h);
    let mut c = Vec::with_capacity(N * h);
    for p in t {
        let tr = p / N;
        let tc = p % N;
        c.push(to_p(r0 + tc, tr));
    }
    c
}

fn split_rows(h_block: usize) -> Vec<(usize, usize)> {
    let mut res = Vec::new();
    let mut r = 0usize;
    while r < N {
        let mut h = h_block.min(N - r);
        if h & 1 == 1 {
            h -= 1;
        }
        if h == 0 {
            h = 2;
        }
        res.push((r, h));
        r += h;
    }
    res
}

fn split_cols_around_exit(w_block: usize) -> Vec<(usize, usize)> {
    let mut res = Vec::new();

    let mut x = N / 2;
    let mut left = Vec::new();
    while x > 0 {
        let mut w = w_block.min(x);
        if w & 1 == 1 {
            w -= 1;
        }
        if w == 0 {
            w = 2;
        }
        left.push((x - w, w));
        x -= w;
    }
    left.reverse();
    res.extend(left);

    x = N / 2;
    while x < N {
        let mut w = w_block.min(N - x);
        if w & 1 == 1 {
            w -= 1;
        }
        if w == 0 {
            w = 2;
        }
        res.push((x, w));
        x += w;
    }

    res
}

fn initial_row_layer() -> Layer {
    let mut layer = Layer::empty();
    for (r, h) in split_rows(H_BLOCK) {
        let cells = horizontal_block_cycle(r, h);
        add_cycle_edges(&mut layer, &cells);
    }
    layer
}

fn initial_col_layer() -> Layer {
    let mut layer = Layer::empty();
    for (c, w) in split_cols_around_exit(V_BLOCK) {
        let cells = rect_cycle(0, N, c, w);
        add_cycle_edges(&mut layer, &cells);
    }
    layer
}

fn initial_layers() -> [Layer; 2] {
    [initial_row_layer(), initial_col_layer()]
}

fn append_layer_cycles(layer: &Layer, solution: &mut Solution) {
    let mut used = [false; NN];

    for start in 0..NN {
        if used[start] {
            continue;
        }

        let mut cells = Vec::with_capacity(NN);
        let mut prev = usize::MAX;
        let mut cur = start;

        loop {
            debug_assert!(!used[cur] || cur == start);
            used[cur] = true;
            cells.push(cur);

            let a = layer.adj[cur][0] as usize;
            let b = layer.adj[cur][1] as usize;
            debug_assert!(a < NN && b < NN && a != b);

            let next = if a != prev { a } else { b };
            prev = cur;
            cur = next;

            if cur == start {
                break;
            }
            debug_assert!(cells.len() <= NN);
        }

        debug_assert!(cells.len() >= 4);
        solution.add_conveyor(&cells);
    }
}

fn solution_from_layers(layers: &[Layer; 2]) -> Solution {
    let mut solution = Solution::new();
    append_layer_cycles(&layers[0], &mut solution);
    append_layer_cycles(&layers[1], &mut solution);
    solution
}

fn choose_shortest_path_with_future_tie_break(
    conveyors: &[Conveyor],
    layout: &Layout,
    state: &State,
    target: usize,
) -> (usize, [u16; MAX_PATH]) {
    let Some(start) = state.pos_p(conveyors, target) else {
        return (0, [0; MAX_PATH]);
    };

    let need = layout.dist_to_exit[start];
    if need <= 0 || need == INF_DIST {
        return (0, [0; MAX_PATH]);
    }

    let max_depth = (need as usize).min(MAX_PATH);
    let flen = LOOK.min(NN.saturating_sub(target + 1));

    let mut init = BeamState::new(start);
    for i in 0..flen {
        if let Some(p) = state.pos_p(conveyors, target + 1 + i) {
            init.future_pos[i] = p as u16;
            init.score += i64::from(layout.weight[i]) * i64::from(layout.dist_to_exit[p]);
        }
    }

    let mut beam = Vec::with_capacity(BEAM_WIDTH);
    let mut next_beam = Vec::with_capacity(BEAM_WIDTH * 8);
    beam.push(init);

    for depth in 0..max_depth {
        next_beam.clear();

        for st in &beam {
            let p = st.target_pos as usize;
            let cur_dist = layout.dist_to_exit[p];
            for &a16 in layout.actions_from[p].as_slice() {
                let action_idx = a16 as usize;
                let q = layout.next_pos[action_idx][p] as usize;
                if layout.dist_to_exit[q] + 1 != cur_dist {
                    continue;
                }

                let mut ns = *st;
                ns.target_pos = q as u16;
                ns.seq[depth] = a16;
                ns.len = depth + 1;

                let mut score = st.score;
                for i in 0..flen {
                    let oldp = st.future_pos[i];
                    if oldp == NO_POS {
                        continue;
                    }

                    let oldp = oldp as usize;
                    let newp = layout.next_pos[action_idx][oldp] as usize;
                    ns.future_pos[i] = newp as u16;
                    score += i64::from(layout.weight[i])
                        * i64::from(layout.dist_to_exit[newp] - layout.dist_to_exit[oldp]);
                }
                ns.score = score;

                next_beam.push(ns);
            }
        }

        if next_beam.is_empty() {
            break;
        }

        next_beam.sort_unstable_by(|a, b| {
            a.score
                .cmp(&b.score)
                .then_with(|| a.target_pos.cmp(&b.target_pos))
                .then_with(|| a.seq[0].cmp(&b.seq[0]))
        });
        if next_beam.len() > BEAM_WIDTH {
            next_beam.truncate(BEAM_WIDTH);
        }

        std::mem::swap(&mut beam, &mut next_beam);
    }

    let mut best: Option<usize> = None;
    for (i, st) in beam.iter().enumerate() {
        if st.target_pos as usize != EXIT_P {
            continue;
        }
        if let Some(bi) = best {
            if st.score < beam[bi].score {
                best = Some(i);
            }
        } else {
            best = Some(i);
        }
    }

    if let Some(best) = best {
        (beam[best].len, beam[best].seq)
    } else {
        (0, [0; MAX_PATH])
    }
}

fn plan_with_solution(
    input: &Input,
    base: &Solution,
    target_limit: usize,
    collect_ops: bool,
) -> RunResult {
    let conveyors = &base.conveyors;
    let layout = Layout::new(conveyors);
    if layout.static_penalty() >= 1.0e17 {
        return RunResult {
            delivered: 0,
            steps: 0,
            ops: Vec::new(),
            failed: true,
        };
    }

    let mut state = State::new(input, conveyors);
    let mut ops = if collect_ops {
        Vec::with_capacity(20_000)
    } else {
        Vec::new()
    };
    let limit = target_limit.min(NN);

    while state.delivered < limit && ops.len() < MAX_T {
        if state.pos_p(conveyors, state.delivered) == Some(EXIT_P) {
            let Some(a) = layout.dummy_action else {
                return RunResult {
                    delivered: state.delivered,
                    steps: ops.len(),
                    ops,
                    failed: true,
                };
            };
            let op = layout.action_ops[a];
            state.apply_op(conveyors, op);
            if collect_ops {
                ops.push(op);
            }
            continue;
        }

        let before = state.delivered;
        let (len, seq) =
            choose_shortest_path_with_future_tie_break(conveyors, &layout, &state, state.delivered);
        if len == 0 {
            return RunResult {
                delivered: state.delivered,
                steps: ops.len(),
                ops,
                failed: true,
            };
        }

        for &a16 in seq.iter().take(len) {
            if ops.len() >= MAX_T {
                break;
            }

            let op = layout.action_ops[a16 as usize];
            state.apply_op(conveyors, op);
            if collect_ops {
                ops.push(op);
            }

            if state.delivered > before {
                break;
            }
        }

        if state.delivered == before {
            return RunResult {
                delivered: state.delivered,
                steps: ops.len(),
                ops,
                failed: true,
            };
        }
    }

    let steps = if collect_ops { ops.len() } else { 0 };
    RunResult {
        delivered: state.delivered,
        steps,
        ops,
        failed: state.delivered < limit,
    }
}

fn plan_counting_steps(input: &Input, base: &Solution, target_limit: usize) -> RunResult {
    let conveyors = &base.conveyors;
    let layout = Layout::new(conveyors);
    if layout.static_penalty() >= 1.0e17 {
        return RunResult {
            delivered: 0,
            steps: 0,
            ops: Vec::new(),
            failed: true,
        };
    }

    let mut state = State::new(input, conveyors);
    let mut steps = 0usize;
    let limit = target_limit.min(NN);

    while state.delivered < limit && steps < MAX_T {
        if state.pos_p(conveyors, state.delivered) == Some(EXIT_P) {
            let Some(a) = layout.dummy_action else {
                return RunResult {
                    delivered: state.delivered,
                    steps,
                    ops: Vec::new(),
                    failed: true,
                };
            };
            state.apply_op(conveyors, layout.action_ops[a]);
            steps += 1;
            continue;
        }

        let before = state.delivered;
        let (len, seq) =
            choose_shortest_path_with_future_tie_break(conveyors, &layout, &state, state.delivered);
        if len == 0 {
            return RunResult {
                delivered: state.delivered,
                steps,
                ops: Vec::new(),
                failed: true,
            };
        }

        for &a16 in seq.iter().take(len) {
            if steps >= MAX_T {
                break;
            }
            state.apply_op(conveyors, layout.action_ops[a16 as usize]);
            steps += 1;
            if state.delivered > before {
                break;
            }
        }

        if state.delivered == before {
            return RunResult {
                delivered: state.delivered,
                steps,
                ops: Vec::new(),
                failed: true,
            };
        }
    }

    RunResult {
        delivered: state.delivered,
        steps,
        ops: Vec::new(),
        failed: state.delivered < limit,
    }
}

fn evaluate_layers(input: &Input, layers: &[Layer; 2]) -> f64 {
    let solution = solution_from_layers(layers);
    let layout = Layout::new(&solution.conveyors);
    let static_penalty = layout.static_penalty();
    if static_penalty >= 1.0e17 {
        return 1.0e18;
    }

    let result = plan_counting_steps(input, &solution, EVAL_PREFIX);
    if result.delivered == 0 {
        return 1.0e18;
    }
    if result.failed {
        return 1.0e12 - result.delivered as f64 * 1.0e6 + result.steps as f64;
    }

    result.steps as f64 * (NN as f64 / result.delivered as f64) + static_penalty
}

fn result_key(result: &RunResult) -> i64 {
    if !result.failed && result.delivered == NN {
        result.steps as i64
    } else {
        1_000_000_000_i64 + ((NN - result.delivered) as i64) * 1_000_000 + result.steps as i64
    }
}

fn search_layers(input: &Input, timer: &Timer, rng: &mut XorShift) -> [Layer; 2] {
    let mut cur = initial_layers();
    let initial = cur.clone();
    let mut best = cur.clone();
    let mut cur_score = evaluate_layers(input, &cur);
    let mut best_score = cur_score;

    while timer.elapsed() < SA_TIME_SEC {
        let progress = (timer.elapsed() / SA_TIME_SEC).clamp(0.0, 1.0);
        let temp = 80.0 * (1.0 - progress) + 1.0;

        let layer_idx = if rng.randint(100) < 25 { 0usize } else { 1usize };
        let use_rect = rng.randint(100) < 65;

        let mut square_pos = (0usize, 0usize);
        let mut rect_param = (0usize, 0usize, 0usize, 0usize);

        let applied = if use_rect {
            let r0 = rng.randint(N - 1);
            let c0 = rng.randint(N - 1);
            let max_h = N - 1 - r0;
            let max_w = N - 1 - c0;

            let (h, w) = if rng.randint(100) < 60 {
                (1 + rng.randint(max_h.min(8)), 1 + rng.randint(max_w.min(8)))
            } else {
                (1 + rng.randint(max_h), 1 + rng.randint(max_w))
            };

            rect_param = (r0, c0, h, w);
            cur[layer_idx].try_toggle_rect(r0, c0, h, w)
        } else {
            let i = rng.randint(N - 1);
            let j = rng.randint(N - 1);
            square_pos = (i, j);
            cur[layer_idx].try_flip_square(i, j)
        };

        if !applied {
            continue;
        }

        let next_score = evaluate_layers(input, &cur);
        let accept =
            next_score < cur_score || rng.uniform01() < ((cur_score - next_score) / temp).exp();

        if accept {
            cur_score = next_score;
            if next_score < best_score {
                best_score = next_score;
                best = cur.clone();
            }
        } else {
            let reverted = if use_rect {
                let (r0, c0, h, w) = rect_param;
                cur[layer_idx].try_toggle_rect(r0, c0, h, w)
            } else {
                let (i, j) = square_pos;
                cur[layer_idx].try_flip_square(i, j)
            };
            debug_assert!(reverted);
        }
    }

    if best == initial {
        return initial;
    }

    let initial_solution = solution_from_layers(&initial);
    let best_solution = solution_from_layers(&best);
    let initial_full = plan_counting_steps(input, &initial_solution, NN);
    let best_full = plan_counting_steps(input, &best_solution, NN);

    if result_key(&best_full) < result_key(&initial_full) {
        best
    } else {
        initial
    }
}

fn solve(input: &Input) -> Output {
    let timer = Timer::new();
    let mut seed = 881_726_454_633_252_52u64;
    for &x in &input.a {
        seed = seed
            .wrapping_mul(1_000_003)
            .wrapping_add((x as u64).wrapping_add(1));
    }
    let mut rng = XorShift::new(seed);

    let layers = search_layers(input, &timer, &mut rng);
    let mut solution = solution_from_layers(&layers);
    let result = plan_with_solution(input, &solution, NN, true);
    solution.ops = result.ops;
    solution
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
