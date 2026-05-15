// v002_baseline_manhattan.rs
use std::io::BufRead;
use std::io::Write;

const N: usize = 30;
const Q: usize = 1000;
const H_EDGE_ROWS: usize = N;
const H_EDGE_COLS: usize = N - 1;
const V_EDGE_ROWS: usize = N - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryInput {
    s: Point,
    t: Point,
}

impl QueryInput {
    fn read<R: BufRead>(reader: &mut R) -> Option<Self> {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            return None;
        }
        Some(Self::from_line(&line))
    }

    fn from_line(line: &str) -> Self {
        let mut it = line.split_whitespace();
        let si = it.next().unwrap().parse::<usize>().unwrap();
        let sj = it.next().unwrap().parse::<usize>().unwrap();
        let ti = it.next().unwrap().parse::<usize>().unwrap();
        let tj = it.next().unwrap().parse::<usize>().unwrap();

        Self {
            s: Point::new(si, sj),
            t: Point::new(ti, tj),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservedLength {
    value: i64,
}

impl ObservedLength {
    fn read<R: BufRead>(reader: &mut R) -> Option<Self> {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            return None;
        }

        let value = line.trim().parse::<i64>().unwrap();

        Some(Self { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    U,
    D,
    L,
    R,
}

impl Dir {
    fn to_char(self) -> char {
        match self {
            Dir::U => 'U',
            Dir::D => 'D',
            Dir::L => 'L',
            Dir::R => 'R',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Output {
    dirs: Vec<Dir>,
}

impl Output {
    fn new(dirs: Vec<Dir>) -> Self {
        Self { dirs }
    }

    fn write<W: Write>(&self, writer: &mut W) {
        for &dir in &self.dirs {
            write!(writer, "{}", dir.to_char()).unwrap();
        }
        writeln!(writer).unwrap();
        writer.flush().unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    i: usize,
    j: usize,
}

impl Point {
    fn new(i: usize, j: usize) -> Self {
        debug_assert!(i < N);
        debug_assert!(j < N);
        Self { i, j }
    }
}

#[derive(Debug, Clone)]
struct State {
    /// 0-indexed の現在クエリ番号。
    turn: usize,
}

impl State {
    fn new() -> Self {
        Self { turn: 0 }
    }

    fn update(&mut self, _input: QueryInput, _output: &Output, observed: ObservedLength) {
        let _ = observed.value;
        self.turn += 1;
    }
}

fn solve_query(input: QueryInput) -> Output {
    let mut dirs =
        Vec::with_capacity(input.s.i.abs_diff(input.t.i) + input.s.j.abs_diff(input.t.j));

    if input.s.i <= input.t.i {
        for _ in input.s.i..input.t.i {
            dirs.push(Dir::D);
        }
    } else {
        for _ in input.t.i..input.s.i {
            dirs.push(Dir::U);
        }
    }

    if input.s.j <= input.t.j {
        for _ in input.s.j..input.t.j {
            dirs.push(Dir::R);
        }
    } else {
        for _ in input.t.j..input.s.j {
            dirs.push(Dir::L);
        }
    }

    Output::new(dirs)
}

fn skip_local_edge_lines<R: BufRead>(reader: &mut R) {
    let mut line = String::new();
    for _ in 0..((H_EDGE_ROWS - 1) + V_EDGE_ROWS) {
        line.clear();
        reader.read_line(&mut line).unwrap();
    }
}

fn run_local<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) {
    skip_local_edge_lines(reader);

    let mut state = State::new();
    for _ in 0..Q {
        let input = QueryInput::read(reader).unwrap();
        let output = solve_query(input);
        output.write(writer);
        state.update(input, &output, ObservedLength { value: 0 });
    }
}

fn run_interactive<R: BufRead, W: Write>(reader: &mut R, writer: &mut W, first_line: &str) {
    let mut state = State::new();
    let mut input = QueryInput::from_line(first_line);

    for turn in 0..Q {
        let output = solve_query(input);
        output.write(writer);
        let observed = ObservedLength::read(reader).unwrap();
        state.update(input, &output, observed);

        if turn + 1 == Q {
            break;
        }
        input = QueryInput::read(reader).unwrap();
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());

    let mut first_line = String::new();
    if reader.read_line(&mut first_line).unwrap() == 0 {
        return;
    }

    if first_line.split_whitespace().count() == H_EDGE_COLS {
        run_local(&mut reader, &mut writer);
    } else {
        run_interactive(&mut reader, &mut writer, &first_line);
    }
}
