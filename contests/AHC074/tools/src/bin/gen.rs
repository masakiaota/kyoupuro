#![allow(non_snake_case)]

use clap::Parser;
use std::{
    io::prelude::*,
    path::PathBuf,
    time::{Duration, Instant},
};
use tools::*;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to seeds.txt
    seeds: String,
    /// Path to input directory
    #[clap(short = 'd', long = "dir", default_value = "in")]
    dir: PathBuf,
    /// Print input details in csv format
    #[clap(short = 'v', long = "verbose")]
    verbose: bool,
    // Example for an input whose N varies and sometimes needs to be fixed:
    // /// Fix N for input generation
    // #[clap(long = "fix-n", value_name = "N")]
    // fix_n: Option<i32>,
}

fn main() {
    let cli = Cli::parse();
    if !std::path::Path::new(&cli.dir).exists() {
        std::fs::create_dir(&cli.dir).unwrap();
    }
    let f = std::fs::File::open(&cli.seeds).unwrap_or_else(|_| {
        eprintln!("no such file: {}", cli.seeds);
        std::process::exit(1)
    });
    let f = std::io::BufReader::new(f);
    let seeds = f
        .lines()
        .filter_map(|line| {
            let line = line.unwrap();
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            Some(line.parse::<u64>().unwrap_or_else(|_| {
                eprintln!("parse failed: {}", line);
                std::process::exit(1)
            }))
        })
        .collect::<Vec<_>>();
    let total = seeds.len();
    let started = Instant::now();
    let mut last_progress = Instant::now();
    #[allow(unused_mut)]
    let mut gen_option = GenOption::default();
    // Example for applying a fixed option. Do not omit error handling:
    // if let Some(value) = cli.fix_n {
    //     gen_option.set_i32("N", value).unwrap_or_else(|e| {
    //         eprintln!("{}", e);
    //         std::process::exit(1)
    //     });
    // }
    if cli.verbose {
        let columns = vec!["file", "seed"];
        // Add the columns here. For example:
        // let columns = vec!["file", "seed", "N", "M"];
        println!("{}", columns.join(","));
    }
    eprintln!("Input generation: 0/{} (0.0s elapsed)", total);
    for (id, seed) in seeds.into_iter().enumerate() {
        let input = generate(seed, &gen_option);
        if last_progress.elapsed() >= Duration::from_secs(10) {
            eprintln!(
                "Input generation: {}/{} ({:.1}s elapsed)",
                id + 1,
                total,
                started.elapsed().as_secs_f64()
            );
            last_progress = Instant::now();
        }
        if cli.verbose {
            #[allow(unused_mut)]
            let mut row = vec![format!("{:04}", id), seed.to_string()];
            // Append the values in the same order as the header, for example:
            // row.extend([input.N.to_string(), input.M.to_string()]);
            println!("{}", row.join(","));
        }
        let mut w = std::io::BufWriter::new(
            std::fs::File::create(cli.dir.join(format!("{:04}.txt", id))).unwrap(),
        );
        write!(w, "{}", input).unwrap();
    }
    eprintln!(
        "Input generation: {}/{} complete ({:.1}s elapsed)",
        total,
        total,
        started.elapsed().as_secs_f64()
    );
}
