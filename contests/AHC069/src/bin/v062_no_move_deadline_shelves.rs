// v062_no_move_deadline_shelves.rs
#![allow(non_snake_case)] // 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。

// 中心アイデア: 池地形に合わせて選んだ一方向へ各棚をprefix状に詰め、
// 棚の奥から手前へ退去時刻を単調減少させる。再移動は一切行わない。

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
/// admission と winner 選択を終えた growth 候補だけに使う後処理予算。
const BIASED_SWAP_START_RATIO: f64 = 0.86;
const BIASED_SWAP_LIMIT_RATIO: f64 = 0.90;
const BIASED_SWAP_ITERATIONS: usize = 512;
const BIASED_SWAP_MIN_RECOVERABLE_FEE: f64 = 300_000.0;
const NO_MOVE_CAPACITY_RATIO: f64 = 0.975;
const CAUSAL_VETO_MARGIN_MAX: f64 = 1.13;
const CAUSAL_VETO_DURATION_RATIO_MAX: f64 = 2.0;
const CAUSAL_VETO_SLACK_MAX: usize = 14;

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
    explicit_cells: Vec<usize>,
    shelf_key: ShelfKey,
    /// 1: regular、2: frontier growth。0 は旧コード内の一時候補だけに使う。
    shelf_kind: u8,
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
            explicit_cells: Vec::new(),
            shelf_key: ShelfKey {
                new_runs: usize::MAX,
                deadline_slack: usize::MAX,
                transverse_span: usize::MAX,
                transverse_start: usize::MAX,
            },
            shelf_kind: 0,
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShelfDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl ShelfDirection {
    const ALL: [Self; 4] = [
        Self::LeftToRight,
        Self::RightToLeft,
        Self::TopToBottom,
        Self::BottomToTop,
    ];

    #[inline]
    fn physical_cell(self, N: usize, transverse: usize, depth: usize) -> usize {
        let (r, c) = match self {
            Self::LeftToRight => (transverse, depth),
            Self::RightToLeft => (transverse, N - 1 - depth),
            Self::TopToBottom => (depth, transverse),
            Self::BottomToTop => (N - 1 - depth, transverse),
        };
        r * N + c
    }

    #[cfg(feature = "local")]
    #[inline]
    fn trace_key(self) -> &'static str {
        match self {
            Self::LeftToRight => "shelf_direction_left_to_right",
            Self::RightToLeft => "shelf_direction_right_to_left",
            Self::TopToBottom => "shelf_direction_top_to_bottom",
            Self::BottomToTop => "shelf_direction_bottom_to_top",
        }
    }
}

#[derive(Clone, Debug)]
struct ShelfRun {
    transverse: usize,
    start_depth: usize,
    cells: Vec<usize>,
}

#[derive(Clone, Debug)]
struct ShelfLayout {
    direction: ShelfDirection,
    runs: Vec<ShelfRun>,
    runs_by_transverse: Vec<Vec<usize>>,
    run_of_cell: Vec<usize>,
    depth_in_run: Vec<usize>,
}

impl ShelfLayout {
    fn build(N: usize, grass_rows: &Rows, direction: ShelfDirection) -> Self {
        let mut runs = Vec::new();
        let mut runs_by_transverse = vec![Vec::new(); N];
        let mut run_of_cell = vec![usize::MAX; N * N];
        let mut depth_in_run = vec![usize::MAX; N * N];
        for transverse in 0..N {
            let mut depth = 0;
            while depth < N {
                let cell = direction.physical_cell(N, transverse, depth);
                let r = cell / N;
                let c = cell % N;
                if ((grass_rows[r] >> c) & 1) == 0 {
                    depth += 1;
                    continue;
                }
                let start_depth = depth;
                let mut cells = Vec::new();
                while depth < N {
                    let cell = direction.physical_cell(N, transverse, depth);
                    let r = cell / N;
                    let c = cell % N;
                    if ((grass_rows[r] >> c) & 1) == 0 {
                        break;
                    }
                    cells.push(cell);
                    depth += 1;
                }
                let run_id = runs.len();
                for (run_depth, &cell) in cells.iter().enumerate() {
                    run_of_cell[cell] = run_id;
                    depth_in_run[cell] = run_depth;
                }
                runs_by_transverse[transverse].push(run_id);
                runs.push(ShelfRun {
                    transverse,
                    start_depth,
                    cells,
                });
            }
        }
        Self {
            direction,
            runs,
            runs_by_transverse,
            run_of_cell,
            depth_in_run,
        }
    }

    fn largest_remaining_component(&self, N: usize, grass_rows: &Rows, numerator: usize) -> usize {
        let mut blocked = vec![false; N * N];
        for run in &self.runs {
            let prefix = run.cells.len() * numerator / 4;
            for &cell in run.cells.iter().take(prefix) {
                blocked[cell] = true;
            }
        }
        let mut seen = vec![false; N * N];
        let mut largest = 0;
        for start in 0..N * N {
            let r = start / N;
            let c = start % N;
            if blocked[start] || seen[start] || ((grass_rows[r] >> c) & 1) == 0 {
                continue;
            }
            let mut size = 0;
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(cell) = stack.pop() {
                size += 1;
                let r = cell / N;
                let c = cell % N;
                for next in [
                    (r > 0).then_some(cell - N),
                    (r + 1 < N).then_some(cell + N),
                    (c > 0).then_some(cell - 1),
                    (c + 1 < N).then_some(cell + 1),
                ]
                .into_iter()
                .flatten()
                {
                    let nr = next / N;
                    let nc = next % N;
                    if !blocked[next] && !seen[next] && ((grass_rows[nr] >> nc) & 1) != 0 {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            largest = largest.max(size);
        }
        largest
    }

    fn choose(N: usize, grass_rows: &Rows) -> Self {
        let mut best: Option<([usize; 3], usize, Self)> = None;
        for direction in ShelfDirection::ALL {
            let layout = Self::build(N, grass_rows, direction);
            let score = [
                layout.largest_remaining_component(N, grass_rows, 3),
                layout.largest_remaining_component(N, grass_rows, 2),
                layout.largest_remaining_component(N, grass_rows, 1),
            ];
            let replace = best.as_ref().is_none_or(|(best_score, best_runs, _)| {
                score > *best_score || (score == *best_score && layout.runs.len() < *best_runs)
            });
            if replace {
                best = Some((score, layout.runs.len(), layout));
            }
        }
        best.expect("at least one shelf direction").2
    }

    #[inline]
    fn canonical_cell(&self, N: usize, transverse: usize, depth: usize) -> usize {
        self.direction.physical_cell(N, transverse, depth)
    }
}

#[derive(Clone, Debug)]
struct ShelfState {
    frontier: Vec<usize>,
    gap_violations: usize,
    deadline_violations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ShelfKey {
    new_runs: usize,
    deadline_slack: usize,
    transverse_span: usize,
    transverse_start: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShelfCandidateError {
    Geometry,
    Gap,
    Deadline,
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
    shelf_layout: ShelfLayout,
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
        let shelf_layout = ShelfLayout::choose(N, &grass_rows);
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
            shelf_layout,
            timer,
            #[cfg(feature = "local")]
            trace: TraceStats::default(),
        };
        solver.initialize_p_distribution();
        solver.initialize_static_capacity();
        local! {
            for direction in ShelfDirection::ALL {
                solver.trace.count_by(direction.trace_key(), 0);
            }
            solver.trace.count(solver.shelf_layout.direction.trace_key());
            solver
                .trace
                .count_by("shelf_run_count", solver.shelf_layout.runs.len() as i64);
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
    fn slot_penalty(calendar: &SlotCalendar, slot_delay: usize) -> f64 {
        // 414kは大型targetを悪形に追いやった1件あたりの観測平均損失である。
        (slot_delay as f64) * calendar.target_arrival_rate * 414_000.0 / (calendar.K.max(1) as f64)
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

    fn analyze_shelf_state(&self) -> ShelfState {
        let mut frontier = vec![0; self.shelf_layout.runs.len()];
        let mut gap_violations = 0;
        let mut deadline_violations = 0;
        for (run_id, run) in self.shelf_layout.runs.iter().enumerate() {
            let mut seen_free = false;
            let mut previous_owner = usize::MAX;
            let mut previous_T = usize::MAX;
            for (depth, &cell) in run.cells.iter().enumerate() {
                let owner = self.owner_cell[cell];
                if owner < 0 {
                    seen_free = true;
                    continue;
                }
                if seen_free {
                    gap_violations += 1;
                    continue;
                }
                frontier[run_id] = depth + 1;
                let owner = owner as usize;
                if owner >= self.M || !self.groups[owner].active {
                    gap_violations += 1;
                    continue;
                }
                let T = self.groups[owner].T;
                if previous_owner != usize::MAX && owner != previous_owner && T >= previous_T {
                    deadline_violations += 1;
                }
                previous_owner = owner;
                previous_T = T;
            }
        }
        ShelfState {
            frontier,
            gap_violations,
            deadline_violations,
        }
    }

    #[cfg(feature = "local")]
    fn verify_shelf_invariant(&mut self) {
        let state = self.analyze_shelf_state();
        local! {
            self.trace.count("shelf_invariant_check");
            self.trace.count_by(
                "shelf_invariant_gap_violation",
                state.gap_violations as i64,
            );
            self.trace.count_by(
                "shelf_invariant_deadline_violation",
                state.deadline_violations as i64,
            );
            assert_eq!(state.gap_violations, 0, "shelf prefix invariant violated");
            assert_eq!(
                state.deadline_violations, 0,
                "shelf deadline order invariant violated"
            );
        }
    }

    fn shelf_candidate_key(
        &self,
        cells: &[usize],
        P: usize,
        incoming_T: usize,
        occ: &Rows,
        state: &ShelfState,
    ) -> Result<ShelfKey, ShelfCandidateError> {
        if cells.len() != P {
            return Err(ShelfCandidateError::Geometry);
        }
        // 候補の大半はfrontier不一致で落ちる。2500マスの連結検査を行う前に、
        // P<=150の候補セルだけで棚ごとの連続区間を照合する。
        let mut touched: Vec<(usize, usize, usize, usize)> = Vec::new();
        for &cell in cells {
            if cell >= self.N * self.N {
                return Err(ShelfCandidateError::Geometry);
            }
            let r = cell / self.N;
            let c = cell % self.N;
            if !self.is_free(occ, r, c) {
                return Err(ShelfCandidateError::Geometry);
            }
            let run_id = self.shelf_layout.run_of_cell[cell];
            if run_id == usize::MAX {
                return Err(ShelfCandidateError::Geometry);
            }
            let depth = self.shelf_layout.depth_in_run[cell];
            if let Some((_, count, min_depth, max_depth)) =
                touched.iter_mut().find(|entry| entry.0 == run_id)
            {
                *count += 1;
                *min_depth = (*min_depth).min(depth);
                *max_depth = (*max_depth).max(depth);
            } else {
                touched.push((run_id, 1, depth, depth));
            }
        }

        let mut new_runs = 0;
        let mut support_T = HORIZON + 1;
        let mut has_existing_support = false;
        let mut transverse_start = usize::MAX;
        let mut transverse_end = 0;
        for &(run_id, count, min_depth, max_depth) in &touched {
            let run = &self.shelf_layout.runs[run_id];
            let frontier = state.frontier[run_id];
            if min_depth != frontier
                || max_depth + 1 - min_depth != count
                || max_depth >= run.cells.len()
            {
                return Err(ShelfCandidateError::Gap);
            }
            transverse_start = transverse_start.min(run.transverse);
            transverse_end = transverse_end.max(run.transverse);
            if frontier == 0 {
                new_runs += 1;
                continue;
            }
            let owner = self.owner_cell[run.cells[frontier - 1]];
            if owner < 0 {
                return Err(ShelfCandidateError::Gap);
            }
            let back_T = self.groups[owner as usize].T;
            if incoming_T >= back_T {
                return Err(ShelfCandidateError::Deadline);
            }
            support_T = support_T.min(back_T);
            has_existing_support = true;
        }
        if !has_existing_support {
            support_T = HORIZON + 1;
        }
        if !self.explicit_candidate_is_valid(cells, P, occ) {
            return Err(ShelfCandidateError::Geometry);
        }
        Ok(ShelfKey {
            new_runs,
            deadline_slack: support_T - incoming_T,
            transverse_span: transverse_end + 1 - transverse_start,
            transverse_start,
        })
    }

    fn trace_shelf_candidate(&mut self, _result: &Result<ShelfKey, ShelfCandidateError>) {
        local! {
            self.trace.count("shelf_candidate_checked");
            match _result {
                Ok(_) => self.trace.count("shelf_candidate_valid"),
                Err(ShelfCandidateError::Geometry) => {
                    self.trace.count("shelf_candidate_invalid_geometry")
                }
                Err(ShelfCandidateError::Gap) => {
                    self.trace.count("shelf_candidate_invalid_gap")
                }
                Err(ShelfCandidateError::Deadline) => {
                    self.trace.count("shelf_candidate_invalid_deadline")
                }
            }
        }
    }

    fn explicit_slot_delay(
        &self,
        cells: &[usize],
        incoming_T: usize,
        calendar: &SlotCalendar,
    ) -> usize {
        calendar
            .slots
            .iter()
            .filter(|slot| {
                cells.iter().any(|&cell| {
                    let r = cell / self.N;
                    let c = cell % self.N;
                    slot.x <= r && r < slot.x + slot.h && slot.y <= c && c < slot.y + slot.w
                })
            })
            .map(|slot| incoming_T.saturating_sub(slot.ready))
            .sum()
    }

    #[allow(clippy::too_many_arguments)]
    fn shelf_regular_level(
        &mut self,
        P: usize,
        incoming_T: usize,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
        state: &ShelfState,
        perimeter: usize,
        shape_limit: usize,
        fast_mode: bool,
    ) -> Vec<Placement> {
        let mut candidates = Vec::new();
        let mut used_shapes = 0;
        for shape_index in 0..self.shapes_by_p[P].len() {
            let shape = self.shapes_by_p[P][shape_index].clone();
            if shape.perimeter != perimeter {
                continue;
            }
            if used_shapes >= shape_limit {
                break;
            }
            used_shapes += 1;
            for transverse_start in 0..=self.N - shape.h {
                let first_runs = self.shelf_layout.runs_by_transverse[transverse_start].clone();
                for first_run_id in first_runs {
                    let first_run = &self.shelf_layout.runs[first_run_id];
                    let first_frontier = state.frontier[first_run_id];
                    if first_frontier >= first_run.cells.len() {
                        continue;
                    }
                    let aligned_depth = first_run.start_depth + first_frontier;
                    if aligned_depth < shape.left[0] {
                        continue;
                    }
                    let base_depth = aligned_depth - shape.left[0];
                    if base_depth + shape.w > self.N {
                        continue;
                    }
                    let mut cells = Vec::with_capacity(P);
                    for rr in 0..shape.h {
                        for offset in shape.left[rr]..shape.left[rr] + shape.len[rr] {
                            cells.push(self.shelf_layout.canonical_cell(
                                self.N,
                                transverse_start + rr,
                                base_depth + offset,
                            ));
                        }
                    }
                    let key_result = self.shelf_candidate_key(&cells, P, incoming_T, occ, state);
                    self.trace_shelf_candidate(&key_result);
                    let Ok(shelf_key) = key_result else {
                        continue;
                    };
                    let component = info.component[cells[0]];
                    if component < 0 {
                        continue;
                    }
                    let cheap_score = cells.iter().map(|&cell| weights.cell[cell]).sum();
                    let mut next = *occ;
                    for &cell in &cells {
                        next[cell / self.N] |= Self::bit_at(cell % self.N);
                    }
                    let fragment_delta = if fast_mode {
                        0.0
                    } else {
                        local! {
                            self.trace.count("fragment_evaluated");
                        }
                        self.fragment_metric(&next) - info.metric
                    };
                    candidates.push(Placement {
                        shape_index,
                        x: transverse_start,
                        y: base_depth,
                        perimeter,
                        cheap_score,
                        final_score: cheap_score - 1.4 * fragment_delta,
                        component_size: info.sizes[component as usize],
                        slot_delay: 0,
                        slot_penalty: 0.0,
                        slot_count: 0,
                        explicit_cells: cells,
                        shelf_key,
                        shelf_kind: 1,
                    });
                }
            }
        }
        candidates
    }

    #[allow(clippy::too_many_arguments)]
    fn push_shelf_growth_frontier(
        &self,
        run_id: usize,
        state: &ShelfState,
        increments: &[usize],
        compatible: &[bool],
        selected: &[bool],
        component: isize,
        seed_r: usize,
        seed_c: usize,
        info: &FreeInfo,
        weights: &WeightData,
        cut_loss: &[usize],
        frontier: &mut BinaryHeap<Reverse<(i64, usize, usize, usize, i64, usize, usize, usize)>>,
    ) {
        if !compatible[run_id] {
            return;
        }
        let depth = state.frontier[run_id] + increments[run_id];
        let run = &self.shelf_layout.runs[run_id];
        if depth >= run.cells.len() {
            return;
        }
        let cell = run.cells[depth];
        if selected[cell] || info.component[cell] != component {
            return;
        }
        let d = self.count_selected_neighbors(cell, selected);
        if d == 0 {
            return;
        }
        let (a, b, c, d2, e, f) = self.growth_key(d, cell, seed_r, seed_c, weights, cut_loss);
        frontier.push(Reverse((a, b, c, d2, e, f, run_id, depth)));
    }

    #[allow(clippy::too_many_arguments)]
    fn shelf_growth_placement(
        &mut self,
        P: usize,
        incoming_T: usize,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
        state: &ShelfState,
        seed_limit: usize,
    ) -> Option<Placement> {
        local! {
            self.trace.count("shelf_growth_attempt");
        }
        let free: Vec<bool> = info
            .component
            .iter()
            .map(|&component| component >= 0)
            .collect();
        let cut_loss = free_cut_loss(self.N, &free);
        let mut compatible = vec![false; self.shelf_layout.runs.len()];
        let mut seeds = Vec::new();
        for (run_id, run) in self.shelf_layout.runs.iter().enumerate() {
            let depth = state.frontier[run_id];
            if depth >= run.cells.len() {
                continue;
            }
            let cell = run.cells[depth];
            let component = info.component[cell];
            if component < 0 || info.sizes[component as usize] < P {
                continue;
            }
            let (new_run, slack) = if depth == 0 {
                (1, HORIZON + 1 - incoming_T)
            } else {
                let owner = self.owner_cell[run.cells[depth - 1]];
                if owner < 0 {
                    continue;
                }
                let back_T = self.groups[owner as usize].T;
                if incoming_T >= back_T {
                    continue;
                }
                (0, back_T - incoming_T)
            };
            compatible[run_id] = true;
            seeds.push((new_run, slack, cell, run_id));
        }
        seeds.sort_unstable();
        seeds.truncate(seed_limit);

        let mut candidates = Vec::new();
        for &(_, _, seed, seed_run) in &seeds {
            let component = info.component[seed];
            let seed_r = seed / self.N;
            let seed_c = seed % self.N;
            let mut selected = vec![false; self.N * self.N];
            let mut increments = vec![0_usize; self.shelf_layout.runs.len()];
            let mut region = Vec::with_capacity(P);
            selected[seed] = true;
            increments[seed_run] = 1;
            region.push(seed);
            let mut frontier = BinaryHeap::new();

            let mut affected = vec![seed_run];
            for next in [
                (seed_r > 0).then_some(seed - self.N),
                (seed_r + 1 < self.N).then_some(seed + self.N),
                (seed_c > 0).then_some(seed - 1),
                (seed_c + 1 < self.N).then_some(seed + 1),
            ]
            .into_iter()
            .flatten()
            {
                let run_id = self.shelf_layout.run_of_cell[next];
                if run_id != usize::MAX {
                    affected.push(run_id);
                }
            }
            affected.sort_unstable();
            affected.dedup();
            for run_id in affected {
                self.push_shelf_growth_frontier(
                    run_id,
                    state,
                    &increments,
                    &compatible,
                    &selected,
                    component,
                    seed_r,
                    seed_c,
                    info,
                    weights,
                    &cut_loss,
                    &mut frontier,
                );
            }

            while region.len() < P {
                let Some(Reverse((neg_d, _, _, _, _, cell, run_id, depth))) = frontier.pop() else {
                    break;
                };
                if depth != state.frontier[run_id] + increments[run_id] || selected[cell] {
                    continue;
                }
                let current_d = self.count_selected_neighbors(cell, &selected);
                if current_d == 0 || -neg_d != current_d {
                    continue;
                }
                selected[cell] = true;
                increments[run_id] += 1;
                region.push(cell);

                let r = cell / self.N;
                let c = cell % self.N;
                let mut affected = vec![run_id];
                for next in [
                    (r > 0).then_some(cell - self.N),
                    (r + 1 < self.N).then_some(cell + self.N),
                    (c > 0).then_some(cell - 1),
                    (c + 1 < self.N).then_some(cell + 1),
                ]
                .into_iter()
                .flatten()
                {
                    let next_run = self.shelf_layout.run_of_cell[next];
                    if next_run != usize::MAX {
                        affected.push(next_run);
                    }
                }
                affected.sort_unstable();
                affected.dedup();
                for affected_run in affected {
                    self.push_shelf_growth_frontier(
                        affected_run,
                        state,
                        &increments,
                        &compatible,
                        &selected,
                        component,
                        seed_r,
                        seed_c,
                        info,
                        weights,
                        &cut_loss,
                        &mut frontier,
                    );
                }
            }
            if region.len() != P {
                continue;
            }
            let key_result = self.shelf_candidate_key(&region, P, incoming_T, occ, state);
            self.trace_shelf_candidate(&key_result);
            let Ok(shelf_key) = key_result else {
                continue;
            };
            let perimeter = self.perimeter_of_cells(&region);
            let cheap_score = region.iter().map(|&cell| weights.cell[cell]).sum();
            let mut next = *occ;
            for &cell in &region {
                next[cell / self.N] |= Self::bit_at(cell % self.N);
            }
            let fragment_delta = self.fragment_metric(&next) - info.metric;
            local! {
                self.trace.count("fragment_evaluated");
            }
            candidates.push(Placement {
                perimeter,
                cheap_score,
                final_score: cheap_score - 1.4 * fragment_delta,
                component_size: info.sizes[component as usize],
                explicit_cells: region,
                shelf_key,
                shelf_kind: 2,
                ..Placement::default()
            });
        }

        let best_perimeter = candidates
            .iter()
            .map(|candidate| candidate.perimeter)
            .min()?;
        candidates.retain(|candidate| candidate.perimeter == best_perimeter);
        let best_key = candidates
            .iter()
            .map(|candidate| candidate.shelf_key)
            .min()?;
        candidates.retain(|candidate| candidate.shelf_key == best_key);
        let best = candidates
            .into_iter()
            .max_by(|a, b| a.final_score.total_cmp(&b.final_score));
        local! {
            if best.is_some() {
                self.trace.count("shelf_growth_success");
            }
        }
        best
    }

    fn choose_shelf_regular(
        &self,
        mut candidates: Vec<Placement>,
        fast_mode: bool,
    ) -> Option<Vec<Placement>> {
        let best_key = candidates
            .iter()
            .map(|candidate| candidate.shelf_key)
            .min()?;
        candidates.retain(|candidate| candidate.shelf_key == best_key);
        candidates.sort_by(|a, b| {
            b.final_score
                .total_cmp(&a.final_score)
                .then_with(|| b.cheap_score.total_cmp(&a.cheap_score))
        });
        let component_size = candidates[0].component_size;
        candidates.retain(|candidate| candidate.component_size == component_size);
        candidates.truncate(if fast_mode { 1 } else { 4 });
        Some(candidates)
    }

    /// 最初に置ける周長レベルはv047と同じ範囲で探すが、全候補を棚の
    /// ハード制約で検査してから構造キーを比較する。
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
        if info.free_count < P {
            local! {
                self.trace.count("shelf_reject_total_free_shortage");
            }
            return None;
        }
        if !info.sizes.iter().any(|&size| size >= P) {
            local! {
                self.trace.count("shelf_reject_component_shortage");
            }
            return None;
        }
        let state = self.analyze_shelf_state();
        debug_assert_eq!(state.gap_violations, 0);
        debug_assert_eq!(state.deadline_violations, 0);
        let weights =
            self.build_weight_data(&occ, &info, incoming_S, incoming_T, incoming_T - incoming_S);
        let min_L = minimum_perimeter(P);
        let max_extra = if fast_mode { 2 } else { 6 };

        for perimeter in (min_L..=min_L + max_extra).step_by(2) {
            let mut candidates = self.shelf_regular_level(
                P,
                incoming_T,
                &occ,
                &info,
                &weights,
                &state,
                perimeter,
                if fast_mode { 5 } else { usize::MAX },
                fast_mode,
            );
            if candidates.is_empty() {
                continue;
            }
            // 高価なslot calendarは、棚制約を満たすregular候補があると確定してから
            // 一度だけ作る。候補の構造キーはslot評価で変えない。
            if !is_large_target && !fast_mode {
                let calendar = self.build_slot_calendar(incoming_id, incoming_S, theta);
                for candidate in &mut candidates {
                    candidate.slot_delay =
                        self.explicit_slot_delay(&candidate.explicit_cells, incoming_T, &calendar);
                    candidate.slot_penalty = Self::slot_penalty(&calendar, candidate.slot_delay);
                    candidate.slot_count = calendar.slots.len();
                }
            }
            let choices = self
                .choose_shelf_regular(candidates, fast_mode)
                .expect("non-empty shelf regular candidates");
            if is_large_target {
                local! {
                    self.trace.count("large_target_regular");
                }
            }
            local! {
                self.trace.count("shelf_regular_success");
            }
            return Some(choices);
        }

        if fast_mode || self.timer.reached(GROWTH_LIMIT_RATIO) {
            local! {
                self.trace.count("shelf_reject_time_cutoff");
            }
            return None;
        }
        let growth = self
            .shelf_growth_placement(P, incoming_T, &occ, &info, &weights, &state, 44)
            .map(|placement| vec![placement]);
        if is_large_target && growth.is_some() {
            local! {
                self.trace.count("large_target_growth");
            }
        }
        if growth.is_none() {
            local! {
                self.trace.count("shelf_reject_no_candidate");
            }
        }
        let _ = incoming_V;
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
            self.trace.count_by("rearrangement_output", 0);
            for key in [
                "shelf_candidate_checked",
                "shelf_candidate_valid",
                "shelf_candidate_invalid_geometry",
                "shelf_candidate_invalid_gap",
                "shelf_candidate_invalid_deadline",
                "shelf_regular_placed",
                "shelf_growth_placed",
                "shelf_new_runs_sum",
                "shelf_deadline_slack_sum",
                "shelf_reject_total_free_shortage",
                "shelf_reject_component_shortage",
                "shelf_reject_no_candidate",
                "shelf_reject_time_cutoff",
                "shelf_invariant_check",
                "shelf_invariant_gap_violation",
                "shelf_invariant_deadline_violation",
            ] {
                self.trace.count_by(key, 0);
            }
        }
        for turn in 0..self.M {
            let id: usize = scanner.next();
            let S: usize = scanner.next();
            let T: usize = scanner.next();
            let P: usize = scanner.next();
            let V: i64 = scanner.next();
            debug_assert_eq!(id, turn);
            self.remove_expired(S);
            local! {
                self.verify_shelf_invariant();
            }

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
                    debug_assert!(
                        choices
                            .iter()
                            .all(|placement| placement.slot_count == choices[0].slot_count)
                    );
                    let actual_threshold =
                        base_threshold * self.component_threshold_factor(current_component_size);
                    let quality = q_value * compactness(P, current_perimeter);
                    accepted = base_threshold == 0.0 || quality >= actual_threshold;

                    let winner = if accepted && choices.len() >= 2 {
                        self.select_normal_by_rollout(&choices, id, theta, base_threshold)
                    } else {
                        0
                    };
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
                    }
                    let mut placement = choices.swap_remove(winner);
                    debug_assert_eq!(placement.perimeter, current_perimeter);
                    debug_assert_eq!(placement.component_size, current_component_size);
                    // v035 と同じ admission と最終 winner を確定してから、受理済みの
                    // growth/box 由来候補だけを後処理する。
                    if accepted && placement.shelf_kind == 2 {
                        let baseline = placement.clone();
                        let mut improved = local_time!(self.trace, "biased_swap", {
                            self.improve_final_growth_by_biased_swap(placement, V)
                        });
                        let state = self.analyze_shelf_state();
                        let key_result = self.shelf_candidate_key(
                            &improved.explicit_cells,
                            P,
                            T,
                            &self.occupied_rows,
                            &state,
                        );
                        self.trace_shelf_candidate(&key_result);
                        if let Ok(key) = key_result {
                            if key <= baseline.shelf_key {
                                improved.shelf_key = key;
                                placement = improved;
                                local! {
                                    if placement.explicit_cells != baseline.explicit_cells {
                                        self.trace.count("shelf_local_improvement_kept");
                                    }
                                }
                            } else {
                                placement = baseline;
                                local! {
                                    self.trace.count("shelf_local_improvement_structure_reject");
                                }
                            }
                        } else {
                            placement = baseline;
                            local! {
                                self.trace.count("shelf_local_improvement_invalid_reject");
                            }
                        }
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
                let _shelf_key = placement.shelf_key;
                let _shelf_kind = placement.shelf_kind;
                self.commit_normal_placement(id, placement);
                local! {
                    self.trace
                        .count_by("shelf_new_runs_sum", _shelf_key.new_runs as i64);
                    self.trace.count_by(
                        "shelf_deadline_slack_sum",
                        _shelf_key.deadline_slack as i64,
                    );
                    if _shelf_kind == 1 {
                        self.trace.count("shelf_regular_placed");
                    } else if _shelf_kind == 2 {
                        self.trace.count("shelf_growth_placed");
                    }
                    self.verify_shelf_invariant();
                }
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

#[cfg(feature = "local")]
pub fn verify_deadline_shelves() {
    fn full_grass(N: usize) -> Rows {
        let mut rows = [0_u64; MAX_N];
        for row in rows.iter_mut().take(N) {
            *row = (1_u64 << N) - 1;
        }
        rows
    }

    // 対称盤面では規定の同点順により左→右になる。
    let grass = full_grass(6);
    let chosen = ShelfLayout::choose(6, &grass);
    assert_eq!(chosen.direction, ShelfDirection::LeftToRight);

    // 4方向の正規座標は物理盤面の全セルと一対一に対応する。
    for direction in ShelfDirection::ALL {
        let mut seen = vec![false; 36];
        for transverse in 0..6 {
            for depth in 0..6 {
                let cell = direction.physical_cell(6, transverse, depth);
                assert!(!seen[cell]);
                seen[cell] = true;
            }
        }
        assert!(seen.into_iter().all(|value| value));
    }

    // 同じ行でも池を挟む区間は別の棚になる。
    let mut split_grass = [0_u64; MAX_N];
    split_grass[0] = 0b1_1011;
    let split = ShelfLayout::build(5, &split_grass, ShelfDirection::LeftToRight);
    assert_eq!(split.runs_by_transverse[0].len(), 2);
    assert_ne!(split.run_of_cell[1], split.run_of_cell[3]);

    // 単一棚でfrontier直後かつ短いTだけを許し、gapと遅いTを拒否する。
    let mut solver = Solver::new(6, 8, grass, TimeKeeper::new(60.0));
    solver.groups[0] = Group {
        id: 0,
        T: 100,
        P: 2,
        active: true,
        cells: vec![0, 1],
        ..Group::default()
    };
    solver.place_group_on_board(0, &[0, 1]);
    let state = solver.analyze_shelf_state();
    assert_eq!(state.gap_violations, 0);
    assert_eq!(state.deadline_violations, 0);
    assert!(
        solver
            .shelf_candidate_key(&[2, 3], 2, 90, &solver.occupied_rows, &state)
            .is_ok()
    );
    assert_eq!(
        solver.shelf_candidate_key(&[2, 3], 2, 110, &solver.occupied_rows, &state),
        Err(ShelfCandidateError::Deadline)
    );
    assert_eq!(
        solver.shelf_candidate_key(&[3, 4], 2, 90, &solver.occupied_rows, &state),
        Err(ShelfCandidateError::Gap)
    );

    // 複数棚にまたがるgroupも、手前のgroupが先に退去すればprefixを保つ。
    let mut multi = Solver::new(4, 8, full_grass(4), TimeKeeper::new(60.0));
    multi.groups[0] = Group {
        id: 0,
        T: 100,
        P: 2,
        active: true,
        cells: vec![0, 4],
        ..Group::default()
    };
    multi.place_group_on_board(0, &[0, 4]);
    multi.departures.push(Reverse((100, 0)));
    let state = multi.analyze_shelf_state();
    let front_cells = [1, 5];
    assert!(
        multi
            .shelf_candidate_key(&front_cells, 2, 90, &multi.occupied_rows, &state)
            .is_ok()
    );
    multi.groups[1] = Group {
        id: 1,
        T: 90,
        P: 2,
        active: true,
        cells: front_cells.to_vec(),
        ..Group::default()
    };
    multi.place_group_on_board(1, &front_cells);
    multi.departures.push(Reverse((90, 1)));
    let occupied = multi.analyze_shelf_state();
    assert_eq!(occupied.gap_violations, 0);
    assert_eq!(occupied.deadline_violations, 0);
    multi.remove_expired(91);
    let after_departure = multi.analyze_shelf_state();
    assert_eq!(after_departure.gap_violations, 0);
    assert_eq!(after_departure.deadline_violations, 0);
    assert_eq!(
        after_departure.frontier[multi.shelf_layout.run_of_cell[0]],
        1
    );
    assert_eq!(
        after_departure.frontier[multi.shelf_layout.run_of_cell[4]],
        1
    );

    // 本番と同じfrontier growthが、正確な面積・連結性・棚制約を満たす。
    let mut growth = Solver::new(5, 8, full_grass(5), TimeKeeper::new(60.0));
    let occ = growth.occupied_rows;
    let info = growth.compute_free_info(&occ, true);
    let weights = WeightData {
        prefix: [[0.0; MAX_N + 1]; MAX_N],
        cell: [0.0; MAX_N * MAX_N],
    };
    let state = growth.analyze_shelf_state();
    let regular = growth.shelf_regular_level(
        4,
        90,
        &occ,
        &info,
        &weights,
        &state,
        minimum_perimeter(4),
        usize::MAX,
        false,
    );
    assert!(!regular.is_empty());
    assert!(regular.iter().all(|placement| {
        growth
            .shelf_candidate_key(&placement.explicit_cells, 4, 90, &occ, &state)
            .is_ok()
    }));
    let placement = growth
        .shelf_growth_placement(4, 90, &occ, &info, &weights, &state, 44)
        .expect("frontier growth must find a four-cell region");
    assert_eq!(placement.explicit_cells.len(), 4);
    assert!(growth.explicit_candidate_is_valid(&placement.explicit_cells, 4, &occ));
    assert!(
        growth
            .shelf_candidate_key(&placement.explicit_cells, 4, 90, &occ, &state)
            .is_ok()
    );
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
