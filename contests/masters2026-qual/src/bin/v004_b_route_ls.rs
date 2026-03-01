// v004_b_route_ls.rs
use std::collections::VecDeque;
use std::io::{self, Read};
use std::time::{Duration, Instant};

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DIR_CHARS: [char; 4] = ['U', 'R', 'D', 'L'];
const N_FIXED: usize = 20;
const N2: usize = N_FIXED * N_FIXED;
const M_LIMIT: usize = 4 * N_FIXED * N_FIXED; // 1600
const DEFAULT_SEARCH_TIME_MS: u64 = 1000;

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

#[derive(Clone, Copy)]
struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(mut seed: u64) -> Self {
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        Self { x: seed }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }
    fn gen_usize(&mut self, upper: usize) -> usize {
        (self.next_u64() % upper as u64) as usize
    }
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

fn seed_from_input(input: &Input) -> u64 {
    let mut h = 1469598103934665603u64;
    let mut mix = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(1099511628211);
    };
    mix(input.n as u64);
    mix(input.ak as u64);
    mix(input.am as u64);
    mix(input.aw as u64);
    for i in 0..input.n {
        for j in 0..input.n - 1 {
            mix(input.wall_v[i][j] as u64);
        }
    }
    for i in 0..input.n - 1 {
        for j in 0..input.n {
            mix(input.wall_h[i][j] as u64);
        }
    }
    h
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
    let mut g = vec![Vec::<(usize, usize)>::new(); N2];
    for i in 0..input.n {
        for j in 0..input.n {
            let v = i * input.n + j;
            for d in 0..4 {
                if !has_wall(input, i, j, d) {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    g[v].push((d, ni * input.n + nj));
                }
            }
        }
    }
    g
}

fn precompute_shortest(
    g: &[Vec<(usize, usize)>],
) -> (Vec<[u16; N2]>, Vec<[u8; N2]>, Vec<[u16; 4]>) {
    let mut dist = vec![[u16::MAX; N2]; N2];
    let mut first_dir = vec![[255u8; N2]; N2];
    let mut next_by_dir = vec![[u16::MAX; 4]; N2];
    for v in 0..N2 {
        for &(d, to) in &g[v] {
            next_by_dir[v][d] = to as u16;
        }
    }
    let mut q = VecDeque::<usize>::new();
    for s in 0..N2 {
        q.clear();
        dist[s][s] = 0;
        first_dir[s][s] = 0;
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            let dv = dist[s][v];
            for &(d, to) in &g[v] {
                if dist[s][to] == u16::MAX {
                    dist[s][to] = dv + 1;
                    first_dir[s][to] = if v == s {
                        d as u8
                    } else {
                        first_dir[s][v]
                    };
                    q.push_back(to);
                }
            }
        }
    }
    (dist, first_dir, next_by_dir)
}

fn tour_cost(order: &[usize], dist: &[[u16; N2]]) -> i64 {
    let mut c = 0i64;
    for i in 0..N2 {
        let a = order[i];
        let b = order[(i + 1) % N2];
        c += dist[a][b] as i64;
    }
    c
}

fn two_opt_once(order: &mut [usize], dist: &[[u16; N2]], rng: &mut XorShift64) -> bool {
    let n = order.len();
    let offset = rng.gen_usize(n);
    for t in 0..n {
        let l = (offset + t) % n;
        let a = order[(l + n - 1) % n];
        let b = order[l];
        let mut r = l + 2;
        while r + 1 < n {
            let c = order[r];
            let d = order[(r + 1) % n];
            let old = dist[a][b] as i64 + dist[c][d] as i64;
            let newv = dist[a][c] as i64 + dist[b][d] as i64;
            if newv < old {
                order[l..=r].reverse();
                return true;
            }
            r += 1;
        }
    }
    false
}

fn row_snake() -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(N2);
    for i in 0..N_FIXED {
        if i % 2 == 0 {
            for j in 0..N_FIXED {
                order.push(i * N_FIXED + j);
            }
        } else {
            for j in (0..N_FIXED).rev() {
                order.push(i * N_FIXED + j);
            }
        }
    }
    order
}

fn col_snake() -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(N2);
    for j in 0..N_FIXED {
        if j % 2 == 0 {
            for i in 0..N_FIXED {
                order.push(i * N_FIXED + j);
            }
        } else {
            for i in (0..N_FIXED).rev() {
                order.push(i * N_FIXED + j);
            }
        }
    }
    order
}

fn nearest_neighbor_tour(start: usize, dist: &[[u16; N2]]) -> Vec<usize> {
    let mut used = [false; N2];
    let mut order = Vec::<usize>::with_capacity(N2);
    let mut cur = start;
    used[cur] = true;
    order.push(cur);
    for _ in 1..N2 {
        let mut best = usize::MAX;
        let mut best_d = u16::MAX;
        for (v, &used_v) in used.iter().enumerate() {
            if !used_v {
                let d = dist[cur][v];
                if d < best_d {
                    best_d = d;
                    best = v;
                }
            }
        }
        cur = best;
        used[cur] = true;
        order.push(cur);
    }
    order
}

fn reconstruct_moves(
    order: &[usize],
    first_dir: &[[u8; N2]],
    next_by_dir: &[[u16; 4]],
) -> Vec<u8> {
    let mut moves = Vec::<u8>::new();
    for i in 0..N2 {
        let mut cur = order[i];
        let goal = order[(i + 1) % N2];
        while cur != goal {
            let d = first_dir[cur][goal] as usize;
            moves.push(d as u8);
            cur = next_by_dir[cur][d] as usize;
        }
    }
    moves
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

fn build_route_by_order(
    order: &[usize],
    first_dir: &[[u8; N2]],
    next_by_dir: &[[u16; 4]],
) -> Option<RouteCandidate> {
    let moves = reconstruct_moves(order, first_dir, next_by_dir);
    let mut best_len = usize::MAX;
    let mut best_sd = 0usize;
    for sd in 0..4 {
        let mut turns = 0usize;
        let mut cur = sd;
        for &md in &moves {
            turns += rot_cost(cur, md as usize);
            cur = md as usize;
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
    for &md in &moves {
        append_turns(&mut cur, md as usize, &mut actions);
        actions.push(ACT_F);
        cur = md as usize;
    }
    append_turns(&mut cur, best_sd, &mut actions);
    if actions.is_empty() || actions.len() > M_LIMIT {
        return None;
    }
    Some(RouteCandidate {
        start_cell: order[0],
        start_dir: best_sd,
        actions,
    })
}

fn best_route(input: &Input) -> RouteCandidate {
    let g = build_graph(input);
    let (dist, first_dir, next_by_dir) = precompute_shortest(&g);
    let start_time = Instant::now();
    let limit_ms = std::env::var("SEARCH_TIME_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEARCH_TIME_MS);
    let limit = Duration::from_millis(limit_ms);
    let mut rng = XorShift64::new(seed_from_input(input) ^ 0xA24BAED4963EE407);

    let mut initials = Vec::<Vec<usize>>::new();
    let row = row_snake();
    let mut rev_row = row.clone();
    rev_row.reverse();
    let col = col_snake();
    let mut rev_col = col.clone();
    rev_col.reverse();
    initials.push(row);
    initials.push(rev_row);
    initials.push(col);
    initials.push(rev_col);
    for _ in 0..8 {
        initials.push(nearest_neighbor_tour(rng.gen_usize(N2), &dist));
    }

    let mut best_order = initials[0].clone();
    let mut best_cost = tour_cost(&best_order, &dist);

    for init in initials {
        let mut order = init;
        let mut cost = tour_cost(&order, &dist);
        if cost < best_cost {
            best_cost = cost;
            best_order = order.clone();
        }
        while start_time.elapsed() < limit {
            let improved = two_opt_once(&mut order, &dist, &mut rng);
            if improved {
                cost = tour_cost(&order, &dist);
                if cost < best_cost {
                    best_cost = cost;
                    best_order = order.clone();
                }
            } else {
                break;
            }
        }
        if start_time.elapsed() >= limit {
            break;
        }
    }

    // ランダムキック + 2-opt を時間いっぱいまで反復
    let mut work = best_order.clone();
    let mut work_cost = best_cost;
    while start_time.elapsed() < limit {
        let l = rng.gen_usize(N2 - 3);
        let max_add = (N2 - 1 - l).min(40);
        let r = l + 1 + rng.gen_usize(max_add);
        work[l..=r].reverse();
        let kicked_cost = tour_cost(&work, &dist);
        if kicked_cost > work_cost + 80 {
            work[l..=r].reverse();
            continue;
        }
        let mut updated_cost = kicked_cost;
        while start_time.elapsed() < limit {
            if !two_opt_once(&mut work, &dist, &mut rng) {
                break;
            }
            updated_cost = tour_cost(&work, &dist);
        }
        work_cost = updated_cost;
        if work_cost < best_cost {
            best_cost = work_cost;
            best_order = work.clone();
        } else {
            work = best_order.clone();
            work_cost = best_cost;
        }
    }

    // 巡回の回転位置も数点試して最終コマンド長で比較
    let mut best_route = None;
    let mut best_m = usize::MAX;
    let mut shifts = vec![0usize];
    for _ in 0..7 {
        shifts.push(rng.gen_usize(N2));
    }
    shifts.sort_unstable();
    shifts.dedup();
    for sh in shifts {
        let mut rotated = Vec::<usize>::with_capacity(N2);
        rotated.extend_from_slice(&best_order[sh..]);
        rotated.extend_from_slice(&best_order[..sh]);
        if let Some(route) = build_route_by_order(&rotated, &first_dir, &next_by_dir) {
            if route.actions.len() < best_m {
                best_m = route.actions.len();
                best_route = Some(route);
            }
        }
    }
    best_route.expect("no route")
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);
    let _ = (input.ak, input.am, input.aw);

    let route = best_route(&input);
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
