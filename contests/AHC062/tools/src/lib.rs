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
    N: usize,
    A: Vec<Vec<i64>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.N)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.A[i].iter().join(" "))?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        A: [[i64; N]; N],
    }
    Input { N, A }
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

pub struct Output {
    pub out: Vec<(usize, usize)>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut f = f.split_whitespace().peekable();
    let mut out = vec![];
    while f.peek().is_some() {
        if out.len() >= input.N * input.N {
            return Err("Too many outputs".to_owned());
        }
        out.push((read(f.next(), 0..input.N)?, read(f.next(), 0..input.N)?));
    }
    Ok(Output { out })
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 200;
    let mut A0 = (1..=(N * N) as i64).collect_vec();
    A0.shuffle(&mut rng);
    let mut A = mat![0; N; N];
    for i in 0..N {
        for j in 0..N {
            A[i][j] = A0[i * N + j];
        }
    }
    Input { N, A }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, &out.out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &[(usize, usize)]) -> (i64, String, ()) {
    let mut score = 0;
    let mut used = mat![false; input.N; input.N];
    for t in 0..out.len() {
        let (i, j) = out[t];
        if !used[i][j].setmax(true) {
            return (
                0,
                format!("Duplicate move at turn {}: ({}, {})", t, i, j),
                (),
            );
        }
        if t > 0 {
            let (pi, pj) = out[t - 1];
            if i.abs_diff(pi).max(j.abs_diff(pj)) != 1 {
                return (
                    0,
                    format!(
                        "Invalid move at turn {}: ({}, {}) -> ({}, {})",
                        t, pi, pj, i, j
                    ),
                    (),
                );
            }
        }
        score += t as i64 * input.A[i][j];
    }
    let N2 = (input.N * input.N) as i64;
    let err = if out.len() < N2 as usize {
        "Not finished".to_owned()
    } else {
        String::new()
    };
    ((score + N2 / 2) / N2, err, ())
}

pub mod vis;
pub use vis::*;
