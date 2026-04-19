// tmp_prefix_repair_bench.rs
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::collections::VecDeque;
use std::hint::black_box;
use std::time::{Duration, Instant};

const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const CELL_CAPACITY: usize = 16 * 16;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Cell(u16);

struct Grid;

impl Grid {
    #[inline]
    fn cell(n: usize, i: usize, j: usize) -> Cell {
        debug_assert!(i < n && j < n);
        Cell((i * n + j) as u16)
    }

    #[inline]
    fn index(cell: Cell) -> usize {
        cell.0 as usize
    }

    #[inline]
    fn can_move(n: usize, cell: Cell, dir: usize) -> bool {
        if dir >= DIRS.len() {
            return false;
        }
        let idx = Self::index(cell);
        match dir {
            0 => idx >= n,
            1 => idx + n < n * n,
            2 => idx % n != 0,
            3 => idx % n + 1 < n,
            _ => false,
        }
    }

    #[inline]
    fn next_cell(n: usize, cell: Cell, dir: usize) -> Cell {
        debug_assert!(Self::can_move(n, cell, dir));
        let idx = Self::index(cell);
        match dir {
            0 => Cell((idx - n) as u16),
            1 => Cell((idx + n) as u16),
            2 => Cell((idx - 1) as u16),
            3 => Cell((idx + 1) as u16),
            _ => unreachable!("invalid dir: {dir}"),
        }
    }

    #[inline]
    fn dir_between_cells(n: usize, from: Cell, to: Cell) -> usize {
        let from_idx = Self::index(from);
        let to_idx = Self::index(to);
        if to_idx + n == from_idx {
            0
        } else if from_idx + n == to_idx {
            1
        } else if to_idx + 1 == from_idx && from_idx / n == to_idx / n {
            2
        } else if from_idx + 1 == to_idx && from_idx / n == to_idx / n {
            3
        } else {
            unreachable!("cells are not adjacent: from={from_idx}, to={to_idx}");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dropped {
    cell: Cell,
    color: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    food: Vec<u8>,
    pos: VecDeque<Cell>,
    colors: Vec<u8>,
    pos_occupancy: [u8; CELL_CAPACITY],
}

impl State {
    fn initial(n: usize, food: Vec<u8>) -> Self {
        let pos = VecDeque::from([
            Grid::cell(n, 4, 0),
            Grid::cell(n, 3, 0),
            Grid::cell(n, 2, 0),
            Grid::cell(n, 1, 0),
            Grid::cell(n, 0, 0),
        ]);
        Self::from_parts(food, pos, vec![1; 5])
    }

    fn from_parts(food: Vec<u8>, pos: VecDeque<Cell>, colors: Vec<u8>) -> Self {
        assert_eq!(pos.len(), colors.len());
        let mut pos_occupancy = [0_u8; CELL_CAPACITY];
        for &cell in &pos {
            pos_occupancy[Grid::index(cell)] += 1;
        }
        Self {
            food,
            pos,
            colors,
            pos_occupancy,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.pos.len()
    }

    #[inline]
    fn head(&self) -> Cell {
        self.pos[0]
    }

    #[inline]
    fn is_legal_dir(&self, n: usize, dir: usize) -> bool {
        if !Grid::can_move(n, self.head(), dir) {
            return false;
        }
        if self.len() >= 2 {
            let next = Grid::next_cell(n, self.head(), dir);
            if next == self.pos[1] {
                return false;
            }
        }
        true
    }

    fn legal_dirs(&self, n: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for dir in 0..DIRS.len() {
            if self.is_legal_dir(n, dir) {
                out.push(dir);
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StepResult {
    state: State,
    ate: Option<u8>,
    bite_idx: Option<usize>,
    dropped: Vec<Dropped>,
}

#[inline]
fn matches_prefix_len(a: &[u8], b: &[u8], len: usize) -> bool {
    if a.len() < len || b.len() < len {
        return false;
    }
    let mut i = 0;
    while i < len {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn find_bite_idx(pos: &VecDeque<Cell>) -> Option<usize> {
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

fn step(state: &State, n: usize, dir: usize) -> StepResult {
    debug_assert!(state.is_legal_dir(n, dir));
    let next_head = Grid::next_cell(n, state.head(), dir);

    let mut food = state.food.clone();
    let mut pos = state.pos.clone();
    let mut colors = state.colors.clone();
    let mut occ = state.pos_occupancy;
    let mut ate = None;

    let eat_idx = Grid::index(next_head);
    if food[eat_idx] != 0 {
        let color = food[eat_idx];
        food[eat_idx] = 0;
        colors.push(color);
        ate = Some(color);
    } else {
        let tail = pos.pop_back().unwrap();
        occ[Grid::index(tail)] -= 1;
    }

    let excluded_tail = pos.back().copied();
    let tail_bias = u8::from(excluded_tail == Some(next_head));
    let bite = occ[Grid::index(next_head)] > tail_bias;

    occ[Grid::index(next_head)] += 1;
    pos.push_front(next_head);
    let bite_idx = if bite { find_bite_idx(&pos) } else { None };
    debug_assert!(ate.is_none() || bite_idx.is_none());

    let mut dropped = Vec::new();
    if let Some(h) = bite_idx {
        let mut dropped_rev = Vec::new();
        while pos.len() > h + 1 {
            let cell = pos.pop_back().unwrap();
            occ[Grid::index(cell)] -= 1;
            let color = colors.pop().unwrap();
            food[Grid::index(cell)] = color;
            dropped_rev.push(Dropped { cell, color });
        }
        dropped_rev.reverse();
        dropped = dropped_rev;
    }

    StepResult {
        state: State {
            food,
            pos,
            colors,
            pos_occupancy: occ,
        },
        ate,
        bite_idx,
        dropped,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixRepairResult {
    state: State,
    ops: Vec<usize>,
    repaired: bool,
}

fn repair_prefix_after_bite_direct(
    st_after: &State,
    n: usize,
    prefix_target: &[u8],
    dropped: &[Dropped],
) -> PrefixRepairResult {
    let keep = st_after.colors.len().min(prefix_target.len());
    debug_assert!(matches_prefix_len(&st_after.colors, prefix_target, keep));

    let need = prefix_target.len().saturating_sub(st_after.colors.len());
    if need == 0 {
        return PrefixRepairResult {
            state: st_after.clone(),
            ops: Vec::new(),
            repaired: false,
        };
    }

    debug_assert!(dropped.len() >= need);
    let mut food = st_after.food.clone();
    let mut pos = st_after.pos.clone();
    let mut occ = st_after.pos_occupancy;
    let mut ops = Vec::with_capacity(need);
    let mut prev = st_after.head();

    for (t, ent) in dropped.iter().take(need).enumerate() {
        debug_assert_eq!(food[Grid::index(ent.cell)], ent.color);
        debug_assert_eq!(ent.color, prefix_target[st_after.colors.len() + t]);
        let dir = Grid::dir_between_cells(n, prev, ent.cell);
        ops.push(dir);
        food[Grid::index(ent.cell)] = 0;
        pos.push_front(ent.cell);
        occ[Grid::index(ent.cell)] += 1;
        prev = ent.cell;
    }

    PrefixRepairResult {
        state: State {
            food,
            pos,
            colors: prefix_target.to_vec(),
            pos_occupancy: occ,
        },
        ops,
        repaired: true,
    }
}

fn repair_prefix_after_bite_step(
    st_after: &State,
    n: usize,
    prefix_target: &[u8],
    dropped: &[Dropped],
) -> PrefixRepairResult {
    let need = prefix_target.len().saturating_sub(st_after.colors.len());
    if need == 0 {
        return PrefixRepairResult {
            state: st_after.clone(),
            ops: Vec::new(),
            repaired: false,
        };
    }

    let mut state = st_after.clone();
    let mut ops = Vec::with_capacity(need);
    for ent in dropped.iter().take(need) {
        let dir = Grid::dir_between_cells(n, state.head(), ent.cell);
        let res = step(&state, n, dir);
        assert_eq!(res.ate, Some(ent.color));
        assert!(res.bite_idx.is_none());
        assert!(res.dropped.is_empty());
        ops.push(dir);
        state = res.state;
    }

    PrefixRepairResult {
        state,
        ops,
        repaired: true,
    }
}

#[derive(Debug, Clone)]
struct BenchCase {
    label: String,
    seed: u64,
    turn: usize,
    n: usize,
    dropped_len: usize,
    need: usize,
    st_after: State,
    prefix_target: Vec<u8>,
    dropped: Vec<Dropped>,
}

fn random_food(n: usize, rng: &mut Xoshiro256PlusPlus) -> Vec<u8> {
    let mut food = vec![0_u8; n * n];
    for i in 0..n {
        for j in 0..n {
            if j == 0 && i <= 4 {
                continue;
            }
            if rng.random_range(0..100) < 35 {
                food[Grid::index(Grid::cell(n, i, j))] = rng.random_range(1..=3);
            }
        }
    }
    food
}

fn find_bite_case(seed_start: u64, min_dropped_len: usize, need: usize) -> BenchCase {
    for seed in seed_start..(seed_start + 10_000) {
        let n = [8, 10, 12, 16][(seed as usize) % 4];
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
        let mut state = State::initial(n, random_food(n, &mut rng));
        for turn in 0..10_000 {
            let dirs = state.legal_dirs(n);
            let dir = dirs[rng.random_range(0..dirs.len())];
            let before = state.clone();
            let res = step(&state, n, dir);
            if let Some(_bi) = res.bite_idx
                && res.dropped.len() >= min_dropped_len
            {
                let post_len = res.state.colors.len();
                let prefix_len = post_len + need;
                let prefix_target = before.colors[..prefix_len].to_vec();
                return BenchCase {
                    label: format!("need{need}_n{n}_seed{seed}_turn{turn}"),
                    seed,
                    turn,
                    n,
                    dropped_len: res.dropped.len(),
                    need,
                    st_after: res.state,
                    prefix_target,
                    dropped: res.dropped,
                };
            }
            state = res.state;
        }
    }
    panic!("bite case not found: seed_start={seed_start}, min_dropped_len={min_dropped_len}, need={need}");
}

fn checksum(result: &PrefixRepairResult) -> u64 {
    let head = result.state.head().0 as u64;
    let len = result.state.colors.len() as u64;
    let ops_len = result.ops.len() as u64;
    let repaired = u64::from(result.repaired);
    head ^ (len << 8) ^ (ops_len << 16) ^ (repaired << 24)
}

fn verify_case(case: &BenchCase) {
    let direct = repair_prefix_after_bite_direct(
        &case.st_after,
        case.n,
        &case.prefix_target,
        &case.dropped,
    );
    let step_based = repair_prefix_after_bite_step(
        &case.st_after,
        case.n,
        &case.prefix_target,
        &case.dropped,
    );
    assert_eq!(direct.repaired, step_based.repaired, "repaired mismatch: {}", case.label);
    assert_eq!(direct.ops, step_based.ops, "ops mismatch: {}", case.label);
    assert_eq!(direct.state, step_based.state, "state mismatch: {}", case.label);
}

fn bench_impl<F>(budget: Duration, mut f: F) -> (usize, f64, u64)
where
    F: FnMut() -> PrefixRepairResult,
{
    let start = Instant::now();
    let mut iters = 0usize;
    let mut acc = 0_u64;
    while start.elapsed() < budget {
        let result = black_box(f());
        acc ^= checksum(&result).wrapping_add(iters as u64);
        iters += 1;
    }
    let elapsed_ns = start.elapsed().as_secs_f64() * 1e9;
    let ns_per_iter = elapsed_ns / iters.max(1) as f64;
    black_box(acc);
    (iters, ns_per_iter, acc)
}

fn main() {
    let bench_ms = std::env::var("BENCH_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(150);
    let budget = Duration::from_millis(bench_ms);

    let needs = [0usize, 1, 2, 3, 4];
    let seed_starts = [1_u64, 1002, 2003, 3004, 4005];
    let mut cases = Vec::new();
    for (need, seed_start) in needs.into_iter().zip(seed_starts) {
        let min_drop = need.max(1);
        let case = find_bite_case(seed_start, min_drop, need);
        verify_case(&case);
        cases.push(case);
    }

    println!("case,impl,round,iters,ns_per_iter,checksum");
    for case in &cases {
        let mut step_runs = Vec::new();
        let mut direct_runs = Vec::new();

        for round in 0..2 {
            let (iters_s, ns_s, acc_s) = bench_impl(budget, || {
                repair_prefix_after_bite_step(
                    &case.st_after,
                    case.n,
                    &case.prefix_target,
                    &case.dropped,
                )
            });
            println!("{},step,{}, {},{:.3},{}", case.label, round + 1, iters_s, ns_s, acc_s);
            step_runs.push(ns_s);

            let (iters_d, ns_d, acc_d) = bench_impl(budget, || {
                repair_prefix_after_bite_direct(
                    &case.st_after,
                    case.n,
                    &case.prefix_target,
                    &case.dropped,
                )
            });
            println!("{},direct,{}, {},{:.3},{}", case.label, round + 1, iters_d, ns_d, acc_d);
            direct_runs.push(ns_d);
        }

        let step_avg = step_runs.iter().sum::<f64>() / step_runs.len() as f64;
        let direct_avg = direct_runs.iter().sum::<f64>() / direct_runs.len() as f64;
        println!(
            "summary,{},{},{},{},{},{},{:.3},{:.3},{:.3}",
            case.label,
            case.n,
            case.seed,
            case.turn,
            case.need,
            case.dropped_len,
            step_avg,
            direct_avg,
            step_avg / direct_avg
        );
    }
}
