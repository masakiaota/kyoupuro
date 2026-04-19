
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
const SEGMENT_TRY_SEC: f64 = 0.70;
const SEGMENT_MIN_LEFT_SEC_FOR_FALLBACK: f64 = 0.95;
const BUILD_NODE_LIMIT: usize = 60_000;
const ROUTE_NODE_LIMIT: usize = 80_000;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];

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
struct Dropped {
    cell: u16,
    color: u8,
}

#[derive(Clone)]
struct PosNode {
    pos: Vec<u16>,
    parent: Option<usize>,
    mv: char,
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
fn next_head_cell(st: &State, dir: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(st.head(), st.n);
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

fn step(st: &State, dir: usize) -> (State, u8, Option<usize>, Vec<Dropped>) {
    let nh = next_head_cell(st, dir).unwrap();
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

    (
        State {
            n: st.n,
            food,
            pos: new_pos,
            colors: new_colors,
        },
        ate,
        bite_idx,
        dropped,
    )
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
fn time_over(started: &Instant) -> bool {
    started.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
}

#[inline]
fn time_left(started: &Instant) -> f64 {
    (TIME_LIMIT_SEC - started.elapsed().as_secs_f64()).max(0.0)
}

#[inline]
fn segment_try_over(started: &Instant) -> bool {
    started.elapsed().as_secs_f64() >= SEGMENT_TRY_SEC
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

fn make_segments(d: &[u8], n: usize) -> Vec<Vec<u8>> {
    let tail = &d[5..];
    let mut segs = Vec::new();
    let mut r = tail.len();
    let seg_len = n.saturating_sub(2);
    while r > 0 {
        let l = r.saturating_sub(seg_len);
        segs.push(tail[l..r].to_vec());
        r = l;
    }
    segs
}

fn reconstruct_pos_path(nodes: &[PosNode], mut idx: usize, last: Option<char>) -> String {
    let mut rev = Vec::new();
    if let Some(ch) = last {
        rev.push(ch);
    }
    while let Some(parent) = nodes[idx].parent {
        rev.push(nodes[idx].mv);
        idx = parent;
    }
    rev.reverse();
    rev.into_iter().collect()
}

#[inline]
fn next_head_pos(n: usize, pos: &[u16], dir: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(pos[0], n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
        return None;
    }
    Some(cell_of(nr as usize, nc as usize, n))
}

#[inline]
fn is_legal_dir_pos(n: usize, pos: &[u16], dir: usize) -> bool {
    let Some(nh) = next_head_pos(n, pos, dir) else {
        return false;
    };
    pos.len() < 2 || nh != pos[1]
}

fn empty_step_pos(n: usize, pos: &[u16], dir: usize, min_col: usize) -> Option<Vec<u16>> {
    if !is_legal_dir_pos(n, pos, dir) {
        return None;
    }
    let nh = next_head_pos(n, pos, dir)?;
    let (_, c) = rc_of(nh, n);
    if c < min_col {
        return None;
    }

    let mut new_pos = Vec::with_capacity(pos.len());
    new_pos.push(nh);
    new_pos.extend_from_slice(&pos[..pos.len() - 1]);

    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            return None;
        }
    }
    Some(new_pos)
}

fn build_one_color_exact(
    st: &State,
    target_color: u8,
    min_col: usize,
    reserved: &HashSet<u16>,
    started: &Instant,
) -> Option<String> {
    let mut nodes = Vec::with_capacity(BUILD_NODE_LIMIT.min(20_000) + 8);
    nodes.push(PosNode {
        pos: st.pos.clone(),
        parent: None,
        mv: '\0',
    });

    let mut q = VecDeque::new();
    q.push_back(0usize);

    let mut seen: HashSet<Vec<u16>> = HashSet::new();
    seen.insert(st.pos.clone());

    while let Some(idx) = q.pop_front() {
        if time_over(started) || segment_try_over(started) || time_left(started) < SEGMENT_MIN_LEFT_SEC_FOR_FALLBACK {
            return None;
        }
        if nodes.len() >= BUILD_NODE_LIMIT {
            return None;
        }

        let pos = nodes[idx].pos.clone();
        for dir in 0..4 {
            if !is_legal_dir_pos(st.n, &pos, dir) {
                continue;
            }
            let nh = next_head_pos(st.n, &pos, dir).unwrap();
            let (_, c) = rc_of(nh, st.n);
            if c < min_col {
                continue;
            }

            let col = st.food[nh as usize];
            if col != 0 {
                if col == target_color && !reserved.contains(&nh) {
                    return Some(reconstruct_pos_path(&nodes, idx, Some(DIRS[dir].2)));
                }
                continue;
            }

            let Some(next_pos) = empty_step_pos(st.n, &pos, dir, min_col) else {
                continue;
            };
            if seen.insert(next_pos.clone()) {
                nodes.push(PosNode {
                    pos: next_pos,
                    parent: Some(idx),
                    mv: DIRS[dir].2,
                });
                q.push_back(nodes.len() - 1);
            }
        }
    }

    None
}

fn build_segment(
    st: &mut State,
    seg_prefix: &[u8],
    min_col: usize,
    reserved: &HashSet<u16>,
    ops: &mut String,
    started: &Instant,
) -> bool {
    for &target in seg_prefix {
        let Some(plan) = build_one_color_exact(st, target, min_col, reserved, started) else {
            return false;
        };
        if !apply_ops(st, &plan, ops) {
            return false;
        }
        if st.colors.last().copied() != Some(target) {
            return false;
        }
    }
    true
}

fn route_to_anchor(
    st: &State,
    goal: u16,
    min_col: usize,
    started: &Instant,
) -> Option<String> {
    if st.food[goal as usize] != 0 {
        return None;
    }
    if st.head() == goal {
        return Some(String::new());
    }

    let mut nodes = Vec::with_capacity(ROUTE_NODE_LIMIT.min(25_000) + 8);
    nodes.push(PosNode {
        pos: st.pos.clone(),
        parent: None,
        mv: '\0',
    });

    let mut q = VecDeque::new();
    q.push_back(0usize);

    let mut seen: HashSet<Vec<u16>> = HashSet::new();
    seen.insert(st.pos.clone());

    while let Some(idx) = q.pop_front() {
        if time_over(started) || segment_try_over(started) || time_left(started) < SEGMENT_MIN_LEFT_SEC_FOR_FALLBACK {
            return None;
        }
        if nodes.len() >= ROUTE_NODE_LIMIT {
            return None;
        }

        let pos = nodes[idx].pos.clone();
        for dir in 0..4 {
            if !is_legal_dir_pos(st.n, &pos, dir) {
                continue;
            }
            let nh = next_head_pos(st.n, &pos, dir).unwrap();
            let (_, c) = rc_of(nh, st.n);
            if c < min_col {
                continue;
            }
            if st.food[nh as usize] != 0 {
                continue;
            }

            let Some(next_pos) = empty_step_pos(st.n, &pos, dir, min_col) else {
                continue;
            };
            if nh == goal {
                nodes.push(PosNode {
                    pos: next_pos,
                    parent: Some(idx),
                    mv: DIRS[dir].2,
                });
                return Some(reconstruct_pos_path(&nodes, nodes.len() - 1, None));
            }
            if seen.insert(next_pos.clone()) {
                nodes.push(PosNode {
                    pos: next_pos,
                    parent: Some(idx),
                    mv: DIRS[dir].2,
                });
                q.push_back(nodes.len() - 1);
            }
        }
    }

    None
}

fn deposit_even(n: usize) -> String {
    let mut s = String::with_capacity(n + 5);
    s.push('L');
    for _ in 0..(n - 1) {
        s.push('U');
    }
    s.push_str("RDLUR");
    s
}

fn deposit_odd(n: usize) -> String {
    let mut s = String::with_capacity(n + 5);
    s.push('L');
    for _ in 0..(n - 1) {
        s.push('D');
    }
    s.push_str("RULDR");
    s
}

fn collect_all(seg_count: usize, n: usize) -> String {
    if seg_count == 0 {
        return String::new();
    }

    let mut s = String::new();
    let last = seg_count - 1;

    if last % 2 == 0 {
        s.push('D');
        s.push('L');
        for _ in 0..(n - 2) {
            s.push('D');
        }
    } else {
        s.push('U');
        s.push('L');
        for _ in 0..(n - 2) {
            s.push('U');
        }
    }

    for c in (0..last).rev() {
        s.push('L');
        let mv = if c % 2 == 0 { 'D' } else { 'U' };
        for _ in 0..(n - 1) {
            s.push(mv);
        }
    }
    s
}

fn apply_ops(st: &mut State, plan: &str, ops: &mut String) -> bool {
    for ch in plan.bytes() {
        if ops.len() >= MAX_TURNS {
            return false;
        }
        let Some(dir) = dir_of_char(ch) else {
            return false;
        };
        if !is_legal_dir(st, dir) {
            return false;
        }
        let (ns, _, _, _) = step(st, dir);
        *st = ns;
        ops.push(ch as char);
    }
    true
}

#[inline]
fn remaining_food_count(st: &State) -> usize {
    st.food.iter().filter(|&&c| c != 0).count()
}

fn carrier_ok(st: &State) -> bool {
    st.colors.len() == 5 && st.colors.iter().all(|&x| x == 1)
}

fn exact_build_prefix_ok(st: &State, seg_prefix: &[u8]) -> bool {
    if st.colors.len() != 5 + seg_prefix.len() {
        return false;
    }
    st.colors[..5].iter().all(|&x| x == 1) && st.colors[5..] == *seg_prefix
}

fn deposit_trace_cells(n: usize, col: usize, even: bool) -> Vec<u16> {
    let mut cells = Vec::with_capacity(n + 2);
    if even {
        for r in (0..=n - 1).rev() {
            cells.push(cell_of(r, col, n));
        }
        cells.push(cell_of(0, col + 1, n));
        cells.push(cell_of(1, col + 1, n));
    } else {
        for r in 0..=n - 1 {
            cells.push(cell_of(r, col, n));
        }
        cells.push(cell_of(n - 1, col + 1, n));
        cells.push(cell_of(n - 2, col + 1, n));
    }
    cells
}

fn deposit_reserved_cells_and_colors(st: &State, col: usize, even: bool) -> (Vec<u16>, Vec<u8>) {
    let mut cells = Vec::new();
    let mut colors = Vec::new();
    for cell in deposit_trace_cells(st.n, col, even) {
        let color = st.food[cell as usize];
        if color != 0 {
            cells.push(cell);
            colors.push(color);
        }
    }
    (cells, colors)
}

fn debug_check_column_layout(st: &State, col: usize, seg: &[u8], even: bool) {
    if even {
        for r in 0..st.n {
            let want = if r >= 2 && r - 2 < seg.len() {
                seg[r - 2]
            } else {
                0
            };
            debug_assert_eq!(st.food[cell_of(r, col, st.n) as usize], want);
        }
    } else {
        for r in 0..st.n {
            let mut want = 0u8;
            for (t, &color) in seg.iter().enumerate() {
                if r == st.n - 3 - t {
                    want = color;
                    break;
                }
            }
            debug_assert_eq!(st.food[cell_of(r, col, st.n) as usize], want);
        }
    }
}

fn is_complete_exact(st: &State, input: &Input) -> bool {
    st.colors == input.d && st.colors.len() == input.m && remaining_food_count(st) == 0
}

fn solve_segment(input: &Input, started: &Instant) -> Option<String> {
    let segs = make_segments(&input.d, input.n);
    if segs.is_empty() {
        return Some(String::new());
    }
    if segs.len() >= input.n {
        return None;
    }

    let mut st = State::initial(input);
    let mut ops = String::new();
    let mut stored: Vec<Vec<u8>> = Vec::new();

    for (i, seg_full) in segs.iter().enumerate() {
        if time_over(started) || segment_try_over(started) || time_left(started) < SEGMENT_MIN_LEFT_SEC_FOR_FALLBACK {
            return None;
        }
        if !carrier_ok(&st) {
            return None;
        }

        let even = i % 2 == 0;
        let work_min_col = i + 1;
        let anchor = if even {
            cell_of(input.n - 1, i + 1, input.n)
        } else {
            cell_of(0, i + 1, input.n)
        };
        if st.food[anchor as usize] != 0 {
            return None;
        }

        let (reserved_cells_vec, reserved_colors) = deposit_reserved_cells_and_colors(&st, i, even);
        if reserved_colors.len() > seg_full.len() {
            return None;
        }
        if !seg_full.as_slice().ends_with(reserved_colors.as_slice()) {
            return None;
        }
        let build_len = seg_full.len() - reserved_colors.len();
        let build_prefix = &seg_full[..build_len];
        let reserved_cells: HashSet<u16> = reserved_cells_vec.into_iter().collect();

        if !build_segment(&mut st, build_prefix, work_min_col, &reserved_cells, &mut ops, started) {
            return None;
        }
        if !exact_build_prefix_ok(&st, build_prefix) {
            return None;
        }

        let Some(route) = route_to_anchor(&st, anchor, work_min_col, started) else {
            return None;
        };
        if !apply_ops(&mut st, &route, &mut ops) {
            return None;
        }
        if st.head() != anchor {
            return None;
        }
        if st.food[anchor as usize] != 0 {
            return None;
        }

        let dep = if even {
            deposit_even(input.n)
        } else {
            deposit_odd(input.n)
        };
        if !apply_ops(&mut st, &dep, &mut ops) {
            return None;
        }
        if !carrier_ok(&st) {
            return None;
        }

        stored.push(seg_full.clone());

        debug_assert!(carrier_ok(&st));
        debug_assert_eq!(ops.len() <= MAX_TURNS, true);
        for c in 0..stored.len() {
            debug_check_column_layout(&st, c, &stored[c], c % 2 == 0);
        }
        if i + 1 < input.n {
            let expected_head = if even {
                cell_of(0, i + 1, input.n)
            } else {
                cell_of(input.n - 1, i + 1, input.n)
            };
            debug_assert_eq!(st.head(), expected_head);
        }
    }

    let collect = collect_all(segs.len(), input.n);
    if !apply_ops(&mut st, &collect, &mut ops) {
        return None;
    }
    if is_complete_exact(&st, input) {
        Some(ops)
    } else {
        None
    }
}

mod fallback_v005 {
    use std::collections::{HashMap, VecDeque};
    use std::time::Instant;

    use crate::{Input, DIRS, MAX_TURNS, TIME_LIMIT_SEC};

    const VISIT_REPEAT_LIMIT: u16 = 12;

    #[derive(Clone)]
    struct State {
        n: usize,
        food: Vec<u8>,
        pos: Vec<(usize, usize)>,
        colors: Vec<u8>,
    }

    #[derive(Clone, Debug, Default)]
    struct MoveOutcome {
        ate: Option<u8>,
        bite: bool,
        dropped: Vec<(usize, usize, u8)>,
    }

    #[derive(Clone)]
    struct Solver {
        input: Input,
        state: State,
        ops: Vec<u8>,
        start: Instant,
    }

    #[derive(Clone, Copy)]
    struct PlanConfig {
        depth_limit: u8,
        node_limit: usize,
        non_target_limit: u8,
        bite_limit: u8,
    }

    #[derive(Clone)]
    struct SearchNode {
        state: State,
        parent: usize,
        dir: u8,
        depth: u8,
        non_target: u8,
        bite: u8,
    }

    impl Solver {
        fn new(input: &Input, start: Instant) -> Self {
            let pos = vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)];
            let colors = vec![1_u8; 5];
            let state = State {
                n: input.n,
                food: input.food.clone(),
                pos,
                colors,
            };
            Self {
                input: input.clone(),
                state,
                ops: Vec::new(),
                start,
            }
        }

        fn solve(&mut self) {
            let mut ell = 5_usize;
            while ell < self.input.m && self.ops.len() < MAX_TURNS && !self.time_over() {
                if !self.prefix_match(ell) {
                    break;
                }
                if !self.extend_one(ell) {
                    break;
                }
                ell += 1;
            }
        }

        fn extend_one(&mut self, ell: usize) -> bool {
            let target_color = self.input.d[ell];
            if self.collect_food_cells(target_color).is_empty() {
                return false;
            }

            let safe_cfg = PlanConfig {
                depth_limit: 10,
                node_limit: 3_000,
                non_target_limit: 0,
                bite_limit: 0,
            };
            if let Some(plan) = self.plan_color_goal(ell, target_color, safe_cfg) {
                if self.apply_plan_and_check(ell, &plan) {
                    return true;
                }
            }

            let rescue_cfg = PlanConfig {
                depth_limit: 16,
                node_limit: 10_000,
                non_target_limit: 6,
                bite_limit: 2,
            };
            if let Some(plan) = self.plan_color_goal(ell, target_color, rescue_cfg) {
                if self.apply_plan_and_check(ell, &plan) {
                    return true;
                }
            }

            let mut targets = self.collect_food_cells(target_color);
            let head = self.state.pos[0];
            targets.sort_by_key(|&(r, c)| manhattan(head, (r, c)));

            for target in targets {
                if self.try_target(ell, target, target_color) {
                    return true;
                }
            }
            false
        }

        fn try_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
            let mut cand = self.neighbors(target);
            let head = self.state.pos[0];
            cand.sort_by_key(|&p| {
                (
                    usize::from(self.state.food[self.idx(p.0, p.1)] > 0),
                    manhattan(head, p),
                )
            });

            for goal in cand {
                let backup_state = self.state.clone();
                let backup_len = self.ops.len();

                if self.navigate_to_goal_safe(goal, target)
                    && self.shrink_to_ell(ell, target, target_color)
                    && self.finish_eat_target(ell, target)
                {
                    return true;
                }

                self.state = backup_state.clone();
                self.ops.truncate(backup_len);

                if self.navigate_to_goal(goal, target, ell)
                    && self.shrink_to_ell(ell, target, target_color)
                    && self.finish_eat_target(ell, target)
                {
                    return true;
                }

                self.state = backup_state;
                self.ops.truncate(backup_len);
            }

            false
        }

        fn finish_eat_target(&mut self, ell: usize, target: (usize, usize)) -> bool {
            let head = self.state.pos[0];
            let Some(dir) = dir_between(head, target) else {
                return false;
            };
            if !self.step(dir) {
                return false;
            }
            self.prefix_match(ell + 1)
        }

        fn navigate_to_goal(
            &mut self,
            goal: (usize, usize),
            target: (usize, usize),
            ell: usize,
        ) -> bool {
            let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
            let mut seen = HashMap::new();
            let mut bite_count = 0_usize;
            let bite_limit = self.input.n * self.input.n * 4;
            let mut guard = 0_usize;
            while self.state.pos[0] != goal || !restore_queue.is_empty() {
                guard += 1;
                if guard > self.input.n * self.input.n * 80 {
                    return false;
                }
                if self.visit_over_limit(&mut seen, goal, restore_queue.len()) {
                    return false;
                }

                let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                    let head = self.state.pos[0];
                    dir_between(head, (r, c)).unwrap()
                } else {
                    let mut next = self.bfs_next_dir(goal, target, true);
                    if next.is_none() {
                        next = self.bfs_next_dir(goal, target, false);
                    }
                    let Some(dir) = next else {
                        return false;
                    };
                    dir
                };

                let Some(outcome) =
                    self.advance_with_restore_queue(dir, target, ell, &mut restore_queue)
                else {
                    return false;
                };
                if outcome.bite {
                    bite_count += 1;
                    if bite_count > bite_limit {
                        return false;
                    }
                }
            }

            self.state.colors.len() >= ell
        }

        fn navigate_to_goal_safe(&mut self, goal: (usize, usize), target: (usize, usize)) -> bool {
            let mut seen = HashMap::new();
            let mut guard = 0_usize;
            while self.state.pos[0] != goal {
                guard += 1;
                if guard > self.input.n * self.input.n * 30 {
                    return false;
                }
                if self.visit_over_limit(&mut seen, goal, 0) {
                    return false;
                }
                let Some(dir) = self.bfs_next_dir_strict(goal, target) else {
                    return false;
                };
                let Some(outcome) = self.step_with_outcome(dir) else {
                    return false;
                };
                if self.state.food[self.idx(target.0, target.1)] == 0 {
                    return false;
                }
                if outcome.bite || outcome.ate.is_some() {
                    return false;
                }
            }
            true
        }

        fn shrink_to_ell(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
            if self.state.colors.len() == ell {
                return self.can_reach_target_next(target);
            }

            let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
            let mut seen = HashMap::new();
            let mut bite_count = 0_usize;
            let bite_limit = self.input.n * self.input.n * 3;
            let mut guard = 0_usize;
            while self.state.colors.len() != ell
                || !restore_queue.is_empty()
                || !self.can_reach_target_next(target)
            {
                guard += 1;
                if guard > self.input.n * self.input.n * 60 {
                    return false;
                }
                if self.visit_over_limit(&mut seen, target, restore_queue.len()) {
                    return false;
                }

                let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                    let head = self.state.pos[0];
                    dir_between(head, (r, c)).unwrap()
                } else {
                    let Some(dir) = self.choose_shrink_dir(ell, target) else {
                        return false;
                    };
                    dir
                };

                let Some(outcome) =
                    self.advance_with_restore_queue(dir, target, ell, &mut restore_queue)
                else {
                    return false;
                };
                if outcome.bite {
                    bite_count += 1;
                    if bite_count > bite_limit {
                        return false;
                    }
                }
            }

            self.state.colors.len() == ell
                && self.state.food[self.idx(target.0, target.1)] == target_color
                && self.can_reach_target_next(target)
        }

        fn choose_shrink_dir(&self, ell: usize, target: (usize, usize)) -> Option<usize> {
            let anchor_idx = ell.saturating_sub(1).min(self.state.pos.len() - 1);
            let anchor = self.state.pos[anchor_idx];

            let mut best_bite: Option<((usize, usize, usize, usize), usize)> = None;
            let mut best_move: Option<((usize, usize, usize, usize), usize)> = None;

            for dir in legal_dirs(&self.state) {
                let nh = next_head(&self.state, dir);
                if nh == target {
                    continue;
                }

                let mut sim = self.state.clone();
                let out = apply_move(&mut sim, dir);
                if sim.food[self.idx(target.0, target.1)] == 0 {
                    continue;
                }
                let keep = sim.colors.len().min(ell);
                if !prefix_match_upto_state(&sim, &self.input.d, keep) {
                    continue;
                }

                if out.bite {
                    let under = usize::from(sim.colors.len() < ell);
                    let dist_len = sim.colors.len().abs_diff(ell);
                    let not_ready = usize::from(!can_reach_target_next_state(&sim, target));
                    let target_dist = manhattan(sim.pos[0], target);
                    let anchor_dist = manhattan(sim.pos[0], anchor);
                    let key = (under, dist_len, not_ready, target_dist + anchor_dist);
                    if best_bite.as_ref().map_or(true, |(k, _)| key < *k) {
                        best_bite = Some((key, dir));
                    }
                } else {
                    let len_gap = sim.colors.len().abs_diff(ell);
                    let not_ready = usize::from(!can_reach_target_next_state(&sim, target));
                    let target_dist = manhattan(sim.pos[0], target);
                    let anchor_dist = manhattan(sim.pos[0], anchor);
                    let ate_penalty = usize::from(out.ate.is_some());
                    let key = (len_gap, not_ready, target_dist + anchor_dist, ate_penalty);
                    if best_move.as_ref().map_or(true, |(k, _)| key < *k) {
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

        fn bfs_next_dir(
            &self,
            goal: (usize, usize),
            target: (usize, usize),
            avoid_food: bool,
        ) -> Option<usize> {
            let n = self.state.n;
            let start = self.state.pos[0];
            if start == goal {
                return Some(0);
            }

            let mut blocked = vec![false; n * n];
            if avoid_food {
                for r in 0..n {
                    for c in 0..n {
                        let p = (r, c);
                        if p != goal && p != target && self.state.food[self.idx(r, c)] > 0 {
                            blocked[self.idx(r, c)] = true;
                        }
                    }
                }
            }

            blocked[self.idx(start.0, start.1)] = false;
            blocked[self.idx(goal.0, goal.1)] = false;

            let mut q = VecDeque::new();
            let mut dist = vec![usize::MAX; n * n];
            let mut first_dir = vec![None::<usize>; n * n];

            let sid = self.idx(start.0, start.1);
            dist[sid] = 0;

            for dir in legal_dirs(&self.state) {
                let np = next_head(&self.state, dir);
                let id = self.idx(np.0, np.1);
                if blocked[id] {
                    continue;
                }
                if dist[id] != usize::MAX {
                    continue;
                }
                dist[id] = 1;
                first_dir[id] = Some(dir);
                q.push_back(np);
            }

            let gid = self.idx(goal.0, goal.1);
            while let Some((r, c)) = q.pop_front() {
                let cur_id = self.idx(r, c);
                if cur_id == gid {
                    return first_dir[cur_id];
                }
                for dir in 0..4 {
                    let nr = r as isize + DIRS[dir].0;
                    let nc = c as isize + DIRS[dir].1;
                    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                        continue;
                    }
                    let np = (nr as usize, nc as usize);
                    let id = self.idx(np.0, np.1);
                    if blocked[id] {
                        continue;
                    }
                    if dist[id] != usize::MAX {
                        continue;
                    }
                    dist[id] = dist[cur_id] + 1;
                    first_dir[id] = first_dir[cur_id];
                    q.push_back(np);
                }
            }

            None
        }

        fn bfs_next_dir_strict(&self, goal: (usize, usize), target: (usize, usize)) -> Option<usize> {
            let n = self.state.n;
            let start = self.state.pos[0];
            if start == goal {
                return Some(0);
            }

            let mut blocked = vec![false; n * n];
            for r in 0..n {
                for c in 0..n {
                    if (r, c) != target && self.state.food[self.idx(r, c)] > 0 {
                        blocked[self.idx(r, c)] = true;
                    }
                }
            }
            if self.state.pos.len() >= 3 {
                for &(r, c) in &self.state.pos[1..self.state.pos.len() - 1] {
                    blocked[self.idx(r, c)] = true;
                }
            }

            let sid = self.idx(start.0, start.1);
            let gid = self.idx(goal.0, goal.1);
            blocked[sid] = false;
            if blocked[gid] {
                return None;
            }

            let mut q = VecDeque::new();
            let mut dist = vec![usize::MAX; n * n];
            let mut first_dir = vec![None::<usize>; n * n];
            dist[sid] = 0;

            for dir in legal_dirs(&self.state) {
                let np = next_head(&self.state, dir);
                let id = self.idx(np.0, np.1);
                if blocked[id] || dist[id] != usize::MAX {
                    continue;
                }
                dist[id] = 1;
                first_dir[id] = Some(dir);
                q.push_back(np);
            }

            while let Some((r, c)) = q.pop_front() {
                let cur_id = self.idx(r, c);
                if cur_id == gid {
                    return first_dir[cur_id];
                }
                for dir in 0..4 {
                    let nr = r as isize + DIRS[dir].0;
                    let nc = c as isize + DIRS[dir].1;
                    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                        continue;
                    }
                    let np = (nr as usize, nc as usize);
                    let id = self.idx(np.0, np.1);
                    if blocked[id] || dist[id] != usize::MAX {
                        continue;
                    }
                    dist[id] = dist[cur_id] + 1;
                    first_dir[id] = first_dir[cur_id];
                    q.push_back(np);
                }
            }
            None
        }

        fn plan_color_goal(&self, ell: usize, target_color: u8, cfg: PlanConfig) -> Option<Vec<usize>> {
            let root = SearchNode {
                state: self.state.clone(),
                parent: usize::MAX,
                dir: 0,
                depth: 0,
                non_target: 0,
                bite: 0,
            };
            let mut nodes = vec![root];
            let mut q = VecDeque::new();
            q.push_back(0_usize);

            let mut seen: HashMap<(u64, u8, u8), u8> = HashMap::new();
            let h0 = state_hash(&nodes[0].state);
            seen.insert((h0, 0, 0), 0);

            while let Some(cur_idx) = q.pop_front() {
                if self.time_over() {
                    return None;
                }
                let cur_depth = nodes[cur_idx].depth;
                if cur_depth >= cfg.depth_limit {
                    continue;
                }

                for dir in legal_dirs(&nodes[cur_idx].state) {
                    let mut sim = nodes[cur_idx].state.clone();
                    let out = apply_move(&mut sim, dir);

                    let keep = sim.colors.len().min(ell);
                    if !prefix_match_upto_state(&sim, &self.input.d, keep) {
                        continue;
                    }

                    let mut non_target = nodes[cur_idx].non_target;
                    if let Some(col) = out.ate {
                        if col != target_color {
                            if non_target >= cfg.non_target_limit {
                                continue;
                            }
                            non_target += 1;
                        }
                    }

                    let mut bite = nodes[cur_idx].bite;
                    if out.bite {
                        if bite >= cfg.bite_limit {
                            continue;
                        }
                        bite += 1;
                    }

                    let next_depth = cur_depth + 1;
                    let child = SearchNode {
                        state: sim,
                        parent: cur_idx,
                        dir: dir as u8,
                        depth: next_depth,
                        non_target,
                        bite,
                    };

                    if child.state.colors.len() >= ell + 1
                        && prefix_match_upto_state(&child.state, &self.input.d, ell + 1)
                    {
                        nodes.push(child);
                        let goal_idx = nodes.len() - 1;
                        return Some(reconstruct_path(&nodes, goal_idx));
                    }

                    if nodes.len() >= cfg.node_limit {
                        return None;
                    }

                    let key = (state_hash(&child.state), non_target, bite);
                    if seen
                        .get(&key)
                        .is_some_and(|&best_depth| best_depth <= next_depth)
                    {
                        continue;
                    }
                    seen.insert(key, next_depth);
                    nodes.push(child);
                    q.push_back(nodes.len() - 1);
                }
            }

            None
        }

        fn apply_plan_and_check(&mut self, ell: usize, plan: &[usize]) -> bool {
            for &dir in plan {
                if !self.step(dir) {
                    return false;
                }
                let keep = self.state.colors.len().min(ell);
                if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                    return false;
                }
                if self.time_over() {
                    return false;
                }
            }
            self.state.colors.len() >= ell + 1 && self.prefix_match(ell + 1)
        }

        fn can_reach_target_next(&self, target: (usize, usize)) -> bool {
            can_reach_target_next_state(&self.state, target)
        }

        fn time_over(&self) -> bool {
            self.start.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
        }

        fn advance_with_restore_queue(
            &mut self,
            dir: usize,
            target: (usize, usize),
            ell: usize,
            restore_queue: &mut VecDeque<(usize, usize, u8)>,
        ) -> Option<MoveOutcome> {
            let Some(outcome) = self.step_with_outcome(dir) else {
                return None;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                return None;
            }
            if restore_queue.pop_front().is_some() {
                return Some(outcome);
            }
            self.push_restore_if_needed(ell, &outcome, restore_queue);
            Some(outcome)
        }

        fn push_restore_if_needed(
            &self,
            ell: usize,
            outcome: &MoveOutcome,
            restore_queue: &mut VecDeque<(usize, usize, u8)>,
        ) {
            if !outcome.bite || self.state.colors.len() >= ell {
                return;
            }
            let need = ell - self.state.colors.len();
            for &(r, c, col) in outcome.dropped.iter().take(need) {
                restore_queue.push_back((r, c, col));
            }
        }

        fn step_with_outcome(&mut self, dir: usize) -> Option<MoveOutcome> {
            if self.ops.len() >= MAX_TURNS {
                return None;
            }
            if !self.is_legal_dir(dir) {
                return None;
            }
            let outcome = apply_move(&mut self.state, dir);
            self.ops.push(DIRS[dir].2 as u8);
            Some(outcome)
        }

        fn visit_over_limit(
            &self,
            seen: &mut HashMap<(usize, usize, usize, usize, usize, usize, usize), u16>,
            goal: (usize, usize),
            restore_len: usize,
        ) -> bool {
            let head = self.state.pos[0];
            let neck = if self.state.pos.len() >= 2 {
                self.state.pos[1]
            } else {
                head
            };
            let key = (
                head.0,
                head.1,
                neck.0,
                neck.1,
                self.state.colors.len(),
                self.idx(goal.0, goal.1),
                restore_len,
            );
            let cnt = seen.entry(key).or_insert(0);
            *cnt += 1;
            *cnt > VISIT_REPEAT_LIMIT
        }

        fn step(&mut self, dir: usize) -> bool {
            self.step_with_outcome(dir).is_some()
        }

        fn is_legal_dir(&self, dir: usize) -> bool {
            if self.state.pos.is_empty() {
                return false;
            }
            let head = self.state.pos[0];
            let nr = head.0 as isize + DIRS[dir].0;
            let nc = head.1 as isize + DIRS[dir].1;
            if nr < 0 || nr >= self.state.n as isize || nc < 0 || nc >= self.state.n as isize {
                return false;
            }
            if self.state.pos.len() >= 2 {
                let neck = self.state.pos[1];
                if neck == (nr as usize, nc as usize) {
                    return false;
                }
            }
            true
        }

        fn prefix_match(&self, ell: usize) -> bool {
            prefix_match_state(&self.state, &self.input.d, ell)
        }

        fn collect_food_cells(&self, color: u8) -> Vec<(usize, usize)> {
            let mut res = Vec::new();
            for r in 0..self.state.n {
                for c in 0..self.state.n {
                    if self.state.food[self.idx(r, c)] == color {
                        res.push((r, c));
                    }
                }
            }
            res
        }

        fn neighbors(&self, p: (usize, usize)) -> Vec<(usize, usize)> {
            let mut out = Vec::with_capacity(4);
            for &(dr, dc, _) in &DIRS {
                let nr = p.0 as isize + dr;
                let nc = p.1 as isize + dc;
                if nr < 0 || nr >= self.state.n as isize || nc < 0 || nc >= self.state.n as isize {
                    continue;
                }
                out.push((nr as usize, nc as usize));
            }
            out
        }

        fn idx(&self, r: usize, c: usize) -> usize {
            r * self.state.n + c
        }
    }

    fn reconstruct_path(nodes: &[SearchNode], mut idx: usize) -> Vec<usize> {
        let mut rev = Vec::new();
        while nodes[idx].parent != usize::MAX {
            rev.push(nodes[idx].dir as usize);
            idx = nodes[idx].parent;
        }
        rev.reverse();
        rev
    }

    fn state_hash(state: &State) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        let p = 0x100000001b3_u64;

        h ^= state.pos.len() as u64;
        h = h.wrapping_mul(p);
        h ^= state.colors.len() as u64;
        h = h.wrapping_mul(p);

        for &x in &state.food {
            h ^= x as u64;
            h = h.wrapping_mul(p);
        }
        for &(r, c) in &state.pos {
            h ^= ((r as u64) << 8) ^ (c as u64);
            h = h.wrapping_mul(p);
        }
        for &x in &state.colors {
            h ^= x as u64;
            h = h.wrapping_mul(p);
        }
        h
    }

    fn prefix_match_state(state: &State, d: &[u8], ell: usize) -> bool {
        prefix_match_upto_state(state, d, ell)
    }

    fn prefix_match_upto_state(state: &State, d: &[u8], upto: usize) -> bool {
        if state.colors.len() < upto {
            return false;
        }
        for i in 0..upto {
            if state.colors[i] != d[i] {
                return false;
            }
        }
        true
    }

    fn can_reach_target_next_state(state: &State, target: (usize, usize)) -> bool {
        let head = state.pos[0];
        let Some(dir) = dir_between(head, target) else {
            return false;
        };
        if state.pos.len() >= 2 {
            let neck = state.pos[1];
            let nh = next_head(state, dir);
            if nh == neck {
                return false;
            }
        }
        true
    }

    fn legal_dirs(state: &State) -> Vec<usize> {
        let mut out = Vec::with_capacity(4);
        let head = state.pos[0];
        for dir in 0..4 {
            let nr = head.0 as isize + DIRS[dir].0;
            let nc = head.1 as isize + DIRS[dir].1;
            if nr < 0 || nr >= state.n as isize || nc < 0 || nc >= state.n as isize {
                continue;
            }
            if state.pos.len() >= 2 && state.pos[1] == (nr as usize, nc as usize) {
                continue;
            }
            out.push(dir);
        }
        out
    }

    fn next_head(state: &State, dir: usize) -> (usize, usize) {
        let head = state.pos[0];
        (
            (head.0 as isize + DIRS[dir].0) as usize,
            (head.1 as isize + DIRS[dir].1) as usize,
        )
    }

    fn apply_move(state: &mut State, dir: usize) -> MoveOutcome {
        let mut outcome = MoveOutcome::default();
        let nh = next_head(state, dir);
        let old_len = state.pos.len();
        let old_tail_pos = state.pos[old_len - 1];

        state.pos.insert(0, nh);
        state.pos.pop();

        let eat_idx = nh.0 * state.n + nh.1;
        if state.food[eat_idx] > 0 {
            let food_color = state.food[eat_idx];
            state.food[eat_idx] = 0;
            state.pos.push(old_tail_pos);
            state.colors.push(food_color);
            outcome.ate = Some(food_color);
            return outcome;
        }

        if old_len >= 3 {
            let mut bite_h = None;
            for h in 1..=(old_len - 2) {
                if state.pos[h] == nh {
                    bite_h = Some(h);
                    break;
                }
            }
            if let Some(h) = bite_h {
                outcome.bite = true;
                for p in (h + 1)..old_len {
                    let (r, c) = state.pos[p];
                    let color = state.colors[p];
                    outcome.dropped.push((r, c, color));
                    state.food[r * state.n + c] = color;
                }
                state.pos.truncate(h + 1);
                state.colors.truncate(h + 1);
            }
        }
        outcome
    }

    fn dir_between(a: (usize, usize), b: (usize, usize)) -> Option<usize> {
        for dir in 0..4 {
            let nr = a.0 as isize + DIRS[dir].0;
            let nc = a.1 as isize + DIRS[dir].1;
            if nr == b.0 as isize && nc == b.1 as isize {
                return Some(dir);
            }
        }
        None
    }

    fn manhattan(a: (usize, usize), b: (usize, usize)) -> usize {
        a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
    }

    pub fn solve(input: &Input, start: Instant) -> String {
        let mut solver = Solver::new(input, start);
        solver.solve();
        solver
            .ops
            .iter()
            .map(|&x| (x as char).to_string())
            .collect::<Vec<_>>()
            .join("")
    }
}

fn solve(input: &Input) -> String {
    let started = Instant::now();
    if let Some(ans) = solve_segment(input, &started) {
        return ans;
    }
    fallback_v005::solve(input, started.clone())
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
