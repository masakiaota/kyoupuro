// check_prefix_map.rs
#![allow(non_snake_case)]

use rand::{Rng, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha20Rng;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const N: usize = 50;
const H: f64 = 100_000.0;
const BETA_MIN: f64 = 12.5;
const BETA_MAX: f64 = 50.0;

#[derive(Clone, Copy, Default)]
struct Eval {
    value: f64,
    first: f64,
    second: f64,
}

#[derive(Default)]
struct OnlineStats {
    interval_evals: usize,
    newton_evals: usize,
    guarded_steps: usize,
    invalid_evals: usize,
}

#[derive(Clone, Copy, Default)]
struct ErrorStats {
    count: usize,
    sum: f64,
    sum_sq: f64,
    sum_abs: f64,
    max_abs: f64,
    lower_hits: usize,
    upper_hits: usize,
}

impl ErrorStats {
    fn add(&mut self, estimate: f64, truth: f64) {
        let error = estimate - truth;
        self.count += 1;
        self.sum += error;
        self.sum_sq += error * error;
        self.sum_abs += error.abs();
        self.max_abs = self.max_abs.max(error.abs());
        self.lower_hits += usize::from(estimate <= 2_000.0 + 1e-7);
        self.upper_hits += usize::from(estimate >= 8_000.0 - 1e-7);
    }

    fn print(&self, label: &str) {
        let count = self.count as f64;
        println!(
            "{label}: count={} bias={:.6} rmse={:.6} mae={:.6} max_abs={:.6} lower_hits={} upper_hits={}",
            self.count,
            self.sum / count,
            (self.sum_sq / count).sqrt(),
            self.sum_abs / count,
            self.max_abs,
            self.lower_hits,
            self.upper_hits,
        );
    }
}

fn moments(beta: f64, a: f64, max_k: usize) -> [f64; 10] {
    let mut u = [0.0; 10];
    let x = beta * a;
    if x < 8.0 {
        // u_k = beta*a^(k+1) * sum_j (-beta*a)^j / (j!*(k+j+1)).
        // 小さい beta*a で漸化式の差し引きによる桁落ちを避ける。
        let mut a_power = a;
        for k in 0..=max_k {
            let mut term = 1.0 / ((k + 1) as f64);
            let mut sum = term;
            for j in 1..=128 {
                term *= -x / (j as f64) * ((k + j) as f64) / ((k + j + 1) as f64);
                sum += term;
                if term.abs() <= 1e-17 * sum.abs().max(1.0) {
                    break;
                }
            }
            u[k] = beta * a_power * sum;
            a_power *= a;
        }
    } else {
        let exp_neg = (-x).exp();
        u[0] = -(-x).exp_m1();
        let mut a_power = a;
        for k in 1..=max_k {
            u[k] = (k as f64) * u[k - 1] / beta - a_power * exp_neg;
            a_power *= a;
        }
    }
    u
}

fn evaluate(beta: f64, n: usize, Y: u64, S: usize, M: usize, derivatives: bool) -> Eval {
    let r = M - n;
    let mut result = Eval {
        value: (n as f64) * beta.ln() - (Y as f64 / H) * beta,
        first: (n as f64) / beta - Y as f64 / H,
        second: -(n as f64) / (beta * beta),
    };
    if r == 0 {
        return result;
    }

    let a = (H - S as f64) / H;
    let c = S as f64 / H;
    let u = moments(beta, a, if derivatives { 9 } else { 7 });
    let q = u[0] - c * u[..=7].iter().sum::<f64>();
    if !(q > 0.0 && q.is_finite()) {
        return Eval {
            value: f64::NEG_INFINITY,
            first: f64::NAN,
            second: f64::NAN,
        };
    }
    result.value += (r as f64) * q.ln();
    if derivatives {
        let q_first =
            (u[0] / beta - u[1]) - c * (0..=7).map(|k| u[k] / beta - u[k + 1]).sum::<f64>();
        let q_second = (u[2] - 2.0 * u[1] / beta)
            - c * (0..=7)
                .map(|k| u[k + 2] - 2.0 * u[k + 1] / beta)
                .sum::<f64>();
        let ratio = q_first / q;
        result.first += (r as f64) * ratio;
        result.second += (r as f64) * (q_second / q - ratio * ratio);
    }
    result
}

fn interval_map(n: usize, Y: u64, S: usize, M: usize, stats: &mut OnlineStats) -> f64 {
    const INV_PHI: f64 = 0.618_033_988_749_894_9;
    let mut lo = BETA_MIN;
    let mut hi = BETA_MAX;
    let mut x1 = hi - INV_PHI * (hi - lo);
    let mut x2 = lo + INV_PHI * (hi - lo);
    let mut f1 = evaluate(x1, n, Y, S, M, false).value;
    let mut f2 = evaluate(x2, n, Y, S, M, false).value;
    stats.interval_evals += 2;
    for _ in 2..12 {
        if f1 < f2 {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + INV_PHI * (hi - lo);
            f2 = evaluate(x2, n, Y, S, M, false).value;
        } else {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - INV_PHI * (hi - lo);
            f1 = evaluate(x1, n, Y, S, M, false).value;
        }
        stats.interval_evals += 1;
    }
    if f1 >= f2 { x1 } else { x2 }
}

fn newton_map(n: usize, Y: u64, S: usize, M: usize, previous: f64, stats: &mut OnlineStats) -> f64 {
    let mut beta = previous.clamp(BETA_MIN, BETA_MAX);
    let mut lo = BETA_MIN;
    let mut hi = BETA_MAX;
    for _ in 0..2 {
        let value = evaluate(beta, n, Y, S, M, true);
        stats.newton_evals += 1;
        if !(value.value.is_finite() && value.first.is_finite() && value.second.is_finite()) {
            stats.invalid_evals += 1;
            return f64::NAN;
        }
        if value.first > 0.0 {
            lo = lo.max(beta);
        } else {
            hi = hi.min(beta);
        }
        if hi <= lo {
            beta = lo;
            continue;
        }
        let raw = beta - value.first / value.second;
        if value.second < 0.0 && raw.is_finite() && raw >= lo && raw <= hi {
            beta = raw;
        } else {
            stats.guarded_steps += 1;
            beta = if value.second < 0.0 && raw.is_finite() {
                raw.clamp(lo, hi)
            } else {
                0.5 * (lo + hi)
            };
        }
    }
    beta
}

fn global_map(n: usize, Y: u64, S: usize, M: usize) -> f64 {
    const INV_PHI: f64 = 0.618_033_988_749_894_9;
    let mut lo = BETA_MIN;
    let mut hi = BETA_MAX;
    let mut x1 = hi - INV_PHI * (hi - lo);
    let mut x2 = lo + INV_PHI * (hi - lo);
    let mut f1 = evaluate(x1, n, Y, S, M, false).value;
    let mut f2 = evaluate(x2, n, Y, S, M, false).value;
    for _ in 0..56 {
        if f1 < f2 {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + INV_PHI * (hi - lo);
            f2 = evaluate(x2, n, Y, S, M, false).value;
        } else {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - INV_PHI * (hi - lo);
            f1 = evaluate(x1, n, Y, S, M, false).value;
        }
    }
    let middle = 0.5 * (lo + hi);
    [BETA_MIN, middle, BETA_MAX]
        .into_iter()
        .max_by(|&lhs, &rhs| {
            evaluate(lhs, n, Y, S, M, false)
                .value
                .total_cmp(&evaluate(rhs, n, Y, S, M, false).value)
        })
        .unwrap()
}

fn true_instance_prefix(seed: u64) -> (i32, f64, Vec<Vec<bool>>) {
    // 公式generatorとtheta抽選直前まで同じ乱数消費を再現する。
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let R = rng.gen_range(1..=100) as f64 * 0.001;
    let mut grass = vec![vec![true; N]; N];
    let num_cluster = 2f64.powf(rng.gen_range(1.0..8.0)).round() as usize;
    let mut cells: Vec<(usize, usize)> = (0..N).flat_map(|i| (0..N).map(move |j| (i, j))).collect();
    cells.shuffle(&mut rng);
    for &(i, j) in cells.iter().take(num_cluster) {
        grass[i][j] = false;
    }
    let num_pond = rng.gen_range(0..=(900 - num_cluster) as i32) as usize;
    let dxy = [(0_i32, 1_i32), (0, -1), (1, 0), (-1, 0)];
    let mut in_frontier = vec![vec![false; N]; N];
    let mut frontier = Vec::new();
    let is_pond_neighbor = |grass: &[Vec<bool>], i: usize, j: usize| {
        dxy.iter().any(|&(dx, dy)| {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32 && !grass[ni as usize][nj as usize]
        })
    };
    for i in 0..N {
        for j in 0..N {
            if grass[i][j] && is_pond_neighbor(&grass, i, j) {
                in_frontier[i][j] = true;
                frontier.push((i, j));
            }
        }
    }
    for _ in 0..num_pond {
        if frontier.is_empty() {
            break;
        }
        let index = rng.gen_range(0..frontier.len() as i32) as usize;
        let (i, j) = frontier.swap_remove(index);
        in_frontier[i][j] = false;
        grass[i][j] = false;
        for &(dx, dy) in &dxy {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            if ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32 {
                let (ni, nj) = (ni as usize, nj as usize);
                if grass[ni][nj] && !in_frontier[ni][nj] {
                    in_frontier[ni][nj] = true;
                    frontier.push((ni, nj));
                }
            }
        }
    }
    // 公式generatorではrangeの整数型がi32に推論される。型が変わると同じ乱数状態でも
    // `gen_range`の写像が変わるため、ここでも明示的にi32を使う。
    let theta = rng.gen_range(2_000_i32..=8_000_i32);
    (theta, R, grass)
}

fn input_paths(directory: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn bucket(n: usize) -> usize {
    match n {
        1..=32 => 0,
        33..=100 => 1,
        101..=250 => 2,
        251..=500 => 3,
        _ => 4,
    }
}

fn main() {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tools/in".to_owned());
    let paths = input_paths(Path::new(&directory));
    assert_eq!(paths.len(), 100, "expected 100 input files");

    let mut cases = Vec::with_capacity(paths.len());
    for path in &paths {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let seed = stem.parse::<u64>().unwrap();
        let input = tools::parse_input(&fs::read_to_string(path).unwrap());
        let (theta, R, grass) = true_instance_prefix(seed);
        assert!(
            (input.r - R).abs() < 1e-12,
            "R mismatch in {}",
            path.display()
        );
        assert_eq!(input.grass, grass, "layout mismatch in {}", path.display());
        cases.push((theta as f64, input));
    }

    let mut online_by_case = Vec::with_capacity(cases.len());
    let mut online_stats = OnlineStats::default();
    let online_started = Instant::now();
    for (_, input) in &cases {
        let mut estimates = Vec::with_capacity(input.m);
        let mut Y = 0_u64;
        let mut beta = H / 5_000.0;
        for (index, group) in input.groups.iter().enumerate() {
            let n = index + 1;
            Y += (group.t - group.s - 1) as u64;
            beta = if n <= 32 {
                interval_map(n, Y, group.s as usize, input.m, &mut online_stats)
            } else {
                newton_map(n, Y, group.s as usize, input.m, beta, &mut online_stats)
            };
            assert!(beta.is_finite() && (BETA_MIN..=BETA_MAX).contains(&beta));
            estimates.push(H / beta);
        }
        online_by_case.push(estimates);
    }
    let online_elapsed = online_started.elapsed();

    let mut raw_errors = [ErrorStats::default(); 5];
    let mut online_errors = [ErrorStats::default(); 5];
    let mut global_errors = [ErrorStats::default(); 5];
    let fixed_n = [32_usize, 100, 250, 500, 750, 1_000];
    let mut raw_fixed = [ErrorStats::default(); 6];
    let mut online_fixed = [ErrorStats::default(); 6];
    let mut global_fixed = [ErrorStats::default(); 6];
    let mut tracking = ErrorStats::default();
    let mut objective_loss_sum = 0.0;
    let mut objective_loss_max = 0.0_f64;
    let mut objective_loss_case = 0;
    let mut objective_loss_group = 0;
    let reference_started = Instant::now();
    for (case_index, (truth, input)) in cases.iter().enumerate() {
        let mut Y = 0_u64;
        for (index, group) in input.groups.iter().enumerate() {
            let n = index + 1;
            Y += (group.t - group.s - 1) as u64;
            let raw = (Y as f64 / n as f64).clamp(2_000.0, 8_000.0);
            let online = online_by_case[case_index][index];
            let global_beta = global_map(n, Y, group.s as usize, input.m);
            let global = H / global_beta;
            let b = bucket(n);
            raw_errors[b].add(raw, *truth);
            online_errors[b].add(online, *truth);
            global_errors[b].add(global, *truth);
            if let Some(fixed_index) = fixed_n.iter().position(|&target| target == n) {
                raw_fixed[fixed_index].add(raw, *truth);
                online_fixed[fixed_index].add(online, *truth);
                global_fixed[fixed_index].add(global, *truth);
            }
            tracking.add(online, global);
            let online_beta = H / online;
            let loss = evaluate(global_beta, n, Y, group.s as usize, input.m, false).value
                - evaluate(online_beta, n, Y, group.s as usize, input.m, false).value;
            objective_loss_sum += loss.max(0.0);
            if loss > objective_loss_max {
                objective_loss_max = loss;
                objective_loss_case = case_index;
                objective_loss_group = n;
            }
        }
    }
    let reference_elapsed = reference_started.elapsed();

    println!("cases={} prefixes={}", cases.len(), cases.len() * 1_000);
    println!(
        "online_cpu_ms={:.3} reference_cpu_ms={:.3} interval_evals={} newton_evals={} guarded_steps={} invalid_evals={}",
        online_elapsed.as_secs_f64() * 1_000.0,
        reference_elapsed.as_secs_f64() * 1_000.0,
        online_stats.interval_evals,
        online_stats.newton_evals,
        online_stats.guarded_steps,
        online_stats.invalid_evals,
    );
    let labels = ["1-32", "33-100", "101-250", "251-500", "501-1000"];
    for (index, label) in labels.iter().enumerate() {
        raw_errors[index].print(&format!("raw_{label}"));
        online_errors[index].print(&format!("online_{label}"));
        global_errors[index].print(&format!("global_{label}"));
    }
    for (index, n) in fixed_n.iter().enumerate() {
        raw_fixed[index].print(&format!("raw_n{n}"));
        online_fixed[index].print(&format!("online_n{n}"));
        global_fixed[index].print(&format!("global_n{n}"));
    }
    tracking.print("online_minus_global");
    println!(
        "objective_loss: mean={:.12} max={:.12} max_case={:04} max_group={}",
        objective_loss_sum / 100_000.0,
        objective_loss_max,
        objective_loss_case,
        objective_loss_group,
    );
}
