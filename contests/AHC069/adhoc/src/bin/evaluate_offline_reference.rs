// evaluate_offline_reference.rs
#![allow(non_snake_case)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Clone)]
struct Config {
    seconds: f64,
    jobs: usize,
    seed: u64,
    input_dir: PathBuf,
    output_dir: PathBuf,
    record_dir: PathBuf,
    log_dir: PathBuf,
    summary_path: PathBuf,
    solver_path: PathBuf,
    scorer_path: PathBuf,
    resume: bool,
}

#[derive(Clone, Debug)]
struct Record {
    case_name: String,
    score: i64,
    internal_score: i64,
    base_score: i64,
    initial_score: i64,
    optimistic_selected: i64,
    shape_loss: i64,
    accepted: usize,
    extra_accepted: usize,
    selected_cell_time: u64,
    moves: usize,
    move_cost: i64,
    episodes: usize,
    static_added: usize,
    builds: usize,
    lns_iterations: usize,
    dynamic_candidates: usize,
    elapsed_ms: u128,
    wall_elapsed_ms: u128,
    seconds: f64,
    seed: u64,
    output_path: PathBuf,
}

impl Record {
    const HEADER: &'static str = "case,score,internal_score,base_score,initial_score,optimistic_selected,shape_loss,accepted,extra_accepted,selected_cell_time,moves,move_cost,episodes,static_added,builds,lns_iterations,dynamic_candidates,elapsed_ms,wall_elapsed_ms,seconds,seed,output";

    fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{},{}",
            self.case_name,
            self.score,
            self.internal_score,
            self.base_score,
            self.initial_score,
            self.optimistic_selected,
            self.shape_loss,
            self.accepted,
            self.extra_accepted,
            self.selected_cell_time,
            self.moves,
            self.move_cost,
            self.episodes,
            self.static_added,
            self.builds,
            self.lns_iterations,
            self.dynamic_candidates,
            self.elapsed_ms,
            self.wall_elapsed_ms,
            self.seconds,
            self.seed,
            self.output_path.display(),
        )
    }

    fn from_csv(line: &str) -> Result<Self, String> {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.len() != 22 {
            return Err(format!("record列数が{}である", fields.len()));
        }
        let parse = |index: usize, name: &str| {
            fields[index]
                .parse::<i64>()
                .map_err(|error| format!("{name}が不正: {error}"))
        };
        Ok(Self {
            case_name: fields[0].to_string(),
            score: parse(1, "score")?,
            internal_score: parse(2, "internal_score")?,
            base_score: parse(3, "base_score")?,
            initial_score: parse(4, "initial_score")?,
            optimistic_selected: parse(5, "optimistic_selected")?,
            shape_loss: parse(6, "shape_loss")?,
            accepted: fields[7]
                .parse()
                .map_err(|error| format!("acceptedが不正: {error}"))?,
            extra_accepted: fields[8]
                .parse()
                .map_err(|error| format!("extra_acceptedが不正: {error}"))?,
            selected_cell_time: fields[9]
                .parse()
                .map_err(|error| format!("selected_cell_timeが不正: {error}"))?,
            moves: fields[10]
                .parse()
                .map_err(|error| format!("movesが不正: {error}"))?,
            move_cost: parse(11, "move_cost")?,
            episodes: fields[12]
                .parse()
                .map_err(|error| format!("episodesが不正: {error}"))?,
            static_added: fields[13]
                .parse()
                .map_err(|error| format!("static_addedが不正: {error}"))?,
            builds: fields[14]
                .parse()
                .map_err(|error| format!("buildsが不正: {error}"))?,
            lns_iterations: fields[15]
                .parse()
                .map_err(|error| format!("lns_iterationsが不正: {error}"))?,
            dynamic_candidates: fields[16]
                .parse()
                .map_err(|error| format!("dynamic_candidatesが不正: {error}"))?,
            elapsed_ms: fields[17]
                .parse()
                .map_err(|error| format!("elapsed_msが不正: {error}"))?,
            wall_elapsed_ms: fields[18]
                .parse()
                .map_err(|error| format!("wall_elapsed_msが不正: {error}"))?,
            seconds: fields[19]
                .parse()
                .map_err(|error| format!("secondsが不正: {error}"))?,
            seed: fields[20]
                .parse()
                .map_err(|error| format!("seedが不正: {error}"))?,
            output_path: PathBuf::from(fields[21]),
        })
    }
}

fn usage() -> &'static str {
    "usage: evaluate_offline_reference [--seconds <f64>] [--jobs <usize>] [--seed <u64>] [--no-resume]"
}

fn parse_args() -> Result<Config, String> {
    let mut seconds = 60.0_f64;
    let mut jobs = thread::available_parallelism().map_or(1, usize::from);
    let mut seed = 0_u64;
    let mut resume = true;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seconds" => {
                seconds = args
                    .next()
                    .ok_or_else(|| "--secondsの値がない".to_string())?
                    .parse()
                    .map_err(|error| format!("--secondsが不正: {error}"))?;
            }
            "--jobs" => {
                jobs = args
                    .next()
                    .ok_or_else(|| "--jobsの値がない".to_string())?
                    .parse()
                    .map_err(|error| format!("--jobsが不正: {error}"))?;
            }
            "--seed" => {
                seed = args
                    .next()
                    .ok_or_else(|| "--seedの値がない".to_string())?
                    .parse()
                    .map_err(|error| format!("--seedが不正: {error}"))?;
            }
            "--no-resume" => resume = false,
            "-h" | "--help" => return Err(usage().to_string()),
            _ => return Err(format!("未知の引数: {arg}\n{}", usage())),
        }
    }
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("--secondsは正の有限値にする".to_string());
    }
    if jobs == 0 {
        return Err("--jobsは1以上にする".to_string());
    }

    Ok(Config {
        seconds,
        jobs,
        seed,
        input_dir: PathBuf::from("tools/in"),
        output_dir: PathBuf::from("results/out/v_01_offline_reference"),
        record_dir: PathBuf::from("results/offline_reference/records"),
        log_dir: PathBuf::from("results/offline_reference/logs"),
        summary_path: PathBuf::from("results/offline_reference/practical_scores.csv"),
        solver_path: PathBuf::from("adhoc/target/release/offline_reference"),
        scorer_path: PathBuf::from("tools/target/release/score"),
        resume,
    })
}

fn list_inputs(input_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut inputs = fs::read_dir(input_dir)
        .map_err(|error| format!("{}を読めない: {error}", input_dir.display()))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("入力一覧を読めない: {error}"))?;
    inputs.retain(|path| path.is_file());
    inputs.sort();
    if inputs.is_empty() {
        return Err(format!("{}に入力がない", input_dir.display()));
    }
    Ok(inputs)
}

fn official_score(
    scorer_path: &Path,
    input_path: &Path,
    output_path: &Path,
) -> Result<i64, String> {
    let result = Command::new(scorer_path)
        .arg(input_path)
        .arg(output_path)
        .output()
        .map_err(|error| format!("公式scorerを起動できない: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "公式scorer失敗: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&result.stdout)
        .trim()
        .parse::<i64>()
        .map_err(|error| format!("公式scoreが不正: {error}"))
}

fn read_resume_record(
    config: &Config,
    input_path: &Path,
    case_name: &str,
) -> Result<Option<Record>, String> {
    if !config.resume {
        return Ok(None);
    }
    let record_path = config.record_dir.join(format!("{case_name}.csv"));
    let output_path = config.output_dir.join(
        input_path
            .file_name()
            .ok_or_else(|| "入力basenameがない".to_string())?,
    );
    if !record_path.is_file() || !output_path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&record_path)
        .map_err(|error| format!("{}を読めない: {error}", record_path.display()))?;
    let mut lines = text.lines();
    if lines.next() != Some(Record::HEADER) {
        return Ok(None);
    }
    let Some(line) = lines.next() else {
        return Ok(None);
    };
    let record = Record::from_csv(line)?;
    if record.case_name != case_name
        || (record.seconds - config.seconds).abs() > 1.0e-9
        || record.seed != config.seed
    {
        return Ok(None);
    }
    let score = official_score(&config.scorer_path, input_path, &output_path)?;
    if score != record.score {
        return Ok(None);
    }
    Ok(Some(record))
}

fn parse_solver_csv(stdout: &str) -> Result<Vec<&str>, String> {
    let mut lines = stdout.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "offline solverのCSV headerがない".to_string())?;
    if header
        != "case,score,base_score,initial_score,optimistic_selected,shape_loss,accepted,extra_accepted,selected_cell_time,moves,move_cost,episodes,static_added,builds,lns_iterations,dynamic_candidates,elapsed_ms"
    {
        return Err(format!("offline solverのCSV headerが不正: {header}"));
    }
    let values = lines
        .next()
        .ok_or_else(|| "offline solverのCSV値がない".to_string())?
        .split(',')
        .collect::<Vec<_>>();
    if values.len() != 17 || lines.next().is_some() {
        return Err("offline solverのCSV値の列数または行数が不正".to_string());
    }
    Ok(values)
}

fn run_case(config: &Config, input_path: &Path, worker_id: usize) -> Result<Record, String> {
    let case_name = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("入力名が不正: {}", input_path.display()))?;
    if let Some(record) = read_resume_record(config, input_path, case_name)? {
        return Ok(record);
    }

    let file_name = input_path
        .file_name()
        .ok_or_else(|| "入力basenameがない".to_string())?;
    let output_path = config.output_dir.join(file_name);
    let temporary_output = config.record_dir.join(format!(
        ".{case_name}.{}.{}.tmp",
        std::process::id(),
        worker_id
    ));
    let started = Instant::now();
    let result = Command::new(&config.solver_path)
        .arg(input_path)
        .arg("--seconds")
        .arg(config.seconds.to_string())
        .arg("--seed")
        .arg(config.seed.to_string())
        .arg("--output")
        .arg(&temporary_output)
        .output()
        .map_err(|error| format!("offline solverを起動できない: {error}"))?;
    let wall_elapsed_ms = started.elapsed().as_millis();

    let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    let log_path = config.log_dir.join(format!("{case_name}.log"));
    fs::write(&log_path, format!("[stdout]\n{stdout}\n[stderr]\n{stderr}"))
        .map_err(|error| format!("{}を書けない: {error}", log_path.display()))?;
    if !result.status.success() {
        return Err(format!(
            "offline solver失敗 (status={}): {}",
            result.status,
            stderr.trim()
        ));
    }
    if !temporary_output.is_file() {
        return Err("offline solverが出力ファイルを作らなかった".to_string());
    }

    let values = parse_solver_csv(&stdout)?;
    if values[0] != case_name {
        return Err(format!("case名不一致: {} != {case_name}", values[0]));
    }
    let parse_i64 = |index: usize, name: &str| {
        values[index]
            .parse::<i64>()
            .map_err(|error| format!("{name}が不正: {error}"))
    };
    let internal_score = parse_i64(1, "internal_score")?;
    let score = official_score(&config.scorer_path, input_path, &temporary_output)?;
    if score != internal_score {
        return Err(format!(
            "内部scoreと公式scoreが不一致: internal={internal_score}, official={score}"
        ));
    }

    fs::rename(&temporary_output, &output_path).map_err(|error| {
        format!(
            "{}から{}へ出力を移せない: {error}",
            temporary_output.display(),
            output_path.display()
        )
    })?;

    let record = Record {
        case_name: case_name.to_string(),
        score,
        internal_score,
        base_score: parse_i64(2, "base_score")?,
        initial_score: parse_i64(3, "initial_score")?,
        optimistic_selected: parse_i64(4, "optimistic_selected")?,
        shape_loss: parse_i64(5, "shape_loss")?,
        accepted: values[6]
            .parse()
            .map_err(|error| format!("acceptedが不正: {error}"))?,
        extra_accepted: values[7]
            .parse()
            .map_err(|error| format!("extra_acceptedが不正: {error}"))?,
        selected_cell_time: values[8]
            .parse()
            .map_err(|error| format!("selected_cell_timeが不正: {error}"))?,
        moves: values[9]
            .parse()
            .map_err(|error| format!("movesが不正: {error}"))?,
        move_cost: parse_i64(10, "move_cost")?,
        episodes: values[11]
            .parse()
            .map_err(|error| format!("episodesが不正: {error}"))?,
        static_added: values[12]
            .parse()
            .map_err(|error| format!("static_addedが不正: {error}"))?,
        builds: values[13]
            .parse()
            .map_err(|error| format!("buildsが不正: {error}"))?,
        lns_iterations: values[14]
            .parse()
            .map_err(|error| format!("lns_iterationsが不正: {error}"))?,
        dynamic_candidates: values[15]
            .parse()
            .map_err(|error| format!("dynamic_candidatesが不正: {error}"))?,
        elapsed_ms: values[16]
            .parse()
            .map_err(|error| format!("elapsed_msが不正: {error}"))?,
        wall_elapsed_ms,
        seconds: config.seconds,
        seed: config.seed,
        output_path,
    };
    let record_path = config.record_dir.join(format!("{case_name}.csv"));
    fs::write(
        &record_path,
        format!("{}\n{}\n", Record::HEADER, record.to_csv()),
    )
    .map_err(|error| format!("{}を書けない: {error}", record_path.display()))?;
    Ok(record)
}

fn write_summary(path: &Path, records: &[Record]) -> Result<(), String> {
    let mut csv = String::from(Record::HEADER);
    csv.push('\n');
    for record in records {
        csv.push_str(&record.to_csv());
        csv.push('\n');
    }
    fs::write(path, csv).map_err(|error| format!("{}を書けない: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let config = Config::parse_args_or_error()?;
    if !config.solver_path.is_file() {
        return Err(format!(
            "{}がない。先にoffline_referenceをrelease buildする",
            config.solver_path.display()
        ));
    }
    if !config.scorer_path.is_file() {
        return Err(format!(
            "{}がない。先に公式scoreをrelease buildする",
            config.scorer_path.display()
        ));
    }
    fs::create_dir_all(&config.output_dir)
        .map_err(|error| format!("{}を作れない: {error}", config.output_dir.display()))?;
    fs::create_dir_all(&config.record_dir)
        .map_err(|error| format!("{}を作れない: {error}", config.record_dir.display()))?;
    fs::create_dir_all(&config.log_dir)
        .map_err(|error| format!("{}を作れない: {error}", config.log_dir.display()))?;
    if let Some(parent) = config.summary_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("{}を作れない: {error}", parent.display()))?;
    }

    let inputs = Arc::new(list_inputs(&config.input_dir)?);
    let total = inputs.len();
    let next = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let results = Arc::new(Mutex::new(
        (0..total)
            .map(|_| None)
            .collect::<Vec<Option<Result<Record, String>>>>(),
    ));
    let shared_config = Arc::new(config.clone());
    let mut handles = Vec::new();
    for worker_id in 0..config.jobs.min(total) {
        let inputs = Arc::clone(&inputs);
        let next = Arc::clone(&next);
        let completed = Arc::clone(&completed);
        let results = Arc::clone(&results);
        let config = Arc::clone(&shared_config);
        handles.push(thread::spawn(move || {
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= inputs.len() {
                    break;
                }
                let result = run_case(&config, &inputs[index], worker_id);
                let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                match &result {
                    Ok(record) => eprintln!(
                        "[{count}/{}] {} score={} elapsed={}ms",
                        inputs.len(),
                        record.case_name,
                        record.score,
                        record.elapsed_ms
                    ),
                    Err(error) => eprintln!(
                        "[{count}/{}] {} FAILED: {error}",
                        inputs.len(),
                        inputs[index].display()
                    ),
                }
                results.lock().expect("results lock")[index] = Some(result);
            }
        }));
    }
    for handle in handles {
        handle
            .join()
            .map_err(|_| "worker threadがpanicした".to_string())?;
    }

    let mut records = Vec::with_capacity(total);
    let mut failures = Vec::new();
    for (index, result) in results.lock().expect("results lock").iter().enumerate() {
        match result {
            Some(Ok(record)) => records.push(record.clone()),
            Some(Err(error)) => failures.push(format!("{}: {error}", inputs[index].display())),
            None => failures.push(format!("{}: 未実行", inputs[index].display())),
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "{}ケース失敗\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }
    write_summary(&config.summary_path, &records)?;
    let score_sum = records
        .iter()
        .map(|record| record.score as i128)
        .sum::<i128>();
    let score_avg = score_sum / records.len() as i128;
    println!(
        "cases={},score_sum={},score_avg={},summary={}",
        records.len(),
        score_sum,
        score_avg,
        config.summary_path.display()
    );
    Ok(())
}

impl Config {
    fn parse_args_or_error() -> Result<Self, String> {
        parse_args()
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
