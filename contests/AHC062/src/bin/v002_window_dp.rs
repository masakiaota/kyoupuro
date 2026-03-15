use proconio::input;
use std::time::{Duration, Instant};

const MAX_WINDOW: usize = 9;
const NEG_INF: i64 = i64::MIN / 4;
const TIME_LIMIT: f64 = 1.85;

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let weights = flatten_weights(n, &a);
    let mut route = best_initial_route(n, &weights);
    let mut score = compute_raw_score(&route, &weights);

    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(TIME_LIMIT);
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
                    if len >= 2 && optimize_window(&mut route, &weights, n, pos, len, &mut score) {
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

fn best_initial_route(n: usize, weights: &[i64]) -> Vec<usize> {
    let mut candidates = Vec::with_capacity(4);
    let row = row_snake(n);
    let mut row_rev = row.clone();
    row_rev.reverse();
    let col = col_snake(n);
    let mut col_rev = col.clone();
    col_rev.reverse();
    candidates.push(row);
    candidates.push(row_rev);
    candidates.push(col);
    candidates.push(col_rev);

    let mut best_score = NEG_INF;
    let mut best_route = Vec::new();
    for route in candidates {
        let score = compute_raw_score(&route, weights);
        if score > best_score {
            best_score = score;
            best_route = route;
        }
    }
    best_route
}

fn row_snake(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for i in 0..n {
        if i % 2 == 0 {
            for j in 0..n {
                route.push(i * n + j);
            }
        } else {
            for j in (0..n).rev() {
                route.push(i * n + j);
            }
        }
    }
    route
}

fn col_snake(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for j in 0..n {
        if j % 2 == 0 {
            for i in 0..n {
                route.push(i * n + j);
            }
        } else {
            for i in (0..n).rev() {
                route.push(i * n + j);
            }
        }
    }
    route
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
