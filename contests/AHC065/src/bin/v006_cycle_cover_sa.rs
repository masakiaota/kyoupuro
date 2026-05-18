// v006_cycle_cover_sa.rs
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
const BAD: u16 = u16::MAX;
const INF_DIST: i16 = 30_000;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;
const MAX_LOCAL_PATH: usize = 96;

const LAYOUT_TIME_SEC: f64 = 0.55;
const LAYOUT_EVAL_TARGETS: usize = 120;
const LAYOUT_EVAL_LOOK: usize = 70;
const BEAM_WIDTH: usize = 128;
const EXTRA_DEPTH: usize = 3;
const LOOKAHEAD: usize = 90;

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
    fn remove_box_at_exit(&mut self, k: usize) {
        self.cell_box[EXIT_P] = EMPTY_BOX;
        self.box_pos[k] = EMPTY_BOX;
    }
}

#[derive(Debug, Clone, Copy)]
struct Action {
    to: usize,
    m: usize,
    d: i8,
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

        let a = id(i, j);
        let b = id(i, j + 1);
        let c = id(i + 1, j + 1);
        let d = id(i + 1, j);

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
}

#[derive(Debug, Clone)]
struct Runtime {
    solution: Solution,
    actions: Vec<Vec<Action>>,
    dist_exit: [i16; NN],
}

#[derive(Debug, Clone, Copy)]
struct BeamNode {
    board: Board,
    len: usize,
    last_m: usize,
    last_d: i8,
    path: [Operation; MAX_LOCAL_PATH],
    eval: f64,
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

#[inline(always)]
fn id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn row(p: usize) -> usize {
    p / N
}

#[inline(always)]
fn col(p: usize) -> usize {
    p % N
}

#[inline(always)]
fn adjacent(u: usize, v: usize) -> bool {
    row(u).abs_diff(row(v)) + col(u).abs_diff(col(v)) == 1
}

fn add_cycle_edges(layer: &mut Layer, cells: &[usize]) {
    for x in 0..cells.len() {
        let u = cells[x];
        let v = cells[(x + 1) % cells.len()];
        debug_assert!(adjacent(u, v));
        layer.add_edge(u, v);
    }
}

fn transform_cell(p: usize, mirror_i: bool, mirror_j: bool, transpose: bool) -> usize {
    let mut i = row(p);
    let mut j = col(p);
    if transpose {
        std::mem::swap(&mut i, &mut j);
    }
    if mirror_i {
        i = N - 1 - i;
    }
    if mirror_j {
        j = N - 1 - j;
    }
    id(i, j)
}

fn build_snake_hamiltonian_layer(mirror_i: bool, mirror_j: bool, transpose: bool) -> Layer {
    let mut cells = Vec::with_capacity(NN);

    for j in 0..N {
        cells.push(id(0, j));
    }
    for i in 1..N {
        if i % 2 == 1 {
            for j in (1..N).rev() {
                cells.push(id(i, j));
            }
        } else {
            for j in 1..N {
                cells.push(id(i, j));
            }
        }
    }
    for i in (1..N).rev() {
        cells.push(id(i, 0));
    }

    debug_assert_eq!(cells.len(), NN);

    let mapped = cells
        .iter()
        .map(|&p| transform_cell(p, mirror_i, mirror_j, transpose))
        .collect::<Vec<_>>();

    let mut layer = Layer::empty();
    add_cycle_edges(&mut layer, &mapped);
    layer
}

fn initial_layers() -> [Layer; 2] {
    [
        build_snake_hamiltonian_layer(false, false, false),
        build_snake_hamiltonian_layer(true, false, true),
    ]
}

fn append_layer_cycles(layer: &Layer, solution: &mut Solution, actions: &mut [Vec<Action>]) {
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
        let m = solution.add_conveyor(&cells);
        let len = cells.len();
        for x in 0..len {
            let u = cells[x];
            let v = cells[(x + 1) % len];
            debug_assert!(adjacent(u, v));
            actions[u].push(Action { to: v, m, d: 1 });
            actions[v].push(Action { to: u, m, d: -1 });
        }
    }
}

impl Runtime {
    fn from_layers(layers: &[Layer; 2]) -> Self {
        let mut solution = Solution::new();
        let mut actions = vec![Vec::with_capacity(4); NN];

        append_layer_cycles(&layers[0], &mut solution, &mut actions);
        append_layer_cycles(&layers[1], &mut solution, &mut actions);

        let mut dist_exit = [INF_DIST; NN];
        let mut que = VecDeque::new();
        dist_exit[EXIT_P] = 0;
        que.push_back(EXIT_P);

        while let Some(p) = que.pop_front() {
            let nd = dist_exit[p] + 1;
            for ac in &actions[p] {
                let q = ac.to;
                if dist_exit[q] == INF_DIST {
                    dist_exit[q] = nd;
                    que.push_back(q);
                }
            }
        }

        Self {
            solution,
            actions,
            dist_exit,
        }
    }

    fn all_reachable(&self) -> bool {
        self.dist_exit.iter().all(|&d| d < INF_DIST)
    }
}

fn future_score(board: &Board, dist_exit: &[i16; NN], first_box: usize, look: usize) -> f64 {
    let mut score = 0.0;
    let mut weight = 1.0;
    let last = (NN - 1).min(first_box + look);

    for k in first_box..=last {
        let p = board.box_pos[k];
        if p >= 0 {
            let d = dist_exit[p as usize];
            if d >= INF_DIST {
                score += 100_000.0 * weight;
            } else {
                score += f64::from(d) * weight;
            }
        }
        weight *= 0.93;
    }

    score
}

fn manhattan_to_exit(p: usize) -> i16 {
    (row(p) + col(p).abs_diff(N / 2)) as i16
}

fn evaluate_layout_with(
    input: &Input,
    layers: &[Layer; 2],
    target_count: usize,
    look: usize,
    step_weight: f64,
    inflation_weight: f64,
) -> f64 {
    let runtime = Runtime::from_layers(layers);
    if !runtime.all_reachable() {
        return 1.0e18;
    }

    let mut distance_inflation = 0.0;
    for p in 0..NN {
        let extra = runtime.dist_exit[p] - manhattan_to_exit(p);
        if extra > 8 {
            return 1.0e18;
        }
        if extra > 0 {
            distance_inflation += f64::from(extra);
        }
    }

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;
    if board.cell_box[EXIT_P] == 0 {
        board.remove_box_at_exit(0);
        delivered = 1;
    }

    let target_limit = (delivered + target_count).min(NN);
    let mut steps = 0usize;

    while delivered < target_limit {
        if board.box_pos[delivered] < 0 {
            delivered += 1;
            continue;
        }

        let mut guard = 0usize;
        while board.box_pos[delivered] >= 0 {
            let p = board.box_pos[delivered] as usize;
            let dp = runtime.dist_exit[p];
            if dp <= 0 || dp >= INF_DIST {
                return 1.0e18;
            }

            let mut best: Option<(f64, Operation, Board)> = None;
            for ac in &runtime.actions[p] {
                let q = ac.to;
                if runtime.dist_exit[q] + 1 != dp {
                    continue;
                }

                let op = Operation { m: ac.m, d: ac.d };
                let mut nb = board;
                nb.apply_op(&runtime.solution.conveyors, op);
                let mut next_first = delivered;
                if nb.cell_box[EXIT_P] == delivered as i16 {
                    nb.remove_box_at_exit(delivered);
                    next_first += 1;
                }

                let sc = future_score(&nb, &runtime.dist_exit, next_first + 1, look);
                match best {
                    None => best = Some((sc, op, nb)),
                    Some((bsc, _, _)) if sc < bsc => best = Some((sc, op, nb)),
                    _ => {}
                }
            }

            let Some((_, _op, nb)) = best else {
                return 1.0e18;
            };

            board = nb;
            steps += 1;
            guard += 1;

            if board.box_pos[delivered] < 0 {
                delivered += 1;
                break;
            }
            if guard > 160 || steps > 10_000 {
                return 1.0e18;
            }
        }
    }

    let tail = future_score(&board, &runtime.dist_exit, delivered, look);
    steps as f64 * step_weight + tail + distance_inflation * inflation_weight
}

fn evaluate_layout(input: &Input, layers: &[Layer; 2]) -> f64 {
    evaluate_layout_with(
        input,
        layers,
        LAYOUT_EVAL_TARGETS,
        LAYOUT_EVAL_LOOK,
        28.0,
        12.0,
    )
}

fn search_layers(input: &Input, timer: &Timer, rng: &mut XorShift) -> [Layer; 2] {
    let mut cur = initial_layers();
    let initial = cur.clone();
    let mut best = cur.clone();
    let mut cur_score = evaluate_layout(input, &cur);
    let initial_score = cur_score;
    let mut best_score = cur_score;

    while timer.elapsed() < LAYOUT_TIME_SEC {
        let elapsed = timer.elapsed();
        let progress = (elapsed / LAYOUT_TIME_SEC).clamp(0.0, 1.0);
        let temp = 60.0 * (1.0 - progress) + 0.5;

        let layer_idx = rng.randint(2);
        let i = rng.randint(N - 1);
        let j = rng.randint(N - 1);

        if !cur[layer_idx].try_flip_square(i, j) {
            continue;
        }

        let next_score = evaluate_layout(input, &cur);
        let accept =
            next_score < cur_score || rng.uniform01() < ((cur_score - next_score) / temp).exp();

        if accept {
            cur_score = next_score;
            if next_score < best_score {
                best_score = next_score;
                best = cur.clone();
            }
        } else {
            let reverted = cur[layer_idx].try_flip_square(i, j);
            debug_assert!(reverted);
        }
    }

    if best_score + 80.0 >= initial_score {
        return initial;
    }

    let initial_check = evaluate_layout_with(input, &initial, NN, LOOKAHEAD, 80.0, 20.0);
    let best_check = evaluate_layout_with(input, &best, NN, LOOKAHEAD, 80.0, 20.0);
    if best_check + 1200.0 < initial_check {
        best
    } else {
        initial
    }
}

struct Planner {
    runtime: Runtime,
    answer: Vec<Operation>,
}

impl Planner {
    fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            answer: Vec::with_capacity(20_000),
        }
    }

    fn node_eval(&self, node: &BeamNode, target: usize) -> f64 {
        let p = node.board.box_pos[target];
        let d = if p >= 0 {
            f64::from(self.runtime.dist_exit[p as usize])
        } else {
            0.0
        };

        1000.0 * d
            + future_score(&node.board, &self.runtime.dist_exit, target + 1, LOOKAHEAD)
            + node.len as f64
    }

    fn goal_eval(&self, node: &BeamNode, target: usize) -> f64 {
        future_score(&node.board, &self.runtime.dist_exit, target + 1, LOOKAHEAD) + node.len as f64
    }

    fn fallback_to_exit(&self, start: &Board, target: usize) -> BeamNode {
        let mut cur = BeamNode {
            board: *start,
            len: 0,
            last_m: usize::MAX,
            last_d: 0,
            path: [Operation::default(); MAX_LOCAL_PATH],
            eval: 0.0,
        };

        while cur.board.box_pos[target] >= 0 && cur.len + 1 < MAX_LOCAL_PATH {
            let p = cur.board.box_pos[target] as usize;
            let dp = self.runtime.dist_exit[p];
            if p == EXIT_P || dp <= 0 || dp >= INF_DIST {
                break;
            }

            let mut best: Option<(f64, Operation)> = None;
            for ac in &self.runtime.actions[p] {
                if self.runtime.dist_exit[ac.to] + 1 != dp {
                    continue;
                }
                let op = Operation { m: ac.m, d: ac.d };
                let mut nb = cur.board;
                nb.apply_op(&self.runtime.solution.conveyors, op);
                let mut next_first = target;
                if nb.cell_box[EXIT_P] == target as i16 {
                    nb.remove_box_at_exit(target);
                    next_first += 1;
                }
                let sc = future_score(&nb, &self.runtime.dist_exit, next_first + 1, LOOKAHEAD);
                match best {
                    None => best = Some((sc, op)),
                    Some((bsc, _)) if sc < bsc => best = Some((sc, op)),
                    _ => {}
                }
            }

            let Some((_, op)) = best else {
                break;
            };

            cur.board.apply_op(&self.runtime.solution.conveyors, op);
            cur.path[cur.len] = op;
            cur.len += 1;
            cur.last_m = op.m;
            cur.last_d = op.d;

            if cur.board.cell_box[EXIT_P] == target as i16 {
                cur.board.remove_box_at_exit(target);
                break;
            }
        }

        cur
    }

    fn beam_to_exit(&self, start: &Board, target: usize) -> BeamNode {
        if start.box_pos[target] < 0 {
            return BeamNode {
                board: *start,
                len: 0,
                last_m: usize::MAX,
                last_d: 0,
                path: [Operation::default(); MAX_LOCAL_PATH],
                eval: 0.0,
            };
        }

        let start_pos = start.box_pos[target] as usize;
        let base = self.runtime.dist_exit[start_pos];
        if base <= 0 || base >= INF_DIST {
            return self.fallback_to_exit(start, target);
        }

        let max_depth = (MAX_LOCAL_PATH - 1).min(base as usize + EXTRA_DEPTH);
        let mut beam = Vec::with_capacity(BEAM_WIDTH + 4);
        let mut next_beam = Vec::with_capacity(BEAM_WIDTH * 4 + 16);
        let mut goals = Vec::with_capacity(BEAM_WIDTH * (EXTRA_DEPTH + 2));

        let mut init = BeamNode {
            board: *start,
            len: 0,
            last_m: usize::MAX,
            last_d: 0,
            path: [Operation::default(); MAX_LOCAL_PATH],
            eval: 0.0,
        };
        init.eval = self.node_eval(&init, target);
        beam.push(init);

        for _depth in 0..max_depth {
            next_beam.clear();

            for cur in &beam {
                let p = cur.board.box_pos[target];
                if p < 0 {
                    continue;
                }
                let p = p as usize;

                for ac in &self.runtime.actions[p] {
                    if cur.last_m == ac.m && cur.last_d == -ac.d {
                        continue;
                    }

                    let remaining = max_depth - (cur.len + 1);
                    if self.runtime.dist_exit[ac.to] as usize > remaining {
                        continue;
                    }

                    let op = Operation { m: ac.m, d: ac.d };
                    let mut ns = *cur;
                    ns.board.apply_op(&self.runtime.solution.conveyors, op);
                    ns.path[ns.len] = op;
                    ns.len += 1;
                    ns.last_m = op.m;
                    ns.last_d = op.d;

                    if ns.board.cell_box[EXIT_P] == target as i16 {
                        ns.board.remove_box_at_exit(target);
                        ns.eval = self.goal_eval(&ns, target);
                        goals.push(ns);
                    } else {
                        ns.eval = self.node_eval(&ns, target);
                        next_beam.push(ns);
                    }
                }
            }

            if next_beam.is_empty() {
                break;
            }

            next_beam.sort_by(|a, b| a.eval.total_cmp(&b.eval).then_with(|| a.len.cmp(&b.len)));
            if next_beam.len() > BEAM_WIDTH {
                next_beam.truncate(BEAM_WIDTH);
            }
            std::mem::swap(&mut beam, &mut next_beam);
        }

        if !goals.is_empty() {
            goals.sort_by(|a, b| a.eval.total_cmp(&b.eval).then_with(|| a.len.cmp(&b.len)));
            return goals[0];
        }

        self.fallback_to_exit(start, target)
    }

    fn append_path(&mut self, node: &BeamNode) {
        for i in 0..node.len {
            self.answer.push(node.path[i]);
        }
    }

    fn solve(mut self, input: &Input) -> Output {
        let mut board = Board::from_input(input);
        let mut first = 0usize;

        if board.cell_box[EXIT_P] == 0 {
            board.remove_box_at_exit(0);
            first = 1;
        }

        for target in first..NN {
            let best = self.beam_to_exit(&board, target);
            if self.answer.len() + best.len > MAX_T {
                break;
            }
            self.append_path(&best);
            board = best.board;

            if board.box_pos[target] >= 0 {
                let fb = self.fallback_to_exit(&board, target);
                if self.answer.len() + fb.len > MAX_T {
                    break;
                }
                self.append_path(&fb);
                board = fb.board;
            }
        }

        Solution {
            conveyors: self.runtime.solution.conveyors,
            ops: self.answer,
        }
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

    let initial = initial_layers();
    if std::env::var_os("AHC065_V006_DISABLE_SEARCH").is_some() {
        let runtime = Runtime::from_layers(&initial);
        return Planner::new(runtime).solve(input);
    }

    let layers = search_layers(input, &timer, &mut rng);
    Planner::new(Runtime::from_layers(&layers)).solve(input)
}

fn main() {
    let input = read_input();
    let output = solve(&input);
    output.print();
}
