// v005_beam_lookahead.rs
#![allow(dead_code)]
// rollback beam + greedy 1 手先読み実験版。
// Action は「v の上の塊を複数 move に分割してから v を carry」する macro action。
// 分割候補は v002 と同じ near-lexico スカラー評価で小さいローカル beam を走らせて作る。
// 各 child の beam 選別には、child から v002 greedy action を 1 手だけ仮適用した評価を使う。
use proconio::input;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 箱の総数。全テストケースで固定。
const N: usize = 200;
/// 山の本数。全テストケースで固定。
const M: usize = 10;
/// 各山の初期高さ。`N / M` で固定。
const PER_PILE: usize = N / M;

/// 箱番号 (0-based, `0..N`)。`N <= 256` なので `u8` に収まる。
type BoxId = u8;

/// 1 操作の内部表現。`(v, i)` 形式。
/// - 操作1 (移動): `i < M` で「箱 `v` を山 `i` へ移動」。
/// - 操作2 (運び出し): `i = CARRY_I` で「箱 `v` を運び出す」。
type Op = (BoxId, u8);

/// 操作2 を表す sentinel。出力時には `0` (1-based) に変換される。
const CARRY_I: u8 = u8::MAX;

/// 操作回数の上限。
const MAX_OPS: usize = 5000;

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

/// 初期入力。`b[i][j]` は山 `i` の下から `j` 番目の箱番号 (0-based)。
#[derive(Debug, Clone)]
struct Input {
    b: [[BoxId; PER_PILE]; M],
}

impl Input {
    fn read() -> Self {
        input! {
            _n: usize,
            _m: usize,
            rows: [[usize; PER_PILE]; M],
        }
        let mut b = [[0 as BoxId; PER_PILE]; M];
        for i in 0..M {
            for j in 0..PER_PILE {
                b[i][j] = (rows[i][j] - 1) as BoxId;
            }
        }
        Self { b }
    }
}

/// 各山の `Vec` 初期容量。最悪は N まで積めるが、まずは小さめで様子見。
const PILE_CAP: usize = N / 4;
/// 山の最悪高さ。Zobrist テーブルの j 軸サイズに使う。理論上の上限は N。
const PILE_MAX: usize = N;

/// Zobrist 用乱数テーブル。`z(v, i, j)` を一意な u64 に写す。
/// サイズは N × M × N × 8B ≈ 3.2 MB。初回呼び出し時に xorshift64 で初期化。
#[inline]
fn zob(v: BoxId, i: u8, j: u8) -> u64 {
    static TABLE: OnceLock<Vec<u64>> = OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut rng = 0xCAFEBABE_DEADBEEF_u64;
        let mut t = vec![0u64; N * M * PILE_MAX];
        for x in t.iter_mut() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            *x = rng;
        }
        t
    });
    t[(v as usize) * M * PILE_MAX + (i as usize) * PILE_MAX + (j as usize)]
}

/// macro action 内の 1 move。
/// `src` は Action 側で共通に持つ。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MoveStep {
    j: u8,
    dst: u8,
    len: u8,
}

/// 1 ターン分の macro action。
/// - moves[0..move_count] を順に適用して v の上を退避し、最後に v を carry する。
/// - `src`, `j`: v が運び出される直前 (= 元の) の山と高さ。逆操作の戻し先に使う。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Action {
    src: u8,
    j: u8,
    move_count: u8,
    moves: [MoveStep; MAX_MACRO_MOVES],
}

impl Default for Action {
    fn default() -> Self {
        Self {
            src: 0,
            j: 0,
            move_count: 0,
            moves: [MoveStep::default(); MAX_MACRO_MOVES],
        }
    }
}

/// 1 macro action に入る move 数の上限。正しさ優先で N まで許す。
const MAX_MACRO_MOVES: usize = N;

/// 1 node から展開する macro action 候補数。
const EXPAND_WIDTH: usize = 32;

const UNSET_DST: u8 = u8::MAX;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
struct MacroEval {
    score_key: u64,
}

#[derive(Clone, Copy, Debug)]
struct LocalMacroCandidate {
    eval: MacroEval,
    mins: [u8; M],
    dst_by_t: [u8; N],
    prev_dst: u8,
}

/// beam 内の優先度。小さいほど良い。
/// v002 と同じ near-lexico スカラー評価で状態を評価し、hash は安定化用。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
struct Evaluator {
    score_key: u64,
    total_cost: u32,
    tie_break: u32,
}

/// 盤面状態。`pile[i][j]` と `pos_i` / `pos_j` は常に同期する。
/// `v < next_v` (運び出し済み) の `pos_i[v]`, `pos_j[v]` は未定義 (参照しない)。
/// `hash` は現在 pile に積まれている全箱の `zob(v, i, j)` の XOR。
#[derive(Debug)]
struct State {
    pile: [Vec<BoxId>; M],
    pos_i: [u8; N],
    pos_j: [u8; N],
    next_v: BoxId,
    total_cost: u32,
    hash: u64,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut pile: [Vec<BoxId>; M] = core::array::from_fn(|_| Vec::with_capacity(PILE_CAP));
        let mut pos_i = [0u8; N];
        let mut pos_j = [0u8; N];
        let mut hash = 0u64;
        for i in 0..M {
            for j in 0..PER_PILE {
                let v = input.b[i][j];
                pile[i].push(v);
                pos_i[v as usize] = i as u8;
                pos_j[v as usize] = j as u8;
                hash ^= zob(v, i as u8, j as u8);
            }
        }
        Self {
            pile,
            pos_i,
            pos_j,
            next_v: 0,
            total_cost: 0,
            hash,
        }
    }

    /// src 山の j..end の塊を dst 山に append し、pos と hash を同期更新する。
    /// `cost` は呼び出し側で `+=` / `-=` する (forward/backward の対称性のため)。
    /// `src == dst` のときは呼び出してはならない (Action 列挙で除外している)。
    #[inline]
    fn move_block(&mut self, src: usize, j: usize, dst: usize) {
        let State {
            pile,
            pos_i,
            pos_j,
            hash,
            ..
        } = self;
        let [src_pile, dst_pile] = pile.get_disjoint_mut([src, dst]).unwrap();
        let base = dst_pile.len();
        let k = src_pile.len() - j;
        for off in 0..k {
            let u = src_pile[j + off];
            *hash ^= zob(u, src as u8, (j + off) as u8);
        }
        dst_pile.extend_from_slice(&src_pile[j..]);
        src_pile.truncate(j);
        for off in 0..k {
            let u = dst_pile[base + off];
            pos_i[u as usize] = dst as u8;
            pos_j[u as usize] = (base + off) as u8;
            *hash ^= zob(u, dst as u8, (base + off) as u8);
        }
    }

    #[inline]
    fn apply_macro_step(&mut self, src: u8, step: MoveStep) {
        self.move_block(src as usize, step.j as usize, step.dst as usize);
        self.total_cost += step.len as u32 + 1;
    }

    #[inline]
    fn undo_macro_step(&mut self, src: u8, step: MoveStep) {
        let dst_back_j = self.pile[step.dst as usize].len() - step.len as usize;
        self.move_block(step.dst as usize, dst_back_j, src as usize);
        self.total_cost -= step.len as u32 + 1;
    }

    #[inline]
    fn carry_forward(&mut self, src: u8, j: u8) {
        let v = self.next_v;
        self.pile[src as usize].pop();
        self.hash ^= zob(v, src, j);
        self.next_v += 1;
    }

    #[inline]
    fn carry_backward(&mut self, src: u8, j: u8) {
        self.next_v -= 1;
        let v = self.next_v;
        self.pile[src as usize].push(v);
        self.pos_i[v as usize] = src;
        self.pos_j[v as usize] = j;
        self.hash ^= zob(v, src, j);
    }

    /// 候補手の列挙。v の上の箱列を v002 と同じ near-lexico 評価で分割する。
    fn enumerate_actions(&self, out: &mut Vec<Action>) {
        out.clear();
        let v = self.next_v;
        let src = self.pos_i[v as usize];
        let j = self.pos_j[v as usize];
        let h = self.pile[src as usize].len();
        if j as usize + 1 == h {
            out.push(Action {
                src,
                j,
                ..Action::default()
            });
            return;
        }

        let mut init_mins = [EMPTY_MIN; M];
        for (i, pile) in self.pile.iter().enumerate() {
            init_mins[i] = pile_min(pile);
        }

        let mut current = Vec::with_capacity(EXPAND_WIDTH * M);
        current.push(LocalMacroCandidate {
            eval: MacroEval::default(),
            mins: init_mins,
            dst_by_t: [UNSET_DST; N],
            prev_dst: UNSET_DST,
        });
        let mut next = Vec::with_capacity(EXPAND_WIDTH * M);

        for t in (j as usize + 1..h).rev() {
            let u = self.pile[src as usize][t];
            next.clear();
            for cand in &current {
                for dst in 0..M as u8 {
                    if dst == src {
                        continue;
                    }
                    let m = cand.mins[dst as usize];
                    let mut eval = cand.eval;
                    if cand.prev_dst != dst {
                        eval.score_key += IMMEDIATE_SCORE;
                    }
                    let mut mins = cand.mins;
                    if u >= m {
                        eval.score_key += DOOMED_SCORE + URGENCY_SCORE * (EMPTY_MIN - m) as u64;
                    } else {
                        eval.score_key += SHELF_SCORE * (m - u) as u64;
                        mins[dst as usize] = u;
                    }
                    let mut dst_by_t = cand.dst_by_t;
                    dst_by_t[t] = dst;
                    next.push(LocalMacroCandidate {
                        eval,
                        mins,
                        dst_by_t,
                        prev_dst: dst,
                    });
                }
            }
            next.sort_unstable_by(|a, b| a.eval.cmp(&b.eval));
            next.truncate(EXPAND_WIDTH);
            std::mem::swap(&mut current, &mut next);
        }

        for cand in &current {
            let action = build_macro_action(src, j, h, &cand.dst_by_t);
            if !out.iter().any(|&existing| existing == action) {
                out.push(action);
            }
        }
    }

    /// v002 と同じ逐次 greedy で、次の 1 macro action だけを作る。
    fn greedy_action(&self) -> Action {
        let v = self.next_v;
        let src = self.pos_i[v as usize];
        let j = self.pos_j[v as usize];
        let h = self.pile[src as usize].len();
        if j as usize + 1 == h {
            return Action {
                src,
                j,
                ..Action::default()
            };
        }

        let mut mins = [EMPTY_MIN; M];
        for (i, pile) in self.pile.iter().enumerate() {
            mins[i] = pile_min(pile);
        }

        let mut dst_by_t = [UNSET_DST; N];
        for t in (j as usize + 1..h).rev() {
            let u = self.pile[src as usize][t];
            let prev = if t + 1 < h {
                dst_by_t[t + 1]
            } else {
                UNSET_DST
            };
            let mut best_dst = 0u8;
            let mut best_score = u64::MAX;
            for dst in 0..M as u8 {
                if dst == src {
                    continue;
                }
                let m = mins[dst as usize];
                let mut score = if prev == dst { 0 } else { IMMEDIATE_SCORE };
                if u >= m {
                    score += DOOMED_SCORE + URGENCY_SCORE * (EMPTY_MIN - m) as u64;
                } else {
                    score += SHELF_SCORE * (m - u) as u64;
                }
                if score < best_score {
                    best_score = score;
                    best_dst = dst;
                }
            }
            dst_by_t[t] = best_dst;
            if u < mins[best_dst as usize] {
                mins[best_dst as usize] = u;
            }
        }
        build_macro_action(src, j, h, &dst_by_t)
    }

    #[inline]
    fn move_forward(&mut self, action: Action) {
        for step in action.moves[..action.move_count as usize].iter().copied() {
            self.apply_macro_step(action.src, step);
        }
        self.carry_forward(action.src, action.j);
    }

    #[inline]
    fn move_backward(&mut self, action: Action) {
        self.carry_backward(action.src, action.j);
        for step in action.moves[..action.move_count as usize]
            .iter()
            .rev()
            .copied()
        {
            self.undo_macro_step(action.src, step);
        }
    }

    #[inline]
    fn evaluate(&self) -> Evaluator {
        let mut doomed_count = 0u32;
        let mut shelf_loss = 0u32;
        let mut urgency_sum = 0u32;
        for pile in &self.pile {
            let min_v = pile_min(pile);
            let urgency = (EMPTY_MIN - min_v) as u32;
            shelf_loss += urgency;
            let mut below_min = EMPTY_MIN;
            for &u in pile {
                if u > below_min {
                    doomed_count += 1;
                    urgency_sum += urgency;
                }
                if u < below_min {
                    below_min = u;
                }
            }
        }
        Evaluator {
            score_key: state_score_key(doomed_count, shelf_loss, urgency_sum, self.total_cost),
            total_cost: self.total_cost,
            tie_break: (self.hash & 0xFFFF_FFFF) as u32,
        }
    }

    #[inline]
    fn evaluate_after_greedy_lookahead(&mut self) -> Evaluator {
        if self.next_v as usize >= N {
            return self.evaluate();
        }
        let action = self.greedy_action();
        self.move_forward(action);
        let evaluator = self.evaluate();
        self.move_backward(action);
        evaluator
    }

    #[inline]
    fn hash_key(&self) -> u64 {
        self.hash
    }
}

/// 提出用の操作列。書き出し責務のみを持つ。
#[derive(Debug, Clone)]
struct Output {
    ops: Vec<Op>,
}

impl Output {
    fn write<W: Write>(&self, w: &mut W) {
        for &(v, i) in &self.ops {
            let v1 = (v as usize) + 1;
            if i == CARRY_I {
                writeln!(w, "{} 0", v1).unwrap();
            } else {
                writeln!(w, "{} {}", v1, (i as usize) + 1).unwrap();
            }
        }
    }
}

/// 空山の min を表す sentinel。全箱番号 (< N) より大きい。
const EMPTY_MIN: u8 = N as u8;

/// v002 の評価値を 2 倍して整数化したもの。
/// immediate 1.0 -> 2, ALPHA 1e7 -> 2e7, BETA 1000 -> 2000,
/// GAMMA / N = 300 / 200 = 1.5 -> 3。
const IMMEDIATE_SCORE: u64 = 2;
const DOOMED_SCORE: u64 = 20_000_000;
const SHELF_SCORE: u64 = 2_000;
const URGENCY_SCORE: u64 = 3;

#[inline]
fn pile_min(pile: &[BoxId]) -> u8 {
    pile.iter().copied().min().unwrap_or(EMPTY_MIN)
}

#[inline]
fn state_score_key(doomed_count: u32, shelf_loss: u32, urgency_sum: u32, total_cost: u32) -> u64 {
    DOOMED_SCORE * doomed_count as u64
        + SHELF_SCORE * shelf_loss as u64
        + URGENCY_SCORE * urgency_sum as u64
        + IMMEDIATE_SCORE * total_cost as u64
}

fn build_macro_action(src: u8, j: u8, h: usize, dst_by_t: &[u8; N]) -> Action {
    let mut action = Action {
        src,
        j,
        ..Action::default()
    };
    let mut t_hi = h;
    while t_hi > j as usize + 1 {
        let d = dst_by_t[t_hi - 1];
        debug_assert_ne!(d, UNSET_DST);
        let mut t_lo = t_hi - 1;
        while t_lo > j as usize + 1 && dst_by_t[t_lo - 1] == d {
            t_lo -= 1;
        }
        let move_index = action.move_count as usize;
        debug_assert!(move_index < MAX_MACRO_MOVES);
        action.moves[move_index] = MoveStep {
            j: t_lo as u8,
            dst: d,
            len: (t_hi - t_lo) as u8,
        };
        action.move_count += 1;
        t_hi = t_lo;
    }
    action
}

/// 各 depth で残す node 数。
const BEAM_WIDTH: usize = 128;

// ---- beam search 骨格 (Euler Tour 版テンプレートを問題に合わせて埋めたもの) ----

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CandidatePriority {
    rank_evaluator: Evaluator,
    evaluator: Evaluator,
}

impl Ord for CandidatePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank_evaluator
            .cmp(&other.rank_evaluator)
            .then_with(|| self.evaluator.cmp(&other.evaluator))
    }
}

impl PartialOrd for CandidatePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SlotValue {
    priority: CandidatePriority,
    index: usize,
}

impl Ord for SlotValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for SlotValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// beam 内で「最悪のスロット」を O(log W) で取り出すためのセグ木 (max)。
#[derive(Debug)]
struct MaxSegTree {
    size: usize,
    data: Vec<Option<SlotValue>>,
}

impl MaxSegTree {
    fn new(len: usize) -> Self {
        let size = len.max(1).next_power_of_two();
        Self {
            size,
            data: vec![None; size * 2],
        }
    }

    fn set(&mut self, mut index: usize, value: Option<SlotValue>) {
        index += self.size;
        self.data[index] = value;
        while index > 1 {
            index >>= 1;
            self.data[index] = self.data[index * 2].max(self.data[index * 2 + 1]);
        }
    }

    fn max_all(&self) -> Option<SlotValue> {
        self.data[1]
    }
}

#[derive(Clone, Copy, Debug)]
struct BeamCandidate {
    parent: usize,
    action: Action,
    evaluator: Evaluator,
    rank_evaluator: Evaluator,
    hash_key: u64,
}

impl BeamCandidate {
    fn priority(&self) -> CandidatePriority {
        CandidatePriority {
            rank_evaluator: self.rank_evaluator,
            evaluator: self.evaluator,
        }
    }
}

/// hash 重複排除しつつ上位 W 件を保持するためのセレクタ。
struct Selector {
    capacity: usize,
    candidates: Vec<Option<BeamCandidate>>,
    by_hash: HashMap<u64, usize>,
    worst: MaxSegTree,
}

impl Selector {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            candidates: Vec::with_capacity(capacity),
            by_hash: HashMap::with_capacity(capacity * 2 + 1),
            worst: MaxSegTree::new(capacity),
        }
    }

    fn clear(&mut self) {
        self.candidates.clear();
        self.by_hash.clear();
        self.worst = MaxSegTree::new(self.capacity);
    }

    fn push(&mut self, candidate: BeamCandidate) {
        if self.capacity == 0 {
            return;
        }
        if let Some(&index) = self.by_hash.get(&candidate.hash_key) {
            let current = self.candidates[index].expect("slot occupied");
            if candidate.priority() < current.priority() {
                self.candidates[index] = Some(candidate);
                self.worst.set(
                    index,
                    Some(SlotValue {
                        priority: candidate.priority(),
                        index,
                    }),
                );
            }
            return;
        }
        if self.candidates.len() < self.capacity {
            let index = self.candidates.len();
            self.candidates.push(Some(candidate));
            self.by_hash.insert(candidate.hash_key, index);
            self.worst.set(
                index,
                Some(SlotValue {
                    priority: candidate.priority(),
                    index,
                }),
            );
            return;
        }
        let Some(worst) = self.worst.max_all() else {
            return;
        };
        if candidate.priority() >= worst.priority {
            return;
        }
        let old = self.candidates[worst.index].expect("slot occupied");
        self.by_hash.remove(&old.hash_key);
        self.candidates[worst.index] = Some(candidate);
        self.by_hash.insert(candidate.hash_key, worst.index);
        self.worst.set(
            worst.index,
            Some(SlotValue {
                priority: candidate.priority(),
                index: worst.index,
            }),
        );
    }

    fn take_sorted(&mut self) -> Vec<BeamCandidate> {
        let mut result = self
            .candidates
            .iter()
            .filter_map(|c| *c)
            .collect::<Vec<_>>();
        result.sort_unstable_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .then_with(|| a.hash_key.cmp(&b.hash_key))
        });
        self.clear();
        result
    }
}

#[derive(Clone, Debug)]
struct BeamNode {
    parent: usize,
    action: Option<Action>,
    evaluator: Evaluator,
    rank_evaluator: Evaluator,
    depth: usize,
}

#[derive(Clone, Copy, Debug)]
enum TourEdge {
    Forward(usize),
    Backward(usize),
    Visit(usize),
}

fn path_from_root(nodes: &[BeamNode], mut node: usize) -> Vec<usize> {
    let mut path = Vec::new();
    while node != 0 {
        path.push(node);
        node = nodes[node].parent;
    }
    path.reverse();
    path
}

fn build_tour_edges(leaves: &[usize], nodes: &[BeamNode]) -> Vec<TourEdge> {
    let mut edges = Vec::new();
    let mut previous_path = Vec::<usize>::new();
    for &leaf in leaves {
        let path = path_from_root(nodes, leaf);
        let mut lcp = 0;
        while lcp < previous_path.len() && lcp < path.len() && previous_path[lcp] == path[lcp] {
            lcp += 1;
        }
        for &node in previous_path[lcp..].iter().rev() {
            edges.push(TourEdge::Backward(node));
        }
        for &node in &path[lcp..] {
            edges.push(TourEdge::Forward(node));
        }
        edges.push(TourEdge::Visit(leaf));
        previous_path = path;
    }
    for &node in previous_path.iter().rev() {
        edges.push(TourEdge::Backward(node));
    }
    edges
}

fn reconstruct_actions(nodes: &[BeamNode], mut node: usize) -> Vec<Action> {
    let mut actions = Vec::new();
    while node != 0 {
        actions.push(nodes[node].action.expect("non-root has action"));
        node = nodes[node].parent;
    }
    actions.reverse();
    actions
}

fn rollback_beam_search(state: &mut State, max_turn: usize, beam_width: usize) -> Vec<Action> {
    let root_evaluator = state.evaluate();
    let root = BeamNode {
        parent: 0,
        action: None,
        evaluator: root_evaluator,
        rank_evaluator: root_evaluator,
        depth: 0,
    };
    let mut nodes = vec![root];
    let mut beam = vec![0_usize];
    let mut selector = Selector::new(beam_width);
    let mut actions_buf: Vec<Action> = Vec::with_capacity(M);
    for _turn in 0..max_turn {
        selector.clear();
        let tour_edges = build_tour_edges(&beam, &nodes);
        for edge in tour_edges {
            match edge {
                TourEdge::Forward(node) => {
                    state.move_forward(nodes[node].action.expect("root never forwarded"));
                }
                TourEdge::Backward(node) => {
                    state.move_backward(nodes[node].action.expect("root never rolled back"));
                }
                TourEdge::Visit(parent) => {
                    state.enumerate_actions(&mut actions_buf);
                    for &action in &actions_buf {
                        state.move_forward(action);
                        let evaluator = state.evaluate();
                        let rank_evaluator = state.evaluate_after_greedy_lookahead();
                        selector.push(BeamCandidate {
                            parent,
                            action,
                            evaluator,
                            rank_evaluator,
                            hash_key: state.hash_key(),
                        });
                        state.move_backward(action);
                    }
                }
            }
        }
        let selected = selector.take_sorted();
        if selected.is_empty() {
            break;
        }
        beam.clear();
        for candidate in selected {
            let depth = nodes[candidate.parent].depth + 1;
            nodes.push(BeamNode {
                parent: candidate.parent,
                action: Some(candidate.action),
                evaluator: candidate.evaluator,
                rank_evaluator: candidate.rank_evaluator,
                depth,
            });
            beam.push(nodes.len() - 1);
        }
    }
    let best_leaf = beam
        .iter()
        .copied()
        .min_by(|&a, &b| {
            nodes[a]
                .evaluator
                .total_cost
                .cmp(&nodes[b].evaluator.total_cost)
                .then_with(|| nodes[a].evaluator.cmp(&nodes[b].evaluator))
        })
        .unwrap_or(0);
    reconstruct_actions(&nodes, best_leaf)
}

/// Action 列を新しい State にリプレイして Op 列に展開する。
/// 各 Action は複数 move + 1 carry になる。
fn actions_to_output(input: &Input, actions: &[Action]) -> (Output, u32) {
    let mut state = State::new(input);
    let mut ops: Vec<Op> = Vec::with_capacity(MAX_OPS);
    for &action in actions {
        let v = state.next_v;
        for step in action.moves[..action.move_count as usize].iter().copied() {
            let bottom = state.pile[action.src as usize][step.j as usize];
            ops.push((bottom, step.dst));
            state.apply_macro_step(action.src, step);
        }
        ops.push((v, CARRY_I));
        state.carry_forward(action.src, action.j);
    }
    let total_cost = state.total_cost;
    (Output { ops }, total_cost)
}

/// `move_forward(a)` → `move_backward(a)` が完全に逆操作になっていることを 1 ターン分だけ検証する。
#[cfg(feature = "local")]
fn debug_check_inverse(state: &mut State) {
    let snap_cost = state.total_cost;
    let snap_hash = state.hash;
    let snap_next = state.next_v;
    let snap_lens: [usize; M] = std::array::from_fn(|i| state.pile[i].len());
    let mut buf: Vec<Action> = Vec::new();
    state.enumerate_actions(&mut buf);
    for action in buf {
        state.move_forward(action);
        state.move_backward(action);
        assert_eq!(state.total_cost, snap_cost, "total_cost broken: {action:?}");
        assert_eq!(state.hash, snap_hash, "hash broken: {action:?}");
        assert_eq!(state.next_v, snap_next, "next_v broken: {action:?}");
        for i in 0..M {
            assert_eq!(state.pile[i].len(), snap_lens[i], "pile[{i}] len broken");
        }
    }
}

fn main() {
    // TimeKeeper は main 開始直後に作り、探索打ち切りには PROGRAM_TIME_LIMIT_SEC を使う。
    let _time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
    let input = Input::read();
    let mut state = State::new(&input);
    #[cfg(feature = "local")]
    debug_check_inverse(&mut state);
    let actions = rollback_beam_search(&mut state, N, BEAM_WIDTH);
    let (output, total_cost) = actions_to_output(&input, &actions);
    #[cfg(not(feature = "local"))]
    let _ = total_cost;
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    output.write(&mut writer);
    writer.flush().unwrap();
    local! {
        eprintln!("[summary.count] ops={}", output.ops.len());
        eprintln!("[summary.count] actions={}", actions.len());
        eprintln!("[summary.count] beam_width={}", BEAM_WIDTH);
        eprintln!("[summary.count] expand_width={}", EXPAND_WIDTH);
        eprintln!("[summary.count] rank_lookahead=greedy1");
        eprintln!("[summary.count] total_cost={}", total_cost);
    }
}
