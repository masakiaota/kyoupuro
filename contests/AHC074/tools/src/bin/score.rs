//! 公式ライブラリによる採点。無効出力は低スコアとして扱わず失敗にする。
use std::{env, fs, process};
use tools::{evaluate, parse_input, parse_output};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("Usage: score <input_file> <output_file>".to_owned());
    }
    let input_text = fs::read_to_string(&args[1]).map_err(|error| error.to_string())?;
    let output_text = fs::read_to_string(&args[2]).map_err(|error| error.to_string())?;
    let input = parse_input(&input_text);
    let output = parse_output(&input, &output_text)?;
    let score = evaluate(&input, &output).verdict().clone()?;
    eprintln!("E = {}", output.state.mistakes);
    eprintln!("T = {}", output.state.moves);
    println!("{score}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("score: {error}");
        process::exit(1);
    }
}
