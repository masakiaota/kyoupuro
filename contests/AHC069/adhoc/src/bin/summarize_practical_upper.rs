// summarize_practical_upper.rs
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct TemporalRow {
    theta: f64,
    grass: i64,
    load: f64,
    upper: i64,
}

#[derive(Clone)]
struct PracticalRow {
    score: i64,
    base_score: i64,
    extra_accepted: usize,
    episodes: usize,
}

#[derive(Clone)]
struct CombinedRow {
    case_name: String,
    theta: f64,
    grass: i64,
    load: f64,
    temporal_upper: i64,
    practical: i64,
    batch_official: i64,
    base_score: i64,
    v053: i64,
    accepted: usize,
    extra_accepted: usize,
    moves: usize,
    move_cost: i64,
    episodes: usize,
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "adhocの親ディレクトリを取得できない".to_string())
}

fn split_csv(line: &str) -> Vec<&str> {
    // 対象CSVには引用符や列内カンマがない。
    line.trim_end().split(',').collect()
}

fn read_temporal(path: &Path) -> Result<BTreeMap<String, TemporalRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}を読めない: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| "temporal CSVのheaderがない".to_string())?;
    let columns = split_csv(header);
    let index = |name: &str| {
        columns
            .iter()
            .position(|&value| value == name)
            .ok_or_else(|| format!("temporal CSVに{name}列がない"))
    };
    let case_index = index("case")?;
    let theta_index = index("theta_estimate")?;
    let grass_index = index("grass")?;
    let load_index = index("load")?;
    let upper_index = index("temporal_capacity_upper")?;
    let mut rows = BTreeMap::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let fields = split_csv(line);
        let parse_f64 = |column: usize, name: &str| {
            fields[column]
                .parse::<f64>()
                .map_err(|error| format!("{name}が不正: {error}"))
        };
        let parse_i64 = |column: usize, name: &str| {
            fields[column]
                .parse::<i64>()
                .map_err(|error| format!("{name}が不正: {error}"))
        };
        let case_name = fields[case_index].to_string();
        let row = TemporalRow {
            theta: parse_f64(theta_index, "theta_estimate")?,
            grass: parse_i64(grass_index, "grass")?,
            load: parse_f64(load_index, "load")?,
            upper: parse_i64(upper_index, "temporal_capacity_upper")?,
        };
        if rows.insert(case_name.clone(), row).is_some() {
            return Err(format!("temporal CSVでcase {case_name}が重複"));
        }
    }
    Ok(rows)
}

fn read_practical(path: &Path) -> Result<BTreeMap<String, PracticalRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}を読めない: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| "practical CSVのheaderがない".to_string())?;
    let columns = split_csv(header);
    let index = |name: &str| {
        columns
            .iter()
            .position(|&value| value == name)
            .ok_or_else(|| format!("practical CSVに{name}列がない"))
    };
    let case_index = index("case")?;
    let score_index = index("score")?;
    let base_score_index = index("base_score")?;
    let extra_accepted_index = index("extra_accepted")?;
    let episodes_index = index("episodes")?;
    let mut rows = BTreeMap::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let fields = split_csv(line);
        let case_name = fields[case_index].to_string();
        let row = PracticalRow {
            score: fields[score_index]
                .parse()
                .map_err(|error| format!("practical scoreが不正: {error}"))?,
            base_score: fields[base_score_index]
                .parse()
                .map_err(|error| format!("base_scoreが不正: {error}"))?,
            extra_accepted: fields[extra_accepted_index]
                .parse()
                .map_err(|error| format!("extra_acceptedが不正: {error}"))?,
            episodes: fields[episodes_index]
                .parse()
                .map_err(|error| format!("episodesが不正: {error}"))?,
        };
        if rows.insert(case_name.clone(), row).is_some() {
            return Err(format!("practical CSVでcase {case_name}が重複"));
        }
    }
    Ok(rows)
}

fn read_v053(path: &Path) -> Result<BTreeMap<String, i64>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}を読めない: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = split_csv(
        lines
            .next()
            .ok_or_else(|| "score_detail CSVのheaderがない".to_string())?,
    );
    let values = lines
        .map(split_csv)
        .find(|fields| fields.first() == Some(&"v053_posterior_rollout"))
        .ok_or_else(|| "score_detail CSVにv053がない".to_string())?;
    if values.len() != header.len() {
        return Err("v053行とheaderの列数が一致しない".to_string());
    }
    let mut rows = BTreeMap::new();
    for (name, value) in header.iter().zip(values) {
        let Some(case_name) = name.strip_suffix(".txt") else {
            continue;
        };
        let score = value
            .parse::<i64>()
            .map_err(|error| format!("v053のcase {case_name}が不正: {error}"))?;
        rows.insert(case_name.to_string(), score);
    }
    Ok(rows)
}

struct ReplayStats {
    score: i64,
    accepted: usize,
    moves: usize,
    move_cost: i64,
}

fn replay_output(root: &Path, case_name: &str, output_path: &Path) -> Result<ReplayStats, String> {
    let input_path = root.join("tools/in").join(format!("{case_name}.txt"));
    let input_text = fs::read_to_string(&input_path)
        .map_err(|error| format!("{}を読めない: {error}", input_path.display()))?;
    let output_text = fs::read_to_string(output_path)
        .map_err(|error| format!("{}を読めない: {error}", output_path.display()))?;
    let input = tools::parse_input(&input_text);
    let output = tools::parse_output(&input, &output_text);
    if let Some(error) = &output.error {
        return Err(format!("{}が不正: {error}", output_path.display()));
    }
    let frame = output
        .frames
        .last()
        .ok_or_else(|| format!("{}のframeがない", output_path.display()))?;
    Ok(ReplayStats {
        score: output.score,
        accepted: frame.accepted,
        moves: output.frames.iter().map(|frame| frame.moved.len()).sum(),
        move_cost: frame.total_move_cost,
    })
}

fn comma(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3 + usize::from(negative));
    if negative {
        result.push('-');
    }
    for (index, byte) in digits.bytes().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push(',');
        }
        result.push(byte as char);
    }
    result
}

fn median(mut values: Vec<i64>) -> i64 {
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2
    }
}

fn correlation(
    rows: &[CombinedRow],
    x: impl Fn(&CombinedRow) -> f64,
    y: impl Fn(&CombinedRow) -> f64,
) -> f64 {
    let count = rows.len() as f64;
    let mean_x = rows.iter().map(&x).sum::<f64>() / count;
    let mean_y = rows.iter().map(&y).sum::<f64>() / count;
    let covariance = rows
        .iter()
        .map(|row| (x(row) - mean_x) * (y(row) - mean_y))
        .sum::<f64>();
    let variance_x = rows
        .iter()
        .map(|row| (x(row) - mean_x).powi(2))
        .sum::<f64>();
    let variance_y = rows
        .iter()
        .map(|row| (y(row) - mean_y).powi(2))
        .sum::<f64>();
    covariance / (variance_x * variance_y).sqrt()
}

fn write_combined_csv(path: &Path, rows: &[CombinedRow]) -> Result<(), String> {
    let mut text = String::from(
        "case,theta_estimate,grass,load,temporal_capacity_upper,practical_score,batch_official_score,base_score,v053,accepted,extra_accepted,moves,move_cost,episodes,dynamic_gain,practical_to_temporal,v053_to_practical,improvement\n",
    );
    for row in rows {
        text.push_str(&format!(
            "{},{:.3},{},{:.9},{},{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{}\n",
            row.case_name,
            row.theta,
            row.grass,
            row.load,
            row.temporal_upper,
            row.practical,
            row.batch_official,
            row.base_score,
            row.v053,
            row.accepted,
            row.extra_accepted,
            row.moves,
            row.move_cost,
            row.episodes,
            row.practical - row.base_score,
            row.practical as f64 / row.temporal_upper as f64,
            row.v053 as f64 / row.practical as f64,
            row.practical - row.v053,
        ));
    }
    fs::write(path, text).map_err(|error| format!("{}を書けない: {error}", path.display()))
}

fn write_markdown_table(path: &Path, rows: &[CombinedRow]) -> Result<(), String> {
    let mut text = String::from(
        "| case | 推定 `θ` | `G(空きマス)` | 負荷率 | 時間容量上限 | practical基準 | v053 | 移動回数 | 再配置による増加 | 時間容量上限への到達率 | v053からの増加 |\n|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for row in rows {
        text.push_str(&format!(
            "| {} | {} | {} | {:.3} | {} | {} | {} | {} | {} | {:.2}% | {:+.2}% |\n",
            row.case_name,
            comma(row.theta.round() as i64),
            comma(row.grass),
            row.load,
            comma(row.temporal_upper),
            comma(row.practical),
            comma(row.v053),
            comma(row.moves as i64),
            comma(row.practical - row.base_score),
            100.0 * row.practical as f64 / row.temporal_upper as f64,
            100.0 * (row.practical as f64 / row.v053 as f64 - 1.0),
        ));
    }
    fs::write(path, text).map_err(|error| format!("{}を書けない: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let root = project_root()?;
    let temporal = read_temporal(&root.join("results/offline_reference/temporal_capacity.csv"))?;
    let practical = read_practical(&root.join("results/offline_reference/practical_scores.csv"))?;
    let score_detail_path = root.join("results/score_detail.csv");
    let v053 = read_v053(&score_detail_path)?;
    if temporal.len() != 100 || practical.len() != 100 || v053.len() != 100 {
        return Err(format!(
            "case数が100でない: temporal={}, practical={}, v053={}",
            temporal.len(),
            practical.len(),
            v053.len(),
        ));
    }

    let mut rows = Vec::with_capacity(100);
    let visualizer_dir = root.join("results/out/v_01_offline_reference");
    fs::create_dir_all(&visualizer_dir)
        .map_err(|error| format!("{}を作れない: {error}", visualizer_dir.display()))?;
    for (case_name, temporal_row) in temporal {
        let practical_row = practical
            .get(&case_name)
            .ok_or_else(|| format!("case {case_name}のpractical scoreがない"))?;
        let &v053_score = v053
            .get(&case_name)
            .ok_or_else(|| format!("case {case_name}のv053 scoreがない"))?;
        let batch_path = visualizer_dir.join(format!("{case_name}.txt"));
        let mut source_path = batch_path.clone();
        let mut practical_score = practical_row.score;
        let mut base_score = practical_row.base_score;
        let mut extra_accepted = practical_row.extra_accepted;
        let mut episodes = practical_row.episodes;

        // case 0000には、同じ60秒制限で単独実行した一括実行より高い出力がある。
        if case_name == "0000" && 63_951_466 > practical_score {
            source_path = root.join("results/offline_reference/0000_move60_final.txt");
            practical_score = 63_951_466;
            base_score = 63_828_377;
            extra_accepted = 8;
            episodes = 8;
        }
        let replay = replay_output(&root, &case_name, &source_path)?;
        if replay.score != practical_score {
            return Err(format!(
                "case {case_name}の採用score不一致: record={practical_score}, replay={}",
                replay.score
            ));
        }
        if source_path != batch_path {
            fs::copy(&source_path, &batch_path).map_err(|error| {
                format!(
                    "case {case_name}の採用出力を{}へ反映できない: {error}",
                    batch_path.display()
                )
            })?;
        }
        rows.push(CombinedRow {
            case_name,
            theta: temporal_row.theta,
            grass: temporal_row.grass,
            load: temporal_row.load,
            temporal_upper: temporal_row.upper,
            practical: practical_score,
            batch_official: practical_row.score,
            base_score,
            v053: v053_score,
            accepted: replay.accepted,
            extra_accepted,
            moves: replay.moves,
            move_cost: replay.move_cost,
            episodes,
        });
    }

    let upper_violations = rows
        .iter()
        .filter(|row| row.practical > row.temporal_upper)
        .count();
    let practical_wins = rows.iter().filter(|row| row.practical > row.v053).count();
    if upper_violations != 0 {
        return Err(format!("時間容量上限違反が{upper_violations}ケースある"));
    }

    let output_dir = root.join("results/offline_reference");
    write_combined_csv(&output_dir.join("practical_upper_bounds.csv"), &rows)?;
    write_markdown_table(&output_dir.join("practical_upper_bounds_table.md"), &rows)?;

    let practical_sum = rows.iter().map(|row| row.practical as i128).sum::<i128>();
    let v053_sum = rows.iter().map(|row| row.v053 as i128).sum::<i128>();
    let temporal_sum = rows
        .iter()
        .map(|row| row.temporal_upper as i128)
        .sum::<i128>();
    let practical_min = rows.iter().min_by_key(|row| row.practical).unwrap();
    let practical_max = rows.iter().max_by_key(|row| row.practical).unwrap();
    let accepted_sum = rows.iter().map(|row| row.accepted).sum::<usize>();
    let extra_accepted_sum = rows.iter().map(|row| row.extra_accepted).sum::<usize>();
    let moves_sum = rows.iter().map(|row| row.moves).sum::<usize>();
    let moved_cases = rows.iter().filter(|row| row.moves > 0).count();
    let move_cost_sum = rows.iter().map(|row| row.move_cost as i128).sum::<i128>();
    let episodes_sum = rows.iter().map(|row| row.episodes).sum::<usize>();
    let dynamic_gain_sum = rows
        .iter()
        .map(|row| (row.practical - row.base_score) as i128)
        .sum::<i128>();
    println!("cases={}", rows.len());
    println!("practical_sum={practical_sum}");
    println!("practical_avg={}", practical_sum / rows.len() as i128);
    println!(
        "practical_median={}",
        median(rows.iter().map(|row| row.practical).collect())
    );
    println!(
        "practical_min={},{}",
        practical_min.case_name, practical_min.practical
    );
    println!(
        "practical_max={},{}",
        practical_max.case_name, practical_max.practical
    );
    println!(
        "accepted_avg={:.2}",
        accepted_sum as f64 / rows.len() as f64
    );
    println!("extra_accepted_sum={extra_accepted_sum}");
    println!("moves_sum={moves_sum}");
    println!("moved_cases={moved_cases}");
    println!("move_cost_sum={move_cost_sum}");
    println!("episodes_sum={episodes_sum}");
    println!("dynamic_gain_sum={dynamic_gain_sum}");
    println!("practical_wins={practical_wins}");
    println!(
        "practical_to_temporal={:.9}",
        practical_sum as f64 / temporal_sum as f64
    );
    println!(
        "v053_to_practical={:.9}",
        v053_sum as f64 / practical_sum as f64
    );
    println!(
        "improvement_over_v053={:.9}",
        practical_sum as f64 / v053_sum as f64 - 1.0
    );
    println!(
        "theta_reach_correlation={:.9}",
        correlation(
            &rows,
            |row| row.theta,
            |row| row.practical as f64 / row.temporal_upper as f64,
        )
    );
    for (name, minimum, maximum) in [
        ("lt_0_8", f64::NEG_INFINITY, 0.8),
        ("0_8_to_1_3", 0.8, 1.3),
        ("1_3_to_1_9", 1.3, 1.9),
        ("ge_1_9", 1.9, f64::INFINITY),
    ] {
        let band = rows
            .iter()
            .filter(|row| minimum <= row.load && row.load < maximum)
            .collect::<Vec<_>>();
        let practical_band_sum = band.iter().map(|row| row.practical as i128).sum::<i128>();
        let temporal_band_sum = band
            .iter()
            .map(|row| row.temporal_upper as i128)
            .sum::<i128>();
        let v053_band_sum = band.iter().map(|row| row.v053 as i128).sum::<i128>();
        println!(
            "band_{name}=count:{},practical_avg:{},practical_to_temporal:{:.9},improvement_over_v053:{:.9}",
            band.len(),
            practical_band_sum / band.len() as i128,
            practical_band_sum as f64 / temporal_band_sum as f64,
            practical_band_sum as f64 / v053_band_sum as f64 - 1.0,
        );
    }
    println!("upper_violations={upper_violations}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
