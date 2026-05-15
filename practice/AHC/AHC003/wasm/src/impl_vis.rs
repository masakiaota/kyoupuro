use rand::prelude::*;

const N: usize = 30;
const Q: usize = 1000;
const INF: i32 = 1_000_000_000;

type Output = Vec<String>;

#[derive(Clone, Debug)]
struct Input {
    h: Vec<Vec<i32>>,
    v: Vec<Vec<i32>>,
    s: Vec<(usize, usize)>,
    t: Vec<(usize, usize)>,
    a: Vec<i32>,
    e: Vec<f64>,
}

fn escaped_text(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_input(raw: &str) -> Result<Input, String> {
    let mut tokens = raw.split_whitespace();
    let mut next = |name: &str| -> Result<&str, String> {
        tokens
            .next()
            .ok_or_else(|| format!("input is missing token: {name}"))
    };

    let mut h = vec![vec![0; N - 1]; N];
    for row in h.iter_mut().take(N) {
        for value in row.iter_mut().take(N - 1) {
            *value = next("h")?
                .parse::<i32>()
                .map_err(|e| format!("failed to parse h: {e}"))?;
        }
    }

    let mut v = vec![vec![0; N]; N - 1];
    for row in v.iter_mut().take(N - 1) {
        for value in row.iter_mut().take(N) {
            *value = next("v")?
                .parse::<i32>()
                .map_err(|e| format!("failed to parse v: {e}"))?;
        }
    }

    let mut s = Vec::with_capacity(Q);
    let mut t = Vec::with_capacity(Q);
    let mut a = Vec::with_capacity(Q);
    let mut e = Vec::with_capacity(Q);
    for k in 0..Q {
        let si = next("si")?
            .parse::<usize>()
            .map_err(|err| format!("failed to parse si at query {}: {err}", k + 1))?;
        let sj = next("sj")?
            .parse::<usize>()
            .map_err(|err| format!("failed to parse sj at query {}: {err}", k + 1))?;
        let ti = next("ti")?
            .parse::<usize>()
            .map_err(|err| format!("failed to parse ti at query {}: {err}", k + 1))?;
        let tj = next("tj")?
            .parse::<usize>()
            .map_err(|err| format!("failed to parse tj at query {}: {err}", k + 1))?;
        let ak = next("a")?
            .parse::<i32>()
            .map_err(|err| format!("failed to parse a at query {}: {err}", k + 1))?;
        let ek = next("e")?
            .parse::<f64>()
            .map_err(|err| format!("failed to parse e at query {}: {err}", k + 1))?;
        if si >= N || sj >= N || ti >= N || tj >= N {
            return Err(format!("query {} has vertex outside the map", k + 1));
        }
        s.push((si, sj));
        t.push((ti, tj));
        a.push(ak);
        e.push(ek);
    }

    Ok(Input { h, v, s, t, a, e })
}

fn input_to_string(input: &Input) -> String {
    let mut out = String::new();
    for i in 0..N {
        for j in 0..N - 1 {
            if j > 0 {
                out.push(' ');
            }
            out.push_str(&input.h[i][j].to_string());
        }
        out.push('\n');
    }
    for i in 0..N - 1 {
        for j in 0..N {
            if j > 0 {
                out.push(' ');
            }
            out.push_str(&input.v[i][j].to_string());
        }
        out.push('\n');
    }
    for k in 0..Q {
        out.push_str(&format!(
            "{} {} {} {} {} {}\n",
            input.s[k].0, input.s[k].1, input.t[k].0, input.t[k].1, input.a[k], input.e[k]
        ));
    }
    out
}

fn read_output(raw: &str) -> Output {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn dist(p: (usize, usize), q: (usize, usize)) -> usize {
    p.0.abs_diff(q.0) + p.1.abs_diff(q.1)
}

fn neighbors(p: (usize, usize)) -> impl Iterator<Item = (usize, (usize, usize), char)> {
    let mut next = Vec::with_capacity(4);
    if p.0 > 0 {
        next.push((0, (p.0 - 1, p.1), 'U'));
    }
    if p.1 > 0 {
        next.push((1, (p.0, p.1 - 1), 'L'));
    }
    if p.0 + 1 < N {
        next.push((2, (p.0 + 1, p.1), 'D'));
    }
    if p.1 + 1 < N {
        next.push((3, (p.0, p.1 + 1), 'R'));
    }
    next.into_iter()
}

fn edge_length(input: &Input, p: (usize, usize), dir: char) -> i32 {
    match dir {
        'U' => input.v[p.0 - 1][p.1],
        'L' => input.h[p.0][p.1 - 1],
        'D' => input.v[p.0][p.1],
        'R' => input.h[p.0][p.1],
        _ => unreachable!(),
    }
}

fn compute_path_length(
    input: &Input,
    k: usize,
    path: &str,
    visited: &mut [Vec<usize>],
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
                sum += input.v[p.0 - 1][p.1];
                p.0 -= 1;
            }
            'L' => {
                if p.1 == 0 {
                    return Err(format!("going outside the map (query {})", k + 1));
                }
                sum += input.h[p.0][p.1 - 1];
                p.1 -= 1;
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
            _ => return Err(format!("unexpected char: {c}")),
        }
        ps.push(p);
    }
    if p != input.t[k] {
        return Err(format!("not an s-t path (query {})", k + 1));
    }
    Ok((sum, ps))
}

fn compute_score_detail(input: &Input, out: &Output) -> (i64, String) {
    let mut score = 0.0;
    let mut visited = vec![vec![usize::MAX; N]; N];
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
    ((score * 2_312_311.0).round() as i64, String::new())
}

fn compute_shortest_path(input: &Input, s: (usize, usize), t: (usize, usize)) -> (String, i32) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let mut d = vec![vec![INF; N]; N];
    let mut prev = vec![vec![None::<(usize, usize, char)>; N]; N];
    let mut heap = BinaryHeap::new();
    d[s.0][s.1] = 0;
    heap.push((Reverse(0), s));

    while let Some((Reverse(cost), p)) = heap.pop() {
        if cost != d[p.0][p.1] {
            continue;
        }
        if p == t {
            break;
        }
        for (_, q, dir) in neighbors(p) {
            let next_cost = cost + edge_length(input, p, dir);
            if next_cost < d[q.0][q.1] {
                d[q.0][q.1] = next_cost;
                prev[q.0][q.1] = Some((p.0, p.1, dir));
                heap.push((Reverse(next_cost), q));
            }
        }
    }

    let mut p = t;
    let mut out = Vec::new();
    while p != s {
        let Some((pi, pj, dir)) = prev[p.0][p.1] else {
            return (String::new(), INF);
        };
        out.push(dir);
        p = (pi, pj);
    }
    out.reverse();
    (out.into_iter().collect(), d[t.0][t.1])
}

fn gen_input(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let d = rng.gen_range(100..=2000);
    let m = rng.gen_range(1..=2usize);

    let h_base = (0..N)
        .map(|_| {
            (0..m)
                .map(|_| rng.gen_range((1000 + d)..=(9000 - d)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut x = vec![vec![0]; N];
    for row in x.iter_mut().take(N) {
        if m == 2 {
            row.push(rng.gen_range(1..N - 1));
        }
        row.push(N - 1);
    }
    let mut h = vec![vec![0; N - 1]; N];
    for i in 0..N {
        for p in 0..m {
            for j in x[i][p]..x[i][p + 1] {
                h[i][j] = h_base[i][p] + rng.gen_range(-d..=d);
            }
        }
    }

    let v_base = (0..N)
        .map(|_| {
            (0..m)
                .map(|_| rng.gen_range((1000 + d)..=(9000 - d)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut y = vec![vec![0]; N];
    for col in y.iter_mut().take(N) {
        if m == 2 {
            col.push(rng.gen_range(1..N - 1));
        }
        col.push(N - 1);
    }
    let mut v = vec![vec![0; N]; N - 1];
    for j in 0..N {
        for p in 0..m {
            for i in y[j][p]..y[j][p + 1] {
                v[i][j] = v_base[j][p] + rng.gen_range(-d..=d);
            }
        }
    }

    let mut input = Input {
        h,
        v,
        s: Vec::with_capacity(Q),
        t: Vec::with_capacity(Q),
        a: Vec::with_capacity(Q),
        e: Vec::with_capacity(Q),
    };
    for _ in 0..Q {
        let mut sk = (0, 0);
        let mut tk = (0, 0);
        while dist(sk, tk) < 10 {
            sk = (rng.gen_range(0..N), rng.gen_range(0..N));
            tk = (rng.gen_range(0..N), rng.gen_range(0..N));
        }
        input.s.push(sk);
        input.t.push(tk);
        input.a.push(compute_shortest_path(&input, sk, tk).1);
        input.e.push(rng.gen_range(0.9..1.1));
    }
    input
}

fn color(val: f64) -> String {
    let val = val.clamp(0.0, 1.0);
    let tmp = ((-(2.0 * std::f64::consts::PI * val).cos() / 2.0 + 0.5) * 255.0) as i32;
    if val >= 0.5 {
        format!("#{:02x}{:02x}{:02x}", 255, 0, tmp)
    } else {
        format!("#{:02x}{:02x}{:02x}", tmp, 0, 255)
    }
}

fn svg_text(x: usize, y: usize, size: usize, s: &str) -> String {
    format!(
        r#"<text x="{x}" y="{y}" font-size="{size}" text-anchor="middle" font-family="sans-serif">{}</text>"#,
        escaped_text(s)
    )
}

fn svg_path(points: &[(usize, usize)], offset: isize, stroke: &str, width: usize) -> String {
    if points.is_empty() {
        return String::new();
    }
    const S: isize = 30;
    let mut d = format!(
        "M {} {}",
        20 + points[0].1 as isize * S + S / 2 + offset,
        20 + points[0].0 as isize * S + S / 2 + offset
    );
    for p in points.iter().skip(1) {
        d.push_str(&format!(
            " L {} {}",
            20 + p.1 as isize * S + S / 2 + offset,
            20 + p.0 as isize * S + S / 2 + offset
        ));
    }
    format!(r#"<path fill="none" stroke="{stroke}" stroke-width="{width}" d="{d}"/>"#)
}

fn render_svg(input: &Input, out: &Output, show_k: usize, err: &mut String) -> String {
    const S: usize = 30;
    const CHART_H: usize = 300;
    let map_h = 20 + N * S;
    let width = 20 + N * S + 20;
    let height = map_h + CHART_H;
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg id="vis" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#
    ));
    svg.push_str(&format!(
        r#"<rect x="0" y="0" width="{width}" height="{height}" fill="white"/>"#
    ));

    for i in 0..N {
        svg.push_str(&svg_text(16, 20 + S * i + S / 2 + 7, 16, &i.to_string()));
        svg.push_str(&svg_text(20 + S * i + S / 2, 24, 16, &i.to_string()));
    }
    for i in 0..N {
        for j in 0..N {
            if j + 1 < N {
                let x1 = 20 + S * j + S / 2;
                let y1 = 20 + S * i + S / 2;
                let x2 = x1 + S;
                let stroke = color((input.h[i][j] as f64 - 1000.0) / 8000.0);
                svg.push_str(&format!(
                    r#"<path stroke="{stroke}" stroke-width="3" d="M {x1} {y1} L {x2} {y1}"/>"#
                ));
            }
            if i + 1 < N {
                let x1 = 20 + S * j + S / 2;
                let y1 = 20 + S * i + S / 2;
                let y2 = y1 + S;
                let stroke = color((input.v[i][j] as f64 - 1000.0) / 8000.0);
                svg.push_str(&format!(
                    r#"<path stroke="{stroke}" stroke-width="3" d="M {x1} {y1} L {x1} {y2}"/>"#
                ));
            }
        }
    }

    if show_k < Q {
        let (path_a, a) = compute_shortest_path(input, input.s[show_k], input.t[show_k]);
        let path_a = compute_path_length(input, show_k, &path_a, &mut vec![vec![usize::MAX; N]; N])
            .map(|(_, path)| path)
            .unwrap_or_default();
        if !err.is_empty() {
            err.push('\n');
        }
        err.push_str(&format!("query {}: a = {}", show_k + 1, a));
        svg.push_str(&svg_path(&path_a, 8, "forestgreen", 6));

        if show_k < out.len() {
            if let Ok((b, path_b)) = compute_path_length(
                input,
                show_k,
                &out[show_k],
                &mut vec![vec![usize::MAX; N]; N],
            ) {
                err.push_str(&format!("  b = {}", b));
                svg.push_str(&svg_path(&path_b, -8, "chocolate", 6));
            }
        }

        let (si, sj) = input.s[show_k];
        let (ti, tj) = input.t[show_k];
        let sx = 20 + sj * S + S / 2;
        let sy = 20 + si * S + S / 2;
        let tx = 20 + tj * S + S / 2;
        let ty = 20 + ti * S + S / 2;
        svg.push_str(&format!(
            r##"<circle cx="{sx}" cy="{sy}" r="8" fill="#16a34a"/>"##
        ));
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="8" fill="#dc2626"/>"##
        ));
    }

    for i in 0..N {
        for j in 0..N {
            let cx = 20 + j * S + S / 2;
            let cy = 20 + i * S + S / 2;
            svg.push_str(&format!(
                r#"<circle cx="{cx}" cy="{cy}" r="5" fill="black"/>"#
            ));
        }
    }

    svg.push_str(&format!(
        r#"<rect x="40" y="{}" width="{}" height="{}" fill="none" stroke="black" stroke-width="2"/>"#,
        map_h + 20,
        S * N - 40,
        CHART_H - 60
    ));
    svg.push_str(&svg_text(20, map_h + CHART_H / 2, 20, "a/b"));
    svg.push_str(&svg_text(20, map_h + 30, 20, "1"));
    svg.push_str(&svg_text(20, map_h + CHART_H - 30, 20, "0"));
    svg.push_str(&svg_text(20 + S * N / 2, map_h + CHART_H - 15, 20, "k"));
    svg.push_str(&svg_text(40, map_h + CHART_H - 15, 20, "1"));
    svg.push_str(&svg_text(
        20 + S * N - S / 2,
        map_h + CHART_H - 15,
        20,
        &Q.to_string(),
    ));

    let mut used = vec![vec![usize::MAX; N]; N];
    for k in 0..Q.min(out.len()) {
        if let Ok((b, _)) = compute_path_length(input, k, &out[k], &mut used) {
            let ab = (input.a[k] as f64 / b as f64).clamp(0.0, 1.0);
            let cx = 40 + (S * N - 40) * k / (Q - 1);
            let cy = map_h + CHART_H - 40 - ((CHART_H - 60) as f64 * ab).round() as usize;
            let r = if k == show_k { 6 } else { 3 };
            let fill = if k == show_k { "blue" } else { "red" };
            svg.push_str(&format!(
                r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" fill-opacity="0.5"/>"#
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

pub fn generate(seed: i32) -> String {
    input_to_string(&gen_input(seed.max(0) as u64))
}

pub fn calc_max_turn(input: &str, _output: &str) -> usize {
    if parse_input(input).is_ok() {
        Q - 1
    } else {
        0
    }
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let input = parse_input(input)?;
    let output = read_output(output);
    let (score, mut err) = compute_score_detail(&input, &output);
    let show_k = turn.min(Q - 1);
    let svg = render_svg(&input, &output, show_k, &mut err);
    Ok((score, err, svg))
}
