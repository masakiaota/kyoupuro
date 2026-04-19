// probe_inventory_harvest.rs

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

    fn direct_harvestable_state_search(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        stock_cnt: usize,
        timer: &TimeKeeper,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
        accept_now: fn(&State, &Input, usize) -> bool,
    ) -> Option<BeamState> {
        let mut nodes = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(16_384) + 8);
        nodes.push(InventoryStateNode {
            state: st.clone(),
            parent: None,
            mv: 0,
        });
        depths.push(0u8);
        eat_counts.push(0u8);
        bite_counts.push(0u8);

        let mut uid = 0usize;
        let mut pq = BinaryHeap::new();
        pq.push(Reverse((
            (
                usize::from(!accept_now(st, input, stock_cnt)),
                0usize,
                4usize.saturating_sub(legal_dir_count(st)),
                0usize,
                0usize,
                0usize,
            ),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit
                || time_over(&timer.start)
                || timer.exact_remaining_sec() < INVENTORY_RESCUE_MIN_LEFT_SEC
            {
                break;
            }
            expansions += 1;

            let cur = nodes[idx].state.clone();
            if accept_now(&cur, input, stock_cnt) {
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
                let Some((ns, ate, bit)) =
                    inventory_transport_step_state(&cur, dir_u8 as usize, protect_colors, protect_len)
                else {
                    continue;
                };
                let next_eat = eat_counts[idx].saturating_add(u8::from(ate != 0));
                let next_bite = bite_counts[idx].saturating_add(u8::from(bit));
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
                        usize::from(!accept_now(&ns, input, stock_cnt)),
                        ns.len().saturating_sub(protect_len),
                        4usize.saturating_sub(legal_dir_count(&ns)),
                        next_bite as usize,
                        next_eat as usize,
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
                pq.push(Reverse((key, uid, child)));
                uid += 1;
            }
        }

        None
    }

    fn harvest_accept_direct(st: &State, input: &Input, stock_cnt: usize) -> bool {
        inventory_harvest_from_state(st, input, stock_cnt).is_some()
    }

    fn harvest_accept_with_entry(st: &State, input: &Input, stock_cnt: usize) -> bool {
        if inventory_harvest_from_state(st, input, stock_cnt).is_some() {
            return true;
        }
        if stock_cnt == 0 {
            return false;
        }
        let c = 2 * stock_cnt - 1;
        let goal = cell_of(input.n - 3, c, input.n);
        let entry_hi = cell_of(input.n - 2, c, input.n);
        let entry_lo = cell_of(input.n - 1, c, input.n);

        if st.head() == entry_hi {
            let (s1, _, b1) = step(st, 0);
            return b1.is_none()
                && s1.head() == goal
                && inventory_harvest_from_state(&s1, input, stock_cnt).is_some();
        }
        if st.head() == entry_lo {
            let (s1, _, b1) = step(st, 0);
            if b1.is_some() || s1.head() != entry_hi {
                return false;
            }
            let (s2, _, b2) = step(&s1, 0);
            return b2.is_none()
                && s2.head() == goal
                && inventory_harvest_from_state(&s2, input, stock_cnt).is_some();
        }
        false
    }

    fn apply_dirs_no_bite(st: &State, dirs: &[u8]) -> Option<State> {
        let mut cur = st.clone();
        for &dir in dirs {
            let (ns, _, bite_idx) = step(&cur, dir as usize);
            if bite_idx.is_some() {
                return None;
            }
            cur = ns;
        }
        Some(cur)
    }

    fn harvest_accept_with_right_entry(st: &State, input: &Input, stock_cnt: usize) -> bool {
        if inventory_harvest_from_state(st, input, stock_cnt).is_some() {
            return true;
        }
        if stock_cnt == 0 {
            return false;
        }
        let c = 2 * stock_cnt - 1;
        let right = c + 1;
        if right >= input.n {
            return false;
        }
        let starts = [
            (cell_of(input.n - 3, right, input.n), vec![2_u8]),
            (cell_of(input.n - 2, right, input.n), vec![0_u8, 2_u8]),
            (cell_of(input.n - 2, right, input.n), vec![2_u8, 0_u8]),
            (cell_of(input.n - 1, right, input.n), vec![0_u8, 0_u8, 2_u8]),
            (cell_of(input.n - 1, right, input.n), vec![0_u8, 2_u8, 0_u8]),
            (cell_of(input.n - 1, right, input.n), vec![2_u8, 0_u8, 0_u8]),
        ];
        for (start, ops) in starts {
            if st.head() != start {
                continue;
            }
            let Some(ns) = apply_dirs_no_bite(st, &ops) else {
                continue;
            };
            if inventory_harvest_from_state(&ns, input, stock_cnt).is_some() {
                return true;
            }
        }
        false
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

    pub fn probe(path: &str) {
        let input = read_input_from_str(&std::fs::read_to_string(path).unwrap());
        let timer = TimeKeeper::new(30.0, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let ctx = InventoryCtx {
            input: &input,
            timer: &timer,
            seg_len,
            stock_cnt,
        };

        let mut bs = BeamState {
            state: State::initial(&input),
            ops: Ops::new(),
        };
        for seg_idx in 0..stock_cnt {
            bs = inventory_run_segment_phase(&ctx, &bs, seg_idx)
                .unwrap_or_else(|| panic!("phase failed seg={seg_idx}"));
        }
        let tail_base = bs.clone();
        let (bs, tail_colors) = inventory_build_tail_exact(&ctx, &bs).expect("tail build failed");

        eprintln!(
            "tail_ready head={:?} neck={:?} len={} stock_cnt={} goal={:?}",
            rc_of(bs.state.head(), input.n),
            rc_of(bs.state.neck(), input.n),
            bs.state.len(),
            stock_cnt,
            rc_of(cell_of(input.n - 3, 2 * stock_cnt - 1, input.n), input.n),
        );

        let _constraint = push_movement_constraint(movement_constraint_for_harvest_entry(
            stock_cnt,
            input.n,
        ));
        let goal = cell_of(input.n - 3, 2 * stock_cnt - 1, input.n);

        let oracle = inventory_transport_to_goal_oracle_plan(
            &bs.state,
            goal,
            &tail_colors,
            tail_colors.len(),
        );
        eprintln!("oracle={}", oracle.is_some());

        let mild = inventory_transport_to_goal_plan_with_limits(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            goal,
            &timer.start,
            INVENTORY_TRANSPORT_DEPTH_LIMIT,
            INVENTORY_TRANSPORT_NODE_LIMIT,
            INVENTORY_TRANSPORT_EAT_LIMIT,
            INVENTORY_TRANSPORT_BITE_LIMIT,
        );
        eprintln!("mild={}", mild.is_some());
        if let Some(plan) = mild.as_ref() {
            eprintln!(
                "mild_steps={} final={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let strong = inventory_transport_to_goal_plan_with_limits(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            goal,
            &timer.start,
            160,
            500_000,
            20,
            10,
        );
        eprintln!("strong={}", strong.is_some());
        if let Some(plan) = strong.as_ref() {
            eprintln!(
                "strong_steps={} final={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let route_safe = inventory_transport_to_harvest_entry_plan_with_limits(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer.start,
            200,
            600_000,
            24,
            12,
        );
        eprintln!("route_safe={}", route_safe.is_some());
        if let Some(plan) = route_safe.as_ref() {
            eprintln!(
                "route_safe_steps={} final={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let mega_route_safe = inventory_transport_to_harvest_entry_plan_with_limits(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer.start,
            320,
            2_000_000,
            40,
            16,
        );
        eprintln!("mega_route_safe={}", mega_route_safe.is_some());
        if let Some(plan) = mega_route_safe.as_ref() {
            eprintln!(
                "mega_route_safe_steps={} final={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let any_harvestable = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            320,
            2_000_000,
            40,
            16,
            harvest_accept_direct,
        );
        eprintln!("any_harvestable={}", any_harvestable.is_some());
        if let Some(plan) = any_harvestable.as_ref() {
            eprintln!(
                "any_harvestable_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let any_harvestable_entry = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            320,
            2_000_000,
            40,
            16,
            harvest_accept_with_entry,
        );
        eprintln!("any_harvestable_entry={}", any_harvestable_entry.is_some());
        if let Some(plan) = any_harvestable_entry.as_ref() {
            eprintln!(
                "any_harvestable_entry_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let any_harvestable_right = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            320,
            2_000_000,
            40,
            16,
            harvest_accept_with_right_entry,
        );
        eprintln!("any_harvestable_right={}", any_harvestable_right.is_some());
        if let Some(plan) = any_harvestable_right.as_ref() {
            eprintln!(
                "any_harvestable_right_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        drop(_constraint);
        let _two_col_constraint =
            push_movement_constraint(movement_constraint_with_min_col(2 * stock_cnt - 2));
        let any_harvestable_two_col = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            320,
            2_000_000,
            40,
            16,
            harvest_accept_with_right_entry,
        );
        eprintln!("any_harvestable_two_col={}", any_harvestable_two_col.is_some());
        if let Some(plan) = any_harvestable_two_col.as_ref() {
            eprintln!(
                "any_harvestable_two_col_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        for extra in 3..=4 {
            if 2 * stock_cnt < extra {
                continue;
            }
            let _more_constraint =
                push_movement_constraint(movement_constraint_with_min_col(2 * stock_cnt - extra));
            let more = direct_harvestable_state_search(
                &bs.state,
                &input,
                &tail_colors,
                tail_colors.len(),
                stock_cnt,
                &timer,
                320,
                2_000_000,
                40,
                16,
                harvest_accept_with_right_entry,
            );
            eprintln!("any_harvestable_{}col={}", extra + 1, more.is_some());
            drop(_more_constraint);
        }

        let alt = inventory_try_harvestable_tail_build(&ctx, &tail_base, &tail_colors);
        eprintln!("harvestable_tail_build={}", alt.is_some());
        if let Some(alt) = alt.as_ref() {
            eprintln!(
                "alt_tail head={:?} len={}",
                rc_of(alt.state.head(), input.n),
                alt.state.len()
            );
        }

        let target = InventoryTarget {
            colors: &tail_colors,
            protect_len: tail_colors.len(),
        };
        let long_direct = grow_to_target_prefix(
            &input,
            tail_base.state.clone(),
            &tail_colors,
            &timer,
            2.40,
        )
        .and_then(|inc| append_incremental_beam(&tail_base, inc))
        .filter(|candidate| exact_prefix(&candidate.state, target.colors, target.protect_len));
        eprintln!("long_direct_exact={}", long_direct.is_some());
        if let Some(candidate) = long_direct.as_ref() {
            eprintln!(
                "long_direct_harvestable={}",
                inventory_tail_build_is_harvestable(&ctx, &tail_colors, candidate)
            );
        }
        let fastlane = fastlane_exact_build(&tail_base, &input, &tail_colors, &timer);
        eprintln!("fastlane_exact={}", fastlane.is_some());
        if let Some(candidate) = fastlane.as_ref() {
            eprintln!(
                "fastlane_harvestable={}",
                inventory_tail_build_is_harvestable(&ctx, &tail_colors, candidate)
            );
        }
        let strong_alt = inventory_try_launch_rescue_build_with_validator(
            &tail_base,
            &input,
            &tail_colors,
            &timer,
            2.00,
            true,
            |candidate| inventory_tail_build_is_harvestable(&ctx, &tail_colors, candidate),
        );
        eprintln!("strong_harvestable_tail_build={}", strong_alt.is_some());
        if let Some(alt) = strong_alt.as_ref() {
            eprintln!(
                "strong_alt_tail head={:?} len={} exact={}",
                rc_of(alt.state.head(), input.n),
                alt.state.len(),
                exact_prefix(&alt.state, target.colors, target.protect_len),
            );
        }

        let mega_alt = inventory_try_launch_rescue_build_with_validator_cfg(
            &tail_base,
            &input,
            &tail_colors,
            &timer,
            2.40,
            true,
            InventoryLaunchRescueCfg {
                depth_limit: 24,
                node_limit: 1_000_000,
                parked_probe_limit: 80_000,
                first_per_depth: 120,
                best_per_depth: 120,
            },
            |candidate| inventory_tail_build_is_harvestable(&ctx, &tail_colors, candidate),
        );
        eprintln!("mega_harvestable_tail_build={}", mega_alt.is_some());
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_inventory_harvest <input>");
    solver::probe(&path);
}
