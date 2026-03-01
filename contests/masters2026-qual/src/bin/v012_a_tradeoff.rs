// v012_a_tradeoff.rs
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::time::{Duration, Instant};

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DIR_CHARS: [char; 4] = ['U', 'R', 'D', 'L'];

const BIT_WORDS: usize = 7; // 7 * 64 = 448 >= 400
const N_FIXED: usize = 20;
const CELLS: usize = N_FIXED * N_FIXED;
const DEFAULT_SEARCH_TIME_MS: u64 = 1000;
const DEFAULT_BASE_RANDOM_TRIALS: usize = 2800;
const DEFAULT_MAX_RANDOM_M: usize = 4;
const DEFAULT_FORCED_TOP: usize = 56;
const FULL_ROUTE_TRIALS: usize = 100;
const DEFAULT_ENABLE_TEAM_TEMPLATES: bool = true;
const DEFAULT_ENABLE_TOGGLE_HILL: bool = false;

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
struct Env {
    n: usize,
    wall: Vec<bool>,                     // oriented-state -> wall in front
    next_o: Vec<[usize; 3]>,             // oriented-state -> next oriented-state by action (R/L/F)
    neighbors: Vec<Vec<(usize, usize)>>, // cell -> [(dir, to_cell)]
    degree: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BitSet {
    w: [u64; BIT_WORDS],
}

impl BitSet {
    fn empty() -> Self {
        Self { w: [0; BIT_WORDS] }
    }
    fn all_400() -> Self {
        let mut w = [u64::MAX; BIT_WORDS];
        w[BIT_WORDS - 1] = (1u64 << 16) - 1;
        Self { w }
    }
    fn set_cell(&mut self, cell: usize) {
        let wi = cell >> 6;
        let bi = cell & 63;
        self.w[wi] |= 1u64 << bi;
    }
    fn or_assign(&mut self, other: &Self) {
        for k in 0..BIT_WORDS {
            self.w[k] |= other.w[k];
        }
    }
    fn count_total(&self) -> u32 {
        self.w.iter().map(|x| x.count_ones()).sum()
    }
    fn count_new(&self, covered: &Self) -> u32 {
        let mut s = 0u32;
        for k in 0..BIT_WORDS {
            s += (self.w[k] & !covered.w[k]).count_ones();
        }
        s
    }
}

#[derive(Clone)]
struct RobotSpec {
    m: usize,
    start_cell: usize,
    start_dir: usize,
    a0: Vec<u8>,
    b0: Vec<usize>,
    a1: Vec<u8>,
    b1: Vec<usize>,
    cover: BitSet,
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
        let line = it.next().unwrap().as_bytes().to_vec();
        for (j, v) in row.iter_mut().enumerate().take(n - 1) {
            *v = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    let mut wall_h = vec![vec![0u8; n]; n - 1];
    for row in wall_h.iter_mut().take(n - 1) {
        let line = it.next().unwrap().as_bytes().to_vec();
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

fn build_env(input: &Input) -> Env {
    let n = input.n;
    let cells = n * n;
    let orients = cells * 4;
    let mut wall = vec![false; orients];
    let mut next_o = vec![[0usize; 3]; orients];
    let mut neighbors = vec![Vec::<(usize, usize)>::new(); cells];
    for i in 0..n {
        for j in 0..n {
            let cell = i * n + j;
            for d in 0..4 {
                let o = cell * 4 + d;
                let w = has_wall(input, i, j, d);
                wall[o] = w;
                next_o[o][ACT_R as usize] = cell * 4 + (d + 1) % 4;
                next_o[o][ACT_L as usize] = cell * 4 + (d + 3) % 4;
                if w {
                    next_o[o][ACT_F as usize] = o;
                } else {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    let ncell = ni * n + nj;
                    next_o[o][ACT_F as usize] = ncell * 4 + d;
                    neighbors[cell].push((d, ncell));
                }
            }
        }
    }
    let degree = neighbors.iter().map(|v| v.len()).collect();
    Env {
        n,
        wall,
        next_o,
        neighbors,
        degree,
    }
}

fn add_candidate(best: &mut HashMap<BitSet, RobotSpec>, cand: RobotSpec) {
    if cand.m == 0 || cand.m > 4 * N_FIXED * N_FIXED {
        return;
    }
    match best.get(&cand.cover) {
        Some(old) if old.m <= cand.m => {}
        _ => {
            best.insert(cand.cover, cand);
        }
    }
}

fn collect_from_automaton(
    env: &Env,
    m: usize,
    a0: &[u8],
    b0: &[usize],
    a1: &[u8],
    b1: &[usize],
    best: &mut HashMap<BitSet, RobotSpec>,
) {
    let orients = env.n * env.n * 4;
    let s_total = orients * m;
    let mut next = vec![0usize; s_total];
    for o in 0..orients {
        let wall = env.wall[o];
        for s in 0..m {
            let act = if wall { a1[s] } else { a0[s] };
            let ns = if wall { b1[s] } else { b0[s] };
            let no = env.next_o[o][act as usize];
            next[o * m + s] = no * m + ns;
        }
    }

    let mut mark = vec![0u8; s_total];
    let mut comp = vec![usize::MAX; s_total];
    let mut cycles = Vec::<BitSet>::new();
    let mut stack = Vec::<usize>::new();

    for st in 0..s_total {
        if mark[st] != 0 {
            continue;
        }
        stack.clear();
        let mut u = st;
        while mark[u] == 0 {
            mark[u] = 1;
            stack.push(u);
            u = next[u];
        }

        let cid = if mark[u] == 1 {
            let pos = stack.iter().rposition(|&x| x == u).unwrap();
            let mut bits = BitSet::empty();
            for &x in &stack[pos..] {
                let o = x / m;
                bits.set_cell(o / 4);
            }
            let cid = cycles.len();
            cycles.push(bits);
            for &x in &stack[pos..] {
                comp[x] = cid;
            }
            cid
        } else {
            comp[u]
        };

        for &x in &stack {
            if comp[x] == usize::MAX {
                comp[x] = cid;
            }
            mark[x] = 2;
        }
    }

    let mut local = HashMap::<BitSet, usize>::new();
    for o in 0..orients {
        let st0 = o * m;
        let bits = cycles[comp[st0]];
        local.entry(bits).or_insert(o);
    }

    for (cover, &start_o) in &local {
        let cell = start_o / 4;
        let dir = start_o % 4;
        let cand = RobotSpec {
            m,
            start_cell: cell,
            start_dir: dir,
            a0: a0.to_vec(),
            b0: b0.to_vec(),
            a1: a1.to_vec(),
            b1: b1.to_vec(),
            cover: *cover,
        };
        add_candidate(best, cand);
    }
}

fn add_stationary_candidates(input: &Input, best: &mut HashMap<BitSet, RobotSpec>) {
    let cells = input.n * input.n;
    for cell in 0..cells {
        let mut cover = BitSet::empty();
        cover.set_cell(cell);
        let cand = RobotSpec {
            m: 1,
            start_cell: cell,
            start_dir: 0,
            a0: vec![ACT_R],
            b0: vec![0],
            a1: vec![ACT_R],
            b1: vec![0],
            cover,
        };
        add_candidate(best, cand);
    }
}

fn add_m1_m2_candidates(env: &Env, best: &mut HashMap<BitSet, RobotSpec>) {
    for a0 in 0..3u8 {
        for a1 in 0..2u8 {
            let a0v = vec![a0];
            let b0v = vec![0usize];
            let a1v = vec![a1];
            let b1v = vec![0usize];
            collect_from_automaton(env, 1, &a0v, &b0v, &a1v, &b1v, best);
        }
    }

    for a00 in 0..3u8 {
        for a01 in 0..3u8 {
            for b00 in 0..2usize {
                for b01 in 0..2usize {
                    for a10 in 0..2u8 {
                        for a11 in 0..2u8 {
                            for b10 in 0..2usize {
                                for b11 in 0..2usize {
                                    let a0v = vec![a00, a01];
                                    let b0v = vec![b00, b01];
                                    let a1v = vec![a10, a11];
                                    let b1v = vec![b10, b11];
                                    collect_from_automaton(env, 2, &a0v, &b0v, &a1v, &b1v, best);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mirror_lr_actions(src: &[u8]) -> Vec<u8> {
    src.iter()
        .map(|&a| match a {
            ACT_L => ACT_R,
            ACT_R => ACT_L,
            _ => a,
        })
        .collect()
}

fn add_teammate_template_candidates(env: &Env, best: &mut HashMap<BitSet, RobotSpec>) {
    let push = |m: usize,
                a0: Vec<u8>,
                b0: Vec<usize>,
                a1: Vec<u8>,
                b1: Vec<usize>,
                best: &mut HashMap<BitSet, RobotSpec>| {
        collect_from_automaton(env, m, &a0, &b0, &a1, &b1, best);
    };

    // down_snake
    let down_a0 = vec![
        ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_L,
    ];
    let down_b0 = vec![0, 2, 3, 3, 5, 0, 6, 8, 9, 9, 11, 6];
    let down_a1 = vec![
        ACT_L, ACT_L, ACT_L, ACT_R, ACT_R, ACT_R, ACT_R, ACT_R, ACT_R, ACT_L, ACT_L, ACT_L,
    ];
    let down_b1 = vec![1, 9, 3, 4, 6, 0, 7, 3, 9, 10, 0, 6];
    push(
        12,
        down_a0.clone(),
        down_b0.clone(),
        down_a1.clone(),
        down_b1.clone(),
        best,
    );
    push(
        12,
        mirror_lr_actions(&down_a0),
        down_b0.clone(),
        mirror_lr_actions(&down_a1),
        down_b1.clone(),
        best,
    );

    // right_snake
    let right_a0 = vec![
        ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_R,
    ];
    let right_b0 = vec![0, 2, 3, 3, 5, 0, 6, 8, 9, 9, 11, 6];
    let right_a1 = vec![
        ACT_R, ACT_L, ACT_R, ACT_L, ACT_R, ACT_L, ACT_L, ACT_R, ACT_L, ACT_R, ACT_L, ACT_R,
    ];
    let right_b1 = vec![1, 6, 3, 4, 9, 0, 7, 0, 9, 10, 3, 6];
    push(
        12,
        right_a0.clone(),
        right_b0.clone(),
        right_a1.clone(),
        right_b1.clone(),
        best,
    );
    push(
        12,
        mirror_lr_actions(&right_a0),
        right_b0.clone(),
        mirror_lr_actions(&right_a1),
        right_b1.clone(),
        best,
    );

    // vertical/horizontal only
    let axis_a0 = vec![ACT_F, ACT_R];
    let axis_b0 = vec![0, 0];
    let axis_a1 = vec![ACT_R, ACT_R];
    let axis_b1 = vec![1, 0];
    push(
        2,
        axis_a0.clone(),
        axis_b0.clone(),
        axis_a1.clone(),
        axis_b1.clone(),
        best,
    );

    // right-hand rule
    push(
        4,
        vec![ACT_R, ACT_F, ACT_F, ACT_F],
        vec![1, 0, 0, 0],
        vec![ACT_R, ACT_L, ACT_L, ACT_L],
        vec![1, 2, 3, 3],
        best,
    );

    // alternating bounce
    push(
        4,
        vec![ACT_F, ACT_F, ACT_F, ACT_F],
        vec![1, 0, 3, 2],
        vec![ACT_R, ACT_L, ACT_R, ACT_L],
        vec![2, 3, 0, 1],
        best,
    );

    // down-right-up-left loop
    push(
        6,
        vec![ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_L],
        vec![0, 2, 3, 3, 5, 0],
        vec![ACT_L, ACT_L, ACT_L, ACT_L, ACT_L, ACT_L],
        vec![1, 3, 3, 4, 0, 0],
        best,
    );

    // user_m3_u / user_m2_u
    push(
        3,
        vec![ACT_F, ACT_L, ACT_R],
        vec![1, 2, 0],
        vec![ACT_R, ACT_R, ACT_R],
        vec![0, 0, 2],
        best,
    );
    push(
        2,
        vec![ACT_L, ACT_F],
        vec![1, 0],
        vec![ACT_R, ACT_L],
        vec![1, 1],
        best,
    );

    // extra_automata
    let extras = vec![
        (
            2,
            vec![ACT_F, ACT_F],
            vec![0, 1],
            vec![ACT_R, ACT_L],
            vec![1, 0],
        ),
        (
            2,
            vec![ACT_F, ACT_R],
            vec![0, 0],
            vec![ACT_L, ACT_R],
            vec![1, 0],
        ),
        (
            2,
            vec![ACT_F, ACT_L],
            vec![0, 0],
            vec![ACT_R, ACT_L],
            vec![1, 0],
        ),
        (
            3,
            vec![ACT_F, ACT_R, ACT_F],
            vec![1, 2, 0],
            vec![ACT_R, ACT_L, ACT_L],
            vec![2, 0, 1],
        ),
        (
            3,
            vec![ACT_F, ACT_L, ACT_F],
            vec![1, 2, 0],
            vec![ACT_L, ACT_R, ACT_R],
            vec![2, 0, 1],
        ),
        (
            4,
            vec![ACT_F, ACT_F, ACT_R, ACT_L],
            vec![1, 0, 3, 2],
            vec![ACT_R, ACT_R, ACT_L, ACT_L],
            vec![2, 3, 0, 1],
        ),
        (
            4,
            vec![ACT_F, ACT_F, ACT_L, ACT_R],
            vec![1, 0, 3, 2],
            vec![ACT_L, ACT_L, ACT_R, ACT_R],
            vec![2, 3, 0, 1],
        ),
        (
            3,
            vec![ACT_F, ACT_F, ACT_F],
            vec![0, 2, 1],
            vec![ACT_R, ACT_R, ACT_L],
            vec![1, 2, 0],
        ),
        (
            3,
            vec![ACT_F, ACT_L, ACT_R],
            vec![1, 2, 0],
            vec![ACT_R, ACT_L, ACT_R],
            vec![2, 0, 1],
        ),
    ];
    for (m, a0, b0, a1, b1) in extras {
        push(m, a0, b0, a1, b1, best);
    }
}

fn add_sampled_candidates_random_m(
    env: &Env,
    best: &mut HashMap<BitSet, RobotSpec>,
    rng: &mut XorShift64,
    trials: usize,
    max_m: usize,
) {
    let max_m = max_m.max(3);
    for _ in 0..trials {
        let m = 3 + rng.gen_usize(max_m - 2);
        let mut a0v = vec![0u8; m];
        let mut b0v = vec![0usize; m];
        let mut a1v = vec![0u8; m];
        let mut b1v = vec![0usize; m];
        for s in 0..m {
            a0v[s] = rng.gen_usize(3) as u8;
            b0v[s] = rng.gen_usize(m);
            a1v[s] = rng.gen_usize(2) as u8;
            b1v[s] = rng.gen_usize(m);
        }
        collect_from_automaton(env, m, &a0v, &b0v, &a1v, &b1v, best);
    }
}

fn add_sampled_candidates_random_m_timed(
    env: &Env,
    best: &mut HashMap<BitSet, RobotSpec>,
    rng: &mut XorShift64,
    deadline: Instant,
    max_m: usize,
) {
    let max_m = max_m.max(3);
    while Instant::now() < deadline {
        let m = 3 + rng.gen_usize(max_m - 2);
        let mut a0v = vec![0u8; m];
        let mut b0v = vec![0usize; m];
        let mut a1v = vec![0u8; m];
        let mut b1v = vec![0usize; m];
        for s in 0..m {
            a0v[s] = rng.gen_usize(3) as u8;
            b0v[s] = rng.gen_usize(m);
            a1v[s] = rng.gen_usize(2) as u8;
            b1v[s] = rng.gen_usize(m);
        }
        collect_from_automaton(env, m, &a0v, &b0v, &a1v, &b1v, best);
    }
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

fn build_tree_dfs(
    v: usize,
    env: &Env,
    visited: &mut [bool],
    children: &mut [Vec<(usize, usize)>],
    rng: &mut XorShift64,
) {
    visited[v] = true;
    let mut cands = Vec::<(usize, usize, u64)>::new();
    for &(d, to) in &env.neighbors[v] {
        if !visited[to] {
            cands.push((d, to, rng.next_u64()));
        }
    }
    cands.sort_by_key(|&(_, to, r)| (env.degree[to], r));
    for (d, to, _) in cands {
        if !visited[to] {
            children[v].push((d, to));
            build_tree_dfs(to, env, visited, children, rng);
        }
    }
}

fn collect_euler_moves(v: usize, children: &[Vec<(usize, usize)>], moves: &mut Vec<usize>) {
    for &(d, to) in &children[v] {
        moves.push(d);
        collect_euler_moves(to, children, moves);
        moves.push((d + 2) % 4);
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

fn build_full_route_candidate(input: &Input, env: &Env, trials: usize) -> Option<RobotSpec> {
    let n = input.n;
    let cells = n * n;
    let mut rng = XorShift64::new(seed_from_input(input));

    let mut best_len = usize::MAX;
    let mut best_root = 0usize;
    let mut best_start_dir = 0usize;
    let mut best_moves = Vec::<usize>::new();

    for t in 0..trials {
        let root = if t < 4 {
            match t {
                0 => 0,
                1 => n - 1,
                2 => (n - 1) * n,
                _ => n * n - 1,
            }
        } else {
            rng.gen_usize(cells)
        };

        let mut visited = vec![false; cells];
        let mut children = vec![Vec::<(usize, usize)>::new(); cells];
        build_tree_dfs(root, env, &mut visited, &mut children, &mut rng);
        if visited.iter().any(|&x| !x) {
            continue;
        }

        let mut moves = Vec::<usize>::with_capacity(2 * (cells - 1));
        collect_euler_moves(root, &children, &mut moves);

        for sd in 0..4 {
            let mut cur = sd;
            let mut turns = 0usize;
            for &md in &moves {
                turns += rot_cost(cur, md);
                cur = md;
            }
            turns += rot_cost(cur, sd);
            let len = moves.len() + turns;
            if len < best_len {
                best_len = len;
                best_root = root;
                best_start_dir = sd;
                best_moves = moves.clone();
            }
        }
    }

    if best_len == usize::MAX {
        return None;
    }

    let mut actions = Vec::<u8>::with_capacity(best_len);
    let mut cur_dir = best_start_dir;
    for &md in &best_moves {
        append_turns(&mut cur_dir, md, &mut actions);
        actions.push(ACT_F);
        cur_dir = md;
    }
    append_turns(&mut cur_dir, best_start_dir, &mut actions);
    if actions.len() > 4 * N_FIXED * N_FIXED {
        return None;
    }

    let m = actions.len();
    let mut a0 = vec![ACT_R; m];
    let mut b0 = vec![0usize; m];
    let mut a1 = vec![ACT_R; m];
    let mut b1 = vec![0usize; m];

    let mut cover = BitSet::empty();
    let mut cell = best_root;
    let mut dir = best_start_dir;
    for s in 0..m {
        cover.set_cell(cell);
        let o = cell * 4 + dir;
        let wall = env.wall[o];
        let act = actions[s];
        let ns = (s + 1) % m;

        if wall {
            a1[s] = if act == ACT_F { ACT_R } else { act };
            b1[s] = ns;
            a0[s] = act;
            b0[s] = ns;
        } else {
            a0[s] = act;
            b0[s] = ns;
            a1[s] = if act == ACT_F { ACT_R } else { act };
            b1[s] = ns;
        }

        let no = env.next_o[o][act as usize];
        cell = no / 4;
        dir = no % 4;
    }

    let cand = RobotSpec {
        m,
        start_cell: best_root,
        start_dir: best_start_dir,
        a0,
        b0,
        a1,
        b1,
        cover,
    };
    Some(cand)
}

fn eval_cost(input: &Input, selected: &[usize], cands: &[RobotSpec]) -> (i64, usize, usize) {
    let k = selected.len();
    let m_sum: usize = selected.iter().map(|&idx| cands[idx].m).sum();
    let v = input.ak * (k as i64 - 1) + input.am * m_sum as i64;
    (v, k, m_sum)
}

fn delta_v(
    input: &Input,
    old_k: usize,
    old_m: usize,
    old_w: usize,
    new_k: usize,
    new_m: usize,
    new_w: usize,
) -> i64 {
    input.ak * (new_k as i64 - old_k as i64)
        + input.am * (new_m as i64 - old_m as i64)
        + input.aw * (new_w as i64 - old_w as i64)
}

fn prune_redundant(selected: &mut Vec<usize>, cands: &[RobotSpec], all: BitSet) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0usize;
        while i < selected.len() {
            let mut cov = BitSet::empty();
            for (j, &idx) in selected.iter().enumerate() {
                if i == j {
                    continue;
                }
                cov.or_assign(&cands[idx].cover);
            }
            if cov == all {
                selected.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
    }
}

fn greedy_cover_random(
    cands: &[RobotSpec],
    weights: &[i64],
    all: BitSet,
    forced: Option<usize>,
    rng: &mut XorShift64,
    jitter_ppm: u32,
) -> Option<Vec<usize>> {
    let mut covered = BitSet::empty();
    let mut selected = Vec::<usize>::new();

    if let Some(idx) = forced {
        selected.push(idx);
        covered.or_assign(&cands[idx].cover);
    }

    while covered != all {
        let mut best_idx: Option<usize> = None;
        let mut best_gain = 0u32;
        let mut best_w = 1i64;
        for i in 0..cands.len() {
            let gain = cands[i].cover.count_new(&covered);
            if gain == 0 {
                continue;
            }
            let noise = if jitter_ppm == 0 {
                1_000_000i64
            } else {
                1_000_000i64 + rng.gen_usize(jitter_ppm as usize + 1) as i64
            };
            let w = weights[i] * noise;
            let better = match best_idx {
                None => true,
                Some(_) => {
                    let left = gain as i128 * best_w as i128;
                    let right = best_gain as i128 * w as i128;
                    if left != right {
                        left > right
                    } else if w != best_w {
                        w < best_w
                    } else {
                        gain > best_gain
                    }
                }
            };
            if better {
                best_idx = Some(i);
                best_gain = gain;
                best_w = w;
            }
        }
        let idx = best_idx?;
        selected.push(idx);
        covered.or_assign(&cands[idx].cover);
    }

    prune_redundant(&mut selected, cands, all);
    Some(selected)
}

fn build_cover_cells(cands: &[RobotSpec]) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::<usize>::new(); cands.len()];
    for (idx, c) in cands.iter().enumerate() {
        let mut cells = Vec::<usize>::new();
        for cell in 0..CELLS {
            if ((c.cover.w[cell >> 6] >> (cell & 63)) & 1) != 0 {
                cells.push(cell);
            }
        }
        out[idx] = cells;
    }
    out
}

fn local_search_add_remove(
    input: &Input,
    cands: &[RobotSpec],
    cover_cells: &[Vec<usize>],
    selected: &mut Vec<usize>,
    deadline: Instant,
    rng: &mut XorShift64,
) {
    let n = cands.len();
    let mut in_sel = vec![false; n];
    for &idx in selected.iter() {
        in_sel[idx] = true;
    }
    let mut counts = [0u16; CELLS];
    let mut m_sum = 0usize;
    for &idx in selected.iter() {
        m_sum += cands[idx].m;
        for &cell in &cover_cells[idx] {
            counts[cell] += 1;
        }
    }

    let mut cur_sel = selected.clone();
    let mut steps = 0usize;
    while Instant::now() < deadline {
        steps += 1;
        let add_idx = rng.gen_usize(n);
        if in_sel[add_idx] {
            continue;
        }

        let mut tmp_counts = counts;
        for &cell in &cover_cells[add_idx] {
            tmp_counts[cell] += 1;
        }

        let mut remove_order = cur_sel.clone();
        remove_order.sort_by_key(|&idx| std::cmp::Reverse(cands[idx].m));
        let mut removed = Vec::<usize>::new();
        let mut removed_m = 0usize;
        for rid in remove_order {
            let mut ok = true;
            for &cell in &cover_cells[rid] {
                if tmp_counts[cell] <= 1 {
                    ok = false;
                    break;
                }
            }
            if ok {
                for &cell in &cover_cells[rid] {
                    tmp_counts[cell] -= 1;
                }
                removed.push(rid);
                removed_m += cands[rid].m;
            }
        }

        let new_k = cur_sel.len() + 1 - removed.len();
        let new_m = m_sum + cands[add_idx].m - removed_m;
        let old_k = cur_sel.len();
        let old_m = m_sum;
        // 壁は固定 (W=0) のまま、ΔV<0 の更新だけ採用する。
        let dv = delta_v(input, old_k, old_m, 0, new_k, new_m, 0);
        if dv > 0 || (dv == 0 && (new_m, new_k) >= (old_m, old_k)) {
            continue;
        }

        counts = tmp_counts;
        m_sum = new_m;
        in_sel[add_idx] = true;
        for rid in removed {
            in_sel[rid] = false;
        }
        let mut next_sel = Vec::<usize>::with_capacity(new_k);
        for &idx in &cur_sel {
            if in_sel[idx] {
                next_sel.push(idx);
            }
        }
        next_sel.push(add_idx);
        cur_sel = next_sel;

        if steps % 32 == 0 {
            prune_redundant(&mut cur_sel, cands, BitSet::all_400());
            in_sel.fill(false);
            m_sum = 0;
            counts = [0u16; CELLS];
            for &idx in &cur_sel {
                in_sel[idx] = true;
                m_sum += cands[idx].m;
                for &cell in &cover_cells[idx] {
                    counts[cell] += 1;
                }
            }
        }
    }

    prune_redundant(&mut cur_sel, cands, BitSet::all_400());
    *selected = cur_sel;
}

fn parse_env_bool(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            !(t == "0" || t == "false" || t == "off" || t == "no")
        }
        Err(_) => default,
    }
}

fn eval_energy(
    ak: i64,
    am: i64,
    k: usize,
    m_sum: usize,
    uncovered: usize,
    penalty_unit: i64,
) -> i64 {
    let k_cost = ak * (k.saturating_sub(1) as i64);
    let m_cost = am * (m_sum as i64);
    let penalty = penalty_unit * (uncovered as i64);
    k_cost + m_cost + penalty
}

fn toggle_hill_climb(
    input: &Input,
    cands: &[RobotSpec],
    cover_cells: &[Vec<usize>],
    selected: &mut Vec<usize>,
    deadline: Instant,
    rng: &mut XorShift64,
) {
    let n = cands.len();
    if n == 0 {
        return;
    }

    let mut in_sel = vec![false; n];
    let mut best_in_sel = vec![false; n];
    let mut counts = [0u16; CELLS];
    let mut k = 0usize;
    let mut m_sum = 0usize;
    let mut uncovered = CELLS;
    for &idx in selected.iter() {
        if in_sel[idx] {
            continue;
        }
        in_sel[idx] = true;
        k += 1;
        m_sum += cands[idx].m;
        for &cell in &cover_cells[idx] {
            if counts[cell] == 0 {
                uncovered -= 1;
            }
            counts[cell] += 1;
        }
    }

    let penalty_unit = input.ak * CELLS as i64 + input.am * (4 * CELLS) as i64 + 1;
    let mut cur_energy = eval_energy(input.ak, input.am, k, m_sum, uncovered, penalty_unit);
    let mut best_energy = cur_energy;
    best_in_sel.clone_from(&in_sel);

    while Instant::now() < deadline {
        let idx = rng.gen_usize(n);
        if in_sel[idx] {
            let mut next_uncovered = uncovered;
            for &cell in &cover_cells[idx] {
                if counts[cell] == 1 {
                    next_uncovered += 1;
                }
            }
            let next_k = k - 1;
            let next_m = m_sum - cands[idx].m;
            let next_energy = eval_energy(
                input.ak,
                input.am,
                next_k,
                next_m,
                next_uncovered,
                penalty_unit,
            );
            if next_energy < cur_energy {
                for &cell in &cover_cells[idx] {
                    counts[cell] -= 1;
                }
                in_sel[idx] = false;
                k = next_k;
                m_sum = next_m;
                uncovered = next_uncovered;
                cur_energy = next_energy;
                if cur_energy < best_energy {
                    best_energy = cur_energy;
                    best_in_sel.clone_from(&in_sel);
                }
            }
        } else {
            let mut next_uncovered = uncovered;
            for &cell in &cover_cells[idx] {
                if counts[cell] == 0 {
                    next_uncovered -= 1;
                }
            }
            let next_k = k + 1;
            let next_m = m_sum + cands[idx].m;
            let next_energy = eval_energy(
                input.ak,
                input.am,
                next_k,
                next_m,
                next_uncovered,
                penalty_unit,
            );
            if next_energy < cur_energy {
                for &cell in &cover_cells[idx] {
                    counts[cell] += 1;
                }
                in_sel[idx] = true;
                k = next_k;
                m_sum = next_m;
                uncovered = next_uncovered;
                cur_energy = next_energy;
                if cur_energy < best_energy {
                    best_energy = cur_energy;
                    best_in_sel.clone_from(&in_sel);
                }
            }
        }
    }

    let mut next_sel = Vec::<usize>::new();
    for (idx, &on) in best_in_sel.iter().enumerate() {
        if on {
            next_sel.push(idx);
        }
    }
    prune_redundant(&mut next_sel, cands, BitSet::all_400());
    *selected = next_sel;
}

fn pick_solution_timed(
    input: &Input,
    cands: &[RobotSpec],
    deadline: Instant,
    seed: u64,
    enable_toggle_hill: bool,
) -> Vec<usize> {
    let all = BitSet::all_400();
    let weights: Vec<i64> = cands
        .iter()
        .map(|c| input.ak + input.am * c.m as i64)
        .collect();

    let mut forced_list = Vec::<Option<usize>>::new();
    forced_list.push(None);

    let mut full_idx = None;
    for (i, c) in cands.iter().enumerate() {
        if c.cover == all {
            match full_idx {
                None => full_idx = Some(i),
                Some(j) => {
                    if c.m < cands[j].m {
                        full_idx = Some(i);
                    }
                }
            }
        }
    }
    if let Some(i) = full_idx {
        forced_list.push(Some(i));
    }

    let mut order: Vec<usize> = (0..cands.len()).collect();
    order.sort_by(|&i, &j| {
        let gi = cands[i].cover.count_total() as i128;
        let gj = cands[j].cover.count_total() as i128;
        let wi = weights[i] as i128;
        let wj = weights[j] as i128;
        let lhs = gi * wj;
        let rhs = gj * wi;
        rhs.cmp(&lhs)
            .then_with(|| wi.cmp(&wj))
            .then_with(|| j.cmp(&i))
    });
    let mut used = HashSet::<usize>::new();
    for &idx in order.iter().take(DEFAULT_FORCED_TOP) {
        if used.insert(idx) {
            forced_list.push(Some(idx));
        }
    }

    let mut rng = XorShift64::new(seed ^ 0xC2B2AE3D27D4EB4F);
    let mut best_sel = Vec::<usize>::new();
    let mut best_key = (i64::MAX, usize::MAX, usize::MAX);

    for forced in forced_list {
        if let Some(sel) = greedy_cover_random(cands, &weights, all, forced, &mut rng, 0) {
            let (v, k, m_sum) = eval_cost(input, &sel, cands);
            let key = (v, m_sum, k);
            if key < best_key {
                best_key = key;
                best_sel = sel;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
    }

    while Instant::now() < deadline {
        let forced = if rng.gen_usize(100) < 60 {
            Some(order[rng.gen_usize(order.len().min(128))])
        } else {
            None
        };
        let jitter = match rng.gen_usize(5) {
            0 => 0,
            1 => 500,
            2 => 1500,
            3 => 3000,
            _ => 6000,
        };
        if let Some(sel) = greedy_cover_random(cands, &weights, all, forced, &mut rng, jitter) {
            let (v, k, m_sum) = eval_cost(input, &sel, cands);
            let key = (v, m_sum, k);
            if key < best_key {
                best_key = key;
                best_sel = sel;
            }
        }
    }

    if best_sel.is_empty() {
        for (i, c) in cands.iter().enumerate() {
            if c.cover == all {
                return vec![i];
            }
        }
        panic!("no valid solution");
    }

    if Instant::now() < deadline {
        let cover_cells = build_cover_cells(cands);
        if enable_toggle_hill {
            let now = Instant::now();
            let remain = deadline.saturating_duration_since(now);
            let ls_deadline = now + remain.mul_f64(0.7);
            local_search_add_remove(
                input,
                cands,
                &cover_cells,
                &mut best_sel,
                ls_deadline,
                &mut rng,
            );
            if Instant::now() < deadline {
                toggle_hill_climb(
                    input,
                    cands,
                    &cover_cells,
                    &mut best_sel,
                    deadline,
                    &mut rng,
                );
            }
        } else {
            local_search_add_remove(
                input,
                cands,
                &cover_cells,
                &mut best_sel,
                deadline,
                &mut rng,
            );
        }
    }
    best_sel
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);
    let _ = input.aw;
    let enable_team_templates =
        parse_env_bool("V012_ENABLE_TEAM_TEMPLATES", DEFAULT_ENABLE_TEAM_TEMPLATES);
    let enable_toggle_hill = parse_env_bool("V012_ENABLE_TOGGLE_HILL", DEFAULT_ENABLE_TOGGLE_HILL);
    let limit_ms = std::env::var("SEARCH_TIME_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEARCH_TIME_MS);
    let start = Instant::now();
    let deadline = start + Duration::from_millis(limit_ms);
    let mut rng = XorShift64::new(seed_from_input(&input) ^ 0x94D049BB133111EB);
    let env = build_env(&input);

    let mut best = HashMap::<BitSet, RobotSpec>::new();
    add_stationary_candidates(&input, &mut best);
    add_m1_m2_candidates(&env, &mut best);
    if enable_team_templates {
        add_teammate_template_candidates(&env, &mut best);
    }
    add_sampled_candidates_random_m(
        &env,
        &mut best,
        &mut rng,
        DEFAULT_BASE_RANDOM_TRIALS,
        DEFAULT_MAX_RANDOM_M,
    );
    let gen_deadline = start + Duration::from_millis(limit_ms.saturating_mul(45) / 100);
    if Instant::now() < gen_deadline {
        add_sampled_candidates_random_m_timed(
            &env,
            &mut best,
            &mut rng,
            gen_deadline,
            DEFAULT_MAX_RANDOM_M,
        );
    }
    if let Some(full) = build_full_route_candidate(&input, &env, FULL_ROUTE_TRIALS) {
        add_candidate(&mut best, full);
    }

    let mut cands: Vec<RobotSpec> = best.into_values().collect();
    cands.sort_by_key(|c| c.m);
    let selected = pick_solution_timed(
        &input,
        &cands,
        deadline,
        seed_from_input(&input),
        enable_toggle_hill,
    );

    println!("{}", selected.len());
    for &idx in &selected {
        let r = &cands[idx];
        let i = r.start_cell / input.n;
        let j = r.start_cell % input.n;
        println!("{} {} {} {}", r.m, i, j, DIR_CHARS[r.start_dir]);
        for s in 0..r.m {
            println!(
                "{} {} {} {}",
                act_char(r.a0[s]),
                r.b0[s],
                act_char(r.a1[s]),
                r.b1[s]
            );
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
