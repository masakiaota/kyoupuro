// v000_template.rs
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

// Conveyor の使い方:
// let cells = vec![p0, p1, p2, p3]; // p = i * N + j
// let m = solution.add_conveyor(&cells);
// let mut state = State::new(&input, &solution.conveyors);
// state.apply_op(&solution.conveyors, Operation { m, d: 1 });

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
    fn at(&self, conveyors: &[Conveyor], i: usize, j: usize) -> usize {
        self.at_p(conveyors, i * N + j)
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
    fn pos(&self, conveyors: &[Conveyor], k: usize) -> Option<(usize, usize)> {
        self.pos_p(conveyors, k).map(|p| (p / N, p % N))
    }

    #[inline(always)]
    fn next_box(&self) -> Option<usize> {
        if self.delivered < NN {
            Some(self.delivered)
        } else {
            None
        }
    }

    #[inline(always)]
    fn is_all_delivered(&self) -> bool {
        self.delivered == NN
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

fn simulate(input: &Input, solution: &Solution) -> State {
    let mut state = State::new(input, &solution.conveyors);
    state.apply_ops(&solution.conveyors, &solution.ops);
    state
}

fn calc_score(b: usize, t: usize) -> i64 {
    if b == NN {
        if t == 0 {
            return i64::MAX;
        }
        let score = 1_000_000.0 + 1_000_000.0 * ((MAX_T as f64) / (t as f64)).log2();
        score.round() as i64
    } else {
        (1_000_000.0 * (b as f64) / (NN as f64)).round() as i64
    }
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,

    iter: u64,
    check_mask: u64,

    elapsed_sec: f64,
    progress: f64,
    is_over: bool,
}

impl TimeKeeper {
    /// `check_interval_log2 = 8` なら 2^8 = 256 反復ごとに時計更新
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        assert!(time_limit_sec > 0.0);
        assert!(check_interval_log2 < 63);

        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };

        let mut tk = Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            elapsed_sec: 0.0,
            progress: 0.0,
            is_over: false,
        };
        tk.force_update();
        tk
    }

    /// ホットループではこれだけ呼ぶ
    /// true: 継続, false: 打ち切り
    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    /// 明示的に時計を更新したいときに使う
    #[inline(always)]
    fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    /// batched な経過時間
    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    /// batched な進捗率 [0, 1]
    #[inline(always)]
    fn progress(&self) -> f64 {
        self.progress
    }

    /// batched な時間切れ判定
    #[inline(always)]
    fn is_time_over(&self) -> bool {
        self.is_over
    }

    /// ログ用の正確な経過時間
    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// ログ用の正確な残り時間
    #[inline]
    fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn main() {}
