// v004_pro_baseline_sa.rs
#![allow(dead_code)]
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const EXIT_P: usize = N / 2;
const EMPTY_BOX: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;

const LOOK: usize = 80;
const MAX_ITER: usize = 2000;
const TIME_LIMIT_SEC: f64 = 1.85;

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
}

#[derive(Debug, Clone)]
struct Precomputed {
    dist_to_exit: [i16; NN],
    weight_future: [f64; LOOK + 1],
    temp_table: [f64; MAX_ITER],
}

impl Precomputed {
    fn new() -> Self {
        let mut dist_to_exit = [0i16; NN];
        for (p, dist) in dist_to_exit.iter_mut().enumerate() {
            let i = p / N;
            let j = p % N;
            *dist = (i + j.abs_diff(N / 2)) as i16;
        }

        let mut weight_future = [0.0f64; LOOK + 1];
        for (i, w) in weight_future.iter_mut().enumerate().skip(1) {
            *w = 0.93f64.powi((i - 1) as i32);
        }

        let mut temp_table = [0.0f64; MAX_ITER];
        for (i, temp) in temp_table.iter_mut().enumerate() {
            let t = if MAX_ITER <= 1 {
                1.0
            } else {
                i as f64 / (MAX_ITER - 1) as f64
            };
            *temp = 4.0 * 0.03f64.powf(t) + 0.02;
        }

        Self {
            dist_to_exit,
            weight_future,
            temp_table,
        }
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
    fn randint(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    #[inline(always)]
    fn mask30(&mut self) -> u32 {
        (self.next() & ((1u64 << 30) - 1)) as u32
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

fn build_loops(solution: &mut Solution) {
    for r in 0..(N / 2) {
        let r0 = 2 * r;
        let r1 = 2 * r + 1;
        let mut cells = Vec::with_capacity(2 * N);

        for j in 0..N {
            cells.push(id(r0, j));
        }
        for j in (0..N).rev() {
            cells.push(id(r1, j));
        }

        solution.add_conveyor(&cells);
    }

    for c in 0..(N / 2) {
        let c0 = 2 * c;
        let c1 = 2 * c + 1;
        let mut cells = Vec::with_capacity(2 * N);

        for i in 0..N {
            cells.push(id(i, c0));
        }
        for i in (0..N).rev() {
            cells.push(id(i, c1));
        }

        solution.add_conveyor(&cells);
    }
}

#[inline(always)]
fn operation_for_move(from: usize, to: usize) -> Operation {
    let i = from / N;
    let j = from % N;
    let ni = to / N;
    let nj = to % N;

    if i == ni {
        let m = i / 2;
        let d = if i % 2 == 0 {
            if nj == j + 1 { 1 } else { -1 }
        } else if nj + 1 == j {
            1
        } else {
            -1
        };
        Operation { m, d }
    } else {
        let m = N / 2 + j / 2;
        let d = if j % 2 == 0 {
            if ni == i + 1 { 1 } else { -1 }
        } else if ni + 1 == i {
            1
        } else {
            -1
        };
        Operation { m, d }
    }
}

#[inline(always)]
fn next_cell_by_mask(p: usize, mask: u32, step: usize) -> usize {
    let i = p / N;
    let j = p % N;

    let can_v = i > 0;
    let can_h = j != N / 2;

    let choose_v = if can_v && can_h {
        ((mask >> step) & 1) != 0
    } else {
        can_v
    };

    if choose_v {
        p - N
    } else if j < N / 2 {
        p + 1
    } else {
        p - 1
    }
}

fn evaluate_mask(
    conveyors: &[Conveyor],
    prec: &Precomputed,
    board0: Board,
    target: usize,
    mask: u32,
) -> f64 {
    let mut board = board0;
    let mut step = 0usize;

    while board.box_pos[target] >= 0 {
        let p = board.box_pos[target] as usize;

        if p == EXIT_P {
            break;
        }

        let q = next_cell_by_mask(p, mask, step);
        let op = operation_for_move(p, q);

        board.apply_op(conveyors, op);
        step += 1;

        if board.cell_box[EXIT_P] == target as i16 {
            board.cell_box[EXIT_P] = EMPTY_BOX;
            board.box_pos[target] = EMPTY_BOX;
            break;
        }

        if step > 60 {
            break;
        }
    }

    let mut score = 0.0;
    let last = (NN - 1).min(target + LOOK);
    for k in (target + 1)..=last {
        let p = board.box_pos[k];
        if p >= 0 {
            score += prec.weight_future[k - target] * f64::from(prec.dist_to_exit[p as usize]);
        }
    }

    score
}

fn choose_route_mask(
    conveyors: &[Conveyor],
    prec: &Precomputed,
    board: Board,
    target: usize,
    rng: &mut XorShift,
    timer: &Timer,
) -> u32 {
    const FULL: u32 = (1u32 << 30) - 1;

    let initials = [
        0u32,
        FULL,
        0x1555_5555u32,
        0x2AAA_AAAAu32,
        rng.mask30(),
        rng.mask30(),
    ];

    let mut best = initials[0];
    let mut best_score = f64::INFINITY;

    for &mask in &initials {
        let score = evaluate_mask(conveyors, prec, board, target, mask);
        if score < best_score {
            best_score = score;
            best = mask;
        }
    }

    let mut cur = best;
    let mut cur_score = best_score;

    for it in 0..MAX_ITER {
        if (it & 63) == 0 && timer.elapsed() > TIME_LIMIT_SEC {
            break;
        }

        let mut prop = cur;
        let typ = rng.randint(10);

        if typ < 5 {
            prop ^= 1u32 << rng.randint(30);
        } else if typ < 8 {
            prop ^= 1u32 << rng.randint(30);
            prop ^= 1u32 << rng.randint(30);
        } else {
            prop = rng.mask30();
        }

        let score = evaluate_mask(conveyors, prec, board, target, prop);
        if score < cur_score || rng.uniform01() < ((cur_score - score) / prec.temp_table[it]).exp()
        {
            cur = prop;
            cur_score = score;

            if score < best_score {
                best_score = score;
                best = prop;
            }
        }
    }

    best
}

fn solve(input: &Input) -> Output {
    let timer = Timer::new();
    let prec = Precomputed::new();

    let mut solution = Solution::new();
    build_loops(&mut solution);

    let mut board = Board::from_input(input);
    let mut delivered = 0usize;

    if board.cell_box[EXIT_P] == 0 {
        board.cell_box[EXIT_P] = EMPTY_BOX;
        board.box_pos[0] = EMPTY_BOX;
        delivered = 1;
    }

    let mut seed = 146_527u64;
    for &x in &input.a {
        seed = seed.wrapping_mul(1_000_003).wrapping_add((x + 1) as u64);
    }
    let mut rng = XorShift::new(seed);

    while delivered < NN && solution.ops.len() < MAX_T {
        let mask = choose_route_mask(
            &solution.conveyors,
            &prec,
            board,
            delivered,
            &mut rng,
            &timer,
        );

        let mut step = 0usize;

        while delivered < NN && board.box_pos[delivered] >= 0 && solution.ops.len() < MAX_T {
            let p = board.box_pos[delivered] as usize;

            if p == EXIT_P {
                let m = 0usize;
                let op1 = Operation { m, d: 1 };
                board.apply_op(&solution.conveyors, op1);
                solution.add_op(op1.m, op1.d);

                if solution.ops.len() >= MAX_T {
                    break;
                }

                let op2 = Operation { m, d: -1 };
                board.apply_op(&solution.conveyors, op2);
                solution.add_op(op2.m, op2.d);

                if board.cell_box[EXIT_P] == delivered as i16 {
                    board.cell_box[EXIT_P] = EMPTY_BOX;
                    board.box_pos[delivered] = EMPTY_BOX;
                    delivered += 1;
                }

                break;
            }

            let q = next_cell_by_mask(p, mask, step);
            let op = operation_for_move(p, q);

            board.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);

            if board.cell_box[EXIT_P] == delivered as i16 {
                board.cell_box[EXIT_P] = EMPTY_BOX;
                board.box_pos[delivered] = EMPTY_BOX;
                delivered += 1;
                break;
            }

            step += 1;
        }
    }

    solution
}

fn main() {
    let input = read_input();
    let output = solve(&input);
    output.print();
}
