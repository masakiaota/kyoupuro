// analyze_output_quality.rs
#![allow(non_snake_case)] // 問題文の `P`, `V`, `L` を対応づけたまま使う。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Aggregate {
    cases: usize,
    score: i128,
    accepted: usize,
    total_fee: i128,
    total_move_cost: i128,
    ideal_fee: i128,
    initial_fee: i128,
    moved_groups: usize,
    initial_slack_sum: i128,
    initial_bad_shape: usize,
    slack_count: [usize; 7],
    slack_ideal_fee: [i128; 7],
    slack_actual_fee: [i128; 7],
}

const SLACK_LABELS: [&str; 7] = ["0", "2", "4", "6", "8..14", "16..30", "32+"];

fn slack_bucket(slack: i64) -> usize {
    match slack {
        0 => 0,
        2 => 1,
        4 => 2,
        6 => 3,
        8..=14 => 4,
        16..=30 => 5,
        _ => 6,
    }
}

fn minimum_perimeter(P: usize) -> i64 {
    2 * (2.0 * (P as f64).sqrt() - 1e-12).ceil() as i64
}

fn fee(V: i64, P: usize, L: i64) -> i64 {
    ((V as f64) * 4.0 * (P as f64).sqrt() / (L as f64)).round() as i64
}

fn input_paths(input_dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(input_dir)
        .expect("read input dir")
        .map(|entry| entry.expect("read dir entry").path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "usage: analyze_output_quality <input_dir> <output_dir>"
    );
    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);
    let mut aggregate = Aggregate::default();

    for input_path in input_paths(input_dir) {
        let output_path = output_dir.join(input_path.file_name().expect("input basename"));
        if !output_path.is_file() {
            continue;
        }
        let input_text = fs::read_to_string(&input_path).expect("read input");
        let output_text = fs::read_to_string(&output_path).expect("read output");
        let input = tools::parse_input(&input_text);
        let output = tools::parse_output(&input, &output_text);
        assert!(
            output.error.is_none(),
            "{}: {:?}",
            input_path.display(),
            output.error
        );
        let final_frame = output.frames.last().expect("final frame");

        aggregate.cases += 1;
        aggregate.score += output.score as i128;
        aggregate.total_fee += final_frame.total_fee as i128;
        aggregate.total_move_cost += final_frame.total_move_cost as i128;
        aggregate.moved_groups += output
            .frames
            .iter()
            .map(|frame| frame.moved.len())
            .sum::<usize>();

        for frame in &output.frames {
            let Some((group_id, true)) = frame.arrival else {
                continue;
            };
            let group = input.groups[group_id];
            let active = frame
                .actives
                .iter()
                .find(|active| active.id == group_id)
                .expect("accepted arrival is active");
            let min_L = minimum_perimeter(group.p);
            let ideal_fee = fee(group.v, group.p, min_L);
            let actual_fee = fee(group.v, group.p, active.l);
            let slack = active.l - min_L;
            let bucket = slack_bucket(slack);
            aggregate.accepted += 1;
            aggregate.ideal_fee += ideal_fee as i128;
            aggregate.initial_fee += actual_fee as i128;
            aggregate.initial_slack_sum += slack as i128;
            aggregate.initial_bad_shape += usize::from(slack >= 8);
            aggregate.slack_count[bucket] += 1;
            aggregate.slack_ideal_fee[bucket] += ideal_fee as i128;
            aggregate.slack_actual_fee[bucket] += actual_fee as i128;
        }
    }

    let cases = aggregate.cases.max(1) as f64;
    let ideal = aggregate.ideal_fee.max(1) as f64;
    println!("cases={}", aggregate.cases);
    println!("score_avg={:.0}", aggregate.score as f64 / cases);
    println!(
        "accepted={} accepted_avg={:.2}",
        aggregate.accepted,
        aggregate.accepted as f64 / cases
    );
    println!("fee_avg={:.0}", aggregate.total_fee as f64 / cases);
    println!(
        "move_cost_avg={:.0}",
        aggregate.total_move_cost as f64 / cases
    );
    println!("ideal_fee_avg={:.0}", aggregate.ideal_fee as f64 / cases);
    println!(
        "initial_fee_ratio={:.6}",
        aggregate.initial_fee as f64 / ideal
    );
    println!("final_fee_ratio={:.6}", aggregate.total_fee as f64 / ideal);
    println!(
        "moved_groups={} moved_avg={:.2}",
        aggregate.moved_groups,
        aggregate.moved_groups as f64 / cases
    );
    println!(
        "initial_slack_avg={:.3}",
        aggregate.initial_slack_sum as f64 / aggregate.accepted.max(1) as f64
    );
    println!(
        "initial_bad_shape={} initial_bad_avg={:.2}",
        aggregate.initial_bad_shape,
        aggregate.initial_bad_shape as f64 / cases
    );
    for (bucket, label) in SLACK_LABELS.iter().enumerate() {
        let bucket_ideal = aggregate.slack_ideal_fee[bucket];
        let bucket_actual = aggregate.slack_actual_fee[bucket];
        println!(
            "slack_bucket={} count={} count_avg={:.2} fee_avg={:.0} ideal_avg={:.0} loss_avg={:.0} realize={:.6}",
            label,
            aggregate.slack_count[bucket],
            aggregate.slack_count[bucket] as f64 / cases,
            bucket_actual as f64 / cases,
            bucket_ideal as f64 / cases,
            (bucket_ideal - bucket_actual) as f64 / cases,
            bucket_actual as f64 / bucket_ideal.max(1) as f64,
        );
    }
}
