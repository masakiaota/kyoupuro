// v115_fast_next_surface_mc.rs
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

    fn tilted_with_counts(&self, dir: u8) -> (Self, [u8; N]) {
        let mut next = [0u8; NN];
        let mut counts = [0u8; N];
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
                    counts[r] = write as u8;
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
                    counts[r] = (N - write) as u8;
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
                    counts[c] = write as u8;
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
                    counts[c] = (N - write) as u8;
                }
            }
            _ => unreachable!("invalid dir"),
        }
        (Self { cells: next }, counts)
    }

    fn component_sq_sum(&self) -> i64 {
        let mut seen = [false; NN];
        let mut stack = [0usize; NN];
        let mut comp_sq = 0i64;
        for idx in 0..NN {
            let color = self.cells[idx];
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
                    let ni = cur - N;
                    if !seen[ni] && self.cells[ni] == color {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if r + 1 < N {
                    let ni = cur + N;
                    if !seen[ni] && self.cells[ni] == color {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c > 0 {
                    let ni = cur - 1;
                    if !seen[ni] && self.cells[ni] == color {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c + 1 < N {
                    let ni = cur + 1;
                    if !seen[ni] && self.cells[ni] == color {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
            }
            comp_sq += size * size;
        }
        comp_sq
    }

    fn next_surface_gain_with_counts(&self, dir: u8, next_flavor: u8, counts: &[u8; N]) -> i64 {
        let mut comp_ids = [-1i16; NN];
        let mut comp_sizes = [0usize; NN];
        let mut stack = [0usize; NN];
        let mut members = [0usize; NN];
        let mut comp_count = 0i16;

        let component_size = |start: usize,
                              board: &Board,
                              comp_ids: &mut [i16; NN],
                              comp_sizes: &mut [usize; NN],
                              stack: &mut [usize; NN],
                              members: &mut [usize; NN],
                              comp_count: &mut i16| {
            let id = comp_ids[start];
            if id >= 0 {
                return comp_sizes[id as usize];
            }
            let new_id = *comp_count as usize;
            *comp_count += 1;
            let mut top = 0usize;
            let mut len = 0usize;
            stack[top] = start;
            top += 1;
            comp_ids[start] = new_id as i16;
            while top > 0 {
                top -= 1;
                let cur = stack[top];
                members[len] = cur;
                len += 1;
                let r = cur / N;
                let c = cur % N;
                if r > 0 {
                    let ni = cur - N;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if r + 1 < N {
                    let ni = cur + N;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c > 0 {
                    let ni = cur - 1;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c + 1 < N {
                    let ni = cur + 1;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
            }
            comp_sizes[new_id] = len;
            for &idx in &members[..len] {
                comp_ids[idx] = new_id as i16;
            }
            len
        };

        let mut gain = 0i64;
        match dir {
            b'L' => {
                for r in 0..N {
                    let occupied = counts[r] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + occupied - 1;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    let occupied = counts[r] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + (N - occupied);
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'F' => {
                for c in 0..N {
                    let occupied = counts[c] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (occupied - 1) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    let occupied = counts[c] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (N - occupied) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            _ => unreachable!("invalid direction: {dir}"),
        }
        gain
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
    board.component_sq_sum()
}

fn choose_greedy_direction(board: &Board, next_flavor: Option<u8>) -> u8 {
    choose_greedy_move(board, next_flavor).0
}

fn choose_greedy_move(board: &Board, next_flavor: Option<u8>) -> (u8, Board) {
    let mut tilted_boards = [Board::new(); 4];
    match next_flavor {
        Some(flavor) => {
            let mut best_surface = i64::MIN;
            let mut best_idx = 0usize;
            let mut tied = [false; 4];
            let mut tied_count = 0usize;
            for (idx, dir) in DIRS.into_iter().enumerate() {
                let (tilted, counts) = board.tilted_with_counts(dir);
                let surface = tilted.next_surface_gain_with_counts(dir, flavor, &counts);
                tilted_boards[idx] = tilted;
                if surface > best_surface {
                    best_surface = surface;
                    best_idx = idx;
                    tied = [false; 4];
                    tied[idx] = true;
                    tied_count = 1;
                } else if surface == best_surface {
                    tied[idx] = true;
                    tied_count += 1;
                }
            }
            if tied_count == 1 {
                return (DIRS[best_idx], tilted_boards[best_idx]);
            }

            let mut best_comp_sq = i64::MIN;
            for idx in 0..4 {
                if !tied[idx] {
                    continue;
                }
                let comp_sq = tilted_boards[idx].component_sq_sum();
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (DIRS[best_idx], tilted_boards[best_idx])
        }
        None => {
            let mut best_idx = 0usize;
            let mut best_comp_sq = i64::MIN;
            for (idx, dir) in DIRS.into_iter().enumerate() {
                let tilted = board.tilted(dir);
                let comp_sq = tilted.component_sq_sum();
                tilted_boards[idx] = tilted;
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (DIRS[best_idx], tilted_boards[best_idx])
        }
    }
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
        let next_flavor = if turn + 1 < NN {
            Some(fs[turn + 1])
        } else {
            None
        };
        let (_, next_board) = choose_greedy_move(&board, next_flavor);
        board = next_board;
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
