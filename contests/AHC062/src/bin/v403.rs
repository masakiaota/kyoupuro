//403

use proconio::input;
use std::time::{Duration, Instant};

const MAX_WINDOW: usize = 12;
const NEG_INF: i64 = i64::MIN / 4;
const TIME_LIMIT: f64 = 2.72;
const GUIDED_RATIO: f64 = 0.46;
const LNS_RATIO: f64 = 0.72;
const IMPROVE_RATIO: f64 = 0.90;
const HIGH_COUNTS: [usize; 3] = [256, 1024, 2048];
const LOW_COUNTS: [usize; 2] = [96, 256];
const TARGET_OFFSETS: [isize; 23] = [
    0, -1, 1, -2, 2, -4, 4, -8, 8, -12, 12, -20, 20, -32, 32, -48, 48, -64, 64, -96, 96, -128, 128,
];
const MOVE_RADIUS: usize = 192;
const GUIDED_SEG_MAX: usize = 8;
const HIGH_CAND_LIMIT: usize = 19;
const LOW_CAND_LIMIT: usize = 10;
const NEG_DIVISOR: i64 = 4;
const LNS_HIGH_COUNT: usize = 2048;
const LNS_LOW_COUNT: usize = 384;
const LNS_ANCHOR_HIGH: usize = 15;
const LNS_ANCHOR_LOW: usize = 8;
const LNS_WINDOWS: [usize; 4] = [12, 11, 10, 9];

#[derive(Clone, Copy)]
struct Symmetry {
    transpose: bool,
    flip_row: bool,
    flip_col: bool,
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

    let weights = flatten_weights(n, &a);
    let neighbors = build_neighbors(n);
    let rank_to_cell = build_rank_to_cell(&weights);
    let mut route = best_initial_cycle(n, &weights);
    let mut score = relinearize_best_cut(&mut route, &weights);

    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(TIME_LIMIT);
    let guided_deadline = Duration::from_secs_f64(TIME_LIMIT * GUIDED_RATIO);
    let lns_deadline = Duration::from_secs_f64(TIME_LIMIT * LNS_RATIO);
    let improve_deadline = Duration::from_secs_f64(TIME_LIMIT * IMPROVE_RATIO);

    run_cycle_window_descent(
        &mut route,
        &weights,
        n,
        &mut score,
        &start,
        Duration::from_secs_f64(TIME_LIMIT * 0.14),
        1,
    );

    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];

    while start.elapsed() < guided_deadline {
        score = relinearize_best_cut(&mut route, &weights);
        rebuild_aux(
            &route,
            &weights,
            &mut position,
            &mut prefix_weight,
            &mut prefix_pos_weight,
        );
        let moved = run_guided_pass(
            &mut route,
            &weights,
            &rank_to_cell,
            &position,
            &prefix_weight,
            n,
            &start,
            guided_deadline,
            &mut score,
        );
        if !moved {
            break;
        }
    }

    score = relinearize_best_cut(&mut route, &weights);
    run_targeted_path_lns(
        &mut route,
        &weights,
        &rank_to_cell,
        n,
        &start,
        lns_deadline,
        &mut score,
    );

    while start.elapsed() < improve_deadline {
        score = relinearize_best_cut(&mut route, &weights);
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
            refine_cycle_around(&mut route, &weights, n, &mut score, &[mv.l, mv.p]);
        } else {
            let mv = best_two_opt.unwrap();
            route[mv.l..=mv.r].reverse();
            score += mv.delta;
            refine_cycle_around(&mut route, &weights, n, &mut score, &[mv.l, mv.r]);
        }
    }

    score = relinearize_best_cut(&mut route, &weights);
    run_path_window_descent(&mut route, &weights, n, &mut score, &start, time_limit, 1);

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

fn build_rank_to_cell(weights: &[i64]) -> Vec<usize> {
    let mut rank_to_cell = vec![0usize; weights.len()];
    for (cell, &w) in weights.iter().enumerate() {
        rank_to_cell[(w - 1) as usize] = cell;
    }
    rank_to_cell
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

fn best_initial_cycle(n: usize, weights: &[i64]) -> Vec<usize> {
    let row = row_cycle(n);
    let col = transpose_route(&row, n);
    let bases = [row, col];

    let mut best_route = Vec::new();
    let mut best_score = NEG_INF;

    for &symmetry in &SYMMETRIES {
        for base in &bases {
            let mut route = apply_symmetry_to_route(base, n, symmetry);
            if !check_cycle(&route, n) {
                continue;
            }
            let score = relinearize_best_cut(&mut route, weights);
            if score > best_score {
                best_score = score;
                best_route = route;
            }
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

fn transpose_route(route: &[usize], n: usize) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| (cell % n) * n + cell / n)
        .collect()
}

fn apply_symmetry_to_route(route: &[usize], n: usize, symmetry: Symmetry) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| {
            let i = cell / n;
            let j = cell % n;
            let (r, c) = transform_cell(i, j, n, symmetry);
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

fn check_cycle(route: &[usize], n: usize) -> bool {
    route.windows(2).all(|w| is_adj(w[0], w[1], n))
        && is_adj(*route.first().unwrap(), *route.last().unwrap(), n)
}

fn best_rotation_score(route: &[usize], weights: &[i64]) -> (i64, usize) {
    let len = route.len();
    let mut prefix_weight = vec![0i64; 2 * len + 1];
    let mut prefix_pos_weight = vec![0i64; 2 * len + 1];
    for idx in 0..2 * len {
        let w = weights[route[idx % len]];
        prefix_weight[idx + 1] = prefix_weight[idx] + w;
        prefix_pos_weight[idx + 1] = prefix_pos_weight[idx] + idx as i64 * w;
    }

    let mut best_score = NEG_INF;
    let mut best_start = 0usize;
    for start in 0..len {
        let sum_weight = prefix_weight[start + len] - prefix_weight[start];
        let score =
            prefix_pos_weight[start + len] - prefix_pos_weight[start] - start as i64 * sum_weight;
        if score > best_score {
            best_score = score;
            best_start = start;
        }
    }
    (best_score, best_start)
}

fn relinearize_best_cut(route: &mut Vec<usize>, weights: &[i64]) -> i64 {
    let (forward_score, forward_start) = best_rotation_score(route, weights);
    let mut reversed = route.clone();
    reversed.reverse();
    let (reverse_score, reverse_start) = best_rotation_score(&reversed, weights);

    if reverse_score > forward_score {
        reversed.rotate_left(reverse_start);
        *route = reversed;
        reverse_score
    } else {
        route.rotate_left(forward_start);
        forward_score
    }
}

fn is_adj(u: usize, v: usize, n: usize) -> bool {
    let ui = u / n;
    let uj = u % n;
    let vi = v / n;
    let vj = v % n;
    ui.abs_diff(vi).max(uj.abs_diff(vj)) == 1
}

fn run_guided_pass(
    route: &mut [usize],
    weights: &[i64],
    rank_to_cell: &[usize],
    position: &[usize],
    prefix_weight: &[i64],
    n: usize,
    start: &Instant,
    deadline: Duration,
    total_score: &mut i64,
) -> bool {
    let len = route.len();
    for &count in &HIGH_COUNTS {
        let candidates =
            collect_high_targets(rank_to_cell, position, count.min(len), HIGH_CAND_LIMIT);
        for &(cell, target, _) in &candidates {
            if start.elapsed() >= deadline {
                return false;
            }
            if let Some(mv) =
                best_guided_move(route, weights, position, prefix_weight, n, cell, target)
            {
                apply_relocation(route, mv);
                *total_score += mv.delta;
                refine_cycle_around(&mut route[..], weights, n, total_score, &[mv.l, mv.p]);
                return true;
            }
        }
    }

    for &count in &LOW_COUNTS {
        let candidates =
            collect_low_targets(rank_to_cell, position, count.min(len), LOW_CAND_LIMIT);
        for &(cell, target, _) in &candidates {
            if start.elapsed() >= deadline {
                return false;
            }
            if let Some(mv) =
                best_guided_move(route, weights, position, prefix_weight, n, cell, target)
            {
                apply_relocation(route, mv);
                *total_score += mv.delta;
                refine_cycle_around(&mut route[..], weights, n, total_score, &[mv.l, mv.p]);
                return true;
            }
        }
    }

    false
}

fn collect_high_targets(
    rank_to_cell: &[usize],
    position: &[usize],
    count: usize,
    limit: usize,
) -> Vec<(usize, usize, usize)> {
    let len = position.len();
    let mut slots = Vec::with_capacity(count);
    for idx in 0..count {
        let rank = len - count + idx;
        slots.push(position[rank_to_cell[rank]]);
    }
    slots.sort_unstable();

    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let rank = len - count + idx;
        let cell = rank_to_cell[rank];
        let cur = position[cell];
        let target = slots[idx];
        let disp = cur.abs_diff(target);
        if disp > 6 {
            out.push((cell, target, disp));
        }
    }
    out.sort_unstable_by(|a, b| b.2.cmp(&a.2));
    out.truncate(limit);
    out
}

fn collect_low_targets(
    rank_to_cell: &[usize],
    position: &[usize],
    count: usize,
    limit: usize,
) -> Vec<(usize, usize, usize)> {
    let mut slots = Vec::with_capacity(count);
    for rank in 0..count {
        slots.push(position[rank_to_cell[rank]]);
    }
    slots.sort_unstable();

    let mut out = Vec::with_capacity(count);
    for rank in 0..count {
        let cell = rank_to_cell[rank];
        let cur = position[cell];
        let target = slots[rank];
        let disp = cur.abs_diff(target);
        if disp > 6 {
            out.push((cell, target, disp));
        }
    }
    out.sort_unstable_by(|a, b| b.2.cmp(&a.2));
    out.truncate(limit);
    out
}

fn best_guided_move(
    route: &[usize],
    weights: &[i64],
    position: &[usize],
    prefix_weight: &[i64],
    n: usize,
    cell: usize,
    target: usize,
) -> Option<RelocationMove> {
    let len = route.len();
    let cur = position[cell];
    if cur == 0 || cur + 1 >= len {
        return None;
    }

    let mut best = None;
    let mut best_key = i128::MIN;
    for seg_len in 1..=GUIDED_SEG_MAX.min(len.saturating_sub(2)) {
        let l_min = cur.saturating_sub(seg_len - 1).max(1);
        let l_max = cur.min(len - seg_len - 1);
        for l in l_min..=l_max {
            let r = l + seg_len - 1;
            let off = cur - l;
            let seg_sum = prefix_weight[r + 1] - prefix_weight[l];
            let mut desired_starts = [0usize; 2];
            desired_starts[0] = target.saturating_sub(off);
            desired_starts[1] = target.saturating_sub(seg_len - 1 - off);
            for (reversed_idx, &desired_start) in desired_starts.iter().enumerate() {
                let reversed = reversed_idx == 1;
                let want_late = target > cur;
                let base_anchor = if want_late {
                    desired_start.saturating_add(seg_len - 1)
                } else {
                    desired_start.saturating_sub(1)
                };
                for &delta in &TARGET_OFFSETS {
                    let cand_anchor = shift_index(base_anchor, delta, len - 2);
                    let dist = cand_anchor.abs_diff(base_anchor);
                    if dist > MOVE_RADIUS {
                        continue;
                    }
                    if let Some(mv) = evaluate_guided_relocation(
                        route,
                        weights,
                        prefix_weight,
                        n,
                        l,
                        r,
                        cand_anchor,
                        reversed,
                    ) {
                        let new_pos = moved_cell_position(cur, l, r, cand_anchor, reversed);
                        let before = cur.abs_diff(target);
                        let after = new_pos.abs_diff(target);
                        if after >= before {
                            continue;
                        }
                        let improvement = (before - after) as i64;
                        let slot_gain = improvement * weights[cell];
                        if mv.delta + slot_gain / NEG_DIVISOR <= 0 {
                            continue;
                        }
                        let key = (mv.delta as i128) * 4096 + (slot_gain as i128) * 24
                            - (seg_sum as i128 - weights[cell] as i128) * 16
                            - dist as i128 * (weights[cell] as i128 / 4 + 1);
                        if key > best_key {
                            best_key = key;
                            best = Some(mv);
                        }
                    }
                }
            }
        }
    }
    best
}

fn shift_index(base: usize, delta: isize, upper: usize) -> usize {
    if delta >= 0 {
        base.saturating_add(delta as usize).min(upper)
    } else {
        base.saturating_sub((-delta) as usize).min(upper)
    }
}

fn evaluate_guided_relocation(
    route: &[usize],
    weights: &[i64],
    prefix_weight: &[i64],
    n: usize,
    l: usize,
    r: usize,
    p: usize,
    reversed: bool,
) -> Option<RelocationMove> {
    if !is_adj(route[l - 1], route[r + 1], n) {
        return None;
    }
    if !edge_disjoint_from_segment(p, l, r) {
        return None;
    }
    let first = if reversed { route[r] } else { route[l] };
    let last = if reversed { route[l] } else { route[r] };
    if !is_adj(route[p], first, n) || !is_adj(last, route[p + 1], n) {
        return None;
    }
    let delta = relocation_delta(route, weights, prefix_weight, l, r, p, reversed);
    Some(RelocationMove {
        l,
        r,
        p,
        reversed,
        delta,
    })
}

fn moved_cell_position(cur: usize, l: usize, r: usize, p: usize, reversed: bool) -> usize {
    let seg_len = r - l + 1;
    let off = cur - l;
    let new_off = if reversed { seg_len - 1 - off } else { off };
    if p < l {
        p + 1 + new_off
    } else {
        p + 1 - seg_len + new_off
    }
}

fn run_targeted_path_lns(
    route: &mut [usize],
    weights: &[i64],
    rank_to_cell: &[usize],
    n: usize,
    start: &Instant,
    deadline: Duration,
    total_score: &mut i64,
) {
    let len = route.len();
    let mut position = vec![0usize; len];
    let mut prefix_weight = vec![0i64; len + 1];
    let mut prefix_pos_weight = vec![0i64; len + 1];

    loop {
        if start.elapsed() >= deadline {
            break;
        }
        rebuild_aux(
            route,
            weights,
            &mut position,
            &mut prefix_weight,
            &mut prefix_pos_weight,
        );
        let mut anchors = collect_high_targets(
            rank_to_cell,
            &position,
            LNS_HIGH_COUNT.min(len),
            LNS_ANCHOR_HIGH,
        );
        let low = collect_low_targets(
            rank_to_cell,
            &position,
            LNS_LOW_COUNT.min(len),
            LNS_ANCHOR_LOW,
        );
        anchors.extend(low);

        let mut improved = false;
        for (cell, target, _) in anchors {
            if start.elapsed() >= deadline {
                break;
            }
            let cur = position[cell];
            for &window in &LNS_WINDOWS {
                if window + 2 >= len {
                    continue;
                }
                let mid = (cur + target) / 2;
                let centers = [
                    cur,
                    target,
                    mid,
                    cur.min(target) + window / 2,
                    cur.max(target).saturating_sub(window / 2),
                ];
                for &center in &centers {
                    let start_idx = clamp_window_start(center, window, len);
                    if optimize_path_window(route, weights, n, start_idx, window, total_score) {
                        improved = true;
                        break;
                    }
                }
                if improved {
                    break;
                }
            }
            if improved {
                break;
            }
        }

        if !improved {
            break;
        }
    }
}

fn clamp_window_start(center: usize, window: usize, len: usize) -> usize {
    center.saturating_sub(window / 2).clamp(1, len - window - 1)
}

fn run_cycle_window_descent(
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
                    if optimize_cycle_window(route, weights, n, pos, window, total_score) {
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

fn optimize_cycle_window(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    start_idx: usize,
    len: usize,
    total_score: &mut i64,
) -> bool {
    if len >= route.len() {
        return false;
    }

    let total_len = route.len();
    let mut seg = [0usize; MAX_WINDOW];
    let mut pos_list = [0usize; MAX_WINDOW];
    for idx in 0..len {
        let pos = (start_idx + idx) % total_len;
        seg[idx] = route[pos];
        pos_list[idx] = pos;
    }

    let left = route[(start_idx + total_len - 1) % total_len];
    let right = route[(start_idx + len) % total_len];

    let old_local = (0..len)
        .map(|idx| pos_list[idx] as i64 * weights[seg[idx]])
        .sum::<i64>();

    let states = 1usize << len;
    let width = len;
    let mut dp = vec![NEG_INF; states * width];
    let mut parent = vec![u8::MAX; states * width];

    for first in 0..len {
        if is_adj(left, seg[first], n) {
            let mask = 1usize << first;
            dp[mask * width + first] = pos_list[0] as i64 * weights[seg[first]];
        }
    }

    for mask in 1usize..states {
        let used = mask.count_ones() as usize;
        if used >= len {
            continue;
        }
        let next_pos = pos_list[used];
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
        if value == NEG_INF || !is_adj(seg[last], right, n) {
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
        let pos = pos_list[idx];
        if route[pos] != cell {
            route[pos] = cell;
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

fn refine_cycle_around(
    route: &mut [usize],
    weights: &[i64],
    n: usize,
    total_score: &mut i64,
    anchors: &[usize],
) {
    let len = route.len();
    for &anchor in anchors {
        for &window in &[9usize, 8usize] {
            let span = window + 2;
            for shift in 0..=(2 * span) {
                let start = (anchor + len - span + shift) % len;
                optimize_cycle_window(route, weights, n, start, window, total_score);
            }
        }
    }
}

fn run_path_window_descent(
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
                    if len >= 2 && optimize_path_window(route, weights, n, pos, len, total_score) {
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

fn optimize_path_window(
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

    if changed {
        *total_score += best_local - old_local;
    }
    changed
}

fn reversal_delta(l: usize, r: usize, prefix_weight: &[i64], prefix_pos_weight: &[i64]) -> i64 {
    let sum_weight = prefix_weight[r + 1] - prefix_weight[l];
    let sum_pos_weight = prefix_pos_weight[r + 1] - prefix_pos_weight[l];
    (l as i64 + r as i64) * sum_weight - 2 * sum_pos_weight
}
