#![allow(non_snake_case, unused_macros)]

use proconio::input;
use rand::prelude::*;
use std::ops::RangeBounds;
use svg::node::element::{Group, Rectangle, Style, Text, Title};

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
    /// V[i][j] = true iff there is a wall between (i,j) and (i,j+1)
    pub V: Vec<Vec<bool>>,
    /// H[i][j] = true iff there is a wall between (i,j) and (i+1,j)
    pub H: Vec<Vec<bool>>,
    pub A: Vec<Vec<usize>>,
    pub W: usize,
}

fn parse_wall_strings(rows: &[String], len: usize) -> Vec<Vec<bool>> {
    rows.iter()
        .map(|s| {
            assert_eq!(s.len(), len, "Invalid wall string length: {}", s);
            s.chars()
                .map(|ch| match ch {
                    '0' => false,
                    '1' => true,
                    _ => panic!("Invalid wall character: {}", ch),
                })
                .collect()
        })
        .collect()
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.N)?;
        for i in 0..self.N {
            for j in 0..self.N {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.A[i][j])?;
            }
            writeln!(f)?;
        }
        for i in 0..self.N {
            for j in 0..self.N - 1 {
                write!(f, "{}", if self.V[i][j] { '1' } else { '0' })?;
            }
            writeln!(f)?;
        }
        for i in 0..self.N - 1 {
            for j in 0..self.N {
                write!(f, "{}", if self.H[i][j] { '1' } else { '0' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        A: [[usize; N]; N],
        V_s: [String; N],
        H_s: [String; N - 1],
    }
    Input {
        N,
        V: parse_wall_strings(&V_s, N - 1),
        H: parse_wall_strings(&H_s, N),
        A,
        W: 0,
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

#[derive(Clone, Debug)]
pub struct Op {
    /// 'V' swaps the top and bottom halves, and 'H' swaps the left and right halves.
    pub d: char,
    pub r: usize,
    pub c: usize,
    pub h: usize,
    pub w: usize,
}

pub struct Output {
    pub out: Vec<Op>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut it = f.split_whitespace();
    let mut out = vec![];

    while let Some(d_s) = it.next() {
        let t = out.len();
        if t >= 100000 {
            return Err(format!("Too many operations: at least {}", t + 1));
        }

        let d = match d_s {
            "V" => 'V',
            "H" => 'H',
            _ => return Err(format!("operation {} d: expected V or H, got {}", t, d_s)),
        };
        let r: usize =
            read(it.next(), 0..input.N).map_err(|e| format!("operation {} r: {}", t, e))?;
        let c: usize =
            read(it.next(), 0..input.N).map_err(|e| format!("operation {} c: {}", t, e))?;
        let h: usize =
            read(it.next(), 1..=input.N).map_err(|e| format!("operation {} h: {}", t, e))?;
        let w: usize =
            read(it.next(), 1..=input.N).map_err(|e| format!("operation {} w: {}", t, e))?;

        out.push(Op { d, r, c, h, w });
    }

    Ok(Output { out })
}

fn h_edge_has_wall(N: usize, H: &[Vec<bool>], r: usize, c: usize) -> bool {
    // edge between vertices (r,c) and (r,c+1)
    if r == 0 || r == N {
        true
    } else {
        H[r - 1][c]
    }
}

fn v_edge_has_wall(N: usize, V: &[Vec<bool>], r: usize, c: usize) -> bool {
    // edge between vertices (r,c) and (r+1,c)
    if c == 0 || c == N {
        true
    } else {
        V[r][c - 1]
    }
}

fn vertex_has_wall(N: usize, V: &[Vec<bool>], H: &[Vec<bool>], r: usize, c: usize) -> bool {
    if c < N && h_edge_has_wall(N, H, r, c) {
        return true;
    }
    if c > 0 && h_edge_has_wall(N, H, r, c - 1) {
        return true;
    }
    if r < N && v_edge_has_wall(N, V, r, c) {
        return true;
    }
    if r > 0 && v_edge_has_wall(N, V, r - 1, c) {
        return true;
    }
    false
}

fn set_h_edge(N: usize, H: &mut [Vec<bool>], r: usize, c: usize) {
    // edge between vertices (r,c) and (r,c+1)
    if 0 < r && r < N {
        H[r - 1][c] = true;
    }
}

fn set_v_edge(N: usize, V: &mut [Vec<bool>], r: usize, c: usize) {
    // edge between vertices (r,c) and (r+1,c)
    if r < N && 0 < c && c < N {
        V[r][c - 1] = true;
    }
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);

    let N = 20usize;

    let mut V = mat![false; N; N - 1];
    let mut H = mat![false; N - 1; N];

    let wall_count = rng.gen_range(0..(N as u32)) as usize;

    for _ in 0..wall_count {
        let mut candidates = vec![];
        for r in 1..N {
            for c in 1..N {
                if !vertex_has_wall(N, &V, &H, r, c) {
                    candidates.push((r, c));
                }
            }
        }

        if candidates.is_empty() {
            break;
        }

        let id = rng.gen_range(0..(candidates.len() as u32)) as usize;
        let (mut r, mut c) = candidates[id];

        let dir = rng.gen_range(0..4u32);

        loop {
            let (nr, nc) = match dir {
                0 => (r - 1, c), // up
                1 => (r, c + 1), // right
                2 => (r + 1, c), // down
                _ => (r, c - 1), // left
            };

            let stop = vertex_has_wall(N, &V, &H, nr, nc);

            match dir {
                0 => set_v_edge(N, &mut V, nr, c),
                1 => set_h_edge(N, &mut H, r, c),
                2 => set_v_edge(N, &mut V, r, c),
                _ => set_h_edge(N, &mut H, r, nc),
            }

            r = nr;
            c = nc;

            if stop {
                break;
            }
        }
    }

    let P = loop {
        let mut P: Vec<usize> = (0..N * N).collect();
        P.shuffle(&mut rng);

        if P.iter().enumerate().any(|(i, &v)| i != v) {
            break P;
        }
    };

    let mut A = mat![0usize; N; N];
    for i in 0..N {
        for j in 0..N {
            A[i][j] = P[i * N + j];
        }
    }

    Input {
        N,
        V,
        H,
        A,
        W: wall_count,
    }
}

fn rect_inside(N: usize, r: usize, c: usize, h: usize, w: usize) -> bool {
    h >= 1 && w >= 1 && r < N && c < N && h <= N - r && w <= N - c
}

fn validate_op(input: &Input, op: &Op, turn: usize) -> Result<(), String> {
    let N = input.N;

    if !rect_inside(N, op.r, op.c, op.h, op.w) {
        return Err(format!(
            "Invalid operation at turn {}: rectangle is out of board",
            turn
        ));
    }

    match op.d {
        'V' if op.h % 2 != 0 => {
            return Err(format!(
                "Invalid operation at turn {}: height must be even for V",
                turn
            ))
        }
        'H' if op.w % 2 != 0 => {
            return Err(format!(
                "Invalid operation at turn {}: width must be even for H",
                turn
            ))
        }
        'V' | 'H' => {}
        _ => unreachable!(),
    }

    // Check vertical walls between horizontally adjacent cells inside R.
    for i in op.r..op.r + op.h {
        for j in op.c..op.c + op.w - 1 {
            if input.V[i][j] {
                return Err(format!(
                    "Invalid operation at turn {}: wall exists between ({},{}) and ({},{})",
                    turn,
                    i,
                    j,
                    i,
                    j + 1
                ));
            }
        }
    }

    // Check horizontal walls between vertically adjacent cells inside R.
    for i in op.r..op.r + op.h - 1 {
        for j in op.c..op.c + op.w {
            if input.H[i][j] {
                return Err(format!(
                    "Invalid operation at turn {}: wall exists between ({},{}) and ({},{})",
                    turn,
                    i,
                    j,
                    i + 1,
                    j
                ));
            }
        }
    }

    Ok(())
}

fn apply_op(B: &mut [Vec<usize>], op: &Op) {
    match op.d {
        'V' => {
            for x in 0..op.h / 2 {
                for y in 0..op.w {
                    let a = B[op.r + x][op.c + y];
                    let b = B[op.r + op.h / 2 + x][op.c + y];
                    B[op.r + x][op.c + y] = b;
                    B[op.r + op.h / 2 + x][op.c + y] = a;
                }
            }
        }
        'H' => {
            for x in 0..op.h {
                for y in 0..op.w / 2 {
                    let a = B[op.r + x][op.c + y];
                    let b = B[op.r + x][op.c + op.w / 2 + y];
                    B[op.r + x][op.c + y] = b;
                    B[op.r + x][op.c + op.w / 2 + y] = a;
                }
            }
        }
        _ => unreachable!(),
    }
}

fn count_errors(input: &Input, B: &[Vec<usize>]) -> usize {
    let mut E = 0usize;
    for i in 0..input.N {
        for j in 0..input.N {
            if B[i][j] != i * input.N + j {
                E += 1;
            }
        }
    }
    E
}

fn abs_score(input: &Input, B: &[Vec<usize>], T: usize) -> i64 {
    let E = count_errors(input, B);
    let base = (input.N * input.N) as i64;
    if E == 0 {
        // Valid inputs are not initially solved, so T is positive in this branch.
        if T == 0 {
            return base;
        }
        base + (1_000_000.0 * (100_000.0 / T as f64).log2()).round() as i64
    } else {
        base - E as i64
    }
}

fn simulate(input: &Input, out: &Output, t: usize) -> (i64, String, Vec<Vec<usize>>) {
    let mut B = input.A.clone();

    if out.out.len() > 100000 {
        return (0, format!("Too many operations: {}", out.out.len()), B);
    }

    if t > out.out.len() {
        return (
            0,
            format!(
                "t = {} exceeds the number of operations {}",
                t,
                out.out.len()
            ),
            B,
        );
    }

    for turn in 0..t {
        let op = &out.out[turn];
        if let Err(err) = validate_op(input, op, turn) {
            return (0, err, B);
        }
        apply_op(&mut B, op);
    }

    let score = abs_score(input, &B, t);
    (score, String::new(), B)
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out, compute_max_turn(input, out));
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &Output, t: usize) -> (i64, String, ()) {
    let (score, err, _) = simulate(input, out, t);
    (score, err, ())
}

/// 0 <= val <= 1
pub fn color(mut val: f64) -> String {
    val.setmin(1.0);
    val.setmax(0.0);
    let (r, g, b) = if val < 0.5 {
        let x = val * 2.0;
        (
            30. * (1.0 - x) + 144. * x,
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
        )
    } else {
        let x = val * 2.0 - 1.0;
        (
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
            30. * (1.0 - x) + 70. * x,
        )
    };
    // Blend the original color toward white so black card numbers remain readable.
    let lighten = 0.32;
    let r = r * (1.0 - lighten) + 255.0 * lighten;
    let g = g * (1.0 - lighten) + 255.0 * lighten;
    let b = b * (1.0 - lighten) + 255.0 * lighten;

    format!(
        "#{:02x}{:02x}{:02x}",
        r.round() as i32,
        g.round() as i32,
        b.round() as i32
    )
}

pub fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

pub fn group(title: String) -> Group {
    Group::new().add(Title::new(title))
}

pub fn vis_default(input: &Input, out: &Output) -> (i64, String, String) {
    let (mut score, err, svg) = vis(input, out, compute_max_turn(input, out));
    if err.len() > 0 {
        score = 0;
    }
    (score, err, svg)
}

pub fn vis(input: &Input, out: &Output, t: usize) -> (i64, String, String) {
    let canvas_w = 600;
    let canvas_h = 600;
    let N = input.N;
    let D = canvas_w as f64 / N as f64;

    let (score, err, B) = simulate(input, out, t);

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, canvas_w + 10, canvas_h + 10))
        .set("width", canvas_w + 10)
        .set("height", canvas_h + 10)
        .set("style", "background-color:white");

    doc = doc.add(Style::new(format!(
        "text {{text-anchor: middle;dominant-baseline: central;pointer-events: none;}}"
    )));

    for i in 0..N {
        for j in 0..N {
            let v = B[i][j];
            let target = i * N + j;
            let fill = if v == target {
                "#f8f8f8".to_string()
            } else {
                color(v as f64 / (N * N - 1) as f64)
            };

            let x = j as f64 * D;
            let y = i as f64 * D;

            let mut g = group(format!("cell=({},{}), card={}, target={}", i, j, v, target));

            g = g.add(
                Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", D)
                    .set("height", D)
                    .set("fill", fill)
                    .set("stroke", "#dddddd")
                    .set("stroke-width", 0.5),
            );

            if D >= 12.0 {
                g = g.add(
                    Text::new(format!("{}", v))
                        .set("x", x + D / 2.0)
                        .set("y", y + D / 2.0)
                        .set("font-size", (D * 0.36).max(4.0))
                        .set("fill", "black"),
                );
            }

            doc = doc.add(g);
        }
    }

    // Highlight the two halves swapped by the last operation.
    if t > 0 && t <= out.out.len() {
        let op = &out.out[t - 1];

        if rect_inside(N, op.r, op.c, op.h, op.w) {
            match op.d {
                'V' if op.h % 2 == 0 => {
                    doc = doc.add(
                        Rectangle::new()
                            .set("x", op.c as f64 * D)
                            .set("y", op.r as f64 * D)
                            .set("width", op.w as f64 * D)
                            .set("height", (op.h / 2) as f64 * D)
                            .set("fill", "#ff6666")
                            .set("fill-opacity", 0.25)
                            .set("stroke", "#ff0000")
                            .set("stroke-width", 2),
                    );
                    doc = doc.add(
                        Rectangle::new()
                            .set("x", op.c as f64 * D)
                            .set("y", (op.r + op.h / 2) as f64 * D)
                            .set("width", op.w as f64 * D)
                            .set("height", (op.h / 2) as f64 * D)
                            .set("fill", "#6666ff")
                            .set("fill-opacity", 0.25)
                            .set("stroke", "#0000ff")
                            .set("stroke-width", 2),
                    );
                }
                'H' if op.w % 2 == 0 => {
                    doc = doc.add(
                        Rectangle::new()
                            .set("x", op.c as f64 * D)
                            .set("y", op.r as f64 * D)
                            .set("width", (op.w / 2) as f64 * D)
                            .set("height", op.h as f64 * D)
                            .set("fill", "#ff6666")
                            .set("fill-opacity", 0.25)
                            .set("stroke", "#ff0000")
                            .set("stroke-width", 2),
                    );
                    doc = doc.add(
                        Rectangle::new()
                            .set("x", (op.c + op.w / 2) as f64 * D)
                            .set("y", op.r as f64 * D)
                            .set("width", (op.w / 2) as f64 * D)
                            .set("height", op.h as f64 * D)
                            .set("fill", "#6666ff")
                            .set("fill-opacity", 0.25)
                            .set("stroke", "#0000ff")
                            .set("stroke-width", 2),
                    );
                }
                _ => {
                    // Invalid parity is shown as one orange rectangle together with the error.
                    doc = doc.add(
                        Rectangle::new()
                            .set("x", op.c as f64 * D)
                            .set("y", op.r as f64 * D)
                            .set("width", op.w as f64 * D)
                            .set("height", op.h as f64 * D)
                            .set("fill", "#ffcc66")
                            .set("fill-opacity", 0.25)
                            .set("stroke", "#ff8800")
                            .set("stroke-width", 2),
                    );
                }
            }
        }
    }

    let wall_w = (D * 0.16).max(2.0);

    // Internal vertical walls V.
    for i in 0..N {
        for j in 0..N - 1 {
            if input.V[i][j] {
                doc = doc.add(
                    Rectangle::new()
                        .set("x", (j + 1) as f64 * D - wall_w / 2.0)
                        .set("y", i as f64 * D)
                        .set("width", wall_w)
                        .set("height", D)
                        .set("fill", "black"),
                );
            }
        }
    }

    // Internal horizontal walls H.
    for i in 0..N - 1 {
        for j in 0..N {
            if input.H[i][j] {
                doc = doc.add(
                    Rectangle::new()
                        .set("x", j as f64 * D)
                        .set("y", (i + 1) as f64 * D - wall_w / 2.0)
                        .set("width", D)
                        .set("height", wall_w)
                        .set("fill", "black"),
                );
            }
        }
    }

    // Outer wall.
    doc = doc.add(
        Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", canvas_w)
            .set("height", canvas_h)
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", wall_w),
    );

    (score, err, doc.to_string())
}

pub fn compute_max_turn(_input: &Input, out: &Output) -> usize {
    out.out.len()
}
