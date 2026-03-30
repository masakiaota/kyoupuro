// v301_reachability_multistart.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const H: usize = 50;
const W: usize = 50;
const N: usize = H * W;
const BIT_WORDS: usize = (N + 63) / 64;
const TIME_LIMIT_SEC: f64 = 1.95;

#[derive(Clone, Copy)]
struct BeamParams {
    width: usize,
    w_out1: f64,
    w_out2: f64,
    dead_end_penalty: f64,
    w_noise: f64,
}

#[derive(Clone, Copy)]
struct TraceNode {
    parent: u32,
    dir: u8,
}

#[derive(Clone)]
struct State {
    pos: usize,
    score: i32,
    est: f64,
    visited: [u64; BIT_WORDS],
    trace_idx: u32,
}

struct BeamResult {
    score: i32,
    path: Vec<u8>,
}

#[inline(always)]
fn idx(i: usize, j: usize) -> usize {
    i * W + j
}

#[inline(always)]
fn is_visited(visited: &[u64; BIT_WORDS], tile: usize) -> bool {
    (visited[tile >> 6] >> (tile & 63)) & 1 == 1
}

#[inline(always)]
fn set_visited(visited: &mut [u64; BIT_WORDS], tile: usize) {
    visited[tile >> 6] |= 1u64 << (tile & 63);
}

#[inline(always)]
fn out_degree(
    pos: usize,
    visited: &[u64; BIT_WORDS],
    tile_of: &[usize],
    adj_to: &[[usize; 4]],
    adj_len: &[u8],
) -> i32 {
    let mut deg = 0_i32;
    let cur_tile = tile_of[pos];
    for k in 0..adj_len[pos] as usize {
        let v = adj_to[pos][k];
        let tv = tile_of[v];
        if tv != cur_tile && !is_visited(visited, tv) {
            deg += 1;
        }
    }
    deg
}

#[inline(always)]
fn second_degree(
    pos: usize,
    visited: &[u64; BIT_WORDS],
    tile_of: &[usize],
    adj_to: &[[usize; 4]],
    adj_len: &[u8],
) -> i32 {
    let mut val = 0_i32;
    let cur_tile = tile_of[pos];

    for k in 0..adj_len[pos] as usize {
        let mid = adj_to[pos][k];
        let mt = tile_of[mid];
        if mt == cur_tile || is_visited(visited, mt) {
            continue;
        }

        let mut inner = 0_i32;
        for kk in 0..adj_len[mid] as usize {
            let to = adj_to[mid][kk];
            let tt = tile_of[to];
            if tt != mt && !is_visited(visited, tt) {
                inner += 1;
            }
        }
        val += inner;
    }

    val
}

fn sample_beam_params(rng: &mut ChaCha8Rng, progress: f64) -> BeamParams {
    let mut p = match rng.random_range(0..4) {
        0 => BeamParams {
            width: 160,
            w_out1: 12.0,
            w_out2: 3.0,
            dead_end_penalty: -1800.0,
            w_noise: 12.0,
        },
        1 => BeamParams {
            width: 220,
            w_out1: 10.0,
            w_out2: 4.0,
            dead_end_penalty: -2200.0,
            w_noise: 8.0,
        },
        2 => BeamParams {
            width: 280,
            w_out1: 8.0,
            w_out2: 5.0,
            dead_end_penalty: -2500.0,
            w_noise: 6.0,
        },
        _ => BeamParams {
            width: 120,
            w_out1: 14.0,
            w_out2: 2.0,
            dead_end_penalty: -1600.0,
            w_noise: 15.0,
        },
    };

    let width_scale = (1.0 + 0.7 * progress).clamp(1.0, 1.7);
    p.width = ((p.width as f64) * width_scale) as usize;

    let noise_scale = (1.0 - 0.8 * progress).max(0.15);
    p.w_noise *= noise_scale;

    p.w_out1 *= rng.random_range(0.9..1.15);
    p.w_out2 *= rng.random_range(0.9..1.15);
    p.dead_end_penalty *= rng.random_range(0.9..1.15);
    p
}

fn reconstruct_path(history: &[Vec<TraceNode>], depth: usize, trace_idx: u32) -> Vec<u8> {
    let mut path = vec![b'?'; depth];
    let mut cur_idx = trace_idx as usize;

    for d in (1..=depth).rev() {
        let node = history[d][cur_idx];
        path[d - 1] = node.dir;
        cur_idx = node.parent as usize;
    }

    path
}

#[allow(clippy::too_many_arguments)]
fn beam_search(
    start: usize,
    score_of: &[i32],
    tile_of: &[usize],
    adj_to: &[[usize; 4]],
    adj_dir: &[[u8; 4]],
    adj_len: &[u8],
    rng: &mut ChaCha8Rng,
    params: BeamParams,
    run_deadline: f64,
    timer_start: &Instant,
) -> BeamResult {
    let mut root_visited = [0u64; BIT_WORDS];
    set_visited(&mut root_visited, tile_of[start]);

    let root = State {
        pos: start,
        score: score_of[start],
        est: score_of[start] as f64,
        visited: root_visited,
        trace_idx: 0,
    };

    let mut cur_states = vec![root];
    let mut history: Vec<Vec<TraceNode>> = vec![vec![TraceNode {
        parent: 0,
        dir: b'?',
    }]];

    let mut best_score = score_of[start];
    let mut best_depth = 0usize;
    let mut best_trace_idx = 0u32;
    let mut depth = 0usize;

    while !cur_states.is_empty() {
        if timer_start.elapsed().as_secs_f64() >= run_deadline {
            break;
        }

        depth += 1;
        let mut next_states: Vec<State> = Vec::with_capacity(cur_states.len() * 3);
        let mut next_trace: Vec<TraceNode> = Vec::with_capacity(cur_states.len() * 3);

        for st in &cur_states {
            for k in 0..adj_len[st.pos] as usize {
                let v = adj_to[st.pos][k];
                let tv = tile_of[v];
                if is_visited(&st.visited, tv) {
                    continue;
                }

                let mut vis = st.visited;
                set_visited(&mut vis, tv);

                let next_score = st.score + score_of[v];
                let out1 = out_degree(v, &vis, tile_of, adj_to, adj_len);
                let out2 = second_degree(v, &vis, tile_of, adj_to, adj_len);

                let mut est = next_score as f64
                    + params.w_out1 * out1 as f64
                    + params.w_out2 * out2 as f64
                    + (rng.random::<f64>() - 0.5) * params.w_noise;

                if out1 == 0 {
                    est += params.dead_end_penalty;
                }

                let trace_idx = next_trace.len() as u32;
                next_trace.push(TraceNode {
                    parent: st.trace_idx,
                    dir: adj_dir[st.pos][k],
                });

                next_states.push(State {
                    pos: v,
                    score: next_score,
                    est,
                    visited: vis,
                    trace_idx,
                });
            }
        }

        history.push(next_trace);

        if next_states.is_empty() {
            break;
        }

        next_states.sort_unstable_by(|a, b| b.est.total_cmp(&a.est));
        if next_states.len() > params.width {
            next_states.truncate(params.width);
        }

        for st in &next_states {
            if st.score > best_score {
                best_score = st.score;
                best_depth = depth;
                best_trace_idx = st.trace_idx;
            }
        }

        cur_states = next_states;
    }

    BeamResult {
        score: best_score,
        path: reconstruct_path(&history, best_depth, best_trace_idx),
    }
}

fn main() {
    input! {
        si: usize,
        sj: usize,
        t: [[usize; W]; H],
        p: [[i32; W]; H],
    }

    let start = idx(si, sj);

    let mut tile_of = vec![0usize; N];
    let mut score_of = vec![0i32; N];

    for i in 0..H {
        for j in 0..W {
            let u = idx(i, j);
            tile_of[u] = t[i][j];
            score_of[u] = p[i][j];
        }
    }

    let mut adj_to = vec![[usize::MAX; 4]; N];
    let mut adj_dir = vec![[b'?'; 4]; N];
    let mut adj_len = vec![0u8; N];

    for i in 0..H {
        for j in 0..W {
            let u = idx(i, j);
            if i > 0 {
                let k = adj_len[u] as usize;
                adj_to[u][k] = idx(i - 1, j);
                adj_dir[u][k] = b'U';
                adj_len[u] += 1;
            }
            if i + 1 < H {
                let k = adj_len[u] as usize;
                adj_to[u][k] = idx(i + 1, j);
                adj_dir[u][k] = b'D';
                adj_len[u] += 1;
            }
            if j > 0 {
                let k = adj_len[u] as usize;
                adj_to[u][k] = idx(i, j - 1);
                adj_dir[u][k] = b'L';
                adj_len[u] += 1;
            }
            if j + 1 < W {
                let k = adj_len[u] as usize;
                adj_to[u][k] = idx(i, j + 1);
                adj_dir[u][k] = b'R';
                adj_len[u] += 1;
            }
        }
    }

    let mut seed = (start as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for u in 0..N {
        let mix = ((tile_of[u] as u64) << 32) ^ (score_of[u] as u64);
        seed ^= mix.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        seed = seed.rotate_left(11) ^ 0x94D0_49BB_1331_11EB;
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let timer_start = Instant::now();
    let mut best_score = i32::MIN;
    let mut best_path: Vec<u8> = Vec::new();

    while timer_start.elapsed().as_secs_f64() < TIME_LIMIT_SEC {
        let elapsed = timer_start.elapsed().as_secs_f64();
        let progress = (elapsed / TIME_LIMIT_SEC).min(1.0);
        let remain = TIME_LIMIT_SEC - elapsed;
        if remain <= 0.002 {
            break;
        }

        let params = sample_beam_params(&mut rng, progress);
        let mut budget = 0.22 - 0.10 * progress;
        if budget < 0.05 {
            budget = 0.05;
        }
        let run_deadline = (elapsed + budget).min(TIME_LIMIT_SEC - 0.001);

        let result = beam_search(
            start,
            &score_of,
            &tile_of,
            &adj_to,
            &adj_dir,
            &adj_len,
            &mut rng,
            params,
            run_deadline,
            &timer_start,
        );

        if result.score > best_score {
            best_score = result.score;
            best_path = result.path;
        }
    }

    let ans = String::from_utf8(best_path).unwrap_or_default();
    println!("{}", ans);
}
