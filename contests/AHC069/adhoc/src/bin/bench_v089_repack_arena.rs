// bench_v089_repack_arena.rs
#![allow(non_snake_case)]

use std::hint::black_box;
use std::time::Instant;

const MAX_N: usize = 50;
const MAX_DEPTH: usize = 4;
const BEAM_WIDTH: usize = 9;
const BRANCHES: usize = 4;
type Rows = [u64; MAX_N];
type RunTable = [[u64; MAX_N + 1]; MAX_N];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Geometry {
    Regular {
        x: usize,
        y: usize,
        h: usize,
        w: usize,
    },
    Explicit(Vec<usize>),
}

#[derive(Clone, Debug)]
struct Candidate {
    id: u64,
    score: f64,
    geometry: Geometry,
}

#[derive(Clone)]
struct OldState {
    occ: Rows,
    path: Vec<(u64, Vec<usize>)>,
    score: f64,
}

#[derive(Clone, Copy)]
struct NewState {
    occ: Rows,
    path: [usize; MAX_DEPTH],
    depth: u8,
    score: f64,
}

#[inline(always)]
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn rows_hash(rows: &Rows, N: usize) -> u64 {
    let mut hash = 0x9e37_79b9_7f4a_7c15;
    for (r, &row) in rows[..N].iter().enumerate() {
        hash = mix(hash ^ row ^ (r as u64).wrapping_mul(0x1000_0000_01b3));
    }
    hash
}

fn candidates(seed: u64, depth: usize, occ: &Rows, N: usize) -> Vec<Candidate> {
    let base = mix(seed ^ rows_hash(occ, N) ^ ((depth as u64) << 56));
    let count = 1 + base as usize % BRANCHES;
    let mut result = Vec::with_capacity(count);
    for k in 0..count {
        let value = mix(base ^ (k as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let geometry = if value & 3 != 0 {
            let h = 1 + (value as usize >> 8) % 4;
            let w = 1 + (value as usize >> 16) % 4;
            let x = (value as usize >> 24) % (N - h + 1);
            let y = (value as usize >> 32) % (N - w + 1);
            Geometry::Regular { x, y, h, w }
        } else {
            let len = 1 + (value as usize >> 8) % 8;
            let mut cells = Vec::with_capacity(len);
            let mut state = value;
            for _ in 0..len {
                state = mix(state);
                cells.push(state as usize % (N * N));
            }
            Geometry::Explicit(cells)
        };
        result.push(Candidate {
            id: value,
            // 有限な二進小数にして同点を多く作り、stable sortの順序も照合する。
            score: ((value >> 40) % 41) as f64 * 0.25,
            geometry,
        });
    }
    result
}

fn materialize(geometry: &Geometry, N: usize) -> Vec<usize> {
    match geometry {
        Geometry::Regular { x, y, h, w } => {
            let mut cells = Vec::with_capacity(h * w);
            for r in *x..(*x + *h) {
                for c in *y..(*y + *w) {
                    cells.push(r * N + c);
                }
            }
            cells
        }
        Geometry::Explicit(cells) => cells.clone(),
    }
}

fn apply_cells(rows: &mut Rows, cells: &[usize], N: usize) {
    for &cell in cells {
        rows[cell / N] |= 1_u64 << (cell % N);
    }
}

fn apply_direct(rows: &mut Rows, geometry: &Geometry, N: usize) {
    match geometry {
        Geometry::Regular { x, y, h, w } => {
            let mask = ((1_u64 << *w) - 1) << *y;
            for row in &mut rows[*x..(*x + *h)] {
                *row |= mask;
            }
        }
        Geometry::Explicit(cells) => apply_cells(rows, cells, N),
    }
}

fn metric(rows: &Rows, N: usize) -> f64 {
    let mut value = 0_u64;
    for (r, &row) in rows[..N].iter().enumerate() {
        value += row.count_ones() as u64 * (1 + r as u64 % 7);
    }
    value as f64
}

fn old_run_table(N: usize, grass: &Rows, occ: &Rows) -> RunTable {
    let mut runs = [[0_u64; MAX_N + 1]; MAX_N];
    for r in 0..N {
        let free = grass[r] & !occ[r];
        runs[r][1] = free;
        for len in 2..=N {
            runs[r][len] = runs[r][len - 1] & (free >> (len - 1));
        }
    }
    runs
}

fn reused_run_table(N: usize, grass: &Rows, occ: &Rows, runs: &mut RunTable) {
    for r in 0..N {
        let free = grass[r] & !occ[r];
        runs[r][1] = free;
        for len in 2..=N {
            runs[r][len] = runs[r][len - 1] & (free >> (len - 1));
        }
    }
}

fn run_old(seed: u64, N: usize, depth_limit: usize) -> (u64, Rows, Vec<(u64, Vec<usize>)>) {
    let mut beam = vec![OldState {
        occ: [0; MAX_N],
        path: Vec::new(),
        score: 0.0,
    }];
    for depth in 0..depth_limit {
        let mut next = Vec::with_capacity(BEAM_WIDTH * BRANCHES);
        for state in &beam {
            for candidate in candidates(seed, depth, &state.occ, N) {
                let mut child = state.clone();
                let cells = materialize(&candidate.geometry, N);
                apply_cells(&mut child.occ, &cells, N);
                child.score += candidate.score;
                child.path.push((candidate.id, cells));
                next.push(child);
            }
        }
        next.sort_by(|a, b| b.score.total_cmp(&a.score));
        next.truncate(BEAM_WIDTH);
        beam = next;
    }
    let mut best = 0;
    let mut best_score = -1e100;
    for (index, state) in beam.iter().enumerate() {
        let score = state.score - 1.15 * metric(&state.occ, N);
        if score > best_score {
            best = index;
            best_score = score;
        }
    }
    let state = beam.swap_remove(best);
    (best_score.to_bits(), state.occ, state.path)
}

fn run_new(seed: u64, N: usize, depth_limit: usize) -> (u64, Rows, Vec<(u64, Vec<usize>)>) {
    let mut beam = Vec::with_capacity(BEAM_WIDTH * BRANCHES);
    let mut next = Vec::with_capacity(BEAM_WIDTH * BRANCHES);
    let mut arena: Vec<(u64, Geometry)> = Vec::with_capacity(MAX_DEPTH * BEAM_WIDTH * BRANCHES);
    beam.push(NewState {
        occ: [0; MAX_N],
        path: [0; MAX_DEPTH],
        depth: 0,
        score: 0.0,
    });
    for depth in 0..depth_limit {
        next.clear();
        for &state in &beam {
            for candidate in candidates(seed, depth, &state.occ, N) {
                let mut child_occ = state.occ;
                apply_direct(&mut child_occ, &candidate.geometry, N);
                let arena_index = arena.len();
                arena.push((candidate.id, candidate.geometry));
                let mut path = state.path;
                path[state.depth as usize] = arena_index;
                next.push(NewState {
                    occ: child_occ,
                    path,
                    depth: state.depth + 1,
                    score: state.score + candidate.score,
                });
            }
        }
        next.sort_by(|a, b| b.score.total_cmp(&a.score));
        next.truncate(BEAM_WIDTH);
        std::mem::swap(&mut beam, &mut next);
    }
    let mut best = 0;
    let mut best_score = -1e100;
    for (index, state) in beam.iter().enumerate() {
        let score = state.score - 1.15 * metric(&state.occ, N);
        if score > best_score {
            best = index;
            best_score = score;
        }
    }
    let state = beam[best];
    let path = state.path[..state.depth as usize]
        .iter()
        .map(|&index| {
            let (id, geometry) = &arena[index];
            (*id, materialize(geometry, N))
        })
        .collect();
    (best_score.to_bits(), state.occ, path)
}

fn main() {
    let mut state = 0x1234_5678_9abc_def0;
    for case in 0..20_000 {
        state = mix(state);
        let N = 20 + state as usize % 31;
        let depth = 1 + (state as usize >> 8) % MAX_DEPTH;
        let old = run_old(state, N, depth);
        let new = run_new(state, N, depth);
        assert_eq!(old, new, "beam case {case}");
    }

    // 規則形状の直接mask反映だけも広い座標・寸法で照合する。
    for case in 0..500_000 {
        state = mix(state);
        let N = 20 + state as usize % 31;
        let h = 1 + (state as usize >> 8) % 8;
        let w = 1 + (state as usize >> 16) % 8;
        let x = (state as usize >> 24) % (N - h + 1);
        let y = (state as usize >> 32) % (N - w + 1);
        let geometry = Geometry::Regular { x, y, h, w };
        let mut old = [0_u64; MAX_N];
        let mut new = old;
        apply_cells(&mut old, &materialize(&geometry, N), N);
        apply_direct(&mut new, &geometry, N);
        assert_eq!(old, new, "mask case {case}");
    }

    // dirtyな再利用tableでも、探索が参照するN×(1..=N)は毎回完全上書きされる。
    let mut reused = [[u64::MAX; MAX_N + 1]; MAX_N];
    for case in 0..20_000 {
        state = mix(state);
        let N = 20 + state as usize % 31;
        let mask = (1_u64 << N) - 1;
        let mut grass = [0_u64; MAX_N];
        let mut occ = [0_u64; MAX_N];
        for r in 0..N {
            state = mix(state);
            grass[r] = state & mask;
            state = mix(state);
            occ[r] = state & grass[r];
        }
        let expected = old_run_table(N, &grass, &occ);
        reused_run_table(N, &grass, &occ, &mut reused);
        for r in 0..N {
            assert_eq!(
                expected[r][1..=N],
                reused[r][1..=N],
                "run table case {case} row {r}"
            );
        }
    }

    let rounds = 2_000;
    let start = Instant::now();
    let mut old_sum = 0_u64;
    for case in 0..rounds {
        old_sum ^= black_box(run_old(case as u64 + 1, 50, MAX_DEPTH).0);
    }
    let old_elapsed = start.elapsed();
    let start = Instant::now();
    let mut new_sum = 0_u64;
    for case in 0..rounds {
        new_sum ^= black_box(run_new(case as u64 + 1, 50, MAX_DEPTH).0);
    }
    let new_elapsed = start.elapsed();
    assert_eq!(old_sum, new_sum);
    println!("verified_beam_trees={}", 20_000);
    println!("verified_direct_masks={}", 500_000);
    println!("verified_reused_run_tables={}", 20_000);
    println!(
        "old_tree_ns={:.1}",
        old_elapsed.as_nanos() as f64 / rounds as f64
    );
    println!(
        "new_tree_ns={:.1}",
        new_elapsed.as_nanos() as f64 / rounds as f64
    );
    println!(
        "tree_speedup={:.3}",
        old_elapsed.as_secs_f64() / new_elapsed.as_secs_f64()
    );
}
