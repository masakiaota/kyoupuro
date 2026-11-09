use super::*;
use svg::node::element::{Group, Path, Rectangle, Style, Text, Title};

#[derive(Clone, Debug)]
pub struct VisOption {
    pub t: usize,
}

impl Default for VisOption {
    fn default() -> Self {
        Self {
            t: usize::MAX,
        }
    }
}

impl VisOption {
    #[allow(unused_variables)]
    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<(), String> {
        match name {
            _ => Err(format!("Unknown visualizer option: {}", name)),
        }
    }

    #[allow(unused_variables)]
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), String> {
        match name {
            _ => Err(format!("Unknown visualizer option: {}", name)),
        }
    }

    #[allow(unused_variables)]
    pub fn set_string(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            _ => Err(format!("Unknown visualizer option: {}", name)),
        }
    }
}

#[allow(dead_code)]
fn group(title: String) -> Group {
    Group::new().add(Title::new(title))
}

fn text_left(x: i32, y: i32, s: String, size: usize) -> Text {
    Text::new(s)
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("fill", "black")
        .set("style", "text-anchor:start")
}

#[allow(dead_code)]
struct Segments {
    d: String,
}

#[allow(dead_code)]
impl Segments {
    fn new() -> Self {
        Segments { d: String::new() }
    }
    fn add(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.d += &format!("M{} {}L{} {}", x1, y1, x2, y2);
    }
    fn is_empty(&self) -> bool {
        self.d.is_empty()
    }
    fn build(&self, width: usize, color: &str) -> Path {
        Path::new()
            .set("d", self.d.clone())
            .set("stroke", color)
            .set("stroke-width", width)
            .set("fill", "none")
    }
}

fn error_text(x: i32, y: i32, s: String) -> Text {
    text_left(x, y, s, 15)
        .set("fill", "red")
        .set("style", "text-anchor:start;user-select:text;cursor:text")
}

fn info_line(x: i32, y: i32, col_w: i32, size: usize, items: &[String]) -> Group {
    let mut g = Group::new().set("style", "user-select:text;cursor:text");
    for (i, s) in items.iter().enumerate() {
        g = g.add(text_left(x + col_w * i as i32, y, s.clone(), size));
    }
    g
}

pub struct OrderSummary {
    pub start: usize,
    pub moves: usize,
    pub mistake: bool,
}

pub struct VisData {
    output: Output,
    evaluation: Evaluation,
    checkpoints: Vec<State>,
    frame_ends: Vec<usize>,
    pub orders: Vec<OrderSummary>,
    moves: Vec<(usize, usize, usize)>,
}
impl VisData {
    pub fn new(input: &Input, output: Output) -> Self {
        let evaluation = evaluate(input, &output);
        let mut state = State::new(input);
        let mut checkpoints = vec![state.clone()];
        let mut frame_ends = vec![0];
        let mut orders = Vec::new();
        let mut moves = Vec::new();
        let mut start = 0;
        for (i, a) in output.actions.iter().enumerate() {
            let old_mistakes = state.mistakes;
            if let Action::Move(k, d) = a {
                moves.push((*k, state.order.iter().position(|v| v == k).unwrap(), *d));
            }
            state.apply(input, a).unwrap();
            if matches!(a, Action::Finish) {
                orders.push(OrderSummary {
                    start,
                    moves: state.moves - start,
                    mistake: state.mistakes > old_mistakes,
                });
                start = state.moves;
            }
            if matches!(a, Action::Move(_, _)) {
                frame_ends.push(i + 1);
            } else {
                *frame_ends.last_mut().unwrap() = i + 1;
            }
            if (i + 1) % 256 == 0 {
                checkpoints.push(state.clone());
            }
        }
        Self {
            output,
            evaluation,
            checkpoints,
            frame_ends,
            orders,
            moves,
        }
    }
    pub fn verdict(&self) -> &Result<i64, String> {
        self.evaluation.verdict()
    }
    pub fn state_at(&self, input: &Input, t: usize) -> State {
        let t = self.frame_ends[t.min(self.frame_ends.len() - 1)];
        let base = t / 256;
        let mut s = self.checkpoints[base].clone();
        for a in &self.output.actions[base * 256..t] {
            s.apply(input, a).unwrap();
        }
        s
    }
}
pub fn get_max_turn(_input: &Input, data: &VisData, _option: &VisOption) -> usize {
    data.frame_ends.len() - 1
}
pub struct VisResult {
    pub svg: String,
}
fn rect(x: f64, y: f64, w: f64, h: f64, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}
fn draw(
    input: &Input,
    state: &State,
    t: usize,
    score: i64,
    error: &str,
    last: Option<(usize, usize, usize)>,
    changed: u64,
    orders: &[OrderSummary],
) -> String {
    let rows = input.K.div_ceil(3).max(1);
    let graph_y = 330 + rows as i32 * 8 + 48;
    let height = graph_y + 110;
    let mut d = svg::Document::new()
        .set("id", "vis")
        .set("width", 1040)
        .set("height", height)
        .set("viewBox", (0, 0, 1040, height))
        .set("style", "background:#f5f7fa")
        .add(Style::new(
            "text {font-family:sans-serif;dominant-baseline:central;}",
        ));
    d = d.add(rect(12., 10., 1016., 270., "white").set("rx", 12));
    d = d.add(info_line(
        28,
        32,
        240,
        16,
        &[
            format!("t = {}", t),
            format!("score = {}", score),
            format!("E = {}", state.mistakes),
        ],
    ));
    if !error.is_empty() {
        d = d.add(error_text(28, 55, error.into()));
    }
    let (black, exposed) = state.display();
    let r = state.completed.min(input.R - 1);
    let revealed = state.plates.len() == input.K;
    let target = mask(&input.A[input.requests[r]], b'1');
    d = d.add(info_line(
        28,
        78,
        240,
        14,
        &[
            format!("Processed = {}/{}", state.completed, input.R),
            format!("Moves = {}", state.moves),
            if revealed {
                format!("Target = {}", input.requests[r])
            } else {
                "Target = -".into()
            },
            if revealed {
                format!("Mismatch = {}", (black ^ target).count_ones())
            } else {
                "Mismatch = -".into()
            },
        ],
    ));
    let cell = 896. / input.N as f64;
    for (row, bits) in [target, black].iter().enumerate() {
        let y = 108. + row as f64 * 80.;
        d = d.add(text_left(
            28,
            y as i32 + 30,
            if row == 0 { "Target" } else { "Display" }.into(),
            14,
        ));
        if row == 0 && !revealed {
            continue;
        }
        for j in 0..input.N {
            let x = 108. + j as f64 * cell;
            let k = exposed.iter().position(|e| e & (1 << j) != 0);
            let title = if row == 0 {
                format!("window {}: target {}", j, (bits >> j) & 1)
            } else {
                format!(
                    "window {}: {}, visible paper: {}",
                    j,
                    (bits >> j) & 1,
                    k.map_or("background".into(), |k| k.to_string())
                )
            };
            d = d.add(
                group(title).add(
                    rect(
                        x,
                        y,
                        cell,
                        56.,
                        if bits & (1 << j) != 0 {
                            "#202630"
                        } else {
                            "white"
                        },
                    )
                    .set("stroke", "#dce2e8")
                    .set("stroke-width", 0.5),
                ),
            );
            if row == 1 && revealed && (black ^ target) & (1 << j) != 0 {
                d = d.add(rect(x + 1., y + 59., cell - 2., 4., "#dd4855"));
            }
            if row == 1 && changed & (1 << j) != 0 {
                d = d.add(rect(x + 1., y - 7., cell - 2., 4., "#287de0"));
            }
        }
    }
    d = d.add(text_left(20, 302, "Stack".into(), 16));
    if let Some((k, from, to)) = last {
        d = d.add(
            text_left(500, 302, format!("Paper {}: rank {} → {}", k, from, to), 15)
                .set("fill", "#216bc0"),
        );
    }
    for col in 0..3 {
        let first = col * rows;
        if first >= input.K {
            continue;
        }
        let x = 16 + col as i32 * 340;
        d = d.add(rect(x as f64 - 4., 318., 332., (rows * 8 + 18) as f64, "#e4eaf1").set("rx", 5));
        d = d.add(
            text_left(
                x,
                320,
                format!("{} → {}  ↓", first, ((col + 1) * rows).min(input.K) - 1),
                12,
            )
            .set("fill", "#526478"),
        );
    }
    for (rank, &k) in state.order.iter().enumerate() {
        let x = 16 + (rank / rows) as i32 * 340;
        let y = 334 + (rank % rows) as i32 * 8;
        if let Some(p) = state.plates.get(k) {
            let pattern: String = (0..input.N)
                .map(|j| {
                    if p.opaque & (1 << j) == 0 {
                        '.'
                    } else if p.black & (1 << j) == 0 {
                        '0'
                    } else {
                        '1'
                    }
                })
                .collect();
            let mut g = group(format!(
                "rank {}, paper {}; pattern (windows 0..{}): {}",
                rank,
                k,
                input.N - 1,
                pattern
            ));
            g = g.add(rect(x as f64, y as f64 - 3., 320., 6., "#dce5ef"));
            for (bits, color) in [
                (p.opaque & !p.black & !exposed[k], "#f7f9fc"),
                (p.black & !exposed[k], "#8793a3"),
                (exposed[k] & !p.black, "#ffffff"),
                (exposed[k] & p.black, "#202630"),
            ] {
                let mut path = String::new();
                for j in 0..input.N {
                    if bits & (1 << j) != 0 {
                        let xx = x as f64 + j as f64 * 320. / input.N as f64;
                        path += &format!("M{} {}h{}", xx, y, 320. / input.N as f64);
                    }
                }
                if !path.is_empty() {
                    g = g.add(
                        Path::new()
                            .set("d", path)
                            .set("stroke", color)
                            .set("stroke-width", 6)
                            .set("fill", "none"),
                    );
                }
            }
            if exposed[k] != 0 {
                g = g.add(rect(x as f64 - 3., y as f64 - 3., 2., 6., "#e59132"));
            }
            if last.is_some_and(|v| v.0 == k) {
                g = g.add(
                    rect(x as f64 - 1., y as f64 - 4., 322., 8., "none")
                        .set("stroke", "#287de0")
                        .set("stroke-width", 1.5),
                );
            }
            d = d.add(g);
        }
    }
    d = d.add(text_left(20, graph_y - 16, "Moves per order".into(), 15));
    let max_moves = orders.iter().map(|o| o.moves).max().unwrap_or(1).max(1);
    d = d.add(text_left(
        840,
        graph_y - 16,
        format!("max = {}", max_moves),
        13,
    ));
    d = d.add(rect(20., graph_y as f64, 1000., 72., "#e4eaf1"));
    let w = 1000. / input.R as f64;
    for (r, order) in orders.iter().enumerate() {
        let x = 20. + r as f64 * w;
        let h = (order.moves as f64 / max_moves as f64 * 66.).max(3.);
        let color = if order.mistake { "#dd4855" } else { "#718ca7" };
        let title = format!(
            "order {}: target {}, moves {}, {}, t = {}",
            r,
            input.requests[r],
            order.moves,
            if order.mistake {
                "incorrect"
            } else {
                "correct"
            },
            order.start
        );
        d = d.add(
            group(title)
                .set("data-turn", order.start)
                .set("style", "cursor:pointer")
                .add(rect(x, graph_y as f64 + 72. - h, w.max(0.5), h, color))
                .add(rect(x, graph_y as f64, w, 72., "transparent")),
        );
    }
    if revealed {
        let x = 20. + (state.completed.min(input.R - 1) as f64 + 0.5) * w;
        d = d.add(
            Path::new()
                .set("d", format!("M{} {}v76", x, graph_y - 2))
                .set("stroke", "#287de0")
                .set("stroke-width", 2)
                .set("pointer-events", "none"),
        );
    }
    d = d.add(text_left(20, graph_y + 90, "0 →".into(), 12));
    d = d.add(text_left(968, graph_y + 90, format!("{}", input.R - 1), 12));
    d.to_string()
}
pub fn vis(input: &Input, data: &VisData, option: &VisOption) -> VisResult {
    let max = get_max_turn(input, data, option);
    let t = option.t.min(max);
    let s = data.state_at(input, t);
    let (score, error) = if t == max {
        match data.verdict() {
            Ok(v) => (*v, ""),
            Err(e) => (0, e.as_str()),
        }
    } else {
        (score(input.K, s.mistakes, s.moves), "")
    };
    VisResult {
        svg: draw(
            input,
            &s,
            t,
            score,
            error,
            t.checked_sub(1).map(|i| data.moves[i]),
            if t > 0 {
                data.state_at(input, t - 1).display().0 ^ s.display().0
            } else {
                0
            },
            &data.orders,
        ),
    }
}

pub fn draw_parse_error(input: &Input, error: &str, _option: &VisOption) -> String {
    draw(input, &State::new(input), 0, 0, error, None, 0, &[])
}
