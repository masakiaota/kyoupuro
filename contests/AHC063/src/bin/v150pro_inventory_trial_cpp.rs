// v150pro_inventory_trial_cpp.rs
mod base {
    #![allow(dead_code)]
    #![allow(unused_imports)]
    #![allow(unused_variables)]

    include!("v141pro_inventory_stock.rs");

    const V150_TIME_LIMIT_SEC: f64 = 1.92;
    const V150_SAFE_LEFT_SEC: f64 = 0.18;
    const V150_BUILD_TIME_CAP_SEC: f64 = 0.36;
    const V150_TRANSPORT_TIME_CAP_SEC: f64 = 0.22;
    const V150_DEPART_TIME_CAP_SEC: f64 = 0.20;
    const V150_GOAL_EXTRA_LEN_LIMIT: usize = 20;
    const V150_DEPART_EXTRA_LEN_LIMIT: usize = 24;
    const V150_DEPART_NODE_LIMIT: usize = 160_000;
    const V150_PHASE_CANDIDATE_LIMIT: usize = 3;
    const V150_INF: usize = 1_000_000_000;
    const V150_BUDGETS: &[(usize, usize)] = &[(2_000, 20), (8_000, 20), (25_000, 24)];

    type FxHashMap<K, V> =
        std::collections::HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;
    type FxHashSet<T> = std::collections::HashSet<T, std::hash::BuildHasherDefault<FxHasher>>;
    type Score5 = [usize; 5];

    #[inline]
    fn v150_debug_enabled() -> bool {
        std::env::var_os("V150_DEBUG").is_some()
    }

    #[derive(Default)]
    struct FxHasher {
        hash: u64,
    }

    impl FxHasher {
        #[inline]
        fn mix(&mut self, x: u64) {
            self.hash ^= x;
            self.hash = self.hash.rotate_left(5).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }

    impl std::hash::Hasher for FxHasher {
        #[inline]
        fn finish(&self) -> u64 {
            self.hash
        }

        #[inline]
        fn write(&mut self, bytes: &[u8]) {
            let mut idx = 0usize;
            while idx + 8 <= bytes.len() {
                let mut chunk = [0u8; 8];
                chunk.copy_from_slice(&bytes[idx..idx + 8]);
                self.mix(u64::from_le_bytes(chunk));
                idx += 8;
            }
            if idx < bytes.len() {
                let mut tail = [0u8; 8];
                tail[..bytes.len() - idx].copy_from_slice(&bytes[idx..]);
                self.mix(u64::from_le_bytes(tail));
            }
        }

        #[inline]
        fn write_u8(&mut self, i: u8) {
            self.mix(i as u64);
        }

        #[inline]
        fn write_u16(&mut self, i: u16) {
            self.mix(i as u64);
        }

        #[inline]
        fn write_u32(&mut self, i: u32) {
            self.mix(i as u64);
        }

        #[inline]
        fn write_u64(&mut self, i: u64) {
            self.mix(i);
        }

        #[inline]
        fn write_usize(&mut self, i: usize) {
            self.mix(i as u64);
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct MovementConstraint {
        min_allowed_col: u8,
        allow_special_strip: bool,
        special_col: u8,
        special_row_min: u8,
    }

    impl MovementConstraint {
        #[inline]
        fn unconstrained() -> Self {
            Self {
                min_allowed_col: 0,
                allow_special_strip: false,
                special_col: 0,
                special_row_min: 0,
            }
        }

        #[inline]
        fn allows(&self, n: usize, cell: Cell) -> bool {
            let row = Grid::i(n, cell);
            let col = Grid::j(n, cell);
            if col >= self.min_allowed_col as usize {
                return true;
            }
            self.allow_special_strip
                && col == self.special_col as usize
                && row >= self.special_row_min as usize
        }
    }

    #[derive(Debug, Clone)]
    struct DroppedBuf {
        cells: [Cell; INTERNAL_POS_DEQUE_CAPACITY],
        colors: [u8; INTERNAL_POS_DEQUE_CAPACITY],
        len: usize,
    }

    impl DroppedBuf {
        #[inline]
        fn new() -> Self {
            Self {
                cells: [Cell(0); INTERNAL_POS_DEQUE_CAPACITY],
                colors: [0; INTERNAL_POS_DEQUE_CAPACITY],
                len: 0,
            }
        }

        #[inline]
        fn clear(&mut self) {
            self.len = 0;
        }

        #[inline]
        fn push(&mut self, cell: Cell, color: u8) {
            self.cells[self.len] = cell;
            self.colors[self.len] = color;
            self.len += 1;
        }

        #[inline]
        fn reverse(&mut self) {
            if self.len <= 1 {
                return;
            }
            let mut l = 0usize;
            let mut r = self.len - 1;
            while l < r {
                self.cells.swap(l, r);
                self.colors.swap(l, r);
                l += 1;
                r -= 1;
            }
        }
    }

    #[derive(Debug, Clone)]
    struct StepBufResult {
        state: State,
        ate: u8,
        bite_idx: Option<usize>,
    }

    #[derive(Debug, Clone, Copy)]
    struct BuildProfile {
        stage_keep: usize,
        node_limit: usize,
        budgets: &'static [(usize, usize)],
    }

    #[derive(Debug, Clone, Copy)]
    struct InventoryGeom {
        entry_hi: Cell,
        entry_lo: Cell,
        pre_hi: Cell,
        pre_lo: Cell,
    }

    #[derive(Debug, Clone)]
    struct StageNode {
        state: State,
        parent: Option<usize>,
        move_seg: Ops,
    }

    #[derive(Debug, Clone)]
    struct GoalNode {
        state: State,
        parent: Option<usize>,
        move_seg: Ops,
    }

    #[inline]
    fn dir_between_opt(n: usize, from: Cell, to: Cell) -> Option<Dir> {
        let from_idx = Grid::index(from);
        let to_idx = Grid::index(to);
        if to_idx + n == from_idx {
            Some(DIR_U)
        } else if from_idx + n == to_idx {
            Some(DIR_D)
        } else if to_idx + 1 == from_idx && from_idx / n == to_idx / n {
            Some(DIR_L)
        } else if from_idx + 1 == to_idx && from_idx / n == to_idx / n {
            Some(DIR_R)
        } else {
            None
        }
    }

    #[inline]
    fn is_legal_dir_with_constraint(
        state: &State,
        n: usize,
        dir: Dir,
        constraint: MovementConstraint,
    ) -> bool {
        if !state.is_legal_dir(n, dir) {
            return false;
        }
        let next = Grid::next_cell(n, state.head(), dir);
        constraint.allows(n, next)
    }

    #[inline]
    fn legal_dirs_buf(
        state: &State,
        n: usize,
        constraint: MovementConstraint,
    ) -> ([Dir; DIR_COUNT], usize) {
        let mut out = [DIR_U; DIR_COUNT];
        let mut len = 0usize;
        for dir in ALL_DIRS {
            if is_legal_dir_with_constraint(state, n, dir, constraint) {
                out[len] = dir;
                len += 1;
            }
        }
        (out, len)
    }

    #[inline]
    fn step_with_dropped_buf(
        state: &State,
        n: usize,
        dir: Dir,
        dropped: &mut DroppedBuf,
    ) -> StepBufResult {
        debug_assert!(state.is_legal_dir(n, dir));
        dropped.clear();

        let next_head = Grid::next_cell(n, state.head(), dir);
        let mut food = state.food.clone();
        let mut new_pos = state.pos.clone();
        let mut new_colors = state.colors.clone();
        let mut new_pos_occupancy = state.pos_occupancy.clone();

        let mut ate = 0u8;
        let eat_idx = Grid::index(next_head);
        if food[eat_idx] != 0 {
            ate = food[eat_idx];
            food[eat_idx] = 0;
            new_colors.push(ate);
        } else {
            let old_tail = new_pos.pop_back().unwrap();
            new_pos_occupancy.dec(old_tail);
        }

        let excluded_tail = new_pos.back();
        let tail_bias = u8::from(excluded_tail == Some(next_head));
        let bite = new_pos_occupancy.count(next_head) > tail_bias;

        new_pos_occupancy.inc(next_head);
        new_pos.push_front(next_head);
        let bite_idx = if bite {
            find_internal_bite_idx(&new_pos)
        } else {
            None
        };

        if let Some(h) = bite_idx {
            while new_pos.len() > h + 1 {
                let cell = new_pos.pop_back().unwrap();
                new_pos_occupancy.dec(cell);
                let color = new_colors.pop().unwrap();
                food[Grid::index(cell)] = color;
                dropped.push(cell, color);
            }
            dropped.reverse();
        }

        StepBufResult {
            state: State {
                food,
                pos: new_pos,
                colors: new_colors,
                pos_occupancy: new_pos_occupancy,
            },
            ate,
            bite_idx,
        }
    }

    #[inline]
    fn prefix_ok_v150(state: &State, target: &[u8], ell: usize) -> bool {
        matches_prefix_len(&state.colors, target, state.colors.len().min(ell))
    }

    #[inline]
    fn exact_prefix_v150(state: &State, target: &[u8], ell: usize) -> bool {
        state.colors.len() == ell && matches_prefix_len(&state.colors, target, ell)
    }

    #[inline]
    fn dropped_respects_constraint(
        n: usize,
        dropped: &DroppedBuf,
        constraint: MovementConstraint,
    ) -> bool {
        for idx in 0..dropped.len {
            if !constraint.allows(n, dropped.cells[idx]) {
                return false;
            }
        }
        true
    }

    fn repair_prefix_after_bite_constrained(
        st_after: &State,
        n: usize,
        prefix_target: &[u8],
        ell: usize,
        dropped: &DroppedBuf,
        constraint: MovementConstraint,
    ) -> Option<(State, Ops)> {
        let need = ell.saturating_sub(st_after.colors.len());
        if need == 0 {
            return Some((st_after.clone(), Ops::new()));
        }
        if dropped.len < need {
            return None;
        }

        let mut food = st_after.food.clone();
        let mut pos = st_after.pos.clone();
        let mut pos_occupancy = st_after.pos_occupancy.clone();
        let mut ops = Ops::with_capacity(need);
        let mut prev = st_after.head();

        for offset in 0..need {
            let cell = dropped.cells[offset];
            let color = dropped.colors[offset];
            let color_idx = st_after.colors.len() + offset;
            if color != prefix_target[color_idx] {
                return None;
            }
            if !constraint.allows(n, cell) {
                return None;
            }
            if food[Grid::index(cell)] != color {
                return None;
            }
            let dir = dir_between_opt(n, prev, cell)?;
            if pos.len() >= 2 && cell == pos[1] {
                return None;
            }
            food[Grid::index(cell)] = 0;
            pos.push_front(cell);
            pos_occupancy.inc(cell);
            ops.push(dir);
            prev = cell;
        }

        Some((
            State {
                food,
                pos,
                colors: InternalColors::from_slice(&prefix_target[..ell]),
                pos_occupancy,
            },
            ops,
        ))
    }

    #[inline]
    fn nearest_food_dist(
        state: &State,
        n: usize,
        color: u8,
        constraint: MovementConstraint,
    ) -> (usize, usize) {
        let mut best = V150_INF;
        let mut cnt = 0usize;
        for idx in 0..(n * n) {
            if state.food[idx] == color {
                let cell = Cell(idx as u16);
                if !constraint.allows(n, cell) {
                    continue;
                }
                cnt += 1;
                best = best.min(manhattan_cell(n, state.head(), cell));
            }
        }
        (best, cnt)
    }

    #[inline]
    fn target_adjacent(
        state: &State,
        n: usize,
        color: u8,
        constraint: MovementConstraint,
    ) -> Option<Dir> {
        for dir in ALL_DIRS {
            if !is_legal_dir_with_constraint(state, n, dir, constraint) {
                continue;
            }
            let next = Grid::next_cell(n, state.head(), dir);
            if state.food[Grid::index(next)] == color {
                return Some(dir);
            }
        }
        None
    }

    #[inline]
    fn target_suffix_info(state: &State, n: usize, ell: usize, target: u8) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        for idx in ell..state.colors.len() {
            if state.colors[idx] != target {
                continue;
            }
            let prev = state.pos[idx - 1];
            let cand = (manhattan_cell(n, state.head(), prev), idx);
            if best.map_or(true, |cur| cand < cur) {
                best = Some(cand);
            }
        }
        best
    }

    #[inline]
    fn local_score(
        state: &State,
        n: usize,
        target_colors: &[u8],
        ell: usize,
        constraint: MovementConstraint,
    ) -> Score5 {
        let target = target_colors[ell];
        if exact_prefix_v150(state, target_colors, ell) {
            let (dist, _) = nearest_food_dist(state, n, target, constraint);
            let adj = target_adjacent(state, n, target, constraint).is_some();
            return [0, usize::from(!adj), dist, 0, state.colors.len().saturating_sub(ell)];
        }
        if let Some((dist, idx)) = target_suffix_info(state, n, ell, target) {
            return [1, 0, dist, idx - ell, state.colors.len().saturating_sub(ell)];
        }
        let (dist, _) = nearest_food_dist(state, n, target, constraint);
        [2, 0, dist, 0, state.colors.len().saturating_sub(ell)]
    }

    fn append_reconstruct_stage_plan(nodes: &[StageNode], mut idx: usize, out: &mut Ops) {
        let mut rev = Vec::new();
        let mut total = 0usize;
        while let Some(parent) = nodes[idx].parent {
            total += nodes[idx].move_seg.len();
            rev.push(idx);
            idx = parent;
        }
        out.reserve(total);
        for &node_idx in rev.iter().rev() {
            out.extend_from_slice(&nodes[node_idx].move_seg);
        }
    }

    fn append_reconstruct_goal_plan(nodes: &[GoalNode], mut idx: usize, out: &mut Ops) {
        let mut rev = Vec::new();
        let mut total = 0usize;
        while let Some(parent) = nodes[idx].parent {
            total += nodes[idx].move_seg.len();
            rev.push(idx);
            idx = parent;
        }
        out.reserve(total);
        for &node_idx in rev.iter().rev() {
            out.extend_from_slice(&nodes[node_idx].move_seg);
        }
    }

    fn stage_search_bestfirst(
        input: &Input,
        start_bs: &BeamState,
        target_colors: &[u8],
        ell: usize,
        constraint: MovementConstraint,
        budgets: &[(usize, usize)],
        keep_solutions: usize,
        time_limit_sec: f64,
        timer: &TimeKeeper,
    ) -> Vec<BeamState> {
        let local_start = Instant::now();
        let local_over = || {
            local_start.elapsed().as_secs_f64() > time_limit_sec
                || timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC
        };

        let mut nodes = Vec::with_capacity(budgets.last().map_or(1024, |x| x.0.min(30_000)) + 8);
        nodes.push(StageNode {
            state: start_bs.state.clone(),
            parent: None,
            move_seg: Ops::new(),
        });

        let mut pq = BinaryHeap::new();
        let mut uid = 0usize;
        pq.push(Reverse((
            local_score(&start_bs.state, input.n, target_colors, ell, constraint),
            0usize,
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<StateSig, usize>::default();
        seen.insert(state_sig(&start_bs.state), 0);
        let mut sol_seen = FxHashSet::<StateSig>::default();
        let mut sols = Vec::with_capacity(keep_solutions);
        let mut dropped = DroppedBuf::new();

        let mut expansions = 0usize;
        let mut stage_idx = 0usize;
        let mut stage_limit = budgets[0].0;
        let mut extra_limit = budgets[0].1;
        let final_limit = budgets.last().unwrap().0;

        while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
            if expansions >= final_limit || sols.len() >= keep_solutions || local_over() {
                break;
            }
            expansions += 1;
            let st = nodes[idx].state.clone();

            if exact_prefix_v150(&st, target_colors, ell) {
                if let Some(dir) = target_adjacent(&st, input.n, target_colors[ell], constraint) {
                    let res = step_with_dropped_buf(&st, input.n, dir, &mut dropped);
                    if res.bite_idx.is_none() && exact_prefix_v150(&res.state, target_colors, ell + 1)
                    {
                        let sig = state_sig(&res.state);
                        if sol_seen.insert(sig) {
                            let mut ops = start_bs.ops.clone();
                            append_reconstruct_stage_plan(&nodes, idx, &mut ops);
                            ops.push(dir);
                            sols.push(BeamState {
                                state: res.state,
                                ops,
                            });
                            if sols.len() >= keep_solutions {
                                break;
                            }
                        }
                    }
                }
            }

            let (dirs, dir_count) = legal_dirs_buf(&st, input.n, constraint);
            for &dir in dirs[..dir_count].iter() {
                let res = step_with_dropped_buf(&st, input.n, dir, &mut dropped);
                if res.bite_idx.is_some()
                    && !dropped_respects_constraint(input.n, &dropped, constraint)
                {
                    continue;
                }

                let mut next_state = res.state;
                let mut move_seg = vec![dir];
                if res.bite_idx.is_some() && next_state.len() < ell {
                    if !prefix_ok_v150(&next_state, target_colors, ell) {
                        continue;
                    }
                    let Some((repaired, repair_ops)) = repair_prefix_after_bite_constrained(
                        &next_state,
                        input.n,
                        target_colors,
                        ell,
                        &dropped,
                        constraint,
                    ) else {
                        continue;
                    };
                    next_state = repaired;
                    move_seg.extend_from_slice(&repair_ops);
                }
                if !prefix_ok_v150(&next_state, target_colors, ell) {
                    continue;
                }
                if next_state.len() > ell + extra_limit {
                    continue;
                }

                let nd = depth + move_seg.len();
                let sig = state_sig(&next_state);
                if seen.get(&sig).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(sig, nd);

                let child = nodes.len();
                nodes.push(StageNode {
                    state: next_state,
                    parent: Some(idx),
                    move_seg,
                });
                pq.push(Reverse((
                    local_score(&nodes[child].state, input.n, target_colors, ell, constraint),
                    nd,
                    uid,
                    child,
                )));
                uid += 1;
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

        let next_stage_rank = |bs: &BeamState| -> (usize, usize, usize) {
            if ell + 1 >= target_colors.len() {
                return (0, 0, 0);
            }
            let (dist, _) = nearest_food_dist(&bs.state, input.n, target_colors[ell + 1], constraint);
            let (hr, hc) = Grid::ij(input.n, bs.state.head());
            let center = hr.abs_diff(input.n / 2) + hc.abs_diff(input.n / 2);
            (dist, center, 0)
        };

        sols.sort_unstable_by_key(|bs| (next_stage_rank(bs), bs.ops.len()));

        let mut out = Vec::with_capacity(keep_solutions);
        let mut uniq = FxHashSet::<StateSig>::default();
        for bs in sols {
            let sig = state_sig(&bs.state);
            if uniq.insert(sig) {
                out.push(bs);
                if out.len() >= keep_solutions {
                    break;
                }
            }
        }
        out
    }

    fn grow_to_target_prefix_beam_with_goal(
        input: &Input,
        start_bs: &BeamState,
        target_colors: &[u8],
        constraint: MovementConstraint,
        final_goal_cells: &[Cell],
        stage_keep: usize,
        total_time_sec: f64,
        budgets: &[(usize, usize)],
        timer: &TimeKeeper,
    ) -> Vec<BeamState> {
        let start_time = Instant::now();
        let mut beam = vec![start_bs.clone()];
        let start_ell = start_bs.state.colors.len();

        for ell in start_ell..target_colors.len() {
            if start_time.elapsed().as_secs_f64() > total_time_sec
                || timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC
            {
                break;
            }

            let mut next = Vec::new();
            for bs in &beam {
                let rem_time = total_time_sec - start_time.elapsed().as_secs_f64();
                if rem_time <= 0.0 || timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC {
                    break;
                }
                let per = (rem_time / beam.len().max(1) as f64).clamp(0.05, 0.8);
                let sols = stage_search_bestfirst(
                    input,
                    bs,
                    target_colors,
                    ell,
                    constraint,
                    budgets,
                    stage_keep,
                    per,
                    timer,
                );
                next.extend(sols);
            }
            if next.is_empty() {
                return Vec::new();
            }

            let rem = target_colors.len().saturating_sub(ell + 1);
            let goal_dist = |bs: &BeamState| -> usize {
                if final_goal_cells.is_empty() {
                    return 0;
                }
                final_goal_cells
                    .iter()
                    .map(|&g| manhattan_cell(input.n, bs.state.head(), g))
                    .min()
                    .unwrap_or(V150_INF)
            };
            let rank_main = |bs: &BeamState| -> (usize, usize, usize) {
                let dist_next = if ell + 1 < target_colors.len() {
                    nearest_food_dist(&bs.state, input.n, target_colors[ell + 1], constraint)
                        .0
                } else {
                    0
                };
                let scaled = dist_next.saturating_mul(rem.max(1) + 1) + goal_dist(bs);
                let (_, dir_count) = legal_dirs_buf(&bs.state, input.n, constraint);
                (scaled, bs.ops.len(), DIR_COUNT - dir_count)
            };

            let mut ord1: Vec<usize> = (0..next.len()).collect();
            let mut ord2 = ord1.clone();
            ord1.sort_unstable_by_key(|&idx| rank_main(&next[idx]));
            ord2.sort_unstable_by_key(|&idx| (goal_dist(&next[idx]), next[idx].ops.len()));

            let mut uniq = Vec::with_capacity(stage_keep);
            let mut seen = FxHashSet::<StateSig>::default();
            for ord in [&ord1, &ord2] {
                for &idx in ord {
                    let sig = state_sig(&next[idx].state);
                    if seen.insert(sig) {
                        uniq.push(next[idx].clone());
                        if uniq.len() >= stage_keep {
                            break;
                        }
                    }
                }
                if uniq.len() >= stage_keep {
                    break;
                }
            }
            beam = uniq;
        }

        beam.into_iter()
            .filter(|bs| exact_prefix_v150(&bs.state, target_colors, target_colors.len()))
            .collect()
    }

    fn search_preserve_prefix_to_goal<F>(
        input: &Input,
        start_bs: &BeamState,
        target_colors: &[u8],
        protect_len: usize,
        constraint: MovementConstraint,
        goal_predicate: F,
        goal_hint_cells: &[Cell],
        node_limit: usize,
        extra_limit: usize,
        time_limit_sec: f64,
        timer: &TimeKeeper,
    ) -> Option<BeamState>
    where
        F: Fn(&State) -> bool,
    {
        let local_start = Instant::now();
        let local_over = || {
            local_start.elapsed().as_secs_f64() > time_limit_sec
                || timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC
        };

        let rank_state = |state: &State| -> [usize; 4] {
            let dist = if goal_hint_cells.is_empty() {
                0
            } else {
                goal_hint_cells
                    .iter()
                    .map(|&goal| manhattan_cell(input.n, state.head(), goal))
                    .min()
                    .unwrap_or(V150_INF)
            };
            let (_, dir_count) = legal_dirs_buf(state, input.n, constraint);
            [
                usize::from(!goal_predicate(state)),
                dist,
                state.len().saturating_sub(protect_len),
                DIR_COUNT - dir_count,
            ]
        };

        let mut nodes = Vec::with_capacity(node_limit.min(30_000) + 8);
        nodes.push(GoalNode {
            state: start_bs.state.clone(),
            parent: None,
            move_seg: Ops::new(),
        });

        let mut pq = BinaryHeap::new();
        let mut uid = 0usize;
        pq.push(Reverse((rank_state(&start_bs.state), 0usize, uid, 0usize)));
        uid += 1;

        let mut seen = FxHashMap::<StateSig, usize>::default();
        seen.insert(state_sig(&start_bs.state), 0);
        let mut dropped = DroppedBuf::new();

        while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
            if nodes.len() >= node_limit || local_over() {
                break;
            }

            let st = nodes[idx].state.clone();
            if goal_predicate(&st) {
                let mut ops = start_bs.ops.clone();
                append_reconstruct_goal_plan(&nodes, idx, &mut ops);
                return Some(BeamState { state: st, ops });
            }

            let (dirs, dir_count) = legal_dirs_buf(&st, input.n, constraint);
            for &dir in dirs[..dir_count].iter() {
                let res = step_with_dropped_buf(&st, input.n, dir, &mut dropped);
                if res.bite_idx.is_some()
                    && !dropped_respects_constraint(input.n, &dropped, constraint)
                {
                    continue;
                }

                let mut next_state = res.state;
                let mut move_seg = vec![dir];
                if res.bite_idx.is_some() && next_state.len() < protect_len {
                    if !prefix_ok_v150(&next_state, target_colors, protect_len) {
                        continue;
                    }
                    let Some((repaired, repair_ops)) = repair_prefix_after_bite_constrained(
                        &next_state,
                        input.n,
                        target_colors,
                        protect_len,
                        &dropped,
                        constraint,
                    ) else {
                        continue;
                    };
                    next_state = repaired;
                    move_seg.extend_from_slice(&repair_ops);
                }
                if !prefix_ok_v150(&next_state, target_colors, protect_len) {
                    continue;
                }
                if next_state.len() > protect_len + extra_limit {
                    continue;
                }

                let nd = depth + move_seg.len();
                let sig = state_sig(&next_state);
                if seen.get(&sig).copied().unwrap_or(usize::MAX) <= nd {
                    continue;
                }
                seen.insert(sig, nd);

                let child = nodes.len();
                nodes.push(GoalNode {
                    state: next_state,
                    parent: Some(idx),
                    move_seg,
                });
                pq.push(Reverse((rank_state(&nodes[child].state), nd, uid, child)));
                uid += 1;
            }
        }

        None
    }

    #[inline]
    fn movement_constraint_with_min_col(min_allowed_col: usize) -> MovementConstraint {
        MovementConstraint {
            min_allowed_col: min_allowed_col as u8,
            allow_special_strip: false,
            special_col: 0,
            special_row_min: 0,
        }
    }

    #[inline]
    fn movement_constraint_for_harvest_entry(stock_cnt: usize, n: usize) -> MovementConstraint {
        MovementConstraint {
            min_allowed_col: (2 * stock_cnt) as u8,
            allow_special_strip: true,
            special_col: (2 * stock_cnt - 1) as u8,
            special_row_min: (n - 2) as u8,
        }
    }

    #[inline]
    fn inventory_entry_geometry(n: usize, seg_idx: usize) -> Option<InventoryGeom> {
        let entry_col = 2 * seg_idx + 1;
        let pre_col = entry_col + 1;
        if pre_col >= n {
            return None;
        }
        Some(InventoryGeom {
            entry_hi: Grid::cell(n, n - 2, entry_col),
            entry_lo: Grid::cell(n, n - 1, entry_col),
            pre_hi: Grid::cell(n, n - 2, pre_col),
            pre_lo: Grid::cell(n, n - 1, pre_col),
        })
    }

    fn inventory_segment_target(input: &Input, seg_idx: usize, seg_len: usize) -> Vec<u8> {
        let seg_end = input.m - seg_idx * seg_len;
        let seg_begin = seg_end - seg_len;
        let mut out = Vec::with_capacity(5 + seg_len);
        out.extend_from_slice(&input.d[..5]);
        out.extend_from_slice(&input.d[seg_begin..seg_end]);
        out
    }

    fn inventory_tail_target(input: &Input, stock_cnt: usize, seg_len: usize) -> Vec<u8> {
        let tail_end = input.m - stock_cnt * seg_len;
        input.d[..tail_end].to_vec()
    }

    fn inventory_place_from_entry(
        input: &Input,
        start: &State,
        protect_target: &[u8],
        protect_len: usize,
        seg_idx: usize,
    ) -> Option<(State, Ops)> {
        let geom = inventory_entry_geometry(input.n, seg_idx)?;
        let mut ops = Ops::new();
        if start.head() == geom.entry_hi {
            ops.extend_from_slice(&[DIR_D, DIR_L]);
        } else if start.head() == geom.entry_lo {
            ops.push(DIR_L);
        } else {
            return None;
        }

        for _ in 0..(input.n - 1) {
            ops.push(DIR_U);
        }
        ops.push(DIR_R);
        for _ in 0..(input.n - 2) {
            ops.push(DIR_D);
        }
        ops.extend_from_slice(&[DIR_D, DIR_L, DIR_U, DIR_R]);

        let mut cur = start.clone();
        let mut dropped = DroppedBuf::new();
        let unconstrained = MovementConstraint::unconstrained();
        for (idx, &dir) in ops.iter().enumerate() {
            if !is_legal_dir_with_constraint(&cur, input.n, dir, unconstrained) {
                return None;
            }
            let res = step_with_dropped_buf(&cur, input.n, dir, &mut dropped);
            let is_last = idx + 1 == ops.len();
            if !is_last {
                if res.state.len() < protect_len || !prefix_ok_v150(&res.state, protect_target, protect_len)
                {
                    return None;
                }
                if res.bite_idx.is_some()
                    && res.state.head() != geom.entry_hi
                    && res.state.head() != geom.entry_lo
                {
                    return None;
                }
            } else if res.bite_idx.is_none() || res.state.len() != 5 {
                return None;
            }
            cur = res.state;
        }

        Some((cur, ops))
    }

    fn place_inventory_segment(
        input: &Input,
        start_bs: &BeamState,
        protect_target: &[u8],
        seg_idx: usize,
    ) -> Option<BeamState> {
        let (state, seg_ops) =
            inventory_place_from_entry(input, &start_bs.state, protect_target, protect_target.len(), seg_idx)?;
        let mut ops = start_bs.ops.clone();
        ops.extend_from_slice(&seg_ops);
        if ops.len() > MAX_TURNS {
            return None;
        }
        Some(BeamState { state, ops })
    }

    fn harvest_inventory(input: &Input, start_bs: &BeamState, stock_cnt: usize) -> Option<BeamState> {
        let mut state = start_bs.state.clone();
        let mut ops = start_bs.ops.clone();
        let mut dropped = DroppedBuf::new();

        for seg_idx in (0..stock_cnt).rev() {
            let up_steps = if seg_idx + 1 == stock_cnt {
                input.n - 2
            } else {
                input.n - 3
            };
            for _ in 0..up_steps {
                if !state.is_legal_dir(input.n, DIR_U) {
                    return None;
                }
                let res = step_with_dropped_buf(&state, input.n, DIR_U, &mut dropped);
                if res.bite_idx.is_some() {
                    return None;
                }
                state = res.state;
                ops.push(DIR_U);
            }

            if !state.is_legal_dir(input.n, DIR_L) {
                return None;
            }
            let res_left = step_with_dropped_buf(&state, input.n, DIR_L, &mut dropped);
            if res_left.bite_idx.is_some() {
                return None;
            }
            state = res_left.state;
            ops.push(DIR_L);

            for _ in 0..(input.n - 3) {
                if !state.is_legal_dir(input.n, DIR_D) {
                    return None;
                }
                let res = step_with_dropped_buf(&state, input.n, DIR_D, &mut dropped);
                if res.bite_idx.is_some() {
                    return None;
                }
                state = res.state;
                ops.push(DIR_D);
            }

            if seg_idx > 0 {
                if !state.is_legal_dir(input.n, DIR_L) {
                    return None;
                }
                let res = step_with_dropped_buf(&state, input.n, DIR_L, &mut dropped);
                if res.bite_idx.is_some() {
                    return None;
                }
                state = res.state;
                ops.push(DIR_L);
            }
        }

        Some(BeamState { state, ops })
    }

    fn depart_after_placement(
        input: &Input,
        start_bs: &BeamState,
        next_seg_idx: usize,
        time_limit_sec: f64,
        timer: &TimeKeeper,
    ) -> Option<BeamState> {
        let target = [1_u8; 5];
        let constraint = movement_constraint_with_min_col(2 * next_seg_idx);
        let goal = |state: &State| -> bool {
            if state.len() != 5 {
                return false;
            }
            if !exact_prefix_v150(state, &target, 5) {
                return false;
            }
            state.pos.iter().all(|cell| constraint.allows(input.n, cell))
        };
        let hint_col = constraint.min_allowed_col as usize;
        let hints: Vec<Cell> = (0..input.n)
            .map(|row| Grid::cell(input.n, row, hint_col.min(input.n - 1)))
            .collect();

        search_preserve_prefix_to_goal(
            input,
            start_bs,
            &target,
            5,
            constraint,
            goal,
            &hints,
            V150_DEPART_NODE_LIMIT,
            V150_DEPART_EXTRA_LEN_LIMIT,
            time_limit_sec,
            timer,
        )
    }

    fn validate_full_match_v150(input: &Input, ops: &[Dir]) -> bool {
        if ops.len() > MAX_TURNS {
            return false;
        }
        let mut state = State::initial(input);
        for &dir in ops {
            if !state.is_legal_dir(input.n, dir) {
                return false;
            }
            let step_result = step(&state, input.n, dir);
            state = step_result.state;
        }
        state.len() == input.m && exact_prefix_v150(&state, &input.d, input.m)
    }

    #[inline]
    fn inventory_build_time_limit(timer: &TimeKeeper, phases_left: usize) -> f64 {
        let free = (timer.exact_remaining_sec() - V150_SAFE_LEFT_SEC).max(0.0);
        (free / (phases_left.max(1) as f64 + 1.0)).min(V150_BUILD_TIME_CAP_SEC)
    }

    #[inline]
    fn inventory_transport_time_limit(timer: &TimeKeeper) -> f64 {
        (timer.exact_remaining_sec() - 0.02)
            .max(0.0)
            .min(V150_TRANSPORT_TIME_CAP_SEC)
    }

    #[inline]
    fn inventory_depart_time_limit(timer: &TimeKeeper) -> f64 {
        (timer.exact_remaining_sec() - 0.02)
            .max(0.0)
            .min(V150_DEPART_TIME_CAP_SEC)
    }

    fn solve_inventory_v150(input: &Input, timer: &TimeKeeper) -> BeamState {
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;

        let profiles = [
            BuildProfile {
                stage_keep: 12,
                node_limit: 100_000,
                budgets: V150_BUDGETS,
            },
            BuildProfile {
                stage_keep: 20,
                node_limit: 160_000,
                budgets: V150_BUDGETS,
            },
        ];

        let initial = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
        let mut current_candidates = vec![initial.clone()];
        let mut best = initial;

        let trim_candidates = |cands: &mut Vec<BeamState>| {
            cands.sort_unstable_by_key(|bs| (bs.ops.len(), bs.state.len()));
            let mut uniq = Vec::with_capacity(V150_PHASE_CANDIDATE_LIMIT);
            let mut seen = FxHashSet::<StateSig>::default();
            for cand in cands.drain(..) {
                let sig = state_sig(&cand.state);
                if seen.insert(sig) {
                    uniq.push(cand);
                    if uniq.len() >= V150_PHASE_CANDIDATE_LIMIT {
                        break;
                    }
                }
            }
            *cands = uniq;
        };

        for seg_idx in 0..stock_cnt {
            if timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC {
                return best;
            }

            let target = inventory_segment_target(input, seg_idx, seg_len);
            let constraint = movement_constraint_with_min_col(2 * seg_idx);
            let Some(geom) = inventory_entry_geometry(input.n, seg_idx) else {
                return best;
            };
            let goal_cells = [geom.entry_hi, geom.entry_lo];

            let mut next_candidates = Vec::new();
            for base in &current_candidates {
                for profile in profiles {
                    let build_time = inventory_build_time_limit(timer, stock_cnt - seg_idx + 1);
                    if build_time <= 0.0 {
                        break;
                    }
                    let mut exacts = grow_to_target_prefix_beam_with_goal(
                        input,
                        base,
                        &target,
                        constraint,
                        &goal_cells,
                        profile.stage_keep,
                        build_time,
                        profile.budgets,
                        timer,
                    );
                    if exacts.is_empty() {
                        continue;
                    }
                    exacts.sort_unstable_by_key(|bs| {
                        let dist = goal_cells
                            .iter()
                            .map(|&cell| manhattan_cell(input.n, bs.state.head(), cell))
                            .min()
                            .unwrap_or(V150_INF);
                        (dist, bs.ops.len())
                    });

                    let goal = |state: &State| -> bool {
                        (state.head() == geom.entry_hi || state.head() == geom.entry_lo)
                            && inventory_place_from_entry(input, state, &target, target.len(), seg_idx)
                                .is_some()
                    };

                    let mut transported = None;
                    for exact in exacts.iter().take(profile.stage_keep) {
                        let transport_time = inventory_transport_time_limit(timer);
                        if transport_time <= 0.0 {
                            break;
                        }
                        transported = search_preserve_prefix_to_goal(
                            input,
                            exact,
                            &target,
                            target.len(),
                            constraint,
                            goal,
                            &goal_cells,
                            profile.node_limit,
                            V150_GOAL_EXTRA_LEN_LIMIT,
                            transport_time,
                            timer,
                        );
                        if transported.is_some() {
                            break;
                        }
                    }

                    let Some(transported) = transported else {
                        continue;
                    };
                    let Some(placed) =
                        place_inventory_segment(input, &transported, &target, seg_idx)
                    else {
                        continue;
                    };
                    if v150_debug_enabled() {
                        eprintln!("v150 placed seg={seg_idx} ops={}", placed.ops.len());
                    }
                    next_candidates.push(placed.clone());
                    if seg_idx + 1 < stock_cnt {
                        let depart_time = inventory_depart_time_limit(timer);
                        if depart_time > 0.0 {
                            if let Some(departed) = depart_after_placement(
                                input,
                                &placed,
                                seg_idx + 1,
                                depart_time,
                                timer,
                            ) {
                                if v150_debug_enabled() {
                                    eprintln!(
                                        "v150 departed seg={} ops={}",
                                        seg_idx + 1,
                                        departed.ops.len()
                                    );
                                }
                                next_candidates.push(departed);
                            } else if v150_debug_enabled() {
                                eprintln!("v150 depart_fail seg={}", seg_idx + 1);
                            }
                        }
                    }
                    if next_candidates.len() >= V150_PHASE_CANDIDATE_LIMIT {
                        break;
                    }
                }
                if next_candidates.len() >= V150_PHASE_CANDIDATE_LIMIT {
                    break;
                }
            }

            if next_candidates.is_empty() {
                if v150_debug_enabled() {
                    eprintln!("v150 segment_fail seg={seg_idx}");
                }
                return best;
            }
            trim_candidates(&mut next_candidates);
            best = next_candidates[0].clone();
            current_candidates = next_candidates;
        }

        if stock_cnt > 0 {
            let mut departed_candidates = Vec::new();
            for cand in &current_candidates {
                departed_candidates.push(cand.clone());
                let depart_time = inventory_depart_time_limit(timer);
                if depart_time > 0.0 {
                    if let Some(departed) =
                        depart_after_placement(input, cand, stock_cnt, depart_time, timer)
                    {
                        if v150_debug_enabled() {
                            eprintln!("v150 tail_depart_ok ops={}", departed.ops.len());
                        }
                        departed_candidates.push(departed);
                    } else if v150_debug_enabled() {
                        eprintln!("v150 tail_depart_fail");
                    }
                }
            }
            trim_candidates(&mut departed_candidates);
            current_candidates = departed_candidates;
            best = current_candidates[0].clone();
        }

        let tail_target = inventory_tail_target(input, stock_cnt, seg_len);
        let mut tail_ok = false;
        let mut tail_candidates = Vec::new();
        for base in &current_candidates {
            for profile in profiles {
                if timer.exact_remaining_sec() < V150_SAFE_LEFT_SEC {
                    return best;
                }

                let constraint = movement_constraint_with_min_col(2 * stock_cnt);
                let goal_build = if stock_cnt > 0 {
                    vec![Grid::cell(input.n, input.n - 2, 2 * stock_cnt - 1)]
                } else {
                    Vec::new()
                };
                let build_time = inventory_build_time_limit(timer, 1);
                if build_time <= 0.0 {
                    break;
                }
                let mut exacts = grow_to_target_prefix_beam_with_goal(
                    input,
                    base,
                    &tail_target,
                    constraint,
                    &goal_build,
                    profile.stage_keep,
                    build_time,
                    profile.budgets,
                    timer,
                );
                if exacts.is_empty() {
                    continue;
                }

                let chosen = if stock_cnt > 0 {
                    let goal = goal_build[0];
                    exacts.sort_unstable_by_key(|bs| {
                        (manhattan_cell(input.n, bs.state.head(), goal), bs.ops.len())
                    });
                    let harvest_constraint = movement_constraint_for_harvest_entry(stock_cnt, input.n);
                    let goal_predicate = |state: &State| state.head() == goal;

                    let mut chosen = None;
                    for exact in exacts.iter().take(profile.stage_keep) {
                        let transport_time = inventory_transport_time_limit(timer);
                        if transport_time <= 0.0 {
                            break;
                        }
                        chosen = search_preserve_prefix_to_goal(
                            input,
                            exact,
                            &tail_target,
                            tail_target.len(),
                            harvest_constraint,
                            goal_predicate,
                            &[goal],
                            profile.node_limit,
                            V150_GOAL_EXTRA_LEN_LIMIT,
                            transport_time,
                            timer,
                        );
                        if chosen.is_some() {
                            break;
                        }
                    }
                    chosen
                } else {
                    exacts.sort_unstable_by_key(|bs| bs.ops.len());
                    Some(exacts[0].clone())
                };

                let Some(chosen) = chosen else {
                    continue;
                };
                if v150_debug_enabled() {
                    eprintln!("v150 tail_ready ops={}", chosen.ops.len());
                }
                tail_candidates.push(chosen);
                tail_ok = true;
                if tail_candidates.len() >= V150_PHASE_CANDIDATE_LIMIT {
                    break;
                }
            }
            if tail_candidates.len() >= V150_PHASE_CANDIDATE_LIMIT {
                break;
            }
        }

        if !tail_ok {
            if v150_debug_enabled() {
                eprintln!("v150 tail_fail");
            }
            return best;
        }
        trim_candidates(&mut tail_candidates);
        current_candidates = tail_candidates;
        best = current_candidates[0].clone();

        if stock_cnt > 0 {
            let mut harvested_candidates = Vec::new();
            for cand in &current_candidates {
                if let Some(harvested) = harvest_inventory(input, cand, stock_cnt) {
                    if v150_debug_enabled() {
                        eprintln!("v150 harvest_ok ops={}", harvested.ops.len());
                    }
                    harvested_candidates.push(harvested);
                }
            }
            if harvested_candidates.is_empty() {
                if v150_debug_enabled() {
                    eprintln!("v150 harvest_fail");
                }
                return best;
            }
            trim_candidates(&mut harvested_candidates);
            current_candidates = harvested_candidates;
            best = current_candidates[0].clone();
        }

        for cand in current_candidates {
            if validate_full_match_v150(input, &cand.ops) {
                return cand;
            }
        }
        best
    }

    fn solve_v150(input: &Input) -> Ops {
        let timer = TimeKeeper::new(V150_TIME_LIMIT_SEC, 8);
        let mut best = solve_inventory_v150(input, &timer);
        if best.ops.len() > MAX_TURNS {
            best.ops.truncate(MAX_TURNS);
        }
        best.ops
    }

    pub fn run_v150() {
        let input = read_input();
        let ans = solve_v150(&input);
        let mut out = String::new();
        for dir in ans {
            out.push(DIR_CHARS[dir as usize]);
            out.push('\n');
        }
        print!("{out}");
    }
}

fn main() {
    base::run_v150();
}
