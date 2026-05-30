// stack_bench.rs
use std::hint::black_box;
use std::time::{Duration, Instant};

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const EMPTY: usize = usize::MAX;

#[derive(Clone)]
struct FixedStack {
    data: [usize; NN],
    len: usize,
}

impl FixedStack {
    fn new() -> Self {
        Self {
            data: [0; NN],
            len: 0,
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    fn push(&mut self, v: usize) {
        self.data[self.len] = v;
        self.len += 1;
    }

    #[inline(always)]
    fn pop(&mut self) -> usize {
        self.len -= 1;
        self.data[self.len]
    }

    #[inline(always)]
    fn top2_equal(&self) -> bool {
        self.len >= 2 && self.data[self.len - 1] == self.data[self.len - 2]
    }

    #[inline(always)]
    fn pop2(&mut self) {
        self.len -= 2;
    }

    #[inline(always)]
    fn checksum(&self) -> usize {
        let mut acc = self.len;
        for i in 0..self.len {
            acc = acc.wrapping_mul(1_000_003).wrapping_add(self.data[i]);
        }
        acc
    }
}

#[derive(Clone)]
struct VecStack {
    data: Vec<usize>,
}

impl VecStack {
    fn new() -> Self {
        Self {
            data: Vec::with_capacity(NN),
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    fn push(&mut self, v: usize) {
        self.data.push(v);
    }

    #[inline(always)]
    fn pop(&mut self) -> usize {
        self.data.pop().unwrap()
    }

    #[inline(always)]
    fn top2_equal(&self) -> bool {
        let len = self.data.len();
        len >= 2 && self.data[len - 1] == self.data[len - 2]
    }

    #[inline(always)]
    fn pop2(&mut self) {
        self.data.pop();
        self.data.pop();
    }

    #[inline(always)]
    fn checksum(&self) -> usize {
        let mut acc = self.data.len();
        for &v in &self.data {
            acc = acc.wrapping_mul(1_000_003).wrapping_add(v);
        }
        acc
    }
}

#[derive(Clone)]
struct FixedState {
    board: [usize; NN],
    cur: usize,
    deck: FixedStack,
    remaining: usize,
    move_count: usize,
    turn_count: usize,
}

#[derive(Clone)]
struct VecState {
    board: [usize; NN],
    cur: usize,
    deck: VecStack,
    remaining: usize,
    move_count: usize,
    turn_count: usize,
}

#[derive(Clone, Copy)]
enum StackOp {
    Push(usize),
    Pop,
}

struct BenchResult {
    name: &'static str,
    fixed: Duration,
    vec: Duration,
    checksum_fixed: usize,
    checksum_vec: usize,
}

fn main() {
    let mut results = Vec::new();
    results.push(bench_pair_cancel());
    results.push(bench_deep_cycle());
    results.push(bench_mixed_ops());
    results.push(bench_state_clone_shallow());
    results.push(bench_state_clone_deep());

    println!("scenario,fixed_ms,vec_ms,fixed/vec,checksum_match");
    for result in results {
        let fixed_ms = result.fixed.as_secs_f64() * 1000.0;
        let vec_ms = result.vec.as_secs_f64() * 1000.0;
        println!(
            "{},{:.3},{:.3},{:.3},{}",
            result.name,
            fixed_ms,
            vec_ms,
            fixed_ms / vec_ms,
            result.checksum_fixed == result.checksum_vec
        );
    }
}

fn bench_pair_cancel() -> BenchResult {
    let seq: Vec<usize> = (0..M).cycle().take(20_000_000).collect();
    let fixed = bench_fixed("pair_cancel", |stack, sink| {
        for &v in &seq {
            stack.push(v);
            stack.push(v);
            if stack.top2_equal() {
                stack.pop2();
            }
            *sink = sink.wrapping_add(stack.len());
        }
    });
    let vec = bench_vec("pair_cancel", |stack, sink| {
        for &v in &seq {
            stack.push(v);
            stack.push(v);
            if stack.top2_equal() {
                stack.pop2();
            }
            *sink = sink.wrapping_add(stack.len());
        }
    });
    result("pair_cancel", fixed, vec)
}

fn bench_deep_cycle() -> BenchResult {
    let reps = 200_000;
    let fixed = bench_fixed("deep_cycle", |stack, sink| {
        for r in 0..reps {
            for i in 0..NN {
                stack.push((r + i) % M);
                *sink = sink.wrapping_add(stack.len());
            }
            for _ in 0..NN {
                *sink = sink.wrapping_add(stack.pop());
            }
        }
    });
    let vec = bench_vec("deep_cycle", |stack, sink| {
        for r in 0..reps {
            for i in 0..NN {
                stack.push((r + i) % M);
                *sink = sink.wrapping_add(stack.len());
            }
            for _ in 0..NN {
                *sink = sink.wrapping_add(stack.pop());
            }
        }
    });
    result("deep_cycle", fixed, vec)
}

fn bench_mixed_ops() -> BenchResult {
    let ops = make_mixed_ops(20_000_000);
    let fixed = bench_fixed("mixed_ops", |stack, sink| {
        for &op in &ops {
            match op {
                StackOp::Push(v) => {
                    stack.push(v);
                    if stack.top2_equal() {
                        stack.pop2();
                    }
                }
                StackOp::Pop => {
                    *sink = sink.wrapping_add(stack.pop());
                }
            }
            *sink = sink.wrapping_add(stack.len());
        }
    });
    let vec = bench_vec("mixed_ops", |stack, sink| {
        for &op in &ops {
            match op {
                StackOp::Push(v) => {
                    stack.push(v);
                    if stack.top2_equal() {
                        stack.pop2();
                    }
                }
                StackOp::Pop => {
                    *sink = sink.wrapping_add(stack.pop());
                }
            }
            *sink = sink.wrapping_add(stack.len());
        }
    });
    result("mixed_ops", fixed, vec)
}

fn bench_state_clone_shallow() -> BenchResult {
    bench_state_clone("state_clone_shallow", 8, 2_000_000)
}

fn bench_state_clone_deep() -> BenchResult {
    bench_state_clone("state_clone_deep", NN, 2_000_000)
}

fn bench_state_clone(name: &'static str, deck_len: usize, reps: usize) -> BenchResult {
    let mut fixed_state = FixedState {
        board: [EMPTY; NN],
        cur: 123,
        deck: FixedStack::new(),
        remaining: NN,
        move_count: 456,
        turn_count: 789,
    };
    let mut vec_state = VecState {
        board: [EMPTY; NN],
        cur: fixed_state.cur,
        deck: VecStack::new(),
        remaining: fixed_state.remaining,
        move_count: fixed_state.move_count,
        turn_count: fixed_state.turn_count,
    };
    for i in 0..NN {
        fixed_state.board[i] = i % M;
        vec_state.board[i] = i % M;
    }
    for i in 0..deck_len {
        fixed_state.deck.push(i % M);
        vec_state.deck.push(i % M);
    }

    let mut fixed_checksum = 0usize;
    let fixed_start = Instant::now();
    for r in 0..reps {
        let mut cloned = black_box(fixed_state.clone());
        cloned.cur ^= r & 31;
        fixed_checksum = fixed_checksum.wrapping_add(cloned.cur);
        fixed_checksum = fixed_checksum.wrapping_add(cloned.deck.checksum());
        fixed_checksum = fixed_checksum.wrapping_add(cloned.board[r % NN]);
        fixed_checksum = fixed_checksum.wrapping_add(cloned.remaining);
        fixed_checksum = fixed_checksum.wrapping_add(cloned.move_count);
        fixed_checksum = fixed_checksum.wrapping_add(cloned.turn_count);
    }
    let fixed_duration = fixed_start.elapsed();

    let mut vec_checksum = 0usize;
    let vec_start = Instant::now();
    for r in 0..reps {
        let mut cloned = black_box(vec_state.clone());
        cloned.cur ^= r & 31;
        vec_checksum = vec_checksum.wrapping_add(cloned.cur);
        vec_checksum = vec_checksum.wrapping_add(cloned.deck.checksum());
        vec_checksum = vec_checksum.wrapping_add(cloned.board[r % NN]);
        vec_checksum = vec_checksum.wrapping_add(cloned.remaining);
        vec_checksum = vec_checksum.wrapping_add(cloned.move_count);
        vec_checksum = vec_checksum.wrapping_add(cloned.turn_count);
    }
    let vec_duration = vec_start.elapsed();

    BenchResult {
        name,
        fixed: fixed_duration,
        vec: vec_duration,
        checksum_fixed: fixed_checksum,
        checksum_vec: vec_checksum,
    }
}

fn bench_fixed<F>(name: &'static str, f: F) -> (Duration, usize)
where
    F: FnOnce(&mut FixedStack, &mut usize),
{
    let mut stack = FixedStack::new();
    let mut sink = name.len();
    let start = Instant::now();
    f(black_box(&mut stack), black_box(&mut sink));
    let duration = start.elapsed();
    (duration, sink.wrapping_add(stack.checksum()))
}

fn bench_vec<F>(name: &'static str, f: F) -> (Duration, usize)
where
    F: FnOnce(&mut VecStack, &mut usize),
{
    let mut stack = VecStack::new();
    let mut sink = name.len();
    let start = Instant::now();
    f(black_box(&mut stack), black_box(&mut sink));
    let duration = start.elapsed();
    (duration, sink.wrapping_add(stack.checksum()))
}

fn result(name: &'static str, fixed: (Duration, usize), vec: (Duration, usize)) -> BenchResult {
    BenchResult {
        name,
        fixed: fixed.0,
        vec: vec.0,
        checksum_fixed: fixed.1,
        checksum_vec: vec.1,
    }
}

fn make_mixed_ops(len: usize) -> Vec<StackOp> {
    let mut ops = Vec::with_capacity(len);
    let mut rng = XorShift64::new(0x1234_5678_9abc_def0);
    let mut simulated = Vec::with_capacity(NN);
    while ops.len() < len {
        let push = if simulated.is_empty() {
            true
        } else if simulated.len() >= NN {
            false
        } else {
            rng.next_usize(100) < 58
        };
        if push {
            let value = if !simulated.is_empty() && rng.next_usize(20) == 0 {
                simulated[simulated.len() - 1]
            } else {
                rng.next_usize(M)
            };
            ops.push(StackOp::Push(value));
            simulated.push(value);
            if simulated.len() >= 2
                && simulated[simulated.len() - 1] == simulated[simulated.len() - 2]
            {
                simulated.pop();
                simulated.pop();
            }
        } else {
            ops.push(StackOp::Pop);
            simulated.pop();
        }
    }
    ops
}

struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { x: seed }
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    #[inline(always)]
    fn next_usize(&mut self, modulo: usize) -> usize {
        (self.next() as usize) % modulo
    }
}
