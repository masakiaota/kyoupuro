// v002_greedy_only.rs

use std::io::{self, Read};

const N: usize = 1000;
const M: usize = 50;
const CENTER: (i32, i32) = (400, 400);

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

fn insert_order_events(seq: &mut Vec<Event>, order: usize, pick_pos: usize, drop_pos: usize) {
    seq.insert(pick_pos, Event { order, kind: 0 });
    seq.insert(drop_pos, Event { order, kind: 1 });
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

fn main() {
    let orders = input_orders();

    let mut seq: Vec<Event> = Vec::with_capacity(2 * M);
    let mut selected = vec![false; N];
    let mut selected_ids: Vec<usize> = Vec::with_capacity(M);

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

    let _ = nodes_length(&path);
}
