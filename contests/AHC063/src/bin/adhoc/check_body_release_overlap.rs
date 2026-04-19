// check_body_release_overlap.rs
use std::collections::VecDeque;

const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Cell(u16);

struct Grid;

impl Grid {
    fn cell(n: usize, i: usize, j: usize) -> Cell {
        Cell((i * n + j) as u16)
    }

    fn index(cell: Cell) -> usize {
        cell.0 as usize
    }

    fn ij(n: usize, cell: Cell) -> (usize, usize) {
        let idx = Self::index(cell);
        (idx / n, idx % n)
    }

    fn can_move(n: usize, cell: Cell, dir: usize) -> bool {
        let idx = Self::index(cell);
        match dir {
            0 => idx >= n,
            1 => idx + n < n * n,
            2 => idx % n != 0,
            3 => idx % n + 1 < n,
            _ => false,
        }
    }

    fn next_cell(n: usize, cell: Cell, dir: usize) -> Cell {
        let idx = Self::index(cell);
        match dir {
            0 => Cell((idx - n) as u16),
            1 => Cell((idx + n) as u16),
            2 => Cell((idx - 1) as u16),
            3 => Cell((idx + 1) as u16),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
struct State {
    n: usize,
    food: Vec<u8>,
    pos: Vec<Cell>,
}

struct Case {
    name: &'static str,
    state: State,
    dir: usize,
    expect_actual_legal: bool,
    expect_current_dist: Option<usize>,
    expect_fixed_dist: Option<usize>,
}

fn compute_body_release_dist_current(state: &State) -> Vec<usize> {
    let n = state.n;
    let cell_count = n * n;
    let inf = usize::MAX;

    let mut release = vec![0usize; cell_count];
    for j in 0..state.pos.len() {
        let idx = Grid::index(state.pos[j]);
        let t = state.pos.len() - j;
        release[idx] = release[idx].max(t);
    }

    let mut dist = vec![inf; cell_count];
    let mut q = VecDeque::new();
    let head = state.pos[0];
    dist[Grid::index(head)] = 0;
    q.push_back(head);

    while let Some(cur) = q.pop_front() {
        let cd = dist[Grid::index(cur)];
        for dir in 0..4 {
            if !Grid::can_move(n, cur, dir) {
                continue;
            }
            let nxt = Grid::next_cell(n, cur, dir);
            let nxt_idx = Grid::index(nxt);
            let nd = cd + 1;
            if dist[nxt_idx] != inf {
                continue;
            }
            if state.food[nxt_idx] != 0 {
                continue;
            }
            if nd < release[nxt_idx] {
                continue;
            }
            dist[nxt_idx] = nd;
            q.push_back(nxt);
        }
    }

    dist
}

fn compute_body_release_dist_fixed(state: &State) -> Vec<usize> {
    let n = state.n;
    let cell_count = n * n;
    let inf = usize::MAX;
    let len = state.pos.len();

    let mut front_idx = vec![None; cell_count];
    for j in 0..len {
        let idx = Grid::index(state.pos[j]);
        if front_idx[idx].is_none() {
            front_idx[idx] = Some(j);
        }
    }

    let mut release = vec![0usize; cell_count];
    for idx in 0..cell_count {
        if let Some(j) = front_idx[idx] {
            release[idx] = (len - 1 - j).max(1);
        }
    }

    let mut dist = vec![inf; cell_count];
    let mut q = VecDeque::new();
    let head = state.pos[0];
    dist[Grid::index(head)] = 0;
    q.push_back(head);

    while let Some(cur) = q.pop_front() {
        let cd = dist[Grid::index(cur)];
        for dir in 0..4 {
            if !Grid::can_move(n, cur, dir) {
                continue;
            }
            let nxt = Grid::next_cell(n, cur, dir);
            let nxt_idx = Grid::index(nxt);
            let nd = cd + 1;
            if dist[nxt_idx] != inf {
                continue;
            }
            if state.food[nxt_idx] != 0 {
                continue;
            }
            if nd < release[nxt_idx] {
                continue;
            }
            dist[nxt_idx] = nd;
            q.push_back(nxt);
        }
    }

    dist
}

fn is_legal_dir(state: &State, dir: usize) -> bool {
    let head = state.pos[0];
    if !Grid::can_move(state.n, head, dir) {
        return false;
    }
    if state.pos.len() >= 2 {
        let nxt = Grid::next_cell(state.n, head, dir);
        if nxt == state.pos[1] {
            return false;
        }
    }
    true
}

fn step_noeat_like_template(state: &State, dir: usize) -> Option<State> {
    if !is_legal_dir(state, dir) {
        return None;
    }
    let n = state.n;
    let next_head = Grid::next_cell(n, state.pos[0], dir);
    if state.food[Grid::index(next_head)] != 0 {
        return None;
    }

    let mut pos = state.pos.clone();
    pos.pop();
    let excluded_tail = *pos.last()?;

    let mut occ = vec![0u8; n * n];
    for &cell in &pos {
        occ[Grid::index(cell)] += 1;
    }
    let tail_bias = u8::from(excluded_tail == next_head);
    if occ[Grid::index(next_head)] > tail_bias {
        return None;
    }

    pos.insert(0, next_head);
    Some(State {
        n,
        food: state.food.clone(),
        pos,
    })
}

fn format_pos(state: &State) -> Vec<(usize, usize)> {
    state
        .pos
        .iter()
        .map(|&cell| Grid::ij(state.n, cell))
        .collect()
}

fn format_dist(d: usize) -> String {
    match d {
        usize::MAX => "INF".to_string(),
        _ => d.to_string(),
    }
}

fn make_food_blocked_state(pos: Vec<Cell>) -> State {
    let n = 5;
    let mut food = vec![0; n * n];
    for &(i, j) in &[
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (1, 0),
        (1, 3),
        (2, 0),
        (2, 3),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
    ] {
        food[i * n + j] = 1;
    }
    State { n, food, pos }
}

fn run_case(case: &Case) {
    let state = &case.state;
    let head = state.pos[0];
    let target = Grid::next_cell(state.n, head, case.dir);
    let current = compute_body_release_dist_current(state);
    let fixed = compute_body_release_dist_fixed(state);
    let stepped = step_noeat_like_template(state, case.dir);
    let actual_legal = stepped.is_some();
    let current_dist = current[Grid::index(target)];
    let fixed_dist = fixed[Grid::index(target)];

    println!("== {} ==", case.name);
    println!("state.pos = {:?}", format_pos(state));
    println!(
        "head = {:?}, target = {:?}, dir = {}",
        Grid::ij(state.n, head),
        Grid::ij(state.n, target),
        DIRS[case.dir].2
    );
    println!("actual legal = {}", actual_legal);
    println!("current dist = {}", format_dist(current_dist));
    println!("fixed dist = {}", format_dist(fixed_dist));
    if let Some(ns) = stepped {
        println!("next_state.pos = {:?}", format_pos(&ns));
    }
    println!();

    assert_eq!(actual_legal, case.expect_actual_legal, "{}", case.name);
    match case.expect_current_dist {
        Some(d) => assert_eq!(current_dist, d, "{}", case.name),
        None => assert_eq!(current_dist, usize::MAX, "{}", case.name),
    }
    match case.expect_fixed_dist {
        Some(d) => assert_eq!(fixed_dist, d, "{}", case.name),
        None => assert_eq!(fixed_dist, usize::MAX, "{}", case.name),
    }
}

fn main() {
    let n = 5;
    let cases = vec![
        Case {
            name: "overlap_minimal_counterexample",
            state: make_food_blocked_state(vec![
                Grid::cell(n, 1, 1),
                Grid::cell(n, 2, 1),
                Grid::cell(n, 2, 2),
                Grid::cell(n, 1, 2),
                Grid::cell(n, 1, 1),
            ]),
            dir: 3,
            expect_actual_legal: true,
            expect_current_dist: None,
            expect_fixed_dist: Some(1),
        },
        Case {
            name: "excluded_tail_without_overlap",
            state: make_food_blocked_state(vec![
                Grid::cell(n, 1, 1),
                Grid::cell(n, 2, 1),
                Grid::cell(n, 2, 2),
                Grid::cell(n, 1, 2),
                Grid::cell(n, 0, 2),
            ]),
            dir: 3,
            expect_actual_legal: true,
            expect_current_dist: None,
            expect_fixed_dist: Some(1),
        },
        Case {
            name: "front_occupancy_prevents_overcorrection",
            state: make_food_blocked_state(vec![
                Grid::cell(n, 1, 1),
                Grid::cell(n, 1, 2),
                Grid::cell(n, 2, 2),
                Grid::cell(n, 1, 2),
                Grid::cell(n, 1, 1),
            ]),
            dir: 3,
            expect_actual_legal: false,
            expect_current_dist: None,
            expect_fixed_dist: Some(3),
        },
    ];

    for case in &cases {
        run_case(case);
    }
}
