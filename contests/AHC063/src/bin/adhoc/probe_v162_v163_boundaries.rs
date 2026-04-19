// probe_v162_v163_boundaries.rs

mod v162 {
    #![allow(dead_code)]
    include!("../v162_no_parked_fallback.rs");

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

    fn state_sig(st: &State) -> (usize, usize, Vec<u8>) {
        let head = st.head() as usize;
        let n = st.n;
        let colors = st.colors.as_slice()[..st.len().min(12)].to_vec();
        (head / n, head % n, colors)
    }

    pub fn phase_states(input_path: &str) -> (Vec<(usize, (usize, usize, Vec<u8>))>, f64, bool) {
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
        let mut out = Vec::new();
        let mut bs = BeamState {
            state: State::initial(&input),
            ops: Ops::new(),
        };
        for seg_idx in 0..stock_cnt {
            match inventory_run_segment_phase(&ctx, &bs, seg_idx) {
                InventoryPhaseOutcome::Completed(next_bs) => {
                    bs = next_bs;
                    out.push((bs.ops.len(), state_sig(&bs.state)));
                }
                InventoryPhaseOutcome::BuiltOnly(next_bs) => {
                    out.push((next_bs.ops.len(), state_sig(&next_bs.state)));
                    break;
                }
                InventoryPhaseOutcome::PreparedOnly(next_bs) => {
                    out.push((next_bs.ops.len(), state_sig(&next_bs.state)));
                    break;
                }
                InventoryPhaseOutcome::Failed => {
                    out.push((bs.ops.len(), state_sig(&bs.state)));
                    break;
                }
            }
        }
        let rem = timer.exact_remaining_sec();
        let tail_ok = inventory_build_tail_exact(&ctx, &bs).is_some();
        (out, rem, tail_ok)
    }
}

mod v163 {
    #![allow(dead_code)]
    include!("../v163_phase_rollback.rs");

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

    fn state_sig(st: &State) -> (usize, usize, Vec<u8>) {
        let head = st.head() as usize;
        let n = st.n;
        let colors = st.colors.as_slice()[..st.len().min(12)].to_vec();
        (head / n, head % n, colors)
    }

    pub fn phase_states(input_path: &str) -> (Vec<(usize, (usize, usize, Vec<u8>))>, f64, bool) {
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
        let mut out = Vec::new();
        let mut bs = BeamState {
            state: State::initial(&input),
            ops: Ops::new(),
        };
        for seg_idx in 0..stock_cnt {
            match inventory_run_segment_phase(&ctx, &bs, seg_idx) {
                InventoryPhaseOutcome::Completed(next_bs) => {
                    bs = next_bs;
                    out.push((bs.ops.len(), state_sig(&bs.state)));
                }
                InventoryPhaseOutcome::BuiltOnly(next_bs) => {
                    out.push((next_bs.ops.len(), state_sig(&next_bs.state)));
                    break;
                }
                InventoryPhaseOutcome::PreparedOnly(next_bs) => {
                    out.push((next_bs.ops.len(), state_sig(&next_bs.state)));
                    break;
                }
                InventoryPhaseOutcome::Failed => {
                    out.push((bs.ops.len(), state_sig(&bs.state)));
                    break;
                }
            }
        }
        let rem = timer.exact_remaining_sec();
        let tail_ok = inventory_build_tail_exact(&ctx, &bs).is_some();
        (out, rem, tail_ok)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: probe_v162_v163_boundaries <input>");
        std::process::exit(1);
    }
    let input = &args[1];
    let (a, rem_a, tail_ok_a) = v162::phase_states(input);
    let (b, rem_b, tail_ok_b) = v163::phase_states(input);
    println!(
        "v162 {:?}",
        a.iter().map(|(ops, _)| *ops).collect::<Vec<_>>()
    );
    println!(
        "v163 {:?}",
        b.iter().map(|(ops, _)| *ops).collect::<Vec<_>>()
    );
    println!("v162 rem={:.6} tail_ok={}", rem_a, tail_ok_a);
    println!("v163 rem={:.6} tail_ok={}", rem_b, tail_ok_b);
    for (i, ((ops_a, st_a), (ops_b, st_b))) in a.iter().zip(b.iter()).enumerate() {
        println!(
            "phase {} ops_equal={} state_equal={} sig_a={:?} sig_b={:?}",
            i,
            ops_a == ops_b,
            st_a == st_b,
            st_a,
            st_b
        );
    }
}
