// v005_funnel_highway.rs
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

const N: usize = 20;
const NN: usize = N * N;
const EXIT_COL: usize = N / 2;
const E: (usize, usize) = (0, EXIT_COL);
const EXIT_P: usize = E.0 * N + E.1;
const EMPTY_BOX: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;

const KLOOK: usize = 120;
const BEAM_WIDTH: usize = 192;
const MAXD: usize = 80;
const HIGHWAY_COLS: [usize; 5] = [2, 6, EXIT_COL, 14, 18];
const EXTRA_DEPTH: usize = 2;
const TARGET_DIST_WEIGHT: f64 = 24.0;
const LEN_PENALTY: f64 = 1.2;

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
struct Action {
    to: usize,
    op: Operation,
}

#[derive(Debug, Clone)]
struct BeamState {
    target_pos: usize,
    flen: usize,
    future_pos: [u16; KLOOK],
    score: f64,
    slen: usize,
    seq: [Operation; MAXD],
}

impl BeamState {
    fn new(target_pos: usize, flen: usize) -> Self {
        Self {
            target_pos,
            flen,
            future_pos: [0; KLOOK],
            score: 0.0,
            slen: 0,
            seq: [Operation { m: 0, d: 1 }; MAXD],
        }
    }
}

#[inline(always)]
fn id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn dir_idx(d: i8) -> usize {
    if d == 1 { 1 } else { 0 }
}

fn build_loops(solution: &mut Solution) {
    // Feeder: all 2-row rings.  Every cell can be moved horizontally toward a highway.
    for r in (0..N).step_by(2) {
        let mut cells = Vec::with_capacity(2 * N);
        for j in 0..N {
            cells.push(id(r, j));
        }
        for j in (0..N).rev() {
            cells.push(id(r + 1, j));
        }
        solution.add_conveyor(&cells);
    }

    // Highway: sparse vertical rings.  They create strong upward flows and share cells
    // with feeders as transfer points.  The middle highway contains E.
    for &c0 in &HIGHWAY_COLS {
        let mut cells = Vec::with_capacity(2 * N);
        for i in 0..N {
            cells.push(id(i, c0));
        }
        for i in (0..N).rev() {
            cells.push(id(i, c0 + 1));
        }
        solution.add_conveyor(&cells);
    }

    debug_assert!(is_valid_cover(&solution.conveyors));
}

fn is_valid_cover(conveyors: &[Conveyor]) -> bool {
    let mut cover = [0u8; NN];
    for conveyor in conveyors {
        for &p in conveyor.as_slice() {
            cover[p] += 1;
            if cover[p] > 2 {
                return false;
            }
        }
    }
    true
}

fn build_next_cell(conveyors: &[Conveyor]) -> Vec<[[usize; NN]; 2]> {
    let mut next_cell = vec![[[0usize; NN]; 2]; conveyors.len()];

    for (m, conveyor) in conveyors.iter().enumerate() {
        for p in 0..NN {
            next_cell[m][0][p] = p;
            next_cell[m][1][p] = p;
        }

        let cells = conveyor.as_slice();
        let len = cells.len();
        for x in 0..len {
            let u = cells[x];
            next_cell[m][1][u] = cells[(x + 1) % len];
            next_cell[m][0][u] = cells[(x + len - 1) % len];
        }
    }

    next_cell
}

fn build_actions(conveyors: &[Conveyor]) -> Vec<Vec<Action>> {
    let mut actions = vec![Vec::with_capacity(6); NN];

    for (m, conveyor) in conveyors.iter().enumerate() {
        let cells = conveyor.as_slice();
        let len = cells.len();
        for x in 0..len {
            let u = cells[x];
            let next = cells[(x + 1) % len];
            let prev = cells[(x + len - 1) % len];
            actions[u].push(Action {
                to: next,
                op: Operation { m, d: 1 },
            });
            actions[u].push(Action {
                to: prev,
                op: Operation { m, d: -1 },
            });
        }
    }

    actions
}

fn build_dist_to_exit(actions: &[Vec<Action>]) -> [i16; NN] {
    let mut dist = [i16::MAX; NN];
    let mut que = VecDeque::new();
    dist[EXIT_P] = 0;
    que.push_back(EXIT_P);

    while let Some(p) = que.pop_front() {
        let nd = dist[p] + 1;
        for ac in &actions[p] {
            if dist[ac.to] == i16::MAX {
                dist[ac.to] = nd;
                que.push_back(ac.to);
            }
        }
    }

    debug_assert!(dist.iter().all(|&d| d < i16::MAX));
    dist
}

fn build_loop_contains_exit(conveyors: &[Conveyor]) -> Vec<bool> {
    conveyors
        .iter()
        .map(|c| c.as_slice().iter().any(|&p| p == EXIT_P))
        .collect()
}

fn build_pos_cost(dist_to_exit: &[i16; NN]) -> [f64; NN] {
    let mut cost = [0.0; NN];

    for p in 0..NN {
        let j = p % N;
        let highway_dist = HIGHWAY_COLS
            .iter()
            .map(|&c0| {
                let d0 = j.abs_diff(c0);
                let d1 = j.abs_diff(c0 + 1);
                d0.min(d1)
            })
            .min()
            .unwrap();
        let center_dist = j.abs_diff(EXIT_COL).min(j.abs_diff(EXIT_COL + 1));

        cost[p] =
            f64::from(dist_to_exit[p]) + 0.20 * highway_dist as f64 + 0.10 * center_dist as f64;
    }

    cost
}

fn initial_future_score(st: &BeamState, pos_cost: &[f64; NN], weight: &[f64; KLOOK]) -> f64 {
    let mut score = 0.0;
    for i in 0..st.flen {
        score += weight[i] * pos_cost[st.future_pos[i] as usize];
    }
    score
}

fn choose_idle_op(
    board: &Board,
    delivered: usize,
    conveyors: &[Conveyor],
    next_cell: &[[[usize; NN]; 2]],
    loop_contains_exit: &[bool],
    pos_cost: &[f64; NN],
    weight: &[f64; KLOOK],
) -> Operation {
    let mut best_op = Operation { m: 1, d: 1 };
    let mut best_delta = f64::INFINITY;
    let flen = KLOOK.min(NN - (delivered + 1));

    for (m, _) in conveyors.iter().enumerate() {
        if loop_contains_exit[m] {
            continue;
        }
        for d in [-1i8, 1i8] {
            let di = dir_idx(d);
            let mut delta = 0.0;
            for r in 0..flen {
                let k = delivered + 1 + r;
                let p = board.box_pos[k];
                if p < 0 {
                    continue;
                }
                let oldp = p as usize;
                let newp = next_cell[m][di][oldp];
                delta += weight[r] * (pos_cost[newp] - pos_cost[oldp]);
            }

            if delta < best_delta {
                best_delta = delta;
                best_op = Operation { m, d };
            }
        }
    }

    best_op
}

fn choose_path_to_exit(
    board: &Board,
    target: usize,
    actions: &[Vec<Action>],
    next_cell: &[[[usize; NN]; 2]],
    dist_to_exit: &[i16; NN],
    pos_cost: &[f64; NN],
    weight: &[f64; KLOOK],
) -> BeamState {
    let start = board.box_pos[target] as usize;
    let need = dist_to_exit[start] as usize;
    let max_depth = (need + EXTRA_DEPTH).min(MAXD - 1);
    debug_assert!(need <= max_depth);

    let flen = KLOOK.min(NN - (target + 1));
    let mut init = BeamState::new(start, flen);
    for i in 0..flen {
        let p = board.box_pos[target + 1 + i];
        init.future_pos[i] = if p >= 0 { p as u16 } else { EXIT_P as u16 };
    }
    init.score = initial_future_score(&init, pos_cost, weight);

    let mut beam = Vec::with_capacity(BEAM_WIDTH);
    let mut goals = Vec::with_capacity(BEAM_WIDTH * (EXTRA_DEPTH + 1));
    beam.push(init);

    for _depth in 0..max_depth {
        let mut next_beam = Vec::with_capacity(BEAM_WIDTH * 4);

        for st in &beam {
            for &ac in &actions[st.target_pos] {
                if st.slen + 1 > max_depth {
                    continue;
                }
                let remaining = max_depth - (st.slen + 1);
                if dist_to_exit[ac.to] as usize > remaining {
                    continue;
                }

                let mut ns = st.clone();
                ns.target_pos = ac.to;
                ns.seq[st.slen] = ac.op;
                ns.slen = st.slen + 1;

                let di = dir_idx(ac.op.d);
                let mut score = st.score;
                for i in 0..st.flen {
                    let oldp = st.future_pos[i] as usize;
                    let newp = next_cell[ac.op.m][di][oldp];
                    ns.future_pos[i] = newp as u16;
                    score += weight[i] * (pos_cost[newp] - pos_cost[oldp]);
                }
                ns.score = score;

                if ns.target_pos == EXIT_P {
                    goals.push(ns);
                } else {
                    next_beam.push(ns);
                }
            }
        }

        if next_beam.is_empty() {
            break;
        }

        next_beam.sort_by(|a, b| {
            let ae = a.score
                + TARGET_DIST_WEIGHT * f64::from(dist_to_exit[a.target_pos])
                + LEN_PENALTY * a.slen as f64;
            let be = b.score
                + TARGET_DIST_WEIGHT * f64::from(dist_to_exit[b.target_pos])
                + LEN_PENALTY * b.slen as f64;
            ae.total_cmp(&be)
                .then_with(|| a.target_pos.cmp(&b.target_pos))
        });
        if next_beam.len() > BEAM_WIDTH {
            next_beam.truncate(BEAM_WIDTH);
        }
        beam = next_beam;
    }

    if !goals.is_empty() {
        let mut best = 0usize;
        for i in 1..goals.len() {
            let a = goals[i].score + LEN_PENALTY * goals[i].slen as f64;
            let b = goals[best].score + LEN_PENALTY * goals[best].slen as f64;
            if a < b {
                best = i;
            }
        }
        return goals[best].clone();
    }

    let mut best = 0usize;
    for i in 1..beam.len() {
        if beam[i].score < beam[best].score {
            best = i;
        }
    }
    beam[best].clone()
}

fn solve(input: &Input) -> Solution {
    let mut solution = Solution::new();
    build_loops(&mut solution);

    let actions = build_actions(&solution.conveyors);
    let next_cell = build_next_cell(&solution.conveyors);
    let dist_to_exit = build_dist_to_exit(&actions);
    let loop_contains_exit = build_loop_contains_exit(&solution.conveyors);
    let pos_cost = build_pos_cost(&dist_to_exit);

    let mut weight = [0.0f64; KLOOK];
    for (i, w) in weight.iter_mut().enumerate() {
        *w = 0.96f64.powi(i as i32);
    }

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;
    board.deliver_if_possible(&mut delivered);

    while delivered < NN && solution.ops.len() < MAX_T {
        let target = delivered;
        let p = board.box_pos[target];

        if p < 0 {
            delivered += 1;
            continue;
        }

        if p as usize == EXIT_P {
            let op = choose_idle_op(
                &board,
                delivered,
                &solution.conveyors,
                &next_cell,
                &loop_contains_exit,
                &pos_cost,
                &weight,
            );
            board.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);
            board.deliver_if_possible(&mut delivered);
            continue;
        }

        let chosen = choose_path_to_exit(
            &board,
            target,
            &actions,
            &next_cell,
            &dist_to_exit,
            &pos_cost,
            &weight,
        );

        let before = delivered;
        for i in 0..chosen.slen {
            if solution.ops.len() >= MAX_T {
                break;
            }
            let op = chosen.seq[i];
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
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
