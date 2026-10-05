#![allow(non_snake_case)]

use clap::Parser;
use tools::vis::*;
use tools::*;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to the input file
    input: String,
    /// Path to the output file
    output: String,
    /// Visualization position (default: the last position). Only affects vis.html;
    /// the printed score is always the one for the whole output
    #[clap(short = 't', long = "turn")]
    turn: Option<usize>,
    /// Print the score only (do not write vis.html)
    #[clap(long = "no-vis")]
    no_vis: bool,
    /// Where to write the visualization (default: vis.html in the current directory)
    #[clap(long = "html", value_name = "FILE", default_value = "vis.html")]
    html: String,
    /// Progress of the next operation, from 0 to 1000
    #[clap(long, default_value_t = 0)]
    progress: i32,
    // Example for a problem that has display options. Give every option a description,
    // and leave the range and the choices to the VisOption setters:
    // /// Show the number in each cell
    // #[clap(long)]
    // show_number: bool,
    // /// Length of the trail to draw
    // #[clap(long, default_value_t = 20)]
    // trail_length: i32,
    // /// What the cell color represents
    // #[clap(long, default_value = "type")]
    // color_mode: String,
}

fn main() {
    let cli = Cli::parse();
    #[allow(unused_mut)]
    let mut vis_option = VisOption::default();
    // Example for applying a display option. Pass the value to the same setter that
    // getVisOption() in web/index.html calls, so both versions accept the same values.
    // Do not omit error handling:
    // if let Err(err) = vis_option.set_i32("trail_length", cli.trail_length) {
    //     eprintln!("{}", err);
    //     std::process::exit(1)
    // }
    if let Err(err) = vis_option.set_i32("progress", cli.progress) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
    vis_option.t = cli.turn.unwrap_or(usize::MAX);
    let input = std::fs::read_to_string(&cli.input).unwrap_or_else(|_| {
        eprintln!("no such file: {}", cli.input);
        std::process::exit(1)
    });
    let output = std::fs::read_to_string(&cli.output).unwrap_or_else(|_| {
        eprintln!("no such file: {}", cli.output);
        std::process::exit(1)
    });
    let input = parse_input(&input);
    let out = parse_output(&input, &output);
    let (verdict, svg) = match out {
        Ok(out) => {
            if cli.no_vis {
                (evaluate(&input, &out).verdict().clone(), String::new())
            } else {
                let data = VisData::new(&input, out);
                let svg = vis(&input, &data, &vis_option).svg;
                (data.verdict().clone(), svg)
            }
        }
        // Draw the input even if parsing fails, so that vis.html is not left blank.
        Err(err) => {
            let svg = if cli.no_vis {
                String::new()
            } else {
                draw_parse_error(&input, &err, &vis_option)
            };
            (Err(err), svg)
        }
    };
    match verdict {
        Ok(score) => println!("Score = {}", score),
        Err(err) => {
            println!("{}", err);
            println!("Score = {}", 0);
        }
    }
    if !cli.no_vis {
        let vis = format!("<html><body>{}</body></html>", svg);
        std::fs::write(&cli.html, &vis).unwrap_or_else(|err| {
            eprintln!("could not write {}: {}", cli.html, err);
            std::process::exit(1)
        });
    }
}
