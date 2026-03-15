use proconio::input;
use std::time::{Duration, Instant};

const BAND_H: usize = 5;
const BLOCK_W: usize = 2;
const BLOCK_CELLS: usize = BAND_H * BLOCK_W;
const MAX_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;
const TIME_LIMIT: f64 = 1.85;

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

#[derive(Clone, Copy)]
struct TwoOptMove {
    l: usize,
    r: usize,
    delta: i64,
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

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(TIME_LIMIT);

    let weights = flatten_weights(n, &a);
    let neighbors = build_neighbors(n);
    let paths_by_pair = generate_local_paths();

    let mut route = build_best_initial_route(n, &weights, &paths_by_pair);
    let mut score = compute_raw_score(&route, &weights);

    let initial_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.45);
    let two_opt_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.80);

    run_window_descent(
        &mut route,
        &weights,
        n,
        &mut score,
        &start,
        initial_deadline.min(time_limit),
        1,
    );
    run_two_opt_refine_descent(
        &mut route,
        &weights,
        &neighbors,
        n,
        &mut score,
        &start,
        two_opt_deadline.min(time_limit),
    );
    run_window_descent(&mut route, &weights, n, &mut score, &start, time_limit, 1);

    for cell in route {
        println!("{} {}", cell / n, cell % n);
    }
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

fn compute_raw_score(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}

fn compute_raw_score_reversed(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .rev()
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

fn run_window_descent(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    total_score: &mut i64,
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
                    if len >= 2 && optimize_window(route, weights, n, pos, len, total_score) {
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

fn run_two_opt_refine_descent(
    route: &mut [usize],
    weights: &[i64],
    neighbors: &[Vec<usize>],
    n: usize,
    total_score: &mut i64,
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
        let Some(mv) = best_two_opt_move(
            route,
            &position,
            neighbors,
            &prefix_weight,
            &prefix_pos_weight,
            n,
        ) else {
            break;
        };
        if mv.delta <= 0 {
            break;
        }
        route[mv.l..=mv.r].reverse();
        *total_score += mv.delta;
        refine_around(route, weights, n, total_score, &[mv.l, mv.r]);
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

fn best_two_opt_move(
    route: &[usize],
    position: &[usize],
    neighbors: &[Vec<usize>],
    prefix_weight: &[i64],
    prefix_pos_weight: &[i64],
    n: usize,
) -> Option<TwoOptMove> {
    let len = route.len();
    let mut best = None;
    let mut best_delta = 0i64;

    let full_delta = reversal_delta(0, len - 1, prefix_weight, prefix_pos_weight);
    if full_delta > best_delta {
        best_delta = full_delta;
        best = Some(TwoOptMove {
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
            best = Some(TwoOptMove { l: 0, r, delta });
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
            best = Some(TwoOptMove {
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
                best = Some(TwoOptMove { l, r, delta });
            }
        }
    }

    best
}

fn refine_around(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    total_score: &mut i64,
    anchors: &[usize],
) {
    let len = route.len();
    for &anchor in anchors {
        for &window in &[9usize, 8usize] {
            if len < window {
                continue;
            }
            let start_lo = anchor.saturating_sub(window + 2);
            let start_hi = anchor.saturating_add(2).min(len - window);
            for start in start_lo..=start_hi {
                optimize_window(route, weights, n, start, window, total_score);
            }
        }
    }
}

fn reversal_delta(l: usize, r: usize, prefix_weight: &[i64], prefix_pos_weight: &[i64]) -> i64 {
    let sum_weight = prefix_weight[r + 1] - prefix_weight[l];
    let sum_pos_weight = prefix_pos_weight[r + 1] - prefix_pos_weight[l];
    (l as i64 + r as i64) * sum_weight - 2 * sum_pos_weight
}
