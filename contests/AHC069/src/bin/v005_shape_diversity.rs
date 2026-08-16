// v005_shape_diversity.rs
#![allow(non_snake_case)] // 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。

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

// 参照 C++ の各打ち切り時刻を、基準時間 1.90 秒に対する割合として保持する。
const FAST_MODE_RATIO: f64 = 148.0 / 190.0;
const RELOCATION_START_LIMIT_RATIO: f64 = 152.0 / 190.0;
const TARGET_SCAN_LIMIT_RATIO: f64 = 160.0 / 190.0;
const GROWTH_LIMIT_RATIO: f64 = 162.0 / 190.0;
const REPACK_LIMIT_RATIO: f64 = 166.0 / 190.0;

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
    baseline_kept: bool,
    left: Vec<usize>,
    len: Vec<usize>,
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
    #[cfg(feature = "local")]
    cost: i64,
    rank: f64,
    #[cfg(feature = "local")]
    surplus: f64,
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
                        baseline_kept: false,
                        left: vec![0; h],
                        len: vec![w; h],
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
                        baseline_kept: false,
                        left: vec![0; h],
                        len: vec![w; h],
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
                            baseline_kept: false,
                            left: vec![0; h],
                            len: vec![w; h],
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
        pushed: &mut [bool],
        frontier: &mut BinaryHeap<Reverse<(i64, usize)>>,
    ) {
        if pushed[id] || selected[id] || info.component[id] != component {
            return;
        }
        pushed[id] = true;
        let r = id / self.N;
        let c = id % self.N;
        let ring = r.abs_diff(seed_r).max(c.abs_diff(seed_c));
        let manhattan = r.abs_diff(seed_r) + c.abs_diff(seed_c);
        let attraction = (weights.cell[id] * 30.0).round() as i64;
        let key = 100_000 * (ring as i64) + 1_000 * (manhattan as i64) - attraction;
        frontier.push(Reverse((key, id)));
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
            let mut pushed = vec![false; self.N * self.N];
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
                        &mut pushed,
                        &mut frontier,
                    );
                }
            }
            while region.len() < P {
                let Some(Reverse((_, id))) = frontier.pop() else {
                    break;
                };
                if selected[id] {
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
                            &mut pushed,
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
        if best.is_some() {
            local! {
                self.trace.count("growth_placement_success");
            }
        }
        best
    }

    fn find_normal_placement(
        &mut self,
        P: usize,
        incoming_T: usize,
        theta: f64,
        fast_mode: bool,
    ) -> Option<Placement> {
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
            let mut best: Option<Placement> = None;
            for mut candidate in candidates {
                let cells = self.materialize(&candidate, P);
                let component_id = info.component[cells[0]];
                candidate.component_size = if component_id >= 0 {
                    info.sizes[component_id as usize]
                } else {
                    0
                };
                if fast_mode {
                    candidate.final_score = candidate.cheap_score;
                } else {
                    let mut next = occ;
                    for id in cells {
                        next[id / self.N] |= Self::bit_at(id % self.N);
                    }
                    let delta = self.fragment_metric(&next) - info.metric;
                    local! {
                        self.trace.count("fragment_evaluated");
                    }
                    candidate.final_score = candidate.cheap_score - 1.4 * delta;
                }
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.final_score > current.final_score)
                {
                    best = Some(candidate);
                }
            }
            if let Some(candidate) = &best {
                let shape = &self.shapes_by_p[P][candidate.shape_index];
                if !shape.baseline_kept {
                    local! {
                        self.trace.count("extra_shape_chosen");
                    }
                }
            }
            return best;
        }

        if fast_mode || self.timer.reached(GROWTH_LIMIT_RATIO) {
            return None;
        }
        self.growth_placement(P, &occ, &info, &weights, usize::MAX, 44)
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
    ) -> Vec<TargetOption> {
        let empty_occ = [0_u64; MAX_N];
        let runs = self.build_run_table(&empty_occ);
        let grass_info = self.compute_free_info(&empty_occ, false);
        let permanent_weights = self.build_weight_data(&empty_occ, &grass_info, 0, 5_000.0, false);
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
                        ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
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
                                if !self.groups[owner].active || self.groups[owner].move_count >= 2
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
                        let component_id = grass_info.component[first_cell];
                        let component_size = if component_id >= 0 {
                            grass_info.sizes[component_id as usize]
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
                        if surplus <= 1.12 * (cost as f64) {
                            continue;
                        }

                        let mut contact = 0.0;
                        for rr in 0..shape.h {
                            let begin = y + shape.left[rr];
                            let end = begin + shape.len[rr];
                            contact += permanent_weights.prefix[x + rr][end]
                                - permanent_weights.prefix[x + rr][begin];
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
                                #[cfg(feature = "local")]
                                cost,
                                #[cfg(feature = "local")]
                                surplus,
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

    fn repack_blockers(
        &mut self,
        blocker_ids: &[usize],
        target_cells: &[usize],
    ) -> Option<Vec<(usize, Placement)>> {
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
                for perimeter in (minimum_perimeter(P)..=worst_perimeter).step_by(2) {
                    candidates =
                        self.scan_regular_level(P, &runs, &weights, perimeter, BRANCHES, 8);
                    if !candidates.is_empty() {
                        break;
                    }
                }
                if candidates.is_empty()
                    && !self.timer.reached(GROWTH_LIMIT_RATIO)
                    && let Some(candidate) =
                        self.growth_placement(P, &state.occ, &info, &weights, worst_perimeter, 20)
                {
                    candidates.push(candidate);
                }
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
        for (i, state) in beam.iter().enumerate() {
            let score = state.score - 1.15 * self.fragment_metric(&state.occ);
            local! {
                self.trace.count("fragment_evaluated");
            }
            if score > best_score {
                best_score = score;
                best_index = i;
            }
        }
        local! {
            self.trace.count("repack_success");
        }
        Some(beam.swap_remove(best_index).placements)
    }

    fn attempt_relocation(
        &mut self,
        incoming: &Group,
        q_value: f64,
        base_threshold: f64,
    ) -> MovePlan {
        if self.timer.reached(RELOCATION_START_LIMIT_RATIO) {
            return MovePlan::default();
        }
        let max_compactness = compactness(incoming.P, minimum_perimeter(incoming.P));
        if q_value * max_compactness <= base_threshold * 0.92 {
            return MovePlan::default();
        }
        let blocker_limit = if self.R_milli <= 15 { 4 } else { 3 };
        let options =
            self.collect_relocation_targets(incoming, q_value, base_threshold, blocker_limit);
        for (attempt, option) in options.into_iter().enumerate() {
            if attempt >= 12 || self.timer.reached(REPACK_LIMIT_RATIO) {
                break;
            }
            let target_cells = self.materialize(&option.placement, incoming.P);
            let Some(repacked) = self.repack_blockers(&option.blockers, &target_cells) else {
                continue;
            };
            let mut incoming_placement = option.placement;
            incoming_placement.explicit_cells = target_cells;
            local! {
                self.trace.count("relocation_success");
                self.trace
                    .count_by("moved_groups", repacked.len() as i64);
                self.trace.count_by("move_cost", option.cost);
                self.trace
                    .count_by("relocation_surplus", option.surplus.round() as i64);
            }
            return MovePlan {
                ok: true,
                incoming: incoming_placement,
                moved: repacked,
            };
        }
        MovePlan::default()
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
        if placement.explicit_cells.is_empty()
            && !self.shapes_by_p[P][placement.shape_index].baseline_kept
        {
            local! {
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
            let mut move_plan = MovePlan::default();
            let passed_price_prefilter =
                base_threshold == 0.0 || q_value * optimistic_C >= 0.74 * base_threshold;
            if passed_price_prefilter {
                local! {
                    self.trace.count("normal_search");
                }
                normal = local_time!(self.trace, "normal_search", {
                    self.find_normal_placement(P, T, theta, fast_mode)
                });
                if let Some(placement) = &normal {
                    let actual_threshold =
                        base_threshold * self.component_threshold_factor(placement.component_size);
                    let quality = q_value * compactness(P, placement.perimeter);
                    accepted = base_threshold == 0.0 || quality >= actual_threshold;
                    if !accepted {
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

            if !accepted
                && !fast_mode
                && normal
                    .as_ref()
                    .is_none_or(|placement| placement.perimeter > minimum_perimeter(P))
                && q_value * optimistic_C >= 0.88 * base_threshold
            {
                local! {
                    self.trace.count("relocation_attempt");
                }
                let incoming = self.groups[id].clone();
                move_plan = local_time!(self.trace, "relocation", {
                    self.attempt_relocation(&incoming, q_value, base_threshold)
                });
                accepted = move_plan.ok;
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

fn main() -> io::Result<()> {
    // timer は入力や前計算も含めるため、main の開始直後に作る。
    let timer = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut scanner = Scanner::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    let N: usize = scanner.next();
    let M: usize = scanner.next();
    let R_text: String = scanner.next();
    assert!(N <= MAX_N);
    let R_milli = parse_R_milli(&R_text);
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

    let mut solver = Solver::new(N, M, R_milli, grass_rows, timer);
    solver.run(&mut scanner, &mut writer)
}
