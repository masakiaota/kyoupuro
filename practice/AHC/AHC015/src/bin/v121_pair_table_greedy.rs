// v121_pair_table_greedy.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;
const DIR_TABLE: [[u8; 3]; 3] = [
    [b'B', b'F', b'F'],
    [b'B', b'R', b'L'],
    [b'B', b'R', b'L'],
];

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
        unreachable!("invalid p: {p}");
    }

    fn tilt(&mut self, dir: u8) {
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

fn choose_dir(cur_flavor: u8, next_flavor: Option<u8>) -> u8 {
    match next_flavor {
        Some(next) => DIR_TABLE[(cur_flavor - 1) as usize][(next - 1) as usize],
        None => b'F',
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
        let cur_flavor = flavors[turn];
        board.place_pth_empty(p, cur_flavor);
        let next_flavor = if turn + 1 < NN {
            Some(flavors[turn + 1])
        } else {
            None
        };
        let dir = choose_dir(cur_flavor, next_flavor);
        board.tilt(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
