// v008_stack_schedule_delta.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const ORDER_SEARCH_TIME_LIMIT_SEC: f64 = 1.35;
const SCHEDULE_SEARCH_TIME_LIMIT_SEC: f64 = 0.45;
const CHECKPOINT_BLOCK: usize = 16;
const INF: usize = usize::MAX / 4;

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    initial_order_dist: usize,
    final_order_dist: usize,
    initial_schedule_dist: usize,
    final_schedule_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    early_close_count: usize,
    max_stack_depth: usize,
    move_count: usize,
    turn_count: usize,
    order_iterations: usize,
    order_accepted: usize,
    order_improved: usize,
    schedule_iterations: usize,
    schedule_accepted: usize,
    schedule_improved: usize,
    order_elapsed_ms: f64,
    schedule_elapsed_ms: f64,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] order={} -> {} schedule={} -> {} collect={} delete={} early_close={} max_depth={} moves={} turns={} score_est={} order_iter={} order_accepted={} order_improved={} schedule_iter={} schedule_accepted={} schedule_improved={} order_elapsed_ms={:.3} schedule_elapsed_ms={:.3}",
            self.initial_order_dist,
            self.final_order_dist,
            self.initial_schedule_dist,
            self.final_schedule_dist,
            self.collect_dist,
            self.delete_dist,
            self.early_close_count,
            self.max_stack_depth,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count,
            self.order_iterations,
            self.order_accepted,
            self.order_improved,
            self.schedule_iterations,
            self.schedule_accepted,
            self.schedule_improved,
            self.order_elapsed_ms,
            self.schedule_elapsed_ms,
        );
    }
}

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[derive(Debug, Clone)]
struct Input {
    pos: [[usize; 2]; M],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n = it.next().unwrap().parse::<usize>().unwrap();
        assert_eq!(n, N);

        let mut pos = [[usize::MAX; 2]; M];
        let mut count = [0usize; M];
        for id in 0..NN {
            let v = it.next().unwrap().parse::<usize>().unwrap();
            assert!(v < M);
            let k = count[v];
            assert!(k < 2);
            pos[v][k] = id;
            count[v] += 1;
        }
        for &c in &count {
            assert_eq!(c, 2);
        }

        Self { pos }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteNode {
    v: usize,
    side: usize,
}

#[derive(Debug, Clone, Copy)]
struct SearchStats {
    initial_total_dist: usize,
    final_total_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    elapsed_ms: f64,
}

#[derive(Debug, Clone, Copy)]
struct ScheduleStats {
    initial_total_dist: usize,
    final_total_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    early_close_count: usize,
    max_stack_depth: usize,
    elapsed_ms: f64,
}

#[derive(Debug, Clone, Copy)]
struct ScheduleBreakdown {
    total_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    early_close_count: usize,
    max_stack_depth: usize,
}

#[derive(Debug, Clone)]
struct ScheduleSnapshot {
    idx: usize,
    cur: usize,
    stack: [usize; M],
    stack_len: usize,
    collect_dist: usize,
    delete_dist: usize,
    early_close_count: usize,
    max_stack_depth: usize,
}

#[derive(Debug, Clone)]
struct Rng {
    x: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { x: seed.max(1) }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    #[inline(always)]
    fn gen_usize(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n
    }

    #[inline(always)]
    fn gen_f64(&mut self) -> f64 {
        const DEN: f64 = (1_u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / DEN
    }
}

#[inline(always)]
fn dist(p: usize, q: usize) -> usize {
    let pi = p / N;
    let pj = p % N;
    let qi = q / N;
    let qj = q % N;
    pi.abs_diff(qi) + pj.abs_diff(qj)
}

#[inline(always)]
fn start_cost(input: &Input, v: usize, side: usize) -> usize {
    dist(0, input.pos[v][side])
}

#[inline(always)]
fn pair_cost(input: &Input, v: usize) -> usize {
    dist(input.pos[v][0], input.pos[v][1])
}

#[inline(always)]
fn transition_cost(input: &Input, prev_v: usize, prev_side: usize, v: usize, side: usize) -> usize {
    dist(input.pos[prev_v][prev_side], input.pos[v][side])
        + dist(input.pos[prev_v][prev_side ^ 1], input.pos[v][side ^ 1])
}

fn transition_min_cost(input: &Input, prev_v: usize, v: usize) -> usize {
    let mut best = INF;
    for prev_side in 0..2 {
        for side in 0..2 {
            best = best.min(transition_cost(input, prev_v, prev_side, v, side));
        }
    }
    best
}

fn score_order(input: &Input, order: &[usize]) -> usize {
    let first = order[0];
    let mut dp = [start_cost(input, first, 0), start_cost(input, first, 1)];

    for i in 1..order.len() {
        let prev_v = order[i - 1];
        let v = order[i];
        let mut next = [INF; 2];
        for prev_side in 0..2 {
            for side in 0..2 {
                let cost = dp[prev_side] + transition_cost(input, prev_v, prev_side, v, side);
                if cost < next[side] {
                    next[side] = cost;
                }
            }
        }
        dp = next;
    }

    let last = order[order.len() - 1];
    dp[0].min(dp[1]) + pair_cost(input, last)
}

fn restore_route(input: &Input, order: &[usize]) -> (Vec<RouteNode>, usize) {
    let first = order[0];
    let mut dp = [start_cost(input, first, 0), start_cost(input, first, 1)];
    let mut parent = [[0usize; 2]; M];

    for i in 1..order.len() {
        let prev_v = order[i - 1];
        let v = order[i];
        let mut next = [INF; 2];
        for prev_side in 0..2 {
            for side in 0..2 {
                let cost = dp[prev_side] + transition_cost(input, prev_v, prev_side, v, side);
                if cost < next[side] {
                    next[side] = cost;
                    parent[i][side] = prev_side;
                }
            }
        }
        dp = next;
    }

    let last = order[order.len() - 1];
    let mut last_side = if dp[0] <= dp[1] { 0 } else { 1 };
    let score = dp[last_side] + pair_cost(input, last);
    let mut sides = [0usize; M];
    sides[M - 1] = last_side;
    for i in (1..M).rev() {
        last_side = parent[i][last_side];
        sides[i - 1] = last_side;
    }

    let mut route = Vec::with_capacity(M);
    for i in 0..M {
        route.push(RouteNode {
            v: order[i],
            side: sides[i],
        });
    }
    (route, score)
}

fn make_initial_order(input: &Input) -> Vec<usize> {
    let mut used = [false; M];
    let mut order = Vec::with_capacity(M);

    let mut first = usize::MAX;
    let mut first_key = (INF, INF, INF);
    for v in 0..M {
        let start = start_cost(input, v, 0).min(start_cost(input, v, 1));
        let key = (start, pair_cost(input, v), v);
        if key < first_key {
            first = v;
            first_key = key;
        }
    }
    used[first] = true;
    order.push(first);

    while order.len() < M {
        let prev = *order.last().unwrap();
        let mut best_v = usize::MAX;
        let mut best_key = (INF, INF);
        for v in 0..M {
            if used[v] {
                continue;
            }
            let key = (transition_min_cost(input, prev, v), v);
            if key < best_key {
                best_v = v;
                best_key = key;
            }
        }
        used[best_v] = true;
        order.push(best_v);
    }

    order
}

fn improve_initial_by_reversal(input: &Input, order: &mut [usize], score: &mut usize) {
    let deadline = Instant::now();
    let mut improved = true;
    while improved && deadline.elapsed().as_secs_f64() < 0.05 {
        improved = false;
        for l in 0..M - 1 {
            for r in l + 1..M {
                order[l..=r].reverse();
                let next_score = score_order(input, order);
                if next_score < *score {
                    *score = next_score;
                    improved = true;
                } else {
                    order[l..=r].reverse();
                }
                if deadline.elapsed().as_secs_f64() >= 0.05 {
                    return;
                }
            }
        }
    }
}

fn accept(delta: i32, temp: f64, rng: &mut Rng) -> bool {
    delta <= 0 || rng.gen_f64() < (-(delta as f64) / temp).exp()
}

fn apply_relocate(order: &mut [usize], i: usize, j: usize) {
    if i < j {
        order[i..=j].rotate_left(1);
    } else {
        order[j..=i].rotate_right(1);
    }
}

fn undo_relocate(order: &mut [usize], i: usize, j: usize) {
    if i < j {
        order[i..=j].rotate_right(1);
    } else {
        order[j..=i].rotate_left(1);
    }
}

fn optimize_order(input: &Input) -> (Vec<RouteNode>, SearchStats) {
    let start = Instant::now();
    let mut order = make_initial_order(input);
    let mut current_score = score_order(input, &order);
    improve_initial_by_reversal(input, &mut order, &mut current_score);

    let initial_score = current_score;
    let mut best_score = current_score;
    let mut best_order = order.clone();
    let mut rng = Rng::new(0x517c_c1b7_d24b_8f1d ^ initial_score as u64);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut temp = 10.0;

    loop {
        iterations += 1;
        if (iterations & 255) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= ORDER_SEARCH_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / ORDER_SEARCH_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 10.0_f64.powf(1.0 - progress) * 0.05_f64.powf(progress);
        }

        let move_type = rng.gen_usize(100);
        let old_score = current_score;
        if move_type < 45 {
            let mut l = rng.gen_usize(M);
            let mut r = rng.gen_usize(M);
            if l == r {
                continue;
            }
            if l > r {
                std::mem::swap(&mut l, &mut r);
            }
            order[l..=r].reverse();
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
            } else {
                order[l..=r].reverse();
                continue;
            }
        } else if move_type < 75 {
            let mut i = rng.gen_usize(M);
            let mut j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            order.swap(i, j);
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
            } else {
                order.swap(i, j);
                continue;
            }
        } else {
            let i = rng.gen_usize(M);
            let j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            apply_relocate(&mut order, i, j);
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
            } else {
                undo_relocate(&mut order, i, j);
                continue;
            }
        }

        if current_score < best_score {
            best_score = current_score;
            best_order.clone_from(&order);
            improved += 1;
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let (route, restored_score) = restore_route(input, &best_order);
    assert_eq!(best_score, restored_score);
    (
        route,
        SearchStats {
            initial_total_dist: initial_score,
            final_total_dist: best_score,
            iterations,
            accepted,
            improved,
            elapsed_ms: elapsed * 1000.0,
        },
    )
}

#[inline(always)]
fn selected_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side]
}

#[inline(always)]
fn other_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side ^ 1]
}

fn collect_score(input: &Input, route: &[RouteNode]) -> usize {
    let mut cur = 0usize;
    let mut score = 0usize;
    for &node in route {
        let p = selected_pos(input, node);
        score += dist(cur, p);
        cur = p;
    }
    score
}

fn delete_score(input: &Input, route: &[RouteNode]) -> usize {
    let mut cur = selected_pos(input, *route.last().unwrap());
    let mut score = 0usize;
    for &node in route.iter().rev() {
        let p = other_pos(input, node);
        score += dist(cur, p);
        cur = p;
    }
    score
}

fn score_schedule(
    input: &Input,
    route: &[RouteNode],
    close_after: &[usize; M],
) -> Option<ScheduleBreakdown> {
    let mut cur = 0usize;
    let mut stack = Vec::with_capacity(M);
    let mut collect_dist = 0usize;
    let mut delete_dist = 0usize;
    let mut early_close_count = 0usize;
    let mut max_stack_depth = 0usize;

    for i in 0..M {
        let selected = selected_pos(input, route[i]);
        collect_dist += dist(cur, selected);
        cur = selected;
        stack.push(i);
        max_stack_depth = max_stack_depth.max(stack.len());

        for _ in 0..close_after[i] {
            let Some(top) = stack.pop() else {
                return None;
            };
            let other = other_pos(input, route[top]);
            if i + 1 == M {
                delete_dist += dist(cur, other);
            } else {
                collect_dist += dist(cur, other);
                early_close_count += 1;
            }
            cur = other;
        }
    }

    if !stack.is_empty() {
        return None;
    }

    Some(ScheduleBreakdown {
        total_dist: collect_dist + delete_dist,
        collect_dist,
        delete_dist,
        early_close_count,
        max_stack_depth,
    })
}

fn find_nonzero_close(close_after: &[usize; M], rng: &mut Rng) -> Option<usize> {
    for _ in 0..8 {
        let i = rng.gen_usize(M);
        if close_after[i] > 0 {
            return Some(i);
        }
    }
    let offset = rng.gen_usize(M);
    for d in 0..M {
        let i = (offset + d) % M;
        if close_after[i] > 0 {
            return Some(i);
        }
    }
    None
}

fn simulate_schedule_suffix(
    input: &Input,
    route: &[RouteNode],
    close_after: &[usize; M],
    snapshot: &ScheduleSnapshot,
    delta_from: usize,
    delta_to: usize,
) -> Option<ScheduleBreakdown> {
    let mut cur = snapshot.cur;
    let mut stack = [0usize; M];
    stack[..snapshot.stack_len].copy_from_slice(&snapshot.stack[..snapshot.stack_len]);
    let mut stack_len = snapshot.stack_len;
    let mut collect_dist = snapshot.collect_dist;
    let mut delete_dist = snapshot.delete_dist;
    let mut early_close_count = snapshot.early_close_count;
    let mut max_stack_depth = snapshot.max_stack_depth;

    for i in snapshot.idx..M {
        let selected = selected_pos(input, route[i]);
        collect_dist += dist(cur, selected);
        cur = selected;
        stack[stack_len] = i;
        stack_len += 1;
        max_stack_depth = max_stack_depth.max(stack_len);

        let mut close_count = close_after[i];
        if i == delta_from {
            close_count -= 1;
        }
        if i == delta_to {
            close_count += 1;
        }

        for _ in 0..close_count {
            if stack_len == 0 {
                return None;
            }
            stack_len -= 1;
            let top = stack[stack_len];
            let other = other_pos(input, route[top]);
            if i + 1 == M {
                delete_dist += dist(cur, other);
            } else {
                collect_dist += dist(cur, other);
                early_close_count += 1;
            }
            cur = other;
        }
    }

    if stack_len != 0 {
        return None;
    }

    Some(ScheduleBreakdown {
        total_dist: collect_dist + delete_dist,
        collect_dist,
        delete_dist,
        early_close_count,
        max_stack_depth,
    })
}

fn build_schedule_snapshots(
    input: &Input,
    route: &[RouteNode],
    close_after: &[usize; M],
) -> Vec<ScheduleSnapshot> {
    let mut snapshots = Vec::with_capacity(M / CHECKPOINT_BLOCK + 2);
    let mut cur = 0usize;
    let mut stack = [0usize; M];
    let mut stack_len = 0usize;
    let mut collect_dist = 0usize;
    let mut delete_dist = 0usize;
    let mut early_close_count = 0usize;
    let mut max_stack_depth = 0usize;

    snapshots.push(ScheduleSnapshot {
        idx: 0,
        cur,
        stack,
        stack_len,
        collect_dist,
        delete_dist,
        early_close_count,
        max_stack_depth,
    });

    for i in 0..M {
        let selected = selected_pos(input, route[i]);
        collect_dist += dist(cur, selected);
        cur = selected;
        stack[stack_len] = i;
        stack_len += 1;
        max_stack_depth = max_stack_depth.max(stack_len);

        for _ in 0..close_after[i] {
            stack_len -= 1;
            let top = stack[stack_len];
            let other = other_pos(input, route[top]);
            if i + 1 == M {
                delete_dist += dist(cur, other);
            } else {
                collect_dist += dist(cur, other);
                early_close_count += 1;
            }
            cur = other;
        }

        let next_idx = i + 1;
        if next_idx < M && next_idx % CHECKPOINT_BLOCK == 0 {
            snapshots.push(ScheduleSnapshot {
                idx: next_idx,
                cur,
                stack,
                stack_len,
                collect_dist,
                delete_dist,
                early_close_count,
                max_stack_depth,
            });
        }
    }

    snapshots
}

fn snapshot_for_change(
    snapshots: &[ScheduleSnapshot],
    from: usize,
    to: usize,
) -> &ScheduleSnapshot {
    let start_idx = from.min(to) / CHECKPOINT_BLOCK * CHECKPOINT_BLOCK;
    let snapshot_idx = start_idx / CHECKPOINT_BLOCK;
    &snapshots[snapshot_idx]
}

fn optimize_schedule(
    input: &Input,
    route: &[RouteNode],
    pure_stack_score: usize,
) -> ([usize; M], ScheduleStats) {
    let start = Instant::now();
    let mut close_after = [0usize; M];
    close_after[M - 1] = M;
    let initial = score_schedule(input, route, &close_after).unwrap();
    assert_eq!(initial.total_dist, pure_stack_score);

    let mut current_score = initial.total_dist;
    let mut best_score = current_score;
    let mut best_close_after = close_after;
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15 ^ pure_stack_score as u64);
    let mut snapshots = build_schedule_snapshots(input, route, &close_after);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut temp = 4.0;

    loop {
        iterations += 1;
        if (iterations & 255) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= SCHEDULE_SEARCH_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / SCHEDULE_SEARCH_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 4.0_f64.powf(1.0 - progress) * 0.02_f64.powf(progress);
        }

        let Some(from) = find_nonzero_close(&close_after, &mut rng) else {
            break;
        };
        let mut to = rng.gen_usize(M);
        if to == from {
            to = (to + 1) % M;
        }

        let snapshot = snapshot_for_change(&snapshots, from, to);
        let Some(next) = simulate_schedule_suffix(input, route, &close_after, snapshot, from, to)
        else {
            continue;
        };

        let delta = next.total_dist as i32 - current_score as i32;
        if accept(delta, temp, &mut rng) {
            close_after[from] -= 1;
            close_after[to] += 1;
            current_score = next.total_dist;
            accepted += 1;
            snapshots = build_schedule_snapshots(input, route, &close_after);
            if current_score < best_score {
                best_score = current_score;
                best_close_after = close_after;
                improved += 1;
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let best = score_schedule(input, route, &best_close_after).unwrap();
    assert_eq!(best_score, best.total_dist);
    (
        best_close_after,
        ScheduleStats {
            initial_total_dist: initial.total_dist,
            final_total_dist: best_score,
            iterations,
            accepted,
            improved,
            early_close_count: best.early_close_count,
            max_stack_depth: best.max_stack_depth,
            elapsed_ms: elapsed * 1000.0,
        },
    )
}

#[inline(always)]
fn push_op(ops: &mut Vec<u8>, op: u8) {
    ops.push(op);
    ops.push(b'\n');
}

fn move_to(cur: &mut usize, dst: usize, ops: &mut Vec<u8>) -> usize {
    let mut moved = 0usize;
    let mut i = *cur / N;
    let mut j = *cur % N;
    let ti = dst / N;
    let tj = dst % N;

    while i < ti {
        push_op(ops, b'D');
        i += 1;
        moved += 1;
    }
    while i > ti {
        push_op(ops, b'U');
        i -= 1;
        moved += 1;
    }
    while j < tj {
        push_op(ops, b'R');
        j += 1;
        moved += 1;
    }
    while j > tj {
        push_op(ops, b'L');
        j -= 1;
        moved += 1;
    }

    *cur = dst;
    moved
}

fn solve(input: &Input) -> Vec<u8> {
    let (route, search_stats) = optimize_order(input);
    let pure_collect_dist = collect_score(input, &route);
    let pure_delete_dist = delete_score(input, &route);
    assert_eq!(
        search_stats.final_total_dist,
        pure_collect_dist + pure_delete_dist
    );

    let (close_after, schedule_stats) =
        optimize_schedule(input, &route, search_stats.final_total_dist);
    let expected = score_schedule(input, &route, &close_after).unwrap();

    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;
    let mut stack = Vec::with_capacity(M);
    let mut collect_dist = 0usize;
    let mut delete_dist = 0usize;

    for i in 0..M {
        let p = selected_pos(input, route[i]);
        let moved = move_to(&mut cur, p, &mut ops);
        move_count += moved;
        collect_dist += moved;
        push_op(&mut ops, b'Z');
        stack.push(i);

        for _ in 0..close_after[i] {
            let top = stack.pop().unwrap();
            let p = other_pos(input, route[top]);
            let moved = move_to(&mut cur, p, &mut ops);
            move_count += moved;
            if i + 1 == M {
                delete_dist += moved;
            } else {
                collect_dist += moved;
            }
            push_op(&mut ops, b'Z');
        }
    }

    assert!(stack.is_empty());
    assert_eq!(collect_dist, expected.collect_dist);
    assert_eq!(delete_dist, expected.delete_dist);
    assert_eq!(move_count, schedule_stats.final_total_dist);
    assert_eq!(move_count, expected.total_dist);
    assert!(ops.len() / 2 <= MAX_T);
    assert!(move_count <= MAX_T);

    #[cfg(not(feature = "local"))]
    {
        let _ = (
            search_stats.initial_total_dist,
            search_stats.iterations,
            search_stats.accepted,
            search_stats.improved,
            search_stats.elapsed_ms,
            schedule_stats.initial_total_dist,
            schedule_stats.iterations,
            schedule_stats.accepted,
            schedule_stats.improved,
            schedule_stats.early_close_count,
            schedule_stats.max_stack_depth,
            schedule_stats.elapsed_ms,
        );
    }

    local! {
        let trace = TraceStats {
            initial_order_dist: search_stats.initial_total_dist,
            final_order_dist: search_stats.final_total_dist,
            initial_schedule_dist: schedule_stats.initial_total_dist,
            final_schedule_dist: schedule_stats.final_total_dist,
            collect_dist,
            delete_dist,
            early_close_count: schedule_stats.early_close_count,
            max_stack_depth: schedule_stats.max_stack_depth,
            move_count,
            turn_count: ops.len() / 2,
            order_iterations: search_stats.iterations,
            order_accepted: search_stats.accepted,
            order_improved: search_stats.improved,
            schedule_iterations: schedule_stats.iterations,
            schedule_accepted: schedule_stats.accepted,
            schedule_improved: schedule_stats.improved,
            order_elapsed_ms: search_stats.elapsed_ms,
            schedule_elapsed_ms: schedule_stats.elapsed_ms,
        };
        trace.summary();
    }

    ops
}

fn main() {
    let input = Input::read();
    let ops = solve(&input);
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    out.write_all(&ops).unwrap();
}
