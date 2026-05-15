use tools::*;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(1);
    }

    let input_str = std::fs::read_to_string(&args[1]).unwrap_or_else(|_| {
        eprintln!("no such file: {}", args[1]);
        std::process::exit(1);
    });
    let output_str = std::fs::read_to_string(&args[2]).unwrap_or_else(|_| {
        eprintln!("no such file: {}", args[2]);
        std::process::exit(1);
    });

    let input = read_input_str(&input_str);
    let output = read_output_str(&input, &output_str);
    let (score, error) = compute_score_detail(&input, &output);
    if !error.is_empty() {
        eprintln!("{}", error);
    }
    println!("{}", score);
}
