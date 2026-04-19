// probe_v148_departure_case.rs

mod solver {
    #![allow(dead_code)]
    include!("../v148_prefix_fast.rs");

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

    fn parse_output_dirs(s: &str) -> Vec<Dir> {
        s.lines()
            .filter_map(|line| match line.as_bytes().first().copied() {
                Some(b'U') => Some(0),
                Some(b'D') => Some(1),
                Some(b'L') => Some(2),
                Some(b'R') => Some(3),
                _ => None,
            })
            .collect()
    }

    pub fn probe(input_path: &str, output_path: &str, seg_idx: usize) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let dirs = parse_output_dirs(&std::fs::read_to_string(output_path).unwrap());

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
        for done in 0..seg_idx {
            bs = match inventory_run_segment_phase(&ctx, &bs, done) {
                InventoryPhaseOutcome::Completed(next_bs) => next_bs,
                _ => panic!("failed before seg_idx={seg_idx}, done={done}"),
            };
        }

        let target_colors = inventory_segment_target(&input, seg_idx, seg_len);
        let phases_left = stock_cnt - seg_idx + 1;
        let min_allowed_col = 2 * seg_idx;
        let _goal_hash = push_goal_prefix_hash(&target_colors);
        let _constraint = push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));
        let direct = inventory_try_direct_build_from_current(&ctx, &bs, &InventoryTarget {
            colors: &target_colors,
            protect_len: target_colors.len(),
        }, phases_left);
        let legacy = inventory_build_target_exact_legacy_parked(
            &ctx,
            &bs,
            &InventoryTarget {
                colors: &target_colors,
                protect_len: target_colors.len(),
            },
            phases_left,
        );
        let transportable = inventory_try_transportable_build_from_parked(
            &ctx,
            &bs,
            &InventoryTarget {
                colors: &target_colors,
                protect_len: target_colors.len(),
            },
            phases_left,
            seg_idx,
        );
        let prepared = inventory_prepare_build_start(&ctx, &bs, &target_colors, phases_left, min_allowed_col)
            .expect("prepare failed");

        let start_turn = bs.ops.len();
        let depart_len = prepared.ops.len().saturating_sub(bs.ops.len());
        let actual: String = dirs[start_turn..(start_turn + depart_len).min(dirs.len())]
            .iter()
            .map(|&d| DIR_CHARS[d as usize])
            .collect();
        let expected: String = prepared.ops[bs.ops.len()..]
            .iter()
            .map(|&d| DIR_CHARS[d as usize])
            .collect();

        println!(
            "seg_idx={} start_turn={} depart_len={} actual={} expected={} match={}",
            seg_idx,
            start_turn,
            depart_len,
            actual,
            expected,
            actual == expected
        );

        if let Some(direct) = direct {
            let direct_len = direct.ops.len().saturating_sub(bs.ops.len());
            let direct_actual: String = dirs[start_turn..(start_turn + direct_len).min(dirs.len())]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            let direct_expected: String = direct.ops[bs.ops.len()..]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            println!(
                "direct_len={} direct_actual={} direct_expected={} direct_match={}",
                direct_len,
                direct_actual,
                direct_expected,
                direct_actual == direct_expected
            );

            let mut st = bs.state.clone();
            let mut dropped = DroppedBuf::new();
            for (i, &dir) in direct.ops[bs.ops.len()..].iter().enumerate().take(30) {
                let (ns, _, bite_idx) = step_with_dropped(&st, dir as usize, &mut dropped);
                if bite_idx.is_some() {
                    let bad: Vec<_> = dropped
                        .as_slice()
                        .iter()
                        .filter_map(|ent| {
                            let (r, c) = rc_of(ent.cell, input.n);
                            (c < min_allowed_col).then_some((r, c, ent.color))
                        })
                        .collect();
                    println!(
                        "direct_bite rel_turn={} abs_turn={} head={:?} bad={:?}",
                        i + 1,
                        start_turn + i + 1,
                        rc_of(ns.head(), input.n),
                        bad
                    );
                }
                st = ns;
            }
        } else {
            println!("direct_build=None");
        }

        if let InventoryBuildOutcome::Built(legacy) = legacy {
            let legacy_len = legacy.ops.len().saturating_sub(bs.ops.len()).min(20);
            let legacy_actual: String = dirs[start_turn..(start_turn + legacy_len).min(dirs.len())]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            let legacy_expected: String = legacy.ops[bs.ops.len()..(bs.ops.len() + legacy_len)]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            println!(
                "legacy_prefix actual={} expected={} match={}",
                legacy_actual,
                legacy_expected,
                legacy_actual == legacy_expected
            );
            let mut st = bs.state.clone();
            let mut dropped = DroppedBuf::new();
            for (i, &dir) in legacy.ops[bs.ops.len()..].iter().enumerate().take(20) {
                let (ns, _, bite_idx) = step_with_dropped(&st, dir as usize, &mut dropped);
                if bite_idx.is_some() {
                    let bad: Vec<_> = dropped
                        .as_slice()
                        .iter()
                        .filter_map(|ent| {
                            let (r, c) = rc_of(ent.cell, input.n);
                            (c < min_allowed_col).then_some((r, c, ent.color))
                        })
                        .collect();
                    println!(
                        "legacy_bite rel_turn={} abs_turn={} head={:?} bad={:?}",
                        i + 1,
                        start_turn + i + 1,
                        rc_of(ns.head(), input.n),
                        bad
                    );
                }
                st = ns;
            }
        } else {
            println!("legacy_build!=Built");
        }

        if let Some(transportable) = transportable {
            let tr_len = transportable.ops.len().saturating_sub(bs.ops.len()).min(20);
            let tr_actual: String = dirs[start_turn..(start_turn + tr_len).min(dirs.len())]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            let tr_expected: String = transportable.ops[bs.ops.len()..(bs.ops.len() + tr_len)]
                .iter()
                .map(|&d| DIR_CHARS[d as usize])
                .collect();
            println!(
                "transportable_prefix actual={} expected={} match={}",
                tr_actual,
                tr_expected,
                tr_actual == tr_expected
            );
        } else {
            println!("transportable=None");
        }

        let mut st = bs.state.clone();
        let mut dropped = DroppedBuf::new();
        for (i, &dir) in prepared.ops[bs.ops.len()..].iter().enumerate() {
            let (ns, _, bite_idx) = step_with_dropped(&st, dir as usize, &mut dropped);
            if bite_idx.is_some() {
                let bad: Vec<_> = dropped
                    .as_slice()
                    .iter()
                    .filter_map(|ent| {
                        let (r, c) = rc_of(ent.cell, input.n);
                        (c < min_allowed_col).then_some((r, c, ent.color))
                    })
                    .collect();
                println!(
                    "prepare_bite rel_turn={} abs_turn={} head={:?} bad={:?}",
                    i + 1,
                    start_turn + i + 1,
                    rc_of(ns.head(), input.n),
                    bad
                );
            }
            st = ns;
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: probe_v148_departure_case <input> <output> <seg_idx>");
        std::process::exit(1);
    }
    let seg_idx: usize = args[3].parse().unwrap();
    solver::probe(&args[1], &args[2], seg_idx);
}
