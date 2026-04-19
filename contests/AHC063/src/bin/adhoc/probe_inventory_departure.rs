// probe_inventory_departure.rs

mod solver {
    #![allow(dead_code)]
    include!("../v143_launch_split.rs");

    fn read_input_from_str(s: &str) -> Input {
        let mut it = s.split_whitespace();
        let n: usize = it.next().unwrap().parse().unwrap();
        let m: usize = it.next().unwrap().parse().unwrap();
        let _c: usize = it.next().unwrap().parse().unwrap();

        let mut d = [0_u8; MAX_LEN];
        for x in d.iter_mut().take(m) {
            *x = it.next().unwrap().parse::<u8>().unwrap();
        }

        let mut food = [0_u8; MAX_CELLS];
        for r in 0..n {
            for c in 0..n {
                food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
            }
        }

        Input {
            n,
            m,
            d,
            food,
            manhattan: ManhattanTable::new(n),
        }
    }

    fn reconstruct_after_placed_seg(input: &Input, placed_seg_idx: usize) -> BeamState {
        let timer = TimeKeeper::new(30.0, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let ctx = InventoryCtx {
            input,
            timer: &timer,
            seg_len,
            stock_cnt,
        };
        let mut bs = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
        for seg_idx in 0..=placed_seg_idx {
            bs = inventory_run_segment_phase(&ctx, &bs, seg_idx)
                .unwrap_or_else(|| panic!("phase failed before placed_seg_idx={placed_seg_idx}"));
        }
        bs
    }

    fn dump_state(label: &str, st: &State, goal_colors: &[u8], min_allowed_col: usize) {
        eprintln!(
            "{label}: head={:?} neck={:?} len={} residue={} lcp5={} lcp_goal={}",
            rc_of(st.head(), st.n),
            rc_of(st.neck(), st.n),
            st.len(),
            inventory_depart_bad_count(st, min_allowed_col),
            lcp_state(st, &goal_colors[..5]),
            lcp_state(st, goal_colors),
        );
    }

    fn dump_local_moves(label: &str, st: &State, goal_colors: &[u8], min_allowed_col: usize) {
        let mut rows = Vec::new();
        for &dir_u8 in legal_dirs(st).as_slice() {
            let dir = dir_u8 as usize;
            let Some(nh) = next_head_cell(st, dir) else {
                continue;
            };
            let (r, c) = rc_of(nh, st.n);
            let mut dropped = DroppedBuf::new();
            let (ns, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
            rows.push(format!(
                "{} -> ({},{}) food={} bite={} residue={} prefix5={} drop={:?}",
                DIR_CHARS[dir],
                r,
                c,
                ate,
                bite_idx.is_some(),
                inventory_depart_bad_count(&ns, min_allowed_col),
                inventory_launch_prefix_ok(&ns, goal_colors),
                dropped
                    .as_slice()
                    .iter()
                    .map(|ent| rc_of(ent.cell, st.n))
                    .collect::<Vec<_>>()
            ));
        }
        eprintln!("{label} legal: {}", rows.join(" | "));
    }

    fn fastlane_exact_build(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
    ) -> Option<BeamState> {
        let mut cur = base.clone();
        loop {
            let ell = lcp_state(&cur.state, goal_colors);
            if ell >= goal_colors.len() {
                return Some(cur);
            }
            cur = extend_fastlane_one(&cur, input, goal_colors, ell, &timer.start)?;
        }
    }

    fn depart_len5_noeat_search(
        base: &BeamState,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
    ) -> Option<BeamState> {
        if base.state.len() != 5 {
            return None;
        }
        if inventory_is_launch_ready(&base.state, min_allowed_col) {
            return Some(base.clone());
        }
        let mut nodes = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut q = std::collections::VecDeque::new();
        let mut seen = rustc_hash::FxHashSet::<State>::default();
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        q.push_back(0usize);
        seen.insert(base.state.clone());
        let mut expansions = 0usize;

        while let Some(idx) = q.pop_front() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;
            let cur = nodes[idx].state.clone();
            if inventory_is_launch_ready(&cur, min_allowed_col) {
                let mut ops = base.ops.clone();
                ops.extend_from_slice(&reconstruct_inventory_state_path(&nodes, idx));
                return Some(BeamState { state: cur, ops });
            }
            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let (ns, ate, bite_idx) = step(&cur, dir);
                if ate != 0 || bite_idx.is_some() || !inventory_launch_prefix_ok(&ns, goal_colors) {
                    continue;
                }
                if !seen.insert(ns.clone()) {
                    continue;
                }
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                q.push_back(child);
            }
        }
        None
    }

    fn normalize_departed_len5_search(
        base: &BeamState,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        if base.state.len() == 5 {
            return Some(base.clone());
        }
        if !inventory_is_launch_ready(&base.state, min_allowed_col) {
            return None;
        }
        let mut nodes = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = std::collections::BinaryHeap::new();
        pq.push(std::cmp::Reverse((
            (
                base.state.len().saturating_sub(5),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&base.state)),
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = rustc_hash::FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(std::cmp::Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;
            let cur = nodes[idx].state.clone();
            if cur.len() == 5 && inventory_is_launch_ready(&cur, min_allowed_col) {
                let mut ops = base.ops.clone();
                ops.extend_from_slice(&reconstruct_inventory_state_path(&nodes, idx));
                return Some(BeamState { state: cur, ops });
            }
            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }
            let mut cands = Vec::new();
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < 5
                    || !inventory_launch_prefix_ok(&ns, goal_colors)
                    || !inventory_is_launch_ready(&ns, min_allowed_col)
                {
                    continue;
                }
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + 1;
                if seen
                    .get(&ns)
                    .is_some_and(|&(best_d, best_e, best_b)| {
                        best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                    })
                {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        ns.len().saturating_sub(5),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(std::cmp::Reverse((key, uid, child)));
                uid += 1;
            }
        }
        None
    }

    fn best_ready_depart_search(
        base: &BeamState,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
        max_len: usize,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = std::collections::BinaryHeap::new();
        pq.push(std::cmp::Reverse((
            (
                inventory_depart_bad_count(&base.state, min_allowed_col),
                goal_colors.len().saturating_sub(lcp_state(&base.state, goal_colors)),
                usize::from(base.state.len() != 5),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&base.state)),
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = rustc_hash::FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0, 0));
        let mut best_ready: Option<(usize, (usize, usize, usize, usize, usize, usize, usize))> = None;
        let mut expansions = 0usize;

        while let Some(std::cmp::Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;
            let cur = nodes[idx].state.clone();
            let depth = depths[idx] as usize;
            if inventory_is_launch_ready(&cur, min_allowed_col) {
                let key = (
                    goal_colors.len().saturating_sub(lcp_state(&cur, goal_colors)),
                    usize::from(cur.len() != 5),
                    cur.len().saturating_sub(5),
                    bite_counts[idx] as usize,
                    eat_counts[idx] as usize,
                    4usize.saturating_sub(legal_dir_count(&cur)),
                    depth,
                );
                if best_ready.as_ref().map_or(true, |(_, best)| key < *best) {
                    best_ready = Some((idx, key));
                }
            }
            if depth >= depth_limit {
                continue;
            }

            let mut cands = Vec::with_capacity(4);
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < 5
                    || ns.len() > max_len
                    || !inventory_launch_prefix_ok(&ns, goal_colors)
                {
                    continue;
                }
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + 1;
                if seen
                    .get(&ns)
                    .is_some_and(|&(best_d, best_e, best_b)| {
                        best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                    })
                {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        inventory_depart_bad_count(&ns, min_allowed_col),
                        goal_colors.len().saturating_sub(lcp_state(&ns, goal_colors)),
                        usize::from(ns.len() != 5),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(std::cmp::Reverse((key, uid, child)));
                uid += 1;
            }
        }

        best_ready.map(|(idx, _)| BeamState {
            state: nodes[idx].state.clone(),
            ops: reconstruct_inventory_state_path(&nodes, idx),
        })
    }

    fn strong_exact_parked_search(
        base: &BeamState,
        target: &InventoryTarget<'_>,
        timer: &TimeKeeper,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
        extra_len: usize,
    ) -> Option<BeamState> {
        if base.state.len() != 5 {
            return None;
        }
        let keep_prefix_len = lcp_state(&base.state, target.colors).max(5);
        let max_len = target.protect_len + extra_len;
        let min_allowed_col = current_movement_constraint().min_allowed_col as usize;

        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = std::collections::BinaryHeap::new();
        pq.push(std::cmp::Reverse((
            (
                target.protect_len.saturating_sub(lcp_state(&base.state, target.colors)),
                inventory_depart_bad_count(&base.state, min_allowed_col),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&base.state)),
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = rustc_hash::FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(std::cmp::Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;
            let cur = nodes[idx].state.clone();
            if exact_prefix(&cur, target.colors, target.protect_len) {
                return Some(BeamState {
                    state: cur,
                    ops: reconstruct_inventory_state_path(&nodes, idx),
                });
            }
            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }

            let mut cands = Vec::with_capacity(4);
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < keep_prefix_len
                    || ns.len() > max_len
                    || !prefix_ok(&ns, target.colors, keep_prefix_len)
                {
                    continue;
                }
                if bite_idx.is_some() && !dropped_respects_active_constraint(cur.n, &dropped) {
                    continue;
                }
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + 1;
                if seen
                    .get(&ns)
                    .is_some_and(|&(best_d, best_e, best_b)| {
                        best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                    })
                {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        target.protect_len.saturating_sub(lcp_state(&ns, target.colors)),
                        inventory_depart_bad_count(&ns, min_allowed_col),
                        ns.len().saturating_sub(target.protect_len),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(std::cmp::Reverse((key, uid, child)));
                uid += 1;
            }
        }
        None
    }

    fn dropped_respects_depart_constraint(
        n: usize,
        min_allowed_col: usize,
        dropped: &DroppedBuf,
    ) -> bool {
        let tail_cells = inventory_depart_tail_cells(n, min_allowed_col);
        dropped.as_slice().iter().all(|ent| {
            let col = ent.cell as usize % n;
            col >= min_allowed_col
                || tail_cells
                    .as_ref()
                    .is_some_and(|&(a, b)| ent.cell == a || ent.cell == b)
        })
    }

    fn normalize_departed_len5_depart_safe(
        base: &BeamState,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        if base.state.len() == 5 {
            return Some(base.clone());
        }
        if !inventory_is_launch_ready(&base.state, min_allowed_col) {
            return None;
        }
        let mut nodes = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(8192) + 8);
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);
        let mut uid = 0usize;
        let mut pq = std::collections::BinaryHeap::new();
        pq.push(std::cmp::Reverse((
            (
                base.state.len().saturating_sub(5),
                0usize,
                0usize,
                4usize.saturating_sub(legal_dir_count(&base.state)),
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;
        let mut seen = rustc_hash::FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(base.state.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(std::cmp::Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;
            let cur = nodes[idx].state.clone();
            if cur.len() == 5 && inventory_is_launch_ready(&cur, min_allowed_col) {
                return Some(BeamState {
                    state: cur,
                    ops: reconstruct_inventory_state_path(&nodes, idx),
                });
            }
            let depth = depths[idx] as usize;
            if depth >= depth_limit {
                continue;
            }

            let mut cands = Vec::new();
            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, ate, bite_idx) = step_with_dropped(&cur, dir, &mut dropped);
                if ns.len() < 5
                    || !inventory_launch_prefix_ok(&ns, goal_colors)
                    || !inventory_is_launch_ready(&ns, min_allowed_col)
                {
                    continue;
                }
                if bite_idx.is_some()
                    && !dropped_respects_depart_constraint(cur.n, min_allowed_col, &dropped)
                {
                    continue;
                }
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bite_idx.is_some()));
                if next_eat > eat_limit || next_bite > bite_limit {
                    continue;
                }
                let nd = depth + 1;
                if seen
                    .get(&ns)
                    .is_some_and(|&(best_d, best_e, best_b)| {
                        best_d <= nd as u8 && best_e <= next_eat && best_b <= next_bite
                    })
                {
                    continue;
                }
                seen.insert(ns.clone(), (nd as u8, next_eat, next_bite));
                cands.push((
                    (
                        ns.len().saturating_sub(5),
                        next_bite as usize,
                        next_eat as usize,
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        nd,
                    ),
                    ns,
                    dir_u8,
                    next_eat,
                    next_bite,
                ));
            }
            cands.sort_unstable_by_key(|(key, _, _, _, _)| *key);
            for (key, ns, dir_u8, next_eat, next_bite) in cands {
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                eat_counts.push(next_eat);
                bite_counts.push(next_bite);
                pq.push(std::cmp::Reverse((key, uid, child)));
                uid += 1;
            }
        }
        None
    }

    fn depart_emptyk_static(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        min_allowed_col: usize,
        empty_goal: u8,
    ) -> Option<BeamState> {
        let st = &base.state;
        let n = st.n;
        let mut blocked = [false; MAX_CELLS];
        for idx in 0..st.len() {
            blocked[st.pos[idx] as usize] = true;
        }
        blocked[st.head() as usize] = false;

        let mut q = std::collections::VecDeque::new();
        let mut cells = Vec::<(Cell, [Cell; 12], u8, u8, Option<usize>, u8)>::new();
        let mut seen = rustc_hash::FxHashSet::<(Cell, [Cell; 12], u8, u8)>::default();
        cells.push((st.head(), [0; 12], 0, 0, None, 0));
        q.push_back(0usize);
        seen.insert((st.head(), [0; 12], 0, 0));

        while let Some(idx) = q.pop_front() {
            let (cur, used, used_len, depth, _parent, _mv) = cells[idx];
            if used_len >= empty_goal {
                let mut rev = Vec::new();
                let mut cur_idx = idx;
                while let Some(parent) = cells[cur_idx].4 {
                    rev.push(cells[cur_idx].5);
                    cur_idx = parent;
                }
                rev.reverse();
                let mut sim = base.state.clone();
                let mut ops = base.ops.clone();
                let mut ok = true;
                for &dir_u8 in &rev {
                    let (ns, ate, bite_idx) = step(&sim, dir_u8 as usize);
                    if ate != 0 || bite_idx.is_some() || !inventory_launch_prefix_ok(&ns, goal_colors) {
                        ok = false;
                        break;
                    }
                    sim = ns;
                    ops.push(dir_u8);
                }
                if ok && inventory_is_launch_ready(&sim, min_allowed_col) {
                    return Some(BeamState { state: sim, ops });
                }
                continue;
            }
            if depth as usize >= 32 {
                continue;
            }

            let mut nexts = Vec::new();
            for &nxt in neighbors(n, cur).as_slice() {
                if !is_cell_allowed(n, nxt)
                    || nxt as usize % n < min_allowed_col
                    || blocked[nxt as usize]
                    || st.food[nxt as usize] != 0
                {
                    continue;
                }
                nexts.push(nxt);
            }
            nexts.sort_unstable_by_key(|&nxt| {
                let (r, c) = rc_of(nxt, n);
                (usize::MAX - c, r)
            });
            for nxt in nexts {
                let mut next_used = used;
                let mut next_used_len = used_len;
                if !used[..used_len as usize].contains(&nxt) {
                    next_used[next_used_len as usize] = nxt;
                    next_used_len += 1;
                }
                let dir = dir_between_cells(n, cur, nxt)? as u8;
                let node = (nxt, next_used, next_used_len, depth + 1, Some(idx), dir);
                if !seen.insert((node.0, node.1, node.2, node.3)) {
                    continue;
                }
                let child = cells.len();
                cells.push(node);
                q.push_back(child);
            }
        }
        let _ = input;
        None
    }

    fn eat_seed_then_emptyk(
        base: &BeamState,
        input: &Input,
        goal_colors: &[u8],
        timer: &TimeKeeper,
        min_allowed_col: usize,
        depth_limit: usize,
        node_limit: usize,
        empty_goal: u8,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(4096) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(4096) + 8);
        let mut q = std::collections::VecDeque::new();
        let mut seen = rustc_hash::FxHashSet::<State>::default();
        nodes.push(InventoryStateNode {
            state: base.state.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        q.push_back(0usize);
        seen.insert(base.state.clone());

        while let Some(idx) = q.pop_front() {
            let cur = nodes[idx].state.clone();
            let cur_bs = BeamState {
                state: cur.clone(),
                ops: reconstruct_inventory_state_path(&nodes, idx),
            };
            if let Some(suffix) = depart_emptyk_static(
                &cur_bs,
                input,
                goal_colors,
                min_allowed_col,
                empty_goal,
            ) {
                let mut ops = base.ops.clone();
                ops.extend_from_slice(&reconstruct_inventory_state_path(&nodes, idx));
                let suffix_only = &suffix.ops[cur_bs.ops.len()..];
                ops.extend_from_slice(suffix_only);
                return Some(BeamState {
                    state: suffix.state,
                    ops,
                });
            }

            let depth = depths[idx] as usize;
            if depth >= depth_limit || seen.len() >= node_limit {
                continue;
            }

            for &dir_u8 in legal_dirs(&cur).as_slice() {
                let dir = dir_u8 as usize;
                let (ns, ate, bite_idx) = step(&cur, dir);
                if ate == 0 || bite_idx.is_some() || !inventory_launch_prefix_ok(&ns, goal_colors) {
                    continue;
                }
                if !seen.insert(ns.clone()) {
                    continue;
                }
                let child = nodes.len();
                nodes.push(InventoryStateNode {
                    state: ns,
                    parent: Some(idx),
                    mv: dir_u8,
                });
                depths.push((depth + 1) as u8);
                q.push_back(child);
            }
        }

        let _ = timer;
        None
    }

    pub fn probe(path: &str, seg_idx: usize) {
        assert!(seg_idx > 0, "seg_idx must be >= 1");
        let input = read_input_from_str(&std::fs::read_to_string(path).unwrap());
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let ctx = InventoryCtx {
            input: &input,
            timer: &timer,
            seg_len,
            stock_cnt,
        };
        let base = reconstruct_after_placed_seg(&input, seg_idx - 1);
        let goal_colors = inventory_segment_target(&input, seg_idx, seg_len);
        let target = InventoryTarget {
            colors: &goal_colors,
            protect_len: goal_colors.len(),
        };
        let phases_left = stock_cnt - seg_idx + 1;
        let min_allowed_col = 2 * seg_idx;
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));

        eprintln!(
            "case={} seg_idx={} n={} seg_len={} stock_cnt={}",
            path, seg_idx, input.n, seg_len, stock_cnt
        );
        let mut food_cnt = [0usize; 10];
        for cell in 0..input.n * input.n {
            let col = cell % input.n;
            if col >= min_allowed_col {
                food_cnt[input.food[cell] as usize] += 1;
            }
        }
        let mut need_cnt = [0usize; 10];
        for &c in &goal_colors[5..] {
            need_cnt[c as usize] += 1;
        }
        eprintln!("allowed_food_cnt={:?}", &food_cnt[..6]);
        eprintln!("need_seg_cnt={:?}", &need_cnt[..6]);
        dump_state("base", &base.state, &goal_colors, min_allowed_col);
        dump_local_moves("base", &base.state, &goal_colors, min_allowed_col);
        eprintln!(
            "launch_ready={} tail_cells={:?}",
            inventory_is_launch_ready(&base.state, min_allowed_col),
            inventory_depart_tail_cells(input.n, min_allowed_col)
                .map(|(a, b)| (rc_of(a, input.n), rc_of(b, input.n)))
        );
        if legal_dirs(&base.state).as_slice().len() == 1 {
            let dir = legal_dirs(&base.state).as_slice()[0] as usize;
            let (ns, ate, bite_idx) = step(&base.state, dir);
            eprintln!(
                "forced1 {} -> head={:?} len={} ate={} bite={} residue={}",
                DIR_CHARS[dir],
                rc_of(ns.head(), input.n),
                ns.len(),
                ate,
                bite_idx.is_some(),
                inventory_depart_bad_count(&ns, min_allowed_col),
            );
            dump_local_moves("forced1_state", &ns, &goal_colors, min_allowed_col);
        }

        let direct = inventory_try_direct_build_from_current(&ctx, &base, &target, phases_left);
        eprintln!("direct_build={}", direct.is_some());
        if let Some(bs) = direct.as_ref() {
            dump_state("direct_built", &bs.state, &goal_colors, min_allowed_col);
        }
        let base_fastlane = fastlane_exact_build(&base, &input, &goal_colors, &timer);
        eprintln!("base_fastlane_build={}", base_fastlane.is_some());
        let base_long = grow_to_target_prefix(&input, base.state.clone(), &goal_colors, &timer, 2.40);
        eprintln!(
            "base_long_build={}",
            base_long
                .as_ref()
                .is_some_and(|inc| exact_prefix(&inc.state, &goal_colors, target.protect_len))
        );

        let len5_depart =
            depart_len5_noeat_search(&base, &goal_colors, &timer, min_allowed_col, 28, 120_000);
        eprintln!("len5_depart={}", len5_depart.is_some());
        if let Some(bs) = len5_depart.as_ref() {
            dump_state("len5_depart_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!(
                "len5_depart_build={}",
                matches!(built, InventoryBuildOutcome::Built(_))
            );
        }
        let len5_depart_strong =
            depart_len5_noeat_search(&base, &goal_colors, &timer, min_allowed_col, 48, 1_000_000);
        eprintln!("len5_depart_strong={}", len5_depart_strong.is_some());
        if let Some(bs) = len5_depart_strong.as_ref() {
            dump_state("len5_depart_strong_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!(
                "len5_depart_strong_build={}",
                matches!(built, InventoryBuildOutcome::Built(_))
            );
        }

        let len5_any = inventory_try_depart_len5_any(&base, &goal_colors, &timer, min_allowed_col);
        eprintln!("len5_any={}", len5_any.is_some());
        if let Some(bs) = len5_any.as_ref() {
            dump_state("len5_any_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("len5_any_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
            let legacy = inventory_build_target_exact_legacy_parked(&ctx, bs, &target, phases_left);
            eprintln!("len5_any_legacy_build={}", matches!(legacy, InventoryBuildOutcome::Built(_)));
        }

        let empty4 = depart_emptyk_static(&base, &input, &goal_colors, min_allowed_col, 4);
        eprintln!("empty4={}", empty4.is_some());
        if let Some(bs) = empty4.as_ref() {
            dump_state("empty4_state", &bs.state, &goal_colors, min_allowed_col);
            dump_local_moves("empty4_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("empty4_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
        }

        let depart = inventory_depart_from_parked(&base, &input, &goal_colors, &timer, min_allowed_col);
        eprintln!("depart={}", depart.is_some());
        if let Some(bs) = depart.as_ref() {
            dump_state("depart_state", &bs.state, &goal_colors, min_allowed_col);
            dump_local_moves("depart_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("depart_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
            let normalized =
                inventory_normalize_departed_len5(bs, &goal_colors, &timer, min_allowed_col);
            eprintln!("depart_normalize={}", normalized.is_some());
            if let Some(norm) = normalized.as_ref() {
                dump_state("depart_normalized", &norm.state, &goal_colors, min_allowed_col);
                let built = inventory_build_target_exact(&ctx, norm, &target, phases_left);
                eprintln!(
                    "depart_normalized_build={}",
                    matches!(built, InventoryBuildOutcome::Built(_))
                );
            }
            let normalized_strong = normalize_departed_len5_search(
                bs,
                &goal_colors,
                &timer,
                min_allowed_col,
                48,
                1_000_000,
                24,
                16,
            );
            eprintln!("depart_normalize_strong={}", normalized_strong.is_some());
            if let Some(norm) = normalized_strong.as_ref() {
                dump_state("depart_normalized_strong", &norm.state, &goal_colors, min_allowed_col);
                let built = inventory_build_target_exact(&ctx, norm, &target, phases_left);
                eprintln!(
                    "depart_normalized_strong_build={}",
                    matches!(built, InventoryBuildOutcome::Built(_))
                );
            }
        }

        let strong_depart = inventory_depart_from_parked_search(
            &base,
            &input,
            &goal_colors,
            &timer,
            min_allowed_col,
            28,
            160_000,
            8,
            4,
            18,
        );
        eprintln!("strong_depart={}", strong_depart.is_some());
        if let Some(bs) = strong_depart.as_ref() {
            dump_state("strong_depart_state", &bs.state, &goal_colors, min_allowed_col);
            dump_local_moves("strong_depart_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!(
                "strong_depart_build={}",
                matches!(built, InventoryBuildOutcome::Built(_))
            );
        }

        let transportable = inventory_try_transportable_build_from_parked(
            &ctx,
            &base,
            &target,
            phases_left,
            seg_idx,
        );
        eprintln!("transportable_build={}", transportable.is_some());
        if let Some(bs) = transportable.as_ref() {
            dump_state("transportable", &bs.state, &goal_colors, min_allowed_col);
        }

        let exact_parked = inventory_try_exact_build_from_parked(&ctx, &base, &target);
        eprintln!("exact_parked_build={}", exact_parked.is_some());
        if let Some(bs) = exact_parked.as_ref() {
            dump_state("exact_parked_state", &bs.state, &goal_colors, min_allowed_col);
        }

        let mega_rescue = inventory_try_launch_rescue_build_with_validator_cfg(
            &base,
            &input,
            &goal_colors,
            &timer,
            2.40,
            true,
            InventoryLaunchRescueCfg {
                depth_limit: 24,
                node_limit: 1_000_000,
                parked_probe_limit: 120_000,
                first_per_depth: 120,
                best_per_depth: 120,
            },
            |_| true,
        );
        eprintln!("mega_rescue_build={}", mega_rescue.is_some());
        if let Some(bs) = mega_rescue.as_ref() {
            dump_state("mega_rescue_state", &bs.state, &goal_colors, min_allowed_col);
        }

        let prepared =
            inventory_prepare_build_start(&ctx, &base, &goal_colors, phases_left, min_allowed_col);
        eprintln!("prepare_build_start={}", prepared.is_some());
        if let Some(bs) = prepared.as_ref() {
            dump_state("prepared_state", &bs.state, &goal_colors, min_allowed_col);
            dump_local_moves("prepared_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("prepared_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
            let long = grow_to_target_prefix(&input, bs.state.clone(), &goal_colors, &timer, 2.40);
            eprintln!(
                "prepared_long_build={}",
                long.as_ref()
                    .is_some_and(|inc| exact_prefix(&inc.state, &goal_colors, target.protect_len))
            );
            let fastlane = fastlane_exact_build(bs, &input, &goal_colors, &timer);
            eprintln!("prepared_fastlane_build={}", fastlane.is_some());
        }

        let best_ready = best_ready_depart_search(
            &base,
            &goal_colors,
            &timer,
            min_allowed_col,
            40,
            1_000_000,
            16,
            8,
            target.protect_len + 20,
        );
        eprintln!("best_ready_depart={}", best_ready.is_some());
        if let Some(bs) = best_ready.as_ref() {
            dump_state("best_ready_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("best_ready_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
            let long = grow_to_target_prefix(&input, bs.state.clone(), &goal_colors, &timer, 2.40);
            eprintln!(
                "best_ready_long_build={}",
                long.as_ref()
                    .is_some_and(|inc| exact_prefix(&inc.state, &goal_colors, target.protect_len))
            );
        }

        let strong_exact = strong_exact_parked_search(
            &base,
            &target,
            &timer,
            80,
            1_500_000,
            32,
            16,
            28,
        );
        eprintln!("strong_exact_parked={}", strong_exact.is_some());
        if let Some(bs) = strong_exact.as_ref() {
            dump_state("strong_exact_state", &bs.state, &goal_colors, min_allowed_col);
        }

        let eat_seed = eat_seed_then_emptyk(
            &base,
            &input,
            &goal_colors,
            &timer,
            min_allowed_col,
            8,
            20_000,
            4,
        );
        eprintln!("eat_seed_empty4={}", eat_seed.is_some());
        if let Some(bs) = eat_seed.as_ref() {
            dump_state("eat_seed_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("eat_seed_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
            let legacy = inventory_build_target_exact_legacy_parked(&ctx, bs, &target, phases_left);
            eprintln!(
                "eat_seed_legacy_build={}",
                matches!(legacy, InventoryBuildOutcome::Built(_))
            );
            let long = grow_to_target_prefix(&input, bs.state.clone(), &goal_colors, &timer, 2.40);
            eprintln!(
                "eat_seed_long_build={}",
                long.as_ref()
                    .is_some_and(|inc| exact_prefix(&inc.state, &goal_colors, target.protect_len))
            );
            let norm = inventory_normalize_departed_len5(bs, &goal_colors, &timer, min_allowed_col);
            eprintln!("eat_seed_normalize={}", norm.is_some());
            if let Some(norm) = norm.as_ref() {
                dump_state("eat_seed_normalized", &norm.state, &goal_colors, min_allowed_col);
                let built = inventory_build_target_exact(&ctx, norm, &target, phases_left);
                eprintln!(
                    "eat_seed_normalized_build={}",
                    matches!(built, InventoryBuildOutcome::Built(_))
                );
            }
            let norm_depart = normalize_departed_len5_depart_safe(
                bs,
                &goal_colors,
                &timer,
                min_allowed_col,
                32,
                400_000,
                16,
                16,
            );
            eprintln!("eat_seed_normalize_depart_safe={}", norm_depart.is_some());
            if let Some(norm) = norm_depart.as_ref() {
                dump_state("eat_seed_depart_safe_norm", &norm.state, &goal_colors, min_allowed_col);
                let built = inventory_build_target_exact(&ctx, norm, &target, phases_left);
                eprintln!(
                    "eat_seed_depart_safe_norm_build={}",
                    matches!(built, InventoryBuildOutcome::Built(_))
                );
            }
        }
        let eat_seed6 = eat_seed_then_emptyk(
            &base,
            &input,
            &goal_colors,
            &timer,
            min_allowed_col,
            8,
            20_000,
            6,
        );
        eprintln!("eat_seed_empty6={}", eat_seed6.is_some());
        if let Some(bs) = eat_seed6.as_ref() {
            dump_state("eat_seed6_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("eat_seed6_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
        }
        let eat_seed8 = eat_seed_then_emptyk(
            &base,
            &input,
            &goal_colors,
            &timer,
            min_allowed_col,
            10,
            40_000,
            8,
        );
        eprintln!("eat_seed_empty8={}", eat_seed8.is_some());
        if let Some(bs) = eat_seed8.as_ref() {
            dump_state("eat_seed8_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("eat_seed8_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
        }
        let eat_seed12 = eat_seed_then_emptyk(
            &base,
            &input,
            &goal_colors,
            &timer,
            min_allowed_col,
            14,
            80_000,
            12,
        );
        eprintln!("eat_seed_empty12={}", eat_seed12.is_some());
        if let Some(bs) = eat_seed12.as_ref() {
            dump_state("eat_seed12_state", &bs.state, &goal_colors, min_allowed_col);
            let built = inventory_build_target_exact(&ctx, bs, &target, phases_left);
            eprintln!("eat_seed12_build={}", matches!(built, InventoryBuildOutcome::Built(_)));
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: probe_inventory_departure <input> <seg_idx>");
    let seg_idx: usize = args.next().unwrap().parse().unwrap();
    solver::probe(&path, seg_idx);
}
