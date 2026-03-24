// v123_depth3_table_mcts.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const DIR_TABLE: [[u8; 3]; 3] = [
    [b'B', b'F', b'F'],
    [b'B', b'R', b'L'],
    [b'B', b'R', b'L'],
];
const TIME_LIMIT_SEC: f64 = 1.90;
const SAFETY_SEC: f64 = 0.05;
const MCTS_MAX_DEPTH: usize = 3;
const MCTS_EXPAND_THRESHOLD: u32 = 8;
const MCTS_UCB_C: f64 = 0.65;

struct Timer {
    start: Instant,
    limit_sec: f64,
    safety_sec: f64,
}

impl Timer {
    fn new(limit_sec: f64, safety_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            limit_sec,
            safety_sec,
        }
    }

    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn soft_limit(&self) -> f64 {
        self.limit_sec - self.safety_sec
    }

    fn remaining(&self) -> f64 {
        (self.soft_limit() - self.elapsed()).max(0.0)
    }

    fn is_time_up(&self) -> bool {
        self.elapsed() >= self.soft_limit()
    }

    fn local_deadline(&self, remaining_decisions: usize) -> f64 {
        if remaining_decisions <= 1 {
            return self.soft_limit();
        }
        let base = self.remaining() / remaining_decisions as f64;
        let factor = if remaining_decisions >= 60 {
            0.60
        } else if remaining_decisions >= 25 {
            0.85
        } else {
            1.15
        };
        (self.elapsed() + base * factor + 0.0003).min(self.soft_limit())
    }
}

#[derive(Clone, Copy)]
struct Board {
    cells: [u8; NN],
}

impl Board {
    fn new() -> Self {
        Self { cells: [0; NN] }
    }

    fn place_pth_empty(&mut self, p: usize, flavor: u8) {
        let mut kth = 0usize;
        for idx in 0..NN {
            if self.cells[idx] == 0 {
                kth += 1;
                if kth == p {
                    self.cells[idx] = flavor;
                    return;
                }
            }
        }
        unreachable!("invalid p: {p}");
    }

    fn tilted(&self, dir: u8) -> Self {
        let mut next = [0u8; NN];
        match dir {
            b'F' => {
                for col in 0..N {
                    let mut write = 0usize;
                    for row in 0..N {
                        let v = self.cells[row * N + col];
                        if v != 0 {
                            next[write * N + col] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'B' => {
                for col in 0..N {
                    let mut write = N;
                    for row in (0..N).rev() {
                        let v = self.cells[row * N + col];
                        if v != 0 {
                            write -= 1;
                            next[write * N + col] = v;
                        }
                    }
                }
            }
            b'L' => {
                for row in 0..N {
                    let mut write = 0usize;
                    for col in 0..N {
                        let v = self.cells[row * N + col];
                        if v != 0 {
                            next[row * N + write] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'R' => {
                for row in 0..N {
                    let mut write = N;
                    for col in (0..N).rev() {
                        let v = self.cells[row * N + col];
                        if v != 0 {
                            write -= 1;
                            next[row * N + write] = v;
                        }
                    }
                }
            }
            _ => unreachable!("invalid dir: {dir}"),
        }
        Self { cells: next }
    }
}

struct Scanner<R> {
    reader: R,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn next_usize(&mut self) -> usize {
        let mut value = 0usize;
        let mut started = false;
        loop {
            let buf = self.reader.fill_buf().unwrap();
            if buf.is_empty() {
                assert!(started, "unexpected EOF while reading token");
                return value;
            }
            let mut consumed = 0usize;
            while consumed < buf.len() {
                let b = buf[consumed];
                if b.is_ascii_whitespace() {
                    consumed += 1;
                    if started {
                        self.reader.consume(consumed);
                        return value;
                    }
                } else {
                    started = true;
                    value = value * 10 + usize::from(b - b'0');
                    consumed += 1;
                }
            }
            self.reader.consume(consumed);
        }
    }
}

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        };
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.state = x;
        x
    }
}

#[derive(Clone)]
struct Node {
    actions: Vec<u8>,
    w: f64,
    n: u32,
    child_nodes: Vec<Node>,
}

impl Node {
    fn new(actions: Vec<u8>) -> Self {
        Self {
            actions,
            w: 0.0,
            n: 0,
            child_nodes: Vec::new(),
        }
    }

    fn expand(&mut self) {
        if !self.child_nodes.is_empty() {
            return;
        }
        for &dir in &DIRS {
            let mut next_actions = self.actions.clone();
            next_actions.push(dir);
            self.child_nodes.push(Node::new(next_actions));
        }
    }

    fn next_child_index(&self, rng: &mut XorShift64) -> usize {
        let mut unvisited = [0usize; 4];
        let mut len = 0usize;
        for (idx, child) in self.child_nodes.iter().enumerate() {
            if child.n == 0 {
                unvisited[len] = idx;
                len += 1;
            }
        }
        if len > 0 {
            return unvisited[(rng.next_u64() as usize) % len];
        }

        let total_n = self.child_nodes.iter().map(|c| c.n as f64).sum::<f64>().max(1.0);
        let scale = self
            .child_nodes
            .iter()
            .map(|c| (c.w / c.n as f64).abs())
            .fold(1.0_f64, f64::max);
        let mut best_idx = 0usize;
        let mut best_ucb = f64::NEG_INFINITY;
        for (idx, child) in self.child_nodes.iter().enumerate() {
            let mean = child.w / child.n as f64;
            let bonus = scale * MCTS_UCB_C * ((total_n.ln()) / child.n as f64).sqrt();
            let ucb = mean + bonus;
            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = idx;
            }
        }
        best_idx
    }

    fn redo_playout(
        &self,
        root_board: &Board,
        root_turn: usize,
        fs: &[u8; NN],
        rng: &mut XorShift64,
    ) -> f64 {
        let (board, turn, terminal) = replay_prefix(root_board, root_turn, &self.actions, fs, rng);
        if terminal {
            final_raw_score(&board) as f64
        } else {
            rollout_value_from_decision_state(&board, turn, fs, rng) as f64
        }
    }

    fn evaluate(
        &mut self,
        root_board: &Board,
        root_turn: usize,
        fs: &[u8; NN],
        rng: &mut XorShift64,
    ) -> f64 {
        if self.child_nodes.is_empty() {
            let value = self.redo_playout(root_board, root_turn, fs, rng);
            self.w += value;
            self.n += 1;
            if self.actions.len() < MCTS_MAX_DEPTH && self.n == MCTS_EXPAND_THRESHOLD {
                self.expand();
            }
            return value;
        }

        let idx = self.next_child_index(rng);
        let value = self.child_nodes[idx].evaluate(root_board, root_turn, fs, rng);
        self.w += value;
        self.n += 1;
        value
    }
}

fn final_raw_score(board: &Board) -> i64 {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i64;

    for idx in 0..NN {
        let color = board.cells[idx];
        if color == 0 || seen[idx] {
            continue;
        }
        seen[idx] = true;
        let mut top = 0usize;
        stack[top] = idx;
        top += 1;
        let mut size = 0i64;
        while top > 0 {
            top -= 1;
            let cur = stack[top];
            size += 1;
            let r = cur / N;
            let c = cur % N;
            if r > 0 {
                let next = cur - N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if r + 1 < N {
                let next = cur + N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if c > 0 {
                let next = cur - 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if c + 1 < N {
                let next = cur + 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
        }
        comp_sq += size * size;
    }
    comp_sq
}

fn table_dir(cur_flavor: u8, next_flavor: Option<u8>) -> u8 {
    match next_flavor {
        Some(next) => DIR_TABLE[(cur_flavor - 1) as usize][(next - 1) as usize],
        None => b'F',
    }
}

fn sample_random_place(board: &mut Board, turn: usize, fs: &[u8; NN], rng: &mut XorShift64) {
    let remaining_empty = NN - turn;
    let p = (rng.next_u64() as usize % remaining_empty) + 1;
    board.place_pth_empty(p, fs[turn]);
}

fn replay_prefix(
    root_board: &Board,
    root_turn: usize,
    actions: &[u8],
    fs: &[u8; NN],
    rng: &mut XorShift64,
) -> (Board, usize, bool) {
    let mut board = *root_board;
    let mut turn = root_turn;
    for &action in actions {
        board = board.tilted(action);
        if turn == NN - 1 {
            return (board, turn, true);
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return (board, turn, true);
        }
    }
    (board, turn, false)
}

fn rollout_value_from_decision_state(
    board: &Board,
    current_turn: usize,
    fs: &[u8; NN],
    rng: &mut XorShift64,
) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    while turn < NN - 1 {
        let dir = table_dir(fs[turn], Some(fs[turn + 1]));
        board = board.tilted(dir);
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
    }
    final_raw_score(&board)
}

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    timer: &Timer,
    rng: &mut XorShift64,
) -> u8 {
    let remaining_decisions = NN - 1 - turn;
    let deadline = timer.local_deadline(remaining_decisions.max(1));
    let mut root = Node::new(Vec::new());
    root.expand();

    while !timer.is_time_up() && timer.elapsed() < deadline {
        root.evaluate(board_after_place, turn, fs, rng);
    }

    let mut best_idx = 0usize;
    let mut best_visits = 0u32;
    let mut best_mean = f64::NEG_INFINITY;
    for (idx, child) in root.child_nodes.iter().enumerate() {
        let mean = if child.n == 0 {
            f64::NEG_INFINITY
        } else {
            child.w / child.n as f64
        };
        if child.n > best_visits || (child.n == best_visits && mean > best_mean) {
            best_visits = child.n;
            best_mean = mean;
            best_idx = idx;
        }
    }
    DIRS[best_idx]
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut fs = [0u8; NN];
    for flavor in &mut fs {
        *flavor = scanner.next_usize() as u8;
    }

    let mut seed = 0x1234_5678_9abc_def0u64;
    for &f in &fs {
        seed = seed
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::from(f) + 1);
    }
    let mut rng = XorShift64::new(seed);
    let timer = Timer::new(TIME_LIMIT_SEC, SAFETY_SEC);
    let mut board = Board::new();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for turn in 0..NN {
        let p = scanner.next_usize();
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = choose_dir(&board, turn, &fs, &timer, &mut rng);
        board = board.tilted(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
