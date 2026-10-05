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
    /// Fix board size (12 <= N <= 20)
    #[clap(long = "fix-n", value_name = "N")]
    fix_n: Option<i32>,
    /// Fix number of colors (4 <= K <= 12)
    #[clap(long = "fix-k", value_name = "K")]
    fix_k: Option<i32>,
    /// Fix slime density (0.12 <= rho < 0.65)
    #[clap(long = "fix-rho", value_name = "RHO")]
    fix_rho: Option<f64>,
    /// Fix wall proposal rate (0 <= beta < 0.40)
    #[clap(long = "fix-beta", value_name = "BETA")]
    fix_beta: Option<f64>,
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
    for (name, value) in [("N", cli.fix_n), ("K", cli.fix_k)] {
        if let Some(value) = value {
            gen_option.set_i32(name, value).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1)
            });
        }
    }
    for (name, value) in [("rho", cli.fix_rho), ("beta", cli.fix_beta)] {
        if let Some(value) = value {
            gen_option.set_f64(name, value).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1)
            });
        }
    }
    if cli.verbose {
        let columns = vec!["file", "seed", "N", "K", "M", "walls"];
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
            row.extend([
                input.n.to_string(),
                input.k.to_string(),
                input.b.iter().sum::<usize>().to_string(),
                input.floor.iter().filter(|&&v| !v).count().to_string(),
            ]);
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
