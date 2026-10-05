use rand::prelude::*;
use std::ops::RangeBounds;
#[derive(Clone, Debug)]
pub struct Input {
    pub n: usize,
    pub k: usize,
    pub floor: Vec<bool>,
    pub a: Vec<usize>,
    pub plates: Vec<usize>,
    pub b: Vec<usize>,
}
impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {}", self.n, self.k)?;
        let mut grid = (0..self.n * self.n)
            .map(|p| {
                if !self.floor[p] {
                    '#'
                } else if self.a[p] > 0 {
                    (b'a' + (self.a[p] - 1) as u8) as char
                } else {
                    '.'
                }
            })
            .collect::<Vec<_>>();
        for (i, &p) in self.plates.iter().enumerate() {
            grid[p] = (b'A' + i as u8) as char;
        }
        for row in grid.chunks(self.n) {
            writeln!(f, "{}", row.iter().collect::<String>())?;
        }
        Ok(())
    }
}
pub fn parse_input(text: &str) -> Input {
    let mut it = text.split_whitespace();
    let n = it.next().unwrap().parse().unwrap();
    let k = it.next().unwrap().parse().unwrap();
    let mut floor = vec![true; n * n];
    let mut a = vec![0; n * n];
    let mut plates = vec![0; k];
    let mut b = vec![0; k];
    for i in 0..n {
        for (j, c) in it.next().unwrap().bytes().enumerate() {
            let p = i * n + j;
            match c {
                b'#' => floor[p] = false,
                b'.' => {}
                b'a'..=b'z' => {
                    let color = (c - b'a') as usize;
                    a[p] = color + 1;
                    b[color] += 1;
                }
                b'A'..=b'Z' => plates[(c - b'A') as usize] = p,
                _ => panic!("Invalid grid character"),
            }
        }
    }
    Input {
        n,
        k,
        floor,
        a,
        plates,
        b,
    }
}
fn abbreviate(s: &str) -> String {
    s.chars().take(32).collect()
}
fn read<T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr, R: RangeBounds<T>>(
    s: Option<&str>,
    range: R,
) -> Result<T, String> {
    let s = s.ok_or("Unexpected EOF")?;
    let v = s
        .parse::<T>()
        .map_err(|_| format!("Parse error: {}", abbreviate(s)))?;
    if range.contains(&v) {
        Ok(v)
    } else {
        Err(format!("Out of range: {}", v))
    }
}
#[derive(Clone, Debug, Default)]
pub struct GenOption {
    pub n: Option<usize>,
    pub k: Option<usize>,
    pub rho: Option<f64>,
    pub beta: Option<f64>,
}
impl GenOption {
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), String> {
        match name {
            "N" if (12..=20).contains(&value) => self.n = Some(value as usize),
            "K" if (4..=12).contains(&value) => self.k = Some(value as usize),
            _ => return Err(format!("Invalid generator option: {}={}", name, value)),
        }
        Ok(())
    }
    pub fn set_f64(&mut self, name: &str, value: f64) -> Result<(), String> {
        match name {
            "rho" if (0.12..0.65).contains(&value) => self.rho = Some(value),
            "beta" if (0.0..0.40).contains(&value) => self.beta = Some(value),
            _ => return Err(format!("Invalid generator option: {}={}", name, value)),
        };
        Ok(())
    }
}
pub const DIR: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
pub fn neighbor(n: usize, p: usize, d: usize, l: usize) -> Option<usize> {
    let (r, c) = (p / n, p % n);
    let r = r as isize + DIR[d].0 * l as isize;
    let c = c as isize + DIR[d].1 * l as isize;
    if r < 0 || c < 0 || r >= n as isize || c >= n as isize {
        None
    } else {
        Some(r as usize * n + c as usize)
    }
}
pub fn connected(n: usize, floor: &[bool]) -> bool {
    let Some(start) = floor.iter().position(|&v| v) else {
        return false;
    };
    let mut seen = vec![false; n * n];
    seen[start] = true;
    let mut q = vec![start];
    let mut at = 0;
    while at < q.len() {
        let p = q[at];
        at += 1;
        for d in 0..4 {
            if let Some(v) = neighbor(n, p, d, 1) {
                if floor[v] && !seen[v] {
                    seen[v] = true;
                    q.push(v);
                }
            }
        }
    }
    q.len() == floor.iter().filter(|&&v| v).count()
}
pub fn generate(seed: u64, option: &GenOption) -> Input {
    if seed == 0
        && option.n.is_none()
        && option.k.is_none()
        && option.rho.is_none()
        && option.beta.is_none()
    {
        return parse_input(
            "12 4
............
...c.##.....
.A..c##.....
...c#.##....
c..##.###B..
aa.#...##.b.
..##C..###..
.a#.d#..##.b
a##d.##b.##.
d...####b...
...######...
.d.......D..
",
        );
    }
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut n = rng.random_range(12..=20);
    if let Some(v) = option.n {
        n = v;
    }
    let mut k = rng.random_range(4..=12);
    if let Some(v) = option.k {
        k = v;
    }
    let mut rho = rng.random_range(0.12..0.65);
    if let Some(v) = option.rho {
        rho = v;
    }
    let mut beta = rng.random_range(0.0..0.40);
    if let Some(v) = option.beta {
        beta = v;
    }
    let mut floor = vec![true; n * n];
    let mut order = (0..n * n).collect::<Vec<_>>();
    order.shuffle(&mut rng);
    let mut count = n * n;
    for p in order {
        if rng.random::<f64>() < beta && count > 2 * k {
            floor[p] = false;
            if connected(n, &floor) {
                count -= 1;
            } else {
                floor[p] = true;
            }
        }
    }
    let mut cells = (0..n * n).filter(|&p| floor[p]).collect::<Vec<_>>();
    cells.shuffle(&mut rng);
    let plates = cells[..k].to_vec();
    let f = cells.len() - k;
    let m = k.max((rho * f as f64).round() as usize);
    let positions = &cells[k..k + m];
    let mut cuts = (0..k - 1)
        .map(|_| rng.random_range(0..=(m - k) as u32) as usize)
        .collect::<Vec<_>>();
    cuts.extend([0, m - k]);
    cuts.sort_unstable();
    let b = cuts.windows(2).map(|w| w[1] - w[0] + 1).collect::<Vec<_>>();
    let mut colors = (0..k).flat_map(|i| vec![i + 1; b[i]]).collect::<Vec<_>>();
    colors.shuffle(&mut rng);
    let mut a = vec![0; n * n];
    for (&p, &c) in positions.iter().zip(&colors) {
        a[p] = c;
    }
    Input {
        n,
        k,
        floor,
        a,
        plates,
        b,
    }
}
#[derive(Clone, Debug)]
pub struct Operation {
    pub p: usize,
    pub k: usize,
    pub d: usize,
    pub l: usize,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub ops: Vec<Operation>,
}
pub fn parse_output(input: &Input, text: &str) -> Result<Output, String> {
    let mut it = text.split_whitespace();
    let mut ops = vec![];
    while let Some(s) = it.next() {
        if ops.len() == 100000 {
            return Err("Too many operations".into());
        }
        let r = read(Some(s), 0..input.n)?;
        let c = read(it.next(), 0..input.n)?;
        let k = read(it.next(), 0..8)?;
        let d = match it.next() {
            Some("U") => 0,
            Some("D") => 1,
            Some("L") => 2,
            Some("R") => 3,
            _ => return Err("Direction must be U, D, L or R".into()),
        };
        let l = read(it.next(), 1..=8)?;
        ops.push(Operation {
            p: r * input.n + c,
            k,
            d,
            l,
        });
    }
    Ok(Output { ops })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub stacks: Vec<Vec<usize>>,
    pub received: Vec<usize>,
    pub t: usize,
}
impl State {
    pub fn new(input: &Input) -> Self {
        Self {
            stacks: input
                .a
                .iter()
                .map(|&c| if c == 0 { vec![] } else { vec![c] })
                .collect(),
            received: vec![0; input.k],
            t: 0,
        }
    }
    pub fn score(&self, input: &Input) -> i64 {
        self.t as i64
            + 100000 * (input.b.iter().sum::<usize>() - self.received.iter().sum::<usize>()) as i64
    }
    pub fn apply(&mut self, input: &Input, o: &Operation) -> Result<(), String> {
        let h = self.stacks[o.p].len();
        if !input.floor[o.p] || o.k >= h {
            return Err("Source is empty or k is not below height".into());
        }
        if o.l > o.k + 1 {
            return Err("Jump length exceeds k + 1".into());
        }
        let mut q = o.p;
        for l in 1..=o.l {
            q = neighbor(input.n, o.p, o.d, l).ok_or("Jump leaves board")?;
            if !input.floor[q] {
                return Err("Jump crosses a wall".into());
            }
        }
        if self.stacks[q].len() + h - o.k > 8 {
            return Err("Landing height exceeds 8 before receiving".into());
        }
        let mut block = self.stacks[o.p].split_off(o.k);
        block.reverse();
        self.stacks[q].extend(block);
        for p in [o.p, q] {
            if let Some(i) = input.plates.iter().position(|&v| v == p) {
                while self.stacks[p].last() == Some(&(i + 1)) {
                    self.stacks[p].pop();
                    self.received[i] += 1;
                }
            }
        }
        self.t += 1;
        Ok(())
    }
}
pub struct Evaluation {
    verdict: Result<i64, String>,
}
impl Evaluation {
    pub fn verdict(&self) -> &Result<i64, String> {
        &self.verdict
    }
}
pub fn evaluate(input: &Input, out: &Output) -> Evaluation {
    let mut s = State::new(input);
    for (i, o) in out.ops.iter().enumerate() {
        if let Err(e) = s.apply(input, o) {
            return Evaluation {
                verdict: Err(format!("Operation {}: {}", i + 1, e)),
            };
        }
    }
    Evaluation {
        verdict: Ok(s.score(input)),
    }
}
pub mod vis;
