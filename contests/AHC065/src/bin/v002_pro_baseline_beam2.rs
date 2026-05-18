// v002_pro_baseline_beam2.rs
#![allow(dead_code)]

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

const N: usize = 20;
const NN: usize = N * N;
const EXIT_P: usize = N / 2;
const MAX_T: usize = 100_000;
const MAX_LOCAL_PATH: usize = 64;

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
            conveyors: Vec::new(),
            ops: Vec::with_capacity(MAX_T),
        }
    }

    fn add_conveyor(&mut self, cells: &[usize]) -> usize {
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
    cell: [i16; NN],
    pos: [i16; NN],
}

impl Board {
    fn from_input(input: &Input) -> Self {
        let mut cell = [-1i16; NN];
        let mut pos = [-1i16; NN];

        for p in 0..NN {
            let k = input.a[p];
            cell[p] = k as i16;
            pos[k] = p as i16;
        }

        Self { cell, pos }
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    b: Board,
    pm: [u8; MAX_LOCAL_PATH],
    pd: [i8; MAX_LOCAL_PATH],
    len: usize,
    last_m: usize,
    last_d: i8,
    eval: f64,
}

impl Node {
    fn new(board: Board) -> Self {
        Self {
            b: board,
            pm: [0; MAX_LOCAL_PATH],
            pd: [0; MAX_LOCAL_PATH],
            len: 0,
            last_m: usize::MAX,
            last_d: 0,
            eval: 0.0,
        }
    }
}

const NO_OP: Operation = Operation {
    m: usize::MAX,
    d: 0,
};

#[derive(Debug, Clone, Copy)]
struct CellTrans {
    len: usize,
    ops: [Operation; 4],
}

impl CellTrans {
    const fn new() -> Self {
        Self {
            len: 0,
            ops: [NO_OP; 4],
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn push(&mut self, op: Operation) {
        debug_assert!(self.len < self.ops.len());
        self.ops[self.len] = op;
        self.len += 1;
    }

    #[inline(always)]
    fn first(&self) -> Option<Operation> {
        if self.len == 0 {
            None
        } else {
            Some(self.ops[0])
        }
    }

    #[inline(always)]
    fn as_slice(&self) -> &[Operation] {
        &self.ops[..self.len]
    }
}

struct Solver {
    loops: Vec<Conveyor>,
    trans: Vec<Vec<CellTrans>>,
    answer: Vec<Operation>,

    beam_width: usize,
    extra_depth: usize,
    lookahead: usize,
    decay: f64,
    len_penalty: f64,
    target_dist_weight: f64,
}

impl Solver {
    fn new() -> Self {
        Self {
            loops: Vec::new(),
            trans: vec![vec![CellTrans::new(); NN]; NN],
            answer: Vec::with_capacity(20_000),
            beam_width: 160,
            extra_depth: 2,
            lookahead: 80,
            decay: 0.93,
            len_penalty: 0.9,
            target_dist_weight: 1000.0,
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
    fn manhattan_exit(p: usize) -> usize {
        Self::row(p) + Self::col(p).abs_diff(N / 2)
    }

    fn build_loops(&mut self) {
        self.loops.clear();

        // 0..9: 2-row rings.
        for r in (0..N).step_by(2) {
            let mut cyc = Vec::with_capacity(2 * N);
            for j in 0..N {
                cyc.push(Self::id(r, j));
            }
            for j in (0..N).rev() {
                cyc.push(Self::id(r + 1, j));
            }
            self.loops.push(Conveyor::from_slice(&cyc));
        }

        // 10..19: 2-column rings.
        for c in (0..N).step_by(2) {
            let mut cyc = Vec::with_capacity(2 * N);
            for i in 0..N {
                cyc.push(Self::id(i, c));
            }
            for i in (0..N).rev() {
                cyc.push(Self::id(i, c + 1));
            }
            self.loops.push(Conveyor::from_slice(&cyc));
        }

        for p in 0..NN {
            for q in 0..NN {
                self.trans[p][q].clear();
            }
        }

        for (m, conveyor) in self.loops.iter().enumerate() {
            let cyc = conveyor.as_slice();
            let l = cyc.len();
            for x in 0..l {
                let p = cyc[x];
                let q = cyc[(x + 1) % l];
                self.trans[p][q].push(Operation { m, d: 1 });
                self.trans[q][p].push(Operation { m, d: -1 });
            }
        }
    }

    fn rotate_loop(&self, b: &mut Board, m: usize, d: i8) {
        let cyc = self.loops[m].as_slice();
        let l = cyc.len();

        if d == 1 {
            let last = b.cell[cyc[l - 1]];

            for x in (1..l).rev() {
                let v = b.cell[cyc[x - 1]];
                b.cell[cyc[x]] = v;
                if v >= 0 {
                    b.pos[v as usize] = cyc[x] as i16;
                }
            }

            b.cell[cyc[0]] = last;
            if last >= 0 {
                b.pos[last as usize] = cyc[0] as i16;
            }
        } else {
            let first = b.cell[cyc[0]];

            for x in 0..l - 1 {
                let v = b.cell[cyc[x + 1]];
                b.cell[cyc[x]] = v;
                if v >= 0 {
                    b.pos[v as usize] = cyc[x] as i16;
                }
            }

            b.cell[cyc[l - 1]] = first;
            if first >= 0 {
                b.pos[first as usize] = cyc[l - 1] as i16;
            }
        }
    }

    fn future_score(&self, b: &Board, first_box: usize) -> f64 {
        let mut s = 0.0;
        let mut w = 1.0;
        let mut used = 0usize;

        for k in first_box..NN {
            if used >= self.lookahead {
                break;
            }

            let p = b.pos[k];
            if p >= 0 {
                s += w * Self::manhattan_exit(p as usize) as f64;
            }
            w *= self.decay;
            used += 1;
        }

        s
    }

    fn node_eval(&self, nd: &Node, target: usize) -> f64 {
        let p = nd.b.pos[target];
        let dist = if p >= 0 {
            Self::manhattan_exit(p as usize)
        } else {
            0
        };

        self.target_dist_weight * dist as f64
            + self.future_score(&nd.b, target + 1)
            + self.len_penalty * nd.len as f64
    }

    fn goal_eval(&self, nd: &Node, target: usize) -> f64 {
        self.future_score(&nd.b, target + 1) + self.len_penalty * nd.len as f64
    }

    fn append_path(&mut self, nd: &Node) {
        for i in 0..nd.len {
            self.answer.push(Operation {
                m: nd.pm[i] as usize,
                d: nd.pd[i],
            });
        }
    }

    fn fallback_to_exit(&self, start: &Board, target: usize) -> Node {
        let mut cur = Node::new(*start);

        while cur.b.pos[target] as usize != EXIT_P && cur.len + 1 < MAX_LOCAL_PATH {
            let p = cur.b.pos[target];
            if p < 0 {
                break;
            }

            let p = p as usize;
            let i = Self::row(p);
            let j = Self::col(p);

            let q = if i > 0 {
                Some(Self::id(i - 1, j))
            } else if j < N / 2 {
                Some(Self::id(i, j + 1))
            } else if j > N / 2 {
                Some(Self::id(i, j - 1))
            } else {
                None
            };

            let Some(q) = q else {
                break;
            };

            let Some(a) = self.trans[p][q].first() else {
                break;
            };

            self.rotate_loop(&mut cur.b, a.m, a.d);

            cur.pm[cur.len] = a.m as u8;
            cur.pd[cur.len] = a.d;
            cur.len += 1;
            cur.last_m = a.m;
            cur.last_d = a.d;
        }

        if cur.b.pos[target] as usize == EXIT_P {
            cur.b.cell[EXIT_P] = -1;
            cur.b.pos[target] = -1;
        }

        cur
    }

    fn beam_to_exit(&self, start: &Board, target: usize) -> Node {
        let start_pos = start.pos[target];

        if start_pos < 0 {
            return Node::new(*start);
        }

        let base = Self::manhattan_exit(start_pos as usize);
        let max_depth = (MAX_LOCAL_PATH - 1).min(base + self.extra_depth);

        let mut beam = Vec::with_capacity(self.beam_width + 4);
        let mut next_beam = Vec::with_capacity(self.beam_width * 8 + 16);
        let mut goals = Vec::with_capacity(self.beam_width * (self.extra_depth + 2));

        let mut st = Node::new(*start);
        st.eval = self.node_eval(&st, target);
        beam.push(st);

        const DI: [isize; 4] = [-1, 1, 0, 0];
        const DJ: [isize; 4] = [0, 0, -1, 1];

        for _depth in 0..max_depth {
            next_beam.clear();

            for cur in &beam {
                let p = cur.b.pos[target];
                if p < 0 {
                    continue;
                }

                let p = p as usize;
                let ci = Self::row(p) as isize;
                let cj = Self::col(p) as isize;

                for dir in 0..4 {
                    let ni = ci + DI[dir];
                    let nj = cj + DJ[dir];

                    if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                        continue;
                    }

                    let q = Self::id(ni as usize, nj as usize);
                    let remaining = max_depth - (cur.len + 1);

                    // 残り手数で出口に届かない遷移は捨てる。
                    if Self::manhattan_exit(q) > remaining {
                        continue;
                    }

                    for &a in self.trans[p][q].as_slice() {
                        // 直前操作の完全な打ち消しは不要。
                        if cur.last_m == a.m && cur.last_d == -a.d {
                            continue;
                        }

                        let mut ns = *cur;
                        self.rotate_loop(&mut ns.b, a.m, a.d);

                        ns.pm[ns.len] = a.m as u8;
                        ns.pd[ns.len] = a.d;
                        ns.len += 1;
                        ns.last_m = a.m;
                        ns.last_d = a.d;

                        if ns.b.pos[target] as usize == EXIT_P {
                            ns.b.cell[EXIT_P] = -1;
                            ns.b.pos[target] = -1;
                            ns.eval = self.goal_eval(&ns, target);
                            goals.push(ns);
                        } else {
                            ns.eval = self.node_eval(&ns, target);
                            next_beam.push(ns);
                        }
                    }
                }
            }

            if next_beam.is_empty() {
                break;
            }

            next_beam.sort_by(|a, b| a.eval.total_cmp(&b.eval).then_with(|| a.len.cmp(&b.len)));

            if next_beam.len() > self.beam_width {
                next_beam.truncate(self.beam_width);
            }

            std::mem::swap(&mut beam, &mut next_beam);
        }

        if !goals.is_empty() {
            goals.sort_by(|a, b| a.eval.total_cmp(&b.eval).then_with(|| a.len.cmp(&b.len)));
            return goals[0];
        }

        self.fallback_to_exit(start, target)
    }

    fn solve(&mut self, input: &Input) -> Output {
        self.build_loops();

        let mut cur = Board::from_input(input);
        let mut first = 0usize;

        // 初期状態で箱0が出口にある場合だけ、操作前に搬出される。
        if cur.cell[EXIT_P] == 0 {
            cur.cell[EXIT_P] = -1;
            cur.pos[0] = -1;
            first = 1;
        }

        self.answer.clear();

        for target in first..NN {
            let best = self.beam_to_exit(&cur, target);

            if self.answer.len() + best.len > MAX_T {
                break;
            }

            self.append_path(&best);
            cur = best.b;

            // 念のため。通常は通らない。
            if cur.pos[target] != -1 {
                let fb = self.fallback_to_exit(&cur, target);

                if self.answer.len() + fb.len > MAX_T {
                    break;
                }

                self.append_path(&fb);
                cur = fb.b;
            }
        }

        Solution {
            conveyors: self.loops.clone(),
            ops: self.answer.clone(),
        }
    }
}

fn main() {
    let input = read_input();
    let mut solver = Solver::new();
    let output = solver.solve(&input);
    output.print();
}
