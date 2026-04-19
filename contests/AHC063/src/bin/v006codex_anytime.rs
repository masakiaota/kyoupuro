// v006codex_anytime.rs
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const VISIT_REPEAT_LIMIT: u16 = 12;

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
        let rescue_cfg = PlanConfig {
            depth_limit: 20,
            node_limit: 40_000,
            non_target_limit: 8,
            bite_limit: 3,
        };
        let mut best_metric = self.progress_metric(ell, target_color);
        let mut neutral_rounds = 0_usize;
        for _ in 0..4 {
            if self.time_over() || self.ops.len() >= MAX_TURNS {
                break;
            }

            let mut applied = false;
            for cfg in [safe_cfg, rescue_cfg] {
                let Some(plan) = self.plan_color_goal(ell, target_color, cfg) else {
                    continue;
                };
                if plan.is_empty() {
                    continue;
                }

                let backup_state = self.state.clone();
                let backup_len = self.ops.len();
                if self.apply_plan_and_check(ell, &plan) {
                    return true;
                }
                if self.ops.len() == backup_len {
                    self.state = backup_state;
                    continue;
                }

                let next_metric = self.progress_metric(ell, target_color);
                if next_metric <= best_metric {
                    if next_metric == best_metric {
                        neutral_rounds += 1;
                    } else {
                        best_metric = next_metric;
                        neutral_rounds = 0;
                    }
                    applied = true;
                    break;
                }

                self.state = backup_state;
                self.ops.truncate(backup_len);
            }
            if self.prefix_match(ell + 1) {
                return true;
            }
            if !applied || neutral_rounds >= 2 {
                break;
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
                && self.finish_eat_target(ell, target, target_color)
            {
                return true;
            }

            self.state = backup_state.clone();
            self.ops.truncate(backup_len);

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

    fn finish_eat_target(&mut self, ell: usize, target: (usize, usize), _target_color: u8) -> bool {
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
        let mut best_idx = 0_usize;
        let mut best_score = plan_score_state(&nodes[0].state, ell, target_color, 0, 0, 0);
        let mut limit_hit = false;

        while let Some(cur_idx) = q.pop_front() {
            if self.time_over() {
                break;
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
                    limit_hit = true;
                    break;
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
                let child_idx = nodes.len() - 1;
                let score =
                    plan_score_state(&nodes[child_idx].state, ell, target_color, non_target, bite, next_depth);
                if score < best_score {
                    best_score = score;
                    best_idx = child_idx;
                }
                q.push_back(child_idx);
            }
            if limit_hit {
                break;
            }
        }

        if best_idx == 0 {
            None
        } else {
            Some(reconstruct_path(&nodes, best_idx))
        }
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

    fn progress_metric(&self, ell: usize, target_color: u8) -> (usize, usize, usize) {
        progress_metric_state(&self.state, ell, target_color)
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

fn min_dist_to_color_state(state: &State, color: u8) -> Option<usize> {
    let head = state.pos[0];
    let mut best = None::<usize>;
    for r in 0..state.n {
        for c in 0..state.n {
            if state.food[r * state.n + c] != color {
                continue;
            }
            let d = manhattan(head, (r, c));
            best = Some(best.map_or(d, |cur| cur.min(d)));
        }
    }
    best
}

fn progress_metric_state(state: &State, ell: usize, target_color: u8) -> (usize, usize, usize) {
    let wrong_extra = if state.colors.len() > ell {
        usize::from(state.colors[ell] != target_color)
    } else {
        0
    };
    let len_gap = (ell + 1).saturating_sub(state.colors.len());
    let no_target_dist = state.n * state.n + 1;
    let target_dist = min_dist_to_color_state(state, target_color).unwrap_or(no_target_dist);
    (wrong_extra, len_gap, target_dist)
}

fn plan_score_state(
    state: &State,
    ell: usize,
    target_color: u8,
    non_target: u8,
    bite: u8,
    depth: u8,
) -> (usize, usize, usize, usize, usize, usize) {
    let (wrong_extra, len_gap, target_dist) = progress_metric_state(state, ell, target_color);
    (
        wrong_extra,
        len_gap,
        target_dist,
        non_target as usize,
        bite as usize,
        depth as usize,
    )
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
