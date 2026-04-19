// bench_food_cow.rs
use std::env;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rustc_hash::FxHasher;

const DEFAULT_INPUT_COUNT: usize = 4;
const DEFAULT_CLONE_MS: u64 = 70;
const DEFAULT_HASH_MS: u64 = 120;
const DEFAULT_SCAN_MS: u64 = 90;
const DEFAULT_COMBINED_MS: u64 = 120;
const PROBE_COUNT: usize = 4096;
const CELL_COUNTS: [usize; 4] = [6 * 6, 8 * 8, 12 * 12, 16 * 16];
const DENSITY_PCTS: [u8; 3] = [10, 35, 60];

#[derive(Clone, Copy)]
struct Update {
    idx: u16,
    color: u8,
}

type Update4 = [Update; 4];

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
struct BenchInput {
    vec_food: VecFood,
    cow_food: CowFood,
    updates: Vec<Update4>,
}

#[derive(Clone, Copy)]
enum Workload {
    CloneOnly,
    CloneRead1,
    CloneWrite1,
    CloneWrite4,
    HashOnly,
    CloneHash,
    ScanNonZero,
    CloneWrite4HashScan,
}

impl Workload {
    fn all() -> [Self; 8] {
        [
            Self::CloneOnly,
            Self::CloneRead1,
            Self::CloneWrite1,
            Self::CloneWrite4,
            Self::HashOnly,
            Self::CloneHash,
            Self::ScanNonZero,
            Self::CloneWrite4HashScan,
        ]
    }

    fn name(self) -> &'static str {
        match self {
            Self::CloneOnly => "clone_only",
            Self::CloneRead1 => "clone_read1",
            Self::CloneWrite1 => "clone_write1",
            Self::CloneWrite4 => "clone_write4",
            Self::HashOnly => "hash_only",
            Self::CloneHash => "clone_hash",
            Self::ScanNonZero => "scan_nonzero",
            Self::CloneWrite4HashScan => "clone_write4_hash_scan",
        }
    }

    fn min_duration(self, clone_ms: u64, hash_ms: u64, scan_ms: u64, combined_ms: u64) -> Duration {
        match self {
            Self::CloneOnly | Self::CloneRead1 | Self::CloneWrite1 | Self::CloneWrite4 => {
                Duration::from_millis(clone_ms)
            }
            Self::HashOnly | Self::CloneHash => Duration::from_millis(hash_ms),
            Self::ScanNonZero => Duration::from_millis(scan_ms),
            Self::CloneWrite4HashScan => Duration::from_millis(combined_ms),
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

fn make_input(cell_count: usize, density_pct: u8, seed: u64) -> BenchInput {
    let mut data = vec![0_u8; cell_count];
    let mut idx = 0;
    while idx < cell_count {
        let x = splitmix64(seed ^ idx as u64);
        if (x % 100) < density_pct as u64 {
            data[idx] = ((x >> 8) % 7) as u8 + 1;
        }
        idx += 1;
    }

    let mut updates = Vec::with_capacity(PROBE_COUNT);
    for probe_idx in 0..PROBE_COUNT {
        let mut batch = [Update { idx: 0, color: 0 }; 4];
        let mut k = 0;
        while k < 4 {
            let x = splitmix64(seed ^ 0xD00D_BEEF_F00D_CAFE ^ ((probe_idx * 4 + k) as u64));
            batch[k] = Update {
                idx: (x as usize % cell_count) as u16,
                color: ((x >> 10) % 8) as u8,
            };
            k += 1;
        }
        updates.push(batch);
    }

    BenchInput {
        vec_food: VecFood { data: data.clone() },
        cow_food: CowFood::from_vec(data),
        updates,
    }
}

fn measure_vec(workload: Workload, input: &BenchInput, min_duration: Duration) -> BenchAccum {
    const CHUNK: usize = 512;
    let start = Instant::now();
    let mut turn = 0_usize;
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for _ in 0..CHUNK {
            let updates = &input.updates[turn & (PROBE_COUNT - 1)];
            match workload {
                Workload::CloneOnly => {
                    let cloned = black_box(&input.vec_food).clone();
                    acc ^= black_box(*cloned.data.first().unwrap_or(&0) as u64);
                    black_box(cloned);
                }
                Workload::CloneRead1 => {
                    let cloned = black_box(&input.vec_food).clone();
                    let idx = updates[0].idx as usize;
                    acc ^= black_box(cloned.data[idx] as u64);
                    black_box(cloned);
                }
                Workload::CloneWrite1 => {
                    let mut cloned = black_box(&input.vec_food).clone();
                    let update = updates[0];
                    let idx = update.idx as usize;
                    let old = cloned.data[idx];
                    cloned.data[idx] = update.color;
                    acc ^= black_box((old as u64) ^ cloned.data[idx] as u64);
                    black_box(cloned);
                }
                Workload::CloneWrite4 => {
                    let mut cloned = black_box(&input.vec_food).clone();
                    for &update in updates {
                        cloned.data[update.idx as usize] = update.color;
                    }
                    acc ^= black_box(cloned.data[updates[3].idx as usize] as u64);
                    black_box(cloned);
                }
                Workload::HashOnly => {
                    let mut hasher = FxHasher::default();
                    input.vec_food.data.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::CloneHash => {
                    let cloned = black_box(&input.vec_food).clone();
                    let mut hasher = FxHasher::default();
                    cloned.data.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::ScanNonZero => {
                    acc ^= black_box(scan_nonzero_sum(&input.vec_food.data));
                }
                Workload::CloneWrite4HashScan => {
                    let mut cloned = black_box(&input.vec_food).clone();
                    for &update in updates {
                        cloned.data[update.idx as usize] = update.color;
                    }
                    let mut hasher = FxHasher::default();
                    cloned.data.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                    acc ^= black_box(scan_nonzero_sum(&cloned.data));
                }
            }
            ops += 1;
            turn += 1;
        }
    }
    black_box(acc);
    BenchAccum {
        elapsed_ns: start.elapsed().as_secs_f64() * 1e9,
        ops,
    }
}

fn measure_cow(workload: Workload, input: &BenchInput, min_duration: Duration) -> BenchAccum {
    const CHUNK: usize = 512;
    let start = Instant::now();
    let mut turn = 0_usize;
    let mut ops = 0_u64;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for _ in 0..CHUNK {
            let updates = &input.updates[turn & (PROBE_COUNT - 1)];
            match workload {
                Workload::CloneOnly => {
                    let cloned = black_box(&input.cow_food).clone();
                    acc ^= black_box(*cloned.as_slice().first().unwrap_or(&0) as u64);
                    black_box(cloned);
                }
                Workload::CloneRead1 => {
                    let cloned = black_box(&input.cow_food).clone();
                    let idx = updates[0].idx as usize;
                    acc ^= black_box(cloned.as_slice()[idx] as u64);
                    black_box(cloned);
                }
                Workload::CloneWrite1 => {
                    let mut cloned = black_box(&input.cow_food).clone();
                    let update = updates[0];
                    let old = cloned.set(update.idx as usize, update.color);
                    acc ^= black_box((old as u64) ^ cloned.as_slice()[update.idx as usize] as u64);
                    black_box(cloned);
                }
                Workload::CloneWrite4 => {
                    let mut cloned = black_box(&input.cow_food).clone();
                    for &update in updates {
                        let _ = cloned.set(update.idx as usize, update.color);
                    }
                    acc ^= black_box(cloned.as_slice()[updates[3].idx as usize] as u64);
                    black_box(cloned);
                }
                Workload::HashOnly => {
                    let mut hasher = FxHasher::default();
                    input.cow_food.as_slice().hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::CloneHash => {
                    let cloned = black_box(&input.cow_food).clone();
                    let mut hasher = FxHasher::default();
                    cloned.as_slice().hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::ScanNonZero => {
                    acc ^= black_box(scan_nonzero_sum(input.cow_food.as_slice()));
                }
                Workload::CloneWrite4HashScan => {
                    let mut cloned = black_box(&input.cow_food).clone();
                    for &update in updates {
                        let _ = cloned.set(update.idx as usize, update.color);
                    }
                    let mut hasher = FxHasher::default();
                    cloned.as_slice().hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                    acc ^= black_box(scan_nonzero_sum(cloned.as_slice()));
                }
            }
            ops += 1;
            turn += 1;
        }
    }
    black_box(acc);
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

fn print_header(input_count: usize, clone_ms: u64, hash_ms: u64, scan_ms: u64, combined_ms: u64) {
    println!(
        "# Food COW micro benchmark\n\
         # Vec<u8> vs Rc<Vec<u8>> copy-on-write food\n\
         # cell_counts={:?}, density_pcts={:?}, inputs_per_config={}, clone_ms={}, hash_ms={}, scan_ms={}, combined_ms={}, probes={}\n",
        CELL_COUNTS, DENSITY_PCTS, input_count, clone_ms, hash_ms, scan_ms, combined_ms, PROBE_COUNT
    );
    println!(
        "{:<24} {:>6} {:>8} {:>16} {:>16} {:>14}",
        "workload", "cells", "dens%", "vec ns/op", "cow ns/op", "cow/vec"
    );
}

fn main() {
    let input_count = getenv_usize("BENCH_INPUT_COUNT", DEFAULT_INPUT_COUNT);
    let clone_ms = getenv_u64("BENCH_CLONE_MS", DEFAULT_CLONE_MS);
    let hash_ms = getenv_u64("BENCH_HASH_MS", DEFAULT_HASH_MS);
    let scan_ms = getenv_u64("BENCH_SCAN_MS", DEFAULT_SCAN_MS);
    let combined_ms = getenv_u64("BENCH_COMBINED_MS", DEFAULT_COMBINED_MS);
    print_header(input_count, clone_ms, hash_ms, scan_ms, combined_ms);

    for workload in Workload::all() {
        for &cell_count in &CELL_COUNTS {
            for &density_pct in &DENSITY_PCTS {
                let min_duration = workload.min_duration(clone_ms, hash_ms, scan_ms, combined_ms);
                let mut vec_acc = BenchAccum::default();
                let mut cow_acc = BenchAccum::default();
                for input_idx in 0..input_count {
                    let seed = 0xCAFEBABE_DEADC0DE
                        ^ (workload as u64) << 56
                        ^ (cell_count as u64) << 24
                        ^ (density_pct as u64) << 16
                        ^ input_idx as u64;
                    let input = make_input(cell_count, density_pct, seed);
                    if input_idx % 2 == 0 {
                        vec_acc.add(measure_vec(workload, &input, min_duration));
                        cow_acc.add(measure_cow(workload, &input, min_duration));
                    } else {
                        cow_acc.add(measure_cow(workload, &input, min_duration));
                        vec_acc.add(measure_vec(workload, &input, min_duration));
                    }
                }
                let vec_ns = vec_acc.ns_per_op();
                let cow_ns = cow_acc.ns_per_op();
                println!(
                    "{:<24} {:>6} {:>8} {:>16.3} {:>16.3} {:>14.3}",
                    workload.name(),
                    cell_count,
                    density_pct,
                    vec_ns,
                    cow_ns,
                    cow_ns / vec_ns
                );
            }
            println!();
        }
        println!();
    }
}
