// v001_mc_target.rs
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const N: usize = 10;
const NN: usize = N * N;
const FLAVORS: usize = 3;
const DIRS: [u8; 4] = [b'F', b'B', b'L', b'R'];
const SAMPLE_COUNT: usize = 4;
const ROLLOUT_HORIZON: usize = 6;

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
    score_bias: i64,
}

struct EvalFeatures {
    comp_sq: i32,
    same_adj: i32,
    diff_adj: i32,
    target_match: i32,
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
                targets.push(Target { label, score_bias });
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

fn evaluate_features(board: &Board, target: &Target) -> EvalFeatures {
    let mut seen = [false; NN];
    let mut stack = [0usize; NN];
    let mut comp_sq = 0i32;
    let mut same_adj = 0i32;
    let mut diff_adj = 0i32;
    let mut target_match = 0i32;

    for idx in 0..NN {
        let color = board.cells[idx];
        if color == 0 {
            continue;
        }
        if color == target.label[idx] {
            target_match += 1;
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
        target_match,
    }
}

fn static_value(board: &Board, target: &Target, placed: usize) -> i64 {
    let f = evaluate_features(board, target);
    let phase = placed as i64;
    let target_w = (20 - phase / 8).max(4);
    i64::from(f.comp_sq) * 14 + i64::from(f.same_adj) * 18 - i64::from(f.diff_adj) * 10
        + i64::from(f.target_match) * target_w
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

fn choose_best_target(board: &Board, placed: usize, targets: &[Target]) -> usize {
    let mut best_idx = 0usize;
    let mut best_score = i64::MIN;
    for (idx, target) in targets.iter().enumerate() {
        let score = static_value(board, target, placed);
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }
    best_idx
}

fn rollout_value(
    board: &Board,
    next_turn: usize,
    fs: &[u8; NN],
    target: &Target,
    samples: &[usize],
) -> i64 {
    let mut board = *board;
    let mut turn = next_turn;
    let end_turn = (next_turn + ROLLOUT_HORIZON).min(NN);
    let mut sample_idx = 0usize;

    while turn < end_turn {
        let remaining_empty = NN - turn;
        let p = (samples[sample_idx] % remaining_empty) + 1;
        sample_idx += 1;
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

fn choose_dir(
    board_after_place: &Board,
    turn: usize,
    fs: &[u8; NN],
    targets: &[Target],
    rng: &mut XorShift64,
) -> u8 {
    let mut sampled_ps = [[0usize; ROLLOUT_HORIZON]; SAMPLE_COUNT];
    for sample in &mut sampled_ps {
        for depth in sample.iter_mut() {
            *depth = rng.next_u64() as usize;
        }
    }

    let mut best_dir = DIRS[0];
    let mut best_value = i64::MIN;

    for &dir in &DIRS {
        let tilted = board_after_place.tilted(dir);
        let target_idx = choose_best_target(&tilted, turn + 1, targets);
        let target = &targets[target_idx];

        let mut total = static_value(&tilted, target, turn + 1) * 2;
        for sample in &sampled_ps {
            total += rollout_value(&tilted, turn + 1, fs, target, sample);
        }
        if total > best_value {
            best_value = total;
            best_dir = dir;
        }
    }
    best_dir
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
    let mut board = Board::new();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for turn in 0..NN {
        let p = scanner.next_usize();
        board.place_pth_empty(p, fs[turn]);
        if turn == NN - 1 {
            break;
        }
        let dir = choose_dir(&board, turn, &fs, &targets, &mut rng);
        board = board.tilted(dir);
        writeln!(out, "{}", dir as char).unwrap();
        out.flush().unwrap();
    }
}
