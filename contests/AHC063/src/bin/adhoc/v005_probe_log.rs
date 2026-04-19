// v005_probe_log.rs
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
    stats: Stats,
    stop_reason: StopReason,
    final_ell: usize,
}

#[derive(Clone, Debug, Default)]
struct Stats {
    solve_iters: usize,
    solve_prefix_break: usize,
    solve_extend_fail: usize,
    solve_time_break: usize,
    solve_turn_break: usize,
    extend_calls: usize,
    extend_success: usize,
    extend_no_target: usize,
    plan_safe_calls: usize,
    plan_safe_found: usize,
    plan_safe_applied: usize,
    plan_safe_apply_fail: usize,
    plan_rescue_calls: usize,
    plan_rescue_found: usize,
    plan_rescue_applied: usize,
    plan_rescue_apply_fail: usize,
    plan_color_expands: usize,
    plan_color_seen_skip: usize,
    plan_color_limit_abort: usize,
    fallback_target_trials: usize,
    try_calls: usize,
    try_goal_trials: usize,
    try_success: usize,
    try_fail_all: usize,
    try_safe_success: usize,
    try_fallback_success: usize,
    try_fail_safe: usize,
    try_fail_fallback_nav: usize,
    try_fail_fallback_shrink: usize,
    try_fail_fallback_finish: usize,
    safe_nav_calls: usize,
    safe_nav_success: usize,
    safe_nav_fail_bfs_none: usize,
    safe_nav_fail_repeat: usize,
    safe_nav_fail_guard: usize,
    safe_nav_fail_step: usize,
    safe_nav_fail_target_eaten: usize,
    safe_nav_fail_bite: usize,
    safe_nav_fail_ate: usize,
    nav_calls: usize,
    nav_success: usize,
    nav_fail_guard: usize,
    nav_fail_repeat: usize,
    nav_fail_bfs_none: usize,
    nav_fail_step: usize,
    nav_fail_target_eaten: usize,
    nav_fail_bite_limit: usize,
    nav_bites: usize,
    nav_ate: usize,
    shrink_calls: usize,
    shrink_success: usize,
    shrink_fail_guard: usize,
    shrink_fail_repeat: usize,
    shrink_fail_choose_none: usize,
    shrink_fail_step: usize,
    shrink_fail_target_eaten: usize,
    shrink_fail_bite_limit: usize,
    shrink_fail_final: usize,
    shrink_bites: usize,
    shrink_ate: usize,
    finish_calls: usize,
    finish_success: usize,
    finish_fail_not_adjacent: usize,
    finish_fail_step: usize,
    finish_fail_prefix: usize,
}

#[derive(Clone, Copy, Debug, Default)]
enum StopReason {
    #[default]
    None,
    PrefixMismatch,
    ExtendFailed,
    TimeLimit,
    MaxTurns,
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
            stats: Stats::default(),
            stop_reason: StopReason::None,
            final_ell: 5,
        }
    }

    fn solve(&mut self) {
        let mut ell = 5_usize;
        while ell < self.input.m {
            self.final_ell = ell;
            if self.ops.len() >= MAX_TURNS {
                self.stats.solve_turn_break += 1;
                self.stop_reason = StopReason::MaxTurns;
                break;
            }
            if self.time_over() {
                self.stats.solve_time_break += 1;
                self.stop_reason = StopReason::TimeLimit;
                break;
            }
            self.stats.solve_iters += 1;
            if !self.prefix_match(ell) {
                self.stats.solve_prefix_break += 1;
                self.stop_reason = StopReason::PrefixMismatch;
                break;
            }
            let before = self.stats.clone();
            if !self.extend_one(ell) {
                self.stats.solve_extend_fail += 1;
                self.stop_reason = StopReason::ExtendFailed;
                let target_color = self.input.d[ell];
                let mut targets = self.collect_food_cells(target_color);
                let head = self.state.pos[0];
                targets.sort_by_key(|&(r, c)| manhattan(head, (r, c)));
                let preview = targets.iter().take(8).copied().collect::<Vec<_>>();
                eprintln!(
                    "extend_fail_detail ell={} target_color={} turn={} len={} head=({}, {}) targets={} \
plan_safe_calls={} plan_safe_found={} plan_safe_applied={} plan_safe_apply_fail={} \
plan_rescue_calls={} plan_rescue_found={} plan_rescue_applied={} plan_rescue_apply_fail={} \
plan_color_expands={} plan_color_seen_skip={} plan_color_limit_abort={} \
fallback_target_trials={} try_calls={} try_goal_trials={} try_success={} \
try_fail_safe={} try_fail_fallback_nav={} try_fail_fallback_shrink={} try_fail_fallback_finish={} \
safe_nav_calls={} safe_nav_success={} safe_nav_fail_bfs_none={} safe_nav_fail_repeat={} safe_nav_fail_target_eaten={} \
nav_calls={} nav_success={} nav_fail_guard={} nav_fail_repeat={} nav_fail_target_eaten={} nav_fail_bite_limit={} nav_bites={} nav_ate={} \
shrink_calls={} shrink_success={} shrink_fail_guard={} shrink_fail_repeat={} shrink_fail_choose_none={} shrink_fail_bite_limit={} shrink_fail_final={} shrink_bites={} shrink_ate={} \
target_preview={:?}",
                    ell,
                    target_color,
                    self.ops.len(),
                    self.state.colors.len(),
                    head.0,
                    head.1,
                    targets.len(),
                    self.stats.plan_safe_calls - before.plan_safe_calls,
                    self.stats.plan_safe_found - before.plan_safe_found,
                    self.stats.plan_safe_applied - before.plan_safe_applied,
                    self.stats.plan_safe_apply_fail - before.plan_safe_apply_fail,
                    self.stats.plan_rescue_calls - before.plan_rescue_calls,
                    self.stats.plan_rescue_found - before.plan_rescue_found,
                    self.stats.plan_rescue_applied - before.plan_rescue_applied,
                    self.stats.plan_rescue_apply_fail - before.plan_rescue_apply_fail,
                    self.stats.plan_color_expands - before.plan_color_expands,
                    self.stats.plan_color_seen_skip - before.plan_color_seen_skip,
                    self.stats.plan_color_limit_abort - before.plan_color_limit_abort,
                    self.stats.fallback_target_trials - before.fallback_target_trials,
                    self.stats.try_calls - before.try_calls,
                    self.stats.try_goal_trials - before.try_goal_trials,
                    self.stats.try_success - before.try_success,
                    self.stats.try_fail_safe - before.try_fail_safe,
                    self.stats.try_fail_fallback_nav - before.try_fail_fallback_nav,
                    self.stats.try_fail_fallback_shrink - before.try_fail_fallback_shrink,
                    self.stats.try_fail_fallback_finish - before.try_fail_fallback_finish,
                    self.stats.safe_nav_calls - before.safe_nav_calls,
                    self.stats.safe_nav_success - before.safe_nav_success,
                    self.stats.safe_nav_fail_bfs_none - before.safe_nav_fail_bfs_none,
                    self.stats.safe_nav_fail_repeat - before.safe_nav_fail_repeat,
                    self.stats.safe_nav_fail_target_eaten - before.safe_nav_fail_target_eaten,
                    self.stats.nav_calls - before.nav_calls,
                    self.stats.nav_success - before.nav_success,
                    self.stats.nav_fail_guard - before.nav_fail_guard,
                    self.stats.nav_fail_repeat - before.nav_fail_repeat,
                    self.stats.nav_fail_target_eaten - before.nav_fail_target_eaten,
                    self.stats.nav_fail_bite_limit - before.nav_fail_bite_limit,
                    self.stats.nav_bites - before.nav_bites,
                    self.stats.nav_ate - before.nav_ate,
                    self.stats.shrink_calls - before.shrink_calls,
                    self.stats.shrink_success - before.shrink_success,
                    self.stats.shrink_fail_guard - before.shrink_fail_guard,
                    self.stats.shrink_fail_repeat - before.shrink_fail_repeat,
                    self.stats.shrink_fail_choose_none - before.shrink_fail_choose_none,
                    self.stats.shrink_fail_bite_limit - before.shrink_fail_bite_limit,
                    self.stats.shrink_fail_final - before.shrink_fail_final,
                    self.stats.shrink_bites - before.shrink_bites,
                    self.stats.shrink_ate - before.shrink_ate,
                    preview
                );
                break;
            }
            self.stats.extend_success += 1;
            ell += 1;
        }
        self.final_ell = ell;
    }

    fn log_summary(&self) {
        let remaining_food = self.state.food.iter().filter(|&&x| x > 0).count();
        eprintln!(
            "probe_summary stop={:?} final_ell={} len={} ops={} remaining_food={} solve_iters={} \
extend_calls={} extend_success={} extend_no_target={} \
plan_safe_calls={} plan_safe_found={} plan_safe_applied={} plan_safe_apply_fail={} \
plan_rescue_calls={} plan_rescue_found={} plan_rescue_applied={} plan_rescue_apply_fail={} \
plan_color_expands={} plan_color_seen_skip={} plan_color_limit_abort={} \
fallback_target_trials={} try_calls={} try_goal_trials={} try_success={} try_fail_all={} \
try_safe_success={} try_fallback_success={} try_fail_safe={} try_fail_fallback_nav={} try_fail_fallback_shrink={} try_fail_fallback_finish={} \
safe_nav_calls={} safe_nav_success={} safe_nav_fail_bfs_none={} safe_nav_fail_repeat={} safe_nav_fail_guard={} safe_nav_fail_step={} safe_nav_fail_target_eaten={} safe_nav_fail_bite={} safe_nav_fail_ate={} \
nav_calls={} nav_success={} nav_fail_guard={} nav_fail_repeat={} nav_fail_bfs_none={} nav_fail_step={} nav_fail_target_eaten={} nav_fail_bite_limit={} nav_bites={} nav_ate={} \
shrink_calls={} shrink_success={} shrink_fail_guard={} shrink_fail_repeat={} shrink_fail_choose_none={} shrink_fail_step={} shrink_fail_target_eaten={} shrink_fail_bite_limit={} shrink_fail_final={} shrink_bites={} shrink_ate={} \
finish_calls={} finish_success={} finish_fail_not_adjacent={} finish_fail_step={} finish_fail_prefix={}",
            self.stop_reason,
            self.final_ell,
            self.state.colors.len(),
            self.ops.len(),
            remaining_food,
            self.stats.solve_iters,
            self.stats.extend_calls,
            self.stats.extend_success,
            self.stats.extend_no_target,
            self.stats.plan_safe_calls,
            self.stats.plan_safe_found,
            self.stats.plan_safe_applied,
            self.stats.plan_safe_apply_fail,
            self.stats.plan_rescue_calls,
            self.stats.plan_rescue_found,
            self.stats.plan_rescue_applied,
            self.stats.plan_rescue_apply_fail,
            self.stats.plan_color_expands,
            self.stats.plan_color_seen_skip,
            self.stats.plan_color_limit_abort,
            self.stats.fallback_target_trials,
            self.stats.try_calls,
            self.stats.try_goal_trials,
            self.stats.try_success,
            self.stats.try_fail_all,
            self.stats.try_safe_success,
            self.stats.try_fallback_success,
            self.stats.try_fail_safe,
            self.stats.try_fail_fallback_nav,
            self.stats.try_fail_fallback_shrink,
            self.stats.try_fail_fallback_finish,
            self.stats.safe_nav_calls,
            self.stats.safe_nav_success,
            self.stats.safe_nav_fail_bfs_none,
            self.stats.safe_nav_fail_repeat,
            self.stats.safe_nav_fail_guard,
            self.stats.safe_nav_fail_step,
            self.stats.safe_nav_fail_target_eaten,
            self.stats.safe_nav_fail_bite,
            self.stats.safe_nav_fail_ate,
            self.stats.nav_calls,
            self.stats.nav_success,
            self.stats.nav_fail_guard,
            self.stats.nav_fail_repeat,
            self.stats.nav_fail_bfs_none,
            self.stats.nav_fail_step,
            self.stats.nav_fail_target_eaten,
            self.stats.nav_fail_bite_limit,
            self.stats.nav_bites,
            self.stats.nav_ate,
            self.stats.shrink_calls,
            self.stats.shrink_success,
            self.stats.shrink_fail_guard,
            self.stats.shrink_fail_repeat,
            self.stats.shrink_fail_choose_none,
            self.stats.shrink_fail_step,
            self.stats.shrink_fail_target_eaten,
            self.stats.shrink_fail_bite_limit,
            self.stats.shrink_fail_final,
            self.stats.shrink_bites,
            self.stats.shrink_ate,
            self.stats.finish_calls,
            self.stats.finish_success,
            self.stats.finish_fail_not_adjacent,
            self.stats.finish_fail_step,
            self.stats.finish_fail_prefix
        );
    }

    fn extend_one(&mut self, ell: usize) -> bool {
        self.stats.extend_calls += 1;
        let target_color = self.input.d[ell];
        if self.collect_food_cells(target_color).is_empty() {
            self.stats.extend_no_target += 1;
            return false;
        }

        let safe_cfg = PlanConfig {
            depth_limit: 10,
            node_limit: 3_000,
            non_target_limit: 0,
            bite_limit: 0,
        };
        self.stats.plan_safe_calls += 1;
        if let Some(plan) = self.plan_color_goal(ell, target_color, safe_cfg) {
            self.stats.plan_safe_found += 1;
            if self.apply_plan_and_check(ell, &plan) {
                self.stats.plan_safe_applied += 1;
                return true;
            }
            self.stats.plan_safe_apply_fail += 1;
        }

        let rescue_cfg = PlanConfig {
            depth_limit: 16,
            node_limit: 10_000,
            non_target_limit: 6,
            bite_limit: 2,
        };
        self.stats.plan_rescue_calls += 1;
        if let Some(plan) = self.plan_color_goal(ell, target_color, rescue_cfg) {
            self.stats.plan_rescue_found += 1;
            if self.apply_plan_and_check(ell, &plan) {
                self.stats.plan_rescue_applied += 1;
                return true;
            }
            self.stats.plan_rescue_apply_fail += 1;
        }

        let mut targets = self.collect_food_cells(target_color);

        let head = self.state.pos[0];
        targets.sort_by_key(|&(r, c)| manhattan(head, (r, c)));

        for target in targets {
            self.stats.fallback_target_trials += 1;
            if self.try_target(ell, target, target_color) {
                return true;
            }
        }
        false
    }

    fn try_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        self.stats.try_calls += 1;
        let mut cand = self.neighbors(target);
        let head = self.state.pos[0];
        cand.sort_by_key(|&p| {
            (
                usize::from(self.state.food[self.idx(p.0, p.1)] > 0),
                manhattan(head, p),
            )
        });

        for goal in cand {
            self.stats.try_goal_trials += 1;
            let backup_state = self.state.clone();
            let backup_len = self.ops.len();

            let safe_ok = self.navigate_to_goal_safe(goal, target);
            if safe_ok {
                if self.shrink_to_ell(ell, target, target_color)
                    && self.finish_eat_target(ell, target, target_color)
                {
                    self.stats.try_success += 1;
                    self.stats.try_safe_success += 1;
                    return true;
                }
            }
            self.stats.try_fail_safe += 1;

            self.state = backup_state.clone();
            self.ops.truncate(backup_len);

            let nav_ok = self.navigate_to_goal(goal, target, ell);
            if !nav_ok {
                self.stats.try_fail_fallback_nav += 1;
            }
            let shrink_ok = nav_ok && self.shrink_to_ell(ell, target, target_color);
            if nav_ok && !shrink_ok {
                self.stats.try_fail_fallback_shrink += 1;
            }
            let finish_ok = shrink_ok && self.finish_eat_target(ell, target, target_color);
            if shrink_ok && !finish_ok {
                self.stats.try_fail_fallback_finish += 1;
            }
            if finish_ok {
                self.stats.try_success += 1;
                self.stats.try_fallback_success += 1;
                return true;
            }

            self.state = backup_state;
            self.ops.truncate(backup_len);
        }

        self.stats.try_fail_all += 1;
        false
    }

    fn finish_eat_target(&mut self, ell: usize, target: (usize, usize), _target_color: u8) -> bool {
        self.stats.finish_calls += 1;
        let head = self.state.pos[0];
        let Some(dir) = dir_between(head, target) else {
            self.stats.finish_fail_not_adjacent += 1;
            return false;
        };
        if !self.step(dir) {
            self.stats.finish_fail_step += 1;
            return false;
        }
        if self.prefix_match(ell + 1) {
            self.stats.finish_success += 1;
            true
        } else {
            self.stats.finish_fail_prefix += 1;
            false
        }
    }

    fn navigate_to_goal(
        &mut self,
        goal: (usize, usize),
        target: (usize, usize),
        ell: usize,
    ) -> bool {
        self.stats.nav_calls += 1;
        let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
        let mut seen = HashMap::new();
        let mut bite_count = 0_usize;
        let bite_limit = self.input.n * self.input.n * 4;
        let mut guard = 0_usize;
        while self.state.pos[0] != goal || !restore_queue.is_empty() {
            guard += 1;
            if guard > self.input.n * self.input.n * 80 {
                self.stats.nav_fail_guard += 1;
                return false;
            }
            if self.visit_over_limit(&mut seen, goal, restore_queue.len()) {
                self.stats.nav_fail_repeat += 1;
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
                    self.stats.nav_fail_bfs_none += 1;
                    return false;
                };
                dir
            };

            let Some(outcome) =
                self.advance_with_restore_queue(dir, target, ell, &mut restore_queue)
            else {
                self.stats.nav_fail_step += 1;
                return false;
            };
            if outcome.ate.is_some() {
                self.stats.nav_ate += 1;
            }
            if outcome.bite {
                self.stats.nav_bites += 1;
                bite_count += 1;
                if bite_count > bite_limit {
                    self.stats.nav_fail_bite_limit += 1;
                    return false;
                }
            }
        }

        let ok = self.state.colors.len() >= ell;
        if ok {
            self.stats.nav_success += 1;
        }
        ok
    }

    fn navigate_to_goal_safe(&mut self, goal: (usize, usize), target: (usize, usize)) -> bool {
        self.stats.safe_nav_calls += 1;
        let mut seen = HashMap::new();
        let mut guard = 0_usize;
        while self.state.pos[0] != goal {
            guard += 1;
            if guard > self.input.n * self.input.n * 30 {
                self.stats.safe_nav_fail_guard += 1;
                return false;
            }
            if self.visit_over_limit(&mut seen, goal, 0) {
                self.stats.safe_nav_fail_repeat += 1;
                return false;
            }
            let Some(dir) = self.bfs_next_dir_strict(goal, target) else {
                self.stats.safe_nav_fail_bfs_none += 1;
                return false;
            };
            let Some(outcome) = self.step_with_outcome(dir) else {
                self.stats.safe_nav_fail_step += 1;
                return false;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                self.stats.safe_nav_fail_target_eaten += 1;
                return false;
            }
            if outcome.bite {
                self.stats.safe_nav_fail_bite += 1;
                return false;
            }
            if outcome.ate.is_some() {
                self.stats.safe_nav_fail_ate += 1;
                return false;
            }
        }
        self.stats.safe_nav_success += 1;
        true
    }

    fn shrink_to_ell(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        self.stats.shrink_calls += 1;
        if self.state.colors.len() == ell {
            let ok = self.can_reach_target_next(target);
            if ok {
                self.stats.shrink_success += 1;
            } else {
                self.stats.shrink_fail_final += 1;
            }
            return ok;
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
                self.stats.shrink_fail_guard += 1;
                return false;
            }
            if self.visit_over_limit(&mut seen, target, restore_queue.len()) {
                self.stats.shrink_fail_repeat += 1;
                return false;
            }

            let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                let head = self.state.pos[0];
                dir_between(head, (r, c)).unwrap()
            } else {
                let Some(dir) = self.choose_shrink_dir(ell, target) else {
                    self.stats.shrink_fail_choose_none += 1;
                    return false;
                };
                dir
            };

            let Some(outcome) =
                self.advance_with_restore_queue(dir, target, ell, &mut restore_queue)
            else {
                self.stats.shrink_fail_step += 1;
                return false;
            };
            if outcome.ate.is_some() {
                self.stats.shrink_ate += 1;
            }
            if outcome.bite {
                self.stats.shrink_bites += 1;
                bite_count += 1;
                if bite_count > bite_limit {
                    self.stats.shrink_fail_bite_limit += 1;
                    return false;
                }
            }
        }

        let ok = self.state.colors.len() == ell
            && self.state.food[self.idx(target.0, target.1)] == target_color
            && self.can_reach_target_next(target);
        if ok {
            self.stats.shrink_success += 1;
        } else {
            self.stats.shrink_fail_final += 1;
        }
        ok
    }

    fn choose_shrink_dir(&mut self, ell: usize, target: (usize, usize)) -> Option<usize> {
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

    fn plan_color_goal(
        &mut self,
        ell: usize,
        target_color: u8,
        cfg: PlanConfig,
    ) -> Option<Vec<usize>> {
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
                self.stats.plan_color_expands += 1;
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
                    self.stats.plan_color_limit_abort += 1;
                    return None;
                }

                let key = (state_hash(&child.state), non_target, bite);
                if seen
                    .get(&key)
                    .is_some_and(|&best_depth| best_depth <= next_depth)
                {
                    self.stats.plan_color_seen_skip += 1;
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
    solver.log_summary();

    let out = solver
        .ops
        .iter()
        .map(|&x| (x as char).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    println!("{out}");
}
