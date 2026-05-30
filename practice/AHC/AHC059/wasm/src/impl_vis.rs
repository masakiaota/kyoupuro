use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use svg::node::element::{Group, Rectangle, Style, Text, Title};

const EMPTY: i32 = -1;
const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[derive(Clone, Debug)]
struct Input {
    n: usize,
    a: Vec<Vec<i32>>,
}

#[derive(Clone, Copy, Debug)]
enum Cmd {
    Move(usize),
    Take,
    Put,
}

struct Output {
    out: Vec<Cmd>,
}

#[derive(Clone, Debug)]
struct State {
    n: usize,
    i: usize,
    j: usize,
    a: Vec<Vec<i32>>,
    deck: Vec<i32>,
    k: usize,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.n)?;
        for i in 0..self.n {
            for j in 0..self.n {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.a[i][j])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn parse_input(input: &str) -> Result<Input, String> {
    let mut it = input.split_whitespace();
    let n = it
        .next()
        .ok_or_else(|| "Unexpected EOF while reading N".to_owned())?
        .parse::<usize>()
        .map_err(|_| "Parse error: N".to_owned())?;
    let mut a = vec![vec![0; n]; n];
    for row in a.iter_mut().take(n) {
        for cell in row.iter_mut().take(n) {
            *cell = it
                .next()
                .ok_or_else(|| "Unexpected EOF while reading a[i,j]".to_owned())?
                .parse::<i32>()
                .map_err(|_| "Parse error: a[i,j]".to_owned())?;
        }
    }
    Ok(Input { n, a })
}

fn parse_output(input: &Input, output: &str) -> Result<Output, String> {
    let mut out = Vec::new();
    for token in output.split_whitespace() {
        if out.len() >= 2 * input.n * input.n * input.n {
            return Err("Too many commands".to_owned());
        }
        match token {
            "U" => out.push(Cmd::Move(0)),
            "D" => out.push(Cmd::Move(1)),
            "L" => out.push(Cmd::Move(2)),
            "R" => out.push(Cmd::Move(3)),
            "Z" => out.push(Cmd::Take),
            "X" => out.push(Cmd::Put),
            _ => return Err(format!("Invalid command: {}", token)),
        }
    }
    Ok(Output { out })
}

fn gen(seed: u64) -> Input {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let n = 20;
    let mut cards = Vec::with_capacity(n * n);
    for i in 0..n * n {
        cards.push((i / 2) as i32);
    }
    cards.shuffle(&mut rng);
    let a = (0..n).map(|i| cards[i * n..(i + 1) * n].to_vec()).collect();
    Input { n, a }
}

impl State {
    fn new(input: &Input) -> Self {
        Self {
            n: input.n,
            i: 0,
            j: 0,
            a: input.a.clone(),
            deck: Vec::new(),
            k: 0,
        }
    }

    fn apply(&mut self, c: Cmd) -> Result<(), String> {
        match c {
            Cmd::Move(dir) => {
                let (di, dj) = DIRS[dir];
                let ni = self.i as isize + di;
                let nj = self.j as isize + dj;
                if ni < 0 || nj < 0 || ni >= self.n as isize || nj >= self.n as isize {
                    return Err("Illegal action: Move out of bounds".to_owned());
                }
                self.i = ni as usize;
                self.j = nj as usize;
                self.k += 1;
            }
            Cmd::Take => {
                if self.a[self.i][self.j] == EMPTY {
                    return Err("Illegal action: Take from empty cell".to_owned());
                }
                self.deck.push(self.a[self.i][self.j]);
                self.a[self.i][self.j] = EMPTY;
                if self.deck.len() >= 2
                    && self.deck[self.deck.len() - 1] == self.deck[self.deck.len() - 2]
                {
                    self.deck.pop();
                    self.deck.pop();
                }
            }
            Cmd::Put => {
                if self.a[self.i][self.j] != EMPTY {
                    return Err("Illegal action: Put into non-empty cell".to_owned());
                }
                let v = self
                    .deck
                    .pop()
                    .ok_or_else(|| "Illegal action: Put from empty deck".to_owned())?;
                self.a[self.i][self.j] = v;
            }
        }
        Ok(())
    }

    fn score(&self) -> i64 {
        let mut x = self.deck.len();
        for row in &self.a {
            for &v in row {
                if v != EMPTY {
                    x += 1;
                }
            }
        }
        if x == 0 {
            (self.n * self.n + 2 * self.n * self.n * self.n - self.k) as i64
        } else {
            (self.n * self.n - x) as i64
        }
    }
}

fn compute_score_details(input: &Input, out: &[Cmd]) -> (i64, String, State) {
    let mut state = State::new(input);
    for &c in out {
        if let Err(err) = state.apply(c) {
            return (0, err, state);
        }
    }
    (state.score(), String::new(), state)
}

fn color(mut val: f64) -> String {
    val = val.clamp(0.0, 1.0);
    let (r, g, b) = if val < 0.5 {
        let x = val * 2.0;
        (
            30.0 * (1.0 - x) + 144.0 * x,
            144.0 * (1.0 - x) + 255.0 * x,
            255.0 * (1.0 - x) + 30.0 * x,
        )
    } else {
        let x = val * 2.0 - 1.0;
        (
            144.0 * (1.0 - x) + 255.0 * x,
            255.0 * (1.0 - x) + 30.0 * x,
            30.0 * (1.0 - x) + 70.0 * x,
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}80",
        r.round() as i32,
        g.round() as i32,
        b.round() as i32
    )
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
    Group::new().add(Title::new(title))
}

fn vis(input: &Input, out: &[Cmd]) -> (i64, String, String, i64) {
    let d = 600 / input.n;
    let w = d * input.n + 2 * d;
    let h = d * input.n;
    let (score, err, state) = compute_score_details(input, out);
    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, w + 10, h + 10))
        .set("width", w + 10)
        .set("height", h + 10)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(
        "text {text-anchor: middle;dominant-baseline: central;}".to_owned(),
    ));

    let mut deck_pos = vec![usize::MAX; input.n * input.n / 2];
    for (idx, &v) in state.deck.iter().enumerate() {
        deck_pos[v as usize] = idx;
    }

    for i in 0..input.n {
        for j in 0..input.n {
            let mut g = group(format!("({}, {})", i, j));
            let cell = state.a[i][j];
            let fill = if cell == EMPTY || deck_pos[cell as usize] == usize::MAX {
                "white".to_owned()
            } else if state.deck.len() == 1 {
                color(1.0)
            } else {
                color(deck_pos[cell as usize] as f64 / (state.deck.len() - 1) as f64)
            };
            g = g.add(
                rect(j * d, i * d, d, d, &fill)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            if cell != EMPTY {
                g = g.add(
                    Text::new(format!("{}", cell))
                        .set("x", j * d + d / 2)
                        .set("y", i * d + d / 2)
                        .set("font-size", d / 2 - 2)
                        .set("fill", "black"),
                );
            }
            doc = doc.add(g);
        }
    }

    doc = doc.add(
        rect(state.j * d + 2, state.i * d + 2, d - 4, d - 4, "none")
            .set("stroke", "blue")
            .set("stroke-width", 2),
    );

    if state.deck.len() > input.n {
        doc = doc.add(
            Text::new("...".to_owned())
                .set("x", w - d / 2)
                .set("y", h - d + d / 2)
                .set("font-size", d * 2 / 3)
                .set("fill", "black"),
        );
        for k in 1..input.n {
            let deck_idx = state.deck.len() - input.n + k;
            let mut g = group(format!("deck[{}]", deck_idx));
            let fill = color(deck_idx as f64 / (state.deck.len() - 1) as f64);
            g = g.add(
                rect(w - d, h - (k + 1) * d, d, d, &fill)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            g = g.add(
                Text::new(format!("{}", state.deck[deck_idx]))
                    .set("x", w - d / 2)
                    .set("y", h - (k + 1) * d + d / 2)
                    .set("font-size", d / 2 - 2)
                    .set("fill", "black"),
            );
            doc = doc.add(g);
        }
    } else {
        for k in 0..state.deck.len() {
            let mut g = group(format!("deck[{}]", k));
            let fill = if state.deck.len() == 1 {
                color(1.0)
            } else {
                color(k as f64 / (state.deck.len() - 1) as f64)
            };
            g = g.add(
                rect(w - d, h - (k + 1) * d, d, d, &fill)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            g = g.add(
                Text::new(format!("{}", state.deck[k]))
                    .set("x", w - d / 2)
                    .set("y", h - (k + 1) * d + d / 2)
                    .set("font-size", d / 2 - 2)
                    .set("fill", "black"),
            );
            doc = doc.add(g);
        }
    }

    (score, err, doc.to_string(), state.k as i64)
}

pub fn generate(seed: i32) -> String {
    gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    let Ok(input) = parse_input(input) else {
        return output.split_whitespace().count();
    };
    match parse_output(&input, output) {
        Ok(out) => out.out.len(),
        Err(_) => output.split_whitespace().count(),
    }
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let input = parse_input(input)?;
    let out = parse_output(&input, output)?;
    let shown_turn = turn.min(out.out.len());
    let (mut score, err, svg, _) = vis(&input, &out.out[..shown_turn]);
    if !err.is_empty() {
        score = 0;
    }
    Ok((score, err, svg))
}
