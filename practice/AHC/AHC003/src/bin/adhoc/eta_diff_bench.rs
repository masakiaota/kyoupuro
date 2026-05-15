// eta_diff_bench.rs
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

const N: usize = 30;
const Q: usize = 1000;
const SEG_COUNT: usize = 60;
const EDGE_POS: usize = 29;
const PREFIX_LEN: usize = EDGE_POS + 1;
const Z_MIN: usize = 1;
const Z_MAX: usize = 28;

const MU0: f64 = 5000.0;
const TAU2: f64 = 27010000.0 / 9.0;
const PRIOR_ETA: f64 = MU0 / TAU2;
const D2_OVER3_PRIOR: f64 = 4210000.0 / 9.0;

#[derive(Clone)]
struct Observation {
    observed: f64,
    model_total: f64,
    prefix: [[u8; PREFIX_LEN]; SEG_COUNT],
    inv_sigma2: [f64; SEG_COUNT],
    touched: Vec<usize>,
}

#[derive(Clone, Copy)]
struct SegmentState {
    z: usize,
    x: [f64; 2],
}

impl SegmentState {
    fn initial() -> Self {
        Self {
            z: 14,
            x: [MU0, MU0],
        }
    }
}

struct ParsedCase {
    h: [[i64; EDGE_POS]; N],
    v: [[i64; N]; EDGE_POS],
    queries: Vec<Query>,
    paths: Vec<Vec<u8>>,
}

#[derive(Clone, Copy)]
struct Query {
    s: (usize, usize),
    e: f64,
}

#[derive(Clone)]
struct BenchData {
    observations: Vec<Observation>,
    history_by_segment: [Vec<usize>; SEG_COUNT],
    avg_touched: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("tools/in/0000.txt");
    let output_path = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("results/out/v001_bayes_gibbs_lcb/0000.txt");
    let rounds = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    let repeats = args
        .get(4)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3);

    let parsed = read_case(input_path, output_path);
    let update_states = make_update_states(rounds * SEG_COUNT);

    println!(
        "input={}, output={}, rounds={}, repeats={}",
        input_path, output_path, rounds, repeats
    );
    println!(
        "turns,avg_touched,scan_ms,diff_init_ms,diff_update_ms,diff_total_ms,ratio_update,ratio_total,max_eta_error"
    );

    for turns in [100_usize, 300, 600, 1000] {
        let data = build_bench_data(&parsed, turns.min(parsed.queries.len()));
        let (scan_time, scan_sum) = best_of(repeats, || bench_scan(&data, rounds, &update_states));
        let (diff_init, diff_time, diff_sum, max_error) =
            best_diff_of(repeats, || bench_diff(&data, rounds, &update_states));

        black_box(scan_sum);
        black_box(diff_sum);
        let scan_ms = ms(scan_time);
        let diff_init_ms = ms(diff_init);
        let diff_update_ms = ms(diff_time);
        let diff_total_ms = diff_init_ms + diff_update_ms;
        println!(
            "{},{:.2},{:.3},{:.3},{:.3},{:.3},{:.2},{:.2},{:.3e}",
            turns,
            data.avg_touched,
            scan_ms,
            diff_init_ms,
            diff_update_ms,
            diff_total_ms,
            diff_update_ms / scan_ms,
            diff_total_ms / scan_ms,
            max_error
        );
    }
}

fn best_of<F>(repeats: usize, mut f: F) -> (Duration, f64)
where
    F: FnMut() -> (Duration, f64),
{
    let mut best = Duration::MAX;
    let mut best_sum = 0.0;
    for _ in 0..repeats.max(1) {
        let (time, sum) = f();
        if time < best {
            best = time;
            best_sum = sum;
        }
    }
    (best, best_sum)
}

fn best_diff_of<F>(repeats: usize, mut f: F) -> (Duration, Duration, f64, f64)
where
    F: FnMut() -> (Duration, Duration, f64, f64),
{
    let mut best_total = Duration::MAX;
    let mut best = (Duration::ZERO, Duration::ZERO, 0.0, 0.0);
    for _ in 0..repeats.max(1) {
        let result = f();
        let total = result.0 + result.1;
        if total < best_total {
            best_total = total;
            best = result;
        }
    }
    best
}

fn read_case(input_path: &str, output_path: &str) -> ParsedCase {
    let input = fs::read_to_string(input_path).unwrap();
    let mut tokens = input.split_whitespace();

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

    let mut queries = Vec::with_capacity(Q);
    for _ in 0..Q {
        let Some(si) = tokens.next() else { break };
        let sj = tokens.next().unwrap().parse::<usize>().unwrap();
        let ti = tokens.next().unwrap().parse::<usize>().unwrap();
        let tj = tokens.next().unwrap().parse::<usize>().unwrap();
        let _a = tokens.next().unwrap().parse::<i64>().unwrap();
        let e = tokens.next().unwrap().parse::<f64>().unwrap();
        queries.push(Query {
            s: (si.parse::<usize>().unwrap(), sj),
            e,
        });
        black_box((ti, tj));
    }

    let paths = if Path::new(output_path).exists() {
        fs::read_to_string(output_path)
            .unwrap()
            .split_whitespace()
            .map(|s| s.as_bytes().to_vec())
            .collect()
    } else {
        panic!("output path not found: {}", output_path);
    };

    ParsedCase {
        h,
        v,
        queries,
        paths,
    }
}

fn build_bench_data(parsed: &ParsedCase, turns: usize) -> BenchData {
    let states = [SegmentState::initial(); SEG_COUNT];
    let mut observations = Vec::with_capacity(turns);
    let mut history_by_segment: [Vec<usize>; SEG_COUNT] =
        std::array::from_fn(|_| Vec::with_capacity(turns / 2));
    let mut touched_sum = 0_usize;

    for obs_index in 0..turns {
        let query = parsed.queries[obs_index];
        let path = &parsed.paths[obs_index];
        let (prefix, touched) = build_prefix_counts(query.s, path);
        let true_len = true_path_length(path, query.s, &parsed.h, &parsed.v);
        let observed = (true_len as f64 * query.e).round();
        let mut model_total = 0.0;
        for &g in &touched {
            model_total += contribution_from_prefix(&prefix, g, states[g]);
        }

        let mut inv_sigma2 = [0.0; SEG_COUNT];
        for &g in &touched {
            let total = prefix[g][EDGE_POS] as f64;
            let sigma2 = (observed * observed / 300.0) + total * D2_OVER3_PRIOR;
            inv_sigma2[g] = 1.0 / sigma2.max(1e-9);
            history_by_segment[g].push(obs_index);
        }
        touched_sum += touched.len();
        observations.push(Observation {
            observed,
            model_total,
            prefix,
            inv_sigma2,
            touched,
        });
    }

    BenchData {
        observations,
        history_by_segment,
        avg_touched: touched_sum as f64 / turns.max(1) as f64,
    }
}

fn bench_scan(data: &BenchData, rounds: usize, update_states: &[SegmentState]) -> (Duration, f64) {
    let mut observations = data.observations.clone();
    let mut states = [SegmentState::initial(); SEG_COUNT];
    let mut checksum = 0.0;

    let start = Instant::now();
    let mut step = 0;
    for _ in 0..rounds {
        for g in 0..SEG_COUNT {
            let current = states[g];
            let mut eta0 = [PRIOR_ETA; Z_MAX + 1];
            let mut eta1 = [PRIOR_ETA; Z_MAX + 1];
            for &obs_idx in &data.history_by_segment[g] {
                let obs = &observations[obs_idx];
                let total = obs.prefix[g][EDGE_POS] as f64;
                let old = contribution_from_prefix(&obs.prefix, g, current);
                let residual = obs.observed - (obs.model_total - old);
                let w = obs.inv_sigma2[g];
                for z in Z_MIN..=Z_MAX {
                    let l = obs.prefix[g][z] as f64;
                    let r = total - l;
                    eta0[z] += w * residual * l;
                    eta1[z] += w * residual * r;
                }
            }
            checksum += black_box(eta0[14] + eta1[14]);

            let new_state = update_states[step];
            step += 1;
            for &obs_idx in &data.history_by_segment[g] {
                let obs = &mut observations[obs_idx];
                let old = contribution_from_prefix(&obs.prefix, g, current);
                let new = contribution_from_prefix(&obs.prefix, g, new_state);
                obs.model_total += new - old;
            }
            states[g] = new_state;
        }
    }
    (start.elapsed(), checksum)
}

fn bench_diff(
    data: &BenchData,
    rounds: usize,
    update_states: &[SegmentState],
) -> (Duration, Duration, f64, f64) {
    let mut observations = data.observations.clone();
    let mut states = [SegmentState::initial(); SEG_COUNT];

    let init_start = Instant::now();
    let (mut eta0, mut eta1) = compute_eta_cache(&observations, &states);
    let init_time = init_start.elapsed();

    let mut checksum = 0.0;
    let update_start = Instant::now();
    let mut step = 0;
    for _ in 0..rounds {
        for k in 0..SEG_COUNT {
            checksum += black_box(eta0[k][14] + eta1[k][14]);
            let current = states[k];
            let new_state = update_states[step];
            step += 1;

            for &obs_idx in &data.history_by_segment[k] {
                let obs = &mut observations[obs_idx];
                let old = contribution_from_prefix(&obs.prefix, k, current);
                let new = contribution_from_prefix(&obs.prefix, k, new_state);
                let delta = new - old;
                obs.model_total += delta;
                if delta == 0.0 {
                    continue;
                }
                for &g in &obs.touched {
                    if g == k {
                        continue;
                    }
                    let total = obs.prefix[g][EDGE_POS] as f64;
                    let w_delta = obs.inv_sigma2[g] * delta;
                    for z in Z_MIN..=Z_MAX {
                        let l = obs.prefix[g][z] as f64;
                        let r = total - l;
                        eta0[g][z] -= w_delta * l;
                        eta1[g][z] -= w_delta * r;
                    }
                }
            }
            states[k] = new_state;
        }
    }
    let update_time = update_start.elapsed();
    let max_error = validate_eta_cache(&observations, &states, &eta0, &eta1);
    (init_time, update_time, checksum, max_error)
}

fn compute_eta_cache(
    observations: &[Observation],
    states: &[SegmentState; SEG_COUNT],
) -> ([[f64; Z_MAX + 1]; SEG_COUNT], [[f64; Z_MAX + 1]; SEG_COUNT]) {
    let mut eta0 = [[PRIOR_ETA; Z_MAX + 1]; SEG_COUNT];
    let mut eta1 = [[PRIOR_ETA; Z_MAX + 1]; SEG_COUNT];
    for obs in observations {
        for &g in &obs.touched {
            let total = obs.prefix[g][EDGE_POS] as f64;
            let old = contribution_from_prefix(&obs.prefix, g, states[g]);
            let residual = obs.observed - (obs.model_total - old);
            let weighted = obs.inv_sigma2[g] * residual;
            for z in Z_MIN..=Z_MAX {
                let l = obs.prefix[g][z] as f64;
                let r = total - l;
                eta0[g][z] += weighted * l;
                eta1[g][z] += weighted * r;
            }
        }
    }
    (eta0, eta1)
}

fn validate_eta_cache(
    observations: &[Observation],
    states: &[SegmentState; SEG_COUNT],
    eta0: &[[f64; Z_MAX + 1]; SEG_COUNT],
    eta1: &[[f64; Z_MAX + 1]; SEG_COUNT],
) -> f64 {
    let (expected0, expected1) = compute_eta_cache(observations, states);
    let mut max_error: f64 = 0.0;
    for g in 0..SEG_COUNT {
        for z in Z_MIN..=Z_MAX {
            max_error = max_error.max((eta0[g][z] - expected0[g][z]).abs());
            max_error = max_error.max((eta1[g][z] - expected1[g][z]).abs());
        }
    }
    max_error
}

fn make_update_states(count: usize) -> Vec<SegmentState> {
    let mut rng = XorShift64::new(0x7a6d_9e31_c4b2_1085);
    let mut states = Vec::with_capacity(count);
    for _ in 0..count {
        states.push(SegmentState {
            z: Z_MIN + (rng.next_u64() as usize % (Z_MAX - Z_MIN + 1)),
            x: [
                2000.0 + 6000.0 * rng.next_f64(),
                2000.0 + 6000.0 * rng.next_f64(),
            ],
        });
    }
    states
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

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
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
}
