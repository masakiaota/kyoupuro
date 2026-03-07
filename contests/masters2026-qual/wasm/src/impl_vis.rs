#![allow(non_snake_case, unused_macros)]

use proconio::{input, marker::Chars};
use rand::prelude::*;
use std::collections::HashMap;
use std::ops::RangeBounds;
use svg::node::element::{Circle, Line, Rectangle, Text as SvgText};
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
    AK: i64,
    AM: i64,
    AW: i64,
    wall_v: Vec<Vec<char>>,
    wall_h: Vec<Vec<char>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {} {}", self.N, self.AK, self.AM, self.AW)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.wall_v[i].iter().collect::<String>())?;
        }
        for i in 0..self.N - 1 {
            writeln!(f, "{}", self.wall_h[i].iter().collect::<String>())?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        AK: i64,
        AM: i64,
        AW: i64,
        wall_v: [Chars; N],
        wall_h: [Chars; N - 1],
    }
    Input {
        N,
        AK,
        AM,
        AW,
        wall_v,
        wall_h,
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

const DIR: [char; 4] = ['U', 'R', 'D', 'L'];
const DIJ: [(usize, usize); 4] = [(!0, 0), (0, 1), (1, 0), (0, !0)];

#[derive(Clone, Debug)]
pub struct Robot {
    m: usize,
    i: usize,
    j: usize,
    d: usize,
    a0: Vec<char>,
    b0: Vec<usize>,
    a1: Vec<char>,
    b1: Vec<usize>,
}

pub struct Output {
    pub robots: Vec<Robot>,
    pub wall_v: Vec<Vec<char>>,
    pub wall_h: Vec<Vec<char>>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut f = f.split_whitespace();
    let K = read(f.next(), 1..=input.N * input.N)?;
    let mut robots = vec![];
    for _ in 0..K {
        let m = read(f.next(), 1..=4 * input.N * input.N)?;
        let i = read(f.next(), 0..input.N)?;
        let j = read(f.next(), 0..input.N)?;
        let d = read(f.next(), 'A'..='Z')?;
        let Some(d) = DIR.iter().position(|&c| c == d) else {
            return Err(format!("Invalid direction: {}", d));
        };
        let mut a0 = vec![];
        let mut b0 = vec![];
        let mut a1 = vec![];
        let mut b1 = vec![];
        for _ in 0..m {
            let a = read(f.next(), 'A'..='Z')?;
            if !['R', 'L', 'F'].contains(&a) {
                return Err(format!("Invalid action: {}", a));
            }
            a0.push(a);
            b0.push(read(f.next(), 0..m)?);
            let a = read(f.next(), 'A'..='Z')?;
            if !['R', 'L'].contains(&a) {
                return Err(format!("Invalid action: {}", a));
            }
            a1.push(a);
            b1.push(read(f.next(), 0..m)?);
        }
        robots.push(Robot {
            m,
            i,
            j,
            d,
            a0,
            b0,
            a1,
            b1,
        });
    }
    let mut wall_v = vec![];
    for _ in 0..input.N {
        let Some(line) = f.next() else {
            return Err("Unexpected EOF".to_owned());
        };
        let line = line.trim();
        if line.len() != input.N - 1 {
            return Err(format!("Invalid wall_v line: {}", line));
        }
        if line.chars().any(|c| c != '0' && c != '1') {
            return Err(format!("Invalid wall_v line: {}", line));
        }
        wall_v.push(line.chars().collect());
    }
    let mut wall_h = vec![];
    for _ in 0..input.N - 1 {
        let Some(line) = f.next() else {
            return Err("Unexpected EOF".to_owned());
        };
        let line = line.trim();
        if line.len() != input.N {
            return Err(format!("Invalid wall_h line: {}", line));
        }
        if line.chars().any(|c| c != '0' && c != '1') {
            return Err(format!("Invalid wall_h line: {}", line));
        }
        wall_h.push(line.chars().collect());
    }
    if f.next().is_some() {
        return Err("Extra tokens".to_owned());
    }
    Ok(Output {
        robots,
        wall_v,
        wall_h,
    })
}

fn has_wall(wall_v: &[Vec<char>], wall_h: &[Vec<char>], i: usize, j: usize, d: usize) -> bool {
    let N = wall_v.len();
    let i2 = i + DIJ[d].0;
    let j2 = j + DIJ[d].1;
    if i2 >= N || j2 >= N {
        return true;
    }
    if i == i2 {
        wall_v[i][j.min(j2)] == '1'
    } else {
        wall_h[i.min(i2)][j] == '1'
    }
}

pub fn gen(seed: u64, problem: &str) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 20;
    let (AK, AM, AW) = match problem {
        "A" => (0, 1, 1000),
        "B" => (1000, rng.gen_range(1..=10), rng.gen_range(1..=10)),
        "C" => (1000, 1, 1000),
        _ => (0, 1, 1000),
    };
    let X = rng.gen_range(1..=6i32);
    let Y = rng.gen_range(1..=6i32);
    loop {
        let mut wall_v = mat!['0'; N; N - 1];
        let mut wall_h = mat!['0'; N - 1; N];
        for _ in 0..X {
            let dir = rng.gen_range(0..2i32);
            let L = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..N as i32) as usize;
            let j = rng.gen_range(0..N as i32 - 1) as usize;
            if dir == 0 {
                for i in (i + 1).saturating_sub(L)..=i {
                    wall_v[i][j] = '1';
                }
            } else {
                for i in i..=(i + L - 1).min(N - 1) {
                    wall_v[i][j] = '1';
                }
            }
        }
        for _ in 0..Y {
            let dir = rng.gen_range(0..2i32);
            let L = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..N as i32 - 1) as usize;
            let j = rng.gen_range(0..N as i32) as usize;
            if dir == 0 {
                for j in (j + 1).saturating_sub(L)..=j {
                    wall_h[i][j] = '1';
                }
            } else {
                for j in j..=(j + L - 1).min(N - 1) {
                    wall_h[i][j] = '1';
                }
            }
        }
        let mut visited = mat![false; N; N];
        let mut stack = vec![(0, 0)];
        visited[0][0] = true;
        let mut num = 0;
        while let Some((i, j)) = stack.pop() {
            num += 1;
            for d in 0..4 {
                if !has_wall(&wall_v, &wall_h, i, j, d) {
                    let i2 = i + DIJ[d].0;
                    let j2 = j + DIJ[d].1;
                    if visited[i2][j2].setmax(true) {
                        stack.push((i2, j2));
                    }
                }
            }
        }
        if num == N * N {
            return Input {
                N,
                AK,
                AM,
                AW,
                wall_v,
                wall_h,
            };
        }
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Route {
    head: Vec<(usize, usize)>,
    tail: Vec<(usize, usize)>,
}

pub fn compute_score_details(
    input: &Input,
    out: &Output,
) -> (i64, String, (Vec<Vec<bool>>, Vec<Route>)) {
    let mut routes = vec![];
    for robot in &out.robots {
        let mut visited = mat![!0; input.N; input.N; 4; robot.m];
        let mut route = vec![];
        let mut i = robot.i;
        let mut j = robot.j;
        let mut d = robot.d;
        let mut s = 0;
        for t in 0.. {
            route.push((i, j));
            if !visited[i][j][d][s].setmin(t) {
                let head = route[..=visited[i][j][d][s]].to_vec();
                let tail = route[visited[i][j][d][s]..].to_vec();
                routes.push(Route { head, tail });
                break;
            }
            let (a, b) = if has_wall(&input.wall_v, &input.wall_h, i, j, d)
                || has_wall(&out.wall_v, &out.wall_h, i, j, d)
            {
                (robot.a1[s], robot.b1[s])
            } else {
                (robot.a0[s], robot.b0[s])
            };
            match a {
                'R' => d = (d + 1) % 4,
                'L' => d = (d + 3) % 4,
                'F' => {
                    i += DIJ[d].0;
                    j += DIJ[d].1;
                }
                _ => unreachable!(),
            }
            s = b;
        }
    }
    let mut checked = mat![false; input.N; input.N];
    let mut num = 0;
    for k in 0..routes.len() {
        for &(i, j) in &routes[k].tail {
            if checked[i][j].setmax(true) {
                num += 1;
            }
        }
    }
    if num != input.N * input.N {
        return (
            0,
            format!("Not all cells are patrolled: {}", num),
            (checked, routes),
        );
    }
    let K = out.robots.len() as i64;
    let M = out.robots.iter().map(|r| r.m).sum::<usize>() as i64;
    let W = out
        .wall_v
        .iter()
        .map(|r| r.iter().filter(|&&c| c == '1').count())
        .sum::<usize>() as i64
        + out
            .wall_h
            .iter()
            .map(|r| r.iter().filter(|&&c| c == '1').count())
            .sum::<usize>() as i64;
    let V = input.AK * (K - 1) + input.AM * M + input.AW * W;
    let mut score = 1;
    let base = input.AK * (input.N * input.N - 1) as i64 + input.AM * (input.N * input.N) as i64;
    if V < base {
        score += (1e6 * (base as f64 / V as f64).log2()).round() as i64;
    }
    (score, String::new(), (checked, routes))
}

#[derive(Clone, Copy)]
struct RobotSnapshot {
    i: usize,
    j: usize,
    d: usize,
    s: usize,
    k: usize,
}

fn parse_input_safe(input: &str) -> Result<Input, String> {
    std::panic::catch_unwind(|| parse_input(input)).map_err(|_| "Failed to parse input".to_owned())
}

fn step_robot(input: &Input, out: &Output, robot: &Robot, i: &mut usize, j: &mut usize, d: &mut usize, s: &mut usize) {
    let (a, b) = if has_wall(&input.wall_v, &input.wall_h, *i, *j, *d)
        || has_wall(&out.wall_v, &out.wall_h, *i, *j, *d)
    {
        (robot.a1[*s], robot.b1[*s])
    } else {
        (robot.a0[*s], robot.b0[*s])
    };
    match a {
        'R' => *d = (*d + 1) % 4,
        'L' => *d = (*d + 3) % 4,
        'F' => {
            *i += DIJ[*d].0;
            *j += DIJ[*d].1;
        }
        _ => {}
    }
    *s = b;
}

fn build_state_cycle(input: &Input, out: &Output, robot: &Robot, k: usize) -> (Vec<RobotSnapshot>, usize, usize) {
    let mut visited = mat![!0; input.N; input.N; 4; robot.m];
    let mut states = vec![];
    let mut i = robot.i;
    let mut j = robot.j;
    let mut d = robot.d;
    let mut s = 0usize;

    loop {
        let prev = visited[i][j][d][s];
        if prev != !0 {
            let cycle_start = prev;
            let cycle_len = states.len().saturating_sub(cycle_start).max(1);
            return (states, cycle_start, cycle_len);
        }
        visited[i][j][d][s] = states.len();
        states.push(RobotSnapshot { i, j, d, s, k });
        step_robot(input, out, robot, &mut i, &mut j, &mut d, &mut s);
    }
}

fn state_at_turn(states: &[RobotSnapshot], cycle_start: usize, cycle_len: usize, turn: usize) -> RobotSnapshot {
    if turn < states.len() {
        states[turn]
    } else {
        let t = cycle_start + (turn - cycle_start) % cycle_len;
        states[t]
    }
}

fn count_added_walls(out: &Output) -> i64 {
    out.wall_v
        .iter()
        .map(|r| r.iter().filter(|&&c| c == '1').count() as i64)
        .sum::<i64>()
        + out
            .wall_h
            .iter()
            .map(|r| r.iter().filter(|&&c| c == '1').count() as i64)
            .sum::<i64>()
}

fn normalized_view_mode(mode: usize) -> usize {
    match mode {
        2 | 3 => mode,
        _ => 1,
    }
}

fn mode_label(mode: usize) -> &'static str {
    match mode {
        2 => "2: Overlap",
        3 => "3: Robot Focus",
        _ => "1: Territory",
    }
}

fn territory_color(idx: usize) -> &'static str {
    const COLORS: [&str; 12] = [
        "#dbeafe", "#dcfce7", "#fee2e2", "#fef3c7", "#e0f2fe", "#fce7f3",
        "#ede9fe", "#ffedd5", "#ccfbf1", "#f3e8ff", "#e2e8f0", "#ecfccb",
    ];
    COLORS[idx % COLORS.len()]
}

fn robot_color(idx: usize) -> &'static str {
    const COLORS: [&str; 12] = [
        "#2563eb", "#059669", "#dc2626", "#d97706", "#0f766e", "#be185d",
        "#7c3aed", "#ea580c", "#0891b2", "#65a30d", "#4f46e5", "#db2777",
    ];
    COLORS[idx % COLORS.len()]
}

#[derive(Clone, Debug)]
struct CoverageStats {
    owner: Vec<Vec<Option<usize>>>,
    cover_count: Vec<Vec<usize>>,
    robot_unique_cover: Vec<usize>,
    robot_head_len: Vec<usize>,
    robot_tail_len: Vec<usize>,
    per_robot_cell_count: Vec<Vec<Vec<usize>>>,
}

fn analyze_coverage(n: usize, routes: &[Route]) -> CoverageStats {
    let k = routes.len();
    let mut per_robot_cell_count = vec![vec![vec![0usize; n]; n]; k];
    let mut robot_unique_cover = vec![0usize; k];
    let mut robot_head_len = vec![0usize; k];
    let mut robot_tail_len = vec![0usize; k];

    for (rk, route) in routes.iter().enumerate() {
        robot_head_len[rk] = route.head.len();
        let tail = &route.tail;
        let tail_len = tail.len();
        let has_duplicate_tail_endpoint =
            tail_len >= 2 && tail[0] == tail[tail_len - 1];
        robot_tail_len[rk] = if has_duplicate_tail_endpoint {
            tail_len.saturating_sub(1)
        } else {
            tail_len
        };

        let mut seen = vec![vec![false; n]; n];
        for (idx, &(i, j)) in tail.iter().enumerate() {
            if has_duplicate_tail_endpoint && idx + 1 == tail_len {
                continue;
            }
            per_robot_cell_count[rk][i][j] += 1;
            if !seen[i][j] {
                seen[i][j] = true;
                robot_unique_cover[rk] += 1;
            }
        }
    }

    let mut owner = vec![vec![None; n]; n];
    let mut cover_count = vec![vec![0usize; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut best_robot = None;
            let mut best_count = 0usize;
            let mut cnum = 0usize;
            for rk in 0..k {
                let c = per_robot_cell_count[rk][i][j];
                if c == 0 {
                    continue;
                }
                cnum += 1;
                if best_robot.is_none()
                    || c > best_count
                    || (c == best_count && rk < best_robot.unwrap())
                {
                    best_robot = Some(rk);
                    best_count = c;
                }
            }
            owner[i][j] = best_robot;
            cover_count[i][j] = cnum;
        }
    }

    CoverageStats {
        owner,
        cover_count,
        robot_unique_cover,
        robot_head_len,
        robot_tail_len,
        per_robot_cell_count,
    }
}

fn draw_route_lines(
    mut doc: Document,
    route: &[(usize, usize)],
    ox: f64,
    oy: f64,
    cell: f64,
    stroke: &str,
    width: f64,
    opacity: f64,
    dasharray: Option<&str>,
) -> Document {
    for pair in route.windows(2) {
        let (i1, j1) = pair[0];
        let (i2, j2) = pair[1];
        let x1 = ox + (j1 as f64 + 0.5) * cell;
        let y1 = oy + (i1 as f64 + 0.5) * cell;
        let x2 = ox + (j2 as f64 + 0.5) * cell;
        let y2 = oy + (i2 as f64 + 0.5) * cell;
        let mut line = Line::new()
            .set("x1", x1)
            .set("y1", y1)
            .set("x2", x2)
            .set("y2", y2)
            .set("stroke", stroke)
            .set("stroke-width", width)
            .set("stroke-opacity", opacity)
            .set("stroke-linecap", "round");
        if let Some(dash) = dasharray {
            line = line.set("stroke-dasharray", dash);
        }
        doc = doc.add(line);
    }
    doc
}

fn build_svg(
    input: &Input,
    out: &Output,
    routes: &[Route],
    checked: &[Vec<bool>],
    states: &[RobotSnapshot],
    score: i64,
    err: &str,
    turn: usize,
    view_mode: usize,
    focus_robot: usize,
) -> String {
    let mode = normalized_view_mode(view_mode);
    let coverage = analyze_coverage(input.N, routes);
    let robot_num = out.robots.len();
    let focus = if robot_num == 0 {
        0
    } else {
        focus_robot.min(robot_num - 1)
    };

    let n = input.N as f64;
    let margin = 30.0;
    let board_size = 760.0;
    let panel_width = 320.0;
    let w = board_size + panel_width + margin * 2.0;
    let h = board_size + margin * 2.0;
    let cell = board_size / n;
    let ox = margin;
    let oy = margin;

    let mut doc = Document::new()
        .set("viewBox", format!("0 0 {} {}", w, h))
        .set("width", w)
        .set("height", h);

    doc = doc.add(
        Rectangle::new()
            .set("x", 0.0)
            .set("y", 0.0)
            .set("width", w)
            .set("height", h)
            .set("fill", "#f8fafc"),
    );

    for i in 0..input.N {
        for j in 0..input.N {
            let cover_count = coverage.cover_count[i][j];
            let fill = match mode {
                2 => match cover_count {
                    0 => "#fee2e2",
                    1 => "#dcfce7",
                    2 => "#bbf7d0",
                    3 => "#86efac",
                    4 => "#4ade80",
                    _ => "#16a34a",
                },
                3 => {
                    if robot_num > 0 && coverage.per_robot_cell_count[focus][i][j] > 0 {
                        "#fde68a"
                    } else if cover_count > 0 {
                        "#f3f4f6"
                    } else {
                        "#fee2e2"
                    }
                }
                _ => {
                    if cover_count == 0 {
                        "#fee2e2"
                    } else {
                        territory_color(coverage.owner[i][j].unwrap())
                    }
                }
            };

            let x = ox + j as f64 * cell;
            let y = oy + i as f64 * cell;
            doc = doc.add(
                Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", fill),
            );

            if mode == 1 && cover_count >= 2 {
                doc = doc.add(
                    Line::new()
                        .set("x1", x + cell * 0.12)
                        .set("y1", y + cell * 0.88)
                        .set("x2", x + cell * 0.88)
                        .set("y2", y + cell * 0.12)
                        .set("stroke", "#6b7280")
                        .set("stroke-width", 1.2),
                );
                doc = doc.add(
                    Line::new()
                        .set("x1", x + cell * 0.12)
                        .set("y1", y + cell * 0.62)
                        .set("x2", x + cell * 0.62)
                        .set("y2", y + cell * 0.12)
                        .set("stroke", "#6b7280")
                        .set("stroke-width", 1.2),
                );
            }
            if mode == 2 && cover_count >= 2 {
                doc = doc.add(
                    SvgText::new(format!("{}", cover_count))
                        .set("x", x + cell * 0.5)
                        .set("y", y + cell * 0.62)
                        .set("text-anchor", "middle")
                        .set("font-size", cell * 0.32)
                        .set("font-weight", 700)
                        .set("fill", "#14532d"),
                );
            }
        }
    }

    if mode == 3 && robot_num > 0 {
        let route = &routes[focus];
        doc = draw_route_lines(
            doc,
            &route.head,
            ox,
            oy,
            cell,
            "#64748b",
            (cell * 0.11).max(1.4),
            0.7,
            Some("4 3"),
        );
        doc = draw_route_lines(
            doc,
            &route.tail,
            ox,
            oy,
            cell,
            "#f97316",
            (cell * 0.16).max(2.0),
            0.95,
            None,
        );
        if let Some(&(si, sj)) = route.head.first().or(route.tail.first()) {
            let sx = ox + (sj as f64 + 0.5) * cell;
            let sy = oy + (si as f64 + 0.5) * cell;
            doc = doc.add(
                Circle::new()
                    .set("cx", sx)
                    .set("cy", sy)
                    .set("r", cell * 0.22)
                    .set("fill", "none")
                    .set("stroke", "#ea580c")
                    .set("stroke-width", 2.5),
            );
        }
    }
    if mode == 1 {
        for (rk, route) in routes.iter().enumerate() {
            let color = robot_color(rk);
            doc = draw_route_lines(
                doc,
                &route.head,
                ox,
                oy,
                cell,
                color,
                (cell * 0.06).max(1.0),
                0.30,
                Some("3 3"),
            );
            doc = draw_route_lines(
                doc,
                &route.tail,
                ox,
                oy,
                cell,
                color,
                (cell * 0.08).max(1.2),
                0.48,
                None,
            );
        }
    }

    for i in 0..=input.N {
        let y = oy + i as f64 * cell;
        doc = doc.add(
            Line::new()
                .set("x1", ox)
                .set("y1", y)
                .set("x2", ox + board_size)
                .set("y2", y)
                .set("stroke", "#d1d5db")
                .set("stroke-width", 1),
        );
    }
    for j in 0..=input.N {
        let x = ox + j as f64 * cell;
        doc = doc.add(
            Line::new()
                .set("x1", x)
                .set("y1", oy)
                .set("x2", x)
                .set("y2", oy + board_size)
                .set("stroke", "#d1d5db")
                .set("stroke-width", 1),
        );
    }

    for i in 0..input.N {
        for j in 0..input.N - 1 {
            if input.wall_v[i][j] == '1' {
                let x = ox + (j + 1) as f64 * cell;
                let y1 = oy + i as f64 * cell;
                let y2 = oy + (i + 1) as f64 * cell;
                doc = doc.add(
                    Line::new()
                        .set("x1", x)
                        .set("y1", y1)
                        .set("x2", x)
                        .set("y2", y2)
                        .set("stroke", "#111827")
                        .set("stroke-width", 3),
                );
            }
            if out.wall_v[i][j] == '1' {
                let x = ox + (j + 1) as f64 * cell;
                let y1 = oy + i as f64 * cell;
                let y2 = oy + (i + 1) as f64 * cell;
                doc = doc.add(
                    Line::new()
                        .set("x1", x)
                        .set("y1", y1)
                        .set("x2", x)
                        .set("y2", y2)
                        .set("stroke", "#b91c1c")
                        .set("stroke-width", 4),
                );
            }
        }
    }
    for i in 0..input.N - 1 {
        for j in 0..input.N {
            if input.wall_h[i][j] == '1' {
                let y = oy + (i + 1) as f64 * cell;
                let x1 = ox + j as f64 * cell;
                let x2 = ox + (j + 1) as f64 * cell;
                doc = doc.add(
                    Line::new()
                        .set("x1", x1)
                        .set("y1", y)
                        .set("x2", x2)
                        .set("y2", y)
                        .set("stroke", "#111827")
                        .set("stroke-width", 3),
                );
            }
            if out.wall_h[i][j] == '1' {
                let y = oy + (i + 1) as f64 * cell;
                let x1 = ox + j as f64 * cell;
                let x2 = ox + (j + 1) as f64 * cell;
                doc = doc.add(
                    Line::new()
                        .set("x1", x1)
                        .set("y1", y)
                        .set("x2", x2)
                        .set("y2", y)
                        .set("stroke", "#b91c1c")
                        .set("stroke-width", 4),
                );
            }
        }
    }

    doc = doc.add(
        Rectangle::new()
            .set("x", ox)
            .set("y", oy)
            .set("width", board_size)
            .set("height", board_size)
            .set("fill", "none")
            .set("stroke", "#111827")
            .set("stroke-width", 4),
    );

    let offsets = [
        (0.0, 0.0),
        (-0.18, -0.18),
        (0.18, -0.18),
        (-0.18, 0.18),
        (0.18, 0.18),
        (0.0, -0.24),
        (0.0, 0.24),
        (-0.24, 0.0),
        (0.24, 0.0),
    ];
    let mut cell_counts: HashMap<(usize, usize), usize> = HashMap::new();

    for st in states {
        let c = cell_counts.entry((st.i, st.j)).or_insert(0);
        let off = offsets[*c % offsets.len()];
        *c += 1;
        let cx = ox + (st.j as f64 + 0.5 + off.0) * cell;
        let cy = oy + (st.i as f64 + 0.5 + off.1) * cell;
        let is_focus = mode != 3 || st.k == focus;
        let color = if is_focus {
            robot_color(st.k)
        } else {
            "#9ca3af"
        };
        let r = if is_focus { cell * 0.18 } else { cell * 0.12 };
        doc = doc.add(
            Circle::new()
                .set("cx", cx)
                .set("cy", cy)
                .set("r", r)
                .set("fill", color)
                .set("stroke", "#ffffff")
                .set("stroke-width", 1.5)
                .set("fill-opacity", if is_focus { 1.0 } else { 0.7 }),
        );

        let (vx, vy) = match st.d {
            0 => (0.0, -1.0),
            1 => (1.0, 0.0),
            2 => (0.0, 1.0),
            _ => (-1.0, 0.0),
        };
        let tip_x = cx + vx * cell * 0.26;
        let tip_y = cy + vy * cell * 0.26;
        doc = doc.add(
            Line::new()
                .set("x1", cx)
                .set("y1", cy)
                .set("x2", tip_x)
                .set("y2", tip_y)
                .set("stroke", "#ffffff")
                .set("stroke-width", if is_focus { 2 } else { 1 }),
        );

        if is_focus || mode != 3 {
            doc = doc.add(
                SvgText::new(format!("{}:{}", st.k, st.s))
                    .set("x", cx)
                    .set("y", cy - cell * 0.24)
                    .set("text-anchor", "middle")
                    .set("font-size", cell * 0.22)
                    .set("fill", "#111827"),
            );
        }
    }

    let uncovered = checked
        .iter()
        .map(|r| r.iter().filter(|&&v| !v).count())
        .sum::<usize>();
    let k = out.robots.len();
    let m = out.robots.iter().map(|r| r.m).sum::<usize>();
    let added = count_added_walls(out);
    let panel_x = ox + board_size + 20.0;

    let mut info = vec![
        format!("Mode: {}", mode_label(mode)),
        format!("Turn: {}", turn),
        format!("Score: {}", score),
        format!("Robots(K): {}", k),
        format!("States(M): {}", m),
        format!("Added walls(W): {}", added),
        format!("Uncovered: {}", uncovered),
    ];
    if mode == 3 && k > 0 {
        info.push(format!(
            "Focus R{}: covered {} cells",
            focus, coverage.robot_unique_cover[focus]
        ));
        info.push(format!(
            "Focus route: head={}, tail={}",
            coverage.robot_head_len[focus], coverage.robot_tail_len[focus]
        ));
    }
    let mut ranking = coverage
        .robot_unique_cover
        .iter()
        .enumerate()
        .map(|(rk, &cells)| (rk, cells))
        .collect::<Vec<_>>();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (rk, cells) in ranking.into_iter().take(6) {
        info.push(format!("R{} patrol cells: {}", rk, cells));
    }

    for (idx, text) in info.iter().enumerate() {
        doc = doc.add(
            SvgText::new(text.clone())
                .set("x", panel_x)
                .set("y", oy + 30.0 + idx as f64 * 24.0)
                .set("font-size", 18)
                .set("fill", "#111827"),
        );
    }
    if !err.is_empty() {
        doc = doc.add(
            SvgText::new(format!("Error: {}", err))
                .set("x", panel_x)
                .set("y", oy + 40.0 + info.len() as f64 * 24.0)
                .set("font-size", 16)
                .set("fill", "#b91c1c"),
        );
    }

    doc.to_string()
}

pub fn generate(seed: i32, problem_id: &str) -> String {
    let problem = match problem_id.chars().next().unwrap_or('A') {
        'B' | 'b' => "B",
        'C' | 'c' => "C",
        _ => "A",
    };
    let input = gen(seed as u64, problem);
    format!("{}", input)
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    let Ok(input) = parse_input_safe(input) else {
        return 1;
    };
    let Ok(out) = parse_output(&input, output) else {
        return 1;
    };

    let mut max_turn = 1usize;
    for (k, robot) in out.robots.iter().enumerate() {
        let (states, _, _) = build_state_cycle(&input, &out, robot, k);
        max_turn = max_turn.max(states.len().max(1));
    }
    max_turn
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    visualize_with_mode(input, output, turn, 1, 0)
}

pub fn visualize_with_mode(
    input: &str,
    output: &str,
    turn: usize,
    view_mode: usize,
    focus_robot: usize,
) -> Result<(i64, String, String), String> {
    let input = parse_input_safe(input)?;
    let out = parse_output(&input, output)?;
    let (score, err, (checked, routes)) = compute_score_details(&input, &out);

    let mut states_now = Vec::with_capacity(out.robots.len());
    for (k, robot) in out.robots.iter().enumerate() {
        let (states, cycle_start, cycle_len) = build_state_cycle(&input, &out, robot, k);
        let now = state_at_turn(&states, cycle_start, cycle_len, turn);
        states_now.push(now);
    }
    let svg = build_svg(
        &input,
        &out,
        &routes,
        &checked,
        &states_now,
        score,
        &err,
        turn,
        view_mode,
        focus_robot,
    );
    Ok((score, err, svg))
}
