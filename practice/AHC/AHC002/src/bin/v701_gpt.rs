// v701_gpt.rs
use proconio::input;
use smallvec::SmallVec;
use std::time::Instant;

const N: usize = 50;
const V: usize = N * N;
const TIME_LIMIT_SEC: f64 = 1.90;
const TIME_GUARD_SEC: f64 = 0.02;

#[derive(Debug, Clone)]
pub struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
    iter: u64,
    check_mask: u64,
    elapsed_sec: f64,
    progress: f64,
    is_over: bool,
}

impl TimeKeeper {
    pub fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
        assert!(time_limit_sec > 0.0);
        assert!(check_interval_log2 < 63);
        let check_mask = if check_interval_log2 == 0 {
            0
        } else {
            (1_u64 << check_interval_log2) - 1
        };
        let mut tk = Self {
            start: Instant::now(),
            time_limit_sec,
            iter: 0,
            check_mask,
            elapsed_sec: 0.0,
            progress: 0.0,
            is_over: false,
        };
        tk.force_update();
        tk
    }

    #[inline(always)]
    pub fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    #[inline(always)]
    pub fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    #[inline(always)]
    pub fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    #[inline(always)]
    pub fn is_time_over(&self) -> bool {
        self.is_over
    }

    #[inline]
    pub fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Degree,
    Random,
}

#[derive(Clone, Copy, Debug)]
struct Task {
    first: usize,
    mode: Mode,
    seed: u64,
    weight: usize,
}

struct Solver {
    start: usize,
    tile_of: Vec<usize>,
    score: Vec<i32>,
    partner: Vec<usize>,
    neigh: Vec<SmallVec<[usize; 4]>>,

    used: Vec<u8>,
    path: Vec<usize>,
    best_path: Vec<usize>,
    best_score: i32,

    cell_seen: Vec<u32>,
    tile_seen: Vec<u32>,
    tile_best: Vec<i32>,
    bfs_queue: Vec<usize>,
    bfs_stamp: u32,
}

impl Solver {
    fn new(
        start: usize,
        tile_of: Vec<usize>,
        score: Vec<i32>,
        partner: Vec<usize>,
        neigh: Vec<SmallVec<[usize; 4]>>,
        m: usize,
    ) -> Self {
        Self {
            start,
            tile_of,
            score,
            partner,
            neigh,
            used: vec![0; m],
            path: Vec::with_capacity(V),
            best_path: vec![start],
            best_score: 0,
            cell_seen: vec![0; V],
            tile_seen: vec![0; m],
            tile_best: vec![0; m],
            bfs_queue: Vec::with_capacity(V),
            bfs_stamp: 1,
        }
    }

    #[inline(always)]
    fn available_degree(&self, v: usize) -> i32 {
        let mut deg = 0;
        for &to in &self.neigh[v] {
            if self.used[self.tile_of[to]] == 0 {
                deg += 1;
            }
        }
        deg
    }

    fn upper_bound_from(&mut self, cur: usize) -> i32 {
        self.bfs_stamp = self.bfs_stamp.wrapping_add(1);
        if self.bfs_stamp == 0 {
            self.cell_seen.fill(0);
            self.tile_seen.fill(0);
            self.bfs_stamp = 1;
        }
        let stamp = self.bfs_stamp;
        self.bfs_queue.clear();
        self.bfs_queue.push(cur);
        self.cell_seen[cur] = stamp;
        let cur_tile = self.tile_of[cur];
        self.tile_seen[cur_tile] = stamp;
        self.tile_best[cur_tile] = self.score[cur];
        let mut sum = self.score[cur];
        let banned_partner = self.partner[cur];
        let mut head = 0;
        while head < self.bfs_queue.len() {
            let v = self.bfs_queue[head];
            head += 1;
            for &to in &self.neigh[v] {
                if to == banned_partner {
                    continue;
                }
                let tid = self.tile_of[to];
                if self.used[tid] != 0 && to != cur {
                    continue;
                }
                if self.cell_seen[to] != stamp {
                    self.cell_seen[to] = stamp;
                    self.bfs_queue.push(to);
                }
                let s = self.score[to];
                if self.tile_seen[tid] != stamp {
                    self.tile_seen[tid] = stamp;
                    self.tile_best[tid] = s;
                    sum += s;
                } else if s > self.tile_best[tid] {
                    sum += s - self.tile_best[tid];
                    self.tile_best[tid] = s;
                }
            }
        }
        sum - self.score[cur]
    }

    fn update_best(&mut self, cur_score: i32) {
        if cur_score > self.best_score {
            self.best_score = cur_score;
            self.best_path.clear();
            self.best_path.extend_from_slice(&self.path);
        }
    }

    fn dfs(
        &mut self,
        cur: usize,
        cur_score: i32,
        tk: &mut TimeKeeper,
        task_deadline_sec: f64,
        mode: Mode,
        seed: u64,
    ) {
        if !tk.step() || tk.elapsed_sec() >= task_deadline_sec {
            return;
        }

        let ub_total = cur_score + self.upper_bound_from(cur);
        if ub_total <= self.best_score {
            return;
        }

        let neighbors = self.neigh[cur].clone();
        let mut cand: SmallVec<[(i32, i32, i32, u64, usize); 4]> = SmallVec::new();
        for &to in &neighbors {
            let tid = self.tile_of[to];
            if self.used[tid] != 0 {
                continue;
            }
            self.used[tid] = 1;
            let add_score = self.score[to];
            let next_score = cur_score + add_score;
            let ub2 = next_score + self.upper_bound_from(to);
            let deg = self.available_degree(to);
            let rand_key =
                splitmix64(seed ^ ((self.path.len() as u64) << 32) ^ ((cur as u64) << 16) ^ to as u64);
            self.used[tid] = 0;
            cand.push((ub2, deg, add_score, rand_key, to));
        }

        if cand.is_empty() {
            self.update_best(cur_score);
            return;
        }

        cand.sort_unstable_by(|a, b| match mode {
            Mode::Degree => b
                .0
                .cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| b.3.cmp(&a.3)),
            Mode::Random => b
                .0
                .cmp(&a.0)
                .then_with(|| b.3.cmp(&a.3))
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2)),
        });

        for (ub2, _deg, add_score, _rand_key, to) in cand {
            if ub2 <= self.best_score {
                continue;
            }
            let tid = self.tile_of[to];
            self.used[tid] = 1;
            self.path.push(to);
            self.dfs(to, cur_score + add_score, tk, task_deadline_sec, mode, seed);
            self.path.pop();
            self.used[tid] = 0;
            if tk.is_time_over() || tk.elapsed_sec() >= task_deadline_sec {
                return;
            }
        }
    }

    fn search_from_first(
        &mut self,
        first: usize,
        tk: &mut TimeKeeper,
        task_deadline_sec: f64,
        mode: Mode,
        seed: u64,
    ) {
        self.used.fill(0);
        self.path.clear();
        let start_tile = self.tile_of[self.start];
        let first_tile = self.tile_of[first];
        self.used[start_tile] = 1;
        self.used[first_tile] = 1;
        self.path.push(self.start);
        self.path.push(first);
        let start_score = self.score[self.start];
        let first_score = self.score[first];
        let init_score = start_score + first_score;
        self.dfs(first, init_score, tk, task_deadline_sec, mode, seed);
    }
}

#[inline(always)]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

fn main() {
    input! {
        si: usize,
        sj: usize,
        tiles: [[usize; N]; N],
        ps: [[i32; N]; N],
    }

    let start = si * N + sj;
    let mut m = 0usize;
    for i in 0..N {
        for j in 0..N {
            m = m.max(tiles[i][j] + 1);
        }
    }

    let mut tile_cells = vec![SmallVec::<[usize; 2]>::new(); m];
    let mut tile_of = vec![0usize; V];
    let mut score = vec![0i32; V];
    let mut neigh = vec![SmallVec::<[usize; 4]>::new(); V];

    for i in 0..N {
        for j in 0..N {
            let id = i * N + j;
            let tid = tiles[i][j];
            tile_cells[tid].push(id);
            tile_of[id] = tid;
            score[id] = ps[i][j];
        }
    }

    let mut partner = vec![0usize; V];
    for cells in &tile_cells {
        match cells.len() {
            1 => {
                let c = cells[0];
                partner[c] = c;
            }
            2 => {
                let a = cells[0];
                let b = cells[1];
                partner[a] = b;
                partner[b] = a;
            }
            _ => unreachable!(),
        }
    }

    for i in 0..N {
        for j in 0..N {
            let id = i * N + j;
            if i > 0 && tiles[i - 1][j] != tiles[i][j] {
                neigh[id].push((i - 1) * N + j);
            }
            if i + 1 < N && tiles[i + 1][j] != tiles[i][j] {
                neigh[id].push((i + 1) * N + j);
            }
            if j > 0 && tiles[i][j - 1] != tiles[i][j] {
                neigh[id].push(i * N + (j - 1));
            }
            if j + 1 < N && tiles[i][j + 1] != tiles[i][j] {
                neigh[id].push(i * N + (j + 1));
            }
        }
    }

    let mut solver = Solver::new(start, tile_of, score, partner, neigh, m);
    let start_score_only = solver.score[start];
    solver.best_score = start_score_only;
    solver.best_path = vec![start];

    let mut first_moves: Vec<(i32, i32, i32, usize)> = Vec::new();
    let start_tile = solver.tile_of[start];
    let start_neighbors = solver.neigh[start].clone();
    for &to in &start_neighbors {
        let tid = solver.tile_of[to];
        solver.used.fill(0);
        solver.used[start_tile] = 1;
        solver.used[tid] = 1;
        let start_score = solver.score[start];
        let to_score = solver.score[to];
        let ub = start_score + to_score + solver.upper_bound_from(to);
        let deg = solver.available_degree(to);
        first_moves.push((ub, deg, to_score, to));
    }
    first_moves.sort_unstable_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| b.2.cmp(&a.2))
    });

    if first_moves.is_empty() {
        println!();
        return;
    }

    let mut tasks: Vec<Task> = Vec::new();
    for entry in &first_moves {
        let first = entry.3;
        tasks.push(Task {
            first,
            mode: Mode::Degree,
            seed: 0x1234_5678_9ABC_DEF0 ^ first as u64,
            weight: 3,
        });
    }
    for (idx, entry) in first_moves.iter().enumerate() {
        let first = entry.3;
        tasks.push(Task {
            first,
            mode: Mode::Random,
            seed: 0x9E37_79B9_7F4A_7C15 ^ ((idx as u64) << 32) ^ first as u64,
            weight: 2,
        });
    }

    let mut tk = TimeKeeper::new(TIME_LIMIT_SEC, 8);
    let mut remaining_weight: usize = tasks.iter().map(|t| t.weight).sum();
    for task in tasks {
        tk.force_update();
        let remaining = TIME_LIMIT_SEC - tk.exact_elapsed_sec() - TIME_GUARD_SEC;
        if remaining <= 0.0 {
            break;
        }
        let budget = remaining * (task.weight as f64) / (remaining_weight as f64);
        let task_deadline_sec = (tk.exact_elapsed_sec() + budget).min(TIME_LIMIT_SEC - TIME_GUARD_SEC);
        solver.search_from_first(task.first, &mut tk, task_deadline_sec, task.mode, task.seed);
        remaining_weight = remaining_weight.saturating_sub(task.weight);
    }

    let mut ans = String::with_capacity(solver.best_path.len().saturating_sub(1));
    for w in solver.best_path.windows(2) {
        let a = w[0];
        let b = w[1];
        let ai = a / N;
        let aj = a % N;
        let bi = b / N;
        let bj = b % N;
        let ch = if bi + 1 == ai {
            'U'
        } else if bi == ai + 1 {
            'D'
        } else if bj + 1 == aj {
            'L'
        } else {
            'R'
        };
        ans.push(ch);
    }
    println!("{}", ans);
}
