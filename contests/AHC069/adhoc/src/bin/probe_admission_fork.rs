// probe_admission_fork.rs
//
// B-044 admission fork oracle for v035_no_move_growth_cutloss.
//
// 1 case:
//   cargo run --release --manifest-path adhoc/Cargo.toml --bin probe_admission_fork -- \
//     --input tools/in/0000.txt \
//     --baseline results/out/v035_no_move_growth_cutloss/0000.txt
//
// All matching cases in two directories (the full run is intentionally left to the caller):
//   cargo run --release --manifest-path adhoc/Cargo.toml --bin probe_admission_fork -- \
//     --input tools/in \
//     --baseline results/out/v035_no_move_growth_cutloss
//
// The probe compiles a temporary, instrumented copy of the current v035 source. This keeps the
// forked admission/placement path identical to v035 without duplicating its implementation here.
// The saved output is authoritative for the baseline trajectory; forked future decisions use the
// same actual arrival sequence with all wall-clock cutoffs disabled and never emit move operations.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

const V035_SOURCE: &str = include_str!("../../../src/bin/v035_no_move_growth_cutloss.rs");

const HELP: &str = r#"Usage:
  probe_admission_fork --input INPUT --baseline BASELINE [--future 64] [--limit N]

INPUT and BASELINE must both be files or both be directories. For directories, every *.txt input
with a same-named baseline output is evaluated in lexicographic order. --limit is useful for a
small smoke test. The default future horizon is 64 actual arrivals.

The baseline state is reconstructed from the saved v035 output. Each accepted decision with
base_threshold > 0 and actual-placement margin in [1.0, 1.30] is independently forked into accept
and reject branches. The reported optimistic upper bound is the non-composable sum of positive
reject advantages. A separate no-fork, unlimited-time replay reports decision and placement match
rates against the saved trajectory.
"#;

// Appended in the same crate as v035, so this code can use its private Solver implementation.
const ORACLE_HARNESS: &str = r#"

#[derive(Clone)]
struct OfflineArrival {
    id: usize,
    S: usize,
    T: usize,
    P: usize,
    V: i64,
}

struct OfflineInput {
    N: usize,
    M: usize,
    grass_rows: Rows,
    arrivals: Vec<OfflineArrival>,
}

#[derive(Clone)]
struct BaselineDecision {
    cells: Option<Vec<usize>>,
}

struct OracleStep {
    cells: Option<Vec<usize>>,
    fee: i64,
}

#[derive(Default)]
struct ReplayCheck {
    total: usize,
    decision_match: usize,
    both_accepted: usize,
    placement_set_match: usize,
    placement_order_match: usize,
    full_action_match: usize,
    exact_prefix: usize,
    first_mismatch: Option<usize>,
    baseline_accepted: usize,
    replay_accepted: usize,
}

struct ForkMeasurement {
    id: usize,
    margin: f64,
    base_threshold: f64,
    actual_threshold: f64,
    P: usize,
    D: usize,
    theta: f64,
    D_theta: f64,
    S: usize,
    S_ratio: f64,
    turn_ratio: f64,
    perimeter: usize,
    perimeter_slack: usize,
    current_fee: i64,
    component_size: usize,
    fragment_delta: f64,
    accept_fee: i64,
    reject_fee: i64,
    reject_advantage: i64,
    accept_count: usize,
    reject_count: usize,
    actual_future: usize,
}

struct SavedTargetFeatures {
    margin: f64,
    actual_threshold: f64,
    perimeter: usize,
    perimeter_slack: usize,
    component_size: usize,
    fragment_delta: f64,
}

struct CaseMeasurement {
    name: String,
    replay: ReplayCheck,
    forks: Vec<ForkMeasurement>,
}

struct Cli {
    input: std::path::PathBuf,
    baseline: std::path::PathBuf,
    future: usize,
    limit: Option<usize>,
}

impl Solver {
    fn oracle_clone(&self) -> Self {
        Self {
            N: self.N,
            M: self.M,
            grass_rows: self.grass_rows,
            occupied_rows: self.occupied_rows,
            owner_cell: self.owner_cell.clone(),
            groups: self.groups.clone(),
            shapes_by_p: self.shapes_by_p.clone(),
            p_probability: self.p_probability.clone(),
            p_cdf: self.p_cdf.clone(),
            departures: self.departures.clone(),
            duration_sum: self.duration_sum,
            duration_count: self.duration_count,
            expected_p: self.expected_p,
            compactness_bar: self.compactness_bar,
            effective_capacity: self.effective_capacity,
            threshold_cache: self.threshold_cache.clone(),
            c_max_table: self.c_max_table.clone(),
            timer: TimeKeeper::new(f64::INFINITY),
            #[cfg(feature = "local")]
            trace: TraceStats::default(),
        }
    }

    fn oracle_begin_arrival(&mut self, arrival: &OfflineArrival) -> (f64, f64) {
        self.remove_expired(arrival.S);
        self.groups[arrival.id] = Group {
            id: arrival.id,
            S: arrival.S,
            T: arrival.T,
            P: arrival.P,
            V: arrival.V,
            ..Group::default()
        };
        let duration = arrival.T - arrival.S;
        self.duration_sum += duration as f64;
        self.duration_count += 1;
        let theta = self.posterior_theta();
        let base_threshold = self.base_dynamic_threshold(arrival.S, duration, arrival.P, theta);
        (theta, base_threshold)
    }

    fn oracle_commit_fixed(&mut self, arrival: &OfflineArrival, cells: Option<&[usize]>) -> i64 {
        let Some(cells) = cells else {
            self.groups[arrival.id].active = false;
            self.groups[arrival.id].cells.clear();
            return 0;
        };
        assert_eq!(cells.len(), arrival.P, "group {} has wrong cell count", arrival.id);
        assert!(
            self.explicit_candidate_is_valid(cells, arrival.P, &self.occupied_rows),
            "saved placement is invalid at group {}",
            arrival.id
        );
        self.groups[arrival.id].cells = cells.to_vec();
        self.groups[arrival.id].active = true;
        self.place_group_on_board(arrival.id, cells);
        self.departures.push(Reverse((arrival.T, arrival.id)));
        let perimeter = self.perimeter_of_cells(cells);
        ((arrival.V as f64) * compactness(arrival.P, perimeter)).round() as i64
    }

    // This is v035::run's decision body with output and timer-dependent fast mode removed.
    fn oracle_decide_unlimited(&mut self, arrival: &OfflineArrival) -> OracleStep {
        let (theta, base_threshold) = self.oracle_begin_arrival(arrival);
        let duration = arrival.T - arrival.S;
        let q_value = (arrival.V as f64)
            / ((arrival.P as f64) * (duration as f64).powf(0.9));
        let is_large_target = arrival.P >= 96 && duration >= 6_000 && q_value >= 1.0;
        let optimistic_C = compactness(arrival.P, minimum_perimeter(arrival.P));
        let passed_price_prefilter =
            base_threshold == 0.0 || q_value * optimistic_C >= 0.74 * base_threshold;

        let mut accepted = false;
        let mut normal = None;
        if passed_price_prefilter {
            if let Some(mut choices) = self.find_normal_placements(
                arrival.id,
                arrival.P,
                arrival.V,
                arrival.S,
                arrival.T,
                theta,
                is_large_target,
                false,
            ) {
                let current_perimeter = choices[0].perimeter;
                let current_component_size = choices[0].component_size;
                let actual_threshold =
                    base_threshold * self.component_threshold_factor(current_component_size);
                let quality = q_value * compactness(arrival.P, current_perimeter);
                accepted = base_threshold == 0.0 || quality >= actual_threshold;
                let winner = if accepted && choices.len() >= 2 {
                    self.select_normal_by_rollout(&choices, arrival.id, theta, base_threshold)
                } else {
                    0
                };
                normal = Some(choices.swap_remove(winner));
            }
        }

        if accepted {
            let placement = normal.as_ref().expect("accepted placement");
            self.commit_normal_placement(arrival.id, placement);
            let cells = self.groups[arrival.id].cells.clone();
            let perimeter = self.perimeter_of_cells(&cells);
            let fee = ((arrival.V as f64) * compactness(arrival.P, perimeter)).round() as i64;
            OracleStep {
                cells: Some(cells),
                fee,
            }
        } else {
            self.groups[arrival.id].active = false;
            OracleStep {
                cells: None,
                fee: 0,
            }
        }
    }

    fn oracle_saved_features(
        &mut self,
        arrival: &OfflineArrival,
        cells: &[usize],
        base_threshold: f64,
    ) -> SavedTargetFeatures {
        assert!(base_threshold > 0.0);
        assert!(
            self.explicit_candidate_is_valid(cells, arrival.P, &self.occupied_rows),
            "saved target placement is invalid at group {}",
            arrival.id
        );
        let info = self.compute_free_info(&self.occupied_rows, false);
        let component_id = info.component[cells[0]];
        assert!(component_id >= 0, "target placement is not in a free component");
        let component_size = info.sizes[component_id as usize];
        let actual_threshold =
            base_threshold * self.component_threshold_factor(component_size);
        let duration = arrival.T - arrival.S;
        let q_value = (arrival.V as f64)
            / ((arrival.P as f64) * (duration as f64).powf(0.9));
        let perimeter = self.perimeter_of_cells(cells);
        let quality = q_value * compactness(arrival.P, perimeter);
        let mut next = self.occupied_rows;
        for &cell in cells {
            next[cell / self.N] |= Self::bit_at(cell % self.N);
        }
        let fragment_delta = self.fragment_metric(&next) - self.fragment_metric(&self.occupied_rows);
        SavedTargetFeatures {
            margin: quality / actual_threshold,
            actual_threshold,
            perimeter,
            perimeter_slack: perimeter - minimum_perimeter(arrival.P),
            component_size,
            fragment_delta,
        }
    }
}

fn parse_cli() -> Result<Cli, String> {
    let mut input = None;
    let mut baseline = None;
    let mut future = 64_usize;
    let mut limit = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                input = Some(std::path::PathBuf::from(
                    args.next().ok_or("--input requires a path")?,
                ));
            }
            "--baseline" => {
                baseline = Some(std::path::PathBuf::from(
                    args.next().ok_or("--baseline requires a path")?,
                ));
            }
            "--future" => {
                future = args
                    .next()
                    .ok_or("--future requires an integer")?
                    .parse()
                    .map_err(|_| "invalid --future value")?;
            }
            "--limit" => {
                limit = Some(
                    args.next()
                        .ok_or("--limit requires an integer")?
                        .parse()
                        .map_err(|_| "invalid --limit value")?,
                );
            }
            "-h" | "--help" => {
                println!("{}", std::env::var("ADMISSION_ORACLE_HELP").unwrap_or_default());
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    if future == 0 {
        return Err("--future must be positive".to_string());
    }
    Ok(Cli {
        input: input.ok_or("--input is required")?,
        baseline: baseline.ok_or("--baseline is required")?,
        future,
        limit,
    })
}

fn read_input(path: &std::path::Path) -> Result<OfflineInput, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut scanner = Scanner::new(std::io::BufReader::new(file));
    let N: usize = scanner.next();
    let M: usize = scanner.next();
    let _R: String = scanner.next();
    if N > MAX_N {
        return Err(format!("N={N} exceeds MAX_N"));
    }
    let mut grass_rows = [0_u64; MAX_N];
    for row_mask in grass_rows.iter_mut().take(N) {
        let row: String = scanner.next();
        if row.len() != N {
            return Err("invalid park row length".to_string());
        }
        for (c, byte) in row.bytes().enumerate() {
            if byte == b'.' {
                *row_mask |= Solver::bit_at(c);
            }
        }
    }
    let mut arrivals = Vec::with_capacity(M);
    for turn in 0..M {
        let arrival = OfflineArrival {
            id: scanner.next(),
            S: scanner.next(),
            T: scanner.next(),
            P: scanner.next(),
            V: scanner.next(),
        };
        if arrival.id != turn {
            return Err(format!("expected group {turn}, got {}", arrival.id));
        }
        arrivals.push(arrival);
    }
    Ok(OfflineInput {
        N,
        M,
        grass_rows,
        arrivals,
    })
}

fn read_baseline(
    path: &std::path::Path,
    input: &OfflineInput,
) -> Result<Vec<BaselineDecision>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut tokens = text.split_whitespace();
    let mut decisions = Vec::with_capacity(input.M);
    for arrival in &input.arrivals {
        let moves: usize = tokens
            .next()
            .ok_or_else(|| format!("baseline ended before group {} moves", arrival.id))?
            .parse()
            .map_err(|_| format!("invalid move count at group {}", arrival.id))?;
        if moves != 0 {
            return Err(format!(
                "baseline group {} has {moves} moves; v035 must have zero",
                arrival.id
            ));
        }
        let answer = tokens
            .next()
            .ok_or_else(|| format!("baseline ended before group {} answer", arrival.id))?;
        let cells = match answer {
            "No" => None,
            "Yes" => {
                let mut cells = Vec::with_capacity(arrival.P);
                for _ in 0..arrival.P {
                    let r: usize = tokens
                        .next()
                        .ok_or("baseline ended inside placement")?
                        .parse()
                        .map_err(|_| "invalid placement row")?;
                    let c: usize = tokens
                        .next()
                        .ok_or("baseline ended inside placement")?
                        .parse()
                        .map_err(|_| "invalid placement column")?;
                    if r >= input.N || c >= input.N {
                        return Err(format!("out-of-range cell ({r},{c}) at group {}", arrival.id));
                    }
                    cells.push(r * input.N + c);
                }
                Some(cells)
            }
            _ => return Err(format!("invalid answer {answer:?} at group {}", arrival.id)),
        };
        decisions.push(BaselineDecision { cells });
    }
    if let Some(extra) = tokens.next() {
        return Err(format!("unexpected trailing baseline token: {extra:?}"));
    }
    Ok(decisions)
}

fn same_cell_set(a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

fn replay_check(input: &OfflineInput, baseline: &[BaselineDecision]) -> ReplayCheck {
    let mut solver = Solver::new(
        input.N,
        input.M,
        input.grass_rows,
        TimeKeeper::new(f64::INFINITY),
    );
    let mut check = ReplayCheck::default();
    let mut prefix_open = true;
    for (arrival, saved) in input.arrivals.iter().zip(baseline) {
        let replay = solver.oracle_decide_unlimited(arrival);
        check.total += 1;
        check.baseline_accepted += saved.cells.is_some() as usize;
        check.replay_accepted += replay.cells.is_some() as usize;
        let decision_match = saved.cells.is_some() == replay.cells.is_some();
        check.decision_match += decision_match as usize;
        let mut set_match = false;
        let mut order_match = false;
        if let (Some(saved_cells), Some(replay_cells)) = (&saved.cells, &replay.cells) {
            check.both_accepted += 1;
            set_match = same_cell_set(saved_cells, replay_cells);
            order_match = saved_cells == replay_cells;
            check.placement_set_match += set_match as usize;
            check.placement_order_match += order_match as usize;
        }
        let full_match = decision_match && (saved.cells.is_none() || set_match);
        check.full_action_match += full_match as usize;
        if prefix_open {
            if full_match {
                check.exact_prefix += 1;
            } else {
                check.first_mismatch = Some(arrival.id);
                prefix_open = false;
            }
        }
    }
    check
}

fn measure_case(
    name: String,
    input: OfflineInput,
    baseline: Vec<BaselineDecision>,
    future: usize,
) -> CaseMeasurement {
    let replay = replay_check(&input, &baseline);
    let mut fixed = Solver::new(
        input.N,
        input.M,
        input.grass_rows,
        TimeKeeper::new(f64::INFINITY),
    );
    let mut forks = Vec::new();

    for id in 0..input.M {
        let arrival = &input.arrivals[id];
        let (theta, base_threshold) = fixed.oracle_begin_arrival(arrival);
        let saved_cells = baseline[id].cells.as_deref();

        if let Some(cells) = saved_cells {
            if base_threshold > 0.0 {
                let features = fixed.oracle_saved_features(arrival, cells, base_threshold);
                if features.margin >= 1.0 - 1e-12 && features.margin <= 1.30 + 1e-12 {
                    let mut accept = fixed.oracle_clone();
                    let mut reject = fixed.oracle_clone();
                    let current_fee = accept.oracle_commit_fixed(arrival, Some(cells));
                    let mut accept_fee = current_fee;
                    let mut reject_fee = reject.oracle_commit_fixed(arrival, None);
                    let mut accept_count = 1_usize;
                    let mut reject_count = 0_usize;
                    let future_end = (id + 1 + future).min(input.M);
                    for next in (id + 1)..future_end {
                        let accept_step = accept.oracle_decide_unlimited(&input.arrivals[next]);
                        accept_fee += accept_step.fee;
                        accept_count += accept_step.cells.is_some() as usize;
                        let reject_step = reject.oracle_decide_unlimited(&input.arrivals[next]);
                        reject_fee += reject_step.fee;
                        reject_count += reject_step.cells.is_some() as usize;
                    }
                    forks.push(ForkMeasurement {
                        id,
                        margin: features.margin,
                        base_threshold,
                        actual_threshold: features.actual_threshold,
                        P: arrival.P,
                        D: arrival.T - arrival.S,
                        theta,
                        D_theta: ((arrival.T - arrival.S) as f64) / theta,
                        S: arrival.S,
                        S_ratio: (arrival.S as f64) / (HORIZON as f64),
                        turn_ratio: (id as f64) / (input.M as f64),
                        perimeter: features.perimeter,
                        perimeter_slack: features.perimeter_slack,
                        current_fee,
                        component_size: features.component_size,
                        fragment_delta: features.fragment_delta,
                        accept_fee,
                        reject_fee,
                        reject_advantage: reject_fee - accept_fee,
                        accept_count,
                        reject_count,
                        actual_future: future_end - id - 1,
                    });
                }
            }
        }

        fixed.oracle_commit_fixed(arrival, saved_cells);
    }

    CaseMeasurement {
        name,
        replay,
        forks,
    }
}

fn collect_cases(cli: &Cli) -> Result<Vec<(String, std::path::PathBuf, std::path::PathBuf)>, String> {
    let input_is_file = cli.input.is_file();
    let baseline_is_file = cli.baseline.is_file();
    if input_is_file != baseline_is_file {
        return Err("--input and --baseline must both be files or both be directories".to_string());
    }
    if input_is_file {
        let name = cli
            .input
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("case")
            .to_string();
        return Ok(vec![(name, cli.input.clone(), cli.baseline.clone())]);
    }
    if !cli.input.is_dir() || !cli.baseline.is_dir() {
        return Err("input or baseline path does not exist".to_string());
    }
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(&cli.input).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("txt") {
            continue;
        }
        let baseline = cli.baseline.join(path.file_name().expect("input filename"));
        if baseline.is_file() {
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("case")
                .to_string();
            cases.push((name, path, baseline));
        }
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(limit) = cli.limit {
        cases.truncate(limit);
    }
    if cases.is_empty() {
        return Err("no matching *.txt cases found".to_string());
    }
    Ok(cases)
}

fn quantile(values: &[i64], q: f64) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (q * ((sorted.len() - 1) as f64)).round() as usize;
    Some(sorted[index])
}

fn print_quantiles(prefix: &str, values: &[i64]) {
    if values.is_empty() {
        println!("{prefix} q00=NA q25=NA q50=NA q75=NA q90=NA q95=NA q99=NA q100=NA");
        return;
    }
    println!(
        "{prefix} q00={} q25={} q50={} q75={} q90={} q95={} q99={} q100={}",
        quantile(values, 0.00).unwrap(),
        quantile(values, 0.25).unwrap(),
        quantile(values, 0.50).unwrap(),
        quantile(values, 0.75).unwrap(),
        quantile(values, 0.90).unwrap(),
        quantile(values, 0.95).unwrap(),
        quantile(values, 0.99).unwrap(),
        quantile(values, 1.00).unwrap(),
    );
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        (numerator as f64) / (denominator as f64)
    }
}

#[derive(Default)]
struct BandStats {
    targets: usize,
    reject_better: usize,
    positive_advantage_sum: i64,
    signed_advantage_sum: i64,
}

impl BandStats {
    fn add(&mut self, fork: &ForkMeasurement) {
        self.targets += 1;
        self.reject_better += (fork.reject_advantage > 0) as usize;
        self.positive_advantage_sum += fork.reject_advantage.max(0);
        self.signed_advantage_sum += fork.reject_advantage;
    }
}

fn print_band_dimension<F>(
    dimension: &str,
    labels: &[&str],
    forks: &[&ForkMeasurement],
    classify: F,
) where
    F: Fn(&ForkMeasurement) -> usize,
{
    let mut stats: Vec<BandStats> = (0..labels.len()).map(|_| BandStats::default()).collect();
    for &fork in forks {
        stats[classify(fork)].add(fork);
    }
    for (label, stat) in labels.iter().zip(stats) {
        println!(
            "[overall.band] dimension={} band={} targets={} reject_better={} positive_advantage_sum={} signed_advantage_sum={}",
            dimension,
            label,
            stat.targets,
            stat.reject_better,
            stat.positive_advantage_sum,
            stat.signed_advantage_sum,
        );
    }
}

fn print_overall_bands(forks: &[&ForkMeasurement]) {
    print_band_dimension(
        "margin",
        &[
            "1.00-1.02",
            "1.02-1.04",
            "1.04-1.06",
            "1.06-1.08",
            "1.08-1.10",
            "1.10-1.12",
            "1.12-1.14",
            "1.14-1.16",
            "1.16-1.18",
            "1.18-1.20",
            "1.20-1.22",
            "1.22-1.24",
            "1.24-1.26",
            "1.26-1.28",
            "1.28-1.30",
        ],
        forks,
        |fork| {
            const UPPER_BOUNDS: [f64; 14] = [
                1.02, 1.04, 1.06, 1.08, 1.10, 1.12, 1.14, 1.16, 1.18, 1.20, 1.22, 1.24,
                1.26, 1.28,
            ];
            UPPER_BOUNDS
                .iter()
                .position(|&upper| fork.margin < upper)
                .unwrap_or(14)
        },
    );
    print_band_dimension(
        "perimeter_slack",
        &["0-2", "4-6", "8-14", "16-30", "32+"],
        forks,
        |fork| {
            if fork.perimeter_slack <= 2 {
                0
            } else if fork.perimeter_slack <= 6 {
                1
            } else if fork.perimeter_slack <= 14 {
                2
            } else if fork.perimeter_slack <= 30 {
                3
            } else {
                4
            }
        },
    );
    print_band_dimension(
        "P",
        &["4-31", "32-63", "64-95", "96+"],
        forks,
        |fork| {
            if fork.P <= 31 {
                0
            } else if fork.P <= 63 {
                1
            } else if fork.P <= 95 {
                2
            } else {
                3
            }
        },
    );
    print_band_dimension(
        "D_theta",
        &["0-0.5", "0.5-1", "1-2", "2+"],
        forks,
        |fork| {
            if fork.D_theta < 0.5 {
                0
            } else if fork.D_theta < 1.0 {
                1
            } else if fork.D_theta < 2.0 {
                2
            } else {
                3
            }
        },
    );
    print_band_dimension(
        "turn_ratio",
        &["0-0.25", "0.25-0.50", "0.50-0.75", "0.75-1.00"],
        forks,
        |fork| {
            if fork.turn_ratio < 0.25 {
                0
            } else if fork.turn_ratio < 0.50 {
                1
            } else if fork.turn_ratio < 0.75 {
                2
            } else {
                3
            }
        },
    );
}

fn print_case(measurement: &CaseMeasurement, future: usize) {
    let replay = &measurement.replay;
    println!(
        "[replay] case={} total={} decision_match={} decision_match_rate={:.6} both_accepted={} placement_set_match={} placement_set_match_rate={:.6} placement_order_match={} full_action_match={} full_action_match_rate={:.6} exact_prefix={} first_mismatch={} baseline_accepted={} replay_accepted={} mode=unlimited_time",
        measurement.name,
        replay.total,
        replay.decision_match,
        ratio(replay.decision_match, replay.total),
        replay.both_accepted,
        replay.placement_set_match,
        ratio(replay.placement_set_match, replay.both_accepted),
        replay.placement_order_match,
        replay.full_action_match,
        ratio(replay.full_action_match, replay.total),
        replay.exact_prefix,
        replay.first_mismatch.map_or_else(|| "none".to_string(), |id| id.to_string()),
        replay.baseline_accepted,
        replay.replay_accepted,
    );
    for fork in &measurement.forks {
        println!(
            "[target] case={} id={} margin={:.9} base_threshold={:.9} actual_threshold={:.9} future={} accept_fee={} reject_fee={} reject_advantage={} accept_count={} reject_count={} P={} D={} theta={:.3} D_theta={:.6} S={} S_ratio={:.6} turn_ratio={:.6} perimeter={} perimeter_slack={} current_fee={} component_size={} fragment_delta={:.6}",
            measurement.name,
            fork.id,
            fork.margin,
            fork.base_threshold,
            fork.actual_threshold,
            fork.actual_future,
            fork.accept_fee,
            fork.reject_fee,
            fork.reject_advantage,
            fork.accept_count,
            fork.reject_count,
            fork.P,
            fork.D,
            fork.theta,
            fork.D_theta,
            fork.S,
            fork.S_ratio,
            fork.turn_ratio,
            fork.perimeter,
            fork.perimeter_slack,
            fork.current_fee,
            fork.component_size,
            fork.fragment_delta,
        );
    }
    let advantages: Vec<i64> = measurement
        .forks
        .iter()
        .map(|fork| fork.reject_advantage)
        .collect();
    let reject_better = advantages.iter().filter(|&&value| value > 0).count();
    let upper_bound: i64 = advantages.iter().map(|&value| value.max(0)).sum();
    let signed_sum: i64 = advantages.iter().sum();
    println!(
        "[case] case={} requested_future={} targets={} reject_better={} optimistic_upper_bound={} signed_advantage_sum={} mean_advantage_per_target={:.3}",
        measurement.name,
        future,
        advantages.len(),
        reject_better,
        upper_bound,
        signed_sum,
        if advantages.is_empty() { 0.0 } else { (signed_sum as f64) / (advantages.len() as f64) },
    );
    print_quantiles(
        &format!("[case.quantile] case={} metric=reject_advantage method=nearest_index_round", measurement.name),
        &advantages,
    );
}

fn oracle_main() -> Result<(), String> {
    let cli = parse_cli()?;
    let cases = collect_cases(&cli)?;
    println!(
        "[meta] baseline_solver=v035_no_move_growth_cutloss source_fnv1a64={} cases={} requested_future={} target_margin_min=1.00 target_margin_max=1.30 baseline_mode=saved_output fork_mode=unlimited_time move_count=0",
        std::env::var("ADMISSION_ORACLE_SOURCE_FNV").unwrap_or_else(|_| "unknown".to_string()),
        cases.len(),
        cli.future,
    );
    println!(
        "[meta] replay_interpretation=saved_output_is_authoritative; unlimited replay mismatch can reflect v035 wall-clock cutoffs or source/output drift; forks remain anchored to the saved pre-decision state"
    );
    println!(
        "[meta] band_intervals=lower_inclusive_upper_exclusive_except_final D_theta=D/posterior_theta turn_ratio=id/M S_ratio=S/HORIZON"
    );

    let mut measurements = Vec::with_capacity(cases.len());
    for (name, input_path, baseline_path) in cases {
        eprintln!("[progress] case={name}");
        let input = read_input(&input_path)?;
        let baseline = read_baseline(&baseline_path, &input)?;
        let measurement = measure_case(name, input, baseline, cli.future);
        print_case(&measurement, cli.future);
        measurements.push(measurement);
    }

    let all_forks: Vec<&ForkMeasurement> = measurements
        .iter()
        .flat_map(|measurement| measurement.forks.iter())
        .collect();
    let all_advantages: Vec<i64> = all_forks
        .iter()
        .map(|fork| fork.reject_advantage)
        .collect();
    let total_upper_bound: i64 = all_advantages.iter().map(|&value| value.max(0)).sum();
    let reject_better = all_advantages.iter().filter(|&&value| value > 0).count();
    let decision_total: usize = measurements.iter().map(|value| value.replay.total).sum();
    let decision_match: usize = measurements
        .iter()
        .map(|value| value.replay.decision_match)
        .sum();
    let action_match: usize = measurements
        .iter()
        .map(|value| value.replay.full_action_match)
        .sum();
    println!(
        "[overall] cases={} targets={} reject_better={} optimistic_upper_bound_sum={} case_average_upper_bound={:.3} replay_decision_match_rate={:.6} replay_full_action_match_rate={:.6}",
        measurements.len(),
        all_advantages.len(),
        reject_better,
        total_upper_bound,
        (total_upper_bound as f64) / (measurements.len() as f64),
        ratio(decision_match, decision_total),
        ratio(action_match, decision_total),
    );
    print_quantiles(
        "[overall.quantile] metric=reject_advantage method=nearest_index_round",
        &all_advantages,
    );
    print_overall_bands(&all_forks);
    Ok(())
}

fn main() {
    if let Err(error) = oracle_main() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}
"#;

struct TempDir {
    path: PathBuf,
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn make_temp_dir() -> io::Result<TempDir> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "ahc069-admission-fork-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path)?;
    Ok(TempDir { path })
}

fn fnv1a64(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn statrs_candidates(deps: &Path) -> io::Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(deps)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with("libstatrs-") && name.ends_with(".rlib") {
            let modified = fs::metadata(&path)?.modified().unwrap_or(UNIX_EPOCH);
            candidates.push((modified, path));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(candidates.into_iter().map(|(_, path)| path).collect())
}

fn compile_inner(source: &Path, output: &Path) -> io::Result<ExitStatus> {
    let current_exe = env::current_exe()?;
    let target_dir = current_exe
        .parent()
        .ok_or_else(|| io::Error::other("current executable has no parent"))?;
    let deps = target_dir.join("deps");
    let candidates = statrs_candidates(&deps)?;
    if candidates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no libstatrs-*.rlib found in {}", deps.display()),
        ));
    }

    let mut last_stderr = Vec::new();
    for statrs in candidates {
        let result = Command::new(env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc")))
            .arg(source)
            .arg("--crate-name=probe_admission_fork_inner")
            .arg("--edition=2024")
            .arg("-O")
            .arg("--cap-lints=allow")
            .arg("-L")
            .arg(format!("dependency={}", deps.display()))
            .arg("--extern")
            .arg(format!("statrs={}", statrs.display()))
            .arg("-o")
            .arg(output)
            .output()?;
        if result.status.success() {
            return Ok(result.status);
        }
        last_stderr = result.stderr;
    }
    Err(io::Error::other(format!(
        "failed to compile instrumented v035:\n{}",
        String::from_utf8_lossy(&last_stderr)
    )))
}

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        print!("{HELP}");
        return Ok(());
    }

    let renamed = V035_SOURCE.replacen(
        "fn main() -> io::Result<()> {",
        "fn original_solver_main() -> io::Result<()> {",
        1,
    );
    if renamed.len() == V035_SOURCE.len() {
        return Err("could not locate v035 main; source layout changed".into());
    }
    let generated = format!("{renamed}{ORACLE_HARNESS}");
    let temp = make_temp_dir()?;
    let source = temp.path.join("probe_admission_fork_inner.rs");
    let binary = temp.path.join("probe_admission_fork_inner");
    fs::write(&source, generated)?;
    compile_inner(&source, &binary)?;

    let status = Command::new(binary)
        .args(env::args_os().skip(1))
        .env("ADMISSION_ORACLE_HELP", HELP)
        .env(
            "ADMISSION_ORACLE_SOURCE_FNV",
            format!("{:016x}", fnv1a64(V035_SOURCE)),
        )
        .status()?;
    if !status.success() {
        return Err(format!("oracle process exited with {status}").into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = real_main() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
