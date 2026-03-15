use proconio::input;
use std::time::{Duration, Instant};

const TIME_LIMIT: f64 = 1.93;
const PHASES: usize = 10;
const MAX_DP_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;
const RANK_JUMP_THRESHOLD: usize = 28;
const BASE_EJECTION_DEPTH: usize = 5;
const REPAIR_WINDOW_MIN: usize = 80;
const REPAIR_WINDOW_MAX: usize = 220;

const BAND_H: usize = 5;
const BLOCK_W: usize = 2;
const BLOCK_CELLS: usize = BAND_H * BLOCK_W;

type LocalPath = [u8; BLOCK_CELLS];

#[derive(Clone, Copy)]
struct Symmetry {
    transpose: bool,
    flip_row: bool,
    flip_col: bool,
}

#[derive(Clone)]
struct BlockEval {
    scores: [i64; BAND_H * BAND_H],
    choices: [u16; BAND_H * BAND_H],
}

const SYMMETRIES: [Symmetry; 8] = [
    Symmetry {
        transpose: false,
        flip_row: false,
        flip_col: false,
    },
    Symmetry {
        transpose: false,
        flip_row: true,
        flip_col: false,
    },
    Symmetry {
        transpose: false,
        flip_row: false,
        flip_col: true,
    },
    Symmetry {
        transpose: false,
        flip_row: true,
        flip_col: true,
    },
    Symmetry {
        transpose: true,
        flip_row: false,
        flip_col: false,
    },
    Symmetry {
        transpose: true,
        flip_row: true,
        flip_col: false,
    },
    Symmetry {
        transpose: true,
        flip_row: false,
        flip_col: true,
    },
    Symmetry {
        transpose: true,
        flip_row: true,
        flip_col: true,
    },
];

#[derive(Clone)]
struct State {
    n: usize,
    m: usize,
    path: Vec<usize>,
    pos: Vec<usize>,
    raw_score: i64,
    bad: Vec<u8>,
    viol_cnt: i32,
    breakpoints: Vec<usize>,
    bp_index: Vec<usize>,
    rank_bad: i32,
    risk: i32,
}

impl State {
    fn new(n: usize, path: Vec<usize>, weights: &[i64], rank_by_cell: &[usize]) -> Self {
        let m = path.len();
        let mut state = Self {
            n,
            m,
            path,
            pos: vec![0; m],
            raw_score: 0,
            bad: vec![0; m.saturating_sub(1)],
            viol_cnt: 0,
            breakpoints: Vec::with_capacity(m / 2),
            bp_index: vec![usize::MAX; m.saturating_sub(1)],
            rank_bad: 0,
            risk: 0,
        };
        state.rebuild_metrics(weights, rank_by_cell);
        state
    }

    fn rebuild_metrics(&mut self, weights: &[i64], rank_by_cell: &[usize]) {
        self.raw_score = 0;
        for (idx, &cell) in self.path.iter().enumerate() {
            self.pos[cell] = idx;
            self.raw_score += idx as i64 * weights[cell];
        }

        self.breakpoints.clear();
        self.rank_bad = 0;
        self.viol_cnt = 0;
        for k in 0..self.m.saturating_sub(1) {
            let u = self.path[k];
            let v = self.path[k + 1];
            let b = if is_adj(u, v, self.n) { 0 } else { 1 };
            self.bad[k] = b;
            if b == 1 {
                self.bp_index[k] = self.breakpoints.len();
                self.breakpoints.push(k);
                self.viol_cnt += 1;
            } else {
                self.bp_index[k] = usize::MAX;
            }
            if is_rank_jump_bad(u, v, rank_by_cell) {
                self.rank_bad += 1;
            }
        }
        self.risk = compute_risk(&self.bad);
    }

    fn objective(&self, lambda: i64, xi: i64) -> i64 {
        self.raw_score - lambda * self.viol_cnt as i64 - xi * self.rank_bad as i64
    }
}

#[derive(Clone, Copy)]
enum MoveKind {
    Reverse {
        l: usize,
        r: usize,
    },
    Relocate {
        l: usize,
        r: usize,
        p: usize,
        reversed: bool,
    },
    TwoCutReconnect {
        l: usize,
        m: usize,
        r: usize,
        pattern: u8,
    },
}

#[derive(Clone, Copy)]
struct Candidate {
    mv: MoveKind,
    delta_raw: i64,
    delta_viol: i32,
    delta_obj: i64,
}

#[derive(Clone, Copy)]
enum ProposalMode {
    Breakpoint,
    Rank,
    Random,
}

#[derive(Clone)]
struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let init = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { x: init }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    fn gen_usize(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }

    fn gen_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }

    fn gen_f64(&mut self) -> f64 {
        const DEN: f64 = (u64::MAX as f64) + 1.0;
        (self.next_u64() as f64) / DEN
    }
}

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let start = Instant::now();
    let hard_deadline = Duration::from_secs_f64(TIME_LIMIT);

    let weights = flatten_weights(n, &a);
    let neighbors = build_neighbors(n);
    let (rank_by_cell, cell_by_rank) = build_rank_arrays(&weights);

    let seed = weights
        .iter()
        .step_by((weights.len() / 97).max(1))
        .fold(0xC0FF_EE12_3456_789Au64, |acc, &w| {
            acc ^ ((w as u64).wrapping_mul(0x9E37_79B1))
        });
    let mut rng = XorShift64::new(seed);

    let init_path = cell_by_rank.clone();
    let mut state = State::new(n, init_path, &weights, &rank_by_cell);

    let baseline_route = build_baseline_feasible(n, &weights);
    let baseline_score = compute_raw_score(&baseline_route, &weights);
    let mut best_feasible_path = baseline_route.clone();
    let mut best_feasible_score = baseline_score;

    if state.viol_cnt == 0 && state.raw_score > best_feasible_score {
        best_feasible_score = state.raw_score;
        best_feasible_path = state.path.clone();
    }

    let scale_s = calibrate_raw_scale(&state, &weights, &mut rng);
    let budgets = build_budget_schedule(state.viol_cnt, PHASES);
    let lambdas = build_lambda_schedule(scale_s, PHASES);
    let xis = build_xi_schedule(scale_s, PHASES);
    let phase_deadlines = build_phase_deadlines();

    for p in 0..PHASES {
        let phase_deadline = phase_deadlines[p].min(hard_deadline);
        if start.elapsed() >= phase_deadline {
            break;
        }

        let budget = budgets[p];
        let lambda = lambdas[p];
        let xi = xis[p];
        let next_budget = if p + 1 < PHASES { budgets[p + 1] } else { 0 };

        let mut attempted_moves: u64 = 0;
        let mut accepted_moves: u64 = 0;
        let mut repair_calls: u64 = 0;
        let mut repair_reduced_total: i64 = 0;

        if state.viol_cnt > budget {
            let (calls, reduced) = force_to_budget(
                &mut state,
                &weights,
                &rank_by_cell,
                &neighbors,
                budget,
                lambda,
                xi,
                BASE_EJECTION_DEPTH + p / 3,
                &start,
                phase_deadline,
                &mut rng,
            );
            repair_calls += calls;
            repair_reduced_total += reduced as i64;
            update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
        }

        let phase_start = start.elapsed();
        let phase_total_secs = (phase_deadline
            .checked_sub(phase_start)
            .unwrap_or_else(|| Duration::from_millis(1))
            .as_secs_f64())
        .max(1e-6);

        let temp_start = ((scale_s as f64) * (1.0 - 0.07 * p as f64)).max(1.0);
        let temp_end = (temp_start / 30.0).max(if p + 1 == PHASES { 0.12 } else { 0.25 });

        while start.elapsed() < phase_deadline {
            let mode = sample_mode(p, &mut rng);
            let max_seg = phase_max_seg_len(p);
            let max_span = phase_max_span(p);

            let mut best: Option<Candidate> = None;
            for _ in 0..2 {
                if let Some(cand) = propose_candidate(
                    &state,
                    &weights,
                    &rank_by_cell,
                    &cell_by_rank,
                    &neighbors,
                    mode,
                    max_seg,
                    max_span,
                    lambda,
                    xi,
                    &mut rng,
                ) {
                    match best {
                        None => best = Some(cand),
                        Some(cur) => {
                            if cand.delta_obj > cur.delta_obj
                                || (cand.delta_obj == cur.delta_obj
                                    && cand.delta_viol < cur.delta_viol)
                                || (cand.delta_obj == cur.delta_obj
                                    && cand.delta_viol == cur.delta_viol
                                    && cand.delta_raw > cur.delta_raw)
                            {
                                best = Some(cand);
                            }
                        }
                    }
                }
            }

            let Some(cand) = best else {
                continue;
            };
            attempted_moves += 1;

            let new_viol = state.viol_cnt + cand.delta_viol;
            if new_viol > budget {
                continue;
            }

            let elapsed_phase =
                (start.elapsed().as_secs_f64() - phase_start.as_secs_f64()).max(0.0);
            let progress = (elapsed_phase / phase_total_secs).clamp(0.0, 1.0);
            let temp = temp_start * (temp_end / temp_start).powf(progress);

            if should_accept(cand.delta_obj, temp, &mut rng) {
                apply_move(&mut state.path, cand.mv);
                state.rebuild_metrics(&weights, &rank_by_cell);
                accepted_moves += 1;
                update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
            }

            if attempted_moves % 512 == 0 && state.viol_cnt > budget {
                let (calls, reduced) = force_to_budget(
                    &mut state,
                    &weights,
                    &rank_by_cell,
                    &neighbors,
                    budget,
                    lambda,
                    xi,
                    BASE_EJECTION_DEPTH + p / 3,
                    &start,
                    phase_deadline,
                    &mut rng,
                );
                repair_calls += calls;
                repair_reduced_total += reduced as i64;
                update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
            }
        }

        if p + 1 < PHASES && start.elapsed() < phase_deadline {
            let repair_window = phase_repair_window(p, state.m);
            let boosted_lambda = ((lambda as f64) * 1.35).round() as i64;
            let (calls, reduced) = phase_transition_repair(
                &mut state,
                &weights,
                &rank_by_cell,
                &neighbors,
                next_budget,
                boosted_lambda,
                xi,
                repair_window,
                BASE_EJECTION_DEPTH + 1 + p / 3,
                &start,
                phase_deadline,
                &mut rng,
            );
            repair_calls += calls;
            repair_reduced_total += reduced as i64;
            update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
        }

        let curr_obj = state.objective(lambda, xi);
        let bp_density = state.breakpoints.len() as f64 / (state.m.saturating_sub(1).max(1) as f64);
        let avg_repair = if repair_calls == 0 {
            0.0
        } else {
            repair_reduced_total as f64 / repair_calls as f64
        };
        eprintln!(
            "phase={} best_feasible={} curr_raw={} curr_viol={} curr_obj={} accepted={} attempted={} bp_density={:.6} repair_avg_delta_viol={:.3}",
            p,
            best_feasible_score,
            state.raw_score,
            state.viol_cnt,
            curr_obj,
            accepted_moves,
            attempted_moves,
            bp_density,
            avg_repair,
        );
    }

    let final_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.995);
    if start.elapsed() < final_deadline {
        let lambda_final = ((lambdas[PHASES - 1] as f64) * 1.8).round() as i64;
        let xi_final = xis[PHASES - 1];
        let (calls, reduced) = force_to_budget(
            &mut state,
            &weights,
            &rank_by_cell,
            &neighbors,
            0,
            lambda_final,
            xi_final,
            BASE_EJECTION_DEPTH + 3,
            &start,
            final_deadline,
            &mut rng,
        );
        if calls > 0 {
            eprintln!(
                "final_repair calls={} total_delta_viol={} curr_viol={} curr_raw={}",
                calls, reduced, state.viol_cnt, state.raw_score
            );
        }

        while state.viol_cnt > 0 && start.elapsed() < final_deadline {
            let before = state.viol_cnt;
            let hotspots = collect_hotspots(&state, 8);
            if hotspots.is_empty() {
                break;
            }

            let mut progressed = false;
            for bp in hotspots {
                if start.elapsed() >= final_deadline || state.viol_cnt == 0 {
                    break;
                }
                let (l, r) =
                    repair_window_bounds(state.m, bp, phase_repair_window(PHASES - 1, state.m));
                if local_rebuild_window(
                    &mut state,
                    &weights,
                    &rank_by_cell,
                    lambda_final,
                    xi_final,
                    l,
                    r,
                    2,
                ) {
                    progressed = true;
                    update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
                    if state.viol_cnt == 0 {
                        break;
                    }
                }

                let dec = breakpoint_ejection_chain(
                    &mut state,
                    &weights,
                    &rank_by_cell,
                    &neighbors,
                    0,
                    lambda_final,
                    xi_final,
                    bp,
                    BASE_EJECTION_DEPTH + 3,
                    &start,
                    final_deadline,
                    &mut rng,
                );
                if dec > 0 {
                    progressed = true;
                    update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);
                    if state.viol_cnt == 0 {
                        break;
                    }
                }
            }

            if !progressed || state.viol_cnt >= before {
                break;
            }
        }
    }

    update_best_feasible(&state, &mut best_feasible_path, &mut best_feasible_score);

    let mut final_path = if state.viol_cnt == 0 && state.raw_score >= best_feasible_score {
        state.path.clone()
    } else {
        best_feasible_path.clone()
    };

    if !is_valid_route(&final_path, n) {
        final_path = baseline_route;
    }

    for cell in final_path {
        println!("{} {}", cell / n, cell % n);
    }
}

fn flatten_weights(n: usize, a: &[Vec<i64>]) -> Vec<i64> {
    let mut weights = vec![0; n * n];
    for i in 0..n {
        for j in 0..n {
            weights[i * n + j] = a[i][j];
        }
    }
    weights
}

fn build_neighbors(n: usize) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); n * n];
    for i in 0..n {
        for j in 0..n {
            let cell = i * n + j;
            for di in -1isize..=1 {
                for dj in -1isize..=1 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let ni = i as isize + di;
                    let nj = j as isize + dj;
                    if ni < 0 || ni >= n as isize || nj < 0 || nj >= n as isize {
                        continue;
                    }
                    neighbors[cell].push(ni as usize * n + nj as usize);
                }
            }
        }
    }
    neighbors
}

fn build_rank_arrays(weights: &[i64]) -> (Vec<usize>, Vec<usize>) {
    let mut cell_by_rank: Vec<usize> = (0..weights.len()).collect();
    cell_by_rank.sort_unstable_by_key(|&cell| weights[cell]);
    let mut rank_by_cell = vec![0usize; weights.len()];
    for (rank, &cell) in cell_by_rank.iter().enumerate() {
        rank_by_cell[cell] = rank;
    }
    (rank_by_cell, cell_by_rank)
}

fn build_budget_schedule(v0: i32, phases: usize) -> Vec<i32> {
    let mut budgets = vec![0i32; phases];
    let init = v0.max(0) as f64;
    for p in 0..phases {
        let t = p as f64 / (phases.saturating_sub(1).max(1) as f64);
        let ratio = 1.0 - t;
        budgets[p] = (init * ratio * ratio).round() as i32;
    }
    budgets[phases - 1] = 0;
    for p in 1..phases {
        if budgets[p] > budgets[p - 1] {
            budgets[p] = budgets[p - 1];
        }
    }
    budgets
}

fn build_lambda_schedule(scale_s: i64, phases: usize) -> Vec<i64> {
    let s = scale_s.max(1) as f64;
    let lambda0 = (s / 20.0).max(1.0);
    let lambda_last = (s * 5.0).max(lambda0 + 1.0);
    let ratio = lambda_last / lambda0;
    let mut lambdas = vec![0i64; phases];
    for p in 0..phases {
        let t = p as f64 / (phases.saturating_sub(1).max(1) as f64);
        lambdas[p] = (lambda0 * ratio.powf(t)).round() as i64;
    }
    lambdas
}

fn build_xi_schedule(scale_s: i64, phases: usize) -> Vec<i64> {
    let s = scale_s.max(1) as f64;
    let xi0 = (s / 120.0).max(1.0);
    let mut xis = vec![0i64; phases];
    for p in 0..phases {
        let t = p as f64 / (phases.saturating_sub(1).max(1) as f64);
        let val = xi0 * (1.0 - 0.85 * t);
        xis[p] = val.max(0.0).round() as i64;
    }
    xis
}

fn build_phase_deadlines() -> Vec<Duration> {
    let phase_weights = [
        0.1167f64, 0.1167, 0.1166, 0.10, 0.10, 0.10, 0.10, 0.0833, 0.0833, 0.0834,
    ];
    let phase_total = TIME_LIMIT * 0.94;
    let mut cum = 0.0;
    let mut deadlines = Vec::with_capacity(PHASES);
    for &w in &phase_weights {
        cum += w;
        deadlines.push(Duration::from_secs_f64(
            (phase_total * cum).min(TIME_LIMIT * 0.985),
        ));
    }
    deadlines
}

fn phase_max_seg_len(p: usize) -> usize {
    if p <= 2 {
        12
    } else if p <= 6 {
        10
    } else {
        8
    }
}

fn phase_max_span(p: usize) -> usize {
    if p <= 2 {
        420
    } else if p <= 6 {
        300
    } else {
        220
    }
}

fn phase_repair_window(p: usize, m: usize) -> usize {
    let t = p as f64 / (PHASES.saturating_sub(1).max(1) as f64);
    let len = REPAIR_WINDOW_MIN as f64 + (REPAIR_WINDOW_MAX - REPAIR_WINDOW_MIN) as f64 * t;
    len.round().clamp(32.0, (m.saturating_sub(2)) as f64) as usize
}

fn calibrate_raw_scale(state: &State, weights: &[i64], rng: &mut XorShift64) -> i64 {
    let mut samples = Vec::with_capacity(256);
    let m = state.m;
    if m < 4 {
        return 1;
    }

    for _ in 0..160 {
        let l = rng.gen_usize(m - 1);
        let max_span = (m - 1 - l).min(400);
        if max_span == 0 {
            continue;
        }
        let r = l + 1 + rng.gen_usize(max_span);
        let delta = reverse_delta_raw(&state.path, weights, l, r).abs();
        if delta > 0 {
            samples.push(delta);
        }
    }

    for _ in 0..160 {
        if let Some((l, r, p, reversed)) = random_relocate_params(state, 6, 300, rng) {
            let delta = relocate_delta_raw(&state.path, weights, l, r, p, reversed).abs();
            if delta > 0 {
                samples.push(delta);
            }
        }
    }

    if samples.is_empty() {
        return 1;
    }
    samples.sort_unstable();
    samples[samples.len() / 2].max(1)
}

fn sample_mode(phase: usize, rng: &mut XorShift64) -> ProposalMode {
    let roll = rng.gen_usize(100);
    if phase <= 2 {
        if roll < 50 {
            ProposalMode::Breakpoint
        } else if roll < 85 {
            ProposalMode::Rank
        } else {
            ProposalMode::Random
        }
    } else if phase <= 6 {
        if roll < 60 {
            ProposalMode::Breakpoint
        } else if roll < 85 {
            ProposalMode::Rank
        } else {
            ProposalMode::Random
        }
    } else if roll < 70 {
        ProposalMode::Breakpoint
    } else if roll < 85 {
        ProposalMode::Rank
    } else {
        ProposalMode::Random
    }
}

fn propose_candidate(
    state: &State,
    weights: &[i64],
    rank_by_cell: &[usize],
    cell_by_rank: &[usize],
    neighbors: &[Vec<usize>],
    mode: ProposalMode,
    max_seg: usize,
    max_span: usize,
    lambda: i64,
    xi: i64,
    rng: &mut XorShift64,
) -> Option<Candidate> {
    for _ in 0..28 {
        let mv = match mode {
            ProposalMode::Breakpoint => {
                gen_breakpoint_move(state, neighbors, max_seg, max_span, rng)
            }
            ProposalMode::Rank => gen_rank_move(state, cell_by_rank, max_seg, max_span, rng),
            ProposalMode::Random => gen_random_move(state, max_seg, max_span, rng),
        };

        let Some(mv) = mv else {
            continue;
        };
        if let Some(cand) = evaluate_move(state, weights, rank_by_cell, mv, lambda, xi) {
            return Some(cand);
        }
    }
    None
}

fn gen_breakpoint_move(
    state: &State,
    neighbors: &[Vec<usize>],
    max_seg: usize,
    max_span: usize,
    rng: &mut XorShift64,
) -> Option<MoveKind> {
    if state.breakpoints.is_empty() {
        return gen_random_move(state, max_seg, max_span, rng);
    }

    let k = state.breakpoints[rng.gen_usize(state.breakpoints.len())];
    let roll = rng.gen_usize(100);
    if roll < 42 {
        if rng.gen_bool() {
            let l = k + 1;
            if l + 1 >= state.m {
                return None;
            }
            let left = state.path[k];
            let mut candidates = Vec::new();
            for &anchor in &neighbors[left] {
                let r = state.pos[anchor];
                if r > l && r - l <= max_span {
                    candidates.push(r);
                }
            }
            if candidates.is_empty() {
                return None;
            }
            let r = candidates[rng.gen_usize(candidates.len())];
            Some(MoveKind::Reverse { l, r })
        } else {
            if k < 1 {
                return None;
            }
            let right = state.path[k + 1];
            let mut candidates = Vec::new();
            for &anchor in &neighbors[right] {
                let l = state.pos[anchor];
                if l < k && k - l <= max_span {
                    candidates.push(l);
                }
            }
            if candidates.is_empty() {
                return None;
            }
            let l = candidates[rng.gen_usize(candidates.len())];
            Some(MoveKind::Reverse { l, r: k })
        }
    } else if roll < 78 {
        if rng.gen_bool() {
            let l = k + 1;
            if l == 0 || l + 1 >= state.m {
                return None;
            }
            let seg_len = 1 + rng.gen_usize(max_seg.max(1));
            let r = l + seg_len - 1;
            if r + 1 >= state.m {
                return None;
            }
            let left = state.path[k];
            let mut params = Vec::new();
            for &anchor in &neighbors[left] {
                let p = state.pos[anchor];
                if p + 1 < state.m && edge_disjoint_from_segment(p, l, r) {
                    params.push((p, false));
                    params.push((p, true));
                }
            }
            if params.is_empty() {
                return None;
            }
            let (p, reversed) = params[rng.gen_usize(params.len())];
            Some(MoveKind::Relocate { l, r, p, reversed })
        } else {
            if k == 0 {
                return None;
            }
            let seg_len = 1 + rng.gen_usize(max_seg.max(1));
            if k + 1 < seg_len {
                return None;
            }
            let r = k;
            let l = r + 1 - seg_len;
            if l == 0 || r + 1 >= state.m {
                return None;
            }
            let right = state.path[k + 1];
            let mut params = Vec::new();
            for &anchor in &neighbors[right] {
                let p = state.pos[anchor];
                if p + 1 < state.m && edge_disjoint_from_segment(p, l, r) {
                    params.push((p, false));
                    params.push((p, true));
                }
            }
            if params.is_empty() {
                return None;
            }
            let (p, reversed) = params[rng.gen_usize(params.len())];
            Some(MoveKind::Relocate { l, r, p, reversed })
        }
    } else {
        gen_two_cut_reconnect(state, max_span, rng)
    }
}

fn gen_two_cut_reconnect(state: &State, max_span: usize, rng: &mut XorShift64) -> Option<MoveKind> {
    if state.breakpoints.len() < 2 {
        return None;
    }

    let k1 = state.breakpoints[rng.gen_usize(state.breakpoints.len())];
    let mut k2 = state.breakpoints[rng.gen_usize(state.breakpoints.len())];
    if k1 == k2 {
        k2 = state.breakpoints[(state.bp_index[k1] + 1) % state.breakpoints.len()];
    }

    let l = k1.min(k2) + 1;
    let r = k1.max(k2);
    if l >= r || r - l > max_span || r + 1 >= state.m {
        return None;
    }

    let m = l + (r - l) / 2;
    let pattern = rng.gen_usize(3) as u8;
    Some(MoveKind::TwoCutReconnect { l, m, r, pattern })
}

fn gen_rank_move(
    state: &State,
    cell_by_rank: &[usize],
    max_seg: usize,
    max_span: usize,
    rng: &mut XorShift64,
) -> Option<MoveKind> {
    let m = state.m;
    if m < 4 {
        return None;
    }

    let rank_idx = rng.gen_usize(m);
    let cell = cell_by_rank[rank_idx];
    let cur = state.pos[cell];
    let ideal = rank_idx;

    if cur > ideal + 6 {
        let seg_len = 1 + rng.gen_usize(max_seg.min(6).max(1));
        let mut l = cur.saturating_sub(seg_len / 2);
        if l + seg_len >= m {
            l = m - 1 - seg_len;
        }
        let r = l + seg_len - 1;
        if l == 0 || r + 1 >= m {
            return None;
        }
        let mut p = ideal.saturating_sub(1);
        p = p.min(m - 2);
        if !edge_disjoint_from_segment(p, l, r) {
            return None;
        }
        Some(MoveKind::Relocate {
            l,
            r,
            p,
            reversed: false,
        })
    } else if ideal > cur + 6 {
        let seg_len = 1 + rng.gen_usize(max_seg.min(6).max(1));
        let mut l = cur.saturating_sub(seg_len / 2);
        if l + seg_len >= m {
            l = m - 1 - seg_len;
        }
        let r = l + seg_len - 1;
        if l == 0 || r + 1 >= m {
            return None;
        }
        let mut p = ideal.min(m - 2);
        if p.abs_diff(l) > max_span && p.abs_diff(r) > max_span {
            p = if p > r {
                (r + max_span).min(m - 2)
            } else {
                l.saturating_sub(max_span).min(m - 2)
            };
        }
        if !edge_disjoint_from_segment(p, l, r) {
            return None;
        }
        Some(MoveKind::Relocate {
            l,
            r,
            p,
            reversed: false,
        })
    } else {
        let other_rank = rng.gen_usize(m);
        let other_cell = cell_by_rank[other_rank];
        let other_pos = state.pos[other_cell];
        let mut l = cur.min(other_pos);
        let mut r = cur.max(other_pos);
        if l == r {
            return None;
        }
        if r - l > max_span {
            if cur < other_pos {
                r = l + max_span;
            } else {
                l = r - max_span;
            }
        }
        if l < r {
            Some(MoveKind::Reverse { l, r })
        } else {
            None
        }
    }
}

fn gen_random_move(
    state: &State,
    max_seg: usize,
    max_span: usize,
    rng: &mut XorShift64,
) -> Option<MoveKind> {
    let m = state.m;
    if m < 4 {
        return None;
    }

    let roll = rng.gen_usize(100);
    if roll < 55 {
        let l = rng.gen_usize(m - 1);
        let max_len = (m - 1 - l).min(max_span.max(2));
        if max_len < 1 {
            return None;
        }
        let r = l + 1 + rng.gen_usize(max_len);
        Some(MoveKind::Reverse { l, r })
    } else if roll < 90 {
        random_relocate_params(state, max_seg, max_span, rng)
            .map(|(l, r, p, reversed)| MoveKind::Relocate { l, r, p, reversed })
    } else {
        gen_two_cut_reconnect(state, max_span, rng)
    }
}

fn random_relocate_params(
    state: &State,
    max_seg: usize,
    max_span: usize,
    rng: &mut XorShift64,
) -> Option<(usize, usize, usize, bool)> {
    let m = state.m;
    if m < 6 {
        return None;
    }

    for _ in 0..24 {
        let l = 1 + rng.gen_usize(m - 3);
        let seg_cap = (m - 2 - l).min(max_seg.max(1));
        if seg_cap == 0 {
            continue;
        }
        let seg_len = 1 + rng.gen_usize(seg_cap);
        let r = l + seg_len - 1;
        if r + 1 >= m {
            continue;
        }

        let lo = l.saturating_sub(max_span).min(m - 2);
        let hi = (r + max_span).min(m - 2);
        if lo > hi {
            continue;
        }

        let p = lo + rng.gen_usize(hi - lo + 1);
        if !edge_disjoint_from_segment(p, l, r) {
            continue;
        }

        return Some((l, r, p, rng.gen_bool()));
    }
    None
}

fn evaluate_move(
    state: &State,
    weights: &[i64],
    rank_by_cell: &[usize],
    mv: MoveKind,
    lambda: i64,
    xi: i64,
) -> Option<Candidate> {
    let (base_mv, delta_raw, delta_viol, delta_rank) = match mv {
        MoveKind::Reverse { l, r } => {
            if l >= r || r >= state.m {
                return None;
            }
            let delta_raw = reverse_delta_raw(&state.path, weights, l, r);
            let (delta_viol, delta_rank) = reverse_delta_penalties(state, rank_by_cell, l, r);
            (mv, delta_raw, delta_viol, delta_rank)
        }
        MoveKind::Relocate { l, r, p, reversed } => {
            if !is_valid_relocate(state.m, l, r, p) {
                return None;
            }
            let delta_raw = relocate_delta_raw(&state.path, weights, l, r, p, reversed);
            let (delta_viol, delta_rank) =
                relocate_delta_penalties(state, rank_by_cell, l, r, p, reversed);
            (mv, delta_raw, delta_viol, delta_rank)
        }
        MoveKind::TwoCutReconnect { l, m, r, pattern } => {
            if l >= r || r >= state.m {
                return None;
            }
            if pattern == 0 {
                let delta_raw = reverse_delta_raw(&state.path, weights, l, r);
                let (delta_viol, delta_rank) = reverse_delta_penalties(state, rank_by_cell, l, r);
                (
                    MoveKind::TwoCutReconnect { l, m, r, pattern },
                    delta_raw,
                    delta_viol,
                    delta_rank,
                )
            } else {
                if m < l || m >= r {
                    return None;
                }
                let reversed = pattern >= 2;
                if !is_valid_relocate(state.m, l, m, r) {
                    return None;
                }
                let delta_raw = relocate_delta_raw(&state.path, weights, l, m, r, reversed);
                let (delta_viol, delta_rank) =
                    relocate_delta_penalties(state, rank_by_cell, l, m, r, reversed);
                (
                    MoveKind::TwoCutReconnect { l, m, r, pattern },
                    delta_raw,
                    delta_viol,
                    delta_rank,
                )
            }
        }
    };

    let delta_obj = delta_raw - lambda * delta_viol as i64 - xi * delta_rank as i64;
    Some(Candidate {
        mv: base_mv,
        delta_raw,
        delta_viol,
        delta_obj,
    })
}

fn apply_move(path: &mut [usize], mv: MoveKind) {
    match mv {
        MoveKind::Reverse { l, r } => {
            path[l..=r].reverse();
        }
        MoveKind::Relocate { l, r, p, reversed } => {
            apply_relocation(path, RelocationMove { l, r, p, reversed });
        }
        MoveKind::TwoCutReconnect { l, m, r, pattern } => {
            if pattern == 0 {
                path[l..=r].reverse();
            } else {
                let reversed = pattern >= 2;
                apply_relocation(
                    path,
                    RelocationMove {
                        l,
                        r: m,
                        p: r,
                        reversed,
                    },
                );
            }
        }
    }
}

fn should_accept(delta_obj: i64, temperature: f64, rng: &mut XorShift64) -> bool {
    if delta_obj >= 0 {
        return true;
    }
    let temp = temperature.max(1e-9);
    let prob = ((delta_obj as f64) / temp).exp();
    rng.gen_f64() < prob
}

fn reverse_delta_raw(path: &[usize], weights: &[i64], l: usize, r: usize) -> i64 {
    let mut old = 0i64;
    let mut new = 0i64;
    let len = r - l + 1;
    for t in 0..len {
        old += (l + t) as i64 * weights[path[l + t]];
        new += (l + t) as i64 * weights[path[r - t]];
    }
    new - old
}

fn reverse_delta_penalties(
    state: &State,
    rank_by_cell: &[usize],
    l: usize,
    r: usize,
) -> (i32, i32) {
    let mut old_v = 0i32;
    let mut new_v = 0i32;
    let mut old_rank = 0i32;
    let mut new_rank = 0i32;

    if l > 0 {
        let a = state.path[l - 1];
        let b = state.path[l];
        let c = state.path[r];
        old_v += edge_violation_cost(a, b, state.n);
        new_v += edge_violation_cost(a, c, state.n);
        old_rank += edge_rank_bad_cost(a, b, rank_by_cell);
        new_rank += edge_rank_bad_cost(a, c, rank_by_cell);
    }
    if r + 1 < state.m {
        let a = state.path[l];
        let b = state.path[r];
        let c = state.path[r + 1];
        old_v += edge_violation_cost(b, c, state.n);
        new_v += edge_violation_cost(a, c, state.n);
        old_rank += edge_rank_bad_cost(b, c, rank_by_cell);
        new_rank += edge_rank_bad_cost(a, c, rank_by_cell);
    }

    (new_v - old_v, new_rank - old_rank)
}

fn relocate_delta_raw(
    path: &[usize],
    weights: &[i64],
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
) -> i64 {
    let seg_len = r - l + 1;
    let mut old = 0i64;
    let mut new = 0i64;

    if p < l {
        let start = p + 1;
        for idx in start..=r {
            old += idx as i64 * weights[path[idx]];
        }

        for t in 0..seg_len {
            let cell = if reversed { path[r - t] } else { path[l + t] };
            new += (start + t) as i64 * weights[cell];
        }
        let tail_len = l - start;
        for t in 0..tail_len {
            let cell = path[start + t];
            new += (start + seg_len + t) as i64 * weights[cell];
        }
    } else {
        let start = l;
        for idx in l..=p {
            old += idx as i64 * weights[path[idx]];
        }

        let tail_len = p - r;
        for t in 0..tail_len {
            let cell = path[r + 1 + t];
            new += (start + t) as i64 * weights[cell];
        }
        for t in 0..seg_len {
            let cell = if reversed { path[r - t] } else { path[l + t] };
            new += (start + tail_len + t) as i64 * weights[cell];
        }
    }

    new - old
}

fn relocate_delta_penalties(
    state: &State,
    rank_by_cell: &[usize],
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
) -> (i32, i32) {
    let a = state.path[l - 1];
    let b = state.path[l];
    let c = state.path[r];
    let d = state.path[r + 1];
    let e = state.path[p];
    let f = state.path[p + 1];

    let old_v = edge_violation_cost(a, b, state.n)
        + edge_violation_cost(c, d, state.n)
        + edge_violation_cost(e, f, state.n);
    let old_rank = edge_rank_bad_cost(a, b, rank_by_cell)
        + edge_rank_bad_cost(c, d, rank_by_cell)
        + edge_rank_bad_cost(e, f, rank_by_cell);

    let mut new_v = edge_violation_cost(a, d, state.n);
    let mut new_rank = edge_rank_bad_cost(a, d, rank_by_cell);

    if reversed {
        new_v += edge_violation_cost(e, c, state.n);
        new_v += edge_violation_cost(b, f, state.n);
        new_rank += edge_rank_bad_cost(e, c, rank_by_cell);
        new_rank += edge_rank_bad_cost(b, f, rank_by_cell);
    } else {
        new_v += edge_violation_cost(e, b, state.n);
        new_v += edge_violation_cost(c, f, state.n);
        new_rank += edge_rank_bad_cost(e, b, rank_by_cell);
        new_rank += edge_rank_bad_cost(c, f, rank_by_cell);
    }

    (new_v - old_v, new_rank - old_rank)
}

fn force_to_budget(
    state: &mut State,
    weights: &[i64],
    rank_by_cell: &[usize],
    neighbors: &[Vec<usize>],
    target_budget: i32,
    lambda: i64,
    xi: i64,
    depth: usize,
    start: &Instant,
    deadline: Duration,
    rng: &mut XorShift64,
) -> (u64, i32) {
    let mut calls = 0u64;
    let mut reduced_total = 0i32;

    while state.viol_cnt > target_budget && start.elapsed() < deadline {
        let hotspots = collect_hotspots(state, 12);
        if hotspots.is_empty() {
            break;
        }

        let mut progressed = false;
        for bp in hotspots {
            if start.elapsed() >= deadline || state.viol_cnt <= target_budget {
                break;
            }

            if let Some(cand) = best_repair_candidate_around_breakpoint(
                state,
                weights,
                rank_by_cell,
                neighbors,
                bp,
                lambda,
                xi,
                phase_max_seg_len(PHASES - 1),
                phase_max_span(PHASES - 1),
            ) {
                let new_viol = state.viol_cnt + cand.delta_viol;
                if new_viol <= target_budget || cand.delta_viol < 0 {
                    let before = state.viol_cnt;
                    apply_move(&mut state.path, cand.mv);
                    state.rebuild_metrics(weights, rank_by_cell);
                    let reduced = (before - state.viol_cnt).max(0);
                    if reduced > 0 {
                        reduced_total += reduced;
                    }
                    calls += 1;
                    progressed = true;
                    if state.viol_cnt <= target_budget {
                        break;
                    }
                    continue;
                }
            }

            let (l, r) = repair_window_bounds(state.m, bp, REPAIR_WINDOW_MIN);
            let before = state.viol_cnt;
            if local_rebuild_window(state, weights, rank_by_cell, lambda, xi, l, r, 1) {
                let reduced = (before - state.viol_cnt).max(0);
                if reduced > 0 {
                    reduced_total += reduced;
                }
                calls += 1;
                progressed = true;
                if state.viol_cnt <= target_budget {
                    break;
                }
            }

            let dec = breakpoint_ejection_chain(
                state,
                weights,
                rank_by_cell,
                neighbors,
                target_budget,
                lambda,
                xi,
                bp,
                depth,
                start,
                deadline,
                rng,
            );
            if dec > 0 {
                reduced_total += dec;
                calls += 1;
                progressed = true;
                if state.viol_cnt <= target_budget {
                    break;
                }
            }
        }

        if !progressed {
            break;
        }
    }

    (calls, reduced_total)
}

fn phase_transition_repair(
    state: &mut State,
    weights: &[i64],
    rank_by_cell: &[usize],
    neighbors: &[Vec<usize>],
    target_budget: i32,
    lambda: i64,
    xi: i64,
    repair_window: usize,
    depth: usize,
    start: &Instant,
    deadline: Duration,
    rng: &mut XorShift64,
) -> (u64, i32) {
    if state.viol_cnt <= target_budget {
        return (0, 0);
    }

    let mut calls = 0u64;
    let mut reduced_total = 0i32;

    let hotspots = collect_hotspots(state, 28);
    for bp in hotspots {
        if state.viol_cnt <= target_budget || start.elapsed() >= deadline {
            break;
        }

        let before = state.viol_cnt;
        let (l, r) = repair_window_bounds(state.m, bp, repair_window);
        if local_rebuild_window(state, weights, rank_by_cell, lambda, xi, l, r, 2) {
            let reduced = (before - state.viol_cnt).max(0);
            if reduced > 0 {
                reduced_total += reduced;
            }
            calls += 1;
            if state.viol_cnt <= target_budget {
                break;
            }
        }

        let dec = breakpoint_ejection_chain(
            state,
            weights,
            rank_by_cell,
            neighbors,
            target_budget,
            lambda,
            xi,
            bp,
            depth,
            start,
            deadline,
            rng,
        );
        if dec > 0 {
            reduced_total += dec;
            calls += 1;
            if state.viol_cnt <= target_budget {
                break;
            }
        }
    }

    if state.viol_cnt > target_budget && start.elapsed() < deadline {
        let (extra_calls, extra_reduced) = force_to_budget(
            state,
            weights,
            rank_by_cell,
            neighbors,
            target_budget,
            lambda,
            xi,
            depth + 1,
            start,
            deadline,
            rng,
        );
        calls += extra_calls;
        reduced_total += extra_reduced;
    }

    (calls, reduced_total)
}

fn best_repair_candidate_around_breakpoint(
    state: &State,
    weights: &[i64],
    rank_by_cell: &[usize],
    neighbors: &[Vec<usize>],
    bp: usize,
    lambda: i64,
    xi: i64,
    max_seg: usize,
    max_span: usize,
) -> Option<Candidate> {
    if bp + 1 >= state.m {
        return None;
    }

    let left_cell = state.path[bp];
    let right_cell = state.path[bp + 1];
    let mut best: Option<Candidate> = None;

    let l = bp + 1;
    for &anchor in &neighbors[left_cell] {
        let r = state.pos[anchor];
        if r <= l || r - l > max_span {
            continue;
        }
        if let Some(cand) = evaluate_move(
            state,
            weights,
            rank_by_cell,
            MoveKind::Reverse { l, r },
            lambda,
            xi,
        ) {
            consider_repair_candidate(&mut best, cand);
        }
    }

    for &anchor in &neighbors[right_cell] {
        let l = state.pos[anchor];
        if l >= bp || bp - l > max_span {
            continue;
        }
        if let Some(cand) = evaluate_move(
            state,
            weights,
            rank_by_cell,
            MoveKind::Reverse { l, r: bp },
            lambda,
            xi,
        ) {
            consider_repair_candidate(&mut best, cand);
        }
    }

    for seg_len in 1..=max_seg {
        let l = bp + 1;
        let r = l + seg_len - 1;
        if l == 0 || r + 1 >= state.m {
            break;
        }
        for &anchor in &neighbors[left_cell] {
            let p = state.pos[anchor];
            if !is_valid_relocate(state.m, l, r, p) {
                continue;
            }
            for &reversed in &[false, true] {
                if let Some(cand) = evaluate_move(
                    state,
                    weights,
                    rank_by_cell,
                    MoveKind::Relocate { l, r, p, reversed },
                    lambda,
                    xi,
                ) {
                    consider_repair_candidate(&mut best, cand);
                }
            }
        }
    }

    for seg_len in 1..=max_seg {
        if bp + 1 < seg_len {
            break;
        }
        let r = bp;
        let l = r + 1 - seg_len;
        if l == 0 || r + 1 >= state.m {
            continue;
        }
        for &anchor in &neighbors[right_cell] {
            let p = state.pos[anchor];
            if !is_valid_relocate(state.m, l, r, p) {
                continue;
            }
            for &reversed in &[false, true] {
                if let Some(cand) = evaluate_move(
                    state,
                    weights,
                    rank_by_cell,
                    MoveKind::Relocate { l, r, p, reversed },
                    lambda,
                    xi,
                ) {
                    consider_repair_candidate(&mut best, cand);
                }
            }
        }
    }

    best
}

fn consider_repair_candidate(best: &mut Option<Candidate>, cand: Candidate) {
    if cand.delta_viol > 0 {
        return;
    }
    match best {
        None => *best = Some(cand),
        Some(cur) => {
            if cand.delta_viol < cur.delta_viol
                || (cand.delta_viol == cur.delta_viol && cand.delta_obj > cur.delta_obj)
                || (cand.delta_viol == cur.delta_viol
                    && cand.delta_obj == cur.delta_obj
                    && cand.delta_raw > cur.delta_raw)
            {
                *best = Some(cand);
            }
        }
    }
}

fn breakpoint_ejection_chain(
    state: &mut State,
    weights: &[i64],
    rank_by_cell: &[usize],
    neighbors: &[Vec<usize>],
    budget: i32,
    lambda: i64,
    xi: i64,
    start_bp: usize,
    depth: usize,
    start: &Instant,
    deadline: Duration,
    _rng: &mut XorShift64,
) -> i32 {
    if state.breakpoints.is_empty() {
        return 0;
    }

    let mut focus = nearest_breakpoint(state, start_bp);
    let mut reduced_total = 0i32;

    for _ in 0..depth {
        if start.elapsed() >= deadline || state.viol_cnt <= budget {
            break;
        }

        let Some(cand) = best_repair_candidate_around_breakpoint(
            state,
            weights,
            rank_by_cell,
            neighbors,
            focus,
            lambda,
            xi,
            phase_max_seg_len(PHASES - 1),
            phase_max_span(PHASES - 1),
        ) else {
            break;
        };

        if cand.delta_viol > 0 {
            break;
        }
        if state.viol_cnt + cand.delta_viol > budget {
            break;
        }

        let before = state.viol_cnt;
        apply_move(&mut state.path, cand.mv);
        state.rebuild_metrics(weights, rank_by_cell);
        let reduced = (before - state.viol_cnt).max(0);
        reduced_total += reduced;

        if state.breakpoints.is_empty() {
            break;
        }
        focus = nearest_breakpoint(state, focus);
    }

    reduced_total
}

fn local_rebuild_window(
    state: &mut State,
    weights: &[i64],
    rank_by_cell: &[usize],
    lambda: i64,
    xi: i64,
    left: usize,
    right: usize,
    passes: usize,
) -> bool {
    if state.m < 2 || left >= state.m || right >= state.m || left >= right {
        return false;
    }

    let mut changed_any = false;
    for _ in 0..passes {
        for &window in &[9usize, 8usize] {
            let mut pos = left;
            while pos <= right {
                let max_len = right + 1 - pos;
                let len = max_len.min(window);
                if len >= 2
                    && optimize_penalty_window(
                        &mut state.path,
                        weights,
                        state.n,
                        rank_by_cell,
                        pos,
                        len,
                        lambda,
                        xi,
                    )
                {
                    changed_any = true;
                }
                if pos == right {
                    break;
                }
                pos += 1;
            }
        }
    }

    if changed_any {
        state.rebuild_metrics(weights, rank_by_cell);
    }
    changed_any
}

fn optimize_penalty_window(
    path: &mut [usize],
    weights: &[i64],
    n: usize,
    rank_by_cell: &[usize],
    start: usize,
    len: usize,
    lambda: i64,
    xi: i64,
) -> bool {
    if len < 2 || len > MAX_DP_WINDOW {
        return false;
    }

    let mut seg = [0usize; MAX_DP_WINDOW];
    for i in 0..len {
        seg[i] = path[start + i];
    }

    let left = if start > 0 {
        Some(path[start - 1])
    } else {
        None
    };
    let right = if start + len < path.len() {
        Some(path[start + len])
    } else {
        None
    };

    let old_obj = local_penalty_objective(path, weights, n, rank_by_cell, start, len, lambda, xi);

    let states = 1usize << len;
    let width = len;
    let mut dp = vec![NEG_INF; states * width];
    let mut parent = vec![u8::MAX; states * width];

    for first in 0..len {
        let mut val = start as i64 * weights[seg[first]];
        if let Some(cell) = left {
            val -= lambda * edge_violation_cost(cell, seg[first], n) as i64;
            val -= xi * edge_rank_bad_cost(cell, seg[first], rank_by_cell) as i64;
        }
        let mask = 1usize << first;
        dp[mask * width + first] = val;
    }

    for mask in 1usize..states {
        let used = mask.count_ones() as usize;
        if used >= len {
            continue;
        }
        let next_pos = start + used;
        for last in 0..len {
            if (mask & (1usize << last)) == 0 {
                continue;
            }
            let cur = dp[mask * width + last];
            if cur == NEG_INF {
                continue;
            }
            for nxt in 0..len {
                if (mask & (1usize << nxt)) != 0 {
                    continue;
                }
                let next_mask = mask | (1usize << nxt);
                let mut next_val = cur + next_pos as i64 * weights[seg[nxt]];
                next_val -= lambda * edge_violation_cost(seg[last], seg[nxt], n) as i64;
                next_val -= xi * edge_rank_bad_cost(seg[last], seg[nxt], rank_by_cell) as i64;
                let slot = next_mask * width + nxt;
                if next_val > dp[slot] {
                    dp[slot] = next_val;
                    parent[slot] = last as u8;
                }
            }
        }
    }

    let full = states - 1;
    let mut best_last = usize::MAX;
    let mut best_obj = old_obj;

    for last in 0..len {
        let mut val = dp[full * width + last];
        if val == NEG_INF {
            continue;
        }
        if let Some(cell) = right {
            val -= lambda * edge_violation_cost(seg[last], cell, n) as i64;
            val -= xi * edge_rank_bad_cost(seg[last], cell, rank_by_cell) as i64;
        }
        if val > best_obj {
            best_obj = val;
            best_last = last;
        }
    }

    if best_last == usize::MAX {
        return false;
    }

    let mut order = [0usize; MAX_DP_WINDOW];
    let mut mask = full;
    let mut last = best_last;
    for pos in (0..len).rev() {
        order[pos] = last;
        if pos == 0 {
            break;
        }
        let prev = parent[mask * width + last];
        mask ^= 1usize << last;
        last = prev as usize;
    }

    let mut changed = false;
    for i in 0..len {
        let cell = seg[order[i]];
        if path[start + i] != cell {
            path[start + i] = cell;
            changed = true;
        }
    }

    changed
}

fn local_penalty_objective(
    path: &[usize],
    weights: &[i64],
    n: usize,
    rank_by_cell: &[usize],
    start: usize,
    len: usize,
    lambda: i64,
    xi: i64,
) -> i64 {
    let mut obj = 0i64;
    for i in 0..len {
        obj += (start + i) as i64 * weights[path[start + i]];
    }
    if start > 0 {
        let u = path[start - 1];
        let v = path[start];
        obj -= lambda * edge_violation_cost(u, v, n) as i64;
        obj -= xi * edge_rank_bad_cost(u, v, rank_by_cell) as i64;
    }
    for i in start..start + len - 1 {
        let u = path[i];
        let v = path[i + 1];
        obj -= lambda * edge_violation_cost(u, v, n) as i64;
        obj -= xi * edge_rank_bad_cost(u, v, rank_by_cell) as i64;
    }
    if start + len < path.len() {
        let u = path[start + len - 1];
        let v = path[start + len];
        obj -= lambda * edge_violation_cost(u, v, n) as i64;
        obj -= xi * edge_rank_bad_cost(u, v, rank_by_cell) as i64;
    }
    obj
}

fn collect_hotspots(state: &State, max_count: usize) -> Vec<usize> {
    if state.breakpoints.is_empty() {
        return Vec::new();
    }

    let mut scored = Vec::with_capacity(state.breakpoints.len());
    for &k in &state.breakpoints {
        let mut score = 1i32;
        if k < 96 || k + 96 >= state.m.saturating_sub(1) {
            score += 3;
        }
        let lo = k.saturating_sub(2);
        let hi = (k + 2).min(state.m.saturating_sub(2));
        for t in lo..=hi {
            score += state.bad[t] as i32;
        }
        scored.push((score, k));
    }

    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(max_count).map(|(_, k)| k).collect()
}

fn repair_window_bounds(m: usize, bp: usize, window: usize) -> (usize, usize) {
    if m < 2 {
        return (0, 0);
    }
    let half = window / 2;
    let mut l = bp.saturating_sub(half);
    let mut r = (bp + half).min(m - 1);
    if r <= l {
        r = (l + 1).min(m - 1);
    }
    if r - l + 1 < window && window < m {
        let need = window - (r - l + 1);
        let add_left = need.min(l);
        l -= add_left;
        let rem = need - add_left;
        r = (r + rem).min(m - 1);
    }
    (l, r)
}

fn nearest_breakpoint(state: &State, anchor: usize) -> usize {
    if state.breakpoints.is_empty() {
        return 0;
    }
    let mut lo = 0usize;
    let mut hi = state.breakpoints.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if state.breakpoints[mid] < anchor {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    if lo == 0 {
        state.breakpoints[0]
    } else if lo >= state.breakpoints.len() {
        *state.breakpoints.last().unwrap_or(&state.breakpoints[0])
    } else {
        let a = state.breakpoints[lo - 1];
        let b = state.breakpoints[lo];
        if anchor.abs_diff(a) <= anchor.abs_diff(b) {
            a
        } else {
            b
        }
    }
}

fn compute_risk(bad: &[u8]) -> i32 {
    if bad.is_empty() {
        return 0;
    }
    let endpoint_bad = bad[0] as i32 + bad[bad.len() - 1] as i32;
    let mut cluster_bad = 0i32;
    for i in 1..bad.len() {
        if bad[i - 1] == 1 && bad[i] == 1 {
            cluster_bad += 1;
        }
    }
    3 * cluster_bad + 5 * endpoint_bad
}

fn edge_violation_cost(u: usize, v: usize, n: usize) -> i32 {
    if is_adj(u, v, n) { 0 } else { 1 }
}

fn edge_rank_bad_cost(u: usize, v: usize, rank_by_cell: &[usize]) -> i32 {
    if is_rank_jump_bad(u, v, rank_by_cell) {
        1
    } else {
        0
    }
}

fn is_rank_jump_bad(u: usize, v: usize, rank_by_cell: &[usize]) -> bool {
    rank_by_cell[u].abs_diff(rank_by_cell[v]) > RANK_JUMP_THRESHOLD
}

fn is_valid_relocate(m: usize, l: usize, r: usize, p: usize) -> bool {
    if l == 0 || l > r || r + 1 >= m || p + 1 >= m {
        return false;
    }
    edge_disjoint_from_segment(p, l, r)
}

fn edge_disjoint_from_segment(p: usize, l: usize, r: usize) -> bool {
    !(l <= p && p <= r) && !(l <= p + 1 && p + 1 <= r)
}

#[derive(Clone, Copy)]
struct RelocationMove {
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
}

fn apply_relocation(path: &mut [usize], mv: RelocationMove) {
    let seg_len = mv.r - mv.l + 1;
    let mut seg = path[mv.l..=mv.r].to_vec();
    if mv.reversed {
        seg.reverse();
    }

    if mv.p < mv.l {
        path.copy_within(mv.p + 1..mv.l, mv.p + 1 + seg_len);
        path[mv.p + 1..mv.p + 1 + seg_len].copy_from_slice(&seg);
    } else {
        path.copy_within(mv.r + 1..=mv.p, mv.l);
        let q = mv.p + 1 - seg_len;
        path[q..q + seg_len].copy_from_slice(&seg);
    }
}

fn update_best_feasible(state: &State, best_path: &mut Vec<usize>, best_score: &mut i64) {
    if state.viol_cnt == 0 && state.raw_score > *best_score {
        *best_score = state.raw_score;
        *best_path = state.path.clone();
    }
}

fn compute_raw_score(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}

fn is_adj(u: usize, v: usize, n: usize) -> bool {
    let ui = u / n;
    let uj = u % n;
    let vi = v / n;
    let vj = v % n;
    ui.abs_diff(vi).max(uj.abs_diff(vj)) == 1
}

fn is_valid_route(route: &[usize], n: usize) -> bool {
    let m = route.len();
    let mut seen = vec![false; m];
    for &cell in route {
        if cell >= m || seen[cell] {
            return false;
        }
        seen[cell] = true;
    }
    for i in 0..m.saturating_sub(1) {
        if !is_adj(route[i], route[i + 1], n) {
            return false;
        }
    }
    true
}

fn build_baseline_feasible(n: usize, weights: &[i64]) -> Vec<usize> {
    let paths_by_pair = generate_local_paths();
    build_best_initial_route(n, weights, &paths_by_pair)
}

fn build_best_initial_route(
    n: usize,
    weights: &[i64],
    paths_by_pair: &[Vec<LocalPath>],
) -> Vec<usize> {
    let mut best_route = Vec::new();
    let mut best_score = NEG_INF;

    for symmetry in SYMMETRIES {
        let transformed = transform_weights(n, weights, symmetry);

        let mut band_route = build_banded_route(n, &transformed, paths_by_pair);
        let mut band_score = compute_raw_score(&band_route, &transformed);
        let band_rev_score = compute_raw_score_reversed(&band_route, &transformed);
        if band_rev_score > band_score {
            band_route.reverse();
            band_score = band_rev_score;
        }
        if band_score > best_score {
            best_score = band_score;
            best_route = map_route_back(&band_route, n, symmetry);
        }
    }

    best_route
}

fn generate_local_paths() -> Vec<Vec<LocalPath>> {
    let adj = build_local_neighbors();
    let mut paths_by_pair = vec![Vec::new(); BAND_H * BAND_H];
    for start_row in 0..BAND_H {
        let start = start_row * BLOCK_W;
        let mut path = [u8::MAX; BLOCK_CELLS];
        path[0] = start as u8;
        dfs_local_paths(1u16 << start, start, 1, &adj, &mut path, &mut paths_by_pair);
    }
    paths_by_pair
}

fn build_local_neighbors() -> [Vec<u8>; BLOCK_CELLS] {
    let mut adj: [Vec<u8>; BLOCK_CELLS] = std::array::from_fn(|_| Vec::new());
    for r in 0..BAND_H {
        for c in 0..BLOCK_W {
            let v = r * BLOCK_W + c;
            for dr in -1isize..=1 {
                for dc in -1isize..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= BAND_H as isize || nc < 0 || nc >= BLOCK_W as isize {
                        continue;
                    }
                    adj[v].push((nr as usize * BLOCK_W + nc as usize) as u8);
                }
            }
        }
    }
    adj
}

fn dfs_local_paths(
    mask: u16,
    last: usize,
    depth: usize,
    adj: &[Vec<u8>; BLOCK_CELLS],
    path: &mut LocalPath,
    paths_by_pair: &mut [Vec<LocalPath>],
) {
    if depth == BLOCK_CELLS {
        if last % BLOCK_W == BLOCK_W - 1 {
            let start_row = path[0] as usize / BLOCK_W;
            let end_row = last / BLOCK_W;
            paths_by_pair[start_row * BAND_H + end_row].push(*path);
        }
        return;
    }
    for &next in &adj[last] {
        let bit = 1u16 << next;
        if (mask & bit) != 0 {
            continue;
        }
        path[depth] = next;
        dfs_local_paths(
            mask | bit,
            next as usize,
            depth + 1,
            adj,
            path,
            paths_by_pair,
        );
    }
}

fn transform_weights(n: usize, weights: &[i64], symmetry: Symmetry) -> Vec<i64> {
    let mut transformed = vec![0; n * n];
    for i in 0..n {
        for j in 0..n {
            let (oi, oj) = map_from_transformed(i, j, n, symmetry);
            transformed[i * n + j] = weights[oi * n + oj];
        }
    }
    transformed
}

fn map_route_back(route: &[usize], n: usize, symmetry: Symmetry) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| {
            let i = cell / n;
            let j = cell % n;
            let (oi, oj) = map_from_transformed(i, j, n, symmetry);
            oi * n + oj
        })
        .collect()
}

fn map_from_transformed(row: usize, col: usize, n: usize, symmetry: Symmetry) -> (usize, usize) {
    let (mut r, mut c) = if symmetry.transpose {
        (col, row)
    } else {
        (row, col)
    };
    if symmetry.flip_row {
        r = n - 1 - r;
    }
    if symmetry.flip_col {
        c = n - 1 - c;
    }
    (r, c)
}

fn build_banded_route(n: usize, weights: &[i64], paths_by_pair: &[Vec<LocalPath>]) -> Vec<usize> {
    debug_assert_eq!(n % BAND_H, 0);
    debug_assert_eq!(n % BLOCK_W, 0);

    let band_count = n / BAND_H;
    let mut route = Vec::with_capacity(n * n);

    for band in 0..band_count {
        let dir_lr = band % 2 == 0;
        let band_row0 = band * BAND_H;
        let start_row = if band == 0 { None } else { Some(0usize) };
        let end_row = if band + 1 == band_count {
            None
        } else {
            Some(BAND_H - 1)
        };
        let band_route = solve_band(
            n,
            weights,
            band_row0,
            dir_lr,
            start_row,
            end_row,
            paths_by_pair,
        );
        route.extend(band_route);
    }

    route
}

fn solve_band(
    n: usize,
    weights: &[i64],
    band_row0: usize,
    dir_lr: bool,
    start_fixed: Option<usize>,
    end_fixed: Option<usize>,
    paths_by_pair: &[Vec<LocalPath>],
) -> Vec<usize> {
    let blocks = n / BLOCK_W;
    let mut evals = Vec::with_capacity(blocks);
    for step in 0..blocks {
        let block_col0 = if dir_lr {
            step * BLOCK_W
        } else {
            n - (step + 1) * BLOCK_W
        };
        let oriented = oriented_block_weights(n, weights, band_row0, block_col0, dir_lr);
        evals.push(evaluate_block(&oriented, paths_by_pair));
    }

    let mut dp = vec![[NEG_INF; BAND_H]; blocks];
    let mut parent_exit = vec![[u8::MAX; BAND_H]; blocks];
    let mut parent_start = vec![[u8::MAX; BAND_H]; blocks];

    for start_row in 0..BAND_H {
        if start_fixed.is_some_and(|fixed| fixed != start_row) {
            continue;
        }
        for end_row in 0..BAND_H {
            let pair = start_row * BAND_H + end_row;
            let local = evals[0].scores[pair];
            if local == NEG_INF {
                continue;
            }
            if local > dp[0][end_row] {
                dp[0][end_row] = local;
                parent_start[0][end_row] = start_row as u8;
            }
        }
    }

    for step in 1..blocks {
        for prev_exit_row in 0..BAND_H {
            let cur = dp[step - 1][prev_exit_row];
            if cur == NEG_INF {
                continue;
            }
            let min_start = prev_exit_row.saturating_sub(1);
            let max_start = (prev_exit_row + 1).min(BAND_H - 1);
            for start_row in min_start..=max_start {
                for end_row in 0..BAND_H {
                    let pair = start_row * BAND_H + end_row;
                    let local = evals[step].scores[pair];
                    if local == NEG_INF {
                        continue;
                    }
                    let cand = cur + local;
                    if cand > dp[step][end_row] {
                        dp[step][end_row] = cand;
                        parent_exit[step][end_row] = prev_exit_row as u8;
                        parent_start[step][end_row] = start_row as u8;
                    }
                }
            }
        }
    }

    let last = blocks - 1;
    let mut best_end_row = usize::MAX;
    let mut best_score = NEG_INF;
    for end_row in 0..BAND_H {
        if end_fixed.is_some_and(|fixed| fixed != end_row) {
            continue;
        }
        if dp[last][end_row] > best_score {
            best_score = dp[last][end_row];
            best_end_row = end_row;
        }
    }
    assert!(best_end_row != usize::MAX);

    let mut chosen_starts = vec![0usize; blocks];
    let mut chosen_ends = vec![0usize; blocks];
    let mut exit_row = best_end_row;
    for step in (0..blocks).rev() {
        chosen_ends[step] = exit_row;
        chosen_starts[step] = parent_start[step][exit_row] as usize;
        if step > 0 {
            exit_row = parent_exit[step][exit_row] as usize;
        }
    }

    let mut band_route = Vec::with_capacity(BAND_H * n);
    for step in 0..blocks {
        let block_col0 = if dir_lr {
            step * BLOCK_W
        } else {
            n - (step + 1) * BLOCK_W
        };
        let start_row = chosen_starts[step];
        let end_row = chosen_ends[step];
        let pair = start_row * BAND_H + end_row;
        let choice = evals[step].choices[pair] as usize;
        let path = &paths_by_pair[pair][choice];
        for &local in path {
            let local = local as usize;
            let row = local / BLOCK_W;
            let col = local % BLOCK_W;
            let global_col = if dir_lr {
                block_col0 + col
            } else {
                block_col0 + (BLOCK_W - 1 - col)
            };
            band_route.push((band_row0 + row) * n + global_col);
        }
    }
    band_route
}

fn oriented_block_weights(
    n: usize,
    weights: &[i64],
    band_row0: usize,
    block_col0: usize,
    dir_lr: bool,
) -> [i64; BLOCK_CELLS] {
    let mut oriented = [0i64; BLOCK_CELLS];
    for row in 0..BAND_H {
        for col in 0..BLOCK_W {
            let global_col = if dir_lr {
                block_col0 + col
            } else {
                block_col0 + (BLOCK_W - 1 - col)
            };
            oriented[row * BLOCK_W + col] = weights[(band_row0 + row) * n + global_col];
        }
    }
    oriented
}

fn evaluate_block(
    oriented_weights: &[i64; BLOCK_CELLS],
    paths_by_pair: &[Vec<LocalPath>],
) -> BlockEval {
    let mut scores = [NEG_INF; BAND_H * BAND_H];
    let mut choices = [u16::MAX; BAND_H * BAND_H];
    for pair in 0..BAND_H * BAND_H {
        for (idx, path) in paths_by_pair[pair].iter().enumerate() {
            let mut local_score = 0i64;
            for pos in 0..BLOCK_CELLS {
                local_score += pos as i64 * oriented_weights[path[pos] as usize];
            }
            if local_score > scores[pair] {
                scores[pair] = local_score;
                choices[pair] = idx as u16;
            }
        }
    }
    BlockEval { scores, choices }
}

fn compute_raw_score_reversed(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .rev()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}
