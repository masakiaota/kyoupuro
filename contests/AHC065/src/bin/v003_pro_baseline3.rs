// v003_pro_baseline3.rs
#![allow(dead_code)]
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

const N: usize = 20;
const NN: usize = N * N;
const EXIT_P: usize = N / 2;
const EMPTY_BOX: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;

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

#[derive(Debug, Clone, Copy)]
struct Action {
    to: usize,
    m: usize,
    d: i8,
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
    fn deliver_if_possible(&mut self, delivered: &mut usize) {
        if *delivered < NN && self.cell_box[EXIT_P] == *delivered as i16 {
            self.cell_box[EXIT_P] = EMPTY_BOX;
            self.box_pos[*delivered] = EMPTY_BOX;
            *delivered += 1;
        }
    }
}

#[derive(Debug, Clone)]
struct BeamState<const KLOOK: usize, const MAXD: usize> {
    target: usize,
    flen: usize,
    fp: [u16; KLOOK],
    score: f64,
    slen: usize,
    seq: [Operation; MAXD],
}

impl<const KLOOK: usize, const MAXD: usize> BeamState<KLOOK, MAXD> {
    fn new(target: usize, flen: usize) -> Self {
        Self {
            target,
            flen,
            fp: [0; KLOOK],
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
fn dist_to_exit(p: usize) -> usize {
    p / N + (p % N).abs_diff(N / 2)
}

fn build_loops(solution: &mut Solution) {
    for r in (0..N).step_by(2) {
        let mut c = Vec::with_capacity(2 * N);
        for j in 0..N {
            c.push(id(r, j));
        }
        for j in (0..N).rev() {
            c.push(id(r + 1, j));
        }
        solution.add_conveyor(&c);
    }

    for col in (0..N).step_by(2) {
        let mut c = Vec::with_capacity(2 * N);
        for i in 0..N {
            c.push(id(i, col));
        }
        for i in (0..N).rev() {
            c.push(id(i, col + 1));
        }
        solution.add_conveyor(&c);
    }
}

fn build_next_cell(conveyors: &[Conveyor]) -> Vec<[[usize; NN]; 2]> {
    let mut next_cell = vec![[[0usize; NN]; 2]; conveyors.len()];

    for (m, conveyor) in conveyors.iter().enumerate() {
        for p in 0..NN {
            next_cell[m][0][p] = p;
            next_cell[m][1][p] = p;
        }

        let c = conveyor.as_slice();
        let len = c.len();
        for x in 0..len {
            let u = c[x];
            next_cell[m][1][u] = c[(x + 1) % len];
            next_cell[m][0][u] = c[(x + len - 1) % len];
        }
    }

    next_cell
}

fn build_candidates(conveyors: &[Conveyor]) -> Vec<Vec<Action>> {
    let mut cand = vec![Vec::new(); NN];

    for (m, conveyor) in conveyors.iter().enumerate() {
        let c = conveyor.as_slice();
        let len = c.len();
        for x in 0..len {
            let u = c[x];
            for d in [-1i8, 1i8] {
                let v = if d == 1 {
                    c[(x + 1) % len]
                } else {
                    c[(x + len - 1) % len]
                };

                if dist_to_exit(v) + 1 == dist_to_exit(u) {
                    cand[u].push(Action { to: v, m, d });
                }
            }
        }
    }

    cand
}

fn solve(input: &Input) -> Output {
    const KLOOK: usize = 100;
    const BEAM: usize = 128;
    const MAXD: usize = 64;

    let mut solution = Solution::new();
    build_loops(&mut solution);

    let next_cell = build_next_cell(&solution.conveyors);
    let cand = build_candidates(&solution.conveyors);

    let mut weight = [0.0f64; KLOOK];
    for (i, w) in weight.iter_mut().enumerate() {
        *w = 1.0 / ((i + 1) as f64).sqrt();
    }

    let calc_score = |st: &BeamState<KLOOK, MAXD>| -> f64 {
        let mut s = 0.0;
        for i in 0..st.flen {
            s += weight[i] * dist_to_exit(st.fp[i] as usize) as f64;
        }
        s
    };

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;

    // 初期状態で箱0が出口にある場合だけ、操作前に搬出される。
    if board.cell_box[EXIT_P] == 0 {
        board.cell_box[EXIT_P] = EMPTY_BOX;
        board.box_pos[0] = EMPTY_BOX;
        delivered = 1;
    }

    while delivered < NN {
        let k = delivered;
        let start = board.box_pos[k];

        if start < 0 {
            delivered += 1;
            continue;
        }

        let start = start as usize;
        let need = dist_to_exit(start);

        if need == 0 {
            break;
        }

        let flen = KLOOK.min(NN - (k + 1));
        let mut init = BeamState::<KLOOK, MAXD>::new(start, flen);
        for i in 0..flen {
            init.fp[i] = board.box_pos[k + 1 + i] as u16;
        }
        init.score = calc_score(&init);

        let mut beam = Vec::with_capacity(BEAM);
        beam.push(init);

        // 現在の箱は必ず出口距離を1ずつ減らす。
        // その範囲で、未来の箱の位置がよくなる経路を選ぶ。
        for _depth in 0..need {
            let mut next_beam = Vec::with_capacity(BEAM * 4);

            for st in &beam {
                for &ac in &cand[st.target] {
                    let mut ns = st.clone();
                    ns.target = ac.to;
                    ns.seq[st.slen] = Operation { m: ac.m, d: ac.d };
                    ns.slen = st.slen + 1;

                    let di = if ac.d == 1 { 1 } else { 0 };
                    let mut sc = st.score;

                    for i in 0..st.flen {
                        let oldp = st.fp[i] as usize;
                        let newp = next_cell[ac.m][di][oldp];
                        ns.fp[i] = newp as u16;
                        sc += weight[i] * (dist_to_exit(newp) as f64 - dist_to_exit(oldp) as f64);
                    }

                    ns.score = sc;
                    next_beam.push(ns);
                }
            }

            next_beam.sort_by(|a, b| {
                a.score
                    .total_cmp(&b.score)
                    .then_with(|| a.target.cmp(&b.target))
            });

            if next_beam.len() > BEAM {
                next_beam.truncate(BEAM);
            }
            beam = next_beam;
        }

        let mut best = 0usize;
        for i in 1..beam.len() {
            if beam[i].score < beam[best].score {
                best = i;
            }
        }

        let before_target = k;
        for i in 0..beam[best].slen {
            if solution.ops.len() >= MAX_T {
                break;
            }

            let op = beam[best].seq[i];
            board.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);
            board.deliver_if_possible(&mut delivered);

            // 現在の箱が搬出されたら次の箱へ。
            if delivered > before_target {
                break;
            }
        }

        if solution.ops.len() >= MAX_T {
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
