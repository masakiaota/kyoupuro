// bench_theta_sampling.rs
#![allow(non_snake_case)]

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

const THETA_MIN: f64 = 2_000.0;
const THETA_MAX: f64 = 8_000.0;
const LOG_THETA_MIN: f64 = 7.600_902_459_542_082;
const LOG_THETA_WIDTH: f64 = 1.386_294_361_119_890_6;
const SAMPLES_PER_STATE: usize = 8;
const REPEATS: usize = 7;

struct XorShift64 {
    state: u64,
    spare_gauss: Option<f64>,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
            spare_gauss: None,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / 9_007_199_254_740_992.0)
    }

    #[inline]
    fn gauss(&mut self) -> f64 {
        if let Some(value) = self.spare_gauss.take() {
            return value;
        }
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * std::f64::consts::PI * u2;
        self.spare_gauss = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

#[derive(Clone, Copy)]
struct ThetaState {
    n: usize,
    Y: f64,
    theta_map: f64,
    laplace_sd: f64,
    log_density_max: f64,
}

impl ThetaState {
    fn new(n: usize, Y: u64) -> Self {
        let Y = Y as f64;
        let theta_map = (Y / (n as f64)).clamp(THETA_MIN, THETA_MAX);
        // log(theta) 上の密度は exp(-(n-1)z - Y exp(-z)) に比例する。
        // n=1かつY=0では一様なので、上端を選んでも最大値は同じである。
        let theta_z_mode = if n <= 1 {
            THETA_MAX
        } else {
            (Y / ((n - 1) as f64)).clamp(THETA_MIN, THETA_MAX)
        };
        let z_mode = theta_z_mode.ln();
        let log_density_max = -((n - 1) as f64) * z_mode - Y / theta_z_mode;
        Self {
            n,
            Y,
            theta_map,
            laplace_sd: theta_map / (n as f64).sqrt(),
            log_density_max,
        }
    }
}

/// log(theta) の一様提案から棄却する。受理後のthetaは、範囲で切断された
/// p(theta|D) ∝ theta^-n exp(-Y/theta) に厳密に従う。
#[inline]
fn sample_exact(state: ThetaState, rng: &mut XorShift64) -> (f64, usize) {
    let mut proposals = 0;
    loop {
        proposals += 1;
        let z = LOG_THETA_MIN + LOG_THETA_WIDTH * rng.next_f64();
        let theta = z.exp();
        let log_density = -((state.n - 1) as f64) * z - state.Y / theta;
        if rng.next_f64().ln() <= log_density - state.log_density_max {
            return (theta, proposals);
        }
    }
}

/// theta側MAPと局所曲率 Var(theta)≈theta_map^2/n を使う切断正規近似。
#[inline]
fn sample_laplace(state: ThetaState, rng: &mut XorShift64) -> (f64, usize) {
    let mut proposals = 0;
    loop {
        proposals += 1;
        let theta = state.theta_map + state.laplace_sd * rng.gauss();
        if (THETA_MIN..=THETA_MAX).contains(&theta) {
            return (theta, proposals);
        }
    }
}

fn load_states() -> Vec<ThetaState> {
    let mut paths: Vec<PathBuf> = fs::read_dir("tools/in")
        .expect("tools/in must exist")
        .map(|entry| entry.expect("input entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    paths.sort();

    let mut states = Vec::new();
    for path in paths {
        let input = fs::read_to_string(&path).expect("read input");
        let mut tokens = input.split_whitespace();
        let N: usize = tokens.next().unwrap().parse().unwrap();
        let M: usize = tokens.next().unwrap().parse().unwrap();
        let _: f64 = tokens.next().unwrap().parse().unwrap();
        for _ in 0..N {
            tokens.next().unwrap();
        }
        let mut Y = 0_u64;
        for n in 1..=M {
            let _: usize = tokens.next().unwrap().parse().unwrap();
            let S: usize = tokens.next().unwrap().parse().unwrap();
            let T: usize = tokens.next().unwrap().parse().unwrap();
            let _: usize = tokens.next().unwrap().parse().unwrap();
            let _: i64 = tokens.next().unwrap().parse().unwrap();
            Y += (T - S - 1) as u64;
            states.push(ThetaState::new(n, Y));
        }
    }
    states
}

#[derive(Clone, Copy)]
struct BenchResult {
    ns_per_sample: f64,
    proposals_per_sample: f64,
    checksum: f64,
}

fn run_exact(states: &[ThetaState], seed: u64) -> BenchResult {
    let mut rng = XorShift64::new(seed);
    let mut proposals = 0_u64;
    let mut checksum = 0.0;
    let sample_count = states.len() * SAMPLES_PER_STATE;
    let start = Instant::now();
    for &state in states {
        for _ in 0..SAMPLES_PER_STATE {
            let (theta, used) = sample_exact(state, &mut rng);
            checksum += black_box(theta);
            proposals += used as u64;
        }
    }
    BenchResult {
        ns_per_sample: start.elapsed().as_nanos() as f64 / sample_count as f64,
        proposals_per_sample: proposals as f64 / sample_count as f64,
        checksum,
    }
}

fn run_laplace(states: &[ThetaState], seed: u64) -> BenchResult {
    let mut rng = XorShift64::new(seed);
    let mut proposals = 0_u64;
    let mut checksum = 0.0;
    let sample_count = states.len() * SAMPLES_PER_STATE;
    let start = Instant::now();
    for &state in states {
        for _ in 0..SAMPLES_PER_STATE {
            let (theta, used) = sample_laplace(state, &mut rng);
            checksum += black_box(theta);
            proposals += used as u64;
        }
    }
    BenchResult {
        ns_per_sample: start.elapsed().as_nanos() as f64 / sample_count as f64,
        proposals_per_sample: proposals as f64 / sample_count as f64,
        checksum,
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    let states = load_states();
    assert_eq!(states.len(), 100_000);

    // 本計測前にコードと数学的境界を通す。結果は捨てる。
    let warm_states = &states[..1_000];
    black_box(run_exact(warm_states, 1));
    black_box(run_laplace(warm_states, 2));

    let mut exact = Vec::new();
    let mut laplace = Vec::new();
    for repeat in 0..REPEATS {
        let seed = 0x9E37_79B9_7F4A_7C15_u64 ^ repeat as u64;
        if repeat % 2 == 0 {
            exact.push(run_exact(&states, seed));
            laplace.push(run_laplace(&states, seed ^ 0xA5A5_A5A5_A5A5_A5A5));
        } else {
            laplace.push(run_laplace(&states, seed ^ 0xA5A5_A5A5_A5A5_A5A5));
            exact.push(run_exact(&states, seed));
        }
    }

    let exact_ns = median(exact.iter().map(|result| result.ns_per_sample).collect());
    let laplace_ns = median(
        laplace
            .iter()
            .map(|result| result.ns_per_sample)
            .collect(),
    );
    let exact_proposals = median(
        exact
            .iter()
            .map(|result| result.proposals_per_sample)
            .collect(),
    );
    let laplace_proposals = median(
        laplace
            .iter()
            .map(|result| result.proposals_per_sample)
            .collect(),
    );
    let checksum = exact.last().unwrap().checksum + laplace.last().unwrap().checksum;

    println!("states={} samples_per_method_per_repeat={}", states.len(), states.len() * SAMPLES_PER_STATE);
    println!("exact_ns_per_sample={exact_ns:.3}");
    println!("exact_proposals_per_sample={exact_proposals:.3}");
    println!("laplace_ns_per_sample={laplace_ns:.3}");
    println!("laplace_proposals_per_sample={laplace_proposals:.3}");
    println!("exact_over_laplace={:.3}", exact_ns / laplace_ns);
    // v059 traceのrollout本数82017/100ケースに、最大22到着を掛けた上限換算。
    let upper_samples_per_case = 82_017.0 / 100.0 * 22.0;
    println!("upper_samples_per_case={upper_samples_per_case:.1}");
    println!(
        "exact_upper_ms_per_case={:.3}",
        exact_ns * upper_samples_per_case / 1_000_000.0
    );
    println!(
        "laplace_upper_ms_per_case={:.3}",
        laplace_ns * upper_samples_per_case / 1_000_000.0
    );
    println!("checksum={checksum:.6}");
}
