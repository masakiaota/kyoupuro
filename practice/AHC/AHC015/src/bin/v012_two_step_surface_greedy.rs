// v012_two_step_surface_greedy.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const IMMEDIATE_WEIGHT: i128 = 2;

#[derive(Clone, Copy)]
struct Board {
    cells: [u8; NN],
}

impl Board {
    fn new() -> Self {
        Self { cells: [0; NN] }
    }

    fn place_pth_empty(&mut self, p: usize, flavor: u8) {
        let mut empty_count = 0usize;
        for idx in 0..NN {
            if self.cells[idx] == 0 {
                empty_count += 1;
                if empty_count == p {
                    self.cells[idx] = flavor;
                    return;
                }
            }
        }
        unreachable!("invalid empty index: {p}");
    }

    fn with_added_at(&self, idx: usize, flavor: u8) -> Self {
        let mut next = *self;
        assert_eq!(next.cells[idx], 0, "cell must be empty");
        next.cells[idx] = flavor;
        next
    }

    fn empty_indices(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(NN);
        for idx in 0..NN {
            if self.cells[idx] == 0 {
                indices.push(idx);
            }
        }
        indices
    }

    fn tilted(&self, dir: u8) -> Self {
        let mut next = [0u8; NN];
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
            _ => unreachable!("invalid direction: {dir}"),
        }
        Self { cells: next }
    }

    fn component_sq_sum(&self) -> i64 {
        let mut seen = [false; NN];
        let mut stack = Vec::with_capacity(NN);
        let mut members = Vec::with_capacity(NN);
        let mut total = 0i64;
        for start in 0..NN {
            let flavor = self.cells[start];
            if flavor == 0 || seen[start] {
                continue;
            }
            seen[start] = true;
            stack.push(start);
            members.clear();
            while let Some(idx) = stack.pop() {
                members.push(idx);
                let r = idx / N;
                let c = idx % N;
                if r > 0 {
                    let ni = idx - N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if r + 1 < N {
                    let ni = idx + N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if c > 0 {
                    let ni = idx - 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if c + 1 < N {
                    let ni = idx + 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
            }
            let size = members.len() as i64;
            total += size * size;
        }
        total
    }

    fn component_sizes_for_flavor(&self, flavor: u8) -> [usize; NN] {
        let mut seen = [false; NN];
        let mut sizes = [0usize; NN];
        let mut stack = Vec::with_capacity(NN);
        let mut members = Vec::with_capacity(NN);
        for start in 0..NN {
            if self.cells[start] != flavor || seen[start] {
                continue;
            }
            seen[start] = true;
            stack.push(start);
            members.clear();
            while let Some(idx) = stack.pop() {
                members.push(idx);
                let r = idx / N;
                let c = idx % N;
                if r > 0 {
                    let ni = idx - N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if r + 1 < N {
                    let ni = idx + N;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if c > 0 {
                    let ni = idx - 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
                if c + 1 < N {
                    let ni = idx + 1;
                    if !seen[ni] && self.cells[ni] == flavor {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
            }
            let size = members.len();
            for &idx in &members {
                sizes[idx] = size;
            }
        }
        sizes
    }

    fn next_surface_gain(&self, dir: u8, next_flavor: u8) -> i64 {
        let sizes = self.component_sizes_for_flavor(next_flavor);
        let mut gain = 0i64;
        match dir {
            b'L' => {
                for r in 0..N {
                    let mut occupied = 0usize;
                    for c in 0..N {
                        if self.cells[r * N + c] != 0 {
                            occupied += 1;
                        }
                    }
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + occupied - 1;
                    if self.cells[idx] == next_flavor {
                        let size = sizes[idx] as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    let mut occupied = 0usize;
                    for c in 0..N {
                        if self.cells[r * N + c] != 0 {
                            occupied += 1;
                        }
                    }
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + (N - occupied);
                    if self.cells[idx] == next_flavor {
                        let size = sizes[idx] as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'F' => {
                for c in 0..N {
                    let mut occupied = 0usize;
                    for r in 0..N {
                        if self.cells[r * N + c] != 0 {
                            occupied += 1;
                        }
                    }
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (occupied - 1) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = sizes[idx] as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    let mut occupied = 0usize;
                    for r in 0..N {
                        if self.cells[r * N + c] != 0 {
                            occupied += 1;
                        }
                    }
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (N - occupied) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = sizes[idx] as i64;
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
struct OneStepEval {
    dir: u8,
    surface: i64,
    comp_sq: i64,
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

fn best_one_step_eval(board: &Board, next_flavor: Option<u8>) -> OneStepEval {
    let mut best = OneStepEval {
        dir: b'F',
        surface: -1,
        comp_sq: -1,
    };
    for dir in DIRS {
        let tilted = board.tilted(dir);
        let comp_sq = tilted.component_sq_sum();
        let surface = match next_flavor {
            Some(flavor) => tilted.next_surface_gain(dir, flavor),
            None => -1,
        };
        if surface > best.surface || (surface == best.surface && comp_sq > best.comp_sq) {
            best = OneStepEval {
                dir,
                surface,
                comp_sq,
            };
        }
    }
    best
}

fn choose_direction(board: &Board, next1: Option<u8>, next2: Option<u8>) -> u8 {
    if next1.is_none() {
        return best_one_step_eval(board, None).dir;
    }

    let next1_flavor = next1.unwrap();
    let mut best_dir = b'F';
    let mut best_primary = i128::MIN;
    let mut best_immediate = -1i64;
    let mut best_future_surface_sum = -1i64;
    let mut best_future_comp_sum = -1i64;
    let mut best_comp_sq = -1i64;

    for dir in DIRS {
        let tilted = board.tilted(dir);
        let comp_sq = tilted.component_sq_sum();
        let immediate_surface = tilted.next_surface_gain(dir, next1_flavor);
        let empties = tilted.empty_indices();

        let mut future_surface_sum = 0i64;
        let mut future_comp_sum = 0i64;
        if !empties.is_empty() {
            for idx in empties {
                let placed = tilted.with_added_at(idx, next1_flavor);
                let eval2 = best_one_step_eval(&placed, next2);
                future_surface_sum += eval2.surface.max(0);
                future_comp_sum += eval2.comp_sq;
            }
        }

        let primary = IMMEDIATE_WEIGHT * immediate_surface as i128 * (NN as i128)
            + future_surface_sum as i128;
        if primary > best_primary
            || (primary == best_primary && immediate_surface > best_immediate)
            || (primary == best_primary
                && immediate_surface == best_immediate
                && future_surface_sum > best_future_surface_sum)
            || (primary == best_primary
                && immediate_surface == best_immediate
                && future_surface_sum == best_future_surface_sum
                && future_comp_sum > best_future_comp_sum)
            || (primary == best_primary
                && immediate_surface == best_immediate
                && future_surface_sum == best_future_surface_sum
                && future_comp_sum == best_future_comp_sum
                && comp_sq > best_comp_sq)
        {
            best_primary = primary;
            best_immediate = immediate_surface;
            best_future_surface_sum = future_surface_sum;
            best_future_comp_sum = future_comp_sum;
            best_comp_sq = comp_sq;
            best_dir = dir;
        }
    }

    best_dir
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
        let flavor = flavors[turn];
        board.place_pth_empty(p, flavor);
        let next1 = if turn + 1 < NN {
            Some(flavors[turn + 1])
        } else {
            None
        };
        let next2 = if turn + 2 < NN {
            Some(flavors[turn + 2])
        } else {
            None
        };
        let dir = choose_direction(&board, next1, next2);
        board = board.tilted(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
