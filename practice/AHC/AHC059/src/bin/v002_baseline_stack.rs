// v002_baseline_stack.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const TSP_TIME_LIMIT_SEC: f64 = 1.80;

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    initial_collect_dist: usize,
    final_collect_dist: usize,
    delete_dist: usize,
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
            "[summary] collect={} -> {} delete={} moves={} turns={} score_est={} iter={} accepted={} improved={} elapsed_ms={:.3}",
            self.initial_collect_dist,
            self.final_collect_dist,
            self.delete_dist,
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

#[derive(Debug, Clone, Copy)]
struct SearchStats {
    initial_collect_dist: usize,
    final_collect_dist: usize,
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
fn selected_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side]
}

#[inline(always)]
fn other_pos(input: &Input, node: RouteNode) -> usize {
    input.pos[node.v][node.side ^ 1]
}

#[inline(always)]
fn edge_cost(input: &Input, route: &[RouteNode], k: usize) -> usize {
    let to = selected_pos(input, route[k]);
    let from = if k == 0 {
        0
    } else {
        selected_pos(input, route[k - 1])
    };
    dist(from, to)
}

#[inline(always)]
fn edge_cost_after_swap(input: &Input, route: &[RouteNode], k: usize, i: usize, j: usize) -> usize {
    let node_at = |idx: usize| {
        if idx == i {
            route[j]
        } else if idx == j {
            route[i]
        } else {
            route[idx]
        }
    };
    let to = selected_pos(input, node_at(k));
    let from = if k == 0 {
        0
    } else {
        selected_pos(input, node_at(k - 1))
    };
    dist(from, to)
}

fn collect_score(input: &Input, route: &[RouteNode]) -> usize {
    let mut score = 0usize;
    for k in 0..route.len() {
        score += edge_cost(input, route, k);
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

fn make_initial_route(input: &Input) -> Vec<RouteNode> {
    let mut used = [false; M];
    let mut route = Vec::with_capacity(M);
    let mut cur = 0usize;

    for _ in 0..M {
        let mut best = RouteNode {
            v: usize::MAX,
            side: usize::MAX,
        };
        let mut best_dist = usize::MAX;
        let mut best_pos = usize::MAX;

        for v in 0..M {
            if used[v] {
                continue;
            }
            for side in 0..2 {
                let p = input.pos[v][side];
                let d = dist(cur, p);
                if (d, p, v, side) < (best_dist, best_pos, best.v, best.side) {
                    best = RouteNode { v, side };
                    best_dist = d;
                    best_pos = p;
                }
            }
        }

        used[best.v] = true;
        cur = best_pos;
        route.push(best);
    }

    route
}

fn flip_delta(input: &Input, route: &[RouteNode], i: usize) -> i32 {
    let old = route[i];
    let new = RouteNode {
        v: old.v,
        side: old.side ^ 1,
    };
    let old_p = selected_pos(input, old);
    let new_p = selected_pos(input, new);
    let prev = if i == 0 {
        0
    } else {
        selected_pos(input, route[i - 1])
    };

    let mut before = dist(prev, old_p);
    let mut after = dist(prev, new_p);
    if i + 1 < route.len() {
        let next = selected_pos(input, route[i + 1]);
        before += dist(old_p, next);
        after += dist(new_p, next);
    }

    after as i32 - before as i32
}

fn two_opt_delta(input: &Input, route: &[RouteNode], l: usize, r: usize) -> i32 {
    debug_assert!(l < r);
    let prev = if l == 0 {
        0
    } else {
        selected_pos(input, route[l - 1])
    };
    let left = selected_pos(input, route[l]);
    let right = selected_pos(input, route[r]);

    let mut before = dist(prev, left);
    let mut after = dist(prev, right);
    if r + 1 < route.len() {
        let next = selected_pos(input, route[r + 1]);
        before += dist(right, next);
        after += dist(left, next);
    }

    after as i32 - before as i32
}

fn swap_delta(input: &Input, route: &[RouteNode], i: usize, j: usize) -> i32 {
    debug_assert!(i < j);
    let mut ks = [usize::MAX; 4];
    let mut len = 0usize;
    for k in [i, i + 1, j, j + 1] {
        if k < route.len() && !ks[..len].contains(&k) {
            ks[len] = k;
            len += 1;
        }
    }

    let mut before = 0usize;
    let mut after = 0usize;
    for &k in &ks[..len] {
        before += edge_cost(input, route, k);
        after += edge_cost_after_swap(input, route, k, i, j);
    }
    after as i32 - before as i32
}

fn accept(delta: i32, temp: f64, rng: &mut Rng) -> bool {
    delta <= 0 || rng.gen_f64() < (-(delta as f64) / temp).exp()
}

fn optimize_collect_route(input: &Input) -> (Vec<RouteNode>, SearchStats) {
    let start = Instant::now();
    let mut route = make_initial_route(input);
    let initial_score = collect_score(input, &route);
    let mut current_score = initial_score;
    let mut best_score = initial_score;
    let mut best_route = route.clone();
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15 ^ initial_score as u64);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut temp = 2.0;

    loop {
        iterations += 1;
        if (iterations & 4095) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= TSP_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / TSP_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 2.0_f64.powf(1.0 - progress) * 0.02_f64.powf(progress);
        }

        let move_type = rng.gen_usize(100);
        let delta;
        if move_type < 25 {
            let i = rng.gen_usize(M);
            delta = flip_delta(input, &route, i);
            if accept(delta, temp, &mut rng) {
                route[i].side ^= 1;
                current_score = (current_score as i32 + delta) as usize;
                accepted += 1;
            } else {
                continue;
            }
        } else if move_type < 70 {
            let mut l = rng.gen_usize(M);
            let mut r = rng.gen_usize(M);
            if l == r {
                continue;
            }
            if l > r {
                std::mem::swap(&mut l, &mut r);
            }
            delta = two_opt_delta(input, &route, l, r);
            if accept(delta, temp, &mut rng) {
                route[l..=r].reverse();
                current_score = (current_score as i32 + delta) as usize;
                accepted += 1;
            } else {
                continue;
            }
        } else {
            let mut i = rng.gen_usize(M);
            let mut j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            delta = swap_delta(input, &route, i, j);
            if accept(delta, temp, &mut rng) {
                route.swap(i, j);
                current_score = (current_score as i32 + delta) as usize;
                accepted += 1;
            } else {
                continue;
            }
        }

        if current_score < best_score {
            best_score = current_score;
            best_route.clone_from(&route);
            improved += 1;
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    (
        best_route,
        SearchStats {
            initial_collect_dist: initial_score,
            final_collect_dist: best_score,
            iterations,
            accepted,
            improved,
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
    let (route, search_stats) = optimize_collect_route(input);
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
    assert!(ops.len() / 2 <= MAX_T);
    assert!(move_count <= MAX_T);

    #[cfg(not(feature = "local"))]
    {
        let _ = (
            search_stats.initial_collect_dist,
            search_stats.final_collect_dist,
            search_stats.iterations,
            search_stats.accepted,
            search_stats.improved,
            search_stats.elapsed_ms,
        );
    }

    local! {
        let trace = TraceStats {
            initial_collect_dist: search_stats.initial_collect_dist,
            final_collect_dist: search_stats.final_collect_dist,
            delete_dist,
            move_count,
            turn_count: ops.len() / 2,
            iterations: search_stats.iterations,
            accepted: search_stats.accepted,
            improved: search_stats.improved,
            elapsed_ms: search_stats.elapsed_ms,
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
