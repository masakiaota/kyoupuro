// v003_bayes_gibbs_lcb_frontloaded.rs
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const N: usize = 30;
const Q: usize = 1000;
const VERTEX_COUNT: usize = N * N;
const SEG_COUNT: usize = 60;
const EDGE_POS: usize = 29;
const PREFIX_LEN: usize = EDGE_POS + 1;
const Z_MIN: usize = 1;
const Z_MAX: usize = 28;

const MU0: f64 = 5000.0;
const TAU2: f64 = 27010000.0 / 9.0;
const PRIOR_LAMBDA: f64 = 1.0 / TAU2;
const PRIOR_ETA: f64 = MU0 / TAU2;
const D2_OVER3_PRIOR: f64 = 4210000.0 / 9.0;
const TIME_LIMIT_SEC: f64 = 1.85;

#[derive(Clone)]
struct Observation {
    observed: f64,
    model_total: f64,
    prefix: [[u8; PREFIX_LEN]; SEG_COUNT],
    inv_sigma2: [f64; SEG_COUNT],
}

#[derive(Clone, Copy)]
struct SegmentState {
    z: usize,
    x: [f64; 2],
    mean: [f64; 2],
    var: [f64; 2],
}

impl SegmentState {
    fn new() -> Self {
        Self {
            z: 14,
            x: [MU0, MU0],
            mean: [MU0, MU0],
            var: [TAU2, TAU2],
        }
    }
}

#[derive(Clone, Copy)]
struct Posterior2 {
    a: f64,
    b: f64,
    c: f64,
    eta0: f64,
    eta1: f64,
    log_weight: f64,
}

#[derive(Clone, Copy, Default)]
struct PrecisionCache {
    a: f64,
    b: f64,
    c: f64,
    inv_det: f64,
    log_det_term: f64,
}

impl PrecisionCache {
    fn new() -> Self {
        let mut cache = Self::default();
        cache.recompute();
        cache
    }

    fn recompute(&mut self) {
        let a = PRIOR_LAMBDA + self.a;
        let b = self.b;
        let c = PRIOR_LAMBDA + self.c;
        let det = (a * c - b * b).max(1e-300);
        self.inv_det = 1.0 / det;
        self.log_det_term = -0.5 * det.ln();
    }
}

impl Posterior2 {
    fn prior() -> Self {
        Self {
            a: PRIOR_LAMBDA,
            b: 0.0,
            c: PRIOR_LAMBDA,
            eta0: PRIOR_ETA,
            eta1: PRIOR_ETA,
            log_weight: 0.0,
        }
    }

    fn finish_log_weight(&mut self, cache: PrecisionCache) {
        let quad = (self.c * self.eta0 * self.eta0 - 2.0 * self.b * self.eta0 * self.eta1
            + self.a * self.eta1 * self.eta1)
            * cache.inv_det;
        self.log_weight = 0.5 * quad + cache.log_det_term;
    }

    fn det(&self) -> f64 {
        self.a * self.c - self.b * self.b
    }

    fn mean_var(&self) -> ([f64; 2], [f64; 2]) {
        let det = self.det().max(1e-300);
        let mean0 = (self.c * self.eta0 - self.b * self.eta1) / det;
        let mean1 = (-self.b * self.eta0 + self.a * self.eta1) / det;
        let var0 = self.c / det;
        let var1 = self.a / det;
        ([mean0, mean1], [var0.max(0.0), var1.max(0.0)])
    }

    fn sample(&self, rng: &mut XorShift64) -> ([f64; 2], [f64; 2]) {
        let det = self.det().max(1e-300);
        let (mean, var) = self.mean_var();
        let (e0, e1) = rng.normal_pair();
        let l00 = (self.c / det).max(0.0).sqrt();
        let l10 = -self.b / (self.c * det).max(1e-300).sqrt();
        let l11 = (1.0 / self.c.max(1e-300)).sqrt();
        let x0 = mean[0] + l00 * e0;
        let x1 = mean[1] + l10 * e0 + l11 * e1;
        ([x0, x1], var)
    }
}

struct Solver {
    segments: [SegmentState; SEG_COUNT],
    observations: Vec<Observation>,
    history_by_segment: [Vec<usize>; SEG_COUNT],
    precision_cache: [[PrecisionCache; Z_MAX + 1]; SEG_COUNT],
    rng: XorShift64,
    start: Instant,
    profile: Option<ProfileStats>,
}

impl Solver {
    fn new() -> Self {
        Self {
            segments: [SegmentState::new(); SEG_COUNT],
            observations: Vec::with_capacity(Q),
            history_by_segment: std::array::from_fn(|_| Vec::with_capacity(Q / 2)),
            precision_cache: std::array::from_fn(|_| {
                std::array::from_fn(|_| PrecisionCache::new())
            }),
            rng: XorShift64::new(0x8d12_4f7a_c9e3_5521),
            start: Instant::now(),
            profile: if std::env::var_os("AHC003_PROFILE").is_some() {
                Some(ProfileStats::default())
            } else {
                None
            },
        }
    }

    fn solve_query(&mut self, s: (usize, usize), t: (usize, usize), turn: usize) -> Vec<u8> {
        let beta = beta_schedule(turn);
        let start = self.profile.as_ref().map(|_| Instant::now());
        let path = self.shortest_path_lcb(s, t, beta);
        if let (Some(profile), Some(start)) = (&mut self.profile, start) {
            profile.dijkstra_ns += start.elapsed().as_nanos() as u64;
            profile.dijkstra_count += 1;
        }
        path
    }

    fn observe_and_update(&mut self, s: (usize, usize), path: &[u8], observed: f64, turn: usize) {
        let prefix_start = self.profile.as_ref().map(|_| Instant::now());
        let (prefix, touched) = build_prefix_counts(s, path);
        if let (Some(profile), Some(prefix_start)) = (&mut self.profile, prefix_start) {
            profile.prefix_ns += prefix_start.elapsed().as_nanos() as u64;
            profile.prefix_count += 1;
        }

        let total_start = self.profile.as_ref().map(|_| Instant::now());
        let mut model_total = 0.0;
        for &g in &touched {
            model_total += contribution_from_prefix(&prefix, g, self.segments[g]);
        }
        if let (Some(profile), Some(total_start)) = (&mut self.profile, total_start) {
            profile.observe_total_ns += total_start.elapsed().as_nanos() as u64;
        }

        let obs_index = self.observations.len();
        let mut inv_sigma2 = [0.0; SEG_COUNT];
        for &g in &touched {
            let total = prefix[g][EDGE_POS] as f64;
            let sigma2 = (observed * observed / 300.0) + total * D2_OVER3_PRIOR;
            let w = 1.0 / sigma2.max(1e-9);
            inv_sigma2[g] = w;
            for z in Z_MIN..=Z_MAX {
                let l = prefix[g][z] as f64;
                let r = total - l;
                let cache = &mut self.precision_cache[g][z];
                cache.a += w * l * l;
                cache.b += w * l * r;
                cache.c += w * r * r;
                cache.recompute();
            }
        }
        self.observations.push(Observation {
            observed,
            model_total,
            prefix,
            inv_sigma2,
        });
        for &g in &touched {
            self.history_by_segment[g].push(obs_index);
        }

        self.run_mcmc(turn);
    }

    fn run_mcmc(&mut self, turn: usize) {
        let mcmc_start = self.profile.as_ref().map(|_| Instant::now());
        let max_rounds = round_schedule(turn);
        for _ in 0..max_rounds {
            for g in 0..SEG_COUNT {
                if (g & 15) == 0 && self.elapsed_sec() > TIME_LIMIT_SEC {
                    if let (Some(profile), Some(mcmc_start)) = (&mut self.profile, mcmc_start) {
                        profile.mcmc_ns += mcmc_start.elapsed().as_nanos() as u64;
                    }
                    return;
                }
                self.update_segment(g);
            }
        }
        if let (Some(profile), Some(mcmc_start)) = (&mut self.profile, mcmc_start) {
            profile.mcmc_ns += mcmc_start.elapsed().as_nanos() as u64;
        }
    }

    fn elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn update_segment(&mut self, g: usize) {
        if self.history_by_segment[g].is_empty() {
            return;
        }
        let segment_start = self.profile.as_ref().map(|_| Instant::now());

        let mut posts = [Posterior2::prior(); Z_MAX + 1];
        for (z, post) in posts.iter_mut().enumerate().take(Z_MAX + 1).skip(Z_MIN) {
            let cache = self.precision_cache[g][z];
            post.a += cache.a;
            post.b += cache.b;
            post.c += cache.c;
        }
        let current = self.segments[g];
        let scan_start = self.profile.as_ref().map(|_| Instant::now());

        for &obs_idx in &self.history_by_segment[g] {
            let obs = &self.observations[obs_idx];
            let total = obs.prefix[g][EDGE_POS] as f64;
            if total == 0.0 {
                continue;
            }
            let old = contribution_from_prefix(&obs.prefix, g, current);
            let residual = obs.observed - (obs.model_total - old);
            let inv_sigma2 = obs.inv_sigma2[g];

            for z in Z_MIN..=Z_MAX {
                let l = obs.prefix[g][z] as f64;
                let r = total - l;
                let p = &mut posts[z];
                p.eta0 += inv_sigma2 * residual * l;
                p.eta1 += inv_sigma2 * residual * r;
            }
        }
        if let (Some(profile), Some(scan_start)) = (&mut self.profile, scan_start) {
            profile.segment_scan_ns += scan_start.elapsed().as_nanos() as u64;
            profile.history_items += self.history_by_segment[g].len() as u64;
        }

        let finish_start = self.profile.as_ref().map(|_| Instant::now());
        let mut max_log = f64::NEG_INFINITY;
        for (z, post) in posts.iter_mut().enumerate().take(Z_MAX + 1).skip(Z_MIN) {
            post.finish_log_weight(self.precision_cache[g][z]);
            max_log = max_log.max(post.log_weight);
        }

        let mut weight_sum = 0.0;
        let mut weights = [0.0; Z_MAX + 1];
        for z in Z_MIN..=Z_MAX {
            let w = (posts[z].log_weight - max_log).exp();
            weights[z] = w;
            weight_sum += w;
        }

        let selected_z = if weight_sum.is_finite() && weight_sum > 0.0 {
            let mut target = self.rng.next_f64() * weight_sum;
            let mut selected = Z_MAX;
            for z in Z_MIN..=Z_MAX {
                target -= weights[z];
                if target <= 0.0 {
                    selected = z;
                    break;
                }
            }
            selected
        } else {
            Z_MIN + (self.rng.next_u64() as usize % (Z_MAX - Z_MIN + 1))
        };

        let (sample_x, var) = posts[selected_z].sample(&mut self.rng);
        let (mean, _) = posts[selected_z].mean_var();
        if let (Some(profile), Some(finish_start)) = (&mut self.profile, finish_start) {
            profile.segment_finish_ns += finish_start.elapsed().as_nanos() as u64;
        }
        let new_state = SegmentState {
            z: selected_z,
            x: sample_x,
            mean,
            var,
        };

        let apply_start = self.profile.as_ref().map(|_| Instant::now());
        for &obs_idx in &self.history_by_segment[g] {
            let obs = &mut self.observations[obs_idx];
            let old = contribution_from_prefix(&obs.prefix, g, current);
            let new = contribution_from_prefix(&obs.prefix, g, new_state);
            obs.model_total += new - old;
        }
        if let (Some(profile), Some(apply_start), Some(segment_start)) =
            (&mut self.profile, apply_start, segment_start)
        {
            profile.segment_apply_ns += apply_start.elapsed().as_nanos() as u64;
            profile.segment_total_ns += segment_start.elapsed().as_nanos() as u64;
            profile.segment_updates += 1;
        }

        self.segments[g] = new_state;
    }

    fn print_profile(&self) {
        if let Some(profile) = &self.profile {
            profile.print();
        }
    }

    fn shortest_path_lcb(&self, s: (usize, usize), t: (usize, usize), beta: f64) -> Vec<u8> {
        let (h_weight, v_weight) = self.build_lcb_weights(beta);
        let start = point_id(s.0, s.1);
        let goal = point_id(t.0, t.1);
        let mut dist = [f64::INFINITY; VERTEX_COUNT];
        let mut prev = [usize::MAX; VERTEX_COUNT];
        let mut prev_dir = [0_u8; VERTEX_COUNT];
        let mut heap = BinaryHeap::new();

        dist[start] = 0.0;
        heap.push(HeapEntry {
            cost: 0.0,
            node: start,
        });

        while let Some(entry) = heap.pop() {
            if entry.cost != dist[entry.node] {
                continue;
            }
            if entry.node == goal {
                break;
            }

            let i = entry.node / N;
            let j = entry.node % N;
            let relax = |ni: usize,
                         nj: usize,
                         dir: u8,
                         w: f64,
                         dist: &mut [f64; VERTEX_COUNT],
                         prev: &mut [usize; VERTEX_COUNT],
                         prev_dir: &mut [u8; VERTEX_COUNT],
                         heap: &mut BinaryHeap<HeapEntry>| {
                let next = point_id(ni, nj);
                let nd = entry.cost + w;
                if nd < dist[next] {
                    dist[next] = nd;
                    prev[next] = entry.node;
                    prev_dir[next] = dir;
                    heap.push(HeapEntry {
                        cost: nd,
                        node: next,
                    });
                }
            };

            if i > 0 {
                relax(
                    i - 1,
                    j,
                    b'U',
                    v_weight[i - 1][j],
                    &mut dist,
                    &mut prev,
                    &mut prev_dir,
                    &mut heap,
                );
            }
            if i + 1 < N {
                relax(
                    i + 1,
                    j,
                    b'D',
                    v_weight[i][j],
                    &mut dist,
                    &mut prev,
                    &mut prev_dir,
                    &mut heap,
                );
            }
            if j > 0 {
                relax(
                    i,
                    j - 1,
                    b'L',
                    h_weight[i][j - 1],
                    &mut dist,
                    &mut prev,
                    &mut prev_dir,
                    &mut heap,
                );
            }
            if j + 1 < N {
                relax(
                    i,
                    j + 1,
                    b'R',
                    h_weight[i][j],
                    &mut dist,
                    &mut prev,
                    &mut prev_dir,
                    &mut heap,
                );
            }
        }

        let mut path = Vec::new();
        let mut cur = goal;
        while cur != start {
            let dir = prev_dir[cur];
            path.push(dir);
            cur = prev[cur];
        }
        path.reverse();
        path
    }

    fn build_lcb_weights(&self, beta: f64) -> ([[f64; EDGE_POS]; N], [[f64; N]; EDGE_POS]) {
        let mut h_weight = [[0.0; EDGE_POS]; N];
        let mut v_weight = [[0.0; N]; EDGE_POS];

        for (i, row) in h_weight.iter_mut().enumerate().take(N) {
            let seg = self.segments[i];
            let w0 = (seg.mean[0] - beta * seg.var[0].sqrt()).max(1.0);
            let w1 = (seg.mean[1] - beta * seg.var[1].sqrt()).max(1.0);
            for (j, cell) in row.iter_mut().enumerate().take(EDGE_POS) {
                *cell = if j < seg.z { w0 } else { w1 };
            }
        }

        for j in 0..N {
            let seg = self.segments[N + j];
            let w0 = (seg.mean[0] - beta * seg.var[0].sqrt()).max(1.0);
            let w1 = (seg.mean[1] - beta * seg.var[1].sqrt()).max(1.0);
            for (i, row) in v_weight.iter_mut().enumerate().take(EDGE_POS) {
                row[j] = if i < seg.z { w0 } else { w1 };
            }
        }

        (h_weight, v_weight)
    }
}

#[derive(Default)]
struct ProfileStats {
    dijkstra_ns: u64,
    dijkstra_count: u64,
    prefix_ns: u64,
    prefix_count: u64,
    observe_total_ns: u64,
    mcmc_ns: u64,
    segment_total_ns: u64,
    segment_scan_ns: u64,
    segment_finish_ns: u64,
    segment_apply_ns: u64,
    segment_updates: u64,
    history_items: u64,
}

impl ProfileStats {
    fn print(&self) {
        eprintln!(
            "profile: dijkstra={:.3}ms count={} avg={:.3}us",
            ns_to_ms(self.dijkstra_ns),
            self.dijkstra_count,
            ns_to_us_div(self.dijkstra_ns, self.dijkstra_count)
        );
        eprintln!(
            "profile: prefix={:.3}ms count={} avg={:.3}us observe_total={:.3}ms",
            ns_to_ms(self.prefix_ns),
            self.prefix_count,
            ns_to_us_div(self.prefix_ns, self.prefix_count),
            ns_to_ms(self.observe_total_ns)
        );
        eprintln!(
            "profile: mcmc={:.3}ms segment_total={:.3}ms updates={} avg_update={:.3}us avg_history_items={:.2}",
            ns_to_ms(self.mcmc_ns),
            ns_to_ms(self.segment_total_ns),
            self.segment_updates,
            ns_to_us_div(self.segment_total_ns, self.segment_updates),
            div_f64(self.history_items, self.segment_updates)
        );
        eprintln!(
            "profile: segment_scan={:.3}ms finish_sample={:.3}ms apply={:.3}ms",
            ns_to_ms(self.segment_scan_ns),
            ns_to_ms(self.segment_finish_ns),
            ns_to_ms(self.segment_apply_ns)
        );
    }
}

fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

fn ns_to_us_div(ns: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        ns as f64 / count as f64 / 1_000.0
    }
}

fn div_f64(a: u64, b: u64) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

#[derive(Clone, Copy)]
struct HeapEntry {
    cost: f64,
    node: usize,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node == other.node
    }
}

impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.node.cmp(&self.node))
    }
}

struct XorShift64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    fn normal_pair(&mut self) -> (f64, f64) {
        if let Some(x) = self.spare_normal.take() {
            return (x, self.standard_normal());
        }
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = std::f64::consts::TAU * u2;
        (r * theta.cos(), r * theta.sin())
    }

    fn standard_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = std::f64::consts::TAU * u2;
        self.spare_normal = Some(r * theta.sin());
        r * theta.cos()
    }
}

fn beta_schedule(turn: usize) -> f64 {
    let x = 1.0 - (turn as f64 / 999.0).clamp(0.0, 1.0);
    2.0 * x.powf(1.5)
}

fn round_schedule(turn: usize) -> usize {
    match turn {
        0..=99 => 32,
        100..=299 => 40,
        300..=499 => 32,
        500..=699 => 24,
        700..=849 => 16,
        850..=949 => 8,
        _ => 4,
    }
}

fn contribution_from_prefix(
    prefix: &[[u8; PREFIX_LEN]; SEG_COUNT],
    g: usize,
    seg: SegmentState,
) -> f64 {
    let l = prefix[g][seg.z] as f64;
    let total = prefix[g][EDGE_POS] as f64;
    l * seg.x[0] + (total - l) * seg.x[1]
}

fn build_prefix_counts(
    s: (usize, usize),
    path: &[u8],
) -> ([[u8; PREFIX_LEN]; SEG_COUNT], Vec<usize>) {
    let mut prefix = [[0_u8; PREFIX_LEN]; SEG_COUNT];
    let mut seen = [false; SEG_COUNT];
    let mut touched = Vec::new();
    let (mut i, mut j) = s;

    for &dir in path {
        let (g, pos) = match dir {
            b'U' => {
                let g = N + j;
                i -= 1;
                (g, i)
            }
            b'D' => {
                let g = N + j;
                let pos = i;
                i += 1;
                (g, pos)
            }
            b'L' => {
                let g = i;
                j -= 1;
                (g, j)
            }
            b'R' => {
                let g = i;
                let pos = j;
                j += 1;
                (g, pos)
            }
            _ => unreachable!(),
        };
        if !seen[g] {
            seen[g] = true;
            touched.push(g);
        }
        for z in (pos + 1)..=EDGE_POS {
            prefix[g][z] += 1;
        }
    }

    (prefix, touched)
}

fn point_id(i: usize, j: usize) -> usize {
    i * N + j
}

fn path_to_string(path: &[u8]) -> String {
    String::from_utf8(path.to_vec()).unwrap()
}

fn true_path_length(
    path: &[u8],
    s: (usize, usize),
    h: &[[i64; EDGE_POS]; N],
    v: &[[i64; N]; EDGE_POS],
) -> i64 {
    let (mut i, mut j) = s;
    let mut sum = 0_i64;
    for &dir in path {
        match dir {
            b'U' => {
                i -= 1;
                sum += v[i][j];
            }
            b'D' => {
                sum += v[i][j];
                i += 1;
            }
            b'L' => {
                j -= 1;
                sum += h[i][j];
            }
            b'R' => {
                sum += h[i][j];
                j += 1;
            }
            _ => unreachable!(),
        }
    }
    sum
}

fn run_local<R: BufRead, W: Write>(first_line: String, reader: &mut R, writer: &mut W) {
    let mut rest = String::new();
    reader.read_to_string(&mut rest).unwrap();
    let mut tokens = first_line.split_whitespace().chain(rest.split_whitespace());

    let mut h = [[0_i64; EDGE_POS]; N];
    let mut v = [[0_i64; N]; EDGE_POS];
    for row in h.iter_mut().take(N) {
        for value in row.iter_mut().take(EDGE_POS) {
            *value = tokens.next().unwrap().parse::<i64>().unwrap();
        }
    }
    for row in v.iter_mut().take(EDGE_POS) {
        for value in row.iter_mut().take(N) {
            *value = tokens.next().unwrap().parse::<i64>().unwrap();
        }
    }

    let mut solver = Solver::new();
    for turn in 0..Q {
        let si = tokens.next().unwrap().parse::<usize>().unwrap();
        let sj = tokens.next().unwrap().parse::<usize>().unwrap();
        let ti = tokens.next().unwrap().parse::<usize>().unwrap();
        let tj = tokens.next().unwrap().parse::<usize>().unwrap();
        let _a = tokens.next().unwrap().parse::<i64>().unwrap();
        let e = tokens.next().unwrap().parse::<f64>().unwrap();

        let s = (si, sj);
        let t = (ti, tj);
        let path = solver.solve_query(s, t, turn);
        writeln!(writer, "{}", path_to_string(&path)).unwrap();

        let b = true_path_length(&path, s, &h, &v);
        let observed = (b as f64 * e).round();
        solver.observe_and_update(s, &path, observed, turn);
    }
    solver.print_profile();
}

fn run_interactive<R: BufRead, W: Write>(first_line: String, reader: &mut R, writer: &mut W) {
    let mut solver = Solver::new();
    let mut query_line = first_line;

    for turn in 0..Q {
        let vals = query_line
            .split_whitespace()
            .map(|s| s.parse::<usize>().unwrap())
            .collect::<Vec<_>>();
        let s = (vals[0], vals[1]);
        let t = (vals[2], vals[3]);

        let path = solver.solve_query(s, t, turn);
        writeln!(writer, "{}", path_to_string(&path)).unwrap();
        writer.flush().unwrap();

        let mut observed_line = String::new();
        reader.read_line(&mut observed_line).unwrap();
        let observed = observed_line.trim().parse::<f64>().unwrap();
        solver.observe_and_update(s, &path, observed, turn);

        if turn + 1 < Q {
            query_line.clear();
            reader.read_line(&mut query_line).unwrap();
        }
    }
    solver.print_profile();
}

fn main() {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let mut first_line = String::new();
    if reader.read_line(&mut first_line).unwrap() == 0 {
        return;
    }
    let first_count = first_line.split_whitespace().count();
    if first_count == 4 {
        run_interactive(first_line, &mut reader, &mut writer);
    } else {
        run_local(first_line, &mut reader, &mut writer);
    }
}
