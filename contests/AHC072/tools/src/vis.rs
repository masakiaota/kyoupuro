use super::*;
use svg::node::element::{
    Definitions, Ellipse, Group, Line, LinearGradient, Path, RadialGradient, Rectangle, Stop, Text,
    Title,
};
#[derive(Clone, Debug)]
pub struct VisOption {
    pub t: usize,
    pub progress: i32,
}
impl Default for VisOption {
    fn default() -> Self {
        Self {
            t: usize::MAX,
            progress: 0,
        }
    }
}
impl VisOption {
    pub fn set_bool(&mut self, name: &str, _value: bool) -> Result<(), String> {
        Err(format!("Unknown visualizer option: {}", name))
    }
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), String> {
        if name == "progress" {
            if !(0..=1000).contains(&value) {
                return Err("progress must be in 0..=1000".into());
            }
            self.progress = value;
            return Ok(());
        }
        Err(format!("Unknown visualizer option: {}", name))
    }
    pub fn set_string(&mut self, name: &str, _value: &str) -> Result<(), String> {
        Err(format!("Unknown visualizer option: {}", name))
    }
}
pub struct VisData {
    output: Output,
    evaluation: Evaluation,
    checkpoints: Vec<State>,
    valid: usize,
}
impl VisData {
    pub fn new(input: &Input, output: Output) -> Self {
        let evaluation = evaluate(input, &output);
        let mut s = State::new(input);
        let mut checkpoints = vec![s.clone()];
        for o in &output.ops {
            if s.apply(input, o).is_err() {
                break;
            }
            if s.t % 256 == 0 {
                checkpoints.push(s.clone());
            }
        }
        Self {
            output,
            evaluation,
            checkpoints,
            valid: s.t,
        }
    }
    pub fn verdict(&self) -> &Result<i64, String> {
        self.evaluation.verdict()
    }
    pub fn state_at(&self, input: &Input, t: usize) -> State {
        let t = t.min(self.valid);
        let mut s = self.checkpoints[t / 256].clone();
        for o in &self.output.ops[s.t..t] {
            s.apply(input, o).unwrap();
        }
        s
    }
}
pub fn get_max_turn(_input: &Input, data: &VisData, _option: &VisOption) -> usize {
    data.valid + usize::from(data.verdict().is_err())
}
const COLORS: [&str; 12] = [
    "#ef8c8c", "#80b4f0", "#9bcf81", "#f5d66e", "#bc9bdd", "#79d3cf", "#f2b574", "#dba7c5",
    "#b5bd6b", "#6a91a8", "#ce8964", "#a3a3a3",
];
fn text(x: f64, y: f64, s: String, size: usize) -> Text {
    Text::new(s)
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("fill", "#18212a")
        .set("font-family", "sans-serif")
}
fn rect(x: f64, y: f64, w: f64, h: f64, color: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", color)
}
fn artwork() -> Definitions {
    let mut defs = Definitions::new();
    for (name, top, bottom) in [
        ("floor", "#ffffff", "#f0f3f8"),
        ("wall", "#7a8da5", "#4b5d76"),
    ] {
        defs = defs.add(
            LinearGradient::new()
                .set("id", name)
                .set("x1", "0%")
                .set("y1", "0%")
                .set("x2", "0%")
                .set("y2", "100%")
                .add(Stop::new().set("offset", "0%").set("stop-color", top))
                .add(Stop::new().set("offset", "100%").set("stop-color", bottom)),
        );
    }
    for (i, color) in COLORS.iter().enumerate() {
        defs = defs.add(
            RadialGradient::new()
                .set("id", format!("slime{i}"))
                .set("cx", "35%")
                .set("cy", "15%")
                .set("r", "90%")
                .add(Stop::new().set("offset", "0%").set("stop-color", "#ffffff"))
                .add(Stop::new().set("offset", "35%").set("stop-color", *color))
                .add(Stop::new().set("offset", "100%").set("stop-color", *color)),
        );
        defs = defs.add(
            RadialGradient::new()
                .set("id", format!("nest{i}"))
                .add(
                    Stop::new()
                        .set("offset", "0%")
                        .set("stop-color", *color)
                        .set("stop-opacity", 0.8),
                )
                .add(
                    Stop::new()
                        .set("offset", "70%")
                        .set("stop-color", *color)
                        .set("stop-opacity", 0.45),
                )
                .add(
                    Stop::new()
                        .set("offset", "100%")
                        .set("stop-color", *color)
                        .set("stop-opacity", 0.05),
                ),
        );
    }
    defs
}
fn slime(c: usize, cell: f64, eyes: bool) -> Group {
    let mut g = Group::new().set("transform", format!("scale({})", cell));
    g = g.add(Path::new().set("d",
        "M -.33 .07 C -.36 -.01 -.25 -.11 -.12 -.12 Q -.025 -.125 0 -.18 Q .035 -.12 .12 -.12 C .26 -.11 .36 .01 .33 .07 Q .28 .13 0 .12 Q -.28 .13 -.33 .07 Z")
        .set("fill", format!("url(#slime{})", (c - 1) % COLORS.len()))
        .set("stroke", "#425269").set("stroke-opacity", 0.45).set("stroke-width", 0.018));
    g = g.add(
        Ellipse::new()
            .set("cx", -0.15)
            .set("cy", -0.035)
            .set("rx", 0.07)
            .set("ry", 0.024)
            .set("fill", "white")
            .set("opacity", 0.55),
    );
    if !eyes {
        return g;
    }
    for x in [-0.075, 0.075] {
        g = g.add(
            Ellipse::new()
                .set("cx", x)
                .set("cy", 0.065)
                .set("rx", 0.023)
                .set("ry", 0.022)
                .set("fill", "#253044"),
        );
    }
    g.add(
        Path::new()
            .set("d", "M -.03 .098 Q 0 .117 .03 .098")
            .set("fill", "none")
            .set("stroke", "#253044")
            .set("stroke-width", 0.012)
            .set("stroke-linecap", "round"),
    )
}
fn slime_at(
    c: usize,
    cell: f64,
    x: f64,
    y: f64,
    sx: f64,
    sy: f64,
    angle: f64,
    eyes: bool,
) -> Group {
    Group::new()
        .set("data-slime", c - 1)
        .set(
            "transform",
            format!("translate({x} {y}) rotate({angle}) scale({sx} {sy})"),
        )
        .add(slime(c, cell, eyes))
}
fn center(input: &Input, p: usize, level: f64) -> (f64, f64) {
    let cell = 720. / input.n as f64;
    (
        36. + (p % input.n) as f64 * cell + cell * 0.5,
        76. + (p / input.n) as f64 * cell + cell * (0.83 - level * 0.09),
    )
}

struct Motion {
    board: State,
    overlay: Group,
    covered: Vec<usize>,
}
fn motion(input: &Input, before: &State, op: &Operation, progress: f64) -> Motion {
    let mut after = before.clone();
    after.apply(input, op).unwrap();
    let q = neighbor(input.n, op.p, op.d, op.l).unwrap();
    let cell = 720. / input.n as f64;
    let mut board = before.clone();
    let moving = board.stacks[op.p].split_off(op.k);
    let mut overlay = Group::new().set("id", "motion");
    let mut covered = Vec::new();
    if progress < 0.65 {
        let u = progress / 0.65;
        let smooth = u * u * (3. - 2. * u);
        let half = (moving.len() - 1) as f64 / 2.;
        let (x0, y0) = center(input, op.p, op.k as f64 + half);
        let (x1, y1) = center(input, q, board.stacks[q].len() as f64 + half);
        let x = x0 + (x1 - x0) * smooth;
        let lift = (cell * 0.65).min(y0.min(y1) - 52.).max(0.);
        let y = y0 + (y1 - y0) * smooth - lift * (std::f64::consts::PI * u).sin();
        let angle = std::f64::consts::PI * smooth * if op.d == 2 { -1. } else { 1. };
        overlay = overlay.add(
            Ellipse::new()
                .set("cx", x)
                .set("cy", y0 + (y1 - y0) * smooth + cell * 0.12)
                .set("rx", cell * 0.30)
                .set("ry", cell * 0.055)
                .set("fill", "#334155")
                .set("opacity", 0.12),
        );
        let mut order: Vec<_> = (0..moving.len()).collect();
        if angle.cos() < 0. {
            order.reverse();
        }
        let straighten = ((u - 0.8) / 0.2).clamp(0., 1.);
        let body_angle =
            angle.to_degrees() * (1. - straighten * straighten * (3. - 2. * straighten));
        let top = *order.last().unwrap();
        for j in order {
            let c = moving[j];
            let offset = (half - j as f64) * cell * 0.09;
            overlay = overlay.add(slime_at(
                c,
                cell,
                x - offset * angle.sin(),
                y + offset * angle.cos(),
                1.,
                1.,
                body_angle,
                j == top,
            ));
        }
    } else {
        board.stacks[q].extend(moving.iter().rev().copied());
        if progress < 0.78 {
            let u = (progress - 0.65) / 0.13;
            let squash = 0.23 * (std::f64::consts::PI * u).sin();
            for (j, &c) in board.stacks[q].iter().enumerate() {
                let (x, y) = center(input, q, j as f64);
                let (_, bottom) = center(input, q, 0.);
                overlay = overlay.add(slime_at(
                    c,
                    cell,
                    x,
                    bottom + (y - bottom) * (1. - squash),
                    1. + squash,
                    1. - squash,
                    0.,
                    j + 1 == board.stacks[q].len(),
                ));
            }
            board.stacks[q].clear();
        } else {
            for p in [op.p, q] {
                let keep = after.stacks[p].len();
                let count = board.stacks[p].len() - keep;
                let phase = ((progress - 0.78) / 0.22 * count as f64).min(count as f64);
                let removed = phase.floor() as usize;
                let remaining = board.stacks[p].len() - removed;
                board.stacks[p].truncate(remaining);
                if removed < count {
                    covered.push(p);
                    let c = board.stacks[p].pop().unwrap();
                    let (x, y) = center(input, p, board.stacks[p].len() as f64);
                    let u = phase.fract();
                    let (_, nest_y) = center(input, p, 0.);
                    overlay = overlay.add(
                        slime_at(c, cell, x, y + (nest_y - y) * u, 1. - u, 1. - u, 0., true)
                            .set("opacity", 1. - u),
                    );
                }
            }
        }
    }
    Motion {
        board,
        overlay,
        covered,
    }
}
fn draw(
    input: &Input,
    s: &State,
    t: usize,
    score: i64,
    error: &str,
    last: Option<&Operation>,
    motion: Option<&Motion>,
) -> String {
    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("width", 1060)
        .set("height", 840)
        .set("viewBox", (0, 0, 1060, 840))
        .set("style", "background:white")
        .add(artwork())
        .add(rect(0., 0., 1060., 840., "white"));
    let mut info = Group::new().set("style", "user-select:text;cursor:text");
    let remaining = input.b.iter().sum::<usize>() - s.received.iter().sum::<usize>();
    let action = last.map_or_else(
        || "—".to_string(),
        |o| {
            format!(
                "{} {} {} {} {}",
                o.p / input.n,
                o.p % input.n,
                o.k,
                ["U", "D", "L", "R"][o.d],
                o.l
            )
        },
    );
    for (x, width, label, value, accessible, accent) in [
        (
            36.,
            170.,
            "TURN",
            t.to_string(),
            format!("t = {}", t),
            "#53657c",
        ),
        (
            222.,
            330.,
            "SCORE",
            score.to_string(),
            format!("score = {}", score),
            "#53657c",
        ),
        (
            568.,
            188.,
            "REMAINING",
            remaining.to_string(),
            format!("Remaining slimes: {}", remaining),
            "#53657c",
        ),
        (
            774.,
            272.,
            "LAST ACTION",
            action.clone(),
            format!("Last Action: {}", action),
            "#53657c",
        ),
    ] {
        let card = Group::new()
            .set("role", "group")
            .set("aria-label", accessible)
            .add(
                rect(x, 4., width, 44., "white")
                    .set("rx", 9)
                    .set("stroke", "#dce4ef")
                    .set("stroke-width", 1),
            )
            .add(rect(x + 12., 13., 3., 25., accent).set("rx", 1.5))
            .add(
                text(x + 25., 17., label.into(), 10)
                    .set("fill", "#66758a")
                    .set("font-weight", 600)
                    .set("letter-spacing", 0.8),
            )
            .add(
                text(x + 25., 40., value, 23)
                    .set("fill", accent)
                    .set("font-weight", 700)
                    .set("style", "font-variant-numeric:tabular-nums"),
            );
        info = info.add(card);
    }
    if !error.is_empty() {
        info = info.add(text(36., 826., error.to_string(), 12).set("fill", "#c02020"));
    }
    doc = doc.add(info);
    let cell = 720.0 / input.n as f64;
    let board = motion.map_or(s, |m| &m.board);
    let mut slimes = Group::new();
    for p in 0..input.n * input.n {
        let x = 36. + (p % input.n) as f64 * cell;
        let y = 76. + (p / input.n) as f64 * cell;
        let plate = input.plates.iter().position(|&q| q == p);
        let tooltip = format!(
            "({}, {}) | nest: {} | bottom to top: {:?}",
            p / input.n,
            p % input.n,
            plate.map_or_else(|| "none".to_string(), |i| i.to_string()),
            board.stacks[p].iter().map(|&c| c - 1).collect::<Vec<_>>()
        );
        let mut g = Group::new()
            .set("data-cell", p)
            .add(Title::new(tooltip.clone()));
        g = g.add(rect(x, y, cell, cell, "#dfe5ee"));
        if input.floor[p] {
            g = g.add(
                rect(x + 0.7, y + 0.7, cell - 1.4, cell - 1.4, "url(#floor)")
                    .set("rx", cell * 0.07),
            );
        } else {
            g = g.add(rect(x + 1., y + 3., cell - 2., cell - 3., "#37465d").set("rx", cell * 0.13));
            g = g.add(
                rect(x + 1., y + 0.8, cell - 2., cell - 4., "url(#wall)").set("rx", cell * 0.13),
            );
            g = g.add(
                rect(
                    x + cell * 0.15,
                    y + cell * 0.10,
                    cell * 0.45,
                    cell * 0.045,
                    "#b0bfd1",
                )
                .set("rx", cell * 0.025)
                .set("opacity", 0.6),
            );
        }
        if let Some(i) = plate {
            g = g.add(
                rect(
                    x + 1.,
                    y + 1.,
                    cell - 2.,
                    cell - 2.,
                    &format!("url(#nest{i})"),
                )
                .set("rx", cell * 0.2),
            );
            g = g.add(
                Ellipse::new()
                    .set("cx", x + cell * 0.5)
                    .set("cy", y + cell * 0.78)
                    .set("rx", cell * 0.34)
                    .set("ry", cell * 0.13)
                    .set("fill", COLORS[i % COLORS.len()])
                    .set("opacity", 0.8),
            );
            g = g.add(
                Ellipse::new()
                    .set("cx", x + cell * 0.5)
                    .set("cy", y + cell * 0.76)
                    .set("rx", cell * 0.25)
                    .set("ry", cell * 0.065)
                    .set("fill", "#ffffff")
                    .set("opacity", 0.45),
            );
        }
        let mut occupants = Group::new().set("data-cell", p).add(Title::new(tooltip));
        let h = board.stacks[p].len();
        if h > 0 {
            occupants = occupants.add(
                Ellipse::new()
                    .set("cx", x + cell * 0.5)
                    .set("cy", y + cell * 0.94)
                    .set("rx", cell * 0.31)
                    .set("ry", cell * 0.045)
                    .set("fill", "#455870")
                    .set("opacity", 0.15),
            );
        }
        for (j, &c) in board.stacks[p].iter().enumerate() {
            let (cx, cy) = center(input, p, j as f64);
            occupants = occupants.add(slime_at(
                c,
                cell,
                cx,
                cy,
                1.,
                1.,
                0.,
                j + 1 == h && motion.is_none_or(|m| !m.covered.contains(&p)),
            ));
        }
        doc = doc.add(g);
        if h > 0 {
            slimes = slimes.add(occupants);
        }
    }
    for i in 0..input.n {
        doc = doc.add(text(
            36. + i as f64 * cell + cell / 2. - 5.,
            69.,
            i.to_string(),
            10,
        ));
        doc = doc.add(text(
            10.,
            76. + i as f64 * cell + cell / 2. + 4.,
            i.to_string(),
            10,
        ));
    }
    if let Some(o) = last {
        if let Some(q) = neighbor(input.n, o.p, o.d, o.l) {
            let x1 = 36. + (o.p % input.n) as f64 * cell + cell / 2.;
            let y1 = 76. + (o.p / input.n) as f64 * cell + cell / 2.;
            let x2 = 36. + (q % input.n) as f64 * cell + cell / 2.;
            let y2 = 76. + (q / input.n) as f64 * cell + cell / 2.;
            doc = doc.add(
                Line::new()
                    .set("x1", x1)
                    .set("y1", y1)
                    .set("x2", x2)
                    .set("y2", y2)
                    .set("stroke", "#7b8ba0")
                    .set("stroke-width", 2)
                    .set("stroke-dasharray", "3 5")
                    .set("stroke-linecap", "round"),
            );
            doc = doc.add(
                svg::node::element::Circle::new()
                    .set("cx", x2)
                    .set("cy", y2)
                    .set("r", 4)
                    .set("fill", "#7b8ba0"),
            );
        }
    }
    doc = doc.add(slimes);
    if let Some(m) = motion {
        doc = doc.add(m.overlay.clone());
    }
    doc = doc.add(text(792., 100., "Received / Total".into(), 17).set("font-weight", 600));
    for i in 0..input.k {
        let y = 130. + i as f64 * 47.;
        doc = doc.add(rect(788., y - 20., 244., 42., "#f1f5fa").set("rx", 11));
        doc = doc.add(
            Group::new()
                .set("transform", format!("translate(811 {})", y - 3.))
                .add(slime(i + 1, 35., true)),
        );
        doc = doc.add(text(835., y, format!("{}", i), 12).set("fill", "#7b8ba0"));
        doc = doc.add(rect(863., y + 7., 153., 4., "#dce4ef").set("rx", 2));
        let ratio = s.received[i] as f64 / input.b[i].max(1) as f64;
        if ratio > 0. {
            doc = doc
                .add(rect(863., y + 7., 153. * ratio, 4., COLORS[i % COLORS.len()]).set("rx", 2));
        }
        doc = doc.add(text(
            863.,
            y,
            format!("{} / {}", s.received[i], input.b[i]),
            18,
        ));
    }
    doc.to_string()
}
pub struct VisResult {
    pub svg: String,
}

// 手動プレイでは確定済みの状態を直接描く。ルールと盤面描画は再生画面と共有する。
pub fn draw_state(input: &Input, state: &State, last: Option<&Operation>) -> String {
    draw(input, state, state.t, state.score(input), "", last, None)
}

pub fn vis(input: &Input, data: &VisData, option: &VisOption) -> VisResult {
    let max = get_max_turn(input, data, option);
    let mut t = option.t.min(max);
    let progress = option.progress.clamp(0, 1000);
    if progress == 1000 && t < data.valid {
        t += 1;
    }
    let s = data.state_at(input, t);
    let (score, error) = if t == max {
        match data.verdict() {
            Ok(v) => (*v, String::new()),
            Err(e) => (0, e.clone()),
        }
    } else {
        (s.score(input), String::new())
    };
    let last = if s.t > 0 {
        Some(&data.output.ops[s.t - 1])
    } else {
        None
    };
    let animation = if progress > 0 && progress < 1000 && t < data.valid {
        Some(motion(
            input,
            &s,
            &data.output.ops[t],
            progress as f64 / 1000.,
        ))
    } else {
        None
    };
    let last = if animation.is_some() {
        Some(&data.output.ops[t])
    } else {
        last
    };
    VisResult {
        svg: draw(input, &s, t, score, &error, last, animation.as_ref()),
    }
}
pub fn draw_parse_error(input: &Input, error: &str, _option: &VisOption) -> String {
    draw(input, &State::new(input), 0, 0, error, None, None)
}

pub fn manual_selection(input: &Input, state: &State, p: usize, k: usize) -> String {
    let Some(stack) = state.stacks.get(p) else {
        return String::new();
    };
    if k >= stack.len() {
        return String::new();
    }
    let cell = 720. / input.n as f64;
    let mut group = Group::new().set("id", "manual-selection");
    for d in 0..4 {
        for l in 1..=k + 1 {
            let Some(q) = neighbor(input.n, p, d, l) else {
                continue;
            };
            let op = Operation { p, k, d, l };
            if state.clone().apply(input, &op).is_ok() {
                let x = 36. + (q % input.n) as f64 * cell;
                let y = 76. + (q / input.n) as f64 * cell;
                group = group.add(
                    rect(x + 2., y + 2., cell - 4., cell - 4., "#2563eb")
                        .set("fill-opacity", 0.12)
                        .set("stroke", "#2563eb")
                        .set("stroke-width", 2)
                        .set("rx", 4)
                        .set("data-manual-target", q)
                        .set("style", "cursor:pointer"),
                );
            }
        }
    }
    let x = 36. + (p % input.n) as f64 * cell;
    let y = 76. + (p / input.n) as f64 * cell;
    group = group.add(
        rect(x + 2., y + 2., cell - 4., cell - 4., "none")
            .set("stroke", "#2563eb")
            .set("stroke-width", 2)
            .set("rx", 4)
            .set("pointer-events", "none"),
    );
    group.to_string()
}
