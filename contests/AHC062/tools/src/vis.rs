use crate::*;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use image::{ImageBuffer, Rgb};
use svg::node::element::{Circle, Group, Image, Rectangle, Style, Title};

/// 0 <= val <= 1
pub fn color(val: f64) -> String {
    let (r, g, b) = color_rgb(val);
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// 0 <= val <= 1
pub fn color_rgb(mut val: f64) -> (u8, u8, u8) {
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
    let alpha = 0.75;
    let r = 255.0 * (1.0 - alpha) + r * alpha;
    let g = 255.0 * (1.0 - alpha) + g * alpha;
    let b = 255.0 * (1.0 - alpha) + b * alpha;

    (r.round() as u8, g.round() as u8, b.round() as u8)
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
    let (mut score, err, svg) = vis(input, &out.out, None, 'A');
    if err.len() > 0 {
        score = 0;
    }
    (score, err, svg)
}

fn clip_line_to_rect(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let p = [-dx, dx, -dy, dy];
    let q = [x0 - xmin, xmax - x0, y0 - ymin, ymax - y0];

    let mut t0 = 0.0;
    let mut t1 = 1.0;

    for k in 0..4 {
        if p[k] == 0.0 {
            if q[k] < 0.0 {
                return None;
            }
        } else {
            let r = q[k] / p[k];
            if p[k] < 0.0 {
                if r > t1 {
                    return None;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return None;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }

    Some(((x0 + dx * t0, y0 + dy * t0), (x0 + dx * t1, y0 + dy * t1)))
}

fn global_map_data_uri(input: &Input, used_turn: &Vec<Vec<isize>>, color_mode: char) -> String {
    let n = input.N;
    let n2 = n * n;

    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(n as u32, n as u32);

    for i in 0..n {
        for j in 0..n {
            let (r, g, b) = match color_mode {
                'A' => {
                    let v = (input.A[i][j] - 1) as f64 / (n2 as f64 - 1.0);
                    color_rgb(v)
                }
                'k' => {
                    if used_turn[i][j] >= 0 {
                        let v = used_turn[i][j] as f64 / ((n2 - 1).max(1)) as f64;
                        color_rgb(v)
                    } else {
                        (0xee, 0xee, 0xee)
                    }
                }
                _ => {
                    let v = (input.A[i][j] - 1) as f64 / (n2 as f64 - 1.0);
                    color_rgb(v)
                }
            };
            img.put_pixel(j as u32, i as u32, Rgb([r, g, b]));
        }
    }

    let dynimg = image::DynamicImage::ImageRgb8(img);
    let mut bytes = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut bytes);
        dynimg
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
    }

    let encoded = BASE64_STANDARD.encode(bytes);
    format!("data:image/png;base64,{}", encoded)
}

pub fn vis(
    input: &Input,
    out: &[(usize, usize)],
    focus: Option<(usize, usize)>, // None の場合は最終位置
    color_mode: char,              // 'A' or 'k'
) -> (i64, String, String) {
    let H = 920usize;
    let outer = 20usize;
    let gap = 20usize;
    let title_h = 28usize;
    let top_panel_side = 420usize;
    let W = outer * 2 + top_panel_side * 2 + gap;

    let (score, err, _) = compute_score_details(input, &out);
    let tmax = out.len();
    let (cur_i, cur_j) = if tmax > 0 { out[tmax - 1] } else { (0, 0) };
    let n = input.N;
    let n2 = n * n;
    let zoom_n = 25usize.min(n);

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (0, 0, W, H))
        .set("width", W)
        .set("height", H)
        .set("data-current-i", cur_i)
        .set("data-current-j", cur_j)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
    r#"
text { text-anchor: middle; font-family: sans-serif; }
.title { font-size: 20px; font-weight: 700; fill: #222; }
.gridline { stroke: #d8d8d8; stroke-width: 1; }
.axis { stroke: #333333; stroke-width: 1.5; fill: none; }
.pathline { stroke: #111111; stroke-width: 2; fill: none; stroke-linecap: round; stroke-linejoin: round; pointer-events: none; }
.focus_frame { stroke: #000000; stroke-width: 2; fill: none; pointer-events: none; }
.cell_border { stroke: #000000; stroke-width: 0.3; fill: none; pointer-events: none; }
.panel_border { stroke: #444444; stroke-width: 1; fill: none; pointer-events: none; }
.band1 { fill: #9ecae1; opacity: 0.35; stroke: none; }
.band2 { fill: #fdd0a2; opacity: 0.45; stroke: none; }
.meanline { stroke: #f28e2b; stroke-width: 2; fill: none; }
circle { pointer-events: none; }
.global-image { cursor: pointer; }
.focused-cell { cursor: default; }
"#,
));

    doc = doc.add(
        Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", W)
            .set("height", H)
            .set("fill", "white"),
    );

    let mut used_turn = mat![-1isize; n; n];
    for (t, &(i, j)) in out.iter().enumerate() {
        if i < n && j < n {
            used_turn[i][j] = t as isize;
        }
    }

    let (fi, fj) = if let Some((i, j)) = focus {
        (i.min(n - 1), j.min(n - 1))
    } else if tmax > 0 {
        out[tmax - 1]
    } else {
        (0, 0)
    };

    let half = zoom_n as isize / 2;
    let si = (fi as isize - half)
        .max(0)
        .min((n as isize - zoom_n as isize).max(0)) as usize;
    let sj = (fj as isize - half)
        .max(0)
        .min((n as isize - zoom_n as isize).max(0)) as usize;

    // ============================================================
    // layout
    // ============================================================
    let p1w = top_panel_side;
    let p1h = top_panel_side;
    let p2w = top_panel_side;
    let p2h = top_panel_side;

    let p1x = outer;
    let p1y = outer + title_h;

    let p2x = p1x + p1w + gap;
    let p2y = p1y;

    let p3x = outer;
    let p3y = p1y + p1h + 40;
    let p3w = W - outer * 2;
    let p3h = H - p3y - outer;

    // タイトル
    doc = doc
        .add(
            svg::node::element::Text::new("Global map")
                .set("x", p1x + p1w / 2)
                .set("y", outer + 20)
                .set("class", "title"),
        )
        .add(
            svg::node::element::Text::new("Focused map")
                .set("x", p2x + 170)
                .set("y", outer + 20)
                .set("class", "title")
                .set("text-anchor", "middle"),
        )
        .add(
            svg::node::element::Text::new(format!(
                "[{:03}, {:03}) × [{:03}, {:03})",
                si,
                si + zoom_n,
                sj,
                sj + zoom_n
            ))
            .set("x", p2x + 330)
            .set("y", outer + 20)
            .set("font-size", 13)
            .set("fill", "#444")
            .set("font-family", "monospace")
            .set("text-anchor", "start"),
        )
        .add(
            svg::node::element::Text::new("A[i_k][j_k] Plot (mean, 25–75%, min–max)")
                .set("x", p3x + p3w / 2)
                .set("y", p3y - 10)
                .set("class", "title"),
        );

    // パネル枠
    doc = doc
        .add(
            Rectangle::new()
                .set("x", p1x)
                .set("y", p1y)
                .set("width", p1w)
                .set("height", p1h)
                .set("class", "panel_border")
                .set("fill", "white"),
        )
        .add(
            Rectangle::new()
                .set("x", p2x)
                .set("y", p2y)
                .set("width", p2w)
                .set("height", p2h)
                .set("class", "panel_border")
                .set("fill", "white"),
        )
        .add(
            Rectangle::new()
                .set("x", p3x)
                .set("y", p3y)
                .set("width", p3w)
                .set("height", p3h)
                .set("class", "panel_border")
                .set("fill", "white"),
        );

    // ============================================================
    // 1. Global map
    // ============================================================
    let mut g1 = Group::new().set("id", "global-map");
    let pad1 = 10usize;
    let grid_x = p1x + pad1;
    let grid_y = p1y + pad1;
    let grid_w = p1w - pad1 * 2;
    let grid_h = p1h - pad1 * 2;

    let side1 = (grid_w.min(grid_h) as f64) / n as f64;
    let actual_w1 = side1 * n as f64;
    let actual_h1 = side1 * n as f64;
    let offset_x1 = grid_x as f64 + (grid_w as f64 - actual_w1) * 0.5;
    let offset_y1 = grid_y as f64 + (grid_h as f64 - actual_h1) * 0.5;

    // raster image for global map
    let data_uri = global_map_data_uri(input, &used_turn, color_mode);

    g1 = g1.add(
        Image::new()
            .set("x", offset_x1)
            .set("y", offset_y1)
            .set("width", actual_w1)
            .set("height", actual_h1)
            .set("preserveAspectRatio", "none")
            .set("image-rendering", "pixelated")
            .set("class", "global-image")
            .set("data-role", "global-image")
            .set("data-grid-x", offset_x1)
            .set("data-grid-y", offset_y1)
            .set("data-grid-size", actual_w1)
            .set("data-n", n)
            .set("href", data_uri),
    );

    let fx = offset_x1 + sj as f64 * side1;
    let fy = offset_y1 + si as f64 * side1;
    let fw = zoom_n as f64 * side1;
    let fh = zoom_n as f64 * side1;
    g1 = g1.add(
        Rectangle::new()
            .set("x", fx)
            .set("y", fy)
            .set("width", fw)
            .set("height", fh)
            .set("class", "focus_frame"),
    );

    if tmax > 0 {
        let (si0, sj0) = out[0];
        let cx = offset_x1 + (sj0 as f64 + 0.5) * side1;
        let cy = offset_y1 + (si0 as f64 + 0.5) * side1;
        g1 = g1.add(
            Circle::new()
                .set("cx", cx)
                .set("cy", cy)
                .set("r", 3)
                .set("fill", "#00aa00"),
        );

        let (ei, ej) = out[tmax - 1];
        let cx = offset_x1 + (ej as f64 + 0.5) * side1;
        let cy = offset_y1 + (ei as f64 + 0.5) * side1;
        g1 = g1.add(
            Circle::new()
                .set("cx", cx)
                .set("cy", cy)
                .set("r", 3)
                .set("fill", "#cc0000"),
        );
    }

    doc = doc.add(g1);

    // ============================================================
    // 2. Focused map
    // ============================================================
    let mut g2 = Group::new().set("id", "focused-map");

    let pad2 = 10usize;
    let gx = p2x + pad2;
    let gy = p2y + pad2;
    let gw = p2w - pad2 * 2;
    let gh = p2h - pad2 * 2;

    let side2 = (gw.min(gh) as f64) / zoom_n as f64;
    let actual_w2 = side2 * zoom_n as f64;
    let actual_h2 = side2 * zoom_n as f64;
    let offset_x2 = gx as f64 + (gw as f64 - actual_w2) * 0.5;
    let offset_y2 = gy as f64 + (gh as f64 - actual_h2) * 0.5;

    for di in 0..zoom_n {
        for dj in 0..zoom_n {
            let i = si + di;
            let j = sj + dj;
            let fill = match color_mode {
                'A' => {
                    let v = (input.A[i][j] - 1) as f64 / (n2 as f64 - 1.0);
                    color(v)
                }
                'k' => {
                    if used_turn[i][j] >= 0 {
                        let v = used_turn[i][j] as f64 / ((n2 - 1).max(1)) as f64;
                        color(v)
                    } else {
                        "#eeeeee".to_string()
                    }
                }
                _ => {
                    let v = (input.A[i][j] - 1) as f64 / (n2 as f64 - 1.0);
                    color(v)
                }
            };
            let x = offset_x2 + dj as f64 * side2;
            let y = offset_y2 + di as f64 * side2;
            let title = match used_turn[i][j] {
                -1 => format!("({}, {}) A={} unvisited", i, j, input.A[i][j]),
                t => format!("({}, {}) A={} k={}", i, j, input.A[i][j], t),
            };
            g2 = g2.add(
                group(title)
                    .add(
                        Rectangle::new()
                            .set("x", x)
                            .set("y", y)
                            .set("width", side2)
                            .set("height", side2)
                            .set("fill", fill)
                            .set("class", "focused-cell"),
                    )
                    .add(
                        Rectangle::new()
                            .set("x", x)
                            .set("y", y)
                            .set("width", side2)
                            .set("height", side2)
                            .set("class", "cell_border"),
                    ),
            );
        }
    }

    if tmax >= 2 {
        let xmin = offset_x2;
        let ymin = offset_y2;
        let xmax = offset_x2 + actual_w2;
        let ymax = offset_y2 + actual_h2;

        for t in 0..(tmax - 1) {
            let (i0, j0) = out[t];
            let (i1, j1) = out[t + 1];

            let x0 = offset_x2 + (j0 as f64 - sj as f64 + 0.5) * side2;
            let y0 = offset_y2 + (i0 as f64 - si as f64 + 0.5) * side2;
            let x1 = offset_x2 + (j1 as f64 - sj as f64 + 0.5) * side2;
            let y1 = offset_y2 + (i1 as f64 - si as f64 + 0.5) * side2;

            if let Some(((cx0, cy0), (cx1, cy1))) =
                clip_line_to_rect(x0, y0, x1, y1, xmin, ymin, xmax, ymax)
            {
                let data = svg::node::element::path::Data::new()
                    .move_to((cx0, cy0))
                    .line_to((cx1, cy1));
                g2 = g2.add(
                    svg::node::element::Path::new()
                        .set("d", data)
                        .set("class", "pathline"),
                );
            }
        }
    }

    if tmax > 0 {
        let (i0, j0) = out[0];
        if si <= i0 && i0 < si + zoom_n && sj <= j0 && j0 < sj + zoom_n {
            let cx = offset_x2 + (j0 - sj) as f64 * side2 + side2 * 0.5;
            let cy = offset_y2 + (i0 - si) as f64 * side2 + side2 * 0.5;
            g2 = g2.add(
                svg::node::element::Circle::new()
                    .set("cx", cx)
                    .set("cy", cy)
                    .set("r", (side2 * 0.22).max(2.0))
                    .set("fill", "#00aa00"),
            );
        }
        let (ie, je) = out[tmax - 1];
        if si <= ie && ie < si + zoom_n && sj <= je && je < sj + zoom_n {
            let cx = offset_x2 + (je - sj) as f64 * side2 + side2 * 0.5;
            let cy = offset_y2 + (ie - si) as f64 * side2 + side2 * 0.5;
            g2 = g2.add(
                svg::node::element::Circle::new()
                    .set("cx", cx)
                    .set("cy", cy)
                    .set("r", (side2 * 0.22).max(2.0))
                    .set("fill", "#cc0000"),
            );
        }
    }

    doc = doc.add(g2);

    // ============================================================
    // 3. A-k trend
    // ============================================================
    let mut g3 = Group::new();

    let graph_margin_x = 58usize;
    let graph_margin_y = 34usize;

    let graph_w = p3w - graph_margin_x * 2;
    let graph_h = p3h - graph_margin_y * 2;
    let graph_x = p3x + (p3w - graph_w) / 2 + 20;
    let graph_y = p3y + (p3h - graph_h) / 2;

    g3 = g3.add(
        Rectangle::new()
            .set("x", graph_x)
            .set("y", graph_y)
            .set("width", graph_w)
            .set("height", graph_h)
            .set("fill", "white")
            .set("stroke", "#dddddd"),
    );

    let x0 = graph_x as f64;
    let y0 = (graph_y + graph_h) as f64;
    let x1 = (graph_x + graph_w) as f64;
    let y1 = graph_y as f64;

    g3 = g3
        .add(
            svg::node::element::Line::new()
                .set("x1", x0)
                .set("y1", y0)
                .set("x2", x1)
                .set("y2", y0)
                .set("class", "axis"),
        )
        .add(
            svg::node::element::Line::new()
                .set("x1", x0)
                .set("y1", y0)
                .set("x2", x0)
                .set("y2", y1)
                .set("class", "axis"),
        );

    let sx = |k: f64| -> f64 { x0 + graph_w as f64 * k / n2 as f64 };
    let sy = |a: f64| -> f64 { y0 - graph_h as f64 * a / n2 as f64 };

    // grid + ticks: x axis
    for r in 0..=4 {
        let kval = (n2 as f64 * r as f64 / 4.0).round() as usize;
        let x = sx(kval as f64);
        g3 = g3
            .add(
                svg::node::element::Line::new()
                    .set("x1", x)
                    .set("y1", y0)
                    .set("x2", x)
                    .set("y2", y1)
                    .set("class", "gridline"),
            )
            .add(
                svg::node::element::Line::new()
                    .set("x1", x)
                    .set("y1", y0)
                    .set("x2", x)
                    .set("y2", y0 + 6.0)
                    .set("stroke", "#333333")
                    .set("stroke-width", 1.2),
            )
            .add(
                svg::node::element::Text::new(format!("{}", kval))
                    .set("x", x)
                    .set("y", y0 + 16.0)
                    .set("font-size", 11)
                    .set("fill", "#333333"),
            );
    }

    // grid + ticks: y axis
    for r in 0..=4 {
        let aval = n2 as f64 * r as f64 / 4.0;
        let y = sy(aval);
        g3 = g3
            .add(
                svg::node::element::Line::new()
                    .set("x1", x0)
                    .set("y1", y)
                    .set("x2", x1)
                    .set("y2", y)
                    .set("class", "gridline"),
            )
            .add(
                svg::node::element::Line::new()
                    .set("x1", x0 - 6.0)
                    .set("y1", y)
                    .set("x2", x0)
                    .set("y2", y)
                    .set("stroke", "#333333")
                    .set("stroke-width", 1.2),
            )
            .add(
                svg::node::element::Text::new(format!("{}", aval.round() as usize))
                    .set("x", x0 - 26.0)
                    .set("y", y)
                    .set("font-size", 11)
                    .set("fill", "#333333"),
            );
    }

    // axis labels
    g3 = g3
        .add(
            svg::node::element::Text::new("k")
                .set("x", (x0 + x1) * 0.5)
                .set("y", y0 + 26.0)
                .set("font-size", 13)
                .set("fill", "#222222"),
        )
        .add(
            svg::node::element::Text::new("A[i_k][j_k]")
                .set("x", x0 - 50.0)
                .set("y", (y0 + y1) * 0.5)
                .set(
                    "transform",
                    format!("rotate(-90, {}, {})", x0 - 50.0, (y0 + y1) * 0.5),
                )
                .set("font-size", 13)
                .set("fill", "#222222"),
        );

    let bucket_count = 320usize.min(n2.max(1));
    let mut bucket_vals: Vec<Vec<i64>> = vec![vec![]; bucket_count];
    for t in 0..tmax {
        let b = (t * bucket_count / n2).min(bucket_count - 1);
        let (i, j) = out[t];
        bucket_vals[b].push(input.A[i][j]);
    }

    let mut minv = vec![0.0; bucket_count];
    let mut q25 = vec![0.0; bucket_count];
    let mut mean = vec![0.0; bucket_count];
    let mut q75 = vec![0.0; bucket_count];
    let mut maxv = vec![0.0; bucket_count];
    let mut nonempty = vec![false; bucket_count];

    for b in 0..bucket_count {
        if bucket_vals[b].is_empty() {
            continue;
        }
        nonempty[b] = true;
        bucket_vals[b].sort();
        let v = &bucket_vals[b];
        let m = v.len();
        let pick = |p: f64| -> f64 {
            if m == 1 {
                return v[0] as f64;
            }
            let idx = ((m - 1) as f64 * p).round() as usize;
            v[idx] as f64
        };
        minv[b] = v[0] as f64;
        q25[b] = pick(0.25);
        q75[b] = pick(0.75);
        maxv[b] = v[m - 1] as f64;
        mean[b] = v.iter().map(|&x| x as f64).sum::<f64>() / m as f64;
    }

    let build_band = |low: &Vec<f64>, high: &Vec<f64>, class_name: &str, mut g: Group| -> Group {
        let mut l = 0usize;
        while l < bucket_count {
            while l < bucket_count && !nonempty[l] {
                l += 1;
            }
            if l >= bucket_count {
                break;
            }
            let mut r = l;
            while r + 1 < bucket_count && nonempty[r + 1] {
                r += 1;
            }
            let mut data = svg::node::element::path::Data::new();
            let xl = sx((l as f64 + 0.5) * n2 as f64 / bucket_count as f64);
            data = data.move_to((xl, sy(low[l])));
            for b in l..=r {
                let x = sx((b as f64 + 0.5) * n2 as f64 / bucket_count as f64);
                data = data.line_to((x, sy(low[b])));
            }
            for b in (l..=r).rev() {
                let x = sx((b as f64 + 0.5) * n2 as f64 / bucket_count as f64);
                data = data.line_to((x, sy(high[b])));
            }
            data = data.close();
            g = g.add(
                svg::node::element::Path::new()
                    .set("d", data)
                    .set("class", class_name),
            );
            l = r + 1;
        }
        g
    };

    g3 = build_band(&minv, &maxv, "band1", g3);
    g3 = build_band(&q25, &q75, "band2", g3);

    // mean line
    {
        let mut l = 0usize;
        while l < bucket_count {
            while l < bucket_count && !nonempty[l] {
                l += 1;
            }
            if l >= bucket_count {
                break;
            }
            let mut r = l;
            while r + 1 < bucket_count && nonempty[r + 1] {
                r += 1;
            }
            let mut data = svg::node::element::path::Data::new();
            let x = sx((l as f64 + 0.5) * n2 as f64 / bucket_count as f64);
            data = data.move_to((x, sy(mean[l])));
            for b in (l + 1)..=r {
                let x = sx((b as f64 + 0.5) * n2 as f64 / bucket_count as f64);
                data = data.line_to((x, sy(mean[b])));
            }
            g3 = g3.add(
                svg::node::element::Path::new()
                    .set("d", data)
                    .set("class", "meanline"),
            );
            l = r + 1;
        }
    }

    doc = doc.add(g3);

    (score, err, doc.to_string())
}
