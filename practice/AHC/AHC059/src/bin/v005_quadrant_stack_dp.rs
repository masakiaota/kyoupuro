// v005_quadrant_stack_dp.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const GROUPS: usize = 4;
const GROUP_TIME_LIMIT_SEC: f64 = 0.45;
const INF: usize = usize::MAX / 4;

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    group_order: [usize; GROUPS],
    group_sizes: [usize; GROUPS],
    group_initial_scores: [usize; GROUPS],
    group_scores: [usize; GROUPS],
    move_count: usize,
    turn_count: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    elapsed_ms: f64,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] group_order={:?} group_sizes={:?} group_initial_scores={:?} group_scores={:?} moves={} turns={} score_est={} iter={} accepted={} improved={} elapsed_ms={:.3}",
            self.group_order,
            self.group_sizes,
            self.group_initial_scores,
            self.group_scores,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count,
            self.iterations,
            self.accepted,
            self.improved,
            self.elapsed_ms,
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

#[derive(Debug, Clone, Copy, Default)]
struct SearchStats {
    initial_total_dist: usize,
    final_total_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    elapsed_ms: f64,
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
fn start_cost(input: &Input, start_pos: usize, v: usize, side: usize) -> usize {
    dist(start_pos, input.pos[v][side])
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

fn score_order(input: &Input, start_pos: usize, order: &[usize]) -> usize {
    debug_assert!(!order.is_empty());
    let first = order[0];
    let mut dp = [
        start_cost(input, start_pos, first, 0),
        start_cost(input, start_pos, first, 1),
    ];

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

fn restore_route(input: &Input, start_pos: usize, order: &[usize]) -> (Vec<RouteNode>, usize) {
    if order.is_empty() {
        return (Vec::new(), 0);
    }

    let first = order[0];
    let mut dp = [
        start_cost(input, start_pos, first, 0),
        start_cost(input, start_pos, first, 1),
    ];
    let mut parent = vec![[0usize; 2]; order.len()];

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
    let mut side = if dp[0] <= dp[1] { 0 } else { 1 };
    let score = dp[side] + pair_cost(input, last);
    let mut sides = vec![0usize; order.len()];
    sides[order.len() - 1] = side;
    for i in (1..order.len()).rev() {
        side = parent[i][side];
        sides[i - 1] = side;
    }

    let mut route = Vec::with_capacity(order.len());
    for i in 0..order.len() {
        route.push(RouteNode {
            v: order[i],
            side: sides[i],
        });
    }
    (route, score)
}

fn make_initial_order(input: &Input, start_pos: usize, labels: &[usize]) -> Vec<usize> {
    let len = labels.len();
    let mut used = vec![false; len];
    let mut order = Vec::with_capacity(len);

    let mut first_idx = usize::MAX;
    let mut first_key = (INF, INF, INF);
    for (idx, &v) in labels.iter().enumerate() {
        let start = start_cost(input, start_pos, v, 0).min(start_cost(input, start_pos, v, 1));
        let key = (start, pair_cost(input, v), v);
        if key < first_key {
            first_idx = idx;
            first_key = key;
        }
    }
    used[first_idx] = true;
    order.push(labels[first_idx]);

    while order.len() < len {
        let prev = *order.last().unwrap();
        let mut best_idx = usize::MAX;
        let mut best_key = (INF, INF);
        for (idx, &v) in labels.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let key = (transition_min_cost(input, prev, v), v);
            if key < best_key {
                best_idx = idx;
                best_key = key;
            }
        }
        used[best_idx] = true;
        order.push(labels[best_idx]);
    }

    order
}

fn improve_initial_by_reversal(
    input: &Input,
    start_pos: usize,
    order: &mut [usize],
    score: &mut usize,
    search_start: Instant,
    time_limit_sec: f64,
) {
    if order.len() < 2 {
        return;
    }

    let local_limit = (time_limit_sec * 0.08).min(0.03);
    let mut improved = true;
    while improved && search_start.elapsed().as_secs_f64() < local_limit {
        improved = false;
        for l in 0..order.len() - 1 {
            for r in l + 1..order.len() {
                order[l..=r].reverse();
                let next_score = score_order(input, start_pos, order);
                if next_score < *score {
                    *score = next_score;
                    improved = true;
                } else {
                    order[l..=r].reverse();
                }
                if search_start.elapsed().as_secs_f64() >= local_limit {
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

fn optimize_order(
    input: &Input,
    labels: &[usize],
    start_pos: usize,
    time_limit_sec: f64,
) -> (Vec<RouteNode>, SearchStats) {
    if labels.is_empty() {
        return (Vec::new(), SearchStats::default());
    }

    let search_start = Instant::now();
    let mut order = make_initial_order(input, start_pos, labels);
    let mut current_score = score_order(input, start_pos, &order);
    improve_initial_by_reversal(
        input,
        start_pos,
        &mut order,
        &mut current_score,
        search_start,
        time_limit_sec,
    );

    let initial_score = current_score;
    let mut best_score = current_score;
    let mut best_order = order.clone();
    let mut rng =
        Rng::new(0x243f_6a88_85a3_08d3 ^ initial_score as u64 ^ ((labels.len() as u64) << 32));

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut temp = 8.0;

    while order.len() >= 2 {
        iterations += 1;
        if (iterations & 255) == 0 {
            let elapsed = search_start.elapsed().as_secs_f64();
            if elapsed >= time_limit_sec {
                break;
            }
            let progress = (elapsed / time_limit_sec).clamp(0.0, 1.0);
            temp = 8.0_f64.powf(1.0 - progress) * 0.05_f64.powf(progress);
        }

        let move_type = rng.gen_usize(100);
        let old_score = current_score;
        if move_type < 45 {
            let mut l = rng.gen_usize(order.len());
            let mut r = rng.gen_usize(order.len());
            if l == r {
                continue;
            }
            if l > r {
                std::mem::swap(&mut l, &mut r);
            }
            order[l..=r].reverse();
            let next_score = score_order(input, start_pos, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
            } else {
                order[l..=r].reverse();
                continue;
            }
        } else if move_type < 75 {
            let mut i = rng.gen_usize(order.len());
            let mut j = rng.gen_usize(order.len());
            if i == j {
                continue;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            order.swap(i, j);
            let next_score = score_order(input, start_pos, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
            } else {
                order.swap(i, j);
                continue;
            }
        } else {
            let i = rng.gen_usize(order.len());
            let j = rng.gen_usize(order.len());
            if i == j {
                continue;
            }
            apply_relocate(&mut order, i, j);
            let next_score = score_order(input, start_pos, &order);
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

    let elapsed = search_start.elapsed().as_secs_f64();
    let (route, restored_score) = restore_route(input, start_pos, &best_order);
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

#[inline(always)]
fn quadrant_of_pair(input: &Input, v: usize) -> usize {
    let p0 = input.pos[v][0];
    let p1 = input.pos[v][1];
    let i_sum = p0 / N + p1 / N;
    let j_sum = p0 % N + p1 % N;
    let qi = if i_sum < N { 0 } else { 1 };
    let qj = if j_sum < N { 0 } else { 1 };
    qi * 2 + qj
}

fn estimate_group_order(
    input: &Input,
    groups: &[Vec<usize>; GROUPS],
    order: [usize; GROUPS],
) -> usize {
    let mut cur = 0usize;
    let mut total = 0usize;
    for &g in &order {
        if groups[g].is_empty() {
            continue;
        }
        let card_order = make_initial_order(input, cur, &groups[g]);
        let (route, score) = restore_route(input, cur, &card_order);
        total += score;
        if let Some(&first) = route.first() {
            cur = other_pos(input, first);
        }
    }
    total
}

fn choose_group_order(input: &Input, groups: &[Vec<usize>; GROUPS]) -> [usize; GROUPS] {
    let mut best_order = [0usize, 1, 3, 2];
    let mut best_score = estimate_group_order(input, groups, best_order);

    for a in 0..GROUPS {
        for b in 0..GROUPS {
            if b == a {
                continue;
            }
            for c in 0..GROUPS {
                if c == a || c == b {
                    continue;
                }
                for d in 0..GROUPS {
                    if d == a || d == b || d == c {
                        continue;
                    }
                    let order = [a, b, c, d];
                    let score = estimate_group_order(input, groups, order);
                    if (score, order) < (best_score, best_order) {
                        best_score = score;
                        best_order = order;
                    }
                }
            }
        }
    }

    best_order
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
    let mut groups: [Vec<usize>; GROUPS] = std::array::from_fn(|_| Vec::new());
    for v in 0..M {
        groups[quadrant_of_pair(input, v)].push(v);
    }

    let group_order = choose_group_order(input, &groups);
    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;

    let mut group_sizes = [0usize; GROUPS];
    let mut group_initial_scores = [0usize; GROUPS];
    let mut group_scores = [0usize; GROUPS];
    let mut total_iterations = 0usize;
    let mut total_accepted = 0usize;
    let mut total_improved = 0usize;
    let mut total_elapsed_ms = 0.0;

    for &g in &group_order {
        group_sizes[g] = groups[g].len();
        let (route, stats) = optimize_order(input, &groups[g], cur, GROUP_TIME_LIMIT_SEC);
        let before_move_count = move_count;

        for &node in &route {
            let p = selected_pos(input, node);
            move_count += move_to(&mut cur, p, &mut ops);
            push_op(&mut ops, b'Z');
        }
        for &node in route.iter().rev() {
            let p = other_pos(input, node);
            move_count += move_to(&mut cur, p, &mut ops);
            push_op(&mut ops, b'Z');
        }

        let block_score = move_count - before_move_count;
        assert_eq!(block_score, stats.final_total_dist);
        group_initial_scores[g] = stats.initial_total_dist;
        group_scores[g] = block_score;
        total_iterations += stats.iterations;
        total_accepted += stats.accepted;
        total_improved += stats.improved;
        total_elapsed_ms += stats.elapsed_ms;
    }

    assert_eq!(group_sizes.iter().sum::<usize>(), M);
    assert!(ops.len() / 2 <= MAX_T);
    assert!(move_count <= MAX_T);

    local! {
        let trace = TraceStats {
            group_order,
            group_sizes,
            group_initial_scores,
            group_scores,
            move_count,
            turn_count: ops.len() / 2,
            iterations: total_iterations,
            accepted: total_accepted,
            improved: total_improved,
            elapsed_ms: total_elapsed_ms,
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
