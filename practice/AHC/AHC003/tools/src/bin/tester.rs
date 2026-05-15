use std::io::prelude::*;
use std::process::Stdio;
use tools::*;

fn exec(input: &str, command: &str, args: &Vec<String>) {
    let input = std::fs::read_to_string(&input).unwrap_or_else(|_| {
        eprintln!("no such file: {}", input);
        std::process::exit(1)
    });
    let input = read_input_str(&input);
    let mut p = std::process::Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("failed to execute the command");
            eprintln!("{}", e);
            std::process::exit(1)
        });
    let mut stdin = p.stdin.take().unwrap();
    let mut stdout = std::io::BufReader::new(p.stdout.take().unwrap());
    let mut visited = mat![!0; N; N];
    let mut outs = vec![];
    for k in 0..Q {
        let err = || -> ! {
            eprintln!(
                "Your program terminated unexpectedly during the {}-th query",
                k + 1
            );
            std::process::exit(1)
        };
        writeln!(
            stdin,
            "{} {} {} {}",
            input.s[k].0, input.s[k].1, input.t[k].0, input.t[k].1
        )
        .unwrap_or_else(|_| err());
        stdin.flush().unwrap_or_else(|_| err());
        let mut out = String::new();
        stdout.read_line(&mut out).unwrap_or_else(|_| err());
        print!("{}", out);
        out = out.trim().to_owned();
        match compute_path_length(&input, k, &out, &mut visited) {
            Ok((d, _)) => {
                writeln!(stdin, "{}", (d as f64 * input.e[k]).round() as i32)
                    .unwrap_or_else(|_| err());
                stdin.flush().unwrap_or_else(|_| err());
            }
            Err(s) => {
                eprintln!("Error: {}", s);
                eprintln!("Score = {}", 0);
                p.kill().unwrap();
                break;
            }
        }
        outs.push(out);
    }
    p.wait().unwrap();
    let (score, _) = compute_score_detail(&input, &outs);
    eprintln!("Score = {}", score);
}

fn main() {
    if std::env::args().len() < 3 {
        eprintln!(
            "Usage: {} <input> <command> [<args>...]",
            std::env::args().nth(0).unwrap()
        );
        return;
    }
    let input = std::env::args().nth(1).unwrap();
    let command = std::env::args().nth(2).unwrap();
    let args = std::env::args().skip(3).collect::<Vec<_>>();
    exec(&input, &command, &args);
}
