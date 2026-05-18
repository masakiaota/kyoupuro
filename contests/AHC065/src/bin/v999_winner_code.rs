// v999_winner_code.rs
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};

const EMPTY_BOX: isize = -1;

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    nn: usize,
    a0: Vec<isize>,
    p0: Vec<usize>,
    pe: usize,
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n: usize = it.next().unwrap().parse().unwrap();
        let nn = n * n;
        let mut a0 = vec![0isize; nn];
        let mut p0 = vec![0usize; nn];

        for p in 0..nn {
            let b: usize = it.next().unwrap().parse().unwrap();
            a0[p] = b as isize;
            p0[b] = p;
        }

        let pe = n / 2;
        Self { n, nn, a0, p0, pe }
    }
}

#[derive(Debug, Clone)]
struct Conveyor {
    mem: Vec<usize>,
}

impl Conveyor {
    fn from_cells(cells: Vec<usize>) -> Self {
        Self { mem: cells }
    }

    #[inline(always)]
    fn size(&self) -> usize {
        self.mem.len()
    }

    #[inline(always)]
    fn p(&self, i: isize) -> usize {
        let n = self.mem.len() as isize;
        let mut ii = i % n;
        if ii < 0 {
            ii += n;
        }
        self.mem[ii as usize]
    }
}

#[derive(Debug, Clone, Copy)]
struct Move {
    cv: usize,
    d: i32,
}

#[derive(Debug, Clone)]
struct Board<'a> {
    input: &'a Input,
    box_at: Vec<isize>,
    pos_of: Vec<usize>,
    nb: usize,
}

impl<'a> Board<'a> {
    fn new(input: &'a Input) -> Self {
        Self {
            input,
            box_at: input.a0.clone(),
            pos_of: input.p0.clone(),
            nb: 0,
        }
    }

    fn apply(&mut self, conveyors: &[Conveyor], mv: Move) {
        let cv = &conveyors[mv.cv];
        if mv.d == 1 {
            let b0 = self.box_at[cv.p(0)];
            for i in (1..cv.size()).rev() {
                let p1 = cv.p(i as isize);
                let p2 = cv.p(i as isize + 1);
                let b = self.box_at[p1];
                self.set(p2, b);
            }
            self.set(cv.p(1), b0);
        } else {
            let b0 = self.box_at[cv.p(0)];
            for i in 1..cv.size() {
                let p1 = cv.p(i as isize);
                let p2 = cv.p(i as isize - 1);
                let b = self.box_at[p1];
                self.set(p2, b);
            }
            self.set(cv.p(cv.size() as isize - 1), b0);
        }
        self.check_exit();
    }

    fn check_exit(&mut self) {
        if self.box_at[self.input.pe] == self.nb as isize {
            self.set(self.input.pe, EMPTY_BOX);
            self.nb += 1;
        }
    }

    #[inline(always)]
    fn box_at(&self, p: usize) -> isize {
        self.box_at[p]
    }

    #[inline(always)]
    fn set(&mut self, p: usize, b: isize) {
        self.box_at[p] = b;
        if b >= 0 {
            self.pos_of[b as usize] = p;
        }
    }
}

#[derive(Debug, Clone)]
struct ResultData {
    score: i64,
    conveyors: Vec<Conveyor>,
    moves: Vec<Move>,
}

impl ResultData {
    fn print(&self, n: usize) {
        let mut out = String::new();
        writeln!(&mut out, "{}", self.conveyors.len()).unwrap();
        for cv in &self.conveyors {
            write!(&mut out, "{}", cv.size()).unwrap();
            for &p in &cv.mem {
                write!(&mut out, " {} {}", p / n, p % n).unwrap();
            }
            out.push('\n');
        }

        writeln!(&mut out, "{}", self.moves.len()).unwrap();
        for mv in &self.moves {
            writeln!(&mut out, "{} {}", mv.cv, mv.d).unwrap();
        }
        io::stdout().write_all(out.as_bytes()).unwrap();
    }
}

struct Solver<'a> {
    input: &'a Input,
    pattern: usize,
    conveyors: Vec<Conveyor>,
    main_cv: usize,
    swap_cv: Vec<Option<usize>>,
}

impl<'a> Solver<'a> {
    fn new(input: &'a Input, pattern: usize) -> Self {
        Self {
            input,
            pattern,
            conveyors: Vec::new(),
            main_cv: 0,
            swap_cv: vec![None; input.nn],
        }
    }

    fn solve(&mut self) -> ResultData {
        self.create_conveyor();

        let main_size = self.conveyors[self.main_cv].size() as i32;
        let mut best_moves: Option<Vec<Move>> = None;
        let mut min_t = -main_size;
        let mut max_t = 0;

        while max_t - min_t > 1 {
            let mid = (min_t + max_t) / 2;
            let moves = self.solve0(mid);
            if let Some(moves) = moves {
                if best_moves
                    .as_ref()
                    .is_none_or(|best| moves.len() < best.len())
                {
                    best_moves = Some(moves);
                }
                min_t = mid;
            } else {
                max_t = mid;
            }
        }

        let moves = best_moves.unwrap();
        let score = 1_000_000.0
            + (1_000_000.0 * (100_000.0 / moves.len() as f64).ln() / 2.0_f64.ln()).round();

        ResultData {
            score: score as i64,
            conveyors: self.conveyors.clone(),
            moves,
        }
    }

    fn solve0(&self, t0: i32) -> Option<Vec<Move>> {
        let mut moves = Vec::new();
        let mut board = Board::new(self.input);
        let main_cv = &self.conveyors[self.main_cv];
        let main_size = main_cv.size() as i32;

        for t in t0..self.input.nn as i32 {
            for i in 0..main_cv.size() {
                let sb = t + i as i32;
                let p = main_cv.p(i as isize);
                let Some(sc_id) = self.swap_cv[p] else {
                    continue;
                };
                let sc = &self.conveyors[sc_id];

                if sb < 0 {
                    let b = board.box_at(p);
                    if 0 <= b && b < (sb + main_size) as isize {
                        let b1 = board.box_at(sc.p(1));
                        if b1 != EMPTY_BOX && b1 >= main_size as isize {
                            let mv = Move { cv: sc_id, d: 1 };
                            moves.push(mv);
                            board.apply(&self.conveyors, mv);
                        }
                    }
                } else {
                    let b1 = board.box_at(sc.p(1));
                    if sb as isize == b1 {
                        let mv = Move { cv: sc_id, d: 1 };
                        moves.push(mv);
                        board.apply(&self.conveyors, mv);
                    }
                }
            }

            let mv = Move {
                cv: self.main_cv,
                d: -1,
            };
            moves.push(mv);
            board.apply(&self.conveyors, mv);
        }

        if board.nb == self.input.nn {
            Some(moves)
        } else {
            None
        }
    }

    fn create_conveyor(&mut self) {
        if self.pattern < 4 {
            self.create_conveyor0(self.pattern % 2, (self.pattern % 4) / 2);
        } else {
            self.create_conveyor1(self.pattern % 2, (self.pattern % 4) / 2);
        }
    }

    fn create_conveyor0(&mut self, dir: usize, reflect: usize) {
        let n = self.input.n;
        let mut main_loop = Vec::new();
        for j in (0..=n - 2).rev() {
            main_loop.push(self.pos(0, j, reflect));
        }
        main_loop.push(self.pos(1, 0, reflect));

        for j in (0..n).step_by(4) {
            for i in 2..n {
                main_loop.push(self.pos(i, j, reflect));
            }
            main_loop.push(self.pos(n - 1, j + 1, reflect));
            for i in (2..n).rev() {
                main_loop.push(self.pos(i, j + 2, reflect));
            }
            if j + 3 != n - 1 {
                main_loop.push(self.pos(2, j + 3, reflect));
            }
        }
        main_loop.push(self.pos(1, n - 2, reflect));

        if dir == 0 {
            main_loop.reverse();
        }
        main_loop = self.shift(main_loop);

        self.main_cv = self.conveyors.len();
        self.conveyors.push(Conveyor::from_cells(main_loop));
        self.swap_cv = vec![None; self.input.nn];

        for j in 1..n - 2 {
            let p0 = self.pos(0, j, reflect);
            let p1 = self.pos(1, j, reflect);
            self.add_swap(p0, p1);
        }

        for j in (0..n).step_by(2) {
            for i in 0..n {
                if i == 0 && j != n - 2 {
                    continue;
                }
                if i == 1 && j != 0 && j != n - 2 {
                    continue;
                }
                if i == 2 && j % 4 != 0 && j != n - 2 {
                    continue;
                }
                if i == n - 1 && j % 4 == 0 {
                    continue;
                }
                let p0 = self.pos(i, j, reflect);
                let p1 = self.pos(i, j + 1, reflect);
                self.add_swap(p0, p1);
            }
        }
    }

    fn create_conveyor1(&mut self, dir: usize, reflect: usize) {
        let n = self.input.n;
        let mut main_loop = Vec::new();

        for i in (0..n).step_by(4) {
            let j0 = if i == 0 { 0 } else { 2 };
            for j in j0..n {
                main_loop.push(self.pos(i, j, reflect));
            }
            main_loop.push(self.pos(i + 1, n - 1, reflect));

            let j1 = if i == n - 4 { 0 } else { 2 };
            for j in (j1..n).rev() {
                main_loop.push(self.pos(i + 2, j, reflect));
            }
            if i != n - 4 {
                main_loop.push(self.pos(i + 3, 2, reflect));
            }
        }

        for i in (1..=n - 3).rev() {
            main_loop.push(self.pos(i, 0, reflect));
        }

        if dir == 0 {
            main_loop.reverse();
        }
        main_loop = self.shift(main_loop);

        self.main_cv = self.conveyors.len();
        self.conveyors.push(Conveyor::from_cells(main_loop));
        self.swap_cv = vec![None; self.input.nn];

        for i in 1..n - 2 {
            let p0 = self.pos(i, 0, reflect);
            let p1 = self.pos(i, 1, reflect);
            self.add_swap(p0, p1);
        }

        for i in (0..n).step_by(2) {
            for j in 0..n {
                if j == 0 && i != n - 2 {
                    continue;
                }
                if j == 1 && i != 0 && i != n - 2 {
                    continue;
                }
                if j == 2 && i % 4 != 0 && i != n - 2 {
                    continue;
                }
                if j == n - 1 && i % 4 == 0 {
                    continue;
                }
                let p0 = self.pos(i, j, reflect);
                let p1 = self.pos(i + 1, j, reflect);
                self.add_swap(p0, p1);
            }
        }
    }

    #[inline(always)]
    fn add_swap(&mut self, p0: usize, p1: usize) {
        let id = self.conveyors.len();
        self.swap_cv[p0] = Some(id);
        self.conveyors.push(Conveyor::from_cells(vec![p0, p1]));
    }

    #[inline(always)]
    fn pos(&self, i: usize, j: usize, reflect: usize) -> usize {
        if reflect == 0 {
            i * self.input.n + j
        } else {
            i * self.input.n + (self.input.n - j - 1)
        }
    }

    fn shift(&self, org: Vec<usize>) -> Vec<usize> {
        let ii = org.iter().position(|&p| p == self.input.pe).unwrap();
        (0..org.len()).map(|i| org[(i + ii) % org.len()]).collect()
    }
}

fn main() {
    let input = Input::read();
    let mut best: Option<ResultData> = None;

    for pattern in 0..8 {
        let mut solver = Solver::new(&input, pattern);
        let res = solver.solve();
        if best.as_ref().is_none_or(|best| res.score > best.score) {
            best = Some(res);
        }
    }

    best.unwrap().print(input.n);
}
