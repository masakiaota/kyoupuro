// probe_inventory_entry_repair.rs

mod solver {
    #![allow(dead_code)]
    include!("../v146_entry_repair.rs");

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

    fn dump_state(label: &str, st: &State, goal_colors: &[u8]) {
        eprintln!(
            "{label}: head={:?} neck={:?} len={} lcp_goal={}",
            rc_of(st.head(), st.n),
            rc_of(st.neck(), st.n),
            st.len(),
            lcp_state(st, goal_colors),
        );
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

        let goal_colors = inventory_segment_target(&input, seg_idx, seg_len);
        let target = InventoryTarget {
            colors: &goal_colors,
            protect_len: goal_colors.len(),
        };
        let phases_left = stock_cnt.saturating_sub(seg_idx) + 1;
        let min_allowed_col = 2 * seg_idx;
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));

        eprintln!(
            "case={} seg_idx={} min_col={} goal_len={}",
            path,
            seg_idx,
            min_allowed_col,
            goal_colors.len()
        );

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
        dump_state("prepared", &prepared.state, &goal_colors);

        let built = if exact_prefix(&prepared.state, &goal_colors, goal_colors.len()) {
            Some(prepared)
        } else {
            match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                InventoryBuildOutcome::Built(next_bs) => Some(next_bs),
                InventoryBuildOutcome::GrowFailed => {
                    eprintln!("build=grow_failed");
                    None
                }
                InventoryBuildOutcome::ExactFailed => {
                    eprintln!("build=exact_failed");
                    None
                }
            }
        };
        let Some(built) = built else {
            return;
        };
        dump_state("built", &built.state, &goal_colors);

        let geom = inventory_entry_geometry(input.n, seg_idx).unwrap();
        let oracle =
            inventory_transport_oracle_direct_plan(&built.state, &goal_colors, goal_colors.len(), geom);
        eprintln!("oracle_direct={}", oracle.is_some());
        if let Some(plan) = oracle.as_ref() {
            dump_state("oracle_state", &plan.state, &goal_colors);
        }

        let repair = inventory_transport_plan_with_repair(
            &built.state,
            &input,
            &goal_colors,
            goal_colors.len(),
            geom,
            &timer.start,
        );
        eprintln!("repair_plan={}", repair.is_some());
        if let Some(plan) = repair.as_ref() {
            dump_state("repair_state", &plan.state, &goal_colors);
        }

        let final_transport = transport_to_entry_from_right(
            &built,
            &input,
            &goal_colors,
            goal_colors.len(),
            seg_idx,
            &timer.start,
        );
        eprintln!("final_transport={}", final_transport.is_some());
        if let Some(plan) = final_transport.as_ref() {
            dump_state("final_state", &plan.state, &goal_colors);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: probe_inventory_entry_repair <input> <seg_idx>");
    let seg_idx: usize = args.next().unwrap().parse().unwrap();
    solver::probe(&path, seg_idx);
}
