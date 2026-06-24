// v001_baseline.rs
use proconio::{input, marker::Bytes};
use std::fmt::Write as _;

const N: usize = 20;
const M: usize = 50;
const K: usize = 10;
const CELL_COUNT: usize = N * N;
const START_ID: usize = 0;
const GOAL_ID: usize = CELL_COUNT - 1;

#[derive(Debug, Clone)]
struct Input {
    grid: [u8; CELL_COUNT],
}

impl Input {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            k: usize,
            rows: [Bytes; N],
        }

        assert_eq!(n, N);
        assert_eq!(m, M);
        assert_eq!(k, K);

        let mut grid = [b'#'; CELL_COUNT];
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), N);
            for (j, &cell) in row.iter().enumerate() {
                assert!(cell == b'.' || cell == b'#');
                grid[Self::id(i, j)] = cell;
            }
        }

        assert_eq!(grid[START_ID], b'.');
        assert_eq!(grid[GOAL_ID], b'.');

        Self { grid }
    }

    #[inline(always)]
    fn id(i: usize, j: usize) -> usize {
        i * N + j
    }
}

#[derive(Debug, Clone)]
struct Output;

impl Output {
    fn empty() -> Self {
        Self
    }

    fn to_output_string(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "0").unwrap();
        writeln!(&mut out, "0").unwrap();
        out
    }

    fn print(&self) {
        print!("{}", self.to_output_string());
    }
}

fn main() {
    let input = Input::read();
    let _ = input.grid[START_ID];

    Output::empty().print();
}
