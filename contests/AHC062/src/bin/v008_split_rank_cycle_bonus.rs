//! # v008
//! v004 + rank-aware cut bonus

use proconio::input;
use std::time::{Duration, Instant};

const MAX_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;
const TIME_LIMIT: f64 = 1.85;
const LAMBDA_CUT: i64 = 400_000;

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let weights = flatten_weights(n, &a);
    let rank_by_cell = build_rank_by_cell(&weights);
    let neighbors = build_neighbors(n);
    let mut route = best_initial_cycle(n, &weights, &rank_by_cell);
    let mut score = relinearize_best_cut(&mut route, &weights, &rank_by_cell);

    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(TIME_LIMIT);
    let initial_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.20);
    let improve_deadline = Duration::from_secs_f64(TIME_LIMIT * 0.88);

    run_window_descent(
        &mut route,
        &weights,
        n,
        &mut score,
        &start,
        initial_deadline,
        1,
    );

    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];

    while start.elapsed() < improve_deadline {
        score = relinearize_best_cut(&mut route, &weights, &rank_by_cell);
        rebuild_aux(
            &route,
            &weights,
            &mut position,
            &mut prefix_weight,
            &mut prefix_pos_weight,
        );

        let best_reloc =
            best_relocation_move(&route, &weights, &neighbors, &position, &prefix_weight, n);
        let best_two_opt = best_cycle_two_opt_move(
            &route,
            &position,
            &neighbors,
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
            apply_relocation(&mut route, mv);
            score += mv.delta;
            refine_around(&mut route, &weights, n, &mut score, &[mv.l, mv.p]);
        } else {
            let mv = best_two_opt.unwrap();
            route[mv.l..=mv.r].reverse();
            score += mv.delta;
            refine_around(&mut route, &weights, n, &mut score, &[mv.l, mv.r]);
        }
    }

    score = relinearize_best_cut(&mut route, &weights, &rank_by_cell);
    run_window_descent(&mut route, &weights, n, &mut score, &start, time_limit, 1);

    let total_desc = total_cycle_penalty(&route, &rank_by_cell);
    let cut_edge_penalty = edge_rank_penalty(route[route.len() - 1], route[0], &rank_by_cell);
    eprintln!(
        "diag v008 final_raw={} total_desc={} cut_edge_penalty={} path_rank_penalty={}",
        score,
        total_desc,
        cut_edge_penalty,
        total_desc - cut_edge_penalty,
    );

    for cell in route {
        println!("{} {}", cell / n, cell % n);
    }
}

#[derive(Clone, Copy)]
struct RelocationMove {
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
    delta: i64,
}

#[derive(Clone, Copy)]
struct TwoOptMove {
    l: usize,
    r: usize,
    delta: i64,
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

fn build_rank_by_cell(weights: &[i64]) -> Vec<usize> {
    weights.iter().map(|&w| (w - 1) as usize).collect()
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

fn best_initial_cycle(n: usize, weights: &[i64], rank_by_cell: &[usize]) -> Vec<usize> {
    let row = row_cycle(n);
    let col = transpose_cycle(&row, n);
    let mut candidates = [row, col];
    let mut best_score = NEG_INF;
    let mut best_route = Vec::new();
    for route in &mut candidates {
        let score = relinearize_best_cut(route, weights, rank_by_cell);
        if score > best_score {
            best_score = score;
            best_route = route.clone();
        }
    }
    best_route
}

fn row_cycle(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for i in 0..n {
        route.push(i * n);
    }
    let mut forward = true;
    for i in (1..n).rev() {
        if forward {
            for j in 1..n {
                route.push(i * n + j);
            }
        } else {
            for j in (1..n).rev() {
                route.push(i * n + j);
            }
        }
        forward = !forward;
    }
    for j in (1..n).rev() {
        route.push(j);
    }
    route
}

fn transpose_cycle(route: &[usize], n: usize) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| (cell % n) * n + cell / n)
        .collect()
}

fn edge_rank_penalty(u: usize, v: usize, rank_by_cell: &[usize]) -> i64 {
    (rank_by_cell[v] < rank_by_cell[u]) as i64
}

fn total_cycle_penalty(route: &[usize], rank_by_cell: &[usize]) -> i64 {
    let len = route.len();
    let mut total = 0i64;
    for idx in 0..len {
        total += edge_rank_penalty(route[idx], route[(idx + 1) % len], rank_by_cell);
    }
    total
}

fn best_rotation_score(
    route: &[usize],
    weights: &[i64],
    rank_by_cell: &[usize],
) -> (i64, i64, usize) {
    let len = route.len();
    let mut prefix_weight = vec![0i64; 2 * len + 1];
    let mut prefix_pos_weight = vec![0i64; 2 * len + 1];
    for idx in 0..2 * len {
        let w = weights[route[idx % len]];
        prefix_weight[idx + 1] = prefix_weight[idx] + w;
        prefix_pos_weight[idx + 1] = prefix_pos_weight[idx] + idx as i64 * w;
    }

    let mut best_raw_score = NEG_INF;
    let mut best_aug_score = NEG_INF;
    let mut best_start = 0usize;
    for start in 0..len {
        let sum_weight = prefix_weight[start + len] - prefix_weight[start];
        let raw_score =
            prefix_pos_weight[start + len] - prefix_pos_weight[start] - start as i64 * sum_weight;
        let cut_idx = if start == 0 { len - 1 } else { start - 1 };
        let aug_score =
            raw_score + LAMBDA_CUT * edge_rank_penalty(route[cut_idx], route[start], rank_by_cell);
        if aug_score > best_aug_score || (aug_score == best_aug_score && raw_score > best_raw_score)
        {
            best_raw_score = raw_score;
            best_aug_score = aug_score;
            best_start = start;
        }
    }
    (best_raw_score, best_aug_score, best_start)
}

fn relinearize_best_cut(route: &mut Vec<usize>, weights: &[i64], rank_by_cell: &[usize]) -> i64 {
    let (forward_raw, forward_aug, forward_start) =
        best_rotation_score(route, weights, rank_by_cell);
    let mut reversed = route.clone();
    reversed.reverse();
    let (reverse_raw, reverse_aug, reverse_start) =
        best_rotation_score(&reversed, weights, rank_by_cell);

    if reverse_aug > forward_aug || (reverse_aug == forward_aug && reverse_raw > forward_raw) {
        reversed.rotate_left(reverse_start);
        *route = reversed;
        reverse_raw
    } else {
        route.rotate_left(forward_start);
        forward_raw
    }
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
                let mut pos = 1 + offset;
                while pos + window < route.len() {
                    if optimize_window(route, weights, n, pos, window, total_score) {
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

    let left = Some(route[start - 1]);
    let right = Some(route[start + len]);

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

fn best_cycle_two_opt_move(
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

    for l in 0..len {
        let prev_idx = if l == 0 { len - 1 } else { l - 1 };
        let prev = route[prev_idx];
        let first_in = route[l];
        for &cell in &neighbors[prev] {
            let r = position[cell];
            if r <= l {
                continue;
            }
            let next_idx = if r + 1 == len { 0 } else { r + 1 };
            if next_idx == l {
                continue;
            }
            if !is_adj(first_in, route[next_idx], n) {
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

fn reversal_delta(l: usize, r: usize, prefix_weight: &[i64], prefix_pos_weight: &[i64]) -> i64 {
    let sum_weight = prefix_weight[r + 1] - prefix_weight[l];
    let sum_pos_weight = prefix_pos_weight[r + 1] - prefix_pos_weight[l];
    (l as i64 + r as i64) * sum_weight - 2 * sum_pos_weight
}

fn best_relocation_move(
    route: &[usize],
    weights: &[i64],
    neighbors: &[Vec<usize>],
    position: &[usize],
    prefix_weight: &[i64],
    n: usize,
) -> Option<RelocationMove> {
    let len = route.len();
    let mut best = None;
    let mut best_delta = 0i64;

    for seg_len in 1..=3 {
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
                    best = Some(RelocationMove {
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
                    best = Some(RelocationMove {
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
            if len <= window + 2 {
                continue;
            }
            let start_lo = anchor.saturating_sub(window + 2).max(1);
            let start_hi = anchor.saturating_add(2).min(len - window - 1);
            for start in start_lo..=start_hi {
                optimize_window(route, weights, n, start, window, total_score);
            }
        }
    }
}
