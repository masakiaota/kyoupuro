// v007_rank_aware_layout.rs
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
const EMPTY_BOX: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;
const INF_DIST: i16 = 30_000;

const FINAL_LOOK: usize = 96;
const FINAL_BEAM: usize = 96;
const FINAL_EXTRA: usize = 3;
const MAX_LOCAL_PATH: usize = 72;
const TIME_LIMIT_SEC: f64 = 1.80;

#[inline(always)]
fn to_p(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn to_ij(p: usize) -> (usize, usize) {
    (p / N, p % N)
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

    #[inline(always)]
    fn at(&self, i: usize, j: usize) -> usize {
        self.a[i * N + j]
    }

    #[inline(always)]
    fn pos_of(&self, k: usize) -> (usize, usize) {
        let p = self.pos_of_box[k];
        (p / N, p % N)
    }
}

fn read_input() -> Input {
    Input::read()
}

#[derive(Debug, Clone, Copy, Default)]
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

#[derive(Debug, Clone, Copy)]
struct Board {
    cell_box: [i16; NN],
    box_pos: [i16; NN],
}

impl Board {
    fn from_input(input: &Input) -> Self {
        let mut cell_box = [EMPTY_BOX; NN];
        let mut box_pos = [EMPTY_BOX; NN];

        for p in 0..NN {
            let k = input.a[p];
            cell_box[p] = k as i16;
            box_pos[k] = p as i16;
        }

        Self { cell_box, box_pos }
    }

    #[inline(always)]
    fn apply_op(&mut self, conveyors: &[Conveyor], op: Operation) {
        debug_assert!(op.m < conveyors.len());
        debug_assert!(op.d == -1 || op.d == 1);

        let c = conveyors[op.m].as_slice();
        let len = c.len();

        if op.d == 1 {
            let last = self.cell_box[c[len - 1]];
            for x in (1..len).rev() {
                let val = self.cell_box[c[x - 1]];
                self.cell_box[c[x]] = val;
                if val != EMPTY_BOX {
                    self.box_pos[val as usize] = c[x] as i16;
                }
            }
            self.cell_box[c[0]] = last;
            if last != EMPTY_BOX {
                self.box_pos[last as usize] = c[0] as i16;
            }
        } else {
            let first = self.cell_box[c[0]];
            for x in 0..(len - 1) {
                let val = self.cell_box[c[x + 1]];
                self.cell_box[c[x]] = val;
                if val != EMPTY_BOX {
                    self.box_pos[val as usize] = c[x] as i16;
                }
            }
            self.cell_box[c[len - 1]] = first;
            if first != EMPTY_BOX {
                self.box_pos[first as usize] = c[len - 1] as i16;
            }
        }
    }

    #[inline(always)]
    fn deliver_if_possible(&mut self, delivered: &mut usize) -> bool {
        if *delivered < NN && self.cell_box[EXIT_P] == *delivered as i16 {
            self.cell_box[EXIT_P] = EMPTY_BOX;
            self.box_pos[*delivered] = EMPTY_BOX;
            *delivered += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline(always)]
    fn is_time_over(&self) -> bool {
        self.elapsed_sec() >= self.time_limit_sec
    }
}

#[derive(Debug, Clone, Copy)]
struct XorShift {
    x: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        Self { x: seed.max(1) }
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn randint(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

type LayerAdj = [[usize; 2]; NN];

#[inline(always)]
fn empty_layer_adj() -> LayerAdj {
    [[usize::MAX; 2]; NN]
}

fn push_layer_edge(adj: &mut LayerAdj, u: usize, v: usize) {
    for x in 0..2 {
        if adj[u][x] == v {
            return;
        }
    }
    let slot_u = if adj[u][0] == usize::MAX { 0 } else { 1 };
    let slot_v = if adj[v][0] == usize::MAX { 0 } else { 1 };
    debug_assert_eq!(adj[u][slot_u], usize::MAX);
    debug_assert_eq!(adj[v][slot_v], usize::MAX);
    adj[u][slot_u] = v;
    adj[v][slot_v] = u;
}

#[inline(always)]
fn has_layer_edge(adj: &LayerAdj, u: usize, v: usize) -> bool {
    adj[u][0] == v || adj[u][1] == v
}

fn replace_layer_neighbor(adj: &mut LayerAdj, u: usize, old_v: usize, new_v: usize) {
    if adj[u][0] == old_v {
        adj[u][0] = new_v;
    } else {
        debug_assert_eq!(adj[u][1], old_v);
        adj[u][1] = new_v;
    }
}

fn remove_layer_edge(adj: &mut LayerAdj, u: usize, v: usize) {
    replace_layer_neighbor(adj, u, v, usize::MAX);
    replace_layer_neighbor(adj, v, u, usize::MAX);
}

fn add_layer_edge(adj: &mut LayerAdj, u: usize, v: usize) {
    replace_layer_neighbor(adj, u, usize::MAX, v);
    replace_layer_neighbor(adj, v, usize::MAX, u);
}

fn try_flip_square(adj: &mut LayerAdj, i: usize, j: usize) -> bool {
    let a = to_p(i, j);
    let b = to_p(i, j + 1);
    let c = to_p(i + 1, j + 1);
    let d = to_p(i + 1, j);

    let has_ab = has_layer_edge(adj, a, b);
    let has_cd = has_layer_edge(adj, c, d);
    let has_ad = has_layer_edge(adj, a, d);
    let has_bc = has_layer_edge(adj, b, c);

    if has_ab && has_cd && !has_ad && !has_bc {
        remove_layer_edge(adj, a, b);
        remove_layer_edge(adj, c, d);
        add_layer_edge(adj, a, d);
        add_layer_edge(adj, b, c);
        true
    } else if has_ad && has_bc && !has_ab && !has_cd {
        remove_layer_edge(adj, a, d);
        remove_layer_edge(adj, b, c);
        add_layer_edge(adj, a, b);
        add_layer_edge(adj, c, d);
        true
    } else {
        false
    }
}

fn layer_from_cycles(cycles: &[Vec<usize>]) -> LayerAdj {
    let mut adj = empty_layer_adj();
    for cyc in cycles {
        let len = cyc.len();
        debug_assert!(len >= 4);
        for x in 0..len {
            let u = cyc[x];
            let v = cyc[(x + 1) % len];
            push_layer_edge(&mut adj, u, v);
        }
    }
    for a in &adj {
        debug_assert_ne!(a[0], usize::MAX);
        debug_assert_ne!(a[1], usize::MAX);
    }
    adj
}

fn vertical_strip_layer() -> LayerAdj {
    let mut cycles = Vec::new();
    for c0 in (0..N).step_by(2) {
        let mut cyc = Vec::with_capacity(2 * N);
        for i in 0..N {
            cyc.push(to_p(i, c0));
        }
        for i in (0..N).rev() {
            cyc.push(to_p(i, c0 + 1));
        }
        cycles.push(cyc);
    }
    layer_from_cycles(&cycles)
}

fn transform_cell(p: usize, mirror_i: bool, mirror_j: bool, transpose: bool) -> usize {
    let (mut i, mut j) = to_ij(p);
    if transpose {
        std::mem::swap(&mut i, &mut j);
    }
    if mirror_i {
        i = N - 1 - i;
    }
    if mirror_j {
        j = N - 1 - j;
    }
    to_p(i, j)
}

fn snake_hamiltonian_layer(mirror_i: bool, mirror_j: bool, transpose: bool) -> LayerAdj {
    let mut cyc = Vec::with_capacity(NN);

    for j in 0..N {
        cyc.push(to_p(0, j));
    }
    for i in 1..N {
        if i % 2 == 1 {
            for j in (1..N).rev() {
                cyc.push(to_p(i, j));
            }
        } else {
            for j in 1..N {
                cyc.push(to_p(i, j));
            }
        }
    }
    for i in (1..N).rev() {
        cyc.push(to_p(i, 0));
    }

    debug_assert_eq!(cyc.len(), NN);
    let mapped = cyc
        .iter()
        .map(|&p| transform_cell(p, mirror_i, mirror_j, transpose))
        .collect::<Vec<_>>();
    layer_from_cycles(&[mapped])
}

fn cycles_from_layer(adj: &LayerAdj) -> Vec<Vec<usize>> {
    let mut visited = [false; NN];
    let mut cycles = Vec::new();

    for start in 0..NN {
        if visited[start] {
            continue;
        }

        let mut cyc = Vec::new();
        let mut prev = usize::MAX;
        let mut cur = start;

        loop {
            if visited[cur] {
                break;
            }
            visited[cur] = true;
            cyc.push(cur);

            let next = if adj[cur][0] != prev {
                adj[cur][0]
            } else {
                adj[cur][1]
            };
            prev = cur;
            cur = next;
            if cur == start {
                break;
            }
        }

        debug_assert!(cyc.len() >= 4);
        cycles.push(cyc);
    }

    cycles
}

fn horizontal_layer_conveyors() -> Vec<Conveyor> {
    let mut conveyors = Vec::new();
    for r in (0..N).step_by(2) {
        let mut cells = Vec::with_capacity(2 * N);
        for j in 0..N {
            cells.push(to_p(r, j));
        }
        for j in (0..N).rev() {
            cells.push(to_p(r + 1, j));
        }
        conveyors.push(Conveyor::from_slice(&cells));
    }
    conveyors
}

fn vertical_layer_conveyors() -> Vec<Conveyor> {
    let mut conveyors = Vec::new();
    for c0 in (0..N).step_by(2) {
        let mut cells = Vec::with_capacity(2 * N);
        for i in 0..N {
            cells.push(to_p(i, c0));
        }
        for i in (0..N).rev() {
            cells.push(to_p(i, c0 + 1));
        }
        conveyors.push(Conveyor::from_slice(&cells));
    }
    conveyors
}

fn build_conveyors_from_second_layer(second: &LayerAdj) -> Vec<Conveyor> {
    let mut conveyors = horizontal_layer_conveyors();
    for cyc in cycles_from_layer(second) {
        conveyors.push(Conveyor::from_slice(&cyc));
    }
    conveyors
}

fn build_conveyors_from_first_and_second_layer(
    mut first: Vec<Conveyor>,
    second: &LayerAdj,
) -> Vec<Conveyor> {
    for cyc in cycles_from_layer(second) {
        first.push(Conveyor::from_slice(&cyc));
    }
    first
}

fn build_conveyors_from_second_cycles(second_cycles: Vec<Vec<usize>>) -> Vec<Conveyor> {
    let mut conveyors = horizontal_layer_conveyors();
    for cyc in second_cycles {
        conveyors.push(Conveyor::from_slice(&cyc));
    }
    conveyors
}

fn build_conveyors_from_vertical_and_second_cycles(
    second_cycles: Vec<Vec<usize>>,
) -> Vec<Conveyor> {
    let mut conveyors = vertical_layer_conveyors();
    for cyc in second_cycles {
        conveyors.push(Conveyor::from_slice(&cyc));
    }
    conveyors
}

fn segmented_vertical_conveyors(chunk_h: usize) -> Vec<Conveyor> {
    let mut cycles = Vec::new();
    for c0 in (0..N).step_by(2) {
        let mut r0 = 0usize;
        while r0 < N {
            let r1 = (r0 + chunk_h).min(N);
            let mut cyc = Vec::with_capacity(2 * (r1 - r0));
            for i in r0..r1 {
                cyc.push(to_p(i, c0));
            }
            for i in (r0..r1).rev() {
                cyc.push(to_p(i, c0 + 1));
            }
            cycles.push(cyc);
            r0 = r1;
        }
    }
    build_conveyors_from_second_cycles(cycles)
}

fn rank_aware_segmented_vertical_conveyors(input: &Input, variant: usize) -> Vec<Conveyor> {
    let mut cycles = Vec::new();
    let threshold = match variant {
        0 => 60,
        1 => 100,
        _ => 150,
    };
    let short_h = match variant {
        0 => 3,
        1 => 4,
        _ => 5,
    };
    let long_h = match variant {
        0 => 10,
        1 => 8,
        _ => 7,
    };

    for c0 in (0..N).step_by(2) {
        let mut row_best = [NN; N];
        for (i, best) in row_best.iter_mut().enumerate() {
            *best = input.a[to_p(i, c0)].min(input.a[to_p(i, c0 + 1)]);
        }

        let mut r0 = 0usize;
        while r0 < N {
            let probe_end = (r0 + long_h).min(N);
            let important = row_best[r0..probe_end].iter().any(|&rank| rank < threshold);
            let mut h = if important { short_h } else { long_h };
            if N - r0 <= h {
                h = N - r0;
            } else if N - (r0 + h) == 1 {
                h += 1;
            }

            let r1 = r0 + h;
            let mut cyc = Vec::with_capacity(2 * h);
            for i in r0..r1 {
                cyc.push(to_p(i, c0));
            }
            for i in (r0..r1).rev() {
                cyc.push(to_p(i, c0 + 1));
            }
            cycles.push(cyc);
            r0 = r1;
        }
    }

    build_conveyors_from_second_cycles(cycles)
}

fn segmented_horizontal_conveyors(chunk_w: usize) -> Vec<Conveyor> {
    let mut cycles = Vec::new();
    for r0 in (0..N).step_by(2) {
        let mut c0 = 0usize;
        while c0 < N {
            let c1 = (c0 + chunk_w).min(N);
            let mut cyc = Vec::with_capacity(2 * (c1 - c0));
            for j in c0..c1 {
                cyc.push(to_p(r0, j));
            }
            for j in (c0..c1).rev() {
                cyc.push(to_p(r0 + 1, j));
            }
            cycles.push(cyc);
            c0 = c1;
        }
    }
    build_conveyors_from_vertical_and_second_cycles(cycles)
}

fn rank_aware_segmented_horizontal_conveyors(input: &Input, variant: usize) -> Vec<Conveyor> {
    let mut cycles = Vec::new();
    let threshold = match variant {
        0 => 60,
        1 => 100,
        _ => 150,
    };
    let short_w = match variant {
        0 => 3,
        1 => 4,
        _ => 5,
    };
    let long_w = match variant {
        0 => 10,
        1 => 8,
        _ => 7,
    };

    for r0 in (0..N).step_by(2) {
        let mut col_best = [NN; N];
        for (j, best) in col_best.iter_mut().enumerate() {
            *best = input.a[to_p(r0, j)].min(input.a[to_p(r0 + 1, j)]);
        }

        let mut c0 = 0usize;
        while c0 < N {
            let probe_end = (c0 + long_w).min(N);
            let important = col_best[c0..probe_end].iter().any(|&rank| rank < threshold);
            let mut w = if important { short_w } else { long_w };
            if N - c0 <= w {
                w = N - c0;
            } else if N - (c0 + w) == 1 {
                w += 1;
            }

            let c1 = c0 + w;
            let mut cyc = Vec::with_capacity(2 * w);
            for j in c0..c1 {
                cyc.push(to_p(r0, j));
            }
            for j in (c0..c1).rev() {
                cyc.push(to_p(r0 + 1, j));
            }
            cycles.push(cyc);
            c0 = c1;
        }
    }

    build_conveyors_from_vertical_and_second_cycles(cycles)
}

#[derive(Debug, Clone, Copy, Default)]
struct Action {
    to: usize,
    op: Operation,
    op_id: usize,
}

#[derive(Debug, Clone)]
struct PlannerData {
    actions: Vec<Vec<Action>>,
    next_pos: Vec<[usize; NN]>,
    dist: [i16; NN],
    op_len: Vec<usize>,
}

fn build_planner_data(conveyors: &[Conveyor]) -> PlannerData {
    let mut actions = vec![Vec::new(); NN];
    let mut next_pos = Vec::with_capacity(conveyors.len() * 2);
    let mut op_len = Vec::with_capacity(conveyors.len() * 2);

    for (m, conveyor) in conveyors.iter().enumerate() {
        let cells = conveyor.as_slice();
        let len = cells.len();

        let op_id_neg = next_pos.len();
        let mut next_neg = [0usize; NN];
        for (p, to) in next_neg.iter_mut().enumerate() {
            *to = p;
        }
        for x in 0..len {
            let u = cells[x];
            let v = cells[(x + len - 1) % len];
            next_neg[u] = v;
            actions[u].push(Action {
                to: v,
                op: Operation { m, d: -1 },
                op_id: op_id_neg,
            });
        }
        next_pos.push(next_neg);
        op_len.push(len);

        let op_id_pos = next_pos.len();
        let mut next_pos_one = [0usize; NN];
        for (p, to) in next_pos_one.iter_mut().enumerate() {
            *to = p;
        }
        for x in 0..len {
            let u = cells[x];
            let v = cells[(x + 1) % len];
            next_pos_one[u] = v;
            actions[u].push(Action {
                to: v,
                op: Operation { m, d: 1 },
                op_id: op_id_pos,
            });
        }
        next_pos.push(next_pos_one);
        op_len.push(len);
    }

    let mut dist = [INF_DIST; NN];
    let mut que = VecDeque::new();
    dist[EXIT_P] = 0;
    que.push_back(EXIT_P);

    while let Some(p) = que.pop_front() {
        let nd = dist[p] + 1;
        for ac in &actions[p] {
            if dist[ac.to] == INF_DIST {
                dist[ac.to] = nd;
                que.push_back(ac.to);
            }
        }
    }

    PlannerData {
        actions,
        next_pos,
        dist,
        op_len,
    }
}

fn seed_from_input(input: &Input) -> u64 {
    let mut seed = 88_172_645_463_393_265u64;
    for &x in &input.a {
        seed ^= (x as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        seed = seed.rotate_left(9).wrapping_mul(1_000_003);
    }
    seed
}

fn weighted_squares(input: &Input) -> Vec<(usize, usize)> {
    let mut items = Vec::new();
    for i in 0..N - 1 {
        for j in 0..N - 1 {
            let mut best_rank = NN;
            let cells = [
                to_p(i, j),
                to_p(i + 1, j),
                to_p(i, j + 1),
                to_p(i + 1, j + 1),
            ];
            for &p in &cells {
                best_rank = best_rank.min(input.a[p]);
            }
            let weight = if best_rank < 160 {
                12 + (160 - best_rank) / 8
            } else if best_rank < 320 {
                4
            } else {
                1
            };
            for _ in 0..weight {
                items.push((i, j));
            }
        }
    }
    items
}

fn future_score(board: &Board, data: &PlannerData, first_box: usize, look: usize) -> f64 {
    let mut score = 0.0;
    let mut w = 1.0;
    let last = (NN - 1).min(first_box + look - 1);

    for k in first_box..=last {
        let p = board.box_pos[k];
        if p >= 0 {
            let d = data.dist[p as usize];
            score += w * f64::from(d);
        }
        w *= 0.94;
    }

    score
}

fn layout_proxy(input: &Input, conveyors: &[Conveyor]) -> f64 {
    const PROXY_TARGETS: usize = 120;
    const PROXY_LOOK: usize = 48;

    let data = build_planner_data(conveyors);
    if data.dist.iter().any(|&d| d == INF_DIST) {
        return 1.0e100;
    }

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;
    board.deliver_if_possible(&mut delivered);

    let mut steps = 0usize;
    let stop_at = PROXY_TARGETS.min(NN);

    while delivered < stop_at && steps < 5000 {
        let target = delivered;
        let p0 = board.box_pos[target];
        if p0 < 0 {
            delivered += 1;
            continue;
        }
        let p = p0 as usize;
        if p == EXIT_P {
            if !board.deliver_if_possible(&mut delivered) {
                break;
            }
            continue;
        }

        let cur_d = data.dist[p];
        let mut best: Option<Action> = None;
        let mut best_eval = f64::INFINITY;

        for &ac in &data.actions[p] {
            if data.dist[ac.to] + 1 != cur_d {
                continue;
            }

            let mut eval = 0.0;
            let mut w = 1.0;
            let last = (NN - 1).min(target + PROXY_LOOK);
            for k in (target + 1)..=last {
                let oldp = board.box_pos[k];
                if oldp >= 0 {
                    let newp = data.next_pos[ac.op_id][oldp as usize];
                    eval += w * f64::from(data.dist[newp]);
                }
                w *= 0.94;
            }
            eval += 0.002 * data.op_len[ac.op_id] as f64;

            if eval < best_eval {
                best_eval = eval;
                best = Some(ac);
            }
        }

        let Some(ac) = best else {
            return 1.0e90;
        };

        board.apply_op(conveyors, ac.op);
        steps += 1;
        board.deliver_if_possible(&mut delivered);
    }

    let remain = future_score(&board, &data, delivered, 120);
    steps as f64 + 0.35 * remain
}

fn choose_conveyors(input: &Input, timer: &TimeKeeper) -> Vec<Conveyor> {
    let mut conveyor_candidates = Vec::new();
    conveyor_candidates.push(build_conveyors_from_second_layer(&vertical_strip_layer()));

    for &chunk_h in &[2usize, 4, 5, 10] {
        conveyor_candidates.push(segmented_vertical_conveyors(chunk_h));
    }
    for variant in 0..3 {
        conveyor_candidates.push(rank_aware_segmented_vertical_conveyors(input, variant));
    }
    for &chunk_w in &[2usize, 4, 5, 10] {
        conveyor_candidates.push(segmented_horizontal_conveyors(chunk_w));
    }
    for variant in 0..3 {
        conveyor_candidates.push(rank_aware_segmented_horizontal_conveyors(input, variant));
    }

    let mut layer_candidates = Vec::new();

    for &transpose in &[false, true] {
        for &mirror_i in &[false, true] {
            for &mirror_j in &[false, true] {
                layer_candidates.push(snake_hamiltonian_layer(mirror_i, mirror_j, transpose));
            }
        }
    }

    let base = vertical_strip_layer();
    let squares = weighted_squares(input);
    let mut rng = XorShift::new(seed_from_input(input));

    for sample in 0..18 {
        if timer.elapsed_sec() > 0.55 {
            break;
        }

        let mut adj = base;
        let flips = 80 + 20 * (sample % 8);
        let mut accepted = 0usize;

        for t in 0..(flips * 6) {
            if accepted >= flips {
                break;
            }

            let (i, j) = if !squares.is_empty() && rng.randint(100) < 75 {
                squares[rng.randint(squares.len())]
            } else {
                (rng.randint(N - 1), rng.randint(N - 1))
            };

            if try_flip_square(&mut adj, i, j) {
                accepted += 1;
            }

            if (t & 63) == 0 && timer.elapsed_sec() > 0.70 {
                break;
            }
        }

        layer_candidates.push(adj);
    }

    for adj in layer_candidates {
        conveyor_candidates.push(build_conveyors_from_second_layer(&adj));
        conveyor_candidates.push(build_conveyors_from_first_and_second_layer(
            vertical_layer_conveyors(),
            &adj,
        ));
    }

    let mut best_conveyors = conveyor_candidates[0].clone();
    let mut best_score = f64::INFINITY;

    for conveyors in conveyor_candidates {
        if timer.elapsed_sec() > 1.05 {
            break;
        }

        let score = layout_proxy(input, &conveyors);
        if score < best_score {
            best_score = score;
            best_conveyors = conveyors;
        }
    }

    best_conveyors
}

#[derive(Debug, Clone)]
struct BeamState {
    target_pos: usize,
    fp: [u16; FINAL_LOOK],
    flen: usize,
    seq: [Operation; MAX_LOCAL_PATH],
    slen: usize,
    last_op_id: usize,
    score: f64,
}

impl BeamState {
    fn new(target_pos: usize, flen: usize) -> Self {
        Self {
            target_pos,
            fp: [0; FINAL_LOOK],
            flen,
            seq: [Operation { m: 0, d: 1 }; MAX_LOCAL_PATH],
            slen: 0,
            last_op_id: usize::MAX,
            score: 0.0,
        }
    }
}

fn final_future_score(st: &BeamState, data: &PlannerData, weight: &[f64; FINAL_LOOK]) -> f64 {
    let mut score = 0.0;
    for (idx, &w) in weight.iter().enumerate().take(st.flen) {
        score += w * f64::from(data.dist[st.fp[idx] as usize]);
    }
    score
}

fn shortest_path_ops(start: usize, data: &PlannerData) -> Vec<Operation> {
    let mut ops = Vec::new();
    let mut p = start;
    while p != EXIT_P && ops.len() < MAX_LOCAL_PATH {
        let cur_d = data.dist[p];
        let mut best: Option<Action> = None;
        let mut best_len = usize::MAX;
        for &ac in &data.actions[p] {
            if data.dist[ac.to] + 1 == cur_d && data.op_len[ac.op_id] < best_len {
                best = Some(ac);
                best_len = data.op_len[ac.op_id];
            }
        }
        let Some(ac) = best else {
            break;
        };
        ops.push(ac.op);
        p = ac.to;
    }
    ops
}

fn beam_route_to_exit(board: &Board, target: usize, data: &PlannerData) -> Vec<Operation> {
    let p0 = board.box_pos[target];
    if p0 < 0 {
        return Vec::new();
    }
    let start = p0 as usize;
    if start == EXIT_P {
        return Vec::new();
    }

    let need = data.dist[start];
    if need == INF_DIST {
        return Vec::new();
    }

    let max_depth = (need as usize + FINAL_EXTRA).min(MAX_LOCAL_PATH - 1);
    let flen = FINAL_LOOK.min(NN.saturating_sub(target + 1));

    let mut weight = [0.0f64; FINAL_LOOK];
    for (i, w) in weight.iter_mut().enumerate() {
        *w = 1.0 / ((i + 1) as f64).sqrt();
    }

    let mut init = BeamState::new(start, flen);
    for i in 0..flen {
        let p = board.box_pos[target + 1 + i];
        init.fp[i] = if p >= 0 { p as u16 } else { EXIT_P as u16 };
    }
    init.score = 900.0 * f64::from(data.dist[start]) + final_future_score(&init, data, &weight);

    let mut beam = Vec::with_capacity(FINAL_BEAM);
    let mut goals = Vec::with_capacity(FINAL_BEAM);
    beam.push(init);

    for _depth in 0..max_depth {
        let mut next_beam = Vec::with_capacity(FINAL_BEAM * 6);

        for st in &beam {
            let cur_d = data.dist[st.target_pos];
            if cur_d == 0 {
                goals.push(st.clone());
                continue;
            }

            let remaining_after = max_depth - st.slen - 1;
            for &ac in &data.actions[st.target_pos] {
                if st.last_op_id != usize::MAX && ac.op_id == (st.last_op_id ^ 1) {
                    continue;
                }
                if data.dist[ac.to] as usize > remaining_after {
                    continue;
                }
                if data.dist[ac.to] > cur_d + 1 {
                    continue;
                }

                let mut ns = st.clone();
                ns.target_pos = ac.to;
                ns.seq[ns.slen] = ac.op;
                ns.slen += 1;
                ns.last_op_id = ac.op_id;

                let mut fs = 0.0;
                for i in 0..ns.flen {
                    let oldp = st.fp[i] as usize;
                    let newp = data.next_pos[ac.op_id][oldp];
                    ns.fp[i] = newp as u16;
                    fs += weight[i] * f64::from(data.dist[newp]);
                }

                if ns.target_pos == EXIT_P {
                    ns.score = fs + 0.85 * ns.slen as f64 + 0.001 * data.op_len[ac.op_id] as f64;
                    goals.push(ns);
                } else {
                    ns.score = 900.0 * f64::from(data.dist[ns.target_pos])
                        + fs
                        + 0.85 * ns.slen as f64
                        + 0.001 * data.op_len[ac.op_id] as f64;
                    next_beam.push(ns);
                }
            }
        }

        if next_beam.is_empty() {
            break;
        }
        next_beam.sort_by(|a, b| {
            a.score
                .total_cmp(&b.score)
                .then_with(|| a.slen.cmp(&b.slen))
        });
        if next_beam.len() > FINAL_BEAM {
            next_beam.truncate(FINAL_BEAM);
        }
        beam = next_beam;
    }

    if goals.is_empty() {
        return shortest_path_ops(start, data);
    }

    goals.sort_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then_with(|| a.slen.cmp(&b.slen))
    });
    let best = &goals[0];
    best.seq[..best.slen].to_vec()
}

fn solve(input: &Input) -> Output {
    let timer = TimeKeeper::new(TIME_LIMIT_SEC);
    let conveyors = choose_conveyors(input, &timer);
    let data = build_planner_data(&conveyors);

    let mut solution = Solution {
        conveyors,
        ops: Vec::with_capacity(MAX_T),
    };

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;
    board.deliver_if_possible(&mut delivered);

    while delivered < NN && solution.ops.len() < MAX_T {
        if timer.is_time_over() {
            break;
        }

        let target = delivered;
        if board.box_pos[target] < 0 {
            delivered += 1;
            continue;
        }

        if board.box_pos[target] as usize == EXIT_P {
            if board.deliver_if_possible(&mut delivered) {
                continue;
            }
        }

        let ops = beam_route_to_exit(&board, target, &data);
        if ops.is_empty() {
            break;
        }

        let before = delivered;
        for op in ops {
            if solution.ops.len() >= MAX_T {
                break;
            }
            board.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);
            board.deliver_if_possible(&mut delivered);
            if delivered > before {
                break;
            }
        }

        if delivered == before {
            break;
        }
    }

    solution
}

fn main() {
    let input = read_input();
    let output = solve(&input);
    output.print();
}
