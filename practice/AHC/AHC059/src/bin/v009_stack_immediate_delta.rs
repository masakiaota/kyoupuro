// v009_stack_immediate_delta.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const ORDER_SEARCH_TIME_LIMIT_SEC: f64 = 1.55;
const IMMEDIATE_SEARCH_TIME_LIMIT_SEC: f64 = 0.25;
const INF: usize = usize::MAX / 4;
const BIT_WORDS: usize = M.div_ceil(64);

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    initial_order_dist: usize,
    final_order_dist: usize,
    initial_immediate_dist: usize,
    final_immediate_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    immediate_count: usize,
    move_count: usize,
    turn_count: usize,
    order_iterations: usize,
    order_accepted: usize,
    order_improved: usize,
    immediate_iterations: usize,
    immediate_accepted: usize,
    immediate_improved: usize,
    order_elapsed_ms: f64,
    immediate_elapsed_ms: f64,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] order={} -> {} immediate={} -> {} collect={} delete={} immediate_count={} moves={} turns={} score_est={} order_iter={} order_accepted={} order_improved={} immediate_iter={} immediate_accepted={} immediate_improved={} order_elapsed_ms={:.3} immediate_elapsed_ms={:.3}",
            self.initial_order_dist,
            self.final_order_dist,
            self.initial_immediate_dist,
            self.final_immediate_dist,
            self.collect_dist,
            self.delete_dist,
            self.immediate_count,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count,
            self.order_iterations,
            self.order_accepted,
            self.order_improved,
            self.immediate_iterations,
            self.immediate_accepted,
            self.immediate_improved,
            self.order_elapsed_ms,
            self.immediate_elapsed_ms,
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
struct ImmediateStats {
    initial_total_dist: usize,
    final_total_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    immediate_count: usize,
    elapsed_ms: f64,
}

#[derive(Debug, Clone, Copy)]
struct ImmediateBreakdown {
    total_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    immediate_count: usize,
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

fn score_immediate(
    input: &Input,
    route: &[RouteNode],
    immediate: &[bool; M],
) -> ImmediateBreakdown {
    let mut cur = 0usize;
    let mut collect_dist = 0usize;
    let mut immediate_count = 0usize;

    for i in 0..M {
        let node = route[i];
        let selected = selected_pos(input, node);
        collect_dist += dist(cur, selected);
        cur = selected;

        if immediate[i] {
            let other = other_pos(input, node);
            collect_dist += dist(cur, other);
            cur = other;
            immediate_count += 1;
        }
    }

    let mut delete_dist = 0usize;
    for i in (0..M).rev() {
        if immediate[i] {
            continue;
        }
        let other = other_pos(input, route[i]);
        delete_dist += dist(cur, other);
        cur = other;
    }

    ImmediateBreakdown {
        total_dist: collect_dist + delete_dist,
        collect_dist,
        delete_dist,
        immediate_count,
    }
}

#[inline(always)]
fn end_collection_pos(input: &Input, route: &[RouteNode], immediate: &[bool; M]) -> usize {
    if immediate[M - 1] {
        other_pos(input, route[M - 1])
    } else {
        selected_pos(input, route[M - 1])
    }
}

#[inline(always)]
fn end_collection_pos_after_toggle(
    input: &Input,
    route: &[RouteNode],
    immediate: &[bool; M],
    toggle_idx: usize,
) -> usize {
    if toggle_idx == M - 1 {
        if immediate[M - 1] {
            selected_pos(input, route[M - 1])
        } else {
            other_pos(input, route[M - 1])
        }
    } else {
        end_collection_pos(input, route, immediate)
    }
}

fn collection_delta_toggle(
    input: &Input,
    route: &[RouteNode],
    immediate: &[bool; M],
    i: usize,
) -> i32 {
    let s_i = selected_pos(input, route[i]);
    let o_i = other_pos(input, route[i]);
    let old = if i + 1 < M {
        let s_next = selected_pos(input, route[i + 1]);
        if immediate[i] {
            dist(s_i, o_i) + dist(o_i, s_next)
        } else {
            dist(s_i, s_next)
        }
    } else if immediate[i] {
        dist(s_i, o_i)
    } else {
        0
    };
    let new = if i + 1 < M {
        let s_next = selected_pos(input, route[i + 1]);
        if immediate[i] {
            dist(s_i, s_next)
        } else {
            dist(s_i, o_i) + dist(o_i, s_next)
        }
    } else if immediate[i] {
        0
    } else {
        dist(s_i, o_i)
    };
    new as i32 - old as i32
}

fn add_node(nodes: &mut [usize; 4], len: &mut usize, node: Option<usize>) {
    if let Some(x) = node {
        if !nodes[..*len].contains(&x) {
            nodes[*len] = x;
            *len += 1;
        }
    }
}

type NonImmediateBits = [u64; BIT_WORDS];

fn initial_non_immediate_bits() -> NonImmediateBits {
    let mut bits = [0u64; BIT_WORDS];
    for i in 0..M {
        bits[i >> 6] |= 1u64 << (i & 63);
    }
    bits
}

#[inline(always)]
fn bit_contains(bits: &NonImmediateBits, i: usize) -> bool {
    ((bits[i >> 6] >> (i & 63)) & 1) != 0
}

#[inline(always)]
fn bit_insert(bits: &mut NonImmediateBits, i: usize) {
    bits[i >> 6] |= 1u64 << (i & 63);
}

#[inline(always)]
fn bit_remove(bits: &mut NonImmediateBits, i: usize) {
    bits[i >> 6] &= !(1u64 << (i & 63));
}

fn bit_prev_lower(bits: &NonImmediateBits, i: usize) -> Option<usize> {
    if i == 0 {
        return None;
    }
    let word = i >> 6;
    let bit = i & 63;
    let mut mask = if bit == 0 {
        0
    } else {
        bits[word] & ((1u64 << bit) - 1)
    };
    if mask != 0 {
        return Some((word << 6) + (63 - mask.leading_zeros() as usize));
    }
    for w in (0..word).rev() {
        mask = bits[w];
        if mask != 0 {
            return Some((w << 6) + (63 - mask.leading_zeros() as usize));
        }
    }
    None
}

fn bit_next_higher(bits: &NonImmediateBits, i: usize) -> Option<usize> {
    if i + 1 >= M {
        return None;
    }
    let word = i >> 6;
    let bit = i & 63;
    let mask = if bit == 63 {
        0
    } else {
        bits[word] & (!0u64 << (bit + 1))
    };
    if mask != 0 {
        return Some((word << 6) + mask.trailing_zeros() as usize);
    }
    for (w, &word_bits) in bits.iter().enumerate().skip(word + 1) {
        if word_bits != 0 {
            let idx = (w << 6) + word_bits.trailing_zeros() as usize;
            if idx < M {
                return Some(idx);
            }
        }
    }
    None
}

fn bit_head(bits: &NonImmediateBits) -> Option<usize> {
    bit_prev_lower(bits, M)
}

fn delete_edge_cost(
    input: &Input,
    route: &[RouteNode],
    non_immediate: &NonImmediateBits,
    start_pos: usize,
    i: usize,
) -> usize {
    debug_assert!(bit_contains(non_immediate, i));
    let prev_pos = if let Some(higher) = bit_next_higher(non_immediate, i) {
        other_pos(input, route[higher])
    } else {
        start_pos
    };
    dist(prev_pos, other_pos(input, route[i]))
}

fn delete_nodes_cost(
    input: &Input,
    route: &[RouteNode],
    non_immediate: &NonImmediateBits,
    start_pos: usize,
    nodes: &[usize],
) -> usize {
    let mut total = 0usize;
    for &i in nodes {
        if bit_contains(non_immediate, i) {
            total += delete_edge_cost(input, route, non_immediate, start_pos, i);
        }
    }
    total
}

fn delete_delta_toggle(
    input: &Input,
    route: &[RouteNode],
    immediate: &[bool; M],
    non_immediate: &NonImmediateBits,
    i: usize,
) -> i32 {
    let old_start = end_collection_pos(input, route, immediate);
    let mut old_nodes = [0usize; 4];
    let mut old_len = 0usize;
    add_node(&mut old_nodes, &mut old_len, Some(i));
    add_node(
        &mut old_nodes,
        &mut old_len,
        bit_prev_lower(non_immediate, i),
    );
    add_node(&mut old_nodes, &mut old_len, bit_head(non_immediate));
    let old = delete_nodes_cost(
        input,
        route,
        non_immediate,
        old_start,
        &old_nodes[..old_len],
    );

    let mut next_bits = *non_immediate;
    if immediate[i] {
        bit_insert(&mut next_bits, i);
    } else {
        bit_remove(&mut next_bits, i);
    }

    let new_start = end_collection_pos_after_toggle(input, route, immediate, i);
    let mut new_nodes = [0usize; 4];
    let mut new_len = 0usize;
    add_node(&mut new_nodes, &mut new_len, Some(i));
    add_node(&mut new_nodes, &mut new_len, bit_prev_lower(&next_bits, i));
    add_node(&mut new_nodes, &mut new_len, bit_head(&next_bits));
    let new = delete_nodes_cost(input, route, &next_bits, new_start, &new_nodes[..new_len]);

    new as i32 - old as i32
}

fn apply_immediate_toggle(
    immediate: &mut [bool; M],
    non_immediate: &mut NonImmediateBits,
    i: usize,
) {
    immediate[i] = !immediate[i];
    if immediate[i] {
        bit_remove(non_immediate, i);
    } else {
        bit_insert(non_immediate, i);
    }
}

fn optimize_immediate(
    input: &Input,
    route: &[RouteNode],
    pure_stack_score: usize,
) -> ([bool; M], ImmediateStats) {
    let start = Instant::now();
    let mut immediate = [false; M];
    let mut non_immediate = initial_non_immediate_bits();
    let initial = score_immediate(input, route, &immediate);
    assert_eq!(initial.total_dist, pure_stack_score);

    let mut current_score = initial.total_dist;
    let mut best_score = current_score;
    let mut best_immediate = immediate;
    let mut rng = Rng::new(0xa076_1d64_78bd_642f ^ pure_stack_score as u64);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;

    let mut changed = true;
    while changed && start.elapsed().as_secs_f64() < IMMEDIATE_SEARCH_TIME_LIMIT_SEC * 0.25 {
        changed = false;
        for i in 0..M {
            iterations += 1;
            let delta = collection_delta_toggle(input, route, &immediate, i)
                + delete_delta_toggle(input, route, &immediate, &non_immediate, i);
            if delta < 0 {
                apply_immediate_toggle(&mut immediate, &mut non_immediate, i);
                current_score = (current_score as i32 + delta) as usize;
                accepted += 1;
                changed = true;
                if current_score < best_score {
                    best_score = current_score;
                    best_immediate = immediate;
                    improved += 1;
                }
            }
            if start.elapsed().as_secs_f64() >= IMMEDIATE_SEARCH_TIME_LIMIT_SEC * 0.25 {
                break;
            }
        }
    }

    let mut temp = 4.0;
    loop {
        iterations += 1;
        if (iterations & 255) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= IMMEDIATE_SEARCH_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / IMMEDIATE_SEARCH_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 4.0_f64.powf(1.0 - progress) * 0.03_f64.powf(progress);
        }

        let i = rng.gen_usize(M);
        let delta = collection_delta_toggle(input, route, &immediate, i)
            + delete_delta_toggle(input, route, &immediate, &non_immediate, i);
        if accept(delta, temp, &mut rng) {
            apply_immediate_toggle(&mut immediate, &mut non_immediate, i);
            current_score = (current_score as i32 + delta) as usize;
            accepted += 1;
            if current_score < best_score {
                best_score = current_score;
                best_immediate = immediate;
                improved += 1;
            }
        }

        #[cfg(feature = "local")]
        if (iterations & ((1 << 18) - 1)) == 0 {
            debug_assert_eq!(
                current_score,
                score_immediate(input, route, &immediate).total_dist
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let best_breakdown = score_immediate(input, route, &best_immediate);
    assert_eq!(best_score, best_breakdown.total_dist);
    (
        best_immediate,
        ImmediateStats {
            initial_total_dist: initial.total_dist,
            final_total_dist: best_score,
            iterations,
            accepted,
            improved,
            immediate_count: best_breakdown.immediate_count,
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

    let (immediate, immediate_stats) =
        optimize_immediate(input, &route, search_stats.final_total_dist);
    let expected = score_immediate(input, &route, &immediate);

    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;

    for i in 0..M {
        let node = route[i];
        let p = selected_pos(input, node);
        move_count += move_to(&mut cur, p, &mut ops);
        push_op(&mut ops, b'Z');
        if immediate[i] {
            let p = other_pos(input, node);
            move_count += move_to(&mut cur, p, &mut ops);
            push_op(&mut ops, b'Z');
        }
    }

    let mut delete_dist = 0usize;
    for i in (0..M).rev() {
        if immediate[i] {
            continue;
        }
        let p = other_pos(input, route[i]);
        let moved = move_to(&mut cur, p, &mut ops);
        move_count += moved;
        delete_dist += moved;
        push_op(&mut ops, b'Z');
    }

    assert_eq!(delete_dist, expected.delete_dist);
    assert_eq!(move_count, immediate_stats.final_total_dist);
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
            immediate_stats.initial_total_dist,
            immediate_stats.iterations,
            immediate_stats.accepted,
            immediate_stats.improved,
            immediate_stats.immediate_count,
            immediate_stats.elapsed_ms,
        );
    }

    local! {
        let trace = TraceStats {
            initial_order_dist: search_stats.initial_total_dist,
            final_order_dist: search_stats.final_total_dist,
            initial_immediate_dist: immediate_stats.initial_total_dist,
            final_immediate_dist: immediate_stats.final_total_dist,
            collect_dist: expected.collect_dist,
            delete_dist,
            immediate_count: immediate_stats.immediate_count,
            move_count,
            turn_count: ops.len() / 2,
            order_iterations: search_stats.iterations,
            order_accepted: search_stats.accepted,
            order_improved: search_stats.improved,
            immediate_iterations: immediate_stats.iterations,
            immediate_accepted: immediate_stats.accepted,
            immediate_improved: immediate_stats.improved,
            order_elapsed_ms: search_stats.elapsed_ms,
            immediate_elapsed_ms: immediate_stats.elapsed_ms,
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
