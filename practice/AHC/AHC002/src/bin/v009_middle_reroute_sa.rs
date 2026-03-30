// v009_middle_reroute_sa.rs
use std::time::Instant;

use proconio::input;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: usize = usize::MAX;
const TOTAL_TIME_LIMIT_SEC: f64 = 1.90;
const INITIAL_DFS_SEC: f64 = 0.20; // v003 の 0.27s 相当版を初期解生成に使う
const SEG_MIN: usize = 8;
const SEG_MAX: usize = 120;
const SEG_EXTRA_LEN: usize = 20;
const NODE_LIMIT_BASE: u32 = 500;
const NODE_LIMIT_SCALE: u32 = 24;
const SA_TEMP_START: f64 = 450.0;
const SA_TEMP_END: f64 = 5.0;
const LEN_WEIGHT: f64 = 1_000.0;
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
    tile_count: usize,
}

struct InitialSolver<'a> {
    board: &'a Board,
    used_tiles: Vec<u8>,
    path: Vec<u8>,
    best_path: Vec<u8>,
    cur_score_delta: i32,
    best_score_delta: i32,
    tk: TimeKeeper,
}

impl<'a> InitialSolver<'a> {
    fn new(board: &'a Board, time_limit_sec: f64) -> Self {
        let mut used_tiles = vec![0_u8; board.tile_count];
        used_tiles[board.start_tile] = 1;
        Self {
            board,
            used_tiles,
            path: Vec::with_capacity(CELL_COUNT),
            best_path: Vec::new(),
            cur_score_delta: 0,
            best_score_delta: 0,
            tk: TimeKeeper::new(time_limit_sec, 10),
        }
    }

    fn solve(mut self) -> (Vec<u8>, i32) {
        self.update_best();
        self.dfs(self.board.start);
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
            self.cur_score_delta += self.board.scores[candidate.next];
            self.dfs(candidate.next);
            self.cur_score_delta -= self.board.scores[candidate.next];
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
            || (self.path.len() == self.best_path.len() && self.cur_score_delta > self.best_score_delta)
        {
            self.best_path.clear();
            self.best_path.extend_from_slice(&self.path);
            self.best_score_delta = self.cur_score_delta;
        }
    }
}

struct RerouteSearch<'a> {
    board: &'a Board,
    used_tiles: &'a mut [u8],
    b_cell: usize,
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
    cur_score_delta: i32,
    best_score_delta: i32,
}

impl<'a> RerouteSearch<'a> {
    fn solve(mut self, start_cell: usize) -> Option<(Vec<u8>, i32)> {
        self.dfs(start_cell);
        if self.best_path.is_empty() {
            None
        } else {
            Some((self.best_path, self.best_score_delta))
        }
    }

    fn dfs(&mut self, cur: usize) {
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
            let next = self.board.adj[cur][dir_rank];
            if next == INVALID {
                continue;
            }
            let dir_char = DIRS[dir_rank].2;
            if next == self.b_cell {
                self.path.push(dir_char);
                let cand_score = self.cur_score_delta + self.board.scores[next];
                if self.path.len() > self.best_path.len()
                    || (self.path.len() == self.best_path.len() && cand_score > self.best_score_delta)
                {
                    self.best_path.clear();
                    self.best_path.extend_from_slice(&self.path);
                    self.best_score_delta = cand_score;
                }
                self.path.pop();
                continue;
            }

            let ni = next / N;
            let nj = next % N;
            if ni < self.min_i || ni > self.max_i || nj < self.min_j || nj > self.max_j {
                continue;
            }

            let next_tile = self.board.tiles[next];
            if self.used_tiles[next_tile] != 0 {
                continue;
            }

            self.used_tiles[next_tile] = 1;
            self.path.push(dir_char);
            self.cur_score_delta += self.board.scores[next];
            self.dfs(next);
            self.cur_score_delta -= self.board.scores[next];
            self.path.pop();
            self.used_tiles[next_tile] = 0;
            if self.nodes >= self.node_limit {
                return;
            }
        }
    }
}

#[inline(always)]
fn manhattan(a: usize, b: usize) -> usize {
    let ai = a / N;
    let aj = a % N;
    let bi = b / N;
    let bj = b % N;
    ai.abs_diff(bi) + aj.abs_diff(bj)
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
    Board {
        start,
        start_tile: tiles[start],
        tiles,
        scores,
        adj,
        tile_count,
    }
}

fn path_to_cells(board: &Board, path: &[u8]) -> Vec<usize> {
    let mut cells = Vec::with_capacity(path.len() + 1);
    let mut cur = board.start;
    cells.push(cur);
    for &dir in path {
        let next = board.adj[cur][dir_to_idx(dir)];
        debug_assert!(next != INVALID);
        cur = next;
        cells.push(cur);
    }
    cells
}

fn path_score_delta(board: &Board, cells: &[usize]) -> i32 {
    cells.iter().skip(1).map(|&c| board.scores[c]).sum()
}

fn choose_segment(rng: &mut Xoshiro256PlusPlus, len: usize) -> Option<(usize, usize)> {
    if len <= SEG_MIN {
        return None;
    }
    let seg_max = SEG_MAX.min(len);
    let seg_len = rng.random_range(SEG_MIN..=seg_max);
    let l = rng.random_range(0..=(len - seg_len));
    let r = l + seg_len;
    Some((l, r))
}

#[inline(always)]
fn is_better(len: usize, score_delta: i32, best_len: usize, best_score_delta: i32) -> bool {
    len > best_len || (len == best_len && score_delta > best_score_delta)
}

#[inline(always)]
fn objective_value(len: usize, score_delta: i32) -> f64 {
    (len as f64) * LEN_WEIGHT + (score_delta as f64)
}

fn main() {
    let board = build_board();
    let global_start = Instant::now();

    let (mut best_path, mut best_score_delta) =
        InitialSolver::new(&board, INITIAL_DFS_SEC.min(TOTAL_TIME_LIMIT_SEC)).solve();
    let best_cells = path_to_cells(&board, &best_path);
    if best_score_delta == 0 {
        best_score_delta = path_score_delta(&board, &best_cells);
    }

    let mut cur_path = best_path.clone();
    let mut cur_cells = best_cells;
    let mut cur_score_delta = best_score_delta;

    let mut used_tiles = vec![0_u8; board.tile_count];
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(((board.start as u64) << 32) ^ 0x4E5B_96F1_7C20_DA13);

    while global_start.elapsed().as_secs_f64() < TOTAL_TIME_LIMIT_SEC {
        let elapsed = global_start.elapsed().as_secs_f64();
        let progress = (elapsed / TOTAL_TIME_LIMIT_SEC).clamp(0.0, 1.0);
        let temp = SA_TEMP_START * (SA_TEMP_END / SA_TEMP_START).powf(progress);

        let len = cur_path.len();
        let Some((l, r)) = choose_segment(&mut rng, len) else {
            break;
        };

        used_tiles.fill(0);
        for &cell in cur_cells.iter().take(l + 1) {
            used_tiles[board.tiles[cell]] = 1;
        }
        for &cell in cur_cells.iter().skip(r) {
            used_tiles[board.tiles[cell]] = 1;
        }

        let a_cell = cur_cells[l];
        let b_cell = cur_cells[r];
        let old_seg_score_delta: i32 = cur_cells[l + 1..=r]
            .iter()
            .map(|&c| board.scores[c])
            .sum();
        let old_seg_len = r - l;

        let mut min_i = N - 1;
        let mut max_i = 0;
        let mut min_j = N - 1;
        let mut max_j = 0;
        for &cell in &cur_cells[l..=r] {
            let i = cell / N;
            let j = cell % N;
            min_i = min_i.min(i);
            max_i = max_i.max(i);
            min_j = min_j.min(j);
            max_j = max_j.max(j);
        }
        let margin = rng.random_range(2..=5);
        min_i = min_i.saturating_sub(margin);
        min_j = min_j.saturating_sub(margin);
        max_i = (max_i + margin).min(N - 1);
        max_j = (max_j + margin).min(N - 1);

        let mut dir_order = [0, 1, 2, 3];
        for i in 0..4 {
            let j = rng.random_range(i..4);
            dir_order.swap(i, j);
        }

        let node_limit = NODE_LIMIT_BASE + NODE_LIMIT_SCALE * (old_seg_len as u32);
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
            node_limit,
            nodes: 0,
            path: Vec::with_capacity(old_seg_len + SEG_EXTRA_LEN),
            best_path: Vec::new(),
            cur_score_delta: 0,
            best_score_delta: i32::MIN,
        };

        let Some((new_seg_path, new_seg_score_delta)) = search.solve(a_cell) else {
            continue;
        };

        let candidate_len = len - old_seg_len + new_seg_path.len();
        let candidate_score_delta = cur_score_delta - old_seg_score_delta + new_seg_score_delta;
        let delta = objective_value(candidate_len, candidate_score_delta) - objective_value(len, cur_score_delta);
        let accept = if delta >= 0.0 {
            true
        } else {
            rng.random::<f64>() < (delta / temp).exp()
        };
        if !accept {
            continue;
        }

        let mut next_path = Vec::with_capacity(candidate_len);
        next_path.extend_from_slice(&cur_path[..l]);
        next_path.extend_from_slice(&new_seg_path);
        next_path.extend_from_slice(&cur_path[r..]);

        cur_path = next_path;
        cur_cells = path_to_cells(&board, &cur_path);
        cur_score_delta = candidate_score_delta;

        if is_better(cur_path.len(), cur_score_delta, best_path.len(), best_score_delta) {
            best_path = cur_path.clone();
            best_score_delta = cur_score_delta;
        }
    }

    println!("{}", String::from_utf8(best_path).unwrap());
}
