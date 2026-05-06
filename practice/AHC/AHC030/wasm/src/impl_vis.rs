use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use svg::node::element::{Circle, Group, Line, Polyline, Rectangle, Style, Text as SvgText, Title};
use svg::node::Text;

const DIRS: [(isize, isize); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
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
}

#[derive(Clone, Debug)]
struct Input {
    n: usize,
    m: usize,
    eps: f64,
    ts: Vec<Vec<(usize, usize)>>,
    ps: Vec<(usize, usize)>,
    ans: Vec<Vec<i32>>,
    es: Vec<f64>,
}

fn join_cells(cells: &[(usize, usize)]) -> String {
    cells
        .iter()
        .map(|&(i, j)| format!("{} {}", i, j))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_input(input: &Input) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} {} {:.2}\n", input.n, input.m, input.eps));
    for shape in &input.ts {
        out.push_str(&format!("{} {}\n", shape.len(), join_cells(shape)));
    }
    for &(i, j) in &input.ps {
        out.push_str(&format!("{} {}\n", i, j));
    }
    for row in &input.ans {
        out.push_str(
            &row.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
        out.push('\n');
    }
    for &e in &input.es {
        out.push_str(&format!("{:.10}\n", e));
    }
    out
}

fn next_parse<T>(it: &mut std::str::SplitWhitespace<'_>, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    let token = it.next().ok_or_else(|| format!("Unexpected EOF while reading {}", name))?;
    token
        .parse::<T>()
        .map_err(|_| format!("Parse error in {}: {}", name, token))
}

fn parse_input(raw: &str) -> Result<Input, String> {
    let mut it = raw.split_whitespace();
    let n: usize = next_parse(&mut it, "N")?;
    let m: usize = next_parse(&mut it, "M")?;
    let eps: f64 = next_parse(&mut it, "eps")?;
    if !(10..=20).contains(&n) {
        return Err(format!("N is out of range: {}", n));
    }
    if !(2..=20).contains(&m) {
        return Err(format!("M is out of range: {}", m));
    }

    let mut ts = Vec::with_capacity(m);
    for oil_id in 0..m {
        let d: usize = next_parse(&mut it, "shape size")?;
        let mut shape = Vec::with_capacity(d);
        for _ in 0..d {
            let i: usize = next_parse(&mut it, "shape i")?;
            let j: usize = next_parse(&mut it, "shape j")?;
            if i >= n || j >= n {
                return Err(format!("shape {} contains out-of-board cell ({}, {})", oil_id, i, j));
            }
            shape.push((i, j));
        }
        ts.push(shape);
    }

    let mut ps = Vec::with_capacity(m);
    for oil_id in 0..m {
        let i: usize = next_parse(&mut it, "placement i")?;
        let j: usize = next_parse(&mut it, "placement j")?;
        let max_i = ts[oil_id].iter().map(|p| p.0).max().unwrap_or(0);
        let max_j = ts[oil_id].iter().map(|p| p.1).max().unwrap_or(0);
        if i + max_i >= n || j + max_j >= n {
            return Err(format!("placement {} is out of board: ({}, {})", oil_id, i, j));
        }
        ps.push((i, j));
    }

    let mut ans = vec![vec![0; n]; n];
    for i in 0..n {
        for j in 0..n {
            ans[i][j] = next_parse(&mut it, "answer grid")?;
        }
    }

    let mut es = Vec::with_capacity(n * n * 2);
    for _ in 0..n * n * 2 {
        es.push(next_parse(&mut it, "noise")?);
    }

    Ok(Input {
        n,
        m,
        eps,
        ts,
        ps,
        ans,
        es,
    })
}

#[derive(Clone, Debug)]
enum Query {
    Survey(Vec<(usize, usize)>),
    Mining((usize, usize)),
    Ans(Vec<(usize, usize)>),
}

#[derive(Clone, Debug)]
struct Sim {
    eps: f64,
    ans: Vec<Vec<i32>>,
    es: Vec<f64>,
    query: Vec<Query>,
    resp: Vec<i32>,
    cost: f64,
    costs: Vec<f64>,
    count: usize,
    mined: Vec<Vec<usize>>,
    finished: bool,
}

impl Sim {
    fn new(input: &Input) -> Self {
        let count = input.ans.iter().flatten().filter(|&&v| v > 0).count();
        Self {
            eps: input.eps,
            ans: input.ans.clone(),
            es: input.es.clone(),
            query: vec![],
            resp: vec![],
            cost: 0.0,
            costs: vec![0.0],
            count,
            mined: vec![vec![usize::MAX; input.n]; input.n],
            finished: false,
        }
    }

    fn query(&mut self, q: Query) -> i32 {
        let resp = match q {
            Query::Mining((i, j)) => {
                self.cost += 1.0;
                self.mined[i][j].setmin(self.resp.len());
                self.ans[i][j]
            }
            Query::Survey(ref ps) => {
                self.cost += 1.0 / (ps.len() as f64).sqrt();
                let sum = ps.iter().map(|&(i, j)| self.ans[i][j]).sum::<i32>();
                let k = ps.len() as f64;
                let mu = (k - sum as f64) * self.eps + sum as f64 * (1.0 - self.eps);
                let sigma = (k * self.eps * (1.0 - self.eps)).sqrt();
                let e = self.es.get(self.resp.len()).copied().unwrap_or(0.0);
                ((mu + e * sigma).round() as i32).max(0)
            }
            Query::Ans(ref ps) => {
                if ps.len() == self.count && ps.iter().all(|&(i, j)| self.ans[i][j] > 0) {
                    self.finished = true;
                    1
                } else {
                    self.cost += 1.0;
                    0
                }
            }
        };
        self.query.push(q);
        self.resp.push(resp);
        self.costs.push(self.cost);
        resp
    }
}

struct Output {
    sim: Sim,
    comments: Vec<String>,
}

fn read_value<T>(
    token: Option<&str>,
    lb: T,
    ub: T,
    label: &str,
) -> Result<T, String>
where
    T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr,
{
    let Some(raw) = token else {
        return Err(format!("Unexpected EOF while reading {}", label));
    };
    let Ok(value) = raw.parse::<T>() else {
        return Err(format!("Parse error in {}: {}", label, raw));
    };
    if value < lb || ub < value {
        return Err(format!("{} is out of range: {}", label, value));
    }
    Ok(value)
}

fn parse_points<'a>(
    ss: &mut impl Iterator<Item = &'a str>,
    n: usize,
    len: usize,
) -> Result<Vec<(usize, usize)>, String> {
    let mut ps = Vec::with_capacity(len);
    for _ in 0..len {
        let i = read_value(ss.next(), 0, n - 1, "i")?;
        let j = read_value(ss.next(), 0, n - 1, "j")?;
        ps.push((i, j));
    }
    ps.sort();
    ps.dedup();
    if ps.len() != len {
        return Err("Query contains the same square multiple times.".to_owned());
    }
    Ok(ps)
}

fn parse_output(input: &Input, raw: &str) -> Result<Output, String> {
    let mut sim = Sim::new(input);
    let mut comments = vec![];
    let mut comment = String::new();

    for line in raw.lines() {
        if sim.resp.len() >= 2 * input.n * input.n {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            comment.push_str(line.trim_start_matches('#').trim());
            comment.push('\n');
            continue;
        }

        comments.push(std::mem::take(&mut comment));
        let mut ss = line.split_whitespace();
        let ty = ss
            .next()
            .ok_or_else(|| format!("Invalid query format: {}", line))?;
        let num = read_value(ss.next(), 1, input.n * input.n, "query size")?;

        match ty {
            "a" => {
                let ps = parse_points(&mut ss, input.n, num)?;
                if ss.next().is_some() {
                    return Err(format!("Invalid query format: {}", line));
                }
                let resp = sim.query(Query::Ans(ps));
                if resp == 1 {
                    break;
                }
            }
            "q" => {
                if num == 1 {
                    let i = read_value(ss.next(), 0, input.n - 1, "i")?;
                    let j = read_value(ss.next(), 0, input.n - 1, "j")?;
                    if ss.next().is_some() {
                        return Err(format!("Invalid query format: {}", line));
                    }
                    sim.query(Query::Mining((i, j)));
                } else {
                    let ps = parse_points(&mut ss, input.n, num)?;
                    if ss.next().is_some() {
                        return Err(format!("Invalid query format: {}", line));
                    }
                    sim.query(Query::Survey(ps));
                }
            }
            _ => return Err(format!("Invalid query format: {}", line)),
        }
    }

    Ok(Output { sim, comments })
}

fn gen_case(seed: u64) -> Input {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let n = rng.gen_range(10..=20);
    let m = rng.gen_range(2..=n * n / 20);
    let eps = rng.gen_range(1..=20) as f64 / 100.0;
    let avg = (rng.gen_range(n * n / 5..=n * n / 2) / m).max(4);
    let delta = rng.gen_range(0..=avg - 4);

    let mut ts = vec![];
    for _ in 0..m {
        let size = rng.gen_range(avg - delta..=avg + delta);
        let mut used = vec![vec![false; n]; n];
        let mut list = vec![(n / 2, n / 2)];
        used[n / 2][n / 2] = true;
        let mut adj = vec![];
        for (di, dj) in DIRS {
            let i = (n / 2) as isize + di;
            let j = (n / 2) as isize + dj;
            if 0 <= i && i < n as isize && 0 <= j && j < n as isize {
                adj.push((i as usize, j as usize));
                used[i as usize][j as usize] = true;
            }
        }

        while list.len() < size {
            let p = rng.gen_range(0..adj.len());
            let (i, j) = adj.remove(p);
            list.push((i, j));
            for (di, dj) in DIRS {
                let i2 = i as isize + di;
                let j2 = j as isize + dj;
                if 0 <= i2
                    && i2 < n as isize
                    && 0 <= j2
                    && j2 < n as isize
                    && !used[i2 as usize][j2 as usize]
                {
                    adj.push((i2 as usize, j2 as usize));
                    used[i2 as usize][j2 as usize] = true;
                }
            }
        }

        let min_i = list.iter().map(|p| p.0).min().unwrap();
        let min_j = list.iter().map(|p| p.1).min().unwrap();
        for p in &mut list {
            p.0 -= min_i;
            p.1 -= min_j;
        }
        list.sort();
        ts.push(list);
    }

    let mut ans = vec![vec![0; n]; n];
    let mut ps = vec![];
    for shape in &ts {
        let max_i = shape.iter().map(|p| p.0).max().unwrap();
        let max_j = shape.iter().map(|p| p.1).max().unwrap();
        let di = rng.gen_range(0..=n - max_i - 1);
        let dj = rng.gen_range(0..=n - max_j - 1);
        for &(i, j) in shape {
            ans[i + di][j + dj] += 1;
        }
        ps.push((di, dj));
    }

    let es = (0..n * n * 2)
        .map(|_| rng.sample(rand_distr::StandardNormal))
        .collect();

    Input {
        n,
        m,
        eps,
        ts,
        ps,
        ans,
        es,
    }
}

fn compute_score_details(input: &Input, out: &Output) -> (i64, String) {
    let mut cost = out.sim.cost;
    let mut error = String::new();
    if !out.sim.finished {
        cost = 1000.0;
        if out.sim.resp.len() < 2 * input.n * input.n {
            error = "Unexpected EOF".to_owned();
        }
    }
    ((1e6 * cost.max(1.0 / input.n as f64)).round() as i64, error)
}

fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

fn group(title: String) -> Group {
    Group::new().add(Title::new().add(Text::new(title)))
}

fn add_shape_panel(
    mut doc: svg::Document,
    input: &Input,
    panel_left: i32,
    panel_w: usize,
    panel_h: usize,
) -> svg::Document {
    let title_h = 24.0;
    let slot_h = ((panel_h as f64 - title_h) / input.m.max(1) as f64).max(1.0);
    doc = doc.add(
        Rectangle::new()
            .set("x", panel_left)
            .set("y", 0)
            .set("width", panel_w)
            .set("height", panel_h)
            .set("fill", "#ffffff")
            .set("stroke", "#cbd5e1")
            .set("stroke-width", 1),
    );
    doc = doc.add(
        SvgText::new()
            .add(Text::new("Oil fields"))
            .set("x", panel_left + panel_w as i32 / 2)
            .set("y", 13)
            .set("font-size", 12)
            .set("font-weight", 700)
            .set("fill", "#263241"),
    );

    let label_w = 28.0;
    let usable_w = (panel_w as f64 - label_w - 18.0).max(1.0);
    let usable_h = (slot_h - 6.0).max(1.0);
    let mut max_shape_h = 1usize;
    let mut max_shape_w = 1usize;
    for shape in &input.ts {
        let min_i = shape.iter().map(|p| p.0).min().unwrap_or(0);
        let min_j = shape.iter().map(|p| p.1).min().unwrap_or(0);
        let max_i = shape.iter().map(|p| p.0).max().unwrap_or(0);
        let max_j = shape.iter().map(|p| p.1).max().unwrap_or(0);
        max_shape_h = max_shape_h.max(max_i - min_i + 1);
        max_shape_w = max_shape_w.max(max_j - min_j + 1);
    }
    let cell = (usable_w / max_shape_w as f64)
        .min(usable_h / max_shape_h as f64)
        .max(1.0);

    for (oil_id, shape) in input.ts.iter().enumerate() {
        let min_i = shape.iter().map(|p| p.0).min().unwrap_or(0);
        let min_j = shape.iter().map(|p| p.1).min().unwrap_or(0);
        let max_i = shape.iter().map(|p| p.0).max().unwrap_or(0);
        let max_j = shape.iter().map(|p| p.1).max().unwrap_or(0);
        let shape_h = (max_i - min_i + 1).max(1) as f64;
        let shape_w = (max_j - min_j + 1).max(1) as f64;
        let top = title_h + oil_id as f64 * slot_h;
        let shape_px_w = shape_w * cell;
        let shape_px_h = shape_h * cell;
        let x0 = panel_left as f64 + label_w + 8.0 + (usable_w - shape_px_w) / 2.0;
        let y0 = top + (slot_h - shape_px_h) / 2.0;

        doc = doc.add(
            Line::new()
                .set("x1", panel_left)
                .set("y1", top)
                .set("x2", panel_left + panel_w as i32)
                .set("y2", top)
                .set("stroke", "#e5e7eb")
                .set("stroke-width", 1),
        );
        doc = doc.add(
            SvgText::new()
                .add(Text::new(format!("#{}", oil_id)))
                .set("x", panel_left + 14)
                .set("y", top + slot_h / 2.0)
                .set("font-size", 10)
                .set("fill", "#5b6472"),
        );

        let mut g = group(format!("oil field #{} size={}", oil_id, shape.len()));
        for &(i, j) in shape {
            let x = x0 + (j - min_j) as f64 * cell;
            let y = y0 + (i - min_i) as f64 * cell;
            g = g.add(
                Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", "#bae6fd")
                    .set("stroke", "#0369a1")
                    .set("stroke-width", 0.6),
            );
        }
        doc = doc.add(g);
    }

    doc
}

fn render(input: &Input, out: &Output, turn: usize, show_ans: bool) -> (i64, String, String) {
    let d = (600 / input.n).max(1);
    let w = d * input.n;
    let h = d * input.n;
    let shape_panel_w = 172;
    let shape_panel_gap = 16;
    let shape_panel_left = -((shape_panel_w + shape_panel_gap) as i32);
    let (score, err) = compute_score_details(input, out);
    let turn = turn.min(out.sim.resp.len());
    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set(
            "viewBox",
            (
                shape_panel_left - 5,
                -5,
                shape_panel_w + shape_panel_gap + w + 10,
                h + 10,
            ),
        )
        .set("width", shape_panel_w + shape_panel_gap + w + 10)
        .set("height", h + 10)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
        "text {text-anchor: middle;dominant-baseline: central;}".to_owned(),
    ));
    doc = add_shape_panel(doc, input, shape_panel_left, shape_panel_w, h);

    let mut colors = vec![vec!["white".to_owned(); input.n]; input.n];
    for comment in out.comments.iter().take(turn) {
        for line in comment.lines() {
            if let Some(rest) = line.strip_prefix('c') {
                let ss = rest.trim().split_whitespace().collect::<Vec<_>>();
                if ss.len() == 3 {
                    if let (Ok(i), Ok(j)) = (ss[0].parse::<usize>(), ss[1].parse::<usize>()) {
                        if i < input.n && j < input.n {
                            colors[i][j] = ss[2].to_owned();
                        }
                    }
                }
            }
        }
    }

    for i in 0..input.n {
        for j in 0..input.n {
            if out.sim.mined[i][j] < turn && colors[i][j] == "white" {
                colors[i][j] = "#e5e7eb".to_owned();
            }
        }
    }

    if turn > 0 {
        match &out.sim.query[turn - 1] {
            Query::Mining((i, j)) => {
                colors[*i][*j] = "tomato".to_owned();
            }
            Query::Survey(ps) => {
                for &(i, j) in ps {
                    colors[i][j] = "tomato".to_owned();
                }
            }
            Query::Ans(ps) => {
                for &(i, j) in ps {
                    colors[i][j] = "skyblue".to_owned();
                }
            }
        }
    }

    for i in 0..input.n {
        for j in 0..input.n {
            let mut g = group(format!("({}, {})", i, j)).add(rect(j * d, i * d, d, d, &colors[i][j]));
            if out.sim.mined[i][j] < turn {
                g = g.add(
                    SvgText::new()
                        .add(Text::new(format!("{}", out.sim.ans[i][j])))
                        .set("x", (j * d + d / 2) as i32)
                        .set("y", (i * d + d / 2) as i32)
                        .set("font-size", d / 3)
                        .set("fill", "black"),
                );
            } else if show_ans && out.sim.ans[i][j] > 0 {
                g = g.add(
                    SvgText::new()
                        .add(Text::new(format!("{}", out.sim.ans[i][j])))
                        .set("x", (j * d + d / 2) as i32)
                        .set("y", (i * d + d / 2) as i32)
                        .set("font-size", d / 3)
                        .set("fill", "darkgray"),
                );
            }
            doc = doc.add(g);
        }
    }

    if show_ans {
        for oil_id in 0..input.m {
            let (base_i, base_j) = input.ps[oil_id];
            let mut inside = vec![vec![false; input.n]; input.n];
            for &(i, j) in &input.ts[oil_id] {
                inside[i + base_i][j + base_j] = true;
            }
            for i in 0..input.n {
                for j in 0..input.n {
                    if !inside[i][j] {
                        continue;
                    }
                    for (di, dj) in DIRS {
                        let i2 = i as isize + di;
                        let j2 = j as isize + dj;
                        let outside = i2 < 0
                            || i2 >= input.n as isize
                            || j2 < 0
                            || j2 >= input.n as isize
                            || !inside[i2 as usize][j2 as usize];
                        if outside {
                            let cx = (j * d + d / 2) as i32;
                            let cy = (i * d + d / 2) as i32;
                            let r = (d / 2).saturating_sub(3) as i32;
                            doc = doc.add(
                                Line::new()
                                    .set("x1", cx + (dj as i32 - di as i32) * r)
                                    .set("y1", cy + (di as i32 + dj as i32) * r)
                                    .set("x2", cx + (dj as i32 + di as i32) * r)
                                    .set("y2", cy + (di as i32 - dj as i32) * r)
                                    .set("stroke", "green")
                                    .set("stroke-width", 2),
                            );
                        }
                    }
                }
            }
        }
    }

    for i in 0..=input.n {
        doc = doc.add(
            Line::new()
                .set("x1", 0)
                .set("y1", i * d)
                .set("x2", w)
                .set("y2", i * d)
                .set("stroke", "black")
                .set("stroke-width", 2),
        );
        doc = doc.add(
            Line::new()
                .set("x1", i * d)
                .set("y1", 0)
                .set("x2", i * d)
                .set("y2", h)
                .set("stroke", "black")
                .set("stroke-width", 2),
        );
    }

    (score, err, doc.to_string())
}

fn format_cost(value: f64) -> String {
    if value >= 100.0 {
        format!("{:.1}", value)
    } else if value >= 10.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    }
}

fn render_cost_graph(out: &Output, turn: usize) -> String {
    let width = 860usize;
    let height = 190usize;
    let left = 58.0;
    let right = 18.0;
    let top = 20.0;
    let bottom = 40.0;
    let plot_w = width as f64 - left - right;
    let plot_h = height as f64 - top - bottom;
    let costs = &out.sim.costs;
    let max_turn = costs.len().saturating_sub(1).max(1);
    let current_turn = turn.min(costs.len().saturating_sub(1));
    let current_cost = costs.get(current_turn).copied().unwrap_or(0.0);
    let prev_cost = if current_turn > 0 {
        costs[current_turn - 1]
    } else {
        0.0
    };
    let delta_cost = current_cost - prev_cost;
    let max_cost = costs
        .iter()
        .copied()
        .fold(0.0f64, f64::max)
        .max(1.0)
        * 1.08;

    let x_of = |t: usize| left + (t as f64 / max_turn as f64) * plot_w;
    let y_of = |cost: f64| top + plot_h - (cost / max_cost) * plot_h;
    let mut doc = svg::Document::new()
        .set("id", "cost-graph")
        .set("viewBox", (0, 0, width, height))
        .set("width", width)
        .set("height", height)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
        "text {font-family: sans-serif; fill: #263241;} .muted {fill: #5b6472;} .grid {stroke: #e5e7eb;} .axis {stroke: #94a3b8;} .line {fill: none; stroke: #155e63; stroke-width: 2.5;} .marker {stroke: #155e63; stroke-width: 1.8; fill: white;}".to_owned(),
    ));

    doc = doc.add(
        SvgText::new()
            .add(Text::new(format!(
                "Cost by turn  current={}  +{}",
                format_cost(current_cost),
                format_cost(delta_cost)
            )))
            .set("x", left as i32)
            .set("y", 14)
            .set("font-size", 12)
            .set("text-anchor", "start"),
    );

    for k in 0..=4 {
        let value = max_cost * k as f64 / 4.0;
        let y = y_of(value);
        doc = doc.add(
            Line::new()
                .set("x1", left)
                .set("y1", y)
                .set("x2", left + plot_w)
                .set("y2", y)
                .set("class", "grid"),
        );
        doc = doc.add(
            SvgText::new()
                .add(Text::new(format_cost(value)))
                .set("x", (left - 8.0) as i32)
                .set("y", y + 4.0)
                .set("font-size", 11)
                .set("text-anchor", "end")
                .set("class", "muted"),
        );
    }

    for &t in &[0usize, current_turn, max_turn] {
        let x = x_of(t);
        doc = doc.add(
            Line::new()
                .set("x1", x)
                .set("y1", top)
                .set("x2", x)
                .set("y2", top + plot_h)
                .set("class", "grid"),
        );
        doc = doc.add(
            SvgText::new()
                .add(Text::new(format!("{}", t)))
                .set("x", x)
                .set("y", top + plot_h + 18.0)
                .set("font-size", 11)
                .set("text-anchor", "middle")
                .set("class", "muted"),
        );
    }

    doc = doc.add(
        Line::new()
            .set("x1", left)
            .set("y1", top + plot_h)
            .set("x2", left + plot_w)
            .set("y2", top + plot_h)
            .set("class", "axis"),
    );
    doc = doc.add(
        Line::new()
            .set("x1", left)
            .set("y1", top)
            .set("x2", left)
            .set("y2", top + plot_h)
            .set("class", "axis"),
    );

    let points = costs
        .iter()
        .enumerate()
        .map(|(t, &cost)| format!("{:.2},{:.2}", x_of(t), y_of(cost)))
        .collect::<Vec<_>>()
        .join(" ");
    doc = doc.add(
        Polyline::new()
            .set("points", points)
            .set("class", "line")
            .add(Title::new().add(Text::new("cumulative cost"))),
    );

    for t in 1..costs.len() {
        let delta = costs[t] - costs[t - 1];
        if delta <= 0.0 {
            continue;
        }
        let x = x_of(t);
        let y0 = y_of(costs[t - 1]);
        let y1 = y_of(costs[t]);
        doc = doc.add(
            Line::new()
                .set("x1", x)
                .set("y1", y0)
                .set("x2", x)
                .set("y2", y1)
                .set("stroke", if t == current_turn { "#dc2626" } else { "#94a3b8" })
                .set("stroke-width", if t == current_turn { 3.0 } else { 1.2 })
                .set("opacity", if t == current_turn { 0.95 } else { 0.45 })
                .add(Title::new().add(Text::new(format!(
                    "turn {}: +{} cost={}",
                    t,
                    format_cost(delta),
                    format_cost(costs[t])
                )))),
        );
    }

    let current_x = x_of(current_turn);
    let current_y = y_of(current_cost);
    doc = doc.add(
        Line::new()
            .set("x1", current_x)
            .set("y1", top)
            .set("x2", current_x)
            .set("y2", top + plot_h)
            .set("stroke", "#dc2626")
            .set("stroke-width", 1.4)
            .set("stroke-dasharray", "4 4"),
    );
    doc = doc.add(
        Circle::new()
            .set("cx", current_x)
            .set("cy", current_y)
            .set("r", 4.5)
            .set("class", "marker")
            .add(Title::new().add(Text::new(format!(
                "turn {}: cost {} (+{})",
                current_turn,
                format_cost(current_cost),
                format_cost(delta_cost)
            )))),
    );
    doc = doc.add(
        SvgText::new()
            .add(Text::new("turn"))
            .set("x", (left + plot_w / 2.0) as i32)
            .set("y", (height - 8) as i32)
            .set("font-size", 11)
            .set("text-anchor", "middle")
            .set("class", "muted"),
    );
    doc = doc.add(
        SvgText::new()
            .add(Text::new("cost"))
            .set("x", 12)
            .set("y", (top + plot_h / 2.0) as i32)
            .set("font-size", 11)
            .set("text-anchor", "middle")
            .set("transform", format!("rotate(-90 12 {})", top + plot_h / 2.0))
            .set("class", "muted"),
    );

    doc.to_string()
}

fn counted_turns(output: &str) -> usize {
    output
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .count()
}

pub fn generate(seed: i32) -> String {
    format_input(&gen_case(seed.max(0) as u64))
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    let Ok(input) = parse_input(input) else {
        return counted_turns(output);
    };
    parse_output(&input, output)
        .map(|out| out.sim.resp.len())
        .unwrap_or_else(|_| counted_turns(output).max(1))
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String, String), String> {
    let input = parse_input(input)?;
    match parse_output(&input, output) {
        Ok(out) => {
            let (mut score, err, svg) = render(&input, &out, turn, true);
            let graph_svg = render_cost_graph(&out, turn);
            if !err.is_empty() {
                score = 0;
            }
            Ok((score, err, svg, graph_svg))
        }
        Err(err) => {
            let out = Output {
                sim: Sim::new(&input),
                comments: vec![],
            };
            let (_, _, svg) = render(&input, &out, 0, true);
            let graph_svg = render_cost_graph(&out, 0);
            Ok((0, err, svg, graph_svg))
        }
    }
}
