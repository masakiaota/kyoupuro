// v007_no_loss_fast_eval.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const N: usize = 10;
const NN: usize = N * N;
const FLAVORS: usize = 3;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const TIME_LIMIT_SEC: f64 = 1.90;
const SAFETY_SEC: f64 = 0.05;
const MAX_ROLLOUT_HORIZON: usize = 8;

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
            _ => unreachable!("invalid dir"),
        }
        Self { cells: next }
    }
}

#[derive(Clone)]
struct Target {
    label: [u8; NN],
    masks: [[u64; 2]; FLAVORS + 1],
    score_bias: i64,
}

#[derive(Clone, Copy)]
struct BoardSummary {
    comp_sq: i32,
    same_adj: i32,
    diff_adj: i32,
    masks: [[u64; 2]; FLAVORS + 1],
}

#[derive(Clone, Copy)]
struct Candidate {
    dir: u8,
    board: Board,
    target_idx: usize,
    sum: f64,
    weight: f64,
}

struct Scanner<R> {
    reader: R,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn next_usize(&mut self) -> usize {
        let mut val = 0usize;
        let mut started = false;
        loop {
            let buf = self.reader.fill_buf().unwrap();
            if buf.is_empty() {
                assert!(started, "unexpected EOF while reading token");
                return val;
            }
            let mut consumed = 0usize;
            while consumed < buf.len() {
                let b = buf[consumed];
                if b.is_ascii_whitespace() {
                    consumed += 1;
                    if started {
                        self.reader.consume(consumed);
                        return val;
                    }
                } else {
                    started = true;
                    val = val * 10 + usize::from(b - b'0');
                    consumed += 1;
                }
            }
            self.reader.consume(consumed);
        }
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

fn build_corner_orders() -> [[usize; NN]; 4] {
    let mut pairs = [
        [(0i32, 0usize); NN],
        [(0i32, 0usize); NN],
        [(0i32, 0usize); NN],
        [(0i32, 0usize); NN],
    ];
    for r in 0..N {
        for c in 0..N {
            let idx = r * N + c;
            pairs[0][idx] = (((r + c) * 32 + r * 2 + c) as i32, idx);
            pairs[1][idx] = (((r + (N - 1 - c)) * 32 + r * 2 + (N - 1 - c)) as i32, idx);
            pairs[2][idx] = ((((N - 1 - r) + c) * 32 + (N - 1 - r) * 2 + c) as i32, idx);
            pairs[3][idx] = (
                ((((N - 1 - r) + (N - 1 - c)) * 32 + (N - 1 - r) * 2 + (N - 1 - c)) as i32),
                idx,
            );
        }
    }
    let mut orders = [[0usize; NN]; 4];
    for corner in 0..4 {
        pairs[corner].sort_unstable_by_key(|&(key, idx)| (key, idx));
        for i in 0..NN {
            orders[corner][i] = pairs[corner][i].1;
        }
    }
    orders
}

fn build_targets(flavor_counts: &[usize; FLAVORS + 1]) -> Vec<Target> {
    let orders = build_corner_orders();
    let anchor_sets = [
        [0usize, 1usize, 2usize],
        [0usize, 1usize, 3usize],
        [0usize, 2usize, 3usize],
        [1usize, 2usize, 3usize],
    ];
    let fill_orders = [
        [1usize, 2usize, 3usize],
        [1usize, 3usize, 2usize],
        [2usize, 1usize, 3usize],
        [2usize, 3usize, 1usize],
        [3usize, 1usize, 2usize],
        [3usize, 2usize, 1usize],
    ];
    let mut targets = Vec::with_capacity(96);
    for anchors in anchor_sets {
        let perms = [
            [anchors[0], anchors[1], anchors[2]],
            [anchors[0], anchors[2], anchors[1]],
            [anchors[1], anchors[0], anchors[2]],
            [anchors[1], anchors[2], anchors[0]],
            [anchors[2], anchors[0], anchors[1]],
            [anchors[2], anchors[1], anchors[0]],
        ];
        for anchor_perm in perms {
            let mut anchor_of = [0usize; FLAVORS + 1];
            for flavor in 1..=FLAVORS {
                anchor_of[flavor] = anchor_perm[flavor - 1];
            }
            for fill_order in fill_orders {
                let mut label = [0u8; NN];
                let mut used = [false; NN];
                let mut score_bias = 0i64;
                for &flavor in &fill_order[..2] {
                    let need = flavor_counts[flavor];
                    let order = &orders[anchor_of[flavor]];
                    let mut placed = 0usize;
                    for &idx in order {
                        if !used[idx] {
                            used[idx] = true;
                            label[idx] = flavor as u8;
                            let r = idx / N;
                            let c = idx % N;
                            score_bias += anchor_distance(anchor_of[flavor], r, c) as i64;
                            placed += 1;
                            if placed == need {
                                break;
                            }
                        }
                    }
                }
                let last = fill_order[2] as u8;
                for idx in 0..NN {
                    if !used[idx] {
                        label[idx] = last;
                        let r = idx / N;
                        let c = idx % N;
                        score_bias += anchor_distance(anchor_of[last as usize], r, c) as i64;
                    }
                }
                let mut masks = [[0u64; 2]; FLAVORS + 1];
                for (idx, &flavor) in label.iter().enumerate() {
                    let word = idx / 64;
                    let bit = idx % 64;
                    masks[flavor as usize][word] |= 1u64 << bit;
                }
                targets.push(Target {
                    label,
                    masks,
                    score_bias,
                });
            }
        }
    }
    targets.sort_unstable_by_key(|t| t.score_bias);
    targets.truncate(24);
    targets
}

fn anchor_distance(anchor: usize, r: usize, c: usize) -> usize {
    match anchor {
        0 => r + c,
        1 => r + (N - 1 - c),
        2 => (N - 1 - r) + c,
        3 => (N - 1 - r) + (N - 1 - c),
        _ => unreachable!(),
    }
}

fn summarize_board(board: &Board) -> BoardSummary {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i32;
    let mut same_adj = 0i32;
    let mut diff_adj = 0i32;
    let mut masks = [[0u64; 2]; FLAVORS + 1];

    for idx in 0..NN {
        let color = board.cells[idx];
        if color == 0 {
            continue;
        }
        let word = idx / 64;
        let bit = idx % 64;
        masks[color as usize][word] |= 1u64 << bit;
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

    BoardSummary {
        comp_sq,
        same_adj,
        diff_adj,
        masks,
    }
}

fn target_match(summary: &BoardSummary, target: &Target) -> i32 {
    let mut total = 0i32;
    for flavor in 1..=FLAVORS {
        total += (summary.masks[flavor][0] & target.masks[flavor][0]).count_ones() as i32;
        total += (summary.masks[flavor][1] & target.masks[flavor][1]).count_ones() as i32;
    }
    total
}

fn score_from_summary(summary: &BoardSummary, target_match: i32, placed: usize) -> i64 {
    let phase = placed as i64;
    let target_w = (20 - phase / 8).max(4);
    i64::from(summary.comp_sq) * 14
        + i64::from(summary.same_adj) * 18
        - i64::from(summary.diff_adj) * 10
        + i64::from(target_match) * target_w
}

fn static_value(board: &Board, target: &Target, placed: usize) -> i64 {
    let summary = summarize_board(board);
    score_from_summary(&summary, target_match(&summary, target), placed)
}

fn candidate_value(candidate: &Candidate) -> f64 {
    candidate.sum / candidate.weight
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

fn final_raw_score(board: &Board) -> i64 {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i64;
    for idx in 0..NN {
        let color = board.cells[idx];
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
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if r + 1 < N {
                let next = cur + N;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if c > 0 {
                let next = cur - 1;
                if !seen[next] && board.cells[next] == color {
                    seen[next] = true;
                    stack[top] = next;
                    top += 1;
                }
            }
            if c + 1 < N {
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
    comp_sq
}

fn choose_best_target(board: &Board, placed: usize, targets: &[Target]) -> (usize, i64) {
    let summary = summarize_board(board);
    let mut best_idx = 0usize;
    let mut best_score = i64::MIN;
    for (idx, target) in targets.iter().enumerate() {
        let score = score_from_summary(&summary, target_match(&summary, target), placed);
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }
    (best_idx, best_score)
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

fn rollout_value(
    board: &Board,
    next_turn: usize,
    fs: &[u8; NN],
    target: &Target,
    rng: &mut XorShift64,
    horizon: usize,
) -> i64 {
    let mut board = *board;
    let mut turn = next_turn;
    let end_turn = (next_turn + horizon).min(NN);

    while turn < end_turn {
        let remaining_empty = NN - turn;
        let p = (rng.next_u64() as usize % remaining_empty) + 1;
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            return final_raw_score(&board) * 1000;
        }
        let placed = turn + 1;
        let mut best_board = board.tilted(DIRS[0]);
        let mut best_score = static_value(&best_board, target, placed);
        for &dir in &DIRS[1..] {
            let cand = board.tilted(dir);
            let score = static_value(&cand, target, placed);
            if score > best_score {
                best_score = score;
                best_board = cand;
            }
        }
        board = best_board;
        turn += 1;
    }

    if turn == NN {
        final_raw_score(&board) * 1000
    } else {
        static_value(&board, target, turn)
    }
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

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    targets: &[Target],
    timer: &Timer,
    rng: &mut XorShift64,
) -> u8 {
    let placed = turn + 1;
    let remaining_decisions = NN - 1 - turn;
    let remaining_empty = NN - placed;
    let base_deadline = timer.local_deadline(remaining_decisions.max(1));
    let mut candidates = [Candidate {
        dir: DIRS[0],
        board: Board::new(),
        target_idx: 0,
        sum: 0.0,
        weight: 1.0,
    }; 4];
    for &dir in &DIRS {
        let tilted = board_after_place.tilted(dir);
        let (target_idx, best_score) = choose_best_target(&tilted, placed, targets);
        candidates[dir_index(dir)] = Candidate {
            dir,
            board: tilted,
            target_idx,
            sum: (best_score * 2) as f64,
            weight: 2.0,
        };
    }

    let mut best_idx = 0usize;
    for i in 1..4 {
        if candidate_value(&candidates[i]) > candidate_value(&candidates[best_idx]) {
            best_idx = i;
        }
    }
    let mut best_value = candidate_value(&candidates[best_idx]);
    let mut second_value = f64::NEG_INFINITY;
    for (idx, candidate) in candidates.iter().enumerate() {
        if idx == best_idx {
            continue;
        }
        let value = candidate_value(candidate);
        if value > second_value {
            second_value = value;
        }
    }
    let confidence = rollout_confidence(best_value, second_value);

    if remaining_empty <= 4 {
        let mut exact_best_idx = 0usize;
        let mut exact_best_value = f64::NEG_INFINITY;
        for (idx, candidate) in candidates.iter().enumerate() {
            let value = exact_expected_value(&candidate.board, placed, fs);
            if value > exact_best_value {
                exact_best_value = value;
                exact_best_idx = idx;
            }
        }
        return candidates[exact_best_idx].dir;
    }

    if should_skip_rollout(placed, remaining_empty, confidence) {
        return candidates[best_idx].dir;
    }

    let local_deadline = shaped_deadline(timer, base_deadline, placed, remaining_empty, confidence);

    while !timer.is_time_up() && timer.elapsed() < local_deadline {
        let horizon = choose_rollout_horizon(timer, NN - placed);
        if horizon == 0 {
            break;
        }
        let sample_seed = rng.next_u64();
        for candidate in &mut candidates {
            if timer.is_time_up() || timer.elapsed() >= local_deadline {
                break;
            }
            let mut sample_rng = XorShift64::new(sample_seed);
            let target = &targets[candidate.target_idx];
            let value = rollout_value(
                &candidate.board,
                placed,
                fs,
                target,
                &mut sample_rng,
                horizon,
            );
            candidate.sum += value as f64;
            candidate.weight += 1.0;
            let current_value = candidate_value(candidate);
            if current_value > best_value {
                best_value = current_value;
                best_idx = dir_index(candidate.dir);
            }
        }
    }

    candidates[best_idx].dir
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(BufReader::new(stdin.lock()));
    let mut fs = [0u8; NN];
    let mut flavor_counts = [0usize; FLAVORS + 1];
    for f in &mut fs {
        *f = scanner.next_usize() as u8;
        flavor_counts[*f as usize] += 1;
    }
    let targets = build_targets(&flavor_counts);
    let seed = flavor_counts[1] as u64 * 1_000_003
        + flavor_counts[2] as u64 * 1_000_033
        + flavor_counts[3] as u64 * 1_000_037;
    let mut rng = XorShift64::new(seed);
    let timer = Timer::new(TIME_LIMIT_SEC, SAFETY_SEC);
    let mut board = Board::new();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for turn in 0..NN {
        let p = scanner.next_usize();
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = choose_dir(&board, turn, &fs, &targets, &timer, &mut rng);
        board = board.tilted(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
