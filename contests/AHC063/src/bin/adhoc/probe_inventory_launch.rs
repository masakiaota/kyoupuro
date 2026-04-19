// probe_inventory_launch.rs

mod solver {
    #![allow(dead_code)]
    include!("../v143_launch_split.rs");

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

    fn pos_rc(st: &State) -> Vec<(usize, usize)> {
        (0..st.len()).map(|i| rc_of(st.pos[i], st.n)).collect()
    }

    pub fn run(input_path: &str) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let timer = TimeKeeper::new(1000.0, 0);
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

        println!("n={} m={} seg_len={} stock_cnt={}", input.n, input.m, seg_len, stock_cnt);

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
                    .unwrap_or_else(|| bs.clone());
            let built = if exact_prefix(&prepared.state, target.colors, target.protect_len) {
                prepared
            } else {
                match inventory_build_target_exact(&ctx, &prepared, &target, phases_left) {
                    InventoryBuildOutcome::Built(next_bs) => next_bs,
                    _ => {
                        println!("seg={} build failed after prepare", seg_idx);
                        return;
                    }
                }
            };
            let moved = match transport_to_entry_from_right(
                &built,
                &input,
                target.colors,
                target.protect_len,
                seg_idx,
                &timer.start,
            ) {
                Some(next_bs) => next_bs,
                None => {
                    println!("seg={} transport failed", seg_idx);
                    return;
                }
            };
            let placed = match place_inventory_segment(
                &moved,
                seg_idx,
                input.n,
                target.colors,
                target.protect_len,
            ) {
                Some(next_bs) => next_bs,
                None => {
                    println!("seg={} place failed", seg_idx);
                    return;
                }
            };
            println!(
                "placed seg={} head={:?} neck={:?} len={} colors={:?} pos={:?}",
                seg_idx,
                rc_of(placed.state.head(), input.n),
                rc_of(placed.state.neck(), input.n),
                placed.state.len(),
                colors_prefix(&placed.state, 8),
                pos_rc(&placed.state)
            );
            bs = placed;

            if seg_idx + 1 < stock_cnt {
                let next_target = inventory_segment_target(&input, seg_idx + 1, seg_len);
                let next_min_col = 2 * (seg_idx + 1);
                let _next_constraint =
                    push_movement_constraint(movement_constraint_with_min_col(next_min_col));
                let residue = inventory_depart_bad_count(&bs.state, next_min_col);
                let empty4 =
                    inventory_try_depart_empty4_static(&bs, &input, &next_target, next_min_col);
                let depart = inventory_depart_from_parked(
                    &bs,
                    &input,
                    &next_target,
                    &timer,
                    next_min_col,
                );
                println!(
                    "next seg={} launch_ready={} residue={} empty4_hit={} depart_hit={} head={:?} len={}",
                    seg_idx + 1,
                    inventory_is_launch_ready(&bs.state, next_min_col),
                    residue,
                    empty4.is_some(),
                    depart.is_some(),
                    rc_of(bs.state.head(), input.n),
                    bs.state.len()
                );
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: probe_inventory_launch <input.txt>");
        std::process::exit(1);
    }
    solver::run(&args[1]);
}
