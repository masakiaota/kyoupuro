use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::thread;

const N_FIXED: usize = 20;
const CELLS: usize = N_FIXED * N_FIXED;
const ORIENTS: usize = CELLS * 4;
const BIT_WORDS: usize = 7; // 7*64 >= 400

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];

#[derive(Clone)]
struct MapInput {
    n: usize,
    wall_v: Vec<Vec<u8>>,
    wall_h: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct Env {
    wall: Vec<bool>,         // oriented state -> front wall?
    next_o: Vec<[usize; 3]>, // oriented state -> next oriented state for R/L/F
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BitSet {
    w: [u64; BIT_WORDS],
}

impl BitSet {
    fn empty() -> Self {
        Self { w: [0; BIT_WORDS] }
    }
    fn set(&mut self, cell: usize) {
        self.w[cell >> 6] |= 1u64 << (cell & 63);
    }
    fn count(&self) -> u32 {
        self.w.iter().map(|x| x.count_ones()).sum()
    }
    fn jaccard_distance_scaled(&self, other: &Self) -> i32 {
        let mut inter = 0u32;
        let mut uni = 0u32;
        for k in 0..BIT_WORDS {
            inter += (self.w[k] & other.w[k]).count_ones();
            uni += (self.w[k] | other.w[k]).count_ones();
        }
        if uni == 0 {
            return 0;
        }
        (((uni - inter) as i64 * 10_000) / uni as i64) as i32
    }
}

#[derive(Clone, Copy)]
struct Rule {
    a0: u8,
    b0: u8,
    a1: u8,
    b1: u8,
}

#[derive(Clone)]
struct Automaton {
    m: usize,
    rules: Vec<Rule>,
}

#[derive(Clone)]
struct EvalMetrics {
    total_cover: i32,
    min_cover: i32,
    total_cycle_len: i32,
    proxy_cover: i32,
    fingerprint: BitSet,
}

#[derive(Clone)]
struct Cand {
    aut: Automaton,
    metrics: EvalMetrics,
    fitness: i64,
    origin: String,
}

impl Cand {
    fn avg_cover(&self, maps: usize) -> f64 {
        self.metrics.total_cover as f64 / maps as f64
    }
    fn avg_cycle(&self, maps: usize) -> f64 {
        self.metrics.total_cycle_len as f64 / maps as f64
    }
}

#[derive(Clone, Copy)]
struct HeapKey {
    fitness: i64,
    idx: usize,
}

impl Ord for HeapKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fitness
            .cmp(&other.fitness)
            .then_with(|| self.idx.cmp(&other.idx))
    }
}
impl PartialOrd for HeapKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for HeapKey {
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness && self.idx == other.idx
    }
}
impl Eq for HeapKey {}

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
        if upper == 0 {
            0
        } else {
            (self.next_u64() % upper as u64) as usize
        }
    }
}

struct Config {
    problem: char,
    input_dir: String,
    stride: usize,
    max_seed: usize,
    exact_m: usize,
    max_m: usize,
    trials_per_m: usize,
    top_cap: usize,
    keep_diverse: usize,
    min_diverse_dist: i32,
    quick_cover_floor: i32,
    parallel: usize,
    output: String,
    seed: u64,
}

impl Config {
    fn from_args() -> Self {
        let mut cfg = Config {
            problem: 'B',
            input_dir: "tools/inB".to_string(),
            stride: 4,
            max_seed: 96,
            exact_m: 4,
            max_m: 12,
            trials_per_m: 25_000,
            top_cap: 3500,
            keep_diverse: 80,
            min_diverse_dist: 900,
            quick_cover_floor: 90,
            parallel: std::thread::available_parallelism()
                .map(|v| v.get())
                .unwrap_or(1),
            output: "results/automata_library_B.txt".to_string(),
            seed: 1,
        };
        for arg in std::env::args().skip(1) {
            if let Some(v) = arg.strip_prefix("--problem=") {
                let c = v.chars().next().unwrap_or('B');
                cfg.problem = c;
                cfg.input_dir = format!("tools/in{}", c);
                cfg.output = format!("results/automata_library_{}.txt", c);
            } else if let Some(v) = arg.strip_prefix("--input-dir=") {
                cfg.input_dir = v.to_string();
            } else if let Some(v) = arg.strip_prefix("--stride=") {
                cfg.stride = v.parse().unwrap_or(cfg.stride);
            } else if let Some(v) = arg.strip_prefix("--max-seed=") {
                cfg.max_seed = v.parse().unwrap_or(cfg.max_seed);
            } else if let Some(v) = arg.strip_prefix("--exact-m=") {
                cfg.exact_m = v.parse().unwrap_or(cfg.exact_m);
            } else if let Some(v) = arg.strip_prefix("--max-m=") {
                cfg.max_m = v.parse().unwrap_or(cfg.max_m);
            } else if let Some(v) = arg.strip_prefix("--trials-per-m=") {
                cfg.trials_per_m = v.parse().unwrap_or(cfg.trials_per_m);
            } else if let Some(v) = arg.strip_prefix("--top-cap=") {
                cfg.top_cap = v.parse().unwrap_or(cfg.top_cap);
            } else if let Some(v) = arg.strip_prefix("--keep-diverse=") {
                cfg.keep_diverse = v.parse().unwrap_or(cfg.keep_diverse);
            } else if let Some(v) = arg.strip_prefix("--min-diverse-dist=") {
                cfg.min_diverse_dist = v.parse().unwrap_or(cfg.min_diverse_dist);
            } else if let Some(v) = arg.strip_prefix("--quick-cover-floor=") {
                cfg.quick_cover_floor = v.parse().unwrap_or(cfg.quick_cover_floor);
            } else if let Some(v) = arg.strip_prefix("--parallel=") {
                cfg.parallel = v.parse().unwrap_or(cfg.parallel).max(1);
            } else if let Some(v) = arg.strip_prefix("--output=") {
                cfg.output = v.to_string();
            } else if let Some(v) = arg.strip_prefix("--seed=") {
                cfg.seed = v.parse().unwrap_or(cfg.seed);
            }
        }
        cfg
    }
}

fn has_wall(input: &MapInput, i: usize, j: usize, d: usize) -> bool {
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

fn build_env(map: &MapInput) -> Env {
    let mut wall = vec![false; ORIENTS];
    let mut next_o = vec![[0usize; 3]; ORIENTS];
    for i in 0..N_FIXED {
        for j in 0..N_FIXED {
            let cell = i * N_FIXED + j;
            for d in 0..4 {
                let o = cell * 4 + d;
                let w = has_wall(map, i, j, d);
                wall[o] = w;
                next_o[o][ACT_R as usize] = cell * 4 + (d + 1) % 4;
                next_o[o][ACT_L as usize] = cell * 4 + (d + 3) % 4;
                if w {
                    next_o[o][ACT_F as usize] = o;
                } else {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    let ncell = ni * N_FIXED + nj;
                    next_o[o][ACT_F as usize] = ncell * 4 + d;
                }
            }
        }
    }
    Env { wall, next_o }
}

fn parse_map_file(path: &Path) -> io::Result<MapInput> {
    let mut s = String::new();
    File::open(path)?.read_to_string(&mut s)?;
    let mut it = s.split_whitespace();
    let n: usize = it.next().unwrap().parse().unwrap();
    let _ak: i64 = it.next().unwrap().parse().unwrap();
    let _am: i64 = it.next().unwrap().parse().unwrap();
    let _aw: i64 = it.next().unwrap().parse().unwrap();
    let mut wall_v = vec![vec![0u8; n - 1]; n];
    for i in 0..n {
        let line = it.next().unwrap().as_bytes();
        for j in 0..n - 1 {
            wall_v[i][j] = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    let mut wall_h = vec![vec![0u8; n]; n - 1];
    for i in 0..n - 1 {
        let line = it.next().unwrap().as_bytes();
        for j in 0..n {
            wall_h[i][j] = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    Ok(MapInput { n, wall_v, wall_h })
}

fn load_envs(cfg: &Config) -> io::Result<Vec<Env>> {
    let mut envs = Vec::<Env>::new();
    let mut seed = 0usize;
    while seed <= cfg.max_seed {
        let path = format!("{}/{:04}.txt", cfg.input_dir, seed);
        let map = parse_map_file(Path::new(&path))?;
        if map.n != N_FIXED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected N in {}: {}", path, map.n),
            ));
        }
        envs.push(build_env(&map));
        seed += cfg.stride;
    }
    Ok(envs)
}

fn reachable_states_count(aut: &Automaton) -> usize {
    let m = aut.m;
    let mut vis = vec![false; m];
    let mut st = vec![0usize];
    vis[0] = true;
    while let Some(v) = st.pop() {
        let r = aut.rules[v];
        let to0 = r.b0 as usize;
        let to1 = r.b1 as usize;
        if !vis[to0] {
            vis[to0] = true;
            st.push(to0);
        }
        if !vis[to1] {
            vis[to1] = true;
            st.push(to1);
        }
    }
    vis.into_iter().filter(|&x| x).count()
}

fn build_next_and_action(env: &Env, aut: &Automaton, next: &mut [usize], act_used: &mut [u8]) {
    let m = aut.m;
    for o in 0..ORIENTS {
        let wall = env.wall[o];
        for s in 0..m {
            let r = aut.rules[s];
            let (act, ns) = if wall {
                (r.a1, r.b1 as usize)
            } else {
                (r.a0, r.b0 as usize)
            };
            let no = env.next_o[o][act as usize];
            let idx = o * m + s;
            next[idx] = no * m + ns;
            act_used[idx] = act;
        }
    }
}

fn analyze_on_env(env: &Env, aut: &Automaton) -> (i32, i32, BitSet) {
    let m = aut.m;
    let total = ORIENTS * m;
    let mut next = vec![0usize; total];
    let mut act_used = vec![0u8; total];
    build_next_and_action(env, aut, &mut next, &mut act_used);

    let mut mark = vec![0u8; total];
    let mut comp = vec![usize::MAX; total];
    let mut comp_cover = Vec::<BitSet>::new();
    let mut comp_cycle_len = Vec::<i32>::new();

    let mut stack = Vec::<usize>::new();
    for st in 0..total {
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
            let cyc = &stack[pos..];
            for &x in cyc {
                let o = x / m;
                bits.set(o / 4);
            }
            let id = comp_cover.len();
            comp_cover.push(bits);
            comp_cycle_len.push(cyc.len() as i32);
            for &x in cyc {
                comp[x] = id;
            }
            id
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

    let mut best_cover = -1i32;
    let mut best_cycle = i32::MAX;
    let mut best_bits = BitSet::empty();
    for o in 0..ORIENTS {
        let st0 = o * m;
        let cid = comp[st0];
        let c = comp_cover[cid].count() as i32;
        let cyc = comp_cycle_len[cid];
        if c > best_cover || (c == best_cover && cyc < best_cycle) {
            best_cover = c;
            best_cycle = cyc;
            best_bits = comp_cover[cid];
        }
    }

    (best_cover, best_cycle, best_bits)
}

fn evaluate_candidate(aut: &Automaton, envs: &[Env], quick_cover_floor: i32) -> Option<EvalMetrics> {
    if reachable_states_count(aut) != aut.m {
        return None;
    }

    let (q_cover, _, _) = analyze_on_env(&envs[0], aut);
    if q_cover < quick_cover_floor {
        return None;
    }

    let mut total_cover = 0i32;
    let mut min_cover = i32::MAX;
    let mut total_cycle_len = 0i32;
    let mut fingerprint = BitSet::empty();

    for (idx, env) in envs.iter().enumerate() {
        let (cover, cyc, bits) = analyze_on_env(env, aut);
        total_cover += cover;
        min_cover = min_cover.min(cover);
        total_cycle_len += cyc;
        if idx < 8 {
            for k in 0..BIT_WORDS {
                fingerprint.w[k] ^= bits.w[k].rotate_left((idx as u32) & 31);
            }
        }
    }

    Some(EvalMetrics {
        total_cover,
        min_cover,
        total_cycle_len,
        proxy_cover: q_cover,
        fingerprint,
    })
}

fn compute_fitness(problem: char, aut: &Automaton, met: &EvalMetrics, maps: usize) -> i64 {
    let m = aut.m as i64;
    let total_cover = met.total_cover as i64;
    let min_cover = met.min_cover as i64;
    let cyc = met.total_cycle_len as i64;
    let proxy = met.proxy_cover as i64;
    let base = total_cover * 10_000 + min_cover * 3_000 + proxy * 1_000 - cyc;
    match problem {
        // Aは壁なし前提でM最小化が本質なので、mへの罰則を強める。
        'A' => {
            let full_bonus = if min_cover >= CELLS as i64 {
                2_000_000_000i64
            } else {
                0
            };
            base + full_bonus - m * (maps as i64) * 6_000
        }
        'C' => base - m * (maps as i64) * 2_300,
        _ => base - m * (maps as i64) * 2_000,
    }
}

fn add_top_candidate(pool: &mut Vec<Cand>, heap: &mut BinaryHeap<Reverse<HeapKey>>, cap: usize, cand: Cand) {
    if pool.len() < cap {
        let idx = pool.len();
        let key = HeapKey {
            fitness: cand.fitness,
            idx,
        };
        pool.push(cand);
        heap.push(Reverse(key));
        return;
    }

    if let Some(Reverse(min_key)) = heap.peek().copied() {
        if cand.fitness <= min_key.fitness {
            return;
        }
        let replace_idx = min_key.idx;
        heap.pop();
        pool[replace_idx] = cand;
        heap.push(Reverse(HeapKey {
            fitness: pool[replace_idx].fitness,
            idx: replace_idx,
        }));
    }
}

fn make_automaton_from_rules(rules: &[Rule]) -> Automaton {
    Automaton {
        m: rules.len(),
        rules: rules.to_vec(),
    }
}

fn exact_enumerate_m(
    m: usize,
    envs: &[Env],
    cfg: &Config,
    pool: &mut Vec<Cand>,
    heap: &mut BinaryHeap<Reverse<HeapKey>>,
) {
    fn dfs(
        i: usize,
        m: usize,
        max_seen: usize,
        rules: &mut [Rule],
        envs: &[Env],
        cfg: &Config,
        pool: &mut Vec<Cand>,
        heap: &mut BinaryHeap<Reverse<HeapKey>>,
    ) {
        if i == m {
            if max_seen + 1 != m {
                return;
            }
            let aut = make_automaton_from_rules(rules);
            if let Some(met) = evaluate_candidate(&aut, envs, cfg.quick_cover_floor) {
                let fit = compute_fitness(cfg.problem, &aut, &met, envs.len());
                let cand = Cand {
                    aut,
                    metrics: met,
                    fitness: fit,
                    origin: format!("exact_m{}", m),
                };
                add_top_candidate(pool, heap, cfg.top_cap, cand);
            }
            return;
        }

        let remain_edges = 2 * (m - i - 1);
        let seen_cnt = max_seen + 1;
        if seen_cnt + remain_edges < m {
            return;
        }

        for a0 in 0..3u8 {
            let b0_max = (max_seen + 1).min(m - 1);
            for b0 in 0..=b0_max {
                let ms1 = max_seen.max(b0);
                for a1 in 0..2u8 {
                    let b1_max = (ms1 + 1).min(m - 1);
                    for b1 in 0..=b1_max {
                        let ms2 = ms1.max(b1);
                        rules[i] = Rule {
                            a0,
                            b0: b0 as u8,
                            a1,
                            b1: b1 as u8,
                        };
                        dfs(i + 1, m, ms2, rules, envs, cfg, pool, heap);
                    }
                }
            }
        }
    }

    let mut rules = vec![
        Rule {
            a0: ACT_F,
            b0: 0,
            a1: ACT_R,
            b1: 0,
        };
        m
    ];
    dfs(0, m, 0, &mut rules, envs, cfg, pool, heap);
}

fn mutate_add_state(parent: &Automaton, target_m: usize, rng: &mut XorShift64) -> Automaton {
    let mut aut = parent.clone();
    while aut.m < target_m {
        let old_m = aut.m;
        let new_idx = old_m;
        // 既存のどこかから新状態へ遷移を1本刺す
        let src = rng.gen_usize(old_m);
        if rng.gen_usize(2) == 0 {
            aut.rules[src].b0 = new_idx as u8;
        } else {
            aut.rules[src].b1 = new_idx as u8;
        }
        let rule = Rule {
            a0: rng.gen_usize(3) as u8,
            b0: rng.gen_usize(old_m + 1) as u8,
            a1: rng.gen_usize(2) as u8,
            b1: rng.gen_usize(old_m + 1) as u8,
        };
        aut.rules.push(rule);
        aut.m += 1;
    }
    aut
}

fn mutate_local(mut aut: Automaton, rng: &mut XorShift64, steps: usize) -> Automaton {
    for _ in 0..steps {
        let s = rng.gen_usize(aut.m);
        let op = rng.gen_usize(6);
        match op {
            0 => aut.rules[s].a0 = rng.gen_usize(3) as u8,
            1 => aut.rules[s].a1 = rng.gen_usize(2) as u8,
            2 => aut.rules[s].b0 = rng.gen_usize(aut.m) as u8,
            3 => aut.rules[s].b1 = rng.gen_usize(aut.m) as u8,
            4 => {
                aut.rules[s].a0 = rng.gen_usize(3) as u8;
                aut.rules[s].b0 = rng.gen_usize(aut.m) as u8;
            }
            _ => {
                aut.rules[s].a1 = rng.gen_usize(2) as u8;
                aut.rules[s].b1 = rng.gen_usize(aut.m) as u8;
            }
        }
    }
    aut
}

fn take_best_by_m(pool: &[Cand], m: usize, limit: usize) -> Vec<Cand> {
    let mut v: Vec<Cand> = pool.iter().filter(|c| c.aut.m == m).cloned().collect();
    v.sort_by(|a, b| b.fitness.cmp(&a.fitness));
    if v.len() > limit {
        v.truncate(limit);
    }
    v
}

fn guided_expand(
    envs: &[Env],
    cfg: &Config,
    pool: &mut Vec<Cand>,
    heap: &mut BinaryHeap<Reverse<HeapKey>>,
) {
    let envs_arc = Arc::new(envs.to_vec());
    let workers = cfg.parallel.max(1);

    for m in (cfg.exact_m + 1)..=cfg.max_m {
        let parents = take_best_by_m(pool, m - 1, 120);
        if parents.is_empty() {
            continue;
        }

        let threads = workers.min(cfg.trials_per_m.max(1));
        let base_trials = cfg.trials_per_m / threads;
        let rem_trials = cfg.trials_per_m % threads;
        let parents_arc = Arc::new(parents);

        let mut handles = Vec::with_capacity(threads);
        for tid in 0..threads {
            let trials = base_trials + usize::from(tid < rem_trials);
            if trials == 0 {
                continue;
            }
            let envs_local = Arc::clone(&envs_arc);
            let parents_local = Arc::clone(&parents_arc);
            let quick_cover_floor = cfg.quick_cover_floor;
            let problem = cfg.problem;
            let local_cap = (cfg.top_cap / threads).max(256);
            let thread_seed = cfg.seed
                ^ 0xC2B2AE3D27D4EB4F
                ^ ((m as u64) << 32)
                ^ ((tid as u64).wrapping_mul(0x9E3779B97F4A7C15));
            handles.push(thread::spawn(move || {
                let mut rng = XorShift64::new(thread_seed);
                let mut local_pool = Vec::<Cand>::new();
                let mut local_heap = BinaryHeap::<Reverse<HeapKey>>::new();
                for _ in 0..trials {
                    let p = &parents_local[rng.gen_usize(parents_local.len())];
                    let base = mutate_add_state(&p.aut, m, &mut rng);
                    let steps = 1 + rng.gen_usize(4);
                    let aut = mutate_local(base, &mut rng, steps);
                    if let Some(met) =
                        evaluate_candidate(&aut, &envs_local, quick_cover_floor)
                    {
                        let fit = compute_fitness(problem, &aut, &met, envs_local.len());
                        let cand = Cand {
                            aut,
                            metrics: met,
                            fitness: fit,
                            origin: format!("guided_m{}", m),
                        };
                        add_top_candidate(&mut local_pool, &mut local_heap, local_cap, cand);
                    }
                }
                local_pool
            }));
        }

        let mut merged = 0usize;
        for handle in handles {
            if let Ok(local_pool) = handle.join() {
                merged += local_pool.len();
                for cand in local_pool {
                    add_top_candidate(pool, heap, cfg.top_cap, cand);
                }
            }
        }
        eprintln!(
            "phase guided m={} done (threads={}, merged={}), pool={}",
            m,
            threads,
            merged,
            pool.len()
        );
    }
}

fn diverse_select(mut all: Vec<Cand>, k: usize, min_diverse_dist: i32) -> Vec<Cand> {
    all.sort_by(|a, b| b.fitness.cmp(&a.fitness));
    let mut picked = Vec::<Cand>::new();
    for cand in all.iter().cloned() {
        if picked.is_empty() {
            picked.push(cand);
        } else {
            let mut ok = true;
            for p in &picked {
                let d = cand
                    .metrics
                    .fingerprint
                    .jaccard_distance_scaled(&p.metrics.fingerprint);
                if d < min_diverse_dist {
                    ok = false;
                    break;
                }
            }
            if ok {
                picked.push(cand);
            }
        }
        if picked.len() >= k {
            break;
        }
    }
    if picked.len() < k {
        let mut used = HashSet::<(usize, i64)>::new();
        for c in &picked {
            used.insert((c.aut.m, c.fitness));
        }
        for cand in all {
            if picked.len() >= k {
                break;
            }
            if used.insert((cand.aut.m, cand.fitness)) {
                picked.push(cand);
            }
        }
    }
    picked
}

fn act_char(a: u8) -> char {
    match a {
        ACT_R => 'R',
        ACT_L => 'L',
        ACT_F => 'F',
        _ => '?',
    }
}

fn write_library(path: &str, picked: &[Cand], maps: usize, cfg: &Config) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = File::create(path)?;
    writeln!(
        f,
        "# automata library generated by v104_automata_miner_parallel\n# problem={} input_dir={} stride={} max_seed={} exact_m={} max_m={} maps={} parallel={}\n",
        cfg.problem,
        cfg.input_dir,
        cfg.stride,
        cfg.max_seed,
        cfg.exact_m,
        cfg.max_m,
        maps,
        cfg.parallel
    )?;

    for (idx, c) in picked.iter().enumerate() {
        writeln!(
            f,
            "[candidate {}] m={} fitness={} avg_cover={:.3} min_cover={} avg_cycle={:.3} proxy_cover={} origin={}",
            idx,
            c.aut.m,
            c.fitness,
            c.avg_cover(maps),
            c.metrics.min_cover,
            c.avg_cycle(maps),
            c.metrics.proxy_cover,
            c.origin
        )?;
        for (s, r) in c.aut.rules.iter().enumerate() {
            writeln!(
                f,
                "state {:02}: {} {} {} {}",
                s,
                act_char(r.a0),
                r.b0,
                act_char(r.a1),
                r.b1
            )?;
        }
        writeln!(f)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let cfg = Config::from_args();
    eprintln!(
        "v104_miner start: problem={} exact_m={} max_m={} trials_per_m={} parallel={} stride={} max_seed={}",
        cfg.problem, cfg.exact_m, cfg.max_m, cfg.trials_per_m, cfg.parallel, cfg.stride, cfg.max_seed
    );
    let envs = load_envs(&cfg)?;
    if envs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no maps loaded"));
    }

    let mut pool = Vec::<Cand>::new();
    let mut heap = BinaryHeap::<Reverse<HeapKey>>::new();

    for m in 1..=cfg.exact_m {
        exact_enumerate_m(m, &envs, &cfg, &mut pool, &mut heap);
        eprintln!("phase exact m={} done, pool={}", m, pool.len());
    }

    if cfg.max_m > cfg.exact_m {
        guided_expand(&envs, &cfg, &mut pool, &mut heap);
        eprintln!("phase guided done, pool={}", pool.len());
    }

    let picked = diverse_select(pool.clone(), cfg.keep_diverse, cfg.min_diverse_dist);
    write_library(&cfg.output, &picked, envs.len(), &cfg)?;

    eprintln!(
        "saved {} candidates to {} (from pool={})",
        picked.len(),
        cfg.output,
        pool.len()
    );

    Ok(())
}
