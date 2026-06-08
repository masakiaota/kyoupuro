#![allow(non_snake_case)]

use tools::*;

fn main() {
    if std::env::args().len() != 3 {
        eprintln!(
            "Usage: {} <input> <output>",
            std::env::args().nth(0).unwrap()
        );
        return;
    }

    let in_file = std::env::args().nth(1).unwrap();
    let out_file = std::env::args().nth(2).unwrap();
    let input = std::fs::read_to_string(&in_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", in_file);
        std::process::exit(1)
    });
    let output = std::fs::read_to_string(&out_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", out_file);
        std::process::exit(1)
    });

    let input = parse_input(&input);
    match parse_output(&input, &output) {
        Ok(out) => {
            let (score, err) = compute_score(&input, &out);
            if !err.is_empty() {
                println!("{}", err);
            }
            println!("Score = {}", score);
        }
        Err(err) => {
            println!("{}", err);
            println!("Score = {}", 0);
        }
    }
}
