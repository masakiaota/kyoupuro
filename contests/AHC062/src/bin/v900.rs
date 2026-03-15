use proconio::input;
use std::time::{Duration, Instant};

const TIME_LIMIT: f64 = 1.92;
const STRIP_W: usize = 5;
const SIDE_BIAS: i64 = 5_000_000;
const MAX_SEG_LEN: usize = 4;
const MAX_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;

#[derive(Clone, Copy)]
struct Symmetry {
    flip_row: bool,
    flip_col: bool,
}

#[derive(Clone, Copy)]
enum CandidateMove {
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
}

#[derive(Clone, Copy)]
struct MoveEval {
    mv: CandidateMove,
    delta_obj: i64,
    delta_raw: i64,
    delta_vio: i32,
}

#[derive(Clone, Copy)]
struct LegalTwoOptMove {
    l: usize,
    r: usize,
    delta: i64,
}

#[derive(Clone, Copy)]
struct LegalRelocationMove {
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
    delta: i64,
}

#[derive(Clone)]
struct WindowRepair {
    start: usize,
    len: usize,
    cells: [usize; MAX_WINDOW],
    delta_obj: i64,
}

#[derive(Clone, Copy)]
struct RelocationMove {
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
}

const SYMMETRIES: [Symmetry; 4] = [
    Symmetry {
        flip_row: false,
        flip_col: false,
    },
    Symmetry {
        flip_row: true,
        flip_col: false,
    },
    Symmetry {
        flip_row: false,
        flip_col: true,
    },
    Symmetry {
        flip_row: true,
        flip_col: true,
    },
];

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let weights = flatten_weights(n, &a);
    let neighbors = build_neighbors(n);

    let mut route = ideal_order_route(n, &weights);

    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(TIME_LIMIT);
    let penalty_limit = Duration::from_secs_f64(TIME_LIMIT * 0.58);
    let force_legal_limit = Duration::from_secs_f64(TIME_LIMIT * 0.80);
    let mixed_limit = Duration::from_secs_f64(TIME_LIMIT * 0.90);

    run_penalty_continuation(&mut route, &weights, &neighbors, n, &start, penalty_limit);

    if !is_valid_route(&route, n) {
        run_force_legalization(
            &mut route,
            &weights,
            &neighbors,
            n,
            &start,
            force_legal_limit.min(time_limit),
        );
    }
    if !is_valid_route(&route, n) {
        route = best_legal_strip_route(n, &weights);
    }

    run_legal_mixed_descent(
        &mut route,
        &weights,
        &neighbors,
        n,
        &start,
        mixed_limit.min(time_limit),
    );
    run_legal_window_descent(&mut route, &weights, n, &start, time_limit, 1);

    if !is_valid_route(&route, n) {
        route = best_legal_strip_route(n, &weights);
    }

    for cell in route {
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

fn ideal_order_route(n: usize, weights: &[i64]) -> Vec<usize> {
    let perms = generate_permutations();
    let transitions = build_transitions(&perms);
    let mut best_route = Vec::new();
    let mut best_score = NEG_INF;

    for symmetry in SYMMETRIES {
        let transformed = transform_weights(n, weights, symmetry);
        let route = build_round_interleaved_route(n, &transformed, &perms, &transitions);
        let score = compute_raw_score(&route, &transformed);
        if score > best_score {
            best_score = score;
            best_route = map_route_back(&route, n, symmetry);
        }
    }

    best_route
}

fn build_round_interleaved_route(
    n: usize,
    weights: &[i64],
    perms: &[[usize; STRIP_W]],
    transitions: &[Vec<usize>],
) -> Vec<usize> {
    debug_assert_eq!(n % STRIP_W, 0);

    let strip_count = n / STRIP_W;
    let mut strip_paths = Vec::with_capacity(strip_count);
    for strip in 0..strip_count {
        strip_paths.push(solve_strip_paths(
            n,
            weights,
            strip,
            strip_count,
            perms,
            transitions,
        ));
    }
    let mut route = Vec::with_capacity(n * n);

    for round in 0..STRIP_W {
        let left_to_right = round % 2 == 0;
        for pos in 0..strip_count {
            let strip = if left_to_right {
                pos
            } else {
                strip_count - 1 - pos
            };
            let dir_down = pos % 2 == 0;
            if dir_down {
                for row in 0..n {
                    let local_col = strip_paths[strip][row][round] as usize;
                    route.push(row * n + strip * STRIP_W + local_col);
                }
            } else {
                for row in (0..n).rev() {
                    let local_col = strip_paths[strip][row][round] as usize;
                    route.push(row * n + strip * STRIP_W + local_col);
                }
            }
        }
    }

    route
}

fn solve_strip_paths(
    n: usize,
    weights: &[i64],
    strip: usize,
    strip_count: usize,
    perms: &[[usize; STRIP_W]],
    transitions: &[Vec<usize>],
) -> Vec<[u8; STRIP_W]> {
    let state_count = perms.len();
    let strip_col0 = strip * STRIP_W;
    let mut coeff = vec![[0i64; STRIP_W]; n];
    for round in 0..STRIP_W {
        let left_to_right = round % 2 == 0;
        let pos = if left_to_right {
            strip
        } else {
            strip_count - 1 - strip
        };
        let dir_down = pos % 2 == 0;
        let seg_start = (round * strip_count + pos) * n;
        for row in 0..n {
            let offset = if dir_down { row } else { n - 1 - row };
            coeff[row][round] = (seg_start + offset) as i64;
        }
    }

    let mut row_score = vec![vec![0i64; state_count]; n];
    for row in 0..n {
        for (idx, perm) in perms.iter().enumerate() {
            let mut value = 0i64;
            for round in 0..STRIP_W {
                let col = perm[round];
                value += coeff[row][round] * weights[row * n + strip_col0 + col];
            }
            if row == 0 {
                value += endpoint_bias(strip, true, perm);
            }
            if row + 1 == n {
                value += endpoint_bias(strip, false, perm);
            }
            row_score[row][idx] = value;
        }
    }

    let mut dp_prev = vec![NEG_INF; state_count];
    let mut dp_cur = vec![NEG_INF; state_count];
    let mut parent = vec![vec![u16::MAX; state_count]; n];
    dp_prev[..state_count].copy_from_slice(&row_score[0][..state_count]);

    for row in 1..n {
        for value in &mut dp_cur {
            *value = NEG_INF;
        }
        for cur in 0..state_count {
            let mut best_val = NEG_INF;
            let mut best_prev = u16::MAX;
            for &prev in &transitions[cur] {
                let cand = dp_prev[prev] + row_score[row][cur];
                if cand > best_val {
                    best_val = cand;
                    best_prev = prev as u16;
                }
            }
            dp_cur[cur] = best_val;
            parent[row][cur] = best_prev;
        }
        std::mem::swap(&mut dp_prev, &mut dp_cur);
    }

    let mut best_last = 0usize;
    for idx in 1..state_count {
        if dp_prev[idx] > dp_prev[best_last] {
            best_last = idx;
        }
    }

    let mut rows = vec![[0u8; STRIP_W]; n];
    let mut cur = best_last;
    for row in (0..n).rev() {
        for round in 0..STRIP_W {
            rows[row][round] = perms[cur][round] as u8;
        }
        if row > 0 {
            cur = parent[row][cur] as usize;
        }
    }

    rows
}

fn generate_permutations() -> Vec<[usize; STRIP_W]> {
    let mut perms = Vec::new();
    let mut cur = [0usize; STRIP_W];
    let mut used = [false; STRIP_W];
    dfs_permutations(0, &mut used, &mut cur, &mut perms);
    perms
}

fn dfs_permutations(
    depth: usize,
    used: &mut [bool; STRIP_W],
    cur: &mut [usize; STRIP_W],
    perms: &mut Vec<[usize; STRIP_W]>,
) {
    if depth == STRIP_W {
        perms.push(*cur);
        return;
    }
    for col in 0..STRIP_W {
        if used[col] {
            continue;
        }
        used[col] = true;
        cur[depth] = col;
        dfs_permutations(depth + 1, used, cur, perms);
        used[col] = false;
    }
}

fn build_transitions(perms: &[[usize; STRIP_W]]) -> Vec<Vec<usize>> {
    let state_count = perms.len();
    let mut transitions = vec![Vec::new(); state_count];
    for cur in 0..state_count {
        for prev in 0..state_count {
            if is_transition_valid(&perms[prev], &perms[cur]) {
                transitions[cur].push(prev);
            }
        }
    }
    transitions
}

fn is_transition_valid(prev: &[usize; STRIP_W], cur: &[usize; STRIP_W]) -> bool {
    for round in 0..STRIP_W {
        if prev[round].abs_diff(cur[round]) > 1 {
            return false;
        }
    }
    true
}

fn endpoint_bias(strip: usize, top_row: bool, perm: &[usize; STRIP_W]) -> i64 {
    let mut bias = 0i64;
    for round in 0..STRIP_W {
        let pref_right = if top_row {
            (strip + round) % 2 == 1
        } else {
            (strip + round) % 2 == 0
        };
        bias += side_bias(pref_right, perm[round]);
    }
    bias
}

fn side_bias(prefer_right: bool, col: usize) -> i64 {
    let level = if prefer_right { col } else { STRIP_W - 1 - col };
    level as i64 * SIDE_BIAS
}

fn transform_weights(n: usize, weights: &[i64], symmetry: Symmetry) -> Vec<i64> {
    let mut transformed = vec![0; n * n];
    for row in 0..n {
        for col in 0..n {
            let (orig_row, orig_col) = map_from_transformed(row, col, n, symmetry);
            transformed[row * n + col] = weights[orig_row * n + orig_col];
        }
    }
    transformed
}

fn map_route_back(route: &[usize], n: usize, symmetry: Symmetry) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| {
            let row = cell / n;
            let col = cell % n;
            let (orig_row, orig_col) = map_from_transformed(row, col, n, symmetry);
            orig_row * n + orig_col
        })
        .collect()
}

fn map_from_transformed(row: usize, col: usize, n: usize, symmetry: Symmetry) -> (usize, usize) {
    let mapped_row = if symmetry.flip_row { n - 1 - row } else { row };
    let mapped_col = if symmetry.flip_col { n - 1 - col } else { col };
    (mapped_row, mapped_col)
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

fn run_penalty_continuation(
    route: &mut Vec<usize>,
    weights: &[i64],
    neighbors: &[Vec<usize>],
    n: usize,
    start: &Instant,
    time_limit: Duration,
) {
    let lambdas = [
        50_000_i64,
        200_000,
        1_000_000,
        5_000_000,
        20_000_000,
        80_000_000,
        300_000_000,
        1_000_000_000,
        3_000_000_000,
    ];
    let stage_ratios = [0.06, 0.14, 0.24, 0.36, 0.50, 0.66, 0.80, 0.90, 0.97];

    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];
    let mut violations = Vec::with_capacity(len);
    let mut cursor = 0usize;

    for (&lambda, &ratio) in lambdas.iter().zip(stage_ratios.iter()) {
        let stage_deadline = Duration::from_secs_f64((TIME_LIMIT * ratio).min(TIME_LIMIT));
        while start.elapsed() < stage_deadline && start.elapsed() < time_limit {
            rebuild_aux(
                route,
                weights,
                &mut position,
                &mut prefix_weight,
                &mut prefix_pos_weight,
            );
            collect_violations(route, n, &mut violations);
            if violations.is_empty() {
                return;
            }

            let sampled = sample_breakpoints(&violations, &mut cursor, lambda);
            let best = find_best_breakpoint_move(
                route,
                weights,
                neighbors,
                &position,
                &prefix_weight,
                &prefix_pos_weight,
                n,
                &sampled,
                lambda,
            );

            if let Some(best_mv) = best {
                apply_candidate_move(route, best_mv.mv);
                continue;
            }

            if apply_best_penalty_window_repair(route, weights, n, lambda, &sampled, 24) {
                continue;
            }

            break;
        }
    }
}

fn run_force_legalization(
    route: &mut Vec<usize>,
    weights: &[i64],
    neighbors: &[Vec<usize>],
    n: usize,
    start: &Instant,
    deadline: Duration,
) {
    let force_lambda = 20_000_000_000i64;
    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];
    let mut violations = Vec::with_capacity(len);
    let mut cursor = 0usize;

    while start.elapsed() < deadline {
        rebuild_aux(
            route,
            weights,
            &mut position,
            &mut prefix_weight,
            &mut prefix_pos_weight,
        );
        collect_violations(route, n, &mut violations);
        if violations.is_empty() {
            return;
        }

        let sampled = if violations.len() <= 2048 {
            violations.clone()
        } else {
            sample_breakpoints(&violations, &mut cursor, force_lambda)
        };

        let best = find_best_breakpoint_move(
            route,
            weights,
            neighbors,
            &position,
            &prefix_weight,
            &prefix_pos_weight,
            n,
            &sampled,
            force_lambda,
        );

        if let Some(best_mv) = best {
            apply_candidate_move(route, best_mv.mv);
            continue;
        }

        if apply_best_penalty_window_repair(route, weights, n, force_lambda, &sampled, 320) {
            continue;
        }

        break;
    }
}

fn rebuild_aux(
    route: &[usize],
    weights: &[i64],
    position: &mut [usize],
    prefix_weight: &mut [i64],
    prefix_pos_weight: &mut [i64],
) {
    prefix_weight[0] = 0;
    prefix_pos_weight[0] = 0;
    for (idx, &cell) in route.iter().enumerate() {
        position[cell] = idx;
        prefix_weight[idx + 1] = prefix_weight[idx] + weights[cell];
        prefix_pos_weight[idx + 1] = prefix_pos_weight[idx] + idx as i64 * weights[cell];
    }
}

fn collect_violations(route: &[usize], n: usize, violations: &mut Vec<usize>) {
    violations.clear();
    for i in 0..route.len() - 1 {
        if !is_adj(route[i], route[i + 1], n) {
            violations.push(i);
        }
    }
}

fn sample_breakpoints(violations: &[usize], cursor: &mut usize, lambda: i64) -> Vec<usize> {
    if violations.is_empty() {
        return Vec::new();
    }

    let cap = if lambda < 20_000_000 {
        96usize
    } else if lambda < 300_000_000 {
        192usize
    } else {
        384usize
    };
    let sample_size = violations.len().min(cap.max(16));
    let step = (violations.len() / sample_size).max(1);
    let mut idx = *cursor % violations.len();

    let mut sampled = Vec::with_capacity(sample_size);
    for _ in 0..sample_size {
        sampled.push(violations[idx]);
        idx = (idx + step) % violations.len();
    }

    *cursor = idx;
    sampled
}

fn find_best_breakpoint_move(
    route: &[usize],
    weights: &[i64],
    neighbors: &[Vec<usize>],
    position: &[usize],
    prefix_weight: &[i64],
    prefix_pos_weight: &[i64],
    n: usize,
    sampled_breakpoints: &[usize],
    lambda: i64,
) -> Option<MoveEval> {
    let len = route.len();
    let max_rev_span = if lambda < 20_000_000 {
        256usize
    } else if lambda < 300_000_000 {
        2048usize
    } else {
        usize::MAX
    };

    let mut best: Option<MoveEval> = None;

    for &k in sampled_breakpoints {
        if k + 1 >= len {
            continue;
        }

        let left_cell = route[k];
        let right_cell = route[k + 1];

        for &anchor in &neighbors[left_cell] {
            let r = position[anchor];
            let l = k + 1;
            if r <= l {
                continue;
            }
            if max_rev_span != usize::MAX && r - l > max_rev_span {
                continue;
            }

            let delta_raw = reversal_delta(l, r, prefix_weight, prefix_pos_weight);
            let delta_vio = reversal_violation_delta(route, n, l, r);
            let delta_obj = delta_raw - lambda * delta_vio as i64;
            consider_move(
                &mut best,
                MoveEval {
                    mv: CandidateMove::Reverse { l, r },
                    delta_obj,
                    delta_raw,
                    delta_vio,
                },
            );
        }

        for &anchor in &neighbors[right_cell] {
            let l = position[anchor];
            let r = k;
            if l >= r {
                continue;
            }
            if max_rev_span != usize::MAX && r - l > max_rev_span {
                continue;
            }

            let delta_raw = reversal_delta(l, r, prefix_weight, prefix_pos_weight);
            let delta_vio = reversal_violation_delta(route, n, l, r);
            let delta_obj = delta_raw - lambda * delta_vio as i64;
            consider_move(
                &mut best,
                MoveEval {
                    mv: CandidateMove::Reverse { l, r },
                    delta_obj,
                    delta_raw,
                    delta_vio,
                },
            );
        }

        let right_start = k + 1;
        for seg_len in 1..=MAX_SEG_LEN {
            let l = right_start;
            let r = l + seg_len - 1;
            if r + 1 >= len || l == 0 {
                break;
            }

            for &anchor in &neighbors[left_cell] {
                let p = position[anchor];
                if p + 1 >= len || !edge_disjoint_from_segment(p, l, r) {
                    continue;
                }

                for reversed in [false, true] {
                    let delta_raw =
                        relocation_delta(route, weights, prefix_weight, l, r, p, reversed);
                    let delta_vio = relocation_violation_delta(route, n, l, r, p, reversed);
                    let delta_obj = delta_raw - lambda * delta_vio as i64;
                    consider_move(
                        &mut best,
                        MoveEval {
                            mv: CandidateMove::Relocate { l, r, p, reversed },
                            delta_obj,
                            delta_raw,
                            delta_vio,
                        },
                    );
                }
            }
        }

        for seg_len in 1..=MAX_SEG_LEN {
            if k + 1 < seg_len {
                break;
            }
            let r = k;
            let l = r + 1 - seg_len;
            if l == 0 || r + 1 >= len {
                continue;
            }

            for &anchor in &neighbors[right_cell] {
                let p = position[anchor];
                if p + 1 >= len || !edge_disjoint_from_segment(p, l, r) {
                    continue;
                }

                for reversed in [false, true] {
                    let delta_raw =
                        relocation_delta(route, weights, prefix_weight, l, r, p, reversed);
                    let delta_vio = relocation_violation_delta(route, n, l, r, p, reversed);
                    let delta_obj = delta_raw - lambda * delta_vio as i64;
                    consider_move(
                        &mut best,
                        MoveEval {
                            mv: CandidateMove::Relocate { l, r, p, reversed },
                            delta_obj,
                            delta_raw,
                            delta_vio,
                        },
                    );
                }
            }
        }
    }

    best
}

fn consider_move(best: &mut Option<MoveEval>, cand: MoveEval) {
    if cand.delta_obj <= 0 {
        return;
    }

    match best {
        None => *best = Some(cand),
        Some(cur) => {
            if cand.delta_obj > cur.delta_obj
                || (cand.delta_obj == cur.delta_obj && cand.delta_vio < cur.delta_vio)
                || (cand.delta_obj == cur.delta_obj
                    && cand.delta_vio == cur.delta_vio
                    && cand.delta_raw > cur.delta_raw)
            {
                *best = Some(cand);
            }
        }
    }
}

fn apply_candidate_move(route: &mut [usize], mv: CandidateMove) {
    match mv {
        CandidateMove::Reverse { l, r } => {
            route[l..=r].reverse();
        }
        CandidateMove::Relocate { l, r, p, reversed } => {
            apply_relocation(route, RelocationMove { l, r, p, reversed });
        }
    }
}

fn apply_best_penalty_window_repair(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    lambda: i64,
    sampled_breakpoints: &[usize],
    scan_limit: usize,
) -> bool {
    let len_total = route.len();
    if len_total < 2 || sampled_breakpoints.is_empty() {
        return false;
    }

    let mut best: Option<WindowRepair> = None;
    let scan_limit = sampled_breakpoints.len().min(scan_limit.max(1));

    for &k in sampled_breakpoints.iter().take(scan_limit) {
        for &window in &[9usize, 8usize] {
            if window > len_total {
                continue;
            }
            let start_lo = (k + 1).saturating_sub(window);
            let start_hi = k.min(len_total - window);
            if start_lo > start_hi {
                continue;
            }
            for start in start_lo..=start_hi {
                if let Some(cand) =
                    evaluate_penalty_window(route, weights, n, start, window, lambda)
                {
                    match &best {
                        None => best = Some(cand),
                        Some(cur) => {
                            if cand.delta_obj > cur.delta_obj {
                                best = Some(cand);
                            }
                        }
                    }
                }
            }
        }
    }

    let Some(best_repair) = best else {
        return false;
    };

    for i in 0..best_repair.len {
        route[best_repair.start + i] = best_repair.cells[i];
    }
    true
}

fn evaluate_penalty_window(
    route: &[usize],
    weights: &[i64],
    n: usize,
    start: usize,
    len: usize,
    lambda: i64,
) -> Option<WindowRepair> {
    if len < 2 {
        return None;
    }

    let len_total = route.len();
    let mut seg = [0usize; MAX_WINDOW];
    for i in 0..len {
        seg[i] = route[start + i];
    }

    let left = if start > 0 {
        Some(route[start - 1])
    } else {
        None
    };
    let right = if start + len < len_total {
        Some(route[start + len])
    } else {
        None
    };

    let mut old_raw = 0i64;
    for i in 0..len {
        old_raw += (start + i) as i64 * weights[seg[i]];
    }
    let mut old_vio = 0i64;
    if let Some(cell) = left {
        old_vio += violation_cost(cell, seg[0], n);
    }
    for i in 0..len - 1 {
        old_vio += violation_cost(seg[i], seg[i + 1], n);
    }
    if let Some(cell) = right {
        old_vio += violation_cost(seg[len - 1], cell, n);
    }
    let old_obj = old_raw - lambda * old_vio;

    let states = 1usize << len;
    let width = len;
    let mut dp = vec![NEG_INF; states * width];
    let mut parent = vec![u8::MAX; states * width];

    for first in 0..len {
        let mut val = start as i64 * weights[seg[first]];
        if let Some(cell) = left {
            val -= lambda * violation_cost(cell, seg[first], n);
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
                let mut next_value = cur + next_pos as i64 * weights[seg[nxt]];
                next_value -= lambda * violation_cost(seg[last], seg[nxt], n);
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
    let mut best_obj = old_obj;

    for last in 0..len {
        let mut value = dp[full * width + last];
        if value == NEG_INF {
            continue;
        }
        if let Some(cell) = right {
            value -= lambda * violation_cost(seg[last], cell, n);
        }
        if value > best_obj {
            best_obj = value;
            best_last = last;
        }
    }

    if best_last == usize::MAX {
        return None;
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

    let mut cells = [0usize; MAX_WINDOW];
    let mut changed = false;
    for i in 0..len {
        cells[i] = seg[order[i]];
        if cells[i] != seg[i] {
            changed = true;
        }
    }

    if !changed {
        return None;
    }

    Some(WindowRepair {
        start,
        len,
        cells,
        delta_obj: best_obj - old_obj,
    })
}

fn run_legal_mixed_descent(
    route: &mut [usize],
    weights: &[i64],
    neighbors: &[Vec<usize>],
    n: usize,
    start: &Instant,
    deadline: Duration,
) {
    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];

    while start.elapsed() < deadline {
        rebuild_aux(
            route,
            weights,
            &mut position,
            &mut prefix_weight,
            &mut prefix_pos_weight,
        );

        let best_reloc =
            best_legal_relocation_move(route, weights, neighbors, &position, &prefix_weight, n);
        let best_two_opt = best_legal_two_opt_move(
            route,
            &position,
            neighbors,
            &prefix_weight,
            &prefix_pos_weight,
            n,
        );

        let reloc_delta = best_reloc.as_ref().map_or(0, |mv| mv.delta);
        let two_opt_delta = best_two_opt.as_ref().map_or(0, |mv| mv.delta);
        if reloc_delta <= 0 && two_opt_delta <= 0 {
            break;
        }

        if reloc_delta >= two_opt_delta {
            let mv = best_reloc.unwrap();
            let insert_start = if mv.p < mv.l {
                mv.p + 1
            } else {
                mv.p + 1 - (mv.r - mv.l + 1)
            };
            apply_relocation(
                route,
                RelocationMove {
                    l: mv.l,
                    r: mv.r,
                    p: mv.p,
                    reversed: mv.reversed,
                },
            );
            refine_legal_around(route, weights, n, &[mv.l, insert_start]);
        } else {
            let mv = best_two_opt.unwrap();
            route[mv.l..=mv.r].reverse();
            refine_legal_around(route, weights, n, &[mv.l, mv.r]);
        }
    }
}

fn best_legal_two_opt_move(
    route: &[usize],
    position: &[usize],
    neighbors: &[Vec<usize>],
    prefix_weight: &[i64],
    prefix_pos_weight: &[i64],
    n: usize,
) -> Option<LegalTwoOptMove> {
    let len = route.len();
    let mut best = None;
    let mut best_delta = 0i64;

    let full_delta = reversal_delta(0, len - 1, prefix_weight, prefix_pos_weight);
    if full_delta > best_delta {
        best_delta = full_delta;
        best = Some(LegalTwoOptMove {
            l: 0,
            r: len - 1,
            delta: full_delta,
        });
    }

    let first = route[0];
    for &cell in &neighbors[first] {
        let p = position[cell];
        if p == 0 {
            continue;
        }
        let r = p - 1;
        let delta = reversal_delta(0, r, prefix_weight, prefix_pos_weight);
        if delta > best_delta {
            best_delta = delta;
            best = Some(LegalTwoOptMove { l: 0, r, delta });
        }
    }

    let last = route[len - 1];
    for &cell in &neighbors[last] {
        let p = position[cell];
        if p + 1 >= len {
            continue;
        }
        let l = p + 1;
        let delta = reversal_delta(l, len - 1, prefix_weight, prefix_pos_weight);
        if delta > best_delta {
            best_delta = delta;
            best = Some(LegalTwoOptMove {
                l,
                r: len - 1,
                delta,
            });
        }
    }

    for l in 1..len - 1 {
        let left = route[l - 1];
        let first_in = route[l];
        for &cell in &neighbors[left] {
            let r = position[cell];
            if r <= l || r + 1 >= len {
                continue;
            }
            if !is_adj(first_in, route[r + 1], n) {
                continue;
            }
            let delta = reversal_delta(l, r, prefix_weight, prefix_pos_weight);
            if delta > best_delta {
                best_delta = delta;
                best = Some(LegalTwoOptMove { l, r, delta });
            }
        }
    }

    best
}

fn best_legal_relocation_move(
    route: &[usize],
    weights: &[i64],
    neighbors: &[Vec<usize>],
    position: &[usize],
    prefix_weight: &[i64],
    n: usize,
) -> Option<LegalRelocationMove> {
    let len = route.len();
    let mut best = None;
    let mut best_delta = 0i64;

    for seg_len in 1..=MAX_SEG_LEN.min(3) {
        for l in 1..len - 1 {
            let r = l + seg_len - 1;
            if r >= len - 1 {
                break;
            }
            let prev = route[l - 1];
            let next = route[r + 1];
            if !is_adj(prev, next, n) {
                continue;
            }
            let first = route[l];
            let last = route[r];

            for &anchor in &neighbors[first] {
                let p = position[anchor];
                if p >= len - 1 || !edge_disjoint_from_segment(p, l, r) {
                    continue;
                }
                let b = route[p + 1];
                if !is_adj(last, b, n) {
                    continue;
                }
                let delta = relocation_delta(route, weights, prefix_weight, l, r, p, false);
                if delta > best_delta {
                    best_delta = delta;
                    best = Some(LegalRelocationMove {
                        l,
                        r,
                        p,
                        reversed: false,
                        delta,
                    });
                }
            }

            for &anchor in &neighbors[last] {
                let p = position[anchor];
                if p >= len - 1 || !edge_disjoint_from_segment(p, l, r) {
                    continue;
                }
                let b = route[p + 1];
                if !is_adj(first, b, n) {
                    continue;
                }
                let delta = relocation_delta(route, weights, prefix_weight, l, r, p, true);
                if delta > best_delta {
                    best_delta = delta;
                    best = Some(LegalRelocationMove {
                        l,
                        r,
                        p,
                        reversed: true,
                        delta,
                    });
                }
            }
        }
    }

    best
}

fn refine_legal_around(route: &mut [usize], weights: &[i64], n: usize, anchors: &[usize]) {
    let len = route.len();
    for &anchor in anchors {
        for &window in &[9usize, 8usize] {
            if len < window {
                continue;
            }
            let start_lo = anchor.saturating_sub(window + 2);
            let start_hi = anchor.saturating_add(2).min(len - window);
            for start in start_lo..=start_hi {
                optimize_legal_window(route, weights, n, start, window);
            }
        }
    }
}

fn run_legal_window_descent(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    start: &Instant,
    deadline: Duration,
    min_cycles: usize,
) {
    let window_sizes = [9usize, 8usize];
    let mut cycle = 0usize;
    while start.elapsed() < deadline {
        let mut improved = false;
        for &window in &window_sizes {
            for offset in 0..window {
                if start.elapsed() >= deadline {
                    return;
                }
                let mut pos = offset;
                while pos < route.len() {
                    let len = (route.len() - pos).min(window);
                    if len >= 2 && optimize_legal_window(route, weights, n, pos, len) {
                        improved = true;
                    }
                    pos += window;
                }
            }
        }
        cycle += 1;
        if !improved && cycle >= min_cycles {
            break;
        }
    }
}

fn optimize_legal_window(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    start: usize,
    len: usize,
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
        if used >= len {
            continue;
        }
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

    changed
}

fn is_adj(u: usize, v: usize, n: usize) -> bool {
    let ui = u / n;
    let uj = u % n;
    let vi = v / n;
    let vj = v % n;
    ui.abs_diff(vi).max(uj.abs_diff(vj)) == 1
}

fn violation_cost(u: usize, v: usize, n: usize) -> i64 {
    if is_adj(u, v, n) { 0 } else { 1 }
}

fn reversal_delta(l: usize, r: usize, prefix_weight: &[i64], prefix_pos_weight: &[i64]) -> i64 {
    let sum_weight = prefix_weight[r + 1] - prefix_weight[l];
    let sum_pos_weight = prefix_pos_weight[r + 1] - prefix_pos_weight[l];
    (l as i64 + r as i64) * sum_weight - 2 * sum_pos_weight
}

fn reversal_violation_delta(route: &[usize], n: usize, l: usize, r: usize) -> i32 {
    let len = route.len();
    let mut old_vio = 0i64;
    let mut new_vio = 0i64;

    if l > 0 {
        old_vio += violation_cost(route[l - 1], route[l], n);
        new_vio += violation_cost(route[l - 1], route[r], n);
    }
    if r + 1 < len {
        old_vio += violation_cost(route[r], route[r + 1], n);
        new_vio += violation_cost(route[l], route[r + 1], n);
    }

    (new_vio - old_vio) as i32
}

fn relocation_violation_delta(
    route: &[usize],
    n: usize,
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
) -> i32 {
    let a = route[l - 1];
    let b = route[l];
    let c = route[r];
    let d = route[r + 1];
    let e = route[p];
    let f = route[p + 1];

    let old_vio = violation_cost(a, b, n) + violation_cost(c, d, n) + violation_cost(e, f, n);

    let mut new_vio = violation_cost(a, d, n);
    if reversed {
        new_vio += violation_cost(e, c, n);
        new_vio += violation_cost(b, f, n);
    } else {
        new_vio += violation_cost(e, b, n);
        new_vio += violation_cost(c, f, n);
    }

    (new_vio - old_vio) as i32
}

fn edge_disjoint_from_segment(p: usize, l: usize, r: usize) -> bool {
    !(l <= p && p <= r) && !(l <= p + 1 && p + 1 <= r)
}

fn relocation_delta(
    route: &[usize],
    weights: &[i64],
    prefix_weight: &[i64],
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
) -> i64 {
    let seg_len = r - l + 1;
    let old_seg = segment_contrib(route, weights, l, l, seg_len, false);

    if p < l {
        let q = p + 1;
        let between_weight = prefix_weight[l] - prefix_weight[q];
        let shifted = seg_len as i64 * between_weight;
        let new_seg = segment_contrib(route, weights, l, q, seg_len, reversed);
        new_seg - old_seg + shifted
    } else {
        let q = p + 1 - seg_len;
        let between_weight = prefix_weight[p + 1] - prefix_weight[r + 1];
        let shifted = seg_len as i64 * between_weight;
        let new_seg = segment_contrib(route, weights, l, q, seg_len, reversed);
        new_seg - old_seg - shifted
    }
}

fn segment_contrib(
    route: &[usize],
    weights: &[i64],
    src_l: usize,
    dst_l: usize,
    seg_len: usize,
    reversed: bool,
) -> i64 {
    let mut acc = 0i64;
    for t in 0..seg_len {
        let src = if reversed {
            src_l + seg_len - 1 - t
        } else {
            src_l + t
        };
        acc += (dst_l + t) as i64 * weights[route[src]];
    }
    acc
}

fn apply_relocation(route: &mut [usize], mv: RelocationMove) {
    let seg_len = mv.r - mv.l + 1;
    let mut seg = route[mv.l..=mv.r].to_vec();
    if mv.reversed {
        seg.reverse();
    }

    if mv.p < mv.l {
        route.copy_within(mv.p + 1..mv.l, mv.p + 1 + seg_len);
        route[mv.p + 1..mv.p + 1 + seg_len].copy_from_slice(&seg);
    } else {
        route.copy_within(mv.r + 1..=mv.p, mv.l);
        let q = mv.p + 1 - seg_len;
        route[q..q + seg_len].copy_from_slice(&seg);
    }
}

fn compute_raw_score(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}

fn simple_striped_route(n: usize, first_down: bool) -> Vec<usize> {
    let strip_count = n / STRIP_W;
    let mut route = Vec::with_capacity(n * n);
    for strip in 0..strip_count {
        let dir_down = (strip % 2 == 0) == first_down;
        for local_col in 0..STRIP_W {
            let downward = (local_col % 2 == 0) == dir_down;
            let global_col = strip * STRIP_W + local_col;
            if downward {
                for row in 0..n {
                    route.push(row * n + global_col);
                }
            } else {
                for row in (0..n).rev() {
                    route.push(row * n + global_col);
                }
            }
        }
    }
    route
}

fn best_legal_strip_route(n: usize, weights: &[i64]) -> Vec<usize> {
    let cand_a = simple_striped_route(n, true);
    let mut cand_b = simple_striped_route(n, false);
    let mut cand_c = cand_a.clone();
    cand_c.reverse();
    cand_b.reverse();

    let mut best = cand_a;
    let mut best_score = compute_raw_score(&best, weights);
    for cand in [cand_b, cand_c] {
        let score = compute_raw_score(&cand, weights);
        if score > best_score {
            best_score = score;
            best = cand;
        }
    }
    best
}

fn is_valid_route(route: &[usize], n: usize) -> bool {
    let len = route.len();
    let mut seen = vec![false; len];

    for &cell in route {
        if cell >= len || seen[cell] {
            return false;
        }
        seen[cell] = true;
    }

    for i in 0..len - 1 {
        if !is_adj(route[i], route[i + 1], n) {
            return false;
        }
    }
    true
}
