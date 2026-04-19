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

    fn transport_plan_custom(
        st: &State,
        input: &Input,
        protect_colors: &[u8],
        protect_len: usize,
        geom: InventoryEntryGeometry,
        depth_limit: usize,
        node_limit: usize,
        eat_limit: u8,
        bite_limit: u8,
    ) -> Option<BeamState> {
        let started = Instant::now();
        let mut nodes = Vec::with_capacity(node_limit.min(32_768) + 8);
        let mut depths = Vec::with_capacity(node_limit.min(32_768) + 8);
        let mut eat_counts = Vec::with_capacity(node_limit.min(32_768) + 8);
        let mut bite_counts = Vec::with_capacity(node_limit.min(32_768) + 8);
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
            inventory_transport_plan_rank(st, input, protect_len, geom, 0, 0, 0),
            uid,
            0usize,
        )));
        uid += 1;

        let mut seen = FxHashMap::<State, (u8, u8, u8)>::default();
        seen.insert(st.clone(), (0, 0, 0));
        let mut expansions = 0usize;

        while let Some(Reverse((_, _, idx))) = pq.pop() {
            if expansions >= node_limit || started.elapsed().as_secs_f64() > 30.0 {
                break;
            }
            expansions += 1;

            if let Some(done) =
                inventory_transport_finish_from_node(&nodes, idx, protect_colors, protect_len, geom)
            {
                eprintln!("probe expansions={expansions} direct_finish");
                return Some(done);
            }

            let cur = nodes[idx].state.clone();
            if let Some(suffix) =
                inventory_transport_oracle_direct_plan(&cur, protect_colors, protect_len, geom)
            {
                if let Some(done) = inventory_transport_finish_from_node_with_suffix(
                    &nodes,
                    idx,
                    &suffix,
                    protect_colors,
                    protect_len,
                    geom,
                ) {
                    eprintln!(
                        "probe expansions={expansions} oracle_finish suffix_steps={}",
                        suffix.ops.len()
                    );
                    return Some(done);
                }
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

                if inventory_is_entry_cell(geom, ns.head()) {
                    if inventory_transport_finish_kind(&ns, protect_colors, protect_len, geom).is_some()
                    {
                        let child = nodes.len();
                        nodes.push(InventoryStateNode {
                            state: ns,
                            parent: Some(idx),
                            mv: dir_u8,
                        });
                        depths.push((depth + 1) as u8);
                        eat_counts.push(next_eat);
                        bite_counts.push(next_bite);
                        eprintln!("probe expansions={expansions} entry_finish");
                        return inventory_transport_finish_from_node(
                            &nodes,
                            child,
                            protect_colors,
                            protect_len,
                            geom,
                        );
                    }
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
                    inventory_transport_plan_rank(
                        &ns,
                        input,
                        protect_len,
                        geom,
                        next_eat,
                        next_bite,
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

        eprintln!("probe miss expansions={expansions}");
        None
    }

    fn check_prepared_candidate(
        name: &str,
        prepared: &BeamState,
        ctx: &InventoryCtx<'_>,
        target: &InventoryTarget<'_>,
        phases_left: usize,
        seg_idx: usize,
    ) {
        let built = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
            Some(prepared.clone())
        } else {
            match inventory_build_target_exact(ctx, prepared, target, phases_left) {
                InventoryBuildOutcome::Built(bs) => Some(bs),
                _ => None,
            }
        };
        let Some(built) = built else {
            eprintln!("{name}: build_failed");
            return;
        };
        let geom = inventory_entry_geometry(ctx.input.n, seg_idx).unwrap();
        let strong = transport_plan_custom(
            &built.state,
            ctx.input,
            target.colors,
            target.protect_len,
            geom,
            80,
            300_000,
            12,
            6,
        );
        eprintln!(
            "{name}: built_head={:?} built_len={} transportable={}",
            rc_of(built.state.head(), ctx.input.n),
            built.state.len(),
            strong.is_some()
        );
    }

    pub fn probe(path: &str, target_seg: usize) {
        let input_text = std::fs::read_to_string(path).unwrap();
        let input = read_input_from_str(&input_text);
        let timer = TimeKeeper::new(60.0, 8);
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
            let target_colors = inventory_segment_target(&input, seg_idx, seg_len);
            let target = InventoryTarget {
                colors: &target_colors,
                protect_len: target_colors.len(),
            };
            let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * seg_idx));
            let phases_left = stock_cnt - seg_idx + 1;
            let prepared =
                inventory_prepare_build_start(&ctx, &bs, target.colors, phases_left, 2 * seg_idx)
                    .expect("launch failed before target seg");
            let split_built = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                prepared.clone()
            } else {
                match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                    InventoryBuildOutcome::Built(next_bs) => next_bs,
                    _ => inventory_build_target_exact_legacy_parked(&ctx, &bs, &target, phases_left)
                        .built_or_panic(),
                }
            };

            if seg_idx == target_seg {
                let geom = inventory_entry_geometry(input.n, seg_idx).unwrap();
                let min_allowed_col = 2 * seg_idx;
                eprintln!("prepared variants:");
                check_prepared_candidate("base", &bs, &ctx, &target, phases_left, seg_idx);
                if let Some(empty4) =
                    inventory_try_depart_empty4_static(&bs, &input, target.colors, min_allowed_col)
                {
                    check_prepared_candidate(
                        "empty4",
                        &empty4,
                        &ctx,
                        &target,
                        phases_left,
                        seg_idx,
                    );
                } else {
                    eprintln!("empty4: none");
                }
                if let Some(direct) = inventory_try_launch_direct(
                    &bs,
                    &input,
                    target.colors,
                    &timer,
                    min_allowed_col,
                ) {
                    check_prepared_candidate(
                        "direct",
                        &direct,
                        &ctx,
                        &target,
                        phases_left,
                        seg_idx,
                    );
                } else {
                    eprintln!("direct: none");
                }
                check_prepared_candidate(
                    "prepared",
                    &prepared,
                    &ctx,
                    &target,
                    phases_left,
                    seg_idx,
                );
                eprintln!(
                    "probe seg={seg_idx} head={:?} len={} protect_len={}",
                    rc_of(split_built.state.head(), input.n),
                    split_built.state.len(),
                    target.protect_len
                );
                let mild = transport_plan_custom(
                    &split_built.state,
                    &input,
                    target.colors,
                    target.protect_len,
                    geom,
                    48,
                    60_000,
                    8,
                    3,
                );
                eprintln!("mild={}", mild.is_some());
                let strong = transport_plan_custom(
                    &split_built.state,
                    &input,
                    target.colors,
                    target.protect_len,
                    geom,
                    80,
                    300_000,
                    12,
                    6,
                );
                eprintln!("strong={}", strong.is_some());
                if let Some(plan) = strong {
                    eprintln!(
                        "strong_steps={} final_head={:?} final_len={}",
                        plan.ops.len(),
                        rc_of(plan.state.head(), input.n),
                        plan.state.len()
                    );
                }
                if let Some(alt) = inventory_try_launch_rescue_build(
                    &prepared,
                    &input,
                    target.colors,
                    &timer,
                    0.80,
                    false,
                ) {
                    eprintln!(
                        "rescue_build head={:?} len={}",
                        rc_of(alt.state.head(), input.n),
                        alt.state.len()
                    );
                    let rescue_strong = transport_plan_custom(
                        &alt.state,
                        &input,
                        target.colors,
                        target.protect_len,
                        geom,
                        80,
                        300_000,
                        12,
                        6,
                    );
                    eprintln!("rescue_build_strong={}", rescue_strong.is_some());
                    if let Some(plan) = rescue_strong {
                        eprintln!(
                            "rescue_steps={} final_head={:?} final_len={}",
                            plan.ops.len(),
                            rc_of(plan.state.head(), input.n),
                            plan.state.len()
                        );
                    }
                }
                if bs.state.len() == 5 {
                    if let InventoryBuildOutcome::Built(alt) =
                        inventory_build_target_exact_legacy_parked(&ctx, &bs, &target, phases_left)
                    {
                        eprintln!(
                            "legacy head={:?} len={}",
                            rc_of(alt.state.head(), input.n),
                            alt.state.len()
                        );
                        let legacy_strong = transport_plan_custom(
                            &alt.state,
                            &input,
                            target.colors,
                            target.protect_len,
                            geom,
                            80,
                            300_000,
                            12,
                            6,
                        );
                        eprintln!("legacy_strong={}", legacy_strong.is_some());
                        if let Some(plan) = legacy_strong {
                            eprintln!(
                                "legacy_steps={} final_head={:?} final_len={}",
                                plan.ops.len(),
                                rc_of(plan.state.head(), input.n),
                                plan.state.len()
                            );
                        }
                    }
                }
                return;
            }

            bs = inventory_run_segment_phase(&ctx, &bs, seg_idx).expect("phase failed before target");
        }
    }

    trait BuiltOrPanic {
        fn built_or_panic(self) -> BeamState;
    }

    impl BuiltOrPanic for InventoryBuildOutcome {
        fn built_or_panic(self) -> BeamState {
            match self {
                InventoryBuildOutcome::Built(bs) => bs,
                InventoryBuildOutcome::GrowFailed => panic!("grow failed"),
                InventoryBuildOutcome::ExactFailed => panic!("exact failed"),
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: probe_inventory_transport <input> <seg_idx>");
    let seg_idx: usize = args.next().unwrap().parse().unwrap();
    solver::probe(&path, seg_idx);
}
