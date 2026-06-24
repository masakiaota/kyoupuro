#![allow(non_snake_case, unused_macros)]

use proconio::{input, marker::Chars};
use rand::prelude::*;
use std::collections::{HashSet, VecDeque};
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

const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];

#[derive(Clone, Debug)]
pub struct Input {
    pub N: usize,
    pub M: usize,
    pub K: usize,
    pub c: Vec<Vec<char>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.N, self.M, self.K)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.c[i].iter().collect::<String>())?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize, M: usize, K: usize,
        c: [Chars; N],
    }
    Input { N, M, K, c }
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

#[derive(Clone, Debug)]
pub struct Door {
    pub d: usize,
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

#[derive(Clone, Debug)]
pub struct Switch {
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub doors: Vec<Door>,
    pub switches: Vec<Switch>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut f = f.split_whitespace();
    let D = read(f.next(), 0..=input.M)?;
    let mut doors = vec![];
    let mut used = HashSet::new();
    for _ in 0..D {
        let d = read(f.next(), 0..=1)?;
        let (i, j) = if d == 0 {
            (read(f.next(), 0..input.N - 1)?, read(f.next(), 0..input.N)?)
        } else {
            (read(f.next(), 0..input.N)?, read(f.next(), 0..input.N - 1)?)
        };
        if !used.insert((d, i, j)) {
            return Err(format!("Duplicate door: {} {} {}", d, i, j));
        }
        let k = read(f.next(), 0..2 * input.K)?;
        doors.push(Door { d, i, j, k });
    }
    let S = read(f.next(), 0..=input.N * input.N)?;
    let mut switches = vec![];
    let mut used = HashSet::new();
    for _ in 0..S {
        let i = read(f.next(), 0..input.N)?;
        let j = read(f.next(), 0..input.N)?;
        let k = read(f.next(), 0..input.K)?;
        if !used.insert((i, j)) {
            return Err(format!("Duplicate switch: {} {}", i, j));
        }
        switches.push(Switch { i, j, k });
    }
    if f.next().is_some() {
        return Err("Extra tokens".to_owned());
    }
    Ok(Output { doors, switches })
}

fn is_connected(c: &Vec<Vec<char>>) -> bool {
    let n = c.len();
    let mut visited = mat![false; n; n];
    let mut stack = vec![(0, 0)];
    visited[0][0] = true;
    while let Some((i, j)) = stack.pop() {
        for (di, dj) in DIJ {
            let i2 = i + di;
            let j2 = j + dj;
            if i2 < n && j2 < n && c[i2][j2] == '.' && visited[i2][j2].setmax(true) {
                stack.push((i2, j2));
            }
        }
    }
    for i in 0..n {
        for j in 0..n {
            if c[i][j] == '.' && !visited[i][j] {
                return false;
            }
        }
    }
    true
}

pub fn gen(seed: u64) -> Input {
    let N = 20;
    let M = 50;
    let K = 10;
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut c = mat!['.'; N; N];
    let mut ij = vec![];
    for i in 0..N {
        for j in 0..N {
            if (i, j) != (0, 0) && (i, j) != (N - 1, N - 1) {
                ij.push((i, j));
            }
        }
    }
    ij.shuffle(&mut rng);
    for &(i, j) in &ij {
        let mut cnt = 0;
        for (di, dj) in DIJ {
            let i2 = i + di;
            let j2 = j + dj;
            if i2 >= N || j2 >= N || c[i2][j2] == '#' {
                cnt += 1;
            }
        }
        if cnt >= 3 {
            continue;
        }
        c[i][j] = '#';
        if !is_connected(&c) {
            c[i][j] = '.';
        }
    }
    ij.shuffle(&mut rng);
    let mut cnt = 0;
    for &(i, j) in &ij {
        if c[i][j] == '.' {
            continue;
        }
        let mut ok = false;
        for (di, dj) in DIJ {
            let i2 = i + di;
            let j2 = j + dj;
            if i2 < N && j2 < N && c[i2][j2] == '.' {
                ok = true;
                break;
            }
        }
        if ok {
            c[i][j] = '.';
            cnt += 1;
            if cnt >= N {
                break;
            }
        }
    }
    Input { N, M, K, c }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out);
    if !err.is_empty() {
        score = 0;
    }
    (score, err)
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    i: usize,
    j: usize,
    mask: usize,
}

fn is_open(g: usize, mask: usize) -> bool {
    if g == !0 {
        true
    } else {
        let k = g / 2;
        (mask >> k) & 1 == g & 1
    }
}

pub fn compute_score_details(input: &Input, out: &Output) -> (i64, String, Vec<State>) {
    let mut door_h = mat![!0; input.N - 1; input.N];
    let mut door_v = mat![!0; input.N; input.N - 1];
    for door in &out.doors {
        if door.d == 0 {
            door_h[door.i][door.j] = door.k;
        } else {
            door_v[door.i][door.j] = door.k;
        }
    }
    let mut switch = mat![!0; input.N; input.N];
    for sw in &out.switches {
        switch[sw.i][sw.j] = sw.k;
    }
    let mut trace = Trace::new();
    let mut visited = mat![false; input.N; input.N; 1 << input.K];
    let mut que = VecDeque::new();
    let s = State {
        i: 0,
        j: 0,
        mask: 0,
    };
    visited[0][0][0] = true;
    que.push_back((s, trace.add(s, !0)));
    let mut states = vec![s];
    while let Some((s, id)) = que.pop_front() {
        if (s.i, s.j) == (input.N - 1, input.N - 1) {
            states = trace.get(id);
            break;
        }
        for (di, dj) in DIJ {
            let i = s.i + di;
            let j = s.j + dj;
            if i < input.N && j < input.N && input.c[i][j] == '.' {
                let g = if s.i == i {
                    door_v[s.i][s.j.min(j)]
                } else {
                    door_h[s.i.min(i)][s.j]
                };
                if is_open(g, s.mask) && visited[i][j][s.mask].setmax(true) {
                    let s2 = State { i, j, mask: s.mask };
                    que.push_back((s2, trace.add(s2, id)));
                }
            }
        }
        if switch[s.i][s.j] != !0 {
            let mask = s.mask ^ (1 << switch[s.i][s.j]);
            if visited[s.i][s.j][mask].setmax(true) {
                let s2 = State {
                    i: s.i,
                    j: s.j,
                    mask,
                };
                que.push_back((s2, trace.add(s2, id)));
            }
        }
    }
    let T = states.len() as i64 - 1;
    let score = if T == 0 {
        1
    } else {
        (1e6 * (T as f64 / input.N as f64).log2()).round() as i64
    };
    (score, String::new(), states)
}

pub struct Trace<T: Copy> {
    log: Vec<(T, u32)>,
}

impl<T: Copy> Trace<T> {
    pub fn new() -> Self {
        Trace { log: vec![] }
    }
    pub fn add(&mut self, c: T, p: u32) -> u32 {
        self.log.push((c, p));
        self.log.len() as u32 - 1
    }
    pub fn get(&self, mut i: u32) -> Vec<T> {
        let mut out = vec![];
        while i != !0 {
            out.push(self.log[i as usize].0);
            i = self.log[i as usize].1;
        }
        out.reverse();
        out
    }
}

pub mod vis;
pub use vis::*;
