// 公式 tools 側と同じ検証・採点・描画を利用し、visualizer の挙動を一致させる。
#[allow(dead_code)]
#[path = "../../tools/src/lib.rs"]
mod official;

fn next_token<'a>(tokens: &mut impl Iterator<Item = &'a str>, name: &str) -> Result<&'a str, String> {
    tokens
        .next()
        .ok_or_else(|| format!("Input parse error: missing {}", name))
}

fn parse_usize<'a>(
    tokens: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<usize, String> {
    next_token(tokens, name)?
        .parse::<usize>()
        .map_err(|_| format!("Input parse error: invalid {}", name))
}

fn parse_wall_row(row: &str, len: usize, name: &str) -> Result<Vec<bool>, String> {
    if row.len() != len {
        return Err(format!(
            "Input parse error: {} must have length {}, got {}",
            name,
            len,
            row.len()
        ));
    }
    row.bytes()
        .map(|ch| match ch {
            b'0' => Ok(false),
            b'1' => Ok(true),
            _ => Err(format!("Input parse error: {} contains a non-binary character", name)),
        })
        .collect()
}

fn parse_input(raw: &str) -> Result<official::Input, String> {
    let mut tokens = raw.split_whitespace();
    let n = parse_usize(&mut tokens, "N")?;
    if n == 0 {
        return Err("Input parse error: N must be positive".to_owned());
    }

    let mut a = vec![vec![0; n]; n];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = parse_usize(&mut tokens, &format!("A[{},{}]", i, j))?;
        }
    }

    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let row = next_token(&mut tokens, &format!("V[{}]", i))?;
        v.push(parse_wall_row(row, n - 1, &format!("V[{}]", i))?);
    }

    let mut h = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let row = next_token(&mut tokens, &format!("H[{}]", i))?;
        h.push(parse_wall_row(row, n, &format!("H[{}]", i))?);
    }

    if tokens.next().is_some() {
        return Err("Input parse error: unexpected trailing token".to_owned());
    }

    Ok(official::Input {
        N: n,
        V: v,
        H: h,
        A: a,
        W: 0,
    })
}

pub fn generate(seed: i32) -> String {
    official::gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    let Ok(input) = parse_input(input) else {
        return 0;
    };
    let Ok(output) = official::parse_output(&input, output) else {
        return 0;
    };
    official::compute_max_turn(&input, &output)
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let input = parse_input(input)?;
    match official::parse_output(&input, output) {
        Ok(output) => Ok(official::vis(&input, &output, turn)),
        Err(err) => {
            // 出力エラー時も初期盤面を表示して、入力とエラー位置を確認できるようにする。
            let empty_output = official::Output { out: vec![] };
            let (_, _, svg) = official::vis(&input, &empty_output, 0);
            Ok((0, err, svg))
        }
    }
}
