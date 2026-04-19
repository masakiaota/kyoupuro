// probe_v167_v168_segment.rs

#[derive(Debug)]
struct SegmentTrace {
    base_ops: usize,
    direct_build_hit: bool,
    prepared_ops: Option<usize>,
    build_path: String,
    built_ops: Option<usize>,
    transport_primary_ok: bool,
    rollback_guard: bool,
    retry_candidates: usize,
    rollback_recovered_at: Option<usize>,
    placed_ops: Option<usize>,
    final_head: Option<(usize, usize)>,
}

mod v167 {
    #![allow(dead_code)]
    include!("../v167_phase_resource_budget.rs");

    use super::SegmentTrace;

    fn parse_input_from_str(s: &str) -> Input {
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

    pub fn trace_segment(input_path: &str, target_seg_idx: usize) -> SegmentTrace {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
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
        for seg_idx in 0..target_seg_idx {
            bs = match inventory_run_segment_phase(&ctx, &bs, seg_idx) {
                InventoryPhaseOutcome::Completed(next_bs) => next_bs,
                InventoryPhaseOutcome::BuiltOnly(next_bs) => next_bs,
                InventoryPhaseOutcome::PreparedOnly(next_bs) => next_bs,
                InventoryPhaseOutcome::Failed => bs,
            };
        }

        let target_colors = inventory_segment_target(ctx.input, target_seg_idx, ctx.seg_len);
        let target = InventoryTarget {
            colors: &target_colors,
            protect_len: target_colors.len(),
        };
        let _goal_hash = push_goal_prefix_hash(target.colors);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * target_seg_idx));
        let phases_left = ctx.stock_cnt - target_seg_idx + 1;
        let _phase_budget = push_inventory_phase_budget(ctx.timer, 0.0, INVENTORY_MIN_LEFT_SEC);

        let base_ops = bs.ops.len();
        let mut direct_build_hit = false;
        let mut prepared_ops = None;
        let build_path: String;
        let split_built;
        let prepared_for_rescue;

        if bs.state.len() == 5 && target_seg_idx > 0 {
            if let Some(next_bs) = inventory_try_direct_build_from_current(&ctx, &bs, &target, phases_left) {
                direct_build_hit = true;
                build_path = "direct_build".to_string();
                split_built = next_bs;
                prepared_for_rescue = None;
            } else {
                let prepared = inventory_prepare_build_start(
                    &ctx,
                    &bs,
                    target.colors,
                    phases_left,
                    2 * target_seg_idx,
                )
                .unwrap();
                prepared_ops = Some(prepared.ops.len());
                if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                    build_path = "prepared_exact".to_string();
                    split_built = prepared.clone();
                    prepared_for_rescue = Some(prepared);
                } else {
                    let (next_bs, path_name) = match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "exact"),
                        InventoryBuildOutcome::GrowFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                            InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current"),
                            InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                            InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_legacy_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_legacy_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                            InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current_after_exact_fail"),
                            InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                            InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_double_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_double_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                        },
                    };
                    build_path = path_name.to_string();
                    split_built = next_bs;
                    prepared_for_rescue = Some(prepared);
                }
            }
        } else {
            let prepared = inventory_prepare_build_start(
                &ctx,
                &bs,
                target.colors,
                phases_left,
                2 * target_seg_idx,
            )
            .unwrap();
            prepared_ops = Some(prepared.ops.len());
            let (next_bs, path_name) = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                (prepared.clone(), "prepared_exact")
            } else {
                match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                    InventoryBuildOutcome::Built(next_bs) => (next_bs, "exact"),
                    InventoryBuildOutcome::GrowFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current"),
                        InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_legacy_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_legacy_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                    },
                    InventoryBuildOutcome::ExactFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current_after_exact_fail"),
                        InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_double_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_double_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                    },
                }
            };
            build_path = path_name.to_string();
            split_built = next_bs;
            prepared_for_rescue = Some(prepared);
        }

        let built_ops = Some(split_built.ops.len());
        let primary_transport = transport_to_entry_from_right(
            &split_built,
            ctx.input,
            target.colors,
            target.protect_len,
            target_seg_idx,
            &ctx.timer.start,
        );
        let transport_primary_ok = primary_transport.is_some();
        let rollback_guard = inventory_should_try_phase_rollback(&ctx, phases_left);
        let mut retry_candidates = 0usize;
        let mut rollback_recovered_at = None;
        let transported = if let Some(next_bs) = primary_transport {
            Some(next_bs)
        } else if rollback_guard {
            let alts = inventory_collect_transport_retry_candidates(
                &ctx,
                &bs,
                prepared_for_rescue.as_ref(),
                &target,
                phases_left,
                target_seg_idx,
                &split_built,
            );
            retry_candidates = alts.len();
            let mut found = None;
            for (idx, alt_built) in alts.into_iter().enumerate() {
                if let Some(next_bs) = transport_to_entry_from_right(
                    &alt_built,
                    ctx.input,
                    target.colors,
                    target.protect_len,
                    target_seg_idx,
                    &ctx.timer.start,
                ) {
                    rollback_recovered_at = Some(idx);
                    found = Some(next_bs);
                    break;
                }
            }
            found
        } else {
            None
        };

        let placed = transported.and_then(|bs2| {
            place_inventory_segment(&bs2, target_seg_idx, ctx.input.n, target.colors, target.protect_len)
        });

        SegmentTrace {
            base_ops,
            direct_build_hit,
            prepared_ops,
            build_path,
            built_ops,
            transport_primary_ok,
            rollback_guard,
            retry_candidates,
            rollback_recovered_at,
            placed_ops: placed.as_ref().map(|x| x.ops.len()),
            final_head: placed.as_ref().map(|x| rc_of(x.state.head(), x.state.n)),
        }
    }
}

mod v168 {
    #![allow(dead_code)]
    include!("../v168_phase_resource_budget_no_logs.rs");

    use super::SegmentTrace;

    fn parse_input_from_str(s: &str) -> Input {
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

    pub fn trace_segment(input_path: &str, target_seg_idx: usize) -> SegmentTrace {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
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
        for seg_idx in 0..target_seg_idx {
            bs = match inventory_run_segment_phase(&ctx, &bs, seg_idx) {
                InventoryPhaseOutcome::Completed(next_bs) => next_bs,
                InventoryPhaseOutcome::BuiltOnly(next_bs) => next_bs,
                InventoryPhaseOutcome::PreparedOnly(next_bs) => next_bs,
                InventoryPhaseOutcome::Failed => bs,
            };
        }

        let target_colors = inventory_segment_target(ctx.input, target_seg_idx, ctx.seg_len);
        let target = InventoryTarget {
            colors: &target_colors,
            protect_len: target_colors.len(),
        };
        let _goal_hash = push_goal_prefix_hash(target.colors);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * target_seg_idx));
        let phases_left = ctx.stock_cnt - target_seg_idx + 1;
        let _phase_budget = push_inventory_phase_budget(ctx.timer, 0.0, INVENTORY_MIN_LEFT_SEC);

        let base_ops = bs.ops.len();
        let mut direct_build_hit = false;
        let mut prepared_ops = None;
        let build_path: String;
        let split_built;
        let prepared_for_rescue;

        if bs.state.len() == 5 && target_seg_idx > 0 {
            if let Some(next_bs) = inventory_try_direct_build_from_current(&ctx, &bs, &target, phases_left) {
                direct_build_hit = true;
                build_path = "direct_build".to_string();
                split_built = next_bs;
                prepared_for_rescue = None;
            } else {
                let prepared = inventory_prepare_build_start(
                    &ctx,
                    &bs,
                    target.colors,
                    phases_left,
                    2 * target_seg_idx,
                )
                .unwrap();
                prepared_ops = Some(prepared.ops.len());
                if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                    build_path = "prepared_exact".to_string();
                    split_built = prepared.clone();
                    prepared_for_rescue = Some(prepared);
                } else {
                    let (next_bs, path_name) = match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "exact"),
                        InventoryBuildOutcome::GrowFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                            InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current"),
                            InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                            InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_legacy_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_legacy_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                            InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current_after_exact_fail"),
                            InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                            InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "transportable_current_after_double_exact_fail"),
                                None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                    Some(next_bs) => (next_bs, "safe_parked_after_double_exact_fail"),
                                    None => (prepared.clone(), "prepared_only"),
                                },
                            },
                        },
                    };
                    build_path = path_name.to_string();
                    split_built = next_bs;
                    prepared_for_rescue = Some(prepared);
                }
            }
        } else {
            let prepared = inventory_prepare_build_start(
                &ctx,
                &bs,
                target.colors,
                phases_left,
                2 * target_seg_idx,
            )
            .unwrap();
            prepared_ops = Some(prepared.ops.len());
            let (next_bs, path_name) = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                (prepared.clone(), "prepared_exact")
            } else {
                match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                    InventoryBuildOutcome::Built(next_bs) => (next_bs, "exact"),
                    InventoryBuildOutcome::GrowFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current"),
                        InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_legacy_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_legacy_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                    },
                    InventoryBuildOutcome::ExactFailed => match inventory_build_target_exact_legacy_current(&ctx, &prepared, &target, phases_left) {
                        InventoryBuildOutcome::Built(next_bs) => (next_bs, "legacy_current_after_exact_fail"),
                        InventoryBuildOutcome::GrowFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                        InventoryBuildOutcome::ExactFailed => match inventory_try_transportable_build_from_current(&ctx, &prepared, &target, phases_left, target_seg_idx) {
                            Some(next_bs) => (next_bs, "transportable_current_after_double_exact_fail"),
                            None => match inventory_try_safe_parked_build_fallback(&ctx, &bs, &target, phases_left, target_seg_idx) {
                                Some(next_bs) => (next_bs, "safe_parked_after_double_exact_fail"),
                                None => (prepared.clone(), "prepared_only"),
                            },
                        },
                    },
                }
            };
            build_path = path_name.to_string();
            split_built = next_bs;
            prepared_for_rescue = Some(prepared);
        }

        let built_ops = Some(split_built.ops.len());
        let primary_transport = transport_to_entry_from_right(
            &split_built,
            ctx.input,
            target.colors,
            target.protect_len,
            target_seg_idx,
            &ctx.timer.start,
        );
        let transport_primary_ok = primary_transport.is_some();
        let rollback_guard = inventory_should_try_phase_rollback(&ctx, phases_left);
        let mut retry_candidates = 0usize;
        let mut rollback_recovered_at = None;
        let transported = if let Some(next_bs) = primary_transport {
            Some(next_bs)
        } else if rollback_guard {
            let alts = inventory_collect_transport_retry_candidates(
                &ctx,
                &bs,
                prepared_for_rescue.as_ref(),
                &target,
                phases_left,
                target_seg_idx,
                &split_built,
            );
            retry_candidates = alts.len();
            let mut found = None;
            for (idx, alt_built) in alts.into_iter().enumerate() {
                if let Some(next_bs) = transport_to_entry_from_right(
                    &alt_built,
                    ctx.input,
                    target.colors,
                    target.protect_len,
                    target_seg_idx,
                    &ctx.timer.start,
                ) {
                    rollback_recovered_at = Some(idx);
                    found = Some(next_bs);
                    break;
                }
            }
            found
        } else {
            None
        };

        let placed = transported.and_then(|bs2| {
            place_inventory_segment(&bs2, target_seg_idx, ctx.input.n, target.colors, target.protect_len)
        });

        SegmentTrace {
            base_ops,
            direct_build_hit,
            prepared_ops,
            build_path,
            built_ops,
            transport_primary_ok,
            rollback_guard,
            retry_candidates,
            rollback_recovered_at,
            placed_ops: placed.as_ref().map(|x| x.ops.len()),
            final_head: placed.as_ref().map(|x| rc_of(x.state.head(), x.state.n)),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: probe_v167_v168_segment <input> <seg_idx>");
        std::process::exit(1);
    }
    let input = &args[1];
    let seg_idx: usize = args[2].parse().unwrap();
    let a = v167::trace_segment(input, seg_idx);
    let b = v168::trace_segment(input, seg_idx);
    println!("v167 {:?}", a);
    println!("v168 {:?}", b);
}
