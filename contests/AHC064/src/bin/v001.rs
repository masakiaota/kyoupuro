// v001.rs
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const CAR_COUNT: usize = R * INIT_LEN;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

type CarId = usize;
type LineIdx = usize;
type PosIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CarPos {
    r: LineIdx,
    c: PosIdx,
}

#[derive(Debug, Clone)]
struct Input {
    initial: [[CarId; INIT_LEN]; R],
    initial_pos: [CarPos; CAR_COUNT],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        Self::from_str(&s)
    }

    fn from_str(s: &str) -> Self {
        let mut it = s.split_whitespace();
        let r_count = it.next().unwrap().parse::<usize>().unwrap();
        assert_eq!(r_count, R);

        let mut initial = [[0; INIT_LEN]; R];
        let mut initial_pos = [CarPos { r: 0, c: 0 }; CAR_COUNT];

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = it.next().unwrap().parse::<usize>().unwrap();
                initial[r][c] = car;
                initial_pos[car] = CarPos { r, c };
            }
        }

        Self {
            initial,
            initial_pos,
        }
    }

    #[inline(always)]
    fn target_id(r: usize, c: usize) -> CarId {
        r * INIT_LEN + c
    }

    #[inline(always)]
    fn target_line(car: CarId) -> usize {
        car / INIT_LEN
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Move {
    kind: usize,
    i: LineIdx,
    j: LineIdx,
    k: usize,
}

impl Move {
    #[inline(always)]
    fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    #[inline(always)]
    fn siding_to_dep(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_SIDING_TO_DEP,
            i,
            j,
            k,
        }
    }
}

#[derive(Debug, Clone)]
struct Output {
    turns: Vec<Vec<Move>>,
}

impl Output {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            turns: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    fn push_turn(&mut self, moves: Vec<Move>) {
        self.turns.push(moves);
    }

    fn print(&self) {
        let move_count: usize = self.turns.iter().map(Vec::len).sum();
        let mut s = String::with_capacity(16 + self.turns.len() * 4 + move_count * 16);

        s.push_str(&format!("{}\n", self.turns.len()));
        for moves in &self.turns {
            s.push_str(&format!("{}\n", moves.len()));
            for mv in moves {
                s.push_str(&format!("{} {} {} {}\n", mv.kind, mv.i, mv.j, mv.k));
            }
        }

        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        out.write_all(s.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone)]
struct State {
    dep: Vec<Vec<CarId>>,
    sid: Vec<Vec<CarId>>,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut dep = vec![Vec::with_capacity(DEP_CAP); R];
        let sid = vec![Vec::with_capacity(SIDING_CAP); R];

        for r in 0..R {
            dep[r].extend_from_slice(&input.initial[r]);
        }

        Self { dep, sid }
    }

    fn apply_move(&mut self, mv: Move) {
        assert!(mv.i < R);
        assert!(mv.j < R);
        assert!(mv.k >= 1);

        if mv.kind == MOVE_DEP_TO_SIDING {
            let old_dep_len = self.dep[mv.i].len();
            assert!(mv.k <= old_dep_len);
            assert!(self.sid[mv.j].len() + mv.k <= SIDING_CAP);

            let block = self.dep[mv.i].split_off(old_dep_len - mv.k);
            self.sid[mv.j].splice(0..0, block);
        } else {
            assert_eq!(mv.kind, MOVE_SIDING_TO_DEP);
            assert!(mv.k <= self.sid[mv.j].len());
            assert!(self.dep[mv.i].len() + mv.k <= DEP_CAP);

            let block: Vec<_> = self.sid[mv.j].drain(0..mv.k).collect();
            self.dep[mv.i].extend(block);
        }
    }

    fn is_complete(&self) -> bool {
        for r in 0..R {
            if self.dep[r].len() != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] != Input::target_id(r, c) {
                    return false;
                }
            }
            if !self.sid[r].is_empty() {
                return false;
            }
        }
        true
    }
}

fn emit_move(output: &mut Output, state: &mut State, mv: Move) {
    state.apply_move(mv);
    output.push_turn(vec![mv]);
}

fn solve(input: &Input) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(400);

    for i in 0..R {
        while !state.dep[i].is_empty() {
            let car = *state.dep[i].last().unwrap();
            let j = Input::target_line(car);
            emit_move(&mut output, &mut state, Move::dep_to_siding(i, j, 1));
        }
    }

    for r in 0..R {
        if r == R - 1 {
            emit_move(&mut output, &mut state, Move::dep_to_siding(0, 0, INIT_LEN));
        }

        let tmp = if r == R - 1 { 0 } else { R - 1 };

        while state.dep[r].len() < INIT_LEN {
            let c = state.dep[r].len();
            let target = Input::target_id(r, c);
            let p = state.sid[r].iter().position(|&car| car == target).unwrap();

            if p > 0 {
                emit_move(&mut output, &mut state, Move::siding_to_dep(tmp, r, p));
            }

            emit_move(&mut output, &mut state, Move::siding_to_dep(r, r, 1));

            if p > 0 {
                emit_move(&mut output, &mut state, Move::dep_to_siding(tmp, r, p));
            }
        }

        if r == R - 1 {
            emit_move(&mut output, &mut state, Move::siding_to_dep(0, 0, INIT_LEN));
        }
    }

    assert!(state.is_complete());
    assert!(output.turns.len() <= MAX_TURNS);

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
