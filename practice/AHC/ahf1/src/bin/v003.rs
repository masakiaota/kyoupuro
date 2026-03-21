// v003.rs

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::io::{self, Read};
use std::time::{Duration, Instant};

const N: usize = 1000;
const M: usize = 50;
const CENTER: (i32, i32) = (400, 400);
const REINSERT_PASSES: usize = 2;
const REPLACE_BUDGET_MS: u64 = 800;

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
    let nodes = build_nodes(seq, orders);
    nodes_length(&nodes)
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
        for v in [o.pickup.0 as u64, o.pickup.1 as u64, o.dropoff.0 as u64, o.dropoff.1 as u64] {
            h ^= v.wrapping_add(0x9E37_79B9_7F4A_7C15_u64).rotate_left(17);
            h = h.rotate_left(11).wrapping_mul(0xA24B_AED4_963E_E407_u64);
        }
    }
    h
}

fn reinforce_reinsert(
    seq: &mut Vec<Event>,
    orders: &[Order],
    selected_ids: &[usize],
    current_len: &mut i64,
) {
    for _ in 0..REINSERT_PASSES {
        let mut improved = false;
        for &order in selected_ids {
            let reduced = remove_order_events(seq, order);
            let reduced_nodes = build_nodes(&reduced, orders);
            let reduced_len = nodes_length(&reduced_nodes);
            let ins = best_insertion(&reduced_nodes, orders[order].pickup, orders[order].dropoff);
            let new_len = reduced_len + ins.delta as i64;
            if new_len < *current_len {
                let mut updated = reduced;
                insert_order_events(&mut updated, order, ins.pick_pos, ins.drop_pos);
                *seq = updated;
                *current_len = new_len;
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }
}

fn reinforce_replace(
    seq: &mut Vec<Event>,
    orders: &[Order],
    selected: &mut [bool],
    selected_ids: &mut [usize],
    current_len: &mut i64,
    budget: Duration,
    rng: &mut Xoshiro256PlusPlus,
) {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        let rem_idx = rng.random_range(0..M);
        let rem_order = selected_ids[rem_idx];

        let mut add_order = rng.random_range(0..N);
        while selected[add_order] {
            add_order = rng.random_range(0..N);
        }

        let reduced = remove_order_events(seq, rem_order);
        let reduced_nodes = build_nodes(&reduced, orders);
        let reduced_len = nodes_length(&reduced_nodes);
        let ins = best_insertion(&reduced_nodes, orders[add_order].pickup, orders[add_order].dropoff);
        let new_len = reduced_len + ins.delta as i64;

        if new_len < *current_len {
            let mut updated = reduced;
            insert_order_events(&mut updated, add_order, ins.pick_pos, ins.drop_pos);
            *seq = updated;
            *current_len = new_len;
            selected[rem_order] = false;
            selected[add_order] = true;
            selected_ids[rem_idx] = add_order;
        }
    }
}

fn main() {
    let orders = input_orders();

    let mut seq: Vec<Event> = Vec::with_capacity(2 * M);
    let mut selected = vec![false; N];
    let mut selected_ids: Vec<usize> = Vec::with_capacity(M);
    let mut current_len = 0_i64;

    // Greedy construction: choose 50 orders with minimum insertion increment.
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

    reinforce_reinsert(
        &mut seq,
        &orders,
        &selected_ids,
        &mut current_len,
    );

    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_orders(&orders));
    reinforce_replace(
        &mut seq,
        &orders,
        &mut selected,
        &mut selected_ids,
        &mut current_len,
        Duration::from_millis(REPLACE_BUDGET_MS),
        &mut rng,
    );

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
