// v120_target_surface_greedy.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;
const ALL_DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const NEXT1_DIRS: [u8; 1] = [b'B'];
const NEXT2_DIRS: [u8; 2] = [b'F', b'R'];
const NEXT3_DIRS: [u8; 2] = [b'F', b'L'];

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
        self.tilted_with_counts(dir).0
    }

    fn tilted_with_counts(&self, dir: u8) -> (Self, [u8; N]) {
        let mut next = [0u8; NN];
        let mut counts = [0u8; N];
        match dir {
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
            _ => unreachable!("invalid direction: {dir}"),
        }
        (Self { cells: next }, counts)
    }

    fn component_sq_sum(&self) -> i64 {
        let mut seen = [false; NN];
        let mut stack = [0usize; NN];
        let mut total = 0i64;
        for start in 0..NN {
            let flavor = self.cells[start];
            if flavor == 0 || seen[start] {
                continue;
            }
            let mut top = 0usize;
            let mut size = 0i64;
            seen[start] = true;
            stack[top] = start;
            top += 1;
            while top > 0 {
                top -= 1;
                let idx = stack[top];
                size += 1;
                let r = idx / N;
                let c = idx % N;
                if r > 0 {
                    let ni = idx - N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if r + 1 < N {
                    let ni = idx + N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c > 0 {
                    let ni = idx - 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c + 1 < N {
                    let ni = idx + 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack[top] = ni;
                        top += 1;
                    }
                }
            }
            total += size * size;
        }
        total
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
        let mut value = 0usize;
        let mut started = false;
        loop {
            let buf = self.reader.fill_buf().unwrap();
            if buf.is_empty() {
                assert!(started, "unexpected EOF");
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

fn candidate_dirs(next_flavor: u8) -> &'static [u8] {
    match next_flavor {
        1 => &NEXT1_DIRS,
        2 => &NEXT2_DIRS,
        3 => &NEXT3_DIRS,
        _ => unreachable!("invalid flavor: {next_flavor}"),
    }
}

fn choose_direction(board: &Board, next_flavor: Option<u8>) -> (u8, Board) {
    match next_flavor {
        Some(flavor) => {
            let dirs = candidate_dirs(flavor);
            let mut tilted_boards = [Board::new(); 4];
            let mut used = [false; 4];
            let mut best_idx = 0usize;
            let mut best_surface = i64::MIN;
            let mut tied = [false; 4];
            let mut tied_count = 0usize;

            for (idx, dir) in ALL_DIRS.into_iter().enumerate() {
                if !dirs.contains(&dir) {
                    continue;
                }
                let (tilted, counts) = board.tilted_with_counts(dir);
                let surface = tilted.next_surface_gain_with_counts(dir, flavor, &counts);
                tilted_boards[idx] = tilted;
                used[idx] = true;
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

            if tied_count <= 1 {
                return (ALL_DIRS[best_idx], tilted_boards[best_idx]);
            }

            let mut best_comp_sq = i64::MIN;
            for idx in 0..4 {
                if !used[idx] || !tied[idx] {
                    continue;
                }
                let comp_sq = tilted_boards[idx].component_sq_sum();
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (ALL_DIRS[best_idx], tilted_boards[best_idx])
        }
        None => {
            let mut best_idx = 0usize;
            let mut best_comp_sq = i64::MIN;
            let mut tilted_boards = [Board::new(); 4];
            for (idx, dir) in ALL_DIRS.into_iter().enumerate() {
                let tilted = board.tilted(dir);
                let comp_sq = tilted.component_sq_sum();
                tilted_boards[idx] = tilted;
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (ALL_DIRS[best_idx], tilted_boards[best_idx])
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut out = BufWriter::new(stdout.lock());

    let mut flavors = [0u8; NN];
    for flavor in &mut flavors {
        *flavor = scanner.next_usize() as u8;
    }

    let mut board = Board::new();
    for turn in 0..NN {
        let p = scanner.next_usize();
        board.place_pth_empty(p, flavors[turn]);
        let next_flavor = if turn + 1 < NN {
            Some(flavors[turn + 1])
        } else {
            None
        };
        let (dir, next_board) = choose_direction(&board, next_flavor);
        board = next_board;
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
