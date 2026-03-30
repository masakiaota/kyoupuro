// v003_simple_dfs.rs
use std::time::Instant;

use proconio::input;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: usize = usize::MAX;
const TIME_LIMIT_SEC: f64 = 1.90;
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

struct Solver {
    tiles: Vec<usize>,
    scores: Vec<i32>,
    adj: Vec<[usize; 4]>,
    used_tiles: Vec<bool>,
    path: Vec<u8>,
    best_path: Vec<u8>,
    current_score: i32,
    best_score: i32,
    tk: TimeKeeper,
}

impl Solver {
    fn new(tiles: Vec<usize>, scores: Vec<i32>, adj: Vec<[usize; 4]>, start: usize, tile_count: usize) -> Self {
        let mut used_tiles = vec![false; tile_count];
        used_tiles[tiles[start]] = true;
        let start_score = scores[start];
        Self {
            tiles,
            scores,
            adj,
            used_tiles,
            path: Vec::with_capacity(CELL_COUNT),
            best_path: Vec::new(),
            current_score: start_score,
            best_score: start_score,
            tk: TimeKeeper::new(TIME_LIMIT_SEC, 10),
        }
    }

    fn solve(mut self, start: usize) -> Vec<u8> {
        self.update_best();
        self.dfs(start);
        self.best_path
    }

    fn dfs(&mut self, cur: usize) {
        self.update_best();
        if !self.tk.step() {
            return;
        }

        let mut candidates = [Candidate::default(); 4];
        let mut candidate_count = 0;
        for (dir_rank, &(_, _, dir_char)) in DIRS.iter().enumerate() {
            let next = self.adj[cur][dir_rank];
            if next == INVALID {
                continue;
            }
            let next_tile = self.tiles[next];
            if self.used_tiles[next_tile] {
                continue;
            }

            candidates[candidate_count] = Candidate {
                next,
                dir: dir_char,
                next_degree: self.count_next_degree(next, next_tile),
                immediate_score: self.scores[next],
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
            let next_tile = self.tiles[candidate.next];
            self.used_tiles[next_tile] = true;
            self.path.push(candidate.dir);
            self.current_score += self.scores[candidate.next];

            self.dfs(candidate.next);

            self.current_score -= self.scores[candidate.next];
            self.path.pop();
            self.used_tiles[next_tile] = false;

            if self.tk.is_over {
                return;
            }
        }
    }

    fn count_next_degree(&self, pos: usize, just_used_tile: usize) -> usize {
        self.adj[pos]
            .iter()
            .filter(|&&to| {
                to != INVALID && {
                    let tile = self.tiles[to];
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

fn main() {
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
    let tile_count = max_tile + 1;

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
    let best_path = Solver::new(tiles, scores, adj, start, tile_count).solve(start);
    println!("{}", String::from_utf8(best_path).unwrap());
}
