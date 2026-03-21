// v011_hill_climb.rs

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::io::{self, Read};
use std::time::{Duration, Instant};

const N: usize = 1000;
const M: usize = 50;
const CENTER: (i32, i32) = (400, 400);
const TOTAL_BUDGET_MS: u64 = 1600;
const DEFAULT_REINSERT_WEIGHT: u32 = 35;
const DEFAULT_REPLACE1_WEIGHT: u32 = 55;
const DEFAULT_REPLACE2_WEIGHT: u32 = 10;

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

#[derive(Clone, Copy)]
struct MoveWeights {
    reinsert: u32,
    replace1: u32,
    replace2: u32,
}

fn dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn event_point(ev: Event, orders: &[Order]) -> (i32, i32) {
    if ev.kind == 0 {
        orders[ev.order].pickup
    } else {
        orders[ev.order].dropoff
    }
}

fn build_nodes(seq: &[Event], orders: &[Order]) -> Vec<(i32, i32)> {
    let mut nodes = Vec::with_capacity(seq.len() + 2);
    nodes.push(CENTER);
    for &ev in seq {
        nodes.push(event_point(ev, orders));
    }
    nodes.push(CENTER);
    nodes
}

fn nodes_length(nodes: &[(i32, i32)]) -> i64 {
    let mut total = 0_i64;
    for i in 1..nodes.len() {
        total += dist(nodes[i - 1], nodes[i]) as i64;
    }
    total
}

fn total_length(seq: &[Event], orders: &[Order]) -> i64 {
    nodes_length(&build_nodes(seq, orders))
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

fn best_insertion(nodes: &[(i32, i32)], pickup: (i32, i32), dropoff: (i32, i32)) -> Insertion {
    let edges = nodes.len() - 1;
    let mut best = Insertion {
        delta: i32::MAX,
        pick_pos: 0,
        drop_pos: 1,
    };

    for i in 0..edges {
        let a = nodes[i];
        let b = nodes[i + 1];
        let delta_pick = dist(a, pickup) + dist(pickup, b) - dist(a, b);

        let delta_after_pick = delta_pick + dist(pickup, dropoff) + dist(dropoff, b) - dist(pickup, b);
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
            let delta = delta_pick + dist(l, dropoff) + dist(dropoff, r) - dist(l, r);
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

fn apply_best_insertion(seq: &mut Vec<Event>, orders: &[Order], order: usize) -> i64 {
    let nodes = build_nodes(seq, orders);
    let base_len = nodes_length(&nodes);
    let ins = best_insertion(&nodes, orders[order].pickup, orders[order].dropoff);
    insert_order_events(seq, order, ins.pick_pos, ins.drop_pos);
    base_len + ins.delta as i64
}

fn input_orders() -> Vec<Order> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let mut orders = Vec::with_capacity(N);
    for _ in 0..N {
        let a: i32 = it.next().unwrap().parse().unwrap();
        let b: i32 = it.next().unwrap().parse().unwrap();
        let c: i32 = it.next().unwrap().parse().unwrap();
        let d: i32 = it.next().unwrap().parse().unwrap();
        orders.push(Order {
            pickup: (a, b),
            dropoff: (c, d),
        });
    }
    orders
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

fn read_weight_var(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default)
}

fn read_move_weights() -> MoveWeights {
    let weights = MoveWeights {
        reinsert: read_weight_var("V011_REINSERT_WEIGHT", DEFAULT_REINSERT_WEIGHT),
        replace1: read_weight_var("V011_REPLACE1_WEIGHT", DEFAULT_REPLACE1_WEIGHT),
        replace2: read_weight_var("V011_REPLACE2_WEIGHT", DEFAULT_REPLACE2_WEIGHT),
    };
    let sum = weights.reinsert + weights.replace1 + weights.replace2;
    if sum == 0 {
        MoveWeights {
            reinsert: DEFAULT_REINSERT_WEIGHT,
            replace1: DEFAULT_REPLACE1_WEIGHT,
            replace2: DEFAULT_REPLACE2_WEIGHT,
        }
    } else {
        weights
    }
}

fn random_unselected(selected: &[bool], rng: &mut Xoshiro256PlusPlus) -> usize {
    let mut order = rng.random_range(0..N);
    while selected[order] {
        order = rng.random_range(0..N);
    }
    order
}

fn try_reinsert(
    seq: &mut Vec<Event>,
    orders: &[Order],
    selected_ids: &[usize],
    current_len: &mut i64,
    rng: &mut Xoshiro256PlusPlus,
) {
    let order = selected_ids[rng.random_range(0..M)];
    let mut reduced = remove_order_events(seq, order);
    let new_len = apply_best_insertion(&mut reduced, orders, order);
    if new_len < *current_len {
        *seq = reduced;
        *current_len = new_len;
    }
}

fn try_replace(
    seq: &mut Vec<Event>,
    orders: &[Order],
    selected: &mut [bool],
    selected_ids: &mut [usize],
    current_len: &mut i64,
    rng: &mut Xoshiro256PlusPlus,
) {
    let rem_idx = rng.random_range(0..M);
    let rem_order = selected_ids[rem_idx];
    let add_order = random_unselected(selected, rng);

    let mut reduced = remove_order_events(seq, rem_order);
    let new_len = apply_best_insertion(&mut reduced, orders, add_order);
    if new_len < *current_len {
        *seq = reduced;
        *current_len = new_len;
        selected[rem_order] = false;
        selected[add_order] = true;
        selected_ids[rem_idx] = add_order;
    }
}

fn try_double_replace(
    seq: &mut Vec<Event>,
    orders: &[Order],
    selected: &mut [bool],
    selected_ids: &mut [usize],
    current_len: &mut i64,
    rng: &mut Xoshiro256PlusPlus,
) {
    let idx_a = rng.random_range(0..M);
    let mut idx_b = rng.random_range(0..M);
    while idx_b == idx_a {
        idx_b = rng.random_range(0..M);
    }
    let rem_a = selected_ids[idx_a];
    let rem_b = selected_ids[idx_b];

    let add_a = random_unselected(selected, rng);
    let mut add_b = random_unselected(selected, rng);
    while add_b == add_a {
        add_b = random_unselected(selected, rng);
    }

    let reduced = remove_two_orders_events(seq, rem_a, rem_b);

    let mut cand1 = reduced.clone();
    let len1a = apply_best_insertion(&mut cand1, orders, add_a);
    let len1b = apply_best_insertion(&mut cand1, orders, add_b);
    let cand1_len = len1b.max(len1a);

    let mut cand2 = reduced;
    let len2a = apply_best_insertion(&mut cand2, orders, add_b);
    let len2b = apply_best_insertion(&mut cand2, orders, add_a);
    let cand2_len = len2b.max(len2a);

    if cand1_len < *current_len || cand2_len < *current_len {
        if cand1_len <= cand2_len {
            *seq = cand1;
            *current_len = cand1_len;
            selected_ids[idx_a] = add_a;
            selected_ids[idx_b] = add_b;
        } else {
            *seq = cand2;
            *current_len = cand2_len;
            selected_ids[idx_a] = add_b;
            selected_ids[idx_b] = add_a;
        }
        selected[rem_a] = false;
        selected[rem_b] = false;
        selected[selected_ids[idx_a]] = true;
        selected[selected_ids[idx_b]] = true;
    }
}

fn main() {
    let orders = input_orders();
    let weights = read_move_weights();

    let mut seq: Vec<Event> = Vec::with_capacity(2 * M);
    let mut selected = vec![false; N];
    let mut selected_ids: Vec<usize> = Vec::with_capacity(M);
    let mut current_len = 0_i64;

    for _ in 0..M {
        let nodes = build_nodes(&seq, &orders);
        let mut best_order = usize::MAX;
        let mut best_ins = Insertion {
            delta: i32::MAX,
            pick_pos: 0,
            drop_pos: 1,
        };

        for order in 0..N {
            if selected[order] {
                continue;
            }
            let ins = best_insertion(&nodes, orders[order].pickup, orders[order].dropoff);
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
    current_len = total_length(&seq, &orders);

    let deadline = Instant::now() + Duration::from_millis(TOTAL_BUDGET_MS);
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_orders(&orders));
    let threshold_reinsert = weights.reinsert;
    let threshold_replace1 = weights.reinsert + weights.replace1;
    let threshold_total = weights.reinsert + weights.replace1 + weights.replace2;
    while Instant::now() < deadline {
        let move_type = rng.random_range(0..threshold_total);
        if move_type < threshold_reinsert {
            try_reinsert(&mut seq, &orders, &selected_ids, &mut current_len, &mut rng);
        } else if move_type < threshold_replace1 {
            try_replace(
                &mut seq,
                &orders,
                &mut selected,
                &mut selected_ids,
                &mut current_len,
                &mut rng,
            );
        } else {
            try_double_replace(
                &mut seq,
                &orders,
                &mut selected,
                &mut selected_ids,
                &mut current_len,
                &mut rng,
            );
        }
    }

    let mut path: Vec<(i32, i32)> = Vec::with_capacity(seq.len() + 2);
    path.push(CENTER);
    for &ev in &seq {
        path.push(event_point(ev, &orders));
    }
    path.push(CENTER);

    print!("{}", M);
    for &order in &selected_ids {
        print!(" {}", order + 1);
    }
    println!();

    print!("{}", path.len());
    for &(x, y) in &path {
        print!(" {} {}", x, y);
    }
    println!();
}
