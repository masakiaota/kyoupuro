// v013_random_mi_posterior_cutoff_099.rs
use std::collections::{HashMap, HashSet};
use std::f64::consts::SQRT_2;
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;
use std::time::Instant;

const MASK_WORDS: usize = 7;
const POSTERIOR_THRESHOLD: f64 = 0.95;
const THETA_VISIT_BUDGET: usize = 4_000_000;
const MI_POSTERIOR_MASS_CUTOFF: f64 = 0.99;
const LATE_ANSWER_START_SEC: f64 = 1.9;
const MI_TAIL_SIGMA: f64 = 8.0;
const SINGLE_CELL_MI_TIE_EPS: f64 = 1.0e-12;

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

    fn clear(&mut self, id: usize) {
        self.words[id >> 6] &= !(1_u64 << (id & 63));
    }

    fn count_ones(&self) -> usize {
        let mut count = 0;
        for word in self.words {
            count += word.count_ones() as usize;
        }
        count
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
    cell_ids: Vec<usize>,
}

#[derive(Debug, Clone)]
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
        self.push_with_logw(indices, positive_mask, 0.0);
    }

    fn push_with_logw(&mut self, indices: &[u16], positive_mask: Mask, logw: f64) {
        self.placement_indices.extend_from_slice(indices);
        self.positive_masks.push(positive_mask);
        self.logw.push(logw);
    }

    fn placement_index(&self, candidate_id: usize, oil_id: usize) -> usize {
        self.placement_indices[candidate_id * self.m + oil_id] as usize
    }

    fn placement_indices_for(&self, candidate_id: usize) -> &[u16] {
        let start = candidate_id * self.m;
        &self.placement_indices[start..start + self.m]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateMode {
    Exhaustive,
    Sampled,
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
    let mut turns_used = 0_usize;
    let mut rng = XorShift64::new(make_seed(&input));

    let placements_by_oil = enumerate_placements(&input);
    let target_size = candidate_limit(input.n);
    let is_exhaustive = theta_size(&placements_by_oil, target_size).is_some();
    let zobrist = if is_exhaustive {
        Vec::new()
    } else {
        build_zobrist(&placements_by_oil, &mut rng)
    };
    let (mut answer_candidates, mut mi_candidates, candidate_mode) = if is_exhaustive {
        let candidates = enumerate_candidates(&input, &placements_by_oil);
        (candidates.clone(), candidates, CandidateMode::Exhaustive)
    } else {
        let candidates =
            initialize_candidate_pool(&input, &placements_by_oil, &zobrist, target_size, &mut rng);
        (candidates.clone(), candidates, CandidateMode::Sampled)
    };
    let total_cells = input.n * input.n;
    let max_sum_v = input.shape_cells.iter().map(Vec::len).sum::<usize>();
    let observation_model = ObservationModel::precompute(total_cells, input.epsilon, max_sum_v);
    let mut mi_cell_values = precompute_cell_values(input.n, &mi_candidates, &placements_by_oil);

    eprintln!(
        "v012 mode={:?} target_size={} answer_candidates={} mi_candidates={} m={} mi_cutoff={}",
        candidate_mode,
        target_size,
        answer_candidates.len(),
        mi_candidates.len(),
        input.m,
        MI_POSTERIOR_MASS_CUTOFF
    );

    let mut observations = Vec::new();
    let mut rejected_masks = HashSet::new();
    while turns_used < query_limit {
        if started_at.elapsed().as_secs_f64() >= LATE_ANSWER_START_SEC {
            answer_by_sorted_likelihood(
                &mut scanner,
                &mut out,
                input.n,
                &answer_candidates,
                turns_used,
                query_limit,
            );
            return;
        }

        let best = best_posterior_mask(&answer_candidates);
        if let Some((best_mask, best_prob)) = best {
            if best_prob >= POSTERIOR_THRESHOLD {
                let accepted = ask_answer(&mut scanner, &mut out, input.n, best_mask);
                turns_used += 1;
                if accepted {
                    return;
                }
                rejected_masks.insert(best_mask);
                eliminate_positive_mask(&mut answer_candidates, best_mask);
                eliminate_positive_mask(&mut mi_candidates, best_mask);
                if candidate_mode == CandidateMode::Sampled {
                    (answer_candidates, mi_candidates) = refresh_candidate_pools(
                        &input,
                        &placements_by_oil,
                        &zobrist,
                        &observations,
                        &rejected_masks,
                        &answer_candidates,
                        target_size,
                        &mut rng,
                        started_at,
                        LATE_ANSWER_START_SEC,
                    );
                    mi_cell_values =
                        precompute_cell_values(input.n, &mi_candidates, &placements_by_oil);
                }
                continue;
            }
        } else {
            eprintln!("v012 failed: no finite posterior candidates");
            return;
        }

        if turns_used + 1 == query_limit {
            if let Some(best_mask) = max_likelihood_mask(&answer_candidates) {
                let _ = ask_answer(&mut scanner, &mut out, input.n, best_mask);
            }
            return;
        }

        let query_mask = optimize_query_mask_by_mi_per_cost(
            &input,
            &placements_by_oil,
            &mi_candidates,
            &observation_model,
            &mi_cell_values,
            &mut rng,
            started_at,
            LATE_ANSWER_START_SEC,
        );

        if started_at.elapsed().as_secs_f64() >= LATE_ANSWER_START_SEC {
            answer_by_sorted_likelihood(
                &mut scanner,
                &mut out,
                input.n,
                &answer_candidates,
                turns_used,
                query_limit,
            );
            return;
        }

        let query_size = query_mask.count_ones();
        let query_cells = query_mask.to_cells(input.n);
        let y = ask_survey(&mut scanner, &mut out, &query_cells);
        turns_used += 1;

        let observation = build_observation_data(
            &placements_by_oil,
            &query_mask,
            y,
            query_size,
            input.epsilon,
        );
        update_logw_by_observation(&mut answer_candidates, &observation);
        update_logw_by_observation(&mut mi_candidates, &observation);
        observations.push(observation);

        if candidate_mode == CandidateMode::Sampled {
            (answer_candidates, mi_candidates) = refresh_candidate_pools(
                &input,
                &placements_by_oil,
                &zobrist,
                &observations,
                &rejected_masks,
                &answer_candidates,
                target_size,
                &mut rng,
                started_at,
                LATE_ANSWER_START_SEC,
            );
            mi_cell_values = precompute_cell_values(input.n, &mi_candidates, &placements_by_oil);
        }
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
                let mut cell_ids = Vec::with_capacity(shape.len());
                for &(i, j) in shape {
                    let cell_id = (deltai_k + i) * input.n + (deltaj_k + j);
                    placed_mask.set(cell_id);
                    cell_ids.push(cell_id);
                }
                placements.push(Placement {
                    placed_mask,
                    cell_ids,
                });
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

fn candidate_limit(n: usize) -> usize {
    let denominator = 2 * n * n;
    (THETA_VISIT_BUDGET / denominator).max(1)
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

fn empty_candidates(m: usize, capacity: usize) -> Candidates {
    Candidates {
        m,
        placement_indices: Vec::with_capacity(capacity * m),
        positive_masks: Vec::with_capacity(capacity),
        logw: Vec::with_capacity(capacity),
    }
}

fn build_zobrist(placements_by_oil: &[Vec<Placement>], rng: &mut XorShift64) -> Vec<Vec<u64>> {
    placements_by_oil
        .iter()
        .map(|placements| placements.iter().map(|_| rng.next_u64()).collect())
        .collect()
}

fn initialize_candidate_pool(
    input: &Input,
    placements_by_oil: &[Vec<Placement>],
    zobrist: &[Vec<u64>],
    target_size: usize,
    rng: &mut XorShift64,
) -> Candidates {
    let mut candidates = empty_candidates(input.m, target_size);
    let mut seen = HashSet::with_capacity(target_size * 2);
    let mut indices = vec![0_u16; input.m];

    while candidates.len() < target_size {
        fill_random_candidate_indices(&mut indices, placements_by_oil, rng);
        push_candidate_if_new(
            &mut candidates,
            placements_by_oil,
            zobrist,
            &mut seen,
            &indices,
            0.0,
            None,
        );
    }

    candidates
}

fn refresh_candidate_pools(
    input: &Input,
    placements_by_oil: &[Vec<Placement>],
    zobrist: &[Vec<u64>],
    observations: &[ObservationData],
    rejected_masks: &HashSet<Mask>,
    current: &Candidates,
    target_size: usize,
    rng: &mut XorShift64,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> (Candidates, Candidates) {
    let mut candidate_ids: Vec<usize> = (0..current.len())
        .filter(|&candidate_id| current.logw[candidate_id].is_finite())
        .collect();
    candidate_ids.sort_unstable_by(|&a, &b| current.logw[b].total_cmp(&current.logw[a]));

    let keep_count = (target_size / 100).max(1).min(target_size);
    let mi_target_size = target_size.saturating_sub(keep_count);
    let mut next_answer = empty_candidates(input.m, target_size);
    let mut next_mi = empty_candidates(input.m, mi_target_size);
    let mut seen_answer = HashSet::with_capacity(target_size * 2);
    let mut seen_mi = HashSet::with_capacity(mi_target_size * 2);

    for &candidate_id in candidate_ids.iter().take(keep_count) {
        let indices = current.placement_indices_for(candidate_id);
        push_candidate_if_new(
            &mut next_answer,
            placements_by_oil,
            zobrist,
            &mut seen_answer,
            indices,
            current.logw[candidate_id],
            Some(rejected_masks),
        );
    }

    if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
        return (next_answer, next_mi);
    }

    let mut indices = vec![0_u16; input.m];
    while next_answer.len() < target_size {
        if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
            return (next_answer, next_mi);
        }
        fill_random_candidate_indices(&mut indices, placements_by_oil, rng);
        let logw = compute_candidate_logw(&indices, observations);
        if push_candidate_if_new(
            &mut next_answer,
            placements_by_oil,
            zobrist,
            &mut seen_answer,
            &indices,
            logw,
            Some(rejected_masks),
        ) {
            push_candidate_if_new(
                &mut next_mi,
                placements_by_oil,
                zobrist,
                &mut seen_mi,
                &indices,
                logw,
                Some(rejected_masks),
            );
        }
    }

    (next_answer, next_mi)
}

fn fill_random_candidate_indices(
    indices: &mut [u16],
    placements_by_oil: &[Vec<Placement>],
    rng: &mut XorShift64,
) {
    for (oil_id, value) in indices.iter_mut().enumerate() {
        *value = rng.next_usize(placements_by_oil[oil_id].len()) as u16;
    }
}

fn push_candidate_if_new(
    candidates: &mut Candidates,
    placements_by_oil: &[Vec<Placement>],
    zobrist: &[Vec<u64>],
    seen: &mut HashSet<u64>,
    indices: &[u16],
    logw: f64,
    rejected_masks: Option<&HashSet<Mask>>,
) -> bool {
    if !logw.is_finite() {
        return false;
    }

    let hash = compute_candidate_hash(indices, zobrist);
    if seen.contains(&hash) {
        return false;
    }

    let positive_mask = compute_positive_mask(indices, placements_by_oil);
    if let Some(masks) = rejected_masks {
        if masks.contains(&positive_mask) {
            return false;
        }
    }

    seen.insert(hash);
    candidates.push_with_logw(indices, positive_mask, logw);
    true
}

fn compute_candidate_hash(indices: &[u16], zobrist: &[Vec<u64>]) -> u64 {
    let mut hash = 0_u64;
    for (oil_id, &placement_id) in indices.iter().enumerate() {
        hash ^= zobrist[oil_id][placement_id as usize];
    }
    hash
}

fn compute_positive_mask(indices: &[u16], placements_by_oil: &[Vec<Placement>]) -> Mask {
    let mut positive_mask = Mask::default();
    for (oil_id, &placement_id) in indices.iter().enumerate() {
        positive_mask.or_assign(&placements_by_oil[oil_id][placement_id as usize].placed_mask);
    }
    positive_mask
}

fn compute_candidate_logw(indices: &[u16], observations: &[ObservationData]) -> f64 {
    let mut logw = 0.0;
    for observation in observations {
        let mut sum_v = 0_usize;
        for (oil_id, hits) in observation.hits_by_oil.iter().enumerate() {
            sum_v += hits[indices[oil_id] as usize] as usize;
        }
        logw += observation.logp_by_sum_v[sum_v];
        if !logw.is_finite() {
            return f64::NEG_INFINITY;
        }
    }
    logw
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

fn precompute_cell_values(
    n: usize,
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
) -> CellValues {
    let total_cells = n * n;
    let candidate_count = candidates.len();
    let mut values_by_cell = vec![0_u8; total_cells * candidate_count];

    for candidate_id in 0..candidate_count {
        for oil_id in 0..candidates.m {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            for &cell_id in &placements_by_oil[oil_id][placement_id].cell_ids {
                values_by_cell[cell_id * candidate_count + candidate_id] += 1;
            }
        }
    }

    CellValues {
        candidate_count,
        values_by_cell,
    }
}

#[derive(Debug)]
struct Posterior {
    active_ids: Vec<usize>,
    weights: Vec<f64>,
}

#[derive(Clone)]
struct ObservationTable {
    y_count: usize,
    prob_by_v_y: Vec<f64>,
    entropy_by_v: Vec<f64>,
}

struct ObservationData {
    hits_by_oil: Vec<Vec<u16>>,
    logp_by_sum_v: Vec<f64>,
}

struct ObservationModel {
    max_sum_v: usize,
    tables_by_query_size: Vec<Option<ObservationTable>>,
}

impl ObservationModel {
    fn precompute(total_cells: usize, epsilon: f64, max_sum_v: usize) -> Self {
        let mut tables_by_query_size = (0..=total_cells).map(|_| None).collect::<Vec<_>>();
        for query_size in 2..=total_cells {
            tables_by_query_size[query_size] =
                Some(build_observation_table(query_size, epsilon, max_sum_v));
        }
        Self {
            max_sum_v,
            tables_by_query_size,
        }
    }

    fn table(&self, query_size: usize) -> &ObservationTable {
        self.tables_by_query_size[query_size].as_ref().unwrap()
    }

    fn mi_per_cost_from_mass(&self, query_size: usize, mass_by_v: &[f64]) -> f64 {
        let table = self.table(query_size);
        let mut pred_y = vec![0.0_f64; table.y_count];
        let mut conditional_entropy = 0.0;

        for (sum_v, &mass) in mass_by_v.iter().enumerate() {
            if mass == 0.0 {
                continue;
            }

            conditional_entropy += mass * table.entropy_by_v[sum_v];
            let row_start = sum_v * table.y_count;
            for y in 0..table.y_count {
                pred_y[y] += mass * table.prob_by_v_y[row_start + y];
            }
        }

        let mut predictive_entropy = 0.0;
        for probability in pred_y {
            if probability > 0.0 {
                predictive_entropy -= probability * probability.ln();
            }
        }

        (query_size as f64).sqrt() * (predictive_entropy - conditional_entropy)
    }
}

struct CellValues {
    candidate_count: usize,
    values_by_cell: Vec<u8>,
}

impl CellValues {
    fn values_for_cell(&self, cell_id: usize) -> &[u8] {
        let start = cell_id * self.candidate_count;
        &self.values_by_cell[start..start + self.candidate_count]
    }
}

#[derive(Clone)]
struct QueryState {
    query_mask: Mask,
    in_query: Vec<bool>,
    query_size: usize,
    sum_v_by_active: Vec<u16>,
    mass_by_v: Vec<f64>,
    score: f64,
}

#[derive(Clone, Copy)]
enum QueryMove {
    Add { cell: usize },
    Remove { cell: usize },
}

fn optimize_query_mask_by_mi_per_cost(
    input: &Input,
    placements_by_oil: &[Vec<Placement>],
    candidates: &Candidates,
    observation_model: &ObservationModel,
    cell_values: &CellValues,
    rng: &mut XorShift64,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> Mask {
    let total_cells = input.n * input.n;
    let posterior = build_mi_posterior(candidates, MI_POSTERIOR_MASS_CUTOFF)
        .expect("finite posterior must exist");
    let initial_query_size = (total_cells / 2).max(2).min(total_cells);

    if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
        return sample_query_mask(input.n, initial_query_size, rng);
    }

    let cell_order = compute_single_cell_mi_order(
        total_cells,
        input.m,
        &posterior,
        candidates,
        placements_by_oil,
        rng,
    );

    let mut state = ordered_initial_query_state(
        input.n,
        initial_query_size,
        &cell_order,
        &posterior,
        candidates,
        placements_by_oil,
        observation_model,
    );
    let mut next_sum_v_by_active = vec![0_u16; posterior.active_ids.len()];
    let mut next_mass_by_v = vec![0.0_f64; observation_model.max_sum_v + 1];

    loop {
        if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
            break;
        }

        if try_add_by_single_cell_mi_order(
            &mut state,
            &cell_order,
            &posterior,
            cell_values,
            observation_model,
            &mut next_sum_v_by_active,
            &mut next_mass_by_v,
            started_at,
            late_answer_start_sec,
        ) {
            continue;
        }

        if try_remove_by_single_cell_mi_order(
            &mut state,
            &cell_order,
            &posterior,
            cell_values,
            observation_model,
            &mut next_sum_v_by_active,
            &mut next_mass_by_v,
            started_at,
            late_answer_start_sec,
        ) {
            continue;
        }

        break;
    }

    state.query_mask
}

fn build_mi_posterior(candidates: &Candidates, mass_cutoff: f64) -> Option<Posterior> {
    let mut max_logw = f64::NEG_INFINITY;
    for &logw in &candidates.logw {
        if logw.is_finite() && logw > max_logw {
            max_logw = logw;
        }
    }
    if !max_logw.is_finite() {
        return None;
    }

    let mut entries = Vec::new();
    let mut total_weight = 0.0;
    for candidate_id in 0..candidates.len() {
        let logw = candidates.logw[candidate_id];
        if !logw.is_finite() {
            continue;
        }

        let weight = (logw - max_logw).exp();
        entries.push((candidate_id, weight));
        total_weight += weight;
    }

    if total_weight <= 0.0 {
        return None;
    }

    entries.sort_unstable_by(|&(_, weight_a), &(_, weight_b)| weight_b.total_cmp(&weight_a));

    let target_mass = (total_weight * mass_cutoff.clamp(0.0, 1.0)).max(0.0);
    let mut kept_total_weight = 0.0;
    let mut active_ids = Vec::new();
    let mut weights = Vec::new();

    for (candidate_id, weight) in entries {
        if kept_total_weight >= target_mass && !active_ids.is_empty() {
            break;
        }
        active_ids.push(candidate_id);
        weights.push(weight);
        kept_total_weight += weight;
    }

    if kept_total_weight <= 0.0 {
        return None;
    }

    for weight in &mut weights {
        *weight /= kept_total_weight;
    }

    Some(Posterior {
        active_ids,
        weights,
    })
}

fn compute_single_cell_mi_order(
    total_cells: usize,
    m: usize,
    posterior: &Posterior,
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
    rng: &mut XorShift64,
) -> Vec<usize> {
    let mut mass_by_cell_value = vec![0.0_f64; total_cells * (m + 1)];
    let mut cell_count = vec![0_u8; total_cells];
    let mut touched_cells = Vec::new();

    for (active_index, &candidate_id) in posterior.active_ids.iter().enumerate() {
        let weight = posterior.weights[active_index];
        touched_cells.clear();

        for oil_id in 0..candidates.m {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            for &cell_id in &placements_by_oil[oil_id][placement_id].cell_ids {
                if cell_count[cell_id] == 0 {
                    touched_cells.push(cell_id);
                }
                cell_count[cell_id] += 1;
            }
        }

        for &cell_id in &touched_cells {
            let value = cell_count[cell_id] as usize;
            mass_by_cell_value[cell_id * (m + 1) + value] += weight;
            cell_count[cell_id] = 0;
        }
    }

    let mut scores = Vec::with_capacity(total_cells);

    for cell_id in 0..total_cells {
        let base = cell_id * (m + 1);
        let mut occupied_mass = 0.0;
        for value in 1..=m {
            occupied_mass += mass_by_cell_value[base + value];
        }
        mass_by_cell_value[base] = (1.0 - occupied_mass).max(0.0);

        let mut entropy = 0.0;
        for value in 0..=m {
            let probability = mass_by_cell_value[base + value];
            if probability > 0.0 {
                entropy -= probability * probability.ln();
            }
        }
        let bucket = (entropy / SINGLE_CELL_MI_TIE_EPS).round() as i64;
        scores.push((cell_id, bucket, rng.next_u64()));
    }

    scores.sort_unstable_by(|&(_, bucket_a, key_a), &(_, bucket_b, key_b)| {
        bucket_b.cmp(&bucket_a).then_with(|| key_a.cmp(&key_b))
    });
    scores.into_iter().map(|(cell_id, _, _)| cell_id).collect()
}

fn ordered_initial_query_state(
    n: usize,
    query_size: usize,
    cell_order: &[usize],
    posterior: &Posterior,
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
    observation_model: &ObservationModel,
) -> QueryState {
    let total_cells = n * n;
    let mut query_mask = Mask::default();
    let mut in_query = vec![false; total_cells];

    for &cell_id in cell_order.iter().take(query_size) {
        query_mask.set(cell_id);
        in_query[cell_id] = true;
    }

    let sum_v_by_active =
        compute_sum_v_by_active(&query_mask, posterior, candidates, placements_by_oil);
    let mass_by_v = build_mass_by_v(&sum_v_by_active, posterior, observation_model.max_sum_v);
    let score = observation_model.mi_per_cost_from_mass(query_size, &mass_by_v);

    QueryState {
        query_mask,
        in_query,
        query_size,
        sum_v_by_active,
        mass_by_v,
        score,
    }
}

fn try_add_by_single_cell_mi_order(
    state: &mut QueryState,
    cell_order: &[usize],
    posterior: &Posterior,
    cell_values: &CellValues,
    observation_model: &ObservationModel,
    next_sum_v_by_active: &mut Vec<u16>,
    next_mass_by_v: &mut Vec<f64>,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> bool {
    if state.query_size >= state.in_query.len() {
        return false;
    }

    for &cell in cell_order {
        if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
            return false;
        }
        if state.in_query[cell] {
            continue;
        }

        if try_apply_improving_move(
            state,
            QueryMove::Add { cell },
            posterior,
            cell_values,
            observation_model,
            next_sum_v_by_active,
            next_mass_by_v,
        ) {
            return true;
        }
    }

    false
}

fn try_remove_by_single_cell_mi_order(
    state: &mut QueryState,
    cell_order: &[usize],
    posterior: &Posterior,
    cell_values: &CellValues,
    observation_model: &ObservationModel,
    next_sum_v_by_active: &mut Vec<u16>,
    next_mass_by_v: &mut Vec<f64>,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> bool {
    if state.query_size <= 2 {
        return false;
    }

    for &cell in cell_order.iter().rev() {
        if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
            return false;
        }
        if !state.in_query[cell] {
            continue;
        }

        if try_apply_improving_move(
            state,
            QueryMove::Remove { cell },
            posterior,
            cell_values,
            observation_model,
            next_sum_v_by_active,
            next_mass_by_v,
        ) {
            return true;
        }
    }

    false
}

fn try_apply_improving_move(
    state: &mut QueryState,
    query_move: QueryMove,
    posterior: &Posterior,
    cell_values: &CellValues,
    observation_model: &ObservationModel,
    next_sum_v_by_active: &mut Vec<u16>,
    next_mass_by_v: &mut Vec<f64>,
) -> bool {
    let next_query_size = next_query_size(state.query_size, query_move);
    fill_next_state_buffers(
        &state.sum_v_by_active,
        &state.mass_by_v,
        next_sum_v_by_active,
        next_mass_by_v,
        query_move,
        posterior,
        cell_values,
    );

    let next_score = observation_model.mi_per_cost_from_mass(next_query_size, next_mass_by_v);
    if next_score > state.score {
        apply_query_move(state, query_move, next_query_size);
        std::mem::swap(&mut state.sum_v_by_active, next_sum_v_by_active);
        std::mem::swap(&mut state.mass_by_v, next_mass_by_v);
        state.score = next_score;
        true
    } else {
        false
    }
}

fn build_mass_by_v(sum_v_by_active: &[u16], posterior: &Posterior, max_sum_v: usize) -> Vec<f64> {
    let mut mass_by_v = vec![0.0_f64; max_sum_v + 1];
    for (&sum_v, &weight) in sum_v_by_active.iter().zip(&posterior.weights) {
        mass_by_v[sum_v as usize] += weight;
    }
    mass_by_v
}

fn compute_sum_v_by_active(
    query_mask: &Mask,
    posterior: &Posterior,
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
) -> Vec<u16> {
    let mut hits_by_oil = Vec::with_capacity(placements_by_oil.len());
    for placements in placements_by_oil {
        let mut hits = Vec::with_capacity(placements.len());
        for placement in placements {
            hits.push(query_mask.and_count(&placement.placed_mask) as u16);
        }
        hits_by_oil.push(hits);
    }

    let mut sum_v_by_active = Vec::with_capacity(posterior.active_ids.len());
    for &candidate_id in &posterior.active_ids {
        let mut sum_v = 0_u16;
        for (oil_id, hits) in hits_by_oil.iter().enumerate() {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            sum_v += hits[placement_id];
        }
        sum_v_by_active.push(sum_v);
    }
    sum_v_by_active
}

fn next_query_size(current_size: usize, query_move: QueryMove) -> usize {
    match query_move {
        QueryMove::Add { .. } => current_size + 1,
        QueryMove::Remove { .. } => current_size - 1,
    }
}

fn fill_next_state_buffers(
    current_sum_v_by_active: &[u16],
    current_mass_by_v: &[f64],
    next_sum_v_by_active: &mut [u16],
    next_mass_by_v: &mut [f64],
    query_move: QueryMove,
    posterior: &Posterior,
    cell_values: &CellValues,
) {
    next_mass_by_v.copy_from_slice(current_mass_by_v);

    match query_move {
        QueryMove::Add { cell } => {
            let values = cell_values.values_for_cell(cell);
            for index in 0..current_sum_v_by_active.len() {
                let old_sum_v = current_sum_v_by_active[index] as usize;
                let delta = values[posterior.active_ids[index]] as usize;
                let next_sum_v = old_sum_v + delta;
                next_sum_v_by_active[index] = next_sum_v as u16;
                if delta != 0 {
                    let weight = posterior.weights[index];
                    next_mass_by_v[old_sum_v] -= weight;
                    next_mass_by_v[next_sum_v] += weight;
                }
            }
        }
        QueryMove::Remove { cell } => {
            let values = cell_values.values_for_cell(cell);
            for index in 0..current_sum_v_by_active.len() {
                let old_sum_v = current_sum_v_by_active[index] as usize;
                let delta = values[posterior.active_ids[index]] as usize;
                let next_sum_v = old_sum_v - delta;
                next_sum_v_by_active[index] = next_sum_v as u16;
                if delta != 0 {
                    let weight = posterior.weights[index];
                    next_mass_by_v[old_sum_v] -= weight;
                    next_mass_by_v[next_sum_v] += weight;
                }
            }
        }
    }
}

fn apply_query_move(state: &mut QueryState, query_move: QueryMove, next_query_size: usize) {
    match query_move {
        QueryMove::Add { cell } => {
            state.query_mask.set(cell);
            state.in_query[cell] = true;
        }
        QueryMove::Remove { cell } => {
            state.query_mask.clear(cell);
            state.in_query[cell] = false;
        }
    }
    state.query_size = next_query_size;
}

fn build_observation_table(query_size: usize, epsilon: f64, max_sum_v: usize) -> ObservationTable {
    let sigma = (query_size as f64 * epsilon * (1.0 - epsilon)).sqrt();
    let base_mu = query_size as f64 * epsilon;
    let edge_mu = base_mu + max_sum_v as f64 * (1.0 - 2.0 * epsilon);
    let max_mu = base_mu.max(edge_mu).max(0.0);
    let last_y = (max_mu + MI_TAIL_SIGMA * sigma + 3.0).ceil().max(1.0) as usize;
    let y_count = last_y + 1;

    let mut prob_by_v_y = vec![0.0_f64; (max_sum_v + 1) * y_count];
    let mut entropy_by_v = vec![0.0_f64; max_sum_v + 1];

    for sum_v in 0..=max_sum_v {
        let mu = base_mu + sum_v as f64 * (1.0 - 2.0 * epsilon);
        let row_start = sum_v * y_count;
        let mut entropy = 0.0;

        for y in 0..last_y {
            let probability = if y == 0 {
                standard_normal_cdf((0.5 - mu) / sigma)
            } else {
                let upper = standard_normal_cdf((y as f64 + 0.5 - mu) / sigma);
                let lower = standard_normal_cdf((y as f64 - 0.5 - mu) / sigma);
                upper - lower
            };

            prob_by_v_y[row_start + y] = probability;
            if probability > 0.0 {
                entropy -= probability * probability.ln();
            }
        }

        let tail_probability = 1.0 - standard_normal_cdf((last_y as f64 - 0.5 - mu) / sigma);
        prob_by_v_y[row_start + last_y] = tail_probability;
        if tail_probability > 0.0 {
            entropy -= tail_probability * tail_probability.ln();
        }

        entropy_by_v[sum_v] = entropy;
    }

    ObservationTable {
        y_count,
        prob_by_v_y,
        entropy_by_v,
    }
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

fn build_observation_data(
    placements_by_oil: &[Vec<Placement>],
    query_mask: &Mask,
    y: i32,
    query_size: usize,
    epsilon: f64,
) -> ObservationData {
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

    ObservationData {
        hits_by_oil,
        logp_by_sum_v,
    }
}

fn update_logw_by_observation(candidates: &mut Candidates, observation: &ObservationData) {
    for candidate_id in 0..candidates.len() {
        if !candidates.logw[candidate_id].is_finite() {
            continue;
        }

        let mut sum_v = 0_usize;
        for (oil_id, hits) in observation.hits_by_oil.iter().enumerate() {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            sum_v += hits[placement_id] as usize;
        }

        candidates.logw[candidate_id] += observation.logp_by_sum_v[sum_v];
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
    let mut last_answer_mask = None;
    for candidate_id in candidate_ids {
        if turns_used >= query_limit {
            return;
        }

        let answer_mask = candidates.positive_masks[candidate_id];
        if !tried_masks.insert(answer_mask) {
            continue;
        }

        last_answer_mask = Some(answer_mask);
        let accepted = ask_answer(scanner, out, n, answer_mask);
        turns_used += 1;
        if accepted {
            return;
        }
    }

    let fallback_mask = last_answer_mask.unwrap_or_else(|| {
        let mut mask = Mask::default();
        mask.set(0);
        mask
    });
    while turns_used < query_limit {
        let accepted = ask_answer(scanner, out, n, fallback_mask);
        turns_used += 1;
        if accepted {
            return;
        }
    }
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
