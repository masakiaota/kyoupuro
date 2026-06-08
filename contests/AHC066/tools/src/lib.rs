#![allow(non_snake_case, unused_macros)]

use proconio::{input, marker::Chars};
use rand::prelude::*;
use std::{collections::VecDeque, ops::RangeBounds};
use svg::node::element::{Circle, Group, Line, Polygon, Rectangle, Style, Text, Title};
use svg::node::Blob;

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
    pub T: usize,
    pub wall_v: Vec<Vec<bool>>, // N x (N-1), between (i,j) and (i,j+1)
    pub wall_h: Vec<Vec<bool>>, // (N-1) x N, between (i,j) and (i+1,j)
    pub balls: Vec<(usize, usize)>,
    pub baskets: Vec<(usize, usize)>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.N, self.M, self.T)?;
        for v in &self.wall_v {
            writeln!(
                f,
                "{}",
                v.iter()
                    .map(|&x| if x { '1' } else { '0' })
                    .collect::<String>()
            )?;
        }
        for h in &self.wall_h {
            writeln!(
                f,
                "{}",
                h.iter()
                    .map(|&x| if x { '1' } else { '0' })
                    .collect::<String>()
            )?;
        }
        for k in 0..self.M {
            let (b, c) = self.balls[k];
            let (d, e) = self.baskets[k];
            writeln!(f, "{} {} {} {}", b, c, d, e)?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize, M: usize, T: usize,
        wall_v: [Chars; N],
        wall_h: [Chars; N - 1],
        bcde: [(usize, usize, usize, usize); M],
    }
    let wall_v = wall_v
        .into_iter()
        .map(|s| s.into_iter().map(|c| c == '1').collect())
        .collect();
    let wall_h = wall_h
        .into_iter()
        .map(|s| s.into_iter().map(|c| c == '1').collect())
        .collect();
    let mut balls = vec![];
    let mut baskets = vec![];
    for (b, c, d, e) in bcde {
        balls.push((b, c));
        baskets.push((d, e));
    }
    Input {
        N,
        M,
        T,
        wall_v,
        wall_h,
        balls,
        baskets,
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
pub struct Output {
    pub ops: Vec<char>, // raw button sequence: F/R/L/S/M/P
}

pub const BUTTONS: [char; 6] = ['F', 'R', 'L', 'S', 'M', 'P'];
pub const BASIC: [char; 4] = ['F', 'R', 'L', 'S'];
pub const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)]; // U,R,D,L

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut ops = vec![];
    for token in f.split_whitespace() {
        for c in token.chars() {
            if !BUTTONS.contains(&c) {
                return Err(format!("Invalid operation: {}", c));
            }
            ops.push(c);
        }
    }
    if ops.len() > input.T {
        return Err(format!(
            "Too many button operations: {} > {}",
            ops.len(),
            input.T
        ));
    }
    Ok(Output { ops })
}

fn can_move(input: &Input, (i, j): (usize, usize), dir: usize) -> bool {
    let (di, dj) = DIJ[dir];
    let i2 = i as isize + di;
    let j2 = j as isize + dj;
    if i2 < 0 || i2 >= input.N as isize || j2 < 0 || j2 >= input.N as isize {
        return false;
    }
    let i2 = i2 as usize;
    let j2 = j2 as usize;
    if di == 0 {
        !input.wall_v[i][j.min(j2)]
    } else {
        !input.wall_h[i.min(i2)][j]
    }
}

fn next_pos(input: &Input, p: (usize, usize), dir: usize) -> (usize, usize) {
    if !can_move(input, p, dir) {
        return p;
    }
    let (di, dj) = DIJ[dir];
    ((p.0 as isize + di) as usize, (p.1 as isize + dj) as usize)
}

#[derive(Clone, Debug)]
pub struct ExpandedOp {
    pub op: char,
    pub src: usize, // index in Output::ops.  For P expansion, this is the P button index.
}

pub fn expand_output(input: &Input, out: &Output) -> Vec<ExpandedOp> {
    let mut expanded = vec![];
    let mut last_macro: Vec<char> = vec![];
    let mut cur_macro: Vec<char> = vec![];
    let mut recording = false;
    for (idx, &c) in out.ops.iter().enumerate() {
        if expanded.len() >= input.T {
            break;
        }
        match c {
            'F' | 'R' | 'L' | 'S' => {
                expanded.push(ExpandedOp { op: c, src: idx });
                if recording {
                    cur_macro.push(c);
                }
            }
            'M' => {
                if recording {
                    last_macro = cur_macro.clone();
                    cur_macro.clear();
                    recording = false;
                } else {
                    cur_macro.clear();
                    recording = true;
                }
            }
            'P' => {
                for &b in &last_macro {
                    if expanded.len() >= input.T {
                        break;
                    }
                    expanded.push(ExpandedOp { op: b, src: idx });
                    if recording {
                        cur_macro.push(b);
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    expanded
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub basic_turn: usize,
    pub p: (usize, usize),
    pub dir: usize, // 0:U, 1:R, 2:D, 3:L
    pub hand: Option<usize>,
    pub balls: Vec<Vec<Option<usize>>>,
    pub expanded: Vec<ExpandedOp>,
}

impl Sim {
    pub fn new(input: &Input, out: &Output) -> Self {
        let mut balls = mat![None; input.N; input.N];
        for k in 0..input.M {
            let (i, j) = input.balls[k];
            balls[i][j] = Some(k);
        }
        Sim {
            basic_turn: 0,
            p: (0, 0),
            dir: 1,
            hand: None,
            balls,
            expanded: expand_output(input, out),
        }
    }

    pub fn step_basic(&mut self, input: &Input, op: char) {
        match op {
            'F' => self.p = next_pos(input, self.p, self.dir),
            'R' => self.dir = (self.dir + 1) % 4,
            'L' => self.dir = (self.dir + 3) % 4,
            'S' => {
                let cell = &mut self.balls[self.p.0][self.p.1];
                std::mem::swap(&mut self.hand, cell);
            }
            _ => unreachable!(),
        }
        self.basic_turn += 1;
    }

    pub fn run_until(&mut self, input: &Input, t: usize) {
        let limit = t.min(self.expanded.len()).min(input.T);
        while self.basic_turn < limit {
            let op = self.expanded[self.basic_turn].op;
            self.step_basic(input, op);
        }
    }

    pub fn count_correct(&self, input: &Input) -> usize {
        let mut v = 0;
        for k in 0..input.M {
            let (i, j) = input.baskets[k];
            if self.balls[i][j] == Some(k) {
                v += 1;
            }
        }
        v
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out, input.T);
    if !err.is_empty() {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &Output, t: usize) -> (i64, String, Sim) {
    if out.ops.len() > input.T {
        let sim = Sim::new(input, out);
        return (
            0,
            format!(
                "Too many button operations: {} > {}",
                out.ops.len(),
                input.T
            ),
            sim,
        );
    }
    let mut sim = Sim::new(input, out);
    sim.run_until(input, t);
    let V = sim.count_correct(input);
    let score = if V == input.M {
        out.ops.len() as i64
    } else {
        (input.T * (input.M - V)) as i64
    };
    (score, String::new(), sim)
}

pub fn compute_X(
    N: usize,
    wall_v: &Vec<Vec<bool>>,
    wall_h: &Vec<Vec<bool>>,
    points: &Vec<(usize, usize)>,
) -> i32 {
    let mut x = 0i32;
    for k in 0..points.len() - 1 {
        let mut visited = mat![false; N; N];
        let mut que = VecDeque::new();
        que.push_back((points[k], 0));
        visited[points[k].0][points[k].1] = true;
        while let Some(((i, j), d)) = que.pop_front() {
            if (i, j) == points[k + 1] {
                x += d;
                break;
            }
            for dir in 0..4 {
                let (di, dj) = DIJ[dir];
                let i2 = i as isize + di;
                let j2 = j as isize + dj;
                if i2 < 0 || i2 >= N as isize || j2 < 0 || j2 >= N as isize {
                    continue;
                }
                let i2 = i2 as usize;
                let j2 = j2 as usize;
                if visited[i2][j2] {
                    continue;
                }
                let ok = if di == 0 {
                    !wall_v[i][j.min(j2)]
                } else {
                    !wall_h[i.min(i2)][j]
                };
                if !ok {
                    continue;
                }
                visited[i2][j2] = true;
                que.push_back(((i2, j2), d + 1));
            }
        }
    }
    x
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = rng.gen_range(10i32..=20) as usize;
    let W = rng.gen_range(0..N as i32) as usize;
    let mut wall_v = mat![false; N; N - 1];
    let mut wall_h = mat![false; N - 1; N];
    let mut used = mat![false; N + 1; N + 1];
    for i in 0..=N {
        for j in 0..=N {
            if i == 0 || j == 0 || i == N || j == N {
                used[i][j] = true;
            }
        }
    }
    for _ in 0..W {
        let mut free = vec![];
        for i in 1..N {
            for j in 1..N {
                if !used[i][j] {
                    free.push((i, j));
                }
            }
        }
        if free.is_empty() {
            break;
        }
        let (i, j) = *free.choose(&mut rng).unwrap();
        let dir = rng.gen_range(0..4usize);
        let mut len = 0;
        let mut pi = i;
        let mut pj = j;
        loop {
            len += 1;
            pi = (pi as isize + DIJ[dir].0) as usize;
            pj = (pj as isize + DIJ[dir].1) as usize;
            if used[pi][pj] {
                break;
            }
        }
        match dir {
            0 => {
                for d in 0..len {
                    wall_v[i - d - 1][j - 1] = true;
                    used[i - d][j] = true;
                }
            }
            2 => {
                for d in 0..len {
                    wall_v[i + d][j - 1] = true;
                    used[i + d][j] = true;
                }
            }
            3 => {
                for d in 0..len {
                    wall_h[i - 1][j - d - 1] = true;
                    used[i][j - d] = true;
                }
            }
            1 => {
                for d in 0..len {
                    wall_h[i - 1][j + d] = true;
                    used[i][j + d] = true;
                }
            }
            _ => unreachable!(),
        }
    }

    let u: f64 = rng.gen_range(0.0..1.0);
    let M = ((N as f64 / 2.0) * (4.0f64).powf(u)).round() as usize;
    let mut cells = vec![];
    for i in 0..N {
        for j in 0..N {
            cells.push((i, j));
        }
    }
    cells.shuffle(&mut rng);
    let balls = cells[..M].to_vec();
    let baskets = cells[M..2 * M].to_vec();

    let mut points = vec![(0, 0)];
    for k in 0..M {
        points.push(balls[k]);
        points.push(baskets[k]);
    }
    let X = compute_X(N, &wall_v, &wall_h, &points) as f64;
    let r: f64 = rng.gen_range(0.0..1.0);
    let lo = 2.0 * X + 4.0 * M as f64;
    let hi = 2.0 * (N * N * M) as f64;
    let T = (lo.powf(r) * hi.powf(1.0 - r)).round() as usize;

    Input {
        N,
        M,
        T,
        wall_v,
        wall_h,
        balls,
        baskets,
    }
}

pub fn text_color_for_background(r: f64, g: f64, b: f64) -> &'static str {
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if luminance > 150.0 {
        "#000000"
    } else {
        "#ffffff"
    }
}

/// 0 <= val <= 1
pub fn color(mut val: f64) -> (String, &'static str) {
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
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            r.round() as i32,
            g.round() as i32,
            b.round() as i32
        ),
        text_color_for_background(r, g, b),
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
    let (mut score, err, svg) = vis(input, out, input.T, true, true);
    if !err.is_empty() {
        score = 0;
    }
    (score, err, svg)
}

fn snippet(chars: &[char], idx: Option<usize>, radius: usize) -> String {
    if chars.is_empty() {
        return "(empty)".to_owned();
    }
    let center = idx
        .unwrap_or(chars.len().saturating_sub(1))
        .min(chars.len().saturating_sub(1));
    let l = center.saturating_sub(radius);
    let r = (center + radius + 1).min(chars.len());
    let mut s = String::new();
    if l > 0 {
        s.push_str("...");
    }
    for i in l..r {
        if Some(i) == idx {
            s.push('[');
            s.push(chars[i]);
            s.push(']');
        } else {
            s.push(chars[i]);
        }
    }
    if r < chars.len() {
        s.push_str("...");
    }
    s
}

const TEXT_COLOR: &str = "#222222";
const MACRO_COLOR: &str = "#dc2626";
const PLAY_COLOR: &str = "#2563eb";
const MACRO_PLAY_COLOR: &str = "#9333ea";

fn raw_button_colors(ops: &[char]) -> Vec<&'static str> {
    let mut colors = vec![TEXT_COLOR; ops.len()];
    let mut recording_start = None;

    for (i, &c) in ops.iter().enumerate() {
        if c != 'M' {
            continue;
        }
        if let Some(start) = recording_start {
            for color in &mut colors[start..=i] {
                *color = MACRO_COLOR;
            }
            recording_start = None;
        } else {
            recording_start = Some(i);
        }
    }
    if let Some(start) = recording_start {
        for color in &mut colors[start..] {
            *color = MACRO_COLOR;
        }
    }

    for (i, &c) in ops.iter().enumerate() {
        if c != 'P' {
            continue;
        }
        colors[i] = if colors[i] == MACRO_COLOR {
            MACRO_PLAY_COLOR
        } else {
            PLAY_COLOR
        };
    }

    colors
}

fn escape_svg_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn push_tspan(raw: &mut String, fill: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    raw.push_str(r#"<tspan fill=""#);
    raw.push_str(fill);
    raw.push_str(r#"">"#);
    raw.push_str(&escape_svg_text(text));
    raw.push_str("</tspan>");
}

fn push_snippet_part(parts: &mut Vec<(&'static str, String)>, fill: &'static str, text: String) {
    if text.is_empty() {
        return;
    }
    if let Some((last_fill, last_text)) = parts.last_mut() {
        if *last_fill == fill {
            last_text.push_str(&text);
            return;
        }
    }
    parts.push((fill, text));
}

fn styled_snippet_text(
    label: &str,
    chars: &[char],
    idx: Option<usize>,
    radius: usize,
    colors: &[&'static str],
    y: usize,
) -> Blob {
    let mut parts = vec![(TEXT_COLOR, label.to_owned())];

    if chars.is_empty() {
        push_snippet_part(&mut parts, TEXT_COLOR, "(empty)".to_owned());
    } else {
        let center = idx
            .unwrap_or(chars.len().saturating_sub(1))
            .min(chars.len().saturating_sub(1));
        let l = center.saturating_sub(radius);
        let r = (center + radius + 1).min(chars.len());

        if l > 0 {
            push_snippet_part(&mut parts, TEXT_COLOR, "...".to_owned());
        }
        for i in l..r {
            let part = if Some(i) == idx {
                format!("[{}]", chars[i])
            } else {
                chars[i].to_string()
            };
            push_snippet_part(
                &mut parts,
                colors.get(i).copied().unwrap_or(TEXT_COLOR),
                part,
            );
        }
        if r < chars.len() {
            push_snippet_part(&mut parts, TEXT_COLOR, "...".to_owned());
        }
    }

    let mut raw = format!(
        r#"<text x="5" y="{y}" font-size="14" class="left">"#
    );
    for (fill, text) in parts {
        push_tspan(&mut raw, fill, &text);
    }
    raw.push_str("</text>");
    Blob::new(raw)
}

pub fn registered_macro_at_turn(input: &Input, out: &Output, t: usize) -> Vec<char> {
    // t 個の基本操作を実行済みの時点での「最後に登録が完了したマクロ」を返す。
    // M や空の P など、基本操作を発生しないボタンは、次の基本操作の直前まで処理される。
    let target = t.min(input.T);

    let mut basic_turn = 0usize;
    let mut last_macro: Vec<char> = vec![];
    let mut cur_macro: Vec<char> = vec![];
    let mut recording = false;

    for &c in &out.ops {
        if basic_turn >= input.T {
            break;
        }

        match c {
            'F' | 'R' | 'L' | 'S' => {
                if basic_turn == target {
                    break;
                }
                basic_turn += 1;
                if recording {
                    cur_macro.push(c);
                }
                if basic_turn >= input.T {
                    break;
                }
            }
            'M' => {
                if recording {
                    last_macro = cur_macro.clone();
                    cur_macro.clear();
                    recording = false;
                } else {
                    cur_macro.clear();
                    recording = true;
                }
            }
            'P' => {
                let replay = last_macro.clone();
                for &b in &replay {
                    if basic_turn >= input.T {
                        return last_macro;
                    }
                    if basic_turn == target {
                        return last_macro;
                    }

                    basic_turn += 1;
                    if recording {
                        cur_macro.push(b);
                    }

                    if basic_turn >= input.T {
                        return last_macro;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    last_macro
}

pub fn vis(
    input: &Input,
    out: &Output,
    t: usize,
    show_number: bool,
    _show_targets: bool,
) -> (i64, String, String) {
    // t 個の基本操作を実行した状態、すなわち t 回目(0-indexed)の基本操作の直前の状態を描く。
    let (score, err, sim) = compute_score_details(input, out, t);

    let D = 600 / input.N;
    let board_w = input.N * D;
    let top_h = 78usize;
    let W = board_w.max(1220);
    let H = top_h + board_w;

    let expanded_chars = sim.expanded.iter().map(|e| e.op).collect::<Vec<_>>();
    let cur_expanded = if t < sim.expanded.len() {
        Some(t)
    } else {
        None
    };
    let cur_raw = cur_expanded.map(|x| sim.expanded[x].src);
    let current_from_play = cur_expanded
        .and_then(|x| out.ops.get(sim.expanded[x].src))
        == Some(&'P');

    let macro_chars = registered_macro_at_turn(input, out, t);

    let raw_colors = raw_button_colors(&out.ops);
    let expanded_colors = sim
        .expanded
        .iter()
        .map(|op| {
            if out.ops.get(op.src) == Some(&'P') {
                PLAY_COLOR
            } else {
                TEXT_COLOR
            }
        })
        .collect::<Vec<_>>();
    let macro_line = format!("macro   : {}", snippet(&macro_chars, None, 60));

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, W + 10, H + 10))
        .set("width", W + 10)
        .set("height", H + 10)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
        "text { font-family: monospace; dominant-baseline: middle; } .center { text-anchor: middle; } .left { text-anchor: start; }",
    ));

    doc = doc.add(styled_snippet_text(
        "buttons : ",
        &out.ops,
        cur_raw,
        60,
        &raw_colors,
        14,
    ));
    doc = doc.add(styled_snippet_text(
        "expanded: ",
        &expanded_chars,
        cur_expanded,
        60,
        &expanded_colors,
        32,
    ));
    doc = doc.add(
        Text::new(macro_line)
            .set("x", 5)
            .set("y", 50)
            .set("font-size", 14)
            .set("class", "left")
            .set("fill", TEXT_COLOR),
    );

    let y0 = top_h;
    let mut basket_id = mat![None; input.N; input.N];
    for k in 0..input.M {
        let (i, j) = input.baskets[k];
        basket_id[i][j] = Some(k);
    }

    for i in 0..input.N {
        for j in 0..input.N {
            let title = format!(
                "({}, {})\nball={:?}\nbasket={:?}",
                i, j, sim.balls[i][j], basket_id[i][j]
            );
            let mut g = group(title);
            let fill = match (sim.balls[i][j], basket_id[i][j]) {
                (Some(b), Some(c)) if b == c => "#e5e7eb",
                (Some(_), Some(_)) => "#ffe4d6",
                (None, Some(_)) => "#eef3ff",
                (Some(_), None) => "#fff7d6",
                (None, None) => "#ffffff",
            };
            g = g.add(rect(j * D, y0 + i * D, D, D, fill));
            if let Some(k) = basket_id[i][j] {
                g = g.add(
                    Rectangle::new()
                        .set("x", j * D + D / 5)
                        .set("y", y0 + i * D + D / 5)
                        .set("width", D * 3 / 5)
                        .set("height", D * 3 / 5)
                        .set("fill", "none")
                        .set("stroke", "#4169e1")
                        .set("stroke-width", 2),
                );
                if show_number {
                    g = g.add(
                        Text::new(format!("{}", k))
                            .set("x", j * D + D / 2)
                            .set("y", y0 + i * D + D * 2 / 3)
                            .set("font-size", (D / 4).max(8))
                            .set("class", "center")
                            .set("fill", "#4169e1"),
                    );
                }
            }
            if let Some(k) = sim.balls[i][j] {
                let (bg, fg) = color(k as f64 / input.M.max(1) as f64);
                g = g.add(
                    Circle::new()
                        .set("cx", j * D + D / 2)
                        .set("cy", y0 + i * D + D / 2)
                        .set("r", D * 3 / 10)
                        .set("fill", bg)
                        .set("stroke", "#333333")
                        .set("stroke-width", 1),
                );
                if show_number {
                    g = g.add(
                        Text::new(format!("{}", k))
                            .set("x", j * D + D / 2)
                            .set("y", y0 + i * D + D / 2)
                            .set("font-size", (D / 3).max(8))
                            .set("class", "center")
                            .set("fill", fg),
                    );
                }
            }
            doc = doc.add(g);
        }
    }

    for i in 0..=input.N {
        doc = doc.add(
            Line::new()
                .set("x1", 0)
                .set("y1", y0 + i * D)
                .set("x2", input.N * D)
                .set("y2", y0 + i * D)
                .set(
                    "stroke",
                    if i == 0 || i == input.N {
                        "black"
                    } else {
                        "lightgray"
                    },
                )
                .set("stroke-width", if i == 0 || i == input.N { 2 } else { 1 }),
        );
        doc = doc.add(
            Line::new()
                .set("x1", i * D)
                .set("y1", y0)
                .set("x2", i * D)
                .set("y2", y0 + input.N * D)
                .set(
                    "stroke",
                    if i == 0 || i == input.N {
                        "black"
                    } else {
                        "lightgray"
                    },
                )
                .set("stroke-width", if i == 0 || i == input.N { 2 } else { 1 }),
        );
    }
    for i in 0..input.N {
        for j in 0..input.N {
            if j + 1 < input.N && input.wall_v[i][j] {
                doc = doc.add(
                    Line::new()
                        .set("x1", j * D + D)
                        .set("y1", y0 + i * D)
                        .set("x2", j * D + D)
                        .set("y2", y0 + (i + 1) * D)
                        .set("stroke", "black")
                        .set("stroke-width", 3),
                );
            }
            if i + 1 < input.N && input.wall_h[i][j] {
                doc = doc.add(
                    Line::new()
                        .set("x1", j * D)
                        .set("y1", y0 + i * D + D)
                        .set("x2", (j + 1) * D)
                        .set("y2", y0 + i * D + D)
                        .set("stroke", "black")
                        .set("stroke-width", 3),
                );
            }
        }
    }

    // robot as a direction triangle
    let cx = sim.p.1 * D + D / 2;
    let cy = y0 + sim.p.0 * D + D / 2;
    let r = D * 2 / 5;
    let pts = match sim.dir {
        0 => vec![
            (cx, cy - r),
            (cx - r / 2, cy + r / 2),
            (cx + r / 2, cy + r / 2),
        ],
        1 => vec![
            (cx + r, cy),
            (cx - r / 2, cy - r / 2),
            (cx - r / 2, cy + r / 2),
        ],
        2 => vec![
            (cx, cy + r),
            (cx - r / 2, cy - r / 2),
            (cx + r / 2, cy - r / 2),
        ],
        3 => vec![
            (cx - r, cy),
            (cx + r / 2, cy - r / 2),
            (cx + r / 2, cy + r / 2),
        ],
        _ => unreachable!(),
    };
    let points = pts
        .into_iter()
        .map(|(x, y)| format!("{},{}", x, y))
        .collect::<Vec<_>>()
        .join(" ");
    doc = doc.add(
        Polygon::new()
            .set("points", points)
            .set("fill", if current_from_play { PLAY_COLOR } else { "#222222cc" })
            .set("stroke", "#ffffff")
            .set("stroke-width", 2),
    );

    if let Some(h) = sim.hand {
        doc = doc.add(
            Text::new(format!("H{}", h))
                .set("x", cx)
                .set("y", cy)
                .set("font-size", (D / 4).max(8))
                .set("class", "center")
                .set("fill", "white"),
        );
    }

    (score, err, doc.to_string())
}

pub fn get_max_turn(input: &Input, out: &Output) -> usize {
    expand_output(input, out).len()
}
