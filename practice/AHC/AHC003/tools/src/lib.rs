#![allow(non_snake_case, dead_code, unused_imports, unused_macros)]

use proconio::{input, marker::*};
use rand::prelude::*;
use std::io::prelude::*;
use svg::node::element::{path::Data, Circle, Path, Rectangle};

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

pub const N: usize = 30;
pub const Q: usize = 1000;

pub type Output = Vec<String>;

#[derive(Clone, Debug)]
pub struct Input {
    pub h: Vec<Vec<i32>>,
    pub v: Vec<Vec<i32>>,
    pub s: Vec<(usize, usize)>,
    pub t: Vec<(usize, usize)>,
    pub a: Vec<i32>,
    pub e: Vec<f64>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..N {
            for j in 0..N - 1 {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.h[i][j])?;
            }
            writeln!(f)?;
        }
        for i in 0..N - 1 {
            for j in 0..N {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.v[i][j])?;
            }
            writeln!(f)?;
        }
        for k in 0..Q {
            writeln!(
                f,
                "{} {} {} {} {} {}",
                self.s[k].0, self.s[k].1, self.t[k].0, self.t[k].1, self.a[k], self.e[k]
            )?;
        }
        Ok(())
    }
}

pub fn read_input_str(f: &str) -> Input {
    let mut f = proconio::source::once::OnceSource::from(f);
    input! {
        from &mut f,
        h: [[i32; N - 1]; N],
        v: [[i32; N]; N - 1],
    }
    let mut s = vec![];
    let mut t = vec![];
    let mut a = vec![];
    let mut e = vec![];
    for _ in 0..Q {
        input! {
            from &mut f,
            si: usize,
            sj: usize,
            ti: usize,
            tj: usize,
            ak: i32,
            ek: f64
        }
        s.push((si, sj));
        t.push((ti, tj));
        a.push(ak);
        e.push(ek);
    }
    Input { h, v, s, t, a, e }
}

pub fn read_output_str(_input: &Input, f: &str) -> Output {
    let mut outs = vec![];
    for line in f.lines() {
        let line = line.trim();
        if line.len() > 0 {
            outs.push(line.to_owned());
        }
    }
    outs
}

pub fn compute_score_detail(input: &Input, out: &Output) -> (i64, String) {
    let mut score = 0.0;
    let mut visited = mat![!0; N; N];
    for k in 0..Q {
        if k >= out.len() {
            return (0, "wrong number of outputs".to_owned());
        }
        match compute_path_length(input, k, &out[k], &mut visited) {
            Ok((b, _)) => {
                if input.a[k] > b {
                    return (0, "internal error".to_owned());
                }
                score = score * 0.998 + input.a[k] as f64 / b as f64;
            }
            Err(s) => return (0, s),
        }
    }
    ((score * 2312311.0).round() as i64, String::new())
}

pub fn compute_path_length(
    input: &Input,
    k: usize,
    path: &str,
    visited: &mut Vec<Vec<usize>>,
) -> Result<(i32, Vec<(usize, usize)>), String> {
    let mut p = input.s[k];
    let mut ps = vec![p];
    let mut sum = 0;
    for c in path.chars() {
        if visited[p.0][p.1] == k {
            return Err(format!(
                "visiting ({},{}) twice (query {})",
                p.0,
                p.1,
                k + 1
            ));
        }
        visited[p.0][p.1] = k;
        match c {
            'U' => {
                if p.0 == 0 {
                    return Err(format!("going outside the map (query {})", k + 1));
                }
                p.0 -= 1;
                sum += input.v[p.0][p.1];
            }
            'L' => {
                if p.1 == 0 {
                    return Err(format!("going outside the map (query {})", k + 1));
                }
                p.1 -= 1;
                sum += input.h[p.0][p.1];
            }
            'D' => {
                if p.0 == N - 1 {
                    return Err(format!("going outside the map (query {})", k + 1));
                }
                sum += input.v[p.0][p.1];
                p.0 += 1;
            }
            'R' => {
                if p.1 == N - 1 {
                    return Err(format!("going outside the map (query {})", k + 1));
                }
                sum += input.h[p.0][p.1];
                p.1 += 1;
            }
            _ => return Err(format!("unexpected char: {}", c)),
        }
        ps.push(p);
    }
    if p != input.t[k] {
        return Err(format!("not an s-t path (query {})", k + 1));
    }
    Ok((sum, ps))
}

fn dist(p: (usize, usize), q: (usize, usize)) -> usize {
    let di = if p.0 < q.0 { q.0 - p.0 } else { p.0 - q.0 };
    let dj = if p.1 < q.1 { q.1 - p.1 } else { p.1 - q.1 };
    di + dj
}

fn add(p: (usize, usize), q: (usize, usize)) -> Option<(usize, usize)> {
    let x = p.0 + q.0;
    let y = p.1 + q.1;
    if x < N && y < N {
        Some((x, y))
    } else {
        None
    }
}

const INF: i32 = 1000000000;
const DIJ: [(usize, usize); 4] = [(!0, 0), (0, !0), (1, 0), (0, 1)];
const DIR: [char; 4] = ['U', 'L', 'D', 'R'];

fn length(hs: &Vec<Vec<i32>>, vs: &Vec<Vec<i32>>, p: (usize, usize), dir: usize) -> i32 {
    match dir {
        0 => vs[p.0 - 1][p.1],
        1 => hs[p.0][p.1 - 1],
        2 => vs[p.0][p.1],
        3 => hs[p.0][p.1],
        _ => unreachable!(),
    }
}

pub fn compute_shortest_path(
    hs: &Vec<Vec<i32>>,
    vs: &Vec<Vec<i32>>,
    s: (usize, usize),
    t: (usize, usize),
) -> (String, i32) {
    let mut dist = mat![INF; N; N];
    let mut prev = mat![!0; N; N];
    let mut que = std::collections::BinaryHeap::new();
    dist[s.0][s.1] = 0;
    que.push((0, s));
    while let Some((d, p)) = que.pop() {
        let d = -d;
        if p == t {
            break;
        }
        if dist[p.0][p.1] != d {
            continue;
        }
        for dir in 0..4 {
            if let Some(q) = add(p, DIJ[dir]) {
                if dist[q.0][q.1].setmin(d + length(hs, vs, p, dir)) {
                    prev[q.0][q.1] = dir;
                    que.push((-dist[q.0][q.1], q));
                }
            }
        }
    }
    let mut p = t;
    let mut out = String::new();
    while p != s {
        let dir = prev[p.0][p.1];
        out.push(DIR[dir]);
        p = add(p, DIJ[(dir + 2) % 4]).unwrap();
    }
    let out = out.chars().rev().collect::<String>();
    (out, dist[t.0][t.1])
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let D = rng.gen_range(100, 2001);
    let M = rng.gen_range(1, 3u32) as usize;
    let H = (0..N)
        .map(|_| {
            (0..M)
                .map(|_| rng.gen_range(1000 + D, 9001 - D))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut x = vec![vec![]; N];
    for i in 0..N {
        x[i].push(0);
        if M == 2 {
            x[i].push(rng.gen_range(1, N as u32 - 1) as usize);
        }
        x[i].push(N - 1);
    }
    let mut h = mat![0; N; N - 1];
    for i in 0..N {
        for p in 0..M {
            for j in x[i][p]..x[i][p + 1] {
                h[i][j] = H[i][p] + rng.gen_range(-D, D + 1);
            }
        }
    }
    let V = (0..N)
        .map(|_| {
            (0..M)
                .map(|_| rng.gen_range(1000 + D, 9001 - D))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut y = vec![vec![]; N];
    for j in 0..N {
        y[j].push(0);
        if M == 2 {
            y[j].push(rng.gen_range(1, N as u32 - 1) as usize);
        }
        y[j].push(N - 1);
    }
    let mut v = mat![0; N - 1; N];
    for j in 0..N {
        for p in 0..M {
            for i in y[j][p]..y[j][p + 1] {
                v[i][j] = V[j][p] + rng.gen_range(-D, D + 1);
            }
        }
    }
    let mut s = vec![];
    let mut t = vec![];
    let mut a = vec![];
    let mut e = vec![];
    for _ in 0..Q {
        let mut sk = (0, 0);
        let mut tk = (0, 0);
        while dist(sk, tk) < 10 {
            sk = (
                rng.gen_range(0, N as u32) as usize,
                rng.gen_range(0, N as u32) as usize,
            );
            tk = (
                rng.gen_range(0, N as u32) as usize,
                rng.gen_range(0, N as u32) as usize,
            );
        }
        s.push(sk);
        t.push(tk);
        a.push(compute_shortest_path(&h, &v, sk, tk).1);
        e.push(rng.gen_range(0.9, 1.1));
    }
    Input { h, v, s, t, a, e }
}

fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

fn text(x: usize, y: usize, size: usize, s: &str) -> svg::node::element::Text {
    svg::node::element::Text::new()
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("text-anchor", "middle")
        .add(svg::node::Text::new(s))
}

// 0 <= val <= 1
fn color(val: f64) -> String {
    let tmp = ((-(2.0 * std::f64::consts::PI * val).cos() / 2.0 + 0.5) * 255.0) as i32;
    if val >= 0.5 {
        format!("#{:02x}{:02x}{:02x}", 255, 0, tmp)
    } else {
        format!("#{:02x}{:02x}{:02x}", tmp, 0, 255)
    }
}

pub fn vis(
    input: &Input,
    out: &Output,
    show_map: bool,
    show_k: usize,
    show_edge_length: bool,
) -> (i64, String, String) {
    let (score, mut err) = compute_score_detail(input, out);
    const S: usize = 30;
    const H: usize = 300;
    let height = if show_map { 20 + N * S } else { 0 };
    let mut doc = svg::Document::new()
        .set("viewBox", (0, 0, 20 + N * S + 20, height + H))
        .set("width", 20 + N * S + 20)
        .set("height", height + H);
    doc = doc.add(rect(0, 0, 20 + N * S + 20, height + H, "white"));
    if show_map {
        for i in 0..N {
            doc = doc.add(text(16, 20 + S * i + S / 2 + 7, 16, &format!("{}", i)));
            doc = doc.add(text(20 + S * i + S / 2, 24, 16, &format!("{}", i)));
        }
        for i in 0..N {
            for j in 0..N {
                if j + 1 < N {
                    let data = Data::new()
                        .move_to((20 + S * j + S / 2, 20 + S * i + S / 2))
                        .line_by((S, 0));
                    let path = Path::new()
                        .set("stroke", color((input.h[i][j] as f64 - 1000.0) / 8000.0))
                        .set("stroke-width", 3)
                        .set("d", data);
                    doc = doc.add(path);
                }
                if i + 1 < N {
                    let data = Data::new()
                        .move_to((20 + S * j + S / 2, 20 + S * i + S / 2))
                        .line_by((0, S));
                    let path = Path::new()
                        .set("stroke", color((input.v[i][j] as f64 - 1000.0) / 8000.0))
                        .set("stroke-width", 3)
                        .set("d", data);
                    doc = doc.add(path);
                }
            }
        }
        if show_k < Q {
            let (path_a, a) =
                compute_shortest_path(&input.h, &input.v, input.s[show_k], input.t[show_k]);
            let path_a = compute_path_length(&input, show_k, &path_a, &mut mat![!0; N; N])
                .unwrap()
                .1;
            if err.len() > 0 {
                err += "<br>";
            }
            err += &format!(" <font color='forestgreen'>a = {}</font>", a);
            let mut data = Data::new().move_to((
                20 + S * path_a[0].1 + S / 2 + 8,
                20 + S * path_a[0].0 + S / 2 + 8,
            ));
            for p in 1..path_a.len() {
                let (i, j) = path_a[p - 1];
                let (i2, j2) = path_a[p];
                let di = i2 as i32 - i as i32;
                let dj = j2 as i32 - j as i32;
                data = data.line_by((dj * S as i32, di * S as i32));
            }
            let path = Path::new()
                .set("fill", "none")
                .set("stroke", "forestgreen")
                .set("stroke-width", 6)
                .set("d", data);
            doc = doc.add(path);
            if show_k < out.len() {
                if let Ok((b, path_b)) =
                    compute_path_length(&input, show_k, &out[show_k], &mut mat![!0; N; N])
                {
                    err += &format!(" <font color='chocolate'>b={}</font>", b);
                    let mut data = Data::new().move_to((
                        20 + S * path_b[0].1 + S / 2 - 8,
                        20 + S * path_b[0].0 + S / 2 - 8,
                    ));
                    for p in 1..path_b.len() {
                        let (i, j) = path_b[p - 1];
                        let (i2, j2) = path_b[p];
                        let di = i2 as i32 - i as i32;
                        let dj = j2 as i32 - j as i32;
                        data = data.line_by((dj * S as i32, di * S as i32));
                    }
                    let path = Path::new()
                        .set("fill", "none")
                        .set("stroke", "chocolate")
                        .set("stroke-width", 6)
                        .set("d", data);
                    doc = doc.add(path);
                }
            }
        }
        if show_edge_length {
            for i in 0..N {
                for j in 0..N {
                    if j + 1 < N {
                        doc = doc.add(text(
                            20 + S * j + S,
                            20 + S * i + S / 2 - 3,
                            9,
                            &format!("{}", input.h[i][j]),
                        ));
                    }
                    if i + 1 < N {
                        doc = doc.add(text(
                            20 + S * j + S - 3,
                            20 + S * i + S,
                            9,
                            &format!("{}", input.v[i][j]),
                        ));
                    }
                }
            }
        }
        for i in 0..N {
            for j in 0..N {
                let circle = Circle::new()
                    .set("cx", 20 + i * S + S / 2)
                    .set("cy", 20 + j * S + S / 2)
                    .set("r", 5)
                    .set("fill", "black");
                doc = doc.add(circle);
            }
        }
    }
    doc = doc.add(
        rect(40, height + 20, S * N - 40, H - 60, "none")
            .set("stroke", "black")
            .set("stroke-width", 2),
    );
    doc = doc.add(text(20, height + H / 2, 20, "a/b"));
    doc = doc.add(text(20, height + 30, 20, "1"));
    doc = doc.add(text(20, height + H - 30, 20, "0"));
    doc = doc.add(text(20 + S * N / 2, height + H - 15, 20, "k"));
    doc = doc.add(text(40, height + H - 15, 20, "1"));
    doc = doc.add(text(
        20 + S * N - S / 2,
        height + H - 15,
        20,
        &Q.to_string(),
    ));
    let mut used = mat![!0; N; N];
    for k in 0..Q.min(out.len()) {
        if let Ok((b, _)) = compute_path_length(&input, k, &out[k], &mut used) {
            let ab = input.a[k] as f64 / b as f64;
            let circle = Circle::new()
                .set("cx", 40 + (S * N - 40) * k / (Q - 1))
                .set(
                    "cy",
                    height + H - 40 - ((H - 60) as f64 * ab).round() as usize,
                )
                .set("r", if k != show_k { 3 } else { 6 })
                .set("fill", if k != show_k { "red" } else { "blue" })
                .set("fill-opacity", 0.5);
            doc = doc.add(circle);
        }
    }
    (score, doc.to_string(), err)
}
