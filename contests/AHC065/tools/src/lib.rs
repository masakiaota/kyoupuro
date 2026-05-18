#![allow(non_snake_case, unused_macros)]

use itertools::Itertools;
use proconio::input;
use rand::prelude::*;
use std::ops::RangeBounds;
use svg::node::element::{Group, Path, Rectangle, Style, Text, Title};

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
    a: Vec<Vec<i32>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.N)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.a[i].iter().join(" "))?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        a: [[i32; N]; N],
    }
    Input { N, a }
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
    pub conveyers: Vec<Vec<(usize, usize)>>,
    pub out: Vec<(usize, i32)>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut f = f.split_whitespace();
    let M = read(f.next(), 0..=input.N * input.N)?;
    let mut conveyers = vec![];
    let mut used = mat![!0; input.N; input.N];
    for m in 0..M {
        let l = read(f.next(), 2..=input.N * input.N)?;
        let mut ij = vec![];
        for _ in 0..l {
            let i = read(f.next(), 0..input.N)?;
            let j = read(f.next(), 0..input.N)?;
            ij.push((i, j));
            if used[i][j] == m {
                return Err(format!(
                    "({},{}) is contained twice in the {}-th conveyer belt",
                    i, j, m
                ));
            }
            used[i][j] = m;
        }
        for x in 0..l {
            let p = ij[x];
            let q = ij[(x + 1) % l];
            if p.0.abs_diff(q.0) + p.1.abs_diff(q.1) != 1 {
                return Err(format!(
                    "({},{}) and ({},{}) are not adjacent",
                    p.0, p.1, q.0, q.1
                ));
            }
        }
        conveyers.push(ij);
    }
    let mut out = vec![];
    let T = read(f.next(), 0..=100000)?;
    for _ in 0..T {
        let m = read(f.next(), 0..M)?;
        let d = read(f.next(), -1..=1)?;
        if d == 0 {
            return Err(format!("Invalid direction: {}", d));
        }
        out.push((m, d));
    }
    if let Some(v) = f.next() {
        return Err(format!("Unexpected token: {}", v));
    }
    Ok(Output { conveyers, out })
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 20;
    let mut a0 = (0..(N * N) as i32).collect_vec();
    a0.shuffle(&mut rng);
    let mut a = mat![0; N; N];
    for i in 0..N {
        for j in 0..N {
            a[i][j] = a0[i * N + j];
        }
    }
    Input { N, a }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out, out.out.len());
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(
    input: &Input,
    out: &Output,
    t: usize,
) -> (i64, String, Vec<Vec<i32>>) {
    let mut a = input.a.clone();
    let mut target = 0;
    if a[0][input.N / 2] == 0 {
        a[0][input.N / 2] = -1;
        target += 1;
    }
    let mut cnt = mat![0; input.N; input.N];
    for m in 0..out.conveyers.len() {
        for &(i, j) in &out.conveyers[m] {
            cnt[i][j] += 1;
            if cnt[i][j] > 2 {
                return (
                    0,
                    format!("({},{}) is contained in more than two conveyer belts", i, j),
                    a,
                );
            }
        }
    }
    for &(m, d) in &out.out[..t] {
        let ij = &out.conveyers[m];
        if d > 0 {
            let mut v = a[ij[0].0][ij[0].1];
            for x in 1..ij.len() {
                std::mem::swap(&mut a[ij[x].0][ij[x].1], &mut v);
            }
            a[ij[0].0][ij[0].1] = v;
        } else {
            let v = a[ij[0].0][ij[0].1];
            for x in 0..ij.len() - 1 {
                a[ij[x].0][ij[x].1] = a[ij[x + 1].0][ij[x + 1].1];
            }
            a[ij[ij.len() - 1].0][ij[ij.len() - 1].1] = v;
        }
        if a[0][input.N / 2] == target {
            a[0][input.N / 2] = -1;
            target += 1;
        }
    }
    let score = if target == (input.N * input.N) as i32 {
        1000000 + f64::round(1e6 * f64::log2(1e5 / t as f64)) as i64
    } else {
        f64::round(1e6 * target as f64 / (input.N * input.N) as f64) as i64
    };
    (score, String::new(), a)
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
    let (mut score, err, svg) = vis(input, &out, out.out.len(), true);
    if err.len() > 0 {
        score = 0;
    }
    (score, err, svg)
}

fn color_rgb(mut val: f64) -> (f64, f64, f64) {
    val.setmin(1.0);
    val.setmax(0.0);
    if val < 0.5 {
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
    }
}

fn box_text_color(n: usize, v: i32) -> &'static str {
    let denom = ((n * n).saturating_sub(1)).max(1) as f64;
    let (r, g, b) = color_rgb(v as f64 / denom);
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    if y < 150.0 {
        "white"
    } else {
        "black"
    }
}

fn cell_center(i: usize, j: usize, ox: f64, oy: f64, cell: f64) -> (f64, f64) {
    (ox + (j as f64 + 0.5) * cell, oy + (i as f64 + 0.5) * cell)
}

fn conveyor_path(ij: &[(usize, usize)], ox: f64, oy: f64, cell: f64) -> String {
    let l = ij.len();
    if l == 0 {
        return String::new();
    }

    let r = cell * 0.28;

    let center = |p: (usize, usize)| cell_center(p.0, p.1, ox, oy, cell);

    let mut d = String::new();

    let p0 = ij[0];
    let p1 = ij[1 % l];
    let c0 = center(p0);
    let dir0 = (
        (p1.1 as isize - p0.1 as isize) as f64,
        (p1.0 as isize - p0.0 as isize) as f64,
    );
    let start = (c0.0 + dir0.0 * r, c0.1 + dir0.1 * r);
    d.push_str(&format!("M {:.3} {:.3}", start.0, start.1));

    for k in 1..=l {
        let i = k % l;
        let prev = ij[(i + l - 1) % l];
        let cur = ij[i];
        let next = ij[(i + 1) % l];

        let cc = center(cur);
        let dir_in = (
            (cur.1 as isize - prev.1 as isize) as f64,
            (cur.0 as isize - prev.0 as isize) as f64,
        );
        let dir_out = (
            (next.1 as isize - cur.1 as isize) as f64,
            (next.0 as isize - cur.0 as isize) as f64,
        );

        let entry = (cc.0 - dir_in.0 * r, cc.1 - dir_in.1 * r);
        let exit = (cc.0 + dir_out.0 * r, cc.1 + dir_out.1 * r);

        d.push_str(&format!(
            " L {:.3} {:.3} Q {:.3} {:.3} {:.3} {:.3}",
            entry.0, entry.1, cc.0, cc.1, exit.0, exit.1
        ));
    }
    d.push_str(" Z");
    d
}

fn next_target_from_board(a: &[Vec<i32>], n: usize) -> usize {
    let mut present = vec![false; n * n];
    for i in 0..n {
        for j in 0..n {
            let v = a[i][j];
            if v >= 0 {
                present[v as usize] = true;
            }
        }
    }
    for x in 0..n * n {
        if present[x] {
            return x;
        }
    }
    n * n
}

pub fn vis(input: &Input, out: &Output, t: usize, show_number: bool) -> (i64, String, String) {
    let t = t.min(out.out.len());

    let cell = 35.0;
    let ox = 20.0;
    let oy = 20.0;
    let board_w = input.N as f64 * cell;
    let board_h = input.N as f64 * cell;

    let W = (ox * 2.0 + board_w) as i32;
    let H = (oy * 2.0 + board_h) as i32;

    let (score, err, a) = compute_score_details(input, &out, t);
    let next_target = next_target_from_board(&a, input.N);

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, W + 10, H + 10))
        .set("width", W + 10)
        .set("height", H + 10)
        .set("style", "background-color:white");

    doc = doc.add(Style::new(
        r#"
text {
  text-anchor: middle;
  dominant-baseline: central;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
}
"#
        .to_owned(),
    ));

    for i in 0..input.N {
        for j in 0..input.N {
            let mut fill = "#fbfbfb";
            let mut stroke = "#d8d8d8";
            let mut stroke_width = 1.0;

            if i == 0 && j == input.N / 2 {
                fill = "#fff2bf";
                stroke = "#cc9800";
                stroke_width = 2.5;
            }

            let mut g = group(format!("cell ({},{})", i, j));
            g = g.add(
                Rectangle::new()
                    .set("x", ox + j as f64 * cell)
                    .set("y", oy + i as f64 * cell)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", fill)
                    .set("stroke", stroke)
                    .set("stroke-width", stroke_width),
            );
            doc = doc.add(g);
        }
    }

    let exit_cx = ox + (input.N as f64 / 2.0 + 0.5) * cell;
    let exit_cy = oy + 0.18 * cell;
    doc = doc.add(
        Text::new("OUT")
            .set("x", exit_cx)
            .set("y", exit_cy)
            .set("font-size", cell * 0.24)
            .set("font-weight", "bold")
            .set("fill", "#8a5a00"),
    );

    let last_used = if t > 0 { Some(out.out[t - 1].0) } else { None };

    let add_conveyor = |mut doc: svg::Document, m: usize, highlight: bool| -> svg::Document {
        let ij = &out.conveyers[m];
        let d = conveyor_path(ij, ox, oy, cell);

        let outer;
        let inner;
        let outer_w;
        let inner_w;
        let dash;
        let opacity_outer;
        let opacity_inner;

        if highlight {
            outer = "#8a6f3d";
            inner = "#fff0b3";
            outer_w = cell * 0.43;
            inner_w = cell * 0.17;
            dash = format!("{:.2} {:.2}", cell * 0.32, cell * 0.22);
            opacity_outer = 0.88;
            opacity_inner = 0.95;
        } else {
            outer = "#5b6470";
            inner = "#d7dde6";
            outer_w = cell * 0.38;
            inner_w = cell * 0.15;
            dash = format!("{:.2} {:.2}", cell * 0.28, cell * 0.22);
            opacity_outer = 0.75;
            opacity_inner = 0.9;
        }

        let mut g = group(if highlight {
            format!("conveyor {} [last used at turn {}]", m, t - 1)
        } else {
            format!("conveyor {}", m)
        });

        g = g.add(
            Path::new()
                .set("d", d.clone())
                .set("fill", "none")
                .set("stroke", outer)
                .set("stroke-width", outer_w)
                .set("stroke-linecap", "round")
                .set("stroke-linejoin", "round")
                .set("opacity", opacity_outer),
        );

        g = g.add(
            Path::new()
                .set("d", d)
                .set("fill", "none")
                .set("stroke", inner)
                .set("stroke-width", inner_w)
                .set("stroke-linecap", "round")
                .set("stroke-linejoin", "round")
                .set("stroke-dasharray", dash)
                .set("opacity", opacity_inner),
        );

        doc = doc.add(g);
        doc
    };

    for m in 0..out.conveyers.len() {
        if Some(m) != last_used {
            doc = add_conveyor(doc, m, false);
        }
    }

    if let Some(m) = last_used {
        doc = add_conveyor(doc, m, true);
    }

    let box_size = cell * 0.76;
    let box_margin = (cell - box_size) * 0.5;
    let number_font_size = cell * 0.35;

    for i in 0..input.N {
        for j in 0..input.N {
            let v = a[i][j];
            if v < 0 {
                continue;
            }

            let denom = ((input.N * input.N).saturating_sub(1)).max(1) as f64;
            let fill = color(v as f64 / denom);
            let is_target = next_target < input.N * input.N && v as usize == next_target;

            let x = ox + j as f64 * cell + box_margin;
            let y = oy + i as f64 * cell + box_margin;

            let mut g = group(if is_target {
                format!("box {} at ({}, {}) [NEXT]", v, i, j)
            } else {
                format!("box {} at ({}, {})", v, i, j)
            });

            if is_target {
                g = g.add(
                    Rectangle::new()
                        .set("x", x - 2.5)
                        .set("y", y - 2.5)
                        .set("width", box_size + 5.0)
                        .set("height", box_size + 5.0)
                        .set("fill", "none")
                        .set("stroke", "#e60026")
                        .set("stroke-width", 3.0),
                );
            }

            g = g.add(
                Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", box_size)
                    .set("height", box_size)
                    .set("fill", fill)
                    .set("stroke", if is_target { "#a0001a" } else { "#333333" })
                    .set("stroke-width", if is_target { 2.0 } else { 1.0 }),
            );

            if show_number {
                g = g.add(
                    Text::new(format!("{}", v))
                        .set("x", x + box_size / 2.0)
                        .set("y", y + box_size / 2.0)
                        .set("font-size", number_font_size)
                        .set("font-weight", if is_target { "bold" } else { "normal" })
                        .set("fill", box_text_color(input.N, v)),
                );
            }

            doc = doc.add(g);
        }
    }

    (score, err, doc.to_string())
}
