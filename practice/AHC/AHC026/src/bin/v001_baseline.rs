// v001_baseline.rs
// ベースライン: 各 v について「v の上の塊を隣の山へ 1 回で移動 → v を運び出す」を繰り返す。
use proconio::input;
use std::io::Write;
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

/// 盤面状態。`pile[i][j]` と `pos_i` / `pos_j` は常に同期する。
/// `v < next_v` (運び出し済み) の `pos_i[v]`, `pos_j[v]` は未定義 (参照しない)。
#[derive(Debug, Clone)]
struct State {
    pile: [Vec<BoxId>; M],
    pos_i: [u8; N],
    pos_j: [u8; N],
    next_v: BoxId,
    total_cost: u32,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut pile: [Vec<BoxId>; M] = core::array::from_fn(|_| Vec::with_capacity(PILE_CAP));
        let mut pos_i = [0u8; N];
        let mut pos_j = [0u8; N];
        for i in 0..M {
            for j in 0..PER_PILE {
                let v = input.b[i][j];
                pile[i].push(v);
                pos_i[v as usize] = i as u8;
                pos_j[v as usize] = j as u8;
            }
        }
        Self {
            pile,
            pos_i,
            pos_j,
            next_v: 0,
            total_cost: 0,
        }
    }

    /// 操作1: 箱 `v` とその上の全箱を山 `i` へ移動する。
    /// `i == pos_i[v]` の場合は状態を変えず体力 `k + 1` だけ消費する。
    #[inline]
    fn apply_move(&mut self, v: BoxId, i: u8) {
        let src = self.pos_i[v as usize] as usize;
        let j = self.pos_j[v as usize] as usize;
        let k = self.pile[src].len() - j;
        self.total_cost += (k + 1) as u32;
        let dst = i as usize;
        if src == dst {
            return;
        }
        let [src_pile, dst_pile] = self.pile.get_disjoint_mut([src, dst]).unwrap();
        let base = dst_pile.len();
        dst_pile.extend_from_slice(&src_pile[j..]);
        src_pile.truncate(j);
        for (off, &u) in dst_pile[base..].iter().enumerate() {
            self.pos_i[u as usize] = i;
            self.pos_j[u as usize] = (base + off) as u8;
        }
    }

    /// 操作2: 箱 `v` を運び出す。`v == next_v` かつ山頂にいる前提。
    #[inline]
    fn apply_carry(&mut self, v: BoxId) {
        self.pile[self.pos_i[v as usize] as usize].pop();
        self.next_v += 1;
    }

    /// `Op` 列のリプレイ用ディスパッチ。hot path では `apply_move` / `apply_carry` を直接呼ぶ。
    #[inline]
    fn apply(&mut self, op: Op) {
        let (v, i) = op;
        if i == CARRY_I {
            self.apply_carry(v);
        } else {
            self.apply_move(v, i);
        }
    }

    /// 全箱を運び出し終えたか。
    #[inline]
    fn is_done(&self) -> bool {
        self.next_v as usize == N
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

/// ベースライン解法。v = 0..N の順に、v の上の塊を隣の山へ 1 回で移してから v を運び出す。
/// 操作回数は高々 2N = 400 <= MAX_OPS。
fn solve(state: &mut State) -> Output {
    let mut ops: Vec<Op> = Vec::with_capacity(2 * N);
    for v in 0..N as BoxId {
        let src = state.pos_i[v as usize];
        let j = state.pos_j[v as usize] as usize;
        if j + 1 < state.pile[src as usize].len() {
            let above = state.pile[src as usize][j + 1];
            let dst = if src as usize + 1 == M { 0 } else { src + 1 };
            state.apply_move(above, dst);
            ops.push((above, dst));
        }
        state.apply_carry(v);
        ops.push((v, CARRY_I));
    }
    Output { ops }
}

fn main() {
    // TimeKeeper は main 開始直後に作り、探索打ち切りには PROGRAM_TIME_LIMIT_SEC を使う。
    // フェーズ切替などの時間系パラメータは PROGRAM_TIME_LIMIT_SEC に対する割合で指定する。
    let _time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
    let input = Input::read();
    let mut state = State::new(&input);
    let output = solve(&mut state);
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    output.write(&mut writer);
    writer.flush().unwrap();
    local! {
        eprintln!("[summary.count] ops={}", output.ops.len());
        eprintln!("[summary.count] total_cost={}", state.total_cost);
    }
}
