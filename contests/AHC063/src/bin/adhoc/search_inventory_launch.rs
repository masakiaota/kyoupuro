// search_inventory_launch.rs

mod solver {
    #![allow(dead_code)]
    include!("../v140_inventory_stock.rs");

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

    fn reconstruct_to_seg_start(input: &Input, placed_seg_idx: usize) -> BeamState {
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let mut bs = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
        for seg_idx in 0..=placed_seg_idx {
            let target = inventory_segment_target(input, seg_idx, seg_len);
            let phases_left = stock_cnt - seg_idx + 1;
            let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * seg_idx));
            let grown = grow_to_target_prefix(
                input,
                bs.state.clone(),
                &target,
                &timer,
                inventory_build_time_limit(&timer, phases_left),
            )
            .unwrap();
            bs = append_incremental_beam(&bs, grown).unwrap();
            bs = transport_to_entry_from_right(
                &bs,
                input,
                &target,
                target.len(),
                seg_idx,
                &timer.start,
            )
            .unwrap();
            bs = place_inventory_segment(&bs, seg_idx, input.n, &target, target.len()).unwrap();
        }
        bs
    }

    #[derive(Clone)]
    struct LaunchNode {
        st: State,
        path: Vec<u8>,
    }

    pub fn search(input_path: &str, placed_seg_idx: usize, max_depth: usize) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let seg_len = 2 * (input.n - 2);
        let next_seg_idx = placed_seg_idx + 1;
        let target = inventory_segment_target(&input, next_seg_idx, seg_len);
        let start_bs = reconstruct_to_seg_start(&input, placed_seg_idx);
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * next_seg_idx));

        let base = grow_to_target_prefix(&input, start_bs.state.clone(), &target, &timer, 0.26).unwrap();
        println!(
            "base exact={} len={} head={:?} ops={}",
            exact_prefix(&base.state, &target, target.len()),
            base.state.len(),
            rc_of(base.state.head(), input.n),
            base.ops.len()
        );

        let mut q = std::collections::VecDeque::new();
        let mut seen = FxHashSet::default();
        q.push_back(LaunchNode {
            st: start_bs.state.clone(),
            path: Vec::new(),
        });
        seen.insert(start_bs.state.clone());
        let mut expansions = 0usize;

        while let Some(node) = q.pop_front() {
            expansions += 1;
            if node.path.len() >= max_depth {
                continue;
            }
            for &dir_u8 in legal_dirs(&node.st).as_slice() {
                let dir = dir_u8 as usize;
                let mut dropped = DroppedBuf::new();
                let (ns, _, bite_idx) = step_with_dropped(&node.st, dir, &mut dropped);
                if bite_idx.is_some() && !dropped_respects_active_constraint(node.st.n, &dropped) {
                    continue;
                }
                if !seen.insert(ns.clone()) {
                    continue;
                }
                let mut path = node.path.clone();
                path.push(dir_u8);

                let Some(grown) = grow_to_target_prefix(&input, ns.clone(), &target, &timer, 0.26) else {
                    q.push_back(LaunchNode { st: ns, path });
                    continue;
                };
                if exact_prefix(&grown.state, &target, target.len()) {
                    let probe = grow_to_target_prefix(&input, ns.clone(), &target, &timer, 0.08).unwrap();
                    let pretty: String = path.iter().map(|&d| DIR_CHARS[d as usize] as char).collect();
                    println!(
                        "hit depth={} launch={} head={:?} neck={:?} grown_ops={} total_match={} expansions={} probe_match={} probe_exact={}",
                        path.len(),
                        pretty,
                        rc_of(ns.head(), input.n),
                        rc_of(ns.neck(), input.n),
                        grown.ops.len(),
                        target.len(),
                        expansions,
                        lcp_state(&probe.state, &target),
                        exact_prefix(&probe.state, &target, target.len()),
                    );
                    return;
                }
                q.push_back(LaunchNode { st: ns, path });
            }
        }

        println!("no launch found up to depth={} expansions={}", max_depth, expansions);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: search_inventory_launch <input.txt> <placed_seg_idx> <max_depth>");
        std::process::exit(1);
    }
    solver::search(&args[1], args[2].parse().unwrap(), args[3].parse().unwrap());
}
