// v051_segment_creation.rs
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.80;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const STAGE_BEAM: usize = 5;
const MAX_TARGETS_PER_STAGE: usize = 12;
const MAX_TARGETS_RESCUE: usize = 24;
const VISIT_REPEAT_LIMIT: usize = 12;
const BUDGETS_NORMAL: [(usize, usize); 3] = [(2_000, 20), (8_000, 22), (24_000, 26)];
const BUDGETS_LATE: [(usize, usize); 3] = [(4_000, 22), (14_000, 26), (40_000, 30)];
const BUDGETS_RESCUE: [(usize, usize); 2] = [(16_000, 26), (60_000, 34)];
const SHAPE_EXPANSION_CAP: usize = 90_000;
const SHAPE_DEPTH_LIMIT: usize = 256;
const SHAPE_EXTRA_LIMIT: usize = 6;
const SHAPE_CANDIDATES: usize = 4;
const BUILD_ONE_EXPANSION_CAP: usize = 40_000;
const BUILD_ONE_DEPTH_SLACK: usize = 8;
const BUILD_ONE_SOL_CAP: usize = 24;
const SEGMENT_BEAM_WIDTH: usize = 12;
const SEGMENT_SEARCH_BEAM: usize = 160;
const SEGMENT_SEARCH_DEPTH: usize = 96;

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

#[derive(Hash, Eq, PartialEq)]
struct VisitKey {
    head: u16,
    neck: u16,
    len: u16,
    goal: u16,
    restore_len: u16,
}

struct Node {
    state: State,
    parent: Option<usize>,
    move_seg: String,
}

struct ShapeNode {
    state: State,
    parent: Option<usize>,
    mv: char,
}

struct PosNode {
    pos: Vec<u16>,
    parent: Option<usize>,
    mv: char,
}

#[derive(Clone)]
struct SegmentPlan {
    segments: Vec<Vec<u8>>,
    stored_count: usize,
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

    #[inline]
    fn head(&self) -> u16 {
        self.pos[0]
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
fn time_over(started: &Instant) -> bool {
    started.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
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
fn dir_of_char(ch: u8) -> Option<usize> {
    match ch {
        b'U' => Some(0),
        b'D' => Some(1),
        b'L' => Some(2),
        b'R' => Some(3),
        _ => None,
    }
}

#[inline]
fn dir_between_cells(n: usize, a: u16, b: u16) -> Option<usize> {
    let (ar, ac) = rc_of(a, n);
    let (br, bc) = rc_of(b, n);
    if ar == br + 1 && ac == bc {
        return Some(0);
    }
    if ar + 1 == br && ac == bc {
        return Some(1);
    }
    if ar == br && ac == bc + 1 {
        return Some(2);
    }
    if ar == br && ac + 1 == bc {
        return Some(3);
    }
    None
}

#[inline]
fn next_head_cell_with_min_col(st: &State, dir: usize, min_col: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(st.head(), st.n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= st.n as isize || nc < 0 || nc >= st.n as isize {
        return None;
    }
    if (nc as usize) < min_col {
        return None;
    }
    Some(cell_of(nr as usize, nc as usize, st.n))
}

#[inline]
fn is_legal_dir(st: &State, dir: usize, min_col: usize) -> bool {
    let Some(nh) = next_head_cell_with_min_col(st, dir, min_col) else {
        return false;
    };
    st.pos.len() < 2 || nh != st.pos[1]
}

fn legal_dirs(st: &State, min_col: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(4);
    for dir in 0..4 {
        if is_legal_dir(st, dir, min_col) {
            out.push(dir);
        }
    }
    out
}

fn step(st: &State, dir: usize, min_col: usize) -> Option<(State, u8, Option<usize>, Vec<Dropped>)> {
    let nh = next_head_cell_with_min_col(st, dir, min_col)?;
    let old_len = st.pos.len();

    let mut food = st.food.clone();
    let mut new_pos = Vec::with_capacity(old_len + 1);
    new_pos.push(nh);
    new_pos.extend_from_slice(&st.pos[..old_len - 1]);

    let mut new_colors = st.colors.clone();
    let ate = food[nh as usize];
    if ate != 0 {
        food[nh as usize] = 0;
        new_pos.push(st.pos[old_len - 1]);
        new_colors.push(ate);
    }

    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }

    let mut dropped = Vec::new();
    if let Some(bi) = bite_idx {
        dropped.reserve(new_pos.len().saturating_sub(bi + 1));
        for p in bi + 1..new_pos.len() {
            let cell = new_pos[p];
            let color = new_colors[p];
            food[cell as usize] = color;
            dropped.push(Dropped { cell, color });
        }
        new_pos.truncate(bi + 1);
        new_colors.truncate(bi + 1);
    }

    Some((
        State {
            n: st.n,
            food,
            pos: new_pos,
            colors: new_colors,
        },
        ate,
        bite_idx,
        dropped,
    ))
}

#[inline]
fn lcp(colors: &[u8], target: &[u8]) -> usize {
    let mut i = 0;
    let m = colors.len().min(target.len());
    while i < m && colors[i] == target[i] {
        i += 1;
    }
    i
}

#[inline]
fn prefix_ok(st: &State, target: &[u8], ell: usize) -> bool {
    let keep = st.colors.len().min(ell);
    st.colors[..keep] == target[..keep]
}

#[inline]
fn exact_prefix(st: &State, target: &[u8], ell: usize) -> bool {
    st.colors.len() == ell && st.colors[..ell] == target[..ell]
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

fn nearest_food_dist(st: &State, color: u8, min_col: usize) -> (usize, usize) {
    let head = st.head();
    let mut best = usize::MAX;
    let mut cnt = 0usize;
    for (idx, &col) in st.food.iter().enumerate() {
        if col != color {
            continue;
        }
        let (r, c) = rc_of(idx as u16, st.n);
        if c < min_col {
            let _ = r;
            continue;
        }
        cnt += 1;
        let dist = manhattan(st.n, head, idx as u16);
        if dist < best {
            best = dist;
        }
    }
    if best == usize::MAX {
        (1_000_000_000, cnt)
    } else {
        (best, cnt)
    }
}

fn target_adjacent(st: &State, target: u8, min_col: usize) -> Option<usize> {
    let neck = st.pos[1];
    for dir in 0..4 {
        let Some(nh) = next_head_cell_with_min_col(st, dir, min_col) else {
            continue;
        };
        if nh == neck {
            continue;
        }
        if st.food[nh as usize] == target {
            return Some(dir);
        }
    }
    None
}

fn target_suffix_info(st: &State, ell: usize, target: u8) -> Option<(usize, usize)> {
    let head = st.head();
    let mut best: Option<(usize, usize)> = None;
    for idx in ell..st.colors.len() {
        if st.colors[idx] != target {
            continue;
        }
        let prev = st.pos[idx - 1];
        let cand = (manhattan(st.n, head, prev), idx);
        if best.is_none() || cand < best.unwrap() {
            best = Some(cand);
        }
    }
    best
}

fn local_score(st: &State, target: &[u8], ell: usize, min_col: usize) -> (usize, usize, usize, usize, usize) {
    let want = target[ell];
    if exact_prefix(st, target, ell) {
        let (dist, _) = nearest_food_dist(st, want, min_col);
        let adj = target_adjacent(st, want, min_col).is_some();
        return (0, if adj { 0 } else { 1 }, dist, 0, st.colors.len() - ell);
    }

    if let Some((dist, idx)) = target_suffix_info(st, ell, want) {
        return (1, 0, dist, idx - ell, st.colors.len() - ell);
    }

    let (dist, _) = nearest_food_dist(st, want, min_col);
    (2, 0, dist, 0, st.colors.len().saturating_sub(ell))
}

fn next_stage_rank(st: &State, target: &[u8], ellp1: usize, min_col: usize) -> (usize, usize, usize) {
    if ellp1 >= target.len() {
        return (0, 0, 0);
    }
    let (dist, _) = nearest_food_dist(st, target[ellp1], min_col);
    let (hr, hc) = rc_of(st.head(), st.n);
    let center = hr.abs_diff(st.n / 2) + hc.abs_diff(((st.n - min_col) / 2) + min_col);
    (dist, center, 0)
}

fn reconstruct_plan(nodes: &[Node], mut idx: usize) -> String {
    let mut rev = Vec::new();
    while let Some(parent) = nodes[idx].parent {
        rev.push(nodes[idx].move_seg.clone());
        idx = parent;
    }
    rev.reverse();

    let mut out = String::new();
    for seg in rev {
        out.push_str(&seg);
    }
    out
}

fn try_recover_exact(st: &State, target: &[u8], ell: usize, dropped: &[Dropped]) -> Option<(State, String)> {
    let mut s = st.clone();
    let need_cnt = ell as isize - s.colors.len() as isize;
    if need_cnt < 0 || dropped.len() < need_cnt as usize {
        return None;
    }

    let mut ops = String::new();
    for ent in dropped.iter().take(need_cnt as usize) {
        let need = target[s.colors.len()];
        if ent.color != need {
            return None;
        }
        let dir = dir_between_cells(s.n, s.head(), ent.cell)?;
        if ent.cell == s.pos[1] {
            return None;
        }
        if s.food[ent.cell as usize] != need {
            return None;
        }

        let (ns, ate, bite_idx, _) = step(&s, dir, 0)?;
        if ate != need || bite_idx.is_some() {
            return None;
        }
        s = ns;
        ops.push(DIRS[dir].2);
    }

    if exact_prefix(&s, target, ell) {
        Some((s, ops))
    } else {
        None
    }
}

fn stage_search_bestfirst(
    start_bs: &BeamState,
    target: &[u8],
    ell: usize,
    min_col: usize,
    budgets: &[(usize, usize)],
    keep_solutions: usize,
    started: &Instant,
) -> Vec<BeamState> {
    if budgets.is_empty() {
        return Vec::new();
    }
    let start = start_bs.state.clone();

    let max_expansions = budgets[budgets.len() - 1].0;
    let mut nodes = Vec::with_capacity(max_expansions.min(30_000) + 8);
    nodes.push(Node {
        state: start.clone(),
        parent: None,
        move_seg: String::new(),
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((local_score(&start, target, ell, min_col), 0usize, uid, 0usize)));
    uid += 1;

    let mut seen = FxHashMap::<Key, usize>::default();
    seen.insert(encode_key(&start), 0);

    let mut sols: Vec<BeamState> = Vec::new();
    let mut solkeys: FxHashSet<Key> = FxHashSet::default();
    let mut expansions = 0usize;
    let mut stage_idx = 0usize;
    let mut stage_limit = budgets[0].0;
    let mut extra_limit = budgets[0].1;
    let final_limit = budgets[budgets.len() - 1].0;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= final_limit || sols.len() >= keep_solutions || time_over(started) {
            break;
        }

        expansions += 1;
        let st = nodes[idx].state.clone();

        if exact_prefix(&st, target, ell) {
            if let Some(dir2) = target_adjacent(&st, target[ell], min_col) {
                let (ns2, _, bite2, _) = step(&st, dir2, min_col).unwrap();
                if bite2.is_none() && exact_prefix(&ns2, target, ell + 1) {
                    let key = encode_key(&ns2);
                    if solkeys.insert(key) {
                        let mut plan = reconstruct_plan(&nodes, idx);
                        plan.push(DIRS[dir2].2);

                        let mut ops = start_bs.ops.clone();
                        ops.push_str(&plan);
                        sols.push(BeamState { state: ns2, ops });
                        if sols.len() >= keep_solutions {
                            break;
                        }
                    }
                }
            }
        }
        if sols.len() >= keep_solutions {
            break;
        }

        {
            let mut prefix_plan: Option<String> = None;
            for dir1 in legal_dirs(&st, min_col) {
                let (ns1, _, bite1, dropped1) = step(&st, dir1, min_col).unwrap();
                if bite1.is_none() || !prefix_ok(&ns1, target, ell) {
                    continue;
                }

                let mut rs = ns1;
                let mut recover_ops = String::new();
                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, target, ell, &dropped1) else {
                        continue;
                    };
                    rs = rec_state;
                    recover_ops = rec_ops;
                }

                if !exact_prefix(&rs, target, ell) {
                    continue;
                }

                for dir2 in legal_dirs(&rs, min_col) {
                    let nh = next_head_cell_with_min_col(&rs, dir2, min_col).unwrap();
                    if rs.food[nh as usize] != target[ell] {
                        continue;
                    }

                    let (ns2, _, bite2, _) = step(&rs, dir2, min_col).unwrap();
                    if bite2.is_some() || !exact_prefix(&ns2, target, ell + 1) {
                        continue;
                    }

                    let key = encode_key(&ns2);
                    if !solkeys.insert(key) {
                        continue;
                    }

                    let mut plan = prefix_plan.clone().unwrap_or_else(|| {
                        let s = reconstruct_plan(&nodes, idx);
                        prefix_plan = Some(s.clone());
                        s
                    });
                    plan.push(DIRS[dir1].2);
                    plan.push_str(&recover_ops);
                    plan.push(DIRS[dir2].2);

                    let mut ops = start_bs.ops.clone();
                    ops.push_str(&plan);
                    sols.push(BeamState { state: ns2, ops });
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

        for dir in legal_dirs(&st, min_col) {
            let (mut ns, _, bite_idx, dropped) = step(&st, dir, min_col).unwrap();
            let mut seg = String::new();
            seg.push(DIRS[dir].2);

            if bite_idx.is_some() && ns.colors.len() < ell {
                if !prefix_ok(&ns, target, ell) {
                    continue;
                }
                let Some((rec_state, rec_ops)) = try_recover_exact(&ns, target, ell, &dropped) else {
                    continue;
                };
                ns = rec_state;
                seg.push_str(&rec_ops);
            }

            if !prefix_ok(&ns, target, ell) {
                continue;
            }
            if ns.colors.len() > ell + extra_limit {
                continue;
            }

            let nd = depth + seg.len();
            let key = encode_key(&ns);
            if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(key, nd);

            let child = nodes.len();
            nodes.push(Node {
                state: ns.clone(),
                parent: Some(idx),
                move_seg: seg,
            });
            pq.push(Reverse((local_score(&ns, target, ell, min_col), nd, uid, child)));
            uid += 1;
        }

        if !exact_prefix(&st, target, ell) {
            for dir in legal_dirs(&st, min_col) {
                let (ns, _, bite_idx, dropped) = step(&st, dir, min_col).unwrap();
                if bite_idx.is_none() || !prefix_ok(&ns, target, ell) {
                    continue;
                }

                let mut rs = ns;
                let mut seg = String::new();
                seg.push(DIRS[dir].2);

                if rs.colors.len() < ell {
                    let Some((rec_state, rec_ops)) = try_recover_exact(&rs, target, ell, &dropped) else {
                        continue;
                    };
                    rs = rec_state;
                    seg.push_str(&rec_ops);
                }

                if !exact_prefix(&rs, target, ell) {
                    continue;
                }

                let nd = depth + seg.len();
                let key = encode_key(&rs);
                if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(key, nd);

                let child = nodes.len();
                nodes.push(Node {
                    state: rs.clone(),
                    parent: Some(idx),
                    move_seg: seg,
                });
                pq.push(Reverse((local_score(&rs, target, ell, min_col), nd, uid, child)));
                uid += 1;
            }
        }

        if expansions >= stage_limit {
            if !sols.is_empty() {
                break;
            }
            if stage_idx + 1 < budgets.len() {
                stage_idx += 1;
                stage_limit = budgets[stage_idx].0;
                extra_limit = budgets[stage_idx].1;
            }
        }
    }

    sols.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, target, ell + 1, min_col), bs.ops.len()));

    let mut out = Vec::with_capacity(keep_solutions);
    let mut seen2: FxHashSet<Key> = FxHashSet::default();
    for bs in sols {
        if seen2.insert(encode_key(&bs.state)) {
            out.push(bs);
            if out.len() >= keep_solutions {
                break;
            }
        }
    }
    out
}

fn collect_food_cells(st: &State, color: u8, min_col: usize) -> Vec<u16> {
    let mut out = Vec::new();
    for (idx, &col) in st.food.iter().enumerate() {
        if col != color {
            continue;
        }
        let (_, c) = rc_of(idx as u16, st.n);
        if c < min_col {
            continue;
        }
        out.push(idx as u16);
    }
    out
}

fn neighbors(n: usize, cid: u16, min_col: usize) -> Vec<u16> {
    let (r, c) = rc_of(cid, n);
    let mut out = Vec::with_capacity(4);
    for (dr, dc, _) in DIRS {
        let nr = r as isize + dr;
        let nc = c as isize + dc;
        if nr >= 0 && nr < n as isize && nc >= min_col as isize && nc < n as isize {
            out.push(cell_of(nr as usize, nc as usize, n));
        }
    }
    out
}

fn bfs_next_dir(st: &State, goal: u16, target: u16, avoid_food: bool, strict_body: bool, min_col: usize) -> Option<usize> {
    let n = st.n;
    let start = st.head();
    if start == goal {
        return None;
    }

    let mut blocked = vec![false; n * n];
    for r in 0..n {
        for c in 0..min_col {
            blocked[r * n + c] = true;
        }
    }

    if avoid_food {
        for (idx, &col) in st.food.iter().enumerate() {
            let cell = idx as u16;
            if col != 0 && cell != goal && cell != target {
                blocked[idx] = true;
            }
        }
    }

    if strict_body && st.pos.len() >= 3 {
        for &cell in &st.pos[1..st.pos.len() - 1] {
            blocked[cell as usize] = true;
        }
    }

    blocked[start as usize] = false;
    if blocked[goal as usize] {
        return None;
    }

    let mut dist = vec![-1_i16; n * n];
    let mut first = vec![None::<usize>; n * n];
    let mut q = VecDeque::new();

    for dir in legal_dirs(st, min_col) {
        let nid = next_head_cell_with_min_col(st, dir, min_col).unwrap();
        let idx = nid as usize;
        if blocked[idx] || dist[idx] != -1 {
            continue;
        }
        dist[idx] = 1;
        first[idx] = Some(dir);
        q.push_back(nid);
    }

    while let Some(cid) = q.pop_front() {
        if cid == goal {
            return first[cid as usize];
        }
        let (r, c) = rc_of(cid, n);
        for (dr, dc, _) in DIRS {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr < 0 || nr >= n as isize || nc < min_col as isize || nc >= n as isize {
                continue;
            }
            let nid = cell_of(nr as usize, nc as usize, n);
            let idx = nid as usize;
            if blocked[idx] || dist[idx] != -1 {
                continue;
            }
            dist[idx] = dist[cid as usize] + 1;
            first[idx] = first[cid as usize];
            q.push_back(nid);
        }
    }

    None
}

#[inline]
fn make_visit_key(st: &State, goal: u16, restore_len: usize) -> VisitKey {
    let neck = if st.pos.len() >= 2 { st.pos[1] } else { st.head() };
    VisitKey {
        head: st.head(),
        neck,
        len: st.colors.len() as u16,
        goal,
        restore_len: restore_len as u16,
    }
}

fn advance_with_restore_queue(
    st: &State,
    dir: usize,
    target: u16,
    ell: usize,
    restore_queue: &mut VecDeque<Dropped>,
    min_col: usize,
) -> Option<(State, Option<usize>)> {
    let (ns, _, bite_idx, dropped) = step(st, dir, min_col)?;
    if ns.food[target as usize] == 0 {
        return None;
    }

    if !restore_queue.is_empty() {
        restore_queue.pop_front();
        return Some((ns, bite_idx));
    }

    if bite_idx.is_some() && ns.colors.len() < ell {
        let need = ell - ns.colors.len();
        for ent in dropped.into_iter().take(need) {
            restore_queue.push_back(ent);
        }
    }
    Some((ns, bite_idx))
}

fn navigate_to_goal_safe(
    bs: &BeamState,
    goal: u16,
    target: u16,
    min_col: usize,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut guard = 0usize;

    while st.head() != goal {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 30 {
            return None;
        }

        let key = make_visit_key(&st, goal, 0);
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = bfs_next_dir(&st, goal, target, true, true, min_col)?;
        let (ns, ate, bite_idx, _) = step(&st, dir, min_col)?;
        if ns.food[target as usize] == 0 {
            return None;
        }
        if bite_idx.is_some() || ate != 0 {
            return None;
        }

        st = ns;
        ops.push(DIRS[dir].2);
    }

    Some(BeamState { state: st, ops })
}

fn navigate_to_goal_loose(
    bs: &BeamState,
    goal: u16,
    target: u16,
    ell: usize,
    min_col: usize,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    let mut restore_queue: VecDeque<Dropped> = VecDeque::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 4;
    let mut guard = 0usize;

    while st.head() != goal || !restore_queue.is_empty() {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 80 {
            return None;
        }

        let key = make_visit_key(&st, goal, restore_queue.len());
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if front.cell == st.pos[1] {
                return None;
            }
            dir
        } else if let Some(dir) = bfs_next_dir(&st, goal, target, true, false, min_col) {
            dir
        } else {
            bfs_next_dir(&st, goal, target, false, false, min_col)?
        };

        let (ns, bite_idx) = advance_with_restore_queue(&st, dir, target, ell, &mut restore_queue, min_col)?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(DIRS[dir].2);
    }

    if st.colors.len() < ell {
        return None;
    }

    Some(BeamState { state: st, ops })
}

fn choose_shrink_dir(st: &State, target: &[u8], ell: usize, goal: u16, min_col: usize) -> Option<usize> {
    let anchor_idx = (ell.saturating_sub(1)).min(st.pos.len() - 1);
    let anchor = st.pos[anchor_idx];

    let mut best_bite: Option<((usize, usize, usize, usize), usize)> = None;
    let mut best_move: Option<((usize, usize, usize, usize), usize)> = None;

    for dir in legal_dirs(st, min_col) {
        let nh = next_head_cell_with_min_col(st, dir, min_col).unwrap();
        if nh == goal {
            continue;
        }

        let (sim, ate, bite_idx, _) = step(st, dir, min_col).unwrap();
        if sim.food[goal as usize] == 0 {
            continue;
        }

        let keep = sim.colors.len().min(ell);
        if sim.colors[..keep] != target[..keep] {
            continue;
        }

        let target_dist = manhattan(sim.n, sim.head(), goal);
        let anchor_dist = manhattan(sim.n, sim.head(), anchor);

        if bite_idx.is_some() {
            let under = usize::from(sim.colors.len() < ell);
            let dist_len = sim.colors.len().abs_diff(ell);
            let not_ready = usize::from(!(dir_between_cells(sim.n, sim.head(), goal).is_some() && goal != sim.pos[1]));
            let key = (under, dist_len, not_ready, target_dist + anchor_dist);
            if best_bite.as_ref().is_none_or(|(k, _)| key < *k) {
                best_bite = Some((key, dir));
            }
        } else {
            let len_gap = sim.colors.len().abs_diff(ell);
            let not_ready = usize::from(!(dir_between_cells(sim.n, sim.head(), goal).is_some() && goal != sim.pos[1]));
            let ate_penalty = usize::from(ate != 0);
            let key = (len_gap, not_ready, target_dist + anchor_dist, ate_penalty);
            if best_move.as_ref().is_none_or(|(k, _)| key < *k) {
                best_move = Some((key, dir));
            }
        }
    }

    if let Some((_, dir)) = best_bite {
        return Some(dir);
    }
    if let Some((_, dir)) = best_move {
        return Some(dir);
    }
    None
}

fn shrink_to_ell(
    bs: &BeamState,
    target: &[u8],
    ell: usize,
    goal: u16,
    target_color: u8,
    min_col: usize,
    started: &Instant,
) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();

    if st.colors.len() == ell {
        return (dir_between_cells(st.n, st.head(), goal).is_some() && goal != st.pos[1]).then_some(BeamState {
            state: st,
            ops,
        });
    }

    let mut restore_queue: VecDeque<Dropped> = VecDeque::new();
    let mut seen = FxHashMap::<VisitKey, usize>::default();
    let mut bite_count = 0usize;
    let bite_limit = st.n * st.n * 3;
    let mut guard = 0usize;

    while st.colors.len() != ell || !restore_queue.is_empty() || !(dir_between_cells(st.n, st.head(), goal).is_some() && goal != st.pos[1]) {
        if time_over(started) {
            return None;
        }
        guard += 1;
        if guard > st.n * st.n * 60 {
            return None;
        }

        let key = make_visit_key(&st, goal, restore_queue.len());
        let cnt = seen.entry(key).or_insert(0);
        *cnt += 1;
        if *cnt > VISIT_REPEAT_LIMIT {
            return None;
        }

        let dir = if let Some(front) = restore_queue.front() {
            let dir = dir_between_cells(st.n, st.head(), front.cell)?;
            if front.cell == st.pos[1] {
                return None;
            }
            dir
        } else {
            choose_shrink_dir(&st, target, ell, goal, min_col)?
        };

        let (ns, bite_idx) = advance_with_restore_queue(&st, dir, goal, ell, &mut restore_queue, min_col)?;
        if bite_idx.is_some() {
            bite_count += 1;
            if bite_count > bite_limit {
                return None;
            }
        }

        st = ns;
        ops.push(DIRS[dir].2);
    }

    if st.colors.len() == ell && st.food[goal as usize] == target_color && dir_between_cells(st.n, st.head(), goal).is_some() && goal != st.pos[1] {
        Some(BeamState { state: st, ops })
    } else {
        None
    }
}

fn finish_eat_target(bs: &BeamState, target: &[u8], ell: usize, goal: u16, min_col: usize) -> Option<BeamState> {
    let st = &bs.state;
    let mut ops = bs.ops.clone();
    let dir = dir_between_cells(st.n, st.head(), goal)?;
    if goal == st.pos[1] {
        return None;
    }

    let (ns, _, bite_idx, _) = step(st, dir, min_col)?;
    if bite_idx.is_some() {
        return None;
    }
    if ns.colors.len() >= ell + 1 && ns.colors[..=ell] == target[..=ell] {
        ops.push(DIRS[dir].2);
        Some(BeamState { state: ns, ops })
    } else {
        None
    }
}

fn try_target_exact(
    bs: &BeamState,
    target: &[u8],
    ell: usize,
    goal: u16,
    target_color: u8,
    min_col: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let head = bs.state.head();
    let mut cand = neighbors(bs.state.n, goal, min_col);

    cand.sort_unstable_by_key(|&cid| {
        (
            usize::from(bs.state.food[cid as usize] > 0),
            manhattan(bs.state.n, head, cid),
        )
    });

    let mut sols = Vec::new();
    for near in cand {
        if time_over(started) {
            break;
        }

        if let Some(b1) = navigate_to_goal_safe(bs, near, goal, min_col, started)
            && let Some(b2) = shrink_to_ell(&b1, target, ell, goal, target_color, min_col, started)
            && let Some(b3) = finish_eat_target(&b2, target, ell, goal, min_col)
        {
            sols.push(b3);
        }

        if let Some(b1) = navigate_to_goal_loose(bs, near, goal, ell, min_col, started)
            && let Some(b2) = shrink_to_ell(&b1, target, ell, goal, target_color, min_col, started)
            && let Some(b3) = finish_eat_target(&b2, target, ell, goal, min_col)
        {
            sols.push(b3);
        }
    }

    sols.sort_unstable_by_key(|x| x.ops.len());
    let mut out = Vec::new();
    let mut seen: FxHashSet<Key> = FxHashSet::default();
    for s in sols {
        if seen.insert(encode_key(&s.state)) {
            out.push(s);
        }
    }
    out
}

fn collect_exact_solutions(
    bs: &BeamState,
    target: &[u8],
    ell: usize,
    target_color: u8,
    min_col: usize,
    max_targets: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let mut sols = Vec::new();
    let mut targets = collect_food_cells(&bs.state, target_color, min_col);
    targets.sort_unstable_by_key(|&cid| manhattan(bs.state.n, bs.state.head(), cid));
    if targets.len() > max_targets {
        targets.truncate(max_targets);
    }
    for goal in targets {
        if time_over(started) {
            break;
        }
        let cand = try_target_exact(bs, target, ell, goal, target_color, min_col, started);
        for s in cand {
            sols.push(s);
            if sols.len() >= STAGE_BEAM * 2 {
                break;
            }
        }
        if sols.len() >= STAGE_BEAM * 2 {
            break;
        }
    }
    sols.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, target, ell + 1, min_col), bs.ops.len()));
    let mut out = Vec::new();
    let mut seen: FxHashSet<Key> = FxHashSet::default();
    for s in sols {
        if seen.insert(encode_key(&s.state)) {
            out.push(s);
            if out.len() >= STAGE_BEAM {
                break;
            }
        }
    }
    out
}

fn rescue_stage(
    beam: &[BeamState],
    target: &[u8],
    ell: usize,
    target_color: u8,
    min_col: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let mut order: Vec<usize> = (0..beam.len()).collect();
    order.sort_unstable_by_key(|&idx| (local_score(&beam[idx].state, target, ell, min_col), beam[idx].ops.len()));

    let mut rescue_map: FxHashMap<Key, BeamState> = FxHashMap::default();
    for &idx in &order {
        if time_over(started) {
            break;
        }
        let bs = &beam[idx];
        let mut sols = stage_search_bestfirst(bs, target, ell, min_col, &BUDGETS_RESCUE, STAGE_BEAM, started);
        if sols.is_empty() && !time_over(started) {
            sols = collect_exact_solutions(bs, target, ell, target_color, min_col, MAX_TARGETS_RESCUE, started);
        }

        for s in sols {
            if s.ops.len() > MAX_TURNS {
                continue;
            }
            let key = encode_key(&s.state);
            match rescue_map.get_mut(&key) {
                Some(prev) => {
                    if s.ops.len() < prev.ops.len() {
                        *prev = s;
                    }
                }
                None => {
                    rescue_map.insert(key, s);
                }
            }
        }
        if rescue_map.len() >= STAGE_BEAM * 2 {
            break;
        }
    }

    let mut out: Vec<BeamState> = rescue_map.into_values().collect();
    out.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, target, ell + 1, min_col), bs.ops.len()));
    if out.len() > STAGE_BEAM {
        out.truncate(STAGE_BEAM);
    }
    out
}

fn solve_exact_beam(
    start_bs: BeamState,
    target: &[u8],
    start_ell: usize,
    min_col: usize,
    started: &Instant,
) -> Vec<BeamState> {
    let mut beam = vec![start_bs];
    if start_ell >= target.len() {
        return beam;
    }

    for ell in start_ell..target.len() {
        if time_over(started) {
            break;
        }

        let target_color = target[ell];
        let budgets: &[(usize, usize)] = if target.len() - ell < 10 {
            &BUDGETS_LATE
        } else {
            &BUDGETS_NORMAL
        };
        let mut new_map: FxHashMap<Key, BeamState> = FxHashMap::default();

        for bs in &beam {
            if time_over(started) {
                break;
            }

            let mut sols = stage_search_bestfirst(bs, target, ell, min_col, budgets, STAGE_BEAM, started);
            if sols.is_empty() && !time_over(started) {
                sols = collect_exact_solutions(bs, target, ell, target_color, min_col, MAX_TARGETS_PER_STAGE, started);
            }

            for s in sols {
                if s.ops.len() > MAX_TURNS {
                    continue;
                }
                let key = encode_key(&s.state);
                match new_map.get_mut(&key) {
                    Some(prev) => {
                        if s.ops.len() < prev.ops.len() {
                            *prev = s;
                        }
                    }
                    None => {
                        new_map.insert(key, s);
                    }
                }
            }
        }

        if new_map.is_empty() && !time_over(started) {
            let rescue = rescue_stage(&beam, target, ell, target_color, min_col, started);
            for s in rescue {
                let key = encode_key(&s.state);
                match new_map.get_mut(&key) {
                    Some(prev) => {
                        if s.ops.len() < prev.ops.len() {
                            *prev = s;
                        }
                    }
                    None => {
                        new_map.insert(key, s);
                    }
                }
            }
        }

        if new_map.is_empty() {
            break;
        }

        let mut new_beam: Vec<BeamState> = new_map.into_values().collect();
        new_beam.sort_unstable_by_key(|bs| (next_stage_rank(&bs.state, target, ell + 1, min_col), bs.ops.len()));
        if new_beam.len() > STAGE_BEAM {
            new_beam.truncate(STAGE_BEAM);
        }
        beam = new_beam;
    }

    beam.sort_unstable_by_key(|bs| {
        let got = lcp(&bs.state.colors, target);
        (Reverse(got), bs.ops.len())
    });
    beam
}

fn build_preplacement_positions(n: usize, col: usize, len: usize) -> Option<Vec<u16>> {
    if col + 2 >= n {
        return None;
    }
    let total_len = 5 + len;
    let extra = total_len.saturating_sub(n);
    let mut pos = Vec::with_capacity(total_len);
    if col % 2 == 0 {
        for idx in 0..total_len {
            if idx < n {
                pos.push(cell_of(n - 1 - idx, col + 1, n));
            } else {
                pos.push(cell_of(idx - n, col + 2, n));
            }
        }
    } else {
        for idx in 0..total_len {
            if idx < n {
                pos.push(cell_of(idx, col + 1, n));
            } else {
                pos.push(cell_of(n - 1 - (idx - n), col + 2, n));
            }
        }
    }
    if extra > 3 {
        return None;
    }
    Some(pos)
}

fn shape_rank(
    st: &State,
    target: &[u8],
    goal_pos: &[u16],
    base_len: usize,
    extra_col_floor: usize,
) -> (usize, usize, usize, usize, usize) {
    let built = lcp(&st.colors, target).min(base_len);
    let mut mismatch = 0usize;
    let match_len = built.min(st.pos.len());
    for idx in 0..match_len {
        if st.pos[idx] != goal_pos[idx] {
            mismatch += 1;
        }
    }
    mismatch += built.saturating_sub(match_len);

    let head_dist = manhattan(st.n, st.head(), goal_pos[0]);
    let next_anchor = if built < base_len {
        manhattan(st.n, st.head(), goal_pos[built])
    } else {
        0
    };
    let extra_bad = st
        .pos
        .iter()
        .skip(base_len)
        .filter(|&&cid| rc_of(cid, st.n).1 < extra_col_floor)
        .count();
    (base_len - built, mismatch, extra_bad, head_dist, next_anchor)
}

fn shape_goal_ok(st: &State, goal_pos: &[u16], base_len: usize, extra_col_floor: usize, target: &[u8]) -> bool {
    if !prefix_ok(st, target, base_len) || st.pos.len() < base_len {
        return false;
    }
    if st.pos[..base_len] != goal_pos[..] {
        return false;
    }
    st.pos
        .iter()
        .skip(base_len)
        .all(|&cid| rc_of(cid, st.n).1 >= extra_col_floor)
}

fn reconstruct_shape(nodes: &[ShapeNode], mut idx: usize) -> String {
    let mut rev = Vec::new();
    while let Some(parent) = nodes[idx].parent {
        rev.push(nodes[idx].mv);
        idx = parent;
    }
    rev.reverse();
    rev.into_iter().collect()
}

fn search_to_shape(
    start_bs: &BeamState,
    target: &[u8],
    goal_pos: &[u16],
    min_col: usize,
    extra_col_floor: usize,
    started: &Instant,
) -> (Option<BeamState>, BeamState) {
    let base_len = target.len();
    let mut nodes = Vec::with_capacity(SHAPE_EXPANSION_CAP.min(60_000) + 8);
    nodes.push(ShapeNode {
        state: start_bs.state.clone(),
        parent: None,
        mv: '\0',
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((
        shape_rank(&start_bs.state, target, goal_pos, base_len, extra_col_floor),
        0usize,
        uid,
        0usize,
    )));
    uid += 1;

    let mut seen = FxHashMap::<Key, usize>::default();
    seen.insert(encode_key(&start_bs.state), 0);
    let mut expansions = 0usize;
    let mut best_partial = start_bs.clone();
    let mut best_rank = shape_rank(&start_bs.state, target, goal_pos, base_len, extra_col_floor);

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= SHAPE_EXPANSION_CAP || time_over(started) {
            break;
        }
        expansions += 1;
        let st = nodes[idx].state.clone();
        let cur_rank = shape_rank(&st, target, goal_pos, base_len, extra_col_floor);
        if cur_rank < best_rank {
            let mut ops = start_bs.ops.clone();
            ops.push_str(&reconstruct_shape(&nodes, idx));
            best_partial = BeamState { state: st.clone(), ops };
            best_rank = cur_rank;
        }

        if shape_goal_ok(&st, goal_pos, base_len, extra_col_floor, target) {
            let mut ops = start_bs.ops.clone();
            ops.push_str(&reconstruct_shape(&nodes, idx));
            return (Some(BeamState { state: st, ops }), best_partial);
        }
        if depth >= SHAPE_DEPTH_LIMIT {
            continue;
        }

        for dir in legal_dirs(&st, min_col) {
            let (ns, _, _, _) = step(&st, dir, min_col).unwrap();
            if ns.colors.len() < 5 {
                continue;
            }
            if !prefix_ok(&ns, target, base_len) {
                continue;
            }
            if ns.colors.len() > base_len + SHAPE_EXTRA_LIMIT {
                continue;
            }

            let nd = depth + 1;
            let key = encode_key(&ns);
            if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(key, nd);

            let child = nodes.len();
            nodes.push(ShapeNode {
                state: ns.clone(),
                parent: Some(idx),
                mv: DIRS[dir].2,
            });
            pq.push(Reverse((
                shape_rank(&ns, target, goal_pos, base_len, extra_col_floor),
                nd,
                uid,
                child,
            )));
            uid += 1;
        }
    }

    (None, best_partial)
}

fn apply_ops(bs: &BeamState, ops_suffix: &str, min_col: usize) -> Option<BeamState> {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    for ch in ops_suffix.bytes() {
        let dir = dir_of_char(ch)?;
        let (ns, _, _, _) = step(&st, dir, min_col)?;
        st = ns;
        ops.push(ch as char);
    }
    Some(BeamState { state: st, ops })
}

fn empty_step_pos(n: usize, food: &[u8], pos: &[u16], dir: usize, min_col: usize) -> Option<Vec<u16>> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(pos[0], n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= n as isize || nc < min_col as isize || nc >= n as isize {
        return None;
    }
    let nh = cell_of(nr as usize, nc as usize, n);
    if pos.len() >= 2 && nh == pos[1] {
        return None;
    }
    if food[nh as usize] != 0 {
        return None;
    }

    let mut new_pos = Vec::with_capacity(pos.len());
    new_pos.push(nh);
    new_pos.extend_from_slice(&pos[..pos.len() - 1]);
    for &cell in &new_pos[1..new_pos.len() - 1] {
        if cell == nh {
            return None;
        }
    }
    Some(new_pos)
}

fn navigate_empty_to_goal(
    bs: &BeamState,
    goal: u16,
    min_col: usize,
    started: &Instant,
) -> Option<BeamState> {
    if bs.state.head() == goal {
        return Some(bs.clone());
    }
    if bs.state.food[goal as usize] != 0 {
        return None;
    }

    let mut nodes = Vec::with_capacity(SHAPE_EXPANSION_CAP.min(80_000) + 8);
    nodes.push(PosNode {
        pos: bs.state.pos.clone(),
        parent: None,
        mv: '\0',
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    let head_dist = manhattan(bs.state.n, bs.state.head(), goal);
    pq.push(Reverse(((head_dist, 0usize), 0usize, uid, 0usize)));
    uid += 1;

    let mut seen = FxHashMap::<Vec<u16>, usize>::default();
    seen.insert(bs.state.pos.clone(), 0);
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= SHAPE_EXPANSION_CAP || time_over(started) {
            break;
        }
        expansions += 1;
        let pos = nodes[idx].pos.clone();
        if pos[0] == goal {
            let mut rev = Vec::new();
            let mut cur = idx;
            while let Some(parent) = nodes[cur].parent {
                rev.push(nodes[cur].mv);
                cur = parent;
            }
            rev.reverse();

            let mut state = bs.state.clone();
            let mut ops = bs.ops.clone();
            for ch in rev {
                let dir = dir_of_char(ch as u8).unwrap();
                let (ns, ate, bite_idx, _) = step(&state, dir, min_col)?;
                if ate != 0 || bite_idx.is_some() {
                    return None;
                }
                state = ns;
                ops.push(ch);
            }
            return Some(BeamState { state, ops });
        }
        if depth >= SHAPE_DEPTH_LIMIT {
            continue;
        }

        for dir in 0..4 {
            let Some(next_pos) = empty_step_pos(bs.state.n, &bs.state.food, &pos, dir, min_col) else {
                continue;
            };
            let nd = depth + 1;
            if seen.get(&next_pos).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(next_pos.clone(), nd);
            let child = nodes.len();
            nodes.push(PosNode {
                pos: next_pos.clone(),
                parent: Some(idx),
                mv: DIRS[dir].2,
            });
            let rank = (manhattan(bs.state.n, next_pos[0], goal), 4usize.saturating_sub(legal_dirs_len(bs.state.n, &next_pos, min_col)));
            pq.push(Reverse((rank, nd, uid, child)));
            uid += 1;
        }
    }

    None
}

fn legal_dirs_len(n: usize, pos: &[u16], min_col: usize) -> usize {
    let (hr, hc) = rc_of(pos[0], n);
    let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
    let mut cnt = 0usize;
    for (dr, dc, _) in DIRS {
        let nr = hr as isize + dr;
        let nc = hc as isize + dc;
        if nr < 0 || nr >= n as isize || nc < min_col as isize || nc >= n as isize {
            continue;
        }
        let nh = cell_of(nr as usize, nc as usize, n);
        if nh != neck {
            cnt += 1;
        }
    }
    cnt
}

fn sweep_start_cell(n: usize, stored_count: usize) -> u16 {
    let last = stored_count - 1;
    if last % 2 == 0 {
        cell_of(2, last + 1, n)
    } else {
        cell_of(n - 3, last + 1, n)
    }
}

fn build_sweep_ops(n: usize, segments: &[Vec<u8>]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let lens: Vec<usize> = segments.iter().map(Vec::len).collect();
    let mut out = String::new();
    let mut c = lens.len() - 1;

    if c % 2 == 0 {
        out.push('L');
        for _ in 1..lens[c] {
            out.push('D');
        }
        let prev_len = lens[c];
        if c > 0 {
            for _ in 0..(n - prev_len - 2) {
                out.push('D');
            }
            out.push('L');
        }
    } else {
        out.push('L');
        for _ in 1..lens[c] {
            out.push('U');
        }
        let prev_len = lens[c];
        if c > 0 {
            for _ in 0..(n - prev_len - 2) {
                out.push('U');
            }
            out.push('L');
        }
    }

    while c > 0 {
        c -= 1;
        if c % 2 == 0 {
            for _ in 0..(lens[c] + 1) {
                out.push('D');
            }
            if c > 0 {
                for _ in 0..(n - lens[c] - 2) {
                    out.push('D');
                }
                out.push('L');
            }
        } else {
            for _ in 0..(lens[c] + 1) {
                out.push('U');
            }
            if c > 0 {
                for _ in 0..(n - lens[c] - 2) {
                    out.push('U');
                }
                out.push('L');
            }
        }
    }

    out
}

fn finish_with_sweep(
    prefix_bs: BeamState,
    input: &Input,
    segments: &[Vec<u8>],
    stored_count: usize,
    started: &Instant,
) -> Option<BeamState> {
    if stored_count == 0 {
        return Some(prefix_bs);
    }

    let goal = sweep_start_cell(input.n, stored_count);
    let positioned = navigate_empty_to_goal(&prefix_bs, goal, stored_count, started)?;
    let ops = build_sweep_ops(input.n, &segments[..stored_count]);
    let finished = apply_ops(&positioned, &ops, 0)?;
    if finished.state.colors == input.d && finished.state.food.iter().all(|&x| x == 0) {
        Some(finished)
    } else {
        None
    }
}

fn place_segment(shaped: &BeamState, n: usize, col: usize) -> Option<BeamState> {
    let mut seq = String::new();
    seq.push('L');
    if col % 2 == 0 {
        for _ in 0..n - 1 {
            seq.push('U');
        }
        seq.push_str("RDLUR");
    } else {
        for _ in 0..n - 1 {
            seq.push('D');
        }
        seq.push_str("RULDR");
    }
    apply_ops(shaped, &seq, col)
}

fn segment_drop_ok(st: &State, seg: &[u8], col: usize) -> bool {
    if st.colors.len() != 5 || st.colors.iter().any(|&x| x != 1) {
        return false;
    }
    for r in 0..st.n {
        let expect = if col % 2 == 0 {
            if (2..2 + seg.len()).contains(&r) {
                seg[r - 2]
            } else {
                0
            }
        } else {
            if r + 2 >= st.n {
                0
            } else {
                let idx = st.n - 3 - r;
                if idx < seg.len() {
                    seg[idx]
                } else {
                    0
                }
            }
        };
        if st.food[cell_of(r, col, st.n) as usize] != expect {
            return false;
        }
    }
    true
}

fn build_segment_target(seg: &[u8]) -> Vec<u8> {
    let mut target = vec![1_u8; 5];
    target.extend_from_slice(seg);
    target
}

fn locked_columns_preserved(before: &[u8], after: &[u8], n: usize, limit_col: usize) -> bool {
    for r in 0..n {
        for c in 0..limit_col {
            let idx = r * n + c;
            if before[idx] != after[idx] {
                return false;
            }
        }
    }
    true
}

fn apply_ops_partial(bs: &BeamState, ops_suffix: &str, min_col: usize) -> BeamState {
    let mut st = bs.state.clone();
    let mut ops = bs.ops.clone();
    for ch in ops_suffix.bytes() {
        let Some(dir) = dir_of_char(ch) else {
            break;
        };
        let Some((ns, _, _, _)) = step(&st, dir, min_col) else {
            break;
        };
        st = ns;
        ops.push(ch as char);
    }
    BeamState { state: st, ops }
}

fn deposit_ops(n: usize, col: usize) -> String {
    let mut out = String::new();
    if col % 2 == 0 {
        for _ in 0..n - 1 {
            out.push('U');
        }
        out.push_str("RDLUR");
    } else {
        for _ in 0..n - 1 {
            out.push('D');
        }
        out.push_str("RULDR");
    }
    out
}

fn collect_all(seg_count: usize, n: usize) -> String {
    let mut out = String::new();
    for c in (0..seg_count).rev() {
        out.push('L');
        let mv = if c % 2 == 0 { 'D' } else { 'U' };
        for _ in 0..n - 1 {
            out.push(mv);
        }
    }
    out
}

fn segment_anchor(n: usize, col: usize) -> u16 {
    if col % 2 == 0 {
        cell_of(n - 1, col, n)
    } else {
        cell_of(0, col, n)
    }
}

fn build_rank(st: &State, anchor: u16, min_col: usize) -> (usize, usize, usize, usize) {
    (
        st.pos.iter().filter(|&&cid| rc_of(cid, st.n).1 == min_col).count(),
        usize::from(st.food[anchor as usize] != 0),
        manhattan(st.n, st.head(), anchor),
        4usize.saturating_sub(legal_dirs(st, min_col).len()),
    )
}

fn build_deposit_ready_positions(n: usize, col: usize, seg_len: usize) -> Option<Vec<u16>> {
    let total_len = 5 + seg_len;
    let extra = total_len.saturating_sub(n + 1);
    if col + 1 + extra >= n {
        return None;
    }
    let mut pos = Vec::with_capacity(total_len);
    if col % 2 == 0 {
        pos.push(cell_of(n - 1, col, n));
        for j in 0..n {
            pos.push(cell_of(n - 1 - j, col + 1, n));
        }
        for k in 0..extra {
            pos.push(cell_of(0, col + 2 + k, n));
        }
    } else {
        pos.push(cell_of(0, col, n));
        for j in 0..n {
            pos.push(cell_of(j, col + 1, n));
        }
        for k in 0..extra {
            pos.push(cell_of(n - 1, col + 2 + k, n));
        }
    }
    Some(pos)
}

fn build_carry_ready_positions(n: usize, col: usize, seg_len: usize) -> Option<Vec<u16>> {
    if col + 1 < n {
        return build_deposit_ready_positions(n, col, seg_len);
    }
    let total_len = 5 + seg_len;
    if total_len > n {
        return None;
    }
    let mut pos = Vec::with_capacity(total_len);
    if col % 2 == 0 {
        for idx in 0..total_len {
            pos.push(cell_of(n - 1 - idx, col, n));
        }
    } else {
        for idx in 0..total_len {
            pos.push(cell_of(idx, col, n));
        }
    }
    Some(pos)
}

fn collect_start_cell(stored_count: usize, n: usize) -> Option<u16> {
    if stored_count == 0 {
        return None;
    }
    let rightmost = stored_count - 1;
    let row = if rightmost % 2 == 0 { 0 } else { n - 1 };
    Some(cell_of(row, stored_count, n))
}

fn pos_goal_rank(n: usize, pos: &[u16], goal_pos: &[u16]) -> (usize, usize, usize) {
    let mut mismatch = 0usize;
    let mut total_dist = 0usize;
    let upto = pos.len().min(goal_pos.len());
    for idx in 0..upto {
        if pos[idx] != goal_pos[idx] {
            mismatch += 1;
        }
        total_dist += manhattan(n, pos[idx], goal_pos[idx]);
    }
    mismatch += pos.len().abs_diff(goal_pos.len());
    (mismatch, total_dist, manhattan(n, pos[0], goal_pos[0]))
}

fn scaffold_food_penalty(st: &State, goal_pos: &[u16]) -> usize {
    goal_pos
        .iter()
        .filter(|&&cid| st.food[cid as usize] != 0)
        .count()
}

fn scaffold_body_mismatch(st: &State, goal_pos: &[u16]) -> usize {
    let upto = st.pos.len().min(goal_pos.len());
    let mut mismatch = 0usize;
    for idx in 1..upto {
        if st.pos[idx] != goal_pos[idx] {
            mismatch += 1;
        }
    }
    mismatch
}

fn segment_candidate_rank(st: &State, goal_pos: &[u16], min_col: usize) -> (usize, usize, usize, usize, usize) {
    let forbidden_col_cells = st
        .pos
        .iter()
        .skip(1)
        .filter(|&&cid| rc_of(cid, st.n).1 == min_col)
        .count();
    let head_target = if st.pos.len() < goal_pos.len() {
        goal_pos[st.pos.len().min(goal_pos.len() - 1)]
    } else {
        goal_pos[0]
    };
    (
        scaffold_food_penalty(st, goal_pos),
        forbidden_col_cells,
        scaffold_body_mismatch(st, goal_pos),
        manhattan(st.n, st.head(), head_target),
        4usize.saturating_sub(legal_dirs(st, min_col).len()),
    )
}

fn make_segments(d: &[u8], n: usize) -> Vec<Vec<u8>> {
    let chunk = n - 2;
    let tail = &d[5..];
    let mut segs = Vec::new();
    let mut r = tail.len();
    while r > 0 {
        let l = r.saturating_sub(chunk);
        segs.push(tail[l..r].to_vec());
        r = l;
    }
    segs
}

fn build_one_color_exact(
    start_bs: &BeamState,
    target_color: u8,
    min_col: usize,
    anchor: u16,
    started: &Instant,
) -> Vec<BeamState> {
    let mut nodes = Vec::with_capacity(BUILD_ONE_EXPANSION_CAP.min(20_000) + 8);
    let mut depths = Vec::with_capacity(BUILD_ONE_EXPANSION_CAP.min(20_000) + 8);
    let mut q = VecDeque::new();
    let mut seen = FxHashMap::<Key, usize>::default();
    let mut sol_keys: FxHashSet<Key> = FxHashSet::default();
    let mut sols = Vec::new();
    let mut first_sol_depth: Option<usize> = None;
    let mut expansions = 0usize;

    nodes.push(ShapeNode {
        state: start_bs.state.clone(),
        parent: None,
        mv: '\0',
    });
    depths.push(0);
    q.push_back(0usize);
    seen.insert(encode_key(&start_bs.state), 0);

    while let Some(idx) = q.pop_front() {
        if expansions >= BUILD_ONE_EXPANSION_CAP || time_over(started) {
            break;
        }
        let depth = depths[idx];
        if let Some(best) = first_sol_depth
            && depth > best + BUILD_ONE_DEPTH_SLACK
        {
            break;
        }
        expansions += 1;

        let st = nodes[idx].state.clone();
        for dir in legal_dirs(&st, min_col) {
            let Some((ns, ate, bite_idx, _)) = step(&st, dir, min_col) else {
                continue;
            };
            if bite_idx.is_some() {
                continue;
            }
            if ate != 0 && ate != target_color {
                continue;
            }

            let nd = depth + 1;
            if ate == target_color {
                let mut ops = start_bs.ops.clone();
                ops.push_str(&reconstruct_shape(&nodes, idx));
                ops.push(DIRS[dir].2);
                let bs = BeamState { state: ns, ops };
                let key = encode_key(&bs.state);
                if sol_keys.insert(key) {
                    sols.push(bs);
                    if first_sol_depth.is_none() {
                        first_sol_depth = Some(nd);
                    }
                    if sols.len() >= BUILD_ONE_SOL_CAP {
                        break;
                    }
                }
                continue;
            }

            let key = encode_key(&ns);
            if seen.get(&key).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(key, nd);
            nodes.push(ShapeNode {
                state: ns,
                parent: Some(idx),
                mv: DIRS[dir].2,
            });
            depths.push(nd);
            q.push_back(nodes.len() - 1);
        }
        if sols.len() >= BUILD_ONE_SOL_CAP {
            break;
        }
    }

    sols.sort_unstable_by_key(|bs| (build_rank(&bs.state, anchor, min_col), bs.ops.len()));
    if sols.len() > BUILD_ONE_SOL_CAP {
        sols.truncate(BUILD_ONE_SOL_CAP);
    }
    sols
}

fn build_segment(
    cur: &BeamState,
    seg: &[u8],
    min_col: usize,
    anchor: u16,
    started: &Instant,
) -> (Vec<BeamState>, BeamState) {
    let target = build_segment_target(seg);
    let mut beam = vec![cur.clone()];
    let mut best_partial = cur.clone();

    for &target_color in seg {
        if time_over(started) {
            break;
        }
        let mut next_map = FxHashMap::<Key, BeamState>::default();
        for bs in &beam {
            for cand in build_one_color_exact(bs, target_color, min_col, anchor, started) {
                let key = encode_key(&cand.state);
                match next_map.get_mut(&key) {
                    Some(prev) => {
                        if cand.ops.len() < prev.ops.len() {
                            *prev = cand;
                        }
                    }
                    None => {
                        next_map.insert(key, cand);
                    }
                }
            }
        }
        if next_map.is_empty() {
            return (Vec::new(), best_partial);
        }
        let mut next_beam: Vec<BeamState> = next_map.into_values().collect();
        next_beam.sort_unstable_by_key(|bs| (build_rank(&bs.state, anchor, min_col), bs.ops.len()));
        if next_beam.len() > SEGMENT_BEAM_WIDTH {
            next_beam.truncate(SEGMENT_BEAM_WIDTH);
        }
        best_partial = next_beam[0].clone();
        beam = next_beam;
    }

    beam.retain(|bs| bs.state.colors == target);
    beam.sort_unstable_by_key(|bs| (build_rank(&bs.state, anchor, min_col), bs.ops.len()));
    (beam, best_partial)
}

fn segment_search_rank(st: &State, target: &[u8], goal_pos: &[u16], min_col: usize) -> (usize, usize, usize, usize, usize) {
    let built = st.colors.len().min(target.len());
    let remaining = target.len() - built;
    let next_need_dist = if built < target.len() {
        nearest_food_dist(st, target[built], min_col).0
    } else {
        0
    };
    let col_penalty = if built == target.len() && st.pos != goal_pos {
        st.pos.iter().filter(|&&cid| rc_of(cid, st.n).1 == min_col).count()
    } else {
        0
    };
    let (shape_mismatch, shape_dist, head_dist) = pos_goal_rank(st.n, &st.pos, goal_pos);
    (remaining, col_penalty, next_need_dist, shape_mismatch + head_dist, shape_dist)
}

fn build_segment_to_goal(
    cur: &BeamState,
    seg: &[u8],
    min_col: usize,
    goal_pos: &[u16],
    started: &Instant,
) -> (Option<BeamState>, BeamState) {
    let target = build_segment_target(seg);
    let mut beam = vec![cur.clone()];
    let mut best_partial = cur.clone();

    for ell in 5..target.len() {
        if time_over(started) {
            break;
        }
        let mut next_map = FxHashMap::<Key, BeamState>::default();
        for bs in &beam {
            let mut sols =
                collect_exact_solutions(bs, &target, ell, target[ell], min_col, MAX_TARGETS_PER_STAGE, started);
            if sols.is_empty() && !time_over(started) {
                sols =
                    stage_search_bestfirst(bs, &target, ell, min_col, &BUDGETS_NORMAL, STAGE_BEAM * 2, started);
            }
            for cand in sols {
                if cand.state.colors.len() != ell + 1 || cand.state.colors[..ell + 1] != target[..ell + 1] {
                    continue;
                }
                if cand.ops.len() > best_partial.ops.len() {
                    best_partial = cand.clone();
                }
                let key = encode_key(&cand.state);
                match next_map.get_mut(&key) {
                    Some(prev) => {
                        if cand.ops.len() < prev.ops.len() {
                            *prev = cand;
                        }
                    }
                    None => {
                        next_map.insert(key, cand);
                    }
                }
            }
        }
        if next_map.is_empty() {
            return (None, best_partial);
        }

        let mut next_beam: Vec<BeamState> = next_map.into_values().collect();
        next_beam.sort_unstable_by_key(|bs| (segment_candidate_rank(&bs.state, goal_pos, min_col), bs.ops.len()));
        if next_beam.len() > SEGMENT_SEARCH_BEAM {
            next_beam.truncate(SEGMENT_SEARCH_BEAM);
        }
        if let Some(front) = next_beam.first()
            && front.ops.len() > best_partial.ops.len()
        {
            best_partial = front.clone();
        }
        beam = next_beam;
    }

    let mut exact_beam: Vec<BeamState> = beam.into_iter().filter(|bs| bs.state.colors == target).collect();
    exact_beam.sort_unstable_by_key(|bs| {
        (
            segment_candidate_rank(&bs.state, goal_pos, min_col),
            pos_goal_rank(bs.state.n, &bs.state.pos, goal_pos),
            bs.ops.len(),
        )
    });
    for bs in exact_beam {
        if let Some(shaped) = route_to_deposit_shape(&bs, goal_pos, min_col, started) {
            return (Some(shaped), best_partial);
        }
    }

    (None, best_partial)
}

fn build_segment_to_shape(
    cur: &BeamState,
    seg: &[u8],
    col: usize,
    started: &Instant,
) -> (Option<BeamState>, BeamState) {
    let Some(goal_pos) = build_deposit_ready_positions(cur.state.n, col, seg.len()) else {
        return (None, cur.clone());
    };
    build_segment_to_goal(cur, seg, col, &goal_pos, started)
}

fn build_last_segment_for_collect(
    cur: &BeamState,
    seg: &[u8],
    staging_col: usize,
    started: &Instant,
) -> (Option<BeamState>, BeamState) {
    let anchor = collect_start_cell(staging_col, cur.state.n).unwrap_or_else(|| cell_of(0, staging_col, cur.state.n));
    let (beam, best_partial) = build_segment(cur, seg, staging_col, anchor, started);
    if beam.is_empty() {
        return (None, best_partial);
    }
    if staging_col == 0 {
        return (Some(beam[0].clone()), best_partial);
    }
    let goal = collect_start_cell(staging_col, cur.state.n).unwrap();
    let mut best = best_partial;
    let mut exact_beam = beam;
    exact_beam.sort_unstable_by_key(|bs| (manhattan(bs.state.n, bs.state.head(), goal), bs.ops.len()));
    for bs in exact_beam {
        if bs.ops.len() > best.ops.len() {
            best = bs.clone();
        }
        if let Some(routed) = navigate_empty_to_goal(&bs, goal, staging_col, started) {
            return (Some(routed), best);
        }
    }
    (None, best)
}

fn route_to_deposit_shape(
    bs: &BeamState,
    goal_pos: &[u16],
    min_col: usize,
    started: &Instant,
) -> Option<BeamState> {
    if bs.state.pos == goal_pos {
        return Some(bs.clone());
    }
    if bs.state.pos.iter().any(|&cid| rc_of(cid, bs.state.n).1 == min_col) {
        return None;
    }

    let mut nodes = Vec::with_capacity(SHAPE_EXPANSION_CAP.min(80_000) + 8);
    nodes.push(PosNode {
        pos: bs.state.pos.clone(),
        parent: None,
        mv: '\0',
    });

    let mut uid = 0usize;
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((pos_goal_rank(bs.state.n, &bs.state.pos, goal_pos), 0usize, uid, 0usize)));
    uid += 1;

    let mut seen = FxHashMap::<Vec<u16>, usize>::default();
    seen.insert(bs.state.pos.clone(), 0);
    let mut expansions = 0usize;

    while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
        if expansions >= SHAPE_EXPANSION_CAP || time_over(started) {
            break;
        }
        expansions += 1;
        let pos = nodes[idx].pos.clone();
        if pos == goal_pos {
            let mut rev = Vec::new();
            let mut cur = idx;
            while let Some(parent) = nodes[cur].parent {
                rev.push(nodes[cur].mv);
                cur = parent;
            }
            rev.reverse();

            let mut state = bs.state.clone();
            let mut ops = bs.ops.clone();
            for ch in rev {
                let dir = dir_of_char(ch as u8).unwrap();
                let (ns, ate, bite_idx, _) = step(&state, dir, min_col)?;
                if ate != 0 || bite_idx.is_some() {
                    return None;
                }
                state = ns;
                ops.push(ch);
            }
            return Some(BeamState { state, ops });
        }
        if depth >= SHAPE_DEPTH_LIMIT {
            continue;
        }

        for dir in 0..4 {
            let Some(next_pos) = empty_step_pos(bs.state.n, &bs.state.food, &pos, dir, min_col) else {
                continue;
            };
            if next_pos != goal_pos && next_pos.iter().any(|&cid| rc_of(cid, bs.state.n).1 == min_col) {
                continue;
            }
            let nd = depth + 1;
            if seen.get(&next_pos).copied().unwrap_or(usize::MAX) <= nd {
                continue;
            }
            seen.insert(next_pos.clone(), nd);
            let child = nodes.len();
            nodes.push(PosNode {
                pos: next_pos.clone(),
                parent: Some(idx),
                mv: DIRS[dir].2,
            });
            pq.push(Reverse((pos_goal_rank(bs.state.n, &next_pos, goal_pos), nd, uid, child)));
            uid += 1;
        }
    }

    None
}

fn route_to_anchor(bs: &BeamState, goal: u16, min_col: usize, started: &Instant) -> Option<BeamState> {
    navigate_empty_to_goal(bs, goal, min_col, started)
}

fn try_store_one_segment(cur: &BeamState, seg: &[u8], col: usize, started: &Instant) -> (Option<BeamState>, BeamState) {
    let n = cur.state.n;
    let deposit = deposit_ops(n, col);
    let (shaped, mut best_partial) = build_segment_to_shape(cur, seg, col, started);
    let Some(shaped) = shaped else {
        return (None, best_partial);
    };
    if shaped.ops.len() > best_partial.ops.len() {
        best_partial = shaped.clone();
    }
    let placed_partial = apply_ops_partial(&shaped, &deposit, col);
    if placed_partial.ops.len() > best_partial.ops.len() {
        best_partial = placed_partial.clone();
    }
    let Some(placed) = apply_ops(&shaped, &deposit, col) else {
        return (None, best_partial);
    };
    if !locked_columns_preserved(&cur.state.food, &placed.state.food, n, col) {
        return (None, best_partial);
    }
    if segment_drop_ok(&placed.state, seg, col) {
        (Some(placed), best_partial)
    } else {
        (None, best_partial)
    }
}

fn segment_strategy(input: &Input, started: &Instant) -> BeamState {
    let segs = make_segments(&input.d, input.n);
    let mut cur = BeamState {
        state: State::initial(input),
        ops: String::new(),
    };
    if segs.is_empty() {
        return cur;
    }
    let stored_count = segs.len().saturating_sub(1);

    for (col, seg) in segs.iter().take(stored_count).enumerate() {
        if time_over(started) {
            return cur;
        }
        let (next, partial) = try_store_one_segment(&cur, seg, col, started);
        let Some(next) = next else {
            return partial;
        };
        debug_assert!(segment_drop_ok(&next.state, seg, col));
        cur = next;
    }

    let last_seg = &segs[stored_count];
    let (carried, partial) = build_last_segment_for_collect(&cur, last_seg, stored_count, started);
    let Some(carried) = carried else {
        return partial;
    };
    if stored_count == 0 {
        return carried;
    }

    let collect = collect_all(stored_count, input.n);
    let finished_partial = apply_ops_partial(&carried, &collect, 0);
    apply_ops(&carried, &collect, 0)
        .filter(|bs| bs.state.colors == input.d)
        .unwrap_or(finished_partial)
}

fn solve(input: &Input) -> String {
    let started = Instant::now();
    let mut out = segment_strategy(input, &started);
    if out.ops.len() > MAX_TURNS {
        out.ops.truncate(MAX_TURNS);
    }
    out.ops
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
