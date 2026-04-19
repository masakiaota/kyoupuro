use super::*;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Group, Path as SvgPath, Rectangle, Style, Text, Title};

fn group(title: String) -> Group {
    Group::new().add(Title::new(title))
}

fn rgb(r: i32, g: i32, b: i32) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        r.clamp(0, 255),
        g.clamp(0, 255),
        b.clamp(0, 255)
    )
}

fn mix_color(a: (i32, i32, i32), b: (i32, i32, i32), t: f64) -> (i32, i32, i32) {
    let u = 1.0 - t;
    (
        (a.0 as f64 * u + b.0 as f64 * t).round() as i32,
        (a.1 as f64 * u + b.1 as f64 * t).round() as i32,
        (a.2 as f64 * u + b.2 as f64 * t).round() as i32,
    )
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (i32, i32, i32) {
    let h = ((h % 1.0) + 1.0) % 1.0;
    let x = h * 6.0;
    let i = x.floor() as i32;
    let f = x - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    (
        (r * 255.0).round() as i32,
        (g * 255.0).round() as i32,
        (b * 255.0).round() as i32,
    )
}

fn snake_color(c: usize, total: usize) -> (i32, i32, i32) {
    let h = if total <= 1 {
        0.0
    } else {
        ((c - 1) % total) as f64 / total as f64
    };
    hsv_to_rgb(h, 0.65, 0.9)
}

fn round_rect(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
    fill: &str,
    stroke: &str,
    sw: f64,
) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("rx", r)
        .set("ry", r)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", sw)
}

fn capsule_rect(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: f64) -> Rectangle {
    let r = (w.min(h) / 2.0).max(1.0);
    round_rect(x, y, w, h, r, fill, stroke, sw)
}

fn dir_between(from: (usize, usize), to: (usize, usize)) -> Option<usize> {
    if from.0 == to.0 {
        if from.1 + 1 == to.1 {
            Some(3)
        } else if to.1 + 1 == from.1 {
            Some(2)
        } else {
            None
        }
    } else if from.1 == to.1 {
        if from.0 + 1 == to.0 {
            Some(1)
        } else if to.0 + 1 == from.0 {
            Some(0)
        } else {
            None
        }
    } else {
        None
    }
}

fn opposite_dir(dir: usize) -> usize {
    match dir {
        0 => 1,
        1 => 0,
        2 => 3,
        _ => 2,
    }
}

fn dir_unit(dir: usize) -> (f64, f64) {
    match dir {
        0 => (0.0, -1.0),
        1 => (0.0, 1.0),
        2 => (-1.0, 0.0),
        _ => (1.0, 0.0),
    }
}

fn boundary_point(center: (f64, f64), cell: f64, dir: usize) -> (f64, f64) {
    let (ux, uy) = dir_unit(dir);
    (center.0 + ux * cell * 0.5, center.1 + uy * cell * 0.5)
}

fn snake_cell_stroke_path(points: &[(f64, f64)], stroke: &str, sw: f64, linecap: &str) -> SvgPath {
    let mut data = Data::new();
    if let Some(&(x0, y0)) = points.first() {
        data = data.move_to((x0, y0));
        for &(x, y) in &points[1..] {
            data = data.line_to((x, y));
        }
    }
    SvgPath::new()
        .set("d", data)
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", sw)
        .set("stroke-linecap", linecap)
        .set("stroke-linejoin", "round")
}

fn snake_corner_fill_path(
    center: (f64, f64),
    cell: f64,
    dir_a: usize,
    dir_b: usize,
    half_w: f64,
    fill: &str,
) -> SvgPath {
    let (uax, uay) = dir_unit(dir_a);
    let (ubx, uby) = dir_unit(dir_b);

    let corner = (
        center.0 + (uax + ubx) * cell * 0.5,
        center.1 + (uay + uby) * cell * 0.5,
    );
    let mid_a = boundary_point(center, cell, dir_a);
    let mid_b = boundary_point(center, cell, dir_b);

    let va = (mid_a.0 - corner.0, mid_a.1 - corner.1);
    let vb = (mid_b.0 - corner.0, mid_b.1 - corner.1);
    let cross = va.0 * vb.1 - va.1 * vb.0;
    let sgn = if cross >= 0.0 { 1.0 } else { -1.0 };

    let rot = |v: (f64, f64), ccw: bool| -> (f64, f64) {
        if ccw {
            (-v.1, v.0)
        } else {
            (v.1, -v.0)
        }
    };

    let r_mid = cell * 0.5;
    let r_outer = r_mid + half_w;
    let r_inner = (r_mid - half_w).max(0.5);
    let scale = |v: (f64, f64), r: f64| -> (f64, f64) {
        let len = (v.0 * v.0 + v.1 * v.1).sqrt().max(1e-9);
        (v.0 / len * r, v.1 / len * r)
    };

    let oa_v = scale(va, r_outer);
    let ob_v = scale(vb, r_outer);
    let ia_v = scale(va, r_inner);
    let ib_v = scale(vb, r_inner);

    let outer_a = (corner.0 + oa_v.0, corner.1 + oa_v.1);
    let outer_b = (corner.0 + ob_v.0, corner.1 + ob_v.1);
    let inner_a = (corner.0 + ia_v.0, corner.1 + ia_v.1);
    let inner_b = (corner.0 + ib_v.0, corner.1 + ib_v.1);

    let k = 0.5522847498307936;
    let t_outer_a = rot(oa_v, sgn > 0.0);
    let t_outer_b = rot(ob_v, sgn > 0.0);
    let t_inner_b = rot(ib_v, sgn < 0.0);
    let t_inner_a = rot(ia_v, sgn < 0.0);

    let c1 = (outer_a.0 + t_outer_a.0 * k, outer_a.1 + t_outer_a.1 * k);
    let c2 = (outer_b.0 - t_outer_b.0 * k, outer_b.1 - t_outer_b.1 * k);
    let c3 = (inner_b.0 + t_inner_b.0 * k, inner_b.1 + t_inner_b.1 * k);
    let c4 = (inner_a.0 - t_inner_a.0 * k, inner_a.1 - t_inner_a.1 * k);

    let data = Data::new()
        .move_to(outer_a)
        .cubic_curve_to((c1.0, c1.1, c2.0, c2.1, outer_b.0, outer_b.1))
        .line_to(inner_b)
        .cubic_curve_to((c3.0, c3.1, c4.0, c4.1, inner_a.0, inner_a.1))
        .close();

    SvgPath::new()
        .set("d", data)
        .set("fill", fill)
        .set("stroke", "none")
}

fn rounded_taper_path(
    base_center: (f64, f64),
    dir: (f64, f64),
    base_half: f64,
    tip_half: f64,
    len: f64,
    fill: &str,
    stroke: &str,
    sw: f64,
) -> SvgPath {
    let (ux, uy) = dir;
    let vx = -uy;
    let vy = ux;

    let base_top = (
        base_center.0 + vx * base_half,
        base_center.1 + vy * base_half,
    );
    let base_bot = (
        base_center.0 - vx * base_half,
        base_center.1 - vy * base_half,
    );
    let tip_center = (base_center.0 + ux * len, base_center.1 + uy * len);

    let c1 = (base_top.0 + ux * len * 0.56, base_top.1 + uy * len * 0.56);
    let c2 = (
        tip_center.0 - ux * len * 0.10 + vx * tip_half,
        tip_center.1 - uy * len * 0.10 + vy * tip_half,
    );
    let c3 = (
        tip_center.0 - ux * len * 0.10 - vx * tip_half,
        tip_center.1 - uy * len * 0.10 - vy * tip_half,
    );
    let c4 = (base_bot.0 + ux * len * 0.56, base_bot.1 + uy * len * 0.56);

    let data = Data::new()
        .move_to(base_top)
        .cubic_curve_to((c1.0, c1.1, c2.0, c2.1, tip_center.0, tip_center.1))
        .cubic_curve_to((c3.0, c3.1, c4.0, c4.1, base_bot.0, base_bot.1))
        .close();

    SvgPath::new()
        .set("d", data)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", sw)
        .set("stroke-linejoin", "round")
        .set("stroke-linecap", "round")
}

fn head_cell_path(
    back_center: (f64, f64),
    dir: (f64, f64),
    base_half: f64,
    front_half: f64,
    len: f64,
    fill: &str,
    stroke: &str,
    sw: f64,
) -> SvgPath {
    let (ux, uy) = dir;
    let vx = -uy;
    let vy = ux;

    let back_top = (
        back_center.0 + vx * base_half,
        back_center.1 + vy * base_half,
    );
    let back_bot = (
        back_center.0 - vx * base_half,
        back_center.1 - vy * base_half,
    );
    let front_center = (back_center.0 + ux * len, back_center.1 + uy * len);
    let front_top = (
        front_center.0 + vx * front_half,
        front_center.1 + vy * front_half,
    );
    let front_bot = (
        front_center.0 - vx * front_half,
        front_center.1 - vy * front_half,
    );

    let c1 = (back_top.0 + ux * len * 0.56, back_top.1 + uy * len * 0.56);
    let c2 = (front_top.0 - ux * len * 0.22, front_top.1 - uy * len * 0.22);
    let c3 = (front_top.0 + ux * len * 0.32, front_top.1 + uy * len * 0.32);
    let c4 = (front_bot.0 + ux * len * 0.32, front_bot.1 + uy * len * 0.32);
    let c5 = (front_bot.0 - ux * len * 0.22, front_bot.1 - uy * len * 0.22);
    let c6 = (back_bot.0 + ux * len * 0.56, back_bot.1 + uy * len * 0.56);

    let data = Data::new()
        .move_to(back_top)
        .cubic_curve_to((c1.0, c1.1, c2.0, c2.1, front_top.0, front_top.1))
        .cubic_curve_to((c3.0, c3.1, c4.0, c4.1, front_bot.0, front_bot.1))
        .cubic_curve_to((c5.0, c5.1, c6.0, c6.1, back_bot.0, back_bot.1))
        .close();

    SvgPath::new()
        .set("d", data)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", sw)
        .set("stroke-linejoin", "round")
        .set("stroke-linecap", "round")
}

fn head_outline_path(
    back_center: (f64, f64),
    dir: (f64, f64),
    base_half: f64,
    front_half: f64,
    len: f64,
    stroke: &str,
    sw: f64,
) -> SvgPath {
    let (ux, uy) = dir;
    let vx = -uy;
    let vy = ux;

    let back_top = (
        back_center.0 + vx * base_half,
        back_center.1 + vy * base_half,
    );
    let back_bot = (
        back_center.0 - vx * base_half,
        back_center.1 - vy * base_half,
    );
    let front_center = (back_center.0 + ux * len, back_center.1 + uy * len);
    let front_top = (
        front_center.0 + vx * front_half,
        front_center.1 + vy * front_half,
    );
    let front_bot = (
        front_center.0 - vx * front_half,
        front_center.1 - vy * front_half,
    );

    let c1 = (back_top.0 + ux * len * 0.56, back_top.1 + uy * len * 0.56);
    let c2 = (front_top.0 - ux * len * 0.22, front_top.1 - uy * len * 0.22);
    let c3 = (front_top.0 + ux * len * 0.32, front_top.1 + uy * len * 0.32);
    let c4 = (front_bot.0 + ux * len * 0.32, front_bot.1 + uy * len * 0.32);
    let c5 = (front_bot.0 - ux * len * 0.22, front_bot.1 - uy * len * 0.22);
    let c6 = (back_bot.0 + ux * len * 0.56, back_bot.1 + uy * len * 0.56);

    let data = Data::new()
        .move_to(back_top)
        .cubic_curve_to((c1.0, c1.1, c2.0, c2.1, front_top.0, front_top.1))
        .cubic_curve_to((c3.0, c3.1, c4.0, c4.1, front_bot.0, front_bot.1))
        .cubic_curve_to((c5.0, c5.1, c6.0, c6.1, back_bot.0, back_bot.1));

    SvgPath::new()
        .set("d", data)
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", sw)
        .set("stroke-linejoin", "round")
        .set("stroke-linecap", "round")
}

fn tail_outline_path(
    base_center: (f64, f64),
    dir: (f64, f64),
    base_half: f64,
    tip_half: f64,
    len: f64,
    stroke: &str,
    sw: f64,
) -> SvgPath {
    let (ux, uy) = dir;
    let vx = -uy;
    let vy = ux;

    let base_top = (
        base_center.0 + vx * base_half,
        base_center.1 + vy * base_half,
    );
    let base_bot = (
        base_center.0 - vx * base_half,
        base_center.1 - vy * base_half,
    );
    let tip_center = (base_center.0 + ux * len, base_center.1 + uy * len);

    let c1 = (base_top.0 + ux * len * 0.56, base_top.1 + uy * len * 0.56);
    let c2 = (
        tip_center.0 - ux * len * 0.10 + vx * tip_half,
        tip_center.1 - uy * len * 0.10 + vy * tip_half,
    );
    let c3 = (
        tip_center.0 - ux * len * 0.10 - vx * tip_half,
        tip_center.1 - uy * len * 0.10 - vy * tip_half,
    );
    let c4 = (base_bot.0 + ux * len * 0.56, base_bot.1 + uy * len * 0.56);

    let data = Data::new()
        .move_to(base_top)
        .cubic_curve_to((c1.0, c1.1, c2.0, c2.1, tip_center.0, tip_center.1))
        .cubic_curve_to((c3.0, c3.1, c4.0, c4.1, base_bot.0, base_bot.1));

    SvgPath::new()
        .set("d", data)
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", sw)
        .set("stroke-linejoin", "round")
        .set("stroke-linecap", "round")
}

fn circle_el(cx: f64, cy: f64, r: f64, fill: &str, stroke: &str, sw: f64) -> Circle {
    Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", r)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", sw)
}

fn rel_luminance(c: (i32, i32, i32)) -> f64 {
    0.2126 * c.0 as f64 + 0.7152 * c.1 as f64 + 0.0722 * c.2 as f64
}

fn number_fill(c: (i32, i32, i32)) -> String {
    if rel_luminance(c) < 140.0 {
        rgb(255, 255, 255)
    } else {
        rgb(20, 20, 20)
    }
}

fn text_el(x: f64, y: f64, s: &str, size: f64, fill: &str) -> Text {
    Text::new(s.to_owned())
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("font-weight", "bold")
        .set("fill", fill)
}

#[derive(Clone, Copy, Debug)]
struct MappingLayout {
    square: i32,
    gap_x: i32,
    inner_gap_y: i32,
    row_gap: i32,
    pair_h: i32,
    row_pitch: i32,
    cols_per_row: i32,
    rows_needed: i32,
    content_h: i32,
    content_w: i32,
}

fn choose_mapping_layout(total: usize, w: i32, h: i32) -> MappingLayout {
    let margin = 6;
    for s in (2..=w.min(h)).rev() {
        let gap_x = (s as f64 * 0.20).round() as i32;
        let gap_x = gap_x.max(2);
        let inner_gap_y = (s as f64 * 0.20).round() as i32;
        let inner_gap_y = inner_gap_y.max(2);
        let row_gap = (s as f64 * 0.45).round() as i32;
        let row_gap = row_gap.max(6);
        let pair_h = s * 2 + inner_gap_y;
        let row_pitch = pair_h + row_gap;
        let cols = ((w - margin + gap_x) / (s + gap_x)).max(1);
        let rows = ((total as i32) + cols - 1) / cols;
        let content_h = rows * row_pitch - row_gap;
        let content_w = cols * (s + gap_x) - gap_x;
        if content_h + margin <= h && content_w + margin <= w {
            return MappingLayout {
                square: s,
                gap_x,
                inner_gap_y,
                row_gap,
                pair_h,
                row_pitch,
                cols_per_row: cols,
                rows_needed: rows,
                content_h,
                content_w,
            };
        }
    }
    MappingLayout {
        square: 2,
        gap_x: 2,
        inner_gap_y: 2,
        row_gap: 6,
        pair_h: 6,
        row_pitch: 12,
        cols_per_row: 1,
        rows_needed: total as i32,
        content_h: total as i32 * 12,
        content_w: 2,
    }
}

pub fn vis_default(input: &Input, out: &Output) -> (i64, String, String) {
    let (mut score, err, svg) = vis(input, &out.out, false);
    if !err.is_empty() {
        score = 0;
    }
    (score, err, svg)
}

pub fn vis(input: &Input, out: &[usize], show_number: bool) -> (i64, String, String) {
    const H: i32 = 600;
    const MARGIN: i32 = 12;
    const GAP_MID: i32 = 16;
    const RIGHT_W: i32 = 456;
    const LEFT_W: i32 = H;
    const W: i32 = LEFT_W + GAP_MID + RIGHT_W + MARGIN;
    let (score, err, state) = compute_score_details(input, out);

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (0, 0, W, H))
        .set("width", W)
        .set("height", H)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
        "text {text-anchor: middle;dominant-baseline: central;pointer-events: none;}".to_owned(),
    ));

    let left_w = LEFT_W;
    let left_margin = MARGIN;
    let board_size = H - 2 * left_margin;
    let board_x = left_margin;
    let board_y = left_margin;
    let cell = board_size as f64 / input.N as f64;

    for i in 0..input.N {
        for j in 0..input.N {
            let x = board_x as f64 + j as f64 * cell;
            let y = board_y as f64 + i as f64 * cell;
            let fill = if (i + j) % 2 == 0 {
                rgb(244, 245, 248)
            } else {
                rgb(238, 240, 244)
            };
            let title =
                if state.f[i][j] == 0 && !state.ij.iter().any(|&(ii, jj)| ii == i && jj == j) {
                    format!("coord=({}, {})", i, j)
                } else {
                    format!("coord=({}, {})", i, j)
                };
            doc = doc.add(
                group(title).add(
                    Rectangle::new()
                        .set("x", x)
                        .set("y", y)
                        .set("width", cell)
                        .set("height", cell)
                        .set("fill", fill)
                        .set("stroke", rgb(210, 214, 220))
                        .set("stroke-width", 1),
                ),
            );
        }
    }

    let center = |i: usize, j: usize| -> (f64, f64) {
        (
            board_x as f64 + (j as f64 + 0.5) * cell,
            board_y as f64 + (i as f64 + 0.5) * cell,
        )
    };

    for i in 0..input.N {
        for j in 0..input.N {
            let c = state.f[i][j];
            if c != 0 {
                let fill_rgb = snake_color(c, input.C);
                let stroke_rgb = mix_color(fill_rgb, (0, 0, 0), 0.35);
                let (cx, cy) = center(i, j);
                let title = format!("coord=({}, {}), color={}", i, j, c);
                let mut g = group(title).add(circle_el(
                    cx,
                    cy,
                    cell * 0.25,
                    &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                    &rgb(stroke_rgb.0, stroke_rgb.1, stroke_rgb.2),
                    2.0,
                ));
                if show_number {
                    g = g.add(text_el(
                        cx,
                        cy,
                        &c.to_string(),
                        (cell * 0.34).max(10.0),
                        &number_fill(fill_rgb),
                    ));
                }
                doc = doc.add(g);
            }
        }
    }

    if !state.ij.is_empty() {
        let points_head_to_tail: Vec<(f64, f64)> =
            state.ij.iter().map(|&(i, j)| center(i, j)).collect();
        let outer_w = cell * 0.82;
        let inner_w = cell * 0.68;

        for p in (0..state.ij.len()).rev() {
            let (i, j) = state.ij[p];
            let ctr = points_head_to_tail[p];
            let col = state.c[p];
            let fill_rgb = snake_color(col, input.C);
            let outline_rgb = mix_color(fill_rgb, (0, 0, 0), 0.45);
            let title = format!("coord=({}, {}), color={}, index={}", i, j, col, p);

            let dir_prev = if p >= 1 {
                dir_between(state.ij[p], state.ij[p - 1])
            } else {
                None
            };
            let dir_next = if p + 1 < state.ij.len() {
                dir_between(state.ij[p], state.ij[p + 1])
            } else {
                None
            };

            let mut g = group(title);

            if p == state.ij.len() - 1 && dir_prev.is_some() {
                let outward_dir = opposite_dir(dir_prev.unwrap());
                let (ux, uy) = dir_unit(outward_dir);
                let base_center = boundary_point(ctr, cell, dir_prev.unwrap());
                let tail_len = cell * 0.74;
                let tail_tip = inner_w * 0.04;
                let outline_sw = ((outer_w - inner_w) * 0.5).max(3.0);
                let outline_base_half = (inner_w + outer_w) * 0.25;
                let outline_tip_half = (tail_tip + outer_w * 0.06) * 0.5;
                g = g.add(rounded_taper_path(
                    base_center,
                    (ux, uy),
                    inner_w * 0.50,
                    tail_tip,
                    tail_len,
                    &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                    "none",
                    0.0,
                ));
                g = g.add(tail_outline_path(
                    base_center,
                    (ux, uy),
                    outline_base_half,
                    outline_tip_half,
                    tail_len,
                    &rgb(outline_rgb.0, outline_rgb.1, outline_rgb.2),
                    outline_sw,
                ));
                if show_number {
                    g = g.add(text_el(
                        ctr.0,
                        ctr.1,
                        &col.to_string(),
                        (cell * 0.30).max(9.0),
                        &number_fill(fill_rgb),
                    ));
                }
            } else if p == 0 && dir_next.is_some() {
                let forward_dir = opposite_dir(dir_next.unwrap());
                let (ux, uy) = dir_unit(forward_dir);
                let back_center = boundary_point(ctr, cell, dir_next.unwrap());
                let head_len = cell * 0.64;
                let outline_sw = ((outer_w - inner_w) * 0.5).max(3.0);
                let outline_base_half = (inner_w + outer_w) * 0.25;
                let outline_front_half = (inner_w * 0.23 + outer_w * 0.26) * 0.5;
                g = g.add(head_cell_path(
                    back_center,
                    (ux, uy),
                    inner_w * 0.50,
                    inner_w * 0.23,
                    head_len,
                    &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                    "none",
                    0.0,
                ));
                g = g.add(head_outline_path(
                    back_center,
                    (ux, uy),
                    outline_base_half,
                    outline_front_half,
                    head_len,
                    &rgb(outline_rgb.0, outline_rgb.1, outline_rgb.2),
                    outline_sw,
                ));

                let eye_center = (
                    back_center.0 + ux * (head_len * 0.74),
                    back_center.1 + uy * (head_len * 0.74),
                );
                let eye_sep = inner_w * 0.18;
                let eye_r = (cell * 0.065).max(2.0);
                let pupil_r = (cell * 0.03).max(1.0);
                for sgn in [-1.0_f64, 1.0_f64] {
                    let ex = eye_center.0 + (-uy) * eye_sep * sgn;
                    let ey = eye_center.1 + ux * eye_sep * sgn;
                    g = g.add(circle_el(ex, ey, eye_r, &rgb(255, 255, 255), "none", 0.0));
                    g = g.add(circle_el(
                        ex + ux * (eye_r * 0.28),
                        ey + uy * (eye_r * 0.28),
                        pupil_r,
                        &rgb(0, 0, 0),
                        "none",
                        0.0,
                    ));
                }
            } else {
                let mut pts = vec![];
                if let Some(d) = dir_prev {
                    pts.push(boundary_point(ctr, cell, d));
                }
                pts.push(ctr);
                if let Some(d) = dir_next {
                    pts.push(boundary_point(ctr, cell, d));
                }
                if pts.len() == 1 {
                    g = g.add(circle_el(
                        ctr.0,
                        ctr.1,
                        outer_w / 2.0,
                        &rgb(outline_rgb.0, outline_rgb.1, outline_rgb.2),
                        "none",
                        0.0,
                    ));
                    g = g.add(circle_el(
                        ctr.0,
                        ctr.1,
                        inner_w / 2.0,
                        &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                        "none",
                        0.0,
                    ));
                } else {
                    let is_corner = dir_prev.is_some()
                        && dir_next.is_some()
                        && opposite_dir(dir_prev.unwrap()) != dir_next.unwrap();
                    if is_corner {
                        g = g.add(snake_corner_fill_path(
                            ctr,
                            cell,
                            dir_prev.unwrap(),
                            dir_next.unwrap(),
                            outer_w * 0.5,
                            &rgb(outline_rgb.0, outline_rgb.1, outline_rgb.2),
                        ));
                        g = g.add(snake_corner_fill_path(
                            ctr,
                            cell,
                            dir_prev.unwrap(),
                            dir_next.unwrap(),
                            inner_w * 0.5,
                            &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                        ));
                    } else {
                        g = g.add(snake_cell_stroke_path(
                            &pts,
                            &rgb(outline_rgb.0, outline_rgb.1, outline_rgb.2),
                            outer_w,
                            "butt",
                        ));
                        g = g.add(snake_cell_stroke_path(
                            &pts,
                            &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                            inner_w,
                            "butt",
                        ));
                    }
                }
            }

            doc = doc.add(g);
        }

        if show_number {
            for (p, &(i, j)) in state.ij.iter().enumerate() {
                if p == 0 {
                    continue;
                }
                if p == state.ij.len().saturating_sub(1) {
                    continue;
                }
                let (cx, cy) = center(i, j);
                let col = state.c[p];
                let fill_rgb = snake_color(col, input.C);
                doc = doc.add(text_el(
                    cx,
                    cy,
                    &col.to_string(),
                    (cell * 0.30).max(9.0),
                    &number_fill(fill_rgb),
                ));
            }
        }
    }

    let right_x = left_w + GAP_MID;
    let right_y = board_y;
    let right_w = RIGHT_W;
    let right_h = board_size;
    let total = input.d.len();
    let layout = choose_mapping_layout(total, right_w, right_h);
    let start_x = right_x + ((right_w - layout.content_w).max(0) / 2);
    let start_y = right_y + ((right_h - layout.content_h).max(0) / 2);

    for r in 1..layout.rows_needed {
        let block_bottom = start_y + (r - 1) * layout.row_pitch + layout.pair_h;
        let yline = block_bottom as f64 + layout.row_gap as f64 / 2.0;
        doc = doc.add(capsule_rect(
            (start_x - 2) as f64,
            yline - 1.0,
            (layout.content_w + 4) as f64,
            2.0,
            &rgb(210, 214, 222),
            "none",
            0.0,
        ));
    }

    for idx in 0..total {
        let block_row = idx as i32 / layout.cols_per_row;
        let block_col = idx as i32 % layout.cols_per_row;
        let bx = start_x + block_col * (layout.square + layout.gap_x);
        let by = start_y + block_row * layout.row_pitch;
        let cur = state.c.get(idx).copied();
        let tgt = input.d.get(idx).copied();
        let matched = cur.is_some() && cur == tgt;

        if matched {
            let bg_rgb = mix_color(snake_color(cur.unwrap(), input.C), (255, 255, 255), 0.78);
            let bd_rgb = mix_color(bg_rgb, (0, 0, 0), 0.10);
            doc = doc.add(round_rect(
                (bx - 2) as f64,
                (by - 2) as f64,
                (layout.square + 4) as f64,
                (layout.square * 2 + layout.inner_gap_y + 4) as f64,
                (layout.square / 5).max(3) as f64,
                &rgb(bg_rgb.0, bg_rgb.1, bg_rgb.2),
                &rgb(bd_rgb.0, bd_rgb.1, bd_rgb.2),
                1.0,
            ));
        }

        if let Some(c) = cur {
            let fill_rgb = snake_color(c, input.C);
            let stroke_rgb = mix_color(fill_rgb, (0, 0, 0), 0.30);
            let sw = (layout.square / 10).max(1) as f64;
            let mut g = group(format!("index={}, color={}", idx, c)).add(round_rect(
                bx as f64,
                by as f64,
                layout.square as f64,
                layout.square as f64,
                (layout.square / 6).max(2) as f64,
                &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                &rgb(stroke_rgb.0, stroke_rgb.1, stroke_rgb.2),
                sw,
            ));
            if show_number {
                g = g.add(text_el(
                    bx as f64 + layout.square as f64 / 2.0,
                    by as f64 + layout.square as f64 / 2.0,
                    &c.to_string(),
                    (layout.square as f64 * 0.62).max(8.0),
                    &number_fill(fill_rgb),
                ));
            }
            doc = doc.add(g);
        } else {
            doc = doc.add(group(format!("index={}, color=-", idx)).add(round_rect(
                bx as f64,
                by as f64,
                layout.square as f64,
                layout.square as f64,
                (layout.square / 6).max(2) as f64,
                &rgb(245, 245, 247),
                &rgb(210, 210, 216),
                1.0,
            )));
        }

        let by2 = by + layout.square + layout.inner_gap_y;
        if let Some(c) = tgt {
            let fill_rgb = snake_color(c, input.C);
            let stroke_rgb = mix_color(fill_rgb, (0, 0, 0), 0.30);
            let sw = (layout.square / 10).max(1) as f64;
            let mut g = group(format!("index={}, color={}", idx, c)).add(round_rect(
                bx as f64,
                by2 as f64,
                layout.square as f64,
                layout.square as f64,
                (layout.square / 6).max(2) as f64,
                &rgb(fill_rgb.0, fill_rgb.1, fill_rgb.2),
                &rgb(stroke_rgb.0, stroke_rgb.1, stroke_rgb.2),
                sw,
            ));
            if show_number {
                g = g.add(text_el(
                    bx as f64 + layout.square as f64 / 2.0,
                    by2 as f64 + layout.square as f64 / 2.0,
                    &c.to_string(),
                    (layout.square as f64 * 0.62).max(8.0),
                    &number_fill(fill_rgb),
                ));
            }
            doc = doc.add(g);
        } else {
            doc = doc.add(group(format!("index={}, color=-", idx)).add(round_rect(
                bx as f64,
                by2 as f64,
                layout.square as f64,
                layout.square as f64,
                (layout.square / 6).max(2) as f64,
                &rgb(245, 245, 247),
                &rgb(210, 210, 216),
                1.0,
            )));
        }
    }

    (score, err, doc.to_string())
}
