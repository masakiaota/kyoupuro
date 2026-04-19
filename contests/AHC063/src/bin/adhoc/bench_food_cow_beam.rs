// bench_food_cow_beam.rs
use std::env;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rustc_hash::FxHasher;

const DEFAULT_INPUT_COUNT: usize = 3;
const DEFAULT_MEASURE_MS: u64 = 180;
const DEFAULT_ROUNDS: usize = 24;
const BRANCHES: usize = 3;
const CELL_COUNTS: [usize; 4] = [6 * 6, 8 * 8, 12 * 12, 16 * 16];
const DENSITY_PCTS: [u8; 2] = [15, 35];
const BEAM_WIDTHS: [usize; 4] = [64, 256, 1024, 4096];

#[derive(Clone, Copy)]
struct Update {
    idx: u16,
    color: u8,
}

#[derive(Clone, Copy)]
struct Action {
    kind: u8, // 0: no change, 1: one write, 2: four writes
    updates: [Update; 4],
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[derive(Clone)]
struct VecFood {
    data: Vec<u8>,
}

#[derive(Clone)]
struct CowFood {
    data: Rc<Vec<u8>>,
}

impl CowFood {
    #[inline]
    fn from_vec(data: Vec<u8>) -> Self {
        Self { data: Rc::new(data) }
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    #[inline]
    fn set(&mut self, idx: usize, color: u8) -> u8 {
        let data = Rc::make_mut(&mut self.data);
        let old = data[idx];
        data[idx] = color;
        old
    }
}

#[derive(Clone)]
struct VecState {
    food: VecFood,
    score: u64,
}

#[derive(Clone)]
struct CowState {
    food: CowFood,
    score: u64,
}

#[derive(Clone)]
struct BenchInput {
    init_states: Vec<Vec<u8>>,
    actions: Vec<Action>,
}

#[derive(Clone, Copy)]
enum Workload {
    ExpandNoChange,
    ExpandMixed,
    ExpandMixedHashScan,
}

impl Workload {
    fn all() -> [Self; 3] {
        [Self::ExpandNoChange, Self::ExpandMixed, Self::ExpandMixedHashScan]
    }

    fn name(self) -> &'static str {
        match self {
            Self::ExpandNoChange => "expand_no_change",
            Self::ExpandMixed => "expand_mixed",
            Self::ExpandMixedHashScan => "expand_mixed_hash_scan",
        }
    }
}

#[derive(Default, Clone, Copy)]
struct BenchAccum {
    elapsed_ns: f64,
    ops: u64,
}

impl BenchAccum {
    fn add(&mut self, other: BenchAccum) {
        self.elapsed_ns += other.elapsed_ns;
        self.ops += other.ops;
    }

    fn ns_per_op(self) -> f64 {
        self.elapsed_ns / self.ops as f64
    }
}

#[inline]
fn scan_nonzero_sum(food: &[u8]) -> u64 {
    let mut acc = 0_u64;
    for (idx, &color) in food.iter().enumerate() {
        if color != 0 {
            acc = acc
                .wrapping_add((idx as u64 + 1) * color as u64)
                .rotate_left(7);
        }
    }
    acc
}

fn make_input(cell_count: usize, density_pct: u8, beam_width: usize, rounds: usize, seed: u64) -> BenchInput {
    let mut init_states = Vec::with_capacity(beam_width);
    for parent_idx in 0..beam_width {
        let mut board = vec![0_u8; cell_count];
        let seed_base = splitmix64(seed ^ parent_idx as u64);
        let mut idx = 0;
        while idx < cell_count {
            let x = splitmix64(seed_base ^ idx as u64);
            if (x % 100) < density_pct as u64 {
                board[idx] = ((x >> 8) % 7) as u8 + 1;
            }
            idx += 1;
        }
        init_states.push(board);
    }

    let mut actions = Vec::with_capacity(rounds * beam_width * BRANCHES);
    for round in 0..rounds {
        for parent_idx in 0..beam_width {
            for branch_idx in 0..BRANCHES {
                let x = splitmix64(
                    seed
                        ^ ((round as u64) << 32)
                        ^ ((parent_idx as u64) << 8)
                        ^ branch_idx as u64,
                );
                let bucket = (x % 100) as u8;
                let kind = if bucket < 65 {
                    0
                } else if bucket < 90 {
                    1
                } else {
                    2
                };
                let mut updates = [Update { idx: 0, color: 0 }; 4];
                let mut k = 0;
                while k < 4 {
                    let y = splitmix64(x ^ (k as u64) ^ 0xD1B5_4A32_D192_ED03);
                    updates[k] = Update {
                        idx: (y as usize % cell_count) as u16,
                        color: ((y >> 10) % 8) as u8,
                    };
                    k += 1;
                }
                actions.push(Action { kind, updates });
            }
        }
    }

    BenchInput {
        init_states,
        actions,
    }
}

#[inline]
fn apply_action_vec(food: &mut Vec<u8>, action: Action) {
    match action.kind {
        0 => {}
        1 => {
            let update = action.updates[0];
            food[update.idx as usize] = update.color;
        }
        2 => {
            for &update in &action.updates {
                food[update.idx as usize] = update.color;
            }
        }
        _ => unreachable!(),
    }
}

#[inline]
fn apply_action_cow(food: &mut CowFood, action: Action) {
    match action.kind {
        0 => {}
        1 => {
            let update = action.updates[0];
            let _ = food.set(update.idx as usize, update.color);
        }
        2 => {
            for &update in &action.updates {
                let _ = food.set(update.idx as usize, update.color);
            }
        }
        _ => unreachable!(),
    }
}

fn bench_once_vec(workload: Workload, input: &BenchInput, beam_width: usize, rounds: usize) -> u64 {
    let mut beam = input
        .init_states
        .iter()
        .map(|food| VecState {
            food: VecFood { data: food.clone() },
            score: 0,
        })
        .collect::<Vec<_>>();
    let mut checksum = 0_u64;

    for round in 0..rounds {
        let mut next = Vec::with_capacity(beam_width * BRANCHES);
        for parent_idx in 0..beam_width {
            let parent = &beam[parent_idx];
            let action_base = (round * beam_width + parent_idx) * BRANCHES;
            for branch_idx in 0..BRANCHES {
                let mut child = parent.clone();
                let action = match workload {
                    Workload::ExpandNoChange => Action {
                        kind: 0,
                        updates: [Update { idx: 0, color: 0 }; 4],
                    },
                    _ => input.actions[action_base + branch_idx],
                };
                apply_action_vec(&mut child.food.data, action);
                let update = match workload {
                    Workload::ExpandNoChange => {
                        let first = *child.food.data.first().unwrap_or(&0) as u64;
                        let last = *child.food.data.last().unwrap_or(&0) as u64;
                        first ^ last.rotate_left((branch_idx * 5) as u32)
                    }
                    Workload::ExpandMixed => {
                        let first = *child.food.data.first().unwrap_or(&0) as u64;
                        let last = *child.food.data.last().unwrap_or(&0) as u64;
                        let touched = action.updates[0].idx as u64;
                        first ^ last.rotate_left((branch_idx * 5) as u32) ^ touched
                    }
                    Workload::ExpandMixedHashScan => {
                        let mut hasher = FxHasher::default();
                        child.food.data.hash(&mut hasher);
                        let hash = hasher.finish();
                        let scan = scan_nonzero_sum(&child.food.data);
                        hash ^ scan.rotate_left(13)
                    }
                };
                child.score = parent
                    .score
                    .wrapping_add(update)
                    .wrapping_add((round as u64) << 32)
                    .wrapping_add(parent_idx as u64)
                    .wrapping_add(branch_idx as u64);
                checksum ^= child.score;
                next.push(child);
            }
        }

        let offset = (round * 5 + 1) % BRANCHES;
        beam = next
            .into_iter()
            .skip(offset)
            .step_by(BRANCHES)
            .take(beam_width)
            .collect();
    }

    black_box(checksum)
}

fn bench_once_cow(workload: Workload, input: &BenchInput, beam_width: usize, rounds: usize) -> u64 {
    let mut beam = input
        .init_states
        .iter()
        .map(|food| CowState {
            food: CowFood::from_vec(food.clone()),
            score: 0,
        })
        .collect::<Vec<_>>();
    let mut checksum = 0_u64;

    for round in 0..rounds {
        let mut next = Vec::with_capacity(beam_width * BRANCHES);
        for parent_idx in 0..beam_width {
            let parent = &beam[parent_idx];
            let action_base = (round * beam_width + parent_idx) * BRANCHES;
            for branch_idx in 0..BRANCHES {
                let mut child = parent.clone();
                let action = match workload {
                    Workload::ExpandNoChange => Action {
                        kind: 0,
                        updates: [Update { idx: 0, color: 0 }; 4],
                    },
                    _ => input.actions[action_base + branch_idx],
                };
                apply_action_cow(&mut child.food, action);
                let update = match workload {
                    Workload::ExpandNoChange => {
                        let slice = child.food.as_slice();
                        let first = *slice.first().unwrap_or(&0) as u64;
                        let last = *slice.last().unwrap_or(&0) as u64;
                        first ^ last.rotate_left((branch_idx * 5) as u32)
                    }
                    Workload::ExpandMixed => {
                        let slice = child.food.as_slice();
                        let first = *slice.first().unwrap_or(&0) as u64;
                        let last = *slice.last().unwrap_or(&0) as u64;
                        let touched = action.updates[0].idx as u64;
                        first ^ last.rotate_left((branch_idx * 5) as u32) ^ touched
                    }
                    Workload::ExpandMixedHashScan => {
                        let mut hasher = FxHasher::default();
                        child.food.as_slice().hash(&mut hasher);
                        let hash = hasher.finish();
                        let scan = scan_nonzero_sum(child.food.as_slice());
                        hash ^ scan.rotate_left(13)
                    }
                };
                child.score = parent
                    .score
                    .wrapping_add(update)
                    .wrapping_add((round as u64) << 32)
                    .wrapping_add(parent_idx as u64)
                    .wrapping_add(branch_idx as u64);
                checksum ^= child.score;
                next.push(child);
            }
        }

        let offset = (round * 5 + 1) % BRANCHES;
        beam = next
            .into_iter()
            .skip(offset)
            .step_by(BRANCHES)
            .take(beam_width)
            .collect();
    }

    black_box(checksum)
}

fn measure_vec(
    workload: Workload,
    input: &BenchInput,
    beam_width: usize,
    rounds: usize,
    min_duration: Duration,
) -> BenchAccum {
    let start = Instant::now();
    let mut ops = 0_u64;
    while start.elapsed() < min_duration {
        black_box(bench_once_vec(workload, input, beam_width, rounds));
        ops += (beam_width * BRANCHES * rounds) as u64;
    }
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_cow(
    workload: Workload,
    input: &BenchInput,
    beam_width: usize,
    rounds: usize,
    min_duration: Duration,
) -> BenchAccum {
    let start = Instant::now();
    let mut ops = 0_u64;
    while start.elapsed() < min_duration {
        black_box(bench_once_cow(workload, input, beam_width, rounds));
        ops += (beam_width * BRANCHES * rounds) as u64;
    }
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn getenv_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn getenv_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
}

fn print_header(input_count: usize, rounds: usize, measure_ms: u64) {
    println!(
        "# Food COW beam-style benchmark\n\
         # workload = clone parent state -> optional food update -> optional hash+scan -> materialize next beam\n\
         # cell_counts={:?}, density_pcts={:?}, beam_widths={:?}, rounds={}, inputs_per_config={}, branches={}, measure_ms={}\n",
        CELL_COUNTS, DENSITY_PCTS, BEAM_WIDTHS, rounds, input_count, BRANCHES, measure_ms
    );
    println!(
        "{:<22} {:>6} {:>8} {:>8} {:>16} {:>16} {:>14}",
        "workload", "cells", "dens%", "beam", "vec ns/child", "cow ns/child", "cow/vec"
    );
}

fn main() {
    let input_count = getenv_usize("BENCH_INPUT_COUNT", DEFAULT_INPUT_COUNT);
    let rounds = getenv_usize("BENCH_ROUNDS", DEFAULT_ROUNDS);
    let measure_ms = getenv_u64("BENCH_MEASURE_MS", DEFAULT_MEASURE_MS);
    let min_duration = Duration::from_millis(measure_ms);
    print_header(input_count, rounds, measure_ms);

    for workload in Workload::all() {
        for &cell_count in &CELL_COUNTS {
            for &density_pct in &DENSITY_PCTS {
                for &beam_width in &BEAM_WIDTHS {
                    let mut vec_acc = BenchAccum::default();
                    let mut cow_acc = BenchAccum::default();
                    for input_idx in 0..input_count {
                        let seed = 0x1234_5678_9ABC_DEF0
                            ^ (workload as u64) << 56
                            ^ (cell_count as u64) << 24
                            ^ (density_pct as u64) << 16
                            ^ (beam_width as u64) << 4
                            ^ input_idx as u64;
                        let input = make_input(cell_count, density_pct, beam_width, rounds, seed);
                        if input_idx % 2 == 0 {
                            vec_acc.add(measure_vec(workload, &input, beam_width, rounds, min_duration));
                            cow_acc.add(measure_cow(workload, &input, beam_width, rounds, min_duration));
                        } else {
                            cow_acc.add(measure_cow(workload, &input, beam_width, rounds, min_duration));
                            vec_acc.add(measure_vec(workload, &input, beam_width, rounds, min_duration));
                        }
                    }
                    let vec_ns = vec_acc.ns_per_op();
                    let cow_ns = cow_acc.ns_per_op();
                    println!(
                        "{:<22} {:>6} {:>8} {:>8} {:>16.3} {:>16.3} {:>14.3}",
                        workload.name(),
                        cell_count,
                        density_pct,
                        beam_width,
                        vec_ns,
                        cow_ns,
                        cow_ns / vec_ns
                    );
                }
                println!();
            }
            println!();
        }
        println!();
    }
}
