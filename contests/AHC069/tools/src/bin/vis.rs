#![allow(non_snake_case)]

use tools::*;

fn main() {
    let argc = std::env::args().len();
    if argc != 3 && argc != 4 {
        eprintln!(
            "Usage: {} <input> <output> [group|t|c]",
            std::env::args().nth(0).unwrap()
        );
        return;
    }
    let in_file = std::env::args().nth(1).unwrap();
    let out_file = std::env::args().nth(2).unwrap();
    let mode_arg = std::env::args().nth(3).unwrap_or_default();
    let mode = ColorMode::parse(&mode_arg).unwrap_or_else(|| {
        eprintln!("unknown color mode: {} (expected group, t or c)", mode_arg);
        std::process::exit(1)
    });
    let input = std::fs::read_to_string(&in_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", in_file);
        std::process::exit(1)
    });
    let output = std::fs::read_to_string(&out_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", out_file);
        std::process::exit(1)
    });
    let input = parse_input(&input);
    let out = parse_output(&input, &output);
    let opt = VisOpt { color: mode, ..VisOpt::default() };
    // A log the rules reject still draws the turns before the offending one, so there is
    // something to look at while working out what went wrong.
    let (score, err, svg) = vis_default(&input, &out, opt);
    if err.len() > 0 {
        println!("{}", err);
    }
    println!("Score = {}", score);
    let vis = format!("<html><body>{}</body></html>", svg);
    std::fs::write("vis.html", &vis).unwrap();
}
