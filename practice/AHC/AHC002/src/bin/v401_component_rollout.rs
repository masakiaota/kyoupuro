// v401_component_rollout.rs

use std::io::{self, Read};
use std::time::{Duration, Instant};

const N: usize = 50;
const NN: usize = N * N;
const INVALID: usize = usize::MAX;
const TIME_LIMIT_MS: u64 = 1900;
const ENDGAME_LIMIT: i32 = 18;

#[derive(Clone, Copy)]
struct Edge {
    to: usize,
    dir: u8,
}

#[derive(Clone, Copy, Default)]
struct CompStats {
    tile_sum: i32,
    tile_count: i32,
    cell_count: i32,
}

#[derive(Clone, Copy)]
struct Params {
    w_pot: f64,
    w_deg: f64,
    w_side: f64,
    noise: f64,
}

#[derive(Clone)]
struct State {
    current: usize,
    score: i32,
    used_tile: Vec<bool>,
    path: Vec<u8>,
}

struct Solver {
    start: usize,
    tile_of: Vec<usize>,
    value: Vec<i32>,
    tile_best: Vec<i32>,
    partner: Vec<usize>,
    edges: Vec<[Edge; 4]>,
    deg: Vec<u8>,
    scratch_queue: Vec<usize>,
    seen_cell: Vec<u32>,
    seen_tile: Vec<u32>,
    mark: u32,
}

impl Solver {
    fn new(
        start: usize,
        tile_of: Vec<usize>,
        value: Vec<i32>,
        tile_best: Vec<i32>,
        partner: Vec<usize>,
        edges: Vec<[Edge; 4]>,
        deg: Vec<u8>,
        tile_count: usize,
    ) -> Self {
        Self {
            start,
            tile_of,
            value,
            tile_best,
            partner,
            edges,
            deg,
            scratch_queue: Vec::with_capacity(NN),
            seen_cell: vec![0; NN],
            seen_tile: vec![0; tile_count],
            mark: 1,
        }
    }

    fn next_mark(&mut self) -> u32 {
        self.mark = self.mark.wrapping_add(1);
        if self.mark == 0 {
            self.seen_cell.fill(0);
            self.seen_tile.fill(0);
            self.mark = 1;
        }
        self.mark
    }

    fn legal_moves(&self, current: usize, used_tile: &[bool], out: &mut [Edge; 4]) -> usize {
        let mut len = 0usize;
        for i in 0..self.deg[current] as usize {
            let e = self.edges[current][i];
            if !used_tile[self.tile_of[e.to]] {
                out[len] = e;
                len += 1;
            }
        }
        len
    }

    fn legal_degree(&self, current: usize, used_tile: &[bool]) -> i32 {
        let mut deg = 0i32;
        for i in 0..self.deg[current] as usize {
            let e = self.edges[current][i];
            if !used_tile[self.tile_of[e.to]] {
                deg += 1;
            }
        }
        deg
    }

    fn component_stats(&mut self, src: usize, used_tile: &[bool]) -> CompStats {
        let mark = self.next_mark();
        self.scratch_queue.clear();
        self.scratch_queue.push(src);
        self.seen_cell[src] = mark;
        let mut head = 0usize;
        let mut stats = CompStats::default();

        while head < self.scratch_queue.len() {
            let v = self.scratch_queue[head];
            head += 1;

            for i in 0..self.deg[v] as usize {
                let to = self.edges[v][i].to;
                let tile = self.tile_of[to];
                if used_tile[tile] {
                    continue;
                }
                if self.seen_tile[tile] != mark {
                    self.seen_tile[tile] = mark;
                    stats.tile_sum += self.tile_best[tile];
                    stats.tile_count += 1;
                }
                if self.seen_cell[to] != mark {
                    self.seen_cell[to] = mark;
                    self.scratch_queue.push(to);
                    stats.cell_count += 1;
                }
            }
        }
        stats
    }

    fn eval_move(&mut self, next: usize, used_tile: &mut [bool], params: Params) -> f64 {
        let tile = self.tile_of[next];
        used_tile[tile] = true;
        let comp = self.component_stats(next, used_tile);
        let deg = self.legal_degree(next, used_tile);
        used_tile[tile] = false;

        let side_bonus = if self.partner[next] != INVALID {
            (self.value[next] - self.value[self.partner[next]]).max(0) as f64
        } else {
            0.0
        };

        self.value[next] as f64
            + params.w_pot * comp.tile_sum as f64
            + params.w_deg * deg as f64
            + params.w_side * side_bonus
    }

    fn replay_prefix(&self, prefix: &[u8]) -> State {
        let mut used_tile = vec![false; self.tile_best.len()];
        let mut current = self.start;
        let mut score = self.value[current];
        used_tile[self.tile_of[current]] = true;

        for &dir in prefix {
            let next = step_cell(current, dir);
            let tile = self.tile_of[next];
            used_tile[tile] = true;
            score += self.value[next];
            current = next;
        }

        State {
            current,
            score,
            used_tile,
            path: prefix.to_vec(),
        }
    }

    fn finish_greedy(
        &mut self,
        mut state: State,
        rng: &mut XorShift64,
        params: Params,
        deadline: Instant,
    ) -> State {
        let mut moves = [Edge {
            to: INVALID,
            dir: b'?',
        }; 4];

        loop {
            if Instant::now() >= deadline {
                break;
            }

            let rem = self.component_stats(state.current, &state.used_tile);
            if rem.tile_count <= ENDGAME_LIMIT {
                let suffix = self.solve_endgame(state.current, &mut state.used_tile, deadline);
                for &dir in &suffix {
                    let next = step_cell(state.current, dir);
                    let tile = self.tile_of[next];
                    state.used_tile[tile] = true;
                    state.score += self.value[next];
                    state.path.push(dir);
                    state.current = next;
                }
                break;
            }

            let len = self.legal_moves(state.current, &state.used_tile, &mut moves);
            if len == 0 {
                break;
            }
            if len == 1 {
                let mv = moves[0];
                let tile = self.tile_of[mv.to];
                state.used_tile[tile] = true;
                state.score += self.value[mv.to];
                state.path.push(mv.dir);
                state.current = mv.to;
                continue;
            }

            let mut best_idx = 0usize;
            let mut best_eval = f64::NEG_INFINITY;
            for i in 0..len {
                let e = moves[i];
                let noise = if params.noise > 0.0 {
                    params.noise * (rng.next_f64() - 0.5)
                } else {
                    0.0
                };
                let eval = self.eval_move(e.to, &mut state.used_tile, params) + noise;
                if eval > best_eval {
                    best_eval = eval;
                    best_idx = i;
                }
            }

            let chosen = moves[best_idx];
            let tile = self.tile_of[chosen.to];
            state.used_tile[tile] = true;
            state.score += self.value[chosen.to];
            state.path.push(chosen.dir);
            state.current = chosen.to;
        }

        state
    }

    fn solve_endgame(&mut self, current: usize, used_tile: &mut [bool], deadline: Instant) -> Vec<u8> {
        let mut best_gain = -1i32;
        let mut cur_path = Vec::new();
        let mut best_path = Vec::new();
        self.dfs_endgame(
            current,
            used_tile,
            0,
            &mut cur_path,
            &mut best_path,
            &mut best_gain,
            deadline,
        );
        best_path
    }

    fn dfs_endgame(
        &mut self,
        current: usize,
        used_tile: &mut [bool],
        gain: i32,
        cur_path: &mut Vec<u8>,
        best_path: &mut Vec<u8>,
        best_gain: &mut i32,
        deadline: Instant,
    ) {
        if Instant::now() >= deadline {
            return;
        }

        let upper = gain + self.component_stats(current, used_tile).tile_sum;
        if upper <= *best_gain {
            return;
        }

        let mut moves = [Edge {
            to: INVALID,
            dir: b'?',
        }; 4];
        let len = self.legal_moves(current, used_tile, &mut moves);
        if len == 0 {
            if gain > *best_gain {
                *best_gain = gain;
                *best_path = cur_path.clone();
            }
            return;
        }

        let mut order = [0usize, 1, 2, 3];
        order[..len].sort_by(|&a, &b| {
            let va = self.value[moves[a].to];
            let vb = self.value[moves[b].to];
            vb.cmp(&va)
        });

        for &idx in &order[..len] {
            let mv = moves[idx];
            let tile = self.tile_of[mv.to];
            used_tile[tile] = true;
            cur_path.push(mv.dir);
            self.dfs_endgame(
                mv.to,
                used_tile,
                gain + self.value[mv.to],
                cur_path,
                best_path,
                best_gain,
                deadline,
            );
            cur_path.pop();
            used_tile[tile] = false;
        }
    }
}

#[derive(Clone, Copy)]
struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let mut x = seed + 0x9E3779B97F4A7C15;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^= x >> 31;
        Self { x: x.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.x = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn gen_range(&mut self, l: usize, r: usize) -> usize {
        if l + 1 >= r {
            return l;
        }
        l + (self.next_u64() as usize % (r - l))
    }

    fn gen_bool(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

fn rc_to_idx(r: usize, c: usize) -> usize {
    r * N + c
}

fn step_cell(cell: usize, dir: u8) -> usize {
    let r = cell / N;
    let c = cell % N;
    match dir {
        b'U' => rc_to_idx(r - 1, c),
        b'D' => rc_to_idx(r + 1, c),
        b'L' => rc_to_idx(r, c - 1),
        b'R' => rc_to_idx(r, c + 1),
        _ => unreachable!(),
    }
}

fn base_params() -> [Params; 3] {
    [
        Params {
            w_pot: 0.82,
            w_deg: 9.0,
            w_side: 0.10,
            noise: 0.0,
        },
        Params {
            w_pot: 0.90,
            w_deg: 12.0,
            w_side: 0.18,
            noise: 4.0,
        },
        Params {
            w_pot: 0.76,
            w_deg: 18.0,
            w_side: 0.05,
            noise: 8.0,
        },
    ]
}

fn sample_params(rng: &mut XorShift64, focused: bool) -> Params {
    let (pot_lo, pot_hi) = if focused { (0.74, 0.96) } else { (0.68, 1.02) };
    let (deg_lo, deg_hi) = if focused { (6.0, 18.0) } else { (2.0, 22.0) };
    let (side_lo, side_hi) = if focused { (0.0, 0.30) } else { (0.0, 0.45) };
    let (noise_lo, noise_hi) = if focused { (1.0, 9.0) } else { (0.0, 14.0) };
    Params {
        w_pot: pot_lo + (pot_hi - pot_lo) * rng.next_f64(),
        w_deg: deg_lo + (deg_hi - deg_lo) * rng.next_f64(),
        w_side: side_lo + (side_hi - side_lo) * rng.next_f64(),
        noise: noise_lo + (noise_hi - noise_lo) * rng.next_f64(),
    }
}

fn choose_cut(path_len: usize, rng: &mut XorShift64) -> usize {
    if path_len <= 4 {
        return 0;
    }
    let tail = path_len.saturating_sub(rng.gen_range(1, (path_len / 3).max(2)));
    let mid = rng.gen_range(path_len / 3, path_len);
    let early = rng.gen_range(0, path_len);
    if rng.gen_bool(0.60) {
        tail
    } else if rng.gen_bool(0.70) {
        mid
    } else {
        early
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut it = input.split_whitespace();

    let si: usize = it.next().unwrap().parse().unwrap();
    let sj: usize = it.next().unwrap().parse().unwrap();
    let start = rc_to_idx(si, sj);

    let mut tile_of = vec![0usize; NN];
    let mut max_tile = 0usize;
    for i in 0..NN {
        let t: usize = it.next().unwrap().parse().unwrap();
        tile_of[i] = t;
        max_tile = max_tile.max(t);
    }
    let tile_count = max_tile + 1;

    let mut value = vec![0i32; NN];
    for i in 0..NN {
        value[i] = it.next().unwrap().parse().unwrap();
    }

    let mut tile_cells = vec![[INVALID; 2]; tile_count];
    let mut tile_size = vec![0usize; tile_count];
    let mut tile_best = vec![0i32; tile_count];
    for cell in 0..NN {
        let t = tile_of[cell];
        let idx = tile_size[t];
        tile_cells[t][idx] = cell;
        tile_size[t] += 1;
        tile_best[t] = tile_best[t].max(value[cell]);
    }

    let mut partner = vec![INVALID; NN];
    for t in 0..tile_count {
        if tile_size[t] == 2 {
            let a = tile_cells[t][0];
            let b = tile_cells[t][1];
            partner[a] = b;
            partner[b] = a;
        }
    }

    let mut edges = vec![[Edge {
        to: INVALID,
        dir: b'?',
    }; 4]; NN];
    let mut deg = vec![0u8; NN];
    for r in 0..N {
        for c in 0..N {
            let v = rc_to_idx(r, c);
            let t = tile_of[v];
            let mut d = 0usize;
            if r > 0 {
                let to = rc_to_idx(r - 1, c);
                if tile_of[to] != t {
                    edges[v][d] = Edge { to, dir: b'U' };
                    d += 1;
                }
            }
            if r + 1 < N {
                let to = rc_to_idx(r + 1, c);
                if tile_of[to] != t {
                    edges[v][d] = Edge { to, dir: b'D' };
                    d += 1;
                }
            }
            if c > 0 {
                let to = rc_to_idx(r, c - 1);
                if tile_of[to] != t {
                    edges[v][d] = Edge { to, dir: b'L' };
                    d += 1;
                }
            }
            if c + 1 < N {
                let to = rc_to_idx(r, c + 1);
                if tile_of[to] != t {
                    edges[v][d] = Edge { to, dir: b'R' };
                    d += 1;
                }
            }
            deg[v] = d as u8;
        }
    }

    let mut solver = Solver::new(
        start,
        tile_of,
        value,
        tile_best,
        partner,
        edges,
        deg,
        tile_count,
    );

    let seed = {
        let mut x = 0x1234_5678_9ABC_DEF0u64;
        x ^= (si as u64) << 1;
        x ^= (sj as u64) << 9;
        for i in (0..NN).step_by(31) {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            x ^= solver.tile_of[i] as u64;
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            x ^= solver.value[i] as u64;
        }
        x
    };
    let mut rng = XorShift64::new(seed);

    let start_state = State {
        current: start,
        score: solver.value[start],
        used_tile: {
            let mut used = vec![false; solver.tile_best.len()];
            used[solver.tile_of[start]] = true;
            used
        },
        path: Vec::with_capacity(tile_count),
    };

    let deadline = Instant::now() + Duration::from_millis(TIME_LIMIT_MS);
    let mut best_state = start_state.clone();

    for params in base_params() {
        if Instant::now() >= deadline {
            break;
        }
        let cand = solver.finish_greedy(start_state.clone(), &mut rng, params, deadline);
        if cand.score > best_state.score {
            best_state = cand;
        }
    }

    while Instant::now() < deadline {
        let focused = !best_state.path.is_empty() && rng.gen_bool(0.70);
        let params = sample_params(&mut rng, focused);
        let base_state = if focused {
            let cut = choose_cut(best_state.path.len(), &mut rng);
            solver.replay_prefix(&best_state.path[..cut])
        } else {
            start_state.clone()
        };
        let cand = solver.finish_greedy(base_state, &mut rng, params, deadline);
        if cand.score > best_state.score {
            best_state = cand;
        }
    }

    println!("{}", String::from_utf8(best_state.path).unwrap());
}
