#![allow(non_snake_case)]

use rand::prelude::*;
use svg::node::{
    element::{Rectangle, Style, Title},
    Text,
};

trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}

impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }

    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

const R: usize = 10;
const TMAX: usize = 4000;
const TOTAL_CARS: usize = R * 10;
const CARS_PER_LINE: usize = 10;
const MAIN_LINE_CAPACITY: usize = 15;
const SIDING_LINE_CAPACITY: usize = 20;

#[derive(Clone, Debug)]
pub struct Input {
    pub R: usize,
    pub main_lines: Vec<Vec<usize>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.R)?;
        for line in &self.main_lines {
            for (i, &car) in line.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", car)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn parse_input(f: &str) -> Result<Input, String> {
    let mut it = f.split_whitespace();
    let r = it
        .next()
        .ok_or("No R")?
        .parse::<usize>()
        .map_err(|_| "Failed to parse R".to_owned())?;
    if r != R {
        return Err(format!("Invalid R: {} (expected {})", r, R));
    }

    let mut main_lines = vec![vec![]; R];
    let mut seen = vec![false; TOTAL_CARS];
    for line in main_lines.iter_mut().take(R) {
        for _ in 0..CARS_PER_LINE {
            let car = it
                .next()
                .ok_or("Not enough car IDs in input")?
                .parse::<usize>()
                .map_err(|_| "Failed to parse car ID".to_owned())?;
            if car >= TOTAL_CARS {
                return Err(format!("Car ID out of range: {}", car));
            }
            if seen[car] {
                return Err(format!("Duplicated car ID: {}", car));
            }
            seen[car] = true;
            line.push(car);
        }
    }
    Ok(Input { R: r, main_lines })
}

fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut all_cars: Vec<usize> = (0..TOTAL_CARS).collect();
    all_cars.shuffle(&mut rng);

    let mut main_lines = Vec::new();
    for r in 0..R {
        let start = r * CARS_PER_LINE;
        main_lines.push(all_cars[start..start + CARS_PER_LINE].to_vec());
    }

    Input { R, main_lines }
}

type Move = (u8, usize, usize, usize);

pub struct Output {
    pub turns: Vec<Vec<Move>>,
}

fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let mut lines = f.trim().split('\n');

    let T = lines
        .next()
        .ok_or("No turn count")?
        .trim()
        .parse::<usize>()
        .map_err(|_| "Failed to parse turn count".to_owned())?;

    if T > TMAX {
        return Err(format!("Too many turns: {} > {}", T, TMAX));
    }

    let mut turns = Vec::new();

    for turn_idx in 0..T {
        let K = lines
            .next()
            .ok_or(format!("No move count for turn {}", turn_idx))?
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("Failed to parse move count for turn {}", turn_idx))?;

        if K == 0 || K > input.R {
            return Err(format!(
                "Invalid move count for turn {}: {} (must satisfy 0 < K <= {})",
                turn_idx, K, input.R
            ));
        }

        let mut moves = Vec::new();

        for move_idx in 0..K {
            let line = lines
                .next()
                .ok_or(format!("No move {} for turn {}", move_idx, turn_idx))?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() != 4 {
                return Err(format!(
                    "Invalid format for move {} at turn {}",
                    move_idx, turn_idx
                ));
            }

            let move_type = parts[0]
                .parse::<u8>()
                .map_err(|_| "Failed to parse move_type".to_owned())?;
            let i = parts[1]
                .parse::<usize>()
                .map_err(|_| "Failed to parse i".to_owned())?;
            let j = parts[2]
                .parse::<usize>()
                .map_err(|_| "Failed to parse j".to_owned())?;
            let k = parts[3]
                .parse::<usize>()
                .map_err(|_| "Failed to parse k".to_owned())?;

            if move_type > 1 {
                return Err(format!("Invalid move_type: {}", move_type));
            }
            if i >= input.R || j >= input.R {
                return Err(format!("Index out of range: i={}, j={}", i, j));
            }
            if k == 0 {
                return Err("Number of cars to move is 0".to_owned());
            }

            moves.push((move_type, i, j, k));
        }

        turns.push(moves);
    }

    if lines.any(|line| !line.trim().is_empty()) {
        return Err("Extra output after the last turn".to_owned());
    }

    Ok(Output { turns })
}

fn check_goal(main_lines: &[Vec<usize>]) -> bool {
    for r in 0..R {
        if main_lines[r].len() != CARS_PER_LINE {
            return false;
        }
        for i in 0..CARS_PER_LINE {
            let expected = 10 * r + i;
            if main_lines[r][i] != expected {
                return false;
            }
        }
    }
    true
}

fn check_crossing(moves: &[Move]) -> Result<(), String> {
    use std::collections::HashSet;

    let mut used_main = HashSet::new();
    let mut used_siding = HashSet::new();

    for &(_move_type, i, j, _k) in moves {
        if used_main.contains(&i) {
            return Err(format!("Main line {} is used multiple times", i));
        }
        if used_siding.contains(&j) {
            return Err(format!("Siding line {} is used multiple times", j));
        }
        used_main.insert(i);
        used_siding.insert(j);
    }

    let mut sorted_moves: Vec<Move> = moves.to_vec();
    sorted_moves.sort_by_key(|&(_move_type, i, _j, _k)| i);

    for idx in 1..sorted_moves.len() {
        let (_mt1, _i1, j1, _k1) = sorted_moves[idx - 1];
        let (_mt2, _i2, j2, _k2) = sorted_moves[idx];
        if j2 <= j1 {
            return Err(format!(
                "Crossing conflict: j={} -> j={} is not strictly increasing",
                j1, j2
            ));
        }
    }

    Ok(())
}

fn apply_moves(
    main_lines: &mut [Vec<usize>],
    siding_lines: &mut [Vec<usize>],
    moves: &[Move],
    turn: usize,
) -> Result<(), String> {
    use std::collections::HashMap;

    let mut removed_from_main: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut removed_from_siding: HashMap<usize, Vec<usize>> = HashMap::new();

    for &(move_type, i, j, k) in moves {
        if move_type == 0 {
            if main_lines[i].len() < k {
                return Err(format!(
                    "Turn {}: main line {} does not have {} cars (current: {} cars)",
                    turn,
                    i,
                    k,
                    main_lines[i].len()
                ));
            }
            let start = main_lines[i].len() - k;
            let removed = main_lines[i].drain(start..).collect();
            removed_from_main.insert(i, removed);
        } else {
            if siding_lines[j].len() < k {
                return Err(format!(
                    "Turn {}: siding {} does not have {} cars (current: {} cars)",
                    turn,
                    j,
                    k,
                    siding_lines[j].len()
                ));
            }
            let removed = siding_lines[j].drain(0..k).collect();
            removed_from_siding.insert(j, removed);
        }
    }

    for &(move_type, i, j, _k) in moves {
        if move_type == 0 {
            if let Some(cars) = removed_from_main.get(&i) {
                for (idx, &car) in cars.iter().enumerate() {
                    siding_lines[j].insert(idx, car);
                }
            }
        } else if let Some(cars) = removed_from_siding.get(&j) {
            main_lines[i].extend(cars);
        }
    }

    Ok(())
}

fn compute_score_details(input: &Input, turns: &[Vec<Move>]) -> (i64, String) {
    let mut main_lines: Vec<Vec<usize>> = input.main_lines.clone();
    let mut siding_lines: Vec<Vec<usize>> = vec![vec![]; R];

    for (turn_idx, moves) in turns.iter().enumerate() {
        let turn = turn_idx + 1;

        if let Err(e) = check_crossing(moves) {
            return (0, format!("Turn {}: {}", turn, e));
        }

        if let Err(e) = apply_moves(&mut main_lines, &mut siding_lines, moves, turn) {
            return (0, e);
        }

        for i in 0..R {
            if main_lines[i].len() > MAIN_LINE_CAPACITY {
                return (
                    0,
                    format!(
                        "Turn {}: main line {} exceeds capacity ({} > {})",
                        turn,
                        i,
                        main_lines[i].len(),
                        MAIN_LINE_CAPACITY
                    ),
                );
            }
            if siding_lines[i].len() > SIDING_LINE_CAPACITY {
                return (
                    0,
                    format!(
                        "Turn {}: siding line {} exceeds capacity ({} > {})",
                        turn,
                        i,
                        siding_lines[i].len(),
                        SIDING_LINE_CAPACITY
                    ),
                );
            }
        }
    }

    if check_goal(&main_lines) {
        return ((5000 - turns.len()) as i64, String::new());
    }

    let mut score = 0i64;
    for r in 0..R {
        for (c, &car_id) in main_lines[r].iter().enumerate() {
            if car_id / 10 == r {
                score += 1;
                if car_id % 10 == c {
                    score += 9;
                }
            }
        }
    }
    (score, String::new())
}

fn color(mut val: f64) -> String {
    val.setmin(1.0);
    val.setmax(0.0);
    let (r, g, b) = if val < 0.5 {
        let x = val * 2.0;
        (
            30. * (1.0 - x) + 144. * x,
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
        )
    } else {
        let x = val * 2.0 - 1.0;
        (
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
            30. * (1.0 - x) + 70. * x,
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        r.round() as i32,
        g.round() as i32,
        b.round() as i32
    )
}

fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

fn group(title: String) -> svg::node::element::Group {
    svg::node::element::Group::new().add(Title::new().add(Text::new(title)))
}

const CAR_WIDTH: usize = 40;
const CAR_HEIGHT: usize = 15;
const LINE_WIDTH: usize = 50;
const MARGIN: usize = 25;
const MAIN_LINE_HEIGHT: usize = CAR_HEIGHT * MAIN_LINE_CAPACITY;
const SIDING_LINE_HEIGHT: usize = CAR_HEIGHT * SIDING_LINE_CAPACITY;
const LINE_SPACING: usize = 100;
const MAIN_LINE_Y: usize = 40;
const SIDING_LINE_Y: usize = MAIN_LINE_Y + MAIN_LINE_HEIGHT + LINE_SPACING;
const SVG_WIDTH: usize = MARGIN * 2 + LINE_WIDTH * R;
const SVG_HEIGHT: usize =
    MARGIN + MAIN_LINE_Y + MAIN_LINE_HEIGHT + LINE_SPACING + SIDING_LINE_HEIGHT + 50;

fn get_car_color(id: usize) -> String {
    color(id as f64 / (TOTAL_CARS - 1) as f64)
}

fn vis(input: &Input, turns: &[Vec<Move>]) -> (i64, String, String) {
    let (score, err) = compute_score_details(input, turns);

    let mut main_lines = input.main_lines.clone();
    let mut siding_lines: Vec<Vec<usize>> = vec![vec![]; R];
    let mut current_turn_moves: Vec<Move> = vec![];

    for (turn_idx, moves) in turns.iter().enumerate() {
        let turn = turn_idx + 1;
        current_turn_moves = moves.clone();

        if check_crossing(moves).is_err() {
            break;
        }

        if apply_moves(&mut main_lines, &mut siding_lines, moves, turn).is_err() {
            break;
        }

        let cap_err = (0..R).any(|i| {
            main_lines[i].len() > MAIN_LINE_CAPACITY || siding_lines[i].len() > SIDING_LINE_CAPACITY
        });
        if cap_err {
            break;
        }
    }

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -5, SVG_WIDTH + 10, SVG_HEIGHT + 10))
        .set("width", SVG_WIDTH + 10)
        .set("height", SVG_HEIGHT + 10)
        .set("style", "background-color:white");

    let defs = svg::node::element::Definitions::new().add(
        svg::node::element::Marker::new()
            .set("id", "arrowhead")
            .set("markerWidth", 10)
            .set("markerHeight", 10)
            .set("refX", 9)
            .set("refY", 3)
            .set("orient", "auto")
            .add(
                svg::node::element::Polygon::new()
                    .set("points", "0 0, 10 3, 0 6")
                    .set("fill", "#ff6b6b"),
            ),
    );
    doc = doc.add(defs);

    doc = doc.add(Style::new(
        "text {text-anchor: middle; dominant-baseline: central; font-family: monospace;}"
            .to_owned(),
    ));

    doc = doc.add(rect(0, 0, SVG_WIDTH, SVG_HEIGHT, "#f9f9f9"));

    for i in 0..R {
        let x = MARGIN + i * LINE_WIDTH;
        let y = MAIN_LINE_Y;

        doc = doc.add(
            rect(x, y, CAR_WIDTH, MAIN_LINE_HEIGHT, "#f0f0f0")
                .set("stroke", "#999")
                .set("stroke-width", 1),
        );

        doc = doc.add(
            svg::node::element::Text::new()
                .set("x", x + CAR_WIDTH / 2)
                .set("y", y - 10)
                .set("font-size", 10)
                .set("font-weight", "bold")
                .set("text-anchor", "middle")
                .add(svg::node::Text::new(format!("{}", i))),
        );

        for (idx, &car_id) in main_lines[i].iter().enumerate() {
            let car_y = y + idx * CAR_HEIGHT;
            let car_color = get_car_color(car_id);

            doc = doc.add(
                rect(x, car_y, CAR_WIDTH, CAR_HEIGHT, &car_color)
                    .set("stroke", "#333")
                    .set("stroke-width", 1)
                    .set("rx", 2),
            );

            doc = doc.add(
                svg::node::element::Text::new()
                    .set("x", x + CAR_WIDTH / 2)
                    .set("y", car_y + CAR_HEIGHT / 2 + 1)
                    .set("font-size", 10)
                    .set("font-weight", "bold")
                    .set("fill", "#000")
                    .add(svg::node::Text::new(format!("{}", car_id))),
            );
        }
    }

    for i in 0..R {
        let x = MARGIN + i * LINE_WIDTH;
        let y = SIDING_LINE_Y;

        doc = doc.add(
            rect(x, y, CAR_WIDTH, SIDING_LINE_HEIGHT, "#f0f0f0")
                .set("stroke", "#999")
                .set("stroke-width", 1),
        );

        doc = doc.add(
            svg::node::element::Text::new()
                .set("x", x + CAR_WIDTH / 2)
                .set("y", y + SIDING_LINE_HEIGHT + 10)
                .set("font-size", 10)
                .set("font-weight", "bold")
                .set("text-anchor", "middle")
                .add(svg::node::Text::new(format!("{}", i))),
        );

        let siding_car_offset_y = SIDING_LINE_HEIGHT - siding_lines[i].len() * CAR_HEIGHT;
        for (idx, &car_id) in siding_lines[i].iter().enumerate() {
            let car_y = y + siding_car_offset_y + idx * CAR_HEIGHT;
            let car_color = get_car_color(car_id);

            doc = doc.add(
                rect(x, car_y, CAR_WIDTH, CAR_HEIGHT, &car_color)
                    .set("stroke", "#333")
                    .set("stroke-width", 1)
                    .set("rx", 2),
            );

            doc = doc.add(
                svg::node::element::Text::new()
                    .set("x", x + CAR_WIDTH / 2)
                    .set("y", car_y + CAR_HEIGHT / 2 + 1)
                    .set("font-size", 10)
                    .set("font-weight", "bold")
                    .set("fill", "#000")
                    .add(svg::node::Text::new(format!("{}", car_id))),
            );
        }
    }

    for &(move_type, i, j, _k) in &current_turn_moves {
        let main_x = (MARGIN + i * LINE_WIDTH + CAR_WIDTH / 2) as f64;
        let siding_x = (MARGIN + j * LINE_WIDTH + CAR_WIDTH / 2) as f64;

        let (start_x, start_y, end_x, end_y) = if move_type == 0 {
            (
                main_x,
                (MAIN_LINE_Y + MAIN_LINE_HEIGHT) as f64,
                siding_x,
                SIDING_LINE_Y as f64,
            )
        } else {
            (
                siding_x,
                SIDING_LINE_Y as f64,
                main_x,
                (MAIN_LINE_Y + MAIN_LINE_HEIGHT) as f64,
            )
        };

        let path_data = format!("M {} {} L {} {}", start_x, start_y, end_x, end_y);
        doc = doc.add(
            svg::node::element::Path::new()
                .set("d", path_data)
                .set("stroke", "#ff6b6b")
                .set("stroke-width", 2)
                .set("fill", "none")
                .set("marker-end", "url(#arrowhead)"),
        );
    }

    if err.is_empty() {
        for &(move_type, i, j, k) in &current_turn_moves {
            let (hx, hy, hh) = if move_type == 0 {
                let siding_len = siding_lines[j].len();
                let x = MARGIN + j * LINE_WIDTH;
                let y = SIDING_LINE_Y + (SIDING_LINE_HEIGHT - siding_len * CAR_HEIGHT);
                (x, y, k * CAR_HEIGHT)
            } else {
                let x = MARGIN + i * LINE_WIDTH;
                let len = main_lines[i].len();
                let y = MAIN_LINE_Y + (len - k) * CAR_HEIGHT;
                (x, y, k * CAR_HEIGHT)
            };
            doc = doc.add(
                Rectangle::new()
                    .set("x", hx)
                    .set("y", hy)
                    .set("width", CAR_WIDTH)
                    .set("height", hh)
                    .set("fill", "none")
                    .set("stroke", "#000")
                    .set("stroke-width", 3)
                    .set("rx", 2),
            );
        }

        for &(move_type, i, j, k) in &current_turn_moves {
            let (hx, hy, hh) = if move_type == 0 {
                let x = MARGIN + i * LINE_WIDTH;
                let len = main_lines[i].len() + k;
                let y = MAIN_LINE_Y + (len - k) * CAR_HEIGHT;
                (x, y, k * CAR_HEIGHT)
            } else {
                let siding_len = siding_lines[j].len() + k;
                let x = MARGIN + j * LINE_WIDTH;
                let y = SIDING_LINE_Y + (SIDING_LINE_HEIGHT - siding_len * CAR_HEIGHT);
                (x, y, k * CAR_HEIGHT)
            };
            doc = doc.add(
                Rectangle::new()
                    .set("x", hx)
                    .set("y", hy)
                    .set("width", CAR_WIDTH)
                    .set("height", hh)
                    .set("fill", "none")
                    .set("stroke", "#000")
                    .set("stroke-width", 3)
                    .set("stroke-dasharray", "5,5")
                    .set("rx", 2),
            );
        }
    }

    let _ = group(String::new());
    (score, err, doc.to_string())
}

pub fn generate(seed: i32) -> String {
    gen(seed.max(0) as u64).to_string()
}

pub fn calc_max_turn(_input: &str, output: &str) -> usize {
    let first = match output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
    {
        Some(line) => line,
        None => return 0,
    };
    first.parse::<usize>().unwrap_or(0).min(TMAX)
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    let input = parse_input(input)?;
    if output.trim().is_empty() {
        let (_score, _err, svg) = vis(&input, &[]);
        return Ok((0, String::new(), svg));
    }

    let out = match parse_output(&input, output) {
        Ok(out) => out,
        Err(err) => {
            let (_score, _unused_err, svg) = vis(&input, &[]);
            return Ok((0, err, svg));
        }
    };
    let max_turn = out.turns.len();
    let clipped_turn = turn.min(max_turn);
    let turns = &out.turns[..clipped_turn];
    let (mut score, err, svg) = vis(&input, turns);
    if !err.is_empty() {
        score = 0;
    }
    Ok((score, err, svg))
}
