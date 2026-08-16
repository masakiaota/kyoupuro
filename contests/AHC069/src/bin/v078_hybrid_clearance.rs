// v078_hybrid_clearance.rs
#![allow(non_snake_case)]
// 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。
// 中心アイデア: 静的草地 clearance が 2.10 以下なら rough 専門家、
// それ以外なら v053 相当の smooth 専門家を使う二専門家構成。

use statrs::function::erf::erfc;
use std::cmp::{Ordering, Reverse};
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

// 各打ち切り時刻を、基準時間 1.90 秒に対する割合として保持する。
// v501 の実測 (case 0002 で 271ms/1520ms) から時間に大きな余裕があるため、
// relocation 系の締め切りを後ろへ動かし、代わりに累積時間予算で発動を制御する。
const FAST_MODE_RATIO: f64 = 168.0 / 190.0;
const RELOCATION_START_LIMIT_RATIO: f64 = 172.0 / 190.0;
// v008 の本評価 max 1,347ms の余裕内で、締切を v007 相当へ戻す。
const TARGET_SCAN_LIMIT_RATIO: f64 = 176.0 / 190.0;
const GROWTH_LIMIT_RATIO: f64 = 178.0 / 190.0;
const REPACK_LIMIT_RATIO: f64 = 180.0 / 190.0;
/// relocation (target scan + repack) に使ってよい累積時間の割合。
/// 悪形ターンごとに発動すると回数が一桁増えるため、総量で抑えて TLE を防ぐ。
/// 実測 (dry-run max 1180ms / 1520ms) に余裕があるため 0.65 まで使う。
const RELOC_TIME_BUDGET_RATIO: f64 = 0.68;

// ---- ロールアウト評価のパラメータ ----
/// 1 本のロールアウトで見る将来到着数。盤面差が効くのは空きが一巡する θ 程度の
/// 時間幅であり、平均到着間隔 ~100 に対し 22 件 ≈ 2,200 時間を近傍将来として使う。
const ROLLOUT_ARRIVALS: usize = 22;
/// 共通乱数のサンプル本数。候補間で同一の到着列を使うため差の分散は小さい。
const ROLLOUT_SAMPLES: usize = 3;

// ---- v047 由来の高回収余地 growth 後処理 ----
const BIASED_SWAP_START_RATIO: f64 = 0.86;
const BIASED_SWAP_LIMIT_RATIO: f64 = 0.90;
const BIASED_SWAP_ITERATIONS: usize = 512;
const BIASED_SWAP_MIN_RECOVERABLE_FEE: f64 = 300_000.0;

/// 乱数 (再現性のためターン番号からシードを決める)。標準的な xorshift64*。
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

/// ロールアウト比較にかける候補 1 つ (プラン適用後 or baseline)。
struct RolloutCandidate {
    board: Rows,
    /// V×C − 移動費 − fee_loss の即時実額。棄却 baseline は 0。
    immediate: f64,
    /// incoming を受け入れる候補なら、その (T, cells)。退去処理に使う。
    incoming_dep: Option<(usize, Vec<usize>)>,
    /// 移動した blocker の新しい cells (退去処理でこちらを使う)。
    overrides: Vec<(usize, Vec<usize>)>,
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
    ($trace:expr, $key:expr, $body:block) => {{
        $body
    }};
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn from_start(time_limit_sec: f64, start: Instant) -> Self {
        Self {
            start,
            time_limit_sec,
        }
    }

    #[inline]
    fn reached(&self, ratio: f64) -> bool {
        self.start.elapsed().as_secs_f64() >= self.time_limit_sec * ratio
    }

    #[inline]
    fn elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline]
    fn budget_sec(&self, ratio: f64) -> f64 {
        self.time_limit_sec * ratio
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
    accepted: bool,
    worst_perimeter: usize,
    move_count: usize,
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
            explicit_cells: Vec::new(),
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

#[derive(Default)]
struct MovePlan {
    ok: bool,
    incoming: Placement,
    moved: Vec<(usize, Placement)>,
}

struct TargetOption {
    placement: Placement,
    blockers: Vec<usize>,
    rank: f64,
}

#[derive(Clone)]
struct BeamState {
    occ: Rows,
    placements: Vec<(usize, Placement)>,
    score: f64,
}

struct Solver {
    N: usize,
    M: usize,
    R_milli: i64,
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
    // 空板 (grass のみ) は不変なので、relocation の target scan 用に前計算して使い回す。
    empty_runs: Box<RunTable>,
    empty_component: Vec<isize>,
    empty_sizes: Vec<usize>,
    permanent_weights: Box<WeightData>,
    /// P → 最小周長での C。ロールアウトの admission 判定と価値計算に使う。
    c_max_table: Vec<f64>,
    /// θ 事後グリッド (2000+50k) の累積重み。posterior_theta 更新時に保存し、
    /// ロールアウト各本の θ を事後分布からサンプルするのに使う。
    theta_cum: Vec<f64>,
    /// θ 事後の標準偏差。不確実性が小さくなったらサンプリングを点推定へ戻す。
    theta_sd: f64,
    /// relocation (target scan + repack) に費やした累積秒数。予算超過で発動を止める。
    reloc_spent_sec: f64,
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
    fn new(N: usize, M: usize, R_milli: i64, grass_rows: Rows, timer: TimeKeeper) -> Self {
        let mut groups = vec![Group::default(); M];
        for (id, group) in groups.iter_mut().enumerate() {
            group.id = id;
        }
        let mut solver = Self {
            N,
            M,
            R_milli,
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
            empty_runs: Box::new([[0_u64; MAX_N + 1]; MAX_N]),
            empty_component: Vec::new(),
            empty_sizes: Vec::new(),
            permanent_weights: Box::new(WeightData {
                prefix: [[0.0; MAX_N + 1]; MAX_N],
                cell: [0.0; MAX_N * MAX_N],
            }),
            c_max_table: (0..=MAX_P)
                .map(|P| {
                    if P >= 4 {
                        compactness(P, minimum_perimeter(P))
                    } else {
                        0.0
                    }
                })
                .collect(),
            reloc_spent_sec: 0.0,
            theta_cum: Vec::new(),
            theta_sd: 1_732.0,
            timer,
            #[cfg(feature = "local")]
            trace: TraceStats::default(),
        };
        solver.initialize_p_distribution();
        solver.initialize_static_capacity();
        solver.initialize_empty_board_cache();
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

    /// 空板の run table・連結成分・恒久 weight を 1 回だけ計算する。
    /// relocation の target scan が毎回作り直していたものを置き換える。
    fn initialize_empty_board_cache(&mut self) {
        let empty_occ = [0_u64; MAX_N];
        *self.empty_runs = self.build_run_table(&empty_occ);
        let info = self.compute_free_info(&empty_occ, false);
        *self.permanent_weights = self.build_weight_data(&empty_occ, &info, 0, 5_000.0, false);
        self.empty_component = info.component;
        self.empty_sizes = info.sizes;
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
        &self,
        occ: &Rows,
        info: &FreeInfo,
        incoming_T: usize,
        theta: f64,
        use_owner_time: bool,
    ) -> WeightData {
        let mut data = WeightData {
            prefix: [[0.0; MAX_N + 1]; MAX_N],
            cell: [0.0; MAX_N * MAX_N],
        };
        const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for r in 0..self.N {
            for c in 0..self.N {
                let id = r * self.N + c;
                let mut weight = 0.0;
                if self.is_free(occ, r, c) {
                    for (dr, dc) in DIRS {
                        let nr = (r as isize) + dr;
                        let nc = (c as isize) + dc;
                        if nr < 0 || nr >= self.N as isize || nc < 0 || nc >= self.N as isize {
                            weight += 10.0;
                        } else {
                            let nr = nr as usize;
                            let nc = nc as usize;
                            if !self.is_grass(nr, nc) {
                                weight += 10.0;
                            } else if ((occ[nr] >> nc) & 1) != 0 {
                                if use_owner_time {
                                    let owner = self.owner_cell[nr * self.N + nc];
                                    if owner >= 0
                                        && (owner as usize) < self.M
                                        && self.groups[owner as usize].active
                                    {
                                        let z = (self.groups[owner as usize].T as f64
                                            - incoming_T as f64)
                                            / theta.max(1.0);
                                        if z >= 0.0 {
                                            weight += 8.0 + 8.0 * (-z.min(7.0)).exp();
                                        } else {
                                            weight += 2.0 + 8.0 * z.max(-7.0).exp();
                                        }
                                    } else {
                                        weight += 8.0;
                                    }
                                } else {
                                    weight += 10.0;
                                }
                            }
                        }
                    }
                    let component_id = info.component[id];
                    if component_id >= 0 {
                        let component_size = info.sizes[component_id as usize];
                        if component_size < MAX_P {
                            weight += 2.5 * (1.0 - self.fit_probability(component_size));
                        }
                    }
                    weight -= 1e-7 * (id as f64);
                }
                data.cell[id] = weight;
                data.prefix[r][c + 1] = data.prefix[r][c] + weight;
            }
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
    /// P<=150 なので、提案ごとの明示的な DFS でも十分軽い。
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

    /// 共有辺数 d を最優先する成長キー。性質2 (L += 4−2d) より、d 最大のセル追加が
    /// 周長増加を最小化する。タイブレークは従来の ring/manhattan/attraction。
    #[inline]
    fn growth_key(
        &self,
        d: i64,
        id: usize,
        seed_r: usize,
        seed_c: usize,
        weights: &WeightData,
    ) -> i64 {
        let r = id / self.N;
        let c = id % self.N;
        let ring = r.abs_diff(seed_r).max(c.abs_diff(seed_c));
        let manhattan = r.abs_diff(seed_r) + c.abs_diff(seed_c);
        let attraction = (weights.cell[id] * 30.0).round() as i64;
        -d * 100_000_000 + 100_000 * (ring as i64) + 1_000 * (manhattan as i64) - attraction
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
        selected: &[bool],
        frontier: &mut BinaryHeap<Reverse<(i64, i64, usize)>>,
    ) {
        if selected[id] || info.component[id] != component {
            return;
        }
        let d = self.count_selected_neighbors(id, selected);
        let key = self.growth_key(d, id, seed_r, seed_c, weights);
        frontier.push(Reverse((key, d, id)));
    }

    /// 規則形状が置けないときだけ、同一成分内を中心から成長させて連結形状を作る。
    /// これは失敗を隠す経路ではなく、非矩形な空き領域を利用する明示的な第二配置法である。
    fn growth_placement(
        &mut self,
        P: usize,
        occ: &Rows,
        info: &FreeInfo,
        weights: &WeightData,
        maximum_perimeter: usize,
        seed_limit: usize,
    ) -> Option<Placement> {
        local! {
            self.trace.count("growth_placement_attempt");
        }
        let mut seeds = Vec::new();
        let mut used_seed = vec![false; self.N * self.N];
        for component_id in 0..info.sizes.len() {
            if info.sizes[component_id] < P || component_id >= info.cells.len() {
                continue;
            }
            let list = &info.cells[component_id];
            let mut best1 = None;
            let mut best2 = None;
            for &id in list {
                if best1.is_none() || weights.cell[id] > weights.cell[best1.unwrap()] {
                    best2 = best1;
                    best1 = Some(id);
                } else if best2.is_none() || weights.cell[id] > weights.cell[best2.unwrap()] {
                    best2 = Some(id);
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
                        &selected,
                        &mut frontier,
                    );
                }
            }
            while region.len() < P {
                let Some(Reverse((_, d_recorded, id))) = frontier.pop() else {
                    break;
                };
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
        let best_perimeter = candidates.iter().map(|p| p.perimeter).min()?;
        let mut best: Option<Placement> = None;
        for mut candidate in candidates {
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
                .is_none_or(|current| candidate.final_score > current.final_score)
            {
                best = Some(candidate);
            }
        }
        if let Some(_candidate) = &best {
            local! {
                self.trace.count("growth_placement_success");
                self.trace.count_by(
                    "growth_slack_sum",
                    (_candidate.perimeter - minimum_perimeter(P)) as i64,
                );
            }
        }
        best
    }

    /// v047 で有効だった次数偏重の連結1セル交換を、受理済み growth winner にだけ行う。
    /// 同周長では座標を変えず、厳密な周長改善だけを後続の baseline に反映する。
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
        assert!(initial
            .explicit_cells
            .iter()
            .all(|&id| info.component[id] == component));

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

            // 関節境界を全列挙せず、最大12回の重み付き抽選で外せるセルを選ぶ。
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

            // P-1 セルの frontier へ足すため、追加後の連結性は自動で保たれる。
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

        // best は release でも完全検証し、不正候補を出力させない。
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
            local! {
                self.trace.count("biased_swap_applied");
            }
            let mut result = initial;
            result.perimeter = best_perimeter;
            result.explicit_cells = best_cells;
            result
        } else {
            initial
        }
    }

    /// 最初に置ける周長レベルは現行どおり固定し、候補0に現行評価最大を置く。
    /// 非 fast mode では、同じ component_size 内の局所重み最大・断片化増分最小も
    /// 重複除去して返す。同じ component_size なので admission 閾値は変わらない。
    fn find_normal_placements(
        &mut self,
        P: usize,
        incoming_T: usize,
        theta: f64,
        fast_mode: bool,
    ) -> Option<Vec<Placement>> {
        let occ = self.occupied_rows;
        let info = self.compute_free_info(&occ, true);
        if !info.sizes.iter().any(|&size| size >= P) {
            return None;
        }
        let weights = self.build_weight_data(&occ, &info, incoming_T, theta, true);
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

            let mut choices = Vec::with_capacity(3);
            for index in [current_index, cheap_index, fragment_index] {
                let candidate = &evaluated[index].0;
                if !choices
                    .iter()
                    .any(|choice: &Placement| choice.explicit_cells == candidate.explicit_cells)
                {
                    choices.push(candidate.clone());
                }
            }
            return Some(choices);
        }

        if fast_mode || self.timer.reached(GROWTH_LIMIT_RATIO) {
            return None;
        }
        self.growth_placement(P, &occ, &info, &weights, usize::MAX, 44)
            .map(|placement| vec![placement])
    }

    fn posterior_theta(&mut self) -> f64 {
        if self.duration_count == 0 {
            self.theta_cum.clear();
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
        let mut sum_theta2 = 0.0;
        self.theta_cum.clear();
        self.theta_cum.reserve(GRID);
        for (k, value) in log_weight.iter().enumerate() {
            let theta = 2_000.0 + 50.0 * (k as f64);
            let weight = (*value - max_log).exp();
            sum_w += weight;
            sum_theta += weight * theta;
            sum_theta2 += weight * theta * theta;
            self.theta_cum.push(sum_w);
        }
        let mean = sum_theta / sum_w;
        self.theta_sd = (sum_theta2 / sum_w - mean * mean).max(0.0).sqrt();
        mean
    }

    /// θ 事後分布からのサンプル。u ∈ (0,1) を累積重みで逆変換する。
    /// 観測が無い間は事前 (2000..8000 の一様) からサンプルする。
    fn sample_theta(&self, u: f64) -> f64 {
        if self.theta_cum.is_empty() {
            return 2_000.0 + 6_000.0 * u;
        }
        let total = *self.theta_cum.last().unwrap();
        let target = u * total;
        let idx = self.theta_cum.partition_point(|&c| c < target);
        2_000.0 + 50.0 * (idx.min(self.theta_cum.len() - 1) as f64)
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

    fn movement_cost(&self, group_id: usize) -> i64 {
        ((self.groups[group_id].V * self.R_milli + 500) / 1_000).max(1)
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

    fn insert_target_option(top: &mut Vec<TargetOption>, option: TargetOption, limit: usize) {
        if top.len() < limit {
            top.push(option);
            return;
        }
        let mut worst = 0;
        for i in 1..limit {
            if top[i].rank > top[worst].rank {
                worst = i;
            }
        }
        if option.rank < top[worst].rank {
            top[worst] = option;
        }
    }

    fn collect_relocation_targets(
        &mut self,
        incoming: &Group,
        q_value: f64,
        base_threshold: f64,
        blocker_limit: usize,
        baseline: f64,
    ) -> Vec<TargetOption> {
        let min_L = minimum_perimeter(incoming.P);
        let mut seen = vec![0_usize; self.M];
        let mut stamp = 0_usize;
        const OPTION_LIMIT: usize = 22;
        let shapes = self.shapes_by_p[incoming.P].clone();
        let mut options = Vec::new();

        for perimeter in (min_L..=min_L + 2).step_by(2) {
            let C = compactness(incoming.P, perimeter);
            for (shape_index, shape) in shapes.iter().enumerate() {
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
                        ys &= self.empty_runs[x + rr][shape.len[rr]] >> shape.left[rr];
                    }
                    while ys != 0 {
                        let y = ys.trailing_zeros() as usize;
                        ys &= ys - 1;
                        let mut overlap = 0_u32;
                        for rr in 0..shape.h {
                            let mask = ((1_u64 << shape.len[rr]) - 1) << (y + shape.left[rr]);
                            overlap += (mask & self.occupied_rows[x + rr]).count_ones();
                        }
                        if overlap == 0 {
                            continue;
                        }

                        stamp += 1;
                        let mut blockers = Vec::with_capacity(blocker_limit + 1);
                        let mut invalid = false;
                        for rr in 0..shape.h {
                            if invalid {
                                break;
                            }
                            let r = x + rr;
                            let begin = y + shape.left[rr];
                            let end = begin + shape.len[rr];
                            for c in begin..end {
                                let owner = self.owner_cell[r * self.N + c];
                                if owner < 0 {
                                    continue;
                                }
                                let owner = owner as usize;
                                if seen[owner] == stamp {
                                    continue;
                                }
                                seen[owner] = stamp;
                                // 移動回数上限は無限往復の安全弁だけに残し、採算は純利益判定に任せる。
                                if !self.groups[owner].active || self.groups[owner].move_count >= 4
                                {
                                    invalid = true;
                                    break;
                                }
                                blockers.push(owner);
                                if blockers.len() > blocker_limit {
                                    invalid = true;
                                    break;
                                }
                            }
                        }
                        if invalid || blockers.is_empty() {
                            continue;
                        }

                        let first_cell = x * self.N + y + shape.left[0];
                        let component_id = self.empty_component[first_cell];
                        let component_size = if component_id >= 0 {
                            self.empty_sizes[component_id as usize]
                        } else {
                            MAX_P
                        };
                        let threshold =
                            base_threshold * self.component_threshold_factor(component_size);
                        let scale =
                            (incoming.P as f64) * ((incoming.T - incoming.S) as f64).powf(0.9);
                        let surplus = scale * (q_value * C - threshold);
                        if surplus <= 0.0 {
                            continue;
                        }
                        let cost: i64 = blockers
                            .iter()
                            .map(|&group_id| self.movement_cost(group_id))
                            .sum();
                        // 悪形配置の純利益 (baseline) を上回れない target は repack する価値がない。
                        if surplus <= baseline + 1.12 * (cost as f64) {
                            continue;
                        }

                        let mut contact = 0.0;
                        for rr in 0..shape.h {
                            let begin = y + shape.left[rr];
                            let end = begin + shape.len[rr];
                            contact += self.permanent_weights.prefix[x + rr][end]
                                - self.permanent_weights.prefix[x + rr][begin];
                        }
                        let blocker_count = blockers.len();
                        Self::insert_target_option(
                            &mut options,
                            TargetOption {
                                placement: Placement {
                                    shape_index,
                                    x,
                                    y,
                                    perimeter,
                                    component_size,
                                    ..Placement::default()
                                },
                                blockers,
                                rank: (cost as f64) / surplus + 0.035 * (blocker_count as f64)
                                    - 0.00035 * contact,
                            },
                            OPTION_LIMIT,
                        );
                    }
                }
            }
            if self.timer.reached(TARGET_SCAN_LIMIT_RATIO) {
                break;
            }
        }
        options.sort_by(|a, b| a.rank.total_cmp(&b.rank));
        local! {
            self.trace
                .count_by("relocation_option", options.len() as i64);
        }
        options
    }

    /// blocker たちを target を塞いだ盤面へ置き直す。戻り値は
    /// (配置列, fee_loss 合計, repack 後盤面の断片化メトリック)。
    /// fee_loss は worst_perimeter を超える形へ逃げた blocker の退去時利用料の減少分で、
    /// 呼び出し側が移動費と合わせて採算判定に使う。断片化メトリックはプラン間の品質比較に使う。
    fn repack_blockers(
        &mut self,
        blocker_ids: &[usize],
        target_cells: &[usize],
    ) -> Option<(Vec<(usize, Placement)>, f64, f64)> {
        local! {
            self.trace.count("repack_attempt");
        }
        let mut base = self.occupied_rows;
        for &group_id in blocker_ids {
            for &cell in &self.groups[group_id].cells {
                base[cell / self.N] &= !Self::bit_at(cell % self.N);
            }
        }
        for &cell in target_cells {
            base[cell / self.N] |= Self::bit_at(cell % self.N);
        }

        let mut order = blocker_ids.to_vec();
        order.sort_by(|&a, &b| {
            let ga = &self.groups[a];
            let gb = &self.groups[b];
            let slack_a = ga.worst_perimeter - minimum_perimeter(ga.P);
            let slack_b = gb.worst_perimeter - minimum_perimeter(gb.P);
            slack_a
                .cmp(&slack_b)
                .then_with(|| gb.P.cmp(&ga.P))
                .then_with(|| gb.T.cmp(&ga.T))
        });

        let mut beam = vec![BeamState {
            occ: base,
            placements: Vec::new(),
            score: 0.0,
        }];
        const BEAM_WIDTH: usize = 9;
        const BRANCHES: usize = 4;

        for group_id in order {
            let P = self.groups[group_id].P;
            let worst_perimeter = self.groups[group_id].worst_perimeter;
            let mut next_beam = Vec::new();
            for state in &beam {
                if self.timer.reached(REPACK_LIMIT_RATIO) {
                    break;
                }
                let info = self.compute_free_info(&state.occ, true);
                if !info.sizes.iter().any(|&size| size >= P) {
                    continue;
                }
                let weights = self.build_weight_data(&state.occ, &info, 0, 5_000.0, false);
                let runs = self.build_run_table(&state.occ);
                let mut candidates = Vec::new();
                // まず worst 以下 (C 無損失) を優先する段階探索。見つからなければ worst+6 まで
                // 損失込みで許す。逃げ先の品質制約が repack 失敗の主因 (Q4) だったため、
                // 採否は fee_loss を含む最終採算に任せる。
                for perimeter in (minimum_perimeter(P)..=worst_perimeter + 6).step_by(2) {
                    candidates =
                        self.scan_regular_level(P, &runs, &weights, perimeter, BRANCHES, 8);
                    if !candidates.is_empty() {
                        break;
                    }
                }
                if candidates.is_empty()
                    && !self.timer.reached(GROWTH_LIMIT_RATIO)
                    && let Some(candidate) =
                        self.growth_placement(P, &state.occ, &info, &weights, usize::MAX, 20)
                {
                    candidates.push(candidate);
                }
                // worst 超過の C 低下損失は最終採算で厳密に引く。beam score (無次元) には
                // 混ぜず、perimeter の段階探索で worst 以下を優先することだけで抑える。
                for mut candidate in candidates {
                    let mut child = state.clone();
                    let cells = self.materialize(&candidate, P);
                    for &cell in &cells {
                        child.occ[cell / self.N] |= Self::bit_at(cell % self.N);
                    }
                    candidate.explicit_cells = cells;
                    child.score += candidate.cheap_score;
                    child.placements.push((group_id, candidate));
                    next_beam.push(child);
                }
            }
            if next_beam.is_empty() {
                return None;
            }
            next_beam.sort_by(|a, b| b.score.total_cmp(&a.score));
            next_beam.truncate(BEAM_WIDTH);
            beam = next_beam;
        }

        if beam.is_empty() {
            return None;
        }
        let mut best_index = 0;
        let mut best_score = -1e100;
        let mut best_metric = 0.0;
        for (i, state) in beam.iter().enumerate() {
            let metric = self.fragment_metric(&state.occ);
            let score = state.score - 1.15 * metric;
            local! {
                self.trace.count("fragment_evaluated");
            }
            if score > best_score {
                best_score = score;
                best_index = i;
                best_metric = metric;
            }
        }
        let chosen = beam.swap_remove(best_index).placements;
        let mut fee_loss = 0.0;
        for (group_id, placement) in &chosen {
            let group = &self.groups[*group_id];
            if placement.perimeter > group.worst_perimeter {
                fee_loss += (group.V as f64)
                    * (compactness(group.P, group.worst_perimeter)
                        - compactness(group.P, placement.perimeter));
            }
        }
        local! {
            self.trace.count("repack_success");
            if fee_loss > 0.0 {
                self.trace.count("repack_used_perimeter_over");
            }
        }
        Some((chosen, fee_loss, best_metric))
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
                            self.release_cells_for(&mut occ, gid, &cand.overrides);
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
                        self.release_cells_for(&mut occ, gid, &cand.overrides);
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

    /// gid の占有セルを盤面から解放する。移動プランで動いた blocker は
    /// overrides の新セルを使う。
    fn release_cells_for(&self, occ: &mut Rows, gid: usize, overrides: &[(usize, Vec<usize>)]) {
        let cells: &[usize] = overrides
            .iter()
            .find(|(g, _)| *g == gid)
            .map(|(_, c)| c.as_slice())
            .unwrap_or(&self.groups[gid].cells);
        for &cell in cells {
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
            // predictive rollout: 各本の θ を点推定でなく事後分布からサンプルする。
            // 候補間では同じ θ・同じ到着列 (共通乱数) を使うため比較は公平のまま。
            // SD ゲート (収束後は点推定へ戻す) は実測で悪化したため常時サンプルにする。
            const THETA_SD_GATE: f64 = 0.0;
            let theta_k = if self.theta_sd > THETA_SD_GATE {
                let mut theta_rng = XorShift64::new(seed ^ 0xA5A5_5A5A_1234_5678);
                self.sample_theta(theta_rng.next_f64())
            } else {
                theta
            };
            local! {
                self.trace.count_by(
                    "theta_sample_spread_permille",
                    (((theta_k - theta).abs() / theta.max(1.0)) * 1000.0).round() as i64,
                );
            }
            let arrivals = self.make_future_arrivals(incoming_id, now, theta_k, seed);
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
    /// 即時利用料は全候補で等しいため、勝敗は候補盤面が近い将来に生む差だけで決まる。
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
                immediate: (incoming_V as f64) * compactness(incoming_P, placement.perimeter),
                incoming_dep: Some((incoming_T, cells)),
                overrides: Vec::new(),
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

    /// baseline: 悪形の通常配置を受け入れた場合の純利益 (通常配置が価格判定を通らない
    /// ときは 0)。移動プランの純利益がこれを上回るときだけ ok を返す。
    fn attempt_relocation(
        &mut self,
        incoming: &Group,
        q_value: f64,
        base_threshold: f64,
        baseline: f64,
        baseline_normal: Option<(Vec<usize>, f64)>,
        theta: f64,
    ) -> MovePlan {
        if self.timer.reached(RELOCATION_START_LIMIT_RATIO) {
            return MovePlan::default();
        }
        // 予算超過後は発動しない。悪形ターンごとの発動で総時間が伸びるのを抑える。
        if self.reloc_spent_sec >= self.timer.budget_sec(RELOC_TIME_BUDGET_RATIO) {
            local! {
                self.trace.count("reloc_budget_hit");
            }
            return MovePlan::default();
        }
        let max_compactness = compactness(incoming.P, minimum_perimeter(incoming.P));
        if q_value * max_compactness <= base_threshold * 0.92 {
            return MovePlan::default();
        }
        let blocker_limit = 4;
        let options = self.collect_relocation_targets(
            incoming,
            q_value,
            base_threshold,
            blocker_limit,
            baseline,
        );
        let scale = (incoming.P as f64) * ((incoming.T - incoming.S) as f64).powf(0.9);
        // 採算を満たすプランを最大 3 つ集め、純利益と repack 後盤面の断片化メトリックの
        // 複合評価で最良を採用する。最初の成功で打ち切ると rank 下位 target の質の低い
        // 移動が混ざり、メトリック単独では利益差を捨てる (0077 -1.0% 実測) ため複合にする。
        // (plan, 複合評価に使う純利益 net, repack 後の断片化メトリック, 移動費, fee_loss)
        let mut collected: Vec<(MovePlan, f64, f64, i64, f64)> = Vec::new();
        for (attempt, option) in options.into_iter().enumerate() {
            if attempt >= 12 || collected.len() >= 3 || self.timer.reached(REPACK_LIMIT_RATIO) {
                break;
            }
            let target_cells = self.materialize(&option.placement, incoming.P);
            let Some((repacked, fee_loss, final_metric)) =
                self.repack_blockers(&option.blockers, &target_cells)
            else {
                continue;
            };
            // repack で確定した blocker の C 低下損失込みで最終採算を判定する。
            // surplus - cost - fee_loss > baseline + 0.12*cost が採用条件。
            let cost: i64 = repacked
                .iter()
                .map(|(group_id, _)| self.movement_cost(*group_id))
                .sum();
            let surplus = {
                let C = compactness(incoming.P, option.placement.perimeter);
                let threshold = base_threshold
                    * self.component_threshold_factor(option.placement.component_size);
                scale * (q_value * C - threshold)
            };
            if surplus - (cost as f64) - fee_loss <= baseline + 0.12 * (cost as f64) {
                continue;
            }
            let mut incoming_placement = option.placement;
            incoming_placement.explicit_cells = target_cells;
            let net = surplus - (cost as f64) - fee_loss;
            collected.push((
                MovePlan {
                    ok: true,
                    incoming: incoming_placement,
                    moved: repacked,
                },
                net,
                final_metric,
                cost,
                fee_loss,
            ));
        }
        if collected.is_empty() {
            return MovePlan::default();
        }

        // ---- ロールアウト比較: 各プラン + baseline (悪形受け入れ or 棄却) ----
        // 総合値 = 即時実額 (V×C − 移動費 − fee_loss) + 将来受け入れ価値のロールアウト平均。
        // シャドー価格は使わない (機会費用はロールアウトが直接測る)。
        let mut cands: Vec<RolloutCandidate> = Vec::with_capacity(collected.len() + 1);
        for (plan, _, _, cost, fee_loss) in &collected {
            let mut board = self.occupied_rows;
            for (gid, _) in &plan.moved {
                for &cell in &self.groups[*gid].cells {
                    board[cell / self.N] &= !Self::bit_at(cell % self.N);
                }
            }
            let mut overrides = Vec::with_capacity(plan.moved.len());
            for (gid, placement) in &plan.moved {
                for &cell in &placement.explicit_cells {
                    board[cell / self.N] |= Self::bit_at(cell % self.N);
                }
                overrides.push((*gid, placement.explicit_cells.clone()));
            }
            for &cell in &plan.incoming.explicit_cells {
                board[cell / self.N] |= Self::bit_at(cell % self.N);
            }
            let C = compactness(incoming.P, plan.incoming.perimeter);
            cands.push(RolloutCandidate {
                board,
                immediate: (incoming.V as f64) * C - (*cost as f64) - fee_loss,
                incoming_dep: Some((incoming.T, plan.incoming.explicit_cells.clone())),
                overrides,
            });
        }
        // baseline 候補 (最後の index): 悪形の通常配置を受けるか、何もしない (棄却)。
        let baseline_index = cands.len();
        match &baseline_normal {
            Some((cells, C_normal)) => {
                let mut board = self.occupied_rows;
                for &cell in cells {
                    board[cell / self.N] |= Self::bit_at(cell % self.N);
                }
                cands.push(RolloutCandidate {
                    board,
                    immediate: (incoming.V as f64) * C_normal,
                    incoming_dep: Some((incoming.T, cells.clone())),
                    overrides: Vec::new(),
                });
            }
            None => {
                cands.push(RolloutCandidate {
                    board: self.occupied_rows,
                    immediate: 0.0,
                    incoming_dep: None,
                    overrides: Vec::new(),
                });
            }
        }
        let winner = local_time!(self.trace, "rollout", {
            self.evaluate_candidates_rollout(&cands, incoming.id, incoming.S, theta, base_threshold)
        });
        local! {
            self.trace.count("rollout_session");
            // 旧複合評価 (net − 800×metric) の勝者と比べ、判断が変わった回数を数える。
            const METRIC_TO_MONEY: f64 = 800.0;
            let mut old_best = 0;
            let mut old_key = collected[0].1 - METRIC_TO_MONEY * collected[0].2;
            for i in 1..collected.len() {
                let key = collected[i].1 - METRIC_TO_MONEY * collected[i].2;
                if key > old_key {
                    old_key = key;
                    old_best = i;
                }
            }
            if winner != old_best {
                self.trace.count("rollout_flip");
            }
        }
        if winner == baseline_index {
            // 採算式では黒字でも、将来の受け入れ機会まで含めると baseline が勝った。
            local! {
                self.trace.count("rollout_reject");
            }
            return MovePlan::default();
        }
        local! {
            self.trace.count("relocation_success");
            self.trace
                .count_by("reloc_plan_collected", collected.len() as i64);
            let (plan, _, _, cost, fee_loss) = &collected[winner];
            self.trace
                .count_by("moved_groups", plan.moved.len() as i64);
            self.trace.count_by("move_cost", *cost);
            self.trace
                .count_by("reloc_fee_loss", fee_loss.round() as i64);
        }
        collected.swap_remove(winner).0
    }

    fn commit_move_plan(&mut self, plan: &MovePlan, incoming_id: usize) {
        for &(group_id, _) in &plan.moved {
            self.clear_group_from_board(group_id);
        }
        for (group_id, placement) in &plan.moved {
            let cells = self.materialize(placement, self.groups[*group_id].P);
            {
                let group = &mut self.groups[*group_id];
                group.cells = cells.clone();
                group.worst_perimeter = group.worst_perimeter.max(placement.perimeter);
                group.move_count += 1;
            }
            self.place_group_on_board(*group_id, &cells);
        }

        let cells = plan.incoming.explicit_cells.clone();
        {
            let incoming = &mut self.groups[incoming_id];
            incoming.cells = cells.clone();
            incoming.worst_perimeter = plan.incoming.perimeter;
            incoming.active = true;
            incoming.accepted = true;
        }
        self.place_group_on_board(incoming_id, &cells);
        self.departures
            .push(Reverse((self.groups[incoming_id].T, incoming_id)));
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
            group.worst_perimeter = placement.perimeter;
            group.active = true;
            group.accepted = true;
        }
        self.place_group_on_board(group_id, &cells);
        self.departures
            .push(Reverse((self.groups[group_id].T, group_id)));
    }

    fn print_move_block<W: Write>(
        &self,
        writer: &mut W,
        plan: Option<&MovePlan>,
    ) -> io::Result<()> {
        let Some(plan) = plan.filter(|plan| plan.ok) else {
            writeln!(writer, "0")?;
            writer.flush()?;
            return Ok(());
        };
        writeln!(writer, "{}", plan.moved.len())?;
        for (group_id, placement) in &plan.moved {
            writeln!(writer, "{group_id}")?;
            for &cell in &placement.explicit_cells {
                writeln!(writer, "{} {}", cell / self.N, cell % self.N)?;
            }
        }
        writer.flush()
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
        self.effective_capacity = (packing_efficiency * (usable as f64)).max(1.0);
    }

    fn run<R: BufRead, W: Write>(
        &mut self,
        scanner: &mut Scanner<R>,
        writer: &mut W,
    ) -> io::Result<()> {
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
            let mut net_normal = 0.0;
            let mut move_plan = MovePlan::default();
            let passed_price_prefilter =
                base_threshold == 0.0 || q_value * optimistic_C >= 0.74 * base_threshold;
            if passed_price_prefilter {
                local! {
                    self.trace.count("normal_search");
                }
                let normal_choices = local_time!(self.trace, "normal_search", {
                    self.find_normal_placements(P, T, theta, fast_mode)
                });
                if let Some(mut choices) = normal_choices {
                    let current_perimeter = choices[0].perimeter;
                    let current_component_size = choices[0].component_size;
                    let actual_threshold =
                        base_threshold * self.component_threshold_factor(current_component_size);
                    let quality = q_value * compactness(P, current_perimeter);
                    accepted = base_threshold == 0.0 || quality >= actual_threshold;

                    let winner = if accepted && choices.len() >= 2 {
                        self.select_normal_by_rollout(&choices, id, theta, base_threshold)
                    } else {
                        0
                    };
                    let mut placement = choices.swap_remove(winner);
                    debug_assert_eq!(placement.perimeter, current_perimeter);
                    debug_assert_eq!(placement.component_size, current_component_size);
                    if accepted && placement.shape_index == usize::MAX {
                        let _before_perimeter = placement.perimeter;
                        placement = local_time!(self.trace, "biased_swap", {
                            self.improve_final_growth_by_biased_swap(placement, V)
                        });
                        local! {
                            if _before_perimeter >= minimum_perimeter(P) + 4
                                && placement.perimeter < minimum_perimeter(P) + 4
                            {
                                self.trace.count("biased_swap_avoided_relocation");
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
                    if accepted {
                        // 悪形配置の純利益。移動プランとの比較基準 (baseline) に使う。
                        let scale = (P as f64) * (duration as f64).powf(0.9);
                        let final_perimeter = normal
                            .as_ref()
                            .expect("accepted normal placement exists")
                            .perimeter;
                        let final_quality = q_value * compactness(P, final_perimeter);
                        net_normal = scale * (final_quality - actual_threshold);
                    } else {
                        local! {
                            self.trace.count("post_placement_price_reject");
                        }
                    }
                }
            }
            local! {
                if !passed_price_prefilter {
                    self.trace.count("price_prefilter_reject");
                }
            }

            // 移動探索の発動: 受け入れ不成立なら v501 と同じ救済条件 (slack>=2 か不在)。
            // 受け入れ成立でも slack>=4 の悪形なら、移動による良形配置と純利益で比較する。
            let min_L = minimum_perimeter(P);
            let reloc_worth_trying = if accepted {
                normal
                    .as_ref()
                    .is_some_and(|placement| placement.perimeter >= min_L + 4)
            } else {
                normal
                    .as_ref()
                    .is_none_or(|placement| placement.perimeter > min_L)
            };
            if !fast_mode && reloc_worth_trying && q_value * optimistic_C >= 0.88 * base_threshold {
                local! {
                    self.trace.count("relocation_attempt");
                }
                let baseline = if accepted { net_normal.max(0.0) } else { 0.0 };
                // ロールアウトの baseline 候補: 悪形受け入れが成立しているならその盤面と実額。
                let baseline_normal = if accepted {
                    normal.as_ref().map(|placement| {
                        (
                            self.materialize(placement, P),
                            compactness(P, placement.perimeter),
                        )
                    })
                } else {
                    None
                };
                let incoming = self.groups[id].clone();
                let reloc_start = self.timer.elapsed_sec();
                move_plan = local_time!(self.trace, "relocation", {
                    self.attempt_relocation(
                        &incoming,
                        q_value,
                        base_threshold,
                        baseline,
                        baseline_normal,
                        theta,
                    )
                });
                self.reloc_spent_sec += self.timer.elapsed_sec() - reloc_start;
                if move_plan.ok {
                    local! {
                        if accepted {
                            self.trace.count("reloc_beats_normal");
                        } else {
                            self.trace.count("reloc_rescue");
                        }
                    }
                    accepted = true;
                }
            }

            if move_plan.ok {
                self.commit_move_plan(&move_plan, id);
                self.print_move_block(writer, Some(&move_plan))?;
                self.print_acceptance(writer, true, &self.groups[id].cells)?;
                local! {
                    self.trace.count("accepted");
                    self.trace.count("relocation_placed");
                }
            } else {
                self.print_move_block(writer, None)?;
                if accepted {
                    let placement = normal.as_ref().expect("accepted normal placement");
                    self.commit_normal_placement(id, placement);
                    self.print_acceptance(writer, true, &self.groups[id].cells)?;
                    local! {
                        self.trace.count("accepted");
                        self.trace.count("normal_placed");
                    }
                } else {
                    self.groups[id].accepted = false;
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

mod rough {
    use super::*;

    const MAX_C: usize = MAX_N * MAX_N;
    const EP: f64 = 59.4974499956;
    const SIGMA_Q: f64 = 0.8 * std::f64::consts::LN_2;

    #[inline]
    fn scaled_time(seconds_at_judge: f64) -> f64 {
        PROGRAM_TIME_LIMIT_SEC * seconds_at_judge / JUDGE_TIME_LIMIT_SEC
    }

    struct XorShift64 {
        x: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { x: seed.max(1) }
        }

        fn next_u64(&mut self) -> u64 {
            self.x ^= self.x << 7;
            self.x ^= self.x >> 9;
            self.x
        }

        fn next_int(&mut self, n: usize) -> usize {
            if n <= 1 {
                0
            } else {
                (self.next_u64() % n as u64) as usize
            }
        }
    }

    #[inline]
    fn splitmix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    // Acklam の逆標準正規分布近似。C++ 側と同じ係数と分岐を保つ。
    fn inv_norm_cdf(p: f64) -> f64 {
        if p <= 0.0 {
            return f64::NEG_INFINITY;
        }
        if p >= 1.0 {
            return f64::INFINITY;
        }
        const A: [f64; 6] = [
            -3.969683028665376e1,
            2.209460984245205e2,
            -2.759285104469687e2,
            1.383577518672690e2,
            -3.066479806614716e1,
            2.506628277459239,
        ];
        const B: [f64; 5] = [
            -5.447609879822406e1,
            1.615858368580409e2,
            -1.556989798598866e2,
            6.680131188771972e1,
            -1.328068155288572e1,
        ];
        const C: [f64; 6] = [
            -7.784894002430293e-3,
            -3.223964580411365e-1,
            -2.400758277161838,
            -2.549732539343734,
            4.374664141464968,
            2.938163982698783,
        ];
        const D: [f64; 4] = [
            7.784695709041462e-3,
            3.224671290700398e-1,
            2.445134137142996,
            3.754408661907416,
        ];
        const PLOW: f64 = 0.02425;
        const PHIGH: f64 = 1.0 - PLOW;
        if p < PLOW {
            let q = (-2.0 * p.ln()).sqrt();
            return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
                / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
        }
        if p > PHIGH {
            let q = (-2.0 * (1.0 - p).ln()).sqrt();
            return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
                / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
        }
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }

    #[derive(Clone, Default)]
    struct Group {
        S: usize,
        T: usize,
        P: usize,
        V: i64,
        active: bool,
        cells: Vec<usize>,
        worst_L: usize,
        hash: u64,
    }

    #[derive(Clone, Default)]
    struct Region {
        cells: Vec<usize>,
        L: usize,
        comp: isize,
        layout: i64,
        order_score: i64,
        hash: u64,
    }

    struct FreeState {
        cid: [isize; MAX_C],
        size: Vec<usize>,
        cells: Vec<Vec<usize>>,
        free_count: usize,
        pref: [[i32; MAX_N + 1]; MAX_N + 1],
        hpref: [[i32; MAX_N + 1]; MAX_N + 1],
        vpref: [[i32; MAX_N + 1]; MAX_N + 1],
    }

    impl FreeState {
        fn new() -> Self {
            Self {
                cid: [-1; MAX_C],
                size: Vec::new(),
                cells: Vec::new(),
                free_count: 0,
                pref: [[0; MAX_N + 1]; MAX_N + 1],
                hpref: [[0; MAX_N + 1]; MAX_N + 1],
                vpref: [[0; MAX_N + 1]; MAX_N + 1],
            }
        }
    }

    #[derive(Clone, Copy, Default)]
    struct CompactSpec {
        a: usize,
        full: usize,
        rem: usize,
        off: usize,
        side: usize,
        rot: usize,
        L: usize,
    }

    struct MovePlan {
        ids: Vec<usize>,
        dest: Vec<Region>,
        incoming: Region,
        net_gain: i64,
    }

    impl Default for MovePlan {
        fn default() -> Self {
            Self {
                ids: Vec::new(),
                dest: Vec::new(),
                incoming: Region::default(),
                net_gain: i64::MIN,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct BetaEval {
        value: f64,
        first: f64,
        second: f64,
        ok: bool,
    }

    impl Default for BetaEval {
        fn default() -> Self {
            Self {
                value: -1e100,
                first: 0.0,
                second: 0.0,
                ok: false,
            }
        }
    }

    #[derive(Clone, Copy, Eq, PartialEq)]
    struct HeapNode {
        key: i32,
        z: usize,
    }

    impl Ord for HeapNode {
        fn cmp(&self, other: &Self) -> Ordering {
            self.key.cmp(&other.key).then(self.z.cmp(&other.z))
        }
    }

    impl PartialOrd for HeapNode {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    pub(super) struct Solver {
        N: usize,
        M: usize,
        R_milli: i64,
        board: Vec<String>,
        occ: [i32; MAX_C],
        nb: [[isize; 4]; MAX_C],
        static_rank: [i32; MAX_C],
        static_comp: [isize; MAX_C],
        static_clear: [u8; MAX_C],
        separator_field: [u8; MAX_C],
        static_comp_cells: Vec<Vec<usize>>,
        static_comp_size: Vec<usize>,
        zobrist: [u64; MAX_C],
        groups: Vec<Group>,
        grass_count: usize,
        occupied_cells: usize,
        capacity_cells: f64,
        duration_y: f64,
        observed_n: usize,
        theta_estimate: f64,
        beta_estimate: f64,
        compact_prior: f64,
        compact_ewma: f64,
        compact_observed: usize,
        terrain_roughness: f64,
        mean_static_clearance: f64,
        operation_deadline: f64,
        compact_specs: Vec<Vec<CompactSpec>>,
        min_perimeter: [usize; MAX_P + 1],
        four_root: [f64; MAX_P + 1],
        rng: XorShift64,
        start_time: Instant,
        grow_mark: [i32; MAX_C],
        grow_stamp: i32,
        block_mark: [i32; MAX_C],
        visit_mark: [i32; MAX_C],
        local_index: [isize; MAX_C],
        eval_stamp: i32,
        #[cfg(feature = "local")]
        trace: TraceStats,
    }

    impl Solver {
        pub(super) fn new(
            N: usize,
            M: usize,
            R_milli: i64,
            board: Vec<String>,
            start_time: Instant,
        ) -> Self {
            let mut min_perimeter = [0; MAX_P + 1];
            let mut four_root = [0.0; MAX_P + 1];
            for p in 1..=MAX_P {
                four_root[p] = 4.0 * (p as f64).sqrt();
                let mut best = usize::MAX;
                for a in 1..=p {
                    best = best.min(2 * (a + p.div_ceil(a)));
                }
                min_perimeter[p] = best;
            }
            let mut solver = Self {
                N,
                M,
                R_milli,
                board,
                occ: [-2; MAX_C],
                nb: [[-1; 4]; MAX_C],
                static_rank: [0; MAX_C],
                static_comp: [-1; MAX_C],
                static_clear: [0; MAX_C],
                separator_field: [0; MAX_C],
                static_comp_cells: Vec::new(),
                static_comp_size: Vec::new(),
                zobrist: [0; MAX_C],
                groups: vec![Group::default(); M],
                grass_count: 0,
                occupied_cells: 0,
                capacity_cells: 0.0,
                duration_y: 0.0,
                observed_n: 0,
                theta_estimate: 5_000.0,
                beta_estimate: 20.0,
                compact_prior: 0.74,
                compact_ewma: 0.74,
                compact_observed: 0,
                terrain_roughness: 0.0,
                mean_static_clearance: 1.0,
                operation_deadline: scaled_time(1.84),
                compact_specs: Vec::new(),
                min_perimeter,
                four_root,
                rng: XorShift64::new(1),
                start_time,
                grow_mark: [0; MAX_C],
                grow_stamp: 1,
                block_mark: [0; MAX_C],
                visit_mark: [0; MAX_C],
                local_index: [-1; MAX_C],
                eval_stamp: 1,
                #[cfg(feature = "local")]
                trace: TraceStats::default(),
            };
            solver.initialize_board();
            solver
        }

        #[inline]
        fn elapsed(&self) -> f64 {
            self.start_time.elapsed().as_secs_f64()
        }

        #[inline]
        fn clamp01(x: f64) -> f64 {
            x.clamp(0.0, 1.0)
        }

        fn minimum_polyomino_perimeter_any(p: usize) -> usize {
            let mut best = usize::MAX;
            let lim = (p as f64).sqrt() as usize + 2;
            for a in 1..=lim {
                best = best.min(2 * (a + p.div_ceil(a)));
            }
            best
        }

        fn pace_target(&self, turn: usize) -> f64 {
            scaled_time(1.80).min(
                scaled_time(0.10) + scaled_time(1.70) * (turn + 1) as f64 / self.M.max(1) as f64,
            )
        }

        fn adaptive_mode(&self, turn: usize, priority: f64) -> i32 {
            let elapsed = self.elapsed();
            if elapsed >= scaled_time(1.82) {
                return -1;
            }
            let borrow = scaled_time(0.020) * Self::clamp01((priority - 0.50) * 2.0);
            let slack = self.pace_target(turn) + borrow - elapsed;
            if slack > scaled_time(0.025) {
                2
            } else if slack > scaled_time(-0.010) {
                1
            } else if slack > scaled_time(-0.040) {
                0
            } else {
                -1
            }
        }

        fn set_operation_deadline(&mut self, turn: usize, urgency: f64, base_borrow: f64) {
            let premium = base_borrow * (0.35 + 0.65 * Self::clamp01(urgency));
            self.operation_deadline = scaled_time(1.84)
                .min((self.elapsed() + scaled_time(0.00015)).max(self.pace_target(turn) + premium));
        }

        #[inline]
        fn operation_time_exhausted(&self) -> bool {
            self.elapsed() >= self.operation_deadline || self.elapsed() >= scaled_time(1.84)
        }

        fn placement_priority(&self, group: &Group, q_req: f64) -> f64 {
            let duration = (group.T - group.S).max(1) as f64;
            let raw_q = group.V as f64 / (group.P as f64 * duration.powf(0.9));
            let ratio = raw_q / q_req.max(0.12);
            let mut premium = 0.50 + 0.24 * (1.35 * ratio.max(0.08).ln()).tanh();
            let size_need = Self::clamp01((group.P as f64 - 32.0) / 118.0);
            premium += 0.24 * self.terrain_roughness * size_need;
            let slack_L = group
                .worst_L
                .saturating_sub(self.minimum_polyomino_perimeter(group.P));
            if group.active && slack_L > 0 {
                premium -= 0.24_f64.min(0.025 * slack_L as f64);
            }
            premium.clamp(0.04, 0.96)
        }

        #[inline]
        fn id(&self, x: usize, y: usize) -> usize {
            x * self.N + y
        }

        #[inline]
        fn xof(&self, z: usize) -> usize {
            z / self.N
        }

        #[inline]
        fn yof(&self, z: usize) -> usize {
            z % self.N
        }

        #[inline]
        fn move_cost(&self, V: i64) -> i64 {
            1.max((V * self.R_milli + 500) / 1_000)
        }

        #[inline]
        fn fee(&self, V: i64, P: usize, L: usize) -> i64 {
            (V as f64 * (self.four_root[P] / L as f64) + 0.5).floor() as i64
        }

        fn region_hash(&self, cells: &[usize]) -> u64 {
            let mut hash = splitmix64(cells.len() as u64);
            for &z in cells {
                hash ^= self.zobrist[z];
            }
            hash
        }

        fn next_grow_stamp(&mut self) {
            self.grow_stamp += 1;
            if self.grow_stamp == i32::MAX {
                self.grow_mark.fill(0);
                self.grow_stamp = 1;
            }
        }

        fn next_eval_stamp(&mut self) {
            self.eval_stamp += 1;
            if self.eval_stamp == i32::MAX {
                self.block_mark.fill(0);
                self.visit_mark.fill(0);
                self.eval_stamp = 1;
            }
        }

        fn perimeter(&mut self, cells: &[usize]) -> usize {
            self.next_grow_stamp();
            let stamp = self.grow_stamp;
            for &z in cells {
                self.grow_mark[z] = stamp;
            }
            let mut L = 0;
            for &z in cells {
                for &w in &self.nb[z] {
                    if w < 0 || self.grow_mark[w as usize] != stamp {
                        L += 1;
                    }
                }
            }
            L
        }

        fn initialize_board(&mut self) {
            let mut seed = splitmix64(self.R_milli as u64 + 1_234_567);
            for x in 0..self.N {
                for y in 0..self.N {
                    let z = self.id(x, y);
                    if self.board[x].as_bytes()[y] == b'.' {
                        self.occ[z] = -1;
                        self.grass_count += 1;
                        seed ^= splitmix64(
                            (z as u64)
                                .wrapping_mul(911_382_323)
                                .wrapping_add(972_663_749),
                        );
                    } else {
                        self.occ[z] = -2;
                    }
                    let dirs = [(1_isize, 0_isize), (-1, 0), (0, 1), (0, -1)];
                    for (k, &(dx, dy)) in dirs.iter().enumerate() {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        self.nb[z][k] =
                            if 0 <= nx && nx < self.N as isize && 0 <= ny && ny < self.N as isize {
                                self.id(nx as usize, ny as usize) as isize
                            } else {
                                -1
                            };
                    }
                    self.zobrist[z] = splitmix64(
                        seed.wrapping_add((z as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
                    );
                }
            }
            self.rng = XorShift64::new(seed);
            self.build_static_components_and_rank();
            self.build_compact_specs();
        }

        #[inline]
        fn minimum_polyomino_perimeter(&self, P: usize) -> usize {
            self.min_perimeter[P]
        }

        fn build_compact_specs(&mut self) {
            self.compact_specs = vec![Vec::new(); MAX_P + 1];
            for P in 4..=MAX_P {
                let min_L = self.minimum_polyomino_perimeter(P);
                let mut used = HashSet::new();
                for a in 2..=25.min(P) {
                    let full = P / a;
                    let rem = P % a;
                    if full == 0 {
                        continue;
                    }
                    let bw = full + usize::from(rem > 0);
                    if !(2..=25).contains(&bw) {
                        continue;
                    }
                    let L = 2 * (a + bw);
                    if L > min_L + 4 || a.max(bw) * 10 > a.min(bw) * 28 {
                        continue;
                    }
                    let mut add = |off: usize, side: usize, rot: usize| {
                        let h = if rot != 0 { bw } else { a };
                        let w = if rot != 0 { a } else { bw };
                        let key = (a, full, rem, off, side, rot);
                        if h <= self.N && w <= self.N && used.insert(key) {
                            self.compact_specs[P].push(CompactSpec {
                                a,
                                full,
                                rem,
                                off,
                                side,
                                rot,
                                L,
                            });
                        }
                    };
                    if rem == 0 {
                        add(0, 0, 0);
                        if a != full {
                            add(0, 0, 1);
                        }
                    } else {
                        let mut offsets = vec![0, a - rem, (a - rem) / 2];
                        offsets.sort_unstable();
                        offsets.dedup();
                        for off in offsets {
                            for side in 0..2 {
                                for rot in 0..2 {
                                    add(off, side, rot);
                                }
                            }
                        }
                    }
                }
            }
        }

        fn spec_dims(spec: CompactSpec) -> (usize, usize) {
            let bw = spec.full + usize::from(spec.rem > 0);
            if spec.rot != 0 {
                (bw, spec.a)
            } else {
                (spec.a, bw)
            }
        }

        fn spec_is_free(&self, fs: &FreeState, spec: CompactSpec, x0: usize, y0: usize) -> bool {
            let (h, w) = Self::spec_dims(spec);
            if x0 + h > self.N || y0 + w > self.N {
                return false;
            }
            if spec.rem == 0 {
                return self.rect_count(fs, x0, y0, x0 + h, y0 + w) == spec.a * spec.full;
            }
            if spec.rot == 0 {
                let by = y0 + usize::from(spec.side == 0);
                if self.rect_count(fs, x0, by, x0 + spec.a, by + spec.full) != spec.a * spec.full {
                    return false;
                }
                let py = y0 + if spec.side != 0 { spec.full } else { 0 };
                self.rect_count(fs, x0 + spec.off, py, x0 + spec.off + spec.rem, py + 1) == spec.rem
            } else {
                let bx = x0 + usize::from(spec.side == 0);
                if self.rect_count(fs, bx, y0, bx + spec.full, y0 + spec.a) != spec.a * spec.full {
                    return false;
                }
                let px = x0 + if spec.side != 0 { spec.full } else { 0 };
                self.rect_count(fs, px, y0 + spec.off, px + 1, y0 + spec.off + spec.rem) == spec.rem
            }
        }

        fn spec_anchor(&self, spec: CompactSpec, x0: usize, y0: usize) -> usize {
            if spec.rem > 0 && spec.side == 0 {
                if spec.rot == 0 {
                    self.id(x0 + spec.off, y0)
                } else {
                    self.id(x0, y0 + spec.off)
                }
            } else {
                self.id(x0, y0)
            }
        }

        fn make_spec_region(&self, spec: CompactSpec, x0: usize, y0: usize) -> Region {
            let mut cells = Vec::with_capacity(spec.a * spec.full + spec.rem);
            if spec.rot == 0 {
                let by = y0 + usize::from(spec.rem > 0 && spec.side == 0);
                for x in x0..x0 + spec.a {
                    for y in by..by + spec.full {
                        cells.push(self.id(x, y));
                    }
                }
                if spec.rem > 0 {
                    let py = y0 + if spec.side != 0 { spec.full } else { 0 };
                    for x in x0 + spec.off..x0 + spec.off + spec.rem {
                        cells.push(self.id(x, py));
                    }
                }
            } else {
                let bx = x0 + usize::from(spec.rem > 0 && spec.side == 0);
                for x in bx..bx + spec.full {
                    for y in y0..y0 + spec.a {
                        cells.push(self.id(x, y));
                    }
                }
                if spec.rem > 0 {
                    let px = x0 + if spec.side != 0 { spec.full } else { 0 };
                    for y in y0 + spec.off..y0 + spec.off + spec.rem {
                        cells.push(self.id(px, y));
                    }
                }
            }
            Region {
                L: spec.L,
                hash: self.region_hash(&cells),
                cells,
                ..Region::default()
            }
        }

        fn add_compact_template_candidates(
            &mut self,
            P: usize,
            mode: i32,
            fs: &FreeState,
            candidates: &mut Vec<Region>,
            seen: &mut HashSet<u64>,
            priority: f64,
        ) {
            let min_L = self.minimum_polyomino_perimeter(P);
            let allowed = if mode >= 2 {
                4
            } else if mode == 1 {
                2
            } else {
                0
            };
            let hit_limit = if mode >= 2 {
                48
            } else if mode == 1 {
                28
            } else {
                14
            };
            let extra_limit = [
                if mode >= 2 {
                    12
                } else if mode == 1 {
                    8
                } else if mode == 0 {
                    4
                } else {
                    0
                },
                if mode >= 2 { 6 } else { 0 },
            ];
            let specs = self.compact_specs[P].clone();
            let mut best: BinaryHeap<(i64, usize, usize, usize)> = BinaryHeap::new();
            let mut extra: [BinaryHeap<(i64, usize, usize, usize)>; 2] =
                [BinaryHeap::new(), BinaryHeap::new()];
            for (si, &spec) in specs.iter().enumerate() {
                if spec.L > min_L + allowed {
                    continue;
                }
                let (h, w) = Self::spec_dims(spec);
                for x0 in 0..=self.N - h {
                    for y0 in 0..=self.N - w {
                        if !self.spec_is_free(fs, spec, x0, y0) {
                            continue;
                        }
                        let z = self.spec_anchor(spec, x0, y0);
                        let component = fs.cid[z];
                        if component < 0 || fs.size[component as usize] < P {
                            continue;
                        }
                        let bbox_free = self.rect_count(fs, x0, y0, x0 + h, y0 + w);
                        let ex0 = x0.saturating_sub(1);
                        let ey0 = y0.saturating_sub(1);
                        let ex1 = self.N.min(x0 + h + 1);
                        let ey1 = self.N.min(y0 + w + 1);
                        let ring_free = self.rect_count(fs, ex0, ey0, ex1, ey1) - bbox_free;
                        let missing_free = bbox_free - P;
                        let leftover = fs.size[component as usize] - P;
                        let style = 0.50 - priority;
                        let clear_hint =
                            (self.terrain_roughness * style * 520.0 * self.static_clear[z] as f64)
                                .round() as i64;
                        let reserve_hint = (self.terrain_roughness
                            * (4.5 - 1.5 * priority)
                            * self.separator_field[z] as f64)
                            .round() as i64;
                        let base_score = 15_000 * missing_free as i64
                            + 300 * ring_free as i64
                            + 6 * leftover.min(2_000) as i64
                            + self.static_rank[z] as i64
                            + clear_hint
                            + reserve_hint
                            + (self.zobrist[z] & 127) as i64;
                        let score = 5_000_000 * (spec.L - min_L) as i64 + base_score;
                        let hit = (score, x0, y0, si);
                        if best.len() < hit_limit {
                            best.push(hit);
                        } else if score < best.peek().unwrap().0 {
                            best.pop();
                            best.push(hit);
                        }
                        let cls = (spec.L - min_L) / 2;
                        if cls >= 1 && cls <= 2 && extra_limit[cls - 1] > 0 {
                            let item = (base_score, x0, y0, si);
                            let heap = &mut extra[cls - 1];
                            if heap.len() < extra_limit[cls - 1] {
                                heap.push(item);
                            } else if base_score < heap.peek().unwrap().0 {
                                heap.pop();
                                heap.push(item);
                            }
                        }
                    }
                }
            }
            let mut hits = Vec::with_capacity(best.len() + extra[0].len() + extra[1].len());
            hits.extend(best.into_iter());
            hits.extend(extra[0].drain());
            hits.extend(extra[1].drain());
            hits.sort_by(|a, b| specs[a.3].L.cmp(&specs[b.3].L).then(a.0.cmp(&b.0)));
            for (_, x0, y0, si) in hits {
                let region = self.make_spec_region(specs[si], x0, y0);
                self.add_candidate(candidates, seen, region, P);
            }
        }

        fn build_static_components_and_rank(&mut self) {
            let mut seen = [false; MAX_C];
            let mut queue = [0_usize; MAX_C];
            for z in 0..self.N * self.N {
                if self.occ[z] != -1 || seen[z] {
                    continue;
                }
                let component = self.static_comp_cells.len();
                self.static_comp_cells.push(Vec::new());
                let mut head = 0;
                let mut tail = 0;
                queue[tail] = z;
                tail += 1;
                seen[z] = true;
                while head < tail {
                    let v = queue[head];
                    head += 1;
                    self.static_comp[v] = component as isize;
                    self.static_comp_cells[component].push(v);
                    for &w in &self.nb[v] {
                        if w >= 0 {
                            let w = w as usize;
                            if self.occ[w] == -1 && !seen[w] {
                                seen[w] = true;
                                queue[tail] = w;
                                tail += 1;
                            }
                        }
                    }
                }
            }

            self.static_comp_size
                .resize(self.static_comp_cells.len(), 0);
            let mut max_comp = 0;
            let mut usable = 0_usize;
            let mut dist = [-1_i32; MAX_C];
            for component in 0..self.static_comp_cells.len() {
                let cells = &self.static_comp_cells[component];
                self.static_comp_size[component] = cells.len();
                max_comp = max_comp.max(cells.len());
                if cells.len() >= 4 {
                    usable += cells.len() - 3;
                }
                let mut root = cells[0];
                for &z in cells {
                    let root_key = self.xof(root) + self.yof(root);
                    let z_key = self.xof(z) + self.yof(z);
                    if z_key < root_key || (z_key == root_key && z < root) {
                        root = z;
                    }
                    dist[z] = -1;
                }
                let mut head = 0;
                let mut tail = 0;
                queue[tail] = root;
                tail += 1;
                dist[root] = 0;
                while head < tail {
                    let v = queue[head];
                    head += 1;
                    let x = self.xof(v);
                    let y = self.yof(v);
                    let tie = if dist[v] & 1 != 0 { self.N - 1 - y } else { y };
                    self.static_rank[v] = dist[v] * 64 + tie as i32 + (x & 3) as i32;
                    for &w in &self.nb[v] {
                        if w >= 0 {
                            let w = w as usize;
                            if self.static_comp[w] == component as isize && dist[w] < 0 {
                                dist[w] = dist[v] + 1;
                                queue[tail] = w;
                                tail += 1;
                            }
                        }
                    }
                }
            }

            let mut clear_queue = [0_usize; MAX_C];
            let mut clear_head = 0;
            let mut clear_tail = 0;
            for z in 0..self.N * self.N {
                if self.occ[z] != -1 {
                    continue;
                }
                let boundary = self.nb[z]
                    .iter()
                    .any(|&w| w < 0 || self.occ[w as usize] == -2);
                if boundary {
                    self.static_clear[z] = 1;
                    clear_queue[clear_tail] = z;
                    clear_tail += 1;
                }
            }
            while clear_head < clear_tail {
                let v = clear_queue[clear_head];
                clear_head += 1;
                for &w in &self.nb[v] {
                    if w >= 0 {
                        let w = w as usize;
                        if self.occ[w] == -1 && self.static_clear[w] == 0 {
                            self.static_clear[w] = self.static_clear[v].saturating_add(1);
                            clear_queue[clear_tail] = w;
                            clear_tail += 1;
                        }
                    }
                }
            }
            let mut clear_sum = 0_usize;
            for z in 0..self.N * self.N {
                if self.occ[z] == -1 {
                    if self.static_clear[z] == 0 {
                        self.static_clear[z] = 1;
                    }
                    clear_sum += self.static_clear[z] as usize;
                }
            }
            self.mean_static_clearance = if self.grass_count > 0 {
                clear_sum as f64 / self.grass_count as f64
            } else {
                1.0
            };

            fn dfs_separator(
                u: usize,
                parent: isize,
                nb: &[[isize; 4]; MAX_C],
                static_comp: &[isize; MAX_C],
                disc: &mut [i32; MAX_C],
                low: &mut [i32; MAX_C],
                sub: &mut [i32; MAX_C],
                sep_sum: &mut [i32; MAX_C],
                sep_max: &mut [i32; MAX_C],
                timer: &mut i32,
            ) {
                disc[u] = *timer;
                low[u] = *timer;
                *timer += 1;
                sub[u] = 1;
                for &v in &nb[u] {
                    if v < 0 || static_comp[v as usize] != static_comp[u] {
                        continue;
                    }
                    let v = v as usize;
                    if disc[v] < 0 {
                        dfs_separator(
                            v,
                            u as isize,
                            nb,
                            static_comp,
                            disc,
                            low,
                            sub,
                            sep_sum,
                            sep_max,
                            timer,
                        );
                        sub[u] += sub[v];
                        low[u] = low[u].min(low[v]);
                        if parent < 0 || low[v] >= disc[u] {
                            sep_sum[u] += sub[v];
                            sep_max[u] = sep_max[u].max(sub[v]);
                        }
                    } else if v as isize != parent {
                        low[u] = low[u].min(disc[v]);
                    }
                }
            }

            let mut disc = [-1_i32; MAX_C];
            let mut low = [0_i32; MAX_C];
            let mut sub = [0_i32; MAX_C];
            let mut sep_sum = [0_i32; MAX_C];
            let mut sep_max = [0_i32; MAX_C];
            let mut dfs_timer = 0;
            for component in 0..self.static_comp_cells.len() {
                let cells = &self.static_comp_cells[component];
                if cells.is_empty() {
                    continue;
                }
                dfs_separator(
                    cells[0],
                    -1,
                    &self.nb,
                    &self.static_comp,
                    &mut disc,
                    &mut low,
                    &mut sub,
                    &mut sep_sum,
                    &mut sep_max,
                    &mut dfs_timer,
                );
                let size = cells.len() as i32;
                for &z in cells {
                    let rest = 0.max(size - 1 - sep_sum[z]);
                    let largest_part = rest.max(sep_max[z]);
                    let separated_mass = 0.max(size - 1 - largest_part);
                    let mut importance = 255.min((separated_mass * 255 + 149) / 150);
                    let degree = self.nb[z]
                        .iter()
                        .filter(|&&w| w >= 0 && self.occ[w as usize] == -1)
                        .count();
                    if self.static_clear[z] <= 1 && degree <= 2 {
                        importance = importance.max(72);
                    }
                    self.separator_field[z] = importance as u8;
                }
            }
            for _ in 0..2 {
                let old = self.separator_field;
                for z in 0..self.N * self.N {
                    if self.occ[z] != -1 {
                        continue;
                    }
                    let mut best = old[z] as i32;
                    for &w in &self.nb[z] {
                        if w >= 0 && self.occ[w as usize] == -1 {
                            best = best.max(0.max(old[w as usize] as i32 - 70));
                        }
                    }
                    self.separator_field[z] = best as u8;
                }
            }

            let main_fraction = if self.grass_count > 0 {
                max_comp as f64 / self.grass_count as f64
            } else {
                0.0
            };
            let factor = 0.91 + 0.04 * main_fraction;
            self.capacity_cells = 1.0_f64.max(factor * usable as f64);
            let mut adjacency = 0_usize;
            for z in 0..self.N * self.N {
                if self.occ[z] != -1 {
                    continue;
                }
                let x = self.xof(z);
                let y = self.yof(z);
                if x + 1 < self.N && self.occ[self.id(x + 1, y)] == -1 {
                    adjacency += 1;
                }
                if y + 1 < self.N && self.occ[self.id(x, y + 1)] == -1 {
                    adjacency += 1;
                }
            }
            let average_degree = if self.grass_count > 0 {
                2.0 * adjacency as f64 / self.grass_count as f64
            } else {
                0.0
            };
            let boundary_per_grass = (4.0 - average_degree).max(0.0);
            let rough_boundary = Self::clamp01((boundary_per_grass - 0.16) / 0.78);
            let rough_clear = Self::clamp01((3.8 - self.mean_static_clearance) / 2.5);
            let rough_topology = Self::clamp01((0.985 - main_fraction) / 0.30);
            self.terrain_roughness =
                Self::clamp01(0.55 * rough_boundary + 0.30 * rough_clear + 0.15 * rough_topology);
            self.capacity_cells *= 1.0 - 0.045 * self.terrain_roughness;
            self.compact_prior = (0.866 - 0.277 * boundary_per_grass).clamp(0.52, 0.90);
            self.compact_ewma = self.compact_prior;
        }

        fn build_free_state(&self) -> FreeState {
            let mut fs = FreeState::new();
            let mut queue = [0_usize; MAX_C];
            for z in 0..self.N * self.N {
                if self.occ[z] != -1 || fs.cid[z] >= 0 {
                    continue;
                }
                let component = fs.cells.len();
                fs.cells.push(Vec::new());
                let mut head = 0;
                let mut tail = 0;
                queue[tail] = z;
                tail += 1;
                fs.cid[z] = component as isize;
                while head < tail {
                    let v = queue[head];
                    head += 1;
                    fs.cells[component].push(v);
                    fs.free_count += 1;
                    for &w in &self.nb[v] {
                        if w >= 0 {
                            let w = w as usize;
                            if self.occ[w] == -1 && fs.cid[w] < 0 {
                                fs.cid[w] = component as isize;
                                queue[tail] = w;
                                tail += 1;
                            }
                        }
                    }
                }
            }
            fs.size = fs.cells.iter().map(Vec::len).collect();
            for x in 0..self.N {
                for y in 0..self.N {
                    let here = i32::from(self.occ[self.id(x, y)] == -1);
                    let horizontal =
                        i32::from(here != 0 && y + 1 < self.N && self.occ[self.id(x, y + 1)] == -1);
                    let vertical =
                        i32::from(here != 0 && x + 1 < self.N && self.occ[self.id(x + 1, y)] == -1);
                    fs.pref[x + 1][y + 1] =
                        fs.pref[x][y + 1] + fs.pref[x + 1][y] - fs.pref[x][y] + here;
                    fs.hpref[x + 1][y + 1] =
                        fs.hpref[x][y + 1] + fs.hpref[x + 1][y] - fs.hpref[x][y] + horizontal;
                    fs.vpref[x + 1][y + 1] =
                        fs.vpref[x][y + 1] + fs.vpref[x + 1][y] - fs.vpref[x][y] + vertical;
                }
            }
            fs
        }

        #[inline]
        fn pref_rect(
            pref: &[[i32; MAX_N + 1]; MAX_N + 1],
            x0: usize,
            y0: usize,
            x1: usize,
            y1: usize,
        ) -> usize {
            if x0 >= x1 || y0 >= y1 {
                return 0;
            }
            (pref[x1][y1] - pref[x0][y1] - pref[x1][y0] + pref[x0][y0]) as usize
        }

        #[inline]
        fn rect_count(&self, fs: &FreeState, x0: usize, y0: usize, x1: usize, y1: usize) -> usize {
            Self::pref_rect(&fs.pref, x0, y0, x1, y1)
        }

        fn rect_free_edges(
            &self,
            fs: &FreeState,
            x0: usize,
            y0: usize,
            x1: usize,
            y1: usize,
        ) -> usize {
            Self::pref_rect(&fs.hpref, x0, y0, x1, y1 - 1)
                + Self::pref_rect(&fs.vpref, x0, y0, x1 - 1, y1)
        }

        fn selected_degree(&self, z: usize) -> i32 {
            self.nb[z]
                .iter()
                .filter(|&&w| w >= 0 && self.grow_mark[w as usize] == self.grow_stamp)
                .count() as i32
        }

        fn growth_key(
            &self,
            z: usize,
            center_x2: i32,
            center_y2: i32,
            target_T: Option<usize>,
            priority: f64,
        ) -> i32 {
            let dx = 2 * self.xof(z) as i32 - center_x2;
            let dy = 2 * self.yof(z) as i32 - center_y2;
            let distance2 = dx * dx + dy * dy;
            let degree = self.selected_degree(z);
            let mut static_hard = 0;
            let mut occupied_support = 0;
            let mut temporal = 0_i32;
            for &w in &self.nb[z] {
                if w >= 0 && self.grow_mark[w as usize] == self.grow_stamp {
                    continue;
                }
                if w < 0 || self.occ[w as usize] == -2 {
                    static_hard += 1;
                } else if self.occ[w as usize] != -1 {
                    occupied_support += 1;
                    if let Some(target_T) = target_T {
                        let owner = self.occ[w as usize];
                        if owner >= 0 {
                            temporal +=
                                30_000.min(target_T.abs_diff(self.groups[owner as usize].T)) as i32;
                        }
                    }
                }
            }
            let distance_weight = 14 + (5.0 * priority).round() as i32;
            let degree_weight = 870 + (130.0 * priority).round() as i32;
            let static_hard_coefficient = (-82.0 + 154.0 * priority).round() as i32;
            let clear_coefficient =
                (60.0 * self.terrain_roughness * (0.50 - priority)).round() as i32;
            let separator_coefficient =
                (2.4 * self.terrain_roughness * (1.0 - priority) * (1.0 - priority)).round() as i32;
            let noise = (self.zobrist[z] & 31) as i32;
            distance2 * distance_weight - degree * degree_weight
                + static_hard_coefficient * static_hard
                - occupied_support * 72
                + temporal / 220
                + clear_coefficient * self.static_clear[z] as i32
                + separator_coefficient * self.separator_field[z] as i32
                + (self.static_rank[z] & 127)
                + noise
        }

        #[allow(clippy::too_many_arguments)]
        fn grow_region(
            &mut self,
            P: usize,
            seed: usize,
            center_x2: i32,
            center_y2: i32,
            x0: usize,
            y0: usize,
            x1: usize,
            y1: usize,
            target_T: Option<usize>,
            priority: f64,
        ) -> Option<Region> {
            if self.occ[seed] != -1 {
                return None;
            }
            self.next_grow_stamp();
            let stamp = self.grow_stamp;
            let in_box = |z: usize, N: usize| {
                let x = z / N;
                let y = z % N;
                x0 <= x && x < x1 && y0 <= y && y < y1
            };
            let mut heap: BinaryHeap<Reverse<HeapNode>> = BinaryHeap::new();
            let mut region = Region {
                cells: Vec::with_capacity(P),
                L: 4,
                ..Region::default()
            };
            self.grow_mark[seed] = stamp;
            region.cells.push(seed);
            for &w in &self.nb[seed] {
                if w >= 0 {
                    let w = w as usize;
                    if self.occ[w] == -1 && self.grow_mark[w] != stamp && in_box(w, self.N) {
                        heap.push(Reverse(HeapNode {
                            key: self.growth_key(w, center_x2, center_y2, target_T, priority),
                            z: w,
                        }));
                    }
                }
            }
            let mut perimeter = 4_i32;
            while region.cells.len() < P {
                let Reverse(node) = heap.pop()?;
                let z = node.z;
                if self.occ[z] != -1 || self.grow_mark[z] == stamp || !in_box(z, self.N) {
                    continue;
                }
                let key_now = self.growth_key(z, center_x2, center_y2, target_T, priority);
                if key_now != node.key {
                    heap.push(Reverse(HeapNode { key: key_now, z }));
                    continue;
                }
                let degree = self.selected_degree(z);
                self.grow_mark[z] = stamp;
                region.cells.push(z);
                perimeter += 4 - 2 * degree;
                for &w in &self.nb[z] {
                    if w >= 0 {
                        let w = w as usize;
                        if self.occ[w] == -1 && self.grow_mark[w] != stamp && in_box(w, self.N) {
                            heap.push(Reverse(HeapNode {
                                key: self.growth_key(w, center_x2, center_y2, target_T, priority),
                                z: w,
                            }));
                        }
                    }
                }
            }
            region.L = perimeter as usize;
            region.hash = self.region_hash(&region.cells);
            Some(region)
        }

        fn add_candidate(
            &self,
            output: &mut Vec<Region>,
            seen: &mut HashSet<u64>,
            mut region: Region,
            P: usize,
        ) {
            if region.cells.len() != P {
                return;
            }
            if region.hash == 0 {
                region.hash = self.region_hash(&region.cells);
            }
            if seen.insert(region.hash) {
                output.push(region);
            }
        }

        fn packing_cell_score(
            &self,
            z: usize,
            removed: Option<usize>,
            target_T: Option<usize>,
            priority: f64,
        ) -> i64 {
            let mut score = (self.static_rank[z] / 4) as i64;
            let mut static_hard = 0;
            let mut occupied = 0;
            let mut temporal = 0_usize;
            for &v in &self.nb[z] {
                if v >= 0 {
                    let v = v as usize;
                    if self.local_index[v] >= 0 && Some(v) != removed {
                        continue;
                    }
                }
                if v < 0 || self.occ[v as usize] == -2 {
                    static_hard += 1;
                } else if self.occ[v as usize] != -1 {
                    occupied += 1;
                    if let Some(target_T) = target_T {
                        let owner = self.occ[v as usize];
                        if owner >= 0 {
                            temporal +=
                                30_000.min(target_T.abs_diff(self.groups[owner as usize].T));
                        }
                    }
                }
            }
            let style = (0.50 - priority) * 2.0;
            score += (self.terrain_roughness * style * 34.0 * self.static_clear[z] as f64).round()
                as i64;
            score +=
                (self.terrain_roughness * (1.0 - priority) * 2.0 * self.separator_field[z] as f64)
                    .round() as i64;
            score += ((-62.0 + 116.0 * priority) * static_hard as f64).round() as i64;
            score -= 72 * occupied;
            score + (temporal / 220) as i64
        }

        fn polish_region(
            &mut self,
            region: &mut Region,
            max_iterations: usize,
            target_T: Option<usize>,
            priority: f64,
        ) {
            let P = region.cells.len();
            if P <= 2 || max_iterations == 0 {
                return;
            }
            for _ in 0..max_iterations {
                for (index, &z) in region.cells.iter().enumerate() {
                    self.local_index[z] = index as isize;
                }
                let mut disc = vec![-1_i32; P];
                let mut low = vec![0_i32; P];
                let mut parent = vec![-1_isize; P];
                let mut degree = vec![0_i32; P];
                let mut articulation = vec![false; P];

                fn dfs(
                    u: usize,
                    cells: &[usize],
                    nb: &[[isize; 4]; MAX_C],
                    local_index: &[isize; MAX_C],
                    disc: &mut [i32],
                    low: &mut [i32],
                    parent: &mut [isize],
                    degree: &mut [i32],
                    articulation: &mut [bool],
                    timer: &mut i32,
                ) {
                    disc[u] = *timer;
                    low[u] = *timer;
                    *timer += 1;
                    let mut children = 0;
                    let z = cells[u];
                    for &w in &nb[z] {
                        let v = if w >= 0 { local_index[w as usize] } else { -1 };
                        if v < 0 {
                            continue;
                        }
                        let v = v as usize;
                        degree[u] += 1;
                        if disc[v] < 0 {
                            parent[v] = u as isize;
                            children += 1;
                            dfs(
                                v,
                                cells,
                                nb,
                                local_index,
                                disc,
                                low,
                                parent,
                                degree,
                                articulation,
                                timer,
                            );
                            low[u] = low[u].min(low[v]);
                            if if parent[u] < 0 {
                                children > 1
                            } else {
                                low[v] >= disc[u]
                            } {
                                articulation[u] = true;
                            }
                        } else if v as isize != parent[u] {
                            low[u] = low[u].min(disc[v]);
                        }
                    }
                }

                let mut timer = 0;
                dfs(
                    0,
                    &region.cells,
                    &self.nb,
                    &self.local_index,
                    &mut disc,
                    &mut low,
                    &mut parent,
                    &mut degree,
                    &mut articulation,
                    &mut timer,
                );
                self.next_eval_stamp();
                let stamp = self.eval_stamp;
                let mut additions = Vec::new();
                for &z in &region.cells {
                    for &w in &self.nb[z] {
                        if w < 0 {
                            continue;
                        }
                        let w = w as usize;
                        if self.occ[w] == -1
                            && self.local_index[w] < 0
                            && self.block_mark[w] != stamp
                        {
                            self.block_mark[w] = stamp;
                            additions.push(w);
                        }
                    }
                }
                let mut best_gain = 0_i32;
                let mut best_packing_gain = i64::MIN;
                let mut best_remove = None;
                let mut best_add = 0;
                for &w in &additions {
                    let degree_add = self.nb[w]
                        .iter()
                        .filter(|&&v| v >= 0 && self.local_index[v as usize] >= 0)
                        .count() as i32;
                    if degree_add <= 1 {
                        continue;
                    }
                    for u in 0..P {
                        if articulation[u] {
                            continue;
                        }
                        let z = region.cells[u];
                        let mut adjusted_degree = degree_add;
                        if self.nb[w].iter().any(|&v| v == z as isize) {
                            adjusted_degree -= 1;
                        }
                        if adjusted_degree <= 0 {
                            continue;
                        }
                        let gain = adjusted_degree - degree[u];
                        if gain < 0 {
                            continue;
                        }
                        let packing_gain = self.packing_cell_score(z, None, target_T, priority)
                            - self.packing_cell_score(w, Some(z), target_T, priority);
                        if gain > best_gain
                            || (gain == best_gain && packing_gain > best_packing_gain)
                        {
                            best_gain = gain;
                            best_packing_gain = packing_gain;
                            best_remove = Some(u);
                            best_add = w;
                        }
                    }
                }
                for &z in &region.cells {
                    self.local_index[z] = -1;
                }
                let Some(best_remove) = best_remove else {
                    break;
                };
                if best_gain == 0 && best_packing_gain <= 80 {
                    break;
                }
                region.cells[best_remove] = best_add;
                region.L = (region.L as i32 - 2 * best_gain) as usize;
            }
            region.hash = self.region_hash(&region.cells);
        }

        fn evaluate_layout(
            &mut self,
            region: &mut Region,
            fs: &FreeState,
            P: usize,
            target_T: Option<usize>,
            priority: f64,
        ) -> i64 {
            let component = fs.cid[region.cells[0]];
            region.comp = component;
            if component < 0 {
                return 4_000_000_000_000_000_000_i64;
            }
            self.next_eval_stamp();
            let stamp = self.eval_stamp;
            for &z in &region.cells {
                self.block_mark[z] = stamp;
            }
            let component = component as usize;
            let mut sum_square = 0_i64;
            let mut tiny = 0_usize;
            let mut queue = [0_usize; MAX_C];
            for &start in &fs.cells[component] {
                if self.block_mark[start] == stamp || self.visit_mark[start] == stamp {
                    continue;
                }
                let mut head = 0;
                let mut tail = 0;
                queue[tail] = start;
                tail += 1;
                self.visit_mark[start] = stamp;
                while head < tail {
                    let v = queue[head];
                    head += 1;
                    for &w in &self.nb[v] {
                        if w >= 0 {
                            let w = w as usize;
                            if fs.cid[w] == component as isize
                                && self.block_mark[w] != stamp
                                && self.visit_mark[w] != stamp
                            {
                                self.visit_mark[w] = stamp;
                                queue[tail] = w;
                                tail += 1;
                            }
                        }
                    }
                }
                sum_square += (tail * tail) as i64;
                if tail < 4 {
                    tiny += tail;
                }
            }
            let size = fs.size[component] as i64;
            let remain = size - P as i64;
            let split_loss = remain * remain - sum_square;
            let mut rank_sum = 0_i64;
            let mut flexibility_sum = 0_i64;
            let mut separator_sum = 0_i64;
            let mut cut_edges = 0_i64;
            let mut support_edges = 0_i64;
            let mut temporal_mismatch = 0_usize;
            for &z in &region.cells {
                rank_sum += self.static_rank[z] as i64;
                flexibility_sum += self.static_clear[z] as i64;
                separator_sum += self.separator_field[z] as i64;
                for &w in &self.nb[z] {
                    if w >= 0 && self.block_mark[w as usize] == stamp {
                        continue;
                    }
                    if w >= 0 && self.occ[w as usize] == -1 {
                        cut_edges += 1;
                    } else {
                        support_edges += 1;
                        if let Some(target_T) = target_T {
                            if w >= 0 {
                                let owner = self.occ[w as usize];
                                if owner >= 0 {
                                    temporal_mismatch += 30_000
                                        .min(target_T.abs_diff(self.groups[owner as usize].T));
                                }
                            }
                        }
                    }
                }
            }
            let boundary_delta = cut_edges - support_edges;
            let pressure = (self.occupied_cells as f64 / self.capacity_cells.max(1.0)).min(1.0);
            let reserve = self.terrain_roughness * (1.0 - priority) * (0.55 + 0.45 * pressure);
            let flexibility_term = (10.0
                * self.terrain_roughness
                * (0.50 - priority)
                * flexibility_sum as f64)
                .round() as i64;
            let separator_term =
                (75.0 * reserve * separator_sum as f64 / P.max(1) as f64).round() as i64;
            let boundary_coefficient =
                (60.0 + 55.0 * self.terrain_roughness * (1.0 - priority)).round() as i64;
            region.layout = split_loss / P.max(1) as i64
                + 20_000 * tiny as i64
                + 4 * remain.min(2_000)
                + boundary_coefficient * boundary_delta
                + (temporal_mismatch / 300) as i64
                + rank_sum / (16 * P).max(1) as i64
                + flexibility_term
                + separator_term;
            region.layout
        }

        fn dimensions_for(
            &self,
            P: usize,
            mode: i32,
            priority: f64,
            peel: bool,
        ) -> Vec<(usize, usize)> {
            let mut used = [[false; 26]; 26];
            let mut dimensions = Vec::new();
            let square = (P as f64).sqrt() as usize;
            let old_extra = 36.max(P / 2);
            let rough_extra = (self.terrain_roughness * (12.0 + 0.30 * P as f64)).round() as usize;
            let mut max_extra = 180.min(old_extra + if mode >= 1 { rough_extra } else { 0 });
            if peel {
                max_extra = max_extra.min(42.max(P / 2 + 24));
            }
            {
                let mut add = |h: usize, w: usize| {
                    if h < 2
                        || w < 2
                        || h > 25
                        || w > 25
                        || h * w < P
                        || h * w > P + max_extra
                        || h.max(w) * 10 > h.min(w) * 28
                    {
                        return;
                    }
                    if !used[h][w] {
                        used[h][w] = true;
                        dimensions.push((h, w));
                    }
                };
                for h in square.saturating_sub(5).max(2)..=22.min(square + 7) {
                    let w0 = P.div_ceil(h);
                    for extra in 0..=2 {
                        add(h, w0 + extra);
                        add(w0 + extra, h);
                    }
                }
                if mode >= 1 && self.terrain_roughness > 0.12 {
                    let levels = if peel {
                        2
                    } else if mode >= 2 {
                        3
                    } else {
                        2
                    };
                    for k in 1..=levels {
                        let fraction = k as f64 / levels as f64;
                        let target = P + (max_extra as f64 * fraction).round() as usize;
                        let root = (target as f64).sqrt() as isize;
                        for dh in -2_isize..=2 {
                            let h = (root + dh).max(2) as usize;
                            let w = target.div_ceil(h);
                            add(h, w);
                            add(w, h);
                        }
                    }
                }
                if mode >= 2 && priority > 0.72 && P >= 55 {
                    let target = (P + max_extra).min(P + 20.max(P / 3));
                    let root = ((target as f64).sqrt() as usize).max(2);
                    for dh in -2_isize..=2 {
                        let h = (root as isize + dh).max(2) as usize;
                        let w = target.div_ceil(h);
                        add(h, w);
                        add(w, h);
                    }
                }
            }
            if dimensions.is_empty() {
                let h = square.max(2);
                let w = P.div_ceil(h);
                if h <= 25
                    && w <= 25
                    && h * w >= P
                    && h * w <= P + max_extra
                    && h.max(w) * 10 <= h.min(w) * 28
                {
                    dimensions.push((h, w));
                    if h != w {
                        dimensions.push((w, h));
                    }
                }
            }
            dimensions
        }

        #[allow(clippy::too_many_arguments)]
        fn peel_box_region(
            &mut self,
            P: usize,
            x0: usize,
            y0: usize,
            h: usize,
            w: usize,
            target_T: Option<usize>,
            priority: f64,
        ) -> Option<Region> {
            self.next_eval_stamp();
            let component_stamp = self.eval_stamp;
            let mut best_component = Vec::new();
            let mut queue = [0_usize; MAX_C];
            for x in x0..x0 + h {
                for y in y0..y0 + w {
                    let start = self.id(x, y);
                    if self.occ[start] != -1 || self.visit_mark[start] == component_stamp {
                        continue;
                    }
                    let mut component = Vec::new();
                    let mut head = 0;
                    let mut tail = 0;
                    queue[tail] = start;
                    tail += 1;
                    self.visit_mark[start] = component_stamp;
                    while head < tail {
                        let z = queue[head];
                        head += 1;
                        component.push(z);
                        for &v in &self.nb[z] {
                            if v < 0 {
                                continue;
                            }
                            let v = v as usize;
                            if self.occ[v] != -1 || self.visit_mark[v] == component_stamp {
                                continue;
                            }
                            let vx = self.xof(v);
                            let vy = self.yof(v);
                            if x0 <= vx && vx < x0 + h && y0 <= vy && vy < y0 + w {
                                self.visit_mark[v] = component_stamp;
                                queue[tail] = v;
                                tail += 1;
                            }
                        }
                    }
                    if component.len() > best_component.len() {
                        best_component = component;
                    }
                }
            }
            if best_component.len() < P {
                return None;
            }
            let full_size = best_component.len();
            let mut alive = vec![true; full_size];
            let mut seen = vec![0_i32; full_size];
            let mut bfs_queue = vec![0_usize; full_size];
            for (index, &z) in best_component.iter().enumerate() {
                self.local_index[z] = index as isize;
            }
            let mut alive_count = full_size;
            let mut seen_stamp = 0;
            let mut current_L = self.perimeter(&best_component) as i32;
            while alive_count > P {
                let mut sum_x = 0_i64;
                let mut sum_y = 0_i64;
                for (index, &z) in best_component.iter().enumerate() {
                    if alive[index] {
                        sum_x += self.xof(z) as i64;
                        sum_y += self.yof(z) as i64;
                    }
                }
                let mut candidates: Vec<(i64, usize, i32)> = Vec::with_capacity(alive_count);
                for (index, &z) in best_component.iter().enumerate() {
                    if !alive[index] {
                        continue;
                    }
                    let mut degree = 0;
                    let mut hard_support = 0;
                    let mut temporal_support = 0_i64;
                    for &v in &self.nb[z] {
                        let local = if v >= 0 {
                            self.local_index[v as usize]
                        } else {
                            -1
                        };
                        if local >= 0 && alive[local as usize] {
                            degree += 1;
                        } else if v < 0 || self.occ[v as usize] != -1 {
                            hard_support += 1;
                            if let Some(target_T) = target_T {
                                if v >= 0 {
                                    let owner = self.occ[v as usize];
                                    if owner >= 0 {
                                        temporal_support += (30_000
                                            - 30_000.min(
                                                target_T.abs_diff(self.groups[owner as usize].T),
                                            ))
                                            as i64;
                                    }
                                }
                            }
                        }
                    }
                    if degree == 4 {
                        continue;
                    }
                    let dx = alive_count as i64 * self.xof(z) as i64 - sum_x;
                    let dy = alive_count as i64 * self.yof(z) as i64 - sum_y;
                    let distance2 = dx * dx + dy * dy;
                    let hard_coefficient = (180_000.0 * (0.55 - priority)).round() as i64;
                    let score = (2 * degree - 4) as i64 * 1_000_000_000_000
                        - distance2 * 16
                        - self.static_rank[z] as i64 * 48
                        + hard_support as i64 * hard_coefficient
                        + temporal_support * 2
                        + (self.terrain_roughness
                            * (priority - 0.50)
                            * 1_200.0
                            * self.static_clear[z] as f64)
                            .round() as i64
                        - (self.terrain_roughness
                            * (2.0 - priority)
                            * 600.0
                            * self.separator_field[z] as f64)
                            .round() as i64;
                    candidates.push((score, index, degree));
                }
                candidates.sort_unstable_by_key(|&(score, index, _)| (score, index));
                let mut removed = false;
                for &(_, remove, degree) in &candidates {
                    alive[remove] = false;
                    let mut connected = true;
                    if degree >= 2 {
                        let root = alive.iter().position(|&value| value);
                        seen_stamp += 1;
                        let mut head = 0;
                        let mut tail = 0;
                        let mut got = 0;
                        if let Some(root) = root {
                            seen[root] = seen_stamp;
                            bfs_queue[tail] = root;
                            tail += 1;
                        }
                        while head < tail {
                            let u = bfs_queue[head];
                            head += 1;
                            got += 1;
                            let z = best_component[u];
                            for &v in &self.nb[z] {
                                let local = if v >= 0 {
                                    self.local_index[v as usize]
                                } else {
                                    -1
                                };
                                if local >= 0 {
                                    let local = local as usize;
                                    if alive[local] && seen[local] != seen_stamp {
                                        seen[local] = seen_stamp;
                                        bfs_queue[tail] = local;
                                        tail += 1;
                                    }
                                }
                            }
                        }
                        connected = got == alive_count - 1;
                    }
                    if connected {
                        current_L += 2 * degree - 4;
                        alive_count -= 1;
                        removed = true;
                        break;
                    }
                    alive[remove] = true;
                }
                if !removed {
                    for &z in &best_component {
                        self.local_index[z] = -1;
                    }
                    return None;
                }
            }
            let cells: Vec<usize> = best_component
                .iter()
                .enumerate()
                .filter_map(|(index, &z)| alive[index].then_some(z))
                .collect();
            for &z in &best_component {
                self.local_index[z] = -1;
            }
            if cells.len() != P {
                return None;
            }
            Some(Region {
                L: current_L as usize,
                hash: self.region_hash(&cells),
                cells,
                ..Region::default()
            })
        }

        fn find_peel_regions(
            &mut self,
            P: usize,
            max_results: usize,
            box_limit: usize,
            target_T: Option<usize>,
            priority: f64,
        ) -> Vec<Region> {
            let fs = self.build_free_state();
            if fs.free_count < P || !fs.size.iter().any(|&size| size >= P) {
                return Vec::new();
            }
            let dimension_mode = if box_limit >= 12 { 2 } else { 1 };
            let dimensions = self.dimensions_for(P, dimension_mode, priority, true);
            let mut heap: BinaryHeap<(i64, usize, usize, usize, usize)> = BinaryHeap::new();
            for (h, w) in dimensions {
                let area_extra = h * w - P;
                for x0 in 0..=self.N - h {
                    for y0 in 0..=self.N - w {
                        let free_count = self.rect_count(&fs, x0, y0, x0 + h, y0 + w);
                        if free_count < P {
                            continue;
                        }
                        let slack = free_count - P;
                        let blocked = area_extra - slack;
                        let edges = self.rect_free_edges(&fs, x0, y0, x0 + h, y0 + w);
                        let mask_L = 4 * free_count - 2 * edges;
                        let defect = mask_L
                            .saturating_sub(Self::minimum_polyomino_perimeter_any(free_count));
                        let directional =
                            (x0 + y0) * 4 + if x0 & 1 != 0 { self.N - y0 } else { y0 };
                        let block_coefficient = (36_000.0 + 42_000.0 * priority).round() as i64;
                        let defect_coefficient = (5_200.0 + 3_800.0 * priority).round() as i64;
                        let slack_coefficient = (650.0 + 650.0 * priority).round() as i64;
                        let score = block_coefficient * blocked as i64
                            + defect_coefficient * defect as i64
                            + slack_coefficient * slack as i64
                            + 24 * area_extra as i64
                            + directional as i64;
                        let item = (score, x0, y0, h, w);
                        if heap.len() < box_limit {
                            heap.push(item);
                        } else if score < heap.peek().unwrap().0 {
                            heap.pop();
                            heap.push(item);
                        }
                    }
                }
            }
            let mut boxes: Vec<_> = heap.into_iter().collect();
            boxes.sort_unstable_by_key(|item| item.0);
            let mut candidates = Vec::new();
            let mut seen = HashSet::with_capacity(boxes.len() * 2 + 1);
            for (_, x0, y0, h, w) in boxes {
                if self.elapsed() >= scaled_time(1.83) {
                    break;
                }
                let Some(mut region) = self.peel_box_region(P, x0, y0, h, w, target_T, priority)
                else {
                    continue;
                };
                if region.L > self.minimum_polyomino_perimeter(P) {
                    self.polish_region(&mut region, 1, target_T, priority);
                }
                self.add_candidate(&mut candidates, &mut seen, region, P);
            }
            if candidates.is_empty() {
                return Vec::new();
            }
            let min_L = candidates.iter().map(|region| region.L).min().unwrap();
            for region in &mut candidates {
                self.evaluate_layout(region, &fs, P, target_T, priority);
                region.order_score = region.layout + 600 * (region.L - min_L) as i64;
            }
            candidates.sort_by(|a, b| {
                a.order_score
                    .cmp(&b.order_score)
                    .then(a.L.cmp(&b.L))
                    .then(a.hash.cmp(&b.hash))
            });
            if candidates.len() <= max_results {
                return candidates;
            }
            let mut best_L = 0;
            for index in 1..candidates.len() {
                if candidates[index].L < candidates[best_L].L
                    || (candidates[index].L == candidates[best_L].L
                        && candidates[index].layout < candidates[best_L].layout)
                {
                    best_L = index;
                }
            }
            let mut result = Vec::with_capacity(max_results);
            result.push(candidates[best_L].clone());
            for (index, region) in candidates.into_iter().enumerate() {
                if result.len() >= max_results {
                    break;
                }
                if index != best_L {
                    result.push(region);
                }
            }
            result
        }

        fn find_regions(
            &mut self,
            P: usize,
            max_results: usize,
            mode: i32,
            target_T: Option<usize>,
            priority: f64,
        ) -> Vec<Region> {
            let fs = self.build_free_state();
            if fs.free_count < P {
                return Vec::new();
            }
            let mut eligible: Vec<usize> = fs
                .size
                .iter()
                .enumerate()
                .filter_map(|(component, &size)| (size >= P).then_some(component))
                .collect();
            if eligible.is_empty() {
                return Vec::new();
            }

            let mut box_limit = if mode >= 2 {
                26
            } else if mode == 1 {
                14
            } else if mode == 0 {
                6
            } else {
                0
            };
            if mode >= 1 {
                box_limit += ((if mode >= 2 { 10.0 } else { 5.0 })
                    * self.terrain_roughness
                    * (0.35 + 0.65 * priority))
                    .round() as usize;
            }
            let component_limit = if mode >= 2 {
                7
            } else if mode == 1 {
                5
            } else {
                3
            };
            let seeds_per_component = if mode >= 2 {
                4
            } else if mode == 1 {
                3
            } else {
                2
            };
            let polish_iterations = if mode >= 2 {
                3
            } else if mode == 1 {
                2
            } else if mode == 0 {
                1
            } else {
                0
            };
            let mut candidates = Vec::with_capacity(170);
            let mut seen = HashSet::with_capacity(280);
            if mode >= 0 {
                self.add_compact_template_candidates(
                    P,
                    mode,
                    &fs,
                    &mut candidates,
                    &mut seen,
                    priority,
                );
            }

            if box_limit > 0 {
                let mut heap: BinaryHeap<(i64, usize, usize, usize, usize)> = BinaryHeap::new();
                for (h, w) in self.dimensions_for(P, mode, priority, false) {
                    let area_extra = h * w - P;
                    for x0 in 0..=self.N - h {
                        for y0 in 0..=self.N - w {
                            let free_count = self.rect_count(&fs, x0, y0, x0 + h, y0 + w);
                            if free_count < P {
                                continue;
                            }
                            let slack = free_count - P;
                            let blocked = area_extra - slack;
                            let edges = self.rect_free_edges(&fs, x0, y0, x0 + h, y0 + w);
                            let mask_L = 4 * free_count - 2 * edges;
                            let defect = mask_L
                                .saturating_sub(Self::minimum_polyomino_perimeter_any(free_count));
                            let directional =
                                (x0 + y0) * 4 + if x0 & 1 != 0 { self.N - y0 } else { y0 };
                            let block_coefficient = (40_000.0 + 48_000.0 * priority).round() as i64;
                            let defect_coefficient = (5_800.0 + 4_400.0 * priority).round() as i64;
                            let slack_coefficient = (700.0 + 700.0 * priority).round() as i64;
                            let score = block_coefficient * blocked as i64
                                + defect_coefficient * defect as i64
                                + slack_coefficient * slack as i64
                                + 24 * area_extra as i64
                                + directional as i64;
                            let item = (score, x0, y0, h, w);
                            if heap.len() < box_limit {
                                heap.push(item);
                            } else if score < heap.peek().unwrap().0 {
                                heap.pop();
                                heap.push(item);
                            }
                        }
                    }
                }
                let mut boxes: Vec<_> = heap.into_iter().collect();
                boxes.sort_unstable_by_key(|item| item.0);
                for (_, x0, y0, h, w) in boxes {
                    let center_x2 = (2 * x0 + h - 1) as i32;
                    let center_y2 = (2 * y0 + w - 1) as i32;
                    let mut center_seed = None;
                    let mut rank_seed = None;
                    let mut degree_seed = None;
                    let mut clear_seed = None;
                    let mut pocket_seed = None;
                    let mut best_distance = i32::MAX;
                    let mut best_rank = i32::MAX;
                    let mut best_degree = -1_i32;
                    let mut best_clear = -1_i32;
                    let mut best_pocket = i32::MAX;
                    for x in x0..x0 + h {
                        for y in y0..y0 + w {
                            let z = self.id(x, y);
                            if self.occ[z] != -1 || fs.cid[z] < 0 || fs.size[fs.cid[z] as usize] < P
                            {
                                continue;
                            }
                            let dx = 2 * x as i32 - center_x2;
                            let dy = 2 * y as i32 - center_y2;
                            let distance = dx * dx + dy * dy;
                            if distance < best_distance {
                                best_distance = distance;
                                center_seed = Some(z);
                            }
                            if self.static_rank[z] < best_rank {
                                best_rank = self.static_rank[z];
                                rank_seed = Some(z);
                            }
                            let degree = self.nb[z]
                                .iter()
                                .filter(|&&v| v >= 0 && self.occ[v as usize] == -1)
                                .count() as i32;
                            if degree > best_degree {
                                best_degree = degree;
                                degree_seed = Some(z);
                            }
                            let clear = self.static_clear[z] as i32;
                            if clear > best_clear {
                                best_clear = clear;
                                clear_seed = Some(z);
                            }
                            if clear < best_pocket
                                || (clear == best_pocket
                                    && pocket_seed.is_none_or(|old| {
                                        self.static_rank[z] < self.static_rank[old]
                                    }))
                            {
                                best_pocket = clear;
                                pocket_seed = Some(z);
                            }
                        }
                    }
                    let mut seed_pool =
                        vec![center_seed, rank_seed, degree_seed, clear_seed, pocket_seed]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                    seed_pool.sort_unstable();
                    seed_pool.dedup();
                    seed_pool.sort_by(|&a, &b| {
                        let score = |z: usize| {
                            let dx = 2 * self.xof(z) as i32 - center_x2;
                            let dy = 2 * self.yof(z) as i32 - center_y2;
                            let degree = self.nb[z]
                                .iter()
                                .filter(|&&v| v >= 0 && self.occ[v as usize] == -1)
                                .count() as f64;
                            let clear_term = self.terrain_roughness
                                * (0.50 - priority)
                                * 240.0
                                * self.static_clear[z] as f64;
                            let separator_term = self.terrain_roughness
                                * (2.2 - priority)
                                * 2.0
                                * self.separator_field[z] as f64;
                            3.0 * (dx * dx + dy * dy) as f64
                                + 0.7 * (self.static_rank[z] & 255) as f64
                                - 28.0 * degree
                                + clear_term
                                + separator_term
                        };
                        score(a)
                            .partial_cmp(&score(b))
                            .unwrap_or(Ordering::Equal)
                            .then(a.cmp(&b))
                    });
                    let use_seeds = if mode >= 2 { 3 } else { 2 };
                    for &seed in seed_pool.iter().take(use_seeds) {
                        if let Some(region) = self.grow_region(
                            P,
                            seed,
                            center_x2,
                            center_y2,
                            x0,
                            y0,
                            x0 + h,
                            y0 + w,
                            target_T,
                            priority,
                        ) {
                            self.add_candidate(&mut candidates, &mut seen, region, P);
                        }
                    }
                }
            }

            eligible.sort_by_key(|&component| fs.size[component]);
            let mut chosen: Vec<usize> = eligible.iter().copied().take(component_limit).collect();
            let mut largest = eligible[0];
            for &component in &eligible[1..] {
                if fs.size[component] > fs.size[largest] {
                    largest = component;
                }
            }
            if !chosen.contains(&largest) {
                chosen.push(largest);
            }
            for component in chosen {
                let cells = fs.cells[component].clone();
                if cells.len() == P {
                    let L = self.perimeter(&cells);
                    let region = Region {
                        hash: self.region_hash(&cells),
                        cells,
                        L,
                        ..Region::default()
                    };
                    self.add_candidate(&mut candidates, &mut seen, region, P);
                    continue;
                }
                let mut min_rank = cells[0];
                let mut min_x = cells[0];
                let mut max_x = cells[0];
                let mut center = cells[0];
                let mut max_clear = cells[0];
                let mut min_clear = cells[0];
                let mut x_min = self.N;
                let mut x_max = 0;
                let mut y_min = self.N;
                let mut y_max = 0;
                for &z in &cells {
                    x_min = x_min.min(self.xof(z));
                    x_max = x_max.max(self.xof(z));
                    y_min = y_min.min(self.yof(z));
                    y_max = y_max.max(self.yof(z));
                }
                let center_x2 = (x_min + x_max) as i32;
                let center_y2 = (y_min + y_max) as i32;
                let mut best_center = i32::MAX;
                for &z in &cells {
                    if self.static_rank[z] < self.static_rank[min_rank] {
                        min_rank = z;
                    }
                    if (self.xof(z), self.yof(z)) < (self.xof(min_x), self.yof(min_x)) {
                        min_x = z;
                    }
                    if (self.xof(z), self.yof(z)) > (self.xof(max_x), self.yof(max_x)) {
                        max_x = z;
                    }
                    if self.static_clear[z] > self.static_clear[max_clear] {
                        max_clear = z;
                    }
                    if self.static_clear[z] < self.static_clear[min_clear]
                        || (self.static_clear[z] == self.static_clear[min_clear]
                            && self.static_rank[z] < self.static_rank[min_clear])
                    {
                        min_clear = z;
                    }
                    let dx = 2 * self.xof(z) as i32 - center_x2;
                    let dy = 2 * self.yof(z) as i32 - center_y2;
                    let distance = dx * dx + dy * dy;
                    if distance < best_center {
                        best_center = distance;
                        center = z;
                    }
                }
                let mut seeds = vec![min_rank, center, min_x, max_x, max_clear, min_clear];
                if mode >= 2 {
                    seeds.push(cells[self.rng.next_int(cells.len())]);
                }
                seeds.sort_unstable();
                seeds.dedup();
                seeds.sort_by(|&a, &b| {
                    let score = |z: usize| {
                        let dx = 2 * self.xof(z) as i32 - center_x2;
                        let dy = 2 * self.yof(z) as i32 - center_y2;
                        let clear_term = self.terrain_roughness
                            * (0.50 - priority)
                            * 300.0
                            * self.static_clear[z] as f64;
                        let separator_term = self.terrain_roughness
                            * (2.2 - priority)
                            * 2.2
                            * self.separator_field[z] as f64;
                        2.0 * (dx * dx + dy * dy) as f64
                            + 0.8 * self.static_rank[z] as f64
                            + clear_term
                            + separator_term
                    };
                    score(a)
                        .partial_cmp(&score(b))
                        .unwrap_or(Ordering::Equal)
                        .then(a.cmp(&b))
                });
                for &seed in seeds.iter().take(seeds_per_component) {
                    if let Some(region) = self.grow_region(
                        P,
                        seed,
                        2 * self.xof(seed) as i32,
                        2 * self.yof(seed) as i32,
                        0,
                        0,
                        self.N,
                        self.N,
                        target_T,
                        priority,
                    ) {
                        self.add_candidate(&mut candidates, &mut seen, region, P);
                    }
                }
            }

            if candidates.is_empty() {
                let component = eligible[0];
                let seed = fs.cells[component][0];
                if let Some(region) = self.grow_region(
                    P,
                    seed,
                    2 * self.xof(seed) as i32,
                    2 * self.yof(seed) as i32,
                    0,
                    0,
                    self.N,
                    self.N,
                    target_T,
                    priority,
                ) {
                    self.add_candidate(&mut candidates, &mut seen, region, P);
                }
            }
            if candidates.is_empty() {
                return Vec::new();
            }
            let theoretical_L = self.minimum_polyomino_perimeter(P);
            let mut polish_order: Vec<usize> = (0..candidates.len()).collect();
            polish_order.sort_by_key(|&index| candidates[index].L);
            let mut polished = 0;
            for index in polish_order {
                if polished >= polish_iterations * 4 {
                    break;
                }
                if candidates[index].L <= theoretical_L {
                    continue;
                }
                self.polish_region(
                    &mut candidates[index],
                    polish_iterations,
                    target_T,
                    priority,
                );
                polished += 1;
            }
            let min_L = candidates.iter().map(|region| region.L).min().unwrap();
            let candidate_count = candidates.len();
            let mut filtered = Vec::with_capacity(candidate_count);
            for mut region in candidates {
                if region.L > min_L + 10 && candidate_count > 1 {
                    continue;
                }
                self.evaluate_layout(&mut region, &fs, P, target_T, priority);
                let perimeter_weight = (520.0 + 180.0 * priority).round() as i64;
                region.order_score = region.layout + perimeter_weight * (region.L - min_L) as i64;
                filtered.push(region);
            }
            filtered.sort_by(|a, b| {
                a.order_score
                    .cmp(&b.order_score)
                    .then(a.L.cmp(&b.L))
                    .then(a.hash.cmp(&b.hash))
            });
            if filtered.len() <= max_results {
                return filtered;
            }
            let mut best_L = 0;
            for index in 1..filtered.len() {
                if filtered[index].L < filtered[best_L].L
                    || (filtered[index].L == filtered[best_L].L
                        && filtered[index].layout < filtered[best_L].layout)
                {
                    best_L = index;
                }
            }
            let mut order_by_L: Vec<usize> = (0..filtered.len()).collect();
            order_by_L.sort_by(|&a, &b| {
                filtered[a]
                    .L
                    .cmp(&filtered[b].L)
                    .then(filtered[a].layout.cmp(&filtered[b].layout))
                    .then(filtered[a].hash.cmp(&filtered[b].hash))
            });
            let mut indices = Vec::with_capacity(max_results);
            let mut picked = HashSet::new();
            let selection_order = std::iter::once(best_L)
                .chain(std::iter::once(0))
                .chain(order_by_L)
                .chain(0..filtered.len());
            for index in selection_order {
                if indices.len() >= max_results {
                    break;
                }
                if picked.insert(filtered[index].hash) {
                    indices.push(index);
                }
            }
            indices
                .into_iter()
                .map(|index| filtered[index].clone())
                .collect()
        }

        fn truncated_moments(beta: f64, a: f64, K: usize, output: &mut [f64]) {
            let x = beta * a;
            if a <= 0.0 {
                output[..=K].fill(0.0);
                return;
            }
            if x < 0.5 {
                for (k, slot) in output.iter_mut().enumerate().take(K + 1) {
                    let mut term = beta * a.powi((k + 1) as i32) / (k + 1) as f64;
                    let mut sum = term;
                    for m in 0..40 {
                        term *= (-x) / (m + 1) as f64 * (k + m + 1) as f64 / (k + m + 2) as f64;
                        sum += term;
                        if term.abs() < 1e-24 * sum.abs().max(1.0) {
                            break;
                        }
                    }
                    *slot = sum.max(0.0);
                }
            } else {
                let exponential = (-x).exp();
                output[0] = -(-x).exp_m1();
                let mut a_power = 1.0;
                for k in 1..=K {
                    a_power *= a;
                    output[k] = k as f64 / beta * output[k - 1] - a_power * exponential;
                    if output[k] < 0.0 && output[k] > -1e-24 {
                        output[k] = 0.0;
                    }
                }
            }
        }

        fn evaluate_beta(&self, beta: f64, S: usize, remaining: usize) -> BetaEval {
            if !(12.5..=50.0).contains(&beta) {
                return BetaEval::default();
            }
            let a = (HORIZON - S) as f64 / HORIZON as f64;
            let c = S as f64 / HORIZON as f64;
            let mut moments = [0.0; 10];
            Self::truncated_moments(beta, a, 9, &mut moments);
            let mut q = moments[0];
            let mut q1 = moments[0] / beta - moments[1];
            let mut q2 = moments[2] - 2.0 * moments[1] / beta;
            for k in 0..=7 {
                q -= c * moments[k];
                q1 -= c * (moments[k] / beta - moments[k + 1]);
                q2 -= c * (moments[k + 2] - 2.0 * moments[k + 1] / beta);
            }
            if !q.is_finite() || q <= 1e-35 {
                return BetaEval::default();
            }
            let q_ratio = q1 / q;
            let value = self.observed_n as f64 * beta.ln()
                - self.duration_y / HORIZON as f64 * beta
                + remaining as f64 * q.ln();
            let first = self.observed_n as f64 / beta - self.duration_y / HORIZON as f64
                + remaining as f64 * q_ratio;
            let second = -(self.observed_n as f64) / (beta * beta)
                + remaining as f64 * (q2 / q - q_ratio * q_ratio);
            BetaEval {
                value,
                first,
                second,
                ok: value.is_finite() && first.is_finite() && second.is_finite(),
            }
        }

        fn update_theta(&mut self, S: usize, duration: usize) {
            self.observed_n += 1;
            self.duration_y += (duration - 1) as f64;
            let remaining = self.M - self.observed_n;
            if remaining < 32 || S > 99_000 {
                self.theta_estimate =
                    (self.duration_y / self.observed_n as f64).clamp(2_000.0, 8_000.0);
                self.beta_estimate = HORIZON as f64 / self.theta_estimate;
                return;
            }
            if self.observed_n <= 32 {
                let mut best_beta = self.beta_estimate;
                let mut best = -1e100;
                const GRID: usize = 32;
                for k in 0..=GRID {
                    let beta = 12.5 + (50.0 - 12.5) * k as f64 / GRID as f64;
                    let evaluation = self.evaluate_beta(beta, S, remaining);
                    if evaluation.ok && evaluation.value > best {
                        best = evaluation.value;
                        best_beta = beta;
                    }
                }
                let width = (50.0 - 12.5) / GRID as f64;
                let mut low = (best_beta - width).max(12.5);
                let mut high = (best_beta + width).min(50.0);
                for _ in 0..8 {
                    let m1 = (2.0 * low + high) / 3.0;
                    let m2 = (low + 2.0 * high) / 3.0;
                    let e1 = self.evaluate_beta(m1, S, remaining);
                    let e2 = self.evaluate_beta(m2, S, remaining);
                    if !e1.ok || (e2.ok && e1.value < e2.value) {
                        low = m1;
                    } else {
                        high = m2;
                    }
                }
                self.beta_estimate = (low + high) / 2.0;
            } else {
                let mut beta = self.beta_estimate.clamp(12.5, 50.0);
                for _ in 0..3 {
                    let evaluation = self.evaluate_beta(beta, S, remaining);
                    if !evaluation.ok {
                        break;
                    }
                    let mut candidate = if evaluation.second < -1e-12 {
                        beta - evaluation.first / evaluation.second
                    } else {
                        beta + if evaluation.first > 0.0 { 0.75 } else { -0.75 }
                    };
                    candidate = candidate.clamp(12.5, 50.0);
                    candidate = candidate.min(beta + 4.0).max(beta - 4.0);
                    let mut candidate_evaluation = self.evaluate_beta(candidate, S, remaining);
                    let mut guard = 0;
                    while (!candidate_evaluation.ok
                        || candidate_evaluation.value < evaluation.value)
                        && guard < 3
                    {
                        guard += 1;
                        candidate = (candidate + beta) / 2.0;
                        candidate_evaluation = self.evaluate_beta(candidate, S, remaining);
                    }
                    if !candidate_evaluation.ok || candidate_evaluation.value < evaluation.value {
                        break;
                    }
                    beta = candidate;
                }
                self.beta_estimate = beta;
            }
            self.theta_estimate = HORIZON as f64 / self.beta_estimate;
        }

        fn expected_future_duration(&self, S: usize) -> f64 {
            let a = (HORIZON - S) as f64 / HORIZON as f64;
            if a <= 0.0 {
                return 1.0;
            }
            let beta = HORIZON as f64 / self.theta_estimate;
            const K: usize = 48;
            let mut denominator = 0.0;
            let mut numerator = 0.0;
            for k in 0..K {
                let z = a * (k as f64 + 0.5) / K as f64;
                let weight = (a - z) / (1.0 - z) * beta * (-beta * z).exp();
                denominator += weight;
                numerator += (1.0 + HORIZON as f64 * z) * weight;
            }
            if denominator <= 0.0 {
                return 1.0_f64.max((HORIZON - S) as f64 / 3.0);
            }
            (numerator / denominator).clamp(1.0, (HORIZON - S) as f64)
        }

        fn interval_congestion_threshold(
            &self,
            index: usize,
            S: usize,
            T: usize,
            global_threshold: f64,
        ) -> f64 {
            let remaining = self.M - index - 1;
            if remaining == 0 || T <= S || S >= HORIZON - 1 {
                return global_threshold;
            }
            const J: usize = 7;
            const K: usize = 16;
            let mut sample = [0.0; J];
            let mut active = [0.0; J];
            for j in 0..J {
                sample[j] = S as f64 + (T - S) as f64 * (j as f64 + 0.5) / J as f64;
            }
            for group in self.groups.iter().take(index).filter(|group| group.active) {
                for j in 0..J {
                    if group.T as f64 > sample[j] {
                        active[j] += group.P as f64;
                    }
                }
            }
            let q = S as f64 / HORIZON as f64;
            let a = 1.0 - q;
            let beta = HORIZON as f64 / self.theta_estimate;
            let mut denominator = 0.0;
            let mut numerator = [0.0; J];
            let dz = a / K as f64;
            for k in 0..K {
                let z = dz * (k as f64 + 0.5);
                let base = beta * (-beta * z).exp() * dz / (1.0 - z).max(1e-12);
                denominator += base * (a - z);
                for j in 0..J {
                    let u = sample[j] / HORIZON as f64;
                    let high = u.min(1.0 - z);
                    let low = q.max(u - z);
                    if high > low {
                        numerator[j] += base * (high - low);
                    }
                }
            }
            if denominator <= 1e-24 {
                return global_threshold;
            }
            let mut local = Vec::with_capacity(J);
            for j in 0..J {
                let future = remaining as f64 * EP * numerator[j] / denominator;
                if future < 1.0 {
                    continue;
                }
                let fraction =
                    ((self.capacity_cells - active[j]) / (future * 1.03)).clamp(0.002, 0.999999);
                let threshold = (SIGMA_Q * inv_norm_cdf(1.0 - fraction)).exp();
                local.push(threshold.clamp(0.12, 5.0));
            }
            if local.is_empty() {
                return global_threshold;
            }
            local.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            let local_threshold = local[3 * local.len() / 4];
            if local_threshold <= global_threshold {
                return global_threshold;
            }
            let strength = ((self.observed_n as f64 - 8.0) / 24.0).clamp(0.0, 1.0);
            (global_threshold * (local_threshold / global_threshold).powf(strength)).min(5.0)
        }

        fn quality_threshold(&self, index: usize, S: usize, T: usize, P: usize) -> f64 {
            let mut active_remaining = 0.0;
            for group in self.groups.iter().take(index).filter(|group| group.active) {
                active_remaining += group.P as f64 * (group.T - S) as f64;
            }
            let remaining = self.M - index - 1;
            let capacity_time = self.capacity_cells * (HORIZON - S) as f64;
            let budget = capacity_time - active_remaining;
            let expected_duration = self.expected_future_duration(S);
            let demand = (remaining + 1) as f64 * EP * expected_duration * 1.03;
            let fraction =
                (if demand > 0.0 { budget / demand } else { 1.0 }).clamp(0.002, 0.999999);
            let z = inv_norm_cdf(1.0 - fraction);
            let mut threshold = (SIGMA_Q * z).exp().clamp(0.12, 4.0);
            threshold = self.interval_congestion_threshold(index, S, T, threshold);
            let instantaneous = self.occupied_cells as f64 / self.capacity_cells.max(1.0);
            if instantaneous > 0.90 {
                threshold *= (1.4 * (instantaneous - 0.90)).exp();
            }
            let large = Self::clamp01((P as f64 - 55.0) / 95.0);
            let pressure = Self::clamp01((threshold - 0.55) / 1.65);
            threshold *= 1.0 + 0.105 * self.terrain_roughness * large * pressure;
            threshold.min(5.0)
        }

        fn compact_reference(&self) -> f64 {
            let weight = (0.65 * self.compact_observed as f64 / 40.0).min(0.65);
            (self.compact_prior * (1.0 - weight) + self.compact_ewma * weight).clamp(0.50, 0.92)
        }

        fn observe_compactness(&mut self, compactness: f64) {
            let compactness = compactness.clamp(0.30, 1.0);
            let alpha = if self.compact_observed < 25 {
                0.06
            } else {
                0.018
            };
            self.compact_ewma += alpha * (compactness - self.compact_ewma);
            self.compact_observed += 1;
        }

        fn same_region(region: &Region, group: &Group) -> bool {
            if region.hash != group.hash || region.cells.len() != group.cells.len() {
                return false;
            }
            let mut a = region.cells.clone();
            let mut b = group.cells.clone();
            a.sort_unstable();
            b.sort_unstable();
            a == b
        }

        fn movable_candidates(
            &self,
            incoming_P: usize,
            limit: usize,
            single_only: bool,
        ) -> Vec<usize> {
            let fs = self.build_free_state();
            let mut items: Vec<(f64, usize)> = Vec::new();
            let mut seen_component = vec![-1_i32; fs.size.len()];
            let mut token = 0_i32;
            for group_id in 0..self.M {
                let group = &self.groups[group_id];
                if !group.active {
                    continue;
                }
                token += 1;
                let mut potential = group.P;
                let mut separator_sum = 0_usize;
                for &z in &group.cells {
                    separator_sum += self.separator_field[z] as usize;
                    for &w in &self.nb[z] {
                        if w >= 0 && self.occ[w as usize] == -1 {
                            let component = fs.cid[w as usize];
                            if component >= 0 && seen_component[component as usize] != token {
                                seen_component[component as usize] = token;
                                potential += fs.size[component as usize];
                            }
                        }
                    }
                }
                if single_only && potential < incoming_P {
                    continue;
                }
                let mut max_untouched = 0;
                for (component, &size) in fs.size.iter().enumerate() {
                    if seen_component[component] != token {
                        max_untouched = max_untouched.max(size);
                    }
                }
                let slack = group
                    .worst_L
                    .saturating_sub(self.minimum_polyomino_perimeter(group.P));
                let flexibility = 1.0 + 0.06 * slack as f64;
                let mut score =
                    self.move_cost(group.V) as f64 / (potential.max(1) as f64 * flexibility);
                let separator_mean = separator_sum as f64 / group.P.max(1) as f64;
                score /= 1.0 + 0.75 * separator_mean / 255.0;
                if max_untouched >= group.P {
                    score *= 0.72;
                } else {
                    let deficit =
                        group.P.saturating_sub(max_untouched) as f64 / group.P.max(1) as f64;
                    score *= 1.22 + 0.55 * deficit.max(0.0);
                }
                if potential < incoming_P {
                    score += 1e12;
                }
                score += 0.00008 * (group.P as f64 - incoming_P as f64).abs();
                items.push((score, group_id));
            }
            items.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            items
                .into_iter()
                .take(limit)
                .map(|(_, group_id)| group_id)
                .collect()
        }

        fn movable_pair_candidates(&self, incoming_P: usize, limit: usize) -> Vec<(usize, usize)> {
            let fs = self.build_free_state();
            let mut active_ids = Vec::new();
            let mut active_position = vec![-1_isize; self.M];
            for group_id in 0..self.M {
                if self.groups[group_id].active {
                    active_position[group_id] = active_ids.len() as isize;
                    active_ids.push(group_id);
                }
            }
            let active_count = active_ids.len();
            let mut touching_components = vec![Vec::new(); active_count];
            let mut touching_groups = vec![vec![false; active_count]; active_count];
            let mut seen_component = vec![-1_isize; fs.size.len()];
            for aa in 0..active_count {
                let group_id = active_ids[aa];
                for &z in &self.groups[group_id].cells {
                    for &w in &self.nb[z] {
                        if w < 0 {
                            continue;
                        }
                        let w = w as usize;
                        if self.occ[w] == -1 {
                            let component = fs.cid[w];
                            if component >= 0 && seen_component[component as usize] != aa as isize {
                                seen_component[component as usize] = aa as isize;
                                touching_components[aa].push(component as usize);
                            }
                        } else if self.occ[w] >= 0 {
                            let bb = active_position[self.occ[w] as usize];
                            if bb >= 0 && bb as usize != aa {
                                touching_groups[aa][bb as usize] = true;
                            }
                        }
                    }
                }
                touching_components[aa].sort_unstable();
            }
            let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
            for aa in 0..active_count {
                for bb in aa + 1..active_count {
                    let mut linked = touching_groups[aa][bb] || touching_groups[bb][aa];
                    let mut ia = 0;
                    let mut ib = 0;
                    let group_a = active_ids[aa];
                    let group_b = active_ids[bb];
                    let mut potential = self.groups[group_a].P + self.groups[group_b].P;
                    while ia < touching_components[aa].len() || ib < touching_components[bb].len() {
                        let component;
                        if ib == touching_components[bb].len()
                            || (ia < touching_components[aa].len()
                                && touching_components[aa][ia] < touching_components[bb][ib])
                        {
                            component = touching_components[aa][ia];
                            ia += 1;
                        } else if ia == touching_components[aa].len()
                            || touching_components[bb][ib] < touching_components[aa][ia]
                        {
                            component = touching_components[bb][ib];
                            ib += 1;
                        } else {
                            component = touching_components[aa][ia];
                            ia += 1;
                            ib += 1;
                            linked = true;
                        }
                        potential += fs.size[component];
                    }
                    if !linked || potential < incoming_P {
                        continue;
                    }
                    let slack_a = self.groups[group_a]
                        .worst_L
                        .saturating_sub(self.minimum_polyomino_perimeter(self.groups[group_a].P));
                    let slack_b = self.groups[group_b]
                        .worst_L
                        .saturating_sub(self.minimum_polyomino_perimeter(self.groups[group_b].P));
                    let flexibility = 1.0 + 0.04 * (slack_a + slack_b) as f64;
                    let mut separator = 0.0;
                    for &z in &self.groups[group_a].cells {
                        separator += self.separator_field[z] as f64;
                    }
                    for &z in &self.groups[group_b].cells {
                        separator += self.separator_field[z] as f64;
                    }
                    separator /= (self.groups[group_a].P + self.groups[group_b].P).max(1) as f64;
                    let loss = (self.move_cost(self.groups[group_a].V)
                        + self.move_cost(self.groups[group_b].V))
                        as f64;
                    let score = loss
                        / (potential.max(1) as f64 * flexibility)
                        / (1.0 + 0.45 * separator / 255.0)
                        + 0.00002
                            * ((self.groups[group_a].P + self.groups[group_b].P) as f64
                                - incoming_P as f64)
                                .abs();
                    candidates.push((score, group_a, group_b));
                }
            }
            candidates.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(Ordering::Equal)
                    .then(a.1.cmp(&b.1))
                    .then(a.2.cmp(&b.2))
            });
            candidates
                .into_iter()
                .take(limit)
                .map(|(_, a, b)| (a, b))
                .collect()
        }

        #[allow(clippy::too_many_arguments)]
        fn consider_move_plan(
            &self,
            incoming: &Region,
            placed: &[Region],
            has: &[bool],
            ids: &[usize],
            current_id: usize,
            required_fee_quality: f64,
            best: &mut MovePlan,
        ) {
            let mut output_ids = Vec::new();
            let mut output_regions = Vec::new();
            let mut loss = 0_i64;
            for (index, &group_id) in ids.iter().enumerate() {
                if !has[index] {
                    return;
                }
                let group = &self.groups[group_id];
                if Self::same_region(&placed[index], group) {
                    continue;
                }
                let new_worst_L = group.worst_L.max(placed[index].L);
                loss += self.move_cost(group.V);
                loss += self.fee(group.V, group.P, group.worst_L)
                    - self.fee(group.V, group.P, new_worst_L);
                output_ids.push(group_id);
                output_regions.push(placed[index].clone());
            }
            if output_ids.is_empty() {
                return;
            }
            let current = &self.groups[current_id];
            let net_gain = self.fee(current.V, current.P, incoming.L) - loss;
            let denominator = current.P as f64 * ((current.T - current.S) as f64).powf(0.9);
            let fee_quality = net_gain as f64 / denominator;
            if net_gain > 0 && fee_quality >= required_fee_quality && net_gain > best.net_gain {
                best.ids = output_ids;
                best.dest = output_regions;
                best.incoming = incoming.clone();
                best.net_gain = net_gain;
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn dfs_incoming_first(
            &mut self,
            at: usize,
            order: &[usize],
            ids: &[usize],
            incoming: &Region,
            placed: &mut [Region],
            has: &mut [bool],
            current_id: usize,
            required_fee_quality: f64,
            q_req: f64,
            best: &mut MovePlan,
        ) {
            if self.operation_time_exhausted() {
                return;
            }
            if at == order.len() {
                self.consider_move_plan(
                    incoming,
                    placed,
                    has,
                    ids,
                    current_id,
                    required_fee_quality,
                    best,
                );
                return;
            }
            let group_id = order[at];
            let index = ids.iter().position(|&id| id == group_id).unwrap();
            let priority = self.placement_priority(&self.groups[group_id], q_req);
            let P = self.groups[group_id].P;
            let T = self.groups[group_id].T;
            let regions = self.find_regions(P, 2, 0, Some(T), priority);
            for region in regions {
                for &z in &region.cells {
                    self.occ[z] = -4 - at as i32;
                }
                placed[index] = region.clone();
                has[index] = true;
                self.dfs_incoming_first(
                    at + 1,
                    order,
                    ids,
                    incoming,
                    placed,
                    has,
                    current_id,
                    required_fee_quality,
                    q_req,
                    best,
                );
                has[index] = false;
                for &z in &region.cells {
                    self.occ[z] = -1;
                }
                if self.operation_time_exhausted() {
                    break;
                }
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn dfs_moved_first(
            &mut self,
            at: usize,
            order: &[usize],
            ids: &[usize],
            old_union: &[bool; MAX_C],
            placed: &mut [Region],
            has: &mut [bool],
            current_id: usize,
            required_fee_quality: f64,
            search_mode: i32,
            q_req: f64,
            priority: f64,
            best: &mut MovePlan,
        ) {
            if self.operation_time_exhausted() {
                return;
            }
            if at == order.len() {
                let current_P = self.groups[current_id].P;
                let current_T = self.groups[current_id].T;
                let incoming_regions =
                    self.find_regions(current_P, 3, search_mode, Some(current_T), priority);
                for incoming in incoming_regions {
                    self.consider_move_plan(
                        &incoming,
                        placed,
                        has,
                        ids,
                        current_id,
                        required_fee_quality,
                        best,
                    );
                    if self.operation_time_exhausted() {
                        break;
                    }
                }
                return;
            }
            let group_id = order[at];
            let index = ids.iter().position(|&id| id == group_id).unwrap();
            let group_priority = self.placement_priority(&self.groups[group_id], q_req);
            let P = self.groups[group_id].P;
            let T = self.groups[group_id].T;
            let original_group = self.groups[group_id].clone();
            let mut regions = self.find_regions(P, 3, 0, Some(T), group_priority);
            regions.sort_by_key(|region| {
                let overlap = region.cells.iter().filter(|&&z| old_union[z]).count();
                let same = usize::from(Self::same_region(region, &original_group));
                (same * 10_000 + overlap, region.L)
            });
            let mut used = 0;
            for region in regions {
                if ids.len() == 1 && Self::same_region(&region, &original_group) {
                    continue;
                }
                for &z in &region.cells {
                    self.occ[z] = -4 - at as i32;
                }
                placed[index] = region.clone();
                has[index] = true;
                self.dfs_moved_first(
                    at + 1,
                    order,
                    ids,
                    old_union,
                    placed,
                    has,
                    current_id,
                    required_fee_quality,
                    search_mode,
                    q_req,
                    priority,
                    best,
                );
                has[index] = false;
                for &z in &region.cells {
                    self.occ[z] = -1;
                }
                used += 1;
                if used >= 2 || self.operation_time_exhausted() {
                    break;
                }
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn try_move_set(
            &mut self,
            ids: &[usize],
            current_id: usize,
            required_fee_quality: f64,
            search_mode: i32,
            q_req: f64,
            priority: f64,
            enable_moved_first: bool,
        ) -> MovePlan {
            let mut best = MovePlan::default();
            let mut old_union = [false; MAX_C];
            for &group_id in ids {
                for &z in &self.groups[group_id].cells {
                    self.occ[z] = -1;
                    old_union[z] = true;
                }
            }
            let mut order = ids.to_vec();
            order.sort_by_key(|&group_id| Reverse(self.groups[group_id].P));
            let mut orders = vec![order.clone()];
            if ids.len() == 2 {
                order.reverse();
                orders.push(order);
            }

            if !self.operation_time_exhausted() {
                let current_P = self.groups[current_id].P;
                let current_T = self.groups[current_id].T;
                let incoming_regions =
                    self.find_regions(current_P, 3, search_mode, Some(current_T), priority);
                for incoming in incoming_regions {
                    if self.operation_time_exhausted() {
                        break;
                    }
                    for &z in &incoming.cells {
                        self.occ[z] = -3;
                    }
                    for order in &orders {
                        let mut placed = vec![Region::default(); ids.len()];
                        let mut has = vec![false; ids.len()];
                        self.dfs_incoming_first(
                            0,
                            order,
                            ids,
                            &incoming,
                            &mut placed,
                            &mut has,
                            current_id,
                            required_fee_quality,
                            q_req,
                            &mut best,
                        );
                        if self.operation_time_exhausted() {
                            break;
                        }
                    }
                    for &z in &incoming.cells {
                        self.occ[z] = -1;
                    }
                }
            }

            if enable_moved_first && !self.operation_time_exhausted() {
                for order in &orders {
                    let mut placed = vec![Region::default(); ids.len()];
                    let mut has = vec![false; ids.len()];
                    self.dfs_moved_first(
                        0,
                        order,
                        ids,
                        &old_union,
                        &mut placed,
                        &mut has,
                        current_id,
                        required_fee_quality,
                        search_mode,
                        q_req,
                        priority,
                        &mut best,
                    );
                    if self.operation_time_exhausted() {
                        break;
                    }
                }
            }
            for &group_id in ids {
                for &z in &self.groups[group_id].cells {
                    self.occ[z] = group_id as i32;
                }
            }
            best
        }

        fn search_profitable_upgrade(
            &mut self,
            current_id: usize,
            required_fee_quality: f64,
            q_req: f64,
            priority: f64,
            urgency: f64,
        ) -> MovePlan {
            let mut best = MovePlan::default();
            if self.grass_count - self.occupied_cells < self.groups[current_id].P {
                return best;
            }
            let limit = 3 + (3.0 * Self::clamp01(urgency)).round() as usize;
            for group_id in self.movable_candidates(self.groups[current_id].P, limit, true) {
                let plan = self.try_move_set(
                    &[group_id],
                    current_id,
                    required_fee_quality,
                    0,
                    q_req,
                    priority,
                    self.terrain_roughness > 0.35 && urgency > 0.45,
                );
                if plan.net_gain > best.net_gain {
                    best = plan;
                }
                if self.operation_time_exhausted() {
                    break;
                }
            }
            best
        }

        fn search_moves(
            &mut self,
            current_id: usize,
            required_fee_quality: f64,
            q_req: f64,
            priority: f64,
            urgency: f64,
        ) -> MovePlan {
            let mut best = MovePlan::default();
            if self.grass_count - self.occupied_cells < self.groups[current_id].P {
                return best;
            }
            let limit = 5 + (4.0 * Self::clamp01(urgency)).round() as usize;
            let single_mode = if urgency > 0.62 && self.terrain_roughness > 0.25 {
                1
            } else {
                0
            };
            for group_id in self.movable_candidates(self.groups[current_id].P, limit, true) {
                let plan = self.try_move_set(
                    &[group_id],
                    current_id,
                    required_fee_quality,
                    single_mode,
                    q_req,
                    priority,
                    true,
                );
                if plan.net_gain > best.net_gain {
                    best = plan;
                }
                if self.operation_time_exhausted() {
                    break;
                }
            }
            let current = &self.groups[current_id];
            let raw_q =
                current.V as f64 / (current.P as f64 * ((current.T - current.S) as f64).powf(0.9));
            let ideal_fee = self.fee(
                current.V,
                current.P,
                self.minimum_polyomino_perimeter(current.P),
            );
            let weak_single = best.net_gain == i64::MIN || best.net_gain * 10 < ideal_fee * 9;
            if weak_single
                && (self.R_milli <= 55 || raw_q > q_req * 1.55)
                && raw_q > q_req * 0.95
                && urgency > 0.30
                && !self.operation_time_exhausted()
            {
                let pair_limit = 3 + usize::from(urgency > 0.75) * 2;
                let mut pairs = self.movable_pair_candidates(self.groups[current_id].P, pair_limit);
                let mut used: HashSet<(usize, usize)> = pairs
                    .iter()
                    .map(|&(a, b)| if a < b { (a, b) } else { (b, a) })
                    .collect();
                let base = self.movable_candidates(self.groups[current_id].P, 4, false);
                for a in 0..base.len() {
                    if pairs.len() >= pair_limit + 2 {
                        break;
                    }
                    for b in a + 1..base.len() {
                        if pairs.len() >= pair_limit + 2 {
                            break;
                        }
                        let pair = if base[a] < base[b] {
                            (base[a], base[b])
                        } else {
                            (base[b], base[a])
                        };
                        if used.insert(pair) {
                            pairs.push(pair);
                        }
                    }
                }
                for (a, b) in pairs {
                    let plan = self.try_move_set(
                        &[a, b],
                        current_id,
                        required_fee_quality,
                        0,
                        q_req,
                        priority,
                        urgency > 0.62,
                    );
                    if plan.net_gain > best.net_gain {
                        best = plan;
                    }
                    if self.operation_time_exhausted() {
                        break;
                    }
                }
            }
            best
        }

        fn release_departed(
            &mut self,
            S: usize,
            departures: &mut BinaryHeap<Reverse<(usize, usize)>>,
        ) {
            while departures.peek().is_some_and(|Reverse((T, _))| *T < S) {
                let Reverse((_, group_id)) = departures.pop().unwrap();
                if !self.groups[group_id].active {
                    continue;
                }
                for &z in &self.groups[group_id].cells {
                    self.occ[z] = -1;
                }
                self.occupied_cells -= self.groups[group_id].P;
                self.groups[group_id].active = false;
            }
        }

        fn apply_direct(
            &mut self,
            index: usize,
            region: &Region,
            departures: &mut BinaryHeap<Reverse<(usize, usize)>>,
        ) {
            self.groups[index].active = true;
            self.groups[index].cells = region.cells.clone();
            self.groups[index].worst_L = region.L;
            self.groups[index].hash = region.hash;
            for &z in &region.cells {
                self.occ[z] = index as i32;
            }
            self.occupied_cells += self.groups[index].P;
            departures.push(Reverse((self.groups[index].T, index)));
        }

        fn apply_move_plan(
            &mut self,
            index: usize,
            plan: &MovePlan,
            departures: &mut BinaryHeap<Reverse<(usize, usize)>>,
        ) {
            for &group_id in &plan.ids {
                for &z in &self.groups[group_id].cells {
                    self.occ[z] = -1;
                }
            }
            for (position, &group_id) in plan.ids.iter().enumerate() {
                let region = &plan.dest[position];
                self.groups[group_id].cells = region.cells.clone();
                self.groups[group_id].hash = region.hash;
                self.groups[group_id].worst_L = self.groups[group_id].worst_L.max(region.L);
                for &z in &region.cells {
                    self.occ[z] = group_id as i32;
                }
            }
            self.apply_direct(index, &plan.incoming, departures);
        }

        fn output_turn<W: Write>(
            &self,
            writer: &mut W,
            move_plan: Option<&MovePlan>,
            direct: Option<&Region>,
            accept: bool,
        ) -> io::Result<()> {
            let mut output = String::new();
            if let Some(plan) = move_plan.filter(|plan| plan.net_gain != i64::MIN) {
                use std::fmt::Write as _;
                writeln!(&mut output, "{}", plan.ids.len()).unwrap();
                for (position, &group_id) in plan.ids.iter().enumerate() {
                    writeln!(&mut output, "{}", group_id).unwrap();
                    for &z in &plan.dest[position].cells {
                        writeln!(&mut output, "{} {}", self.xof(z), self.yof(z)).unwrap();
                    }
                }
            } else {
                output.push_str("0\n");
            }
            if !accept {
                output.push_str("No\n");
            } else {
                use std::fmt::Write as _;
                output.push_str("Yes\n");
                let region = if let Some(plan) = move_plan {
                    &plan.incoming
                } else {
                    direct.expect("accepted direct placement")
                };
                for &z in &region.cells {
                    writeln!(&mut output, "{} {}", self.xof(z), self.yof(z)).unwrap();
                }
            }
            writer.write_all(output.as_bytes())?;
            writer.flush()
        }

        pub(super) fn run<R: BufRead, W: Write>(
            &mut self,
            scanner: &mut Scanner<R>,
            writer: &mut W,
        ) -> io::Result<()> {
            local! {
                self.trace.count("route_rough");
                self.trace.count_by(
                    "rough_clearance_milli_sum",
                    (self.mean_static_clearance * 1_000.0).round() as i64,
                );
            }
            let mut departures: BinaryHeap<Reverse<(usize, usize)>> = BinaryHeap::new();
            for index in 0..self.M {
                let input_index: usize = scanner.next();
                let S: usize = scanner.next();
                let T: usize = scanner.next();
                let P: usize = scanner.next();
                let V: i64 = scanner.next();
                assert_eq!(input_index, index);
                self.groups[index].S = S;
                self.groups[index].T = T;
                self.groups[index].P = P;
                self.groups[index].V = V;
                self.release_departed(S, &mut departures);
                let duration = T - S;
                self.update_theta(S, duration);
                let q_req = self.quality_threshold(index, S, T, P);
                let priority = self.placement_priority(&self.groups[index], q_req);
                let mut required_fee_quality = q_req * self.compact_reference();
                let denominator = P as f64 * (duration as f64).powf(0.9);
                let raw_q = V as f64 / denominator;
                let quality_ratio = raw_q / q_req.max(0.12);
                let theoretical_L = self.minimum_polyomino_perimeter(P);
                let theoretical_fee = self.fee(V, P, theoretical_L);
                let max_fee_quality = theoretical_fee as f64 / denominator;
                if max_fee_quality + 1e-18 < required_fee_quality {
                    self.output_turn(writer, None, None, false)?;
                    local! {
                        self.trace.count("rejected");
                        self.trace.count("rough_quality_reject");
                    }
                    continue;
                }

                let mode = self.adaptive_mode(index, priority);
                let result_limit = if mode >= 2 {
                    8
                } else if mode == 1 {
                    6
                } else {
                    4
                };
                let mut regions = self.find_regions(P, result_limit, mode, Some(T), priority);
                local! {
                    self.trace.count("rough_region_search");
                    self.trace.count_by("rough_region_candidate", regions.len() as i64);
                }
                let mut initial_best_L = usize::MAX;
                let mut initial_best_fee = -1_i64;
                for region in &regions {
                    initial_best_L = initial_best_L.min(region.L);
                    initial_best_fee = initial_best_fee.max(self.fee(V, P, region.L));
                }
                let recoverable_shape_loss = if initial_best_fee < 0 {
                    theoretical_fee
                } else {
                    theoretical_fee - initial_best_fee
                };
                let any_initially_profitable = regions.iter().any(|region| {
                    self.fee(V, P, region.L) as f64 / denominator >= required_fee_quality
                });
                let need_peel = mode >= 0
                    && (initial_best_L == usize::MAX || initial_best_L > theoretical_L)
                    && (recoverable_shape_loss >= 10_000 || !any_initially_profitable);
                let peel_allowance = self.pace_target(index)
                    + scaled_time(0.014)
                    + scaled_time(0.018) * priority
                    + scaled_time(0.010) * self.terrain_roughness;
                if need_peel && self.elapsed() < scaled_time(1.82).min(peel_allowance) {
                    let mut boxes = if mode >= 2 {
                        12
                    } else if mode == 1 {
                        9
                    } else {
                        6
                    };
                    boxes +=
                        (7.0 * self.terrain_roughness * (0.35 + 0.65 * priority)).round() as usize;
                    boxes = boxes.clamp(6, 20);
                    let peeled = self.find_peel_regions(P, 4, boxes, Some(T), priority);
                    local! {
                        self.trace.count("rough_peel_attempt");
                        self.trace.count_by("rough_peel_region", peeled.len() as i64);
                    }
                    let mut have: HashSet<u64> = regions.iter().map(|region| region.hash).collect();
                    for region in peeled {
                        if have.insert(region.hash) {
                            regions.push(region);
                        }
                    }
                }
                if !regions.is_empty() {
                    let best_observed_L = regions.iter().map(|region| region.L).min().unwrap();
                    self.observe_compactness(4.0 * (P as f64).sqrt() / best_observed_L as f64);
                    required_fee_quality = q_req * self.compact_reference();
                }

                let mut chosen = None;
                let mut best_fee = -1_i64;
                let fees: Vec<i64> = regions
                    .iter()
                    .map(|region| self.fee(V, P, region.L))
                    .collect();
                for &fee in &fees {
                    if fee as f64 / denominator >= required_fee_quality {
                        best_fee = best_fee.max(fee);
                    }
                }
                if best_fee >= 0 {
                    let best_fee_quality = best_fee as f64 / denominator;
                    let surplus = best_fee_quality / required_fee_quality.max(1e-12);
                    let pressure = (q_req.max(0.45) / 0.45).ln() / 4.0_f64.ln();
                    let pressure = pressure.clamp(0.0, 1.0);
                    let mut sacrifice_fraction =
                        0.010 + 0.050 / (surplus * surplus) + 0.006 * pressure;
                    sacrifice_fraction = sacrifice_fraction.clamp(0.012, 0.065);
                    let terrain_mix = self.terrain_roughness * (0.50 - priority);
                    sacrifice_fraction *= (1.0 + 1.05 * terrain_mix).clamp(0.58, 1.45);
                    let future_fraction = ((self.M - index - 1) as f64 / self.M as f64)
                        .min((HORIZON - S) as f64 / HORIZON as f64);
                    let layout_scale = (0.10 + 1.50 * future_fraction).min(1.0);
                    sacrifice_fraction = (sacrifice_fraction * layout_scale).max(0.002);
                    let budget = 200_i64.max((best_fee as f64 * sacrifice_fraction).floor() as i64);
                    for region_index in 0..regions.len() {
                        if fees[region_index] < best_fee - budget
                            || fees[region_index] as f64 / denominator < required_fee_quality
                        {
                            continue;
                        }
                        if chosen.is_none_or(|old: usize| {
                            regions[region_index].layout < regions[old].layout
                                || (regions[region_index].layout == regions[old].layout
                                    && fees[region_index] > fees[old])
                        }) {
                            chosen = Some(region_index);
                        }
                    }
                }

                if let Some(chosen) = chosen {
                    let direct_fee = fees[chosen];
                    let compact_gap = if theoretical_fee > 0 {
                        1.0 - direct_fee as f64 / theoretical_fee as f64
                    } else {
                        0.0
                    };
                    let urgency = Self::clamp01(
                        (quality_ratio - 1.18) / 1.20 + 2.0 * (compact_gap - 0.04).max(0.0),
                    );
                    let upgrade_worth = compact_gap > 0.050
                        && quality_ratio > 1.24
                        && (self.R_milli <= 60 || compact_gap > 0.105);
                    let upgrade_allowance =
                        self.pace_target(index) + scaled_time(0.010) + scaled_time(0.012) * urgency;
                    if upgrade_worth && self.elapsed() < scaled_time(1.82).min(upgrade_allowance) {
                        self.set_operation_deadline(
                            index,
                            urgency,
                            scaled_time(0.012) + scaled_time(0.006) * self.terrain_roughness,
                        );
                        local! {
                            self.trace.count("rough_upgrade_attempt");
                        }
                        let upgrade = self.search_profitable_upgrade(
                            index,
                            required_fee_quality,
                            q_req,
                            priority,
                            urgency,
                        );
                        let margin = 200_i64.max((direct_fee as f64 * 0.004).floor() as i64);
                        if upgrade.net_gain != i64::MIN && upgrade.net_gain > direct_fee + margin {
                            self.apply_move_plan(index, &upgrade, &mut departures);
                            self.output_turn(writer, Some(&upgrade), None, true)?;
                            local! {
                                self.trace.count("accepted");
                                self.trace.count("rough_upgrade_placed");
                                self.trace.count_by("rough_moved_group", upgrade.ids.len() as i64);
                            }
                            continue;
                        }
                    }
                    let region = regions[chosen].clone();
                    self.apply_direct(index, &region, &mut departures);
                    self.output_turn(writer, None, Some(&region), true)?;
                    local! {
                        self.trace.count("accepted");
                        self.trace.count("rough_direct_placed");
                    }
                    continue;
                }

                if self.grass_count - self.occupied_cells >= P && self.elapsed() < scaled_time(1.82)
                {
                    let fee_headroom = max_fee_quality / required_fee_quality.max(1e-12);
                    let move_gate = 1.015
                        + 0.040 * self.terrain_roughness
                        + 0.45 * self.R_milli as f64 / 1_000.0;
                    if fee_headroom >= move_gate {
                        let urgency = Self::clamp01(
                            (fee_headroom - move_gate) / 0.72
                                + 0.22
                                    * self.terrain_roughness
                                    * Self::clamp01((P as f64 - 55.0) / 95.0),
                        );
                        self.set_operation_deadline(
                            index,
                            urgency,
                            scaled_time(0.020) + scaled_time(0.012) * self.terrain_roughness,
                        );
                        local! {
                            self.trace.count("rough_move_attempt");
                        }
                        let move_plan = self.search_moves(
                            index,
                            required_fee_quality,
                            q_req,
                            priority,
                            urgency,
                        );
                        if move_plan.net_gain != i64::MIN {
                            self.apply_move_plan(index, &move_plan, &mut departures);
                            self.output_turn(writer, Some(&move_plan), None, true)?;
                            local! {
                                self.trace.count("accepted");
                                self.trace.count("rough_move_placed");
                                self.trace.count_by(
                                    "rough_moved_group",
                                    move_plan.ids.len() as i64,
                                );
                            }
                            continue;
                        }
                    }
                }
                self.output_turn(writer, None, None, false)?;
                local! {
                    self.trace.count("rejected");
                    self.trace.count("rough_final_reject");
                }
            }
            local! {
                self.trace.add_time_ms(
                    "program_elapsed",
                    self.start_time.elapsed().as_secs_f64() * 1_000.0,
                );
                self.trace.summary();
            }
            Ok(())
        }
    }
}

fn parse_R_milli(text: &str) -> i64 {
    let mut parts = text.split('.');
    let integer: i64 = parts.next().unwrap().parse().unwrap();
    let fraction = parts.next().unwrap_or("");
    assert!(parts.next().is_none());
    let mut digits = fraction.as_bytes().to_vec();
    digits.resize(3, b'0');
    digits.truncate(3);
    let fractional: i64 = std::str::from_utf8(&digits).unwrap().parse().unwrap();
    integer * 1_000 + fractional
}

fn mean_static_clearance(board: &[String]) -> f64 {
    let N = board.len();
    let mut distance = vec![i32::MAX / 4; N * N];
    let mut queue = vec![0_usize; N * N];
    let mut head = 0;
    let mut tail = 0;
    let mut grass_count = 0;
    let directions = [(1_isize, 0_isize), (-1, 0), (0, 1), (0, -1)];
    for x in 0..N {
        for y in 0..N {
            if board[x].as_bytes()[y] != b'.' {
                continue;
            }
            grass_count += 1;
            let boundary = directions.iter().any(|&(dx, dy)| {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                nx < 0
                    || nx >= N as isize
                    || ny < 0
                    || ny >= N as isize
                    || board[nx as usize].as_bytes()[ny as usize] == b'#'
            });
            if boundary {
                let z = x * N + y;
                distance[z] = 1;
                queue[tail] = z;
                tail += 1;
            }
        }
    }
    while head < tail {
        let z = queue[head];
        head += 1;
        let x = z / N;
        let y = z % N;
        for &(dx, dy) in &directions {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0
                || nx >= N as isize
                || ny < 0
                || ny >= N as isize
                || board[nx as usize].as_bytes()[ny as usize] != b'.'
            {
                continue;
            }
            let w = nx as usize * N + ny as usize;
            if distance[w] > distance[z] + 1 {
                distance[w] = distance[z] + 1;
                queue[tail] = w;
                tail += 1;
            }
        }
    }
    if grass_count == 0 {
        return 1.0;
    }
    let sum: i64 = (0..N * N)
        .filter(|&z| board[z / N].as_bytes()[z % N] == b'.')
        .map(|z| distance[z] as i64)
        .sum();
    sum as f64 / grass_count as f64
}

fn main() -> io::Result<()> {
    // timer は入力・route判定・前計算も含めるため、main の開始直後に作る。
    let overall_start = Instant::now();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    let N: usize = scanner.next();
    let M: usize = scanner.next();
    let R_text: String = scanner.next();
    assert!(N <= MAX_N);
    let R_milli = parse_R_milli(&R_text);
    let board: Vec<String> = (0..N).map(|_| scanner.next()).collect();
    let clearance = mean_static_clearance(&board);
    const ROUGH_CLEARANCE_CUTOFF: f64 = 2.10;
    if clearance <= ROUGH_CLEARANCE_CUTOFF {
        let mut solver = rough::Solver::new(N, M, R_milli, board, overall_start);
        solver.run(&mut scanner, &mut writer)
    } else {
        let mut grass_rows = [0_u64; MAX_N];
        for (row_mask, row) in grass_rows.iter_mut().zip(&board) {
            for (column, byte) in row.bytes().enumerate() {
                if byte == b'.' {
                    *row_mask |= Solver::bit_at(column);
                }
            }
        }
        let timer = TimeKeeper::from_start(PROGRAM_TIME_LIMIT_SEC, overall_start);
        let mut solver = Solver::new(N, M, R_milli, grass_rows, timer);
        local! {
            solver.trace.count("route_smooth");
            solver.trace.count_by(
                "smooth_clearance_milli_sum",
                (clearance * 1_000.0).round() as i64,
            );
        }
        solver.run(&mut scanner, &mut writer)
    }
}
