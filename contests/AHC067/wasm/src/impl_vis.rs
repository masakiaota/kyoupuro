use std::cell::RefCell;

struct CachedCase {
    input_text: String,
    output_text: String,
    input: tools::Input,
    out: tools::Output,
    score: i64,
    err: String,
    states: Vec<tools::State>,
    precalc: tools::VisPrecalc,
}

thread_local! {
    static CASE_CACHE: RefCell<Option<CachedCase>> = const { RefCell::new(None) };
}

pub fn generate(seed: i32) -> String {
    tools::gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    if output.trim().is_empty() {
        return 0;
    }
    with_cached_case(input, output, |case| case.states.len().saturating_sub(1)).unwrap_or(0)
}

pub fn prepare_case(input: &str, output: &str) -> Result<usize, String> {
    with_cached_case(input, output, |case| case.states.len().saturating_sub(1))
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    with_cached_case(input, output, |case| render_cached_case(case, turn))
}

pub fn visualize_prepared(turn: usize) -> Result<(i64, String, String), String> {
    CASE_CACHE.with(|cache| {
        let cache = cache.borrow();
        let case = cache
            .as_ref()
            .ok_or_else(|| "Visualizer case is not prepared".to_owned())?;
        Ok(render_cached_case(case, turn))
    })
}

fn parse_input_checked(input: &str) -> Result<tools::Input, String> {
    std::panic::catch_unwind(|| tools::parse_input(input))
        .map_err(|_| "Failed to parse input".to_owned())
}

fn normalized_output(output: &str) -> &str {
    if output.trim().is_empty() {
        "0\n0\n"
    } else {
        output
    }
}

fn with_cached_case<R>(
    input_text: &str,
    output_text: &str,
    f: impl FnOnce(&CachedCase) -> R,
) -> Result<R, String> {
    let output_text = normalized_output(output_text);
    CASE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let needs_rebuild = cache
            .as_ref()
            .map(|case| case.input_text != input_text || case.output_text != output_text)
            .unwrap_or(true);

        if needs_rebuild {
            let input = parse_input_checked(input_text)?;
            let out = tools::parse_output(&input, output_text)?;
            let (score, err, states) = tools::compute_score_details(&input, &out);
            let precalc = tools::VisPrecalc::new(input.N, &states);
            *cache = Some(CachedCase {
                input_text: input_text.to_owned(),
                output_text: output_text.to_owned(),
                input,
                out,
                score,
                err,
                states,
                precalc,
            });
        }

        Ok(f(cache.as_ref().expect("case cache must be initialized")))
    })
}

fn render_cached_case(case: &CachedCase, turn: usize) -> (i64, String, String) {
    let (score, err, svg) = tools::vis_with_details(
        &case.input,
        &case.out,
        case.score,
        &case.err,
        &case.states,
        Some(&case.precalc),
        turn,
    );
    (if err.is_empty() { score } else { 0 }, err, svg)
}
