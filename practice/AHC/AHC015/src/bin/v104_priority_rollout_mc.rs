// v104_priority_rollout_mc.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const TIME_LIMIT_SEC: f64 = 1.90;
const SAFETY_SEC: f64 = 0.05;
const MAX_ROLLOUT_HORIZON: usize = 8;
const MCTS_EXPAND_THRESHOLD: u32 = 10;
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
            b'L' => {
                for r in 0..N {
                    let mut write = 0usize;
                    for c in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[r * N + write] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    let mut write = N;
                    for c in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[r * N + write] = v;
                        }
                    }
                }
            }
            b'F' => {
                for c in 0..N {
                    let mut write = 0usize;
                    for r in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[write * N + c] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    let mut write = N;
                    for r in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[write * N + c] = v;
                        }
                    }
                }
            }
            _ => unreachable!("invalid dir"),
        }
        Self { cells: next }
    }
}

struct EvalFeatures {
    comp_sq: i32,
    same_adj: i32,
    diff_adj: i32,
}

#[derive(Clone, Copy)]
struct Candidate {
    dir: u8,
    board: Board,
    value: f64,
}

struct Scanner<R> {
    reader: R,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn next_usize(&mut self) -> usize {
        let mut val = 0usize;
        let mut started = false;
        loop {
            let buf = self.reader.fill_buf().unwrap();
            if buf.is_empty() {
                assert!(started, "unexpected EOF while reading token");
                return val;
            }
            let mut consumed = 0usize;
            while consumed < buf.len() {
                let b = buf[consumed];
                if b.is_ascii_whitespace() {
                    consumed += 1;
                    if started {
                        self.reader.consume(consumed);
                        return val;
                    }
                } else {
                    started = true;
                    val = val * 10 + usize::from(b - b'0');
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
        horizon: usize,
    ) -> f64 {
        let (board, turn, terminal) = replay_prefix(root_board, root_turn, &self.actions, fs, rng);
        if terminal {
            final_raw_score(&board) as f64 * 1000.0
        } else {
            rollout_value_from_decision_state(&board, turn, fs, rng, horizon) as f64
        }
    }

    fn evaluate(
        &mut self,
        root_board: &Board,
        root_turn: usize,
        fs: &[u8; NN],
        rng: &mut XorShift64,
        max_depth: usize,
        horizon: usize,
    ) -> f64 {
        if self.child_nodes.is_empty() {
            let value = self.redo_playout(root_board, root_turn, fs, rng, horizon);
            self.w += value;
            self.n += 1;
            if self.actions.len() < max_depth && self.n == MCTS_EXPAND_THRESHOLD {
                self.expand();
            }
            return value;
        }

        let idx = self.next_child_index(rng);
        let value = self.child_nodes[idx].evaluate(root_board, root_turn, fs, rng, max_depth, horizon);
        self.w += value;
        self.n += 1;
        value
    }
}

fn evaluate_features(board: &Board) -> EvalFeatures {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i32;
    let mut same_adj = 0i32;
    let mut diff_adj = 0i32;

    for idx in 0..NN {
        let color = board.cells[idx];
        if color == 0 {
            continue;
        }
        let r = idx / N;
        let c = idx % N;
        if c + 1 < N {
            let right = board.cells[idx + 1];
            if right != 0 {
                if right == color {
                    same_adj += 1;
                } else {
                    diff_adj += 1;
                }
            }
        }
        if r + 1 < N {
            let down = board.cells[idx + N];
            if down != 0 {
                if down == color {
                    same_adj += 1;
                } else {
                    diff_adj += 1;
                }
            }
        }
        if seen[idx] {
            continue;
        }
        seen[idx] = true;
        let mut top = 0usize;
        stack[top] = idx;
        top += 1;
        let mut size = 0i32;
        while top > 0 {
            top -= 1;
            let cur = stack[top];
            size += 1;
            let cr = cur / N;
            let cc = cur % N;
            if cr > 0 {
                let next = cur - N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cr + 1 < N {
                let next = cur + N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cc > 0 {
                let next = cur - 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cc + 1 < N {
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

    EvalFeatures {
        comp_sq,
        same_adj,
        diff_adj,
    }
}

fn static_value(board: &Board, _placed: usize) -> i64 {
    let f = evaluate_features(board);
    i64::from(f.comp_sq) * 14 + i64::from(f.same_adj) * 18 - i64::from(f.diff_adj) * 10
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

fn exact_expected_value(board: &Board, next_turn: usize, fs: &[u8; NN]) -> f64 {
    let remaining_empty = NN - next_turn;
    if remaining_empty == 0 {
        return final_raw_score(board) as f64 * 1000.0;
    }

    let mut sum = 0.0;
    for p in 1..=remaining_empty {
        let mut placed_board = *board;
        placed_board.place_pth_empty(p, fs[next_turn]);
        if next_turn == NN - 1 {
            sum += final_raw_score(&placed_board) as f64 * 1000.0;
            continue;
        }

        let mut best = f64::NEG_INFINITY;
        for &dir in &DIRS {
            let tilted = placed_board.tilted(dir);
            let value = exact_expected_value(&tilted, next_turn + 1, fs);
            if value > best {
                best = value;
            }
        }
        sum += best;
    }
    sum / remaining_empty as f64
}

fn rollout_confidence(best_value: f64, second_value: f64) -> f64 {
    let gap = (best_value - second_value).max(0.0);
    let scale = best_value.abs().max(second_value.abs()).max(1.0);
    gap / scale
}

fn should_skip_rollout(placed: usize, remaining_empty: usize, confidence: f64) -> bool {
    let density = placed as f64 / NN as f64;
    density <= 0.06
        || (density <= 0.12 && confidence >= 0.10)
        || (remaining_empty >= 80 && confidence >= 0.18)
}

fn shaped_deadline(
    timer: &Timer,
    base_deadline: f64,
    placed: usize,
    remaining_empty: usize,
    confidence: f64,
) -> f64 {
    let density = placed as f64 / NN as f64;
    let phase_factor: f64 = if density < 0.15 {
        0.60
    } else if density < 0.55 {
        1.00
    } else {
        1.15
    };
    let ambiguity_factor: f64 = if confidence < 0.025 {
        1.45
    } else if confidence < 0.050 {
        1.20
    } else if confidence < 0.100 {
        0.95
    } else {
        0.65
    };
    let exactness_factor: f64 = if remaining_empty <= 8 { 1.10 } else { 1.00 };
    let factor = (phase_factor * ambiguity_factor * exactness_factor).clamp(0.35_f64, 1.70_f64);
    let base_budget = (base_deadline - timer.elapsed()).max(0.0);
    (timer.elapsed() + base_budget * factor).min(timer.soft_limit())
}

fn choose_rollout_horizon(timer: &Timer, remaining_decisions: usize) -> usize {
    let remaining = timer.remaining();
    if remaining < 0.010 {
        0
    } else if remaining < 0.030 {
        2.min(remaining_decisions)
    } else if remaining < 0.080 {
        3.min(remaining_decisions)
    } else if remaining_decisions >= 50 {
        4
    } else if remaining_decisions >= 20 {
        6.min(remaining_decisions)
    } else {
        MAX_ROLLOUT_HORIZON.min(remaining_decisions)
    }
}

fn sample_random_place(board: &mut Board, next_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) {
    let remaining_empty = NN - next_turn;
    let p = (rng.next_u64() as usize % remaining_empty) + 1;
    board.place_pth_empty(p, fs[next_turn]);
}

fn random_dir(rng: &mut XorShift64) -> u8 {
    DIRS[(rng.next_u64() as usize) & 3]
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
    horizon: usize,
) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    let steps = horizon.min(NN - 1 - turn);

    for _ in 0..steps {
        board = board.tilted(random_dir(rng));
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
    }

    static_value(&board, turn + 1)
}

fn best_root_dir(root: &Node, fallback_dir: u8) -> u8 {
    if root.child_nodes.is_empty() {
        return fallback_dir;
    }
    let mut best_idx = 0usize;
    let mut best_n = 0u32;
    let mut best_w = f64::NEG_INFINITY;
    for (idx, child) in root.child_nodes.iter().enumerate() {
        if child.n > best_n || (child.n == best_n && child.w > best_w) {
            best_n = child.n;
            best_w = child.w;
            best_idx = idx;
        }
    }
    DIRS[best_idx]
}

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    timer: &Timer,
    rng: &mut XorShift64,
) -> u8 {
    let placed = turn + 1;
    let remaining_decisions = NN - 1 - turn;
    let remaining_empty = NN - placed;
    let base_deadline = timer.local_deadline(remaining_decisions.max(1));

    let mut candidates = [Candidate {
        dir: DIRS[0],
        board: Board::new(),
        value: 0.0,
    }; 4];
    for &dir in &DIRS {
        let tilted = board_after_place.tilted(dir);
        candidates[dir_index(dir)] = Candidate {
            dir,
            board: tilted,
            value: static_value(&tilted, placed) as f64,
        };
    }

    let mut best_idx = 0usize;
    for i in 1..4 {
        if candidates[i].value > candidates[best_idx].value {
            best_idx = i;
        }
    }
    let best_value = candidates[best_idx].value;
    let mut second_value = f64::NEG_INFINITY;
    for (idx, candidate) in candidates.iter().enumerate() {
        if idx == best_idx {
            continue;
        }
        if candidate.value > second_value {
            second_value = candidate.value;
        }
    }
    let confidence = rollout_confidence(best_value, second_value);

    if remaining_empty <= 4 {
        let mut exact_best_idx = 0usize;
        let mut exact_best_value = f64::NEG_INFINITY;
        for (idx, candidate) in candidates.iter().enumerate() {
            let value = exact_expected_value(&candidate.board, placed, fs);
            if value > exact_best_value {
                exact_best_value = value;
                exact_best_idx = idx;
            }
        }
        return candidates[exact_best_idx].dir;
    }

    if should_skip_rollout(placed, remaining_empty, confidence) {
        return candidates[best_idx].dir;
    }

    let local_deadline = shaped_deadline(timer, base_deadline, placed, remaining_empty, confidence);
    let horizon = choose_rollout_horizon(timer, NN - placed).max(2);
    let mut root = Node::new(Vec::new());
    root.expand();

    while !timer.is_time_up() && timer.elapsed() < local_deadline {
        let mut sample_rng = XorShift64::new(rng.next_u64());
        let _ = root.evaluate(board_after_place, turn, fs, &mut sample_rng, remaining_decisions, horizon);
    }

    best_root_dir(&root, candidates[best_idx].dir)
}

fn dir_index(dir: u8) -> usize {
    match dir {
        b'F' => 0,
        b'B' => 1,
        b'L' => 2,
        b'R' => 3,
        _ => unreachable!(),
    }
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut fs = [0u8; NN];
    for f in &mut fs {
        *f = scanner.next_usize() as u8;
    }
    let mut flavor_counts = [0usize; 4];
    for &f in &fs {
        flavor_counts[f as usize] += 1;
    }
    let seed = flavor_counts[1] as u64 * 1_000_003
        + flavor_counts[2] as u64 * 1_000_033
        + flavor_counts[3] as u64 * 1_000_037;
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
