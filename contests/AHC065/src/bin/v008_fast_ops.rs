// v008_fast_ops.rs
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

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
const MAX_PATH: usize = 64;
const INF_DIST: i16 = 30_000;
const NO_POS: u16 = u16::MAX;

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
        self.a[to_p(i, j)]
    }

    #[inline(always)]
    fn pos_of(&self, k: usize) -> (usize, usize) {
        to_ij(self.pos_of_box[k])
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
    fn at_p(&self, conveyors: &[Conveyor], p: usize) -> usize {
        debug_assert!(p < NN);
        let refs = self.cell_refs[p];
        if refs.len == 0 {
            self.fixed[p]
        } else {
            self.get_at_ref(conveyors, refs.refs[0])
        }
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

    fn apply_ops(&mut self, conveyors: &[Conveyor], ops: &[Operation]) {
        for &op in ops {
            self.apply_op(conveyors, op);
        }
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
    next_pos: Vec<[usize; NN]>,
    actions_from: [ActionList; NN],
    dist_to_exit: [i16; NN],
    weight: [i32; LOOK],
    dummy_action: usize,
}

impl Layout {
    fn new(conveyors: &[Conveyor]) -> Self {
        let mut action_ops = Vec::with_capacity(conveyors.len() * 2);
        let mut next_pos = Vec::with_capacity(conveyors.len() * 2);

        for (m, conveyor) in conveyors.iter().enumerate() {
            for d in [-1i8, 1i8] {
                action_ops.push(Operation { m, d });

                let mut next = [0usize; NN];
                for (p, v) in next.iter_mut().enumerate() {
                    *v = p;
                }

                let c = conveyor.as_slice();
                let len = c.len();
                for x in 0..len {
                    let to = if d == 1 {
                        c[(x + 1) % len]
                    } else {
                        c[(x + len - 1) % len]
                    };
                    next[c[x]] = to;
                }
                next_pos.push(next);
            }
        }

        let mut actions_from = [ActionList::new(); NN];
        for (action_idx, next) in next_pos.iter().enumerate() {
            for p in 0..NN {
                if next[p] != p {
                    actions_from[p].push(action_idx);
                }
            }
        }

        let dist_to_exit = Self::bfs_dist_to_exit(&actions_from, &next_pos);
        let mut weight = [0i32; LOOK];
        for (i, w) in weight.iter_mut().enumerate() {
            *w = (10_000.0 / ((i + 1) as f64).sqrt()) as i32;
        }

        let mut dummy_action = 0usize;
        for (idx, next) in next_pos.iter().enumerate() {
            if next[EXIT_P] == EXIT_P {
                dummy_action = idx;
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
        next_pos: &[[usize; NN]],
    ) -> [i16; NN] {
        let mut dist = [INF_DIST; NN];
        let mut que = VecDeque::new();
        dist[EXIT_P] = 0;
        que.push_back(EXIT_P);

        while let Some(p) = que.pop_front() {
            let nd = dist[p] + 1;
            for &a in actions_from[p].as_slice() {
                let q = next_pos[a as usize][p];
                if dist[q] == INF_DIST {
                    dist[q] = nd;
                    que.push_back(q);
                }
            }
        }

        dist
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

fn build_row_col_conveyors(solution: &mut Solution) {
    for r in (0..N).step_by(2) {
        let mut c = Vec::with_capacity(2 * N);
        for j in 0..N {
            c.push(to_p(r, j));
        }
        for j in (0..N).rev() {
            c.push(to_p(r + 1, j));
        }
        solution.add_conveyor(&c);
    }

    for col in (0..N).step_by(2) {
        let mut c = Vec::with_capacity(2 * N);
        for i in 0..N {
            c.push(to_p(i, col));
        }
        for i in (0..N).rev() {
            c.push(to_p(i, col + 1));
        }
        solution.add_conveyor(&c);
    }
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
                let q = layout.next_pos[action_idx][p];
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
                    let newp = layout.next_pos[action_idx][oldp];
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

fn solve(input: &Input) -> Output {
    let mut solution = Solution::new();
    build_row_col_conveyors(&mut solution);
    let layout = Layout::new(&solution.conveyors);
    let mut state = State::new(input, &solution.conveyors);

    while state.delivered < NN && solution.ops.len() < MAX_T {
        if state.pos_p(&solution.conveyors, state.delivered) == Some(EXIT_P) {
            let op = layout.action_ops[layout.dummy_action];
            state.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);
            continue;
        }

        let before = state.delivered;
        let (len, seq) = choose_shortest_path_with_future_tie_break(
            &solution.conveyors,
            &layout,
            &state,
            state.delivered,
        );
        if len == 0 {
            break;
        }

        for &a16 in seq.iter().take(len) {
            if solution.ops.len() >= MAX_T {
                break;
            }

            let op = layout.action_ops[a16 as usize];
            state.apply_op(&solution.conveyors, op);
            solution.add_op(op.m, op.d);

            if state.delivered > before {
                break;
            }
        }

        if state.delivered == before {
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
