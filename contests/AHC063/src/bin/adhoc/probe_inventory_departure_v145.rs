// probe_inventory_departure_v145.rs

mod solver {
    #![allow(dead_code)]
    include!("../v145_harvest_repair.rs");

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
            let bad_drop: Vec<_> = dropped
                .as_slice()
                .iter()
                .filter_map(|ent| {
                    let rc = rc_of(ent.cell, st.n);
                    let bad = ent.cell as usize % st.n < min_allowed_col;
                    bad.then_some(rc)
                })
                .collect();
            rows.push(format!(
                "{} -> ({},{}) food={} bite={} residue={} prefix5={} bad_drop={:?}",
                DIR_CHARS[dir],
                r,
                c,
                ate,
                bite_idx.is_some(),
                inventory_depart_bad_count(&ns, min_allowed_col),
                inventory_launch_prefix_ok(&ns, goal_colors),
                bad_drop
            ));
        }
        eprintln!("{label} legal: {}", rows.join(" | "));
    }

    pub fn probe(path: &str, seg_idx: usize) {
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
        for done in 0..seg_idx {
            bs = inventory_run_segment_phase(&ctx, &bs, done)
                .unwrap_or_else(|| panic!("phase failed before seg_idx={seg_idx}, done={done}"));
        }

        let min_allowed_col = 2 * seg_idx;
        let goal_colors = inventory_segment_target(&input, seg_idx, seg_len);
        let target = InventoryTarget {
            colors: &goal_colors,
            protect_len: goal_colors.len(),
        };
        let phases_left = stock_cnt.saturating_sub(seg_idx) + 1;
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));

        eprintln!(
            "case={} seg_idx={} min_col={} stock_cnt={} goal_len={}",
            path,
            seg_idx,
            min_allowed_col,
            stock_cnt,
            goal_colors.len()
        );
        dump_state("after_prev_place", &bs.state, &goal_colors, min_allowed_col);
        dump_local_moves("after_prev_place", &bs.state, &goal_colors, min_allowed_col);

        let prepared = inventory_prepare_build_start(
            &ctx,
            &bs,
            &goal_colors,
            phases_left,
            min_allowed_col,
        );
        eprintln!("prepare={}", prepared.is_some());
        let Some(prepared) = prepared else {
            return;
        };
        dump_state("prepared", &prepared.state, &goal_colors, min_allowed_col);
        dump_local_moves("prepared", &prepared.state, &goal_colors, min_allowed_col);

        if exact_prefix(&prepared.state, &goal_colors, goal_colors.len()) {
            eprintln!("prepared already exact");
        } else {
            let built = inventory_build_target_exact(&ctx, &prepared, &target, phases_left);
            match built {
                InventoryBuildOutcome::Built(next_bs) => {
                    eprintln!("build=ok");
                    dump_state("built", &next_bs.state, &goal_colors, min_allowed_col);
                    let transport = transport_to_entry_from_right(
                        &next_bs,
                        &input,
                        &goal_colors,
                        goal_colors.len(),
                        seg_idx,
                        &timer.start,
                    );
                    eprintln!("transport={}", transport.is_some());
                    if let Some(tr) = transport {
                        dump_state("transported", &tr.state, &goal_colors, min_allowed_col);
                    }
                }
                InventoryBuildOutcome::GrowFailed => {
                    eprintln!("build=grow_failed");
                }
                InventoryBuildOutcome::ExactFailed => {
                    eprintln!("build=exact_failed");
                }
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: probe_inventory_departure_v145 <input> <seg_idx>");
    let seg_idx: usize = args.next().unwrap().parse().unwrap();
    solver::probe(&path, seg_idx);
}
