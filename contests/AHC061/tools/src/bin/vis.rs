#![allow(non_snake_case)]

use tools::*;

fn main() {
    if std::env::args().len() != 3 {
        eprintln!("Usage: {} <input> <output>", std::env::args().nth(0).unwrap());
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
    let (out, parse_err) = parse_output_partial(&input, &output);
    let (mut score, vis_err, svg) = vis_default(&input, &out);
    let err = if !parse_err.is_empty() {
        score = 0;
        parse_err
    } else {
        vis_err
    };
    if err.len() > 0 {
        println!("{}", err);
        println!("Score = {}", 0);
    } else {
        println!("Score = {}", score);
    }
    let vis = format!("<html><body>{}</body></html>", svg);
    std::fs::write("vis.html", &vis).unwrap();
}
