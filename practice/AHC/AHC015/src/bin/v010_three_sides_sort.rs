// v010_three_sides_sort.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;

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

    fn tilt(&mut self, dir: u8) {
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
        self.cells = next;
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

fn direction_for(flavor: u8) -> u8 {
    match flavor {
        1 => b'F',
        2 => b'B',
        3 => b'L',
        _ => unreachable!("invalid flavor: {flavor}"),
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
    for flavor in flavors {
        let p = scanner.next_usize();
        board.place_pth_empty(p, flavor);
        let dir = direction_for(flavor);
        board.tilt(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
