// v005_optimized_multistart_dfs.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: u16 = u16::MAX;
const TOTAL_TIME_LIMIT_SEC: f64 = 0.40;
const RESTART_TIME_SEC: f64 = 0.05;
const DIR_CHARS: [u8; 4] = [b'U', b'D', b'L', b'R'];
const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[derive(Clone, Copy, Default)]
struct Candidate {
    next: u16,
    dir: u8,
    priority: i32,
    dir_rank: u8,
}

struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
    iter: u64,
    check_mask: u64,
    is_over: bool,
}

impl TimeKeeper {
    fn new(check_interval_log2: u32) -> Self {
        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };
        Self {
            start: Instant::now(),
            time_limit_sec: 0.0,
            iter: 0,
            check_mask,
            is_over: false,
        }
    }

    #[inline(always)]
    fn reset(&mut self, time_limit_sec: f64) {
        self.start = Instant::now();
        self.time_limit_sec = time_limit_sec;
        self.iter = 0;
        self.is_over = false;
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
    tiles: Vec<u16>,
    scores: Vec<u8>,
    adj: Vec<[u16; 4]>,
    base_deg: Vec<u8>,
    tile_cells: Vec<[u16; 2]>,
    tile_sizes: Vec<u8>,
    start: u16,
    start_tile: u16,
    start_score: i32,
    tile_count: usize,
}

struct Solver<'a> {
    board: &'a Board,
    used_tiles: Vec<u8>,
    avail_deg: Vec<u8>,
    path: [u8; CELL_COUNT],
    move_cells: [u16; CELL_COUNT],
    best_path: [u8; CELL_COUNT],
    noise: [u8; CELL_COUNT],
    path_len: usize,
    best_len: usize,
    current_score: i32,
    best_score: i32,
    tk: TimeKeeper,
}

impl<'a> Solver<'a> {
    fn new(board: &'a Board) -> Self {
        Self {
            board,
            used_tiles: vec![0; board.tile_count],
            avail_deg: vec![0; CELL_COUNT],
            path: [0; CELL_COUNT],
            move_cells: [0; CELL_COUNT],
            best_path: [0; CELL_COUNT],
            noise: [0; CELL_COUNT],
            path_len: 0,
            best_len: 0,
            current_score: board.start_score,
            best_score: board.start_score,
            tk: TimeKeeper::new(10),
        }
    }

    fn solve(&mut self, time_limit_sec: f64, seed: u64) {
        self.reset(time_limit_sec, seed);
        self.dfs(self.board.start);
    }

    #[inline]
    fn reset(&mut self, time_limit_sec: f64, seed: u64) {
        self.used_tiles.fill(0);
        self.avail_deg.copy_from_slice(&self.board.base_deg);
        self.path_len = 0;
        self.best_len = 0;
        self.current_score = self.board.start_score;
        self.best_score = self.board.start_score;
        self.tk.reset(time_limit_sec);

        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
        for idx in 0..CELL_COUNT {
            self.noise[idx] = rng.random_range(0..64_u8);
        }

        self.use_tile(self.board.start_tile);
        self.update_best();
    }

    fn dfs(&mut self, start_cur: u16) {
        let base_len = self.path_len;
        let mut cur = start_cur;
        let mut candidates = [Candidate::default(); 4];

        loop {
            self.update_best();
            if !self.tk.step() {
                self.undo_to(base_len);
                return;
            }

            let candidate_count = self.collect_candidates(cur, &mut candidates);
            if candidate_count == 0 {
                self.undo_to(base_len);
                return;
            }
            if candidate_count == 1 {
                self.apply_move(candidates[0]);
                cur = candidates[0].next;
                continue;
            }

            sort_candidates(&mut candidates, candidate_count);
            for idx in 0..candidate_count {
                let candidate = candidates[idx];
                self.apply_move(candidate);
                self.dfs(candidate.next);
                self.undo_last_move();
                if self.tk.is_over {
                    self.undo_to(base_len);
                    return;
                }
            }
            self.undo_to(base_len);
            return;
        }
    }

    #[inline(always)]
    fn collect_candidates(&self, cur: u16, out: &mut [Candidate; 4]) -> usize {
        let mut count = 0;
        let adj = self.board.adj[cur as usize];
        let mut dir_rank = 0;
        while dir_rank < 4 {
            let next = adj[dir_rank];
            if next != INVALID {
                let next_idx = next as usize;
                let tile = self.board.tiles[next_idx] as usize;
                if self.used_tiles[tile] == 0 {
                    out[count] = Candidate {
                        next,
                        dir: DIR_CHARS[dir_rank],
                        priority: (self.avail_deg[next_idx] as i32) * 1024
                            - (self.board.scores[next_idx] as i32) * 4
                            + self.noise[next_idx] as i32,
                        dir_rank: dir_rank as u8,
                    };
                    count += 1;
                }
            }
            dir_rank += 1;
        }
        count
    }

    #[inline(always)]
    fn apply_move(&mut self, candidate: Candidate) {
        let next = candidate.next;
        let next_idx = next as usize;
        self.path[self.path_len] = candidate.dir;
        self.move_cells[self.path_len] = next;
        self.path_len += 1;
        self.current_score += self.board.scores[next_idx] as i32;
        self.use_tile(self.board.tiles[next_idx]);
    }

    #[inline(always)]
    fn undo_last_move(&mut self) {
        self.path_len -= 1;
        let cell = self.move_cells[self.path_len];
        let cell_idx = cell as usize;
        self.current_score -= self.board.scores[cell_idx] as i32;
        self.unuse_tile(self.board.tiles[cell_idx]);
    }

    #[inline(always)]
    fn undo_to(&mut self, target_len: usize) {
        while self.path_len > target_len {
            self.undo_last_move();
        }
    }

    #[inline(always)]
    fn use_tile(&mut self, tile: u16) {
        let tile_idx = tile as usize;
        self.used_tiles[tile_idx] = 1;
        let cells = self.board.tile_cells[tile_idx];
        let cell_count = self.board.tile_sizes[tile_idx] as usize;
        let mut k = 0;
        while k < cell_count {
            let cell = cells[k] as usize;
            let adj = self.board.adj[cell];
            let mut dir = 0;
            while dir < 4 {
                let next = adj[dir];
                if next != INVALID {
                    let next_idx = next as usize;
                    let next_tile = self.board.tiles[next_idx] as usize;
                    if self.used_tiles[next_tile] == 0 {
                        self.avail_deg[next_idx] -= 1;
                    }
                }
                dir += 1;
            }
            k += 1;
        }
    }

    #[inline(always)]
    fn unuse_tile(&mut self, tile: u16) {
        let tile_idx = tile as usize;
        self.used_tiles[tile_idx] = 0;
        let cells = self.board.tile_cells[tile_idx];
        let cell_count = self.board.tile_sizes[tile_idx] as usize;
        let mut k = 0;
        while k < cell_count {
            let cell = cells[k] as usize;
            let adj = self.board.adj[cell];
            let mut dir = 0;
            while dir < 4 {
                let next = adj[dir];
                if next != INVALID {
                    let next_idx = next as usize;
                    let next_tile = self.board.tiles[next_idx] as usize;
                    if self.used_tiles[next_tile] == 0 {
                        self.avail_deg[next_idx] += 1;
                    }
                }
                dir += 1;
            }
            k += 1;
        }
    }

    #[inline(always)]
    fn update_best(&mut self) {
        if self.path_len > self.best_len
            || (self.path_len == self.best_len && self.current_score > self.best_score)
        {
            self.best_len = self.path_len;
            self.best_score = self.current_score;
            self.best_path[..self.path_len].copy_from_slice(&self.path[..self.path_len]);
        }
    }
}

#[inline(always)]
fn candidate_precedes(a: Candidate, b: Candidate) -> bool {
    a.priority < b.priority || (a.priority == b.priority && a.dir_rank < b.dir_rank)
}

#[inline(always)]
fn sort_candidates(candidates: &mut [Candidate; 4], count: usize) {
    let mut i = 1;
    while i < count {
        let value = candidates[i];
        let mut j = i;
        while j > 0 && candidate_precedes(value, candidates[j - 1]) {
            candidates[j] = candidates[j - 1];
            j -= 1;
        }
        candidates[j] = value;
        i += 1;
    }
}

fn build_board() -> Board {
    input! {
        si: usize,
        sj: usize,
        t: [[u16; N]; N],
        p: [[u8; N]; N],
    }

    let mut tiles = vec![0_u16; CELL_COUNT];
    let mut scores = vec![0_u8; CELL_COUNT];
    let mut max_tile = 0_u16;
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            tiles[idx] = t[i][j];
            scores[idx] = p[i][j];
            max_tile = max_tile.max(t[i][j]);
        }
    }

    let tile_count = max_tile as usize + 1;
    let mut tile_cells = vec![[INVALID; 2]; tile_count];
    let mut tile_sizes = vec![0_u8; tile_count];
    for cell in 0..CELL_COUNT {
        let tile = tiles[cell] as usize;
        let size = tile_sizes[tile] as usize;
        tile_cells[tile][size] = cell as u16;
        tile_sizes[tile] += 1;
    }

    let mut adj = vec![[INVALID; 4]; CELL_COUNT];
    let mut base_deg = vec![0_u8; CELL_COUNT];
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            let mut deg = 0_u8;
            for dir_rank in 0..4 {
                let (di, dj) = DIRS[dir_rank];
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
                adj[idx][dir_rank] = (ni * N + nj) as u16;
                deg += 1;
            }
            base_deg[idx] = deg;
        }
    }

    let start = (si * N + sj) as u16;
    Board {
        start,
        start_tile: tiles[start as usize],
        start_score: scores[start as usize] as i32,
        tiles,
        scores,
        adj,
        base_deg,
        tile_cells,
        tile_sizes,
        tile_count,
    }
}

fn is_better(len: usize, score: i32, best_len: usize, best_score: i32) -> bool {
    len > best_len || (len == best_len && score > best_score)
}

fn main() {
    let board = build_board();
    let global_start = Instant::now();
    let mut restart_id = 0_u64;
    let mut solver = Solver::new(&board);
    let mut best_path = [0_u8; CELL_COUNT];
    let mut best_len = 0_usize;
    let mut best_score = board.start_score;

    while global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC {
        let elapsed = global_start.elapsed().as_secs_f64();
        let remaining = TOTAL_TIME_LIMIT_SEC - elapsed;
        if remaining <= 0.0 {
            break;
        }
        let run_limit = remaining.min(RESTART_TIME_SEC);
        let seed = ((board.start as u64) << 32) ^ restart_id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        solver.solve(run_limit, seed);
        if is_better(solver.best_len, solver.best_score, best_len, best_score) {
            best_len = solver.best_len;
            best_score = solver.best_score;
            best_path[..best_len].copy_from_slice(&solver.best_path[..best_len]);
        }
        restart_id += 1;
    }

    println!("{}", std::str::from_utf8(&best_path[..best_len]).unwrap());
}
