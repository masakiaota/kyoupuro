// v003_a_setcover.rs
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DIR_CHARS: [char; 4] = ['U', 'R', 'D', 'L'];
const N_FIXED: usize = 20;
const BIT_WORDS: usize = 7; // 7*64=448 >= 400
const M3_SAMPLES: usize = 1200;

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
    wall: Vec<bool>,
    next_o: Vec<[usize; 3]>,
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
    fn set_cell(&mut self, c: usize) {
        self.w[c >> 6] |= 1u64 << (c & 63);
    }
    fn or_assign(&mut self, rhs: &Self) {
        for k in 0..BIT_WORDS {
            self.w[k] |= rhs.w[k];
        }
    }
    fn count_total(&self) -> u32 {
        self.w.iter().map(|x| x.count_ones()).sum()
    }
    fn count_new(&self, covered: &Self) -> u32 {
        let mut s = 0;
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

fn build_env(input: &Input) -> Env {
    let n = input.n;
    let cells = n * n;
    let orients = cells * 4;
    let mut wall = vec![false; orients];
    let mut next_o = vec![[0usize; 3]; orients];
    for i in 0..n {
        for j in 0..n {
            let c = i * n + j;
            for d in 0..4 {
                let o = c * 4 + d;
                let w = has_wall(input, i, j, d);
                wall[o] = w;
                next_o[o][ACT_R as usize] = c * 4 + (d + 1) % 4;
                next_o[o][ACT_L as usize] = c * 4 + (d + 3) % 4;
                if w {
                    next_o[o][ACT_F as usize] = o;
                } else {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    let nc = ni * n + nj;
                    next_o[o][ACT_F as usize] = nc * 4 + d;
                }
            }
        }
    }
    Env { n, wall, next_o }
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
        let w = env.wall[o];
        for s in 0..m {
            let act = if w { a1[s] } else { a0[s] };
            let ns = if w { b1[s] } else { b0[s] };
            next[o * m + s] = env.next_o[o][act as usize] * m + ns;
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
            let p = stack.iter().rposition(|&x| x == u).unwrap();
            let mut bits = BitSet::empty();
            for &x in &stack[p..] {
                bits.set_cell((x / m) / 4);
            }
            let cid = cycles.len();
            cycles.push(bits);
            for &x in &stack[p..] {
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
        let bits = cycles[comp[o * m]];
        local.entry(bits).or_insert(o);
    }
    for (cover, &start_o) in &local {
        let cand = RobotSpec {
            m,
            start_cell: start_o / 4,
            start_dir: start_o % 4,
            a0: a0.to_vec(),
            b0: b0.to_vec(),
            a1: a1.to_vec(),
            b1: b1.to_vec(),
            cover: *cover,
        };
        add_candidate(best, cand);
    }
}

fn add_stationary(input: &Input, best: &mut HashMap<BitSet, RobotSpec>) {
    for c in 0..input.n * input.n {
        let mut cover = BitSet::empty();
        cover.set_cell(c);
        let cand = RobotSpec {
            m: 1,
            start_cell: c,
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

fn add_m1_m2(env: &Env, best: &mut HashMap<BitSet, RobotSpec>) {
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

fn add_random_m3(input: &Input, env: &Env, best: &mut HashMap<BitSet, RobotSpec>) {
    let mut rng = XorShift64::new(seed_from_input(input) ^ 0xD1B54A32D192ED03);
    for _ in 0..M3_SAMPLES {
        let m = 3usize;
        let mut a0 = vec![0u8; m];
        let mut b0 = vec![0usize; m];
        let mut a1 = vec![0u8; m];
        let mut b1 = vec![0usize; m];
        for s in 0..m {
            a0[s] = rng.gen_usize(3) as u8;
            b0[s] = rng.gen_usize(m);
            a1[s] = rng.gen_usize(2) as u8;
            b1[s] = rng.gen_usize(m);
        }
        collect_from_automaton(env, m, &a0, &b0, &a1, &b1, best);
    }
}

fn eval_cost(input: &Input, sel: &[usize], cands: &[RobotSpec]) -> (i64, usize, usize) {
    let k = sel.len();
    let m_sum: usize = sel.iter().map(|&i| cands[i].m).sum();
    let v = input.ak * (k as i64 - 1) + input.am * m_sum as i64;
    (v, k, m_sum)
}

fn prune_redundant(sel: &mut Vec<usize>, cands: &[RobotSpec], all: BitSet) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < sel.len() {
            let mut cov = BitSet::empty();
            for (j, &idx) in sel.iter().enumerate() {
                if i != j {
                    cov.or_assign(&cands[idx].cover);
                }
            }
            if cov == all {
                sel.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
    }
}

fn greedy_cover(
    cands: &[RobotSpec],
    weights: &[i64],
    all: BitSet,
    forced: Option<usize>,
) -> Option<Vec<usize>> {
    let mut covered = BitSet::empty();
    let mut sel = Vec::<usize>::new();
    if let Some(i) = forced {
        sel.push(i);
        covered.or_assign(&cands[i].cover);
    }
    while covered != all {
        let mut best_idx = None;
        let mut best_gain = 0u32;
        let mut best_w = 1i64;
        for i in 0..cands.len() {
            let gain = cands[i].cover.count_new(&covered);
            if gain == 0 {
                continue;
            }
            let w = weights[i];
            let better = match best_idx {
                None => true,
                Some(_) => {
                    let lhs = gain as i128 * best_w as i128;
                    let rhs = best_gain as i128 * w as i128;
                    if lhs != rhs {
                        lhs > rhs
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
        let i = best_idx?;
        sel.push(i);
        covered.or_assign(&cands[i].cover);
    }
    prune_redundant(&mut sel, cands, all);
    Some(sel)
}

fn pick_solution(input: &Input, cands: &[RobotSpec]) -> Vec<usize> {
    let all = BitSet::all_400();
    let weights: Vec<i64> = cands
        .iter()
        .map(|c| input.ak + input.am * c.m as i64)
        .collect();

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

    let mut forced = vec![None];
    let mut used = HashSet::<usize>::new();
    for &idx in order.iter().take(60) {
        if used.insert(idx) {
            forced.push(Some(idx));
        }
    }

    let mut best_sel = Vec::<usize>::new();
    let mut best_key = (i64::MAX, usize::MAX, usize::MAX);
    for f in forced {
        if let Some(sel) = greedy_cover(cands, &weights, all, f) {
            let key = eval_cost(input, &sel, cands);
            if key < best_key {
                best_key = key;
                best_sel = sel;
            }
        }
    }
    if best_sel.is_empty() {
        panic!("failed to find cover");
    }
    best_sel
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);
    let _ = input.aw; // A専用: 壁は追加しない
    let env = build_env(&input);

    let mut best = HashMap::<BitSet, RobotSpec>::new();
    add_stationary(&input, &mut best);
    add_m1_m2(&env, &mut best);
    add_random_m3(&input, &env, &mut best);

    let mut cands: Vec<RobotSpec> = best.into_values().collect();
    cands.sort_by_key(|c| c.m);
    let sel = pick_solution(&input, &cands);

    println!("{}", sel.len());
    for &idx in &sel {
        let r = &cands[idx];
        println!(
            "{} {} {} {}",
            r.m,
            r.start_cell / input.n,
            r.start_cell % input.n,
            DIR_CHARS[r.start_dir]
        );
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
