// v102pro_full_exact_prefix_solver.rs
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
const STAGE_BEAM: usize = 5;
const EXTRA_LIMIT: usize = 20;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const BUDGETS_NORMAL: [usize; 4] = [3_000, 12_000, 40_000, 120_000];
const BUDGETS_LATE: [usize; 4] = [5_000, 20_000, 80_000, 200_000];

#[derive(Clone)]
struct Input {
    n: usize,
    m: usize,
    d: Vec<u8>,
    food: Vec<u8>,
}

#[derive(Clone)]
struct State {
    n: usize,
    food: Vec<u8>,
    pos: Vec<u16>,
    colors: Vec<u8>,
}

#[derive(Clone)]
struct BeamState {
    state: State,
    ops: String,
}

#[derive(Clone, Default)]
struct MoveInfo {
    bite_idx: Option<usize>,
    dropped: Vec<Dropped>,
}

#[derive(Clone)]
struct Dropped {
    cell: u16,
    color: u8,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct Key {
    pos: Vec<u16>,
    colors: Vec<u8>,
    food: Vec<(u16, u8)>,
}

#[derive(Clone)]
struct Node {
    state: State,
    parent: Option<usize>,
    action: String,
}

impl State {
    fn initial(input: &Input) -> Self {
        let n = input.n;
        Self {
            n,
            food: input.food.clone(),
            pos: vec![
                cell_of(4, 0, n),
                cell_of(3, 0, n),
                cell_of(2, 0, n),
                cell_of(1, 0, n),
                cell_of(0, 0, n),
            ],
            colors: vec![1; 5],
        }
    }
}

#[inline]
fn cell_of(r: usize, c: usize, n: usize) -> u16 {
    (r * n + c) as u16
}

#[inline]
fn rc_of(cell: u16, n: usize) -> (usize, usize) {
    let x = cell as usize;
    (x / n, x % n)
}

#[inline]
fn manhattan(n: usize, a: u16, b: u16) -> usize {
    let (ar, ac) = rc_of(a, n);
    let (br, bc) = rc_of(b, n);
    ar.abs_diff(br) + ac.abs_diff(bc)
}

#[inline]
fn time_over(start: &Instant) -> bool {
    start.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
}

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut d = Vec::with_capacity(m);
    for _ in 0..m {
        d.push(it.next().unwrap().parse::<u8>().unwrap());
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input { n, m, d, food }
}

#[inline]
fn next_head_cell(st: &State, dir: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(st.pos[0], st.n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= st.n as isize || nc < 0 || nc >= st.n as isize {
        return None;
    }
    Some(cell_of(nr as usize, nc as usize, st.n))
}

#[inline]
fn is_legal_dir(st: &State, dir: usize) -> bool {
    let Some(nh) = next_head_cell(st, dir) else {
        return false;
    };
    st.pos.len() < 2 || nh != st.pos[1]
}

fn legal_dirs(st: &State) -> Vec<usize> {
    let mut out = Vec::with_capacity(4);
    for dir in 0..4 {
        if is_legal_dir(st, dir) {
            out.push(dir);
        }
    }
    out
}

fn step_info(st: &State, dir: usize) -> (State, MoveInfo) {
    let nh = next_head_cell(st, dir).unwrap();
    let old_len = st.pos.len();

    let mut food = st.food.clone();
    let mut new_pos = Vec::with_capacity(old_len + 1);
    new_pos.push(nh);
    new_pos.extend_from_slice(&st.pos[..old_len - 1]);

    let mut new_colors = st.colors.clone();
    let ate = food[nh as usize];
    if ate != 0 {
        new_pos.push(st.pos[old_len - 1]);
        new_colors.push(ate);
        food[nh as usize] = 0;
    }

    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }

    let mut info = MoveInfo {
        bite_idx,
        ..MoveInfo::default()
    };

    if let Some(bi) = bite_idx {
        let mut dropped = Vec::with_capacity(new_pos.len().saturating_sub(bi + 1));
        for p in bi + 1..new_pos.len() {
            let cell = new_pos[p];
            let color = new_colors[p];
            food[cell as usize] = color;
            dropped.push(Dropped { cell, color });
        }
        new_pos.truncate(bi + 1);
        new_colors.truncate(bi + 1);
        info.dropped = dropped;
    }

    (
        State {
            n: st.n,
            food,
            pos: new_pos,
            colors: new_colors,
        },
        info,
    )
}

#[inline]
fn prefix_ok(st: &State, d: &[u8], ell: usize) -> bool {
    let upto = st.colors.len().min(ell);
    st.colors[..upto] == d[..upto]
}

#[inline]
fn exact_ok(st: &State, d: &[u8], ell: usize) -> bool {
    st.colors.len() == ell && st.colors[..ell] == d[..ell]
}

#[inline]
fn lcp(st: &State, d: &[u8]) -> usize {
    let mut i = 0;
    let upto = st.colors.len().min(d.len());
    while i < upto && st.colors[i] == d[i] {
        i += 1;
    }
    i
}

#[inline]
fn remaining_food_count(st: &State) -> usize {
    st.food.iter().filter(|&&c| c != 0).count()
}

fn encode_key(st: &State) -> Key {
    let mut food = Vec::with_capacity(st.colors.len() + 16);
    for (idx, &col) in st.food.iter().enumerate() {
        if col != 0 {
            food.push((idx as u16, col));
        }
    }
    Key {
        pos: st.pos.clone(),
        colors: st.colors.clone(),
        food,
    }
}

fn recover_exact(st: &State, input: &Input, ell: usize, dropped: &[Dropped]) -> Option<(State, String)> {
    if st.colors.len() > ell {
        return None;
    }

    let mut s = st.clone();
    let mut ops = String::new();
    let need_cnt = ell - s.colors.len();
    if dropped.len() < need_cnt {
        return None;
    }

    for ent in dropped.iter().take(need_cnt) {
        let need = input.d[s.colors.len()];
        if ent.color != need {
            return None;
        }

        let mut moved = false;
        for dir in legal_dirs(&s) {
            let nh = next_head_cell(&s, dir).unwrap();
            if nh != ent.cell {
                continue;
            }
            if s.food[ent.cell as usize] != need {
                continue;
            }
            let (ns, info) = step_info(&s, dir);
            if info.bite_idx.is_some() {
                return None;
            }
            s = ns;
            ops.push(DIRS[dir].2);
            moved = true;
            break;
        }
        if !moved {
            return None;
        }
    }

    if exact_ok(&s, &input.d, ell) {
        Some((s, ops))
    } else {
        None
    }
}

fn nearest_food_dist(st: &State, color: u8) -> usize {
    let head = st.pos[0];
    let mut best = usize::MAX;
    for (idx, &col) in st.food.iter().enumerate() {
        if col == color {
            let dist = manhattan(st.n, head, idx as u16);
            if dist < best {
                best = dist;
            }
        }
    }
    if best == usize::MAX {
        1_000_000
    } else {
        best
    }
}

fn target_suffix_info(st: &State, ell: usize, target: u8) -> Option<(usize, usize)> {
    let head = st.pos[0];
    let mut best: Option<(usize, usize)> = None;
    for idx in ell..st.colors.len() {
        if st.colors[idx] != target {
            continue;
        }
        let prev = st.pos[idx - 1];
        let dist = manhattan(st.n, head, prev);
        let cand = (dist, idx);
        if best.is_none() || cand < best.unwrap() {
            best = Some(cand);
        }
    }
    best
}

fn local_score(st: &State, input: &Input, ell: usize) -> (usize, usize, usize, usize, usize) {
    let target = input.d[ell];
    let excess = st.colors.len().saturating_sub(ell);

    if exact_ok(st, &input.d, ell) {
        let dist = nearest_food_dist(st, target);
        let mut adj = 0;
        for dir in legal_dirs(st) {
            let nh = next_head_cell(st, dir).unwrap();
            if st.food[nh as usize] == target {
                adj = 1;
                break;
            }
        }
        return (0, if adj == 1 { 0 } else { 1 }, dist, 0, excess);
    }

    if let Some((dist, idx)) = target_suffix_info(st, ell, target) {
        return (1, 0, dist, idx - ell, excess);
    }

    (2, 0, nearest_food_dist(st, target), 0, excess)
}

fn next_stage_rank(st: &State, input: &Input, ellp1: usize, ops_len: usize) -> (usize, usize, usize) {
    if ellp1 >= input.m {
        return (0, 0, ops_len);
    }
    let target = input.d[ellp1];
    let dist = nearest_food_dist(st, target);
    let (hr, hc) = rc_of(st.pos[0], st.n);
    let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
    (dist, center, ops_len)
}

fn final_rank(st: &State, input: &Input, ops_len: usize) -> (Reverse<usize>, usize, usize) {
    (Reverse(lcp(st, &input.d)), remaining_food_count(st), ops_len)
}

fn restore_plan(nodes: &[Node], mut id: usize) -> String {
    let mut parts = Vec::new();
    while let Some(parent) = nodes[id].parent {
        parts.push(nodes[id].action.clone());
        id = parent;
    }
    parts.reverse();

    let mut out = String::new();
    for s in parts {
        out.push_str(&s);
    }
    out
}

fn push_solution(
    sols: &mut Vec<(State, String)>,
    sol_keys: &mut FxHashSet<Key>,
    state: State,
    plan: String,
) {
    let key = encode_key(&state);
    if sol_keys.insert(key) {
        sols.push((state, plan));
    }
}

fn stage_search_bestfirst(
    start_state: &State,
    input: &Input,
    ell: usize,
    max_expansions: usize,
    extra_limit: usize,
    keep_solutions: usize,
    started: &Instant,
) -> Vec<(State, String)> {
    let mut nodes = Vec::with_capacity(max_expansions.min(30_000) + 8);
    nodes.push(Node {
        state: start_state.clone(),
        parent: None,
        action: String::new(),
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((local_score(start_state, input, ell), 0usize, uid, 0usize)));
    uid += 1;

    let mut seen = FxHashMap::<Key, usize>::default();
    seen.insert(encode_key(start_state), 0);

    let mut sols: Vec<(State, String)> = Vec::new();
    let mut sol_keys: FxHashSet<Key> = FxHashSet::default();
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, nid))) = pq.pop() {
        if expansions >= max_expansions || sols.len() >= keep_solutions || time_over(started) {
            break;
        }
        expansions += 1;

        let st = nodes[nid].state.clone();

        if exact_ok(&st, &input.d, ell) {
            let mut prefix_plan: Option<String> = None;
            let target = input.d[ell];
            for dir in legal_dirs(&st) {
                let nh = next_head_cell(&st, dir).unwrap();
                if st.food[nh as usize] != target {
                    continue;
                }
                let (ns, _) = step_info(&st, dir);
                if exact_ok(&ns, &input.d, ell + 1) {
                    let mut plan = prefix_plan.clone().unwrap_or_else(|| {
                        let s = restore_plan(&nodes, nid);
                        prefix_plan = Some(s.clone());
                        s
                    });
                    plan.push(DIRS[dir].2);
                    push_solution(&mut sols, &mut sol_keys, ns, plan);
                    if sols.len() >= keep_solutions {
                        break;
                    }
                }
            }
        }
        if sols.len() >= keep_solutions {
            break;
        }

        {
            let mut prefix_plan: Option<String> = None;
            for dir1 in legal_dirs(&st) {
                let (ns1, info1) = step_info(&st, dir1);
                if info1.bite_idx.is_none() || !prefix_ok(&ns1, &input.d, ell) {
                    continue;
                }

                let mut rs = ns1;
                let mut recover_ops = String::new();
                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = recover_exact(&rs, input, ell, &info1.dropped) else {
                        continue;
                    };
                    rs = rec_state;
                    recover_ops = rec_ops;
                }
                if !exact_ok(&rs, &input.d, ell) {
                    continue;
                }

                let target = input.d[ell];
                for dir2 in legal_dirs(&rs) {
                    let nh = next_head_cell(&rs, dir2).unwrap();
                    if rs.food[nh as usize] != target {
                        continue;
                    }
                    let (ns2, _) = step_info(&rs, dir2);
                    if !exact_ok(&ns2, &input.d, ell + 1) {
                        continue;
                    }

                    let mut plan = prefix_plan.clone().unwrap_or_else(|| {
                        let s = restore_plan(&nodes, nid);
                        prefix_plan = Some(s.clone());
                        s
                    });
                    plan.push(DIRS[dir1].2);
                    plan.push_str(&recover_ops);
                    plan.push(DIRS[dir2].2);
                    push_solution(&mut sols, &mut sol_keys, ns2, plan);
                    if sols.len() >= keep_solutions {
                        break;
                    }
                }
                if sols.len() >= keep_solutions {
                    break;
                }
            }
        }
        if sols.len() >= keep_solutions {
            break;
        }

        for dir in legal_dirs(&st) {
            let (mut ns, info) = step_info(&st, dir);
            let mut action = String::new();
            action.push(DIRS[dir].2);

            if info.bite_idx.is_some() && ns.colors.len() < ell {
                if !prefix_ok(&ns, &input.d, ell) {
                    continue;
                }
                let Some((recovered, rec_ops)) = recover_exact(&ns, input, ell, &info.dropped) else {
                    continue;
                };
                ns = recovered;
                action.push_str(&rec_ops);
            }

            if !prefix_ok(&ns, &input.d, ell) {
                continue;
            }
            if ns.colors.len() > ell + extra_limit {
                continue;
            }

            let nd = depth + 1;
            let key = encode_key(&ns);
            if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(key, nd);

            let child = nodes.len();
            nodes.push(Node {
                state: ns.clone(),
                parent: Some(nid),
                action,
            });
            pq.push(Reverse((local_score(&ns, input, ell), nd, uid, child)));
            uid += 1;
        }

        if !exact_ok(&st, &input.d, ell) {
            for dir in legal_dirs(&st) {
                let (ns, info) = step_info(&st, dir);
                if info.bite_idx.is_none() || !prefix_ok(&ns, &input.d, ell) {
                    continue;
                }

                let mut rs = ns;
                let mut action = String::new();
                action.push(DIRS[dir].2);

                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = recover_exact(&rs, input, ell, &info.dropped) else {
                        continue;
                    };
                    rs = rec_state;
                    action.push_str(&rec_ops);
                }
                if !exact_ok(&rs, &input.d, ell) {
                    continue;
                }

                let nd = depth + 1;
                let key = encode_key(&rs);
                if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(key, nd);

                let child = nodes.len();
                nodes.push(Node {
                    state: rs.clone(),
                    parent: Some(nid),
                    action,
                });
                pq.push(Reverse((local_score(&rs, input, ell), nd, uid, child)));
                uid += 1;
            }
        }
    }

    sols.sort_unstable_by_key(|(st, plan)| next_stage_rank(st, input, ell + 1, plan.len()));
    if sols.len() > keep_solutions {
        sols.truncate(keep_solutions);
    }
    sols
}

fn solve(input: &Input) -> String {
    let started = Instant::now();
    let mut beam = vec![BeamState {
        state: State::initial(input),
        ops: String::new(),
    }];

    for ell in 5..input.m {
        if beam.is_empty() || time_over(&started) {
            break;
        }

        let remaining = input.m - ell;
        let budgets: &[usize] = if remaining < 10 {
            &BUDGETS_LATE
        } else {
            &BUDGETS_NORMAL
        };

        let mut new_beam = Vec::new();
        let mut stage_seen: FxHashSet<Key> = FxHashSet::default();

        for bs in &beam {
            if time_over(&started) {
                break;
            }

            let mut sols: Vec<(State, String)> = Vec::new();
            for &budget in budgets {
                if time_over(&started) {
                    break;
                }
                sols = stage_search_bestfirst(
                    &bs.state,
                    input,
                    ell,
                    budget,
                    EXTRA_LIMIT,
                    STAGE_BEAM,
                    &started,
                );
                if !sols.is_empty() {
                    break;
                }
            }

            for (ns, plan) in sols {
                if bs.ops.len() + plan.len() > MAX_TURNS {
                    continue;
                }
                let key = encode_key(&ns);
                if !stage_seen.insert(key) {
                    continue;
                }

                let mut ops = bs.ops.clone();
                ops.push_str(&plan);
                new_beam.push(BeamState { state: ns, ops });
            }
        }

        if new_beam.is_empty() {
            break;
        }

        let next_ell = ell + 1;
        new_beam.sort_unstable_by_key(|bs| next_stage_rank(&bs.state, input, next_ell, bs.ops.len()));
        if new_beam.len() > STAGE_BEAM {
            new_beam.truncate(STAGE_BEAM);
        }
        beam = new_beam;
    }

    if beam.is_empty() {
        return String::new();
    }

    beam.sort_unstable_by_key(|bs| final_rank(&bs.state, input, bs.ops.len()));
    beam[0].ops.clone()
}

fn main() {
    let input = read_input();
    let ans = solve(&input);

    let mut out = String::new();
    for ch in ans.chars() {
        out.push(ch);
        out.push('\n');
    }
    print!("{out}");
}
