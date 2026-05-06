// tune_v019_sa_temperature.rs
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_SOURCE: &str = "src/bin/v019_sa_pool.rs";
const TEMP_BIN: &str = "v019_sa_temp_tune";
const DEFAULT_TIMEOUT_MS: u64 = 3000;
const DEFAULT_TEMPS: &str = "0.25:0.02,0.5:0.03,1.0:0.05,2.0:0.08,4.0:0.125,8.0:0.25,16.0:0.5";
const DEFAULT_CASES: &[&str] = &[
    "0091.txt", "0080.txt", "0064.txt", "0083.txt", "0081.txt", "0073.txt", "0062.txt", "0044.txt",
    "0060.txt", "0003.txt", "0078.txt", "0050.txt",
];

#[derive(Clone, Copy, Debug)]
struct TemperaturePair {
    start: f64,
    end: f64,
}

#[derive(Debug)]
struct Config {
    source: PathBuf,
    input_dir: PathBuf,
    cases: Vec<String>,
    temps: Vec<TemperaturePair>,
    timeout_ms: u64,
    keep_temp_source: bool,
}

#[derive(Clone)]
struct InputData {
    n: usize,
    m: usize,
    epsilon: f64,
    shapes: Vec<Vec<(usize, usize)>>,
    answer: Vec<Vec<i32>>,
    noise: Vec<f64>,
}

#[derive(Clone, Debug)]
struct CaseResult {
    case_name: String,
    status: String,
    score: i64,
    elapsed_ms: u128,
    surveys: usize,
    digs: usize,
    answers: usize,
}

#[derive(Debug)]
struct TempSummary {
    temp: TemperaturePair,
    total_sum: i64,
    total_avg: i64,
    max_score: i64,
    avg_elapsed_ms: u128,
    max_elapsed_ms: u128,
    one_billion_count: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let config = parse_args(&root)?;
    let temp_source = root.join("src/bin").join(format!("{TEMP_BIN}.rs"));
    let output_root = root.join("target/adhoc/sa_temperature_search");
    fs::create_dir_all(&output_root).map_err(|error| error.to_string())?;

    println!(
        "tune: source={} cases={} temps={} timeout_ms={}",
        config.source.display(),
        config.cases.len(),
        config.temps.len(),
        config.timeout_ms
    );

    let mut summaries = Vec::new();
    for temp in &config.temps {
        write_temp_solver(&config.source, &temp_source, *temp)?;
        build_temp_solver(&root)?;

        let label = format!("start{}_end{}", fmt_temp(temp.start), fmt_temp(temp.end));
        let output_dir = output_root.join(&label);
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir).map_err(|error| error.to_string())?;
        }
        fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

        let solver_bin = root.join("target/release").join(TEMP_BIN);
        let mut results = Vec::new();
        for case_name in &config.cases {
            let case_path = config.input_dir.join(case_name);
            let result = run_case(&case_path, &solver_bin, &output_dir, config.timeout_ms)
                .unwrap_or_else(|error| CaseResult {
                    case_name: case_name.clone(),
                    status: format!("runner_error:{error}"),
                    score: 1_000_000_000,
                    elapsed_ms: 0,
                    surveys: 0,
                    digs: 0,
                    answers: 0,
                });
            println!(
                "case temp={}:{} case={} score={} elapsed={}ms status={} survey={} dig={} answer={}",
                temp.start,
                temp.end,
                result.case_name,
                result.score,
                result.elapsed_ms,
                result.status,
                result.surveys,
                result.digs,
                result.answers
            );
            results.push(result);
        }

        let summary = summarize(*temp, &results);
        println!(
            "summary temp={}:{} avg={} sum={} max={} avg_elapsed={}ms max_elapsed={}ms 1e9={}",
            temp.start,
            temp.end,
            summary.total_avg,
            summary.total_sum,
            summary.max_score,
            summary.avg_elapsed_ms,
            summary.max_elapsed_ms,
            summary.one_billion_count
        );
        summaries.push(summary);
    }

    summaries.sort_by_key(|summary| (summary.total_avg, summary.one_billion_count));
    println!();
    println!("ranked_summary");
    for (rank, summary) in summaries.iter().enumerate() {
        println!(
            "{}: temp={}:{} avg={} max={} avg_elapsed={}ms max_elapsed={}ms 1e9={}",
            rank + 1,
            summary.temp.start,
            summary.temp.end,
            summary.total_avg,
            summary.max_score,
            summary.avg_elapsed_ms,
            summary.max_elapsed_ms,
            summary.one_billion_count
        );
    }

    if !config.keep_temp_source {
        let _ = fs::remove_file(temp_source);
    }
    Ok(())
}

fn parse_args(root: &Path) -> Result<Config, String> {
    let mut source = root.join(DEFAULT_SOURCE);
    let mut input_dir = root.join("tools/in");
    let mut cases_spec = DEFAULT_CASES.join(",");
    let mut temps_spec = DEFAULT_TEMPS.to_string();
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    let mut keep_temp_source = false;

    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source" => {
                index += 1;
                source = root.join(args.get(index).ok_or("--source requires a path")?);
            }
            "--input-dir" => {
                index += 1;
                input_dir = root.join(args.get(index).ok_or("--input-dir requires a path")?);
            }
            "--cases" => {
                index += 1;
                cases_spec = args.get(index).ok_or("--cases requires a value")?.clone();
            }
            "--temps" => {
                index += 1;
                temps_spec = args.get(index).ok_or("--temps requires a value")?.clone();
            }
            "--timeout-ms" => {
                index += 1;
                timeout_ms = args
                    .get(index)
                    .ok_or("--timeout-ms requires a value")?
                    .parse::<u64>()
                    .map_err(|error| error.to_string())?;
            }
            "--keep-temp-source" => {
                keep_temp_source = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
        index += 1;
    }

    if !source.is_file() {
        return Err(format!("source not found: {}", source.display()));
    }
    if !input_dir.is_dir() {
        return Err(format!("input dir not found: {}", input_dir.display()));
    }

    let cases = parse_cases(&input_dir, &cases_spec)?;
    let temps = parse_temps(&temps_spec)?;
    Ok(Config {
        source,
        input_dir,
        cases,
        temps,
        timeout_ms,
        keep_temp_source,
    })
}

fn print_help() {
    println!(
        "usage: cargo run --release --bin tune_v019_sa_temperature -- [options]\n\
         options:\n\
           --source PATH       source solver to copy (default: {DEFAULT_SOURCE})\n\
           --input-dir DIR     input directory (default: tools/in)\n\
           --cases LIST        comma-separated case names, or all\n\
           --temps LIST        comma-separated start:end pairs\n\
           --timeout-ms MS     per-case wall timeout (default: {DEFAULT_TIMEOUT_MS})\n\
           --keep-temp-source  keep generated src/bin/{TEMP_BIN}.rs"
    );
}

fn parse_cases(input_dir: &Path, spec: &str) -> Result<Vec<String>, String> {
    if spec == "all" {
        let mut cases = fs::read_dir(input_dir)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        cases.sort();
        return Ok(cases);
    }

    let mut cases = Vec::new();
    for raw in spec.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let case_name = if raw.ends_with(".txt") {
            raw.to_string()
        } else {
            format!("{raw}.txt")
        };
        let path = input_dir.join(&case_name);
        if !path.is_file() {
            return Err(format!("case not found: {}", path.display()));
        }
        cases.push(case_name);
    }
    if cases.is_empty() {
        return Err("no cases specified".to_string());
    }
    Ok(cases)
}

fn parse_temps(spec: &str) -> Result<Vec<TemperaturePair>, String> {
    let mut temps = Vec::new();
    for pair in spec.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let Some((start, end)) = pair.split_once(':') else {
            return Err(format!("invalid temp pair: {pair}"));
        };
        let start = start.parse::<f64>().map_err(|error| error.to_string())?;
        let end = end.parse::<f64>().map_err(|error| error.to_string())?;
        if !(start.is_finite() && end.is_finite() && start > 0.0 && end > 0.0) {
            return Err(format!("invalid temp value: {pair}"));
        }
        temps.push(TemperaturePair { start, end });
    }
    if temps.is_empty() {
        return Err("no temperatures specified".to_string());
    }
    Ok(temps)
}

fn write_temp_solver(
    source: &Path,
    destination: &Path,
    temp: TemperaturePair,
) -> Result<(), String> {
    let source_text = fs::read_to_string(source).map_err(|error| error.to_string())?;
    let mut out = String::with_capacity(source_text.len());
    for line in source_text.lines() {
        if line.starts_with("// ") {
            out.push_str("// v019_sa_temp_tune.rs\n");
        } else if line.starts_with("const SA_START_TEMPERATURE: f64 =") {
            out.push_str(&format!(
                "const SA_START_TEMPERATURE: f64 = {:.12};\n",
                temp.start
            ));
        } else if line.starts_with("const SA_END_TEMPERATURE: f64 =") {
            out.push_str(&format!(
                "const SA_END_TEMPERATURE: f64 = {:.12};\n",
                temp.end
            ));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    fs::write(destination, out).map_err(|error| error.to_string())
}

fn build_temp_solver(root: &Path) -> Result<(), String> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--quiet")
        .arg("--bin")
        .arg(TEMP_BIN)
        .current_dir(root)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo build failed: {status}"))
    }
}

fn run_case(
    case_path: &Path,
    solver_bin: &Path,
    output_dir: &Path,
    timeout_ms: u64,
) -> Result<CaseResult, String> {
    let input = parse_input_file(case_path)?;
    let case_name = case_path
        .file_name()
        .ok_or("case path has no file name")?
        .to_string_lossy()
        .to_string();
    let output_path = output_dir.join(&case_name);
    let err_path = output_dir.join(format!("{case_name}.err"));
    let mut output = File::create(output_path).map_err(|error| error.to_string())?;
    let err = File::create(err_path).map_err(|error| error.to_string())?;

    let mut child = Command::new(solver_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut child_stdin = child.stdin.take().ok_or("solver stdin unavailable")?;
    let child_stdout = child.stdout.take().ok_or("solver stdout unavailable")?;
    let (tx, rx) = mpsc::channel::<Option<String>>();
    thread::spawn(move || {
        let mut reader = BufReader::new(child_stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(None);
                    break;
                }
                Ok(_) => {
                    if tx.send(Some(line)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(None);
                    break;
                }
            }
        }
    });

    write_initial_input(&mut child_stdin, &input)?;
    let mut judge = LocalJudge::new(input.clone());
    let started_at = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let mut status = "ok".to_string();
    let mut surveys = 0_usize;
    let mut digs = 0_usize;
    let mut answers = 0_usize;

    while judge.responses < 2 * input.n * input.n {
        let Some(remaining) = timeout.checked_sub(started_at.elapsed()) else {
            kill_child(&mut child);
            status = "timeout".to_string();
            break;
        };
        let line = match rx.recv_timeout(remaining) {
            Ok(Some(line)) => line,
            Ok(None) => {
                status = "unexpected_exit".to_string();
                break;
            }
            Err(_) => {
                kill_child(&mut child);
                status = "timeout".to_string();
                break;
            }
        };

        output
            .write_all(line.as_bytes())
            .map_err(|error| error.to_string())?;
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        let response = match handle_solver_line(stripped, &mut judge) {
            Ok((response, QueryKind::Survey)) => {
                surveys += 1;
                response
            }
            Ok((response, QueryKind::Dig)) => {
                digs += 1;
                response
            }
            Ok((response, QueryKind::Answer)) => {
                answers += 1;
                response
            }
            Err(error) => {
                kill_child(&mut child);
                status = format!("invalid_output:{error}");
                break;
            }
        };

        judge.responses += 1;
        if writeln!(child_stdin, "{response}")
            .and_then(|_| child_stdin.flush())
            .is_err()
            && !(response == 1 && judge.finished)
        {
            status = "broken_pipe".to_string();
            break;
        }
        if response == 1 && judge.finished {
            break;
        }
    }

    let _ = child_stdin.flush();
    drop(child_stdin);
    wait_or_kill_child(&mut child, Duration::from_millis(100));
    let score = if status == "ok" {
        judge.score()
    } else {
        1_000_000_000
    };

    Ok(CaseResult {
        case_name,
        status,
        score,
        elapsed_ms: started_at.elapsed().as_millis(),
        surveys,
        digs,
        answers,
    })
}

#[derive(Clone, Copy)]
enum QueryKind {
    Survey,
    Dig,
    Answer,
}

struct LocalJudge {
    input: InputData,
    responses: usize,
    cost: f64,
    finished: bool,
    oil_cell_count: usize,
}

impl LocalJudge {
    fn new(input: InputData) -> Self {
        let oil_cell_count = input.answer.iter().flatten().filter(|&&v| v > 0).count();
        Self {
            input,
            responses: 0,
            cost: 0.0,
            finished: false,
            oil_cell_count,
        }
    }

    fn query_dig(&mut self, point: (usize, usize)) -> i32 {
        self.cost += 1.0;
        self.input.answer[point.0][point.1]
    }

    fn query_survey(&mut self, points: &[(usize, usize)]) -> i32 {
        self.cost += 1.0 / (points.len() as f64).sqrt();
        let oil_sum = points
            .iter()
            .map(|&(i, j)| self.input.answer[i][j])
            .sum::<i32>();
        let k = points.len() as f64;
        let mu =
            (k - oil_sum as f64) * self.input.epsilon + oil_sum as f64 * (1.0 - self.input.epsilon);
        let sigma = (k * self.input.epsilon * (1.0 - self.input.epsilon)).sqrt();
        let noise = self.input.noise[self.responses];
        rust_round_to_i32(mu + noise * sigma).max(0)
    }

    fn query_answer(&mut self, points: &[(usize, usize)]) -> i32 {
        if points.len() == self.oil_cell_count
            && points.iter().all(|&(i, j)| self.input.answer[i][j] > 0)
        {
            self.finished = true;
            1
        } else {
            self.cost += 1.0;
            0
        }
    }

    fn score(&self) -> i64 {
        let cost = if self.finished { self.cost } else { 1000.0 };
        (1_000_000.0 * cost.max(1.0 / self.input.n as f64) + 0.5).floor() as i64
    }
}

fn handle_solver_line(line: &str, judge: &mut LocalJudge) -> Result<(i32, QueryKind), String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(format!("too short: {line}"));
    }
    let ty = parts[0];
    let count = parts[1]
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    if count == 0 || count > judge.input.n * judge.input.n {
        return Err(format!("invalid point count: {count}"));
    }
    let points = parse_points(&parts[2..], judge.input.n, count)?;
    match ty {
        "q" if count == 1 => Ok((judge.query_dig(points[0]), QueryKind::Dig)),
        "q" => Ok((judge.query_survey(&points), QueryKind::Survey)),
        "a" => Ok((judge.query_answer(&points), QueryKind::Answer)),
        _ => Err(format!("unknown query type: {ty}")),
    }
}

fn parse_points(tokens: &[&str], n: usize, count: usize) -> Result<Vec<(usize, usize)>, String> {
    if tokens.len() != 2 * count {
        return Err("wrong point token count".to_string());
    }
    let mut points = Vec::with_capacity(count);
    let mut seen = HashSet::with_capacity(count * 2);
    for index in 0..count {
        let i = tokens[2 * index]
            .parse::<usize>()
            .map_err(|error| error.to_string())?;
        let j = tokens[2 * index + 1]
            .parse::<usize>()
            .map_err(|error| error.to_string())?;
        if i >= n || j >= n {
            return Err(format!("point out of range: {i} {j}"));
        }
        if !seen.insert((i, j)) {
            return Err(format!("duplicated point: {i} {j}"));
        }
        points.push((i, j));
    }
    Ok(points)
}

fn parse_input_file(path: &Path) -> Result<InputData, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut it = text.split_whitespace();
    let n = next_parse::<usize>(&mut it, "N")?;
    let m = next_parse::<usize>(&mut it, "M")?;
    let epsilon = next_parse::<f64>(&mut it, "epsilon")?;
    let mut shapes = Vec::with_capacity(m);
    for _ in 0..m {
        let d = next_parse::<usize>(&mut it, "d")?;
        let mut shape = Vec::with_capacity(d);
        for _ in 0..d {
            shape.push((
                next_parse::<usize>(&mut it, "shape i")?,
                next_parse::<usize>(&mut it, "shape j")?,
            ));
        }
        shapes.push(shape);
    }

    for _ in 0..m {
        let _ = next_parse::<usize>(&mut it, "hidden deltai_k")?;
        let _ = next_parse::<usize>(&mut it, "hidden deltaj_k")?;
    }

    let mut answer = vec![vec![0_i32; n]; n];
    for row in &mut answer {
        for value in row {
            *value = next_parse::<i32>(&mut it, "answer")?;
        }
    }

    let mut noise = Vec::with_capacity(2 * n * n);
    for _ in 0..2 * n * n {
        noise.push(next_parse::<f64>(&mut it, "noise")?);
    }

    Ok(InputData {
        n,
        m,
        epsilon,
        shapes,
        answer,
        noise,
    })
}

fn next_parse<T: std::str::FromStr>(
    it: &mut std::str::SplitWhitespace<'_>,
    label: &str,
) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    it.next()
        .ok_or_else(|| format!("unexpected EOF while reading {label}"))?
        .parse::<T>()
        .map_err(|error| format!("parse error in {label}: {error}"))
}

fn write_initial_input<W: Write>(writer: &mut W, input: &InputData) -> Result<(), String> {
    writeln!(writer, "{} {} {:.2}", input.n, input.m, input.epsilon)
        .map_err(|error| error.to_string())?;
    for shape in &input.shapes {
        write!(writer, "{}", shape.len()).map_err(|error| error.to_string())?;
        for &(i, j) in shape {
            write!(writer, " {i} {j}").map_err(|error| error.to_string())?;
        }
        writeln!(writer).map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn summarize(temp: TemperaturePair, results: &[CaseResult]) -> TempSummary {
    let total_sum = results.iter().map(|result| result.score).sum::<i64>();
    let total_avg = ((total_sum as f64 / results.len() as f64) + 0.5).floor() as i64;
    let max_score = results
        .iter()
        .map(|result| result.score)
        .max()
        .unwrap_or(1_000_000_000);
    let elapsed_sum = results.iter().map(|result| result.elapsed_ms).sum::<u128>();
    let avg_elapsed_ms = elapsed_sum / results.len() as u128;
    let max_elapsed_ms = results
        .iter()
        .map(|result| result.elapsed_ms)
        .max()
        .unwrap_or(0);
    let one_billion_count = results
        .iter()
        .filter(|result| result.score >= 1_000_000_000)
        .count();

    TempSummary {
        temp,
        total_sum,
        total_avg,
        max_score,
        avg_elapsed_ms,
        max_elapsed_ms,
        one_billion_count,
    }
}

fn rust_round_to_i32(value: f64) -> i32 {
    if value >= 0.0 {
        (value + 0.5).floor() as i32
    } else {
        (value - 0.5).ceil() as i32
    }
}

fn fmt_temp(value: f64) -> String {
    format!("{value:.6}").replace('.', "p")
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_or_kill_child(child: &mut Child, timeout: Duration) {
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {
                if started_at.elapsed() >= timeout {
                    kill_child(child);
                    return;
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => {
                kill_child(child);
                return;
            }
        }
    }
}
