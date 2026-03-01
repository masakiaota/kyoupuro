#![allow(non_snake_case, unused_macros)]

use proconio::{input, marker::Chars};
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
    AK: i64,
    AM: i64,
    AW: i64,
    wall_v: Vec<Vec<char>>,
    wall_h: Vec<Vec<char>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {} {}", self.N, self.AK, self.AM, self.AW)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.wall_v[i].iter().collect::<String>())?;
        }
        for i in 0..self.N - 1 {
            writeln!(f, "{}", self.wall_h[i].iter().collect::<String>())?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        AK: i64,
        AM: i64,
        AW: i64,
        wall_v: [Chars; N],
        wall_h: [Chars; N - 1],
    }
    Input {
        N,
        AK,
        AM,
        AW,
        wall_v,
        wall_h,
    }
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

const DIR: [char; 4] = ['U', 'R', 'D', 'L'];
const DIJ: [(usize, usize); 4] = [(!0, 0), (0, 1), (1, 0), (0, !0)];

#[derive(Clone, Debug)]
pub struct Robot {
    m: usize,
    i: usize,
    j: usize,
    d: usize,
    a0: Vec<char>,
    b0: Vec<usize>,
    a1: Vec<char>,
    b1: Vec<usize>,
}

pub struct Output {
    pub robots: Vec<Robot>,
    pub wall_v: Vec<Vec<char>>,
    pub wall_h: Vec<Vec<char>>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut f = f.split_whitespace();
    let K = read(f.next(), 1..=input.N * input.N)?;
    let mut robots = vec![];
    for _ in 0..K {
        let m = read(f.next(), 1..=4 * input.N * input.N)?;
        let i = read(f.next(), 0..input.N)?;
        let j = read(f.next(), 0..input.N)?;
        let d = read(f.next(), 'A'..='Z')?;
        let Some(d) = DIR.iter().position(|&c| c == d) else {
            return Err(format!("Invalid direction: {}", d));
        };
        let mut a0 = vec![];
        let mut b0 = vec![];
        let mut a1 = vec![];
        let mut b1 = vec![];
        for _ in 0..m {
            let a = read(f.next(), 'A'..='Z')?;
            if !['R', 'L', 'F'].contains(&a) {
                return Err(format!("Invalid action: {}", a));
            }
            a0.push(a);
            b0.push(read(f.next(), 0..m)?);
            let a = read(f.next(), 'A'..='Z')?;
            if !['R', 'L'].contains(&a) {
                return Err(format!("Invalid action: {}", a));
            }
            a1.push(a);
            b1.push(read(f.next(), 0..m)?);
        }
        robots.push(Robot {
            m,
            i,
            j,
            d,
            a0,
            b0,
            a1,
            b1,
        });
    }
    let mut wall_v = vec![];
    for _ in 0..input.N {
        let Some(line) = f.next() else {
            return Err("Unexpected EOF".to_owned());
        };
        let line = line.trim();
        if line.len() != input.N - 1 {
            return Err(format!("Invalid wall_v line: {}", line));
        }
        if line.chars().any(|c| c != '0' && c != '1') {
            return Err(format!("Invalid wall_v line: {}", line));
        }
        wall_v.push(line.chars().collect());
    }
    let mut wall_h = vec![];
    for _ in 0..input.N - 1 {
        let Some(line) = f.next() else {
            return Err("Unexpected EOF".to_owned());
        };
        let line = line.trim();
        if line.len() != input.N {
            return Err(format!("Invalid wall_h line: {}", line));
        }
        if line.chars().any(|c| c != '0' && c != '1') {
            return Err(format!("Invalid wall_h line: {}", line));
        }
        wall_h.push(line.chars().collect());
    }
    if f.next().is_some() {
        return Err("Extra tokens".to_owned());
    }
    Ok(Output {
        robots,
        wall_v,
        wall_h,
    })
}

fn has_wall(wall_v: &[Vec<char>], wall_h: &[Vec<char>], i: usize, j: usize, d: usize) -> bool {
    let N = wall_v.len();
    let i2 = i + DIJ[d].0;
    let j2 = j + DIJ[d].1;
    if i2 >= N || j2 >= N {
        return true;
    }
    if i == i2 {
        wall_v[i][j.min(j2)] == '1'
    } else {
        wall_h[i.min(i2)][j] == '1'
    }
}

pub fn gen(seed: u64, problem: &str) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 20;
    let (AK, AM, AW) = match problem {
        "A" => (0, 1, 1000),
        "B" => (1000, rng.gen_range(1..=10), rng.gen_range(1..=10)),
        "C" => (1000, 1, 1000),
        _ => panic!("invalid problem id: {}", problem),
    };
    let X = rng.gen_range(1..=6i32);
    let Y = rng.gen_range(1..=6i32);
    loop {
        let mut wall_v = mat!['0'; N; N - 1];
        let mut wall_h = mat!['0'; N - 1; N];
        for _ in 0..X {
            let dir = rng.gen_range(0..2i32);
            let L = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..N as i32) as usize;
            let j = rng.gen_range(0..N as i32 - 1) as usize;
            if dir == 0 {
                for i in (i + 1).saturating_sub(L)..=i {
                    wall_v[i][j] = '1';
                }
            } else {
                for i in i..=(i + L - 1).min(N - 1) {
                    wall_v[i][j] = '1';
                }
            }
        }
        for _ in 0..Y {
            let dir = rng.gen_range(0..2i32);
            let L = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..N as i32 - 1) as usize;
            let j = rng.gen_range(0..N as i32) as usize;
            if dir == 0 {
                for j in (j + 1).saturating_sub(L)..=j {
                    wall_h[i][j] = '1';
                }
            } else {
                for j in j..=(j + L - 1).min(N - 1) {
                    wall_h[i][j] = '1';
                }
            }
        }
        let mut visited = mat![false; N; N];
        let mut stack = vec![(0, 0)];
        visited[0][0] = true;
        let mut num = 0;
        while let Some((i, j)) = stack.pop() {
            num += 1;
            for d in 0..4 {
                if !has_wall(&wall_v, &wall_h, i, j, d) {
                    let i2 = i + DIJ[d].0;
                    let j2 = j + DIJ[d].1;
                    if visited[i2][j2].setmax(true) {
                        stack.push((i2, j2));
                    }
                }
            }
        }
        if num == N * N {
            return Input {
                N,
                AK,
                AM,
                AW,
                wall_v,
                wall_h,
            };
        }
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Route {
    head: Vec<(usize, usize)>,
    tail: Vec<(usize, usize)>,
}

pub fn compute_score_details(
    input: &Input,
    out: &Output,
) -> (i64, String, (Vec<Vec<bool>>, Vec<Route>)) {
    let mut routes = vec![];
    for robot in &out.robots {
        let mut visited = mat![!0; input.N; input.N; 4; robot.m];
        let mut route = vec![];
        let mut i = robot.i;
        let mut j = robot.j;
        let mut d = robot.d;
        let mut s = 0;
        for t in 0.. {
            route.push((i, j));
            if !visited[i][j][d][s].setmin(t) {
                let head = route[..=visited[i][j][d][s]].to_vec();
                let tail = route[visited[i][j][d][s]..].to_vec();
                routes.push(Route { head, tail });
                break;
            }
            let (a, b) = if has_wall(&input.wall_v, &input.wall_h, i, j, d)
                || has_wall(&out.wall_v, &out.wall_h, i, j, d)
            {
                (robot.a1[s], robot.b1[s])
            } else {
                (robot.a0[s], robot.b0[s])
            };
            match a {
                'R' => d = (d + 1) % 4,
                'L' => d = (d + 3) % 4,
                'F' => {
                    i += DIJ[d].0;
                    j += DIJ[d].1;
                }
                _ => unreachable!(),
            }
            s = b;
        }
    }
    let mut checked = mat![false; input.N; input.N];
    let mut num = 0;
    for k in 0..routes.len() {
        for &(i, j) in &routes[k].tail {
            if checked[i][j].setmax(true) {
                num += 1;
            }
        }
    }
    if num != input.N * input.N {
        return (
            0,
            format!("Not all cells are patrolled: {}", num),
            (checked, routes),
        );
    }
    let K = out.robots.len() as i64;
    let M = out.robots.iter().map(|r| r.m).sum::<usize>() as i64;
    let W = out
        .wall_v
        .iter()
        .map(|r| r.iter().filter(|&&c| c == '1').count())
        .sum::<usize>() as i64
        + out
            .wall_h
            .iter()
            .map(|r| r.iter().filter(|&&c| c == '1').count())
            .sum::<usize>() as i64;
    let V = input.AK * (K - 1) + input.AM * M + input.AW * W;
    let mut score = 1;
    let base = input.AK * (input.N * input.N - 1) as i64 + input.AM * (input.N * input.N) as i64;
    if V < base {
        score += (1e6 * (base as f64 / V as f64).log2()).round() as i64;
    }
    (score, String::new(), (checked, routes))
}
