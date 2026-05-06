// v001_baseline.rs
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;

struct Scanner<R> {
    reader: R,
    buffer: Vec<String>,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    fn next<T: FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("failed to parse input token");
            }

            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .expect("failed to read input line");
            if bytes == 0 {
                panic!("unexpected EOF");
            }
            self.buffer = line
                .split_whitespace()
                .rev()
                .map(|token| token.to_string())
                .collect();
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut out = BufWriter::new(stdout.lock());

    let n: usize = scanner.next();
    let m: usize = scanner.next();
    let _eps: String = scanner.next();

    for _ in 0..m {
        let d: usize = scanner.next();
        for _ in 0..d {
            let _i: usize = scanner.next();
            let _j: usize = scanner.next();
        }
    }

    let mut oil_cells = Vec::new();
    for i in 0..n {
        for j in 0..n {
            writeln!(out, "q 1 {} {}", i, j).unwrap();
            out.flush().unwrap();

            let amount: i32 = scanner.next();
            if amount > 0 {
                oil_cells.push((i, j));
            }
        }
    }

    write!(out, "a {}", oil_cells.len()).unwrap();
    for (i, j) in oil_cells {
        write!(out, " {} {}", i, j).unwrap();
    }
    writeln!(out).unwrap();
    out.flush().unwrap();

    let _accepted: i32 = scanner.next();
}
