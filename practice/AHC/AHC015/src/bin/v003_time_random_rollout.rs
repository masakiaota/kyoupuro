// v003_time_random_rollout.rs
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

#[derive(Clone, Copy)]
struct Candidate {
    dir: u8,
    board: Board,
    sum: f64,
    samples: usize,
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

fn dir_index(dir: u8) -> usize {
    match dir {
        b'F' => 0,
        b'B' => 1,
        b'L' => 2,
        b'R' => 3,
        _ => unreachable!(),
    }
}

fn connected_component_raw(board: &Board) -> i64 {
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

fn random_playout_score(
    board: &Board,
    next_turn: usize,
    fs: &[u8; NN],
    rng: &mut XorShift64,
) -> i64 {
    let mut board = *board;
    for turn in next_turn..NN {
        let remaining_empty = NN - turn;
        let p = (rng.next_u64() as usize % remaining_empty) + 1;
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = DIRS[rng.next_u64() as usize & 3];
        board = board.tilted(dir);
    }
    connected_component_raw(&board) * 1000
}

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    timer: &Timer,
    rng: &mut XorShift64,
) -> u8 {
    let next_turn = turn + 1;
    let remaining_decisions = NN - 1 - turn;
    let local_deadline = timer.local_deadline(remaining_decisions.max(1));
    let mut candidates = [Candidate {
        dir: DIRS[0],
        board: Board::new(),
        sum: 0.0,
        samples: 0,
    }; 4];
    let mut fallback_idx = 0usize;
    let mut fallback_score = i64::MIN;

    for &dir in &DIRS {
        let tilted = board_after_place.tilted(dir);
        let idx = dir_index(dir);
        candidates[idx] = Candidate {
            dir,
            board: tilted,
            sum: 0.0,
            samples: 0,
        };
        let score = connected_component_raw(&tilted);
        if score > fallback_score {
            fallback_score = score;
            fallback_idx = idx;
        }
    }

    let mut best_idx = fallback_idx;
    let mut best_avg = f64::NEG_INFINITY;
    while !timer.is_time_up() && timer.elapsed() < local_deadline {
        let sample_seed = rng.next_u64();
        for candidate in &mut candidates {
            if timer.is_time_up() || timer.elapsed() >= local_deadline {
                break;
            }
            let mut sample_rng = XorShift64::new(sample_seed);
            let value = random_playout_score(&candidate.board, next_turn, fs, &mut sample_rng);
            candidate.sum += value as f64;
            candidate.samples += 1;
            let avg = candidate.sum / candidate.samples as f64;
            if avg > best_avg {
                best_avg = avg;
                best_idx = dir_index(candidate.dir);
            }
        }
    }

    candidates[best_idx].dir
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut fs = [0u8; NN];
    let mut flavor_counts = [0usize; 4];
    for f in &mut fs {
        *f = scanner.next_usize() as u8;
        flavor_counts[*f as usize] += 1;
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
