// v901_reference_02_random_pool.rs
use std::collections::{HashMap, VecDeque};
use std::f64::consts::SQRT_2;
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;
use std::time::Instant;

const SMALL_VALUE: f64 = 1.0e-6;
const THETA_VISIT_BUDGET: usize = 4_000_000;
const TIME_LIMIT_SEC: f64 = 2.9;

#[derive(Clone, Debug)]
struct XorShift {
    x: u64,
}

impl XorShift {
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
                return token.parse().ok().expect("failed to parse input token");
            }

            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .expect("failed to read input line");
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

#[derive(Clone, Debug)]
struct OilShape {
    max_i: usize,
    max_j: usize,
    coordinate_ids: Vec<usize>,
    coordinates: Vec<(usize, usize)>,
}

#[derive(Clone, Debug)]
struct Input {
    n: usize,
    n2: usize,
    m: usize,
    eps: f64,
    oils: Vec<OilShape>,
    total: usize,
}

impl Input {
    fn positive_coordinates(&self, top_lefts: &[usize]) -> Vec<usize> {
        self.get_volume(top_lefts)
            .into_iter()
            .enumerate()
            .filter_map(|(ij, volume)| if volume > 0 { Some(ij) } else { None })
            .collect()
    }

    fn get_volume(&self, top_lefts: &[usize]) -> Vec<u8> {
        let mut volume = vec![0_u8; self.n2];
        for (oil_id, &top_left) in top_lefts.iter().enumerate() {
            for &ij in &self.oils[oil_id].coordinate_ids {
                volume[top_left + ij] += 1;
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
            let i: usize = scanner.next();
            let j: usize = scanner.next();
            coordinates.push((i, j));
        }
        oils.push(OilShape {
            max_i: 0,
            max_j: 0,
            coordinate_ids: Vec::new(),
            coordinates,
        });
    }

    let total = oils.iter().map(|oil| oil.coordinates.len()).sum();
    oils.sort_by(|a, b| a.coordinates.cmp(&b.coordinates));

    for oil in &mut oils {
        oil.max_i = oil.coordinates.iter().map(|&(i, _)| i).max().unwrap();
        oil.max_j = oil.coordinates.iter().map(|&(_, j)| j).max().unwrap();
        oil.coordinate_ids = oil.coordinates.iter().map(|&(i, j)| i * n + j).collect();
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

#[derive(Clone, Debug)]
struct OilLayout {
    hash: u64,
    ln_p_r_if_x: f64,
    px_if_r: f64,
    top_lefts: Vec<usize>,
    volume: Vec<u8>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct State {
    oil_states: Vec<OilState>,
    top_lefts: Vec<usize>,
    volumes: Vec<u8>,
    query_volumes: Vec<u8>,
    hash: u64,
}

impl State {
    fn new(input: &Input, rng: &mut XorShift) -> Self {
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

        let top_lefts = vec![0; input.m];
        let mut hash = 0_u64;
        for oil_state in &oil_states {
            hash ^= oil_state.hashes[0];
        }
        Self {
            oil_states,
            top_lefts,
            volumes: Vec::new(),
            query_volumes: Vec::new(),
            hash,
        }
    }

    fn move_to(&mut self, input: &Input, oil_id: usize, new_top_left: usize) {
        let old_top_left = self.top_lefts[oil_id];
        if old_top_left == new_top_left {
            return;
        }

        let oil_state = &self.oil_states[oil_id];
        self.hash ^= oil_state.hashes[old_top_left] ^ oil_state.hashes[new_top_left];

        for q in 0..self.query_volumes.len() {
            let old_v = oil_state.top_left_query_volumes[old_top_left][q] as i16;
            let new_v = oil_state.top_left_query_volumes[new_top_left][q] as i16;
            let next = self.query_volumes[q] as i16 + new_v - old_v;
            self.query_volumes[q] = next as u8;
        }

        if !self.volumes.is_empty() {
            for &ij in &input.oils[oil_id].coordinate_ids {
                self.volumes[old_top_left + ij] -= 1;
                self.volumes[new_top_left + ij] += 1;
            }
        }
        self.top_lefts[oil_id] = new_top_left;
    }

    fn add_query(&mut self, input: &Input, query_coordinates: &[usize]) {
        let mut in_query = vec![false; input.n2];
        for &ij in query_coordinates {
            in_query[ij] = true;
        }

        for oil_id in 0..input.m {
            let oil = &input.oils[oil_id];
            let oil_state = &mut self.oil_states[oil_id];
            for di in 0..input.n - oil.max_i {
                for dj in 0..input.n - oil.max_j {
                    let top_left = di * input.n + dj;
                    let mut count = 0_u8;
                    for &ij in &oil.coordinate_ids {
                        if in_query[top_left + ij] {
                            count += 1;
                        }
                    }
                    oil_state.top_left_query_volumes[top_left].push(count);
                }
            }
        }

        let volume = input.get_volume(&self.top_lefts);
        let mut count = 0_u8;
        for &ij in query_coordinates {
            count += volume[ij];
        }
        self.query_volumes.push(count);
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
        let mut sim = Self {
            n: input.n,
            n2: input.n2,
            total: input.total,
            eps: input.eps,
            queries: Vec::new(),
            failed: Vec::new(),
            pr_if_x_lb: vec![vec![0; input.total + 1]; input.n2 + 1],
            pr_if_x: vec![vec![Vec::new(); input.total + 1]; input.n2 + 1],
            ln_pr_if_s_query: Vec::new(),
            rem: input.n2 * 2,
        };
        sim.precompute_likelihoods();
        sim
    }

    fn precompute_likelihoods(&mut self) {
        for k in 1..=self.n2 {
            for sum_v in 0..=self.total {
                let mu = (k as f64 - sum_v as f64) * self.eps + sum_v as f64 * (1.0 - self.eps);
                let sigma = (k as f64 * self.eps * (1.0 - self.eps)).sqrt();
                let center = mu.round() as i32;

                for r in (0..=center.max(0)).rev() {
                    let probability = self.likelihood(mu, sigma, r as usize);
                    if probability < SMALL_VALUE {
                        self.pr_if_x_lb[k][sum_v] = r as usize + 1;
                        break;
                    }
                    self.pr_if_x[k][sum_v].push((probability, probability.ln()));
                }
                self.pr_if_x[k][sum_v].reverse();

                let mut r = center.max(0) as usize + 1;
                loop {
                    let probability = self.likelihood(mu, sigma, r);
                    if probability < SMALL_VALUE {
                        break;
                    }
                    self.pr_if_x[k][sum_v].push((probability, probability.ln()));
                    r += 1;
                }
            }
        }
    }

    fn ans<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        out: &mut W,
        coordinates: &[usize],
    ) -> bool {
        if self.rem == 0 {
            std::process::exit(0);
        }
        self.rem -= 1;

        write!(out, "a {}", coordinates.len()).unwrap();
        for &ij in coordinates {
            write!(out, " {} {}", ij / self.n, ij % self.n).unwrap();
        }
        writeln!(out).unwrap();
        out.flush().unwrap();

        let ret: usize = scanner.next();
        if ret == 1 {
            true
        } else {
            self.failed.push(coordinates.to_vec());
            false
        }
    }

    fn query<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        out: &mut W,
        query_coordinates: &[usize],
    ) -> usize {
        if self.rem == 0 {
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

        let mut ln_pr_if_s = vec![f64::NEG_INFINITY; self.total + 1];
        let k = query_coordinates.len();
        for sum_v in 0..=self.total {
            let mu = (k as f64 - sum_v as f64) * self.eps + sum_v as f64 * (1.0 - self.eps);
            let sigma = (k as f64 * self.eps * (1.0 - self.eps)).sqrt();
            ln_pr_if_s[sum_v] = self.likelihood(mu, sigma, ret).ln();
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
        if self.rem == 0 {
            std::process::exit(0);
        }
        self.rem -= 1;
        writeln!(out, "q 1 {} {}", i, j).unwrap();
        out.flush().unwrap();
        scanner.next()
    }

    fn likelihood(&self, mean: f64, std_dev: f64, res: usize) -> f64 {
        let b = res as f64 + 0.5;
        if res == 0 {
            normal_cdf(mean, std_dev, b)
        } else {
            let a = res as f64 - 0.5;
            normal_cdf(mean, std_dev, b) - normal_cdf(mean, std_dev, a)
        }
    }

    fn giveup<R: BufRead, W: Write>(&mut self, scanner: &mut Scanner<R>, out: &mut W) {
        let mut que = VecDeque::new();
        que.push_back((self.n / 2, self.n / 2));
        let mut list = Vec::new();
        let mut rem = self.total;
        let mut used = vec![vec![false; self.n]; self.n];
        let dij = [(0_isize, 1_isize), (1, 0), (0, -1), (-1, 0)];

        while let Some((i, j)) = que.pop_front() {
            if used[i][j] {
                continue;
            }
            used[i][j] = true;

            let ret = self.mine(scanner, out, i, j);
            if ret > 0 {
                list.push(i * self.n + j);
                rem = rem.saturating_sub(ret);
                if rem == 0 {
                    break;
                }
            }

            for &(di, dj) in &dij {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if 0 <= ni && ni < self.n as isize && 0 <= nj && nj < self.n as isize {
                    let next = (ni as usize, nj as usize);
                    if ret == 0 {
                        que.push_back(next);
                    } else {
                        que.push_front(next);
                    }
                }
            }
        }

        let _ = self.ans(scanner, out, &list);
    }

    fn is_different(&self, volumes: &[u8], failed_coordinates: &[usize]) -> bool {
        for &ij in failed_coordinates {
            if volumes[ij] == 0 {
                return true;
            }
        }
        false
    }

    fn get_query_volume(&self, oil_states: &[OilState], q: usize, top_lefts: &[usize]) -> usize {
        let mut sum_v = 0_usize;
        for (oil_id, &top_left) in top_lefts.iter().enumerate() {
            let volume = oil_states[oil_id].top_left_query_volumes[top_left][q] as usize;
            if volume > 0 {
                sum_v += volume;
            }
        }
        sum_v
    }

    fn get_ln_p_r_if_x(&self, oil_states: &[OilState], volumes: &[u8], top_lefts: &[usize]) -> f64 {
        for failed_coordinates in &self.failed {
            let skip = self.is_different(volumes, failed_coordinates);
            if !skip {
                let size = volumes.iter().filter(|&&volume| volume > 0).count();
                if size == failed_coordinates.len() {
                    return -1.0e20;
                }
            }
        }

        let mut ln_p_r_if_x = 0.0;
        for q in 0..self.queries.len() {
            let sum_v = self.get_query_volume(oil_states, q, top_lefts);
            ln_p_r_if_x += self.ln_pr_if_s_query[q][sum_v];
        }
        ln_p_r_if_x
    }
}

fn normal_cdf(mean: f64, std_dev: f64, x: f64) -> f64 {
    0.5 * (1.0 + libm::erf((x - mean) / (std_dev * SQRT_2)))
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
                self.volume[x] -= self.pool[x].volume[ij];
            }
            self.coordinate_size -= 1;
        } else {
            self.in_query[ij] = true;
            for x in 0..self.pool.len() {
                self.volume[x] += self.pool[x].volume[ij];
            }
            self.coordinate_size += 1;
        }
    }

    fn get_coordinates(&self) -> Vec<usize> {
        self.in_query
            .iter()
            .enumerate()
            .filter_map(|(ij, &in_query)| if in_query { Some(ij) } else { None })
            .collect()
    }

    fn eval(&self, sim: &Sim, add_k: usize, add_v: usize) -> f64 {
        let k = self.coordinate_size + add_k;

        let mut pr = Vec::<f64>::new();
        for x in 0..self.pool.len() {
            let sum_v = self.volume[x] as usize + add_v;
            let lb = sim.pr_if_x_lb[k][sum_v];
            let row = &sim.pr_if_x[k][sum_v];
            if pr.len() < lb + row.len() {
                pr.resize(lb + row.len(), 0.0);
            }
            let px = self.pool[x].px_if_r;
            for (pi, &(pr_if_x, _)) in row.iter().enumerate() {
                pr[lb + pi] += pr_if_x * px;
            }
        }

        let ln_pr = pr
            .into_iter()
            .map(|probability| probability.ln())
            .collect::<Vec<_>>();

        let mut info = 0.0;
        for x in 0..self.pool.len() {
            let px = self.pool[x].px_if_r;
            let sum_v = self.volume[x] as usize + add_v;
            let lb = sim.pr_if_x_lb[k][sum_v];
            for (pi, &(pr_if_x, ln_pr_if_x)) in sim.pr_if_x[k][sum_v].iter().enumerate() {
                info += pr_if_x * px * (ln_pr_if_x - ln_pr[lb + pi]);
            }
        }

        info * (k as f64).sqrt()
    }
}

fn normalize_pool(pool: &mut [OilLayout]) {
    let total = pool.iter().map(|layout| layout.px_if_r).sum::<f64>();
    if total <= 0.0 {
        return;
    }
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
    let tmp2 = (3.0 - started_at.elapsed().as_secs_f64()).min(1.0).max(0.0);
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
    pool: &mut [OilLayout],
    sim: &Sim,
    rng: &mut XorShift,
) -> Vec<usize> {
    let size = pool.len();
    let mut same = vec![true; input.n2];
    for x in 1..size {
        for (ij, same_value) in same.iter_mut().enumerate() {
            *same_value = *same_value && pool[x].volume[ij] == pool[0].volume[ij];
        }
    }

    let mut query = Query::new(input, pool);
    let mut query_coordinate_evals = Vec::new();
    for (ij, &is_same) in same.iter().enumerate() {
        if !is_same {
            query.flip(ij);
            let eval = query.eval(sim, 0, 0);
            query.flip(ij);
            query_coordinate_evals.push((eval, ij));
        }
    }
    query_coordinate_evals.sort_unstable_by(|&(eval_a, ij_a), &(eval_b, ij_b)| {
        eval_b.total_cmp(&eval_a).then_with(|| ij_b.cmp(&ij_a))
    });

    let mut no_info_coordinates = Vec::new();
    for (ij, &is_same) in same.iter().enumerate() {
        if is_same {
            no_info_coordinates.push(ij);
        }
    }

    let mut evaluation_values = Vec::new();
    for &ij in &no_info_coordinates {
        let evaluation_value = pool[0].volume[ij] as usize * 1000 + rng.randrange(1000);
        evaluation_values.push((ij, evaluation_value));
    }
    evaluation_values.sort_unstable_by(|&(ij_a, value_a), &(ij_b, value_b)| {
        value_b.cmp(&value_a).then_with(|| ij_b.cmp(&ij_a))
    });
    no_info_coordinates = evaluation_values
        .into_iter()
        .map(|(ij, _)| ij)
        .collect::<Vec<_>>();

    let mut current = -1.0e100;
    let mut add_k = 0_usize;
    let mut add_v = 0_usize;
    let best_volume = pool[0].volume.clone();

    for _ in 0..3 {
        let mut changed = false;
        for &(_, ij) in &query_coordinate_evals {
            query.flip(ij);
            let eval = query.eval(sim, add_k, add_v);
            if current < eval {
                current = eval;
                changed = true;
            } else {
                query.flip(ij);
            }
        }

        while add_k < no_info_coordinates.len() {
            let next_v = add_v + best_volume[no_info_coordinates[add_k]] as usize;
            let eval = query.eval(sim, add_k + 1, next_v);
            if current < eval {
                current = eval;
                add_v = next_v;
                add_k += 1;
                changed = true;
            } else {
                break;
            }
        }

        while add_k > 0 {
            let prev_cell = no_info_coordinates[add_k - 1];
            let next_v = add_v - best_volume[prev_cell] as usize;
            let eval = query.eval(sim, add_k - 1, next_v);
            if current < eval {
                current = eval;
                add_v = next_v;
                add_k -= 1;
                changed = true;
            } else {
                break;
            }
        }

        if !changed {
            break;
        }
    }

    let mut query_coordinates = query.get_coordinates();
    query_coordinates.extend_from_slice(&no_info_coordinates[..add_k]);
    query_coordinates
}

fn sort_pool(pool: &mut [OilLayout]) {
    pool.sort_unstable_by(|a, b| b.ln_p_r_if_x.total_cmp(&a.ln_p_r_if_x));
}

fn shuffle_pool(pool: &mut [OilLayout], rng: &mut XorShift) {
    for i in (1..pool.len()).rev() {
        let j = rng.randrange(i + 1);
        pool.swap(i, j);
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut out = BufWriter::new(stdout.lock());

    let input = read_input(&mut scanner);
    let mut rng = XorShift::new(1);
    let mut sim = Sim::new(&input);
    let mut state = State::new(&input, &mut rng);
    let mut pool = Vec::<OilLayout>::new();
    let iter = THETA_VISIT_BUDGET / (2 * input.n2);
    let started_at = Instant::now();

    loop {
        if sim.rem == 0 {
            break;
        }
        if started_at.elapsed().as_secs_f64() > TIME_LIMIT_SEC {
            sim.giveup(&mut scanner, &mut out);
            break;
        }

        let mut hash_ln_likelihood = HashMap::with_capacity(pool.len() + iter);
        for layout in &pool {
            hash_ln_likelihood.insert(layout.hash, layout.ln_p_r_if_x);
        }

        for _ in 0..iter {
            for oil_id in 0..input.m {
                let oil = &input.oils[oil_id];
                let i = rng.randrange(input.n - oil.max_i);
                let j = rng.randrange(input.n - oil.max_j);
                state.move_to(&input, oil_id, i * input.n + j);
            }

            if let std::collections::hash_map::Entry::Vacant(entry) =
                hash_ln_likelihood.entry(state.hash)
            {
                entry.insert(0.0);
                pool.push(OilLayout {
                    hash: state.hash,
                    ln_p_r_if_x: 0.0,
                    px_if_r: 0.0,
                    top_lefts: state.top_lefts.clone(),
                    volume: state.volumes.clone(),
                });
            }
        }

        for layout in &mut pool {
            if layout.volume.is_empty() && !sim.failed.is_empty() {
                layout.volume = input.get_volume(&layout.top_lefts);
            }
            layout.ln_p_r_if_x =
                sim.get_ln_p_r_if_x(&state.oil_states, &layout.volume, &layout.top_lefts);
        }

        shuffle_pool(&mut pool, &mut rng);
        sort_pool(&mut pool);
        if pool.is_empty() {
            sim.giveup(&mut scanner, &mut out);
            break;
        }

        let max_prob = pool[0].ln_p_r_if_x;
        for layout in &mut pool {
            layout.px_if_r = (layout.ln_p_r_if_x - max_prob).exp();
        }
        normalize_pool(&mut pool);

        while pool.len() > 1 && pool.last().unwrap().px_if_r < 1.0e-9 {
            pool.pop();
        }

        let best_top_lefts = pool[0].top_lefts.clone();
        let best_pool_prob = pool[0].px_if_r;
        let best_coordinates = input.positive_coordinates(&best_top_lefts);

        concat_pool(&mut pool, iter, started_at);
        normalize_pool(&mut pool);
        set_volume(&mut pool, &input);

        if best_pool_prob > 0.9 {
            if sim.ans(&mut scanner, &mut out, &best_coordinates) {
                break;
            } else if sim.failed.len() == 1 {
                state.volumes = input.get_volume(&state.top_lefts);
            }
        } else {
            let query_coordinates = get_divination_query(&input, &mut pool, &sim, &mut rng);
            sim.query(&mut scanner, &mut out, &query_coordinates);
            state.add_query(&input, &query_coordinates);
        }
    }
}
