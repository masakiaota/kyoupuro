#![allow(non_snake_case, unused_macros, dead_code)]

use proconio::input;
use rand::prelude::*;
use std::fmt::Write as _;
use std::ops::RangeBounds;
use svg::node::element::{Circle, Line, Polyline, Rectangle, Text as SvgText};
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
    N: usize,
    A: Vec<Vec<i64>>,
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
    let N = 200usize;
    let mut A0 = (1..=(N * N) as i64).collect::<Vec<_>>();
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
    if !err.is_empty() {
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
        format!("Not finished: {} / {}", out.len(), N2)
    } else {
        String::new()
    };
    ((score + N2 / 2) / N2, err, ())
}

struct ParsedOutput {
    out: Vec<(usize, usize)>,
    parse_errors: Vec<String>,
    parse_error_turn: Option<usize>,
}

struct Analysis {
    out: Vec<(usize, usize)>,
    gains: Vec<i64>,
    prefix: Vec<i64>,
    valid_prefix_len: usize,
    first_error_turn: Option<usize>,
    first_error_message: String,
    parse_error_turn: Option<usize>,
    official_score: i64,
    official_error: String,
}

fn parse_output_lenient(input: &Input, f: &str) -> ParsedOutput {
    let mut tokens = f.split_whitespace();
    let mut out = Vec::new();
    let mut parse_errors = Vec::new();
    let mut parse_error_turn = None;
    let mut turn = 0usize;

    loop {
        let ti = match tokens.next() {
            Some(v) => v,
            None => break,
        };
        let tj = match tokens.next() {
            Some(v) => v,
            None => {
                parse_errors.push(format!("turn {}: j is missing", turn));
                parse_error_turn = Some(turn);
                break;
            }
        };
        if out.len() >= input.N * input.N {
            parse_errors.push(format!(
                "turn {}: too many outputs (limit is {})",
                turn,
                input.N * input.N
            ));
            parse_error_turn = Some(turn);
            break;
        }

        let i = match ti.parse::<isize>() {
            Ok(v) => v,
            Err(_) => {
                parse_errors.push(format!("turn {}: failed to parse i ('{}')", turn, ti));
                parse_error_turn = Some(turn);
                break;
            }
        };
        let j = match tj.parse::<isize>() {
            Ok(v) => v,
            Err(_) => {
                parse_errors.push(format!("turn {}: failed to parse j ('{}')", turn, tj));
                parse_error_turn = Some(turn);
                break;
            }
        };

        if i < 0 || i >= input.N as isize || j < 0 || j >= input.N as isize {
            parse_errors.push(format!(
                "turn {}: out of range ({}, {}) (expected 0..{})",
                turn,
                i,
                j,
                input.N - 1
            ));
            parse_error_turn = Some(turn);
            break;
        }

        out.push((i as usize, j as usize));
        turn += 1;
    }

    ParsedOutput {
        out,
        parse_errors,
        parse_error_turn,
    }
}

fn analyze_output(input: &Input, parsed: ParsedOutput) -> Analysis {
    let n2 = input.N * input.N;
    let len = parsed.out.len();
    let mut gains = vec![0i64; len];
    let mut prefix = vec![0i64; len];
    let mut used_turn = mat![None; input.N; input.N];

    let mut running = 0i64;
    let mut valid_prefix_len = 0usize;
    let mut first_error_turn = None;
    let mut first_error_message = String::new();

    for t in 0..len {
        if first_error_turn.is_some() {
            prefix[t] = running;
            continue;
        }
        let (i, j) = parsed.out[t];
        if let Some(first_t) = used_turn[i][j] {
            first_error_turn = Some(t);
            first_error_message = format!(
                "Duplicate move at turn {}: ({}, {}) first visited at turn {}",
                t, i, j, first_t
            );
            prefix[t] = running;
            continue;
        }
        if t > 0 {
            let (pi, pj) = parsed.out[t - 1];
            if i.abs_diff(pi).max(j.abs_diff(pj)) != 1 {
                first_error_turn = Some(t);
                first_error_message = format!(
                    "Invalid move at turn {}: ({}, {}) -> ({}, {})",
                    t, pi, pj, i, j
                );
                prefix[t] = running;
                continue;
            }
        }

        used_turn[i][j] = Some(t);
        let gain = t as i64 * input.A[i][j];
        gains[t] = gain;
        running += gain;
        prefix[t] = running;
        valid_prefix_len = t + 1;
    }

    let mut errs = Vec::new();
    if !parsed.parse_errors.is_empty() {
        errs.push(parsed.parse_errors.join("\n"));
    }
    if !first_error_message.is_empty() {
        errs.push(first_error_message.clone());
    }
    if len < n2 {
        errs.push(format!("Not finished: {} / {}", len, n2));
    }

    let official_error = errs.join("\n");
    let official_score = if official_error.is_empty() {
        let n2_i64 = n2 as i64;
        (running + n2_i64 / 2) / n2_i64
    } else {
        0
    };

    Analysis {
        out: parsed.out,
        gains,
        prefix,
        valid_prefix_len,
        first_error_turn,
        first_error_message,
        parse_error_turn: parsed.parse_error_turn,
        official_score,
        official_error,
    }
}

fn clamp01(t: f64) -> f64 {
    if t < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        t
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * clamp01(t)).round() as u8
}

fn gradient_hex(c0: (u8, u8, u8), c1: (u8, u8, u8), t: f64) -> String {
    let r = lerp_u8(c0.0, c1.0, t);
    let g = lerp_u8(c0.1, c1.1, t);
    let b = lerp_u8(c0.2, c1.2, t);
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn color_for_value(value: i64, max_value: i64) -> String {
    if max_value <= 1 {
        return "#f8fafc".to_owned();
    }
    let t = (value - 1) as f64 / (max_value - 1) as f64;
    gradient_hex((241, 245, 249), (185, 28, 28), t)
}

fn color_for_visit_order(first_turn: Option<usize>, shown_len: usize) -> String {
    if let Some(t) = first_turn {
        let denom = shown_len.saturating_sub(1).max(1) as f64;
        let r = t as f64 / denom;
        gradient_hex((224, 242, 254), (15, 118, 110), r)
    } else {
        "#f8fafc".to_owned()
    }
}

fn color_for_visit_count(cnt: usize) -> &'static str {
    match cnt {
        0 => "#f8fafc",
        1 => "#d1fae5",
        2 => "#fde68a",
        _ => "#ef4444",
    }
}

fn mode_label(mode: usize) -> &'static str {
    match mode {
        2 => "訪問順ヒートマップ",
        3 => "重複/違反チェック",
        _ => "A値ヒートマップ",
    }
}

fn draw_svg(input: &Input, analysis: &Analysis, shown_len: usize, mode: usize) -> String {
    let pad = 24.0f64;
    let cell = 4.0f64;
    let board = input.N as f64 * cell;
    let side_w = 390.0f64;
    let graph_h = 220.0f64;
    let width = pad * 3.0 + board + side_w;
    let height = pad * 4.0 + board + graph_h;
    let board_x = pad;
    let board_y = pad;

    let shown_len = shown_len.min(analysis.out.len());
    let mut first_visit = mat![None; input.N; input.N];
    let mut visit_count = mat![0usize; input.N; input.N];
    for t in 0..shown_len {
        let (i, j) = analysis.out[t];
        visit_count[i][j] += 1;
        if first_visit[i][j].is_none() {
            first_visit[i][j] = Some(t);
        }
    }

    let mut unique_count = 0usize;
    for row in &first_visit {
        for v in row {
            if v.is_some() {
                unique_count += 1;
            }
        }
    }

    let shown_turn = shown_len.saturating_sub(1);
    let current_gain = if shown_len > 0 {
        analysis.gains[shown_turn]
    } else {
        0
    };
    let current_prefix = if shown_len > 0 {
        analysis.prefix[shown_turn]
    } else {
        0
    };
    let n2 = (input.N * input.N) as i64;
    let current_round = if shown_len > 0 {
        (current_prefix + n2 / 2) / n2
    } else {
        0
    };

    let mut doc = Document::new()
        .set("viewBox", format!("0 0 {} {}", width, height))
        .set("width", width)
        .set("height", height)
        .set("style", "background:#ffffff;font-family:'SFMono-Regular',Menlo,monospace;");
    doc = doc.add(
        Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", width)
            .set("height", height)
            .set("fill", "#f9fafb"),
    );

    let max_a = (input.N * input.N) as i64;
    for i in 0..input.N {
        for j in 0..input.N {
            let fill = if mode == 2 {
                color_for_visit_order(first_visit[i][j], shown_len)
            } else if mode == 3 {
                color_for_visit_count(visit_count[i][j]).to_owned()
            } else {
                color_for_value(input.A[i][j], max_a)
            };
            doc = doc.add(
                Rectangle::new()
                    .set("x", board_x + j as f64 * cell)
                    .set("y", board_y + i as f64 * cell)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", fill)
                    .set("stroke", "#e5e7eb")
                    .set("stroke-width", 0.2),
            );
        }
    }

    if shown_len >= 2 {
        let mut points = String::new();
        for t in 0..shown_len {
            let (i, j) = analysis.out[t];
            let x = board_x + (j as f64 + 0.5) * cell;
            let y = board_y + (i as f64 + 0.5) * cell;
            let _ = write!(points, "{:.2},{:.2} ", x, y);
        }
        doc = doc.add(
            Polyline::new()
                .set("points", points)
                .set("fill", "none")
                .set("stroke", "#0f766e")
                .set("stroke-width", if mode == 3 { 1.6 } else { 1.2 })
                .set("stroke-linecap", "round")
                .set("stroke-linejoin", "round")
                .set("opacity", 0.9),
        );
    }

    if shown_len > 0 {
        let (si, sj) = analysis.out[0];
        let sx = board_x + (sj as f64 + 0.5) * cell;
        let sy = board_y + (si as f64 + 0.5) * cell;
        doc = doc.add(
            Circle::new()
                .set("cx", sx)
                .set("cy", sy)
                .set("r", 2.3)
                .set("fill", "#2563eb")
                .set("stroke", "#ffffff")
                .set("stroke-width", 0.8),
        );

        let (ci, cj) = analysis.out[shown_turn];
        let cx = board_x + (cj as f64 + 0.5) * cell;
        let cy = board_y + (ci as f64 + 0.5) * cell;
        doc = doc.add(
            Circle::new()
                .set("cx", cx)
                .set("cy", cy)
                .set("r", 2.8)
                .set("fill", "#f59e0b")
                .set("stroke", "#111827")
                .set("stroke-width", 0.9),
        );
    }

    if let Some(t) = analysis.first_error_turn {
        if t < shown_len {
            let (i, j) = analysis.out[t];
            doc = doc.add(
                Rectangle::new()
                    .set("x", board_x + j as f64 * cell - 0.5)
                    .set("y", board_y + i as f64 * cell - 0.5)
                    .set("width", cell + 1.0)
                    .set("height", cell + 1.0)
                    .set("fill", "none")
                    .set("stroke", "#dc2626")
                    .set("stroke-width", 1.2),
            );
            if t > 0 {
                let (pi, pj) = analysis.out[t - 1];
                let x1 = board_x + (pj as f64 + 0.5) * cell;
                let y1 = board_y + (pi as f64 + 0.5) * cell;
                let x2 = board_x + (j as f64 + 0.5) * cell;
                let y2 = board_y + (i as f64 + 0.5) * cell;
                doc = doc.add(
                    Line::new()
                        .set("x1", x1)
                        .set("y1", y1)
                        .set("x2", x2)
                        .set("y2", y2)
                        .set("stroke", "#dc2626")
                        .set("stroke-width", 1.5)
                        .set("stroke-linecap", "round"),
                );
            }
        }
    }

    let panel_x = board_x + board + pad;
    let panel_y = board_y;
    doc = doc.add(
        Rectangle::new()
            .set("x", panel_x)
            .set("y", panel_y)
            .set("width", side_w)
            .set("height", board)
            .set("fill", "#ffffff")
            .set("stroke", "#d1d5db")
            .set("stroke-width", 1.0)
            .set("rx", 8)
            .set("ry", 8),
    );

    let mut info_y = panel_y + 26.0;
    let info_step = 22.0;
    let add_info = |doc: Document, y: f64, txt: String, color: &str| -> Document {
        doc.add(
            SvgText::new(txt)
                .set("x", panel_x + 14.0)
                .set("y", y)
                .set("font-size", 14)
                .set("fill", color),
        )
    };

    doc = add_info(
        doc,
        info_y,
        "King's Tour Visualizer".to_owned(),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Mode: {}", mode_label(mode)),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Shown cells: {} / {}", shown_len, analysis.out.len()),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Current k: {}", shown_turn),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Gain(k)=k*A: {}", current_gain),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Prefix V(0..k): {}", current_prefix),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Prefix round(V/N^2): {}", current_round),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Final score: {}", analysis.official_score),
        "#0f766e",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Unique visited: {} / {}", unique_count, n2),
        "#111827",
    );
    info_y += info_step;
    doc = add_info(
        doc,
        info_y,
        format!("Valid prefix length: {}", analysis.valid_prefix_len),
        "#111827",
    );
    info_y += info_step;
    if let Some(t) = analysis.parse_error_turn {
        doc = add_info(doc, info_y, format!("Parse error turn: {}", t), "#b91c1c");
        info_y += info_step;
    }
    if let Some(t) = analysis.first_error_turn {
        doc = add_info(doc, info_y, format!("Constraint error turn: {}", t), "#b91c1c");
        info_y += info_step;
    }
    if !analysis.first_error_message.is_empty() {
        for line in analysis.first_error_message.lines().take(4) {
            doc = add_info(doc, info_y, line.to_owned(), "#b91c1c");
            info_y += info_step - 2.0;
        }
    }

    let graph_x = pad;
    let graph_y = pad * 2.0 + board;
    let graph_w = width - pad * 2.0;
    let graph_h_inner = graph_h;

    doc = doc.add(
        Rectangle::new()
            .set("x", graph_x)
            .set("y", graph_y)
            .set("width", graph_w)
            .set("height", graph_h_inner)
            .set("fill", "#ffffff")
            .set("stroke", "#d1d5db")
            .set("stroke-width", 1.0)
            .set("rx", 8)
            .set("ry", 8),
    );

    let inner_x = graph_x + 40.0;
    let inner_y = graph_y + 20.0;
    let inner_w = graph_w - 56.0;
    let inner_h = graph_h_inner - 42.0;
    doc = doc.add(
        Line::new()
            .set("x1", inner_x)
            .set("y1", inner_y + inner_h)
            .set("x2", inner_x + inner_w)
            .set("y2", inner_y + inner_h)
            .set("stroke", "#9ca3af")
            .set("stroke-width", 1.0),
    );
    doc = doc.add(
        Line::new()
            .set("x1", inner_x)
            .set("y1", inner_y)
            .set("x2", inner_x)
            .set("y2", inner_y + inner_h)
            .set("stroke", "#9ca3af")
            .set("stroke-width", 1.0),
    );

    let total = analysis.out.len();
    if total > 0 {
        let x_den = total.saturating_sub(1).max(1) as f64;
        let gain_max = analysis.gains.iter().copied().max().unwrap_or(1).max(1) as f64;
        let pref_max = analysis.prefix.last().copied().unwrap_or(1).max(1) as f64;

        let mut gain_points = String::new();
        let mut pref_points = String::new();
        for k in 0..total {
            let x = inner_x + (k as f64 / x_den) * inner_w;
            let yg = inner_y + inner_h - (analysis.gains[k] as f64 / gain_max) * inner_h;
            let yp = inner_y + inner_h - (analysis.prefix[k] as f64 / pref_max) * inner_h;
            let _ = write!(gain_points, "{:.2},{:.2} ", x, yg);
            let _ = write!(pref_points, "{:.2},{:.2} ", x, yp);
        }

        doc = doc.add(
            Polyline::new()
                .set("points", gain_points)
                .set("fill", "none")
                .set("stroke", "#2563eb")
                .set("stroke-width", 1.2)
                .set("opacity", 0.95),
        );
        doc = doc.add(
            Polyline::new()
                .set("points", pref_points)
                .set("fill", "none")
                .set("stroke", "#f59e0b")
                .set("stroke-width", 1.2)
                .set("opacity", 0.95),
        );

        if shown_len > 0 {
            let x_cur = inner_x + (shown_turn as f64 / x_den) * inner_w;
            doc = doc.add(
                Line::new()
                    .set("x1", x_cur)
                    .set("y1", inner_y)
                    .set("x2", x_cur)
                    .set("y2", inner_y + inner_h)
                    .set("stroke", "#dc2626")
                    .set("stroke-width", 1.0)
                    .set("stroke-dasharray", "4,4"),
            );

            let y_gain =
                inner_y + inner_h - (analysis.gains[shown_turn] as f64 / gain_max) * inner_h;
            let y_pref =
                inner_y + inner_h - (analysis.prefix[shown_turn] as f64 / pref_max) * inner_h;
            doc = doc.add(
                Circle::new()
                    .set("cx", x_cur)
                    .set("cy", y_gain)
                    .set("r", 2.4)
                    .set("fill", "#2563eb"),
            );
            doc = doc.add(
                Circle::new()
                    .set("cx", x_cur)
                    .set("cy", y_pref)
                    .set("r", 2.4)
                    .set("fill", "#f59e0b"),
            );
        }

        doc = doc.add(
            SvgText::new(format!("kごとの獲得スコア (青) / 累積V (橙), max gain={}", gain_max as i64))
                .set("x", inner_x)
                .set("y", graph_y + 14.0)
                .set("font-size", 13)
                .set("fill", "#111827"),
        );
        doc = doc.add(
            SvgText::new(format!("k=0"))
                .set("x", inner_x)
                .set("y", inner_y + inner_h + 16.0)
                .set("font-size", 12)
                .set("fill", "#6b7280"),
        );
        doc = doc.add(
            SvgText::new(format!("k={}", total.saturating_sub(1)))
                .set("x", inner_x + inner_w - 70.0)
                .set("y", inner_y + inner_h + 16.0)
                .set("font-size", 12)
                .set("fill", "#6b7280"),
        );
    } else {
        doc = doc.add(
            SvgText::new("output を入力すると k ごとのグラフを描画する")
                .set("x", inner_x)
                .set("y", inner_y + inner_h / 2.0)
                .set("font-size", 14)
                .set("fill", "#6b7280"),
        );
    }

    doc.to_string()
}

pub fn generate(seed: i32, _problem_id: &str) -> String {
    let input = gen(seed.max(0) as u64);
    format!("{}", input)
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    let input = parse_input(input);
    let parsed = parse_output_lenient(&input, output);
    if parsed.out.is_empty() {
        1
    } else {
        parsed.out.len().min(input.N * input.N).max(1)
    }
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    visualize_with_mode(input, output, turn, 1, 0)
}

pub fn visualize_with_mode(
    input: &str,
    output: &str,
    turn: usize,
    mode: usize,
    _focus_robot: usize,
) -> Result<(i64, String, String), String> {
    let input = parse_input(input);
    let parsed = parse_output_lenient(&input, output);
    let analysis = analyze_output(&input, parsed);
    let shown_len = if analysis.out.is_empty() {
        0
    } else {
        turn.max(1).min(analysis.out.len())
    };
    let svg = draw_svg(&input, &analysis, shown_len, mode);
    Ok((analysis.official_score, analysis.official_error, svg))
}
