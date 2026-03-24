// v124_time_shaped_table_mc.rs
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
const OPENING_LAST_TURN: usize = 4;
const EXACT_FUTURE_COUNT: usize = 6;

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

#[derive(Clone, Copy)]
struct Candidate {
    dir: u8,
    board: Board,
    sum: f64,
    count: u32,
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

fn opening_dir(cur_flavor: u8) -> u8 {
    match cur_flavor {
        1 => b'F',
        2 => b'L',
        3 => b'R',
        _ => unreachable!("invalid flavor: {cur_flavor}"),
    }
}

fn future_count_after_turn(turn: usize) -> usize {
    NN - 1 - turn
}

fn is_exact_end(turn: usize) -> bool {
    future_count_after_turn(turn) <= EXACT_FUTURE_COUNT
}

fn phase_weight(turn: usize) -> f64 {
    if turn == 0 || turn <= OPENING_LAST_TURN || is_exact_end(turn) {
        0.0
    } else {
        1.0
    }
}

fn remaining_mc_weight_sum(turn: usize) -> f64 {
    let mut sum = 0.0;
    for u in turn..(NN - 1) {
        sum += phase_weight(u);
    }
    sum
}

fn rollout_value(board: &Board, next_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) -> i64 {
    let mut board = *board;
    let mut turn = next_turn;
    while turn < NN {
        let remaining_empty = NN - turn;
        let p = (rng.next_u64() as usize % remaining_empty) + 1;
        let cur_flavor = fs[turn];
        board.place_pth_empty(p, cur_flavor);
        if turn == NN - 1 {
            break;
        }
        let dir = table_dir(cur_flavor, Some(fs[turn + 1]));
        board = board.tilted(dir);
        turn += 1;
    }
    final_raw_score(&board)
}

fn exact_expected_value(
    board: &Board,
    next_turn: usize,
    fs: &[u8; NN],
) -> f64 {
    if next_turn >= NN {
        return final_raw_score(board) as f64;
    }

    let remaining_empty = NN - next_turn;
    let mut sum = 0.0;
    for p in 1..=remaining_empty {
        let mut placed_board = *board;
        placed_board.place_pth_empty(p, fs[next_turn]);
        if next_turn == NN - 1 {
            sum += final_raw_score(&placed_board) as f64;
            continue;
        }

        let dir = table_dir(fs[next_turn], Some(fs[next_turn + 1]));
        let tilted = placed_board.tilted(dir);
        sum += exact_expected_value(&tilted, next_turn + 1, fs);
    }
    sum / remaining_empty as f64
}

fn candidate_mean(candidate: &Candidate) -> f64 {
    if candidate.count == 0 {
        f64::NEG_INFINITY
    } else {
        candidate.sum / f64::from(candidate.count)
    }
}

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    timer: &Timer,
    rng: &mut XorShift64,
) -> u8 {
    if turn == 0 {
        return opening_dir(fs[turn]);
    }

    if is_exact_end(turn) {
        let mut best_dir = DIRS[0];
        let mut best_value = f64::NEG_INFINITY;
        for &dir in &DIRS {
            let next_board = board_after_place.tilted(dir);
            let value = exact_expected_value(&next_board, turn + 1, fs);
            if value > best_value {
                best_value = value;
                best_dir = dir;
            }
        }
        return best_dir;
    }

    if turn <= OPENING_LAST_TURN {
        return table_dir(fs[turn], Some(fs[turn + 1]));
    }

    let remaining_weight = remaining_mc_weight_sum(turn).max(1e-9);
    let local_budget = timer.remaining() * phase_weight(turn) / remaining_weight;
    let deadline = (timer.elapsed() + local_budget * 0.98).min(timer.soft_limit());

    let mut candidates = [Candidate {
        dir: DIRS[0],
        board: Board::new(),
        sum: 0.0,
        count: 0,
    }; 4];
    for (i, &dir) in DIRS.iter().enumerate() {
        candidates[i] = Candidate {
            dir,
            board: board_after_place.tilted(dir),
            sum: 0.0,
            count: 0,
        };
    }

    for candidate in &mut candidates {
        let mut sample_rng = XorShift64::new(rng.next_u64());
        let value = rollout_value(&candidate.board, turn + 1, fs, &mut sample_rng);
        candidate.sum += value as f64;
        candidate.count += 1;
    }

    let mut next_idx = 0usize;
    while !timer.is_time_up() && timer.elapsed() < deadline {
        let sample_seed = rng.next_u64();
        let candidate = &mut candidates[next_idx];
        let mut sample_rng = XorShift64::new(sample_seed);
        let value = rollout_value(&candidate.board, turn + 1, fs, &mut sample_rng);
        candidate.sum += value as f64;
        candidate.count += 1;
        next_idx += 1;
        if next_idx == 4 {
            next_idx = 0;
        }
    }

    let mut best_idx = 0usize;
    for i in 1..4 {
        if candidate_mean(&candidates[i]) > candidate_mean(&candidates[best_idx]) {
            best_idx = i;
        }
    }
    candidates[best_idx].dir
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
