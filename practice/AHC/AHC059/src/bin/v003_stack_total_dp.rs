// v003_stack_total_dp.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const SEARCH_TIME_LIMIT_SEC: f64 = 1.80;
const INF: usize = usize::MAX / 4;
const DEFAULT_TWO_OPT_PERCENT: usize = 45;
const DEFAULT_SWAP_PERCENT: usize = 30;
const DEFAULT_RELOCATE_PERCENT: usize = 25;
#[cfg(feature = "local")]
const NEIGHBOR_COUNT: usize = 3;
#[cfg(feature = "local")]
const PHASE_BUCKETS: usize = 10;
#[cfg(feature = "local")]
const KIND_TWO_OPT: usize = 0;
#[cfg(feature = "local")]
const KIND_SWAP: usize = 1;
#[cfg(feature = "local")]
const KIND_RELOCATE: usize = 2;

#[cfg(feature = "local")]
const NEIGHBOR_NAMES: [&str; NEIGHBOR_COUNT] = ["two_opt", "swap", "relocate"];

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone, Copy)]
struct NeighborStat {
    tried: usize,
    accepted: usize,
    accepted_delta_sum: i64,
    improve_accepted: usize,
    improve_gain: usize,
    worsen_accepted: usize,
    worsen_loss: usize,
    equal_accepted: usize,
    best_update: usize,
    best_gain: usize,
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct NeighborStats {
    total: [NeighborStat; NEIGHBOR_COUNT],
    phases: [[NeighborStat; PHASE_BUCKETS]; NEIGHBOR_COUNT],
}

#[cfg(feature = "local")]
impl NeighborStats {
    fn record_try(&mut self, kind: usize, phase: usize) {
        self.total[kind].tried += 1;
        self.phases[kind][phase].tried += 1;
    }

    fn record_accept_one(stat: &mut NeighborStat, delta: i32) {
        stat.accepted += 1;
        stat.accepted_delta_sum += delta as i64;
        if delta < 0 {
            stat.improve_accepted += 1;
            stat.improve_gain += (-delta) as usize;
        } else if delta > 0 {
            stat.worsen_accepted += 1;
            stat.worsen_loss += delta as usize;
        } else {
            stat.equal_accepted += 1;
        }
    }

    fn record_accept(&mut self, kind: usize, phase: usize, delta: i32) {
        Self::record_accept_one(&mut self.total[kind], delta);
        Self::record_accept_one(&mut self.phases[kind][phase], delta);
    }

    fn record_best_update_one(stat: &mut NeighborStat, gain: usize) {
        stat.best_update += 1;
        stat.best_gain += gain;
    }

    fn record_best_update(&mut self, kind: usize, phase: usize, gain: usize) {
        Self::record_best_update_one(&mut self.total[kind], gain);
        Self::record_best_update_one(&mut self.phases[kind][phase], gain);
    }

    fn print_stat(prefix: &str, kind_name: &str, stat: NeighborStat, phase: Option<usize>) {
        let rejected = stat.tried.saturating_sub(stat.accepted);
        let accept_rate = if stat.tried == 0 {
            0.0
        } else {
            stat.accepted as f64 / stat.tried as f64
        };
        if let Some(phase) = phase {
            eprintln!(
                "{} phase={} progress_from={} progress_to={} kind={} tried={} accepted={} rejected={} accept_rate={:.6} accepted_delta_sum={} improve_accepted={} improve_gain={} worsen_accepted={} worsen_loss={} equal_accepted={} best_update={} best_gain={}",
                prefix,
                phase,
                phase * 100 / PHASE_BUCKETS,
                (phase + 1) * 100 / PHASE_BUCKETS,
                kind_name,
                stat.tried,
                stat.accepted,
                rejected,
                accept_rate,
                stat.accepted_delta_sum,
                stat.improve_accepted,
                stat.improve_gain,
                stat.worsen_accepted,
                stat.worsen_loss,
                stat.equal_accepted,
                stat.best_update,
                stat.best_gain,
            );
        } else {
            eprintln!(
                "{} kind={} tried={} accepted={} rejected={} accept_rate={:.6} accepted_delta_sum={} improve_accepted={} improve_gain={} worsen_accepted={} worsen_loss={} equal_accepted={} best_update={} best_gain={}",
                prefix,
                kind_name,
                stat.tried,
                stat.accepted,
                rejected,
                accept_rate,
                stat.accepted_delta_sum,
                stat.improve_accepted,
                stat.improve_gain,
                stat.worsen_accepted,
                stat.worsen_loss,
                stat.equal_accepted,
                stat.best_update,
                stat.best_gain,
            );
        }
    }

    fn summary(&self) {
        for (kind, name) in NEIGHBOR_NAMES.iter().enumerate() {
            Self::print_stat("[neighbor]", name, self.total[kind], None);
        }
        for phase in 0..PHASE_BUCKETS {
            for (kind, name) in NEIGHBOR_NAMES.iter().enumerate() {
                Self::print_stat("[neighbor_phase]", name, self.phases[kind][phase], Some(phase));
            }
        }
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    greedy_total_dist: usize,
    initial_reversal_gain: usize,
    initial_total_dist: usize,
    final_total_dist: usize,
    collect_dist: usize,
    delete_dist: usize,
    move_count: usize,
    turn_count: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    elapsed_ms: f64,
    neighbor_mix: NeighborMix,
    neighbor_stats: NeighborStats,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] greedy={} total={} -> {} init_reversal_gain={} collect={} delete={} moves={} turns={} score_est={} iter={} accepted={} improved={} mix={}/{}/{} elapsed_ms={:.3}",
            self.greedy_total_dist,
            self.initial_total_dist,
            self.final_total_dist,
            self.initial_reversal_gain,
            self.collect_dist,
            self.delete_dist,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count,
            self.iterations,
            self.accepted,
            self.improved,
            self.neighbor_mix.two_opt,
            self.neighbor_mix.swap,
            self.neighbor_mix.relocate,
            self.elapsed_ms,
        );
        self.neighbor_stats.summary();
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

#[derive(Debug, Clone)]
struct SearchStats {
    greedy_total_dist: usize,
    initial_total_dist: usize,
    final_total_dist: usize,
    iterations: usize,
    accepted: usize,
    improved: usize,
    elapsed_ms: f64,
    neighbor_mix: NeighborMix,
    #[cfg(feature = "local")]
    neighbor_stats: NeighborStats,
}

#[derive(Debug, Default, Clone, Copy)]
struct NeighborMix {
    two_opt: usize,
    swap: usize,
    relocate: usize,
}

impl NeighborMix {
    fn total(self) -> usize {
        self.two_opt + self.swap + self.relocate
    }
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

#[cfg(feature = "local")]
fn read_percent_env(name: &str, default: usize) -> usize {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid {}: {}", name, value)),
        Err(_) => default,
    }
}

#[cfg(feature = "local")]
fn neighbor_mix() -> NeighborMix {
    let mix = NeighborMix {
        two_opt: read_percent_env("AHC_NEIGHBOR_TWO_OPT", DEFAULT_TWO_OPT_PERCENT),
        swap: read_percent_env("AHC_NEIGHBOR_SWAP", DEFAULT_SWAP_PERCENT),
        relocate: read_percent_env("AHC_NEIGHBOR_RELOCATE", DEFAULT_RELOCATE_PERCENT),
    };
    assert_eq!(
        mix.total(),
        100,
        "AHC_NEIGHBOR_* percentages must sum to 100"
    );
    mix
}

#[cfg(not(feature = "local"))]
fn neighbor_mix() -> NeighborMix {
    NeighborMix {
        two_opt: DEFAULT_TWO_OPT_PERCENT,
        swap: DEFAULT_SWAP_PERCENT,
        relocate: DEFAULT_RELOCATE_PERCENT,
    }
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
    let mix = neighbor_mix();
    let mut order = make_initial_order(input);
    let mut current_score = score_order(input, &order);
    let greedy_score = current_score;
    improve_initial_by_reversal(input, &mut order, &mut current_score);

    let initial_score = current_score;
    let mut best_score = current_score;
    let mut best_order = order.clone();
    let mut rng = Rng::new(0x517c_c1b7_d24b_8f1d ^ initial_score as u64);

    let mut iterations = 0usize;
    let mut accepted = 0usize;
    let mut improved = 0usize;
    let mut temp = 10.0;
    #[cfg(feature = "local")]
    let mut neighbor_stats = NeighborStats::default();
    #[cfg(feature = "local")]
    let mut phase = 0usize;

    loop {
        iterations += 1;
        if (iterations & 255) == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed >= SEARCH_TIME_LIMIT_SEC {
                break;
            }
            let progress = (elapsed / SEARCH_TIME_LIMIT_SEC).clamp(0.0, 1.0);
            temp = 10.0_f64.powf(1.0 - progress) * 0.05_f64.powf(progress);
            #[cfg(feature = "local")]
            {
                phase = ((progress * PHASE_BUCKETS as f64) as usize).min(PHASE_BUCKETS - 1);
            }
        }

        let move_type = rng.gen_usize(mix.total());
        let old_score = current_score;
        #[cfg(feature = "local")]
        let accepted_kind: usize;
        if move_type < mix.two_opt {
            let mut l = rng.gen_usize(M);
            let mut r = rng.gen_usize(M);
            if l == r {
                continue;
            }
            if l > r {
                std::mem::swap(&mut l, &mut r);
            }
            #[cfg(feature = "local")]
            neighbor_stats.record_try(KIND_TWO_OPT, phase);
            order[l..=r].reverse();
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
                #[cfg(feature = "local")]
                {
                    neighbor_stats.record_accept(KIND_TWO_OPT, phase, delta);
                    accepted_kind = KIND_TWO_OPT;
                }
            } else {
                order[l..=r].reverse();
                continue;
            }
        } else if move_type < mix.two_opt + mix.swap {
            let mut i = rng.gen_usize(M);
            let mut j = rng.gen_usize(M);
            if i == j {
                continue;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            #[cfg(feature = "local")]
            neighbor_stats.record_try(KIND_SWAP, phase);
            order.swap(i, j);
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
                #[cfg(feature = "local")]
                {
                    neighbor_stats.record_accept(KIND_SWAP, phase, delta);
                    accepted_kind = KIND_SWAP;
                }
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
            #[cfg(feature = "local")]
            neighbor_stats.record_try(KIND_RELOCATE, phase);
            apply_relocate(&mut order, i, j);
            let next_score = score_order(input, &order);
            let delta = next_score as i32 - old_score as i32;
            if accept(delta, temp, &mut rng) {
                current_score = next_score;
                accepted += 1;
                #[cfg(feature = "local")]
                {
                    neighbor_stats.record_accept(KIND_RELOCATE, phase, delta);
                    accepted_kind = KIND_RELOCATE;
                }
            } else {
                undo_relocate(&mut order, i, j);
                continue;
            }
        }

        if current_score < best_score {
            #[cfg(feature = "local")]
            neighbor_stats.record_best_update(accepted_kind, phase, best_score - current_score);
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
            greedy_total_dist: greedy_score,
            initial_total_dist: initial_score,
            final_total_dist: best_score,
            iterations,
            accepted,
            improved,
            elapsed_ms: elapsed * 1000.0,
            neighbor_mix: mix,
            #[cfg(feature = "local")]
            neighbor_stats,
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
    assert_eq!(move_count, search_stats.final_total_dist);
    assert_eq!(move_count, expected_collect_dist + expected_delete_dist);
    assert!(ops.len() / 2 <= MAX_T);
    assert!(move_count <= MAX_T);

    #[cfg(not(feature = "local"))]
    {
        let _ = (
            search_stats.greedy_total_dist,
            search_stats.initial_total_dist,
            search_stats.iterations,
            search_stats.accepted,
            search_stats.improved,
            search_stats.elapsed_ms,
            search_stats.neighbor_mix,
        );
    }

    local! {
        let trace = TraceStats {
            greedy_total_dist: search_stats.greedy_total_dist,
            initial_reversal_gain: search_stats.greedy_total_dist - search_stats.initial_total_dist,
            initial_total_dist: search_stats.initial_total_dist,
            final_total_dist: search_stats.final_total_dist,
            collect_dist: expected_collect_dist,
            delete_dist,
            move_count,
            turn_count: ops.len() / 2,
            iterations: search_stats.iterations,
            accepted: search_stats.accepted,
            improved: search_stats.improved,
            elapsed_ms: search_stats.elapsed_ms,
            neighbor_mix: search_stats.neighbor_mix,
            neighbor_stats: search_stats.neighbor_stats,
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
