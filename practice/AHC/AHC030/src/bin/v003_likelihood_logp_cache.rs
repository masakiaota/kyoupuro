// v003_likelihood_logp_cache.rs
use std::collections::{HashMap, HashSet};
use std::f64::consts::SQRT_2;
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;
use std::time::Instant;

const MASK_WORDS: usize = 7;
const POSTERIOR_THRESHOLD: f64 = 0.95;
const MAX_EXHAUSTIVE_CANDIDATES: usize = 1_000_000;
const LATE_ANSWER_START_SEC: f64 = 1.9;

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    m: usize,
    epsilon: f64,
    shape_cells: Vec<Vec<(usize, usize)>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct Mask {
    words: [u64; MASK_WORDS],
}

impl Mask {
    fn set(&mut self, id: usize) {
        self.words[id >> 6] |= 1_u64 << (id & 63);
    }

    fn or_assign(&mut self, other: &Mask) {
        for i in 0..MASK_WORDS {
            self.words[i] |= other.words[i];
        }
    }

    fn and_count(&self, other: &Mask) -> usize {
        let mut count = 0;
        for i in 0..MASK_WORDS {
            count += (self.words[i] & other.words[i]).count_ones() as usize;
        }
        count
    }

    fn to_cells(self, n: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();
        for id in 0..n * n {
            if ((self.words[id >> 6] >> (id & 63)) & 1) != 0 {
                cells.push((id / n, id % n));
            }
        }
        cells
    }
}

#[derive(Debug, Clone)]
struct Placement {
    placed_mask: Mask,
}

#[derive(Debug)]
struct Candidates {
    m: usize,
    placement_indices: Vec<u16>,
    positive_masks: Vec<Mask>,
    logw: Vec<f64>,
}

impl Candidates {
    fn len(&self) -> usize {
        self.logw.len()
    }

    fn push(&mut self, indices: &[u16], positive_mask: Mask) {
        self.placement_indices.extend_from_slice(indices);
        self.positive_masks.push(positive_mask);
        self.logw.push(0.0);
    }

    fn placement_index(&self, candidate_id: usize, oil_id: usize) -> usize {
        self.placement_indices[candidate_id * self.m + oil_id] as usize
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

#[derive(Debug, Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        self.state = x;
        x
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut out = BufWriter::new(stdout.lock());
    let started_at = Instant::now();

    let input = read_initial_input(&mut scanner);
    let query_limit = 2 * input.n * input.n;
    let query_size = input.n * input.n / 2;
    let mut turns_used = 0_usize;

    let placements_by_oil = enumerate_placements(&input);
    let theta_size = theta_size(&placements_by_oil, MAX_EXHAUSTIVE_CANDIDATES);
    if theta_size.is_none() {
        eprintln!(
            "v003 fallback=mine_all reason=theta_size_over_limit limit={} m={} query_size={}",
            MAX_EXHAUSTIVE_CANDIDATES, input.m, query_size
        );
        mine_all_and_answer(&mut scanner, &mut out, input.n);
        return;
    }

    let mut candidates = enumerate_candidates(&input, &placements_by_oil);

    eprintln!(
        "v003 candidates={} m={} query_size={}",
        candidates.len(),
        input.m,
        query_size
    );

    let mut rng = XorShift64::new(make_seed(&input));

    while turns_used < query_limit {
        if started_at.elapsed().as_secs_f64() >= LATE_ANSWER_START_SEC {
            answer_by_sorted_likelihood(
                &mut scanner,
                &mut out,
                input.n,
                &candidates,
                turns_used,
                query_limit,
            );
            return;
        }

        let best = best_posterior_mask(&candidates);
        if let Some((best_mask, best_prob)) = best {
            if best_prob >= POSTERIOR_THRESHOLD {
                let accepted = ask_answer(&mut scanner, &mut out, input.n, best_mask);
                turns_used += 1;
                if accepted {
                    return;
                }
                eliminate_positive_mask(&mut candidates, best_mask);
                continue;
            }
        } else {
            eprintln!("v003 failed: no finite posterior candidates");
            return;
        }

        if turns_used + 1 == query_limit {
            if let Some(best_mask) = max_likelihood_mask(&candidates) {
                let _ = ask_answer(&mut scanner, &mut out, input.n, best_mask);
            }
            return;
        }

        let query_mask = sample_query_mask(input.n, query_size, &mut rng);
        let query_cells = query_mask.to_cells(input.n);
        let y = ask_survey(&mut scanner, &mut out, &query_cells);
        turns_used += 1;

        update_logw_by_observation(
            &mut candidates,
            &placements_by_oil,
            &query_mask,
            y,
            query_size,
            input.epsilon,
        );
    }
}

fn read_initial_input<R: BufRead>(scanner: &mut Scanner<R>) -> Input {
    let n: usize = scanner.next();
    let m: usize = scanner.next();
    let epsilon: f64 = scanner.next();
    let mut shape_cells = Vec::with_capacity(m);
    for _ in 0..m {
        let d_k: usize = scanner.next();
        let mut cells = Vec::with_capacity(d_k);
        for _ in 0..d_k {
            let i: usize = scanner.next();
            let j: usize = scanner.next();
            cells.push((i, j));
        }
        shape_cells.push(cells);
    }
    Input {
        n,
        m,
        epsilon,
        shape_cells,
    }
}

fn enumerate_placements(input: &Input) -> Vec<Vec<Placement>> {
    let mut placements_by_oil = Vec::with_capacity(input.m);
    for oil_id in 0..input.m {
        let shape = &input.shape_cells[oil_id];
        let max_i = shape.iter().map(|&(i, _)| i).max().unwrap();
        let max_j = shape.iter().map(|&(_, j)| j).max().unwrap();
        let mut placements = Vec::new();

        for deltai_k in 0..input.n - max_i {
            for deltaj_k in 0..input.n - max_j {
                let mut placed_mask = Mask::default();
                for &(i, j) in shape {
                    placed_mask.set((deltai_k + i) * input.n + (deltaj_k + j));
                }
                placements.push(Placement { placed_mask });
            }
        }
        placements_by_oil.push(placements);
    }
    placements_by_oil
}

fn theta_size(placements_by_oil: &[Vec<Placement>], limit: usize) -> Option<usize> {
    let mut total = 1_usize;
    for placements in placements_by_oil {
        total = total.checked_mul(placements.len())?;
        if total > limit {
            return None;
        }
    }
    Some(total)
}

fn enumerate_candidates(input: &Input, placements_by_oil: &[Vec<Placement>]) -> Candidates {
    let mut candidates = Candidates {
        m: input.m,
        placement_indices: Vec::new(),
        positive_masks: Vec::new(),
        logw: Vec::new(),
    };
    let mut current_indices = vec![0_u16; input.m];
    dfs_enumerate_candidates(
        0,
        Mask::default(),
        &mut current_indices,
        placements_by_oil,
        &mut candidates,
    );
    candidates
}

fn dfs_enumerate_candidates(
    oil_id: usize,
    current_mask: Mask,
    current_indices: &mut [u16],
    placements_by_oil: &[Vec<Placement>],
    candidates: &mut Candidates,
) {
    if oil_id == placements_by_oil.len() {
        candidates.push(current_indices, current_mask);
        return;
    }

    for placement_id in 0..placements_by_oil[oil_id].len() {
        current_indices[oil_id] = placement_id as u16;
        let mut next_mask = current_mask;
        next_mask.or_assign(&placements_by_oil[oil_id][placement_id].placed_mask);
        dfs_enumerate_candidates(
            oil_id + 1,
            next_mask,
            current_indices,
            placements_by_oil,
            candidates,
        );
    }
}

fn sample_query_mask(n: usize, query_size: usize, rng: &mut XorShift64) -> Mask {
    let total = n * n;
    let mut ids: Vec<usize> = (0..total).collect();
    let mut mask = Mask::default();

    for pos in 0..query_size {
        let swap_pos = pos + rng.next_usize(total - pos);
        ids.swap(pos, swap_pos);
        mask.set(ids[pos]);
    }
    mask
}

fn ask_survey<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    query_cells: &[(usize, usize)],
) -> i32 {
    write!(out, "q {}", query_cells.len()).unwrap();
    for &(i, j) in query_cells {
        write!(out, " {} {}", i, j).unwrap();
    }
    writeln!(out).unwrap();
    out.flush().unwrap();
    scanner.next()
}

fn ask_mining<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    i: usize,
    j: usize,
) -> i32 {
    writeln!(out, "q 1 {} {}", i, j).unwrap();
    out.flush().unwrap();
    scanner.next()
}

fn ask_answer<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    n: usize,
    answer_mask: Mask,
) -> bool {
    let answer_cells = answer_mask.to_cells(n);
    write!(out, "a {}", answer_cells.len()).unwrap();
    for (i, j) in answer_cells {
        write!(out, " {} {}", i, j).unwrap();
    }
    writeln!(out).unwrap();
    out.flush().unwrap();
    let accepted: i32 = scanner.next();
    accepted == 1
}

fn update_logw_by_observation(
    candidates: &mut Candidates,
    placements_by_oil: &[Vec<Placement>],
    query_mask: &Mask,
    y: i32,
    query_size: usize,
    epsilon: f64,
) {
    let mut hits_by_oil = Vec::with_capacity(placements_by_oil.len());
    for placements in placements_by_oil {
        let mut hits = Vec::with_capacity(placements.len());
        for placement in placements {
            hits.push(query_mask.and_count(&placement.placed_mask) as u16);
        }
        hits_by_oil.push(hits);
    }

    let sigma = (query_size as f64 * epsilon * (1.0 - epsilon)).sqrt();
    let max_sum_v: usize = hits_by_oil
        .iter()
        .map(|hits| hits.iter().copied().max().unwrap_or(0) as usize)
        .sum();
    let mut logp_by_sum_v = Vec::with_capacity(max_sum_v + 1);
    for sum_v in 0..=max_sum_v {
        let mu = query_size as f64 * epsilon + sum_v as f64 * (1.0 - 2.0 * epsilon);
        logp_by_sum_v.push(log_observed_probability(y, mu, sigma));
    }

    for candidate_id in 0..candidates.len() {
        if !candidates.logw[candidate_id].is_finite() {
            continue;
        }

        let mut sum_v = 0_usize;
        for (oil_id, hits) in hits_by_oil.iter().enumerate() {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            sum_v += hits[placement_id] as usize;
        }

        candidates.logw[candidate_id] += logp_by_sum_v[sum_v];
    }
}

fn log_observed_probability(y: i32, mu: f64, sigma: f64) -> f64 {
    let probability = if y <= 0 {
        standard_normal_cdf((0.5 - mu) / sigma)
    } else {
        let upper = standard_normal_cdf((y as f64 + 0.5 - mu) / sigma);
        let lower = standard_normal_cdf((y as f64 - 0.5 - mu) / sigma);
        upper - lower
    };

    if probability > 0.0 {
        probability.ln()
    } else {
        f64::NEG_INFINITY
    }
}

fn standard_normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + libm::erf(z / SQRT_2))
}

fn best_posterior_mask(candidates: &Candidates) -> Option<(Mask, f64)> {
    let mut max_logw = f64::NEG_INFINITY;
    for &logw in &candidates.logw {
        if logw.is_finite() && logw > max_logw {
            max_logw = logw;
        }
    }
    if !max_logw.is_finite() {
        return None;
    }

    let mut total_weight = 0.0;
    let mut by_positive_mask: HashMap<Mask, f64> = HashMap::new();
    for candidate_id in 0..candidates.len() {
        let logw = candidates.logw[candidate_id];
        if !logw.is_finite() {
            continue;
        }
        let weight = (logw - max_logw).exp();
        total_weight += weight;
        *by_positive_mask
            .entry(candidates.positive_masks[candidate_id])
            .or_insert(0.0) += weight;
    }

    let mut best_mask = Mask::default();
    let mut best_weight = -1.0;
    for (mask, weight) in by_positive_mask {
        if weight > best_weight {
            best_mask = mask;
            best_weight = weight;
        }
    }

    if best_weight >= 0.0 && total_weight > 0.0 {
        Some((best_mask, best_weight / total_weight))
    } else {
        None
    }
}

fn eliminate_positive_mask(candidates: &mut Candidates, rejected_mask: Mask) {
    for candidate_id in 0..candidates.len() {
        if candidates.positive_masks[candidate_id] == rejected_mask {
            candidates.logw[candidate_id] = f64::NEG_INFINITY;
        }
    }
}

fn max_likelihood_mask(candidates: &Candidates) -> Option<Mask> {
    let mut best_logw = f64::NEG_INFINITY;
    let mut best_mask = None;
    for candidate_id in 0..candidates.len() {
        let logw = candidates.logw[candidate_id];
        if logw.is_finite() && logw > best_logw {
            best_logw = logw;
            best_mask = Some(candidates.positive_masks[candidate_id]);
        }
    }
    best_mask
}

fn answer_by_sorted_likelihood<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    n: usize,
    candidates: &Candidates,
    mut turns_used: usize,
    query_limit: usize,
) {
    let mut candidate_ids: Vec<usize> = (0..candidates.len())
        .filter(|&candidate_id| candidates.logw[candidate_id].is_finite())
        .collect();

    candidate_ids.sort_unstable_by(|&a, &b| candidates.logw[b].total_cmp(&candidates.logw[a]));

    let mut tried_masks = HashSet::new();
    for candidate_id in candidate_ids {
        if turns_used >= query_limit {
            return;
        }

        let answer_mask = candidates.positive_masks[candidate_id];
        if !tried_masks.insert(answer_mask) {
            continue;
        }

        let accepted = ask_answer(scanner, out, n, answer_mask);
        turns_used += 1;
        if accepted {
            return;
        }
    }
}

fn mine_all_and_answer<R: BufRead, W: Write>(scanner: &mut Scanner<R>, out: &mut W, n: usize) {
    let mut answer_mask = Mask::default();
    for id in 0..n * n {
        let i = id / n;
        let j = id % n;
        let amount = ask_mining(scanner, out, i, j);
        if amount > 0 {
            answer_mask.set(id);
        }
    }
    let _ = ask_answer(scanner, out, n, answer_mask);
}

fn make_seed(input: &Input) -> u64 {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    seed ^= input.n as u64;
    seed = seed.rotate_left(13) ^ input.m as u64;
    seed = seed.rotate_left(13) ^ input.epsilon.to_bits();
    for shape in &input.shape_cells {
        seed = seed.rotate_left(7) ^ shape.len() as u64;
        for &(i, j) in shape {
            seed = seed.rotate_left(7) ^ ((i as u64) << 32) ^ j as u64;
        }
    }
    seed
}
