// bench_internal_colors.rs
use std::env;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::ops::Deref;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use rustc_hash::FxHasher;

const FIXED_CAP: usize = 16 * 16;
const COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
const DEFAULT_INPUT_COUNT: usize = 6;
const DEFAULT_CLONE_MS: u64 = 80;
const DEFAULT_HASH_MS: u64 = 180;
const DEFAULT_PREFIX_MS: u64 = 120;
const PROBE_COUNT: usize = 4096;
const LENGTHS: [usize; 6] = [5, 16, 64, 128, 192, 256];

const fn build_hash_pows(base: u64) -> [u64; FIXED_CAP + 1] {
    let mut pows = [0_u64; FIXED_CAP + 1];
    pows[0] = 1;
    let mut i = 1;
    while i <= FIXED_CAP {
        pows[i] = pows[i - 1].wrapping_mul(base);
        i += 1;
    }
    pows
}

const COLOR_HASH_POW1: [u64; FIXED_CAP + 1] = build_hash_pows(COLOR_HASH_BASE1);
const COLOR_HASH_POW2: [u64; FIXED_CAP + 1] = build_hash_pows(COLOR_HASH_BASE2);

#[inline]
fn color_hash_token(color: u8) -> u64 {
    color as u64 + 1
}

#[derive(Clone)]
struct VecColors {
    data: Vec<u8>,
}

#[derive(Clone)]
struct InternalColors {
    buf: [u8; FIXED_CAP],
    len: u16,
    hash1: u64,
    hash2: u64,
}

impl InternalColors {
    #[inline]
    fn new() -> Self {
        Self {
            buf: [0; FIXED_CAP],
            len: 0,
            hash1: 0,
            hash2: 0,
        }
    }

    #[inline]
    fn from_slice(colors: &[u8]) -> Self {
        let mut out = Self::new();
        out.buf[..colors.len()].copy_from_slice(colors);
        out.len = colors.len() as u16;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &color in colors {
            let x = color_hash_token(color);
            out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
            out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(COLOR_HASH_BASE1);
            pow2 = pow2.wrapping_mul(COLOR_HASH_BASE2);
        }
        out
    }

    #[inline]
    fn push(&mut self, color: u8) {
        let idx = self.len as usize;
        self.buf[idx] = color;
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let idx = self.len as usize - 1;
        let color = self.buf[idx];
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len -= 1;
        Some(color)
    }
}

impl Deref for InternalColors {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buf[..self.len as usize]
    }
}

impl Hash for InternalColors {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        self.hash1.hash(state);
        self.hash2.hash(state);
    }
}

#[derive(Clone)]
struct BenchInput {
    vec_colors: VecColors,
    internal_colors: InternalColors,
    next_colors: Vec<u8>,
    target: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Workload {
    CloneOnly,
    ClonePushPop,
    HashOnly,
    CloneHash,
    PrefixLcp,
}

impl Workload {
    fn all() -> [Self; 5] {
        [
            Self::CloneOnly,
            Self::ClonePushPop,
            Self::HashOnly,
            Self::CloneHash,
            Self::PrefixLcp,
        ]
    }

    fn name(self) -> &'static str {
        match self {
            Self::CloneOnly => "clone_only",
            Self::ClonePushPop => "clone_push_pop",
            Self::HashOnly => "hash_only",
            Self::CloneHash => "clone_hash",
            Self::PrefixLcp => "prefix_lcp",
        }
    }

    fn min_duration(self, clone_ms: u64, hash_ms: u64, prefix_ms: u64) -> Duration {
        match self {
            Self::CloneOnly | Self::ClonePushPop => Duration::from_millis(clone_ms),
            Self::HashOnly | Self::CloneHash => Duration::from_millis(hash_ms),
            Self::PrefixLcp => Duration::from_millis(prefix_ms),
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
fn lcp(a: &[u8], b: &[u8]) -> usize {
    let lim = a.len().min(b.len());
    let mut i = 0;
    while i < lim && a[i] == b[i] {
        i += 1;
    }
    i
}

fn make_input(len: usize, seed: u64) -> BenchInput {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    let colors = (0..len).map(|_| rng.random_range(1..=6)).collect::<Vec<_>>();
    let mut target = colors.clone();
    if len > 0 {
        let mode = rng.random_range(0..3);
        match mode {
            0 => {}
            1 => {
                let idx = rng.random_range(0..len);
                target[idx] = (target[idx] % 6) + 1;
                if target[idx] == colors[idx] {
                    target[idx] = (target[idx] % 6) + 1;
                }
            }
            _ => {
                target.truncate(rng.random_range(0..=len));
            }
        }
    }
    let next_colors = (0..PROBE_COUNT)
        .map(|_| rng.random_range(1..=6))
        .collect::<Vec<_>>();
    BenchInput {
        vec_colors: VecColors {
            data: colors.clone(),
        },
        internal_colors: InternalColors::from_slice(&colors),
        next_colors,
        target,
    }
}

fn measure_vec(workload: Workload, input: &BenchInput, min_duration: Duration) -> BenchAccum {
    const CHUNK: usize = 1024;
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut turn = 0_usize;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for _ in 0..CHUNK {
            let next = input.next_colors[turn & (PROBE_COUNT - 1)];
            match workload {
                Workload::CloneOnly => {
                    let cloned = black_box(&input.vec_colors).clone();
                    acc ^= black_box(cloned.data[0] as u64);
                    black_box(cloned);
                }
                Workload::ClonePushPop => {
                    let mut cloned = black_box(&input.vec_colors).clone();
                    let popped = cloned.data.pop().unwrap_or(0);
                    cloned.data.push(next);
                    acc ^= black_box((popped as u64) ^ cloned.data[0] as u64);
                    black_box(cloned);
                }
                Workload::HashOnly => {
                    let mut hasher = FxHasher::default();
                    input.vec_colors.data.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::CloneHash => {
                    let cloned = black_box(&input.vec_colors).clone();
                    let mut hasher = FxHasher::default();
                    cloned.data.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::PrefixLcp => {
                    acc ^= black_box(lcp(&input.vec_colors.data, &input.target) as u64);
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

fn measure_internal(workload: Workload, input: &BenchInput, min_duration: Duration) -> BenchAccum {
    const CHUNK: usize = 1024;
    let start = Instant::now();
    let mut ops = 0_u64;
    let mut turn = 0_usize;
    let mut acc = 0_u64;
    while start.elapsed() < min_duration {
        for _ in 0..CHUNK {
            let next = input.next_colors[turn & (PROBE_COUNT - 1)];
            match workload {
                Workload::CloneOnly => {
                    let cloned = black_box(&input.internal_colors).clone();
                    acc ^= black_box(cloned[0] as u64);
                    black_box(cloned);
                }
                Workload::ClonePushPop => {
                    let mut cloned = black_box(&input.internal_colors).clone();
                    let popped = cloned.pop().unwrap_or(0);
                    cloned.push(next);
                    acc ^= black_box((popped as u64) ^ cloned[0] as u64);
                    black_box(cloned);
                }
                Workload::HashOnly => {
                    let mut hasher = FxHasher::default();
                    input.internal_colors.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::CloneHash => {
                    let cloned = black_box(&input.internal_colors).clone();
                    let mut hasher = FxHasher::default();
                    cloned.hash(&mut hasher);
                    acc ^= black_box(hasher.finish());
                }
                Workload::PrefixLcp => {
                    acc ^= black_box(lcp(&input.internal_colors, &input.target) as u64);
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

fn print_header(input_count: usize, clone_ms: u64, hash_ms: u64, prefix_ms: u64) {
    println!(
        "# InternalColors benchmark\n\
         # Vec<u8> vs fixed-buffer InternalColors(len + [u8; 256] + rolling_hash)\n\
         # lengths={:?}, inputs_per_length={}, clone_measure_ms={}, hash_measure_ms={}, prefix_measure_ms={}, probes={}\n",
        LENGTHS, input_count, clone_ms, hash_ms, prefix_ms, PROBE_COUNT
    );
    println!(
        "{:<16} {:>6} {:>16} {:>16} {:>14}",
        "workload", "len", "vec ns/op", "internal ns/op", "internal/vec"
    );
}

fn main() {
    let input_count = getenv_usize("BENCH_INPUT_COUNT", DEFAULT_INPUT_COUNT);
    let clone_ms = getenv_u64("BENCH_CLONE_MS", DEFAULT_CLONE_MS);
    let hash_ms = getenv_u64("BENCH_HASH_MS", DEFAULT_HASH_MS);
    let prefix_ms = getenv_u64("BENCH_PREFIX_MS", DEFAULT_PREFIX_MS);
    print_header(input_count, clone_ms, hash_ms, prefix_ms);

    for workload in Workload::all() {
        for &len in &LENGTHS {
            let min_duration = workload.min_duration(clone_ms, hash_ms, prefix_ms);
            let mut vec_acc = BenchAccum::default();
            let mut internal_acc = BenchAccum::default();
            for input_idx in 0..input_count {
                let seed = 1_000_000_u64 * workload as u64 + 10_000_u64 * len as u64 + input_idx as u64;
                let input = make_input(len, seed);
                if input_idx % 2 == 0 {
                    vec_acc.add(measure_vec(workload, &input, min_duration));
                    internal_acc.add(measure_internal(workload, &input, min_duration));
                } else {
                    internal_acc.add(measure_internal(workload, &input, min_duration));
                    vec_acc.add(measure_vec(workload, &input, min_duration));
                }
            }
            let vec_ns = vec_acc.ns_per_op();
            let internal_ns = internal_acc.ns_per_op();
            println!(
                "{:<16} {:>6} {:>16.3} {:>16.3} {:>14.3}",
                workload.name(),
                len,
                vec_ns,
                internal_ns,
                internal_ns / vec_ns
            );
        }
        println!();
    }
}
