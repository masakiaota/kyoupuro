// check_v163_phase_drop.rs

mod solver {
    #![allow(dead_code)]
    include!("../v163_phase_rollback.rs");

    fn parse_dir(ch: u8) -> Option<usize> {
        match ch {
            b'U' => Some(0),
            b'D' => Some(1),
            b'L' => Some(2),
            b'R' => Some(3),
            _ => None,
        }
    }

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

    fn phase_start_turn(input: &Input, phase_seg_idx: usize) -> usize {
        let timer = TimeKeeper::new(TIME_LIMIT_SEC, 8);
        let seg_len = 2 * (input.n - 2);
        let stock_cnt = (input.m - 5) / seg_len;
        let ctx = InventoryCtx {
            input,
            timer: &timer,
            seg_len,
            stock_cnt,
        };
        let mut bs = BeamState {
            state: State::initial(input),
            ops: Ops::new(),
        };
        for seg_idx in 0..phase_seg_idx {
            bs = match inventory_run_segment_phase(&ctx, &bs, seg_idx) {
                InventoryPhaseOutcome::Completed(next_bs) => next_bs,
                _ => panic!("phase {seg_idx} did not complete while reconstructing boundaries"),
            };
        }
        bs.ops.len()
    }

    pub fn analyze(input_path: &str, output_path: &str, phase_seg_idx: usize) {
        let input = parse_input_from_str(&std::fs::read_to_string(input_path).unwrap());
        let output = std::fs::read_to_string(output_path).unwrap();
        let phase_start = phase_start_turn(&input, phase_seg_idx);
        let min_allowed_col = 2 * phase_seg_idx;

        let mut st = State::initial(&input);
        let mut dropped = DroppedBuf::new();
        for (turn, line) in output.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let dir = parse_dir(line.as_bytes()[0]).unwrap();
            let in_phase = phase_start < turn + 1;
            if in_phase {
                let _constraint =
                    push_movement_constraint(movement_constraint_with_min_col(min_allowed_col));
                let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped);
                if bite_idx.is_some() {
                    let bad: Vec<_> = dropped
                        .as_slice()
                        .iter()
                        .filter_map(|ent| {
                            let (r, c) = rc_of(ent.cell, input.n);
                            (c < min_allowed_col).then_some((r, c, ent.color))
                        })
                        .collect();
                    if !bad.is_empty() {
                        println!(
                            "bad_drop turn={} phase_seg={} min_col={} head={:?} bad={:?}",
                            turn + 1,
                            phase_seg_idx,
                            min_allowed_col,
                            rc_of(ns.head(), input.n),
                            bad
                        );
                    }
                }
                st = ns;
            } else {
                let (ns, _, _) = step_with_dropped(&st, dir, &mut dropped);
                st = ns;
            }
        }
        println!("phase_start={}", phase_start);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: check_v163_phase_drop <input> <output> <seg_idx>");
        std::process::exit(1);
    }
    let seg_idx: usize = args[3].parse().unwrap();
    solver::analyze(&args[1], &args[2], seg_idx);
}
