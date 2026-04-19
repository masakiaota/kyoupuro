// v002_probe_log.rs
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
    extend_time_over: usize,
    target_candidates_total: usize,
    target_trials: usize,
    try_target_calls: usize,
    try_target_success: usize,
    try_target_no_neighbors: usize,
    try_target_goal_trials: usize,
    try_target_fail_navigate: usize,
    try_target_fail_shrink: usize,
    try_target_fail_finish: usize,
    try_target_rollbacks: usize,
    navigate_calls: usize,
    navigate_success: usize,
    navigate_restore_steps: usize,
    navigate_restore_pushes: usize,
    navigate_fail_time: usize,
    navigate_fail_guard: usize,
    navigate_fail_restore_dir: usize,
    navigate_fail_restore_illegal: usize,
    navigate_fail_bfs_none: usize,
    navigate_fail_step: usize,
    navigate_fail_target_eaten: usize,
    navigate_fail_restore_pos_mismatch: usize,
    navigate_fail_restore_bite: usize,
    navigate_fail_restore_color_or_len: usize,
    navigate_fail_prefix: usize,
    navigate_bite_events: usize,
    navigate_fail_drop_short: usize,
    navigate_fail_drop_color_mismatch: usize,
    bfs_avoid_calls: usize,
    bfs_avoid_found: usize,
    bfs_relaxed_calls: usize,
    bfs_relaxed_found: usize,
    shrink_calls: usize,
    shrink_success: usize,
    shrink_fail_short_len: usize,
    shrink_fail_prefix_start: usize,
    shrink_fail_target_missing_start: usize,
    shrink_early_eq_len_ok: usize,
    shrink_early_eq_len_ng: usize,
    shrink_fail_time: usize,
    shrink_fail_guard: usize,
    shrink_fail_restore_dir: usize,
    shrink_fail_restore_illegal: usize,
    shrink_fail_choose_none: usize,
    shrink_fail_step: usize,
    shrink_fail_target_eaten: usize,
    shrink_fail_restore_pos_mismatch: usize,
    shrink_fail_restore_bite: usize,
    shrink_fail_restore_color_or_len: usize,
    shrink_fail_prefix: usize,
    shrink_bite_events: usize,
    shrink_restore_pushes: usize,
    shrink_restore_steps: usize,
    shrink_fail_drop_short: usize,
    shrink_fail_drop_color_mismatch: usize,
    shrink_final_fail: usize,
    choose_calls: usize,
    choose_pick_bite: usize,
    choose_pick_move: usize,
    choose_pick_fallback_non_target: usize,
    choose_none: usize,
    finish_calls: usize,
    finish_success: usize,
    finish_fail_len: usize,
    finish_fail_target_missing: usize,
    finish_fail_not_adjacent: usize,
    finish_fail_illegal: usize,
    finish_fail_step: usize,
    finish_fail_len_after: usize,
    finish_fail_prefix_after: usize,
    step_calls: usize,
    step_fail: usize,
    moves: usize,
    eats: usize,
    bites: usize,
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
            let before_target_candidates = self.stats.target_candidates_total;
            let before_target_trials = self.stats.target_trials;
            let before_try_calls = self.stats.try_target_calls;
            let before_try_goal_trials = self.stats.try_target_goal_trials;
            let before_try_success = self.stats.try_target_success;
            let before_try_fail_nav = self.stats.try_target_fail_navigate;
            let before_try_fail_shrink = self.stats.try_target_fail_shrink;
            let before_try_fail_finish = self.stats.try_target_fail_finish;
            let before_nav_calls = self.stats.navigate_calls;
            let before_nav_success = self.stats.navigate_success;
            let before_nav_fail_guard = self.stats.navigate_fail_guard;
            let before_nav_fail_target_eaten = self.stats.navigate_fail_target_eaten;
            let before_nav_fail_bfs_none = self.stats.navigate_fail_bfs_none;
            let before_nav_bites = self.stats.navigate_bite_events;
            let before_nav_restore_steps = self.stats.navigate_restore_steps;
            let before_nav_restore_pushes = self.stats.navigate_restore_pushes;
            let before_shrink_calls = self.stats.shrink_calls;
            let before_shrink_success = self.stats.shrink_success;
            let before_shrink_fail_guard = self.stats.shrink_fail_guard;
            let before_shrink_fail_choose_none = self.stats.shrink_fail_choose_none;
            let before_shrink_bites = self.stats.shrink_bite_events;
            let before_shrink_restore_steps = self.stats.shrink_restore_steps;
            let before_shrink_restore_pushes = self.stats.shrink_restore_pushes;
            if !self.extend_one(ell) {
                self.stats.solve_extend_fail += 1;
                self.stop_reason = StopReason::ExtendFailed;
                eprintln!(
                    "extend_fail_detail ell={} target_color={} turn={} len={} target_candidates={} target_trials={} \
try_calls={} try_goal_trials={} try_success={} try_fail_nav={} try_fail_shrink={} try_fail_finish={} \
navigate_calls={} navigate_success={} navigate_bites={} navigate_restore_steps={} navigate_restore_pushes={} navigate_fail_guard={} navigate_fail_target_eaten={} navigate_fail_bfs_none={} \
shrink_calls={} shrink_success={} shrink_bites={} shrink_restore_steps={} shrink_restore_pushes={} shrink_fail_guard={} shrink_fail_choose_none={}",
                    ell,
                    self.input.d[ell],
                    self.ops.len(),
                    self.state.colors.len(),
                    self.stats.target_candidates_total - before_target_candidates,
                    self.stats.target_trials - before_target_trials,
                    self.stats.try_target_calls - before_try_calls,
                    self.stats.try_target_goal_trials - before_try_goal_trials,
                    self.stats.try_target_success - before_try_success,
                    self.stats.try_target_fail_navigate - before_try_fail_nav,
                    self.stats.try_target_fail_shrink - before_try_fail_shrink,
                    self.stats.try_target_fail_finish - before_try_fail_finish,
                    self.stats.navigate_calls - before_nav_calls,
                    self.stats.navigate_success - before_nav_success,
                    self.stats.navigate_bite_events - before_nav_bites,
                    self.stats.navigate_restore_steps - before_nav_restore_steps,
                    self.stats.navigate_restore_pushes - before_nav_restore_pushes,
                    self.stats.navigate_fail_guard - before_nav_fail_guard,
                    self.stats.navigate_fail_target_eaten - before_nav_fail_target_eaten,
                    self.stats.navigate_fail_bfs_none - before_nav_fail_bfs_none,
                    self.stats.shrink_calls - before_shrink_calls,
                    self.stats.shrink_success - before_shrink_success,
                    self.stats.shrink_bite_events - before_shrink_bites,
                    self.stats.shrink_restore_steps - before_shrink_restore_steps,
                    self.stats.shrink_restore_pushes - before_shrink_restore_pushes,
                    self.stats.shrink_fail_guard - before_shrink_fail_guard,
                    self.stats.shrink_fail_choose_none - before_shrink_fail_choose_none
                );
                break;
            }
            ell += 1;
        }
        self.final_ell = ell;
    }

    fn log_summary(&self) {
        let remaining_food = self.state.food.iter().filter(|&&v| v > 0).count();
        let line = format!(
            "probe_summary stop={:?} final_ell={} len={} ops={} remaining_food={} \
solve_iters={} extend_calls={} extend_success={} target_candidates={} target_trials={} \
extend_no_target={} extend_time_over={} \
try_calls={} try_success={} try_no_neighbors={} try_goal_trials={} try_rollbacks={} try_fail_nav={} try_fail_shrink={} try_fail_finish={} \
navigate_calls={} navigate_success={} navigate_restore_steps={} navigate_restore_pushes={} \
navigate_bites={} navigate_fail_time={} navigate_fail_guard={} navigate_fail_bfs_none={} navigate_fail_step={} navigate_fail_target_eaten={} \
navigate_fail_restore_dir={} navigate_fail_restore_illegal={} navigate_fail_restore_pos={} navigate_fail_restore_bite={} navigate_fail_restore_color_or_len={} \
navigate_fail_drop_short={} navigate_fail_drop_color={} navigate_fail_prefix={} \
shrink_calls={} shrink_success={} shrink_bites={} shrink_restore_steps={} shrink_restore_pushes={} \
shrink_fail_short_len={} shrink_fail_prefix_start={} shrink_fail_target_missing_start={} \
shrink_early_eq_ok={} shrink_early_eq_ng={} shrink_fail_time={} shrink_fail_guard={} \
shrink_fail_restore_dir={} shrink_fail_restore_illegal={} shrink_fail_choose_none={} shrink_fail_step={} shrink_fail_target_eaten={} \
shrink_fail_restore_pos={} shrink_fail_restore_bite={} shrink_fail_restore_color_or_len={} \
shrink_fail_drop_short={} shrink_fail_drop_color={} shrink_fail_prefix={} shrink_final_fail={} \
choose_calls={} choose_pick_bite={} choose_pick_move={} choose_pick_fallback={} choose_none={} \
bfs_avoid_calls={} bfs_avoid_found={} bfs_relaxed_calls={} bfs_relaxed_found={} \
finish_calls={} finish_success={} finish_fail_len={} finish_fail_target_missing={} \
step_calls={} step_fail={} moves={} eats={} bites={}",
            self.stop_reason,
            self.final_ell,
            self.state.colors.len(),
            self.ops.len(),
            remaining_food,
            self.stats.solve_iters,
            self.stats.extend_calls,
            self.stats.extend_success,
            self.stats.target_candidates_total,
            self.stats.target_trials,
            self.stats.extend_no_target,
            self.stats.extend_time_over,
            self.stats.try_target_calls,
            self.stats.try_target_success,
            self.stats.try_target_no_neighbors,
            self.stats.try_target_goal_trials,
            self.stats.try_target_rollbacks,
            self.stats.try_target_fail_navigate,
            self.stats.try_target_fail_shrink,
            self.stats.try_target_fail_finish,
            self.stats.navigate_calls,
            self.stats.navigate_success,
            self.stats.navigate_restore_steps,
            self.stats.navigate_restore_pushes,
            self.stats.navigate_bite_events,
            self.stats.navigate_fail_time,
            self.stats.navigate_fail_guard,
            self.stats.navigate_fail_bfs_none,
            self.stats.navigate_fail_step,
            self.stats.navigate_fail_target_eaten,
            self.stats.navigate_fail_restore_dir,
            self.stats.navigate_fail_restore_illegal,
            self.stats.navigate_fail_restore_pos_mismatch,
            self.stats.navigate_fail_restore_bite,
            self.stats.navigate_fail_restore_color_or_len,
            self.stats.navigate_fail_drop_short,
            self.stats.navigate_fail_drop_color_mismatch,
            self.stats.navigate_fail_prefix,
            self.stats.shrink_calls,
            self.stats.shrink_success,
            self.stats.shrink_bite_events,
            self.stats.shrink_restore_steps,
            self.stats.shrink_restore_pushes,
            self.stats.shrink_fail_short_len,
            self.stats.shrink_fail_prefix_start,
            self.stats.shrink_fail_target_missing_start,
            self.stats.shrink_early_eq_len_ok,
            self.stats.shrink_early_eq_len_ng,
            self.stats.shrink_fail_time,
            self.stats.shrink_fail_guard,
            self.stats.shrink_fail_restore_dir,
            self.stats.shrink_fail_restore_illegal,
            self.stats.shrink_fail_choose_none,
            self.stats.shrink_fail_step,
            self.stats.shrink_fail_target_eaten,
            self.stats.shrink_fail_restore_pos_mismatch,
            self.stats.shrink_fail_restore_bite,
            self.stats.shrink_fail_restore_color_or_len,
            self.stats.shrink_fail_drop_short,
            self.stats.shrink_fail_drop_color_mismatch,
            self.stats.shrink_fail_prefix,
            self.stats.shrink_final_fail,
            self.stats.choose_calls,
            self.stats.choose_pick_bite,
            self.stats.choose_pick_move,
            self.stats.choose_pick_fallback_non_target,
            self.stats.choose_none,
            self.stats.bfs_avoid_calls,
            self.stats.bfs_avoid_found,
            self.stats.bfs_relaxed_calls,
            self.stats.bfs_relaxed_found,
            self.stats.finish_calls,
            self.stats.finish_success,
            self.stats.finish_fail_len,
            self.stats.finish_fail_target_missing,
            self.stats.step_calls,
            self.stats.step_fail,
            self.stats.moves,
            self.stats.eats,
            self.stats.bites
        );
        eprintln!("{line}");
    }

    fn extend_one(&mut self, ell: usize) -> bool {
        self.stats.extend_calls += 1;
        if self.time_over() {
            self.stats.extend_time_over += 1;
            return false;
        }
        let target_color = self.input.d[ell];
        let mut targets = self.collect_food_cells(target_color);
        self.stats.target_candidates_total += targets.len();
        if targets.is_empty() {
            self.stats.extend_no_target += 1;
            return false;
        }

        let head = self.state.pos[0];
        targets.sort_by_key(|&(r, c)| manhattan(head, (r, c)));

        for target in targets {
            if self.time_over() {
                self.stats.extend_time_over += 1;
                return false;
            }
            self.stats.target_trials += 1;
            if self.try_target(ell, target, target_color) {
                self.stats.extend_success += 1;
                return true;
            }
        }
        false
    }

    fn try_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        self.stats.try_target_calls += 1;
        let neighbors = self.neighbors(target);
        if neighbors.is_empty() {
            self.stats.try_target_no_neighbors += 1;
            return false;
        }

        let mut cand = neighbors;
        let head = self.state.pos[0];
        cand.sort_by_key(|&p| manhattan(head, p));

        for goal in cand {
            if self.time_over() {
                return false;
            }
            self.stats.try_target_goal_trials += 1;
            let backup_state = self.state.clone();
            let backup_len = self.ops.len();

            let ok_nav = self.navigate_to_goal(goal, target, ell);
            if !ok_nav {
                self.stats.try_target_fail_navigate += 1;
            }
            let ok_shrink = ok_nav && self.shrink_to_ell(ell, target, target_color);
            if ok_nav && !ok_shrink {
                self.stats.try_target_fail_shrink += 1;
            }
            let ok_finish = ok_shrink && self.finish_eat_target(ell, target, target_color);
            if ok_shrink && !ok_finish {
                self.stats.try_target_fail_finish += 1;
            }
            if ok_finish {
                self.stats.try_target_success += 1;
                return true;
            }

            self.stats.try_target_rollbacks += 1;
            self.state = backup_state;
            self.ops.truncate(backup_len);
        }

        false
    }

    fn finish_eat_target(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        self.stats.finish_calls += 1;
        if self.state.colors.len() != ell {
            self.stats.finish_fail_len += 1;
            return false;
        }
        if self.state.food[self.idx(target.0, target.1)] != target_color {
            self.stats.finish_fail_target_missing += 1;
            return false;
        }
        let head = self.state.pos[0];
        let Some(dir) = dir_between(head, target) else {
            self.stats.finish_fail_not_adjacent += 1;
            return false;
        };
        if !self.is_legal_dir(dir) {
            self.stats.finish_fail_illegal += 1;
            return false;
        }
        if !self.step(dir) {
            self.stats.finish_fail_step += 1;
            return false;
        }
        if self.state.colors.len() != ell + 1 {
            self.stats.finish_fail_len_after += 1;
            return false;
        }
        if self.prefix_match(ell + 1) {
            self.stats.finish_success += 1;
            true
        } else {
            self.stats.finish_fail_prefix_after += 1;
            false
        }
    }

    fn navigate_to_goal(
        &mut self,
        goal: (usize, usize),
        target: (usize, usize),
        ell: usize,
    ) -> bool {
        self.stats.navigate_calls += 1;
        let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
        let mut guard = 0_usize;
        while self.state.pos[0] != goal || !restore_queue.is_empty() {
            if self.time_over() {
                self.stats.navigate_fail_time += 1;
                return false;
            }
            guard += 1;
            if guard > self.input.n * self.input.n * 80 {
                self.stats.navigate_fail_guard += 1;
                return false;
            }

            let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                let head = self.state.pos[0];
                let Some(dir) = dir_between(head, (r, c)) else {
                    self.stats.navigate_fail_restore_dir += 1;
                    return false;
                };
                if !self.is_legal_dir(dir) {
                    self.stats.navigate_fail_restore_illegal += 1;
                    return false;
                }
                dir
            } else {
                let mut next = self.bfs_next_dir(goal, target, true);
                if next.is_none() {
                    next = self.bfs_next_dir(goal, target, false);
                }
                let Some(dir) = next else {
                    self.stats.navigate_fail_bfs_none += 1;
                    return false;
                };
                dir
            };

            let prev_len = self.state.colors.len();
            let Some(outcome) = self.step_with_outcome(dir) else {
                self.stats.navigate_fail_step += 1;
                return false;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                self.stats.navigate_fail_target_eaten += 1;
                return false;
            }

            if let Some((rr, cc, expected)) = restore_queue.pop_front() {
                self.stats.navigate_restore_steps += 1;
                if self.state.pos[0] != (rr, cc) {
                    self.stats.navigate_fail_restore_pos_mismatch += 1;
                    return false;
                }
                if outcome.bite {
                    self.stats.navigate_fail_restore_bite += 1;
                    return false;
                }
                if outcome.ate != Some(expected) || self.state.colors.len() != prev_len + 1 {
                    self.stats.navigate_fail_restore_color_or_len += 1;
                    return false;
                }
                let keep = self.state.colors.len().min(ell);
                if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                    self.stats.navigate_fail_prefix += 1;
                    return false;
                }
                continue;
            }

            let keep = self.state.colors.len().min(ell);
            if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                self.stats.navigate_fail_prefix += 1;
                return false;
            }

            if outcome.bite && self.state.colors.len() < ell {
                self.stats.navigate_bite_events += 1;
                let need = ell - self.state.colors.len();
                if outcome.dropped.len() < need {
                    self.stats.navigate_fail_drop_short += 1;
                    return false;
                }
                for &(r, c, col) in outcome.dropped.iter().take(need) {
                    let idx = self.state.colors.len() + restore_queue.len();
                    if idx >= self.input.d.len() || self.input.d[idx] != col {
                        self.stats.navigate_fail_drop_color_mismatch += 1;
                        return false;
                    }
                    restore_queue.push_back((r, c, col));
                    self.stats.navigate_restore_pushes += 1;
                }
            }
        }

        if self.state.colors.len() >= ell && self.prefix_match(ell) {
            self.stats.navigate_success += 1;
            true
        } else {
            self.stats.navigate_fail_prefix += 1;
            false
        }
    }

    fn shrink_to_ell(&mut self, ell: usize, target: (usize, usize), target_color: u8) -> bool {
        self.stats.shrink_calls += 1;
        if self.state.colors.len() < ell {
            self.stats.shrink_fail_short_len += 1;
            return false;
        }
        if !self.prefix_match(ell) {
            self.stats.shrink_fail_prefix_start += 1;
            return false;
        }
        if self.state.food[self.idx(target.0, target.1)] != target_color {
            self.stats.shrink_fail_target_missing_start += 1;
            return false;
        }

        if self.state.colors.len() == ell {
            if self.can_reach_target_next(target) {
                self.stats.shrink_early_eq_len_ok += 1;
                self.stats.shrink_success += 1;
                return true;
            }
            self.stats.shrink_early_eq_len_ng += 1;
            return false;
        }

        let mut restore_queue: VecDeque<(usize, usize, u8)> = VecDeque::new();
        let mut guard = 0_usize;
        while self.state.colors.len() != ell
            || !restore_queue.is_empty()
            || !self.can_reach_target_next(target)
        {
            if self.time_over() {
                self.stats.shrink_fail_time += 1;
                return false;
            }
            guard += 1;
            if guard > self.input.n * self.input.n * 60 {
                self.stats.shrink_fail_guard += 1;
                return false;
            }

            let dir = if let Some(&(r, c, _expected)) = restore_queue.front() {
                let head = self.state.pos[0];
                let Some(dir) = dir_between(head, (r, c)) else {
                    self.stats.shrink_fail_restore_dir += 1;
                    return false;
                };
                if !self.is_legal_dir(dir) {
                    self.stats.shrink_fail_restore_illegal += 1;
                    return false;
                }
                dir
            } else {
                let Some(dir) = self.choose_shrink_dir(ell, target) else {
                    self.stats.shrink_fail_choose_none += 1;
                    return false;
                };
                dir
            };

            let prev_len = self.state.colors.len();
            let Some(outcome) = self.step_with_outcome(dir) else {
                self.stats.shrink_fail_step += 1;
                return false;
            };
            if self.state.food[self.idx(target.0, target.1)] == 0 {
                self.stats.shrink_fail_target_eaten += 1;
                return false;
            }

            if let Some((rr, cc, expected)) = restore_queue.pop_front() {
                self.stats.shrink_restore_steps += 1;
                if self.state.pos[0] != (rr, cc) {
                    self.stats.shrink_fail_restore_pos_mismatch += 1;
                    return false;
                }
                if outcome.bite {
                    self.stats.shrink_fail_restore_bite += 1;
                    return false;
                }
                if outcome.ate != Some(expected) || self.state.colors.len() != prev_len + 1 {
                    self.stats.shrink_fail_restore_color_or_len += 1;
                    return false;
                }
                let keep = self.state.colors.len().min(ell);
                if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                    self.stats.shrink_fail_prefix += 1;
                    return false;
                }
                continue;
            }

            let keep = self.state.colors.len().min(ell);
            if !prefix_match_upto_state(&self.state, &self.input.d, keep) {
                self.stats.shrink_fail_prefix += 1;
                return false;
            }

            if outcome.bite && self.state.colors.len() < ell {
                self.stats.shrink_bite_events += 1;
                let need = ell - self.state.colors.len();
                if outcome.dropped.len() < need {
                    self.stats.shrink_fail_drop_short += 1;
                    return false;
                }
                for &(r, c, col) in outcome.dropped.iter().take(need) {
                    let idx = self.state.colors.len() + restore_queue.len();
                    if idx >= self.input.d.len() || self.input.d[idx] != col {
                        self.stats.shrink_fail_drop_color_mismatch += 1;
                        return false;
                    }
                    restore_queue.push_back((r, c, col));
                    self.stats.shrink_restore_pushes += 1;
                }
            }
        }

        let ok = self.state.colors.len() == ell
            && self.prefix_match(ell)
            && self.state.food[self.idx(target.0, target.1)] == target_color
            && self.can_reach_target_next(target);
        if ok {
            self.stats.shrink_success += 1;
        } else {
            self.stats.shrink_final_fail += 1;
        }
        ok
    }

    fn choose_shrink_dir(&mut self, ell: usize, target: (usize, usize)) -> Option<usize> {
        self.stats.choose_calls += 1;
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
            self.stats.choose_pick_bite += 1;
            return Some(dir);
        }
        if let Some((_, dir)) = best_move {
            self.stats.choose_pick_move += 1;
            return Some(dir);
        }

        for dir in legal_dirs(&self.state) {
            if next_head(&self.state, dir) != target {
                self.stats.choose_pick_fallback_non_target += 1;
                return Some(dir);
            }
        }
        self.stats.choose_none += 1;
        None
    }

    fn bfs_next_dir(
        &mut self,
        goal: (usize, usize),
        target: (usize, usize),
        avoid_food: bool,
    ) -> Option<usize> {
        if avoid_food {
            self.stats.bfs_avoid_calls += 1;
        } else {
            self.stats.bfs_relaxed_calls += 1;
        }
        let n = self.state.n;
        let start = self.state.pos[0];
        if start == goal {
            if avoid_food {
                self.stats.bfs_avoid_found += 1;
            } else {
                self.stats.bfs_relaxed_found += 1;
            }
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
                if avoid_food {
                    self.stats.bfs_avoid_found += 1;
                } else {
                    self.stats.bfs_relaxed_found += 1;
                }
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
        self.stats.step_calls += 1;
        if self.ops.len() >= MAX_TURNS {
            self.stats.step_fail += 1;
            return None;
        }
        if !self.is_legal_dir(dir) {
            self.stats.step_fail += 1;
            return None;
        }
        let outcome = apply_move(&mut self.state, dir);
        self.ops.push(DIRS[dir].2 as u8);
        self.stats.moves += 1;
        if outcome.ate.is_some() {
            self.stats.eats += 1;
        }
        if outcome.bite {
            self.stats.bites += 1;
        }
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
    solver.log_summary();

    let out = solver
        .ops
        .iter()
        .map(|&x| (x as char).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    println!("{out}");
}
