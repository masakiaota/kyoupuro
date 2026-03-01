// v102_c_wall_snake.rs
use std::collections::{HashMap, HashSet, VecDeque};
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
const DEFAULT_SEARCH_TIME_MS: u64 = 1080;
const INITIAL_NN_TOURS: usize = 14;
const EVAL_EXTRA_SHIFTS: usize = 16;
const OFFSET_TRIAL_LIMIT: usize = 128;
const TREE_ROUTE_TRIALS: usize = 120;

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
struct AutoState {
    a0: u8,
    b0: usize,
    a1: u8,
    b1: usize,
}

#[derive(Clone)]
struct RobotPlan {
    start_cell: usize,
    start_dir: usize,
    states: Vec<AutoState>,
    wall_v_add: Vec<Vec<u8>>,
    wall_h_add: Vec<Vec<u8>>,
    w_count: usize,
    value_v: i64,
}

#[derive(Clone, Copy)]
enum PatchKind {
    B1Only,
    Both,
}

#[derive(Clone, Copy)]
struct PatchRef {
    state_id: usize,
    kind: PatchKind,
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

fn or_opt_once(order: &mut Vec<usize>, dist: &[[u16; N2]], rng: &mut XorShift64) -> bool {
    let n = order.len();
    if n <= 6 {
        return false;
    }
    let len = 1 + rng.gen_usize(3).min(n - 2);
    let l = rng.gen_usize(n - len + 1);
    let mut ins = rng.gen_usize(n - len + 1);
    if ins == l {
        return false;
    }
    if ins > l && ins < l + len {
        ins = l + len;
    }

    let old = tour_cost(order, dist);
    let mut cand = order.clone();
    let seg: Vec<usize> = cand[l..l + len].to_vec();
    cand.drain(l..l + len);
    let pos = if ins > l { ins - len } else { ins };
    for (k, &x) in seg.iter().enumerate() {
        cand.insert(pos + k, x);
    }
    let newv = tour_cost(&cand, dist);
    if newv < old {
        *order = cand;
        true
    } else {
        false
    }
}

fn kick_order(order: &mut [usize], rng: &mut XorShift64) {
    let n = order.len();
    if n < 8 {
        return;
    }
    if rng.gen_usize(100) < 70 {
        let l = rng.gen_usize(n - 3);
        let max_add = (n - 1 - l).min(48);
        let r = l + 1 + rng.gen_usize(max_add);
        order[l..=r].reverse();
    } else {
        let mut idx = [
            rng.gen_usize(n),
            rng.gen_usize(n),
            rng.gen_usize(n),
            rng.gen_usize(n),
        ];
        idx.sort_unstable();
        if idx[0] + 2 < idx[1] && idx[1] + 2 < idx[2] && idx[2] + 2 < idx[3] {
            order[idx[0]..idx[1]].reverse();
            order[idx[2]..idx[3]].reverse();
        }
    }
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

fn evaluate_order_with_shifts(
    order: &[usize],
    first_dir: &[[u8; N2]],
    next_by_dir: &[[u16; 4]],
    rng: &mut XorShift64,
) -> Option<RouteCandidate> {
    let mut shifts = Vec::<usize>::new();
    shifts.push(0);
    shifts.push(N2 / 4);
    shifts.push(N2 / 2);
    shifts.push(N2 * 3 / 4);
    for _ in 0..EVAL_EXTRA_SHIFTS {
        shifts.push(rng.gen_usize(N2));
    }
    shifts.sort_unstable();
    shifts.dedup();

    let mut best_route: Option<RouteCandidate> = None;
    let mut best_m = usize::MAX;
    for &sh in &shifts {
        let mut rotated = Vec::<usize>::with_capacity(N2);
        rotated.extend_from_slice(&order[sh..]);
        rotated.extend_from_slice(&order[..sh]);
        if let Some(route) = build_route_by_order(&rotated, first_dir, next_by_dir) {
            let m = route.actions.len();
            if m < best_m {
                best_m = m;
                best_route = Some(route);
            }
        }
    }
    best_route
}

fn best_route(input: &Input) -> Option<RouteCandidate> {
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
    for _ in 0..INITIAL_NN_TOURS {
        initials.push(nearest_neighbor_tour(rng.gen_usize(N2), &dist));
    }
    let base_orders = initials.clone();

    let mut best_order = initials[0].clone();
    let mut best_cost = tour_cost(&best_order, &dist);
    let mut best_route: Option<RouteCandidate> = None;
    let mut best_m = usize::MAX;
    let mut best_route_cost = i64::MAX;

    for init in initials {
        if start_time.elapsed() >= limit {
            break;
        }
        let mut order = init;
        let mut cost = tour_cost(&order, &dist);
        if cost < best_cost {
            best_cost = cost;
            best_order = order.clone();
        }
        if let Some(route) = evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng) {
            let m = route.actions.len();
            if m < best_m || (m == best_m && cost < best_route_cost) {
                best_m = m;
                best_route_cost = cost;
                best_route = Some(route);
                best_order = order.clone();
                best_cost = cost;
            }
        }

        let mut stagnation = 0usize;
        let mut improve_cnt = 0usize;
        while start_time.elapsed() < limit && stagnation < 64 {
            let improved = if rng.gen_usize(100) < 78 {
                two_opt_once(&mut order[..], &dist, &mut rng)
            } else {
                or_opt_once(&mut order, &dist, &mut rng)
            };
            if improved {
                stagnation = 0;
                improve_cnt += 1;
                cost = tour_cost(&order, &dist);
                if cost < best_cost {
                    best_cost = cost;
                    best_order = order.clone();
                }
                if improve_cnt % 10 == 0 {
                    if let Some(route) =
                        evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng)
                    {
                        let m = route.actions.len();
                        if m < best_m || (m == best_m && cost < best_route_cost) {
                            best_m = m;
                            best_route_cost = cost;
                            best_route = Some(route);
                            best_order = order.clone();
                            best_cost = cost;
                        }
                    }
                }
            } else {
                stagnation += 1;
            }
        }
        if let Some(route) = evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng) {
            let m = route.actions.len();
            if m < best_m || (m == best_m && cost < best_route_cost) {
                best_m = m;
                best_route_cost = cost;
                best_route = Some(route);
                best_order = order.clone();
                best_cost = cost;
            }
        }
    }

    // キック + 局所探索を繰り返し、m最小を直接追う
    let mut work = best_order.clone();
    let mut work_cost = tour_cost(&work, &dist);
    while start_time.elapsed() < limit {
        kick_order(&mut work[..], &mut rng);
        let mut no_imp = 0usize;
        while start_time.elapsed() < limit && no_imp < 80 {
            let improved = if rng.gen_usize(100) < 72 {
                two_opt_once(&mut work[..], &dist, &mut rng)
            } else {
                or_opt_once(&mut work, &dist, &mut rng)
            };
            if improved {
                work_cost = tour_cost(&work, &dist);
                no_imp = 0;
            } else {
                no_imp += 1;
            }
        }

        if let Some(route) = evaluate_order_with_shifts(&work, &first_dir, &next_by_dir, &mut rng) {
            let m = route.actions.len();
            if m < best_m || (m == best_m && work_cost < best_route_cost) {
                best_m = m;
                best_route_cost = work_cost;
                best_route = Some(route);
                best_order = work.clone();
            }
        }

        if work_cost > best_route_cost + 120 {
            work = best_order.clone();
            work_cost = best_route_cost;
        }
    }

    if best_route.is_none() {
        if let Some(route) = evaluate_order_with_shifts(&best_order, &first_dir, &next_by_dir, &mut rng)
            .or_else(|| build_route_by_order(&best_order, &first_dir, &next_by_dir))
        {
            best_route = Some(route);
        }
    }
    if best_route.is_none() {
        for order in &base_orders {
            if let Some(route) = evaluate_order_with_shifts(order, &first_dir, &next_by_dir, &mut rng)
                .or_else(|| build_route_by_order(order, &first_dir, &next_by_dir))
            {
                best_route = Some(route);
                break;
            }
        }
    }
    best_route
}

fn rotate_dir(dir: usize, act: u8) -> usize {
    match act {
        ACT_R => (dir + 1) % 4,
        ACT_L => (dir + 3) % 4,
        ACT_F => dir,
        _ => unreachable!(),
    }
}

fn edge_slot_from_cell_dir(cell: usize, dir: usize, n: usize) -> Option<(bool, usize, usize)> {
    let i = cell / n;
    let j = cell % n;
    match dir {
        0 => {
            if i == 0 {
                None
            } else {
                Some((false, i - 1, j))
            }
        }
        1 => {
            if j + 1 >= n {
                None
            } else {
                Some((true, i, j))
            }
        }
        2 => {
            if i + 1 >= n {
                None
            } else {
                Some((false, i, j))
            }
        }
        3 => {
            if j == 0 {
                None
            } else {
                Some((true, i, j - 1))
            }
        }
        _ => unreachable!(),
    }
}

fn add_wall_if_new(
    wall_v_add: &mut [Vec<u8>],
    wall_h_add: &mut [Vec<u8>],
    is_v: bool,
    i: usize,
    j: usize,
) {
    if is_v {
        wall_v_add[i][j] = 1;
    } else {
        wall_h_add[i][j] = 1;
    }
}

fn has_wall_with_add(
    input: &Input,
    wall_v_add: &[Vec<u8>],
    wall_h_add: &[Vec<u8>],
    cell: usize,
    dir: usize,
) -> bool {
    let i = cell / input.n;
    let j = cell % input.n;
    if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(cell, dir, input.n) {
        if is_v {
            input.wall_v[ei][ej] == 1 || wall_v_add[ei][ej] == 1
        } else {
            input.wall_h[ei][ej] == 1 || wall_h_add[ei][ej] == 1
        }
    } else {
        let _ = (i, j);
        true
    }
}

fn apply_action(input: &Input, cell: usize, dir: usize, act: u8) -> (usize, usize) {
    match act {
        ACT_R | ACT_L => (cell, rotate_dir(dir, act)),
        ACT_F => {
            let i = cell / input.n;
            let j = cell % input.n;
            let ni = (i as isize + DIJ[dir].0) as usize;
            let nj = (j as isize + DIJ[dir].1) as usize;
            (ni * input.n + nj, dir)
        }
        _ => unreachable!(),
    }
}

fn rotate_route(route: &RouteCandidate, input: &Input, offset: usize) -> Option<RouteCandidate> {
    let m = route.actions.len();
    if m == 0 {
        return None;
    }
    let offset = offset % m;
    let mut cell = route.start_cell;
    let mut dir = route.start_dir;
    for t in 0..offset {
        let act = route.actions[t];
        if act == ACT_F && has_wall(input, cell / input.n, cell % input.n, dir) {
            return None;
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        cell = nc;
        dir = nd;
    }
    let mut actions = Vec::<u8>::with_capacity(m);
    actions.extend_from_slice(&route.actions[offset..]);
    actions.extend_from_slice(&route.actions[..offset]);
    Some(RouteCandidate {
        start_cell: cell,
        start_dir: dir,
        actions,
    })
}

fn compute_plan_value(input: &Input, m: usize, w_count: usize) -> i64 {
    input.am * m as i64 + input.aw * w_count as i64
}

fn simulate_plan_cover_all(input: &Input, plan: &RobotPlan) -> bool {
    let m = plan.states.len();
    if m == 0 || m > M_LIMIT {
        return false;
    }
    let total = N2 * 4 * m;
    let mut seen = vec![-1i32; total];
    let mut path = Vec::<(usize, usize, usize)>::new();

    let mut cell = plan.start_cell;
    let mut dir = plan.start_dir;
    let mut st = 0usize;

    let cycle_start: usize = loop {
        if st >= m {
            return false;
        }
        let idx = (cell * 4 + dir) * m + st;
        if seen[idx] >= 0 {
            break seen[idx] as usize;
        }
        seen[idx] = path.len() as i32;
        path.push((cell, dir, st));

        let s = plan.states[st];
        let wall = has_wall_with_add(input, &plan.wall_v_add, &plan.wall_h_add, cell, dir);
        let (act, ns) = if wall { (s.a1, s.b1) } else { (s.a0, s.b0) };
        if wall && act == ACT_F {
            return false;
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        cell = nc;
        dir = nd;
        st = ns;
    };

    let mut cover = [false; N2];
    for &(c, _, _) in path.iter().skip(cycle_start) {
        cover[c] = true;
    }
    cover.into_iter().all(|x| x)
}

fn build_explicit_plan(input: &Input, route: &RouteCandidate) -> Option<RobotPlan> {
    let m = route.actions.len();
    if m == 0 || m > M_LIMIT {
        return None;
    }
    let mut states = Vec::<AutoState>::with_capacity(m);
    for s in 0..m {
        let act = route.actions[s];
        let ns = (s + 1) % m;
        states.push(AutoState {
            a0: act,
            b0: ns,
            a1: if act == ACT_F { ACT_R } else { act },
            b1: ns,
        });
    }
    let wall_v_add = vec![vec![0u8; input.n - 1]; input.n];
    let wall_h_add = vec![vec![0u8; input.n]; input.n - 1];
    let plan = RobotPlan {
        start_cell: route.start_cell,
        start_dir: route.start_dir,
        states,
        wall_v_add,
        wall_h_add,
        w_count: 0,
        value_v: compute_plan_value(input, m, 0),
    };
    if simulate_plan_cover_all(input, &plan) {
        Some(plan)
    } else {
        None
    }
}

#[derive(Clone)]
struct SegmentInfo {
    run_start: usize,
    run_end: usize,
    turn_end: usize,
    run_len: usize,
    existing_wall: bool,
    add_edge: Option<(bool, usize, usize)>,
}

fn select_added_edges_smart(
    input: &Input,
    segments: &[SegmentInfo],
) -> HashSet<(bool, usize, usize)> {
    let mut gain_by_edge = HashMap::<(bool, usize, usize), i64>::new();
    for seg in segments {
        if seg.existing_wall || seg.run_len <= 1 {
            continue;
        }
        if let Some(edge) = seg.add_edge {
            *gain_by_edge.entry(edge).or_insert(0) += (seg.run_len - 1) as i64;
        }
    }

    let mut chosen = HashSet::<(bool, usize, usize)>::new();
    for (edge, save_states) in gain_by_edge {
        // 壁1本で得られる状態削減の総和が、壁コストを上回るときのみ採用する。
        if input.aw - input.am * save_states < 0 {
            chosen.insert(edge);
        }
    }
    chosen
}

fn build_segment_plan_for_offset(
    input: &Input,
    base_route: &RouteCandidate,
    offset: usize,
    allow_add_walls: bool,
) -> Option<RobotPlan> {
    let route = rotate_route(base_route, input, offset)?;
    if route.actions.is_empty() || route.actions[0] != ACT_F {
        return None;
    }
    let m = route.actions.len();

    let mut pre_cell = vec![0usize; m];
    let mut pre_dir = vec![0usize; m];
    let mut post_cell = vec![0usize; m];
    let mut post_dir = vec![0usize; m];

    let mut used_v = vec![vec![false; input.n - 1]; input.n];
    let mut used_h = vec![vec![false; input.n]; input.n - 1];

    let mut cell = route.start_cell;
    let mut dir = route.start_dir;
    for t in 0..m {
        pre_cell[t] = cell;
        pre_dir[t] = dir;
        let act = route.actions[t];
        if act == ACT_F {
            if has_wall(input, cell / input.n, cell % input.n, dir) {
                return None;
            }
            if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(cell, dir, input.n) {
                if is_v {
                    used_v[ei][ej] = true;
                } else {
                    used_h[ei][ej] = true;
                }
            }
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        post_cell[t] = nc;
        post_dir[t] = nd;
        cell = nc;
        dir = nd;
    }
    if cell != route.start_cell || dir != route.start_dir {
        return None;
    }

    let mut segments = Vec::<SegmentInfo>::new();
    let mut i = 0usize;
    while i < m {
        if route.actions[i] != ACT_F {
            return None;
        }
        let run_start = i;
        while i < m && route.actions[i] == ACT_F {
            i += 1;
        }
        let run_end = i;
        let turn_start = i;
        while i < m && route.actions[i] != ACT_F {
            i += 1;
        }
        let turn_end = i;
        if turn_start == turn_end {
            return None;
        }

        let run_len = run_end - run_start;
        let end_cell = post_cell[run_end - 1];
        let end_dir = post_dir[run_end - 1];
        let existing_wall = has_wall(input, end_cell / input.n, end_cell % input.n, end_dir);
        let mut add_edge = None;
        if !existing_wall && allow_add_walls {
            if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(end_cell, end_dir, input.n) {
                let used = if is_v { used_v[ei][ej] } else { used_h[ei][ej] };
                if !used {
                    add_edge = Some((is_v, ei, ej));
                }
            }
        }

        segments.push(SegmentInfo {
            run_start,
            run_end,
            turn_end,
            run_len,
            existing_wall,
            add_edge,
        });
    }
    if segments.is_empty() {
        return None;
    }

    let chosen_add_edges = if allow_add_walls {
        select_added_edges_smart(input, &segments)
    } else {
        HashSet::new()
    };

    let mut states = Vec::<AutoState>::new();
    let seg_n = segments.len();
    let mut seg_start_state = vec![usize::MAX; seg_n];
    let mut patches = vec![Vec::<PatchRef>::new(); seg_n];

    for (si, seg) in segments.iter().enumerate() {
        seg_start_state[si] = states.len();
        let turns = &route.actions[seg.run_end..seg.turn_end];
        if turns.is_empty() {
            return None;
        }
        let compressible = seg.existing_wall
            || seg
                .add_edge
                .map(|edge| chosen_add_edges.contains(&edge))
                .unwrap_or(false);

        if compressible {
            let move_id = states.len();
            states.push(AutoState {
                a0: ACT_F,
                b0: move_id,
                a1: turns[0],
                b1: 0,
            });
            if turns.len() == 1 {
                patches[si].push(PatchRef {
                    state_id: move_id,
                    kind: PatchKind::B1Only,
                });
            } else {
                let mut prev = move_id;
                for (idx, &act) in turns.iter().enumerate().skip(1) {
                    let sid = states.len();
                    states.push(AutoState {
                        a0: act,
                        b0: 0,
                        a1: act,
                        b1: 0,
                    });
                    if idx == 1 {
                        states[move_id].b1 = sid;
                    } else {
                        states[prev].b0 = sid;
                        states[prev].b1 = sid;
                    }
                    prev = sid;
                }
                patches[si].push(PatchRef {
                    state_id: prev,
                    kind: PatchKind::Both,
                });
            }
        } else {
            let seq = &route.actions[seg.run_start..seg.turn_end];
            let base = states.len();
            for (k, &act) in seq.iter().enumerate() {
                let next = if k + 1 < seq.len() { base + k + 1 } else { 0 };
                let a1 = if act == ACT_F { ACT_R } else { act };
                states.push(AutoState {
                    a0: act,
                    b0: next,
                    a1,
                    b1: next,
                });
            }
            patches[si].push(PatchRef {
                state_id: states.len() - 1,
                kind: PatchKind::Both,
            });
        }
    }

    if states.is_empty() || states.len() > M_LIMIT {
        return None;
    }

    for si in 0..seg_n {
        let next_start = seg_start_state[(si + 1) % seg_n];
        for p in &patches[si] {
            match p.kind {
                PatchKind::B1Only => {
                    states[p.state_id].b1 = next_start;
                }
                PatchKind::Both => {
                    states[p.state_id].b0 = next_start;
                    states[p.state_id].b1 = next_start;
                }
            }
        }
    }

    let mut wall_v_add = vec![vec![0u8; input.n - 1]; input.n];
    let mut wall_h_add = vec![vec![0u8; input.n]; input.n - 1];
    for &(is_v, ei, ej) in &chosen_add_edges {
        add_wall_if_new(&mut wall_v_add, &mut wall_h_add, is_v, ei, ej);
    }
    let w_count = chosen_add_edges.len();

    let plan = RobotPlan {
        start_cell: route.start_cell,
        start_dir: route.start_dir,
        states,
        wall_v_add,
        wall_h_add,
        w_count,
        value_v: compute_plan_value(input, m.min(M_LIMIT), w_count),
    };
    let mut checked = plan;
    checked.value_v = compute_plan_value(input, checked.states.len(), checked.w_count);
    if simulate_plan_cover_all(input, &checked) {
        Some(checked)
    } else {
        None
    }
}

fn collect_offset_candidates(actions: &[u8], seed: u64) -> Vec<usize> {
    if actions.is_empty() {
        return vec![0];
    }
    let m = actions.len();
    let mut offsets = Vec::<usize>::new();
    for i in 0..m {
        let prev = if i == 0 { actions[m - 1] } else { actions[i - 1] };
        if actions[i] == ACT_F && prev != ACT_F {
            offsets.push(i);
        }
    }
    if offsets.is_empty() {
        for i in 0..m {
            if actions[i] == ACT_F {
                offsets.push(i);
                break;
            }
        }
    }
    offsets.push(0);
    offsets.sort_unstable();
    offsets.dedup();

    if offsets.len() <= OFFSET_TRIAL_LIMIT {
        return offsets;
    }
    let mut rng = XorShift64::new(seed ^ 0x9E3779B97F4A7C15);
    for i in (1..offsets.len()).rev() {
        let j = rng.gen_usize(i + 1);
        offsets.swap(i, j);
    }
    offsets.truncate(OFFSET_TRIAL_LIMIT);
    offsets.sort_unstable();
    offsets
}

fn better_plan(a: &RobotPlan, b: &RobotPlan) -> bool {
    let key_a = (a.value_v, a.states.len(), a.w_count);
    let key_b = (b.value_v, b.states.len(), b.w_count);
    key_a < key_b
}

fn choose_best_plan(input: &Input, route: &RouteCandidate) -> Option<RobotPlan> {
    let mut best = build_explicit_plan(input, route)?;
    let offsets = collect_offset_candidates(&route.actions, seed_from_input(input));
    // C相当 (AW が極端に重い) では壁探索を省く。Bレンジでは壁探索を有効化する。
    let try_wall_mode = input.aw <= input.am * 32;

    for &off in &offsets {
        if let Some(plan) = build_segment_plan_for_offset(input, route, off, false) {
            if better_plan(&plan, &best) {
                best = plan;
            }
        }
        if try_wall_mode {
            if let Some(plan) = build_segment_plan_for_offset(input, route, off, true) {
                if better_plan(&plan, &best) {
                    best = plan;
                }
            }
        }
    }
    Some(best)
}

fn build_tree_dfs_route(
    v: usize,
    g: &[Vec<(usize, usize)>],
    degree: &[usize],
    visited: &mut [bool],
    children: &mut [Vec<(usize, usize)>],
    rng: &mut XorShift64,
) {
    visited[v] = true;
    let mut cands = Vec::<(usize, usize, usize)>::new();
    for &(d, to) in &g[v] {
        if !visited[to] {
            cands.push((degree[to], to, d));
        }
    }
    // 低次数を優先しつつ、同点は乱択で分散させる。
    cands.sort_by_key(|&(deg, to, _)| (deg, rng.gen_usize(1 << 20), to));
    for &(_, to, d) in &cands {
        if !visited[to] {
            children[v].push((d, to));
            build_tree_dfs_route(to, g, degree, visited, children, rng);
        }
    }
}

fn collect_tree_euler_moves(v: usize, children: &[Vec<(usize, usize)>], moves: &mut Vec<u8>) {
    for &(d, to) in &children[v] {
        moves.push(d as u8);
        collect_tree_euler_moves(to, children, moves);
        moves.push(((d + 2) % 4) as u8);
    }
}

fn build_tree_route_candidate(input: &Input) -> Option<RouteCandidate> {
    let g = build_graph(input);
    let degree: Vec<usize> = g.iter().map(|adj| adj.len()).collect();
    let mut rng = XorShift64::new(seed_from_input(input) ^ 0xD6E8FEB86659FD93);

    let mut best_len = usize::MAX;
    let mut best_root = 0usize;
    let mut best_sd = 0usize;
    let mut best_moves = Vec::<u8>::new();

    for t in 0..TREE_ROUTE_TRIALS {
        let root = if t < 4 {
            match t {
                0 => 0,
                1 => N_FIXED - 1,
                2 => (N_FIXED - 1) * N_FIXED,
                _ => N2 - 1,
            }
        } else {
            rng.gen_usize(N2)
        };

        let mut visited = [false; N2];
        let mut children = vec![Vec::<(usize, usize)>::new(); N2];
        build_tree_dfs_route(root, &g, &degree, &mut visited, &mut children, &mut rng);
        if visited.iter().any(|&x| !x) {
            continue;
        }

        let mut moves = Vec::<u8>::with_capacity(2 * (N2 - 1));
        collect_tree_euler_moves(root, &children, &mut moves);
        for sd in 0..4 {
            let mut cur = sd;
            let mut turns = 0usize;
            for &md in &moves {
                turns += rot_cost(cur, md as usize);
                cur = md as usize;
            }
            turns += rot_cost(cur, sd);
            let len = moves.len() + turns;
            if len < best_len {
                best_len = len;
                best_root = root;
                best_sd = sd;
                best_moves = moves.clone();
            }
        }
    }

    if best_len == usize::MAX || best_len > M_LIMIT {
        return None;
    }

    let mut actions = Vec::<u8>::with_capacity(best_len);
    let mut cur = best_sd;
    for &md in &best_moves {
        append_turns(&mut cur, md as usize, &mut actions);
        actions.push(ACT_F);
        cur = md as usize;
    }
    append_turns(&mut cur, best_sd, &mut actions);
    if actions.is_empty() || actions.len() > M_LIMIT {
        return None;
    }

    Some(RouteCandidate {
        start_cell: best_root,
        start_dir: best_sd,
        actions,
    })
}

fn print_single_robot_plan(input: &Input, plan: &RobotPlan) {
    println!("1");
    println!(
        "{} {} {} {}",
        plan.states.len(),
        plan.start_cell / input.n,
        plan.start_cell % input.n,
        DIR_CHARS[plan.start_dir]
    );
    for s in 0..plan.states.len() {
        let st = plan.states[s];
        println!(
            "{} {} {} {}",
            act_char(st.a0),
            st.b0,
            act_char(st.a1),
            st.b1
        );
    }
    for i in 0..input.n {
        let mut line = String::with_capacity(input.n - 1);
        for j in 0..input.n - 1 {
            line.push(if plan.wall_v_add[i][j] == 1 { '1' } else { '0' });
        }
        println!("{}", line);
    }
    for i in 0..input.n - 1 {
        let mut line = String::with_capacity(input.n);
        for j in 0..input.n {
            line.push(if plan.wall_h_add[i][j] == 1 { '1' } else { '0' });
        }
        println!("{}", line);
    }
}

fn print_fallback_all_stationary(input: &Input) {
    println!("{}", N2);
    for cell in 0..N2 {
        println!("1 {} {} U", cell / input.n, cell % input.n);
        println!("R 0 R 0");
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

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);

    let mut best_plan: Option<RobotPlan> = None;

    if let Some(route) = best_route(&input) {
        if let Some(plan) = choose_best_plan(&input, &route) {
            best_plan = Some(plan);
        }
    }
    if let Some(tree_route) = build_tree_route_candidate(&input) {
        if let Some(plan) = choose_best_plan(&input, &tree_route) {
            if best_plan
                .as_ref()
                .map(|cur| better_plan(&plan, cur))
                .unwrap_or(true)
            {
                best_plan = Some(plan);
            }
        }
    }

    if let Some(plan) = best_plan {
        print_single_robot_plan(&input, &plan);
    } else {
        print_fallback_all_stationary(&input);
    }
}
