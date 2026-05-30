// v000_template.rs
use std::io::{self, Read, Write};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const M: usize = NN / 2;
const MAX_T: usize = 2 * N * N * N;
const EMPTY: usize = usize::MAX;
const INVALID: usize = usize::MAX;
const NEXT: [[usize; 4]; NN] = build_next();
const DIST: [[u16; NN]; NN] = build_dist();

const fn build_next() -> [[usize; 4]; NN] {
    let mut next = [[INVALID; 4]; NN];
    let mut id = 0;
    while id < NN {
        let i = id / N;
        let j = id % N;
        if i > 0 {
            next[id][0] = (i - 1) * N + j;
        }
        if i + 1 < N {
            next[id][1] = (i + 1) * N + j;
        }
        if j > 0 {
            next[id][2] = i * N + (j - 1);
        }
        if j + 1 < N {
            next[id][3] = i * N + (j + 1);
        }
        id += 1;
    }
    next
}

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
    fn id(i: usize, j: usize) -> usize {
        i * N + j
    }

    #[inline(always)]
    fn ij(id: usize) -> (usize, usize) {
        (id / N, id % N)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    U, // 1つ上のマスへ移動する
    D, // 1つ下のマスへ移動する
    L, // 1つ左のマスへ移動する
    R, // 1つ右のマスへ移動する
    Z, // 現在位置のカードを山札の一番上に取る
    X, // 山札の一番上のカードを現在位置に置く
}

impl Op {
    #[inline(always)]
    fn as_char(self) -> char {
        match self {
            Op::U => 'U',
            Op::D => 'D',
            Op::L => 'L',
            Op::R => 'R',
            Op::Z => 'Z',
            Op::X => 'X',
        }
    }

    #[inline(always)]
    fn is_move(self) -> bool {
        matches!(self, Op::U | Op::D | Op::L | Op::R)
    }
}

#[derive(Debug, Clone, Default)]
struct Output {
    ops: Vec<Op>,
}

impl Output {
    fn new() -> Self {
        Self { ops: Vec::new() }
    }

    fn push(&mut self, op: Op) {
        self.ops.push(op);
    }

    fn len(&self) -> usize {
        self.ops.len()
    }

    fn move_count(&self) -> usize {
        self.ops.iter().filter(|&&op| op.is_move()).count()
    }

    fn print(&self) {
        let stdout = io::stdout();
        let mut out = io::BufWriter::new(stdout.lock());
        for &op in &self.ops {
            writeln!(out, "{}", op.as_char()).unwrap();
        }
    }
}

#[derive(Debug, Clone)]
struct State {
    /// `board[id]` is the card number on the cell, or `EMPTY`.
    board: [usize; NN],
    /// Current cell id.
    cur: usize,
    /// Deck stack. The last element is the top card.
    deck: Vec<usize>,
    /// Number of cards remaining on the board and in the deck.
    remaining: usize,
    /// Number of move operations. This is `K` in the statement.
    move_count: usize,
    /// Number of all operations. This is `T` in the statement.
    turn_count: usize,
}

// Future: if duplicate-state pruning becomes necessary, add incremental Zobrist hashes for board, deck depth, and current cell.

impl State {
    fn new(input: &Input) -> Self {
        Self {
            board: input.a,
            cur: Input::id(0, 0),
            deck: Vec::with_capacity(NN),
            remaining: NN,
            move_count: 0,
            turn_count: 0,
        }
    }

    #[inline(always)]
    fn score(&self) -> i64 {
        if self.remaining == 0 {
            (NN + 2 * N * N * N - self.move_count) as i64
        } else {
            (NN - self.remaining) as i64
        }
    }

    #[inline(always)]
    fn is_done(&self) -> bool {
        self.remaining == 0
    }

    #[inline(always)]
    fn is_turn_limit_reached(&self) -> bool {
        self.turn_count >= MAX_T
    }

    #[inline(always)]
    fn current_cell(&self) -> usize {
        self.cur
    }

    #[inline(always)]
    fn current_card(&self) -> Option<usize> {
        let v = self.board[self.cur];
        if v == EMPTY { None } else { Some(v) }
    }

    #[inline(always)]
    fn deck_top(&self) -> Option<usize> {
        self.deck.last().copied()
    }

    #[inline(always)]
    fn can_take(&self) -> bool {
        self.current_card().is_some()
    }

    #[inline(always)]
    fn can_put(&self) -> bool {
        self.board[self.cur] == EMPTY && !self.deck.is_empty()
    }

    #[inline(always)]
    fn can_move(&self, op: Op) -> bool {
        match op {
            Op::U => NEXT[self.cur][0] != INVALID,
            Op::D => NEXT[self.cur][1] != INVALID,
            Op::L => NEXT[self.cur][2] != INVALID,
            Op::R => NEXT[self.cur][3] != INVALID,
            Op::Z | Op::X => false,
        }
    }

    fn apply(&mut self, op: Op) -> bool {
        if self.turn_count >= MAX_T {
            return false;
        }

        let ok = match op {
            Op::U => {
                let next = NEXT[self.cur][0];
                if next == INVALID {
                    false
                } else {
                    self.cur = next;
                    self.move_count += 1;
                    true
                }
            }
            Op::D => {
                let next = NEXT[self.cur][1];
                if next == INVALID {
                    false
                } else {
                    self.cur = next;
                    self.move_count += 1;
                    true
                }
            }
            Op::L => {
                let next = NEXT[self.cur][2];
                if next == INVALID {
                    false
                } else {
                    self.cur = next;
                    self.move_count += 1;
                    true
                }
            }
            Op::R => {
                let next = NEXT[self.cur][3];
                if next == INVALID {
                    false
                } else {
                    self.cur = next;
                    self.move_count += 1;
                    true
                }
            }
            Op::Z => {
                let v = self.board[self.cur];
                if v == EMPTY {
                    false
                } else {
                    self.board[self.cur] = EMPTY;
                    self.deck.push(v);

                    let len = self.deck.len();
                    if len >= 2 && self.deck[len - 1] == self.deck[len - 2] {
                        self.deck.pop();
                        self.deck.pop();
                        self.remaining -= 2;
                    }
                    true
                }
            }
            Op::X => {
                if self.board[self.cur] != EMPTY {
                    false
                } else if let Some(v) = self.deck.pop() {
                    self.board[self.cur] = v;
                    true
                } else {
                    false
                }
            }
        };

        if ok {
            self.turn_count += 1;
        }
        ok
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn mark_fallback(&mut self) {
        self.fallback_count += 1;
    }

    fn count(&mut self, key: &'static str) {
        self.count_by(key, 1);
    }

    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }

    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }

    fn summary(&self) {
        eprintln!("[summary] fallback_count={}", self.fallback_count);
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {}={}", key, value);
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {}={:.3}", key, value);
        }
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

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let __local_time_start = std::time::Instant::now();
        let __local_time_result = { $body };
        $trace.add_time_ms($key, __local_time_start.elapsed().as_secs_f64() * 1000.0);
        __local_time_result
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,

    iter: u64,
    check_mask: u64,

    elapsed_sec: f64,
    progress: f64,
    is_over: bool,
}

impl TimeKeeper {
    /// `check_interval_log2 = 8` なら 2^8 = 256 反復ごとに時計更新
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        assert!(time_limit_sec > 0.0);
        assert!(check_interval_log2 < 63);

        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };

        let mut tk = Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            elapsed_sec: 0.0,
            progress: 0.0,
            is_over: false,
        };
        tk.force_update();
        tk
    }

    /// ホットループではこれだけ呼ぶ
    /// true: 継続, false: 打ち切り
    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    /// 明示的に時計を更新したいときに使う
    #[inline(always)]
    fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    /// batched な経過時間
    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    /// batched な進捗率 [0, 1]
    #[inline(always)]
    fn progress(&self) -> f64 {
        self.progress
    }

    /// batched な時間切れ判定
    #[inline(always)]
    fn is_time_over(&self) -> bool {
        self.is_over
    }

    /// ログ用の正確な経過時間
    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// ログ用の正確な残り時間
    #[inline]
    fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn main() {}
