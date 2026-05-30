// v901_greedy_from_null.rs
use std::io::{self, Read, Write};

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const INF: usize = usize::MAX / 4;
const INVALID: usize = usize::MAX;
const DIST: [[u16; NN]; NN] = build_dist();

const fn build_dist() -> [[u16; NN]; NN] {
    let mut dist = [[0; NN]; NN];
    let mut p = 0;
    while p < NN {
        let pi = p / N;
        let pj = p % N;
        let mut q = 0;
        while q < NN {
            let qi = q / N;
            let qj = q % N;
            let di = if pi >= qi { pi - qi } else { qi - pi };
            let dj = if pj >= qj { pj - qj } else { qj - pj };
            dist[p][q] = (di + dj) as u16;
            q += 1;
        }
        p += 1;
    }
    dist
}

#[derive(Debug, Clone)]
struct Input {
    /// cell id -> card number
    a: [usize; NN],
    /// `pos[2 * v]`, `pos[2 * v + 1]` are the two cell ids of card `v`.
    pos: [usize; NN],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n = it.next().unwrap().parse::<usize>().unwrap();
        debug_assert_eq!(n, N);

        let mut a = [0; NN];
        let mut pos = [0; NN];
        let mut count = [0; M];

        for id in 0..NN {
            let v = it.next().unwrap().parse::<usize>().unwrap();
            debug_assert!(v < M);
            let k = count[v];
            debug_assert!(k < 2);
            a[id] = v;
            pos[2 * v + k] = id;
            count[v] += 1;
        }

        #[cfg(feature = "local")]
        {
            for &c in &count {
                debug_assert_eq!(c, 2);
            }
        }

        Self { a, pos }
    }

    #[inline(always)]
    fn dist(p: usize, q: usize) -> usize {
        DIST[p][q] as usize
    }

    #[inline(always)]
    fn card(&self, id: usize) -> usize {
        self.a[id]
    }

    #[inline(always)]
    fn pair(&self, v: usize) -> (usize, usize) {
        (self.pos[2 * v], self.pos[2 * v + 1])
    }
}

#[derive(Debug, Clone, Copy)]
struct Insertion {
    first_cell: usize,
    first_index: usize,
    second_cell: usize,
    second_index: usize,
    cost: usize,
}

impl Insertion {
    fn none() -> Self {
        Self {
            first_cell: INVALID,
            first_index: INVALID,
            second_cell: INVALID,
            second_index: INVALID,
            cost: INF,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    cost: usize,
    index: usize,
}

impl Candidate {
    fn none() -> Self {
        Self {
            cost: INF,
            index: INVALID,
        }
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    greedy_insert_dist: usize,
    move_count: usize,
    turn_count: usize,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] greedy_insert_dist={} moves={} turns={} score_est={}",
            self.greedy_insert_dist,
            self.move_count,
            self.turn_count,
            NN + MAX_T - self.move_count
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

#[inline(always)]
fn edge_after_insert(left: usize, x: usize, right: Option<usize>) -> usize {
    match right {
        Some(right) => Input::dist(left, x) + Input::dist(x, right) - Input::dist(left, right),
        None => Input::dist(left, x),
    }
}

#[inline(always)]
fn insert_delta(state: &[usize], x: usize, index: usize) -> usize {
    let left = if index == 0 { 0 } else { state[index - 1] };
    let right = if index == state.len() {
        None
    } else {
        Some(state[index])
    };
    edge_after_insert(left, x, right)
}

#[inline(always)]
fn adjacent_delta(state: &[usize], x: usize, y: usize, index: usize) -> usize {
    let left = if index == 0 { 0 } else { state[index - 1] };
    if index == state.len() {
        Input::dist(left, x) + Input::dist(x, y)
    } else {
        let right = state[index];
        Input::dist(left, x) + Input::dist(x, y) + Input::dist(y, right)
            - Input::dist(left, right)
    }
}

fn update_best(best: &mut Insertion, next: Insertion) {
    if (next.cost, next.first_index, next.second_index, next.first_cell, next.second_cell)
        < (
            best.cost,
            best.first_index,
            best.second_index,
            best.first_cell,
            best.second_cell,
        )
    {
        *best = next;
    }
}

fn best_insert_positions(input: &Input, state: &[usize], a: usize, b: usize) -> Insertion {
    let mut best = Insertion::none();
    let mut best_a = vec![Candidate::none()];
    let mut best_b = vec![Candidate::none()];
    let mut count = [0u8; M];

    for index in 0..=state.len() {
        let depth = best_a.len() - 1;
        let ab_cost = adjacent_delta(state, a, b, index);
        update_best(
            &mut best,
            Insertion {
                first_cell: a,
                first_index: index,
                second_cell: b,
                second_index: index + 1,
                cost: ab_cost,
            },
        );

        let ba_cost = adjacent_delta(state, b, a, index);
        update_best(
            &mut best,
            Insertion {
                first_cell: b,
                first_index: index,
                second_cell: a,
                second_index: index + 1,
                cost: ba_cost,
            },
        );

        let a_delta = insert_delta(state, a, index);
        let b_delta = insert_delta(state, b, index);

        if best_a[depth].cost < INF {
            update_best(
                &mut best,
                Insertion {
                    first_cell: a,
                    first_index: best_a[depth].index,
                    second_cell: b,
                    second_index: index + 1,
                    cost: best_a[depth].cost + b_delta,
                },
            );
        }
        if best_b[depth].cost < INF {
            update_best(
                &mut best,
                Insertion {
                    first_cell: b,
                    first_index: best_b[depth].index,
                    second_cell: a,
                    second_index: index + 1,
                    cost: best_b[depth].cost + a_delta,
                },
            );
        }

        if (a_delta, index) < (best_a[depth].cost, best_a[depth].index) {
            best_a[depth] = Candidate {
                cost: a_delta,
                index,
            };
        }
        if (b_delta, index) < (best_b[depth].cost, best_b[depth].index) {
            best_b[depth] = Candidate {
                cost: b_delta,
                index,
            };
        }

        if index < state.len() {
            let v = input.card(state[index]);
            count[v] += 1;
            if count[v] == 1 {
                best_a.push(Candidate::none());
                best_b.push(Candidate::none());
            } else {
                debug_assert_eq!(count[v], 2);
                best_a.pop();
                best_b.pop();
            }
        }
    }

    best
}

fn apply_insertion(state: &mut Vec<usize>, insertion: Insertion) {
    debug_assert!(insertion.first_index <= state.len());
    state.insert(insertion.first_index, insertion.first_cell);
    debug_assert!(insertion.second_index <= state.len());
    state.insert(insertion.second_index, insertion.second_cell);
}

fn greedy_from_null(input: &Input) -> (Vec<usize>, usize) {
    let mut state = Vec::with_capacity(NN);
    let mut used = [false; M];
    let mut greedy_insert_dist = 0usize;

    for _ in 0..M {
        let mut best_v = INVALID;
        let mut best_insertion = Insertion::none();

        for v in 0..M {
            if used[v] {
                continue;
            }
            let (a, b) = input.pair(v);
            let insertion = best_insert_positions(input, &state, a, b);
            if (
                insertion.cost,
                v,
                insertion.first_index,
                insertion.second_index,
                insertion.first_cell,
                insertion.second_cell,
            ) < (
                best_insertion.cost,
                best_v,
                best_insertion.first_index,
                best_insertion.second_index,
                best_insertion.first_cell,
                best_insertion.second_cell,
            ) {
                best_v = v;
                best_insertion = insertion;
            }
        }

        debug_assert_ne!(best_v, INVALID);
        greedy_insert_dist += best_insertion.cost;
        apply_insertion(&mut state, best_insertion);
        used[best_v] = true;
    }

    (state, greedy_insert_dist)
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

#[cfg(feature = "local")]
fn verify_state(input: &Input, state: &[usize]) {
    let mut seen_cell = [false; NN];
    let mut stack = Vec::with_capacity(NN);
    let mut removed = 0usize;

    for &id in state {
        debug_assert!(!seen_cell[id]);
        seen_cell[id] = true;
        let v = input.card(id);
        stack.push(v);
        let len = stack.len();
        if len >= 2 && stack[len - 1] == stack[len - 2] {
            stack.pop();
            stack.pop();
            removed += 2;
        }
    }

    assert!(stack.is_empty());
    assert_eq!(removed, state.len());
}

fn solve(input: &Input) -> Vec<u8> {
    let (state, greedy_insert_dist) = greedy_from_null(input);
    local! {
        verify_state(input, &state);
    }

    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;

    for &id in &state {
        move_count += move_to(&mut cur, id, &mut ops);
        push_op(&mut ops, b'Z');
    }

    local! {
        debug_assert_eq!(move_count, greedy_insert_dist);
        let trace = TraceStats {
            greedy_insert_dist,
            move_count,
            turn_count: ops.len() / 2,
        };
        debug_assert!(trace.turn_count <= MAX_T);
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
