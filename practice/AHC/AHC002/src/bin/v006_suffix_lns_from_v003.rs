// v006_suffix_lns_from_v003.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: usize = usize::MAX;
const TOTAL_TIME_LIMIT_SEC: f64 = 0.40;
const INITIAL_TIME_SEC: f64 = 0.08;
const REPAIR_TIME_SEC: f64 = 0.02;
const DIRS: [(isize, isize, u8); 4] = [(-1, 0, b'U'), (1, 0, b'D'), (0, -1, b'L'), (0, 1, b'R')];

#[derive(Clone, Copy, Default)]
struct Candidate {
    next: usize,
    dir: u8,
    next_degree: usize,
    immediate_score: i32,
    dir_rank: usize,
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
    iter: u64,
    check_mask: u64,
    is_over: bool,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };
        Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            is_over: false,
        }
    }

    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.is_over = self.start.elapsed().as_secs_f64() >= self.time_limit_sec;
        }
        !self.is_over
    }
}

struct Board {
    tiles: Vec<usize>,
    scores: Vec<i32>,
    adj: Vec<[usize; 4]>,
    start: usize,
    start_tile: usize,
}

struct SuffixSearch<'a> {
    board: &'a Board,
    used_tiles: &'a mut [u8],
    path: Vec<u8>,
    best_path: Vec<u8>,
    current_score_delta: i32,
    best_score_delta: i32,
    tk: TimeKeeper,
}

impl<'a> SuffixSearch<'a> {
    fn new(board: &'a Board, used_tiles: &'a mut [u8], time_limit_sec: f64) -> Self {
        Self {
            board,
            used_tiles,
            path: Vec::with_capacity(CELL_COUNT),
            best_path: Vec::new(),
            current_score_delta: 0,
            best_score_delta: 0,
            tk: TimeKeeper::new(time_limit_sec, 10),
        }
    }

    fn solve(mut self, cur: usize) -> (Vec<u8>, i32) {
        self.update_best();
        self.dfs(cur);
        (self.best_path, self.best_score_delta)
    }

    fn dfs(&mut self, cur: usize) {
        self.update_best();
        if !self.tk.step() {
            return;
        }

        let mut candidates = [Candidate::default(); 4];
        let mut candidate_count = 0;
        for (dir_rank, &(_, _, dir_char)) in DIRS.iter().enumerate() {
            let next = self.board.adj[cur][dir_rank];
            if next == INVALID {
                continue;
            }
            let next_tile = self.board.tiles[next];
            if self.used_tiles[next_tile] != 0 {
                continue;
            }

            candidates[candidate_count] = Candidate {
                next,
                dir: dir_char,
                next_degree: self.count_next_degree(next, next_tile),
                immediate_score: self.board.scores[next],
                dir_rank,
            };
            candidate_count += 1;
        }

        candidates[..candidate_count].sort_unstable_by(|a, b| {
            a.next_degree
                .cmp(&b.next_degree)
                .then_with(|| b.immediate_score.cmp(&a.immediate_score))
                .then_with(|| a.dir_rank.cmp(&b.dir_rank))
        });

        for candidate in candidates.iter().take(candidate_count).copied() {
            let next_tile = self.board.tiles[candidate.next];
            self.used_tiles[next_tile] = 1;
            self.path.push(candidate.dir);
            self.current_score_delta += self.board.scores[candidate.next];

            self.dfs(candidate.next);

            self.current_score_delta -= self.board.scores[candidate.next];
            self.path.pop();
            self.used_tiles[next_tile] = 0;

            if self.tk.is_over {
                return;
            }
        }
    }

    fn count_next_degree(&self, pos: usize, just_used_tile: usize) -> usize {
        self.board.adj[pos]
            .iter()
            .filter(|&&to| {
                if to == INVALID {
                    return false;
                }
                let tile = self.board.tiles[to];
                tile != just_used_tile && self.used_tiles[tile] == 0
            })
            .count()
    }

    fn update_best(&mut self) {
        if self.path.len() > self.best_path.len()
            || (self.path.len() == self.best_path.len()
                && self.current_score_delta > self.best_score_delta)
        {
            self.best_path.clear();
            self.best_path.extend_from_slice(&self.path);
            self.best_score_delta = self.current_score_delta;
        }
    }
}

#[inline(always)]
fn is_better(path_len: usize, score_delta: i32, best_len: usize, best_score_delta: i32) -> bool {
    path_len > best_len || (path_len == best_len && score_delta > best_score_delta)
}

#[inline(always)]
fn dir_to_idx(dir: u8) -> usize {
    match dir {
        b'U' => 0,
        b'D' => 1,
        b'L' => 2,
        b'R' => 3,
        _ => unreachable!(),
    }
}

fn replay_prefix(
    board: &Board,
    path: &[u8],
    cut: usize,
    used_tiles: &mut [u8],
) -> (usize, i32, usize) {
    used_tiles.fill(0);
    used_tiles[board.start_tile] = 1;
    let mut cur = board.start;
    let mut score_delta = 0;
    let mut actual_cut = 0;

    for &dir in path.iter().take(cut) {
        let dir_idx = dir_to_idx(dir);
        let next = board.adj[cur][dir_idx];
        if next == INVALID {
            break;
        }
        let tile = board.tiles[next];
        if used_tiles[tile] != 0 {
            break;
        }
        cur = next;
        used_tiles[tile] = 1;
        score_delta += board.scores[next];
        actual_cut += 1;
    }

    (cur, score_delta, actual_cut)
}

fn build_board() -> (Board, usize) {
    input! {
        si: usize,
        sj: usize,
        t: [[usize; N]; N],
        p: [[i32; N]; N],
    }

    let mut tiles = vec![0; CELL_COUNT];
    let mut scores = vec![0; CELL_COUNT];
    let mut max_tile = 0;
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            tiles[idx] = t[i][j];
            scores[idx] = p[i][j];
            max_tile = max_tile.max(t[i][j]);
        }
    }

    let mut adj = vec![[INVALID; 4]; CELL_COUNT];
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            for (dir_rank, &(di, dj, _)) in DIRS.iter().enumerate() {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if !(0..N as isize).contains(&ni) || !(0..N as isize).contains(&nj) {
                    continue;
                }
                let ni = ni as usize;
                let nj = nj as usize;
                if t[i][j] == t[ni][nj] {
                    continue;
                }
                adj[idx][dir_rank] = ni * N + nj;
            }
        }
    }

    let start = si * N + sj;
    (
        Board {
            start,
            start_tile: tiles[start],
            tiles,
            scores,
            adj,
        },
        max_tile + 1,
    )
}

fn main() {
    let (board, tile_count) = build_board();
    let global_start = Instant::now();
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(((board.start as u64) << 32) ^ 0xA63B_5F29_D77C_13E9);
    let mut used_tiles = vec![0_u8; tile_count];

    // 初期解: v003 と同じ DFS を start から一度実行する。
    used_tiles[board.start_tile] = 1;
    let initial_budget = INITIAL_TIME_SEC.min(TOTAL_TIME_LIMIT_SEC);
    let (mut best_path, mut best_score_delta) =
        SuffixSearch::new(&board, &mut used_tiles, initial_budget).solve(board.start);
    used_tiles[board.start_tile] = 0;

    while global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC {
        let elapsed = global_start.elapsed().as_secs_f64();
        let remaining = TOTAL_TIME_LIMIT_SEC - elapsed;
        if remaining <= 0.0 {
            break;
        }
        let run_limit = remaining.min(REPAIR_TIME_SEC);
        let best_len = best_path.len();
        if best_len == 0 {
            break;
        }

        let cut = if rng.random_bool(0.8) {
            let low = best_len * 2 / 3;
            rng.random_range(low..=best_len)
        } else {
            rng.random_range(0..=best_len)
        };

        let (cur, prefix_score_delta, actual_cut) =
            replay_prefix(&board, &best_path, cut, &mut used_tiles);
        let (suffix_path, suffix_score_delta) =
            SuffixSearch::new(&board, &mut used_tiles, run_limit).solve(cur);

        let candidate_len = actual_cut + suffix_path.len();
        let candidate_score_delta = prefix_score_delta + suffix_score_delta;
        if is_better(
            candidate_len,
            candidate_score_delta,
            best_path.len(),
            best_score_delta,
        ) {
            let mut next_best = Vec::with_capacity(candidate_len);
            next_best.extend_from_slice(&best_path[..actual_cut]);
            next_best.extend_from_slice(&suffix_path);
            best_path = next_best;
            best_score_delta = candidate_score_delta;
        }
    }

    println!("{}", String::from_utf8(best_path).unwrap());
}
