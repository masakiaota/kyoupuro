// v101_simple_mc.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const TIME_LIMIT_SEC: f64 = 1.90;
const SAFETY_SEC: f64 = 0.05;

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

fn rollout_value(board: &Board, next_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) -> i64 {
    let mut board = *board;
    let mut turn = next_turn;
    while turn < NN {
        let remaining_empty = NN - turn;
        let p = (rng.next_u64() as usize % remaining_empty) + 1;
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = DIRS[rng.next_u64() as usize & 3];
        board = board.tilted(dir);
        turn += 1;
    }
    final_raw_score(&board)
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
    let remaining_decisions = (NN - 1 - turn).max(1);
    let local_budget = timer.remaining() / remaining_decisions as f64;
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
    for f in &mut fs {
        *f = scanner.next_usize() as u8;
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
