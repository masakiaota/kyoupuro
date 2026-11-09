#![allow(non_snake_case)]
use proconio::input;
use rand::prelude::*;
use std::io::{BufReader, prelude::*};
use std::ops::RangeBounds;
use std::process::ChildStdout;

#[derive(Clone, Debug)]
pub struct Input {
    pub N: usize,
    pub K: usize,
    pub P: usize,
    pub R: usize,
    pub A: Vec<String>,
    pub requests: Vec<usize>,
}
impl Input {
    pub fn public_text(&self) -> String {
        format!(
            "{} {} {} {}\n{}",
            self.N,
            self.K,
            self.P,
            self.R,
            self.A.join("\n")
        )
    }
}
impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.public_text())?;
        for a in &self.requests {
            writeln!(f, "{}", a)?;
        }
        Ok(())
    }
}
pub fn parse_input(text: &str) -> Input {
    let source = proconio::source::once::OnceSource::from(text);
    input! { from source, N:usize,K:usize,P:usize,R:usize,A:[String;P],requests:[usize;R] }
    Input {
        N,
        K,
        P,
        R,
        A,
        requests,
    }
}
#[derive(Clone, Debug, Default)]
pub struct GenOption {}
impl GenOption {
    pub fn set_i32(&mut self, name: &str, _value: i32) -> Result<(), String> {
        Err(format!("Unknown generator option: {}", name))
    }
}
pub fn generate(seed: u64, _option: &GenOption) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed ^ 0xA074);
    let mut A = Vec::new();
    while A.len() < 256 {
        let mut bits = vec![b'0'; 64];
        bits[..32].fill(b'1');
        bits.shuffle(&mut rng);
        let s = String::from_utf8(bits).unwrap();
        if !A.contains(&s) {
            A.push(s);
        }
    }
    let mut requests = vec![rng.random_range(0usize..256)];
    for r in 1..1024 {
        let x = rng.random_range(0usize..255);
        requests.push(x + usize::from(x >= requests[r - 1]));
    }
    Input {
        N: 64,
        K: 192,
        P: 256,
        R: 1024,
        A,
        requests,
    }
}
#[derive(Clone, Debug)]
pub struct Plate {
    pub opaque: u64,
    pub black: u64,
}
#[derive(Clone, Debug)]
pub enum Action {
    Design(Plate),
    Move(usize, usize),
    Finish,
}
#[derive(Clone, Debug)]
pub struct State {
    pub plates: Vec<Plate>,
    pub order: Vec<usize>,
    pub completed: usize,
    pub moves: usize,
    pub mistakes: usize,
}
impl State {
    pub fn new(input: &Input) -> Self {
        Self {
            plates: vec![],
            order: (0..input.K).collect(),
            completed: 0,
            moves: 0,
            mistakes: 0,
        }
    }
    pub fn display(&self) -> (u64, Vec<u64>) {
        let mut covered = 0;
        let mut black = 0;
        let mut exposed = vec![0; self.order.len()];
        for &k in &self.order {
            if let Some(p) = self.plates.get(k) {
                let e = p.opaque & !covered;
                exposed[k] = e;
                black |= e & p.black;
                covered |= p.opaque;
            }
        }
        (black, exposed)
    }
    pub fn apply(&mut self, input: &Input, action: &Action) -> Result<(), String> {
        match action {
            Action::Design(p) => self.plates.push(p.clone()),
            Action::Move(k, d) => {
                let mut exposed = self.plates[*k].opaque;
                let mut pos = 0;
                while exposed != 0 && self.order[pos] != *k {
                    exposed &= !self.plates[self.order[pos]].opaque;
                    pos += 1;
                }
                if exposed == 0 {
                    return Err(format!("Paper {} is not visible", k));
                }
                if pos < *d {
                    self.order[pos..=*d].rotate_left(1);
                } else {
                    self.order[*d..=pos].rotate_right(1);
                }
                self.moves += 1;
            }
            Action::Finish => {
                let full = u64::MAX >> (64 - input.N);
                let mut covered = 0;
                let mut black = 0;
                for &k in &self.order {
                    let p = &self.plates[k];
                    black |= p.black & !covered;
                    covered |= p.opaque;
                    if covered == full {
                        break;
                    }
                }
                if black != mask(&input.A[input.requests[self.completed]], b'1') {
                    self.mistakes += 1;
                }
                self.completed += 1;
            }
        }
        Ok(())
    }
}
pub fn mask(s: &str, c: u8) -> u64 {
    s.bytes()
        .enumerate()
        .fold(0, |m, (j, b)| m | if b == c { 1u64 << j } else { 0 })
}
pub fn score(k: usize, e: usize, t: usize) -> i64 {
    2 * k as i64 * e as i64 + t as i64
}

#[derive(Clone, Debug)]
pub struct Output {
    error: String,
    pub actions: Vec<Action>,
    pub state: State,
    pub sent: Vec<String>,
}
impl Output {
    fn new(input: &Input) -> Self {
        Self {
            error: String::new(),
            actions: vec![],
            state: State::new(input),
            sent: vec![input.public_text()],
        }
    }
    pub fn error(&self) -> Option<&str> {
        (!self.error.is_empty()).then_some(self.error.as_str())
    }
}
fn interact(
    input: &Input,
    mut receive: impl FnMut() -> Result<Option<String>, String>,
    mut send: impl FnMut(&str),
) -> Result<Output, String> {
    let mut out = Output::new(input);
    send(&out.sent[0]);
    let result = (|| -> Result<(), String> {
        for _ in 0..input.K {
            let pattern = receive()?.ok_or("Unexpected EOF")?;
            if pattern.len() != input.N || !pattern.bytes().all(|c| matches!(c, b'0' | b'1' | b'.'))
            {
                return Err("Expected a paper pattern of length N using 0, 1, .".into());
            }
            let action = Action::Design(Plate {
                opaque: mask(&pattern, b'0') | mask(&pattern, b'1'),
                black: mask(&pattern, b'1'),
            });
            out.state.apply(input, &action)?;
            out.actions.push(action);
        }
        for r in 0..input.R {
            let request = input.requests[r].to_string();
            send(&request);
            out.sent.push(request);
            let count: usize = read(receive()?.as_deref(), 0..=2 * input.K)?;
            for _ in 0..count {
                let k = read(receive()?.as_deref(), 0..input.K)?;
                let d = read(receive()?.as_deref(), 0..input.K)?;
                let action = Action::Move(k, d);
                out.state.apply(input, &action)?;
                out.actions.push(action);
            }
            out.state.apply(input, &Action::Finish)?;
            out.actions.push(Action::Finish);
        }
        Ok(())
    })();
    if let Err(error) = result {
        out.error = error;
        send("-1");
        out.sent.push("-1".into());
    }
    Ok(out)
}
fn abbreviate(s: &str) -> String {
    const LIMIT: usize = 40;
    if s.chars().count() <= LIMIT {
        s.to_owned()
    } else {
        format!("{}...", s.chars().take(LIMIT).collect::<String>())
    }
}

#[allow(dead_code)]
fn read<T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr, R: RangeBounds<T>>(
    token: Option<&str>,
    range: R,
) -> Result<T, String> {
    let Some(token) = token else {
        return Err("Unexpected EOF".to_owned());
    };
    let Ok(value) = token.parse::<T>() else {
        return Err(format!("Parse error: {}", abbreviate(token)));
    };
    if range.contains(&value) {
        Ok(value)
    } else {
        Err(format!("Out of range: {}", value))
    }
}

fn next_token(
    receive_raw: &mut impl FnMut() -> Result<Option<String>, String>,
    pending: &mut std::collections::VecDeque<String>,
) -> Result<Option<String>, String> {
    loop {
        if let Some(token) = pending.pop_front() {
            return Ok(Some(token));
        }
        let Some(line) = receive_raw()? else {
            return Ok(None);
        };
        pending.extend(line.split_whitespace().map(str::to_owned));
    }
}

fn finish_output(
    out: &mut Output,
    mut receive: impl FnMut() -> Result<Option<String>, String>,
) -> Result<(), String> {
    if out.error().is_none() {
        while receive()?.is_some() {
            out.error = "Extra output after the last turn".into();
        }
    }
    Ok(())
}

pub fn parse_output(input: &Input, text: &str) -> Result<Output, String> {
    let mut tokens = text.split_whitespace();
    let mut receive = || Ok(tokens.next().map(str::to_owned));
    let mut out = interact(input, &mut receive, |_| {})?;
    finish_output(&mut out, &mut receive)?;
    Ok(out)
}

pub struct Evaluation {
    verdict: Result<i64, String>,
}

impl Evaluation {
    pub fn verdict(&self) -> &Result<i64, String> {
        &self.verdict
    }
}

pub fn evaluate(_input: &Input, out: &Output) -> Evaluation {
    let verdict = if let Some(error) = out.error() {
        Err(error.to_owned())
    } else {
        Ok(score(_input.K, out.state.mistakes, out.state.moves))
    };
    Evaluation { verdict }
}

fn read_line(
    stdout: &mut BufReader<ChildStdout>,
    relay: &mut impl Write,
) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    match stdout.read_until(b'\n', &mut bytes) {
        Ok(0) => Ok(None),
        Err(_) => Err("Your program has terminated unexpectedly".to_owned()),
        Ok(_) => {
            relay.write_all(&bytes).map_err(|error| error.to_string())?;
            let line = match String::from_utf8(bytes) {
                Ok(line) => line,
                Err(_) => {
                    return Err("not utf-8".to_owned());
                }
            };
            Ok(Some(line))
        }
    }
}

fn wait_briefly(p: &mut std::process::Child) {
    for _ in 0..10 {
        match p.try_wait() {
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(1)),
            _ => return,
        }
    }
}

pub fn exec(p: &mut std::process::Child, _local: bool) -> Result<(Input, Output), String> {
    let mut input_text = String::new();
    std::io::stdin().read_to_string(&mut input_text).unwrap();
    let input = parse_input(&input_text);
    let mut stdin = std::io::BufWriter::new(p.stdin.take().unwrap());
    let mut stdout = BufReader::new(p.stdout.take().unwrap());
    let relay_stdout = std::io::stdout();
    let mut relay = std::io::BufWriter::with_capacity(65536, relay_stdout.lock());
    let closed = std::cell::Cell::new(false);
    let mut receive_raw = || {
        let result = read_line(&mut stdout, &mut relay);
        if !matches!(result, Ok(Some(_))) {
            closed.set(true);
        }
        result
    };
    let mut pending = std::collections::VecDeque::new();
    let mut receive = || next_token(&mut receive_raw, &mut pending);
    let result = interact(&input, &mut receive, |text| {
        let _ = writeln!(stdin, "{}", text);
        let _ = stdin.flush();
    });
    let ran_out = closed.get();
    drop(stdin);
    let mut out = match result {
        Ok(out) => out,
        Err(error) => {
            if ran_out {
                wait_briefly(p);
            }
            return Err(error);
        }
    };
    if out.error().is_none() {
        finish_output(&mut out, &mut receive)?;
        if ran_out {
            wait_briefly(p);
        } else {
            let _ = p.wait();
        }
    } else if ran_out {
        wait_briefly(p);
    }
    relay.flush().map_err(|error| error.to_string())?;
    Ok((input, out))
}

pub mod vis;
