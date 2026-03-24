use tools::{compute_score, parse_input, parse_output};

fn main() {
    if std::env::args().len() != 3 {
        eprintln!(
            "Usage: {} <input> <output>",
            std::env::args().next().unwrap_or_else(|| "score".to_owned())
        );
        std::process::exit(1);
    }
    let in_file = std::env::args().nth(1).unwrap();
    let out_file = std::env::args().nth(2).unwrap();
    let input_text = std::fs::read_to_string(&in_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", in_file);
        std::process::exit(1);
    });
    let output_text = std::fs::read_to_string(&out_file).unwrap_or_else(|_| {
        eprintln!("no such file: {}", out_file);
        std::process::exit(1);
    });
    let input = parse_input(&input_text);
    let out = parse_output(&input, &output_text).unwrap_or_else(|err| {
        eprintln!("{}", err);
        std::process::exit(1);
    });
    let (score, err, _) = compute_score(&input, &out);
    println!("Score = {}", score);
    if !err.is_empty() {
        eprintln!("{}", err);
    }
}
