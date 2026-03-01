// v003_b_route.rs
use std::collections::VecDeque;
use std::io::{self, Read};

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DIR_CHARS: [char; 4] = ['U', 'R', 'D', 'L'];
const N_FIXED: usize = 20;
const M_LIMIT: usize = 4 * N_FIXED * N_FIXED; // 1600

#[derive(Clone)]
struct Input {
    n: usize,
    ak: i64,
    am: i64,
    aw: i64,
    wall_v: Vec<Vec<u8>>,
    wall_h: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct RouteCandidate {
    start_cell: usize,
    start_dir: usize,
    actions: Vec<u8>,
}

fn act_char(a: u8) -> char {
    match a {
        ACT_R => 'R',
        ACT_L => 'L',
        ACT_F => 'F',
        _ => unreachable!(),
    }
}

fn parse_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let n: usize = it.next().unwrap().parse().unwrap();
    let ak: i64 = it.next().unwrap().parse().unwrap();
    let am: i64 = it.next().unwrap().parse().unwrap();
    let aw: i64 = it.next().unwrap().parse().unwrap();
    let mut wall_v = vec![vec![0u8; n - 1]; n];
    for row in wall_v.iter_mut().take(n) {
        let line = it.next().unwrap().as_bytes();
        for (j, v) in row.iter_mut().enumerate().take(n - 1) {
            *v = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    let mut wall_h = vec![vec![0u8; n]; n - 1];
    for row in wall_h.iter_mut().take(n - 1) {
        let line = it.next().unwrap().as_bytes();
        for (j, v) in row.iter_mut().enumerate().take(n) {
            *v = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    Input {
        n,
        ak,
        am,
        aw,
        wall_v,
        wall_h,
    }
}

fn has_wall(input: &Input, i: usize, j: usize, d: usize) -> bool {
    let ni = i as isize + DIJ[d].0;
    let nj = j as isize + DIJ[d].1;
    if ni < 0 || ni >= input.n as isize || nj < 0 || nj >= input.n as isize {
        return true;
    }
    let ni = ni as usize;
    let nj = nj as usize;
    if ni == i {
        input.wall_v[i][j.min(nj)] == 1
    } else {
        input.wall_h[i.min(ni)][j] == 1
    }
}

fn build_graph(input: &Input) -> Vec<Vec<(usize, usize)>> {
    let n = input.n;
    let mut g = vec![Vec::<(usize, usize)>::new(); n * n];
    for i in 0..n {
        for j in 0..n {
            let v = i * n + j;
            for d in 0..4 {
                if !has_wall(input, i, j, d) {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    let to = ni * n + nj;
                    g[v].push((d, to));
                }
            }
        }
    }
    g
}

fn bfs_dirs(g: &[Vec<(usize, usize)>], start: usize, goal: usize) -> Vec<usize> {
    if start == goal {
        return Vec::new();
    }
    let n2 = g.len();
    let mut prev = vec![usize::MAX; n2];
    let mut pdir = vec![0usize; n2];
    let mut q = VecDeque::<usize>::new();
    prev[start] = start;
    q.push_back(start);
    while let Some(v) = q.pop_front() {
        for &(d, to) in &g[v] {
            if prev[to] == usize::MAX {
                prev[to] = v;
                pdir[to] = d;
                if to == goal {
                    q.clear();
                    break;
                }
                q.push_back(to);
            }
        }
    }
    let mut rev = Vec::<usize>::new();
    let mut cur = goal;
    while cur != start {
        rev.push(pdir[cur]);
        cur = prev[cur];
    }
    rev.reverse();
    rev
}

fn rot_cost(from: usize, to: usize) -> usize {
    let diff = (to + 4 - from) % 4;
    match diff {
        0 => 0,
        1 | 3 => 1,
        2 => 2,
        _ => unreachable!(),
    }
}

fn append_turns(cur: &mut usize, to: usize, actions: &mut Vec<u8>) {
    let diff = (to + 4 - *cur) % 4;
    match diff {
        0 => {}
        1 => {
            actions.push(ACT_R);
            *cur = (*cur + 1) % 4;
        }
        2 => {
            actions.push(ACT_R);
            actions.push(ACT_R);
            *cur = (*cur + 2) % 4;
        }
        3 => {
            actions.push(ACT_L);
            *cur = (*cur + 3) % 4;
        }
        _ => unreachable!(),
    }
}

fn row_snake(n: usize) -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(n * n);
    for i in 0..n {
        if i % 2 == 0 {
            for j in 0..n {
                order.push(i * n + j);
            }
        } else {
            for j in (0..n).rev() {
                order.push(i * n + j);
            }
        }
    }
    order
}

fn col_snake(n: usize) -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(n * n);
    for j in 0..n {
        if j % 2 == 0 {
            for i in 0..n {
                order.push(i * n + j);
            }
        } else {
            for i in (0..n).rev() {
                order.push(i * n + j);
            }
        }
    }
    order
}

fn build_moves_by_order(g: &[Vec<(usize, usize)>], order: &[usize]) -> Vec<usize> {
    let start = order[0];
    let mut cur = start;
    let mut moves = Vec::<usize>::new();
    for &t in order {
        let path = bfs_dirs(g, cur, t);
        moves.extend(path);
        cur = t;
    }
    let back = bfs_dirs(g, cur, start);
    moves.extend(back);
    moves
}

fn build_candidate_from_moves(start_cell: usize, moves: &[usize]) -> Option<RouteCandidate> {
    let mut best_sd = 0usize;
    let mut best_len = usize::MAX;

    for sd in 0..4 {
        let mut turns = 0usize;
        let mut cur = sd;
        for &md in moves {
            turns += rot_cost(cur, md);
            cur = md;
        }
        turns += rot_cost(cur, sd);
        let len = moves.len() + turns;
        if len < best_len {
            best_len = len;
            best_sd = sd;
        }
    }
    if best_len == usize::MAX || best_len > M_LIMIT {
        return None;
    }

    let mut actions = Vec::<u8>::with_capacity(best_len);
    let mut cur = best_sd;
    for &md in moves {
        append_turns(&mut cur, md, &mut actions);
        actions.push(ACT_F);
        cur = md;
    }
    append_turns(&mut cur, best_sd, &mut actions);
    if actions.is_empty() || actions.len() > M_LIMIT {
        return None;
    }
    Some(RouteCandidate {
        start_cell,
        start_dir: best_sd,
        actions,
    })
}

fn build_best_route(input: &Input) -> RouteCandidate {
    let g = build_graph(input);
    let row = row_snake(input.n);
    let col = col_snake(input.n);
    let mut orders = Vec::<Vec<usize>>::new();
    orders.push(row.clone());
    orders.push(row.iter().copied().rev().collect());
    orders.push(col.clone());
    orders.push(col.iter().copied().rev().collect());

    let mut best: Option<RouteCandidate> = None;
    for order in &orders {
        let moves = build_moves_by_order(&g, order);
        if let Some(cand) = build_candidate_from_moves(order[0], &moves) {
            let better = match &best {
                None => true,
                Some(cur) => cand.actions.len() < cur.actions.len(),
            };
            if better {
                best = Some(cand);
            }
        }
    }
    best.expect("route candidate not found")
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);
    let _ = (input.ak, input.am, input.aw);
    let route = build_best_route(&input);

    let m = route.actions.len();
    println!("1");
    println!(
        "{} {} {} {}",
        m,
        route.start_cell / input.n,
        route.start_cell % input.n,
        DIR_CHARS[route.start_dir]
    );
    for s in 0..m {
        let act = route.actions[s];
        let ns = (s + 1) % m;
        let a1 = if act == ACT_F { ACT_R } else { act };
        println!("{} {} {} {}", act_char(act), ns, act_char(a1), ns);
    }

    let zeros_v = "0".repeat(input.n - 1);
    let zeros_h = "0".repeat(input.n);
    for _ in 0..input.n {
        println!("{}", zeros_v);
    }
    for _ in 0..input.n - 1 {
        println!("{}", zeros_h);
    }
}
