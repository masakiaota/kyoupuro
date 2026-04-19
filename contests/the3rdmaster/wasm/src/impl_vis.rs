#![allow(non_snake_case, unused_macros, dead_code)]

use proconio::input;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::any::Any;
use std::ops::RangeBounds;
use svg::node::element::{Line, Rectangle, Text as SvgText};
use svg::Document;

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
        for row in &self.g {
            for (j, value) in row.iter().enumerate() {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{value}")?;
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
                Err(format!("Out of range: {v}"))
            } else {
                Ok(v)
            }
        } else {
            Err(format!("Parse error: {v}"))
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
        if line.is_empty() {
            continue;
        }
        let a = parse_action_line(input, line)?;
        out.push(a);
        if out.len() > input.N * input.N {
            return Err("Too many actions".to_owned());
        }
    }
    Ok(Output { out })
}

fn copy(ck: &[Vec<usize>], ch: &[Vec<usize>], di: i32, dj: i32) -> Option<Vec<Vec<usize>>> {
    let N = ck.len();
    let mut ck = ck.to_vec();
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

fn rot(ch: &[Vec<usize>], r: usize) -> Vec<Vec<usize>> {
    let N = ch.len();
    let mut ch = ch.to_vec();
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

fn bb(ch: &[Vec<usize>]) -> (usize, usize, usize, usize) {
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
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let N = 32;
    let K = rng.gen_range(2..=5i32) as usize;
    let C = rng.gen_range(2..=4i32) as usize;
    let K2 = rng.gen_range(1..=K as i32 * 2) as usize;
    let mut c = mat![0; K2; N; N];
    c[0][N / 2][N / 2] = 1;
    let mut size = vec![0; K2];
    size[0] = 1;
    const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];
    fn check(c: &[Vec<usize>]) -> bool {
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
                    (
                        rng.gen_range(0..N as i32) as usize,
                        rng.gen_range(0..N as i32) as usize,
                    )
                };
                let mut num = vec![0; C + 1];
                for layer in 0..K2 {
                    for y in 0..N {
                        for x in 0..N {
                            num[c[layer][y][x]] += 1;
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
    if !err.is_empty() {
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
                        format!("Invalid copy: k={k}, h={h}, r={r}, di={di}, dj={dj}"),
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

struct ParsedOutput {
    actions: Vec<Action>,
    err: String,
}

struct VisSummary {
    shown_turn: usize,
    slider_max_turn: usize,
    applied_turn: usize,
    total_actions: usize,
    score: i64,
    matched_cells: usize,
    diff_cells: usize,
    target_nonzero: usize,
    layer0_nonzero: usize,
    status: &'static str,
    action_text: String,
    err: String,
}

fn panic_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else if let Some(msg) = payload.downcast_ref::<&str>() {
        (*msg).to_owned()
    } else {
        "panic while parsing input".to_owned()
    }
}

fn parse_input_safe(input: &str) -> Result<Input, String> {
    std::panic::catch_unwind(|| parse_input(input)).map_err(panic_to_string)
}

fn parse_action_line(input: &Input, line: &str) -> Result<Action, String> {
    let mut ss = line.split_whitespace();
    let ty: usize = read(ss.next(), 0..=2)?;
    let action = match ty {
        0 => Action::Paint {
            k: read(ss.next(), 0..input.K)?,
            i: read(ss.next(), 0..input.N)?,
            j: read(ss.next(), 0..input.N)?,
            c: read(ss.next(), 0..=input.C)?,
        },
        1 => Action::Copy {
            k: read(ss.next(), 0..input.K)?,
            h: read(ss.next(), 0..input.K)?,
            r: read(ss.next(), 0..4)?,
            di: read(ss.next(), -(input.N as i32) + 1..=(input.N as i32) - 1)?,
            dj: read(ss.next(), -(input.N as i32) + 1..=(input.N as i32) - 1)?,
        },
        _ => Action::Clear {
            k: read(ss.next(), 0..input.K)?,
        },
    };
    if ss.next().is_some() {
        return Err(format!("Extra token: {line}"));
    }
    Ok(action)
}

fn parse_output_prefix(input: &Input, output: &str) -> ParsedOutput {
    let mut actions = vec![];
    let mut err = String::new();

    for (idx, raw_line) in output.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if actions.len() >= input.N * input.N {
            err = format!("line {}: Too many actions", idx + 1);
            break;
        }
        match parse_action_line(input, line) {
            Ok(action) => actions.push(action),
            Err(message) => {
                err = format!("line {}: {message}", idx + 1);
                break;
            }
        }
    }

    ParsedOutput { actions, err }
}

fn simulate_layers(
    input: &Input,
    actions: &[Action],
    turn: usize,
) -> (Vec<Vec<Vec<usize>>>, usize, Option<String>) {
    let mut layers = mat![0; input.K; input.N; input.N];
    let mut applied_turn = 0;
    for (idx, action) in actions.iter().take(turn.min(actions.len())).enumerate() {
        match *action {
            Action::Paint { k, i, j, c } => {
                layers[k][i][j] = c;
            }
            Action::Copy { k, h, r, di, dj } => {
                let rotated = rot(&layers[h], r);
                if let Some(next) = copy(&layers[k], &rotated, di, dj) {
                    layers[k] = next;
                } else {
                    return (
                        layers,
                        applied_turn,
                        Some(format!(
                            "action {}: Invalid copy: k={}, h={}, r={}, di={}, dj={}",
                            idx + 1,
                            k,
                            h,
                            r,
                            di,
                            dj
                        )),
                    );
                }
            }
            Action::Clear { k } => {
                layers[k] = mat![0; input.N; input.N];
            }
        }
        applied_turn += 1;
    }
    (layers, applied_turn, None)
}

fn count_nonzero(grid: &[Vec<usize>]) -> usize {
    grid.iter().flatten().filter(|&&value| value != 0).count()
}

fn count_equal(lhs: &[Vec<usize>], rhs: &[Vec<usize>]) -> usize {
    lhs.iter()
        .flatten()
        .zip(rhs.iter().flatten())
        .filter(|(a, b)| a == b)
        .count()
}

fn action_label(action: &Action) -> String {
    match *action {
        Action::Paint { k, i, j, c } => format!("paint(k={k}, i={i}, j={j}, c={c})"),
        Action::Copy { k, h, r, di, dj } => {
            format!("copy(k={k}, h={h}, rot={}deg, di={di}, dj={dj})", r * 90)
        }
        Action::Clear { k } => format!("clear(k={k})"),
    }
}

fn merge_errors(parse_err: &str, runtime_err: Option<&str>) -> String {
    match (parse_err.is_empty(), runtime_err) {
        (true, None) => String::new(),
        (false, None) => parse_err.to_owned(),
        (true, Some(runtime_err)) => runtime_err.to_owned(),
        (false, Some(runtime_err)) => format!("{parse_err}\n{runtime_err}"),
    }
}

fn color_for_value(value: usize) -> &'static str {
    match value {
        0 => "#f8fafc",
        1 => "#2563eb",
        2 => "#ef4444",
        3 => "#10b981",
        4 => "#f59e0b",
        _ => "#6b7280",
    }
}

fn accent_for_layer(action: Option<Action>, layer: usize) -> &'static str {
    match action {
        Some(Action::Paint { k, .. }) | Some(Action::Clear { k, .. }) if k == layer => "#f97316",
        Some(Action::Copy { k, h, .. }) if k == layer && h == layer => "#7c3aed",
        Some(Action::Copy { k, .. }) if k == layer => "#f97316",
        Some(Action::Copy { h, .. }) if h == layer => "#0ea5e9",
        _ => "#94a3b8",
    }
}

fn role_for_layer(action: Option<Action>, layer: usize) -> &'static str {
    match action {
        Some(Action::Paint { k, .. }) | Some(Action::Clear { k, .. }) if k == layer => "target",
        Some(Action::Copy { k, h, .. }) if k == layer && h == layer => "source + target",
        Some(Action::Copy { k, .. }) if k == layer => "target",
        Some(Action::Copy { h, .. }) if h == layer => "source",
        _ => "",
    }
}

fn add_text(
    doc: Document,
    text: impl Into<String>,
    x: i32,
    y: i32,
    size: usize,
    color: &str,
    weight: usize,
) -> Document {
    doc.add(
        SvgText::new(text.into())
            .set("x", x)
            .set("y", y)
            .set("fill", color)
            .set("font-size", size)
            .set("font-weight", weight)
            .set("font-family", "system-ui, sans-serif"),
    )
}

fn draw_panel(doc: Document, x: i32, y: i32, w: i32, h: i32, fill: &str, stroke: &str) -> Document {
    doc.add(
        Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", w)
            .set("height", h)
            .set("rx", 14)
            .set("ry", 14)
            .set("fill", fill)
            .set("stroke", stroke)
            .set("stroke-width", 2),
    )
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(current);
            current = String::new();
            continue;
        }
        current.push(ch);
        if current.chars().count() >= width {
            lines.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn draw_board_panel(
    mut doc: Document,
    title: &str,
    subtitle: &str,
    grid: &[Vec<usize>],
    target: Option<&[Vec<usize>]>,
    x: i32,
    y: i32,
    cell: i32,
    accent: &str,
) -> Document {
    let n = grid.len() as i32;
    let board_size = n * cell;
    let panel_w = board_size + 24;
    let panel_h = board_size + 56;
    let board_x = x + 12;
    let board_y = y + 36;

    doc = draw_panel(doc, x, y, panel_w, panel_h, "#ffffff", accent);
    doc = add_text(doc, title, x + 14, y + 22, 18, "#0f172a", 700);
    if !subtitle.is_empty() {
        doc = add_text(doc, subtitle, x + 14, y + 40, 12, "#475569", 500);
    }

    for i in 0..n {
        for j in 0..n {
            let value = grid[i as usize][j as usize];
            let mismatch = target
                .map(|target| target[i as usize][j as usize] != value)
                .unwrap_or(false);
            let px = board_x + j * cell;
            let py = board_y + i * cell;

            doc = doc.add(
                Rectangle::new()
                    .set("x", px)
                    .set("y", py)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", color_for_value(value))
                    .set("fill-opacity", if value == 0 { 0.72 } else { 1.0 })
                    .set("stroke", if mismatch { "#dc2626" } else { "#cbd5e1" })
                    .set("stroke-width", if mismatch { 1.5 } else { 0.8 }),
            );

            if mismatch && cell >= 12 {
                doc = doc.add(
                    Rectangle::new()
                        .set("x", px + 2)
                        .set("y", py + 2)
                        .set("width", cell - 4)
                        .set("height", cell - 4)
                        .set("fill", "none")
                        .set("stroke", "#b91c1c")
                        .set("stroke-width", 1),
                );
            }
        }
    }

    doc
}

fn draw_sidebar(mut doc: Document, summary: &VisSummary, colors: usize, x: i32, y: i32) -> Document {
    let w = 264;
    let h = 520;
    doc = draw_panel(doc, x, y, w, h, "#ffffff", "#cbd5e1");

    let status_color = match summary.status {
        "finished" => "#15803d",
        "invalid output" => "#b91c1c",
        _ => "#0f766e",
    };

    doc = add_text(doc, "Visualizer", x + 16, y + 28, 20, "#0f172a", 800);
    doc = add_text(doc, format!("score {}", summary.score), x + 16, y + 64, 30, "#111827", 800);
    doc = add_text(
        doc,
        format!("status {}", summary.status),
        x + 16,
        y + 92,
        14,
        status_color,
        700,
    );

    let stats = [
        format!("turn {}/{}", summary.shown_turn, summary.slider_max_turn),
        format!("applied {}", summary.applied_turn),
        format!("parsed actions {}", summary.total_actions),
        format!("layer0 nonzero {}", summary.layer0_nonzero),
        format!("target nonzero {}", summary.target_nonzero),
        format!("matched cells {}", summary.matched_cells),
        format!("diff cells {}", summary.diff_cells),
    ];
    let mut cursor_y = y + 126;
    for line in stats {
        doc = add_text(doc, line, x + 16, cursor_y, 14, "#334155", 500);
        cursor_y += 22;
    }

    doc = doc.add(
        Line::new()
            .set("x1", x + 16)
            .set("y1", cursor_y)
            .set("x2", x + w - 16)
            .set("y2", cursor_y)
            .set("stroke", "#e2e8f0")
            .set("stroke-width", 1),
    );
    cursor_y += 28;

    doc = add_text(doc, "Current action", x + 16, cursor_y, 16, "#0f172a", 700);
    cursor_y += 22;
    for line in wrap_text(&summary.action_text, 28).into_iter().take(6) {
        doc = add_text(doc, line, x + 16, cursor_y, 13, "#334155", 500);
        cursor_y += 18;
    }

    cursor_y += 8;
    doc = add_text(doc, "Legend", x + 16, cursor_y, 16, "#0f172a", 700);
    cursor_y += 24;
    for value in 0..=colors {
        doc = doc.add(
            Rectangle::new()
                .set("x", x + 16)
                .set("y", cursor_y - 12)
                .set("width", 16)
                .set("height", 16)
                .set("fill", color_for_value(value))
                .set("stroke", "#94a3b8")
                .set("stroke-width", 1),
        );
        let label = if value == 0 {
            "0 transparent".to_owned()
        } else {
            format!("{value} color")
        };
        doc = add_text(doc, label, x + 40, cursor_y + 1, 13, "#334155", 500);
        cursor_y += 22;
    }

    if !summary.err.is_empty() {
        cursor_y += 8;
        doc = add_text(doc, "Error", x + 16, cursor_y, 16, "#b91c1c", 700);
        cursor_y += 20;
        for line in wrap_text(&summary.err, 28).into_iter().take(8) {
            doc = add_text(doc, line, x + 16, cursor_y, 13, "#991b1b", 500);
            cursor_y += 18;
        }
    }

    doc
}

fn draw_svg(input: &Input, layers: &[Vec<Vec<usize>>], selected_action: Option<Action>, summary: &VisSummary) -> String {
    let big_cell = 16;
    let target_cell = 12;
    let mini_cell = 8;
    let current_x = 24;
    let current_y = 24;
    let target_x = 584;
    let target_y = 24;
    let sidebar_x = 1012;
    let sidebar_y = 24;
    let mini_x = 24;
    let mini_y = 610;
    let mini_cols = 3;

    let mini_rows = (input.K + mini_cols - 1) / mini_cols;
    let mini_panel_h = (input.N as i32 * mini_cell) + 56;
    let mini_total_h = mini_rows as i32 * (mini_panel_h + 20);
    let height = (mini_y + mini_total_h + 24).max(660);
    let width = 1296;

    let mut doc = Document::new()
        .set("viewBox", format!("0 0 {width} {height}"))
        .set("width", width)
        .set("height", height)
        .set("style", "background:#f8fafc");

    doc = draw_board_panel(
        doc,
        "Layer 0",
        &format!(
            "nonzero={}  diff={}",
            summary.layer0_nonzero, summary.diff_cells
        ),
        &layers[0],
        Some(&input.g),
        current_x,
        current_y,
        big_cell,
        accent_for_layer(selected_action, 0),
    );

    doc = draw_board_panel(
        doc,
        "Target",
        &format!("nonzero={}", summary.target_nonzero),
        &input.g,
        None,
        target_x,
        target_y,
        target_cell,
        "#2563eb",
    );

    doc = draw_sidebar(doc, summary, input.C, sidebar_x, sidebar_y);

    for layer in 0..input.K {
        let row = layer / mini_cols;
        let col = layer % mini_cols;
        let x = mini_x + col as i32 * ((input.N as i32 * mini_cell) + 44);
        let y = mini_y + row as i32 * (mini_panel_h + 20);
        let role = role_for_layer(selected_action, layer);
        let subtitle = if role.is_empty() {
            format!("nonzero={}", count_nonzero(&layers[layer]))
        } else {
            format!("nonzero={}  {role}", count_nonzero(&layers[layer]))
        };
        doc = draw_board_panel(
            doc,
            &format!("Layer {layer}"),
            &subtitle,
            &layers[layer],
            None,
            x,
            y,
            mini_cell,
            accent_for_layer(selected_action, layer),
        );
    }

    doc.to_string()
}

pub fn generate(seed: i32) -> String {
    gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    match parse_input_safe(input) {
        Ok(input) => parse_output_prefix(&input, output).actions.len().max(1),
        Err(_) => 1,
    }
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let input = parse_input_safe(input)?;
    let parsed = parse_output_prefix(&input, output);
    let slider_max_turn = parsed.actions.len().max(1);
    let shown_turn = turn.min(slider_max_turn);
    let selected_action = if shown_turn == 0 {
        None
    } else {
        parsed.actions.get(shown_turn.saturating_sub(1)).copied()
    };
    let (layers, applied_turn, runtime_err) = simulate_layers(&input, &parsed.actions, shown_turn);
    let combined_err = merge_errors(&parsed.err, runtime_err.as_deref());

    let (score, _, _) = compute_score_details(&input, &parsed.actions[..applied_turn]);
    let matched_cells = count_equal(&layers[0], &input.g);
    let diff_cells = input.N * input.N - matched_cells;
    let target_nonzero = count_nonzero(&input.g);
    let layer0_nonzero = count_nonzero(&layers[0]);
    let status = if !combined_err.is_empty() {
        "invalid output"
    } else if diff_cells == 0 {
        "finished"
    } else {
        "in progress"
    };

    let action_text = if shown_turn == 0 {
        "step 0: no action".to_owned()
    } else if let Some(action) = selected_action {
        format!("step {shown_turn}: {}", action_label(&action))
    } else {
        format!("step {shown_turn}: no parsed action")
    };

    let return_score = if combined_err.is_empty() { score } else { 0 };
    let summary = VisSummary {
        shown_turn,
        slider_max_turn,
        applied_turn,
        total_actions: parsed.actions.len(),
        score: return_score,
        matched_cells,
        diff_cells,
        target_nonzero,
        layer0_nonzero,
        status,
        action_text,
        err: combined_err.clone(),
    };
    let svg = draw_svg(&input, &layers, selected_action, &summary);
    Ok((return_score, combined_err, svg))
}
