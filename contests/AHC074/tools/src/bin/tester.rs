#![allow(non_snake_case)]

use std::process::Stdio;
use tools::{evaluate, exec};

fn main() {
    if std::env::args().len() < 2 {
        eprintln!(
            "Usage: {} <command> [<args>...]",
            std::env::args().nth(0).unwrap()
        );
        std::process::exit(1);
    }
    let (command, args) = (
        std::env::args().nth(1).unwrap(),
        std::env::args().skip(2).collect::<Vec<_>>(),
    );
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
    let ret = exec(&mut p, true);
    // Report a runtime error if the solver exited abnormally, even when the
    // interaction ran to the end. If its output was cut off, exec has already
    // waited for the exit status to settle.
    let solver_failed = matches!(p.try_wait(), Ok(Some(status)) if !status.success());
    // The solver may still be waiting for a response when an invalid output is found.
    let _ = p.kill();
    if solver_failed {
        eprintln!("Your program terminated abnormally");
        eprintln!("Score = 0");
        std::process::exit(1);
    }
    let verdict = match ret {
        Ok((input, out)) => evaluate(&input, &out).verdict().clone(),
        Err(err) => Err(err),
    };
    match verdict {
        Ok(score) => eprintln!("Score = {}", score),
        Err(err) => {
            eprintln!("{}", err);
            eprintln!("Score = 0");
        }
    }
}
