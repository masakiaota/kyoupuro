// v001_pro_baseline_beam1.rs
use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

const N: usize = 20;
const NN: usize = N * N;
const EXIT_COL: usize = N / 2;
const EXIT_P: usize = EXIT_COL;
const EMPTY: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;
const MAX_PATH: usize = 40;

const BEAM_WIDTH: usize = 80;
const EXTRA: usize = 4;
const STEP_COST: i64 = 50;

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
struct BoardState {
    cell: [i16; NN],
    pos: [i16; NN],
}

impl BoardState {
    fn new(input: &Input) -> Self {
        let mut cell = [EMPTY; NN];
        let mut pos = [EMPTY; NN];

        for p in 0..NN {
            let k = input.a[p];
            cell[p] = k as i16;
        }
        for k in 0..NN {
            pos[k] = input.pos_of_box[k] as i16;
        }

        Self { cell, pos }
    }

    fn apply_op(&mut self, conveyors: &[Conveyor], op: Operation) {
        debug_assert!(op.m < conveyors.len());
        debug_assert!(op.d == -1 || op.d == 1);

        let c = conveyors[op.m].as_slice();
        let len = c.len();

        if op.d == 1 {
            let last = self.cell[c[len - 1]];

            for x in (1..len).rev() {
                let val = self.cell[c[x - 1]];
                self.cell[c[x]] = val;
                if val >= 0 {
                    self.pos[val as usize] = c[x] as i16;
                }
            }

            self.cell[c[0]] = last;
            if last >= 0 {
                self.pos[last as usize] = c[0] as i16;
            }
        } else {
            let first = self.cell[c[0]];

            for x in 0..(len - 1) {
                let val = self.cell[c[x + 1]];
                self.cell[c[x]] = val;
                if val >= 0 {
                    self.pos[val as usize] = c[x] as i16;
                }
            }

            self.cell[c[len - 1]] = first;
            if first >= 0 {
                self.pos[first as usize] = c[len - 1] as i16;
            }
        }
    }

    #[inline(always)]
    fn remove_box_at_exit(&mut self, b: usize) {
        self.cell[EXIT_P] = EMPTY;
        self.pos[b] = EMPTY;
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    st: BoardState,
    p: usize,
    steps: usize,
    path_len: usize,
    path: [Operation; MAX_PATH],
    score: i64,
}

#[inline(always)]
fn id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn dist_to_exit(p: usize) -> usize {
    p / N + (p % N).abs_diff(EXIT_COL)
}

fn build_conveyors(solution: &mut Solution) -> Vec<Vec<(usize, Operation)>> {
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

    for c0 in (0..N).step_by(2) {
        let mut cells = Vec::with_capacity(2 * N);
        for i in 0..N {
            cells.push(id(i, c0));
        }
        for i in (0..N).rev() {
            cells.push(id(i, c0 + 1));
        }
        solution.add_conveyor(&cells);
    }

    let none = Operation {
        m: usize::MAX,
        d: 0,
    };
    let mut edge_op = vec![none; NN * NN];

    for (m, conveyor) in solution.conveyors.iter().enumerate() {
        let cells = conveyor.as_slice();
        let len = cells.len();
        for x in 0..len {
            let u = cells[x];
            let v = cells[(x + 1) % len];

            edge_op[u * NN + v] = Operation { m, d: 1 };
            edge_op[v * NN + u] = Operation { m, d: -1 };
        }
    }

    let mut moves4 = vec![Vec::with_capacity(4); NN];
    let di = [-1isize, 0, 1, 0];
    let dj = [0isize, 1, 0, -1];

    for i in 0..N {
        for j in 0..N {
            let u = id(i, j);
            for z in 0..4 {
                let ni = i as isize + di[z];
                let nj = j as isize + dj[z];
                if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                    continue;
                }

                let v = id(ni as usize, nj as usize);
                let op = edge_op[u * NN + v];
                debug_assert_ne!(op.m, usize::MAX);
                moves4[u].push((v, op));
            }
        }
    }

    moves4
}

fn heuristic(st: &BoardState, b: usize) -> i64 {
    const K: usize = 30;

    let mut sc = 0i64;

    for r in 1..=K {
        if b + r >= NN {
            break;
        }
        let p = st.pos[b + r];
        if p < 0 {
            continue;
        }

        let w = K - r + 1;
        sc += w as i64 * dist_to_exit(p as usize) as i64;
    }

    for r in (K + 1)..=80 {
        if b + r >= NN {
            break;
        }
        let p = st.pos[b + r];
        if p < 0 {
            continue;
        }

        sc += (dist_to_exit(p as usize) / 4) as i64;
    }

    sc
}

fn shortest_ops(start: usize, moves4: &[Vec<(usize, Operation)>]) -> Vec<Operation> {
    let mut prev = [usize::MAX; NN];
    let mut prev_op = [Operation::default(); NN];
    let mut que = VecDeque::new();

    prev[start] = start;
    que.push_back(start);

    while let Some(p) = que.pop_front() {
        if p == EXIT_P {
            break;
        }

        for &(to, op) in &moves4[p] {
            if prev[to] != usize::MAX {
                continue;
            }
            prev[to] = p;
            prev_op[to] = op;
            que.push_back(to);
        }
    }

    debug_assert_ne!(prev[EXIT_P], usize::MAX);

    let mut ops = Vec::new();
    let mut p = EXIT_P;
    while p != start {
        ops.push(prev_op[p]);
        p = prev[p];
    }
    ops.reverse();
    ops
}

fn main() {
    let input = Input::read();
    let mut solution = Solution::new();
    let moves4 = build_conveyors(&mut solution);
    let mut cur = BoardState::new(&input);

    let mut b = 0usize;
    if cur.cell[EXIT_P] == 0 {
        cur.remove_box_at_exit(0);
        b = 1;
    }

    while b < NN {
        let start = cur.pos[b] as usize;

        if start == EXIT_P {
            let op = Operation { m: 1, d: 1 };
            cur.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);

            if cur.cell[EXIT_P] == b as i16 {
                cur.remove_box_at_exit(b);
                b += 1;
            }
            continue;
        }

        let base_dist = dist_to_exit(start);
        let max_depth = base_dist + EXTRA;

        let mut beam = Vec::with_capacity(BEAM_WIDTH * 4);
        beam.push(Node {
            st: cur,
            p: start,
            steps: 0,
            path_len: 0,
            path: [Operation::default(); MAX_PATH],
            score: heuristic(&cur, b),
        });

        let mut done = Vec::with_capacity(BEAM_WIDTH * 4);

        for _depth in 0..max_depth {
            if beam.is_empty() {
                break;
            }

            let mut next = Vec::with_capacity(BEAM_WIDTH * 4);

            for nd in &beam {
                for &(to, op) in &moves4[nd.p] {
                    let new_steps = nd.steps + 1;
                    if new_steps + dist_to_exit(to) > max_depth {
                        continue;
                    }

                    debug_assert!(nd.path_len < MAX_PATH);

                    let mut nx = Node {
                        st: nd.st,
                        p: to,
                        steps: new_steps,
                        path_len: nd.path_len + 1,
                        path: nd.path,
                        score: 0,
                    };
                    nx.path[nd.path_len] = op;
                    nx.st.apply_op(&solution.conveyors, op);

                    if to == EXIT_P {
                        if nx.st.cell[EXIT_P] == b as i16 {
                            nx.st.remove_box_at_exit(b);
                            nx.score = STEP_COST * nx.steps as i64 + heuristic(&nx.st, b);
                            done.push(nx);
                        }
                    } else {
                        nx.score =
                            STEP_COST * (nx.steps + dist_to_exit(to)) as i64 + heuristic(&nx.st, b);
                        next.push(nx);
                    }
                }
            }

            if next.len() > BEAM_WIDTH {
                next.select_nth_unstable_by(BEAM_WIDTH, |a, b| a.score.cmp(&b.score));
                next.truncate(BEAM_WIDTH);
            }

            beam = next;
        }

        if done.is_empty() {
            let ops = shortest_ops(start, &moves4);
            for op in ops {
                cur.apply_op(&solution.conveyors, op);
                solution.add_op(op.m, op.d);
            }
            debug_assert_eq!(cur.cell[EXIT_P], b as i16);
            cur.remove_box_at_exit(b);
            b += 1;
            continue;
        }

        let mut best = 0usize;
        for i in 1..done.len() {
            if done[i].score < done[best].score {
                best = i;
            }
        }

        let chosen = done[best];
        for i in 0..chosen.path_len {
            let op = chosen.path[i];
            solution.add_op(op.m, op.d);
        }

        cur = chosen.st;
        b += 1;
    }

    if solution.ops.len() > MAX_T {
        solution.ops.truncate(MAX_T);
    }

    solution.print();
}
