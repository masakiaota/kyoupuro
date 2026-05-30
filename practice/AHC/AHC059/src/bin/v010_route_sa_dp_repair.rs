// v010_route_sa_dp_repair.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const SEARCH_TIME_LIMIT_SEC: f64 = 1.80;
const DP_REPAIR_INTERVAL: usize = 16_384;
const INF: usize = usize::MAX / 4;

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    initial_dist: usize,
    final_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    move_count: usize,
    turn_count: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    side_flip_count: usize,
    two_opt_count: usize,
    swap_count: usize,
    relocate_count: usize,
    dp_repair_count: usize,
    dp_repair_gain: usize,
    elapsed_ms: f64,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] route={} -> {} collect={} delete={} moves={} turns={} score_est={} iter={} accepted={} improved={} side_flip={} two_opt={} swap={} relocate={} dp_repair={} dp_gain={} elapsed_ms={:.3}",
            self.initial_dist,
            self.final_dist,
            self.collect_dist,
            self.delete_dist,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count,
            self.iterations,
            self.accepted,
            self.improved,
            self.side_flip_count,
            self.two_opt_count,
            self.swap_count,
            self.relocate_count,
            self.dp_repair_count,
            self.dp_repair_gain,
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

#[derive(Debug, Clone, Copy)]
struct SearchStats {
    initial_dist: usize,
    final_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    side_flip_count: usize,
    two_opt_count: usize,
    swap_count: usize,
    relocate_count: usize,
    dp_repair_count: usize,
    dp_repair_gain: usize,
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
fn selected_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side]
}

#[inline(always)]
fn other_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side ^ 1]
}

#[inline(always)]
fn start_cost(input: &Input, node: RouteNode) -> usize {
    dist(0, selected_pos(input, node))
}

#[inline(always)]
fn pair_cost(input: &Input, node: RouteNode) -> usize {
    dist(input.pos[node.v][0], input.pos[node.v][1])
}

#[inline(always)]
fn link_cost(input: &Input, a: RouteNode, b: RouteNode) -> usize {
    dist(selected_pos(input, a), selected_pos(input, b))
        + dist(other_pos(input, a), other_pos(input, b))
}

#[inline(always)]
fn edge_cost(input: &Input, prev: Option<RouteNode>, node: RouteNode) -> usize {
    if let Some(prev) = prev {
        link_cost(input, prev, node)
    } else {
        start_cost(input, node)
    }
}

#[inline(always)]
fn tail_cost(input: &Input, node: RouteNode, next: Option<RouteNode>) -> usize {
    if let Some(next) = next {
        link_cost(input, node, next)
    } else {
        pair_cost(input, node)
    }
}

#[inline(always)]
fn component_cost(input: &Input, route: &[RouteNode], k: usize) -> usize {
    if k == 0 {
        start_cost(input, route[0])
    } else if k < route.len() {
        link_cost(input, route[k - 1], route[k])
    } else {
        pair_cost(input, route[route.len() - 1])
    }
}

fn route_score(input: &Input, route: &[RouteNode]) -> usize {
    let mut score = 0usize;
    for k in 0..=route.len() {
        score += component_cost(input, route, k);
    }
    score
}

#[inline(always)]
fn add_unique(buf: &mut [usize; 8], len: &mut usize, k: usize) {
    if !buf[..*len].contains(&k) {
        buf[*len] = k;
        *len += 1;
    }
}

fn affected_cost(input: &Input, route: &[RouteNode], ks: &[usize]) -> usize {
    let mut score = 0usize;
    for &k in ks {
        score += component_cost(input, route, k);
    }
    score
}

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

fn make_initial_order(input: &Input) -> Vec<usize> {
    let mut used = [false; M];
    let mut order = Vec::with_capacity(M);

    let mut first = usize::MAX;
    let mut first_key = (INF, INF, INF);
    for v in 0..M {
        let start = dist(0, input.pos[v][0]).min(dist(0, input.pos[v][1]));
        let key = (start, dist(input.pos[v][0], input.pos[v][1]), v);
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

fn restore_route_by_dp(input: &Input, order: &[usize]) -> Vec<RouteNode> {
    let first = order[0];
    let mut dp = [dist(0, input.pos[first][0]), dist(0, input.pos[first][1])];
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

    let mut side = if dp[0] <= dp[1] { 0 } else { 1 };
    let mut sides = [0usize; M];
    sides[M - 1] = side;
    for i in (1..M).rev() {
        side = parent[i][side];
        sides[i - 1] = side;
    }

    let mut route = Vec::with_capacity(M);
    for i in 0..M {
        route.push(RouteNode {
            v: order[i],
            side: sides[i],
        });
    }
    route
}

fn repair_route_sides_by_dp(input: &Input, route: &mut [RouteNode]) -> usize {
    let first = route[0].v;
    let mut dp = [dist(0, input.pos[first][0]), dist(0, input.pos[first][1])];
    let mut parent = [[0usize; 2]; M];

    for i in 1..route.len() {
        let prev_v = route[i - 1].v;
        let v = route[i].v;
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

    let mut side = if dp[0] <= dp[1] { 0 } else { 1 };
    let score = dp[side] + pair_cost(input, route[M - 1]);
    route[M - 1].side = side;
    for i in (1..M).rev() {
        side = parent[i][side];
        route[i - 1].side = side;
    }

    score
}

fn accept(delta: i32, temp: f64, rng: &mut Rng) -> bool {
    delta <= 0 || rng.gen_f64() < (-(delta as f64) / temp).exp()
}

fn apply_relocate(route: &mut [RouteNode], i: usize, j: usize) {
    if i < j {
        route[i..=j].rotate_left(1);
    } else {
        route[j..=i].rotate_right(1);
    }
}

fn flip_delta(input: &Input, route: &mut [RouteNode], i: usize) -> i32 {
    let mut ks = [0usize; 8];
    let mut len = 0usize;
    add_unique(&mut ks, &mut len, i);
    add_unique(&mut ks, &mut len, i + 1);

    let before = affected_cost(input, route, &ks[..len]);
    route[i].side ^= 1;
    let after = affected_cost(input, route, &ks[..len]);
    route[i].side ^= 1;
    after as i32 - before as i32
}

fn two_opt_delta(input: &Input, route: &mut [RouteNode], l: usize, r: usize) -> i32 {
    let mut ks = [0usize; 8];
    let mut len = 0usize;
    add_unique(&mut ks, &mut len, l);
    add_unique(&mut ks, &mut len, r + 1);

    let before = affected_cost(input, route, &ks[..len]);
    route[l..=r].reverse();
    let after = affected_cost(input, route, &ks[..len]);
    route[l..=r].reverse();
    after as i32 - before as i32
}

fn swap_delta(input: &Input, route: &mut [RouteNode], i: usize, j: usize) -> i32 {
    let mut ks = [0usize; 8];
    let mut len = 0usize;
    add_unique(&mut ks, &mut len, i);
    add_unique(&mut ks, &mut len, i + 1);
    add_unique(&mut ks, &mut len, j);
    add_unique(&mut ks, &mut len, j + 1);

    let before = affected_cost(input, route, &ks[..len]);
    route.swap(i, j);
    let after = affected_cost(input, route, &ks[..len]);
    route.swap(i, j);
    after as i32 - before as i32
}

fn relocate_delta(input: &Input, route: &[RouteNode], i: usize, j: usize) -> i32 {
    let x = route[i];
    if i < j {
        let prev_i = if i == 0 { None } else { Some(route[i - 1]) };
        let next_i = route[i + 1];
        let node_j = route[j];
        let after_j = if j + 1 < route.len() {
            Some(route[j + 1])
        } else {
            None
        };

        let before = edge_cost(input, prev_i, x)
            + edge_cost(input, Some(x), next_i)
            + tail_cost(input, node_j, after_j);
        let after = edge_cost(input, prev_i, next_i)
            + edge_cost(input, Some(node_j), x)
            + tail_cost(input, x, after_j);
        after as i32 - before as i32
    } else {
        let prev_j = if j == 0 { None } else { Some(route[j - 1]) };
        let node_j = route[j];
        let prev_i = route[i - 1];
        let after_i = if i + 1 < route.len() {
            Some(route[i + 1])
        } else {
            None
        };

        let before = edge_cost(input, prev_j, node_j)
            + edge_cost(input, Some(prev_i), x)
            + tail_cost(input, x, after_i);
        let after = edge_cost(input, prev_j, x)
            + edge_cost(input, Some(x), node_j)
            + tail_cost(input, prev_i, after_i);
        after as i32 - before as i32
    }
}

fn optimize_route(input: &Input) -> (Vec<RouteNode>, SearchStats) {
    let start = Instant::now();
    let order = make_initial_order(input);
    let mut route = restore_route_by_dp(input, &order);
    let initial_score = route_score(input, &route);
    let mut current_score = initial_score;
    let mut best_score = initial_score;
    let mut best_route = route.clone();
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb ^ initial_score as u64);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut side_flip_count = 0usize;
    let mut two_opt_count = 0usize;
    let mut swap_count = 0usize;
    let mut relocate_count = 0usize;
    let mut dp_repair_count = 0usize;
    let mut dp_repair_gain = 0usize;
    let mut temp = 1.0;

    loop {
        iterations += 1;
        if (iterations & 4095) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= SEARCH_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / SEARCH_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 1.0_f64.powf(1.0 - progress) * 0.002_f64.powf(progress);
        }
        if (iterations & (DP_REPAIR_INTERVAL - 1)) == 0 {
            let before = current_score;
            let repaired_score = repair_route_sides_by_dp(input, &mut route);
            debug_assert!(repaired_score <= before);
            current_score = repaired_score;
            dp_repair_count += 1;
            dp_repair_gain += before - repaired_score;
            if current_score < best_score {
                best_score = current_score;
                best_route.clone_from(&route);
                improved += 1;
            }
        }

        let move_type = rng.gen_usize(100);
        let delta;
        if move_type < 3 {
            let i = rng.gen_usize(M);
            delta = flip_delta(input, &mut route, i);
            if accept(delta, temp, &mut rng) {
                route[i].side ^= 1;
                side_flip_count += 1;
            } else {
                continue;
            }
        } else if move_type < 63 {
            let mut l = rng.gen_usize(M);
            let mut r = rng.gen_usize(M);
            if l == r {
                continue;
            }
            if l > r {
                std::mem::swap(&mut l, &mut r);
            }
            delta = two_opt_delta(input, &mut route, l, r);
            if accept(delta, temp, &mut rng) {
                route[l..=r].reverse();
                two_opt_count += 1;
            } else {
                continue;
            }
        } else if move_type < 82 {
            let mut i = rng.gen_usize(M);
            let mut j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            delta = swap_delta(input, &mut route, i, j);
            if accept(delta, temp, &mut rng) {
                route.swap(i, j);
                swap_count += 1;
            } else {
                continue;
            }
        } else {
            let i = rng.gen_usize(M);
            let j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            delta = relocate_delta(input, &route, i, j);
            if accept(delta, temp, &mut rng) {
                apply_relocate(&mut route, i, j);
                relocate_count += 1;
            } else {
                continue;
            }
        }

        current_score = (current_score as i32 + delta) as usize;
        accepted += 1;
        if current_score < best_score {
            best_score = current_score;
            best_route.clone_from(&route);
            improved += 1;
        }

        #[cfg(feature = "local")]
        if (iterations & ((1 << 20) - 1)) == 0 {
            debug_assert_eq!(current_score, route_score(input, &route));
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    assert_eq!(best_score, route_score(input, &best_route));
    (
        best_route,
        SearchStats {
            initial_dist: initial_score,
            final_dist: best_score,
            iterations,
            accepted,
            improved,
            side_flip_count,
            two_opt_count,
            swap_count,
            relocate_count,
            dp_repair_count,
            dp_repair_gain,
            elapsed_ms: elapsed * 1000.0,
        },
    )
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
    let (route, stats) = optimize_route(input);
    let expected_collect_dist = collect_score(input, &route);
    let expected_delete_dist = delete_score(input, &route);

    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;

    for &node in &route {
        let p = selected_pos(input, node);
        move_count += move_to(&mut cur, p, &mut ops);
        push_op(&mut ops, b'Z');
    }

    let mut delete_dist = 0usize;
    for &node in route.iter().rev() {
        let p = other_pos(input, node);
        let moved = move_to(&mut cur, p, &mut ops);
        move_count += moved;
        delete_dist += moved;
        push_op(&mut ops, b'Z');
    }

    assert_eq!(delete_dist, expected_delete_dist);
    assert_eq!(move_count, expected_collect_dist + expected_delete_dist);
    assert_eq!(move_count, stats.final_dist);
    assert!(ops.len() / 2 <= MAX_T);
    assert!(move_count <= MAX_T);

    #[cfg(not(feature = "local"))]
    {
        let _ = (
            stats.initial_dist,
            stats.iterations,
            stats.accepted,
            stats.improved,
            stats.side_flip_count,
            stats.two_opt_count,
            stats.swap_count,
            stats.relocate_count,
            stats.dp_repair_count,
            stats.dp_repair_gain,
            stats.elapsed_ms,
        );
    }

    local! {
        let trace = TraceStats {
            initial_dist: stats.initial_dist,
            final_dist: stats.final_dist,
            collect_dist: expected_collect_dist,
            delete_dist,
            move_count,
            turn_count: ops.len() / 2,
            iterations: stats.iterations,
            accepted: stats.accepted,
            improved: stats.improved,
            side_flip_count: stats.side_flip_count,
            two_opt_count: stats.two_opt_count,
            swap_count: stats.swap_count,
            relocate_count: stats.relocate_count,
            dp_repair_count: stats.dp_repair_count,
            dp_repair_gain: stats.dp_repair_gain,
            elapsed_ms: stats.elapsed_ms,
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
