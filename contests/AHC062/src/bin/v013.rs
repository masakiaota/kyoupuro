//! # v013
//! open-path endpoint-biased annealing

use proconio::input;
use rand::prelude::*;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::time::{Duration, Instant};

const TIME_LIMIT: f64 = 2.70;
const ENDPOINT_K: usize = 4;
const ENDPOINT_BONUS_MAX: i64 = 20_000_000;
const ENDPOINT_HIT_PRIORITY: i64 = 100;
const START_TEMP_FALLBACK: f64 = 50_000_000.0;
const END_TEMP: f64 = 1_500.0;
const MAX_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;
const RELOCATE_TRIES: usize = 10;
const LONG_RELOCATE_TRIES: usize = 8;
const SWAP_TRIES: usize = 8;
const ENDPOINT_TRIES: usize = 6;
const RELOCATE_GUIDE_SAMPLES: usize = 6;
const LONG_RELOCATE_GUIDE_SAMPLES: usize = 10;

#[derive(Clone, Copy)]
struct Symmetry {
    transpose: bool,
    flip_row: bool,
    flip_col: bool,
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

#[derive(Clone, Copy)]
enum MoveKind {
    Relocate { l: usize, r: usize, q: usize },
    Swap { l1: usize, l2: usize, len: usize },
    Reverse { l: usize, r: usize },
}

#[derive(Clone, Copy)]
enum MoveClass {
    Relocate,
    Swap,
    Reverse,
    LongRelocate,
    Endpoint,
}

#[derive(Clone, Copy)]
struct Candidate {
    mv: MoveKind,
    class: MoveClass,
    delta_raw: i64,
    delta_endpoint_hits: i64,
    delta_endpoint_points: i64,
}

struct DiagStats {
    sampled_none: u64,
    attempted: [u64; 5],
    accepted: [u64; 5],
    improved_best: [u64; 5],
    final_attempted: [u64; 5],
    final_accepted: [u64; 5],
    final_improved_best: [u64; 5],
}

impl DiagStats {
    fn new() -> Self {
        Self {
            sampled_none: 0,
            attempted: [0; 5],
            accepted: [0; 5],
            improved_best: [0; 5],
            final_attempted: [0; 5],
            final_accepted: [0; 5],
            final_improved_best: [0; 5],
        }
    }
}

struct State {
    path: Vec<usize>,
    pos: Vec<usize>,
    prefix_weight: Vec<i64>,
    prefix_pos_weight: Vec<i64>,
    prefix_rank_error: Vec<i64>,
    raw_score: i64,
    endpoint_hits: i64,
    endpoint_points: i64,
}

impl State {
    fn new(path: Vec<usize>, weights: &[i64], start_bonus: &[i64], end_bonus: &[i64]) -> Self {
        let m = path.len();
        let mut state = Self {
            path,
            pos: vec![0; m],
            prefix_weight: vec![0; m + 1],
            prefix_pos_weight: vec![0; m + 1],
            prefix_rank_error: vec![0; m + 1],
            raw_score: 0,
            endpoint_hits: 0,
            endpoint_points: 0,
        };
        state.rebuild(weights, start_bonus, end_bonus);
        state
    }

    fn rebuild(&mut self, weights: &[i64], start_bonus: &[i64], end_bonus: &[i64]) {
        self.raw_score = 0;
        self.prefix_weight[0] = 0;
        self.prefix_pos_weight[0] = 0;
        self.prefix_rank_error[0] = 0;
        for (idx, &cell) in self.path.iter().enumerate() {
            self.pos[cell] = idx;
            self.raw_score += idx as i64 * weights[cell];
            self.prefix_weight[idx + 1] = self.prefix_weight[idx] + weights[cell];
            self.prefix_pos_weight[idx + 1] =
                self.prefix_pos_weight[idx] + idx as i64 * weights[cell];
            let desired = weights[cell] - 1;
            self.prefix_rank_error[idx + 1] =
                self.prefix_rank_error[idx] + (idx as i64 - desired).abs();
        }
        let end_cell = *self.path.last().unwrap_or(&self.path[0]);
        self.endpoint_hits =
            endpoint_hits_from_cells(start_bonus, end_bonus, self.path[0], end_cell);
        self.endpoint_points = start_bonus[self.path[0]] + end_bonus[end_cell];
    }
}

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let diag = std::env::var_os("V013_DIAG").is_some()
        || std::env::var_os("V012_DIAG").is_some()
        || std::env::var_os("V011_DIAG").is_some()
        || std::env::var_os("V010_DIAG").is_some();
    let weights = flatten_weights(n, &a);
    let neighbors = build_neighbors(n);
    let (start_bonus, end_bonus, low_cells, high_cells) = build_endpoint_bonuses(&weights);
    let start = Instant::now();
    let hard_deadline = Duration::from_secs_f64(TIME_LIMIT);
    let initial_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.18);
    let sa_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.92);

    let mut initial_path = choose_initial_path(n, &weights, &start_bonus, &end_bonus);
    run_window_descent(
        &mut initial_path,
        &weights,
        n,
        &start,
        initial_deadline.min(hard_deadline),
    );
    let initial_elapsed = start.elapsed();

    let mut state = State::new(initial_path, &weights, &start_bonus, &end_bonus);
    let mut best_path = state.path.clone();
    let mut best_key = state_key(&state);

    let seed = build_seed(&weights);
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    let mut diag_stats = DiagStats::new();
    let start_temp = estimate_start_temp(
        n,
        &state,
        &weights,
        &neighbors,
        &start_bonus,
        &end_bonus,
        &low_cells,
        &high_cells,
        &mut rng,
    );

    while start.elapsed() < sa_deadline {
        let progress = (start.elapsed().as_secs_f64() / sa_deadline.as_secs_f64()).clamp(0.0, 1.0);
        let bonus_weight = endpoint_bonus_weight(progress);
        let temp = schedule_temp(start_temp, progress);

        let candidate = sample_candidate(
            n,
            &state,
            &weights,
            &neighbors,
            &start_bonus,
            &end_bonus,
            &low_cells,
            &high_cells,
            progress,
            &mut rng,
        );
        let Some(cand) = candidate else {
            diag_stats.sampled_none += 1;
            continue;
        };
        diag_stats.attempted[move_class_index(cand.class)] += 1;
        let delta_obj = cand.delta_raw as f64 + bonus_weight * endpoint_metric_delta(&cand) as f64;
        if should_accept(delta_obj, temp, &mut rng) {
            apply_move(&mut state.path, cand.mv);
            state.rebuild(&weights, &start_bonus, &end_bonus);
            diag_stats.accepted[move_class_index(cand.class)] += 1;
            let cur_key = state_key(&state);
            if cur_key > best_key {
                best_key = cur_key;
                best_path = state.path.clone();
                diag_stats.improved_best[move_class_index(cand.class)] += 1;
            }
        }
    }

    let sa_elapsed = start.elapsed();
    let mut final_state = State::new(best_path.clone(), &weights, &start_bonus, &end_bonus);
    run_window_descent(
        &mut final_state.path,
        &weights,
        n,
        &start,
        Duration::from_secs_f64((TIME_LIMIT * 0.97).min(TIME_LIMIT)),
    );
    final_state.rebuild(&weights, &start_bonus, &end_bonus);
    let final_key = state_key(&final_state);
    if final_key > best_key {
        best_key = final_key;
        best_path = final_state.path.clone();
    }
    let final_deadline = hard_deadline;
    while start.elapsed() < final_deadline {
        let candidate = sample_candidate(
            n,
            &final_state,
            &weights,
            &neighbors,
            &start_bonus,
            &end_bonus,
            &low_cells,
            &high_cells,
            1.0,
            &mut rng,
        );
        let Some(cand) = candidate else {
            diag_stats.sampled_none += 1;
            continue;
        };
        diag_stats.final_attempted[move_class_index(cand.class)] += 1;
        let delta_endpoint = endpoint_metric_delta(&cand);
        if delta_endpoint < 0 {
            continue;
        }
        if delta_endpoint == 0 && cand.delta_raw <= 0 {
            continue;
        }
        apply_move(&mut final_state.path, cand.mv);
        final_state.rebuild(&weights, &start_bonus, &end_bonus);
        diag_stats.final_accepted[move_class_index(cand.class)] += 1;
        let cur_key = state_key(&final_state);
        if cur_key > best_key {
            best_key = cur_key;
            best_path = final_state.path.clone();
            diag_stats.final_improved_best[move_class_index(cand.class)] += 1;
        }
    }

    if diag {
        emit_diag(
            &diag_stats,
            initial_elapsed,
            &state,
            &final_state,
            sa_elapsed,
            start.elapsed(),
            start_temp,
            best_key,
        );
    }

    for cell in best_path {
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
    let mut neighbors = vec![Vec::with_capacity(8); n * n];
    for r in 0..n {
        for c in 0..n {
            let cell = r * n + c;
            for dr in -1isize..=1 {
                for dc in -1isize..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                        continue;
                    }
                    neighbors[cell].push(nr as usize * n + nc as usize);
                }
            }
        }
    }
    neighbors
}

fn build_endpoint_bonuses(weights: &[i64]) -> (Vec<i64>, Vec<i64>, Vec<usize>, Vec<usize>) {
    let m = weights.len();
    let mut start_bonus = vec![0i64; m];
    let mut end_bonus = vec![0i64; m];
    let mut low_cells = Vec::new();
    let mut high_cells = Vec::new();
    for (cell, &w) in weights.iter().enumerate() {
        let rank = (w - 1) as usize;
        if rank < ENDPOINT_K {
            let bonus = (ENDPOINT_K - rank) as i64;
            start_bonus[cell] = bonus;
            low_cells.push(cell);
        }
        let rev_rank = m - 1 - rank;
        if rev_rank < ENDPOINT_K {
            let bonus = (ENDPOINT_K - rev_rank) as i64;
            end_bonus[cell] = bonus;
            high_cells.push(cell);
        }
    }
    (start_bonus, end_bonus, low_cells, high_cells)
}

fn choose_initial_path(
    n: usize,
    weights: &[i64],
    start_bonus: &[i64],
    end_bonus: &[i64],
) -> Vec<usize> {
    let bases = vec![
        row_snake_path(n),
        col_snake_path(n),
        diag_snake_path(n),
        two_row_value_path(n, weights),
    ];
    let mut best_route = Vec::new();
    let mut best_score = i64::MIN;
    for base in bases {
        for &symmetry in &SYMMETRIES {
            let route = apply_symmetry_to_route(&base, n, symmetry);
            let mut candidates = [route.clone(), reverse_path(&route)];
            for candidate in &mut candidates {
                let end_cell = *candidate.last().unwrap_or(&candidate[0]);
                let endpoint_hits =
                    endpoint_hits_from_cells(start_bonus, end_bonus, candidate[0], end_cell);
                let endpoint_points = start_bonus[candidate[0]] + end_bonus[end_cell];
                let score = compute_raw_score(candidate, weights)
                    + ENDPOINT_BONUS_MAX * endpoint_metric(endpoint_hits, endpoint_points);
                if score > best_score {
                    best_score = score;
                    best_route = candidate.clone();
                }
            }
        }
    }
    best_route
}

fn row_snake_path(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for r in 0..n {
        if r % 2 == 0 {
            for c in 0..n {
                route.push(r * n + c);
            }
        } else {
            for c in (0..n).rev() {
                route.push(r * n + c);
            }
        }
    }
    route
}

fn col_snake_path(n: usize) -> Vec<usize> {
    let row = row_snake_path(n);
    row.iter().map(|&cell| (cell % n) * n + cell / n).collect()
}

fn diag_snake_path(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for sum in 0..=(2 * (n - 1)) {
        let row_min = sum.saturating_sub(n - 1);
        let row_max = sum.min(n - 1);
        if sum % 2 == 0 {
            for row in (row_min..=row_max).rev() {
                route.push(row * n + (sum - row));
            }
        } else {
            for row in row_min..=row_max {
                route.push(row * n + (sum - row));
            }
        }
    }
    route
}

fn two_row_value_path(n: usize, weights: &[i64]) -> Vec<usize> {
    let mut path = Vec::with_capacity(n * n);
    for strip in 0..(n / 2) {
        let r0 = strip * 2;
        let r1 = r0 + 1;
        let mut cols: Vec<usize> = (0..n).collect();
        if strip % 2 == 1 {
            cols.reverse();
        }
        for (idx, c) in cols.into_iter().enumerate() {
            let top = r0 * n + c;
            let bottom = r1 * n + c;
            let endpoint = idx == 0 || idx + 1 == n;
            if endpoint || weights[top] <= weights[bottom] {
                path.push(top);
                path.push(bottom);
            } else {
                path.push(bottom);
                path.push(top);
            }
        }
    }
    path
}

fn reverse_path(route: &[usize]) -> Vec<usize> {
    let mut out = route.to_vec();
    out.reverse();
    out
}

fn apply_symmetry_to_route(route: &[usize], n: usize, symmetry: Symmetry) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| {
            let row = cell / n;
            let col = cell % n;
            let (r, c) = transform_cell(row, col, n, symmetry);
            r * n + c
        })
        .collect()
}

fn transform_cell(row: usize, col: usize, n: usize, symmetry: Symmetry) -> (usize, usize) {
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

fn compute_raw_score(path: &[usize], weights: &[i64]) -> i64 {
    path.iter()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}

fn run_window_descent(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    start: &Instant,
    time_limit: Duration,
) {
    let mut total_score = compute_raw_score(route, weights);
    let mut cycle = 0usize;
    let window_sizes = [9usize, 8usize];
    while start.elapsed() < time_limit {
        let mut improved = false;
        for &window in &window_sizes {
            for offset in 0..window {
                if start.elapsed() >= time_limit {
                    break;
                }
                let mut pos = offset;
                while pos < route.len() {
                    let len = (route.len() - pos).min(window);
                    if len >= 2 && optimize_window(route, weights, n, pos, len, &mut total_score) {
                        improved = true;
                    }
                    pos += window;
                }
            }
        }
        cycle += 1;
        if !improved && cycle >= 2 {
            break;
        }
    }
}

fn optimize_window(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    start: usize,
    len: usize,
    total_score: &mut i64,
) -> bool {
    let mut seg = [0usize; MAX_WINDOW];
    for idx in 0..len {
        seg[idx] = route[start + idx];
    }

    let left = if start > 0 {
        Some(route[start - 1])
    } else {
        None
    };
    let right = if start + len < route.len() {
        Some(route[start + len])
    } else {
        None
    };

    let old_local = (0..len)
        .map(|idx| (start + idx) as i64 * weights[seg[idx]])
        .sum::<i64>();

    let states = 1usize << len;
    let width = len;
    let mut dp = vec![NEG_INF; states * width];
    let mut parent = vec![u8::MAX; states * width];

    for first in 0..len {
        if left.is_none_or(|cell| is_adj(cell, seg[first], n)) {
            let mask = 1usize << first;
            dp[mask * width + first] = start as i64 * weights[seg[first]];
        }
    }

    for mask in 1usize..states {
        let used = mask.count_ones() as usize;
        let next_pos = start + used;
        for last in 0..len {
            let cur = dp[mask * width + last];
            if cur == NEG_INF || (mask & (1usize << last)) == 0 {
                continue;
            }
            for nxt in 0..len {
                if (mask & (1usize << nxt)) != 0 || !is_adj(seg[last], seg[nxt], n) {
                    continue;
                }
                let next_mask = mask | (1usize << nxt);
                let next_value = cur + next_pos as i64 * weights[seg[nxt]];
                let slot = next_mask * width + nxt;
                if next_value > dp[slot] {
                    dp[slot] = next_value;
                    parent[slot] = last as u8;
                }
            }
        }
    }

    let full = states - 1;
    let mut best_last = usize::MAX;
    let mut best_local = old_local;
    for last in 0..len {
        let value = dp[full * width + last];
        if value == NEG_INF {
            continue;
        }
        if right.is_some_and(|cell| !is_adj(seg[last], cell, n)) {
            continue;
        }
        if value > best_local {
            best_local = value;
            best_last = last;
        }
    }

    if best_last == usize::MAX {
        return false;
    }

    let mut order = [0usize; MAX_WINDOW];
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
    for idx in 0..len {
        let cell = seg[order[idx]];
        if route[start + idx] != cell {
            route[start + idx] = cell;
            changed = true;
        }
    }

    if changed {
        *total_score += best_local - old_local;
    }
    changed
}

fn build_seed(weights: &[i64]) -> u64 {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for &w in weights.iter().step_by(97) {
        seed ^= (w as u64).wrapping_mul(0xA076_1D64_78BD_642F);
        seed = seed.rotate_left(9);
    }
    seed
}

fn endpoint_hits_from_cells(
    start_bonus: &[i64],
    end_bonus: &[i64],
    start_cell: usize,
    end_cell: usize,
) -> i64 {
    (start_bonus[start_cell] > 0) as i64 + (end_bonus[end_cell] > 0) as i64
}

fn endpoint_metric(endpoint_hits: i64, endpoint_points: i64) -> i64 {
    endpoint_hits * ENDPOINT_HIT_PRIORITY + endpoint_points
}

fn endpoint_metric_delta(cand: &Candidate) -> i64 {
    endpoint_metric(cand.delta_endpoint_hits, cand.delta_endpoint_points)
}

fn state_key(state: &State) -> (i64, i64, i64) {
    (state.endpoint_hits, state.endpoint_points, state.raw_score)
}

fn endpoint_bonus_weight(progress: f64) -> f64 {
    let remain = (1.0 - progress).max(0.0);
    (ENDPOINT_BONUS_MAX as f64) * remain * remain
}

fn schedule_temp(start_temp: f64, progress: f64) -> f64 {
    let ratio = (END_TEMP / start_temp.max(1.0)).powf(progress.clamp(0.0, 1.0));
    start_temp * ratio
}

fn should_accept(delta_obj: f64, temperature: f64, rng: &mut Xoshiro256PlusPlus) -> bool {
    if delta_obj >= 0.0 {
        return true;
    }
    let temp = temperature.max(1e-9);
    let prob = (delta_obj / temp).exp();
    rng.random::<f64>() < prob
}

fn move_class_index(class: MoveClass) -> usize {
    match class {
        MoveClass::Relocate => 0,
        MoveClass::Swap => 1,
        MoveClass::Reverse => 2,
        MoveClass::LongRelocate => 3,
        MoveClass::Endpoint => 4,
    }
}

fn move_class_name(class_idx: usize) -> &'static str {
    match class_idx {
        0 => "relocate",
        1 => "swap",
        2 => "reverse",
        3 => "long_relocate",
        4 => "endpoint",
        _ => "unknown",
    }
}

fn emit_diag(
    stats: &DiagStats,
    initial_elapsed: Duration,
    sa_state: &State,
    final_state: &State,
    sa_elapsed: Duration,
    total_elapsed: Duration,
    start_temp: f64,
    best_key: (i64, i64, i64),
) {
    let sa_attempted: u64 = stats.attempted.iter().sum();
    let sa_accepted: u64 = stats.accepted.iter().sum();
    let sa_best: u64 = stats.improved_best.iter().sum();
    let final_attempted: u64 = stats.final_attempted.iter().sum();
    let final_accepted: u64 = stats.final_accepted.iter().sum();
    let final_best: u64 = stats.final_improved_best.iter().sum();
    let sa_secs = (sa_elapsed.as_secs_f64() - initial_elapsed.as_secs_f64()).max(0.0);
    let sa_ips = if sa_secs > 0.0 {
        sa_attempted as f64 / sa_secs
    } else {
        0.0
    };
    eprintln!(
        "diag time initial={:.3}s sa_end={:.3}s total={:.3}s sa_window={:.3}s start_temp={:.1}",
        initial_elapsed.as_secs_f64(),
        sa_elapsed.as_secs_f64(),
        total_elapsed.as_secs_f64(),
        sa_secs,
        start_temp,
    );
    eprintln!(
        "diag score sa_current=({},{},{}) final_current=({},{},{}) best=({},{},{})",
        sa_state.endpoint_hits,
        sa_state.endpoint_points,
        sa_state.raw_score,
        final_state.endpoint_hits,
        final_state.endpoint_points,
        final_state.raw_score,
        best_key.0,
        best_key.1,
        best_key.2
    );
    eprintln!(
        "diag sa attempted={} accepted={} best_updates={} accept_rate={:.4} iter_per_sec={:.0} sampled_none={}",
        sa_attempted,
        sa_accepted,
        sa_best,
        ratio(sa_accepted, sa_attempted),
        sa_ips,
        stats.sampled_none,
    );
    for idx in 0..5 {
        eprintln!(
            "diag sa move={} attempted={} accepted={} best_updates={} accept_rate={:.4}",
            move_class_name(idx),
            stats.attempted[idx],
            stats.accepted[idx],
            stats.improved_best[idx],
            ratio(stats.accepted[idx], stats.attempted[idx]),
        );
    }
    eprintln!(
        "diag final attempted={} accepted={} best_updates={} accept_rate={:.4}",
        final_attempted,
        final_accepted,
        final_best,
        ratio(final_accepted, final_attempted),
    );
    for idx in 0..5 {
        eprintln!(
            "diag final move={} attempted={} accepted={} best_updates={} accept_rate={:.4}",
            move_class_name(idx),
            stats.final_attempted[idx],
            stats.final_accepted[idx],
            stats.final_improved_best[idx],
            ratio(stats.final_accepted[idx], stats.final_attempted[idx]),
        );
    }
}

fn ratio(num: u64, den: u64) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

fn estimate_start_temp(
    n: usize,
    state: &State,
    weights: &[i64],
    neighbors: &[Vec<usize>],
    start_bonus: &[i64],
    end_bonus: &[i64],
    low_cells: &[usize],
    high_cells: &[usize],
    rng: &mut Xoshiro256PlusPlus,
) -> f64 {
    let mut samples = Vec::new();
    for _ in 0..192 {
        let candidate = sample_candidate(
            n,
            state,
            weights,
            neighbors,
            start_bonus,
            end_bonus,
            low_cells,
            high_cells,
            0.0,
            rng,
        );
        if let Some(cand) = candidate {
            let delta_obj = cand.delta_raw.unsigned_abs() as f64
                + (ENDPOINT_BONUS_MAX as f64) * (endpoint_metric_delta(&cand).abs() as f64);
            samples.push(delta_obj);
        }
    }
    if samples.is_empty() {
        return START_TEMP_FALLBACK;
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    let q = samples[samples.len() * 3 / 4].max(1.0);
    q * 1.25
}

fn sample_candidate(
    n: usize,
    state: &State,
    weights: &[i64],
    neighbors: &[Vec<usize>],
    start_bonus: &[i64],
    end_bonus: &[i64],
    low_cells: &[usize],
    high_cells: &[usize],
    progress: f64,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let roll = rng.random::<f64>();
    let endpoint_cut = if progress < 0.35 {
        0.94
    } else if progress < 0.75 {
        0.88
    } else {
        0.78
    };
    let max_seg = if progress < 0.30 {
        7
    } else if progress < 0.75 {
        5
    } else {
        4
    };
    let max_span = if progress < 0.25 {
        320
    } else if progress < 0.60 {
        160
    } else {
        72
    };
    if roll < 0.46 {
        sample_relocate(
            n,
            state,
            weights,
            neighbors,
            start_bonus,
            end_bonus,
            max_seg,
            max_span,
            rng,
        )
    } else if roll < 0.62 {
        sample_swap(
            n,
            state,
            weights,
            start_bonus,
            end_bonus,
            max_seg.min(4),
            max_span,
            rng,
        )
    } else if roll < 0.76 {
        sample_reverse(n, state, start_bonus, end_bonus, max_span.min(48), rng)
    } else if roll < endpoint_cut {
        sample_long_relocate(
            n,
            state,
            weights,
            neighbors,
            start_bonus,
            end_bonus,
            if progress < 0.35 {
                24
            } else if progress < 0.75 {
                16
            } else {
                12
            },
            if progress < 0.35 {
                640
            } else if progress < 0.75 {
                320
            } else {
                144
            },
            rng,
        )
    } else {
        sample_endpoint_relocate(
            n,
            state,
            weights,
            neighbors,
            start_bonus,
            end_bonus,
            low_cells,
            high_cells,
            rng,
        )
    }
}

fn push_unique_q(candidates: &mut Vec<usize>, q: usize) {
    if !candidates.contains(&q) {
        candidates.push(q);
    }
}

fn segment_target_q(state: &State, l: usize, r: usize) -> usize {
    let m = state.path.len();
    let len = r - l + 1;
    let seg_sum = sum_range(&state.prefix_weight, l, r);
    let avg_pos = ((seg_sum / len as i64) - 1).clamp(0, (m - 1) as i64) as usize;
    avg_pos.saturating_sub(len / 2).min(m - len)
}

fn select_guided_segment(
    state: &State,
    min_len: usize,
    max_len: usize,
    max_span: usize,
    sample_count: usize,
    long_bias: bool,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<(usize, usize, usize)> {
    let m = state.path.len();
    let seg_hi = max_len.min(m.saturating_sub(1));
    if min_len == 0 || seg_hi < min_len {
        return None;
    }
    let mut best = None;
    let mut best_key = i64::MIN;
    for _ in 0..sample_count {
        let len = rng.random_range(min_len..=seg_hi);
        let l = rng.random_range(0..=m - len);
        let r = l + len - 1;
        let target_q = segment_target_q(state, l, r);
        let rank_error = sum_range(&state.prefix_rank_error, l, r) / len as i64;
        let move_gap = target_q.abs_diff(l).min(max_span) as i64;
        let key = if long_bias {
            rank_error + 2 * move_gap
        } else {
            rank_error + move_gap
        };
        if key > best_key {
            best_key = key;
            best = Some((l, r, target_q));
        }
    }
    best
}

fn collect_legal_relocate_positions(
    n: usize,
    state: &State,
    neighbors: &[Vec<usize>],
    l: usize,
    r: usize,
    out: &mut Vec<usize>,
) {
    out.clear();
    let m = state.path.len();
    let len = r - l + 1;
    if len >= m {
        return;
    }
    if l > 0 && r + 1 < m && !is_adj(state.path[l - 1], state.path[r + 1], n) {
        return;
    }

    let first = state.path[l];
    let last = state.path[r];

    if l > 0 && is_adj(last, state.path[0], n) {
        push_unique_q(out, 0);
    }

    for &right_before in &neighbors[last] {
        let q = state.pos[right_before];
        if q == 0 || q >= l {
            continue;
        }
        let left_before = state.path[q - 1];
        if is_adj(left_before, first, n) {
            push_unique_q(out, q);
        }
    }

    if r + 1 < m && l != m - len && is_adj(state.path[m - 1], first, n) {
        push_unique_q(out, m - len);
    }

    for &before_insert in &neighbors[first] {
        let pos_before = state.pos[before_insert];
        if pos_before < r || pos_before + 1 >= m {
            continue;
        }
        let next_idx = pos_before + 1;
        let right_after = state.path[next_idx];
        if !is_adj(last, right_after, n) {
            continue;
        }
        let q = next_idx - len;
        if q == l {
            continue;
        }
        push_unique_q(out, q);
    }
}

fn sample_relocate(
    n: usize,
    state: &State,
    _weights: &[i64],
    neighbors: &[Vec<usize>],
    start_bonus: &[i64],
    end_bonus: &[i64],
    max_seg: usize,
    max_span: usize,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let m = state.path.len();
    let mut best = None;
    let mut best_key = i64::MIN;
    let mut legal_qs = Vec::with_capacity(32);
    for _ in 0..RELOCATE_TRIES {
        let guided = rng.random_bool(0.12);
        let (l, r, target_q) = if guided {
            let Some(seg) = select_guided_segment(
                state,
                1,
                max_seg,
                max_span,
                RELOCATE_GUIDE_SAMPLES,
                false,
                rng,
            ) else {
                continue;
            };
            seg
        } else {
            let len = rng.random_range(1..=max_seg.min(m.saturating_sub(1)));
            let l = rng.random_range(0..=m - len);
            let r = l + len - 1;
            let target_q = segment_target_q(state, l, r);
            (l, r, target_q)
        };
        collect_legal_relocate_positions(n, state, neighbors, l, r, &mut legal_qs);
        if legal_qs.is_empty() {
            continue;
        }
        legal_qs.sort_unstable_by_key(|&q| {
            let d_target = q.abs_diff(target_q);
            let d_local = q.abs_diff(l).min(max_span);
            d_target.min(d_local)
        });
        for &q in legal_qs.iter().take(8) {
            if let Some(cand) =
                evaluate_relocate(n, state, _weights, start_bonus, end_bonus, l, r, q)
            {
                let key = cand.delta_raw + 8_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
        for _ in 0..4.min(legal_qs.len()) {
            let q = legal_qs[rng.random_range(0..legal_qs.len())];
            if let Some(cand) =
                evaluate_relocate(n, state, _weights, start_bonus, end_bonus, l, r, q)
            {
                let key = cand.delta_raw + 8_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
    }
    best
}

fn sample_swap(
    n: usize,
    state: &State,
    weights: &[i64],
    start_bonus: &[i64],
    end_bonus: &[i64],
    max_seg: usize,
    max_span: usize,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let m = state.path.len();
    if m < 4 {
        return None;
    }
    let mut best = None;
    let mut best_key = i64::MIN;
    for _ in 0..SWAP_TRIES {
        let len = rng.random_range(1..=max_seg.min((m / 2).max(1)));
        if 2 * len >= m {
            continue;
        }
        let l1 = rng.random_range(0..=m - 2 * len);
        let r1 = l1 + len - 1;
        let seg_sum = sum_range(&state.prefix_weight, l1, r1);
        let avg_pos = ((seg_sum / len as i64) - 1).clamp(0, (m - 1) as i64) as usize;
        let target_l2 = avg_pos.saturating_sub(len / 2).clamp(l1 + len, m - len);
        let mut starts = [
            target_l2,
            l1.saturating_add(max_span).min(m - len),
            l1.saturating_sub(max_span),
        ];
        starts[2] = starts[2].min(m - len);
        for &base in &starts {
            let mut l2 = base;
            if l2 + len <= l1 {
                l2 = l1 + len;
            }
            if l2 < l1 + len {
                l2 = l1 + len;
            }
            if l2 + len > m {
                continue;
            }
            if let Some(cand) =
                evaluate_swap(n, state, weights, start_bonus, end_bonus, l1, l2, len)
            {
                let key = cand.delta_raw + 8_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
        for _ in 0..4 {
            let lo = l1.saturating_add(len).saturating_sub(max_span).min(m - len);
            let hi = (l1 + len + max_span).min(m - len);
            if lo > hi {
                continue;
            }
            let l2 = rng.random_range(lo..=hi).max(l1 + len);
            if l2 + len > m {
                continue;
            }
            if let Some(cand) =
                evaluate_swap(n, state, weights, start_bonus, end_bonus, l1, l2, len)
            {
                let key = cand.delta_raw + 8_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
    }
    best
}

fn sample_long_relocate(
    n: usize,
    state: &State,
    _weights: &[i64],
    neighbors: &[Vec<usize>],
    start_bonus: &[i64],
    end_bonus: &[i64],
    max_seg: usize,
    max_span: usize,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let m = state.path.len();
    if m < 8 {
        return None;
    }
    let mut best = None;
    let mut best_key = i64::MIN;
    let mut legal_qs = Vec::with_capacity(32);
    let seg_hi = max_seg.min(state.path.len().saturating_sub(1));
    let seg_lo = seg_hi.min(8);
    if seg_lo > seg_hi {
        return None;
    }
    for _ in 0..LONG_RELOCATE_TRIES {
        let guided = rng.random_bool(0.20);
        let (l, r, target_q) = if guided {
            let Some(seg) = select_guided_segment(
                state,
                seg_lo,
                seg_hi,
                max_span,
                LONG_RELOCATE_GUIDE_SAMPLES,
                true,
                rng,
            ) else {
                continue;
            };
            seg
        } else {
            let len = rng.random_range(seg_lo..=seg_hi);
            let l = rng.random_range(0..=m - len);
            let r = l + len - 1;
            let target_q = segment_target_q(state, l, r);
            (l, r, target_q)
        };
        collect_legal_relocate_positions(n, state, neighbors, l, r, &mut legal_qs);
        if legal_qs.is_empty() {
            continue;
        }
        legal_qs.sort_unstable_by_key(|&q| {
            let d_target = q.abs_diff(target_q);
            let d_local = q.abs_diff(l).min(max_span);
            d_target.min(d_local)
        });
        for &q in legal_qs.iter().take(10) {
            if let Some(cand) =
                evaluate_relocate(n, state, _weights, start_bonus, end_bonus, l, r, q)
            {
                let cand = Candidate {
                    class: MoveClass::LongRelocate,
                    ..cand
                };
                let key = cand.delta_raw + 10_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
        for _ in 0..4.min(legal_qs.len()) {
            let q = legal_qs[rng.random_range(0..legal_qs.len())];
            if let Some(cand) =
                evaluate_relocate(n, state, _weights, start_bonus, end_bonus, l, r, q)
            {
                let cand = Candidate {
                    class: MoveClass::LongRelocate,
                    ..cand
                };
                let key = cand.delta_raw + 10_000 * endpoint_metric_delta(&cand);
                if key > best_key {
                    best_key = key;
                    best = Some(cand);
                }
            }
        }
    }
    best
}

fn sample_reverse(
    n: usize,
    state: &State,
    start_bonus: &[i64],
    end_bonus: &[i64],
    max_span: usize,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let m = state.path.len();
    if m < 3 {
        return None;
    }
    let mut best = None;
    let mut best_key = i64::MIN;
    for _ in 0..8 {
        let l = rng.random_range(0..m - 1);
        let max_r = (l + max_span).min(m - 1);
        if l >= max_r {
            continue;
        }
        let r = rng.random_range(l + 1..=max_r);
        if let Some(cand) = evaluate_reverse(n, state, start_bonus, end_bonus, l, r) {
            let key = cand.delta_raw + 6_000 * endpoint_metric_delta(&cand);
            if key > best_key {
                best_key = key;
                best = Some(cand);
            }
        }
    }
    best
}

fn sample_endpoint_relocate(
    n: usize,
    state: &State,
    weights: &[i64],
    neighbors: &[Vec<usize>],
    start_bonus: &[i64],
    end_bonus: &[i64],
    low_cells: &[usize],
    high_cells: &[usize],
    rng: &mut Xoshiro256PlusPlus,
) -> Option<Candidate> {
    let m = state.path.len();
    let mut best = None;
    let mut best_key = i64::MIN;
    let mut legal_qs = Vec::with_capacity(32);
    for _ in 0..ENDPOINT_TRIES {
        if !low_cells.is_empty() {
            let cell = low_cells[rng.random_range(0..low_cells.len())];
            let p = state.pos[cell];
            let len = rng.random_range(1..=3.min(m));
            let left = p.saturating_sub(len - 1);
            let l = left;
            let r = (l + len - 1).min(m - 1);
            collect_legal_relocate_positions(n, state, neighbors, l, r, &mut legal_qs);
            for &q in &legal_qs {
                if q > 8 {
                    continue;
                }
                if let Some(cand) =
                    evaluate_relocate(n, state, weights, start_bonus, end_bonus, l, r, q)
                {
                    let cand = Candidate {
                        class: MoveClass::Endpoint,
                        ..cand
                    };
                    let key =
                        cand.delta_raw + 32_000 * endpoint_metric_delta(&cand) - 4_000 * q as i64;
                    if key > best_key {
                        best_key = key;
                        best = Some(cand);
                    }
                }
            }
        }
        if !high_cells.is_empty() {
            let cell = high_cells[rng.random_range(0..high_cells.len())];
            let p = state.pos[cell];
            let len = rng.random_range(1..=3.min(m));
            let l = p.saturating_sub(len / 2).min(m - len);
            let r = l + len - 1;
            collect_legal_relocate_positions(n, state, neighbors, l, r, &mut legal_qs);
            for &q in &legal_qs {
                let dist_to_end = m - (q + len);
                if dist_to_end > 8 {
                    continue;
                }
                if let Some(cand) =
                    evaluate_relocate(n, state, weights, start_bonus, end_bonus, l, r, q)
                {
                    let cand = Candidate {
                        class: MoveClass::Endpoint,
                        ..cand
                    };
                    let key = cand.delta_raw + 32_000 * endpoint_metric_delta(&cand)
                        - 4_000 * dist_to_end as i64;
                    if key > best_key {
                        best_key = key;
                        best = Some(cand);
                    }
                }
            }
        }
    }
    best
}

fn evaluate_relocate(
    n: usize,
    state: &State,
    _weights: &[i64],
    start_bonus: &[i64],
    end_bonus: &[i64],
    l: usize,
    r: usize,
    q: usize,
) -> Option<Candidate> {
    let m = state.path.len();
    let len = r.checked_sub(l)? + 1;
    if len >= m || q > m - len || q == l {
        return None;
    }

    let seg_first = state.path[l];
    let seg_last = state.path[r];
    let old_start = state.path[0];
    let old_end = state.path[m - 1];
    let new_start;
    let new_end;
    let delta_raw;

    if q < l {
        let left_before = (q > 0).then(|| state.path[q - 1]);
        let right_before = state.path[q];
        if let Some(u) = left_before {
            if !is_adj(u, seg_first, n) {
                return None;
            }
        }
        if !is_adj(seg_last, right_before, n) {
            return None;
        }
        if l > 0 && r + 1 < m && !is_adj(state.path[l - 1], state.path[r + 1], n) {
            return None;
        }
        let moved = (l - q) as i64;
        let seg_sum = sum_range(&state.prefix_weight, l, r);
        let mid_sum = sum_range(&state.prefix_weight, q, l - 1);
        delta_raw = len as i64 * mid_sum - moved * seg_sum;
        new_start = if q == 0 { seg_first } else { old_start };
        new_end = if r + 1 == m {
            state.path[l - 1]
        } else {
            old_end
        };
    } else {
        let orig = q + len;
        if orig > m || orig <= r {
            return None;
        }
        if l > 0 && r + 1 < m && !is_adj(state.path[l - 1], state.path[r + 1], n) {
            return None;
        }
        let before_insert = state.path[orig - 1];
        if !is_adj(before_insert, seg_first, n) {
            return None;
        }
        if orig < m && !is_adj(seg_last, state.path[orig], n) {
            return None;
        }
        let moved = (q - l) as i64;
        let seg_sum = sum_range(&state.prefix_weight, l, r);
        let mid_sum = sum_range(&state.prefix_weight, r + 1, orig - 1);
        delta_raw = moved * seg_sum - len as i64 * mid_sum;
        new_start = if l == 0 { state.path[r + 1] } else { old_start };
        new_end = if orig == m { seg_last } else { old_end };
    }

    let new_hits = endpoint_hits_from_cells(start_bonus, end_bonus, new_start, new_end);
    let new_points = start_bonus[new_start] + end_bonus[new_end];
    Some(Candidate {
        mv: MoveKind::Relocate { l, r, q },
        class: MoveClass::Relocate,
        delta_raw,
        delta_endpoint_hits: new_hits - state.endpoint_hits,
        delta_endpoint_points: new_points - state.endpoint_points,
    })
}

fn evaluate_swap(
    n: usize,
    state: &State,
    _weights: &[i64],
    start_bonus: &[i64],
    end_bonus: &[i64],
    mut l1: usize,
    mut l2: usize,
    len: usize,
) -> Option<Candidate> {
    let m = state.path.len();
    if len == 0 || l1 + len > m || l2 + len > m {
        return None;
    }
    if l2 < l1 {
        std::mem::swap(&mut l1, &mut l2);
    }
    if l1 + len > l2 {
        return None;
    }
    let r1 = l1 + len - 1;
    let r2 = l2 + len - 1;
    let left = (l1 > 0).then(|| state.path[l1 - 1]);
    let right = (r2 + 1 < m).then(|| state.path[r2 + 1]);
    let b_first = state.path[l2];
    let b_last = state.path[r2];
    let a_first = state.path[l1];
    let a_last = state.path[r1];
    if let Some(u) = left {
        if !is_adj(u, b_first, n) {
            return None;
        }
    }
    if l2 == r1 + 1 {
        if !is_adj(b_last, a_first, n) {
            return None;
        }
    } else {
        let mid_first = state.path[r1 + 1];
        let mid_last = state.path[l2 - 1];
        if !is_adj(b_last, mid_first, n) || !is_adj(mid_last, a_first, n) {
            return None;
        }
    }
    if let Some(v) = right {
        if !is_adj(a_last, v, n) {
            return None;
        }
    }

    let sum_a = sum_range(&state.prefix_weight, l1, r1);
    let sum_b = sum_range(&state.prefix_weight, l2, r2);
    let delta_raw = (l2 as i64 - l1 as i64) * (sum_a - sum_b);
    let new_start = if l1 == 0 { b_first } else { state.path[0] };
    let new_end = if r2 + 1 == m {
        a_last
    } else {
        state.path[m - 1]
    };
    let new_hits = endpoint_hits_from_cells(start_bonus, end_bonus, new_start, new_end);
    let new_points = start_bonus[new_start] + end_bonus[new_end];
    Some(Candidate {
        mv: MoveKind::Swap { l1, l2, len },
        class: MoveClass::Swap,
        delta_raw,
        delta_endpoint_hits: new_hits - state.endpoint_hits,
        delta_endpoint_points: new_points - state.endpoint_points,
    })
}

fn evaluate_reverse(
    n: usize,
    state: &State,
    start_bonus: &[i64],
    end_bonus: &[i64],
    l: usize,
    r: usize,
) -> Option<Candidate> {
    if l >= r {
        return None;
    }
    let m = state.path.len();
    if l > 0 && !is_adj(state.path[l - 1], state.path[r], n) {
        return None;
    }
    if r + 1 < m && !is_adj(state.path[l], state.path[r + 1], n) {
        return None;
    }
    let sum_w = sum_range(&state.prefix_weight, l, r);
    let sum_pos_w = sum_range(&state.prefix_pos_weight, l, r);
    let delta_raw = (l as i64 + r as i64) * sum_w - 2 * sum_pos_w;
    let new_start = if l == 0 { state.path[r] } else { state.path[0] };
    let new_end = if r + 1 == m {
        state.path[l]
    } else {
        state.path[m - 1]
    };
    let new_hits = endpoint_hits_from_cells(start_bonus, end_bonus, new_start, new_end);
    let new_points = start_bonus[new_start] + end_bonus[new_end];
    Some(Candidate {
        mv: MoveKind::Reverse { l, r },
        class: MoveClass::Reverse,
        delta_raw,
        delta_endpoint_hits: new_hits - state.endpoint_hits,
        delta_endpoint_points: new_points - state.endpoint_points,
    })
}

fn sum_range(prefix: &[i64], l: usize, r: usize) -> i64 {
    if l > r { 0 } else { prefix[r + 1] - prefix[l] }
}

fn is_adj(u: usize, v: usize, n: usize) -> bool {
    let ur = u / n;
    let uc = u % n;
    let vr = v / n;
    let vc = v % n;
    let dr = ur.abs_diff(vr);
    let dc = uc.abs_diff(vc);
    (dr != 0 || dc != 0) && dr <= 1 && dc <= 1
}

fn apply_move(path: &mut Vec<usize>, mv: MoveKind) {
    match mv {
        MoveKind::Relocate { l, r, q } => apply_relocate(path, l, r, q),
        MoveKind::Swap { l1, l2, len } => apply_swap(path, l1, l2, len),
        MoveKind::Reverse { l, r } => path[l..=r].reverse(),
    }
}

fn apply_relocate(path: &mut Vec<usize>, l: usize, r: usize, q: usize) {
    let m = path.len();
    let len = r - l + 1;
    if q < l {
        let mut next = Vec::with_capacity(m);
        next.extend_from_slice(&path[..q]);
        next.extend_from_slice(&path[l..=r]);
        next.extend_from_slice(&path[q..l]);
        next.extend_from_slice(&path[r + 1..]);
        *path = next;
    } else {
        let orig = q + len;
        let mut next = Vec::with_capacity(m);
        next.extend_from_slice(&path[..l]);
        next.extend_from_slice(&path[r + 1..orig]);
        next.extend_from_slice(&path[l..=r]);
        next.extend_from_slice(&path[orig..]);
        *path = next;
    }
}

fn apply_swap(path: &mut Vec<usize>, mut l1: usize, mut l2: usize, len: usize) {
    if l2 < l1 {
        std::mem::swap(&mut l1, &mut l2);
    }
    let r1 = l1 + len - 1;
    let r2 = l2 + len - 1;
    let mut next = Vec::with_capacity(path.len());
    next.extend_from_slice(&path[..l1]);
    next.extend_from_slice(&path[l2..=r2]);
    next.extend_from_slice(&path[r1 + 1..l2]);
    next.extend_from_slice(&path[l1..=r1]);
    next.extend_from_slice(&path[r2 + 1..]);
    *path = next;
}
