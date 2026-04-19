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
    pub K: usize,
    pub C: usize,
    pub g: Vec<Vec<usize>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.N, self.K, self.C)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.g[i].iter().join(" "))?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize, K: usize, C: usize,
        g: [[usize; N]; N],
    }
    Input { N, K, C, g }
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

#[derive(Clone, Debug, Copy)]
pub enum Action {
    Paint {
        k: usize,
        i: usize,
        j: usize,
        c: usize,
    },
    Copy {
        k: usize,
        h: usize,
        r: usize,
        di: i32,
        dj: i32,
    },
    Clear {
        k: usize,
    },
}

pub struct Output {
    pub out: Vec<Action>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut out = vec![];
    for line in f.lines() {
        let line = line.trim();
        if line.len() == 0 {
            continue;
        }
        let mut ss = line.split_whitespace();
        let ty = read(ss.next(), 0..=2)?;
        let a = if ty == 0 {
            Action::Paint {
                k: read(ss.next(), 0..input.K)?,
                i: read(ss.next(), 0..input.N)?,
                j: read(ss.next(), 0..input.N)?,
                c: read(ss.next(), 0..=input.C)?,
            }
        } else if ty == 1 {
            Action::Copy {
                k: read(ss.next(), 0..input.K)?,
                h: read(ss.next(), 0..input.K)?,
                r: read(ss.next(), 0..4)?,
                di: read(ss.next(), -(input.N as i32) + 1..=(input.N as i32) - 1)?,
                dj: read(ss.next(), -(input.N as i32) + 1..=(input.N as i32) - 1)?,
            }
        } else {
            Action::Clear {
                k: read(ss.next(), 0..input.K)?,
            }
        };
        if ss.next().is_some() {
            return Err(format!("Extra token: {}", line));
        }
        out.push(a);
        if out.len() > input.N * input.N {
            return Err(format!("Too many actions"));
        }
    }
    Ok(Output { out })
}

fn copy(ck: &Vec<Vec<usize>>, ch: &Vec<Vec<usize>>, di: i32, dj: i32) -> Option<Vec<Vec<usize>>> {
    let N = ck.len();
    let mut ck = ck.clone();
    for i in 0..N {
        for j in 0..N {
            if ch[i][j] != 0 {
                let i2 = (i as i32 + di) as usize;
                let j2 = (j as i32 + dj) as usize;
                if i2 >= N || j2 >= N {
                    return None;
                }
                ck[i2][j2] = ch[i][j];
            }
        }
    }
    Some(ck)
}

fn rot(ch: &Vec<Vec<usize>>, r: usize) -> Vec<Vec<usize>> {
    let N = ch.len();
    let mut ch = ch.clone();
    for _ in 0..r {
        let mut ch2 = mat![0; N; N];
        for i in 0..N {
            for j in 0..N {
                ch2[i][j] = ch[N - 1 - j][i];
            }
        }
        ch = ch2;
    }
    ch
}

fn bb(ch: &Vec<Vec<usize>>) -> (usize, usize, usize, usize) {
    let N = ch.len();
    let mut min_i = N;
    let mut max_i = 0;
    let mut min_j = N;
    let mut max_j = 0;
    for i in 0..N {
        for j in 0..N {
            if ch[i][j] != 0 {
                min_i.setmin(i);
                max_i.setmax(i);
                min_j.setmin(j);
                max_j.setmax(j);
            }
        }
    }
    (min_i, max_i, min_j, max_j)
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 32;
    let K = rng.gen_range(2..=5i32) as usize;
    let C = rng.gen_range(2..=4i32) as usize;
    let K2 = rng.gen_range(1..=K as i32 * 2) as usize;
    let mut c = mat![0; K2; N; N];
    c[0][N / 2][N / 2] = 1;
    let mut size = vec![0; K2];
    size[0] = 1;
    const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];
    fn check(c: &Vec<Vec<usize>>) -> bool {
        let N = c.len();
        let mut visited = mat![false; N; N];
        let mut first = false;
        for i in 0..N {
            for j in 0..N {
                if c[i][j] != 0 && !visited[i][j] {
                    if first {
                        return false;
                    }
                    first = true;
                    let mut stack = vec![(i, j)];
                    visited[i][j] = true;
                    while let Some((i, j)) = stack.pop() {
                        for (di, dj) in DIJ {
                            let i2 = i + di;
                            let j2 = j + dj;
                            if i2 < N && j2 < N && c[i2][j2] != 0 && visited[i2][j2].setmax(true) {
                                stack.push((i2, j2));
                            }
                        }
                    }
                }
            }
        }
        true
    }
    let p = rng.gen_range(2..=5);
    let g = loop {
        let k = rng.gen_range(0..K2 as i32) as usize;
        if rng.gen_range(0..10) < p {
            loop {
                let (i, j) = if size[k] == 0 {
                    (N / 2, N / 2)
                } else {
                    let i = rng.gen_range(0..N as i32) as usize;
                    let j = rng.gen_range(0..N as i32) as usize;
                    (i, j)
                };
                let mut num = vec![0; C + 1];
                for k in 0..K2 {
                    for i in 0..N {
                        for j in 0..N {
                            num[c[k][i][j]] += 1;
                        }
                    }
                }
                let min = *num[1..].iter().min().unwrap();
                let d = (1..=C).filter(|&x| num[x] == min).choose(&mut rng).unwrap();
                let mut ck = c[k].clone();
                ck[i][j] = d;
                if check(&ck) {
                    c[k] = ck;
                    break;
                }
            }
        } else {
            loop {
                let h = (0..K2).filter(|&x| size[x] > 0).choose(&mut rng).unwrap();
                let r = rng.gen_range(0..4i32) as usize;
                let ch = rot(&c[h], r);
                let (min_i, max_i, min_j, max_j) = bb(&ch);
                let di = rng.gen_range(-(min_i as i32)..=(N - max_i) as i32 - 1);
                let dj = rng.gen_range(-(min_j as i32)..=(N - max_j) as i32 - 1);
                if let Some(ck) = copy(&c[k], &ch, di, dj) {
                    if check(&ck) {
                        c[k] = ck;
                        break;
                    }
                }
            }
        }
        size[k] = c[k].iter().flatten().filter(|&&x| x != 0).count();
        if size[k] >= N * N / 2 {
            break c[k].clone();
        }
    };
    Input { N, K, C, g }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, &out.out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &[Action]) -> (i64, String, Vec<Vec<Vec<usize>>>) {
    let mut c = mat![0; input.K; input.N; input.N];
    for &a in out {
        match a {
            Action::Paint { k, i, j, c: cp } => {
                c[k][i][j] = cp;
            }
            Action::Copy { k, h, r, di, dj } => {
                let ch = rot(&c[h], r);
                if let Some(ck) = copy(&c[k], &ch, di, dj) {
                    c[k] = ck;
                } else {
                    return (
                        0,
                        format!(
                            "Invalid copy: k={}, h={}, r={}, di={}, dj={}",
                            k, h, r, di, dj
                        ),
                        c,
                    );
                }
            }
            Action::Clear { k } => {
                c[k] = mat![0; input.N; input.N];
            }
        }
    }
    if c[0] == input.g {
        let U = input.g.iter().flatten().filter(|&&x| x != 0).count() as f64;
        let score = 1e6 * (1.0 + f64::log2(U / out.len() as f64));
        (score.round() as i64, String::new(), c)
    } else {
        (0, "Not finished".to_owned(), c)
    }
}
