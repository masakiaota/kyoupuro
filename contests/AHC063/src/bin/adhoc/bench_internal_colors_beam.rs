// bench_internal_colors_beam.rs
use std::env;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::ops::Deref;
use std::time::{Duration, Instant};

use rustc_hash::FxHasher;

const FIXED_CAP: usize = 16 * 16;
const COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
const DEFAULT_INPUT_COUNT: usize = 4;
const DEFAULT_MEASURE_MS: u64 = 250;
const DEFAULT_ROUNDS: usize = 24;
const BRANCHES: usize = 3;
const LENGTHS: [usize; 4] = [32, 64, 128, 192];
const BEAM_WIDTHS: [usize; 4] = [64, 256, 1024, 4096];

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

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
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
    fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    fn push(&mut self, color: u8) {
        debug_assert!(self.len() < FIXED_CAP);
        let idx = self.len();
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
        let idx = self.len() - 1;
        let color = self.buf[idx];
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len -= 1;
        Some(color)
    }

    #[inline]
    fn truncate(&mut self, new_len: usize) {
        while self.len() > new_len {
            let _ = self.pop();
        }
    }
}

impl Deref for InternalColors {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buf[..self.len()]
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
struct VecState {
    colors: VecColors,
    score: u64,
}

#[derive(Clone)]
struct InternalState {
    colors: InternalColors,
    score: u64,
}

#[derive(Clone, Copy)]
struct Action {
    kind: u8,
    color: u8,
    truncate_to: u8,
}

#[derive(Clone)]
struct BenchInput {
    init_states: Vec<Vec<u8>>,
    target: Vec<u8>,
    actions: Vec<Action>,
}

#[derive(Clone, Copy)]
enum Workload {
    ExpandOnly,
    ExpandHashLcp,
}

impl Workload {
    fn all() -> [Self; 2] {
        [Self::ExpandOnly, Self::ExpandHashLcp]
    }

    fn name(self) -> &'static str {
        match self {
            Self::ExpandOnly => "expand_only",
            Self::ExpandHashLcp => "expand_hash_lcp",
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

fn make_input(len: usize, beam_width: usize, rounds: usize, seed: u64) -> BenchInput {
    let mut init_states = Vec::with_capacity(beam_width);
    for parent_idx in 0..beam_width {
        let mut colors = Vec::with_capacity(len);
        let seed_base = splitmix64(seed ^ parent_idx as u64);
        for depth in 0..len {
            let x = splitmix64(seed_base ^ depth as u64);
            colors.push((x % 6) as u8 + 1);
        }
        init_states.push(colors);
    }

    let mut target = Vec::with_capacity(len);
    let target_seed = splitmix64(seed ^ 0xC0FF_EE11_2233_4455);
    for idx in 0..len {
        let x = splitmix64(target_seed ^ idx as u64);
        target.push((x % 6) as u8 + 1);
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
                let kind = if bucket < 55 {
                    0
                } else if bucket < 80 {
                    1
                } else if bucket < 90 {
                    2
                } else {
                    3
                };
                actions.push(Action {
                    kind,
                    color: ((x >> 8) % 6) as u8 + 1,
                    truncate_to: ((x >> 16) % FIXED_CAP as u64) as u8,
                });
            }
        }
    }

    BenchInput {
        init_states,
        target,
        actions,
    }
}

#[inline]
fn apply_action_vec(colors: &mut Vec<u8>, action: Action) {
    match action.kind {
        0 => {}
        1 => {
            if colors.len() < FIXED_CAP {
                colors.push(action.color);
            }
        }
        2 => {
            let _ = colors.pop();
        }
        3 => {
            if !colors.is_empty() {
                let new_len = (action.truncate_to as usize % colors.len()) + 1;
                colors.truncate(new_len);
            }
        }
        _ => unreachable!(),
    }
}

#[inline]
fn apply_action_internal(colors: &mut InternalColors, action: Action) {
    match action.kind {
        0 => {}
        1 => {
            if colors.len() < FIXED_CAP {
                colors.push(action.color);
            }
        }
        2 => {
            let _ = colors.pop();
        }
        3 => {
            if colors.len() > 0 {
                let new_len = (action.truncate_to as usize % colors.len()) + 1;
                colors.truncate(new_len);
            }
        }
        _ => unreachable!(),
    }
}

fn bench_once_vec(workload: Workload, input: &BenchInput, beam_width: usize, rounds: usize) -> u64 {
    let mut beam = input
        .init_states
        .iter()
        .map(|colors| VecState {
            colors: VecColors {
                data: colors.clone(),
            },
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
                apply_action_vec(&mut child.colors.data, input.actions[action_base + branch_idx]);

                let update = match workload {
                    Workload::ExpandOnly => {
                        let first = *child.colors.data.first().unwrap_or(&0) as u64;
                        let last = *child.colors.data.last().unwrap_or(&0) as u64;
                        first ^ last.rotate_left((branch_idx * 7) as u32)
                    }
                    Workload::ExpandHashLcp => {
                        let mut hasher = FxHasher::default();
                        child.colors.data.hash(&mut hasher);
                        let hash = hasher.finish();
                        let l = lcp(&child.colors.data, &input.target) as u64;
                        hash ^ l.rotate_left(17)
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

fn bench_once_internal(
    workload: Workload,
    input: &BenchInput,
    beam_width: usize,
    rounds: usize,
) -> u64 {
    let mut beam = input
        .init_states
        .iter()
        .map(|colors| InternalState {
            colors: InternalColors::from_slice(colors),
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
                apply_action_internal(&mut child.colors, input.actions[action_base + branch_idx]);

                let update = match workload {
                    Workload::ExpandOnly => {
                        let first = *child.colors.first().unwrap_or(&0) as u64;
                        let last = *child.colors.last().unwrap_or(&0) as u64;
                        first ^ last.rotate_left((branch_idx * 7) as u32)
                    }
                    Workload::ExpandHashLcp => {
                        let mut hasher = FxHasher::default();
                        child.colors.hash(&mut hasher);
                        let hash = hasher.finish();
                        let l = lcp(&child.colors, &input.target) as u64;
                        hash ^ l.rotate_left(17)
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

fn measure_internal(
    workload: Workload,
    input: &BenchInput,
    beam_width: usize,
    rounds: usize,
    min_duration: Duration,
) -> BenchAccum {
    let start = Instant::now();
    let mut ops = 0_u64;
    while start.elapsed() < min_duration {
        black_box(bench_once_internal(workload, input, beam_width, rounds));
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
        "# InternalColors beam-style benchmark\n\
         # workload = clone parent state -> apply action -> optionally hash+lcp -> materialize next beam\n\
         # lengths={:?}, beam_widths={:?}, rounds={}, inputs_per_config={}, branches={}, measure_ms={}\n",
        LENGTHS, BEAM_WIDTHS, rounds, input_count, BRANCHES, measure_ms
    );
    println!(
        "{:<18} {:>6} {:>8} {:>16} {:>16} {:>14}",
        "workload", "len", "beam", "vec ns/child", "internal ns/child", "internal/vec"
    );
}

fn main() {
    let input_count = getenv_usize("BENCH_INPUT_COUNT", DEFAULT_INPUT_COUNT);
    let rounds = getenv_usize("BENCH_ROUNDS", DEFAULT_ROUNDS);
    let measure_ms = getenv_u64("BENCH_MEASURE_MS", DEFAULT_MEASURE_MS);
    let min_duration = Duration::from_millis(measure_ms);
    print_header(input_count, rounds, measure_ms);

    for workload in Workload::all() {
        for &len in &LENGTHS {
            for &beam_width in &BEAM_WIDTHS {
                let mut vec_acc = BenchAccum::default();
                let mut internal_acc = BenchAccum::default();
                for input_idx in 0..input_count {
                    let seed = 0x1234_5678_9ABC_DEF0
                        ^ (workload as u64) << 56
                        ^ (len as u64) << 24
                        ^ (beam_width as u64) << 8
                        ^ input_idx as u64;
                    let input = make_input(len, beam_width, rounds, seed);
                    if input_idx % 2 == 0 {
                        vec_acc.add(measure_vec(workload, &input, beam_width, rounds, min_duration));
                        internal_acc.add(measure_internal(
                            workload,
                            &input,
                            beam_width,
                            rounds,
                            min_duration,
                        ));
                    } else {
                        internal_acc.add(measure_internal(
                            workload,
                            &input,
                            beam_width,
                            rounds,
                            min_duration,
                        ));
                        vec_acc.add(measure_vec(workload, &input, beam_width, rounds, min_duration));
                    }
                }
                let vec_ns = vec_acc.ns_per_op();
                let internal_ns = internal_acc.ns_per_op();
                println!(
                    "{:<18} {:>6} {:>8} {:>16.3} {:>16.3} {:>14.3}",
                    workload.name(),
                    len,
                    beam_width,
                    vec_ns,
                    internal_ns,
                    internal_ns / vec_ns
                );
            }
            println!();
        }
        println!();
    }
}
