// probe_inventory_build_after_place.rs

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

    fn colors_prefix(st: &State, len: usize) -> Vec<u8> {
        (0..len.min(st.len())).map(|i| st.colors[i]).collect()
    }

    fn longest_exact_prefix(st: &State, goal: &[u8]) -> usize {
        let lim = st.len().min(goal.len());
        let mut k = 0usize;
        while k < lim && st.colors[k] == goal[k] {
            k += 1;
        }
        k
    }

    pub fn run(input_path: &str) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let mut bs = BeamState {
            state: State::initial(&input),
            ops: Ops::new(),
        };

        println!("n={} m={} seg_len={} stock_cnt={}", input.n, input.m, seg_len, stock_cnt);

        for seg_idx in 0..stock_cnt {
            let target = inventory_segment_target(&input, seg_idx, seg_len);
            println!("== seg_idx={} target_len={} ==", seg_idx, target.len());
            {
                let _constraint = push_movement_constraint(movement_constraint_with_min_col(2 * seg_idx));
                let grown = grow_to_target_prefix(
                    &input,
                    bs.state.clone(),
                    &target,
                    &timer,
                    inventory_build_time_limit(&timer, stock_cnt - seg_idx + 1),
                )
                .unwrap();
                println!(
                    " build: ops={} exact={} head={:?} neck={:?} len={} prefix={:?}",
                    grown.ops.len(),
                    exact_prefix(&grown.state, &target, target.len()),
                    rc_of(grown.state.head(), input.n),
                    rc_of(grown.state.neck(), input.n),
                    grown.state.len(),
                    colors_prefix(&grown.state, target.len())
                );
                bs = append_incremental_beam(&bs, grown).unwrap();
                let moved = transport_to_entry_from_right(
                    &bs,
                    &input,
                    &target,
                    target.len(),
                    seg_idx,
                    &timer.start,
                )
                .unwrap();
                println!(
                    " transport: ops={} head={:?} len={} prefix_ok={}",
                    moved.ops.len() - bs.ops.len(),
                    rc_of(moved.state.head(), input.n),
                    moved.state.len(),
                    prefix_ok(&moved.state, &target, target.len())
                );
                bs = moved;
                let placed = place_inventory_segment(&bs, seg_idx, input.n, &target, target.len()).unwrap();
                println!(
                    " placed: head={:?} neck={:?} len={} total_ops={} colors={:?}",
                    rc_of(placed.state.head(), input.n),
                    rc_of(placed.state.neck(), input.n),
                    placed.state.len(),
                    placed.ops.len(),
                    colors_prefix(&placed.state, placed.state.len())
                );
                bs = placed;
            }

            if seg_idx + 1 < stock_cnt {
                let next_target = inventory_segment_target(&input, seg_idx + 1, seg_len);
                let next_min_col = 2 * (seg_idx + 1);
                let _constraint = push_movement_constraint(movement_constraint_with_min_col(next_min_col));
                let legal = legal_dirs(&bs.state);
                let legal_rc: Vec<_> = legal
                    .as_slice()
                    .iter()
                    .map(|&d| {
                        let nh = next_head_cell(&bs.state, d as usize).unwrap();
                        let (r, c) = rc_of(nh, input.n);
                        (DIR_CHARS[d as usize] as char, r, c, is_cell_allowed(input.n, nh))
                    })
                    .collect();
                println!(
                    " next-build start: min_col={} head={:?} neck={:?} len={} legal={:?} target_prefix={:?}",
                    next_min_col,
                    rc_of(bs.state.head(), input.n),
                    rc_of(bs.state.neck(), input.n),
                    bs.state.len(),
                    legal_rc,
                    &next_target[..next_target.len().min(12)]
                );
                let grown = grow_to_target_prefix(
                    &input,
                    bs.state.clone(),
                    &next_target,
                    &timer,
                    inventory_build_time_limit(&timer, stock_cnt - (seg_idx + 1) + 1),
                )
                .unwrap();
                println!(
                    " probe-next-build: ops={} exact={} match_len={} head={:?} neck={:?} len={} prefix12={:?}",
                    grown.ops.len(),
                    exact_prefix(&grown.state, &next_target, next_target.len()),
                    longest_exact_prefix(&grown.state, &next_target),
                    rc_of(grown.state.head(), input.n),
                    rc_of(grown.state.neck(), input.n),
                    grown.state.len(),
                    colors_prefix(&grown.state, next_target.len().min(12))
                );
                let grown_big = grow_to_target_prefix(
                    &input,
                    bs.state.clone(),
                    &next_target,
                    &timer,
                    2.0,
                )
                .unwrap();
                println!(
                    " probe-next-build-big: ops={} exact={} match_len={} head={:?} neck={:?} len={}",
                    grown_big.ops.len(),
                    exact_prefix(&grown_big.state, &next_target, next_target.len()),
                    longest_exact_prefix(&grown_big.state, &next_target),
                    rc_of(grown_big.state.head(), input.n),
                    rc_of(grown_big.state.neck(), input.n),
                    grown_big.state.len(),
                );
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: probe_inventory_build_after_place <input.txt>");
        std::process::exit(1);
    }
    solver::run(&args[1]);
}
