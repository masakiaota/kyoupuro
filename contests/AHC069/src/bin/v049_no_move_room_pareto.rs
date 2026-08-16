// v049_no_move_room_pareto.rs
#![allow(non_snake_case)] // 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。

// 中心アイデア: 既存rolloutと連結room評価を分離し、現在の連結性を落とさず
// 将来roomを明確に広げる場合だけ保守的に反転する。再移動は一切行わない。

use statrs::function::erf::erfc;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::io::{self, BufRead, BufWriter, Write};
use std::time::Instant;

const MAX_N: usize = 50;
const MAX_P: usize = 150;
const HORIZON: usize = 100_000;
type Rows = [u64; MAX_N];
type RunTable = [[u64; MAX_N + 1]; MAX_N];

/// AtCoder 側の基準の探索打ち切り秒数。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

// 再移動を削った時間を通常配置の探索へ回し、終盤だけ安全優先へ切り替える。
const FAST_MODE_RATIO: f64 = 0.94;
const GROWTH_LIMIT_RATIO: f64 = 0.955;
/// SA 自体は早めに閉じ、以降のターン処理と出力に local 約150msを残す。
const GROWTH_SA_LIMIT_RATIO: f64 = 0.90;
// 初版の3候補×768反復は平均1.27秒を使って後半の配置判断を圧迫した。
// 周長上位へ集中し、改善量を残しながら通常経路の時間を確保する。
const GROWTH_SA_CANDIDATES: usize = 2;
const GROWTH_SA_ITERATIONS: usize = 512;
/// SA後の追加LNSは小予算に絞り、通常経路へ入る前に打ち切る。
const GROWTH_LNS_LIMIT_RATIO: f64 = 0.92;
const GROWTH_LNS_ATTEMPTS_LARGE: usize = 40;
const GROWTH_LNS_ATTEMPTS_MID: usize = 8;
/// 利用料回収余地が小さい候補には探索時間を使わない。
const GROWTH_LNS_MIN_RECOVERABLE_FEE: f64 = 100_000.0;
const GROWTH_LNS_BATCHES: [usize; 3] = [4, 8, 16];
/// admission と winner 選択を終えた growth 候補だけに使う後処理予算。
const BIASED_SWAP_START_RATIO: f64 = 0.86;
const BIASED_SWAP_LIMIT_RATIO: f64 = 0.90;
const BIASED_SWAP_ITERATIONS: usize = 512;
const BIASED_SWAP_MIN_RECOVERABLE_FEE: f64 = 300_000.0;
const NO_MOVE_CAPACITY_RATIO: f64 = 0.975;
const CAUSAL_VETO_MARGIN_MAX: f64 = 1.13;
const CAUSAL_VETO_DURATION_RATIO_MAX: f64 = 2.0;
const CAUSAL_VETO_SLACK_MAX: usize = 14;
/// release atlas は通常探索を圧迫しない時間帯だけ作る。
const RELEASE_ATLAS_LIMIT_RATIO: f64 = 0.82;
const RELEASE_ATLAS_P_MAX: usize = 95;
const RELEASE_ATLAS_D_MIN: usize = 3_000;
const RELEASE_ATLAS_BUCKETS: usize = 5;
const RELEASE_ATLAS_SNAPSHOTS: usize = 3;

// ---- ロールアウト評価のパラメータ ----
/// 1 本のロールアウトで見る将来到着数。盤面差が効くのは空きが一巡する θ 程度の
/// 時間幅であり、平均到着間隔 ~100 に対し 22 件 ≈ 2,200 時間を近傍将来として使う。
const ROLLOUT_ARRIVALS: usize = 22;
/// 共通乱数のサンプル本数。候補間で同一の到着列を使うため差の分散は小さい。
const ROLLOUT_SAMPLES: usize = 3;

/// 乱数 (再現性のため呼び出し側の状態からシードを決める)。標準的な xorshift64*。
struct XorShift64 {
    s: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            s: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).max(1),
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.s = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    fn next_f64(&mut self) -> f64 {
        // (0, 1) 開区間。ln に渡すため 0 を避ける。
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / 9_007_199_254_740_992.0)
    }

    #[inline]
    fn weighted_index(&mut self, weights: &[u32]) -> usize {
        let total = weights.iter().map(|&weight| weight as u64).sum::<u64>();
        debug_assert!(total > 0);
        let mut value = self.next_u64() % total;
        for (index, &weight) in weights.iter().enumerate() {
            if value < weight as u64 {
                return index;
            }
            value -= weight as u64;
        }
        weights.len() - 1
    }

    /// Box-Muller。1 呼び出し 1 値で十分 (対の値は捨てる)。
    fn gauss(&mut self) -> f64 {
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// ロールアウトで使う将来の到着 1 件。value は q×P×dur^0.9 で、
/// 置けた形の C を掛けると受け入れ価値 (V×C 相当) になる。
#[derive(Clone, Copy)]
struct FutureArrival {
    at: usize,
    dur: usize,
    P: usize,
    q: f64,
    value: f64,
}

/// 通常配置のロールアウト比較にかける候補 1 つ。
struct RolloutCandidate {
    board: Rows,
    /// V×C の即時実額。
    immediate: f64,
    /// incoming を受け入れる候補なら、その (T, cells)。退去処理に使う。
    incoming_dep: Option<(usize, Vec<usize>)>,
}

#[cfg(feature = "local")]
#[derive(Debug, Default)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
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
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[cfg(feature = "local")]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let start = Instant::now();
        let result = { $body };
        $trace.add_time_ms($key, start.elapsed().as_secs_f64() * 1000.0);
        result
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline]
    fn reached(&self, ratio: f64) -> bool {
        self.start.elapsed().as_secs_f64() >= self.time_limit_sec * ratio
    }

    #[cfg(feature = "local")]
    #[inline]
    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

struct Scanner<R> {
    reader: R,
    tokens: Vec<String>,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            tokens: Vec::new(),
        }
    }

    fn next<T: std::str::FromStr>(&mut self) -> T
    where
        T::Err: std::fmt::Debug,
    {
        loop {
            if let Some(token) = self.tokens.pop() {
                return token.parse().unwrap();
            }
            let mut line = String::new();
            let read = self.reader.read_line(&mut line).unwrap();
            assert!(read > 0, "unexpected EOF");
            self.tokens = line
                .split_whitespace()
                .rev()
                .map(ToOwned::to_owned)
                .collect();
        }
    }
}

#[derive(Clone)]
struct Shape {
    h: usize,
    w: usize,
    perimeter: usize,
    left: Vec<usize>,
    len: Vec<usize>,
    /// v501 相当の選抜 (上限14) に含まれる形状か。機構確認用に追加形状と区別する。
    baseline_kept: bool,
}

#[derive(Clone, Default)]
struct Group {
    id: usize,
    S: usize,
    T: usize,
    P: usize,
    V: i64,
    active: bool,
    cells: Vec<usize>,
}

#[derive(Clone)]
struct Placement {
    shape_index: usize,
    x: usize,
    y: usize,
    perimeter: usize,
    cheap_score: f64,
    final_score: f64,
    component_size: usize,
    slot_delay: usize,
    slot_penalty: f64,
    slot_count: usize,
    release_score: usize,
    release_current_largest: usize,
    release_snapshot_count: usize,
    release_candidate: bool,
    release_added_candidate: bool,
    explicit_cells: Vec<usize>,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            shape_index: usize::MAX,
            x: 0,
            y: 0,
            perimeter: 0,
            cheap_score: -1e100,
            final_score: -1e100,
            component_size: 0,
            slot_delay: 0,
            slot_penalty: 0.0,
            slot_count: 0,
            release_score: 0,
            release_current_largest: 0,
            release_snapshot_count: 0,
            release_candidate: false,
            release_added_candidate: false,
            explicit_cells: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct BoxWindow {
    x: usize,
    y: usize,
    h: usize,
    w: usize,
    free_count: usize,
    perimeter_lower_bound: i32,
    weight_sum: f64,
}

struct FreeInfo {
    component: Vec<isize>,
    sizes: Vec<usize>,
    cells: Vec<Vec<usize>>,
    free_count: usize,
    dead_ends: usize,
    metric: f64,
}

struct WeightData {
    prefix: [[f64; MAX_N + 1]; MAX_N],
    cell: [f64; MAX_N * MAX_N],
}

#[derive(Clone, Copy)]
struct LargeSlot {
    x: usize,
    y: usize,
    h: usize,
    w: usize,
    ready: usize,
}

struct SlotCalendar {
    slots: Vec<LargeSlot>,
    target_arrival_rate: f64,
    K: usize,
}

struct ReleaseAtlas {
    future_occupied: [Rows; RELEASE_ATLAS_SNAPSHOTS],
}

struct Solver {
    N: usize,
    M: usize,
    grass_rows: Rows,
    occupied_rows: Rows,
    owner_cell: Vec<isize>,
    groups: Vec<Group>,
    shapes_by_p: Vec<Vec<Shape>>,
    p_probability: Vec<f64>,
    p_cdf: Vec<f64>,
    departures: BinaryHeap<Reverse<(usize, usize)>>,
    duration_sum: f64,
    duration_count: usize,
    expected_p: f64,
    compactness_bar: f64,
    effective_capacity: f64,
    threshold_cache: HashMap<i32, f64>,
    /// P → 最小周長での C。ロールアウトの admission 判定と価値計算に使う。
    c_max_table: Vec<f64>,
    timer: TimeKeeper,
    #[cfg(feature = "local")]
    trace: TraceStats,
}

fn minimum_perimeter(P: usize) -> usize {
    2 * (2.0 * (P as f64).sqrt() - 1e-12).ceil() as usize
}

fn compactness(P: usize, perimeter: usize) -> f64 {
    4.0 * (P as f64).sqrt() / (perimeter as f64)
}

/// 自由セル v を 1 つ塞いだとき、元の自由連結成分のうち最大の残余成分から
/// 切り離されるセル数を返す。Tarjan の low-link と部分木サイズから全セル分を
/// O(V+E) で同時に求める。
fn free_cut_loss(N: usize, free: &[bool]) -> Vec<usize> {
    let cell_count = N * N;
    debug_assert_eq!(free.len(), cell_count);
    let unvisited = usize::MAX;
    let mut timer = 0_usize;
    let mut tin = vec![unvisited; cell_count];
    let mut low = vec![0_usize; cell_count];
    let mut parent = vec![unvisited; cell_count];
    let mut subtree = vec![0_usize; cell_count];
    let mut root_of = vec![unvisited; cell_count];
    let mut component_size_at_root = vec![0_usize; cell_count];

    #[allow(clippy::too_many_arguments)]
    fn dfs(
        v: usize,
        root: usize,
        N: usize,
        free: &[bool],
        timer: &mut usize,
        tin: &mut [usize],
        low: &mut [usize],
        parent: &mut [usize],
        subtree: &mut [usize],
        root_of: &mut [usize],
    ) {
        tin[v] = *timer;
        low[v] = *timer;
        *timer += 1;
        subtree[v] = 1;
        root_of[v] = root;
        let r = v / N;
        let c = v % N;
        let neighbors = [
            (r > 0).then_some(v.wrapping_sub(N)),
            (r + 1 < N).then_some(v + N),
            (c > 0).then_some(v.wrapping_sub(1)),
            (c + 1 < N).then_some(v + 1),
        ];
        for to in neighbors.into_iter().flatten() {
            if !free[to] || to == parent[v] {
                continue;
            }
            if tin[to] == usize::MAX {
                parent[to] = v;
                dfs(to, root, N, free, timer, tin, low, parent, subtree, root_of);
                subtree[v] += subtree[to];
                low[v] = low[v].min(low[to]);
            } else {
                low[v] = low[v].min(tin[to]);
            }
        }
    }

    for root in 0..cell_count {
        if free[root] && tin[root] == unvisited {
            dfs(
                root,
                root,
                N,
                free,
                &mut timer,
                &mut tin,
                &mut low,
                &mut parent,
                &mut subtree,
                &mut root_of,
            );
            component_size_at_root[root] = subtree[root];
        }
    }

    let mut cut_loss = vec![0_usize; cell_count];
    for v in 0..cell_count {
        if !free[v] {
            continue;
        }
        let component_size = component_size_at_root[root_of[v]];
        let r = v / N;
        let c = v % N;
        let neighbors = [
            (r > 0).then_some(v.wrapping_sub(N)),
            (r + 1 < N).then_some(v + N),
            (c > 0).then_some(v.wrapping_sub(1)),
            (c + 1 < N).then_some(v + 1),
        ];
        let mut separated_sum = 0_usize;
        let mut largest_piece = 0_usize;
        for child in neighbors.into_iter().flatten() {
            if parent[child] == v && low[child] >= tin[v] {
                separated_sum += subtree[child];
                largest_piece = largest_piece.max(subtree[child]);
            }
        }
        let parent_side = component_size - 1 - separated_sum;
        largest_piece = largest_piece.max(parent_side);
        cut_loss[v] = component_size - 1 - largest_piece;
    }
    cut_loss
}

#[cfg(feature = "local")]
fn verify_free_cut_loss() {
    fn naive(N: usize, free: &[bool], removed: usize) -> usize {
        if !free[removed] {
            return 0;
        }
        let mut original = vec![false; N * N];
        let mut stack = vec![removed];
        original[removed] = true;
        let mut original_size = 0_usize;
        while let Some(v) = stack.pop() {
            original_size += 1;
            let r = v / N;
            let c = v % N;
            let neighbors = [
                (r > 0).then_some(v.wrapping_sub(N)),
                (r + 1 < N).then_some(v + N),
                (c > 0).then_some(v.wrapping_sub(1)),
                (c + 1 < N).then_some(v + 1),
            ];
            for to in neighbors.into_iter().flatten() {
                if free[to] && !original[to] {
                    original[to] = true;
                    stack.push(to);
                }
            }
        }

        let mut seen = vec![false; N * N];
        seen[removed] = true;
        let mut largest = 0_usize;
        for start in 0..N * N {
            if !original[start] || seen[start] {
                continue;
            }
            let mut size = 0_usize;
            seen[start] = true;
            stack.push(start);
            while let Some(v) = stack.pop() {
                size += 1;
                let r = v / N;
                let c = v % N;
                let neighbors = [
                    (r > 0).then_some(v.wrapping_sub(N)),
                    (r + 1 < N).then_some(v + N),
                    (c > 0).then_some(v.wrapping_sub(1)),
                    (c + 1 < N).then_some(v + 1),
                ];
                for to in neighbors.into_iter().flatten() {
                    if original[to] && !seen[to] {
                        seen[to] = true;
                        stack.push(to);
                    }
                }
            }
            largest = largest.max(size);
        }
        original_size - 1 - largest
    }

    // 複数成分、閉路、橋、袋小路を同時に含む決定的な小盤面で全セルを照合する。
    for case_id in 0..8_usize {
        const TEST_N: usize = 7;
        let mut free = vec![false; TEST_N * TEST_N];
        for r in 0..TEST_N {
            for c in 0..TEST_N {
                let x = (r * 17 + c * 31 + case_id * 13 + r * c * 7) % 11;
                free[r * TEST_N + c] = x >= 3;
            }
        }
        let actual = free_cut_loss(TEST_N, &free);
        for id in 0..TEST_N * TEST_N {
            assert_eq!(actual[id], naive(TEST_N, &free, id));
        }
    }
}

fn shape_perimeter(shape: &Shape, P: usize) -> usize {
    let mut adjacent = 0;
    for r in 0..shape.h {
        adjacent += shape.len[r] - 1;
        if r > 0 {
            let a0 = shape.left[r - 1];
            let a1 = a0 + shape.len[r - 1];
            let b0 = shape.left[r];
            let b1 = b0 + shape.len[r];
            adjacent += a1.min(b1).saturating_sub(a0.max(b0));
        }
    }
    4 * P - 2 * adjacent
}

fn shape_key(shape: &Shape) -> Vec<u8> {
    let mut key = Vec::with_capacity(2 + 2 * shape.h);
    key.push(shape.h as u8);
    key.push(shape.w as u8);
    for r in 0..shape.h {
        key.push(shape.left[r] as u8);
        key.push(shape.len[r] as u8);
    }
    key
}

fn shape_complexity(shape: &Shape) -> usize {
    let mut result = 0;
    for r in 1..shape.h {
        result += usize::from(shape.left[r] != shape.left[r - 1]);
        result += usize::from(shape.len[r] != shape.len[r - 1]);
    }
    result
}

fn try_add_shape(
    generated: &mut Vec<Shape>,
    seen: &mut HashSet<Vec<u8>>,
    mut shape: Shape,
    P: usize,
    min_L: usize,
) {
    let mut area = 0;
    let mut connected_rows = true;
    for r in 0..shape.h {
        area += shape.len[r];
        if shape.len[r] == 0 || shape.left[r] + shape.len[r] > shape.w {
            return;
        }
        if r > 0 {
            let lo = shape.left[r - 1].max(shape.left[r]);
            let hi = (shape.left[r - 1] + shape.len[r - 1]).min(shape.left[r] + shape.len[r]);
            if lo >= hi {
                connected_rows = false;
            }
        }
    }
    if area != P || !connected_rows {
        return;
    }
    shape.perimeter = shape_perimeter(&shape, P);
    if shape.perimeter > min_L + 8 {
        return;
    }
    if seen.insert(shape_key(&shape)) {
        generated.push(shape);
    }
}

fn generate_shapes(N: usize) -> Vec<Vec<Shape>> {
    let mut shapes_by_p = vec![Vec::new(); MAX_P + 1];
    for (P, target_shapes) in shapes_by_p.iter_mut().enumerate().take(MAX_P + 1).skip(4) {
        let mut generated = Vec::new();
        let mut seen = HashSet::new();
        let min_L = minimum_perimeter(P);

        for h in 1..=N.min(P) {
            let w = P.div_ceil(h);
            if w > N {
                continue;
            }
            let missing = h * w - P;
            if missing == 0 {
                try_add_shape(
                    &mut generated,
                    &mut seen,
                    Shape {
                        h,
                        w,
                        perimeter: 0,
                        left: vec![0; h],
                        len: vec![w; h],
                        baseline_kept: false,
                    },
                    P,
                    min_L,
                );
                continue;
            }
            if w <= 1 {
                continue;
            }

            let mut starts = vec![
                0_isize,
                (h as isize) - (missing as isize),
                ((h as isize) - (missing as isize)) / 2,
                ((h as isize) - (missing as isize) + 1) / 2,
            ];
            starts.sort_unstable();
            starts.dedup();
            for st in starts {
                if st < 0 || (st as usize) + missing > h {
                    continue;
                }
                for remove_left in 0..=1 {
                    let mut shape = Shape {
                        h,
                        w,
                        perimeter: 0,
                        left: vec![0; h],
                        len: vec![w; h],
                        baseline_kept: false,
                    };
                    for r in (st as usize)..(st as usize + missing) {
                        shape.len[r] = w - 1;
                        shape.left[r] = remove_left;
                    }
                    try_add_shape(&mut generated, &mut seen, shape, P, min_L);
                }
            }

            if missing >= 2 {
                let top = missing / 2;
                let bottom = missing - top;
                for side_top in 0..=1 {
                    for side_bottom in 0..=1 {
                        let mut shape = Shape {
                            h,
                            w,
                            perimeter: 0,
                            left: vec![0; h],
                            len: vec![w; h],
                            baseline_kept: false,
                        };
                        for r in 0..top {
                            shape.len[r] = w - 1;
                            shape.left[r] = side_top;
                        }
                        for r in (h - bottom)..h {
                            shape.len[r] = w - 1;
                            shape.left[r] = side_bottom;
                        }
                        try_add_shape(&mut generated, &mut seen, shape, P, min_L);
                    }
                }
            }
        }

        generated.sort_by(|a, b| {
            a.perimeter
                .cmp(&b.perimeter)
                .then_with(|| a.h.abs_diff(a.w).cmp(&b.h.abs_diff(b.w)))
                .then_with(|| shape_complexity(a).cmp(&shape_complexity(b)))
                .then_with(|| (a.h > a.w).cmp(&(b.h > b.w)))
                .then_with(|| a.h.cmp(&b.h))
                .then_with(|| a.left.cmp(&b.left))
                .then_with(|| a.len.cmp(&b.len))
        });

        let mut kept = Vec::new();
        let mut begin = 0;
        while begin < generated.len() {
            let mut end = begin + 1;
            while end < generated.len() && generated[end].perimeter == generated[begin].perimeter {
                end += 1;
            }
            const BASE_CAP_PER_LEVEL: usize = 14;
            const CAP_PER_LEVEL: usize = 20;
            let count = end - begin;

            // v501 の14形状を必ず残し、その上位集合として先頭側の未採用形状を足す。
            let mut baseline_indices = Vec::new();
            if count <= BASE_CAP_PER_LEVEL {
                baseline_indices.extend(0..count);
            } else {
                let first = 8.min(count);
                baseline_indices.extend(0..first);
                let remain = BASE_CAP_PER_LEVEL - first;
                for k in 0..remain {
                    let idx = (first + k * (count - first) / remain.max(1)).min(count - 1);
                    baseline_indices.push(idx);
                }
                baseline_indices.sort_unstable();
                baseline_indices.dedup();
            }

            let mut chosen = baseline_indices.clone();
            for idx in 0..count {
                if chosen.len() >= CAP_PER_LEVEL {
                    break;
                }
                if !chosen.contains(&idx) {
                    chosen.push(idx);
                }
            }
            chosen.sort_unstable();
            for idx in chosen {
                let mut shape = generated[begin + idx].clone();
                shape.baseline_kept = baseline_indices.binary_search(&idx).is_ok();
                kept.push(shape);
            }
            begin = end;
        }
        *target_shapes = kept;
    }
    shapes_by_p
}

impl Solver {
    fn new(N: usize, M: usize, grass_rows: Rows, timer: TimeKeeper) -> Self {
        let mut groups = vec![Group::default(); M];
        for (id, group) in groups.iter_mut().enumerate() {
            group.id = id;
        }
        let mut solver = Self {
            N,
            M,
            grass_rows,
            occupied_rows: [0; MAX_N],
            owner_cell: vec![-1; N * N],
            groups,
            shapes_by_p: generate_shapes(N),
            p_probability: vec![0.0; MAX_P + 1],
            p_cdf: vec![0.0; MAX_P + 1],
            departures: BinaryHeap::new(),
            duration_sum: 0.0,
            duration_count: 0,
            expected_p: 0.0,
            compactness_bar: 1.0,
            effective_capacity: 1.0,
            threshold_cache: HashMap::new(),
            c_max_table: (0..=MAX_P)
                .map(|P| {
                    if P >= 4 {
                        compactness(P, minimum_perimeter(P))
                    } else {
                        0.0
                    }
                })
                .collect(),
            timer,
            #[cfg(feature = "local")]
            trace: TraceStats::default(),
        };
        solver.initialize_p_distribution();
        solver.initialize_static_capacity();
        local! {
            let kept = solver
                .shapes_by_p
                .iter()
                .map(|shapes| shapes.len())
                .sum::<usize>();
            solver.trace.count_by("shape_variants_kept", kept as i64);
        }
        solver
    }

    #[inline]
    fn bit_at(c: usize) -> u64 {
        1_u64 << c
    }

    #[inline]
    fn is_grass(&self, r: usize, c: usize) -> bool {
        ((self.grass_rows[r] >> c) & 1) != 0
    }

    #[inline]
    fn is_free(&self, occ: &Rows, r: usize, c: usize) -> bool {
        self.is_grass(r, c) && ((occ[r] >> c) & 1) == 0
    }

    fn initialize_p_distribution(&mut self) {
        let lo_x = 2.0;
        let hi_x = (150.0_f64).sqrt();
        let width = hi_x - lo_x;
        let mut weighted_c = 0.0;
        for P in 4..=MAX_P {
            let lo = lo_x.max(((P as f64) - 0.5).max(0.0).sqrt());
            let hi = hi_x.min(((P as f64) + 0.5).sqrt());
            let probability = (hi - lo).max(0.0) / width;
            self.p_probability[P] = probability;
            self.expected_p += probability * (P as f64);
            weighted_c += probability * (P as f64) * compactness(P, minimum_perimeter(P));
        }
        let mut cumulative = 0.0;
        for P in 0..=MAX_P {
            cumulative += self.p_probability[P];
            self.p_cdf[P] = cumulative;
        }
        self.compactness_bar = weighted_c / self.expected_p;
    }

    #[inline]
    fn fit_probability(&self, component_size: usize) -> f64 {
        if component_size < 4 {
            0.0
        } else if component_size >= MAX_P {
            1.0
        } else {
            self.p_cdf[component_size]
        }
    }

    fn compute_free_info(&self, occ: &Rows, keep_cells: bool) -> FreeInfo {
        let mut info = FreeInfo {
            component: vec![-1; self.N * self.N],
            sizes: Vec::new(),
            cells: Vec::new(),
            free_count: 0,
            dead_ends: 0,
            metric: 0.0,
        };
        let mut queue_cells = vec![0; self.N * self.N];
        let mut component_id = 0_isize;
        const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        for r in 0..self.N {
            for c in 0..self.N {
                if !self.is_free(occ, r, c) {
                    continue;
                }
                let id = r * self.N + c;
                info.free_count += 1;
                let mut degree = 0;
                for (dr, dc) in DIRS {
                    let nr = (r as isize) + dr;
                    let nc = (c as isize) + dc;
                    if nr >= 0
                        && nr < self.N as isize
                        && nc >= 0
                        && nc < self.N as isize
                        && self.is_free(occ, nr as usize, nc as usize)
                    {
                        degree += 1;
                    }
                }
                if degree <= 1 {
                    info.dead_ends += 1;
                }
                if info.component[id] != -1 {
                    continue;
                }

                let mut head = 0;
                let mut tail = 0;
                queue_cells[tail] = id;
                tail += 1;
                info.component[id] = component_id;
                let mut component_cells = Vec::new();
                while head < tail {
                    let v = queue_cells[head];
                    head += 1;
                    if keep_cells {
                        component_cells.push(v);
                    }
                    let vr = v / self.N;
                    let vc = v % self.N;
                    for (dr, dc) in DIRS {
                        let nr = (vr as isize) + dr;
                        let nc = (vc as isize) + dc;
                        if nr < 0 || nr >= self.N as isize || nc < 0 || nc >= self.N as isize {
                            continue;
                        }
                        let ni = (nr as usize) * self.N + (nc as usize);
                        if info.component[ni] != -1 || !self.is_free(occ, nr as usize, nc as usize)
                        {
                            continue;
                        }
                        info.component[ni] = component_id;
                        queue_cells[tail] = ni;
                        tail += 1;
                    }
                }
                info.sizes.push(tail);
                if keep_cells {
                    info.cells.push(component_cells);
                }
                component_id += 1;
            }
        }

        let mut metric = 4.0 * (info.dead_ends as f64);
        for &size in &info.sizes {
            if size < 4 {
                metric += 100.0 * (size as f64);
            } else {
                metric += 18.0 + 3.0 * (size as f64).sqrt();
                if size < MAX_P {
                    metric += 30.0 * (1.0 - self.fit_probability(size));
                }
            }
        }
        info.metric = metric;
        info
    }

    fn fragment_metric(&self, occ: &Rows) -> f64 {
        self.compute_free_info(occ, false).metric
    }

    fn build_weight_data(
        &mut self,
        occ: &Rows,
        info: &FreeInfo,
        incoming_S: usize,
        incoming_T: usize,
        incoming_D: usize,
    ) -> WeightData {
        let mut data = WeightData {
            prefix: [[0.0; MAX_N + 1]; MAX_N],
            cell: [0.0; MAX_N * MAX_N],
        };
        let mut _integrated_contact_edges = 0_i64;
        let mut _integrated_contact_time = 0_i64;
        const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for r in 0..self.N {
            for c in 0..self.N {
                let id = r * self.N + c;
                let mut contact_time = 0_usize;
                if self.is_free(occ, r, c) {
                    for (dr, dc) in DIRS {
                        let nr = (r as isize) + dr;
                        let nc = (c as isize) + dc;
                        if nr < 0 || nr >= self.N as isize || nc < 0 || nc >= self.N as isize {
                            contact_time += incoming_D;
                            continue;
                        }
                        let nr = nr as usize;
                        let nc = nc as usize;
                        if !self.is_grass(nr, nc) {
                            contact_time += incoming_D;
                        } else if ((occ[nr] >> nc) & 1) != 0 {
                            let owner = self.owner_cell[nr * self.N + nc];
                            if owner >= 0
                                && (owner as usize) < self.M
                                && self.groups[owner as usize].active
                            {
                                // 既存区画との共有辺は、両区画が同時に存在する時間だけ
                                // 境界を2辺分消すので、その重なり時間を接触価値にする。
                                let overlap = self.groups[owner as usize]
                                    .T
                                    .min(incoming_T)
                                    .saturating_sub(incoming_S);
                                contact_time += overlap;
                                _integrated_contact_edges += 1;
                            }
                        }
                    }
                    _integrated_contact_time += contact_time as i64;
                    // 候補の時間積分境界は L*D - 2*contact_time。同一周長では
                    // contact_time 最大が最小境界と一致する。10/D で従来尺度へ正規化する。
                    let mut weight = 10.0 * (contact_time as f64) / (incoming_D.max(1) as f64);
                    let component_id = info.component[id];
                    if component_id >= 0 {
                        let component_size = info.sizes[component_id as usize];
                        if component_size < MAX_P {
                            weight += 2.5 * (1.0 - self.fit_probability(component_size));
                        }
                    }
                    weight -= 1e-7 * (id as f64);
                    data.cell[id] = weight;
                }
                data.prefix[r][c + 1] = data.prefix[r][c] + data.cell[id];
            }
        }
        local! {
            self.trace
                .count_by("integrated_contact_edges", _integrated_contact_edges);
            self.trace
                .count_by("integrated_contact_time", _integrated_contact_time);
        }
        data
    }

    fn build_run_table(&self, occ: &Rows) -> RunTable {
        let mut runs = [[0_u64; MAX_N + 1]; MAX_N];
        for r in 0..self.N {
            let free_mask = self.grass_rows[r] & !occ[r];
            runs[r][1] = free_mask;
            for len in 2..=self.N {
                runs[r][len] = runs[r][len - 1] & (free_mask >> (len - 1));
            }
        }
        runs
    }

    fn materialize(&self, placement: &Placement, P: usize) -> Vec<usize> {
        if !placement.explicit_cells.is_empty() {
            return placement.explicit_cells.clone();
        }
        let shape = &self.shapes_by_p[P][placement.shape_index];
        let mut cells = Vec::with_capacity(P);
        for rr in 0..shape.h {
            let r = placement.x + rr;
            let start = placement.y + shape.left[rr];
            for c in start..(start + shape.len[rr]) {
                cells.push(r * self.N + c);
            }
        }
        cells
    }

    fn insert_top_by_cheap(top: &mut Vec<Placement>, candidate: Placement, limit: usize) {
        if top.len() < limit {
            top.push(candidate);
            return;
        }
        let mut worst = 0;
        for i in 1..limit {
            if top[i].cheap_score < top[worst].cheap_score {
                worst = i;
            }
        }
        if candidate.cheap_score > top[worst].cheap_score {
            top[worst] = candidate;
        }
    }

    #[inline]
    fn slot_order(a: &LargeSlot, b: &LargeSlot) -> std::cmp::Ordering {
        a.ready
            .cmp(&b.ready)
            .then_with(|| (b.h * b.w).cmp(&(a.h * a.w)))
            .then_with(|| a.x.cmp(&b.x))
            .then_with(|| a.y.cmp(&b.y))
            .then_with(|| a.h.cmp(&b.h))
            .then_with(|| a.w.cmp(&b.w))
    }

    #[inline]
    fn slots_overlap(a: &LargeSlot, b: &LargeSlot) -> bool {
        a.x < b.x + b.h && b.x < a.x + a.h && a.y < b.y + b.w && b.y < a.y + a.w
    }

    /// freeは現在時刻、active区画は退去時刻をreadyとし、代表矩形の早いslotを作る。
    /// 各矩形のmax readyは横・縦のsliding maximumで O(N^2) に列挙する。
    fn build_slot_calendar(&mut self, incoming_id: usize, S: usize, theta: f64) -> SlotCalendar {
        local! {
            self.trace.count("slot_calendar_turn");
        }
        let remaining = self.M - 1 - incoming_id;
        let remaining_time = HORIZON.saturating_sub(S);
        if remaining == 0 || remaining_time == 0 {
            local! {
                self.trace.count_by("slot_inventory_sum", 0);
            }
            return SlotCalendar {
                slots: Vec::new(),
                target_arrival_rate: 0.0,
                K: 0,
            };
        }
        let remaining_rate = (remaining as f64) / (remaining_time as f64);
        let target_arrival_rate =
            remaining_rate * (1.0 - self.p_cdf[95]) * 0.5 * (-6_000.0 / theta.max(1.0)).exp();
        let K = (target_arrival_rate * theta).ceil().clamp(1.0, 6.0) as usize;

        const INVALID: usize = usize::MAX / 4;
        const DIMS: [(usize, usize); 9] = [
            (10, 10),
            (10, 12),
            (12, 10),
            (11, 11),
            (10, 15),
            (15, 10),
            (12, 12),
            (12, 13),
            (13, 12),
        ];
        let mut ready = [INVALID; MAX_N * MAX_N];
        for r in 0..self.N {
            for c in 0..self.N {
                let id = r * self.N + c;
                if !self.is_grass(r, c) {
                    continue;
                }
                let owner = self.owner_cell[id];
                ready[id] = if owner >= 0 {
                    let owner = owner as usize;
                    debug_assert!(owner < self.M && self.groups[owner].active);
                    self.groups[owner].T
                } else {
                    S
                };
            }
        }

        let mut pool = Vec::with_capacity(16_000);
        let mut horizontal = [INVALID; MAX_N * MAX_N];
        let mut deque = [0_usize; MAX_N];
        for (h, w) in DIMS {
            if h > self.N || w > self.N {
                continue;
            }
            let y_count = self.N - w + 1;
            for r in 0..self.N {
                let mut head = 0;
                let mut tail = 0;
                for c in 0..self.N {
                    while head < tail && deque[head] + w <= c {
                        head += 1;
                    }
                    let value = ready[r * self.N + c];
                    while head < tail && ready[r * self.N + deque[tail - 1]] <= value {
                        tail -= 1;
                    }
                    deque[tail] = c;
                    tail += 1;
                    if c + 1 >= w {
                        horizontal[r * y_count + c + 1 - w] = ready[r * self.N + deque[head]];
                    }
                }
            }
            for y in 0..y_count {
                let mut head = 0;
                let mut tail = 0;
                for r in 0..self.N {
                    while head < tail && deque[head] + h <= r {
                        head += 1;
                    }
                    let value = horizontal[r * y_count + y];
                    while head < tail && horizontal[deque[tail - 1] * y_count + y] <= value {
                        tail -= 1;
                    }
                    deque[tail] = r;
                    tail += 1;
                    if r + 1 >= h {
                        let slot_ready = horizontal[deque[head] * y_count + y];
                        if slot_ready != INVALID {
                            pool.push(LargeSlot {
                                x: r + 1 - h,
                                y,
                                h,
                                w,
                                ready: slot_ready,
                            });
                        }
                    }
                }
            }
        }

        const POOL_LIMIT: usize = 384;
        if pool.len() > POOL_LIMIT {
            pool.select_nth_unstable_by(POOL_LIMIT, Self::slot_order);
            pool.truncate(POOL_LIMIT);
        }
        pool.sort_unstable_by(Self::slot_order);
        let mut slots: Vec<LargeSlot> = Vec::with_capacity(K);
        for candidate in pool {
            if slots
                .iter()
                .all(|selected| !Self::slots_overlap(selected, &candidate))
            {
                slots.push(candidate);
                if slots.len() == K {
                    break;
                }
            }
        }
        local! {
            self.trace
                .count_by("slot_inventory_sum", slots.len() as i64);
        }
        SlotCalendar {
            slots,
            target_arrival_rate,
            K,
        }
    }

    #[inline]
    fn regular_intersects_slot(&self, shape: &Shape, x: usize, y: usize, slot: &LargeSlot) -> bool {
        for rr in 0..shape.h {
            let r = x + rr;
            if r < slot.x || r >= slot.x + slot.h {
                continue;
            }
            let begin = y + shape.left[rr];
            let end = begin + shape.len[rr];
            if begin < slot.y + slot.w && slot.y < end {
                return true;
            }
        }
        false
    }

    fn regular_slot_delay(
        &self,
        shape: &Shape,
        x: usize,
        y: usize,
        incoming_T: usize,
        calendar: &SlotCalendar,
    ) -> usize {
        calendar
            .slots
            .iter()
            .filter(|slot| self.regular_intersects_slot(shape, x, y, slot))
            .map(|slot| incoming_T.saturating_sub(slot.ready))
            .sum()
    }

    #[inline]
    fn slot_penalty(calendar: &SlotCalendar, slot_delay: usize) -> f64 {
        // 414kは大型targetを悪形に追いやった1件あたりの観測平均損失である。
        (slot_delay as f64) * calendar.target_arrival_rate * 414_000.0 / (calendar.K.max(1) as f64)
    }

    /// baselineのtop集合とは独立に、componentごとのslot delay最小候補を全位置から拾う。
    fn scan_regular_slot_best(
        &self,
        P: usize,
        runs: &RunTable,
        weights: &WeightData,
        info: &FreeInfo,
        perimeter: usize,
        incoming_T: usize,
        calendar: &SlotCalendar,
    ) -> Vec<Option<Placement>> {
        let mut best: Vec<Option<Placement>> = vec![None; info.sizes.len()];
        for (shape_index, shape) in self.shapes_by_p[P].iter().enumerate() {
            if shape.perimeter != perimeter {
                continue;
            }
            let y_count = self.N - shape.w + 1;
            let valid_y = (1_u64 << y_count) - 1;
            for x in 0..=(self.N - shape.h) {
                let mut ys = valid_y;
                for rr in 0..shape.h {
                    if ys == 0 {
                        break;
                    }
                    ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                }
                while ys != 0 {
                    let y = ys.trailing_zeros() as usize;
                    ys &= ys - 1;
                    let first = x * self.N + y + shape.left[0];
                    let component = info.component[first];
                    debug_assert!(component >= 0);
                    let component = component as usize;
                    let mut cheap_score = 0.0;
                    for rr in 0..shape.h {
                        let begin = y + shape.left[rr];
                        let end = begin + shape.len[rr];
                        cheap_score += weights.prefix[x + rr][end] - weights.prefix[x + rr][begin];
                    }
                    let slot_delay = self.regular_slot_delay(shape, x, y, incoming_T, calendar);
                    if best[component].as_ref().is_none_or(|current| {
                        slot_delay < current.slot_delay
                            || (slot_delay == current.slot_delay
                                && cheap_score > current.cheap_score)
                    }) {
                        best[component] = Some(Placement {
                            shape_index,
                            x,
                            y,
                            perimeter,
                            cheap_score,
                            component_size: info.sizes[component],
                            slot_delay,
                            slot_penalty: Self::slot_penalty(calendar, slot_delay),
                            slot_count: calendar.slots.len(),
                            ..Placement::default()
                        });
                    }
                }
            }
        }
        best
    }

    /// incoming が滞在している 1/4, 1/2, 3/4 時点について、現在判明している
    /// active の退去だけを反映した占有盤面を作る。未知の将来到着はここでは足さず、
    /// 候補間の最終比較を共通乱数 rollout に任せる。
    fn build_release_atlas(&mut self, incoming_S: usize, incoming_T: usize) -> ReleaseAtlas {
        let D = incoming_T - incoming_S;
        let mut future_occupied = [[0_u64; MAX_N]; RELEASE_ATLAS_SNAPSHOTS];
        let mut released_sum = 0_i64;
        for (snapshot_index, numerator) in [1_usize, 2, 3].into_iter().enumerate() {
            let at = incoming_S + D * numerator / 4;
            let mut board = self.occupied_rows;
            for group in &self.groups {
                if !group.active || group.T >= at {
                    continue;
                }
                released_sum += 1;
                for &cell in &group.cells {
                    board[cell / self.N] &= !Self::bit_at(cell % self.N);
                }
            }
            future_occupied[snapshot_index] = board;
        }
        local! {
            self.trace
                .count_by("release_atlas_snapshot", RELEASE_ATLAS_SNAPSHOTS as i64);
            self.trace
                .count_by("release_atlas_released_sum", released_sum);
        }
        ReleaseAtlas { future_occupied }
    }

    /// 各 snapshot で候補を置いた後の最大自由連結成分を合計する。
    /// blocker probe と同じ量を使い、固定矩形では捉えられない再接続を直接測る。
    fn regular_release_score(&self, atlas: &ReleaseAtlas, cells: &[usize]) -> usize {
        atlas
            .future_occupied
            .iter()
            .map(|base| {
                let mut board = *base;
                for &cell in cells {
                    board[cell / self.N] |= Self::bit_at(cell % self.N);
                }
                self.compute_free_info(&board, false)
                    .sizes
                    .into_iter()
                    .max()
                    .unwrap_or(0)
            })
            .sum()
    }

    fn regular_current_largest(&self, cells: &[usize]) -> usize {
        let mut board = self.occupied_rows;
        for &cell in cells {
            board[cell / self.N] |= Self::bit_at(cell % self.N);
        }
        self.compute_free_info(&board, false)
            .sizes
            .into_iter()
            .max()
            .unwrap_or(0)
    }

    /// 全位置を5×5の空間bucketへ分け、各bucketの局所重み最大だけをexactな
    /// 連結成分評価へ進める。現行top集合から離れた位置も残しつつ、最大75回の
    /// flood fillへ計算量を制限する。
    #[allow(clippy::too_many_arguments)]
    fn scan_regular_release_best(
        &mut self,
        P: usize,
        runs: &RunTable,
        weights: &WeightData,
        info: &FreeInfo,
        perimeter: usize,
        component: usize,
        atlas: &ReleaseAtlas,
    ) -> Option<Placement> {
        let bucket_count = RELEASE_ATLAS_BUCKETS * RELEASE_ATLAS_BUCKETS;
        let mut representatives: Vec<Option<Placement>> = vec![None; bucket_count];
        for (shape_index, shape) in self.shapes_by_p[P].iter().enumerate() {
            if shape.perimeter != perimeter {
                continue;
            }
            let y_count = self.N - shape.w + 1;
            let valid_y = (1_u64 << y_count) - 1;
            for x in 0..=(self.N - shape.h) {
                let mut ys = valid_y;
                for rr in 0..shape.h {
                    if ys == 0 {
                        break;
                    }
                    ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                }
                while ys != 0 {
                    let y = ys.trailing_zeros() as usize;
                    ys &= ys - 1;
                    let first = x * self.N + y + shape.left[0];
                    if info.component[first] != component as isize {
                        continue;
                    }
                    let mut cheap_score = 0.0;
                    for rr in 0..shape.h {
                        let begin = y + shape.left[rr];
                        let end = begin + shape.len[rr];
                        cheap_score += weights.prefix[x + rr][end] - weights.prefix[x + rr][begin];
                    }
                    let center_x2 = 2 * x + shape.h;
                    let center_y2 = 2 * y + shape.w;
                    let bucket_x = (center_x2 * RELEASE_ATLAS_BUCKETS / (2 * self.N))
                        .min(RELEASE_ATLAS_BUCKETS - 1);
                    let bucket_y = (center_y2 * RELEASE_ATLAS_BUCKETS / (2 * self.N))
                        .min(RELEASE_ATLAS_BUCKETS - 1);
                    let bucket = bucket_x * RELEASE_ATLAS_BUCKETS + bucket_y;
                    if representatives[bucket]
                        .as_ref()
                        .is_none_or(|current| cheap_score > current.cheap_score)
                    {
                        representatives[bucket] = Some(Placement {
                            shape_index,
                            x,
                            y,
                            perimeter,
                            cheap_score,
                            component_size: info.sizes[component],
                            ..Placement::default()
                        });
                    }
                }
            }
        }

        let populated = representatives
            .iter()
            .filter(|candidate| candidate.is_some())
            .count();
        local! {
            self.trace
                .count_by("release_atlas_bucket", populated as i64);
        }
        let mut best: Option<Placement> = None;
        for mut candidate in representatives.into_iter().flatten() {
            let cells = self.materialize(&candidate, P);
            candidate.release_score = self.regular_release_score(atlas, &cells);
            candidate.release_current_largest = self.regular_current_largest(&cells);
            candidate.release_snapshot_count = RELEASE_ATLAS_SNAPSHOTS;
            candidate.release_candidate = true;
            candidate.explicit_cells = cells;
            if best.as_ref().is_none_or(|current| {
                candidate.release_score > current.release_score
                    || (candidate.release_score == current.release_score
                        && candidate.cheap_score > current.cheap_score)
            }) {
                best = Some(candidate);
            }
        }
        best
    }

    fn scan_regular_level(
        &self,
        P: usize,
        runs: &RunTable,
        weights: &WeightData,
        perimeter: usize,
        top_count: usize,
        shape_limit: usize,
    ) -> Vec<Placement> {
        let mut top = Vec::new();
        let mut used_shapes = 0;
        for (shape_index, shape) in self.shapes_by_p[P].iter().enumerate() {
            if shape.perimeter != perimeter {
                continue;
            }
            if used_shapes >= shape_limit {
                break;
            }
            used_shapes += 1;
            let y_count = self.N - shape.w + 1;
            let valid_y = (1_u64 << y_count) - 1;
            for x in 0..=(self.N - shape.h) {
                let mut ys = valid_y;
                for rr in 0..shape.h {
                    if ys == 0 {
                        break;
                    }
                    ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                }
                while ys != 0 {
                    let y = ys.trailing_zeros() as usize;
                    ys &= ys - 1;
                    let mut score = 0.0;
                    for rr in 0..shape.h {
                        let begin = y + shape.left[rr];
                        let end = begin + shape.len[rr];
                        score += weights.prefix[x + rr][end] - weights.prefix[x + rr][begin];
                    }
                    Self::insert_top_by_cheap(
                        &mut top,
                        Placement {
                            shape_index,
                            x,
                            y,
                            perimeter,
                            cheap_score: score,
                            ..Placement::default()
                        },
                        top_count,
                    );
                }
            }
        }
        top
    }

    fn perimeter_of_cells(&self, cells: &[usize]) -> usize {
        let mut rows = [0_u64; MAX_N];
        for &id in cells {
            rows[id / self.N] |= Self::bit_at(id % self.N);
        }
        let mut perimeter = 0;
        const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for &id in cells {
            let r = id / self.N;
            let c = id % self.N;
            for (dr, dc) in DIRS {
                let nr = (r as isize) + dr;
                let nc = (c as isize) + dc;
                if nr < 0
                    || nr >= self.N as isize
                    || nc < 0
                    || nc >= self.N as isize
                    || ((rows[nr as usize] >> (nc as usize)) & 1) == 0
                {
                    perimeter += 1;
                }
            }
        }
        perimeter
    }

    /// removed を外した selected が expected 個すべて連結かを調べる。
    /// P<=150 なので、提案ごとに明示的な DFS をしても十分軽い。
    fn selected_is_connected(&self, selected: &[bool], start: usize, expected: usize) -> bool {
        let mut seen = vec![false; self.N * self.N];
        let mut stack = Vec::with_capacity(expected);
        seen[start] = true;
        stack.push(start);
        let mut reached = 0;
        while let Some(id) = stack.pop() {
            reached += 1;
            let r = id / self.N;
            let c = id % self.N;
            for next in [
                (r > 0).then_some(id.wrapping_sub(self.N)),
                (r + 1 < self.N).then_some(id + self.N),
                (c > 0).then_some(id.wrapping_sub(1)),
                (c + 1 < self.N).then_some(id + 1),
            ]
            .into_iter()
            .flatten()
            {
                if selected[next] && !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        reached == expected
    }

    /// growth の完成領域を、連結性を保つ1セル交換で改善する。
    fn improve_growth_by_sa(
        &mut self,
        initial: Placement,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
    ) -> Placement {
        local! {
            self.trace.count("growth_sa_session");
        }
        if self.timer.reached(GROWTH_SA_LIMIT_RATIO) {
            local! {
                self.trace.count_by("growth_sa_iteration", 0);
                self.trace.count_by("growth_sa_improved", 0);
                self.trace.count_by("growth_perimeter_reduction", 0);
            }
            return initial;
        }

        let P = initial.explicit_cells.len();
        debug_assert!(P >= 2);
        let component = info.component[initial.explicit_cells[0]];
        let mut current_cells = initial.explicit_cells.clone();
        let mut current_perimeter = initial.perimeter;
        let mut current_cheap = initial.cheap_score;
        let mut best = initial;
        let _initial_perimeter = current_perimeter;
        let mut selected = vec![false; self.N * self.N];
        for &id in &current_cells {
            selected[id] = true;
        }

        // 盤面と候補セルだけからseedを作り、同じ入力では常に同じ探索列にする。
        let mut seed = (P as u64) ^ 0xA076_1D64_78BD_642F;
        for &id in &current_cells {
            seed = seed.rotate_left(9) ^ (id as u64).wrapping_mul(0xE703_7ED1_A0B4_28DB);
        }
        let mut rng = XorShift64::new(seed);
        let mut _iterations = 0_i64;
        let mut _improved = 0_i64;

        for iteration in 0..GROWTH_SA_ITERATIONS {
            if iteration % 32 == 0 && self.timer.reached(GROWTH_SA_LIMIT_RATIO) {
                break;
            }
            _iterations += 1;
            let remove_index = (rng.next_u64() as usize) % P;
            let removed = current_cells[remove_index];
            selected[removed] = false;

            let start = current_cells[(remove_index + 1) % P];
            if !self.selected_is_connected(&selected, start, P - 1) {
                selected[removed] = true;
                continue;
            }

            // 連結なP-1セルに隣接セルを1つ足すため、サイズPと連結性を同時に保てる。
            let mut frontier = Vec::new();
            let mut seen_frontier = vec![false; self.N * self.N];
            for &id in &current_cells {
                if !selected[id] {
                    continue;
                }
                let r = id / self.N;
                let c = id % self.N;
                for next in [
                    (r > 0).then_some(id.wrapping_sub(self.N)),
                    (r + 1 < self.N).then_some(id + self.N),
                    (c > 0).then_some(id.wrapping_sub(1)),
                    (c + 1 < self.N).then_some(id + 1),
                ]
                .into_iter()
                .flatten()
                {
                    let nr = next / self.N;
                    let nc = next % self.N;
                    if !selected[next]
                        && !seen_frontier[next]
                        && info.component[next] == component
                        && self.is_free(occ, nr, nc)
                    {
                        seen_frontier[next] = true;
                        frontier.push(next);
                    }
                }
            }
            if frontier.is_empty() {
                selected[removed] = true;
                continue;
            }
            let added = frontier[(rng.next_u64() as usize) % frontier.len()];
            let mut proposal_cells = current_cells.clone();
            proposal_cells[remove_index] = added;
            let proposal_perimeter = self.perimeter_of_cells(&proposal_cells);
            let proposal_cheap = current_cheap - weights.cell[removed] + weights.cell[added];

            // 周長+2を序盤には低確率で受け入れ、温度低下とともに辞書順改善へ収束させる。
            const PERIMETER_WEIGHT: f64 = 50.0;
            const START_TEMP: f64 = 50.0;
            const END_TEMP: f64 = 1.0;
            let progress = (iteration as f64) / ((GROWTH_SA_ITERATIONS - 1) as f64);
            let temperature = START_TEMP * (END_TEMP / START_TEMP).powf(progress);
            let score_delta = (proposal_cheap - current_cheap)
                - PERIMETER_WEIGHT * (proposal_perimeter as f64 - current_perimeter as f64);
            let accept = score_delta >= 0.0 || rng.next_f64() < (score_delta / temperature).exp();
            if accept {
                current_cells = proposal_cells;
                current_perimeter = proposal_perimeter;
                current_cheap = proposal_cheap;
                selected[added] = true;

                if current_perimeter < best.perimeter
                    || (current_perimeter == best.perimeter && current_cheap > best.cheap_score)
                {
                    // 現在解は悪化し得るため、時間切れでも品質を失わない best-so-far を返す。
                    best.perimeter = current_perimeter;
                    best.cheap_score = current_cheap;
                    best.explicit_cells = current_cells.clone();
                    _improved += 1;
                }
            } else {
                selected[removed] = true;
            }
        }
        local! {
            self.trace.count_by("growth_sa_iteration", _iterations);
            self.trace.count_by("growth_sa_improved", _improved);
            self.trace.count_by(
                "growth_perimeter_reduction",
                _initial_perimeter.saturating_sub(best.perimeter) as i64,
            );
        }
        best
    }

    /// admission と既存候補群の winner 選択を変えず、受理済み growth winner のみを
    /// 次数に偏らせた連結1セル交換で後処理する。同周長では座標を含めて baseline を保つ。
    fn improve_final_growth_by_biased_swap(&mut self, initial: Placement, V: i64) -> Placement {
        let P = initial.explicit_cells.len();
        let min_L = minimum_perimeter(P);
        let recoverable_fee =
            (V as f64) * (self.c_max_table[P] - compactness(P, initial.perimeter)).max(0.0);
        if P < 64
            || initial.perimeter < min_L + 8
            || recoverable_fee < BIASED_SWAP_MIN_RECOVERABLE_FEE
        {
            return initial;
        }
        local! {
            self.trace.count("biased_swap_eligible");
            self.trace.count_by("biased_swap_time_limit_hit", 0);
        }
        if self.timer.reached(BIASED_SWAP_START_RATIO) {
            local! {
                self.trace.count("biased_swap_time_limit_hit");
            }
            return initial;
        }
        local! {
            self.trace.count("biased_swap_session");
        }

        let occ = self.occupied_rows;
        let info = self.compute_free_info(&occ, false);
        let component = info.component[initial.explicit_cells[0]];
        assert!(
            component >= 0,
            "growth winner must start in a free component"
        );
        assert!(self.explicit_candidate_is_valid(&initial.explicit_cells, P, &occ));
        assert!(
            initial
                .explicit_cells
                .iter()
                .all(|&id| info.component[id] == component)
        );

        let initial_perimeter = initial.perimeter;
        assert_eq!(
            self.perimeter_of_cells(&initial.explicit_cells),
            initial_perimeter
        );
        let mut current_cells = initial.explicit_cells.clone();
        let mut current_perimeter = initial_perimeter;
        let mut best_perimeter = initial_perimeter;
        let mut best_cells = initial.explicit_cells.clone();
        let mut selected = vec![false; self.N * self.N];
        for &id in &current_cells {
            selected[id] = true;
        }

        // baseline の盤面とセルだけから seed を作り、既存探索の乱数列には干渉しない。
        let mut seed = (P as u64)
            ^ (V as u64).rotate_left(23)
            ^ (initial_perimeter as u64).rotate_left(41)
            ^ 0xD1B5_4A32_D192_ED03;
        for &id in &current_cells {
            seed = seed.rotate_left(9) ^ (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        let mut rng = XorShift64::new(seed);
        let mut _iterations = 0_i64;
        let mut _accepted = 0_i64;
        let mut _improved = 0_i64;

        for iteration in 0..BIASED_SWAP_ITERATIONS {
            if iteration % 16 == 0 && self.timer.reached(BIASED_SWAP_LIMIT_RATIO) {
                local! {
                    self.trace.count("biased_swap_time_limit_hit");
                }
                break;
            }
            _iterations += 1;

            let mut removals = Vec::with_capacity(P);
            let mut remove_weights = Vec::with_capacity(P);
            for (index, &id) in current_cells.iter().enumerate() {
                let degree = self.count_selected_neighbors(id, &selected) as usize;
                if !(1..=3).contains(&degree) {
                    continue;
                }
                removals.push((index, id, degree));
                remove_weights.push([0_u32, 24, 6, 1][degree]);
            }
            if removals.is_empty() {
                break;
            }

            // 関節境界を先に全列挙せず、probe と同じ最大12回の重み付き抽選で選ぶ。
            let mut chosen_remove = None;
            for _ in 0..12 {
                let remove_choice = rng.weighted_index(&remove_weights);
                let (remove_index, removed, degree) = removals[remove_choice];
                selected[removed] = false;
                let start = current_cells[(remove_index + 1) % P];
                let connected = self.selected_is_connected(&selected, start, P - 1);
                selected[removed] = true;
                if connected {
                    chosen_remove = Some((remove_index, removed, degree));
                    break;
                }
            }
            let Some((remove_index, removed, remove_degree)) = chosen_remove else {
                continue;
            };
            selected[removed] = false;

            // P-1セルの frontier だけを候補にすれば、追加後のPセル連結性は自動で保たれる。
            let mut frontier = Vec::with_capacity(P * 4);
            let mut add_weights = Vec::with_capacity(P * 4);
            let mut seen_frontier = vec![false; self.N * self.N];
            for &id in &current_cells {
                if !selected[id] {
                    continue;
                }
                let r = id / self.N;
                let c = id % self.N;
                for next in [
                    (r > 0).then_some(id.wrapping_sub(self.N)),
                    (r + 1 < self.N).then_some(id + self.N),
                    (c > 0).then_some(id.wrapping_sub(1)),
                    (c + 1 < self.N).then_some(id + 1),
                ]
                .into_iter()
                .flatten()
                {
                    if next == removed
                        || selected[next]
                        || seen_frontier[next]
                        || info.component[next] != component
                    {
                        continue;
                    }
                    let nr = next / self.N;
                    let nc = next % self.N;
                    if !self.is_free(&occ, nr, nc) {
                        continue;
                    }
                    let degree = self.count_selected_neighbors(next, &selected) as usize;
                    debug_assert!((1..=4).contains(&degree));
                    seen_frontier[next] = true;
                    frontier.push((next, degree));
                    add_weights.push([0_u32, 1, 6, 36, 216][degree]);
                }
            }
            if frontier.is_empty() {
                selected[removed] = true;
                continue;
            }

            let add_choice = rng.weighted_index(&add_weights);
            let (added, add_degree) = frontier[add_choice];
            let perimeter_delta = 2 * (remove_degree as i64 - add_degree as i64);
            let proposal_perimeter = (current_perimeter as i64 + perimeter_delta) as usize;
            let progress = (iteration as f64) / ((BIASED_SWAP_ITERATIONS - 1) as f64);
            let temperature = 1.0_f64 * (0.03_f64).powf(progress);
            let accept = perimeter_delta <= 0
                || rng.next_f64() < (-(perimeter_delta as f64) / temperature).exp();
            if accept {
                current_cells[remove_index] = added;
                selected[added] = true;
                current_perimeter = proposal_perimeter;
                _accepted += 1;
                debug_assert_eq!(self.perimeter_of_cells(&current_cells), current_perimeter);
                if current_perimeter < best_perimeter {
                    best_perimeter = current_perimeter;
                    best_cells.clone_from(&current_cells);
                    _improved += 1;
                }
            } else {
                selected[removed] = true;
            }
        }

        // full validation は best に対して常時実行し、release でも不正候補を出力させない。
        assert!(self.explicit_candidate_is_valid(&best_cells, P, &occ));
        assert!(best_cells.iter().all(|&id| info.component[id] == component));
        assert_eq!(self.perimeter_of_cells(&best_cells), best_perimeter);
        let _reduction = initial_perimeter - best_perimeter;
        let _fee_gain =
            (V as f64) * (compactness(P, best_perimeter) - compactness(P, initial_perimeter));
        local! {
            self.trace.count_by("biased_swap_iteration", _iterations);
            self.trace.count_by("biased_swap_accepted", _accepted);
            self.trace.count_by("biased_swap_improved", _improved);
            self.trace
                .count_by("biased_swap_perimeter_reduction", _reduction as i64);
            self.trace
                .count_by("biased_swap_fee_gain", _fee_gain.round() as i64);
        }

        if best_perimeter < initial_perimeter {
            let mut result = initial;
            result.perimeter = best_perimeter;
            result.explicit_cells = best_cells;
            result
        } else {
            // 同周長の別座標を返すと後続盤面を無用に変えるため baseline をそのまま返す。
            initial
        }
    }

    #[inline]
    fn rows_has(rows: &Rows, id: usize, N: usize) -> bool {
        ((rows[id / N] >> (id % N)) & 1) != 0
    }

    #[inline]
    fn selected_neighbors_in_rows(&self, rows: &Rows, id: usize) -> usize {
        let r = id / self.N;
        let c = id % self.N;
        let mut degree = 0;
        if r > 0 && Self::rows_has(rows, id - self.N, self.N) {
            degree += 1;
        }
        if r + 1 < self.N && Self::rows_has(rows, id + self.N, self.N) {
            degree += 1;
        }
        if c > 0 && Self::rows_has(rows, id - 1, self.N) {
            degree += 1;
        }
        if c + 1 < self.N && Self::rows_has(rows, id + 1, self.N) {
            degree += 1;
        }
        degree
    }

    /// 世代stampと固定長queueで、侵食後のcoreが全て連結かを確認する。
    fn rows_are_connected(
        &self,
        rows: &Rows,
        source_cells: &[usize],
        expected: usize,
        seen: &mut [u32; MAX_N * MAX_N],
        generation: &mut u32,
        queue: &mut [usize; MAX_P],
    ) -> bool {
        *generation = generation.wrapping_add(1);
        if *generation == 0 {
            seen.fill(0);
            *generation = 1;
        }
        let stamp = *generation;
        let Some(start) = source_cells
            .iter()
            .copied()
            .find(|&id| Self::rows_has(rows, id, self.N))
        else {
            return expected == 0;
        };
        let mut head = 0;
        let mut tail = 1;
        queue[0] = start;
        seen[start] = stamp;
        while head < tail {
            let id = queue[head];
            head += 1;
            let r = id / self.N;
            let c = id % self.N;
            for next in [
                (r > 0).then_some(id.wrapping_sub(self.N)),
                (r + 1 < self.N).then_some(id + self.N),
                (c > 0).then_some(id.wrapping_sub(1)),
                (c + 1 < self.N).then_some(id + 1),
            ]
            .into_iter()
            .flatten()
            {
                if Self::rows_has(rows, next, self.N) && seen[next] != stamp {
                    seen[next] = stamp;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        }
        tail == expected
    }

    /// 4/8/16セルを一度に侵食して再成長し、1セル近傍の局所解を越える。
    /// 現在bestから作った提案のうち辞書順改善だけを採るので、初期候補を悪化させない。
    #[allow(clippy::too_many_arguments)]
    fn improve_growth_by_lns(
        &mut self,
        initial: Placement,
        V: i64,
        _recoverable_fee: f64,
        attempt_limit: usize,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
    ) -> Placement {
        local! {
            self.trace.count("growth_lns_eligible");
            self.trace
                .count_by("growth_fee_loss_before", _recoverable_fee.round() as i64);
        }

        let P = initial.explicit_cells.len();
        let component = info.component[initial.explicit_cells[0]];
        let initial_perimeter = initial.perimeter;
        let mut best = initial;
        let mut best_rows = [0_u64; MAX_N];
        for &id in &best.explicit_cells {
            best_rows[id / self.N] |= Self::bit_at(id % self.N);
        }

        let mut seed = (P as u64) ^ (V as u64).rotate_left(17) ^ 0xA076_1D64_78BD_642F;
        for &id in &best.explicit_cells {
            seed = seed.rotate_left(9) ^ (id as u64).wrapping_mul(0xE703_7ED1_A0B4_28DB);
        }
        let mut rng = XorShift64::new(seed);

        // 反復中に使う盤面サイズの領域は全てここで一度だけ確保する。
        let mut connectivity_seen = [0_u32; MAX_N * MAX_N];
        let mut connectivity_generation = 0_u32;
        let mut queue = [0_usize; MAX_P];
        let mut frontier_seen = [0_u32; MAX_N * MAX_N];
        let mut frontier_generation = 0_u32;
        let mut core_cells = [0_usize; MAX_P];
        let mut frontier_cells = [0_usize; MAX_P * 4];
        let mut removed_cells = [0_usize; 16];

        let mut _attempts = 0_i64;
        let mut _batch_cells = 0_i64;
        let mut _improved = 0_i64;

        for attempt in 0..attempt_limit {
            if attempt % 16 == 0 && self.timer.reached(GROWTH_LNS_LIMIT_RATIO) {
                break;
            }
            _attempts += 1;
            let k = GROWTH_LNS_BATCHES[attempt % GROWTH_LNS_BATCHES.len()];
            let mut proposal_rows = best_rows;
            let mut removed_count = 0;

            // 最初はランダム境界、以降は除去済みpatchに隣接する境界だけを外す。
            // 散在した穴が即座に埋め戻されるのを避けつつ、各段階のcore連結性を保つ。
            for removal_step in 0..k {
                let offset = (rng.next_u64() as usize) % P;
                let mut removed = false;
                for scan in 0..P {
                    let id = best.explicit_cells[(offset + scan) % P];
                    if !Self::rows_has(&proposal_rows, id, self.N)
                        || self.selected_neighbors_in_rows(&proposal_rows, id) == 4
                    {
                        continue;
                    }
                    if removal_step > 0 {
                        let r = id / self.N;
                        let c = id % self.N;
                        let touches_patch =
                            removed_cells[..removed_count].iter().any(|&removed_id| {
                                let rr = removed_id / self.N;
                                let rc = removed_id % self.N;
                                r.abs_diff(rr) + c.abs_diff(rc) == 1
                            });
                        if !touches_patch {
                            continue;
                        }
                    }
                    proposal_rows[id / self.N] &= !Self::bit_at(id % self.N);
                    let expected = P - removed_count - 1;
                    if self.rows_are_connected(
                        &proposal_rows,
                        &best.explicit_cells,
                        expected,
                        &mut connectivity_seen,
                        &mut connectivity_generation,
                        &mut queue,
                    ) {
                        removed_cells[removed_count] = id;
                        removed_count += 1;
                        removed = true;
                        break;
                    }
                    proposal_rows[id / self.N] |= Self::bit_at(id % self.N);
                }
                if !removed {
                    break;
                }
            }
            if removed_count != k {
                continue;
            }

            let mut core_count = 0;
            for &id in &best.explicit_cells {
                if Self::rows_has(&proposal_rows, id, self.N) {
                    core_cells[core_count] = id;
                    core_count += 1;
                }
            }

            let mut regrow_ok = true;
            for _ in 0..k {
                frontier_generation = frontier_generation.wrapping_add(1);
                if frontier_generation == 0 {
                    frontier_seen.fill(0);
                    frontier_generation = 1;
                }
                let mut frontier_count = 0;
                for &id in &core_cells[..core_count] {
                    let r = id / self.N;
                    let c = id % self.N;
                    for next in [
                        (r > 0).then_some(id.wrapping_sub(self.N)),
                        (r + 1 < self.N).then_some(id + self.N),
                        (c > 0).then_some(id.wrapping_sub(1)),
                        (c + 1 < self.N).then_some(id + 1),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        let nr = next / self.N;
                        let nc = next % self.N;
                        if !Self::rows_has(&proposal_rows, next, self.N)
                            && frontier_seen[next] != frontier_generation
                            && info.component[next] == component
                            && self.is_free(occ, nr, nc)
                        {
                            frontier_seen[next] = frontier_generation;
                            frontier_cells[frontier_count] = next;
                            frontier_count += 1;
                        }
                    }
                }
                if frontier_count == 0 {
                    regrow_ok = false;
                    break;
                }

                // 共有辺数を最優先し、同数なら既存weightと小さな乱択で形の固定化を避ける。
                let mut chosen = frontier_cells[0];
                let mut chosen_degree = self.selected_neighbors_in_rows(&proposal_rows, chosen);
                let mut chosen_score = weights.cell[chosen] + 0.35 * rng.next_f64();
                for &id in &frontier_cells[1..frontier_count] {
                    let degree = self.selected_neighbors_in_rows(&proposal_rows, id);
                    let score = weights.cell[id] + 0.35 * rng.next_f64();
                    if degree > chosen_degree || (degree == chosen_degree && score > chosen_score) {
                        chosen = id;
                        chosen_degree = degree;
                        chosen_score = score;
                    }
                }
                proposal_rows[chosen / self.N] |= Self::bit_at(chosen % self.N);
                core_cells[core_count] = chosen;
                core_count += 1;
            }
            if !regrow_ok || core_count != P {
                continue;
            }
            _batch_cells += k as i64;

            let mut proposal_perimeter = 0;
            let mut proposal_cheap = 0.0;
            for &id in &core_cells[..P] {
                proposal_perimeter += 4 - self.selected_neighbors_in_rows(&proposal_rows, id);
                proposal_cheap += weights.cell[id];
            }
            if proposal_perimeter < best.perimeter
                || (proposal_perimeter == best.perimeter && proposal_cheap > best.cheap_score)
            {
                best.perimeter = proposal_perimeter;
                best.cheap_score = proposal_cheap;
                best.explicit_cells.clear();
                best.explicit_cells.extend_from_slice(&core_cells[..P]);
                best_rows = proposal_rows;
                _improved += 1;
            }
        }

        let _fee_loss_after = (V as f64)
            * (compactness(P, minimum_perimeter(P)) - compactness(P, best.perimeter)).max(0.0);
        let _reduction = initial_perimeter.saturating_sub(best.perimeter);
        local! {
            self.trace.count_by("growth_lns_attempt", _attempts);
            self.trace
                .count_by("growth_lns_batch_cells", _batch_cells);
            self.trace.count_by("growth_lns_improved", _improved);
            self.trace
                .count_by("growth_lns_perimeter_reduction", _reduction as i64);
            self.trace
                .count_by("growth_fee_loss_after", _fee_loss_after.round() as i64);
            if P >= 96 {
                self.trace.count_by("growth_lns_attempt_p96", _attempts);
                self.trace
                    .count_by("growth_lns_perimeter_reduction_p96", _reduction as i64);
            } else {
                self.trace
                    .count_by("growth_lns_attempt_p64_95", _attempts);
                self.trace.count_by(
                    "growth_lns_perimeter_reduction_p64_95",
                    _reduction as i64,
                );
            }
        }
        best
    }

    /// 共有辺数 d を最優先する成長キー。性質2 (L += 4−2d) より、d 最大のセル追加が
    /// 周長増加を最小化する。同じ d のときだけ、元自由成分の切断損失を先に比較する。
    #[inline]
    fn growth_key(
        &self,
        d: i64,
        id: usize,
        seed_r: usize,
        seed_c: usize,
        weights: &WeightData,
        cut_loss: &[usize],
    ) -> (i64, usize, usize, usize, i64, usize) {
        let r = id / self.N;
        let c = id % self.N;
        let ring = r.abs_diff(seed_r).max(c.abs_diff(seed_c));
        let manhattan = r.abs_diff(seed_r) + c.abs_diff(seed_c);
        let attraction = (weights.cell[id] * 30.0).round() as i64;
        (-d, cut_loss[id], ring, manhattan, -attraction, id)
    }

    /// id の 4 近傍のうち selected なセル数 (= 追加時の共有辺数 d)。
    #[inline]
    fn count_selected_neighbors(&self, id: usize, selected: &[bool]) -> i64 {
        let r = id / self.N;
        let c = id % self.N;
        let mut d = 0;
        if r > 0 && selected[id - self.N] {
            d += 1;
        }
        if r + 1 < self.N && selected[id + self.N] {
            d += 1;
        }
        if c > 0 && selected[id - 1] {
            d += 1;
        }
        if c + 1 < self.N && selected[id + 1] {
            d += 1;
        }
        d
    }

    #[inline]
    fn rectangle_sum_i32(
        prefix: &[[i32; MAX_N + 1]; MAX_N + 1],
        x: usize,
        y: usize,
        h: usize,
        w: usize,
    ) -> i32 {
        prefix[x + h][y + w] - prefix[x][y + w] - prefix[x + h][y] + prefix[x][y]
    }

    #[inline]
    fn rectangle_sum_f64(
        prefix: &[[f64; MAX_N + 1]; MAX_N + 1],
        x: usize,
        y: usize,
        h: usize,
        w: usize,
    ) -> f64 {
        prefix[x + h][y + w] - prefix[x][y + w] - prefix[x + h][y] + prefix[x][y]
    }

    /// 矩形内の連結成分を、外部に接する非関節点だけ削ってちょうど P セルにする。
    /// 内部穴に接するだけのセルを削らないことで、侵食が穴を広げて周長を悪化させるのを避ける。
    fn erode_box_component(
        &self,
        component_cells: &[usize],
        P: usize,
        window: BoxWindow,
        weights: &WeightData,
    ) -> Option<Vec<usize>> {
        if component_cells.len() < P {
            return None;
        }
        let mut selected = vec![false; self.N * self.N];
        for &id in component_cells {
            selected[id] = true;
        }
        let frame_h = window.h + 2;
        let frame_w = window.w + 2;
        let mut selected_count = component_cells.len();

        while selected_count > P {
            // 1セルの枠から補集合を flood fill し、現在の外部領域を明示する。
            let mut external = vec![false; frame_h * frame_w];
            let mut queue = Vec::with_capacity(frame_h * frame_w);
            external[0] = true;
            queue.push(0_usize);
            let mut head = 0;
            while head < queue.len() {
                let pos = queue[head];
                head += 1;
                let fr = pos / frame_w;
                let fc = pos % frame_w;
                for next in [
                    (fr > 0).then_some(pos - frame_w),
                    (fr + 1 < frame_h).then_some(pos + frame_w),
                    (fc > 0).then_some(pos - 1),
                    (fc + 1 < frame_w).then_some(pos + 1),
                ]
                .into_iter()
                .flatten()
                {
                    if external[next] {
                        continue;
                    }
                    let nr = next / frame_w;
                    let nc = next % frame_w;
                    let is_selected = if nr > 0 && nr + 1 < frame_h && nc > 0 && nc + 1 < frame_w {
                        let r = window.x + nr - 1;
                        let c = window.y + nc - 1;
                        selected[r * self.N + c]
                    } else {
                        false
                    };
                    if !is_selected {
                        external[next] = true;
                        queue.push(next);
                    }
                }
            }

            let center_r2 = 2 * window.x + window.h - 1;
            let center_c2 = 2 * window.y + window.w - 1;
            let mut peel = Vec::new();
            for &id in component_cells {
                if !selected[id] {
                    continue;
                }
                let r = id / self.N;
                let c = id % self.N;
                let lr = r - window.x + 1;
                let lc = c - window.y + 1;
                let frame_pos = lr * frame_w + lc;
                let touches_external = external[frame_pos - frame_w]
                    || external[frame_pos + frame_w]
                    || external[frame_pos - 1]
                    || external[frame_pos + 1];
                if touches_external {
                    let degree = self.count_selected_neighbors(id, &selected);
                    let distance = (2 * r).abs_diff(center_r2) + (2 * c).abs_diff(center_c2);
                    peel.push((id, degree, distance));
                }
            }
            peel.sort_unstable_by(|&(a, degree_a, distance_a), &(b, degree_b, distance_b)| {
                degree_a
                    .cmp(&degree_b)
                    .then_with(|| weights.cell[a].total_cmp(&weights.cell[b]))
                    .then_with(|| distance_b.cmp(&distance_a))
                    .then_with(|| a.cmp(&b))
            });

            let mut removed = false;
            for (id, _, _) in peel {
                selected[id] = false;
                let start = component_cells
                    .iter()
                    .copied()
                    .find(|&other| selected[other])
                    .expect("P>=36 leaves a selected cell");
                if self.selected_is_connected(&selected, start, selected_count - 1) {
                    selected_count -= 1;
                    removed = true;
                    break;
                }
                selected[id] = true;
            }
            if !removed {
                return None;
            }
        }

        Some(
            component_cells
                .iter()
                .copied()
                .filter(|&id| selected[id])
                .collect(),
        )
    }

    fn explicit_candidate_is_valid(&self, cells: &[usize], P: usize, occ: &Rows) -> bool {
        if cells.len() != P {
            return false;
        }
        let mut selected = vec![false; self.N * self.N];
        for &id in cells {
            if id >= self.N * self.N || selected[id] {
                return false;
            }
            let r = id / self.N;
            let c = id % self.N;
            if !self.is_free(occ, r, c) {
                return false;
            }
            selected[id] = true;
        }
        self.selected_is_connected(&selected, cells[0], P)
    }

    /// P に近い面積の矩形窓を全走査し、各寸法の有望窓だけを局所 flood fill する。
    /// prefix sum により窓の列挙は O(N^2)、高価な侵食は寸法ごと最大4窓に限定する。
    fn box_growth_candidate(
        &mut self,
        P: usize,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
    ) -> Option<Placement> {
        if P < 36 {
            return None;
        }
        local! {
            self.trace.count("box_growth_turn");
        }

        let mut free_prefix = [[0_i32; MAX_N + 1]; MAX_N + 1];
        let mut horizontal_prefix = [[0_i32; MAX_N + 1]; MAX_N + 1];
        let mut vertical_prefix = [[0_i32; MAX_N + 1]; MAX_N + 1];
        let mut weight_prefix = [[0.0_f64; MAX_N + 1]; MAX_N + 1];
        for r in 0..self.N {
            for c in 0..self.N {
                let free = i32::from(self.is_free(occ, r, c));
                let horizontal = i32::from(
                    c + 1 < self.N && self.is_free(occ, r, c) && self.is_free(occ, r, c + 1),
                );
                let vertical = i32::from(
                    r + 1 < self.N && self.is_free(occ, r, c) && self.is_free(occ, r + 1, c),
                );
                free_prefix[r + 1][c + 1] =
                    free + free_prefix[r][c + 1] + free_prefix[r + 1][c] - free_prefix[r][c];
                horizontal_prefix[r + 1][c + 1] =
                    horizontal + horizontal_prefix[r][c + 1] + horizontal_prefix[r + 1][c]
                        - horizontal_prefix[r][c];
                vertical_prefix[r + 1][c + 1] =
                    vertical + vertical_prefix[r][c + 1] + vertical_prefix[r + 1][c]
                        - vertical_prefix[r][c];
                weight_prefix[r + 1][c + 1] = weights.cell[r * self.N + c]
                    + weight_prefix[r][c + 1]
                    + weight_prefix[r + 1][c]
                    - weight_prefix[r][c];
            }
        }

        let min_L = minimum_perimeter(P);
        let mut shortlisted = Vec::new();
        let mut _scanned = 0_i64;
        for h in 1..=self.N {
            for w in 1..=self.N {
                let area = h * w;
                if area < P || area > P + 24 || 2 * (h + w) > min_L + 2 {
                    continue;
                }
                let short = h.min(w);
                let long = h.max(w);
                if 2 * long > 3 * short {
                    continue;
                }
                let mut best_for_dimension = Vec::with_capacity(5);
                for x in 0..=self.N - h {
                    for y in 0..=self.N - w {
                        _scanned += 1;
                        let free_count = Self::rectangle_sum_i32(&free_prefix, x, y, h, w) as usize;
                        if free_count < P {
                            continue;
                        }
                        let horizontal_edges = if w >= 2 {
                            Self::rectangle_sum_i32(&horizontal_prefix, x, y, h, w - 1)
                        } else {
                            0
                        };
                        let vertical_edges = if h >= 2 {
                            Self::rectangle_sum_i32(&vertical_prefix, x, y, h - 1, w)
                        } else {
                            0
                        };
                        let free_perimeter =
                            4 * (free_count as i32) - 2 * (horizontal_edges + vertical_edges);
                        let perimeter_lower_bound = free_perimeter - 2 * ((free_count - P) as i32);
                        let candidate = BoxWindow {
                            x,
                            y,
                            h,
                            w,
                            free_count,
                            perimeter_lower_bound,
                            weight_sum: Self::rectangle_sum_f64(&weight_prefix, x, y, h, w),
                        };
                        best_for_dimension.push(candidate);
                        best_for_dimension.sort_unstable_by(|a, b| {
                            a.perimeter_lower_bound
                                .cmp(&b.perimeter_lower_bound)
                                .then_with(|| {
                                    (a.h * a.w - a.free_count).cmp(&(b.h * b.w - b.free_count))
                                })
                                .then_with(|| b.weight_sum.total_cmp(&a.weight_sum))
                                .then_with(|| a.x.cmp(&b.x))
                                .then_with(|| a.y.cmp(&b.y))
                        });
                        best_for_dimension.truncate(4);
                    }
                }
                shortlisted.extend(best_for_dimension);
            }
        }
        local! {
            self.trace.count_by("box_position_scanned", _scanned);
        }

        let mut best: Option<Placement> = None;
        let mut seen_window = HashSet::new();
        for window in shortlisted {
            if !seen_window.insert((window.x, window.y, window.h, window.w)) {
                continue;
            }
            let mut visited = vec![false; self.N * self.N];
            for r in window.x..window.x + window.h {
                for c in window.y..window.y + window.w {
                    let start = r * self.N + c;
                    if visited[start] || !self.is_free(occ, r, c) {
                        continue;
                    }
                    let mut component = Vec::with_capacity(window.free_count);
                    let mut queue = Vec::with_capacity(window.free_count);
                    visited[start] = true;
                    queue.push(start);
                    let mut head = 0;
                    while head < queue.len() {
                        let id = queue[head];
                        head += 1;
                        component.push(id);
                        let cr = id / self.N;
                        let cc = id % self.N;
                        for next in [
                            (cr > window.x).then_some(id - self.N),
                            (cr + 1 < window.x + window.h).then_some(id + self.N),
                            (cc > window.y).then_some(id - 1),
                            (cc + 1 < window.y + window.w).then_some(id + 1),
                        ]
                        .into_iter()
                        .flatten()
                        {
                            let nr = next / self.N;
                            let nc = next % self.N;
                            if !visited[next] && self.is_free(occ, nr, nc) {
                                visited[next] = true;
                                queue.push(next);
                            }
                        }
                    }
                    if component.len() < P {
                        continue;
                    }
                    local! {
                        self.trace.count("box_local_component_pass");
                        self.trace.count("box_erosion_attempt");
                    }
                    let Some(cells) = self.erode_box_component(&component, P, window, weights)
                    else {
                        continue;
                    };
                    local! {
                        self.trace.count("box_erosion_success");
                    }
                    let valid = self.explicit_candidate_is_valid(&cells, P, occ);
                    local! {
                        if valid {
                            self.trace.count("box_candidate_valid");
                        } else {
                            self.trace.count("box_candidate_invalid");
                        }
                    }
                    if !valid {
                        continue;
                    }
                    let perimeter = self.perimeter_of_cells(&cells);
                    let cheap_score = cells.iter().map(|&id| weights.cell[id]).sum();
                    let component_id = info.component[cells[0]];
                    debug_assert!(component_id >= 0);
                    let candidate = Placement {
                        perimeter,
                        cheap_score,
                        component_size: info.sizes[component_id as usize],
                        explicit_cells: cells,
                        ..Placement::default()
                    };
                    if best.as_ref().is_none_or(|current| {
                        candidate.perimeter < current.perimeter
                            || (candidate.perimeter == current.perimeter
                                && candidate.cheap_score > current.cheap_score)
                    }) {
                        best = Some(candidate);
                    }
                }
            }
        }
        best
    }

    /// 隣接セルを現時点の共有辺数 d で frontier へ入れる。隣が選択されるたびに
    /// 呼ばれ、同じセルが d の増加ごとに重複 push される (decrease-key の再挿入方式)。
    /// 古いエントリは pop 側で d 不一致により捨てる。
    #[allow(clippy::too_many_arguments)]
    fn push_growth_cell(
        &self,
        id: usize,
        component: isize,
        seed_r: usize,
        seed_c: usize,
        info: &FreeInfo,
        weights: &WeightData,
        cut_loss: &[usize],
        selected: &[bool],
        frontier: &mut BinaryHeap<Reverse<(i64, usize, usize, usize, i64, usize)>>,
    ) {
        if selected[id] || info.component[id] != component {
            return;
        }
        let d = self.count_selected_neighbors(id, selected);
        let key = self.growth_key(d, id, seed_r, seed_c, weights, cut_loss);
        frontier.push(Reverse(key));
    }

    /// 規則形状が置けないときだけ、同一成分内を中心から成長させて連結形状を作る。
    /// これは失敗を隠す経路ではなく、非矩形な空き領域を利用する明示的な第二配置法である。
    fn growth_placement(
        &mut self,
        P: usize,
        V: i64,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
        maximum_perimeter: usize,
        seed_limit: usize,
    ) -> Option<Placement> {
        local! {
            self.trace.count("growth_placement_attempt");
        }
        let _cutloss_started = Instant::now();
        let free: Vec<bool> = info
            .component
            .iter()
            .map(|&component| component >= 0)
            .collect();
        let cut_loss = free_cut_loss(self.N, &free);
        local! {
            self.trace.count("growth_cutloss_turn");
            if P >= 96 {
                self.trace.count("growth_cutloss_turn_p96");
            } else if P >= 64 {
                self.trace.count("growth_cutloss_turn_p64_95");
            } else {
                self.trace.count("growth_cutloss_turn_p_lt64");
            }
            self.trace.count_by(
                "growth_cutloss_positive_cell",
                cut_loss.iter().filter(|&&loss| loss > 0).count() as i64,
            );
            self.trace.count_by(
                "growth_cutloss_sum",
                cut_loss.iter().sum::<usize>() as i64,
            );
            self.trace.count_by(
                "growth_cutloss_max",
                cut_loss.iter().copied().max().unwrap_or(0) as i64,
            );
            self.trace.add_time_ms(
                "growth_cutloss",
                _cutloss_started.elapsed().as_secs_f64() * 1000.0,
            );
        }
        let mut seeds = Vec::new();
        let mut used_seed = vec![false; self.N * self.N];
        for component_id in 0..info.sizes.len() {
            if info.sizes[component_id] < P || component_id >= info.cells.len() {
                continue;
            }
            let list = &info.cells[component_id];
            let mut weight_best1: Option<usize> = None;
            let mut weight_best2: Option<usize> = None;
            let mut best1: Option<usize> = None;
            let mut best2: Option<usize> = None;
            for &id in list {
                if weight_best1.is_none() || weights.cell[id] > weights.cell[weight_best1.unwrap()]
                {
                    weight_best2 = weight_best1;
                    weight_best1 = Some(id);
                } else if weight_best2.is_none()
                    || weights.cell[id] > weights.cell[weight_best2.unwrap()]
                {
                    weight_best2 = Some(id);
                }

                let better = |a: usize, b: usize| {
                    cut_loss[a]
                        .cmp(&cut_loss[b])
                        .then_with(|| weights.cell[b].total_cmp(&weights.cell[a]))
                        .then_with(|| a.cmp(&b))
                        .is_lt()
                };
                if best1.is_none() || better(id, best1.unwrap()) {
                    best2 = best1;
                    best1 = Some(id);
                } else if best2.is_none() || better(id, best2.unwrap()) {
                    best2 = Some(id);
                }
            }
            local! {
                if [best1, best2] != [weight_best1, weight_best2] {
                    self.trace.count("growth_cutloss_seed_changed");
                }
            }
            let mut local_seeds = Vec::new();
            if let Some(id) = best1 {
                local_seeds.push(id);
            }
            if let Some(id) = best2 {
                local_seeds.push(id);
            }
            const SAMPLES: usize = 6;
            for k in 0..SAMPLES {
                let idx = k * list.len().saturating_sub(1) / (SAMPLES - 1);
                local_seeds.push(list[idx]);
            }
            for id in local_seeds {
                if !used_seed[id] {
                    used_seed[id] = true;
                    seeds.push(id);
                }
            }
        }
        seeds.truncate(seed_limit);

        let mut candidates = Vec::with_capacity(seeds.len());
        let mut _positive_selected = 0_i64;
        let mut _selected_loss_sum = 0_i64;
        let mut _selected_loss_max = 0_i64;
        const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for seed in seeds {
            let component = info.component[seed];
            if component < 0 || info.sizes[component as usize] < P {
                continue;
            }
            let seed_r = seed / self.N;
            let seed_c = seed % self.N;
            let mut selected = vec![false; self.N * self.N];
            let mut region = Vec::with_capacity(P);
            let mut frontier = BinaryHeap::new();

            selected[seed] = true;
            region.push(seed);
            local! {
                if cut_loss[seed] > 0 {
                    _positive_selected += 1;
                }
                _selected_loss_sum += cut_loss[seed] as i64;
                _selected_loss_max = _selected_loss_max.max(cut_loss[seed] as i64);
            }
            for (dr, dc) in DIRS {
                let nr = (seed_r as isize) + dr;
                let nc = (seed_c as isize) + dc;
                if nr >= 0 && nr < self.N as isize && nc >= 0 && nc < self.N as isize {
                    self.push_growth_cell(
                        (nr as usize) * self.N + (nc as usize),
                        component,
                        seed_r,
                        seed_c,
                        info,
                        weights,
                        &cut_loss,
                        &selected,
                        &mut frontier,
                    );
                }
            }
            while region.len() < P {
                let Some(Reverse((neg_d, _, _, _, _, id))) = frontier.pop() else {
                    break;
                };
                let d_recorded = -neg_d;
                if selected[id] {
                    continue;
                }
                // d が増えたセルは選択時 push で最新エントリが別に入っているため、
                // 古い d のエントリは捨てる。これで常に現時点の d 最大が採用される。
                if self.count_selected_neighbors(id, &selected) != d_recorded {
                    continue;
                }
                selected[id] = true;
                region.push(id);
                local! {
                    if cut_loss[id] > 0 {
                        _positive_selected += 1;
                    }
                    _selected_loss_sum += cut_loss[id] as i64;
                    _selected_loss_max = _selected_loss_max.max(cut_loss[id] as i64);
                }
                let r = id / self.N;
                let c = id % self.N;
                for (dr, dc) in DIRS {
                    let nr = (r as isize) + dr;
                    let nc = (c as isize) + dc;
                    if nr >= 0 && nr < self.N as isize && nc >= 0 && nc < self.N as isize {
                        self.push_growth_cell(
                            (nr as usize) * self.N + (nc as usize),
                            component,
                            seed_r,
                            seed_c,
                            info,
                            weights,
                            &cut_loss,
                            &selected,
                            &mut frontier,
                        );
                    }
                }
            }
            if region.len() != P {
                continue;
            }
            let perimeter = self.perimeter_of_cells(&region);
            if perimeter > maximum_perimeter {
                continue;
            }
            let cheap_score = region.iter().map(|&id| weights.cell[id]).sum();
            candidates.push(Placement {
                perimeter,
                cheap_score,
                component_size: info.sizes[component as usize],
                explicit_cells: region,
                ..Placement::default()
            });
        }
        local! {
            self.trace
                .count_by("growth_cutloss_positive_selected", _positive_selected);
            self.trace
                .count_by("growth_cutloss_selected_sum", _selected_loss_sum);
            self.trace
                .count_by("growth_cutloss_selected_max", _selected_loss_max);
        }

        // 既存 seed-growth と同列の候補として追加し、この後の SA/LNS と最終評価は共通化する。
        let mut _box_index = None;
        if P >= 36 {
            let _seed_best_perimeter = candidates.iter().map(|candidate| candidate.perimeter).min();
            if let Some(box_candidate) = self.box_growth_candidate(P, occ, info, weights) {
                local! {
                    self.trace.count("box_candidate_added");
                    if let Some(seed_perimeter) = _seed_best_perimeter {
                        self.trace.count_by(
                            "box_perimeter_delta_vs_seed",
                            box_candidate.perimeter as i64 - seed_perimeter as i64,
                        );
                    }
                }
                _box_index = Some(candidates.len());
                candidates.push(box_candidate);
            }
        }

        // v019と同じ上位候補・順序・予算で、まず基準機構の1セルSAを適用する。
        let mut improvement_order: Vec<usize> = (0..candidates.len()).collect();
        improvement_order.sort_by(|&a, &b| {
            candidates[a]
                .perimeter
                .cmp(&candidates[b].perimeter)
                .then_with(|| {
                    candidates[b]
                        .cheap_score
                        .total_cmp(&candidates[a].cheap_score)
                })
        });
        improvement_order.truncate(GROWTH_SA_CANDIDATES);
        local! {
            if _box_index.is_some_and(|index| improvement_order.contains(&index)) {
                self.trace.count("box_candidate_entered_sa");
            }
        }
        for &index in &improvement_order {
            let improved = self.improve_growth_by_sa(candidates[index].clone(), occ, info, weights);
            candidates[index] = improved;
        }

        // 同じ候補にだけSA後の周長からfeeを計算し、小予算LNSを追加する。
        let min_L = minimum_perimeter(P);
        for index in improvement_order {
            if P < 64 || candidates[index].perimeter < min_L + 8 {
                continue;
            }
            let recoverable_fee = (V as f64)
                * (compactness(P, min_L) - compactness(P, candidates[index].perimeter)).max(0.0);
            if recoverable_fee < GROWTH_LNS_MIN_RECOVERABLE_FEE {
                continue;
            }
            let base_attempts = if P >= 96 {
                GROWTH_LNS_ATTEMPTS_LARGE
            } else {
                GROWTH_LNS_ATTEMPTS_MID
            };
            let fee_bonus_steps = ((recoverable_fee / 25_000.0) as usize).min(2);
            let attempt_limit = base_attempts + fee_bonus_steps * (base_attempts / 4);
            let improved = self.improve_growth_by_lns(
                candidates[index].clone(),
                V,
                recoverable_fee,
                attempt_limit,
                occ,
                info,
                weights,
            );
            candidates[index] = improved;
        }

        let best_perimeter = candidates.iter().map(|p| p.perimeter).min()?;
        let mut best: Option<(usize, Placement)> = None;
        for (index, mut candidate) in candidates.into_iter().enumerate() {
            if candidate.perimeter > best_perimeter + 2 {
                continue;
            }
            let mut next = *occ;
            for &id in &candidate.explicit_cells {
                next[id / self.N] |= Self::bit_at(id % self.N);
            }
            let delta = self.fragment_metric(&next) - info.metric;
            local! {
                self.trace.count("fragment_evaluated");
            }
            candidate.final_score = candidate.cheap_score - 1.4 * delta;
            if best
                .as_ref()
                .is_none_or(|(_, current)| candidate.final_score > current.final_score)
            {
                best = Some((index, candidate));
            }
        }
        if let Some((_index, _candidate)) = &best {
            local! {
                self.trace.count("growth_placement_success");
                let _slack = _candidate.perimeter - minimum_perimeter(P);
                self.trace.count_by(
                    "growth_slack_sum",
                    _slack as i64,
                );
                if _slack >= 8 {
                    self.trace.count("growth_cutloss_selected_slack8_plus");
                } else if _slack >= 4 {
                    self.trace.count("growth_cutloss_selected_slack4_6");
                } else {
                    self.trace.count("growth_cutloss_selected_slack0_2");
                }
                if _box_index.is_some_and(|box_index| box_index == *_index) {
                    self.trace.count("box_candidate_selected");
                }
            }
        }
        best.map(|(_, candidate)| candidate)
    }

    /// 最初に置ける周長レベルは現行どおり固定し、候補0に現行評価最大を置く。
    /// 非 fast mode では、同じ component_size 内の局所重み最大・断片化増分最小も
    /// 重複除去して返す。同じ component_size なので admission 閾値は変わらない。
    fn find_normal_placements(
        &mut self,
        incoming_id: usize,
        P: usize,
        incoming_V: i64,
        incoming_S: usize,
        incoming_T: usize,
        theta: f64,
        is_large_target: bool,
        fast_mode: bool,
    ) -> Option<Vec<Placement>> {
        let occ = self.occupied_rows;
        let info = self.compute_free_info(&occ, true);
        if !info.sizes.iter().any(|&size| size >= P) {
            return None;
        }
        let weights =
            self.build_weight_data(&occ, &info, incoming_S, incoming_T, incoming_T - incoming_S);
        let runs = self.build_run_table(&occ);
        let min_L = minimum_perimeter(P);
        let max_extra = if fast_mode { 2 } else { 6 };
        let top_count = if fast_mode { 7 } else { 20 };

        for perimeter in (min_L..=min_L + max_extra).step_by(2) {
            let candidates = self.scan_regular_level(
                P,
                &runs,
                &weights,
                perimeter,
                top_count,
                if fast_mode { 5 } else { usize::MAX },
            );
            if candidates.is_empty() {
                continue;
            }
            // regular候補が存在すると確定してから作り、growth経路の時間予算を変えない。
            let slot_calendar = if is_large_target || fast_mode {
                None
            } else {
                Some(self.build_slot_calendar(incoming_id, incoming_S, theta))
            };
            let mut evaluated: Vec<(Placement, f64)> = Vec::with_capacity(candidates.len());
            let mut current_index: Option<usize> = None;
            for mut candidate in candidates {
                let cells = self.materialize(&candidate, P);
                let component_id = info.component[cells[0]];
                candidate.component_size = if component_id >= 0 {
                    info.sizes[component_id as usize]
                } else {
                    0
                };
                if let Some(calendar) = slot_calendar
                    .as_ref()
                    .filter(|calendar| !calendar.slots.is_empty())
                {
                    let shape = &self.shapes_by_p[P][candidate.shape_index];
                    candidate.slot_delay = self.regular_slot_delay(
                        shape,
                        candidate.x,
                        candidate.y,
                        incoming_T,
                        calendar,
                    );
                    candidate.slot_penalty = Self::slot_penalty(calendar, candidate.slot_delay);
                    candidate.slot_count = calendar.slots.len();
                }
                let fragment_delta = if fast_mode {
                    candidate.final_score = candidate.cheap_score;
                    0.0
                } else {
                    let mut next = occ;
                    for &id in &cells {
                        next[id / self.N] |= Self::bit_at(id % self.N);
                    }
                    let delta = self.fragment_metric(&next) - info.metric;
                    local! {
                        self.trace.count("fragment_evaluated");
                    }
                    candidate.final_score = candidate.cheap_score - 1.4 * delta;
                    delta
                };
                candidate.explicit_cells = cells;
                if current_index
                    .is_none_or(|index| candidate.final_score > evaluated[index].0.final_score)
                {
                    current_index = Some(evaluated.len());
                }
                evaluated.push((candidate, fragment_delta));
            }
            let current_index = current_index.expect("non-empty regular candidates");
            if fast_mode {
                return Some(vec![evaluated.swap_remove(current_index).0]);
            }
            let component_size = evaluated[current_index].0.component_size;
            let mut cheap_index = current_index;
            let mut fragment_index = current_index;
            for i in 0..evaluated.len() {
                if evaluated[i].0.component_size != component_size {
                    continue;
                }
                if evaluated[i].0.cheap_score > evaluated[cheap_index].0.cheap_score {
                    cheap_index = i;
                }
                if evaluated[i].1 < evaluated[fragment_index].1 {
                    fragment_index = i;
                }
            }

            let mut choices = Vec::with_capacity(4);
            for index in [current_index, cheap_index, fragment_index] {
                let candidate = &evaluated[index].0;
                if !choices
                    .iter()
                    .any(|choice: &Placement| choice.explicit_cells == candidate.explicit_cells)
                {
                    choices.push(candidate.clone());
                }
            }

            if let Some(calendar) = slot_calendar
                .as_ref()
                .filter(|calendar| !calendar.slots.is_empty())
            {
                let current_component = info.component[choices[0].explicit_cells[0]] as usize;
                let slot_best = self.scan_regular_slot_best(
                    P, &runs, &weights, &info, perimeter, incoming_T, calendar,
                );
                if let Some(mut candidate) = slot_best[current_component].clone() {
                    candidate.explicit_cells = self.materialize(&candidate, P);
                    if !choices
                        .iter()
                        .any(|choice| choice.explicit_cells == candidate.explicit_cells)
                    {
                        choices.push(candidate);
                        local! {
                            self.trace.count("slot_candidate_added");
                        }
                    }
                }
            }

            let release_eligible = P <= RELEASE_ATLAS_P_MAX
                && incoming_T - incoming_S >= RELEASE_ATLAS_D_MIN
                && !self.timer.reached(RELEASE_ATLAS_LIMIT_RATIO);
            if release_eligible {
                local! {
                    self.trace.count("release_atlas_eligible");
                }
                let atlas = self.build_release_atlas(incoming_S, incoming_T);
                for choice in &mut choices {
                    choice.release_score =
                        self.regular_release_score(&atlas, &choice.explicit_cells);
                    choice.release_current_largest =
                        self.regular_current_largest(&choice.explicit_cells);
                    choice.release_snapshot_count = RELEASE_ATLAS_SNAPSHOTS;
                }
                let current_component = info.component[choices[0].explicit_cells[0]] as usize;
                if let Some(mut candidate) = self.scan_regular_release_best(
                    P,
                    &runs,
                    &weights,
                    &info,
                    perimeter,
                    current_component,
                    &atlas,
                ) {
                    if let Some(calendar) = slot_calendar
                        .as_ref()
                        .filter(|calendar| !calendar.slots.is_empty())
                    {
                        let shape = &self.shapes_by_p[P][candidate.shape_index];
                        candidate.slot_delay = self.regular_slot_delay(
                            shape,
                            candidate.x,
                            candidate.y,
                            incoming_T,
                            calendar,
                        );
                        candidate.slot_penalty = Self::slot_penalty(calendar, candidate.slot_delay);
                        candidate.slot_count = calendar.slots.len();
                    }
                    debug_assert_eq!(candidate.perimeter, choices[0].perimeter);
                    debug_assert_eq!(candidate.component_size, choices[0].component_size);
                    if let Some(existing) = choices
                        .iter_mut()
                        .find(|choice| choice.explicit_cells == candidate.explicit_cells)
                    {
                        existing.release_candidate = true;
                        local! {
                            self.trace.count("release_atlas_candidate_duplicate");
                        }
                    } else {
                        candidate.release_added_candidate = true;
                        choices.push(candidate);
                        local! {
                            self.trace.count("release_atlas_candidate_added");
                        }
                    }
                }
            }
            if slot_calendar
                .as_ref()
                .is_none_or(|calendar| calendar.slots.is_empty())
            {
                debug_assert!(choices.iter().all(|placement| {
                    placement.slot_delay == 0
                        && placement.slot_penalty == 0.0
                        && placement.slot_count == 0
                }));
            }
            if is_large_target {
                local! {
                    self.trace.count("large_target_regular");
                }
            }
            return Some(choices);
        }

        if fast_mode || self.timer.reached(GROWTH_LIMIT_RATIO) {
            return None;
        }
        let growth = self
            .growth_placement(P, incoming_V, &occ, &info, &weights, usize::MAX, 44)
            .map(|placement| vec![placement]);
        if is_large_target && growth.is_some() {
            local! {
                self.trace.count("large_target_growth");
            }
        }
        growth
    }

    fn posterior_theta(&self) -> f64 {
        if self.duration_count == 0 {
            return 5_000.0;
        }
        const GRID: usize = 121;
        let mut log_weight = [0.0; GRID];
        let mut max_log = -1e300_f64;
        for (k, value) in log_weight.iter_mut().enumerate() {
            let theta = 2_000.0 + 50.0 * (k as f64);
            *value = -(self.duration_count as f64) * theta.ln() - self.duration_sum / theta;
            max_log = max_log.max(*value);
        }
        let mut sum_w = 0.0;
        let mut sum_theta = 0.0;
        for (k, value) in log_weight.iter().enumerate() {
            let theta = 2_000.0 + 50.0 * (k as f64);
            let weight = (*value - max_log).exp();
            sum_w += weight;
            sum_theta += weight * theta;
        }
        sum_theta / sum_w
    }

    fn lognormal_survival(threshold: f64) -> f64 {
        if threshold <= 0.0 {
            return 1.0;
        }
        const SIGMA: f64 = 0.8 * std::f64::consts::LN_2;
        0.5 * erfc(threshold.ln() / (SIGMA * std::f64::consts::SQRT_2))
    }

    fn accepted_load_fraction(q_threshold: f64) -> f64 {
        const STEPS: usize = 160;
        const END: f64 = 16.0;
        const H: f64 = END / (STEPS as f64);
        let mut sum = 0.0;
        for k in 0..=STEPS {
            let x = H * (k as f64);
            let value = if x > 0.0 {
                let threshold = q_threshold * x.powf(0.1);
                x * (-x).exp() * Self::lognormal_survival(threshold)
            } else {
                0.0
            };
            let coefficient = if k == 0 || k == STEPS {
                1
            } else if k % 2 == 1 {
                4
            } else {
                2
            };
            sum += (coefficient as f64) * value;
        }
        sum * H / 3.0
    }

    fn q_threshold_for_fraction(&mut self, fraction: f64) -> f64 {
        if fraction >= 0.9995 {
            return 0.0;
        }
        let key = ((fraction * 1_000.0).round() as i32).clamp(0, 999);
        if let Some(&result) = self.threshold_cache.get(&key) {
            return result;
        }
        let target = (key as f64) / 1_000.0;
        let mut low = 0.0;
        let mut high = 16.0;
        while Self::accepted_load_fraction(high) > target {
            high *= 2.0;
        }
        for _ in 0..34 {
            let mid = (low + high) * 0.5;
            if Self::accepted_load_fraction(mid) > target {
                low = mid;
            } else {
                high = mid;
            }
        }
        let result = (low + high) * 0.5;
        self.threshold_cache.insert(key, result);
        result
    }

    fn occupied_count(&self) -> usize {
        self.occupied_rows[..self.N]
            .iter()
            .map(|row| row.count_ones() as usize)
            .sum()
    }

    fn boundary_load_factor(time: f64, theta: f64) -> f64 {
        let clipped = time.clamp(0.0, HORIZON as f64);
        let left = 1.0 - (-clipped / theta).exp();
        let right = 1.0 - (-((HORIZON as f64) - clipped) / theta).exp();
        (left * right).max(0.0)
    }

    fn local_bid_at(&mut self, time: f64, theta: f64, offered_area: f64) -> f64 {
        let local_offer = offered_area * Self::boundary_load_factor(time, theta);
        if local_offer <= self.effective_capacity {
            return 0.0;
        }
        let fraction = self.effective_capacity / local_offer;
        self.q_threshold_for_fraction(fraction) * self.compactness_bar
    }

    fn base_dynamic_threshold(&mut self, S: usize, duration: usize, P: usize, theta: f64) -> f64 {
        const ARRIVAL_RATE: f64 = 1_000.0 / (HORIZON as f64);
        const X0: f64 = 0.112_701_665_379_258_3;
        const X1: f64 = 0.5;
        const X2: f64 = 0.887_298_334_620_741_7;
        const W0: f64 = 5.0 / 18.0;
        const W1: f64 = 8.0 / 18.0;
        const W2: f64 = 5.0 / 18.0;

        let offered_area = ARRIVAL_RATE * self.expected_p * theta;
        let D = duration as f64;
        let average_bid = W0 * self.local_bid_at((S as f64) + X0 * D, theta, offered_area)
            + W1 * self.local_bid_at((S as f64) + X1 * D, theta, offered_area)
            + W2 * self.local_bid_at((S as f64) + X2 * D, theta, offered_area);
        let threshold = average_bid * (D / theta).powf(0.1);

        let current_offer = offered_area * Self::boundary_load_factor(S as f64, theta);
        let target_occupancy = self.effective_capacity.min(current_offer);
        let error = ((self.occupied_count() as f64) + 0.5 * (P as f64) - target_occupancy)
            / self.effective_capacity.max(1.0);
        let multiplier = (0.70 * error).exp().clamp(0.82, 1.80);
        threshold * multiplier
    }

    fn component_threshold_factor(&self, component_size: usize) -> f64 {
        if component_size >= MAX_P {
            1.0
        } else {
            0.75 + 0.25 * self.fit_probability(component_size)
        }
    }

    fn clear_group_from_board(&mut self, group_id: usize) {
        let cells = self.groups[group_id].cells.clone();
        for cell in cells {
            let r = cell / self.N;
            let c = cell % self.N;
            self.occupied_rows[r] &= !Self::bit_at(c);
            if self.owner_cell[cell] == group_id as isize {
                self.owner_cell[cell] = -1;
            }
        }
    }

    fn place_group_on_board(&mut self, group_id: usize, cells: &[usize]) {
        for &cell in cells {
            let r = cell / self.N;
            let c = cell % self.N;
            self.occupied_rows[r] |= Self::bit_at(c);
            self.owner_cell[cell] = group_id as isize;
        }
    }

    fn remove_expired(&mut self, current_S: usize) {
        while let Some(&Reverse((T, group_id))) = self.departures.peek() {
            if T >= current_S {
                break;
            }
            self.departures.pop();
            if !self.groups[group_id].active {
                continue;
            }
            self.clear_group_from_board(group_id);
            self.groups[group_id].active = false;
            local! {
                self.trace.count("expired");
            }
        }
    }

    /// 将来到着列を 1 本サンプルする。到着レートは残りグループ数/残り時間、
    /// 滞在は θ の指数分布、P は生成分布、q = 2^gauss(0,0.8)。
    fn make_future_arrivals(
        &self,
        incoming_id: usize,
        now: usize,
        theta: f64,
        seed: u64,
    ) -> Vec<FutureArrival> {
        let mut rng = XorShift64::new(seed);
        let remaining = (self.M - 1 - incoming_id) as f64;
        let remaining_time = (HORIZON - now) as f64;
        let mut arrivals = Vec::with_capacity(ROLLOUT_ARRIVALS);
        if remaining < 1.0 || remaining_time < 1.0 {
            return arrivals;
        }
        let mean_gap = remaining_time / remaining;
        let mut t = now as f64;
        for _ in 0..ROLLOUT_ARRIVALS {
            t += (-rng.next_f64().ln()) * mean_gap;
            if t >= HORIZON as f64 {
                break;
            }
            // P: p_cdf の逆関数サンプル。
            let u = rng.next_f64();
            let mut lo = 4;
            let mut hi = MAX_P;
            while lo < hi {
                let mid = (lo + hi) / 2;
                if self.p_cdf[mid] >= u {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            let P = lo;
            let dur =
                ((-rng.next_f64().ln() * theta).round() as usize + 1).min(HORIZON - (t as usize));
            let q = (2.0_f64).powf(0.8 * rng.gauss());
            arrivals.push(FutureArrival {
                at: t as usize,
                dur,
                P,
                q,
                value: q * (P as f64) * (dur as f64).powf(0.9),
            });
        }
        arrivals
    }

    /// ロールアウト用の簡易配置: L_min と L_min+2 の先頭数形状を bitboard で走査し、
    /// 最初に入る位置に置く。growth はしない。戻り値は (セル列, C)。
    fn quick_place(&self, occ: &Rows, P: usize) -> Option<(Vec<usize>, f64)> {
        let runs = self.build_run_table(occ);
        let min_L = minimum_perimeter(P);
        for perimeter in [min_L, min_L + 2] {
            let mut used = 0;
            for shape in &self.shapes_by_p[P] {
                if shape.perimeter != perimeter {
                    continue;
                }
                used += 1;
                if used > 3 {
                    break;
                }
                let y_count = self.N - shape.w + 1;
                let valid_y = (1_u64 << y_count) - 1;
                for x in 0..=(self.N - shape.h) {
                    let mut ys = valid_y;
                    for rr in 0..shape.h {
                        if ys == 0 {
                            break;
                        }
                        ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                    }
                    if ys != 0 {
                        let y = ys.trailing_zeros() as usize;
                        let mut cells = Vec::with_capacity(P);
                        for rr in 0..shape.h {
                            let r = x + rr;
                            let begin = y + shape.left[rr];
                            for c in begin..(begin + shape.len[rr]) {
                                cells.push(r * self.N + c);
                            }
                        }
                        return Some((cells, compactness(P, perimeter)));
                    }
                }
            }
        }
        None
    }

    /// 候補盤面 1 つを到着列 1 本でロールアウトし、将来受け入れ価値の合計を返す。
    /// dep_base は現 active グループの (T, gid) を T 昇順に並べたもの。
    fn rollout_one(
        &self,
        cand: &RolloutCandidate,
        dep_base: &[(usize, usize)],
        arrivals: &[FutureArrival],
        threshold: f64,
    ) -> f64 {
        let mut occ = cand.board;
        // 退去イベント: (T, 既存グループ index) と、ロールアウト内で受け入れた将来分。
        let mut base_i = 0;
        let mut future_deps: Vec<(usize, Vec<usize>)> = Vec::with_capacity(ROLLOUT_ARRIVALS + 1);
        if let Some((t, cells)) = &cand.incoming_dep {
            future_deps.push((*t, cells.clone()));
        }
        let mut score = 0.0;
        for arrival in arrivals {
            // 到着時刻までの退去処理 (既存グループと将来グループの両方)。
            loop {
                let next_base = dep_base.get(base_i).map(|&(t, _)| t);
                let next_future = future_deps
                    .iter()
                    .enumerate()
                    .filter(|(_, (t, _))| *t < arrival.at)
                    .min_by_key(|(_, (t, _))| *t)
                    .map(|(i, (t, _))| (*t, i));
                match (next_base, next_future) {
                    (Some(tb), Some((tf, fi))) if tb < arrival.at || tf < arrival.at => {
                        if tb < arrival.at && (tf >= arrival.at || tb <= tf) {
                            let gid = dep_base[base_i].1;
                            base_i += 1;
                            self.release_cells_for(&mut occ, gid);
                        } else {
                            let (_, cells) = future_deps.swap_remove(fi);
                            for cell in cells {
                                occ[cell / self.N] &= !Self::bit_at(cell % self.N);
                            }
                        }
                    }
                    (Some(tb), None) if tb < arrival.at => {
                        let gid = dep_base[base_i].1;
                        base_i += 1;
                        self.release_cells_for(&mut occ, gid);
                    }
                    (None, Some((_, fi))) => {
                        let (_, cells) = future_deps.swap_remove(fi);
                        for cell in cells {
                            occ[cell / self.N] &= !Self::bit_at(cell % self.N);
                        }
                    }
                    _ => break,
                }
            }
            // 簡易 admission: 今の閾値で選別 (ロールアウト内では固定)。
            if threshold > 0.0 && arrival.q * self.c_max_table[arrival.P] < threshold {
                continue;
            }
            if let Some((cells, C)) = self.quick_place(&occ, arrival.P) {
                score += arrival.value * C;
                for &cell in &cells {
                    occ[cell / self.N] |= Self::bit_at(cell % self.N);
                }
                future_deps.push((arrival.at + arrival.dur, cells));
            }
        }
        score
    }

    /// gid の占有セルを盤面から解放する。
    fn release_cells_for(&self, occ: &mut Rows, gid: usize) {
        for &cell in &self.groups[gid].cells {
            occ[cell / self.N] &= !Self::bit_at(cell % self.N);
        }
    }

    /// 候補群を「即時実額 + 共通乱数ロールアウト平均」で比較し、勝者 index を返す。
    fn evaluate_candidates_rollout(
        &mut self,
        cands: &[RolloutCandidate],
        incoming_id: usize,
        now: usize,
        theta: f64,
        threshold: f64,
    ) -> usize {
        // 現 active の退去表 (T 昇順)。ヒープは順序不定なので Vec 化してソートする。
        let mut dep_base: Vec<(usize, usize)> = self
            .departures
            .iter()
            .map(|&Reverse((t, gid))| (t, gid))
            .filter(|&(_, gid)| self.groups[gid].active)
            .collect();
        dep_base.sort_unstable();

        let mut totals = vec![0.0_f64; cands.len()];
        for (i, cand) in cands.iter().enumerate() {
            totals[i] = cand.immediate * (ROLLOUT_SAMPLES as f64);
        }
        for k in 0..ROLLOUT_SAMPLES {
            let seed = (incoming_id as u64) * 1_000_003 + (k as u64) * 7_919 + 1;
            let arrivals = self.make_future_arrivals(incoming_id, now, theta, seed);
            for (i, cand) in cands.iter().enumerate() {
                totals[i] += self.rollout_one(cand, &dep_base, &arrivals, threshold);
            }
        }
        let mut best = 0;
        for i in 1..totals.len() {
            if totals[i] > totals[best] {
                best = i;
            }
        }
        best
    }

    /// 同一周長・同一 component_size の通常配置候補を、既存の短期ロールアウトで比較する。
    /// 実際の利用料は等しいが、slot calendar有効時だけ期待損失を即時額から差し引く。
    fn select_normal_by_rollout(
        &mut self,
        choices: &[Placement],
        incoming_id: usize,
        theta: f64,
        base_threshold: f64,
    ) -> usize {
        debug_assert!(choices.len() >= 2);
        let incoming_S = self.groups[incoming_id].S;
        let incoming_T = self.groups[incoming_id].T;
        let incoming_P = self.groups[incoming_id].P;
        let incoming_V = self.groups[incoming_id].V;
        let mut cands = Vec::with_capacity(choices.len());
        for placement in choices {
            let cells = self.materialize(placement, incoming_P);
            let mut board = self.occupied_rows;
            for &cell in &cells {
                board[cell / self.N] |= Self::bit_at(cell % self.N);
            }
            cands.push(RolloutCandidate {
                board,
                immediate: (incoming_V as f64) * compactness(incoming_P, placement.perimeter)
                    - placement.slot_penalty,
                incoming_dep: Some((incoming_T, cells)),
            });
        }
        let winner = local_time!(self.trace, "normal_rollout", {
            self.evaluate_candidates_rollout(&cands, incoming_id, incoming_S, theta, base_threshold)
        });
        local! {
            self.trace.count("normal_rollout_session");
            self.trace
                .count_by("normal_rollout_candidate_sum", choices.len() as i64);
            if winner != 0 {
                self.trace.count("normal_rollout_flip");
            }
        }
        winner
    }

    fn commit_normal_placement(&mut self, group_id: usize, placement: &Placement) {
        let P = self.groups[group_id].P;
        local! {
            if placement.shape_index != usize::MAX
                && !self.shapes_by_p[P][placement.shape_index].baseline_kept
            {
                self.trace.count("extra_shape_placed");
            }
        }
        let cells = self.materialize(placement, P);
        {
            let group = &mut self.groups[group_id];
            group.cells = cells.clone();
            group.active = true;
        }
        self.place_group_on_board(group_id, &cells);
        self.departures
            .push(Reverse((self.groups[group_id].T, group_id)));
    }

    fn print_zero_moves<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "0")?;
        writer.flush()?;
        local! {
            self.trace.count("move_zero_output");
        }
        Ok(())
    }

    fn print_acceptance<W: Write>(
        &self,
        writer: &mut W,
        accept: bool,
        cells: &[usize],
    ) -> io::Result<()> {
        if !accept {
            writeln!(writer, "No")?;
            writer.flush()?;
            return Ok(());
        }
        writeln!(writer, "Yes")?;
        for &cell in cells {
            writeln!(writer, "{} {}", cell / self.N, cell % self.N)?;
        }
        writer.flush()
    }

    fn initialize_static_capacity(&mut self) {
        let empty_occ = [0_u64; MAX_N];
        let info = self.compute_free_info(&empty_occ, true);
        let mut usable = 0;
        let mut component_count = 0;
        for &size in &info.sizes {
            if size >= 4 {
                usable += size;
                component_count += 1;
            }
        }
        let grass_count: usize = self.grass_rows[..self.N]
            .iter()
            .map(|row| row.count_ones() as usize)
            .sum();
        let pond_count = self.N * self.N - grass_count;
        let pond_factor = ((pond_count as f64) / 900.0).clamp(0.0, 1.0);
        let split_factor = (((component_count as f64) - 1.0) / 8.0).clamp(0.0, 1.0);
        let packing_efficiency =
            (0.89 - 0.055 * pond_factor - 0.015 * split_factor).clamp(0.80, 0.89);
        self.effective_capacity =
            (packing_efficiency * (usable as f64) * NO_MOVE_CAPACITY_RATIO).max(1.0);
        local! {
            self.trace.count_by("capacity_reserve_milli", 975);
        }
    }

    fn run<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        writer: &mut W,
    ) -> io::Result<()> {
        local! {
            // このversionは元rejectをacceptへ変える経路を持たない。
            self.trace.count_by("causal_veto_reject_to_accept", 0);
        }
        for turn in 0..self.M {
            let id: usize = scanner.next();
            let S: usize = scanner.next();
            let T: usize = scanner.next();
            let P: usize = scanner.next();
            let V: i64 = scanner.next();
            debug_assert_eq!(id, turn);
            self.remove_expired(S);

            self.groups[id] = Group {
                id,
                S,
                T,
                P,
                V,
                ..Group::default()
            };
            let duration = T - S;
            self.duration_sum += duration as f64;
            self.duration_count += 1;
            let theta = self.posterior_theta();
            let q_value = (V as f64) / ((P as f64) * (duration as f64).powf(0.9));
            let is_large_target = P >= 96 && duration >= 6_000 && q_value >= 1.0;
            let base_threshold = self.base_dynamic_threshold(S, duration, P, theta);
            let optimistic_C = compactness(P, minimum_perimeter(P));
            let fast_mode = self.timer.reached(FAST_MODE_RATIO);
            if fast_mode {
                local! {
                    self.trace.count("fast_mode_turn");
                }
            }

            let mut accepted = false;
            let mut normal = None;
            let passed_price_prefilter =
                base_threshold == 0.0 || q_value * optimistic_C >= 0.74 * base_threshold;
            if passed_price_prefilter {
                local! {
                    self.trace.count("normal_search");
                }
                let normal_choices = local_time!(self.trace, "normal_search", {
                    self.find_normal_placements(id, P, V, S, T, theta, is_large_target, fast_mode)
                });
                if let Some(mut choices) = normal_choices {
                    let current_perimeter = choices[0].perimeter;
                    let current_component_size = choices[0].component_size;
                    let _slot_active = choices[0].slot_count > 0;
                    let _release_active = choices[0].release_snapshot_count > 0;
                    debug_assert!(
                        choices
                            .iter()
                            .all(|placement| placement.slot_count == choices[0].slot_count)
                    );
                    debug_assert!(choices.iter().all(|placement| {
                        placement.release_snapshot_count == choices[0].release_snapshot_count
                    }));
                    let actual_threshold =
                        base_threshold * self.component_threshold_factor(current_component_size);
                    let quality = q_value * compactness(P, current_perimeter);
                    accepted = base_threshold == 0.0 || quality >= actual_threshold;

                    let rollout_choice_count = choices
                        .iter()
                        .position(|placement| placement.release_added_candidate)
                        .unwrap_or(choices.len());
                    debug_assert!(
                        choices[..rollout_choice_count]
                            .iter()
                            .all(|placement| !placement.release_added_candidate)
                    );
                    debug_assert!(
                        choices[rollout_choice_count..]
                            .iter()
                            .all(|placement| placement.release_added_candidate)
                    );
                    let rollout_winner = if accepted && rollout_choice_count >= 2 {
                        self.select_normal_by_rollout(
                            &choices[..rollout_choice_count],
                            id,
                            theta,
                            base_threshold,
                        )
                    } else {
                        0
                    };
                    let mut winner = rollout_winner;
                    if accepted && _release_active {
                        let mut room_winner = rollout_winner;
                        for index in 0..choices.len() {
                            if choices[index].release_score > choices[room_winner].release_score {
                                room_winner = index;
                            }
                        }
                        let future_gain = choices[room_winner]
                            .release_score
                            .saturating_sub(choices[rollout_winner].release_score);
                        let current_gain = choices[room_winner].release_current_largest as i64
                            - choices[rollout_winner].release_current_largest as i64;
                        let gain_pass = future_gain >= self.expected_p.round() as usize;
                        let current_pass = current_gain >= 0;
                        local! {
                            self.trace.count("room_pareto_compare");
                            self.trace
                                .count_by("room_pareto_future_gain", future_gain as i64);
                            self.trace
                                .count_by("room_pareto_current_gain", current_gain);
                            if gain_pass {
                                self.trace.count("room_pareto_gain_pass");
                            }
                            if current_pass {
                                self.trace.count("room_pareto_current_pass");
                            }
                            if rollout_choice_count < choices.len() {
                                self.trace.count("room_pareto_added_candidate_available");
                            }
                        }
                        if room_winner != rollout_winner && gain_pass && current_pass {
                            winner = room_winner;
                            local! {
                                self.trace.count("room_pareto_flip");
                                if choices[room_winner].release_added_candidate {
                                    self.trace.count("room_pareto_flip_added_candidate");
                                }
                            }
                        }
                    }
                    local! {
                        if _slot_active && accepted && choices.len() >= 2 {
                            self.trace.count("slot_comparison");
                            self.trace
                                .count_by("slot_delay_before", choices[0].slot_delay as i64);
                            self.trace.count_by(
                                "slot_delay_after",
                                choices[winner].slot_delay as i64,
                            );
                            if choices[winner].slot_delay < choices[0].slot_delay {
                                self.trace.count("slot_preservation_flip");
                            }
                        }
                        if _release_active && accepted {
                            self.trace.count("release_atlas_comparison");
                            self.trace.count_by(
                                "release_atlas_score_before",
                                choices[0].release_score as i64,
                            );
                            self.trace.count_by(
                                "release_atlas_score_after",
                                choices[winner].release_score as i64,
                            );
                            self.trace.count_by(
                                "release_atlas_score_gain",
                                choices[winner].release_score as i64
                                    - choices[0].release_score as i64,
                            );
                            if choices[winner].release_score > choices[0].release_score {
                                self.trace.count("release_atlas_flip");
                            }
                            if choices[winner].release_candidate {
                                self.trace.count("release_atlas_candidate_selected");
                            }
                        }
                    }
                    let mut placement = choices.swap_remove(winner);
                    debug_assert_eq!(placement.perimeter, current_perimeter);
                    debug_assert_eq!(placement.component_size, current_component_size);
                    // v035 と同じ admission と最終 winner を確定してから、受理済みの
                    // growth/box 由来候補だけを後処理する。
                    if accepted && placement.shape_index == usize::MAX {
                        placement = local_time!(self.trace, "biased_swap", {
                            self.improve_final_growth_by_biased_swap(placement, V)
                        });
                    }
                    normal = Some(placement);

                    local! {
                        let placement = normal.as_ref().expect("normal placement selected");
                        if placement.shape_index != usize::MAX {
                            let shape = &self.shapes_by_p[P][placement.shape_index];
                            if !shape.baseline_kept {
                                self.trace.count("extra_shape_chosen");
                            }
                        }
                    }
                    if !accepted {
                        local! {
                            self.trace.count("post_placement_price_reject");
                        }
                    }
                    // 元のadmissionとbiased後のplacementを確定した後、actual-futureで
                    // 事前固定した因果帯だけを単方向にvetoする。
                    if accepted && base_threshold > 0.0 {
                        let placement = normal.as_ref().expect("normal placement selected");
                        let C = compactness(P, placement.perimeter);
                        let G = (V as f64) * C;
                        let lambda = (P as f64) * (duration as f64).powf(0.9) * actual_threshold;
                        assert!(G.is_finite() && lambda.is_finite() && lambda > 0.0);
                        let margin = G / lambda;
                        if (1.0..=CAUSAL_VETO_MARGIN_MAX).contains(&margin) {
                            local! {
                                self.trace.count("causal_veto_near_threshold");
                            }
                            let duration_ratio = (duration as f64) / theta;
                            assert!(duration_ratio.is_finite());
                            if duration_ratio < CAUSAL_VETO_DURATION_RATIO_MAX {
                                local! {
                                    self.trace.count("causal_veto_duration_pass");
                                }
                                let min_perimeter = minimum_perimeter(P);
                                assert!(placement.perimeter >= min_perimeter);
                                let slack = placement.perimeter - min_perimeter;
                                if slack <= CAUSAL_VETO_SLACK_MAX {
                                    local! {
                                        self.trace.count("causal_veto_slack_pass");
                                    }
                                    assert!(accepted);
                                    assert!(base_threshold > 0.0);
                                    assert!((1.0..=CAUSAL_VETO_MARGIN_MAX).contains(&margin));
                                    assert!(duration_ratio < CAUSAL_VETO_DURATION_RATIO_MAX);
                                    assert!(slack <= CAUSAL_VETO_SLACK_MAX);
                                    accepted = false;
                                    local! {
                                        self.trace.count("causal_veto_executed");
                                        self.trace
                                            .count_by("causal_veto_G", G.round() as i64);
                                        self.trace.count_by("causal_veto_P", P as i64);
                                        self.trace.count_by(
                                            "causal_veto_D_theta_milli",
                                            (duration_ratio * 1_000.0).round() as i64,
                                        );
                                        self.trace
                                            .count_by("causal_veto_slack", slack as i64);
                                        self.trace.count_by(
                                            "causal_veto_margin_milli",
                                            (margin * 1_000.0).round() as i64,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            local! {
                if !passed_price_prefilter {
                    self.trace.count("price_prefilter_reject");
                }
            }

            self.print_zero_moves(writer)?;
            if accepted {
                let placement = normal.as_ref().expect("accepted normal placement");
                self.commit_normal_placement(id, placement);
                self.print_acceptance(writer, true, &self.groups[id].cells)?;
                local! {
                    self.trace.count("accepted");
                    self.trace.count("normal_placed");
                }
            } else {
                self.groups[id].active = false;
                self.print_acceptance(writer, false, &[])?;
                local! {
                    self.trace.count("rejected");
                    if normal.is_none() {
                        self.trace.count("geometry_reject");
                    }
                }
            }
        }
        writer.flush()?;
        local! {
            self.trace
                .add_time_ms("program_elapsed", self.timer.elapsed_ms());
            self.trace.summary();
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    // timer は入力や前計算も含めるため、main の開始直後に作る。
    let timer = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    local! {
        verify_free_cut_loss();
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    let N: usize = scanner.next();
    let M: usize = scanner.next();
    let _R: String = scanner.next(); // 入力同期のため読むが、再移動しないので意思決定には使わない。
    assert!(N <= MAX_N);
    let mut grass_rows = [0_u64; MAX_N];
    for row_mask in grass_rows.iter_mut().take(N) {
        let row: String = scanner.next();
        let mut mask = 0_u64;
        for (c, byte) in row.bytes().enumerate() {
            if byte == b'.' {
                mask |= Solver::bit_at(c);
            }
        }
        *row_mask = mask;
    }

    let mut solver = Solver::new(N, M, grass_rows, timer);
    solver.run(&mut scanner, &mut writer)
}
