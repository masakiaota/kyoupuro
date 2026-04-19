// v002_prefix_first.rs
use std::collections::VecDeque;
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
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

impl Solver {
    fn new(input: Input) -> Self {
        let pos = vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)];
        let colors = vec![1_u8; 5];
        let state = State {
            n: input.n,
            food: input.food.clone(),
            pos,
            colors,
        };
        Self {
            input,
            state,
            ops: Vec::new(),
            start: Instant::now(),
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
        if self.time_over() {
            return false;
        }
        let target_color = self.input.d[ell];
        let mut targets = self.collect_food_cells(target_color);
        if targets.is_empty() {
            return false;
        }

        let head = self.state.pos[0];
        targets.sort_by_key(|&(r, c)| manhattan(head, (r, c)));

        for target in targets {
            if self.time_over() {
                return false;
            }
            if self.try_target(ell, target, target_color) {
                return true;
            }
        }
        false
    }

    fn try_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        let neighbors = self.neighbors(target);
        if neighbors.is_empty() {
            return false;
        }

        let mut cand = neighbors;
        let head = self.state.pos[0];
        cand.sort_by_key(|&p| manhattan(head, p));

        for goal in cand {
            if self.time_over() {
                return false;
            }
            let backup_state = self.state.clone();
            let backup_len = self.ops.len();

            if self.navigate_to_goal(goal, target, ell)
                && self.shrink_to_ell(ell, target, target_color)
                && self.finish_eat_target(ell, target, target_color)
            {
                return true;
            }

            self.state = backup_state;
            self.ops.truncate(backup_len);
        }

        false
    }

    fn finish_eat_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        if self.state.colors.len() != ell {
            return false;
        }
        if self.state.food[self.idx(target.0, target.1)] != target_color {
            return false;
        }
        let head = self.state.pos[0];
        let Some(dir) = dir_between(head, target) else {
            return false;
        };
        if !self.is_legal_dir(dir) {
            return false;
        }
        if !self.step(dir) {
            return false;
        }
        if self.state.colors.len() != ell + 1 {
            return false;
        }
        self.prefix_match(ell + 1)
    }

    fn navigate_to_goal(&mut self, goal: (usize, usize), target: (usize, usize), ell: usize) -> bool {
        let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
        let mut guard = 0_usize;
        while self.state.pos[0] != goal || !restore_queue.is_empty() {
            if self.time_over() {
                return false;
            }
            guard += 1;
            if guard > self.input.n * self.input.n * 80 {
                return false;
            }

            let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                let head = self.state.pos[0];
                let Some(dir) = dir_between(head, (r, c)) else {
                    return false;
                };
                if !self.is_legal_dir(dir) {
                    return false;
                }
                dir
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

            let prev_len = self.state.colors.len();
            let Some(outcome) = self.step_with_outcome(dir) else {
                return false;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                return false;
            }

            if let Some((rr, cc, expected)) = restore_queue.pop_front() {
                if self.state.pos[0] != (rr, cc) {
                    return false;
                }
                if outcome.bite {
                    return false;
                }
                if outcome.ate != Some(expected) || self.state.colors.len() != prev_len + 1 {
                    return false;
                }
                let keep = self.state.colors.len().min(ell);
                if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                    return false;
                }
                continue;
            }

            let keep = self.state.colors.len().min(ell);
            if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                return false;
            }

            if outcome.bite && self.state.colors.len() < ell {
                let need = ell - self.state.colors.len();
                if outcome.dropped.len() < need {
                    return false;
                }
                for &(r, c, col) in outcome.dropped.iter().take(need) {
                    let idx = self.state.colors.len() + restore_queue.len();
                    if idx >= self.input.d.len() || self.input.d[idx] != col {
                        return false;
                    }
                    restore_queue.push_back((r, c, col));
                }
            }
        }

        self.state.colors.len() >= ell && self.prefix_match(ell)
    }

    fn shrink_to_ell(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        if self.state.colors.len() < ell {
            return false;
        }
        if !self.prefix_match(ell) {
            return false;
        }
        if self.state.food[self.idx(target.0, target.1)] != target_color {
            return false;
        }

        if self.state.colors.len() == ell {
            return self.can_reach_target_next(target);
        }

        let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
        let mut guard = 0_usize;
        while self.state.colors.len() != ell
            || !restore_queue.is_empty()
            || !self.can_reach_target_next(target)
        {
            if self.time_over() {
                return false;
            }
            guard += 1;
            if guard > self.input.n * self.input.n * 60 {
                return false;
            }

            let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                let head = self.state.pos[0];
                let Some(dir) = dir_between(head, (r, c)) else {
                    return false;
                };
                if !self.is_legal_dir(dir) {
                    return false;
                }
                dir
            } else {
                let Some(dir) = self.choose_shrink_dir(ell, target) else {
                    return false;
                };
                dir
            };

            let prev_len = self.state.colors.len();
            let Some(outcome) = self.step_with_outcome(dir) else {
                return false;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                return false;
            }

            if let Some((rr, cc, expected)) = restore_queue.pop_front() {
                if self.state.pos[0] != (rr, cc) {
                    return false;
                }
                if outcome.bite {
                    return false;
                }
                if outcome.ate != Some(expected) || self.state.colors.len() != prev_len + 1 {
                    return false;
                }
                let keep = self.state.colors.len().min(ell);
                if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                    return false;
                }
                continue;
            }

            let keep = self.state.colors.len().min(ell);
            if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                return false;
            }

            if outcome.bite && self.state.colors.len() < ell {
                let need = ell - self.state.colors.len();
                if outcome.dropped.len() < need {
                    return false;
                }
                for &(r, c, col) in outcome.dropped.iter().take(need) {
                    let idx = self.state.colors.len() + restore_queue.len();
                    if idx >= self.input.d.len() || self.input.d[idx] != col {
                        return false;
                    }
                    restore_queue.push_back((r, c, col));
                }
            }
        }

        self.state.colors.len() == ell
            && self.prefix_match(ell)
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
                if best_bite.as_ref().is_none_or(|(k, _)| key < *k) {
                    best_bite = Some((key, dir));
                }
            } else {
                let len_gap = sim.colors.len().abs_diff(ell);
                let not_ready = usize::from(!can_reach_target_next_state(&sim, target));
                let target_dist = manhattan(sim.pos[0], target);
                let anchor_dist = manhattan(sim.pos[0], anchor);
                let ate_penalty = usize::from(out.ate.is_some());
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

        for dir in legal_dirs(&self.state) {
            if next_head(&self.state, dir) != target {
                return Some(dir);
            }
        }
        None
    }

    fn bfs_next_dir(&self, goal: (usize, usize), target: (usize, usize), avoid_food: bool) -> Option<usize> {
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

    fn can_reach_target_next(&self, target: (usize, usize)) -> bool {
        can_reach_target_next_state(&self.state, target)
    }

    fn time_over(&self) -> bool {
        self.start.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
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

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut d = vec![0_u8; m];
    for x in d.iter_mut().take(m) {
        *x = it.next().unwrap().parse::<u8>().unwrap();
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input { n, m, d, food }
}

fn main() {
    let input = read_input();
    let mut solver = Solver::new(input);
    solver.solve();

    let out = solver
        .ops
        .iter()
        .map(|&x| (x as char).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    println!("{out}");
}
