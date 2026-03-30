#![allow(non_snake_case, dead_code, unused_imports, unused_macros)]

use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Path, Rectangle, Text as SvgText};
use svg::Document;

pub const N: usize = 50;
const CELL_SIZE: usize = 20;
const FOOTER_HEIGHT: usize = 54;
pub type Output = String;

pub struct Input {
    pub s: (usize, usize),
    pub tiles: Vec<Vec<usize>>,
    pub ps: Vec<Vec<i32>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {}", self.s.0, self.s.1)?;
        for row in &self.tiles {
            for (j, value) in row.iter().enumerate() {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{value}")?;
            }
            writeln!(f)?;
        }
        for row in &self.ps {
            for (j, value) in row.iter().enumerate() {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{value}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn next_usize<'a, I>(tokens: &mut I, name: &str) -> Result<usize, String>
where
    I: Iterator<Item = &'a str>,
{
    let token = tokens.next().ok_or_else(|| format!("{name} が不足している"))?;
    token
        .parse::<usize>()
        .map_err(|_| format!("{name} の整数変換に失敗した: {token}"))
}

fn next_i32<'a, I>(tokens: &mut I, name: &str) -> Result<i32, String>
where
    I: Iterator<Item = &'a str>,
{
    let token = tokens.next().ok_or_else(|| format!("{name} が不足している"))?;
    token
        .parse::<i32>()
        .map_err(|_| format!("{name} の整数変換に失敗した: {token}"))
}

pub fn parse_input_str(src: &str) -> Result<Input, String> {
    let mut tokens = src.split_whitespace();
    let si = next_usize(&mut tokens, "si")?;
    let sj = next_usize(&mut tokens, "sj")?;
    if si >= N || sj >= N {
        return Err("開始位置が盤面外である".to_owned());
    }

    let mut tiles = vec![vec![0; N]; N];
    let mut max_tile = 0usize;
    for i in 0..N {
        for j in 0..N {
            let tile = next_usize(&mut tokens, &format!("tiles[{i}][{j}]"))?;
            max_tile = max_tile.max(tile);
            tiles[i][j] = tile;
        }
    }
    if max_tile >= N * N {
        return Err("タイル ID が大きすぎる".to_owned());
    }

    let mut ps = vec![vec![0; N]; N];
    for i in 0..N {
        for j in 0..N {
            ps[i][j] = next_i32(&mut tokens, &format!("ps[{i}][{j}]"))?;
        }
    }

    if tokens.next().is_some() {
        return Err("入力に余分なトークンがある".to_owned());
    }

    Ok(Input {
        s: (si, sj),
        tiles,
        ps,
    })
}

pub fn read_output_str(_input: &Input, f: &str) -> Output {
    f.trim().to_owned()
}

pub fn compute_score_detail(
    input: &Input,
    out: &Output,
) -> (i32, String, Vec<usize>, Vec<(usize, usize)>) {
    let mut used = vec![0; N * N];
    let (mut i, mut j) = input.s;
    used[input.tiles[i][j]] = 1;
    let mut score = input.ps[i][j];
    let mut steps = vec![(i, j)];
    let mut err = String::new();

    for c in out.chars() {
        let (di, dj) = match c {
            'L' => (0, !0),
            'R' => (0, 1),
            'U' => (!0, 0),
            'D' => (1, 0),
            _ => {
                return (0, "Illegal output".to_owned(), used, steps);
            }
        };
        i += di;
        j += dj;
        if i >= N || j >= N {
            return (0, "Out of range".to_owned(), used, steps);
        }
        steps.push((i, j));
        if used[input.tiles[i][j]] != 0 {
            err = "Stepped on the same tile twice".to_owned();
        }
        used[input.tiles[i][j]] += 1;
        score += input.ps[i][j];
    }

    if !err.is_empty() {
        score = 0;
    }

    (score, err, used, steps)
}

const DIJ: [(usize, usize); 4] = [(0, !0), (0, 1), (!0, 0), (1, 0)];

pub fn gen(seed: u64) -> Input {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let s = (rng.gen_range(0..N), rng.gen_range(0..N));

    let mut tiles = vec![vec![0; N]; N];
    let mut cells = Vec::with_capacity(N * N);
    for i in 0..N {
        for j in 0..N {
            tiles[i][j] = i * N + j;
            cells.push((i, j));
        }
    }

    cells.shuffle(&mut rng);
    let mut used = vec![vec![false; N]; N];
    for (i, j) in cells {
        if used[i][j] {
            continue;
        }
        let mut dirs = DIJ.to_vec();
        dirs.shuffle(&mut rng);
        for (di, dj) in dirs {
            let ni = i + di;
            let nj = j + dj;
            if ni < N && nj < N && !used[ni][nj] {
                tiles[ni][nj] = tiles[i][j];
                used[ni][nj] = true;
                used[i][j] = true;
                break;
            }
        }
    }

    let mut remap = vec![usize::MAX; N * N];
    let mut next_id = 0usize;
    for i in 0..N {
        for j in 0..N {
            let tile = tiles[i][j];
            if remap[tile] == usize::MAX {
                remap[tile] = next_id;
                next_id += 1;
            }
            tiles[i][j] = remap[tile];
        }
    }

    let mut ps = vec![vec![0; N]; N];
    for row in &mut ps {
        for value in row {
            *value = rng.gen_range(0..100);
        }
    }

    Input { s, tiles, ps }
}

fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

fn add_cell_score(doc: Document, row: usize, col: usize, value: i32) -> Document {
    doc.add(
        SvgText::new(value.to_string())
            .set("x", col * CELL_SIZE + CELL_SIZE / 2)
            .set("y", row * CELL_SIZE + 14)
            .set("font-size", 11)
            .set("text-anchor", "middle")
            .set("fill", "#0f172a"),
    )
}

fn draw_svg(
    input: &Input,
    used: &[usize],
    steps: &[(usize, usize)],
    score: i64,
    err: &str,
    turn: usize,
    max_turn: usize,
) -> String {
    let board_size = N * CELL_SIZE;
    let current = steps.last().copied().unwrap_or(input.s);
    let mut doc = Document::new()
        .set(
            "viewBox",
            format!("0 0 {} {}", board_size, board_size + FOOTER_HEIGHT),
        )
        .set("width", board_size)
        .set("height", board_size + FOOTER_HEIGHT)
        .set("style", "background:#ffffff");

    doc = doc.add(rect(0, 0, board_size, board_size, "white"));

    for i in 0..N {
        for j in 0..N {
            let tile_id = input.tiles[i][j];
            if used[tile_id] == 1 {
                doc = doc.add(rect(j * CELL_SIZE, i * CELL_SIZE, CELL_SIZE, CELL_SIZE, "skyblue"));
            } else if used[tile_id] > 1 {
                doc = doc.add(rect(
                    j * CELL_SIZE,
                    i * CELL_SIZE,
                    CELL_SIZE,
                    CELL_SIZE,
                    "royalblue",
                ));
            }
        }
    }

    let mut path = Data::new().move_to((
        input.s.1 * CELL_SIZE + CELL_SIZE / 2,
        input.s.0 * CELL_SIZE + CELL_SIZE / 2,
    ));
    for pair in steps.windows(2) {
        let prev = pair[0];
        let next = pair[1];
        path = path.line_by((
            (next.1 as i32 - prev.1 as i32) * CELL_SIZE as i32,
            (next.0 as i32 - prev.0 as i32) * CELL_SIZE as i32,
        ));
    }
    doc = doc.add(
        Path::new()
            .set("fill", "none")
            .set("stroke", "#f97316")
            .set("stroke-width", 4)
            .set("stroke-linecap", "round")
            .set("stroke-linejoin", "round")
            .set("d", path),
    );

    doc = doc.add(
        Circle::new()
            .set("cx", input.s.1 * CELL_SIZE + CELL_SIZE / 2)
            .set("cy", input.s.0 * CELL_SIZE + CELL_SIZE / 2)
            .set("r", 8)
            .set("fill", "#dc2626"),
    );
    if current == input.s {
        doc = doc.add(
            Circle::new()
                .set("cx", current.1 * CELL_SIZE + CELL_SIZE / 2)
                .set("cy", current.0 * CELL_SIZE + CELL_SIZE / 2)
                .set("r", 4)
                .set("fill", "#16a34a"),
        );
    } else {
        doc = doc.add(
            Circle::new()
                .set("cx", current.1 * CELL_SIZE + CELL_SIZE / 2)
                .set("cy", current.0 * CELL_SIZE + CELL_SIZE / 2)
                .set("r", 8)
                .set("fill", "#16a34a"),
        );
    }

    doc = doc.add(
        rect(0, 0, board_size, board_size, "none")
            .set("stroke", "black")
            .set("stroke-width", 2),
    );

    for i in 0..N {
        for j in 0..N {
            if i + 1 < N && input.tiles[i][j] != input.tiles[i + 1][j] {
                doc = doc.add(
                    Path::new()
                        .set(
                            "d",
                            Data::new()
                                .move_to((j * CELL_SIZE, i * CELL_SIZE + CELL_SIZE))
                                .line_by((CELL_SIZE as i32, 0)),
                        )
                        .set("stroke", "black")
                        .set("stroke-width", 2),
                );
            }
            if j + 1 < N && input.tiles[i][j] != input.tiles[i][j + 1] {
                doc = doc.add(
                    Path::new()
                        .set(
                            "d",
                            Data::new()
                                .move_to((j * CELL_SIZE + CELL_SIZE, i * CELL_SIZE))
                                .line_by((0, CELL_SIZE as i32)),
                        )
                        .set("stroke", "black")
                        .set("stroke-width", 2),
                );
            }
        }
    }

    for i in 0..N {
        for j in 0..N {
            doc = add_cell_score(doc, i, j, input.ps[i][j]);
        }
    }

    doc = doc.add(
        SvgText::new(format!("turn {turn}/{max_turn}"))
            .set("x", 8)
            .set("y", board_size + 20)
            .set("font-size", 16)
            .set("fill", "#111827"),
    );
    doc = doc.add(
        SvgText::new(format!("score {score}"))
            .set("x", 130)
            .set("y", board_size + 20)
            .set("font-size", 16)
            .set("fill", "#111827"),
    );
    doc = doc.add(
        SvgText::new(if err.is_empty() {
            "status OK".to_owned()
        } else {
            format!("status {err}")
        })
        .set("x", 8)
        .set("y", board_size + 42)
        .set("font-size", 16)
        .set("fill", if err.is_empty() { "#334155" } else { "#b91c1c" }),
    );

    doc.to_string()
}

pub fn generate(seed: i32, _problem_id: &str) -> String {
    gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(_input: &str, output: &str) -> usize {
    output.trim().chars().count()
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    visualize_with_mode(input, output, turn, 1, 0)
}

pub fn visualize_with_mode(
    input: &str,
    output: &str,
    turn: usize,
    _mode: usize,
    _focus_robot: usize,
) -> Result<(i64, String, String), String> {
    let input = parse_input_str(input)?;
    let output = read_output_str(&input, output);
    let max_turn = output.chars().count();
    let prefix = output
        .chars()
        .take(turn.min(max_turn))
        .collect::<String>();
    let (score, err, used, steps) = compute_score_detail(&input, &prefix);
    let svg = draw_svg(
        &input,
        &used,
        &steps,
        i64::from(score),
        &err,
        turn.min(max_turn),
        max_turn,
    );
    Ok((i64::from(score), err, svg))
}
