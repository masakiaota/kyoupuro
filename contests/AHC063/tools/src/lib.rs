#![allow(non_snake_case, unused_macros)]

use itertools::Itertools;
use proconio::input;
use rand::prelude::*;
use std::ops::RangeBounds;

pub trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}
impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }
    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

#[macro_export]
macro_rules! mat {
	($($e:expr),*) => { Vec::from(vec![$($e),*]) };
	($($e:expr,)*) => { Vec::from(vec![$($e),*]) };
	($e:expr; $d:expr) => { Vec::from(vec![$e; $d]) };
	($e:expr; $d:expr $(; $ds:expr)+) => { Vec::from(vec![mat![$e $(; $ds)*]; $d]) };
}

#[derive(Clone, Debug)]
pub struct Input {
    pub N: usize,
    pub M: usize,
    pub C: usize,
    pub d: Vec<usize>,
    pub f: Vec<Vec<usize>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.N, self.M, self.C)?;
        writeln!(f, "{}", self.d.iter().join(" "))?;
        for i in 0..self.N {
            writeln!(f, "{}", self.f[i].iter().join(" "))?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize, M: usize, C: usize,
        d: [usize; M],
        f: [[usize; N]; N],
    }
    Input { N, M, C, d, f }
}

pub fn read<T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr, R: RangeBounds<T>>(
    token: Option<&str>,
    range: R,
) -> Result<T, String> {
    if let Some(v) = token {
        if let Ok(v) = v.parse::<T>() {
            if !range.contains(&v) {
                Err(format!("Out of range: {}", v))
            } else {
                Ok(v)
            }
        } else {
            Err(format!("Parse error: {}", v))
        }
    } else {
        Err("Unexpected EOF".to_owned())
    }
}

pub const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];
pub const DIR: [char; 4] = ['U', 'D', 'L', 'R'];

pub struct Output {
    pub out: Vec<usize>,
}

pub fn parse_output(_input: &Input, f: &str) -> Result<Output, String> {
    let f = f.split_whitespace();
    let mut out = vec![];
    for s in f {
        let Some(dir) = DIR.iter().position(|&c| c.to_string() == s) else {
            return Err(format!("Invalid direction: {}", s));
        };
        out.push(dir);
        if out.len() > 100000 {
            return Err("Too long output".to_owned());
        }
    }
    Ok(Output { out })
}

pub fn gen(seed: u64, fix_N: Option<usize>, fix_M: Option<usize>, fix_C: Option<usize>) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut N = rng.gen_range(8..=16i32) as usize;
    if let Some(fix_N) = fix_N {
        N = fix_N;
    }
    let mut M = rng.gen_range(((N * N + 3) / 4) as i32..=((3 * N * N) / 4) as i32) as usize;
    if let Some(fix_M) = fix_M {
        M = fix_M;
    }
    let mut C = rng.gen_range(3..=7i32) as usize;
    if let Some(fix_C) = fix_C {
        C = fix_C;
    }
    let mut d = vec![1; M];
    'cont: loop {
        let mut n = vec![0; C + 1];
        n[C] = M - 5 - C;
        for i in 1..C {
            n[i] = rng.gen_range(0..=(M - 5 - C) as i32) as usize;
        }
        n.sort();
        let mut m = vec![0; C + 1];
        for c in 1..=C {
            m[c] = n[c] - n[c - 1] + 1;
            if m[c] > (M - 5) / 2 {
                continue 'cont;
            }
        }
        let mut p = 5;
        for c in 1..=C {
            for _ in 0..m[c] {
                d[p] = c;
                p += 1;
            }
        }
        d[5..M].shuffle(&mut rng);
        break;
    }
    let mut ij = vec![];
    for i in 0..N {
        for j in 0..N {
            if j > 0 || i > 4 {
                ij.push((i, j));
            }
        }
    }
    ij.shuffle(&mut rng);
    let mut d2 = d[5..].to_vec();
    d2.shuffle(&mut rng);
    let mut f = mat![0; N; N];
    for p in 0..M - 5 {
        let (i, j) = ij[p];
        f[i][j] = d2[p];
    }
    Input { N, M, C, d, f }
}

pub struct State {
    N: usize,
    d: Vec<usize>,
    f: Vec<Vec<usize>>,
    ij: Vec<(usize, usize)>,
    c: Vec<usize>,
    turn: usize,
}

impl State {
    pub fn new(input: &Input) -> Self {
        Self {
            N: input.N,
            d: input.d.clone(),
            f: input.f.clone(),
            ij: (0..5).rev().map(|i| (i, 0)).collect(),
            c: vec![1; 5],
            turn: 0,
        }
    }
    pub fn apply(&mut self, dir: usize) -> Result<(), String> {
        let (i0, j0) = self.ij[0];
        let new_i = i0 + DIJ[dir].0;
        let new_j = j0 + DIJ[dir].1;
        if new_i >= self.N || new_j >= self.N {
            return Err(format!("Try to move out of the board (turn {})", self.turn));
        }
        if (new_i, new_j) == self.ij[1] {
            return Err(format!("Try to U-turn (turn {})", self.turn));
        }
        self.ij.insert(0, (new_i, new_j));
        if self.f[new_i][new_j] != 0 {
            let c = self.f[new_i][new_j];
            self.f[new_i][new_j] = 0;
            self.c.push(c);
        } else {
            self.ij.pop();
        }
        if let Some(h) = (1..=self.ij.len() - 2).find(|&h| self.ij[h] == (new_i, new_j)) {
            for p in h + 1..self.ij.len() {
                let (i, j) = self.ij[p];
                let c = self.c[p];
                self.f[i][j] = c;
            }
            self.ij.truncate(h + 1);
            self.c.truncate(h + 1);
        }
        self.turn += 1;
        Ok(())
    }
    pub fn score(&self) -> i64 {
        let mut E = 0;
        for p in 0..self.ij.len() {
            if self.d[p] != self.c[p] {
                E += 1;
            }
        }
        self.turn as i64 + 10000 * (E + 2 * (self.d.len() as i64 - self.ij.len() as i64))
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, &out.out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &[usize]) -> (i64, String, State) {
    let mut state = State::new(input);
    for &dir in out {
        if let Err(err) = state.apply(dir) {
            return (0, err, state);
        }
    }
    (state.score(), String::new(), state)
}

pub mod vis;

pub use vis::*;
