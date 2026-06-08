// v001_baseline.rs
use proconio::{input, marker::Chars};
use std::collections::HashMap;

const MAX_N: usize = 20;
const MAX_M: usize = 40;
const MAX_CELLS: usize = MAX_N * MAX_N;
const INF: u16 = 30000;

const OP_F: u8 = 0;
const OP_R: u8 = 1;
const OP_L: u8 = 2;
const OP_S: u8 = 3;
const OP_M: u8 = 4;
const OP_P: u8 = 5;

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    m: usize,
    t_limit: usize,
    wall_mask: [u8; MAX_CELLS],
    ball_pos: [u16; MAX_M],
    basket_pos: [u16; MAX_M],
}

impl Input {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            t_limit: usize,
            wall_v_raw: [Chars; n],
            wall_h_raw: [Chars; n - 1],
            bcde: [(usize, usize, usize, usize); m],
        }

        let cell = |i: usize, j: usize| -> usize { i * n + j };
        let mut wall_mask = [0u8; MAX_CELLS];

        for i in 0..n {
            for j in 0..n - 1 {
                if wall_v_raw[i][j] == '1' {
                    let left = cell(i, j);
                    let right = cell(i, j + 1);
                    wall_mask[left] |= 1 << 1;
                    wall_mask[right] |= 1 << 3;
                }
            }
        }

        for i in 0..n - 1 {
            for j in 0..n {
                if wall_h_raw[i][j] == '1' {
                    let up = cell(i, j);
                    let down = cell(i + 1, j);
                    wall_mask[up] |= 1 << 2;
                    wall_mask[down] |= 1 << 0;
                }
            }
        }

        let mut ball_pos = [0u16; MAX_M];
        let mut basket_pos = [0u16; MAX_M];
        for (k, &(b, c, d, e)) in bcde.iter().enumerate() {
            ball_pos[k] = cell(b, c) as u16;
            basket_pos[k] = cell(d, e) as u16;
        }

        Self {
            n,
            m,
            t_limit,
            wall_mask,
            ball_pos,
            basket_pos,
        }
    }
}

#[derive(Debug, Clone)]
struct Grid {
    cell_count: usize,
    move_mask: [u8; MAX_CELLS],
    next_cell: [[u16; 4]; MAX_CELLS],
}

impl Grid {
    fn new(input: &Input) -> Self {
        let n = input.n;
        let cell_count = n * n;
        let dir_delta = [-(n as i16), 1, n as i16, -1];
        let mut move_mask = [0u8; MAX_CELLS];
        let mut next_cell = [[0u16; 4]; MAX_CELLS];

        for cell in 0..cell_count {
            let i = cell / n;
            let j = cell % n;
            let mut edge = 0u8;
            if i > 0 {
                edge |= 1 << 0;
            }
            if j + 1 < n {
                edge |= 1 << 1;
            }
            if i + 1 < n {
                edge |= 1 << 2;
            }
            if j > 0 {
                edge |= 1 << 3;
            }
            move_mask[cell] = edge & !input.wall_mask[cell];

            for dir in 0..4 {
                next_cell[cell][dir] = if move_mask[cell] & (1 << dir) != 0 {
                    (cell as i16 + dir_delta[dir]) as u16
                } else {
                    cell as u16
                };
            }
        }

        Self {
            cell_count,
            move_mask,
            next_cell,
        }
    }

    #[inline(always)]
    fn can_move(&self, cell: usize, dir: usize) -> bool {
        self.move_mask[cell] & (1 << dir) != 0
    }

    #[inline(always)]
    fn next(&self, cell: usize, dir: usize) -> usize {
        self.next_cell[cell][dir] as usize
    }
}

fn all_pairs_dist(grid: &Grid) -> [[u16; MAX_CELLS]; MAX_CELLS] {
    let mut dist = [[INF; MAX_CELLS]; MAX_CELLS];
    let mut queue = [0usize; MAX_CELLS];

    for src in 0..grid.cell_count {
        let mut head = 0usize;
        let mut tail = 0usize;
        dist[src][src] = 0;
        queue[tail] = src;
        tail += 1;

        while head < tail {
            let cell = queue[head];
            head += 1;
            let nd = dist[src][cell] + 1;
            for dir in 0..4 {
                if !grid.can_move(cell, dir) {
                    continue;
                }
                let to = grid.next(cell, dir);
                if dist[src][to] == INF {
                    dist[src][to] = nd;
                    queue[tail] = to;
                    tail += 1;
                }
            }
        }
    }

    dist
}

#[inline(always)]
fn turn_cost(from: u8, to: u8) -> u8 {
    let diff = (to + 4 - from) & 3;
    match diff {
        0 => 0,
        1 | 3 => 1,
        2 => 2,
        _ => unreachable!(),
    }
}

fn push_turn_to(ops: &mut Vec<u8>, dir: &mut u8, target_dir: u8) {
    let diff = (target_dir + 4 - *dir) & 3;
    match diff {
        0 => {}
        1 => ops.push(OP_R),
        2 => {
            ops.push(OP_R);
            ops.push(OP_R);
        }
        3 => ops.push(OP_L),
        _ => unreachable!(),
    }
    *dir = target_dir;
}

fn move_to(
    grid: &Grid,
    dist: &[[u16; MAX_CELLS]; MAX_CELLS],
    pos: &mut usize,
    dir: &mut u8,
    target: usize,
    ops: &mut Vec<u8>,
) {
    while *pos != target {
        let cur_dist = dist[*pos][target];
        let mut best_dir = 0u8;
        let mut best_turn = u8::MAX;
        let mut found = false;

        for nd in 0..4u8 {
            let nd_usize = nd as usize;
            if !grid.can_move(*pos, nd_usize) {
                continue;
            }
            let next = grid.next(*pos, nd_usize);
            if dist[next][target] + 1 != cur_dist {
                continue;
            }
            let cost = turn_cost(*dir, nd);
            if !found || cost < best_turn {
                found = true;
                best_turn = cost;
                best_dir = nd;
            }
        }

        debug_assert!(found);
        push_turn_to(ops, dir, best_dir);
        ops.push(OP_F);
        *pos = grid.next(*pos, best_dir as usize);
    }
}

fn build_basic_ops(input: &Input, grid: &Grid, dist: &[[u16; MAX_CELLS]; MAX_CELLS]) -> Vec<u8> {
    let mut done = [false; MAX_M];
    let mut done_count = 0usize;
    let mut pos = 0usize;
    let mut dir = 1u8;
    let mut ops = Vec::with_capacity(input.t_limit.min(4096));

    while done_count < input.m {
        let mut best_k = 0usize;
        let mut best_cost = u32::MAX;
        let mut best_to_ball = u16::MAX;

        for k in 0..input.m {
            if done[k] {
                continue;
            }
            let ball = input.ball_pos[k] as usize;
            let basket = input.basket_pos[k] as usize;
            let to_ball = dist[pos][ball];
            let cost = to_ball as u32 + dist[ball][basket] as u32;
            if cost < best_cost || (cost == best_cost && to_ball < best_to_ball) {
                best_k = k;
                best_cost = cost;
                best_to_ball = to_ball;
            }
        }

        let ball = input.ball_pos[best_k] as usize;
        let basket = input.basket_pos[best_k] as usize;
        move_to(grid, dist, &mut pos, &mut dir, ball, &mut ops);
        ops.push(OP_S);
        move_to(grid, dist, &mut pos, &mut dir, basket, &mut ops);
        ops.push(OP_S);

        done[best_k] = true;
        done_count += 1;
    }

    ops
}

#[derive(Debug, Clone)]
struct MacroPlan {
    start: usize,
    len: usize,
    occurrences: Vec<usize>,
    saving: isize,
}

#[derive(Debug, Clone)]
struct RollingHash {
    prefix: Vec<u64>,
    pow: Vec<u64>,
}

impl RollingHash {
    fn new(s: &[u8]) -> Self {
        const BASE: u64 = 911382323;
        let mut prefix = vec![0u64; s.len() + 1];
        let mut pow = vec![1u64; s.len() + 1];
        for i in 0..s.len() {
            prefix[i + 1] = prefix[i].wrapping_mul(BASE).wrapping_add(s[i] as u64 + 1);
            pow[i + 1] = pow[i].wrapping_mul(BASE);
        }
        Self { prefix, pow }
    }

    #[inline(always)]
    fn get(&self, l: usize, r: usize) -> u64 {
        self.prefix[r].wrapping_sub(self.prefix[l].wrapping_mul(self.pow[r - l]))
    }
}

fn split_hash_group_by_actual(ops: &[u8], len: usize, positions: Vec<usize>) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    'outer: for pos in positions {
        for group in groups.iter_mut() {
            let rep = group[0];
            if ops[rep..rep + len] == ops[pos..pos + len] {
                group.push(pos);
                continue 'outer;
            }
        }
        groups.push(vec![pos]);
    }
    groups
}

fn greedy_non_overlapping_occurrences(positions: &[usize], len: usize) -> Vec<usize> {
    let mut selected = Vec::new();
    let mut next_allowed = 0usize;
    for &pos in positions {
        if pos >= next_allowed {
            selected.push(pos);
            next_allowed = pos + len;
        }
    }
    selected
}

fn find_best_macro_plan(ops: &[u8]) -> Option<MacroPlan> {
    let n = ops.len();
    if n < 4 {
        return None;
    }

    let hash = RollingHash::new(ops);
    let mut best: Option<MacroPlan> = None;

    for len in 2..=n / 2 {
        let max_count = n / len;
        let theoretical = (max_count.saturating_sub(1) * (len - 1)) as isize - 2;
        if theoretical <= best.as_ref().map_or(0, |plan| plan.saving) {
            continue;
        }

        let mut by_hash: HashMap<u64, Vec<usize>> = HashMap::with_capacity(n - len + 1);
        for start in 0..=n - len {
            by_hash
                .entry(hash.get(start, start + len))
                .or_default()
                .push(start);
        }

        for positions in by_hash.into_values() {
            if positions.len() < 2 {
                continue;
            }
            for group in split_hash_group_by_actual(ops, len, positions) {
                if group.len() < 2 {
                    continue;
                }
                let occurrences = greedy_non_overlapping_occurrences(&group, len);
                if occurrences.len() < 2 {
                    continue;
                }
                let saving = ((occurrences.len() - 1) * (len - 1)) as isize - 2;
                if saving <= 0 {
                    continue;
                }

                let should_update = match &best {
                    None => true,
                    Some(plan) => {
                        saving > plan.saving
                            || (saving == plan.saving
                                && (len > plan.len
                                    || (len == plan.len && occurrences[0] < plan.start)))
                    }
                };

                if should_update {
                    best = Some(MacroPlan {
                        start: occurrences[0],
                        len,
                        occurrences,
                        saving,
                    });
                }
            }
        }
    }

    best
}

fn compress_with_single_macro(ops: &[u8]) -> (Vec<u8>, Option<MacroPlan>) {
    let Some(plan) = find_best_macro_plan(ops) else {
        return (ops.to_vec(), None);
    };

    let mut replacement = HashMap::with_capacity(plan.occurrences.len().saturating_sub(1));
    for &pos in plan.occurrences.iter().skip(1) {
        replacement.insert(pos, true);
    }

    let mut compressed = Vec::with_capacity(ops.len() - plan.saving as usize);
    let mut i = 0usize;
    while i < ops.len() {
        if i == plan.start {
            compressed.push(OP_M);
            compressed.extend_from_slice(&ops[i..i + plan.len]);
            compressed.push(OP_M);
            i += plan.len;
        } else if replacement.contains_key(&i) {
            compressed.push(OP_P);
            i += plan.len;
        } else {
            compressed.push(ops[i]);
            i += 1;
        }
    }

    (compressed, Some(plan))
}

fn op_to_char(op: u8) -> char {
    match op {
        OP_F => 'F',
        OP_R => 'R',
        OP_L => 'L',
        OP_S => 'S',
        OP_M => 'M',
        OP_P => 'P',
        _ => unreachable!(),
    }
}

fn print_ops(ops: &[u8]) {
    let mut out = String::with_capacity(ops.len() * 2);
    for &op in ops {
        out.push(op_to_char(op));
        out.push('\n');
    }
    print!("{out}");
}

fn main() {
    let input = Input::read();
    let grid = Grid::new(&input);
    let dist = all_pairs_dist(&grid);

    let basic_ops = build_basic_ops(&input, &grid, &dist);
    let (mut answer, _plan) = compress_with_single_macro(&basic_ops);

    if answer.len() > input.t_limit {
        answer.truncate(input.t_limit);
    }

    #[cfg(feature = "local")]
    {
        if let Some(plan) = &_plan {
            eprintln!(
                "[v001] N={} M={} T={} basic_len={} answer_len={} macro_start={} macro_len={} macro_count={} saving={}",
                input.n,
                input.m,
                input.t_limit,
                basic_ops.len(),
                answer.len(),
                plan.start,
                plan.len,
                plan.occurrences.len(),
                plan.saving,
            );
        } else {
            eprintln!(
                "[v001] N={} M={} T={} basic_len={} answer_len={} macro=none",
                input.n,
                input.m,
                input.t_limit,
                basic_ops.len(),
                answer.len(),
            );
        }
    }

    print_ops(&answer);
}
