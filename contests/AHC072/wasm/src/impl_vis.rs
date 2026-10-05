use std::cell::RefCell;
use tools::vis::{VisData, VisOption};

struct CachedCase {
    input_text: String,
    output_text: String,
    input: tools::Input,
    data: Result<VisData, String>,
}

thread_local! {
    // 公式のチェックポイントを保持し、コマ送りのたびに全操作を再評価しない。
    static CACHE: RefCell<Option<CachedCase>> = const { RefCell::new(None) };
}

#[allow(non_snake_case)]
pub(crate) fn parse_input(text: &str) -> Result<tools::Input, String> {
    // 公式パーサーは不正入力で panic するため、編集中の入力を先に検証する。
    let mut tokens = text.split_whitespace();
    let N = tokens
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| (12..=20).contains(v))
        .ok_or("N は 12〜20 の整数で指定する")?;
    let K = tokens
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| (4..=12).contains(v))
        .ok_or("K は 4〜12 の整数で指定する")?;
    let mut nests = vec![0; K];
    let mut slimes = vec![0; K];
    for i in 0..N {
        let row = tokens
            .next()
            .ok_or_else(|| format!("盤面の {i} 行目がない"))?;
        if row.len() != N {
            return Err(format!("盤面の {i} 行目は {N} 文字必要"));
        }
        for c in row.bytes() {
            match c {
                b'#' | b'.' => {}
                b'A'..=b'L' if usize::from(c - b'A') < K => nests[usize::from(c - b'A')] += 1,
                b'a'..=b'l' if usize::from(c - b'a') < K => slimes[usize::from(c - b'a')] += 1,
                _ => return Err(format!("盤面の {i} 行目に不正な文字がある")),
            }
        }
    }
    if tokens.next().is_some() {
        return Err("盤面の後に余分な入力がある".into());
    }
    if nests.iter().any(|&v| v != 1) || slimes.contains(&0) {
        return Err("各色の巣が1個、スライムが1匹以上必要".into());
    }
    let input = tools::parse_input(text);
    if !tools::connected(input.n, &input.floor) {
        return Err("床は上下左右に連結している必要がある".into());
    }
    Ok(input)
}

fn with_case<R>(input: &str, output: &str, f: impl FnOnce(&CachedCase) -> R) -> Result<R, String> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|c| c.input_text != input || c.output_text != output)
        {
            let parsed = parse_input(input)?;
            let data = tools::parse_output(&parsed, output).map(|out| VisData::new(&parsed, out));
            *cache = Some(CachedCase {
                input_text: input.into(),
                output_text: output.into(),
                input: parsed,
                data,
            });
        }
        Ok(f(cache.as_ref().unwrap()))
    })
}

pub fn generate(seed: i32) -> String {
    tools::generate(seed.max(0) as u64, &tools::GenOption::default()).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    with_case(input, output, |case| match &case.data {
        Ok(data) => tools::vis::get_max_turn(&case.input, data, &VisOption::default()),
        Err(_) => 0,
    })
    .unwrap_or(0)
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    with_case(input, output, |case| {
        let option = VisOption {
            t: turn,
            progress: 0,
        };
        match &case.data {
            Ok(data) => {
                let max_turn = tools::vis::get_max_turn(&case.input, data, &option);
                let (score, error) = if turn >= max_turn {
                    match data.verdict() {
                        Ok(score) => (*score, String::new()),
                        Err(error) => (0, error.clone()),
                    }
                } else {
                    (
                        data.state_at(&case.input, turn).score(&case.input),
                        String::new(),
                    )
                };
                let svg = tools::vis::vis(&case.input, data, &option).svg;
                (score, error, svg)
            }
            Err(error) => (
                0,
                error.clone(),
                tools::vis::draw_parse_error(&case.input, error, &option),
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_and_scores_match_official_visualizer() {
        let input = generate(0);
        let output = "5 0 0 R 1\n5 1 1 U 2\n";
        assert_eq!(calc_max_turn(&input, output), 2);
        let parsed = tools::parse_input(&input);
        let data = VisData::new(&parsed, tools::parse_output(&parsed, output).unwrap());
        for turn in [0, 1, 2, 100, 0] {
            let (score, error, svg) = visualize(&input, output, turn).unwrap();
            assert!(error.is_empty());
            assert_eq!(score, data.state_at(&parsed, turn).score(&parsed));
            assert_eq!(
                svg,
                tools::vis::vis(
                    &parsed,
                    &data,
                    &VisOption {
                        t: turn,
                        progress: 0
                    }
                )
                .svg
            );
        }
    }

    #[test]
    fn invalid_operation_preserves_last_valid_board() {
        let input = generate(0);
        let output = "5 0 0 R 1\n0 0 0 R 1\n";
        assert_eq!(calc_max_turn(&input, output), 2);
        assert!(visualize(&input, output, 1).unwrap().1.is_empty());
        let (score, error, svg) = visualize(&input, output, 2).unwrap();
        assert_eq!(score, 0);
        assert!(error.contains("Operation 2"));
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn editing_errors_do_not_break_subsequent_rendering() {
        let input = generate(0);
        assert!(visualize("12 4\n.", "", 0).is_err());
        let (_, error, svg) = visualize(&input, "5 0 0 X 1", 0).unwrap();
        assert!(!error.is_empty());
        assert!(svg.contains("<svg"));
        assert_eq!(calc_max_turn(&input, ""), 0);
        assert!(visualize(&input, "", 0).unwrap().1.is_empty());
        let other_input = generate(1);
        assert!(visualize(&other_input, "", 0).unwrap().1.is_empty());
    }
}
