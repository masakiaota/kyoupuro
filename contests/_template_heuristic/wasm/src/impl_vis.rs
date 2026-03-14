use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use svg::node::element::{Circle, Line, Rectangle, Text as SvgText};
use svg::Document;

#[derive(Clone)]
struct Input {
    h: usize,
    w: usize,
    start_y: usize,
    start_x: usize,
    cells: Vec<Vec<char>>,
}

#[derive(Clone, Copy)]
struct Pos {
    y: usize,
    x: usize,
}

fn default_problem(problem_id: &str) -> char {
    match problem_id.chars().next().unwrap_or('A') {
        'B' => 'B',
        'C' => 'C',
        _ => 'A',
    }
}

fn parse_input(input: &str) -> Result<Input, String> {
    let mut lines = input.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "input is empty".to_owned())?
        .split_whitespace()
        .collect::<Vec<_>>();
    if header.len() != 4 {
        return Err("input header must be: H W start_y start_x".to_owned());
    }

    let h = header[0]
        .parse::<usize>()
        .map_err(|_| "failed to parse H".to_owned())?;
    let w = header[1]
        .parse::<usize>()
        .map_err(|_| "failed to parse W".to_owned())?;
    let start_y = header[2]
        .parse::<usize>()
        .map_err(|_| "failed to parse start_y".to_owned())?;
    let start_x = header[3]
        .parse::<usize>()
        .map_err(|_| "failed to parse start_x".to_owned())?;

    if h == 0 || w == 0 {
        return Err("H and W must be positive".to_owned());
    }
    if start_y >= h || start_x >= w {
        return Err("start position is out of range".to_owned());
    }

    let mut cells = Vec::with_capacity(h);
    for _ in 0..h {
        let row = lines
            .next()
            .ok_or_else(|| "grid row is missing".to_owned())?
            .chars()
            .collect::<Vec<_>>();
        if row.len() != w {
            return Err("grid width does not match W".to_owned());
        }
        if row.iter().any(|&c| c != '.' && c != '#') {
            return Err("grid can contain only '.' or '#'".to_owned());
        }
        cells.push(row);
    }

    if cells[start_y][start_x] == '#' {
        return Err("start cell must be '.'".to_owned());
    }

    Ok(Input {
        h,
        w,
        start_y,
        start_x,
        cells,
    })
}

fn parse_moves(output: &str) -> (Vec<char>, String) {
    let mut moves = Vec::new();
    let mut errors = Vec::new();

    for (idx, raw_line) in output.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let ch = line.chars().next().unwrap_or('?');
        if matches!(ch, 'U' | 'D' | 'L' | 'R') {
            moves.push(ch);
        } else {
            errors.push(format!("line {}: unsupported move '{}'", idx + 1, line));
        }
    }

    (moves, errors.join("\n"))
}

fn apply_move(input: &Input, pos: Pos, mv: char) -> Pos {
    let (dy, dx) = match mv {
        'U' => (-1isize, 0isize),
        'D' => (1, 0),
        'L' => (0, -1),
        'R' => (0, 1),
        _ => (0, 0),
    };
    let ny = pos.y as isize + dy;
    let nx = pos.x as isize + dx;
    if ny < 0 || ny >= input.h as isize || nx < 0 || nx >= input.w as isize {
        return pos;
    }
    let next = Pos {
        y: ny as usize,
        x: nx as usize,
    };
    if input.cells[next.y][next.x] == '#' {
        pos
    } else {
        next
    }
}

fn simulate(input: &Input, moves: &[char], turn: usize) -> (Vec<Pos>, Vec<Vec<usize>>) {
    let limit = turn.min(moves.len());
    let mut pos = Pos {
        y: input.start_y,
        x: input.start_x,
    };
    let mut history = vec![pos];
    let mut visited = vec![vec![0usize; input.w]; input.h];
    visited[pos.y][pos.x] += 1;

    for &mv in moves.iter().take(limit) {
        pos = apply_move(input, pos, mv);
        history.push(pos);
        visited[pos.y][pos.x] += 1;
    }

    (history, visited)
}

fn color_for_cell(cell: char, visits: usize, mode: usize) -> &'static str {
    if cell == '#' {
        return "#374151";
    }
    if mode == 2 {
        match visits {
            0 => "#f8fafc",
            1 => "#dbeafe",
            2 => "#93c5fd",
            _ => "#2563eb",
        }
    } else {
        "#f8fafc"
    }
}

fn draw_svg(
    input: &Input,
    history: &[Pos],
    visited: &[Vec<usize>],
    turn: usize,
    max_turn: usize,
    mode: usize,
) -> String {
    let cell = 28usize;
    let pad = 24usize;
    let width = pad * 2 + input.w * cell;
    let height = pad * 2 + input.h * cell + 48;

    let mut doc = Document::new()
        .set("viewBox", format!("0 0 {width} {height}"))
        .set("width", width)
        .set("height", height)
        .set("style", "background:#ffffff");

    for y in 0..input.h {
        for x in 0..input.w {
            doc = doc.add(
                Rectangle::new()
                    .set("x", pad + x * cell)
                    .set("y", pad + y * cell)
                    .set("width", cell)
                    .set("height", cell)
                    .set("fill", color_for_cell(input.cells[y][x], visited[y][x], mode))
                    .set("stroke", "#cbd5e1")
                    .set("stroke-width", 1),
            );
        }
    }

    if history.len() >= 2 {
        for seg in history.windows(2) {
            let p0 = seg[0];
            let p1 = seg[1];
            doc = doc.add(
                Line::new()
                    .set("x1", pad + p0.x * cell + cell / 2)
                    .set("y1", pad + p0.y * cell + cell / 2)
                    .set("x2", pad + p1.x * cell + cell / 2)
                    .set("y2", pad + p1.y * cell + cell / 2)
                    .set("stroke", "#0f766e")
                    .set("stroke-width", 4)
                    .set("stroke-linecap", "round")
                    .set("opacity", 0.75),
            );
        }
    }

    let start = history[0];
    doc = doc.add(
        Circle::new()
            .set("cx", pad + start.x * cell + cell / 2)
            .set("cy", pad + start.y * cell + cell / 2)
            .set("r", cell / 4)
            .set("fill", "#f59e0b"),
    );

    let current = *history.last().unwrap_or(&start);
    doc = doc.add(
        Circle::new()
            .set("cx", pad + current.x * cell + cell / 2)
            .set("cy", pad + current.y * cell + cell / 2)
            .set("r", cell / 3)
            .set("fill", "#dc2626"),
    );

    let score = visited
        .iter()
        .flatten()
        .filter(|&&count| count > 0)
        .count();
    doc = doc.add(
        SvgText::new(format!(
            "template visualizer  turn {turn}/{max_turn}  score {score}"
        ))
        .set("x", pad)
        .set("y", height - 16)
        .set("font-size", 16)
        .set("fill", "#111827"),
    );

    doc.to_string()
}

pub fn generate(seed: i32, problem_id: &str) -> String {
    let mut rng = ChaCha20Rng::seed_from_u64(seed.max(0) as u64 + 1);
    let problem = default_problem(problem_id);
    let h = 20usize;
    let w = 20usize;
    let density = match problem {
        'B' => 0.16f64,
        'C' => 0.08f64,
        _ => 0.12f64,
    };
    let mut cells = vec![vec!['.'; w]; h];
    for row in cells.iter_mut().take(h) {
        for cell in row.iter_mut().take(w) {
            if rng.gen_bool(density) {
                *cell = '#';
            }
        }
    }
    cells[0][0] = '.';
    let mut out = format!("{h} {w} 0 0\n");
    for row in cells {
        out.push_str(&row.into_iter().collect::<String>());
        out.push('\n');
    }
    out
}

pub fn calc_max_turn(_input: &str, output: &str) -> usize {
    let (moves, _) = parse_moves(output);
    moves.len()
}

pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String> {
    visualize_with_mode(input, output, turn, 1, 0)
}

pub fn visualize_with_mode(
    input: &str,
    output: &str,
    turn: usize,
    mode: usize,
    _focus_robot: usize,
) -> Result<(i64, String, String), String> {
    let input = parse_input(input)?;
    let (moves, parse_err) = parse_moves(output);
    let max_turn = moves.len();
    let (history, visited) = simulate(&input, &moves, turn);
    let score = visited
        .iter()
        .flatten()
        .filter(|&&count| count > 0)
        .count() as i64;
    let svg = draw_svg(&input, &history, &visited, turn.min(max_turn), max_turn, mode);
    let err = if mode == 3 {
        if parse_err.is_empty() {
            "template mode: focus_robot is unused until problem-specific visualizer is implemented"
                .to_owned()
        } else {
            format!(
                "template mode: focus_robot is unused until problem-specific visualizer is implemented\n{}",
                parse_err
            )
        }
    } else {
        parse_err
    };
    Ok((score, err, svg))
}
