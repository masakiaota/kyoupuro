// probe_inventory_harvest_v144.rs

mod solver {
    #![allow(dead_code)]
    include!("../v144_dead_code_cleanup.rs");

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

    fn inventory_harvest_from_state(st: &State, input: &Input, stock_cnt: usize) -> Option<State> {
        let bs = BeamState {
            state: st.clone(),
            ops: Ops::new(),
        };
        harvest_inventory(&bs, input, stock_cnt).map(|x| x.state)
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

    fn harvest_accept_goal(st: &State, input: &Input, stock_cnt: usize) -> bool {
        if stock_cnt == 0 {
            return inventory_harvest_from_state(st, input, stock_cnt).is_some();
        }
        let goal = cell_of(input.n - 3, 2 * stock_cnt - 1, input.n);
        st.head() == goal && inventory_harvest_from_state(st, input, stock_cnt).is_some()
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

        let goal = if stock_cnt > 0 {
            Some(cell_of(input.n - 3, 2 * stock_cnt - 1, input.n))
        } else {
            None
        };

        eprintln!(
            "tail_ready head={:?} neck={:?} len={} stock_cnt={} goal={:?}",
            rc_of(bs.state.head(), input.n),
            rc_of(bs.state.neck(), input.n),
            bs.state.len(),
            stock_cnt,
            goal.map(|g| rc_of(g, input.n))
        );
        eprintln!(
            "tail_base head={:?} len={}",
            rc_of(tail_base.state.head(), input.n),
            tail_base.state.len()
        );

        if stock_cnt == 0 {
            eprintln!("no stock");
            return;
        }

        let _constraint = push_movement_constraint(movement_constraint_for_harvest_entry(
            stock_cnt,
            input.n,
        ));
        let goal = goal.unwrap();

        let direct = transport_to_cell_preserving_prefix(
            &bs,
            &input,
            &tail_colors,
            tail_colors.len(),
            goal,
            &timer.start,
        );
        eprintln!("transport_to_cell_preserving_prefix={}", direct.is_some());
        if let Some(plan) = direct.as_ref() {
            eprintln!(
                "direct_steps={} final={:?}",
                plan.ops.len() - bs.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let oracle = inventory_transport_to_goal_oracle_plan(&bs.state, goal, &tail_colors, tail_colors.len());
        eprintln!("oracle={}", oracle.is_some());
        if let Some(plan) = oracle.as_ref() {
            eprintln!(
                "oracle_steps={} final={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

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
                "mild_steps={} final={:?} neck={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n),
                rc_of(plan.state.neck(), input.n)
            );
        }

        let strong = inventory_transport_to_goal_plan_with_limits(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            goal,
            &timer.start,
            240,
            1_000_000,
            32,
            16,
        );
        eprintln!("strong={}", strong.is_some());
        if let Some(plan) = strong.as_ref() {
            eprintln!(
                "strong_steps={} final={:?} neck={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n),
                rc_of(plan.state.neck(), input.n)
            );
            let strong_harvest = inventory_harvest_from_state(&plan.state, &input, stock_cnt);
            eprintln!("strong_direct_harvest={}", strong_harvest.is_some());
        }

        let any_harvestable = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            240,
            1_000_000,
            32,
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

        let goal_harvestable = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            240,
            1_000_000,
            32,
            16,
            harvest_accept_goal,
        );
        eprintln!("goal_harvestable={}", goal_harvestable.is_some());
        if let Some(plan) = goal_harvestable.as_ref() {
            eprintln!(
                "goal_harvestable_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        let right_harvestable = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            240,
            1_000_000,
            32,
            16,
            harvest_accept_with_right_entry,
        );
        eprintln!("right_harvestable={}", right_harvestable.is_some());
        if let Some(plan) = right_harvestable.as_ref() {
            eprintln!(
                "right_harvestable_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }

        drop(_constraint);
        if stock_cnt >= 1 {
            let _relaxed = push_movement_constraint(movement_constraint_with_min_col(2 * stock_cnt - 2));
            let relaxed_right = direct_harvestable_state_search(
                &bs.state,
                &input,
                &tail_colors,
                tail_colors.len(),
                stock_cnt,
                &timer,
                240,
                1_000_000,
                32,
                16,
                harvest_accept_with_right_entry,
            );
            eprintln!("relaxed_right_harvestable={}", relaxed_right.is_some());
            if let Some(plan) = relaxed_right.as_ref() {
                eprintln!(
                    "relaxed_right_steps={} head={:?}",
                    plan.ops.len(),
                    rc_of(plan.state.head(), input.n)
                );
            }
        }

        for extra in 3..=5 {
            if 2 * stock_cnt < extra {
                continue;
            }
            let _relaxed_more =
                push_movement_constraint(movement_constraint_with_min_col(2 * stock_cnt - extra));
            let relaxed_direct = direct_harvestable_state_search(
                &bs.state,
                &input,
                &tail_colors,
                tail_colors.len(),
                stock_cnt,
                &timer,
                240,
                1_000_000,
                32,
                16,
                harvest_accept_direct,
            );
            eprintln!("relaxed_direct_{}col={}", extra + 1, relaxed_direct.is_some());
            if let Some(plan) = relaxed_direct.as_ref() {
                eprintln!(
                    "relaxed_direct_{}col_steps={} head={:?}",
                    extra + 1,
                    plan.ops.len(),
                    rc_of(plan.state.head(), input.n)
                );
            }
        }

        let _free = push_movement_constraint(movement_constraint_with_min_col(0));
        let free_direct = direct_harvestable_state_search(
            &bs.state,
            &input,
            &tail_colors,
            tail_colors.len(),
            stock_cnt,
            &timer,
            320,
            2_000_000,
            40,
            20,
            harvest_accept_direct,
        );
        eprintln!("free_direct_harvestable={}", free_direct.is_some());
        if let Some(plan) = free_direct.as_ref() {
            eprintln!(
                "free_direct_steps={} head={:?}",
                plan.ops.len(),
                rc_of(plan.state.head(), input.n)
            );
        }
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_inventory_harvest_v144 <input>");
    solver::probe(&path);
}
