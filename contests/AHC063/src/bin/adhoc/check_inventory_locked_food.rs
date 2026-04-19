// check_inventory_locked_food.rs

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

    fn placed_turn_boundaries(input: &Input) -> Vec<usize> {
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let mut bs = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
        let mut out = Vec::new();

        for seg_idx in 0..stock_cnt {
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
            out.push(bs.ops.len());
        }
        out
    }

    pub fn analyze(input_path: &str, output_path: &str, locked_cols: usize) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let output = std::fs::read_to_string(output_path).unwrap();
        let placed_turns = placed_turn_boundaries(&input);
        let mut st = State::initial(&input);
        let mut dropped = DroppedBuf::new();
        let mut bite_count = 0usize;
        let mut placed_cnt = 0usize;

        for (turn, line) in output.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            while placed_cnt < placed_turns.len() && placed_turns[placed_cnt] < turn + 1 {
                placed_cnt += 1;
            }
            let active_locked_cols = (2 * placed_cnt).min(locked_cols);
            let dir = dir_of_char(line.as_bytes()[0]).unwrap();
            let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped);
            if bite_idx.is_some() {
                bite_count += 1;
                let bad: Vec<_> = dropped
                    .as_slice()
                    .iter()
                    .filter_map(|ent| {
                        let (r, c) = rc_of(ent.cell, input.n);
                        (r + 2 >= input.n && c < active_locked_cols).then_some((r, c, ent.color))
                    })
                    .collect();
                if !bad.is_empty() {
                    println!(
                        "bad_bite turn={} locked_cols={} dropped={:?}",
                        turn + 1,
                        active_locked_cols,
                        bad
                    );
                }
            }
            st = ns;
        }

        let mut final_bad = Vec::new();
        for r in input.n.saturating_sub(2)..input.n {
            for c in 0..locked_cols.min(input.n) {
                let cell = cell_of(r, c, input.n);
                let color = st.food[cell as usize];
                if color != 0 {
                    final_bad.push((r, c, color));
                }
            }
        }
        println!("placed_turns={placed_turns:?}");
        println!("bite_count={bite_count}");
        println!("final_bottom_locked_food={final_bad:?}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "usage: cargo run --bin check_inventory_locked_food -- <input.txt> <output.txt> <locked_cols>"
        );
        std::process::exit(1);
    }
    let locked_cols: usize = args[3].parse().unwrap();
    solver::analyze(&args[1], &args[2], locked_cols);
}
