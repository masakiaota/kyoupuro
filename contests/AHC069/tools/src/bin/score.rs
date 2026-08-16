#![allow(non_snake_case)]

use tools::{compute_score, parse_input, parse_output};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(2);
    }

    let input_text = std::fs::read_to_string(&args[1]).unwrap_or_else(|error| {
        eprintln!("failed to read input {}: {}", args[1], error);
        std::process::exit(2);
    });
    let output_text = std::fs::read_to_string(&args[2]).unwrap_or_else(|error| {
        eprintln!("failed to read output {}: {}", args[2], error);
        std::process::exit(2);
    });

    let input = parse_input(&input_text);
    let output = parse_output(&input, &output_text);
    let (score, error) = compute_score(&input, &output);
    if !error.is_empty() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
    println!("{}", score);
}
