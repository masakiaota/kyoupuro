#![allow(non_snake_case, unused_macros)]

use itertools::Itertools;
use proconio::input;
use rand::prelude::*;
use std::ops::RangeBounds;
use svg::node::element::{Group, Rectangle, Style, Text, Title};

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
    a: Vec<Vec<i32>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.N)?;
        for i in 0..self.N {
            writeln!(f, "{}", self.a[i].iter().join(" "))?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        N: usize,
        a: [[i32; N]; N],
    }
    Input { N, a }
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

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Move(usize),
    Take,
    Put,
}

const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];

pub struct Output {
    pub out: Vec<Cmd>,
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut out = vec![];
    for c in f.split_whitespace() {
        if out.len() >= 2 * input.N * input.N * input.N {
            return Err("Too many commands".to_owned());
        }
        match c {
            "U" => out.push(Cmd::Move(0)),
            "D" => out.push(Cmd::Move(1)),
            "L" => out.push(Cmd::Move(2)),
            "R" => out.push(Cmd::Move(3)),
            "Z" => out.push(Cmd::Take),
            "X" => out.push(Cmd::Put),
            _ => return Err(format!("Invalid command: {}", c)),
        }
    }
    Ok(Output { out })
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let N = 20;
    let mut a = vec![];
    for i in 0..N * N {
        a.push(i as i32 / 2);
    }
    a.shuffle(&mut rng);
    Input {
        N,
        a: (0..N).map(|i| a[i * N..(i + 1) * N].to_vec()).collect_vec(),
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, &out.out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err)
}

pub struct State {
    N: usize,
    i: usize,
    j: usize,
    a: Vec<Vec<i32>>,
    deck: Vec<i32>,
    K: usize,
}

impl State {
    pub fn new(input: &Input) -> Self {
        Self {
            N: input.N,
            i: 0,
            j: 0,
            a: input.a.clone(),
            deck: vec![],
            K: 0,
        }
    }
    pub fn apply(&mut self, c: Cmd) -> Result<(), String> {
        match c {
            Cmd::Move(dir) => {
                let i = self.i + DIJ[dir].0;
                let j = self.j + DIJ[dir].1;
                if i >= self.N || j >= self.N {
                    return Err("Illegal action: Move out of bounds".to_owned());
                }
                self.i = i;
                self.j = j;
                self.K += 1;
            }
            Cmd::Take => {
                if self.a[self.i][self.j] == !0 {
                    return Err("Illegal action: Take from empty cell".to_owned());
                }
                self.deck.push(self.a[self.i][self.j]);
                self.a[self.i][self.j] = !0;
                if self.deck.len() >= 2
                    && self.deck[self.deck.len() - 1] == self.deck[self.deck.len() - 2]
                {
                    self.deck.pop();
                    self.deck.pop();
                }
            }
            Cmd::Put => {
                if self.a[self.i][self.j] != !0 {
                    return Err("Illegal action: Put into non-empty cell".to_owned());
                }
                if self.deck.len() == 0 {
                    return Err("Illegal action: Put from empty deck".to_owned());
                }
                let v = self.deck.pop().unwrap();
                self.a[self.i][self.j] = v;
            }
        }
        Ok(())
    }
    pub fn score(&self) -> i64 {
        let mut X = self.deck.len();
        for i in 0..self.N {
            for j in 0..self.N {
                if self.a[i][j] != !0 {
                    X += 1;
                }
            }
        }
        if X == 0 {
            (self.N * self.N + 2 * self.N * self.N * self.N - self.K) as i64
        } else {
            (self.N * self.N - X) as i64
        }
    }
}

pub fn compute_score_details(input: &Input, out: &[Cmd]) -> (i64, String, State) {
    let mut state = State::new(input);
    for &c in out {
        match state.apply(c) {
            Ok(()) => {}
            Err(err) => return (0, err, state),
        }
    }
    (state.score(), String::new(), state)
}

/// 0 <= val <= 1
pub fn color(mut val: f64) -> String {
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
    format!(
        "#{:02x}{:02x}{:02x}80",
        r.round() as i32,
        g.round() as i32,
        b.round() as i32
    )
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
    let (mut score, err, svg, _) = vis(input, &out.out);
    if err.len() > 0 {
        score = 0;
    }
    (score, err, svg)
}

pub fn vis(input: &Input, out: &[Cmd]) -> (i64, String, String, i64) {
    let D = 600 / input.N;
    let W = D * input.N + 2 * D;
    let H = D * input.N;
    let (score, err, state) = compute_score_details(input, &out);
    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, W + 10, H + 10))
        .set("width", W + 10)
        .set("height", H + 10)
        .set("style", "background-color:white");
    doc = doc.add(Style::new(format!(
        "text {{text-anchor: middle;dominant-baseline: central;}}"
    )));
    let mut deck_pos = vec![!0; input.N * input.N / 2];
    for k in 0..state.deck.len() {
        deck_pos[state.deck[k] as usize] = k;
    }
    for i in 0..input.N {
        for j in 0..input.N {
            let mut g = group(format!("({}, {})", i, j));
            let c = if state.a[i][j] == !0 || deck_pos[state.a[i][j] as usize] == !0 {
                "white".to_owned()
            } else if state.deck.len() == 1 {
                color(1.0)
            } else {
                color(deck_pos[state.a[i][j] as usize] as f64 / (state.deck.len() - 1) as f64)
            };
            g = g.add(
                rect(j * D, i * D, D, D, &c)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            if state.a[i][j] != !0 {
                g = g.add(
                    Text::new(format!("{}", state.a[i][j]))
                        .set("x", j * D + D / 2)
                        .set("y", i * D + D / 2)
                        .set("font-size", D / 2 - 2)
                        .set("fill", "black"),
                );
            }
            doc = doc.add(g);
        }
    }
    doc = doc.add(
        rect(state.j * D + 2, state.i * D + 2, D - 4, D - 4, "none")
            .set("stroke", "blue")
            .set("stroke-width", 2),
    );
    if state.deck.len() > input.N {
        doc = doc.add(
            Text::new(format!("︙"))
                .set("x", W - D / 2)
                .set("y", H - D + D / 2)
                .set("font-size", D * 2 / 3)
                .set("fill", "black"),
        );
        for k in 1..input.N {
            let mut g = group(format!("deck[{}]", state.deck.len() - input.N + k));
            let c = color((state.deck.len() - input.N + k) as f64 / (state.deck.len() - 1) as f64);
            g = g.add(
                rect(W - D, H - (k + 1) * D, D, D, &c)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            g = g.add(
                Text::new(format!("{}", state.deck[state.deck.len() - input.N + k]))
                    .set("x", W - D / 2)
                    .set("y", H - (k + 1) * D + D / 2)
                    .set("font-size", D / 2 - 2)
                    .set("fill", "black"),
            );
            doc = doc.add(g);
        }
    } else {
        for k in 0..state.deck.len() {
            let mut g = group(format!("deck[{}]", k));
            let c = if state.deck.len() == 1 {
                color(1.0)
            } else {
                color(k as f64 / (state.deck.len() - 1) as f64)
            };
            g = g.add(
                rect(W - D, H - (k + 1) * D, D, D, &c)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );
            g = g.add(
                Text::new(format!("{}", state.deck[k]))
                    .set("x", W - D / 2)
                    .set("y", H - (k + 1) * D + D / 2)
                    .set("font-size", D / 2 - 2)
                    .set("fill", "black"),
            );
            doc = doc.add(g);
        }
    }
    (score, err, doc.to_string(), state.K as i64)
}
