// analyze_mcts_leaf_modes.rs
use std::env;
use std::fs;
use std::time::Instant;

const N: usize = 10;
const NN: usize = N * N;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const TIME_LIMIT_SEC: f64 = 1.90;
const SAFETY_SEC: f64 = 0.05;
const MAX_ROLLOUT_HORIZON: usize = 8;
const MCTS_EXPAND_THRESHOLD: u32 = 10;
const MCTS_UCB_C: f64 = 0.65;

#[derive(Clone, Copy)]
enum LeafMode {
    Proxy,
    GreedyToEndFast,
}

impl LeafMode {
    fn name(self) -> &'static str {
        match self {
            Self::Proxy => "v104_proxy",
            Self::GreedyToEndFast => "v114_greedy_to_end_fast",
        }
    }
}

struct Timer {
    start: Instant,
    limit_sec: f64,
    safety_sec: f64,
}

impl Timer {
    fn new(limit_sec: f64, safety_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            limit_sec,
            safety_sec,
        }
    }

    fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn soft_limit(&self) -> f64 {
        self.limit_sec - self.safety_sec
    }

    fn remaining(&self) -> f64 {
        (self.soft_limit() - self.elapsed()).max(0.0)
    }

    fn is_time_up(&self) -> bool {
        self.elapsed() >= self.soft_limit()
    }

    fn local_deadline(&self, remaining_decisions: usize) -> f64 {
        if remaining_decisions <= 1 {
            return self.soft_limit();
        }
        let base = self.remaining() / remaining_decisions as f64;
        let factor = if remaining_decisions >= 60 {
            0.60
        } else if remaining_decisions >= 25 {
            0.85
        } else {
            1.15
        };
        (self.elapsed() + base * factor + 0.0003).min(self.soft_limit())
    }
}

#[derive(Clone, Copy)]
struct Board {
    cells: [u8; NN],
}

impl Board {
    fn new() -> Self {
        Self { cells: [0; NN] }
    }

    fn place_pth_empty(&mut self, p: usize, flavor: u8) {
        let mut kth = 0usize;
        for idx in 0..NN {
            if self.cells[idx] == 0 {
                kth += 1;
                if kth == p {
                    self.cells[idx] = flavor;
                    return;
                }
            }
        }
        unreachable!("invalid p: {p}");
    }

    fn tilted(&self, dir: u8) -> Self {
        let mut next = [0u8; NN];
        match dir {
            b'L' => {
                for r in 0..N {
                    let mut write = 0usize;
                    for c in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[r * N + write] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    let mut write = N;
                    for c in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[r * N + write] = v;
                        }
                    }
                }
            }
            b'F' => {
                for c in 0..N {
                    let mut write = 0usize;
                    for r in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[write * N + c] = v;
                            write += 1;
                        }
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    let mut write = N;
                    for r in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[write * N + c] = v;
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
        Self { cells: next }
    }

    fn tilted_with_counts(&self, dir: u8) -> (Self, [u8; N]) {
        let mut next = [0u8; NN];
        let mut counts = [0u8; N];
        match dir {
            b'L' => {
                for r in 0..N {
                    let mut write = 0usize;
                    for c in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[r * N + write] = v;
                            write += 1;
                        }
                    }
                    counts[r] = write as u8;
                }
            }
            b'R' => {
                for r in 0..N {
                    let mut write = N;
                    for c in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[r * N + write] = v;
                        }
                    }
                    counts[r] = (N - write) as u8;
                }
            }
            b'F' => {
                for c in 0..N {
                    let mut write = 0usize;
                    for r in 0..N {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            next[write * N + c] = v;
                            write += 1;
                        }
                    }
                    counts[c] = write as u8;
                }
            }
            b'B' => {
                for c in 0..N {
                    let mut write = N;
                    for r in (0..N).rev() {
                        let v = self.cells[r * N + c];
                        if v != 0 {
                            write -= 1;
                            next[write * N + c] = v;
                        }
                    }
                    counts[c] = (N - write) as u8;
                }
            }
            _ => unreachable!(),
        }
        (Self { cells: next }, counts)
    }

    fn component_sq_sum(&self) -> i64 {
        let mut seen = [false; NN];
        let mut stack = [0usize; NN];
        let mut comp_sq = 0i64;
        for idx in 0..NN {
            let color = self.cells[idx];
            if color == 0 || seen[idx] {
                continue;
            }
            seen[idx] = true;
            let mut top = 0usize;
            stack[top] = idx;
            top += 1;
            let mut size = 0i64;
            while top > 0 {
                top -= 1;
                let cur = stack[top];
                size += 1;
                let r = cur / N;
                let c = cur % N;
                if r > 0 {
                    let next = cur - N;
                    if !seen[next] && self.cells[next] == color {
                        seen[next] = true;
                        stack[top] = next;
                        top += 1;
                    }
                }
                if r + 1 < N {
                    let next = cur + N;
                    if !seen[next] && self.cells[next] == color {
                        seen[next] = true;
                        stack[top] = next;
                        top += 1;
                    }
                }
                if c > 0 {
                    let next = cur - 1;
                    if !seen[next] && self.cells[next] == color {
                        seen[next] = true;
                        stack[top] = next;
                        top += 1;
                    }
                }
                if c + 1 < N {
                    let next = cur + 1;
                    if !seen[next] && self.cells[next] == color {
                        seen[next] = true;
                        stack[top] = next;
                        top += 1;
                    }
                }
            }
            comp_sq += size * size;
        }
        comp_sq
    }

    fn next_surface_gain(&self, dir: u8, next_flavor: u8) -> i64 {
        let mut gain = 0i64;
        match dir {
            b'F' => {
                for c in 0..N {
                    for r in 0..N {
                        let idx = r * N + c;
                        if self.cells[idx] == 0 {
                            break;
                        }
                        if self.cells[idx] == next_flavor {
                            gain += 1;
                        }
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    for r in (0..N).rev() {
                        let idx = r * N + c;
                        if self.cells[idx] == 0 {
                            break;
                        }
                        if self.cells[idx] == next_flavor {
                            gain += 1;
                        }
                    }
                }
            }
            b'L' => {
                for r in 0..N {
                    for c in 0..N {
                        let idx = r * N + c;
                        if self.cells[idx] == 0 {
                            break;
                        }
                        if self.cells[idx] == next_flavor {
                            gain += 1;
                        }
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    for c in (0..N).rev() {
                        let idx = r * N + c;
                        if self.cells[idx] == 0 {
                            break;
                        }
                        if self.cells[idx] == next_flavor {
                            gain += 1;
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
        gain
    }

    fn next_surface_gain_with_counts(&self, dir: u8, next_flavor: u8, counts: &[u8; N]) -> i64 {
        let mut comp_ids = [-1i16; NN];
        let mut comp_sizes = [0usize; NN];
        let mut stack = [0usize; NN];
        let mut members = [0usize; NN];
        let mut comp_count = 0i16;

        let component_size = |start: usize,
                              board: &Board,
                              comp_ids: &mut [i16; NN],
                              comp_sizes: &mut [usize; NN],
                              stack: &mut [usize; NN],
                              members: &mut [usize; NN],
                              comp_count: &mut i16| {
            let id = comp_ids[start];
            if id >= 0 {
                return comp_sizes[id as usize];
            }
            let new_id = *comp_count as usize;
            *comp_count += 1;
            let mut top = 0usize;
            let mut len = 0usize;
            stack[top] = start;
            top += 1;
            comp_ids[start] = new_id as i16;
            while top > 0 {
                top -= 1;
                let cur = stack[top];
                members[len] = cur;
                len += 1;
                let r = cur / N;
                let c = cur % N;
                if r > 0 {
                    let ni = cur - N;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if r + 1 < N {
                    let ni = cur + N;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c > 0 {
                    let ni = cur - 1;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
                if c + 1 < N {
                    let ni = cur + 1;
                    if board.cells[ni] == next_flavor && comp_ids[ni] < 0 {
                        comp_ids[ni] = new_id as i16;
                        stack[top] = ni;
                        top += 1;
                    }
                }
            }
            comp_sizes[new_id] = len;
            for &idx in &members[..len] {
                comp_ids[idx] = new_id as i16;
            }
            len
        };

        let mut gain = 0i64;
        match dir {
            b'L' => {
                for r in 0..N {
                    let occupied = counts[r] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + occupied - 1;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'R' => {
                for r in 0..N {
                    let occupied = counts[r] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = r * N + (N - occupied);
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'F' => {
                for c in 0..N {
                    let occupied = counts[c] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (occupied - 1) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            b'B' => {
                for c in 0..N {
                    let occupied = counts[c] as usize;
                    let empty = N - occupied;
                    if occupied == 0 || empty == 0 {
                        continue;
                    }
                    let idx = (N - occupied) * N + c;
                    if self.cells[idx] == next_flavor {
                        let size = component_size(
                            idx,
                            self,
                            &mut comp_ids,
                            &mut comp_sizes,
                            &mut stack,
                            &mut members,
                            &mut comp_count,
                        ) as i64;
                        gain += empty as i64 * (2 * size + 1);
                    }
                }
            }
            _ => unreachable!(),
        }
        gain
    }
}

struct EvalFeatures {
    comp_sq: i32,
    same_adj: i32,
    diff_adj: i32,
}

fn evaluate_features(board: &Board) -> EvalFeatures {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i32;
    let mut same_adj = 0i32;
    let mut diff_adj = 0i32;

    for idx in 0..NN {
        let color = board.cells[idx];
        if color == 0 {
            continue;
        }
        let r = idx / N;
        let c = idx % N;
        if c + 1 < N {
            let right = board.cells[idx + 1];
            if right != 0 {
                if right == color {
                    same_adj += 1;
                } else {
                    diff_adj += 1;
                }
            }
        }
        if r + 1 < N {
            let down = board.cells[idx + N];
            if down != 0 {
                if down == color {
                    same_adj += 1;
                } else {
                    diff_adj += 1;
                }
            }
        }
        if seen[idx] {
            continue;
        }
        seen[idx] = true;
        let mut top = 0usize;
        stack[top] = idx;
        top += 1;
        let mut size = 0i32;
        while top > 0 {
            top -= 1;
            let cur = stack[top];
            size += 1;
            let cr = cur / N;
            let cc = cur % N;
            if cr > 0 {
                let next = cur - N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cr + 1 < N {
                let next = cur + N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cc > 0 {
                let next = cur - 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if cc + 1 < N {
                let next = cur + 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
        }
        comp_sq += size * size;
    }

    EvalFeatures {
        comp_sq,
        same_adj,
        diff_adj,
    }
}

fn static_value(board: &Board, _placed: usize) -> i64 {
    let f = evaluate_features(board);
    i64::from(f.comp_sq) * 14 + i64::from(f.same_adj) * 18 - i64::from(f.diff_adj) * 10
}

fn final_raw_score(board: &Board) -> i64 {
    board.component_sq_sum()
}

fn rollout_confidence(best_value: f64, second_value: f64) -> f64 {
    let gap = (best_value - second_value).max(0.0);
    let scale = best_value.abs().max(second_value.abs()).max(1.0);
    gap / scale
}

fn should_skip_rollout(placed: usize, remaining_empty: usize, confidence: f64) -> bool {
    let density = placed as f64 / NN as f64;
    density <= 0.06
        || (density <= 0.12 && confidence >= 0.10)
        || (remaining_empty >= 80 && confidence >= 0.18)
}

fn shaped_deadline(
    timer: &Timer,
    base_deadline: f64,
    placed: usize,
    remaining_empty: usize,
    confidence: f64,
) -> f64 {
    let density = placed as f64 / NN as f64;
    let phase_factor: f64 = if density < 0.15 {
        0.60
    } else if density < 0.55 {
        1.00
    } else {
        1.15
    };
    let ambiguity_factor: f64 = if confidence < 0.025 {
        1.45
    } else if confidence < 0.050 {
        1.20
    } else if confidence < 0.100 {
        0.95
    } else {
        0.65
    };
    let exactness_factor: f64 = if remaining_empty <= 8 { 1.10 } else { 1.00 };
    let factor = (phase_factor * ambiguity_factor * exactness_factor).clamp(0.35_f64, 1.70_f64);
    let base_budget = (base_deadline - timer.elapsed()).max(0.0);
    (timer.elapsed() + base_budget * factor).min(timer.soft_limit())
}

fn choose_rollout_horizon(timer: &Timer, remaining_decisions: usize) -> usize {
    let remaining = timer.remaining();
    if remaining < 0.010 {
        0
    } else if remaining < 0.030 {
        2.min(remaining_decisions)
    } else if remaining < 0.080 {
        3.min(remaining_decisions)
    } else if remaining_decisions >= 50 {
        4
    } else if remaining_decisions >= 20 {
        6.min(remaining_decisions)
    } else {
        MAX_ROLLOUT_HORIZON.min(remaining_decisions)
    }
}

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        };
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.state = x;
        x
    }
}

fn sample_random_place(board: &mut Board, next_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) {
    let remaining_empty = NN - next_turn;
    let p = (rng.next_u64() as usize % remaining_empty) + 1;
    board.place_pth_empty(p, fs[next_turn]);
}

fn random_dir(rng: &mut XorShift64) -> u8 {
    DIRS[(rng.next_u64() as usize) & 3]
}

fn choose_greedy_direction(board: &Board, next_flavor: Option<u8>) -> u8 {
    let mut best_dir = DIRS[0];
    let mut best_surface = -1i64;
    let mut best_comp_sq = -1i64;
    for dir in DIRS {
        let tilted = board.tilted(dir);
        let comp_sq = tilted.component_sq_sum();
        let surface = match next_flavor {
            Some(flavor) => tilted.next_surface_gain(dir, flavor),
            None => -1,
        };
        if surface > best_surface || (surface == best_surface && comp_sq > best_comp_sq) {
            best_surface = surface;
            best_comp_sq = comp_sq;
            best_dir = dir;
        }
    }
    best_dir
}

fn choose_greedy_move_fast(board: &Board, next_flavor: Option<u8>) -> (u8, Board) {
    let mut tilted_boards = [Board::new(); 4];
    match next_flavor {
        Some(flavor) => {
            let mut best_surface = i64::MIN;
            let mut best_idx = 0usize;
            let mut tied = [false; 4];
            let mut tied_count = 0usize;
            for (idx, dir) in DIRS.into_iter().enumerate() {
                let (tilted, counts) = board.tilted_with_counts(dir);
                let surface = tilted.next_surface_gain_with_counts(dir, flavor, &counts);
                tilted_boards[idx] = tilted;
                if surface > best_surface {
                    best_surface = surface;
                    best_idx = idx;
                    tied = [false; 4];
                    tied[idx] = true;
                    tied_count = 1;
                } else if surface == best_surface {
                    tied[idx] = true;
                    tied_count += 1;
                }
            }
            if tied_count == 1 {
                return (DIRS[best_idx], tilted_boards[best_idx]);
            }
            let mut best_comp_sq = i64::MIN;
            for idx in 0..4 {
                if !tied[idx] {
                    continue;
                }
                let comp_sq = tilted_boards[idx].component_sq_sum();
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (DIRS[best_idx], tilted_boards[best_idx])
        }
        None => {
            let mut best_idx = 0usize;
            let mut best_comp_sq = i64::MIN;
            for (idx, dir) in DIRS.into_iter().enumerate() {
                let tilted = board.tilted(dir);
                let comp_sq = tilted.component_sq_sum();
                tilted_boards[idx] = tilted;
                if comp_sq > best_comp_sq {
                    best_comp_sq = comp_sq;
                    best_idx = idx;
                }
            }
            (DIRS[best_idx], tilted_boards[best_idx])
        }
    }
}

fn replay_prefix(
    root_board: &Board,
    root_turn: usize,
    actions: &[u8],
    fs: &[u8; NN],
    rng: &mut XorShift64,
) -> (Board, usize, bool) {
    let mut board = *root_board;
    let mut turn = root_turn;
    for &action in actions {
        board = board.tilted(action);
        if turn == NN - 1 {
            return (board, turn, true);
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return (board, turn, true);
        }
    }
    (board, turn, false)
}

fn rollout_value_from_decision_state(
    board: &Board,
    current_turn: usize,
    fs: &[u8; NN],
    rng: &mut XorShift64,
    horizon: usize,
) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    let steps = horizon.min(NN - 1 - turn);
    for _ in 0..steps {
        board = board.tilted(random_dir(rng));
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
    }
    static_value(&board, turn + 1)
}

fn greedy_rollout_to_end(board: &Board, current_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    while turn < NN - 1 {
        let dir = choose_greedy_direction(&board, Some(fs[turn + 1]));
        board = board.tilted(dir);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
    }
    final_raw_score(&board) * 1000
}

fn greedy_rollout_to_end_fast(board: &Board, current_turn: usize, fs: &[u8; NN], rng: &mut XorShift64) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    while turn < NN - 1 {
        let (_, next_board) = choose_greedy_move_fast(&board, Some(fs[turn + 1]));
        board = next_board;
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
    }
    final_raw_score(&board) * 1000
}

fn mixed_rollout_to_end(
    board: &Board,
    current_turn: usize,
    fs: &[u8; NN],
    rng: &mut XorShift64,
    prefix_depth: usize,
    action_seq_depth: usize,
) -> i64 {
    let mut board = *board;
    let mut turn = current_turn;
    let mut depth = prefix_depth;
    while turn < NN - 1 {
        let dir = if depth < action_seq_depth {
            depth += 1;
            random_dir(rng)
        } else {
            choose_greedy_direction(&board, Some(fs[turn + 1]))
        };
        board = board.tilted(dir);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        turn += 1;
        sample_random_place(&mut board, turn, fs, rng);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
    }
    final_raw_score(&board) * 1000
}

#[derive(Default, Clone)]
struct Stats {
    mcts_iterations: u64,
    leaf_evals: u64,
    expansions: u64,
    total_root_visits: [u64; 4],
    total_root_w: [f64; 4],
    turn_logs: Vec<TurnLog>,
}

#[derive(Clone)]
struct TurnLog {
    turn: usize,
    chosen: u8,
    iterations: u64,
    leaf_evals: u64,
    expansions: u64,
    root_visits: [u64; 4],
    elapsed_ms: f64,
}

#[derive(Clone)]
struct Node {
    actions: Vec<u8>,
    w: f64,
    n: u32,
    child_nodes: Vec<Node>,
}

impl Node {
    fn new(actions: Vec<u8>) -> Self {
        Self {
            actions,
            w: 0.0,
            n: 0,
            child_nodes: Vec::new(),
        }
    }

    fn expand(&mut self, stats: &mut Stats) {
        if !self.child_nodes.is_empty() {
            return;
        }
        for &dir in &DIRS {
            let mut next_actions = self.actions.clone();
            next_actions.push(dir);
            self.child_nodes.push(Node::new(next_actions));
        }
        stats.expansions += 1;
    }

    fn next_child_index(&self, rng: &mut XorShift64) -> usize {
        let mut unvisited = [0usize; 4];
        let mut len = 0usize;
        for (idx, child) in self.child_nodes.iter().enumerate() {
            if child.n == 0 {
                unvisited[len] = idx;
                len += 1;
            }
        }
        if len > 0 {
            return unvisited[(rng.next_u64() as usize) % len];
        }

        let total_n = self.child_nodes.iter().map(|c| c.n as f64).sum::<f64>().max(1.0);
        let scale = self
            .child_nodes
            .iter()
            .map(|c| (c.w / c.n as f64).abs())
            .fold(1.0_f64, f64::max);
        let mut best_idx = 0usize;
        let mut best_ucb = f64::NEG_INFINITY;
        for (idx, child) in self.child_nodes.iter().enumerate() {
            let mean = child.w / child.n as f64;
            let bonus = scale * MCTS_UCB_C * ((total_n.ln()) / child.n as f64).sqrt();
            let ucb = mean + bonus;
            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = idx;
            }
        }
        best_idx
    }

    fn leaf_value(
        &self,
        root_board: &Board,
        root_turn: usize,
        fs: &[u8; NN],
        rng: &mut XorShift64,
        horizon: usize,
        mode: LeafMode,
        stats: &mut Stats,
    ) -> f64 {
        stats.leaf_evals += 1;
        let (board, turn, terminal) = replay_prefix(root_board, root_turn, &self.actions, fs, rng);
        if terminal {
            return final_raw_score(&board) as f64 * 1000.0;
        }
        match mode {
            LeafMode::Proxy => rollout_value_from_decision_state(&board, turn, fs, rng, horizon) as f64,
            LeafMode::GreedyToEndFast => greedy_rollout_to_end_fast(&board, turn, fs, rng) as f64,
        }
    }

    fn evaluate(
        &mut self,
        root_board: &Board,
        root_turn: usize,
        fs: &[u8; NN],
        rng: &mut XorShift64,
        max_depth: usize,
        horizon: usize,
        mode: LeafMode,
        stats: &mut Stats,
    ) -> f64 {
        if self.child_nodes.is_empty() {
            let value = self.leaf_value(root_board, root_turn, fs, rng, horizon, mode, stats);
            self.w += value;
            self.n += 1;
            if self.actions.len() < max_depth && self.n == MCTS_EXPAND_THRESHOLD {
                self.expand(stats);
            }
            return value;
        }

        let idx = self.next_child_index(rng);
        let value = self.child_nodes[idx].evaluate(
            root_board, root_turn, fs, rng, max_depth, horizon, mode, stats,
        );
        self.w += value;
        self.n += 1;
        value
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    dir: u8,
    board: Board,
    value: f64,
}

fn exact_expected_value(board: &Board, next_turn: usize, fs: &[u8; NN]) -> f64 {
    let remaining_empty = NN - next_turn;
    if remaining_empty == 0 {
        return final_raw_score(board) as f64 * 1000.0;
    }
    let mut sum = 0.0;
    for p in 1..=remaining_empty {
        let mut placed_board = *board;
        placed_board.place_pth_empty(p, fs[next_turn]);
        if next_turn == NN - 1 {
            sum += final_raw_score(&placed_board) as f64 * 1000.0;
            continue;
        }
        let mut best = f64::NEG_INFINITY;
        for &dir in &DIRS {
            let tilted = placed_board.tilted(dir);
            let value = exact_expected_value(&tilted, next_turn + 1, fs);
            if value > best {
                best = value;
            }
        }
        sum += best;
    }
    sum / remaining_empty as f64
}

fn best_root_dir(root: &Node, fallback_dir: u8) -> u8 {
    if root.child_nodes.is_empty() {
        return fallback_dir;
    }
    let mut best_idx = 0usize;
    let mut best_n = 0u32;
    let mut best_w = f64::NEG_INFINITY;
    for (idx, child) in root.child_nodes.iter().enumerate() {
        if child.n > best_n || (child.n == best_n && child.w > best_w) {
            best_n = child.n;
            best_w = child.w;
            best_idx = idx;
        }
    }
    DIRS[best_idx]
}

fn dir_index(dir: u8) -> usize {
    match dir {
        b'F' => 0,
        b'B' => 1,
        b'L' => 2,
        b'R' => 3,
        _ => unreachable!(),
    }
}

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    timer: &Timer,
    rng: &mut XorShift64,
    mode: LeafMode,
    stats: &mut Stats,
) -> u8 {
    let turn_start_elapsed = timer.elapsed();
    let before_iters = stats.mcts_iterations;
    let before_leafs = stats.leaf_evals;
    let before_expands = stats.expansions;

    let placed = turn + 1;
    let remaining_decisions = NN - 1 - turn;
    let remaining_empty = NN - placed;
    let base_deadline = timer.local_deadline(remaining_decisions.max(1));

    let mut candidates = [Candidate {
        dir: DIRS[0],
        board: Board::new(),
        value: 0.0,
    }; 4];
    for &dir in &DIRS {
        let tilted = board_after_place.tilted(dir);
        candidates[dir_index(dir)] = Candidate {
            dir,
            board: tilted,
            value: static_value(&tilted, placed) as f64,
        };
    }

    let mut best_idx = 0usize;
    for i in 1..4 {
        if candidates[i].value > candidates[best_idx].value {
            best_idx = i;
        }
    }
    let best_value = candidates[best_idx].value;
    let mut second_value = f64::NEG_INFINITY;
    for (idx, candidate) in candidates.iter().enumerate() {
        if idx == best_idx {
            continue;
        }
        if candidate.value > second_value {
            second_value = candidate.value;
        }
    }
    let confidence = rollout_confidence(best_value, second_value);

    let mut root_visits = [0u64; 4];

    let chosen = if remaining_empty <= 4 {
        let mut exact_best_idx = 0usize;
        let mut exact_best_value = f64::NEG_INFINITY;
        for (idx, candidate) in candidates.iter().enumerate() {
            let value = exact_expected_value(&candidate.board, placed, fs);
            if value > exact_best_value {
                exact_best_value = value;
                exact_best_idx = idx;
            }
        }
        candidates[exact_best_idx].dir
    } else if should_skip_rollout(placed, remaining_empty, confidence) {
        candidates[best_idx].dir
    } else {
        let local_deadline = shaped_deadline(timer, base_deadline, placed, remaining_empty, confidence);
        let horizon = choose_rollout_horizon(timer, NN - placed).max(2);
        let mut root = Node::new(Vec::new());
        root.expand(stats);
        while !timer.is_time_up() && timer.elapsed() < local_deadline {
            let mut sample_rng = XorShift64::new(rng.next_u64());
            let _ = root.evaluate(
                board_after_place,
                turn,
                fs,
                &mut sample_rng,
                remaining_decisions,
                horizon,
                mode,
                stats,
            );
            stats.mcts_iterations += 1;
        }
        for (i, child) in root.child_nodes.iter().enumerate() {
            root_visits[i] = child.n as u64;
            stats.total_root_visits[i] += child.n as u64;
            stats.total_root_w[i] += child.w;
        }
        best_root_dir(&root, candidates[best_idx].dir)
    };

    stats.turn_logs.push(TurnLog {
        turn,
        chosen,
        iterations: stats.mcts_iterations - before_iters,
        leaf_evals: stats.leaf_evals - before_leafs,
        expansions: stats.expansions - before_expands,
        root_visits,
        elapsed_ms: (timer.elapsed() - turn_start_elapsed) * 1000.0,
    });
    chosen
}

fn run_mode(mode: LeafMode, fs_arr: &[u8; NN], ps: &[usize; NN]) -> Stats {
    let mut flavor_counts = [0usize; 4];
    for &f in fs_arr {
        flavor_counts[f as usize] += 1;
    }
    let seed = flavor_counts[1] as u64 * 1_000_003
        + flavor_counts[2] as u64 * 1_000_033
        + flavor_counts[3] as u64 * 1_000_037;
    let mut rng = XorShift64::new(seed);
    let timer = Timer::new(TIME_LIMIT_SEC, SAFETY_SEC);
    let mut stats = Stats::default();
    let mut board = Board::new();
    for turn in 0..NN {
        board.place_pth_empty(ps[turn], fs_arr[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = choose_dir(&board, turn, fs_arr, &timer, &mut rng, mode, &mut stats);
        board = board.tilted(dir);
    }
    stats
}

fn parse_case(path: &str) -> ([u8; NN], [usize; NN]) {
    let text = fs::read_to_string(path).unwrap();
    let values: Vec<usize> = text
        .split_whitespace()
        .map(|s| s.parse::<usize>().unwrap())
        .collect();
    assert_eq!(values.len(), NN * 2);
    let mut fs_arr = [0u8; NN];
    let mut ps = [0usize; NN];
    for i in 0..NN {
        fs_arr[i] = values[i] as u8;
        ps[i] = values[NN + i];
    }
    (fs_arr, ps)
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tools/in/0000.txt".to_string());
    let (fs_arr, ps) = parse_case(&path);
    for mode in [LeafMode::Proxy, LeafMode::GreedyToEndFast] {
        let stats = run_mode(mode, &fs_arr, &ps);
        println!("mode={}", mode.name());
        println!("  total_iterations={}", stats.mcts_iterations);
        println!("  total_leaf_evals={}", stats.leaf_evals);
        println!("  total_expansions={}", stats.expansions);
        println!(
            "  root_visits_total=F:{} B:{} L:{} R:{}",
            stats.total_root_visits[0],
            stats.total_root_visits[1],
            stats.total_root_visits[2],
            stats.total_root_visits[3]
        );
        let interesting = [5usize, 20, 40, 60, 80, 95];
        for &t in &interesting {
            if let Some(log) = stats.turn_logs.iter().find(|log| log.turn == t) {
                println!(
                    "  turn={:02} chosen={} iters={} leafs={} expands={} root=[{},{},{},{}] elapsed_ms={:.3}",
                    log.turn + 1,
                    log.chosen as char,
                    log.iterations,
                    log.leaf_evals,
                    log.expansions,
                    log.root_visits[0],
                    log.root_visits[1],
                    log.root_visits[2],
                    log.root_visits[3],
                    log.elapsed_ms
                );
            }
        }
    }
}
