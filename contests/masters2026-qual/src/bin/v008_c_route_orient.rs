// v008_c_route_orient.rs
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
const O2: usize = N2 * 4;
const M_LIMIT: usize = 4 * N_FIXED * N_FIXED; // 1600
const DEFAULT_SEARCH_TIME_MS: u64 = 1050;
const INITIAL_NN_TOURS: usize = 14;
const INITIAL_M_EVAL_INTERVAL: usize = 8;
const KICK_M_EVAL_INTERVAL: usize = 2;
const M_GUIDED_TRIES_PER_KICK: usize = 3;

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

fn precompute_shortest(g: &[Vec<(usize, usize)>]) -> Vec<[u16; N2]> {
    let mut dist = vec![[u16::MAX; N2]; N2];
    let mut q = VecDeque::<usize>::new();
    for s in 0..N2 {
        q.clear();
        dist[s][s] = 0;
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            let dv = dist[s][v];
            for &(_, to) in &g[v] {
                if dist[s][to] == u16::MAX {
                    dist[s][to] = dv + 1;
                    q.push_back(to);
                }
            }
        }
    }
    dist
}

fn build_oriented_next(input: &Input) -> Vec<[u16; 3]> {
    let mut next_o = vec![[0u16; 3]; O2];
    for i in 0..input.n {
        for j in 0..input.n {
            let cell = i * input.n + j;
            for d in 0..4 {
                let o = cell * 4 + d;
                next_o[o][ACT_R as usize] = (cell * 4 + (d + 1) % 4) as u16;
                next_o[o][ACT_L as usize] = (cell * 4 + (d + 3) % 4) as u16;
                if has_wall(input, i, j, d) {
                    next_o[o][ACT_F as usize] = o as u16;
                } else {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    let ncell = ni * input.n + nj;
                    next_o[o][ACT_F as usize] = (ncell * 4 + d) as u16;
                }
            }
        }
    }
    next_o
}

fn precompute_oriented_shortest(next_o: &[[u16; 3]]) -> Vec<[u16; O2]> {
    let mut dist = vec![[u16::MAX; O2]; O2];
    let mut q = VecDeque::<u16>::new();
    for s in 0..O2 {
        q.clear();
        dist[s][s] = 0;
        q.push_back(s as u16);
        while let Some(vu) = q.pop_front() {
            let v = vu as usize;
            let dv = dist[s][v];
            for a in 0..3 {
                let to = next_o[v][a] as usize;
                if dist[s][to] == u16::MAX {
                    dist[s][to] = dv + 1;
                    q.push_back(to as u16);
                }
            }
        }
    }
    dist
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

fn oriented_cycle_cost(order: &[usize], odist: &[[u16; O2]]) -> Option<usize> {
    let n = order.len();
    const INF: usize = usize::MAX / 4;
    let mut best = INF;

    for sd in 0..4 {
        let mut dp = [INF; 4];
        dp[sd] = 0;
        for i in 0..n {
            let u = order[i];
            let v = order[(i + 1) % n];
            let mut ndp = [INF; 4];
            for du in 0..4 {
                if dp[du] >= INF {
                    continue;
                }
                let src = u * 4 + du;
                for dv in 0..4 {
                    let dst = v * 4 + dv;
                    let c = odist[src][dst];
                    if c == u16::MAX {
                        continue;
                    }
                    let cand = dp[du] + c as usize;
                    if cand < ndp[dv] {
                        ndp[dv] = cand;
                    }
                }
            }
            dp = ndp;
        }
        if dp[sd] < best {
            best = dp[sd];
        }
    }

    if best == INF {
        None
    } else {
        Some(best)
    }
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

fn random_mutation(order: &mut Vec<usize>, rng: &mut XorShift64) {
    let n = order.len();
    if n < 8 {
        return;
    }
    if rng.gen_usize(100) < 60 {
        let l = rng.gen_usize(n - 3);
        let max_add = (n - 1 - l).min(32);
        let r = l + 1 + rng.gen_usize(max_add);
        order[l..=r].reverse();
    } else {
        let len = 1 + rng.gen_usize(4).min(n - 2);
        let l = rng.gen_usize(n - len + 1);
        let mut ins = rng.gen_usize(n - len + 1);
        if ins > l && ins < l + len {
            ins = l + len;
        }
        if ins == l {
            return;
        }
        let seg: Vec<usize> = order[l..l + len].to_vec();
        order.drain(l..l + len);
        let pos = if ins > l { ins - len } else { ins };
        for (k, &x) in seg.iter().enumerate() {
            order.insert(pos + k, x);
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

fn build_route_by_order(order: &[usize], next_o: &[[u16; 3]], odist: &[[u16; O2]]) -> Option<RouteCandidate> {
    let n = order.len();
    const INF: usize = usize::MAX / 4;

    let mut best_len = INF;
    let mut best_oris = vec![0usize; n];

    for sd in 0..4 {
        let mut dp = [INF; 4];
        dp[sd] = 0;
        let mut parents = vec![[255u8; 4]; n];

        for i in 0..n {
            let u = order[i];
            let v = order[(i + 1) % n];
            let mut ndp = [INF; 4];
            let mut par = [255u8; 4];
            for du in 0..4 {
                if dp[du] >= INF {
                    continue;
                }
                let src = u * 4 + du;
                for dv in 0..4 {
                    let dst = v * 4 + dv;
                    let c = odist[src][dst];
                    if c == u16::MAX {
                        continue;
                    }
                    let cand = dp[du] + c as usize;
                    if cand < ndp[dv] {
                        ndp[dv] = cand;
                        par[dv] = du as u8;
                    }
                }
            }
            parents[i] = par;
            dp = ndp;
        }

        let len = dp[sd];
        if len >= best_len || len > M_LIMIT {
            continue;
        }

        let mut oris = vec![0usize; n];
        let mut cur = sd;
        let mut valid = true;
        for i in (0..n).rev() {
            let p = parents[i][cur];
            if p == 255 {
                valid = false;
                break;
            }
            oris[i] = p as usize;
            cur = p as usize;
        }
        if !valid || cur != sd {
            continue;
        }

        best_len = len;
        best_oris = oris;
    }

    if best_len == INF || best_len > M_LIMIT {
        return None;
    }

    let mut actions = Vec::<u8>::with_capacity(best_len);
    for i in 0..n {
        let u = order[i];
        let v = order[(i + 1) % n];
        let mut cur = u * 4 + best_oris[i];
        let goal = v * 4 + best_oris[(i + 1) % n];
        while cur != goal {
            let dcur = odist[cur][goal];
            if dcur == u16::MAX {
                return None;
            }
            let mut moved = false;
            for &a in &[ACT_R, ACT_L, ACT_F] {
                let nxt = next_o[cur][a as usize] as usize;
                let dnxt = odist[nxt][goal];
                if dnxt != u16::MAX && dnxt + 1 == dcur {
                    actions.push(a);
                    cur = nxt;
                    moved = true;
                    break;
                }
            }
            if !moved || actions.len() > M_LIMIT {
                return None;
            }
        }
    }

    if actions.is_empty() || actions.len() > M_LIMIT {
        return None;
    }

    Some(RouteCandidate {
        start_cell: order[0],
        start_dir: best_oris[0],
        actions,
    })
}

fn try_update_best(
    order: &[usize],
    dist: &[[u16; N2]],
    odist: &[[u16; O2]],
    next_o: &[[u16; 3]],
    best_route: &mut Option<RouteCandidate>,
    best_m: &mut usize,
    best_tour_cost: &mut i64,
    best_order: &mut Vec<usize>,
) {
    let Some(m) = oriented_cycle_cost(order, odist) else {
        return;
    };
    if m > M_LIMIT {
        return;
    }
    let tc = tour_cost(order, dist);
    if m < *best_m || (m == *best_m && tc < *best_tour_cost) {
        if let Some(route) = build_route_by_order(order, next_o, odist) {
            *best_m = route.actions.len();
            *best_tour_cost = tc;
            *best_order = order.to_vec();
            *best_route = Some(route);
        }
    }
}

fn best_route(input: &Input) -> Option<RouteCandidate> {
    let g = build_graph(input);
    let dist = precompute_shortest(&g);
    let next_o = build_oriented_next(input);
    let odist = precompute_oriented_shortest(&next_o);

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
    let mut best_route: Option<RouteCandidate> = None;
    let mut best_m = usize::MAX;
    let mut best_tour_cost = i64::MAX;

    for init in initials {
        if start_time.elapsed() >= limit {
            break;
        }
        let mut order = init;
        try_update_best(
            &order,
            &dist,
            &odist,
            &next_o,
            &mut best_route,
            &mut best_m,
            &mut best_tour_cost,
            &mut best_order,
        );

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
                if improve_cnt % INITIAL_M_EVAL_INTERVAL == 0 {
                    try_update_best(
                        &order,
                        &dist,
                        &odist,
                        &next_o,
                        &mut best_route,
                        &mut best_m,
                        &mut best_tour_cost,
                        &mut best_order,
                    );
                }
            } else {
                stagnation += 1;
            }
        }
        try_update_best(
            &order,
            &dist,
            &odist,
            &next_o,
            &mut best_route,
            &mut best_m,
            &mut best_tour_cost,
            &mut best_order,
        );
    }

    // キック + 局所探索を繰り返し、m最小を直接追う
    let mut work = best_order.clone();
    let mut work_cost = tour_cost(&work, &dist);
    let mut work_m = oriented_cycle_cost(&work, &odist).unwrap_or(usize::MAX);
    let mut kick_iter = 0usize;
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
                work_m = oriented_cycle_cost(&work, &odist).unwrap_or(usize::MAX);
            } else {
                no_imp += 1;
            }
        }

        for _ in 0..M_GUIDED_TRIES_PER_KICK {
            if start_time.elapsed() >= limit {
                break;
            }
            let mut cand = work.clone();
            random_mutation(&mut cand, &mut rng);
            let cand_m = oriented_cycle_cost(&cand, &odist).unwrap_or(usize::MAX);
            let cand_cost = tour_cost(&cand, &dist);
            if cand_m < work_m || (cand_m == work_m && cand_cost < work_cost) {
                work = cand;
                work_m = cand_m;
                work_cost = cand_cost;
            }
        }

        kick_iter += 1;
        if kick_iter % KICK_M_EVAL_INTERVAL == 0 {
            try_update_best(
                &work,
                &dist,
                &odist,
                &next_o,
                &mut best_route,
                &mut best_m,
                &mut best_tour_cost,
                &mut best_order,
            );
        }

        if work_cost > best_tour_cost + 120 {
            work = best_order.clone();
            work_cost = best_tour_cost;
            work_m = best_m;
        }
    }

    if best_route.is_none() {
        if let Some(route) = build_route_by_order(&best_order, &next_o, &odist) {
            best_route = Some(route);
        }
    }
    if best_route.is_none() {
        for order in &base_orders {
            if let Some(route) = build_route_by_order(order, &next_o, &odist) {
                best_route = Some(route);
                break;
            }
        }
    }
    best_route
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);
    let _ = (input.ak, input.am, input.aw);

    if let Some(route) = best_route(&input) {
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
    } else {
        println!("{}", N2);
        for cell in 0..N2 {
            println!("1 {} {} U", cell / input.n, cell % input.n);
            println!("R 0 R 0");
        }
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
