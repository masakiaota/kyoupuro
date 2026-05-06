// profile_v902_reference_complete.rs
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;
use std::time::Instant;

mod prof {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    #[derive(Default)]
    struct Stats {
        time_ns: HashMap<&'static str, u128>,
        counts: HashMap<&'static str, u64>,
        values_sum: HashMap<&'static str, u128>,
        values_max: HashMap<&'static str, u64>,
    }

    static STATS: OnceLock<Mutex<Stats>> = OnceLock::new();

    fn stats() -> &'static Mutex<Stats> {
        STATS.get_or_init(|| Mutex::new(Stats::default()))
    }

    pub struct PrintOnDrop;

    impl Drop for PrintOnDrop {
        fn drop(&mut self) {
            print();
        }
    }

    #[inline]
    pub fn start() -> Instant {
        Instant::now()
    }

    #[inline]
    pub fn finish(label: &'static str, start: Instant) {
        add_time_ns(label, start.elapsed().as_nanos(), 1);
    }

    #[inline]
    pub fn add_time_ns(label: &'static str, ns: u128, count: u64) {
        let mut stats = stats().lock().unwrap();
        *stats.time_ns.entry(label).or_insert(0) += ns;
        *stats.counts.entry(label).or_insert(0) += count;
    }

    #[inline]
    pub fn inc(label: &'static str) {
        let mut stats = stats().lock().unwrap();
        *stats.counts.entry(label).or_insert(0) += 1;
    }

    #[inline]
    pub fn add_value(label: &'static str, value: u64) {
        let mut stats = stats().lock().unwrap();
        *stats.values_sum.entry(label).or_insert(0) += value as u128;
        stats
            .values_max
            .entry(label)
            .and_modify(|current| *current = (*current).max(value))
            .or_insert(value);
        *stats.counts.entry(label).or_insert(0) += 1;
    }

    pub fn print() {
        let stats = stats().lock().unwrap();
        let mut keys = stats.time_ns.keys().copied().collect::<Vec<_>>();
        keys.sort_unstable();
        eprintln!("!profile section=timers");
        for key in keys {
            let ns = stats.time_ns[key];
            let count = stats.counts.get(key).copied().unwrap_or(0);
            eprintln!(
                "!profile timer name={} count={} total_ms={:.3} avg_us={:.3}",
                key,
                count,
                ns as f64 / 1.0e6,
                if count == 0 {
                    0.0
                } else {
                    ns as f64 / count as f64 / 1.0e3
                }
            );
        }

        let mut value_keys = stats.values_sum.keys().copied().collect::<Vec<_>>();
        value_keys.sort_unstable();
        eprintln!("!profile section=values");
        for key in value_keys {
            let sum = stats.values_sum[key];
            let count = stats.counts.get(key).copied().unwrap_or(0);
            let max = stats.values_max.get(key).copied().unwrap_or(0);
            eprintln!(
                "!profile value name={} count={} sum={} avg={:.3} max={}",
                key,
                count,
                sum,
                if count == 0 {
                    0.0
                } else {
                    sum as f64 / count as f64
                },
                max
            );
        }
    }
}

const DIJ: [(isize, isize); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
const SMALL_VALUE: f64 = 1.0e-6;
const BITSET_WORDS: usize = 7;

#[derive(Clone)]
struct Xorshift {
    x: u64,
}

impl Xorshift {
    fn new(seed: u64) -> Self {
        assert!(seed != 0);
        Self { x: seed }
    }

    fn next(&mut self) -> u64 {
        self.x ^= self.x << 13;
        self.x ^= self.x >> 17;
        self.x ^= self.x << 5;
        self.x
    }

    fn randrange(&mut self, stop: usize) -> usize {
        assert!(stop > 0);
        (self.next() % stop as u64) as usize
    }

    fn random(&mut self) -> f64 {
        self.next() as f64 * (1.0 / u64::MAX as f64)
    }

    fn gen_bool(&mut self, p: f64) -> bool {
        self.random() < p
    }
}

struct Scanner<R> {
    reader: R,
    buffer: Vec<String>,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    fn next<T: FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("failed to parse token");
            }

            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .expect("failed to read line");
            if bytes == 0 {
                panic!("unexpected EOF");
            }
            self.buffer = line
                .split_whitespace()
                .rev()
                .map(|token| token.to_string())
                .collect();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BitSet400 {
    words: [u64; BITSET_WORDS],
}

impl BitSet400 {
    fn new() -> Self {
        Self {
            words: [0; BITSET_WORDS],
        }
    }

    fn set(&mut self, index: usize, value: bool) {
        if value {
            self.words[index >> 6] |= 1_u64 << (index & 63);
        } else {
            self.words[index >> 6] &= !(1_u64 << (index & 63));
        }
    }

    fn get(&self, index: usize) -> bool {
        ((self.words[index >> 6] >> (index & 63)) & 1) != 0
    }

    fn or_assign(&mut self, other: &Self) {
        for i in 0..BITSET_WORDS {
            self.words[i] |= other.words[i];
        }
    }

    fn shifted_left(&self, shift: usize) -> Self {
        let mut result = Self::new();
        let word_shift = shift >> 6;
        let bit_shift = shift & 63;

        for src in 0..BITSET_WORDS {
            let word = self.words[src];
            if word == 0 {
                continue;
            }
            let dst = src + word_shift;
            if dst < BITSET_WORDS {
                result.words[dst] |= word << bit_shift;
            }
            if bit_shift != 0 && dst + 1 < BITSET_WORDS {
                result.words[dst + 1] |= word >> (64 - bit_shift);
            }
        }
        result
    }
}

#[derive(Clone)]
struct OilLayout {
    hash: u64,
    ln_p_r_if_x: f64,
    px_if_r: f64,
    top_lefts: Vec<usize>,
    volume: Vec<u8>,
}

#[derive(Clone)]
struct OilShape {
    max_i: usize,
    max_j: usize,
    coordinate_ids: Vec<usize>,
    coordinates: Vec<(usize, usize)>,
    mask: BitSet400,
}

struct Input {
    n: usize,
    n2: usize,
    m: usize,
    eps: f64,
    oils: Vec<OilShape>,
    total: usize,
}

impl Input {
    fn get_positives(&self, top_lefts: &[usize]) -> BitSet400 {
        let mut positives = BitSet400::new();
        for (oil_id, &top_left) in top_lefts.iter().enumerate().take(self.m) {
            positives.or_assign(&self.oils[oil_id].mask.shifted_left(top_left));
        }
        positives
    }

    fn get_volume(&self, top_lefts: &[usize]) -> Vec<u8> {
        let mut volume = vec![0_u8; self.n2];
        for (oil_id, &pij) in top_lefts.iter().enumerate() {
            for &ij in &self.oils[oil_id].coordinate_ids {
                volume[ij + pij] = volume[ij + pij].wrapping_add(1);
            }
        }
        volume
    }
}

fn read_input<R: BufRead>(scanner: &mut Scanner<R>) -> Input {
    let n: usize = scanner.next();
    let m: usize = scanner.next();
    let eps: f64 = scanner.next();
    let n2 = n * n;
    let mut oils = Vec::with_capacity(m);

    for _ in 0..m {
        let t_size: usize = scanner.next();
        let mut coordinates = Vec::with_capacity(t_size);
        for _ in 0..t_size {
            let x: usize = scanner.next();
            let y: usize = scanner.next();
            coordinates.push((x, y));
        }
        oils.push(OilShape {
            max_i: 0,
            max_j: 0,
            coordinate_ids: Vec::new(),
            coordinates,
            mask: BitSet400::new(),
        });
    }

    let total = oils.iter().map(|oil| oil.coordinates.len()).sum::<usize>();
    oils.sort_by(|a, b| a.coordinates.cmp(&b.coordinates));

    for oil in &mut oils {
        oil.max_i = 0;
        oil.max_j = 0;
        for &(i, j) in &oil.coordinates {
            oil.max_i = oil.max_i.max(i);
            oil.max_j = oil.max_j.max(j);
        }
        oil.coordinate_ids = oil.coordinates.iter().map(|&(i, j)| i * n + j).collect();
        oil.mask = BitSet400::new();
        for &ij in &oil.coordinate_ids {
            oil.mask.set(ij, true);
        }
    }

    Input {
        n,
        n2,
        m,
        eps,
        oils,
        total,
    }
}

struct OilState {
    top_left_query_volumes: Vec<Vec<u8>>,
    hashes: Vec<u64>,
}

impl OilState {
    fn new(input: &Input) -> Self {
        Self {
            top_left_query_volumes: vec![Vec::new(); input.n2],
            hashes: vec![0; input.n2],
        }
    }
}

struct State<'a> {
    oil_states: Vec<OilState>,
    top_lefts: Vec<usize>,
    volumes: Vec<u8>,
    query_volumes: Vec<u8>,
    hash: u64,
    input: &'a Input,
}

impl<'a> State<'a> {
    fn new(input: &'a Input, rng: &mut Xorshift) -> Self {
        let mut oil_states = (0..input.m)
            .map(|_| OilState::new(input))
            .collect::<Vec<_>>();

        for oil_id in 0..input.m {
            if oil_id > 0
                && input.oils[oil_id - 1].coordinate_ids == input.oils[oil_id].coordinate_ids
            {
                oil_states[oil_id].hashes = oil_states[oil_id - 1].hashes.clone();
            } else {
                for ij in 0..input.n2 {
                    oil_states[oil_id].hashes[ij] = rng.next();
                }
            }
        }

        let mut hash = 0_u64;
        for oil_state in &oil_states {
            hash ^= oil_state.hashes[0];
        }

        Self {
            oil_states,
            top_lefts: vec![0; input.m],
            volumes: Vec::new(),
            query_volumes: Vec::new(),
            hash,
            input,
        }
    }

    fn move_to(&mut self, oil_id: usize, new_top_left: usize) {
        let old_top_left = self.top_lefts[oil_id];
        let oil_state = &self.oil_states[oil_id];
        self.hash ^= oil_state.hashes[old_top_left] ^ oil_state.hashes[new_top_left];

        for q in 0..self.query_volumes.len() {
            let old_v = oil_state.top_left_query_volumes[old_top_left][q] as i16;
            let new_v = oil_state.top_left_query_volumes[new_top_left][q] as i16;
            let next = self.query_volumes[q] as i16 + new_v - old_v;
            self.query_volumes[q] = next as u8;
        }

        if !self.volumes.is_empty() {
            for &ij in &self.input.oils[oil_id].coordinate_ids {
                self.volumes[ij + old_top_left] = self.volumes[ij + old_top_left].wrapping_sub(1);
                self.volumes[ij + new_top_left] = self.volumes[ij + new_top_left].wrapping_add(1);
            }
        }

        self.top_lefts[oil_id] = new_top_left;
    }

    fn add_query(&mut self, query_coordinates: &[usize]) {
        let mut in_query = vec![false; self.input.n2];
        for &ij in query_coordinates {
            in_query[ij] = true;
        }

        for oil_id in 0..self.input.m {
            let oil = &self.input.oils[oil_id];
            let oil_state = &mut self.oil_states[oil_id];
            for di in 0..(self.input.n - oil.max_i) {
                for dj in 0..(self.input.n - oil.max_j) {
                    let top_left = di * self.input.n + dj;
                    let mut c = 0_u8;
                    for &ij in &oil.coordinate_ids {
                        if in_query[top_left + ij] {
                            c = c.wrapping_add(1);
                        }
                    }
                    oil_state.top_left_query_volumes[top_left].push(c);
                }
            }
        }

        let volume = self.input.get_volume(&self.top_lefts);
        let mut c = 0_u8;
        for &ij in query_coordinates {
            c = c.wrapping_add(volume[ij]);
        }
        self.query_volumes.push(c);
    }
}

struct Sim {
    n: usize,
    n2: usize,
    total: usize,
    eps: f64,
    queries: Vec<(Vec<usize>, usize)>,
    failed: Vec<Vec<usize>>,
    pr_if_x_lb: Vec<Vec<usize>>,
    pr_if_x: Vec<Vec<Vec<(f64, f64)>>>,
    ln_pr_if_s_query: Vec<Vec<f64>>,
    rem: usize,
}

impl Sim {
    fn new(input: &Input) -> Self {
        let n = input.n;
        let n2 = input.n2;
        let total = input.total;
        let eps = input.eps;
        let mut pr_if_x_lb = vec![vec![0_usize; total + 1]; n2 + 1];
        let mut pr_if_x = vec![vec![Vec::new(); total + 1]; n2 + 1];

        for k in 1..=n2 {
            for s in 0..=total {
                let mu = (k as f64 - s as f64) * eps + s as f64 * (1.0 - eps);
                let sigma = (k as f64 * eps * (1.0 - eps)).sqrt();
                let center = mu.round() as i32;

                for r in (0..=center.max(0)).rev() {
                    let prob = likelihood(mu, sigma, r as usize);
                    if prob < SMALL_VALUE {
                        pr_if_x_lb[k][s] = r as usize + 1;
                        break;
                    }
                    pr_if_x[k][s].push((prob, prob.ln()));
                }
                pr_if_x[k][s].reverse();

                let mut r = center + 1;
                loop {
                    let prob = likelihood(mu, sigma, r as usize);
                    if prob < SMALL_VALUE {
                        break;
                    }
                    pr_if_x[k][s].push((prob, prob.ln()));
                    r += 1;
                }
            }
        }

        Self {
            n,
            n2,
            total,
            eps,
            queries: Vec::new(),
            failed: Vec::new(),
            pr_if_x_lb,
            pr_if_x,
            ln_pr_if_s_query: Vec::new(),
            rem: n2 * 2,
        }
    }

    fn ans<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        out: &mut W,
        t: &[usize],
    ) -> bool {
        prof::inc("io.ans");
        if self.rem == 0 {
            eprintln!("!log giveup ");
            std::process::exit(0);
        }
        self.rem -= 1;
        write!(out, "a {}", t.len()).unwrap();
        for &ij in t {
            write!(out, " {} {}", ij / self.n, ij % self.n).unwrap();
        }
        writeln!(out).unwrap();
        out.flush().unwrap();
        let ret: usize = scanner.next();
        if ret == 1 {
            true
        } else {
            self.failed.push(t.to_vec());
            false
        }
    }

    fn query<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        out: &mut W,
        query_coordinates: &[usize],
    ) -> usize {
        prof::inc("io.query");
        if self.rem == 0 {
            eprintln!("!log giveup ");
            std::process::exit(0);
        }
        self.rem -= 1;
        write!(out, "q {}", query_coordinates.len()).unwrap();
        for &ij in query_coordinates {
            write!(out, " {} {}", ij / self.n, ij % self.n).unwrap();
        }
        writeln!(out).unwrap();
        out.flush().unwrap();
        let ret: usize = scanner.next();
        self.queries.push((query_coordinates.to_vec(), ret));

        let mut ln_pr_if_s = vec![0.0_f64; self.total + 1];
        let k = query_coordinates.len();
        for s in 0..=self.total {
            let k_s = k as f64 - s as f64;
            let k_s_eps = k_s * self.eps;
            let meps = 1.0 - self.eps;
            let mu = k_s_eps + s as f64 * meps;
            let sigma = (k as f64 * self.eps * meps).sqrt();
            ln_pr_if_s[s] = likelihood(mu, sigma, ret).ln();
        }

        for i in 0..ln_pr_if_s.len() - 1 {
            if !ln_pr_if_s[i].is_infinite() && ln_pr_if_s[i + 1].is_infinite() {
                ln_pr_if_s[i + 1] = ln_pr_if_s[i] - 10.0;
            }
        }
        for i in (1..ln_pr_if_s.len()).rev() {
            if !ln_pr_if_s[i].is_infinite() && ln_pr_if_s[i - 1].is_infinite() {
                ln_pr_if_s[i - 1] = ln_pr_if_s[i] - 10.0;
            }
        }

        self.ln_pr_if_s_query.push(ln_pr_if_s);
        ret
    }

    fn mine<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        out: &mut W,
        i: usize,
        j: usize,
    ) -> usize {
        prof::inc("io.mine");
        if self.rem == 0 {
            eprintln!("!log giveup");
            std::process::exit(0);
        }
        self.rem -= 1;
        writeln!(out, "q 1 {} {}", i, j).unwrap();
        out.flush().unwrap();
        scanner.next()
    }

    fn giveup<R: BufRead, W: Write>(&mut self, scanner: &mut Scanner<R>, out: &mut W) {
        eprintln!("!log giveup");
        let mut que = VecDeque::new();
        que.push_back((self.n / 2, self.n / 2));
        let mut list = Vec::new();
        let mut remaining = self.total;
        let mut used = vec![vec![false; self.n]; self.n];

        while let Some((i, j)) = que.pop_front() {
            if used[i][j] {
                continue;
            }
            used[i][j] = true;

            let ret = self.mine(scanner, out, i, j);
            if ret > 0 {
                list.push(i * self.n + j);
                remaining -= ret;
                if remaining == 0 {
                    break;
                }
            }

            for &(di, dj) in &DIJ {
                let i2 = i as isize + di;
                let j2 = j as isize + dj;
                if 0 <= i2 && i2 < self.n as isize && 0 <= j2 && j2 < self.n as isize {
                    let next = (i2 as usize, j2 as usize);
                    if ret == 0 {
                        que.push_back(next);
                    } else {
                        que.push_front(next);
                    }
                }
            }
        }
        self.ans(scanner, out, &list);
    }

    fn is_different(&self, volumes: &[u8], failed_coordinates: &[usize]) -> bool {
        for &ij in failed_coordinates {
            if volumes[ij] == 0 {
                return true;
            }
        }
        false
    }

    fn get_query_volume(&self, oil_states: &[OilState], q: usize, top_lefts: &[usize]) -> u8 {
        let mut s = 0_u8;
        for oil_id in 0..top_lefts.len() {
            let oil_state = &oil_states[oil_id];
            let ij = top_lefts[oil_id];
            let p_volume = oil_state.top_left_query_volumes[ij][q];
            if p_volume > 0 {
                s = s.wrapping_add(p_volume);
            }
        }
        s
    }

    fn get_ln_p_r_if_x(&self, oil_states: &[OilState], volumes: &[u8], top_lefts: &[usize]) -> f64 {
        for failed_coordinates in &self.failed {
            let skip = self.is_different(volumes, failed_coordinates);
            if !skip {
                let mut size = 0;
                for &volume in volumes.iter().take(self.n2) {
                    if volume > 0 {
                        size += 1;
                    }
                }
                if size == failed_coordinates.len() {
                    return -1.0e20;
                }
            }
        }

        let mut ln_p_r_if_x = 0.0;
        for q in 0..self.queries.len() {
            let s = self.get_query_volume(oil_states, q, top_lefts) as usize;
            ln_p_r_if_x += self.ln_pr_if_s_query[q][s];
        }
        ln_p_r_if_x
    }

    fn ln_prob_state(&self, state: &State<'_>) -> f64 {
        for failed_coordinates in &self.failed {
            let skip = self.is_different(&state.volumes, failed_coordinates);
            if !skip {
                let mut size = 0;
                for &volume in state.volumes.iter().take(self.n2) {
                    if volume > 0 {
                        size += 1;
                    }
                }
                if size == failed_coordinates.len() {
                    return -1.0e20;
                }
            }
        }

        let mut prob = 0.0;
        for q in 0..self.ln_pr_if_s_query.len() {
            prob += self.ln_pr_if_s_query[q][state.query_volumes[q] as usize];
        }
        prob
    }
}

struct Query<'a> {
    in_query: Vec<bool>,
    volume: Vec<u8>,
    coordinate_size: usize,
    pool: &'a [OilLayout],
}

impl<'a> Query<'a> {
    fn new(input: &Input, pool: &'a [OilLayout]) -> Self {
        Self {
            in_query: vec![false; input.n2],
            volume: vec![0; pool.len()],
            coordinate_size: 0,
            pool,
        }
    }

    fn flip(&mut self, ij: usize) {
        if self.in_query[ij] {
            self.in_query[ij] = false;
            for x in 0..self.pool.len() {
                self.volume[x] = self.volume[x].wrapping_sub(self.pool[x].volume[ij]);
            }
            self.coordinate_size -= 1;
        } else {
            self.in_query[ij] = true;
            for x in 0..self.pool.len() {
                self.volume[x] = self.volume[x].wrapping_add(self.pool[x].volume[ij]);
            }
            self.coordinate_size += 1;
        }
    }

    fn get_coordinates(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for ij in 0..self.in_query.len() {
            if self.in_query[ij] {
                result.push(ij);
            }
        }
        result
    }

    fn eval(&self, sim: &Sim, add_k: usize, add_v: u8) -> f64 {
        let k = self.coordinate_size + add_k;
        let mut ln_pr = Vec::<f64>::new();

        for x in 0..self.pool.len() {
            let v = self.volume[x] as usize + add_v as usize;
            let lb = sim.pr_if_x_lb[k][v];
            let need = lb + sim.pr_if_x[k][v].len();
            if ln_pr.len() < need {
                ln_pr.resize(need, 0.0);
            }
            let px = self.pool[x].px_if_r;
            for (pi, &(pr_if_x, _ln_pr_if_x)) in sim.pr_if_x[k][v].iter().enumerate() {
                ln_pr[lb + pi] += pr_if_x * px;
            }
        }

        for value in &mut ln_pr {
            *value = value.ln();
        }

        let mut info = 0.0;
        for x in 0..self.pool.len() {
            let px = self.pool[x].px_if_r;
            let v = self.volume[x] as usize + add_v as usize;
            let lb = sim.pr_if_x_lb[k][v];
            for (pi, &(pr_if_x, ln_pr_if_x)) in sim.pr_if_x[k][v].iter().enumerate() {
                let ln_prr = ln_pr[lb + pi];
                info += pr_if_x * px * (ln_pr_if_x - ln_prr);
            }
        }

        info * (k as f64).sqrt()
    }
}

fn likelihood(mean: f64, std_dev: f64, res: usize) -> f64 {
    let b = res as f64 + 0.5;
    if res == 0 {
        normal_cdf(mean, std_dev, b)
    } else {
        let a = res as f64 - 0.5;
        normal_cdf(mean, std_dev, b) - normal_cdf(mean, std_dev, a)
    }
}

fn normal_cdf(mean: f64, std_dev: f64, x: f64) -> f64 {
    0.5 * (1.0 + libm::erf((x - mean) / (std_dev * 2.0_f64.sqrt())))
}

fn normalize_pool(pool: &mut [OilLayout]) {
    let total = pool.iter().map(|layout| layout.px_if_r).sum::<f64>();
    for layout in pool {
        layout.px_if_r /= total;
    }
}

fn set_volume(pool: &mut [OilLayout], input: &Input) {
    for layout in pool {
        layout.volume = input.get_volume(&layout.top_lefts);
    }
}

fn concat_pool(pool: &mut Vec<OilLayout>, iter: usize, started_at: Instant) {
    let tmp1 = iter as f64 * 0.01;
    let tmp2 = (3.0 - started_at.elapsed().as_secs_f64()).min(1.0);
    let mut size = pool.len().min(((tmp1 * tmp2) as usize).max(2));
    while size > 2 && pool[0].px_if_r * 1.0e-4 > pool[size - 1].px_if_r {
        size -= 1;
    }
    if size > 0 {
        pool.truncate(size);
    }
}

fn get_divination_query(
    input: &Input,
    pool: &[OilLayout],
    sim: &Sim,
    rng: &mut Xorshift,
) -> Vec<usize> {
    let size = pool.len();
    prof::add_value("query.pool_size", size as u64);
    let timer = prof::start();
    let mut same = vec![true; input.n2];
    for layout in pool.iter().take(size).skip(1) {
        for (ij, same_ij) in same.iter_mut().enumerate().take(input.n2) {
            *same_ij = *same_ij && layout.volume[ij] == pool[0].volume[ij];
        }
    }
    prof::finish("query.compute_same", timer);

    let mut query = Query::new(input, pool);
    let mut query_coordinate_evals = Vec::<(f64, usize)>::new();
    let timer = prof::start();
    for (ij, &is_same) in same.iter().enumerate().take(input.n2) {
        if !is_same {
            query.flip(ij);
            let ev = query.eval(sim, 0, 0);
            query.flip(ij);
            query_coordinate_evals.push((ev, ij));
        }
    }
    prof::finish("query.single_cell_evals", timer);
    prof::add_value(
        "query.informative_cells",
        query_coordinate_evals.len() as u64,
    );
    let timer = prof::start();
    query_coordinate_evals.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    prof::finish("query.sort_single_cell_evals", timer);

    let timer = prof::start();
    let mut no_info_coordinates = Vec::new();
    for (ij, &is_same) in same.iter().enumerate().take(input.n2) {
        if is_same {
            no_info_coordinates.push(ij);
        }
    }

    let mut evaluation_values = Vec::<(usize, usize)>::new();
    for &ij in &no_info_coordinates {
        let evaluation_value = pool[0].volume[ij] as usize * 1000 + rng.randrange(1000);
        evaluation_values.push((ij, evaluation_value));
    }
    evaluation_values.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    no_info_coordinates = evaluation_values
        .into_iter()
        .map(|(ij, _)| ij)
        .collect::<Vec<_>>();
    prof::finish("query.no_info_sort", timer);
    prof::add_value("query.no_info_cells", no_info_coordinates.len() as u64);

    let mut crt = -1.0e100;
    let mut add_k = 0_usize;
    let mut add_v = 0_usize;
    let best_layout = &pool[0];
    let timer = prof::start();
    let mut pass_count = 0_u64;
    let mut flip_evals = 0_u64;
    let mut flip_accepts = 0_u64;
    let mut no_info_evals = 0_u64;
    let mut no_info_accepts = 0_u64;
    for _ in 0..3 {
        pass_count += 1;
        let mut change = false;
        for &(_, ij) in &query_coordinate_evals {
            query.flip(ij);
            let eval = query.eval(sim, add_k, add_v as u8);
            flip_evals += 1;

            if crt < eval {
                crt = eval;
                change = true;
                flip_accepts += 1;
            } else {
                query.flip(ij);
            }
        }

        while add_k < no_info_coordinates.len() {
            let next_cell = no_info_coordinates[add_k];
            let next_add_v = add_v + best_layout.volume[next_cell] as usize;
            let eval = query.eval(sim, add_k + 1, next_add_v as u8);
            no_info_evals += 1;
            if crt < eval {
                crt = eval;
                add_v = next_add_v;
                add_k += 1;
                change = true;
                no_info_accepts += 1;
            } else {
                break;
            }
        }
        while add_k > 0 {
            let prev_cell = no_info_coordinates[add_k - 1];
            let next_add_v = add_v - best_layout.volume[prev_cell] as usize;
            let eval = query.eval(sim, add_k - 1, next_add_v as u8);
            no_info_evals += 1;
            if crt < eval {
                crt = eval;
                add_v = next_add_v;
                add_k -= 1;
                change = true;
                no_info_accepts += 1;
            } else {
                break;
            }
        }

        if !change {
            break;
        }
    }
    prof::finish("query.greedy_loop", timer);
    prof::add_value("query.pass_count", pass_count);
    prof::add_value("query.flip_evals", flip_evals);
    prof::add_value("query.flip_accepts", flip_accepts);
    prof::add_value("query.no_info_evals", no_info_evals);
    prof::add_value("query.no_info_accepts", no_info_accepts);

    let mut query_coordinates = query.get_coordinates();
    query_coordinates.extend_from_slice(&no_info_coordinates[..add_k]);
    prof::add_value("query.final_query_size", query_coordinates.len() as u64);
    query_coordinates
}

fn sort_pool(pool: &mut [OilLayout]) {
    pool.sort_by(|a, b| b.ln_p_r_if_x.total_cmp(&a.ln_p_r_if_x));
}

fn shuffle_pool(pool: &mut [OilLayout], rng: &mut Xorshift) {
    for i in (1..pool.len()).rev() {
        let j = rng.randrange(i + 1);
        pool.swap(i, j);
    }
}

fn checked_offset(
    i: usize,
    j: usize,
    di: isize,
    dj: isize,
    input: &Input,
    oil: &OilShape,
) -> Option<usize> {
    let i2 = i as isize + di;
    let j2 = j as isize + dj;
    if 0 <= i2
        && i2 < (input.n - oil.max_i) as isize
        && 0 <= j2
        && j2 < (input.n - oil.max_j) as isize
    {
        Some(i2 as usize * input.n + j2 as usize)
    } else {
        None
    }
}

fn simulated_annealing(
    input: &Input,
    sim: &Sim,
    state: &mut State<'_>,
    pool: &mut Vec<OilLayout>,
    swaps: &[Vec<Vec<(isize, isize)>>],
    hash_ln_likelihood: &mut HashMap<u64, f64>,
    iter: usize,
    rng: &mut Xorshift,
    started_at: Instant,
) {
    let mut crt = pool[0].ln_p_r_if_x;
    for oil_id in 0..input.m {
        state.move_to(oil_id, pool[0].top_lefts[oil_id]);
    }
    let mut max_crt = crt;
    let t0 = 2.0;
    let t1 = 1.0;
    let iter = (iter as f64 * (3.0 - started_at.elapsed().as_secs_f64()).min(1.0)) as usize;
    prof::add_value("sa.iter_budget", iter as u64);
    prof::add_value("sa.pool_start", pool.len() as u64);
    let mut proposal_count = 0_u64;
    let mut accepted_count = 0_u64;
    let mut pushed_count = 0_u64;
    let mut invalid_count = 0_u64;
    let mut cached_count = 0_u64;
    let mut ln_prob_ns = 0_u128;
    let mut ln_prob_count = 0_u64;

    for t in 0..iter {
        proposal_count += 1;
        let temp = t0 + (t1 - t0) * t as f64 / iter as f64;
        let slide_threshold = 30;
        let warp_threshold = slide_threshold + 10;
        let swap_threshold = warp_threshold + 60;
        let coin = rng.randrange(swap_threshold);

        if coin < slide_threshold {
            let oil_id = rng.randrange(input.m);
            let oil = &input.oils[oil_id];
            let (di, dj) = DIJ[rng.randrange(4)];
            let i = state.top_lefts[oil_id] / input.n;
            let j = state.top_lefts[oil_id] % input.n;
            if let Some(next_top_left) = checked_offset(i, j, di, dj, input, oil) {
                let bk = state.top_lefts[oil_id];
                state.move_to(oil_id, next_top_left);
                let existed = hash_ln_likelihood.contains_key(&state.hash);
                let next = if existed {
                    cached_count += 1;
                    hash_ln_likelihood[&state.hash]
                } else {
                    let timer = prof::start();
                    let value = sim.ln_prob_state(state);
                    ln_prob_ns += timer.elapsed().as_nanos();
                    ln_prob_count += 1;
                    value
                };
                if !existed {
                    hash_ln_likelihood.insert(state.hash, next);
                    if next - max_crt >= -10.0 {
                        pushed_count += 1;
                        pool.push(OilLayout {
                            hash: state.hash,
                            ln_p_r_if_x: next,
                            px_if_r: 0.0,
                            top_lefts: state.top_lefts.clone(),
                            volume: state.volumes.clone(),
                        });
                    }
                }
                if crt <= next || rng.gen_bool(((next - crt) / temp).exp()) {
                    accepted_count += 1;
                    crt = next;
                } else {
                    state.move_to(oil_id, bk);
                }
            } else {
                invalid_count += 1;
            }
        } else if coin < warp_threshold {
            let oil_id = rng.randrange(input.m);
            let oil = &input.oils[oil_id];
            let i2 = rng.randrange(input.n - oil.max_i);
            let j2 = rng.randrange(input.n - oil.max_j);
            let bk = state.top_lefts[oil_id];
            state.move_to(oil_id, i2 * input.n + j2);
            let existed = hash_ln_likelihood.contains_key(&state.hash);
            let next = if existed {
                cached_count += 1;
                hash_ln_likelihood[&state.hash]
            } else {
                let timer = prof::start();
                let value = sim.ln_prob_state(state);
                ln_prob_ns += timer.elapsed().as_nanos();
                ln_prob_count += 1;
                value
            };
            if !existed {
                hash_ln_likelihood.insert(state.hash, next);
                if next - max_crt >= -10.0 {
                    pushed_count += 1;
                    pool.push(OilLayout {
                        hash: state.hash,
                        ln_p_r_if_x: next,
                        px_if_r: 0.0,
                        top_lefts: state.top_lefts.clone(),
                        volume: state.volumes.clone(),
                    });
                }
            }
            if crt <= next || rng.gen_bool(((next - crt) / temp).exp()) {
                accepted_count += 1;
                crt = next;
            } else {
                state.move_to(oil_id, bk);
            }
        } else {
            let oil_id_a = rng.randrange(input.m);
            let oil_id_b = rng.randrange(input.m);
            if oil_id_a == oil_id_b {
                continue;
            }

            let oil_a = &input.oils[oil_id_a];
            let oil_b = &input.oils[oil_id_b];
            let ai = state.top_lefts[oil_id_a] / input.n;
            let aj = state.top_lefts[oil_id_a] % input.n;
            let bi = state.top_lefts[oil_id_b] / input.n;
            let bj = state.top_lefts[oil_id_b] % input.n;
            let (dai, daj) =
                swaps[oil_id_b][oil_id_a][rng.randrange(swaps[oil_id_a][oil_id_b].len())];
            let (dbi, dbj) =
                swaps[oil_id_a][oil_id_b][rng.randrange(swaps[oil_id_b][oil_id_a].len())];

            let Some(next_a) = checked_offset(bi, bj, dai, daj, input, oil_a) else {
                invalid_count += 1;
                continue;
            };
            let Some(next_b) = checked_offset(ai, aj, dbi, dbj, input, oil_b) else {
                invalid_count += 1;
                continue;
            };

            state.move_to(oil_id_a, next_a);
            state.move_to(oil_id_b, next_b);
            let existed = hash_ln_likelihood.contains_key(&state.hash);
            let next = if existed {
                cached_count += 1;
                hash_ln_likelihood[&state.hash]
            } else {
                let timer = prof::start();
                let value = sim.ln_prob_state(state);
                ln_prob_ns += timer.elapsed().as_nanos();
                ln_prob_count += 1;
                value
            };
            if !existed {
                hash_ln_likelihood.insert(state.hash, next);
                if next - max_crt >= -10.0 {
                    pushed_count += 1;
                    pool.push(OilLayout {
                        hash: state.hash,
                        ln_p_r_if_x: next,
                        px_if_r: 0.0,
                        top_lefts: state.top_lefts.clone(),
                        volume: state.volumes.clone(),
                    });
                }
            }
            if crt <= next || rng.gen_bool(((next - crt) / temp).exp()) {
                accepted_count += 1;
                crt = next;
            } else {
                state.move_to(oil_id_a, ai * input.n + aj);
                state.move_to(oil_id_b, bi * input.n + bj);
            }
        }

        if max_crt < crt {
            max_crt = crt;
        }
    }

    sort_pool(pool);
    prof::add_value("sa.proposals", proposal_count);
    prof::add_value("sa.accepted", accepted_count);
    prof::add_value("sa.pushed", pushed_count);
    prof::add_value("sa.invalid", invalid_count);
    prof::add_value("sa.cached", cached_count);
    prof::add_time_ns("sa.ln_prob_state", ln_prob_ns, ln_prob_count);
    prof::add_value("sa.pool_end", pool.len() as u64);
}

fn get_swaps(input: &Input) -> Vec<Vec<Vec<(isize, isize)>>> {
    let mut swaps = vec![vec![Vec::<(isize, isize)>::new(); input.m]; input.m];
    for oil_id_a in 0..input.m {
        let oil_a = &input.oils[oil_id_a];
        let mut is_a_coordinate = vec![false; input.n2];
        for &ij in &oil_a.coordinate_ids {
            is_a_coordinate[ij] = true;
        }
        for oil_id_b in 0..input.m {
            if oil_id_a == oil_id_b {
                continue;
            }
            let oil_b = &input.oils[oil_id_b];
            let mut list = Vec::<(usize, (isize, isize))>::new();
            for di in -(oil_b.max_i as isize)..=(oil_a.max_i as isize) {
                for dj in -(oil_b.max_j as isize)..=(oil_a.max_j as isize) {
                    let mut volume = 0_usize;
                    for &ij in &oil_b.coordinate_ids {
                        let i = ij / input.n;
                        let j = ij % input.n;
                        let i2 = i as isize + di;
                        let j2 = j as isize + dj;
                        if 0 <= i2
                            && 0 <= j2
                            && i2 < input.n as isize
                            && j2 < input.n as isize
                            && is_a_coordinate[i2 as usize * input.n + j2 as usize]
                        {
                            volume += 1;
                        }
                    }
                    list.push((volume, (di, dj)));
                }
            }
            list.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| (b.1 .0 as u64).cmp(&(a.1 .0 as u64)))
                    .then_with(|| (b.1 .1 as u64).cmp(&(a.1 .1 as u64)))
            });
            while list.len() < 4 {
                list.push((0, (0, 0)));
            }
            list.truncate(4);
            swaps[oil_id_a][oil_id_b] = list.into_iter().map(|(_, offset)| offset).collect();
        }
    }
    swaps
}

fn main() {
    let _profile_guard = prof::PrintOnDrop;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut out = BufWriter::new(stdout.lock());
    let started_at = Instant::now();
    let mut rng = Xorshift::new(1);

    let input = read_input(&mut scanner);
    let swaps = get_swaps(&input);
    let mut sim = Sim::new(&input);
    let mut state = State::new(&input, &mut rng);
    let mut pool = Vec::<OilLayout>::new();
    let iter = 4_000_000_usize / (2 * input.n2);

    for t in 0_usize.. {
        prof::inc("main.turns");
        prof::add_value("main.pool_at_turn_start", pool.len() as u64);
        if sim.rem == 0 {
            eprintln!("!There is no more query");
            break;
        }
        if started_at.elapsed().as_secs_f64() > 2.9 {
            prof::inc("main.enter_giveup_time");
            prof::add_value("main.turns_before_giveup", t as u64);
            let timer = prof::start();
            sim.giveup(&mut scanner, &mut out);
            prof::finish("main.giveup", timer);
            break;
        }

        let timer = prof::start();
        for layout in &mut pool {
            if layout.volume.is_empty() && !sim.failed.is_empty() {
                layout.volume = input.get_volume(&layout.top_lefts);
            }
            layout.ln_p_r_if_x =
                sim.get_ln_p_r_if_x(&state.oil_states, &layout.volume, &layout.top_lefts);
        }
        prof::finish("main.update_layout_likelihoods", timer);
        let timer = prof::start();
        shuffle_pool(&mut pool, &mut rng);
        sort_pool(&mut pool);
        prof::finish("main.shuffle_sort_pool", timer);

        let timer = prof::start();
        let mut hash_ln_likelihood = HashMap::<u64, f64>::new();
        for layout in &pool {
            hash_ln_likelihood.insert(layout.hash, layout.ln_p_r_if_x);
        }
        prof::finish("main.build_hash_likelihood", timer);

        if t == 0 {
            let timer = prof::start();
            for _ in 0..iter {
                for oil_id in 0..input.m {
                    let oil = &input.oils[oil_id];
                    let i = rng.randrange(input.n - oil.max_i);
                    let j = rng.randrange(input.n - oil.max_j);
                    state.move_to(oil_id, i * input.n + j);
                }
                if !hash_ln_likelihood.contains_key(&state.hash) {
                    hash_ln_likelihood.insert(state.hash, 0.0);
                    pool.push(OilLayout {
                        hash: state.hash,
                        ln_p_r_if_x: 0.0,
                        px_if_r: 0.0,
                        top_lefts: state.top_lefts.clone(),
                        volume: state.volumes.clone(),
                    });
                }
            }
            prof::finish("main.initial_random_pool", timer);
        } else {
            let timer = prof::start();
            simulated_annealing(
                &input,
                &sim,
                &mut state,
                &mut pool,
                &swaps,
                &mut hash_ln_likelihood,
                iter,
                &mut rng,
                started_at,
            );
            prof::finish("main.simulated_annealing", timer);
        }

        prof::add_value("main.pool_after_generate", pool.len() as u64);
        let timer = prof::start();
        let max_prob = pool[0].ln_p_r_if_x;
        for layout in &mut pool {
            layout.px_if_r = (layout.ln_p_r_if_x - max_prob).exp();
        }
        normalize_pool(&mut pool);

        while pool.len() > 1 && pool[pool.len() - 1].px_if_r < 1.0e-9 {
            pool.pop();
        }
        prof::finish("main.normalize_drop_tiny", timer);

        let best_layout = &pool[0];
        let best_bits = input.get_positives(&best_layout.top_lefts);
        let best_pool_prob = best_layout.px_if_r;
        prof::add_value(
            "main.best_pool_prob_x1000000",
            (best_pool_prob * 1.0e6) as u64,
        );

        let timer = prof::start();
        concat_pool(&mut pool, iter, started_at);
        normalize_pool(&mut pool);
        set_volume(&mut pool, &input);
        prof::finish("main.concat_normalize_set_volume", timer);
        prof::add_value("main.pool_after_concat", pool.len() as u64);

        if best_pool_prob > 0.8 {
            prof::inc("main.best_prob_answer_branch");
            let timer = prof::start();
            let mut t_vec = Vec::new();
            let mut t_vec_reverse = Vec::new();
            for ij in 0..input.n2 {
                if best_bits.get(ij) {
                    t_vec.push(ij);
                } else {
                    t_vec_reverse.push(ij);
                }
            }
            prof::finish("main.answer_prepare_vectors", timer);

            let last_query_is_reverse = sim
                .queries
                .last()
                .map(|(coords, _)| coords == &t_vec_reverse)
                .unwrap_or(false);
            if input.m <= 4 || last_query_is_reverse {
                prof::inc("main.try_answer");
                let timer = prof::start();
                if sim.ans(&mut scanner, &mut out, &t_vec) {
                    prof::finish("main.ask_answer", timer);
                    break;
                } else if sim.failed.len() == 1 {
                    prof::finish("main.ask_answer", timer);
                    state.volumes = input.get_volume(&state.top_lefts);
                }
            } else {
                prof::inc("main.reverse_query");
                prof::add_value("main.query_size", t_vec_reverse.len() as u64);
                let timer = prof::start();
                sim.query(&mut scanner, &mut out, &t_vec_reverse);
                prof::finish("main.sim_query", timer);
                let timer = prof::start();
                state.add_query(&t_vec_reverse);
                prof::finish("main.state_add_query", timer);
            }
        } else {
            let timer = prof::start();
            let query_coordinates = get_divination_query(&input, &pool, &sim, &mut rng);
            prof::finish("main.get_divination_query", timer);
            prof::add_value("main.query_size", query_coordinates.len() as u64);
            let timer = prof::start();
            sim.query(&mut scanner, &mut out, &query_coordinates);
            prof::finish("main.sim_query", timer);
            let timer = prof::start();
            state.add_query(&query_coordinates);
            prof::finish("main.state_add_query", timer);
        }
    }

    eprintln!("!Time = {}", started_at.elapsed().as_secs_f64());
    eprintln!("!log miss {}", sim.failed.len());
    eprintln!("!main end");
}
