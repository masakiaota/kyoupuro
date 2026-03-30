// v004_multistart_dfs.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: usize = usize::MAX;
const TOTAL_TIME_LIMIT_SEC: f64 = 0.40;
const RESTART_TIME_SEC: f64 = 0.05;
const DIRS: [(isize, isize, u8); 4] = [(-1, 0, b'U'), (1, 0, b'D'), (0, -1, b'L'), (0, 1, b'R')];

#[derive(Clone, Copy, Default)]
struct Candidate {
    next: usize,
    dir: u8,
    priority: i32,
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
    tile_count: usize,
}

struct Solver<'a> {
    board: &'a Board,
    used_tiles: Vec<bool>,
    path: Vec<u8>,
    best_path: Vec<u8>,
    current_score: i32,
    best_score: i32,
    tk: TimeKeeper,
    rng: Xoshiro256PlusPlus,
}

impl<'a> Solver<'a> {
    fn new(board: &'a Board, time_limit_sec: f64, seed: u64) -> Self {
        let mut used_tiles = vec![false; board.tile_count];
        used_tiles[board.tiles[board.start]] = true;
        let start_score = board.scores[board.start];
        Self {
            board,
            used_tiles,
            path: Vec::with_capacity(CELL_COUNT),
            best_path: Vec::new(),
            current_score: start_score,
            best_score: start_score,
            tk: TimeKeeper::new(time_limit_sec, 10),
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    fn solve(mut self) -> (Vec<u8>, i32) {
        self.update_best();
        self.dfs(self.board.start);
        (self.best_path, self.best_score)
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
            if self.used_tiles[next_tile] {
                continue;
            }

            let next_degree = self.count_next_degree(next, next_tile) as i32;
            let immediate_score = self.board.scores[next];
            let noise = self.rng.random_range(0..64_i32);
            candidates[candidate_count] = Candidate {
                next,
                dir: dir_char,
                priority: next_degree * 1024 - immediate_score * 4 + noise,
                dir_rank,
            };
            candidate_count += 1;
        }

        candidates[..candidate_count].sort_unstable_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.dir_rank.cmp(&b.dir_rank))
        });

        for candidate in candidates.iter().take(candidate_count).copied() {
            let next_tile = self.board.tiles[candidate.next];
            self.used_tiles[next_tile] = true;
            self.path.push(candidate.dir);
            self.current_score += self.board.scores[candidate.next];

            self.dfs(candidate.next);

            self.current_score -= self.board.scores[candidate.next];
            self.path.pop();
            self.used_tiles[next_tile] = false;

            if self.tk.is_over {
                return;
            }
        }
    }

    fn count_next_degree(&self, pos: usize, just_used_tile: usize) -> usize {
        self.board.adj[pos]
            .iter()
            .filter(|&&to| {
                to != INVALID && {
                    let tile = self.board.tiles[to];
                    tile != just_used_tile && !self.used_tiles[tile]
                }
            })
            .count()
    }

    fn update_best(&mut self) {
        if self.path.len() > self.best_path.len()
            || (self.path.len() == self.best_path.len() && self.current_score > self.best_score)
        {
            self.best_path.clear();
            self.best_path.extend_from_slice(&self.path);
            self.best_score = self.current_score;
        }
    }
}

fn is_better(path: &[u8], score: i32, best_path: &[u8], best_score: i32) -> bool {
    path.len() > best_path.len() || (path.len() == best_path.len() && score > best_score)
}

fn build_board() -> Board {
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

    Board {
        tiles,
        scores,
        adj,
        start: si * N + sj,
        tile_count: max_tile + 1,
    }
}

fn main() {
    let board = build_board();
    let global_start = Instant::now();
    let mut restart_id = 0_u64;
    let mut best_path = Vec::new();
    let mut best_score = board.scores[board.start];

    while global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC {
        let elapsed = global_start.elapsed().as_secs_f64();
        let remaining = TOTAL_TIME_LIMIT_SEC - elapsed;
        if remaining <= 0.0 {
            break;
        }
        let run_limit = remaining.min(RESTART_TIME_SEC);
        let seed = ((board.start as u64) << 32) ^ restart_id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (path, score) = Solver::new(&board, run_limit, seed).solve();
        if is_better(&path, score, &best_path, best_score) {
            best_path = path;
            best_score = score;
        }
        restart_id += 1;
    }

    println!("{}", String::from_utf8(best_path).unwrap());
}
