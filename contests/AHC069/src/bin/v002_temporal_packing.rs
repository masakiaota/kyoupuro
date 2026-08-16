// v002_temporal_packing.rs
#![allow(non_snake_case)] // 問題文の `N`, `M`, `S`, `T`, `P`, `V` を対応づけたまま使う。

use statrs::distribution::{ContinuousCDF, Normal};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::io::{self, BufRead, BufWriter, Write};
use std::time::Instant;

const HORIZON: usize = 100_000;
const MAX_P: usize = 150;
const THETA_STEP: usize = 100;
// 同じ最小周長でも外接矩形が大きい形は将来の空きを壊しやすい。
// balanced / 小外接矩形の順に絞り、時間を劣った形の列挙へ費やさない。
const MAX_TEMPLATES: usize = 32;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

// セル時間で重みづけした理論分布について、
// log((fee / (P D)) * theta^0.1) を正規近似したときの定数である。
// P の離散分布、最小周長形状の compactness、D/theta の Gamma(2, 1)
// 分布、2^gauss(0, 0.8) をすべて含む。
const LOG_SCALED_DENSITY_MEAN: f64 = -0.068_674_780_3;
const LOG_SCALED_DENSITY_SD: f64 = 0.560_644_008_5;
const EXPECTED_P: f64 = 59.497_449_995_6;

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
        let start = std::time::Instant::now();
        let result = { $body };
        $trace.add_time_ms($key, start.elapsed().as_secs_f64() * 1000.0);
        result
    }};
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        assert!(time_limit_sec > 0.0);
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline]
    fn time_limit_sec(&self) -> f64 {
        self.time_limit_sec
    }
}

#[cfg(not(feature = "local"))]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
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

struct ThetaModel {
    theta: f64,
    log_p0: f64,
    log_band: f64,
    log_norm: f64,
    log_survival_after_S: Vec<f32>,
}

impl ThetaModel {
    fn new(theta: f64) -> Self {
        // l は指数分布の標本を最近接整数へ丸め、l < 100000 の条件で
        // 再生成される。以下はその正規化済み確率質量である。
        let norm = -(-((HORIZON as f64) - 0.5) / theta).exp_m1();
        let p0 = -(-0.5 / theta).exp_m1() / norm;
        let band = -(-1.0 / theta).exp_m1();
        let r = (-1.0 / theta).exp();

        // f_S(s) = sum_{l <= H-1-s} p(l)/(H-l)。prefix を作れば、
        // 到着時刻の右側確率 P(S > s) を全 s について線形時間で得られる。
        let mut prefix = vec![0.0_f64; HORIZON];
        let mut prefix_sum = 0.0;
        let mut p = p0;
        for (l, slot) in prefix.iter_mut().enumerate() {
            if l == 1 {
                p = (-0.5 / theta).exp() * band / norm;
            } else if l >= 2 {
                p *= r;
            }
            prefix_sum += p / ((HORIZON - l) as f64);
            *slot = prefix_sum;
        }

        let mut log_survival_after_S = vec![f32::NEG_INFINITY; HORIZON];
        let mut cdf = 0.0_f64;
        for s in 0..HORIZON {
            cdf += prefix[HORIZON - 1 - s];
            let survival = (1.0 - cdf).max(0.0);
            if survival > 0.0 {
                log_survival_after_S[s] = survival.ln() as f32;
            }
        }

        Self {
            theta,
            log_p0: p0.ln(),
            log_band: band.ln(),
            log_norm: norm.ln(),
            log_survival_after_S,
        }
    }

    fn log_duration_probability(&self, l: usize) -> f64 {
        if l == 0 {
            self.log_p0
        } else {
            -((l as f64) - 0.5) / self.theta + self.log_band - self.log_norm
        }
    }
}

struct ThetaEstimator {
    M: usize,
    models: Vec<ThetaModel>,
    duration_log_likelihood: Vec<f64>,
}

impl ThetaEstimator {
    fn new(M: usize) -> Self {
        let models: Vec<_> = (2_000..=8_000)
            .step_by(THETA_STEP)
            .map(|theta| ThetaModel::new(theta as f64))
            .collect();
        let duration_log_likelihood = vec![0.0; models.len()];
        Self {
            M,
            models,
            duration_log_likelihood,
        }
    }

    fn observe(&mut self, i: usize, S: usize, D: usize) -> f64 {
        let l = D - 1;
        let remaining = self.M - i - 1;
        let mut max_log_weight = f64::NEG_INFINITY;
        let mut log_weights = Vec::with_capacity(self.models.len());

        for (model, duration_like) in self
            .models
            .iter()
            .zip(self.duration_log_likelihood.iter_mut())
        {
            *duration_like += model.log_duration_probability(l);
            // 入力は S 昇順である。観測済み prefix の尤度には、未観測の
            // remaining 組がすべて現在の S より後に来る確率も必要になる。
            let log_weight = if remaining == 0 {
                *duration_like
            } else {
                *duration_like + (remaining as f64) * (model.log_survival_after_S[S] as f64)
            };
            max_log_weight = max_log_weight.max(log_weight);
            log_weights.push(log_weight);
        }

        let mut weight_sum = 0.0;
        let mut weighted_theta_sum = 0.0;
        for (model, &log_weight) in self.models.iter().zip(&log_weights) {
            let weight = (log_weight - max_log_weight).exp();
            weight_sum += weight;
            weighted_theta_sum += weight * model.theta;
        }
        weighted_theta_sum / weight_sum
    }
}

#[derive(Clone)]
struct Template {
    height: usize,
    width: usize,
    row_masks: Vec<u64>,
    cells: Vec<(usize, usize)>,
}

fn minimum_perimeter(P: usize) -> usize {
    2 * (2.0 * (P as f64).sqrt()).ceil() as usize
}

fn perimeter(cells: &[(usize, usize)]) -> usize {
    let set: HashSet<_> = cells.iter().copied().collect();
    let mut L = 0;
    for &(x, y) in cells {
        for (dx, dy) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
            let nx = (x as i32) + dx;
            let ny = (y as i32) + dy;
            if nx < 0 || ny < 0 || !set.contains(&(nx as usize, ny as usize)) {
                L += 1;
            }
        }
    }
    L
}

fn transformed(cells: &[(usize, usize)], kind: usize) -> Vec<(usize, usize)> {
    let mut transformed: Vec<(i32, i32)> = cells
        .iter()
        .map(|&(x, y)| {
            let x = x as i32;
            let y = y as i32;
            match kind {
                0 => (x, y),
                1 => (x, -y),
                2 => (-x, y),
                3 => (-x, -y),
                4 => (y, x),
                5 => (y, -x),
                6 => (-y, x),
                7 => (-y, -x),
                _ => unreachable!(),
            }
        })
        .collect();
    let min_x = transformed.iter().map(|p| p.0).min().unwrap();
    let min_y = transformed.iter().map(|p| p.1).min().unwrap();
    let mut normalized: Vec<_> = transformed
        .drain(..)
        .map(|(x, y)| ((x - min_x) as usize, (y - min_y) as usize))
        .collect();
    normalized.sort_unstable();
    normalized
}

fn generate_templates(P: usize, N: usize) -> Vec<Template> {
    let target_L = minimum_perimeter(P);
    let mut unique = HashSet::<Vec<(usize, usize)>>::new();

    // h 行に floor(P/h) 個ずつ置き、余りを隣接列に連続して付ける。
    // 最小周長を達成する h だけを残す。
    for height in 1..=P.min(N) {
        let base_width = P / height;
        let remainder = P % height;
        let width = base_width + usize::from(remainder > 0);
        if width > N {
            continue;
        }

        let starts = if remainder == 0 {
            vec![0]
        } else {
            let last = height - remainder;
            let mut starts = vec![0, last / 2, last];
            starts.sort_unstable();
            starts.dedup();
            starts
        };

        for start in starts {
            let mut cells = Vec::with_capacity(P);
            for x in 0..height {
                for y in 0..base_width {
                    cells.push((x, y));
                }
                if remainder > 0 && start <= x && x < start + remainder {
                    cells.push((x, base_width));
                }
            }
            if perimeter(&cells) != target_L {
                continue;
            }
            for kind in 0..8 {
                let variant = transformed(&cells, kind);
                let variant_height = variant.iter().map(|p| p.0).max().unwrap() + 1;
                let variant_width = variant.iter().map(|p| p.1).max().unwrap() + 1;
                if variant_height <= N && variant_width <= N {
                    unique.insert(variant);
                }
            }
        }
    }

    let mut templates: Vec<_> = unique
        .into_iter()
        .map(|cells| {
            let height = cells.iter().map(|p| p.0).max().unwrap() + 1;
            let width = cells.iter().map(|p| p.1).max().unwrap() + 1;
            let mut row_masks = vec![0_u64; height];
            for &(x, y) in &cells {
                row_masks[x] |= 1_u64 << y;
            }
            Template {
                height,
                width,
                row_masks,
                cells,
            }
        })
        .collect();

    templates.sort_by(|a, b| {
        a.height
            .abs_diff(a.width)
            .cmp(&b.height.abs_diff(b.width))
            .then_with(|| (a.height * a.width).cmp(&(b.height * b.width)))
            .then_with(|| a.height.cmp(&b.height))
            .then_with(|| a.width.cmp(&b.width))
            .then_with(|| a.cells.cmp(&b.cells))
    });
    templates.truncate(MAX_TEMPLATES);
    assert!(!templates.is_empty());
    templates
}

fn grass_components(grass: &[Vec<bool>]) -> (Vec<Vec<usize>>, Vec<usize>) {
    let N = grass.len();
    let mut component_of = vec![vec![usize::MAX; N]; N];
    let mut sizes = Vec::new();
    for sx in 0..N {
        for sy in 0..N {
            if !grass[sx][sy] || component_of[sx][sy] != usize::MAX {
                continue;
            }
            let component = sizes.len();
            let mut queue = VecDeque::new();
            queue.push_back((sx, sy));
            component_of[sx][sy] = component;
            let mut size = 0;
            while let Some((x, y)) = queue.pop_front() {
                size += 1;
                for (dx, dy) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = (x as i32) + dx;
                    let ny = (y as i32) + dy;
                    if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                        continue;
                    }
                    let nx = nx as usize;
                    let ny = ny as usize;
                    if grass[nx][ny] && component_of[nx][ny] == usize::MAX {
                        component_of[nx][ny] = component;
                        queue.push_back((nx, ny));
                    }
                }
            }
            sizes.push(size);
        }
    }
    (component_of, sizes)
}

fn contact_scores(
    N: usize,
    grass: &[Vec<bool>],
    release_at: &[Vec<Option<usize>>],
    T: usize,
    theta_estimate: f64,
    template: &Template,
    top: usize,
    left: usize,
) -> (usize, f64, usize) {
    let mut raw_contact = 0;
    let mut temporal_contact = 0.0;
    let mut temporal_contact_edges = 0;
    for &(dx, dy) in &template.cells {
        let x = top + dx;
        let y = left + dy;
        for (sx, sy) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
            let nx = (x as i32) + sx;
            let ny = (y as i32) + sy;
            if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                raw_contact += 1;
                temporal_contact += 1.0;
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if !grass[nx][ny] {
                raw_contact += 1;
                temporal_contact += 1.0;
            } else if let Some(neighbor_T) = release_at[nx][ny] {
                raw_contact += 1;
                temporal_contact_edges += 1;
                // 同時期に空く境界は一つの空き領域へ戻りやすい。theta を時刻差の
                // 自然な尺度として使い、遠い退去時刻との接触は低く評価する。
                let time_gap = T.abs_diff(neighbor_T) as f64;
                temporal_contact += (-time_gap / theta_estimate).exp();
            }
        }
    }
    (raw_contact, temporal_contact, temporal_contact_edges)
}

struct PlacementSearch {
    cells: Option<Vec<(usize, usize)>>,
    checked: usize,
    feasible: usize,
    temporal_contact_edges: usize,
    temporal_choice_changed: bool,
    time_cutoff: bool,
}

fn finish_search(
    temporal_best: Option<(f64, Vec<(usize, usize)>)>,
    raw_best: Option<(usize, Vec<(usize, usize)>)>,
    checked: usize,
    feasible: usize,
    temporal_contact_edges: usize,
    time_cutoff: bool,
) -> PlacementSearch {
    let temporal_choice_changed = match (&temporal_best, &raw_best) {
        (Some((_, temporal_cells)), Some((_, raw_cells))) => temporal_cells != raw_cells,
        _ => false,
    };
    PlacementSearch {
        cells: temporal_best.map(|(_, cells)| cells),
        checked,
        feasible,
        temporal_contact_edges,
        temporal_choice_changed,
        time_cutoff,
    }
}

fn find_placement(
    N: usize,
    P: usize,
    templates: &[Template],
    blocked_rows: &[u64],
    grass: &[Vec<bool>],
    release_at: &[Vec<Option<usize>>],
    T: usize,
    theta_estimate: f64,
    component_of: &[Vec<usize>],
    component_free: &[usize],
    time_keeper: &TimeKeeper,
    deadline_sec: f64,
) -> PlacementSearch {
    // 小さい芝生連結成分を先に使い切り、大成分の融通を温存する。
    let mut components: Vec<_> = (0..component_free.len())
        .filter(|&component| component_free[component] >= P)
        .collect();
    components.sort_by_key(|&component| component_free[component]);

    let mut checked = 0;
    let mut feasible_total = 0;
    let mut temporal_contact_edges_total = 0;
    for component in components {
        let mut temporal_best: Option<(f64, Vec<(usize, usize)>)> = None;
        let mut raw_best: Option<(usize, Vec<(usize, usize)>)> = None;
        for template in templates {
            let representative = template.cells[0];
            for top in 0..=N - template.height {
                for left in 0..=N - template.width {
                    let rx = top + representative.0;
                    let ry = left + representative.1;
                    if component_of[rx][ry] != component {
                        continue;
                    }
                    checked += 1;
                    if (checked & 127) == 0 && time_keeper.exact_elapsed_sec() >= deadline_sec {
                        return finish_search(
                            temporal_best,
                            raw_best,
                            checked,
                            feasible_total,
                            temporal_contact_edges_total,
                            true,
                        );
                    }
                    let fits = template
                        .row_masks
                        .iter()
                        .enumerate()
                        .all(|(dx, &mask)| (blocked_rows[top + dx] & (mask << left)) == 0);
                    if !fits {
                        continue;
                    }

                    feasible_total += 1;
                    let (raw_score, temporal_score, temporal_edges) = contact_scores(
                        N,
                        grass,
                        release_at,
                        T,
                        theta_estimate,
                        template,
                        top,
                        left,
                    );
                    temporal_contact_edges_total += temporal_edges;
                    let improves_temporal = temporal_best
                        .as_ref()
                        .is_none_or(|(best_score, _)| temporal_score > *best_score);
                    let improves_raw = raw_best
                        .as_ref()
                        .is_none_or(|(best_score, _)| raw_score > *best_score);
                    if improves_temporal || improves_raw {
                        let cells: Vec<_> = template
                            .cells
                            .iter()
                            .map(|&(dx, dy)| (top + dx, left + dy))
                            .collect();
                        if improves_temporal {
                            temporal_best = Some((temporal_score, cells.clone()));
                        }
                        if improves_raw {
                            raw_best = Some((raw_score, cells));
                        }
                    }
                }
            }
        }
        if temporal_best.is_some() {
            return finish_search(
                temporal_best,
                raw_best,
                checked,
                feasible_total,
                temporal_contact_edges_total,
                false,
            );
        }
    }
    PlacementSearch {
        cells: None,
        checked,
        feasible: feasible_total,
        temporal_contact_edges: temporal_contact_edges_total,
        temporal_choice_changed: false,
        time_cutoff: false,
    }
}

struct ActiveGroup {
    cells: Vec<(usize, usize)>,
}

fn main() {
    // v000 と同じく、入力待ち・前処理を含めて main 開始直後から計時する。
    let time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let N: usize = scanner.next();
    let M: usize = scanner.next();
    let R: f64 = scanner.next();
    let _ = R; // v002 は移動を行わないため、移動費係数は意思決定に使わない。

    let mut grass = vec![vec![false; N]; N];
    let mut blocked_rows = vec![0_u64; N];
    let mut grass_count = 0;
    for x in 0..N {
        let row: String = scanner.next();
        for (y, c) in row.bytes().enumerate() {
            if c == b'.' {
                grass[x][y] = true;
                grass_count += 1;
            } else {
                blocked_rows[x] |= 1_u64 << y;
            }
        }
    }

    let (component_of, mut component_free) = grass_components(&grass);
    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();
    let mut theta_estimator = local_time!(trace, "theta_init", { ThetaEstimator::new(M) });
    let placement_budget_start_sec = time_keeper.exact_elapsed_sec();
    let placement_budget_sec = (time_keeper.time_limit_sec() - placement_budget_start_sec).max(0.0);
    let standard_normal = Normal::new(0.0, 1.0).unwrap();
    let mut templates_by_P: Vec<Option<Vec<Template>>> = (0..=MAX_P).map(|_| None).collect();
    let mut active: Vec<Option<ActiveGroup>> = (0..M).map(|_| None).collect();
    let mut departures = BinaryHeap::<Reverse<(usize, usize)>>::new();
    let mut release_at = vec![vec![None; N]; N];
    #[cfg(feature = "local")]
    let mut theta_estimate_final = 0.0_f64;

    for turn in 0..M {
        let i: usize = scanner.next();
        let S: usize = scanner.next();
        let T: usize = scanner.next();
        let P: usize = scanner.next();
        let V: i64 = scanner.next();
        assert_eq!(i, turn);

        while let Some(&Reverse((departure_T, j))) = departures.peek() {
            if departure_T >= S {
                break;
            }
            departures.pop();
            let departed = active[j].take().unwrap();
            for &(x, y) in &departed.cells {
                blocked_rows[x] &= !(1_u64 << y);
                release_at[x][y] = None;
            }
            let component = component_of[departed.cells[0].0][departed.cells[0].1];
            component_free[component] += departed.cells.len();
            local! {
                trace.count("departed");
            }
        }

        let D = T - S;
        let theta_estimate = theta_estimator.observe(i, S, D);
        local! {
            trace.count("theta_update");
            theta_estimate_final = theta_estimate;
        }

        // 出力の第1段階。v002 でも移動を一切行わない。
        writeln!(out, "0").unwrap();

        // M 組が長さ HORIZON の時間軸に一様到着するので、時点当たりの
        // 提示セル数には到着率 M/HORIZON が掛かる。
        let expected_offered_cells =
            (M as f64 / HORIZON as f64) * EXPECTED_P * (theta_estimate + 1.0);
        let capacity_fraction = (grass_count as f64 / expected_offered_cells).min(1.0);
        let threshold = if capacity_fraction >= 1.0 {
            local! {
                trace.count("light_load_turn");
            }
            0.0
        } else {
            local! {
                trace.count("overload_turn");
            }
            let lower_tail = (1.0 - capacity_fraction).clamp(1.0e-9, 1.0 - 1.0e-9);
            let z = standard_normal.inverse_cdf(lower_tail);
            (LOG_SCALED_DENSITY_MEAN + LOG_SCALED_DENSITY_SD * z).exp() / theta_estimate.powf(0.1)
        };

        let L = minimum_perimeter(P);
        let compactness = 4.0 * (P as f64).sqrt() / (L as f64);
        let ideal_fee = ((V as f64) * compactness).round();
        let density = ideal_fee / ((P * D) as f64);
        if density < threshold {
            writeln!(out, "No").unwrap();
            out.flush().unwrap();
            local! {
                trace.count("price_reject");
            }
            continue;
        }
        local! {
            trace.count("price_accept");
        }

        let was_missing = templates_by_P[P].is_none();
        if was_missing {
            templates_by_P[P] = Some(generate_templates(P, N));
            local! {
                trace.count("template_size_generated");
                trace.count_by(
                    "template_variants_generated",
                    templates_by_P[P].as_ref().unwrap().len() as i64,
                );
            }
        }
        let search = local_time!(trace, "placement", {
            let deadline_sec =
                placement_budget_start_sec + placement_budget_sec * ((turn + 1) as f64 / M as f64);
            find_placement(
                N,
                P,
                templates_by_P[P].as_ref().unwrap(),
                &blocked_rows,
                &grass,
                &release_at,
                T,
                theta_estimate,
                &component_of,
                &component_free,
                &time_keeper,
                deadline_sec,
            )
        });
        local! {
            trace.count_by("placement_checked", search.checked as i64);
            trace.count_by("placement_feasible", search.feasible as i64);
            trace.count_by(
                "temporal_contact_edges",
                search.temporal_contact_edges as i64,
            );
            if search.temporal_choice_changed {
                trace.count("temporal_choice_changed");
            }
            if search.time_cutoff {
                trace.count("search_time_cutoff");
            }
        }
        let _ = (
            search.checked,
            search.feasible,
            search.temporal_contact_edges,
            search.temporal_choice_changed,
            search.time_cutoff,
        );

        if let Some(cells) = search.cells {
            writeln!(out, "Yes").unwrap();
            for &(x, y) in &cells {
                writeln!(out, "{} {}", x, y).unwrap();
                blocked_rows[x] |= 1_u64 << y;
                release_at[x][y] = Some(T);
            }
            let component = component_of[cells[0].0][cells[0].1];
            component_free[component] -= cells.len();
            active[i] = Some(ActiveGroup { cells });
            departures.push(Reverse((T, i)));
            local! {
                trace.count("placed");
            }
        } else {
            writeln!(out, "No").unwrap();
            local! {
                trace.count("geometry_reject");
            }
        }
        out.flush().unwrap();
    }

    local! {
        trace.count_by("theta_estimate_final", theta_estimate_final.round() as i64);
        trace.summary();
    }
}
