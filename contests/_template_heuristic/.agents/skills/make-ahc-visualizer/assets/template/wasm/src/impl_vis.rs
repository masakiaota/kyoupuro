fn escaped_text(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn placeholder_svg(message: &str) -> String {
    let message = escaped_text(message);
    format!(
        r##"<svg id="vis" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">
  <rect width="640" height="360" fill="#ffffff"/>
  <rect x="24" y="24" width="592" height="312" rx="8" fill="#f8fafc" stroke="#cbd5e1"/>
  <text x="320" y="166" text-anchor="middle" font-size="20" font-family="sans-serif" fill="#155e63">AHC Visualizer Template</text>
  <text x="320" y="202" text-anchor="middle" font-size="13" font-family="sans-serif" fill="#5b6472">{message}</text>
</svg>"##
    )
}

pub fn generate(seed: i32) -> String {
    format!(
        "# Replace wasm/src/impl_vis.rs with problem-specific generator logic.\n# seed = {}\n",
        seed.max(0)
    )
}

pub fn calc_max_turn(_input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count()
        .max(1)
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let score = 0;
    let err = if output.trim().is_empty() {
        String::new()
    } else {
        "Problem-specific visualize/score logic is not implemented yet.".to_owned()
    };
    let message = if input.trim().is_empty() {
        "Input is empty. Implement parse, score, and SVG rendering for this problem."
    } else {
        "Implement parse, score, and SVG rendering for this problem."
    };
    let svg = placeholder_svg(&format!("{} turn={}", message, turn));
    Ok((score, err, svg))
}
