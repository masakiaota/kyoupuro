// v008_v005_plus_middle_hc.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: u16 = u16::MAX;
const TOTAL_TIME_LIMIT_SEC: f64 = 1.90;
const INITIAL_PHASE_SEC: f64 = 0.40;
const RESTART_TIME_SEC: f64 = 0.05;
const SEG_MIN: usize = 8;
const SEG_MAX: usize = 180;
const SEG_EXTRA_LEN: usize = 24;
const NODE_LIMIT_BASE: u32 = 500;
const NODE_LIMIT_SCALE: u32 = 24;
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

struct RerouteSearch<'a> {
    board: &'a Board,
    used_tiles: &'a mut [u8],
    b_cell: u16,
    min_i: usize,
    max_i: usize,
    min_j: usize,
    max_j: usize,
    max_len: usize,
    dir_order: [usize; 4],
    node_limit: u32,
    nodes: u32,
    path: Vec<u8>,
    best_path: Vec<u8>,
    cur_score: i32,
    best_score: i32,
}

impl<'a> RerouteSearch<'a> {
    fn solve(mut self, start_cell: u16) -> Option<(Vec<u8>, i32)> {
        self.dfs(start_cell);
        if self.best_path.is_empty() {
            None
        } else {
            Some((self.best_path, self.best_score))
        }
    }

    fn dfs(&mut self, cur: u16) {
        if self.nodes >= self.node_limit {
            return;
        }
        self.nodes += 1;

        if self.path.len() >= self.max_len {
            return;
        }
        if self.path.len() + manhattan(cur, self.b_cell) > self.max_len {
            return;
        }

        let dir_order = self.dir_order;
        for &dir_rank in &dir_order {
            let next = self.board.adj[cur as usize][dir_rank];
            if next == INVALID {
                continue;
            }
            let dir_char = DIR_CHARS[dir_rank];
            if next == self.b_cell {
                self.path.push(dir_char);
                let cand_score = self.cur_score + self.board.scores[next as usize] as i32;
                if self.path.len() > self.best_path.len()
                    || (self.path.len() == self.best_path.len() && cand_score > self.best_score)
                {
                    self.best_path.clear();
                    self.best_path.extend_from_slice(&self.path);
                    self.best_score = cand_score;
                }
                self.path.pop();
                continue;
            }

            let ni = (next as usize) / N;
            let nj = (next as usize) % N;
            if ni < self.min_i || ni > self.max_i || nj < self.min_j || nj > self.max_j {
                continue;
            }

            let tile = self.board.tiles[next as usize] as usize;
            if self.used_tiles[tile] != 0 {
                continue;
            }

            self.used_tiles[tile] = 1;
            self.path.push(dir_char);
            self.cur_score += self.board.scores[next as usize] as i32;
            self.dfs(next);
            self.cur_score -= self.board.scores[next as usize] as i32;
            self.path.pop();
            self.used_tiles[tile] = 0;

            if self.nodes >= self.node_limit {
                return;
            }
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

#[inline(always)]
fn is_better(len: usize, score: i32, best_len: usize, best_score: i32) -> bool {
    len > best_len || (len == best_len && score > best_score)
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

fn path_to_cells(board: &Board, path: &[u8]) -> Vec<u16> {
    let mut cells = Vec::with_capacity(path.len() + 1);
    let mut cur = board.start;
    cells.push(cur);
    for &dir in path {
        let next = board.adj[cur as usize][dir_to_idx(dir)];
        debug_assert!(next != INVALID);
        cur = next;
        cells.push(cur);
    }
    cells
}

#[inline(always)]
fn manhattan(a: u16, b: u16) -> usize {
    let ai = (a as usize) / N;
    let aj = (a as usize) % N;
    let bi = (b as usize) / N;
    let bj = (b as usize) % N;
    ai.abs_diff(bi) + aj.abs_diff(bj)
}

fn choose_segment(rng: &mut Xoshiro256PlusPlus, len: usize) -> Option<(usize, usize)> {
    if len <= SEG_MIN {
        return None;
    }
    let seg_max = SEG_MAX.min(len);
    let seg_len = rng.random_range(SEG_MIN..=seg_max);
    let l = rng.random_range(0..=(len - seg_len));
    Some((l, l + seg_len))
}

fn main() {
    let board = build_board();
    let global_start = Instant::now();

    // Phase 1: v005 の multistart DFS で初期解を作る
    let mut restart_id = 0_u64;
    let mut solver = Solver::new(&board);
    let mut best_path_fixed = [0_u8; CELL_COUNT];
    let mut best_len = 0_usize;
    let mut best_score = board.start_score;

    while global_start.elapsed().as_secs_f64() < INITIAL_PHASE_SEC
        && global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC
    {
        let elapsed_total = global_start.elapsed().as_secs_f64();
        let remain_total = TOTAL_TIME_LIMIT_SEC - elapsed_total;
        if remain_total <= 0.0 {
            break;
        }
        let elapsed_phase = global_start.elapsed().as_secs_f64();
        let remain_phase = INITIAL_PHASE_SEC - elapsed_phase;
        if remain_phase <= 0.0 {
            break;
        }
        let run_limit = remain_total.min(remain_phase).min(RESTART_TIME_SEC);
        let seed = ((board.start as u64) << 32) ^ restart_id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        solver.solve(run_limit, seed);
        if is_better(solver.best_len, solver.best_score, best_len, best_score) {
            best_len = solver.best_len;
            best_score = solver.best_score;
            best_path_fixed[..best_len].copy_from_slice(&solver.best_path[..best_len]);
        }
        restart_id += 1;
    }

    let mut best_path = best_path_fixed[..best_len].to_vec();
    let mut best_cells = path_to_cells(&board, &best_path);
    let mut used_tiles = vec![0_u8; board.tile_count];
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(((board.start as u64) << 32) ^ 0xA9E3_7A52_B1CD_44F3);

    // Phase 2: middle segment reroute hill climbing
    while global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC {
        let len = best_path.len();
        let Some((l, r)) = choose_segment(&mut rng, len) else {
            break;
        };

        used_tiles.fill(0);
        for &cell in best_cells.iter().take(l + 1) {
            used_tiles[board.tiles[cell as usize] as usize] = 1;
        }
        for &cell in best_cells.iter().skip(r) {
            used_tiles[board.tiles[cell as usize] as usize] = 1;
        }

        let a_cell = best_cells[l];
        let b_cell = best_cells[r];
        let old_seg_score: i32 = best_cells[l + 1..=r]
            .iter()
            .map(|&c| board.scores[c as usize] as i32)
            .sum();
        let old_seg_len = r - l;

        let mut min_i = N - 1;
        let mut max_i = 0;
        let mut min_j = N - 1;
        let mut max_j = 0;
        for &cell in &best_cells[l..=r] {
            let c = cell as usize;
            let i = c / N;
            let j = c % N;
            min_i = min_i.min(i);
            max_i = max_i.max(i);
            min_j = min_j.min(j);
            max_j = max_j.max(j);
        }
        let margin = rng.random_range(2..=6);
        min_i = min_i.saturating_sub(margin);
        min_j = min_j.saturating_sub(margin);
        max_i = (max_i + margin).min(N - 1);
        max_j = (max_j + margin).min(N - 1);

        let mut dir_order = [0_usize, 1, 2, 3];
        for i in 0..4 {
            let j = rng.random_range(i..4);
            dir_order.swap(i, j);
        }

        let search = RerouteSearch {
            board: &board,
            used_tiles: &mut used_tiles,
            b_cell,
            min_i,
            max_i,
            min_j,
            max_j,
            max_len: old_seg_len + SEG_EXTRA_LEN,
            dir_order,
            node_limit: NODE_LIMIT_BASE + NODE_LIMIT_SCALE * (old_seg_len as u32),
            nodes: 0,
            path: Vec::with_capacity(old_seg_len + SEG_EXTRA_LEN),
            best_path: Vec::new(),
            cur_score: 0,
            best_score: i32::MIN,
        };

        let Some((new_seg_path, new_seg_score)) = search.solve(a_cell) else {
            continue;
        };

        let candidate_len = len - old_seg_len + new_seg_path.len();
        let candidate_score = best_score - old_seg_score + new_seg_score;
        if !is_better(candidate_len, candidate_score, len, best_score) {
            continue;
        }

        let mut next_path = Vec::with_capacity(candidate_len);
        next_path.extend_from_slice(&best_path[..l]);
        next_path.extend_from_slice(&new_seg_path);
        next_path.extend_from_slice(&best_path[r..]);
        best_path = next_path;
        best_cells = path_to_cells(&board, &best_path);
        best_score = candidate_score;
    }

    println!("{}", std::str::from_utf8(&best_path).unwrap());
}
