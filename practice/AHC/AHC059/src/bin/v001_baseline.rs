// v001_baseline.rs
use std::io::{self, Read, Write};

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    move_count: usize,
    turn_count: usize,
    pair_count: usize,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn summary(&self) {
        eprintln!(
            "[summary] pairs={} moves={} turns={} score_est={}",
            self.pair_count,
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

#[inline(always)]
fn dist(p: usize, q: usize) -> usize {
    let pi = p / N;
    let pj = p % N;
    let qi = q / N;
    let qj = q % N;
    pi.abs_diff(qi) + pj.abs_diff(qj)
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
    let mut remaining = [true; M];
    let mut cur = 0usize;
    let mut ops = Vec::with_capacity(MAX_T * 2);
    let mut move_count = 0usize;

    for _ in 0..M {
        let mut best_v = usize::MAX;
        let mut best_near = usize::MAX;
        let mut best_far = usize::MAX;
        let mut best_cur_dist = usize::MAX;
        let mut best_pair_dist = usize::MAX;

        for v in 0..M {
            if !remaining[v] {
                continue;
            }
            let p0 = input.pos[v][0];
            let p1 = input.pos[v][1];
            let d0 = dist(cur, p0);
            let d1 = dist(cur, p1);
            let (near, far, cur_dist) = if d0 < d1 || (d0 == d1 && p0 < p1) {
                (p0, p1, d0)
            } else {
                (p1, p0, d1)
            };
            let pair_dist = dist(near, far);

            if cur_dist < best_cur_dist
                || (cur_dist == best_cur_dist
                    && (pair_dist, near, far, v) < (best_pair_dist, best_near, best_far, best_v))
            {
                best_v = v;
                best_near = near;
                best_far = far;
                best_cur_dist = cur_dist;
                best_pair_dist = pair_dist;
            }
        }

        move_count += move_to(&mut cur, best_near, &mut ops);
        push_op(&mut ops, b'Z');
        move_count += move_to(&mut cur, best_far, &mut ops);
        push_op(&mut ops, b'Z');
        remaining[best_v] = false;
    }

    local! {
        let trace = TraceStats {
            move_count,
            turn_count: ops.len() / 2,
            pair_count: M,
        };
        assert!(trace.turn_count <= MAX_T);
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
