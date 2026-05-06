// v024_shape_aware_swap.rs
use std::collections::{HashMap, HashSet};
use std::f64::consts::SQRT_2;
use std::io::{self, BufRead, BufWriter, Write};
use std::str::FromStr;
use std::time::Instant;

const MASK_WORDS: usize = 7;
const POSTERIOR_THRESHOLD: f64 = 0.95;
const THETA_VISIT_BUDGET: usize = 4_000_000;
const MI_POSTERIOR_MASS_CUTOFF: f64 = 0.99;
const MI_ACTIVE_CANDIDATE_LIMIT: usize = 1000;
const LATE_ANSWER_START_SEC: f64 = 2.85;
const MI_TAIL_SIGMA: f64 = 8.0;
const SINGLE_CELL_MI_TIE_EPS: f64 = 1.0e-12;
const SHAPE_NODE_LIMIT: usize = 100;
const SHAPE_MAX_UNIONS: usize = 8;
const SA_START_TEMPERATURE: f64 = 2.0;
const SA_END_TEMPERATURE: f64 = 0.10;
const SA_MAX_PROPOSAL_FACTOR: usize = 6;
const SA_NO_ACCEPT_FACTOR: usize = 3;
const SA_SEED_PER_OIL: usize = 4;
const SA_MAX_SEEDS: usize = 32;
const MOVE_NEIGHBOR_RATIO_PERCENT: usize = 30;
const RANDOM_ONE_OIL_NEIGHBOR_RATIO_PERCENT: usize = 30;
const MAX_NEIGHBOR_SHIFT: usize = 1;
const SHAPE_SWAP_OFFSET_LIMIT: usize = 8;
const SHAPE_SWAP_ALPHA: f64 = 2.0;

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

    fn contains(&self, id: usize) -> bool {
        ((self.words[id >> 6] >> (id & 63)) & 1) != 0
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

    fn is_subset_of(&self, other: &Mask) -> bool {
        for i in 0..MASK_WORDS {
            if (self.words[i] & !other.words[i]) != 0 {
                return false;
            }
        }
        true
    }

    fn for_each_set_bit<F: FnMut(usize)>(&self, total_cells: usize, mut f: F) {
        for word_id in 0..MASK_WORDS {
            let mut word = self.words[word_id];
            while word != 0 {
                let bit = word.trailing_zeros() as usize;
                let cell_id = word_id * 64 + bit;
                if cell_id < total_cells {
                    f(cell_id);
                }
                word &= word - 1;
            }
        }
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

#[derive(Clone, Copy, Debug)]
struct PlacementBounds {
    rows: usize,
    cols: usize,
}

#[derive(Clone, Copy, Debug)]
struct SwapOffsetCandidate {
    delta_i: isize,
    delta_j: isize,
    overlap: u16,
}

#[derive(Clone, Copy, Debug)]
struct ShapeSwapPair {
    oil_a: usize,
    oil_b: usize,
}

#[derive(Debug)]
struct ShapeSwapSampler {
    offsets_by_pair: Vec<Vec<Vec<SwapOffsetCandidate>>>,
    pairs: Vec<ShapeSwapPair>,
    cumulative_weights: Vec<f64>,
    total_weight: f64,
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

    fn next_f64(&mut self) -> f64 {
        const DENOMINATOR: f64 = (1_u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / DENOMINATOR
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
    let placement_bounds = if is_exhaustive {
        Vec::new()
    } else {
        build_placement_bounds(&input)
    };
    let move_neighbors = if is_exhaustive {
        Vec::new()
    } else {
        build_move_neighbors(&placement_bounds)
    };
    let shape_swap_sampler = if is_exhaustive {
        ShapeSwapSampler::empty(input.m)
    } else {
        ShapeSwapSampler::new(&input)
    };
    let (mut answer_candidates, candidate_mode) = if is_exhaustive {
        let candidates = enumerate_candidates(&input, &placements_by_oil);
        (candidates, CandidateMode::Exhaustive)
    } else {
        let candidates =
            initialize_candidate_pool(&input, &placements_by_oil, &zobrist, target_size, &mut rng);
        (candidates, CandidateMode::Sampled)
    };
    let total_cells = input.n * input.n;
    let max_sum_v = input.shape_cells.iter().map(Vec::len).sum::<usize>();
    let observation_model = ObservationModel::precompute(total_cells, input.epsilon, max_sum_v);

    eprintln!(
        "v024 mode={:?} target_size={} candidates={} m={} mi_cutoff={} mi_active_limit={} sa_temp={}->{} sa_proposals={}x sa_seeds=min({}M,{}) sa_delta_eval=true shape_swap_alpha={}",
        candidate_mode,
        target_size,
        answer_candidates.len(),
        input.m,
        MI_POSTERIOR_MASS_CUTOFF,
        MI_ACTIVE_CANDIDATE_LIMIT,
        SA_START_TEMPERATURE,
        SA_END_TEMPERATURE,
        SA_MAX_PROPOSAL_FACTOR,
        SA_SEED_PER_OIL,
        SA_MAX_SEEDS,
        SHAPE_SWAP_ALPHA
    );

    let mut observations = Vec::new();
    let mut rejected_masks = HashSet::new();
    while turns_used < query_limit {
        if started_at.elapsed().as_secs_f64() >= LATE_ANSWER_START_SEC {
            giveup_by_digging_likely_cells(
                &mut scanner,
                &mut out,
                &input,
                &placements_by_oil,
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
                if candidate_mode == CandidateMode::Sampled {
                    answer_candidates = refresh_candidate_pool(
                        &input,
                        &placements_by_oil,
                        &zobrist,
                        &placement_bounds,
                        &move_neighbors,
                        &shape_swap_sampler,
                        &observations,
                        &rejected_masks,
                        &answer_candidates,
                        target_size,
                        &mut rng,
                        started_at,
                        LATE_ANSWER_START_SEC,
                    );
                }
                continue;
            }
        } else {
            eprintln!("v024 failed: no finite posterior candidates");
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
            &answer_candidates,
            &observation_model,
            &mut rng,
            started_at,
            LATE_ANSWER_START_SEC,
        );

        if started_at.elapsed().as_secs_f64() >= LATE_ANSWER_START_SEC {
            giveup_by_digging_likely_cells(
                &mut scanner,
                &mut out,
                &input,
                &placements_by_oil,
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
        observations.push(observation);

        if candidate_mode == CandidateMode::Sampled {
            answer_candidates = refresh_candidate_pool(
                &input,
                &placements_by_oil,
                &zobrist,
                &placement_bounds,
                &move_neighbors,
                &shape_swap_sampler,
                &observations,
                &rejected_masks,
                &answer_candidates,
                target_size,
                &mut rng,
                started_at,
                LATE_ANSWER_START_SEC,
            );
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

fn build_placement_bounds(input: &Input) -> Vec<PlacementBounds> {
    let mut bounds = Vec::with_capacity(input.m);
    for oil_id in 0..input.m {
        let shape = &input.shape_cells[oil_id];
        let max_i = shape.iter().map(|&(i, _)| i).max().unwrap();
        let max_j = shape.iter().map(|&(_, j)| j).max().unwrap();
        bounds.push(PlacementBounds {
            rows: input.n - max_i,
            cols: input.n - max_j,
        });
    }
    bounds
}

fn build_move_neighbors(placement_bounds: &[PlacementBounds]) -> Vec<Vec<[Option<u16>; 4]>> {
    let mut neighbors_by_oil = Vec::with_capacity(placement_bounds.len());
    for bounds in placement_bounds {
        let rows = bounds.rows;
        let cols = bounds.cols;
        let mut neighbors = vec![[None; 4]; rows * cols];

        for deltai_k in 0..rows {
            for deltaj_k in 0..cols {
                let placement_id = deltai_k * cols + deltaj_k;
                if deltai_k >= MAX_NEIGHBOR_SHIFT {
                    neighbors[placement_id][0] =
                        Some(((deltai_k - MAX_NEIGHBOR_SHIFT) * cols + deltaj_k) as u16);
                }
                if deltai_k + MAX_NEIGHBOR_SHIFT < rows {
                    neighbors[placement_id][1] =
                        Some(((deltai_k + MAX_NEIGHBOR_SHIFT) * cols + deltaj_k) as u16);
                }
                if deltaj_k >= MAX_NEIGHBOR_SHIFT {
                    neighbors[placement_id][2] =
                        Some((deltai_k * cols + (deltaj_k - MAX_NEIGHBOR_SHIFT)) as u16);
                }
                if deltaj_k + MAX_NEIGHBOR_SHIFT < cols {
                    neighbors[placement_id][3] =
                        Some((deltai_k * cols + (deltaj_k + MAX_NEIGHBOR_SHIFT)) as u16);
                }
            }
        }
        neighbors_by_oil.push(neighbors);
    }
    neighbors_by_oil
}

impl ShapeSwapSampler {
    fn empty(m: usize) -> Self {
        Self {
            offsets_by_pair: vec![vec![Vec::new(); m]; m],
            pairs: Vec::new(),
            cumulative_weights: Vec::new(),
            total_weight: 0.0,
        }
    }

    fn new(input: &Input) -> Self {
        let m = input.m;
        let mut offsets_by_pair = vec![vec![Vec::new(); m]; m];
        let mut pairs = Vec::new();
        let mut cumulative_weights = Vec::new();
        let mut total_weight = 0.0;

        for oil_a in 0..m {
            for oil_b in 0..m {
                if oil_a == oil_b {
                    continue;
                }
                offsets_by_pair[oil_a][oil_b] = build_swap_offset_candidates(
                    &input.shape_cells[oil_a],
                    &input.shape_cells[oil_b],
                );
            }
        }

        for oil_a in 0..m {
            for oil_b in oil_a + 1..m {
                let best_overlap = offsets_by_pair[oil_a][oil_b]
                    .first()
                    .map_or(0, |candidate| candidate.overlap)
                    as f64;
                let denominator = input.shape_cells[oil_a]
                    .len()
                    .min(input.shape_cells[oil_b].len()) as f64;
                if denominator <= 0.0 || best_overlap <= 0.0 {
                    continue;
                }

                let similarity = best_overlap / denominator;
                let weight = similarity.powf(SHAPE_SWAP_ALPHA);
                if weight <= 0.0 {
                    continue;
                }

                pairs.push(ShapeSwapPair { oil_a, oil_b });
                total_weight += weight;
                cumulative_weights.push(total_weight);
            }
        }

        Self {
            offsets_by_pair,
            pairs,
            cumulative_weights,
            total_weight,
        }
    }

    fn sample_pair(&self, rng: &mut XorShift64) -> Option<ShapeSwapPair> {
        if self.pairs.is_empty() || self.total_weight <= 0.0 {
            return None;
        }

        let key = rng.next_f64() * self.total_weight;
        let index = self
            .cumulative_weights
            .partition_point(|&cumulative_weight| cumulative_weight <= key)
            .min(self.pairs.len() - 1);
        Some(self.pairs[index])
    }

    fn sample_offset(
        &self,
        oil_from: usize,
        oil_to: usize,
        rng: &mut XorShift64,
    ) -> Option<SwapOffsetCandidate> {
        let offsets = &self.offsets_by_pair[oil_from][oil_to];
        if offsets.is_empty() {
            None
        } else {
            Some(offsets[rng.next_usize(offsets.len())])
        }
    }
}

fn build_swap_offset_candidates(
    shape_from: &[(usize, usize)],
    shape_to: &[(usize, usize)],
) -> Vec<SwapOffsetCandidate> {
    let mut counts: HashMap<(isize, isize), u16> = HashMap::new();
    for &(from_i, from_j) in shape_from {
        for &(to_i, to_j) in shape_to {
            let delta_i = from_i as isize - to_i as isize;
            let delta_j = from_j as isize - to_j as isize;
            *counts.entry((delta_i, delta_j)).or_insert(0) += 1;
        }
    }

    let mut candidates = counts
        .into_iter()
        .map(|((delta_i, delta_j), overlap)| SwapOffsetCandidate {
            delta_i,
            delta_j,
            overlap,
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|a, b| b.overlap.cmp(&a.overlap));
    candidates.truncate(SHAPE_SWAP_OFFSET_LIMIT);
    candidates
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

fn refresh_candidate_pool(
    input: &Input,
    placements_by_oil: &[Vec<Placement>],
    zobrist: &[Vec<u64>],
    placement_bounds: &[PlacementBounds],
    move_neighbors: &[Vec<[Option<u16>; 4]>],
    shape_swap_sampler: &ShapeSwapSampler,
    observations: &[ObservationData],
    rejected_masks: &HashSet<Mask>,
    current: &Candidates,
    target_size: usize,
    rng: &mut XorShift64,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> Candidates {
    let mut candidate_ids: Vec<usize> = (0..current.len())
        .filter(|&candidate_id| current.logw[candidate_id].is_finite())
        .collect();
    candidate_ids.sort_unstable_by(|&a, &b| current.logw[b].total_cmp(&current.logw[a]));

    let mut next_answer = empty_candidates(input.m, target_size);
    let mut seen_answer = HashSet::with_capacity(target_size * 2);

    if candidate_ids.is_empty() {
        return next_answer;
    }

    let seed_count = (input.m * SA_SEED_PER_OIL)
        .clamp(1, SA_MAX_SEEDS)
        .min(candidate_ids.len())
        .min(target_size);
    let seed_ids = &candidate_ids[..seed_count];
    for &seed_id in seed_ids {
        push_candidate_if_new(
            &mut next_answer,
            placements_by_oil,
            zobrist,
            &mut seen_answer,
            current.placement_indices_for(seed_id),
            current.logw[seed_id],
            Some(rejected_masks),
        );
    }

    let mut proposal_indices = vec![0_u16; input.m];
    let max_proposals = (target_size * SA_MAX_PROPOSAL_FACTOR).max(target_size);
    let proposals_per_seed = (max_proposals / seed_count).max(1);
    let no_accept_limit = (proposals_per_seed * SA_NO_ACCEPT_FACTOR).max(256);
    let mut global_step = 0_usize;

    for &seed_id in seed_ids {
        if next_answer.len() >= target_size
            || global_step >= max_proposals
            || started_at.elapsed().as_secs_f64() >= late_answer_start_sec
        {
            break;
        }

        let mut current_state = SaState::new(
            current.placement_indices_for(seed_id).to_vec(),
            current.logw[seed_id],
            observations,
        );
        let mut proposal_sum_v_by_observation = vec![0_u16; observations.len()];
        let mut changed_oils = Vec::with_capacity(input.m);
        let mut steps_since_accept = 0_usize;

        for _ in 0..proposals_per_seed {
            if next_answer.len() >= target_size
                || global_step >= max_proposals
                || steps_since_accept >= no_accept_limit
                || started_at.elapsed().as_secs_f64() >= late_answer_start_sec
            {
                break;
            }

            let step = global_step;
            global_step += 1;

            if !fill_sa_neighbor_candidate_indices(
                &mut proposal_indices,
                &current_state.indices,
                &mut changed_oils,
                placements_by_oil,
                placement_bounds,
                move_neighbors,
                shape_swap_sampler,
                rng,
            ) {
                steps_since_accept += 1;
                continue;
            }

            let proposal_logw = compute_neighbor_logw_delta(
                &current_state,
                &proposal_indices,
                &changed_oils,
                observations,
                &mut proposal_sum_v_by_observation,
            );
            if !proposal_logw.is_finite() {
                steps_since_accept += 1;
                continue;
            }

            let temperature = sa_temperature(step, max_proposals);
            let delta = proposal_logw - current_state.logw;
            if delta >= 0.0 || rng.next_f64() < (delta / temperature).exp() {
                current_state.indices.copy_from_slice(&proposal_indices);
                current_state.logw = proposal_logw;
                std::mem::swap(
                    &mut current_state.sum_v_by_observation,
                    &mut proposal_sum_v_by_observation,
                );
                push_candidate_if_new(
                    &mut next_answer,
                    placements_by_oil,
                    zobrist,
                    &mut seen_answer,
                    &current_state.indices,
                    current_state.logw,
                    Some(rejected_masks),
                );
                steps_since_accept = 0;
            } else {
                steps_since_accept += 1;
            }
        }
    }

    next_answer
}

fn sa_temperature(step: usize, max_steps: usize) -> f64 {
    if max_steps <= 1 {
        return SA_END_TEMPERATURE;
    }
    let progress = (step as f64 / (max_steps - 1) as f64).clamp(0.0, 1.0);
    SA_START_TEMPERATURE * (SA_END_TEMPERATURE / SA_START_TEMPERATURE).powf(progress)
}

struct SaState {
    indices: Vec<u16>,
    logw: f64,
    sum_v_by_observation: Vec<u16>,
}

impl SaState {
    fn new(indices: Vec<u16>, logw: f64, observations: &[ObservationData]) -> Self {
        let sum_v_by_observation = compute_sum_v_by_observation_for_indices(&indices, observations);
        Self {
            indices,
            logw,
            sum_v_by_observation,
        }
    }
}

fn compute_sum_v_by_observation_for_indices(
    indices: &[u16],
    observations: &[ObservationData],
) -> Vec<u16> {
    let mut sum_v_by_observation = Vec::with_capacity(observations.len());
    for observation in observations {
        let mut sum_v = 0_u16;
        for (oil_id, hits) in observation.hits_by_oil.iter().enumerate() {
            sum_v += hits[indices[oil_id] as usize];
        }
        sum_v_by_observation.push(sum_v);
    }
    sum_v_by_observation
}

fn compute_neighbor_logw_delta(
    current: &SaState,
    proposal_indices: &[u16],
    changed_oils: &[usize],
    observations: &[ObservationData],
    proposal_sum_v_by_observation: &mut [u16],
) -> f64 {
    let mut proposal_logw = current.logw;

    for (observation_id, observation) in observations.iter().enumerate() {
        let old_sum_v = current.sum_v_by_observation[observation_id] as usize;
        let mut next_sum_v = old_sum_v as i32;

        for &oil_id in changed_oils {
            let old_placement_id = current.indices[oil_id] as usize;
            let next_placement_id = proposal_indices[oil_id] as usize;
            let hits = &observation.hits_by_oil[oil_id];
            next_sum_v += hits[next_placement_id] as i32 - hits[old_placement_id] as i32;
        }

        if next_sum_v < 0 {
            return f64::NEG_INFINITY;
        }

        let next_sum_v = next_sum_v as usize;
        if next_sum_v >= observation.logp_by_sum_v.len() {
            return f64::NEG_INFINITY;
        }

        proposal_logw += observation.logp_by_sum_v[next_sum_v];
        proposal_logw -= observation.logp_by_sum_v[old_sum_v];
        proposal_sum_v_by_observation[observation_id] = next_sum_v as u16;
    }

    proposal_logw
}

fn fill_sa_neighbor_candidate_indices(
    indices: &mut [u16],
    parent_indices: &[u16],
    changed_oils: &mut Vec<usize>,
    placements_by_oil: &[Vec<Placement>],
    placement_bounds: &[PlacementBounds],
    move_neighbors: &[Vec<[Option<u16>; 4]>],
    shape_swap_sampler: &ShapeSwapSampler,
    rng: &mut XorShift64,
) -> bool {
    changed_oils.clear();
    let roll = rng.next_usize(100);
    if roll < MOVE_NEIGHBOR_RATIO_PERCENT {
        fill_move_neighbor_candidate_indices(
            indices,
            parent_indices,
            changed_oils,
            move_neighbors,
            rng,
        )
    } else if roll < MOVE_NEIGHBOR_RATIO_PERCENT + RANDOM_ONE_OIL_NEIGHBOR_RATIO_PERCENT {
        fill_random_one_oil_neighbor_candidate_indices(
            indices,
            parent_indices,
            changed_oils,
            placements_by_oil,
            rng,
        )
    } else if indices.len() >= 2 {
        fill_swap_neighbor_candidate_indices(
            indices,
            parent_indices,
            changed_oils,
            placement_bounds,
            shape_swap_sampler,
            rng,
        )
    } else {
        false
    }
}

fn fill_move_neighbor_candidate_indices(
    indices: &mut [u16],
    parent_indices: &[u16],
    changed_oils: &mut Vec<usize>,
    move_neighbors: &[Vec<[Option<u16>; 4]>],
    rng: &mut XorShift64,
) -> bool {
    indices.copy_from_slice(parent_indices);
    let mut moved_any = false;

    for oil_id in 0..indices.len() {
        if rng.next_usize(2) == 0 {
            continue;
        }

        moved_any = true;
        let placement_id = parent_indices[oil_id] as usize;
        let dir = rng.next_usize(4);
        if let Some(next_placement_id) = move_neighbors[oil_id][placement_id][dir] {
            indices[oil_id] = next_placement_id;
            changed_oils.push(oil_id);
        } else {
            return false;
        }
    }

    moved_any
}

fn fill_random_one_oil_neighbor_candidate_indices(
    indices: &mut [u16],
    parent_indices: &[u16],
    changed_oils: &mut Vec<usize>,
    placements_by_oil: &[Vec<Placement>],
    rng: &mut XorShift64,
) -> bool {
    indices.copy_from_slice(parent_indices);
    let oil_id = rng.next_usize(indices.len());
    indices[oil_id] = rng.next_usize(placements_by_oil[oil_id].len()) as u16;
    if indices[oil_id] != parent_indices[oil_id] {
        changed_oils.push(oil_id);
        true
    } else {
        false
    }
}

fn fill_swap_neighbor_candidate_indices(
    indices: &mut [u16],
    parent_indices: &[u16],
    changed_oils: &mut Vec<usize>,
    placement_bounds: &[PlacementBounds],
    shape_swap_sampler: &ShapeSwapSampler,
    rng: &mut XorShift64,
) -> bool {
    indices.copy_from_slice(parent_indices);

    let Some(pair) = shape_swap_sampler.sample_pair(rng) else {
        return false;
    };
    let oil_a = pair.oil_a;
    let oil_b = pair.oil_b;

    let (deltai_a, deltaj_a) =
        placement_id_to_coord(parent_indices[oil_a], placement_bounds[oil_a]);
    let (deltai_b, deltaj_b) =
        placement_id_to_coord(parent_indices[oil_b], placement_bounds[oil_b]);
    let Some(offset_ab) = shape_swap_sampler.sample_offset(oil_a, oil_b, rng) else {
        return false;
    };
    let Some(offset_ba) = shape_swap_sampler.sample_offset(oil_b, oil_a, rng) else {
        return false;
    };

    let next_a = clamp_signed_coord_to_placement_id(
        deltai_b as isize + offset_ba.delta_i,
        deltaj_b as isize + offset_ba.delta_j,
        placement_bounds[oil_a],
    );
    let next_b = clamp_signed_coord_to_placement_id(
        deltai_a as isize + offset_ab.delta_i,
        deltaj_a as isize + offset_ab.delta_j,
        placement_bounds[oil_b],
    );

    indices[oil_a] = next_a;
    indices[oil_b] = next_b;

    if indices[oil_a] != parent_indices[oil_a] {
        changed_oils.push(oil_a);
    }
    if indices[oil_b] != parent_indices[oil_b] {
        changed_oils.push(oil_b);
    }

    !changed_oils.is_empty()
}

fn placement_id_to_coord(placement_id: u16, bounds: PlacementBounds) -> (usize, usize) {
    let placement_id = placement_id as usize;
    (placement_id / bounds.cols, placement_id % bounds.cols)
}

fn clamp_signed_coord_to_placement_id(
    deltai_k: isize,
    deltaj_k: isize,
    bounds: PlacementBounds,
) -> u16 {
    let clamped_i = deltai_k.clamp(0, bounds.rows as isize - 1) as usize;
    let clamped_j = deltaj_k.clamp(0, bounds.cols as isize - 1) as usize;
    (clamped_i * bounds.cols + clamped_j) as u16
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

fn precompute_active_cell_values(
    n: usize,
    posterior: &Posterior,
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
) -> CellValues {
    let total_cells = n * n;
    let candidate_count = posterior.active_ids.len();
    let mut values_by_cell = vec![0_u8; total_cells * candidate_count];

    for (active_index, &candidate_id) in posterior.active_ids.iter().enumerate() {
        for oil_id in 0..candidates.m {
            let placement_id = candidates.placement_index(candidate_id, oil_id);
            for &cell_id in &placements_by_oil[oil_id][placement_id].cell_ids {
                values_by_cell[cell_id * candidate_count + active_index] += 1;
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
    rng: &mut XorShift64,
    started_at: Instant,
    late_answer_start_sec: f64,
) -> Mask {
    let total_cells = input.n * input.n;
    let posterior = build_mi_posterior(
        candidates,
        MI_POSTERIOR_MASS_CUTOFF,
        Some(MI_ACTIVE_CANDIDATE_LIMIT),
    )
    .expect("finite posterior must exist");
    let initial_query_size = (total_cells / 2).max(2).min(total_cells);

    if started_at.elapsed().as_secs_f64() >= late_answer_start_sec {
        return sample_query_mask(input.n, initial_query_size, rng);
    }

    let cell_values =
        precompute_active_cell_values(input.n, &posterior, candidates, placements_by_oil);

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
            &posterior.weights,
            &cell_values,
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
            &posterior.weights,
            &cell_values,
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

fn build_mi_posterior(
    candidates: &Candidates,
    mass_cutoff: f64,
    active_limit: Option<usize>,
) -> Option<Posterior> {
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

    let active_limit = active_limit.unwrap_or(usize::MAX).max(1);
    for (candidate_id, weight) in entries {
        if kept_total_weight >= target_mass && !active_ids.is_empty() {
            break;
        }
        if active_ids.len() >= active_limit {
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
    weights: &[f64],
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
            weights,
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
    weights: &[f64],
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
            weights,
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
    weights: &[f64],
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
        weights,
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
    weights: &[f64],
    cell_values: &CellValues,
) {
    next_mass_by_v.copy_from_slice(current_mass_by_v);

    match query_move {
        QueryMove::Add { cell } => {
            let values = cell_values.values_for_cell(cell);
            for index in 0..current_sum_v_by_active.len() {
                let old_sum_v = current_sum_v_by_active[index] as usize;
                let delta = values[index] as usize;
                let next_sum_v = old_sum_v + delta;
                next_sum_v_by_active[index] = next_sum_v as u16;
                if delta != 0 {
                    let weight = weights[index];
                    next_mass_by_v[old_sum_v] -= weight;
                    next_mass_by_v[next_sum_v] += weight;
                }
            }
        }
        QueryMove::Remove { cell } => {
            let values = cell_values.values_for_cell(cell);
            for index in 0..current_sum_v_by_active.len() {
                let old_sum_v = current_sum_v_by_active[index] as usize;
                let delta = values[index] as usize;
                let next_sum_v = old_sum_v - delta;
                next_sum_v_by_active[index] = next_sum_v as u16;
                if delta != 0 {
                    let weight = weights[index];
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

fn ask_dig<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    n: usize,
    cell_id: usize,
) -> i32 {
    writeln!(out, "q 1 {} {}", cell_id / n, cell_id % n).unwrap();
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

fn giveup_by_digging_likely_cells<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    input: &Input,
    placements_by_oil: &[Vec<Placement>],
    candidates: &Candidates,
    mut turns_used: usize,
    query_limit: usize,
) {
    let total_cells = input.n * input.n;
    let (positive_prob_by_cell, cell_order) = positive_probability_cell_scores(input.n, candidates);
    let mut order_pos = 0_usize;
    let mut dug = vec![false; total_cells];
    let mut dug_count = 0_usize;
    let mut known_positive_mask = Mask::default();
    let mut known_zero_mask = Mask::default();
    let mut exact_value_by_cell = vec![-1_i8; total_cells];
    let mut alive: Vec<bool> = candidates
        .logw
        .iter()
        .map(|logw| logw.is_finite())
        .collect();
    let mut rejected_masks = HashSet::new();

    while turns_used < query_limit {
        if turns_used + 1 == query_limit {
            let final_mask = if dug_count == total_cells {
                Some(known_positive_mask)
            } else {
                giveup_final_answer_mask(
                    candidates,
                    &alive,
                    &known_positive_mask,
                    &known_zero_mask,
                    &rejected_masks,
                )
            };
            if let Some(answer_mask) = final_mask {
                let _ = ask_answer(scanner, out, input.n, answer_mask);
            }
            return;
        }

        if alive.iter().any(|&is_alive| is_alive) {
            if let Some(answer_mask) = unique_alive_positive_mask(candidates, &alive) {
                let accepted = ask_answer(scanner, out, input.n, answer_mask);
                turns_used += 1;
                if accepted {
                    return;
                }
                rejected_masks.insert(answer_mask);
                alive.fill(false);
                continue;
            }
        } else if known_positive_mask.count_ones() > 0 {
            match bounded_shape_resolve(
                &known_positive_mask,
                &known_zero_mask,
                &exact_value_by_cell,
                placements_by_oil,
                SHAPE_NODE_LIMIT,
                SHAPE_MAX_UNIONS,
            ) {
                ShapeResolve::Unique(mask) => {
                    if rejected_masks.contains(&mask) {
                        continue;
                    }
                    let accepted = ask_answer(scanner, out, input.n, mask);
                    turns_used += 1;
                    if accepted {
                        return;
                    }
                    rejected_masks.insert(mask);
                    continue;
                }
                ShapeResolve::Ambiguous(unions) => {
                    if let Some(cell_id) =
                        choose_disagreement_cell(&unions, &dug, total_cells, &positive_prob_by_cell)
                    {
                        dig_one_cell_and_update(
                            scanner,
                            out,
                            input.n,
                            cell_id,
                            &mut turns_used,
                            &mut dug,
                            &mut dug_count,
                            &mut known_positive_mask,
                            &mut known_zero_mask,
                            &mut exact_value_by_cell,
                            candidates,
                            placements_by_oil,
                            &mut alive,
                        );
                        continue;
                    }
                }
                ShapeResolve::Unknown => {}
            }
        }

        if dug_count == total_cells {
            let accepted = ask_answer(scanner, out, input.n, known_positive_mask);
            turns_used += 1;
            if accepted {
                return;
            }
            rejected_masks.insert(known_positive_mask);
            continue;
        }

        let Some(cell_id) = next_undug_cell(&cell_order, &dug, &mut order_pos) else {
            return;
        };
        dig_one_cell_and_update(
            scanner,
            out,
            input.n,
            cell_id,
            &mut turns_used,
            &mut dug,
            &mut dug_count,
            &mut known_positive_mask,
            &mut known_zero_mask,
            &mut exact_value_by_cell,
            candidates,
            placements_by_oil,
            &mut alive,
        );
    }
}

fn positive_probability_cell_scores(n: usize, candidates: &Candidates) -> (Vec<f64>, Vec<usize>) {
    let total_cells = n * n;
    let mut scores = vec![0.0_f64; total_cells];

    if let Some(posterior) = build_mi_posterior(candidates, MI_POSTERIOR_MASS_CUTOFF, None) {
        for (&candidate_id, &weight) in posterior.active_ids.iter().zip(&posterior.weights) {
            candidates.positive_masks[candidate_id]
                .for_each_set_bit(total_cells, |cell_id| scores[cell_id] += weight);
        }
    }

    let mut cell_order: Vec<usize> = (0..total_cells).collect();
    cell_order.sort_unstable_by(|&a, &b| scores[b].total_cmp(&scores[a]).then_with(|| a.cmp(&b)));
    (scores, cell_order)
}

fn dig_one_cell_and_update<R: BufRead, W: Write>(
    scanner: &mut Scanner<R>,
    out: &mut W,
    n: usize,
    cell_id: usize,
    turns_used: &mut usize,
    dug: &mut [bool],
    dug_count: &mut usize,
    known_positive_mask: &mut Mask,
    known_zero_mask: &mut Mask,
    exact_value_by_cell: &mut [i8],
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
    alive: &mut [bool],
) {
    let value = ask_dig(scanner, out, n, cell_id);
    *turns_used += 1;
    if !dug[cell_id] {
        dug[cell_id] = true;
        *dug_count += 1;
    }
    exact_value_by_cell[cell_id] = value as i8;
    if value > 0 {
        known_positive_mask.set(cell_id);
    } else {
        known_zero_mask.set(cell_id);
    }

    if alive.iter().any(|&is_alive| is_alive) {
        filter_alive_by_exact_dig(candidates, placements_by_oil, alive, cell_id, value);
    }
}

fn next_undug_cell(cell_order: &[usize], dug: &[bool], order_pos: &mut usize) -> Option<usize> {
    while *order_pos < cell_order.len() {
        let cell_id = cell_order[*order_pos];
        *order_pos += 1;
        if !dug[cell_id] {
            return Some(cell_id);
        }
    }
    dug.iter().position(|&is_dug| !is_dug)
}

#[derive(Debug)]
enum ShapeResolve {
    Unique(Mask),
    Ambiguous(Vec<Mask>),
    Unknown,
}

struct ShapeOilChoices {
    oil_id: usize,
    placements: Vec<ShapePlacementChoice>,
}

struct ShapePlacementChoice {
    placement_id: usize,
    observed_hits: Vec<usize>,
}

fn bounded_shape_resolve(
    known_positive_mask: &Mask,
    known_zero_mask: &Mask,
    exact_value_by_cell: &[i8],
    placements_by_oil: &[Vec<Placement>],
    node_limit: usize,
    max_unions: usize,
) -> ShapeResolve {
    let mut oils = Vec::with_capacity(placements_by_oil.len());
    let mut observed_cells = Vec::new();
    let mut observed_targets = Vec::new();

    for (cell_id, &value) in exact_value_by_cell.iter().enumerate() {
        if value > 0 {
            observed_cells.push(cell_id);
            observed_targets.push(value as u8);
        }
    }

    for (oil_id, placements) in placements_by_oil.iter().enumerate() {
        let mut choices = Vec::new();
        for (placement_id, placement) in placements.iter().enumerate() {
            if placement.placed_mask.and_count(known_zero_mask) == 0 {
                let mut observed_hits = Vec::new();
                for (observed_index, &cell_id) in observed_cells.iter().enumerate() {
                    if placement.placed_mask.contains(cell_id) {
                        observed_hits.push(observed_index);
                    }
                }
                choices.push(ShapePlacementChoice {
                    placement_id,
                    observed_hits,
                });
            }
        }

        if choices.is_empty() {
            return ShapeResolve::Unknown;
        }

        choices.sort_unstable_by(|a, b| {
            let cover_a = placements[a.placement_id]
                .placed_mask
                .and_count(known_positive_mask);
            let cover_b = placements[b.placement_id]
                .placed_mask
                .and_count(known_positive_mask);
            cover_b
                .cmp(&cover_a)
                .then_with(|| b.observed_hits.len().cmp(&a.observed_hits.len()))
        });

        oils.push(ShapeOilChoices {
            oil_id,
            placements: choices,
        });
    }

    oils.sort_unstable_by(|a, b| a.placements.len().cmp(&b.placements.len()));

    let mut remaining_coverable = vec![Mask::default(); oils.len() + 1];
    let mut remaining_observed_cover = vec![vec![0_u8; observed_cells.len()]; oils.len() + 1];
    for depth in (0..oils.len()).rev() {
        let mut coverable = remaining_coverable[depth + 1];
        let mut can_cover_observed = vec![0_u8; observed_cells.len()];
        let oil = &oils[depth];
        for choice in &oil.placements {
            coverable.or_assign(&placements_by_oil[oil.oil_id][choice.placement_id].placed_mask);
            for &observed_index in &choice.observed_hits {
                can_cover_observed[observed_index] = 1;
            }
        }
        remaining_coverable[depth] = coverable;
        for observed_index in 0..observed_cells.len() {
            remaining_observed_cover[depth][observed_index] = remaining_observed_cover[depth + 1]
                [observed_index]
                + can_cover_observed[observed_index];
        }
    }

    let mut unions = Vec::new();
    let mut nodes = 0_usize;
    let mut hit_limit = false;
    let mut observed_counts = vec![0_u8; observed_cells.len()];
    dfs_shape_resolve(
        0,
        Mask::default(),
        &oils,
        &remaining_coverable,
        &remaining_observed_cover,
        &observed_targets,
        &mut observed_counts,
        known_positive_mask,
        placements_by_oil,
        node_limit,
        max_unions,
        &mut nodes,
        &mut hit_limit,
        &mut unions,
    );

    if unions.len() >= 2 {
        ShapeResolve::Ambiguous(unions)
    } else if hit_limit {
        ShapeResolve::Unknown
    } else if unions.len() == 1 {
        ShapeResolve::Unique(unions[0])
    } else {
        ShapeResolve::Unknown
    }
}

fn dfs_shape_resolve(
    depth: usize,
    current_union: Mask,
    oils: &[ShapeOilChoices],
    remaining_coverable: &[Mask],
    remaining_observed_cover: &[Vec<u8>],
    observed_targets: &[u8],
    observed_counts: &mut [u8],
    known_positive_mask: &Mask,
    placements_by_oil: &[Vec<Placement>],
    node_limit: usize,
    max_unions: usize,
    nodes: &mut usize,
    hit_limit: &mut bool,
    unions: &mut Vec<Mask>,
) {
    if *hit_limit || unions.len() >= max_unions {
        return;
    }

    *nodes += 1;
    if *nodes > node_limit {
        *hit_limit = true;
        return;
    }

    for observed_index in 0..observed_targets.len() {
        let current_count = observed_counts[observed_index];
        let target = observed_targets[observed_index];
        if current_count > target {
            return;
        }
        if current_count + remaining_observed_cover[depth][observed_index] < target {
            return;
        }
    }

    let mut possible_union = current_union;
    possible_union.or_assign(&remaining_coverable[depth]);
    if !known_positive_mask.is_subset_of(&possible_union) {
        return;
    }

    if depth == oils.len() {
        for observed_index in 0..observed_targets.len() {
            if observed_counts[observed_index] != observed_targets[observed_index] {
                return;
            }
        }
        if known_positive_mask.is_subset_of(&current_union)
            && !unions.iter().any(|&mask| mask == current_union)
        {
            unions.push(current_union);
        }
        return;
    }

    let oil = &oils[depth];
    for choice in &oil.placements {
        if *hit_limit || unions.len() >= max_unions {
            return;
        }

        let mut next_union = current_union;
        next_union.or_assign(&placements_by_oil[oil.oil_id][choice.placement_id].placed_mask);

        let mut exceeded_target = false;
        for &observed_index in &choice.observed_hits {
            observed_counts[observed_index] += 1;
            if observed_counts[observed_index] > observed_targets[observed_index] {
                exceeded_target = true;
            }
        }

        if !exceeded_target {
            dfs_shape_resolve(
                depth + 1,
                next_union,
                oils,
                remaining_coverable,
                remaining_observed_cover,
                observed_targets,
                observed_counts,
                known_positive_mask,
                placements_by_oil,
                node_limit,
                max_unions,
                nodes,
                hit_limit,
                unions,
            );
        }

        for &observed_index in &choice.observed_hits {
            observed_counts[observed_index] -= 1;
        }
    }
}

fn choose_disagreement_cell(
    unions: &[Mask],
    dug: &[bool],
    total_cells: usize,
    positive_prob_by_cell: &[f64],
) -> Option<usize> {
    let mut best_cell = None;
    let mut best_split_score = 0_usize;
    let mut best_probability = -1.0;

    for cell_id in 0..total_cells {
        if dug[cell_id] {
            continue;
        }

        let positive_count = unions
            .iter()
            .filter(|union_mask| union_mask.contains(cell_id))
            .count();
        if positive_count == 0 || positive_count == unions.len() {
            continue;
        }

        let split_score = positive_count * (unions.len() - positive_count);
        let probability = positive_prob_by_cell[cell_id];
        if split_score > best_split_score
            || (split_score == best_split_score && probability > best_probability)
        {
            best_split_score = split_score;
            best_probability = probability;
            best_cell = Some(cell_id);
        }
    }

    best_cell
}

fn filter_alive_by_exact_dig(
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
    alive: &mut [bool],
    cell_id: usize,
    value: i32,
) {
    for candidate_id in 0..candidates.len() {
        if !alive[candidate_id] {
            continue;
        }
        if candidate_cell_value(candidates, placements_by_oil, candidate_id, cell_id) as i32
            != value
        {
            alive[candidate_id] = false;
        }
    }
}

fn candidate_cell_value(
    candidates: &Candidates,
    placements_by_oil: &[Vec<Placement>],
    candidate_id: usize,
    cell_id: usize,
) -> u8 {
    let mut value = 0_u8;
    for oil_id in 0..candidates.m {
        let placement_id = candidates.placement_index(candidate_id, oil_id);
        if placements_by_oil[oil_id][placement_id]
            .placed_mask
            .contains(cell_id)
        {
            value += 1;
        }
    }
    value
}

fn unique_alive_positive_mask(candidates: &Candidates, alive: &[bool]) -> Option<Mask> {
    let mut unique_mask = None;
    for candidate_id in 0..candidates.len() {
        if !alive[candidate_id] {
            continue;
        }
        let mask = candidates.positive_masks[candidate_id];
        if let Some(first_mask) = unique_mask {
            if first_mask != mask {
                return None;
            }
        } else {
            unique_mask = Some(mask);
        }
    }
    unique_mask
}

fn giveup_final_answer_mask(
    candidates: &Candidates,
    alive: &[bool],
    known_positive_mask: &Mask,
    known_zero_mask: &Mask,
    rejected_masks: &HashSet<Mask>,
) -> Option<Mask> {
    best_alive_mask(candidates, alive, rejected_masks)
        .or_else(|| best_finite_mask(candidates, rejected_masks))
        .map(|mut mask| {
            mask.or_assign(known_positive_mask);
            for cell_id in 0..MASK_WORDS * 64 {
                if known_zero_mask.contains(cell_id) {
                    mask.clear(cell_id);
                }
            }
            mask
        })
        .or_else(|| {
            if known_positive_mask.count_ones() > 0 {
                Some(*known_positive_mask)
            } else {
                let mut mask = Mask::default();
                mask.set(0);
                Some(mask)
            }
        })
}

fn best_alive_mask(
    candidates: &Candidates,
    alive: &[bool],
    rejected_masks: &HashSet<Mask>,
) -> Option<Mask> {
    let mut best_logw = f64::NEG_INFINITY;
    let mut best_mask = None;
    for candidate_id in 0..candidates.len() {
        if !alive[candidate_id] {
            continue;
        }
        let mask = candidates.positive_masks[candidate_id];
        if rejected_masks.contains(&mask) {
            continue;
        }
        let logw = candidates.logw[candidate_id];
        if logw.is_finite() && logw > best_logw {
            best_logw = logw;
            best_mask = Some(mask);
        }
    }
    best_mask
}

fn best_finite_mask(candidates: &Candidates, rejected_masks: &HashSet<Mask>) -> Option<Mask> {
    let mut best_logw = f64::NEG_INFINITY;
    let mut best_mask = None;
    for candidate_id in 0..candidates.len() {
        let mask = candidates.positive_masks[candidate_id];
        if rejected_masks.contains(&mask) {
            continue;
        }
        let logw = candidates.logw[candidate_id];
        if logw.is_finite() && logw > best_logw {
            best_logw = logw;
            best_mask = Some(mask);
        }
    }
    best_mask
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
