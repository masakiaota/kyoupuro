// v116codex_future_weighted_surface.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const FUTURE_DISCOUNT: f64 = 0.96;

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

fn choose_weighted_future_move(board: &Board, weights: &[f64; 4]) -> (u8, Board) {
    let mut tilted_boards = [Board::new(); 4];
    let mut scores = [0.0f64; 4];
    for (idx, dir) in DIRS.into_iter().enumerate() {
        let (tilted, counts) = board.tilted_with_counts(dir);
        tilted_boards[idx] = tilted;
        let mut score = 0.0;
        for flavor in 1..=3 {
            let weight = weights[flavor];
            if weight <= 1e-12 {
                continue;
            }
            let gain = tilted_boards[idx].next_surface_gain_with_counts(dir, flavor as u8, &counts);
            score += weight * gain as f64;
        }
        scores[idx] = score;
    }

    let mut best_idx = 0usize;
    for idx in 1..4 {
        if scores[idx] > scores[best_idx] {
            best_idx = idx;
        }
    }
    let best_score = scores[best_idx];
    let mut best_comp_sq = i64::MIN;
    for idx in 0..4 {
        if (scores[idx] - best_score).abs() <= 1e-9 {
            let comp_sq = tilted_boards[idx].component_sq_sum();
            if comp_sq > best_comp_sq {
                best_comp_sq = comp_sq;
                best_idx = idx;
            }
        }
    }
    (DIRS[best_idx], tilted_boards[best_idx])
}

fn build_future_weights(fs: &[u8; NN]) -> [[f64; 4]; NN + 1] {
    let mut weights = [[0.0; 4]; NN + 1];
    for t in (0..NN).rev() {
        for flavor in 1..=3 {
            weights[t][flavor] = weights[t + 1][flavor] * FUTURE_DISCOUNT;
        }
        weights[t][fs[t] as usize] += 1.0;
    }
    weights
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut fs = [0u8; NN];
    for f in &mut fs {
        *f = scanner.next_usize() as u8;
    }
    let future_weights = build_future_weights(&fs);
    let mut board = Board::new();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for turn in 0..NN {
        let p = scanner.next_usize();
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let (dir, next_board) = choose_weighted_future_move(&board, &future_weights[turn + 1]);
        board = next_board;
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
