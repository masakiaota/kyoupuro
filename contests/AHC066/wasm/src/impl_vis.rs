#[path = "../../tools/src/lib.rs"]
mod official;

fn empty_svg(message: &str) -> String {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        r##"<svg id="vis" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 180" width="640" height="180">
  <rect width="640" height="180" fill="#ffffff"/>
  <text x="320" y="90" text-anchor="middle" font-size="16" font-family="sans-serif" fill="#334155">{escaped}</text>
</svg>"##
    )
}

pub fn generate(seed: i32) -> String {
    official::gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if input.trim().is_empty() {
        return 0;
    }
    let input = official::parse_input(input);
    match official::parse_output(&input, output) {
        Ok(out) => official::get_max_turn(&input, &out),
        Err(_) => 0,
    }
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    if input.trim().is_empty() {
        return Ok((0, String::new(), empty_svg("Input is empty.")));
    }

    let input = official::parse_input(input);
    let out = match official::parse_output(&input, output) {
        Ok(out) => out,
        Err(err) => return Ok((0, err, empty_svg("Invalid output."))),
    };
    let (score, err, svg) = official::vis(&input, &out, turn.min(input.T), true, true);
    Ok((score, err, svg))
}
