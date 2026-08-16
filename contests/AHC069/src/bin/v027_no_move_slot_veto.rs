// v027_no_move_slot_veto.rs
#![allow(non_snake_case)] // 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。
// 中心アイデア: v024を保ち、同周長の全admission合法componentでslot損傷が
// 不可避な場合だけ、配置候補とrejectを共通rolloutで比較して受入をvetoする。

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
const NO_MOVE_CAPACITY_RATIO: f64 = 0.975;
const SLOT_VETO_ROBUST_PENALTY_FRACTION: f64 = 0.25;

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
    same_level_damage_unavoidable: bool,
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
            same_level_damage_unavoidable: false,
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

#[cfg(feature = "local")]
fn rounded_i64_safe(value: f64) -> i64 {
    if value.is_nan() {
        0
    } else if value >= i64::MAX as f64 {
        i64::MAX
    } else if value <= i64::MIN as f64 {
        i64::MIN
    } else {
        value.round() as i64
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
        q_value: f64,
        base_threshold: f64,
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
                let quality = q_value * compactness(P, perimeter);
                let has_admissible_zero_penalty = slot_best.iter().flatten().any(|candidate| {
                    let threshold =
                        base_threshold * self.component_threshold_factor(candidate.component_size);
                    (base_threshold == 0.0 || quality >= threshold) && candidate.slot_penalty == 0.0
                });
                let same_level_damage_unavoidable = !has_admissible_zero_penalty;
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
                for choice in &mut choices {
                    choice.same_level_damage_unavoidable = same_level_damage_unavoidable;
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
                        && !placement.same_level_damage_unavoidable
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

    /// 候補群を共通乱数で評価し、3本分の即時額と将来価値の合計を返す。
    fn evaluate_candidates_rollout_totals(
        &self,
        cands: &[RolloutCandidate],
        incoming_id: usize,
        now: usize,
        theta: f64,
        threshold: f64,
    ) -> Vec<f64> {
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
        totals
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
        let totals =
            self.evaluate_candidates_rollout_totals(cands, incoming_id, now, theta, threshold);
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

    /// 全候補がslotを損傷する場合だけ、同じ共通乱数で配置候補とrejectを一度に比較する。
    /// rawは実feeのみ、robust/fullはslot期待損失の0.25倍/1倍を後から差し引く。
    fn select_normal_with_slot_veto(
        &mut self,
        choices: &[Placement],
        incoming_id: usize,
        theta: f64,
        base_threshold: f64,
    ) -> (usize, bool) {
        debug_assert!(choices.len() >= 2);
        debug_assert!(choices.iter().all(|choice| choice.slot_penalty > 0.0));
        debug_assert!(choices[0].same_level_damage_unavoidable);
        let incoming = &self.groups[incoming_id];
        let mut cands = Vec::with_capacity(choices.len() + 1);
        for placement in choices {
            let cells = self.materialize(placement, incoming.P);
            let mut board = self.occupied_rows;
            for &cell in &cells {
                board[cell / self.N] |= Self::bit_at(cell % self.N);
            }
            cands.push(RolloutCandidate {
                board,
                immediate: (incoming.V as f64) * compactness(incoming.P, placement.perimeter),
                incoming_dep: Some((incoming.T, cells)),
            });
        }
        let reject_index = cands.len();
        cands.push(RolloutCandidate {
            board: self.occupied_rows,
            immediate: 0.0,
            incoming_dep: None,
        });

        let totals = local_time!(self.trace, "normal_rollout", {
            self.evaluate_candidates_rollout_totals(
                &cands,
                incoming_id,
                incoming.S,
                theta,
                base_threshold,
            )
        });
        let reject = totals[reject_index];
        let mut raw_best = 0;
        let mut full_best = 0;
        let mut max_robust =
            totals[0] - 3.0 * SLOT_VETO_ROBUST_PENALTY_FRACTION * choices[0].slot_penalty;
        let mut max_full = totals[0] - 3.0 * choices[0].slot_penalty;
        for i in 1..choices.len() {
            if totals[i] > totals[raw_best] {
                raw_best = i;
            }
            let robust =
                totals[i] - 3.0 * SLOT_VETO_ROBUST_PENALTY_FRACTION * choices[i].slot_penalty;
            if robust > max_robust {
                max_robust = robust;
            }
            let full = totals[i] - 3.0 * choices[i].slot_penalty;
            if full > max_full {
                max_full = full;
                full_best = i;
            }
        }
        let max_raw = totals[raw_best];
        let raw_accept_supported = max_raw > reject;
        let _full_flip = raw_accept_supported && max_full < reject;
        let _quarter_robust = raw_accept_supported && max_robust < reject;
        let veto = raw_accept_supported && max_robust < reject;

        local! {
            self.trace.count("normal_rollout_session");
            self.trace
                .count_by("normal_rollout_candidate_sum", choices.len() as i64);
            if full_best != 0 {
                self.trace.count("normal_rollout_flip");
            }
            self.trace.count("slot_reject_rollout_session");
            self.trace
                .count_by("slot_reject_raw_margin_sum", rounded_i64_safe(max_raw - reject));
            self.trace.count_by(
                "slot_reject_robust_margin_sum",
                rounded_i64_safe(max_robust - reject),
            );
            if raw_accept_supported {
                self.trace.count("slot_reject_raw_accept_supported");
            }
            if _full_flip {
                self.trace.count("slot_reject_full_flip");
            }
            if _quarter_robust {
                self.trace.count("slot_reject_quarter_robust");
            }
            if veto {
                self.trace.count("slot_reject_executed");
                let fee = (incoming.V as f64)
                    * compactness(incoming.P, choices[full_best].perimeter);
                self.trace
                    .count_by("slot_reject_fee_foregone", rounded_i64_safe(fee.max(0.0)));
                self.trace.count_by(
                    "slot_reject_penalty_avoided",
                    rounded_i64_safe(choices[full_best].slot_penalty.max(0.0)),
                );
            }
        }
        if veto {
            assert!(max_raw > reject && max_robust < reject);
        }
        (full_best, veto)
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
            let mut slot_vetoed = false;
            let passed_price_prefilter =
                base_threshold == 0.0 || q_value * optimistic_C >= 0.74 * base_threshold;
            if passed_price_prefilter {
                local! {
                    self.trace.count("normal_search");
                }
                let normal_choices = local_time!(self.trace, "normal_search", {
                    self.find_normal_placements(
                        id,
                        P,
                        V,
                        S,
                        T,
                        theta,
                        q_value,
                        base_threshold,
                        is_large_target,
                        fast_mode,
                    )
                });
                if let Some(mut choices) = normal_choices {
                    let current_perimeter = choices[0].perimeter;
                    let current_component_size = choices[0].component_size;
                    let slot_active = choices[0].slot_count > 0;
                    debug_assert!(
                        choices
                            .iter()
                            .all(|placement| placement.slot_count == choices[0].slot_count
                                && placement.same_level_damage_unavoidable
                                    == choices[0].same_level_damage_unavoidable)
                    );
                    let actual_threshold =
                        base_threshold * self.component_threshold_factor(current_component_size);
                    let quality = q_value * compactness(P, current_perimeter);
                    accepted = base_threshold == 0.0 || quality >= actual_threshold;
                    let base_accepted = accepted;

                    let winner = if accepted && choices.len() >= 2 {
                        let veto_scope = !is_large_target && !fast_mode && slot_active;
                        let all_choices_damaged =
                            veto_scope && choices.iter().all(|choice| choice.slot_penalty > 0.0);
                        local! {
                            if all_choices_damaged {
                                self.trace.count("slot_reject_all_choices_damaged");
                            }
                        }
                        let global_same_level_damaged =
                            all_choices_damaged && choices[0].same_level_damage_unavoidable;
                        local! {
                            if global_same_level_damaged {
                                self.trace.count("slot_reject_global_same_level_damaged");
                            }
                        }
                        if global_same_level_damaged {
                            let (winner, veto) = self.select_normal_with_slot_veto(
                                &choices,
                                id,
                                theta,
                                base_threshold,
                            );
                            slot_vetoed = veto;
                            winner
                        } else {
                            self.select_normal_by_rollout(&choices, id, theta, base_threshold)
                        }
                    } else {
                        0
                    };
                    local! {
                        if slot_active && base_accepted && choices.len() >= 2 {
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
                    if slot_vetoed {
                        accepted = false;
                    } else {
                        let placement = choices.swap_remove(winner);
                        debug_assert_eq!(placement.perimeter, current_perimeter);
                        debug_assert_eq!(placement.component_size, current_component_size);
                        normal = Some(placement);
                    }

                    local! {
                        if let Some(placement) = &normal {
                            if placement.shape_index != usize::MAX {
                                let shape = &self.shapes_by_p[P][placement.shape_index];
                                if !shape.baseline_kept {
                                    self.trace.count("extra_shape_chosen");
                                }
                            }
                        }
                    }
                    if !base_accepted {
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
                    if normal.is_none() && !slot_vetoed {
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
