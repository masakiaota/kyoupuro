// v103_multi_start9_sa.rs

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::io::{self, Read};
use std::time::{Duration, Instant};

const N: usize = 1000;
const M: usize = 50;
const CENTER: (i32, i32) = (400, 400);
const CENTER_ID: usize = 2 * N;
const NODE_COUNT: usize = 2 * N + 1;
const TOTAL_BUDGET_MS: u64 = 1600;
const REINSERT_WEIGHT: u32 = 20;
const REPLACE1_WEIGHT: u32 = 60;
const REPLACE2_WEIGHT: u32 = 20;
const START_TEMP: f64 = 50.0;
const END_TEMP: f64 = 5.0;
const INITIAL_CANDIDATES: usize = 90;

#[derive(Clone, Copy)]
struct Order {
    pickup: (i32, i32),
    dropoff: (i32, i32),
}

#[derive(Clone, Copy)]
struct Event {
    order: usize,
    kind: u8, // 0: pickup, 1: dropoff
}

#[derive(Clone, Copy)]
struct Insertion {
    delta: i32,
    pick_pos: usize,
    drop_pos: usize,
}

#[derive(Clone)]
struct Problem {
    orders: Vec<Order>,
    points: Vec<(i32, i32)>,
    dist: Vec<i16>,
}

#[derive(Clone)]
struct State {
    seq: Vec<Event>,
    selected: Vec<bool>,
    selected_ids: Vec<usize>,
    len: i64,
}

fn manhattan(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn event_node_id(ev: Event) -> usize {
    if ev.kind == 0 {
        ev.order
    } else {
        ev.order + N
    }
}

fn build_nodes(seq: &[Event]) -> Vec<usize> {
    let mut nodes = Vec::with_capacity(seq.len() + 2);
    nodes.push(CENTER_ID);
    for &ev in seq {
        nodes.push(event_node_id(ev));
    }
    nodes.push(CENTER_ID);
    nodes
}

fn get_dist(problem: &Problem, a: usize, b: usize) -> i32 {
    problem.dist[a * NODE_COUNT + b] as i32
}

fn nodes_length(problem: &Problem, nodes: &[usize]) -> i64 {
    let mut total = 0_i64;
    for i in 1..nodes.len() {
        total += get_dist(problem, nodes[i - 1], nodes[i]) as i64;
    }
    total
}

fn total_length(problem: &Problem, seq: &[Event]) -> i64 {
    nodes_length(problem, &build_nodes(seq))
}

fn insert_order_events(seq: &mut Vec<Event>, order: usize, pick_pos: usize, drop_pos: usize) {
    seq.insert(pick_pos, Event { order, kind: 0 });
    seq.insert(drop_pos, Event { order, kind: 1 });
}

fn remove_order_events(seq: &[Event], order: usize) -> Vec<Event> {
    let mut reduced = Vec::with_capacity(seq.len().saturating_sub(2));
    for &ev in seq {
        if ev.order != order {
            reduced.push(ev);
        }
    }
    reduced
}

fn remove_two_orders_events(seq: &[Event], order_a: usize, order_b: usize) -> Vec<Event> {
    let mut reduced = Vec::with_capacity(seq.len().saturating_sub(4));
    for &ev in seq {
        if ev.order != order_a && ev.order != order_b {
            reduced.push(ev);
        }
    }
    reduced
}

fn best_insertion(problem: &Problem, nodes: &[usize], order: usize) -> Insertion {
    let pickup = order;
    let dropoff = order + N;
    let edges = nodes.len() - 1;
    let mut best = Insertion {
        delta: i32::MAX,
        pick_pos: 0,
        drop_pos: 1,
    };

    for i in 0..edges {
        let a = nodes[i];
        let b = nodes[i + 1];
        let delta_pick = get_dist(problem, a, pickup) + get_dist(problem, pickup, b) - get_dist(problem, a, b);

        let delta_after_pick = delta_pick
            + get_dist(problem, pickup, dropoff)
            + get_dist(problem, dropoff, b)
            - get_dist(problem, pickup, b);
        if delta_after_pick < best.delta {
            best = Insertion {
                delta: delta_after_pick,
                pick_pos: i,
                drop_pos: i + 1,
            };
        }

        for j in (i + 2)..=edges {
            let l = nodes[j - 1];
            let r = nodes[j];
            let delta = delta_pick + get_dist(problem, l, dropoff) + get_dist(problem, dropoff, r) - get_dist(problem, l, r);
            if delta < best.delta {
                best = Insertion {
                    delta,
                    pick_pos: i,
                    drop_pos: j,
                };
            }
        }
    }

    best
}

fn apply_best_insertion(problem: &Problem, seq: &mut Vec<Event>, order: usize) -> i64 {
    let nodes = build_nodes(seq);
    let base_len = nodes_length(problem, &nodes);
    let ins = best_insertion(problem, &nodes, order);
    insert_order_events(seq, order, ins.pick_pos, ins.drop_pos);
    base_len + ins.delta as i64
}

fn input_problem() -> Problem {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let mut orders = Vec::with_capacity(N);
    let mut points = vec![(0_i32, 0_i32); NODE_COUNT];
    for i in 0..N {
        let a: i32 = it.next().unwrap().parse().unwrap();
        let b: i32 = it.next().unwrap().parse().unwrap();
        let c: i32 = it.next().unwrap().parse().unwrap();
        let d: i32 = it.next().unwrap().parse().unwrap();
        orders.push(Order {
            pickup: (a, b),
            dropoff: (c, d),
        });
        points[i] = (a, b);
        points[i + N] = (c, d);
    }
    points[CENTER_ID] = CENTER;

    let mut dist = vec![0_i16; NODE_COUNT * NODE_COUNT];
    for i in 0..NODE_COUNT {
        for j in 0..NODE_COUNT {
            dist[i * NODE_COUNT + j] = manhattan(points[i], points[j]) as i16;
        }
    }

    Problem { orders, points, dist }
}

fn seed_from_orders(orders: &[Order]) -> u64 {
    let mut h = 0x9E37_79B9_7F4A_7C15_u64;
    for o in orders {
        for v in [
            o.pickup.0 as u64,
            o.pickup.1 as u64,
            o.dropoff.0 as u64,
            o.dropoff.1 as u64,
        ] {
            h ^= v.wrapping_add(0x9E37_79B9_7F4A_7C15_u64).rotate_left(17);
            h = h.rotate_left(11).wrapping_mul(0xA24B_AED4_963E_E407_u64);
        }
    }
    h
}

fn random_unselected(selected: &[bool], rng: &mut Xoshiro256PlusPlus) -> usize {
    let mut order = rng.random_range(0..N);
    while selected[order] {
        order = rng.random_range(0..N);
    }
    order
}

fn greedy_candidate_orders(problem: &Problem, center: (i32, i32)) -> Vec<usize> {
    let mut scored: Vec<(i32, usize)> = (0..N)
        .map(|order| {
            let d = manhattan(center, problem.orders[order].pickup)
                .max(manhattan(center, problem.orders[order].dropoff));
            (d, order)
        })
        .collect();
    scored.sort_unstable();
    scored
        .into_iter()
        .take(INITIAL_CANDIDATES)
        .map(|(_, order)| order)
        .collect()
}

fn greedy_initial_state(problem: &Problem, center: (i32, i32)) -> State {
    let mut seq: Vec<Event> = Vec::with_capacity(2 * M);
    let mut selected = vec![false; N];
    let mut selected_ids: Vec<usize> = Vec::with_capacity(M);
    let mut current_len = 0_i64;
    let candidates = greedy_candidate_orders(problem, center);

    for _ in 0..M {
        let nodes = build_nodes(&seq);
        let mut best_order = usize::MAX;
        let mut best_ins = Insertion {
            delta: i32::MAX,
            pick_pos: 0,
            drop_pos: 1,
        };

        for &order in &candidates {
            if selected[order] {
                continue;
            }
            let ins = best_insertion(problem, &nodes, order);
            if ins.delta < best_ins.delta {
                best_ins = ins;
                best_order = order;
            }
        }

        insert_order_events(&mut seq, best_order, best_ins.pick_pos, best_ins.drop_pos);
        selected[best_order] = true;
        selected_ids.push(best_order);
        current_len += best_ins.delta as i64;
    }

    let len = current_len.max(total_length(problem, &seq));
    State {
        seq,
        selected,
        selected_ids,
        len,
    }
}

fn best_initial_state(problem: &Problem) -> State {
    let centers = [
        (300, 300),
        (300, 400),
        (300, 500),
        (400, 300),
        (400, 400),
        (400, 500),
        (500, 300),
        (500, 400),
        (500, 500),
    ];

    let mut best = greedy_initial_state(problem, centers[0]);
    for &center in &centers[1..] {
        let candidate = greedy_initial_state(problem, center);
        if candidate.len < best.len {
            best = candidate;
        }
    }
    best
}

fn propose_reinsert(state: &State, problem: &Problem, rng: &mut Xoshiro256PlusPlus) -> State {
    let order = state.selected_ids[rng.random_range(0..M)];
    let mut candidate = state.clone();
    candidate.seq = remove_order_events(&state.seq, order);
    candidate.len = apply_best_insertion(problem, &mut candidate.seq, order);
    candidate
}

fn propose_replace(state: &State, problem: &Problem, rng: &mut Xoshiro256PlusPlus) -> State {
    let rem_idx = rng.random_range(0..M);
    let rem_order = state.selected_ids[rem_idx];
    let add_order = random_unselected(&state.selected, rng);

    let mut candidate = state.clone();
    candidate.seq = remove_order_events(&state.seq, rem_order);
    candidate.len = apply_best_insertion(problem, &mut candidate.seq, add_order);
    candidate.selected[rem_order] = false;
    candidate.selected[add_order] = true;
    candidate.selected_ids[rem_idx] = add_order;
    candidate
}

fn propose_double_replace(state: &State, problem: &Problem, rng: &mut Xoshiro256PlusPlus) -> State {
    let idx_a = rng.random_range(0..M);
    let mut idx_b = rng.random_range(0..M);
    while idx_b == idx_a {
        idx_b = rng.random_range(0..M);
    }
    let rem_a = state.selected_ids[idx_a];
    let rem_b = state.selected_ids[idx_b];

    let add_a = random_unselected(&state.selected, rng);
    let mut add_b = random_unselected(&state.selected, rng);
    while add_b == add_a {
        add_b = random_unselected(&state.selected, rng);
    }

    let reduced = remove_two_orders_events(&state.seq, rem_a, rem_b);

    let mut cand1 = state.clone();
    cand1.seq = reduced.clone();
    cand1.len = apply_best_insertion(problem, &mut cand1.seq, add_a);
    cand1.len = apply_best_insertion(problem, &mut cand1.seq, add_b);
    cand1.selected[rem_a] = false;
    cand1.selected[rem_b] = false;
    cand1.selected[add_a] = true;
    cand1.selected[add_b] = true;
    cand1.selected_ids[idx_a] = add_a;
    cand1.selected_ids[idx_b] = add_b;

    let mut cand2 = state.clone();
    cand2.seq = reduced;
    cand2.len = apply_best_insertion(problem, &mut cand2.seq, add_b);
    cand2.len = apply_best_insertion(problem, &mut cand2.seq, add_a);
    cand2.selected[rem_a] = false;
    cand2.selected[rem_b] = false;
    cand2.selected[add_a] = true;
    cand2.selected[add_b] = true;
    cand2.selected_ids[idx_a] = add_b;
    cand2.selected_ids[idx_b] = add_a;

    if cand1.len <= cand2.len {
        cand1
    } else {
        cand2
    }
}

fn temperature(progress: f64) -> f64 {
    let p = progress.clamp(0.0, 1.0);
    START_TEMP * (END_TEMP / START_TEMP).powf(p)
}

fn should_accept(
    current_len: i64,
    candidate_len: i64,
    temp: f64,
    rng: &mut Xoshiro256PlusPlus,
) -> bool {
    if candidate_len <= current_len {
        return true;
    }
    let delta = (candidate_len - current_len) as f64;
    let prob = (-delta / temp).exp();
    rng.random::<f64>() < prob
}

fn main() {
    let problem = input_problem();
    let deadline = Instant::now() + Duration::from_millis(TOTAL_BUDGET_MS);

    let mut current = best_initial_state(&problem);
    let mut best = current.clone();

    let start = Instant::now();
    let total_secs = deadline
        .saturating_duration_since(start)
        .as_secs_f64()
        .max(1e-9);
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_orders(&problem.orders));
    let threshold_reinsert = REINSERT_WEIGHT;
    let threshold_replace1 = REINSERT_WEIGHT + REPLACE1_WEIGHT;
    let threshold_total = REINSERT_WEIGHT + REPLACE1_WEIGHT + REPLACE2_WEIGHT;

    while Instant::now() < deadline {
        let progress = start.elapsed().as_secs_f64() / total_secs;
        let temp = temperature(progress);
        let move_type = rng.random_range(0..threshold_total);

        let candidate = if move_type < threshold_reinsert {
            propose_reinsert(&current, &problem, &mut rng)
        } else if move_type < threshold_replace1 {
            propose_replace(&current, &problem, &mut rng)
        } else {
            propose_double_replace(&current, &problem, &mut rng)
        };

        if should_accept(current.len, candidate.len, temp, &mut rng) {
            current = candidate;
            if current.len < best.len {
                best = current.clone();
            }
        }
    }

    print!("{}", M);
    for &order in &best.selected_ids {
        print!(" {}", order + 1);
    }
    println!();

    print!("{}", best.seq.len() + 2);
    print!(" {} {}", CENTER.0, CENTER.1);
    for &ev in &best.seq {
        let (x, y) = problem.points[event_node_id(ev)];
        print!(" {} {}", x, y);
    }
    print!(" {} {}", CENTER.0, CENTER.1);
    println!();
}
