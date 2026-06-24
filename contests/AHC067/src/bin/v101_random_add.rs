// v101_random_add.rs
use proconio::{input, marker::Bytes};
use std::fmt::Write as _;
use std::time::Instant;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.88;
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

const N: usize = 20;
const M: usize = 50;
const K: usize = 10;
const CELL_COUNT: usize = N * N;
const H_EDGE_COUNT: usize = (N - 1) * N;
const EDGE_COUNT: usize = H_EDGE_COUNT + N * (N - 1);
const MASK_COUNT: usize = 1 << K;
const PACK_BASE: usize = 512;
const MAX_STATE: usize = MASK_COUNT * PACK_BASE;
const NO: u8 = u8::MAX;

#[derive(Debug, Clone, Copy)]
struct Timer {
    start: Instant,
}

impl Timer {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[inline(always)]
    fn elapsed(self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline(always)]
    fn ok(self) -> bool {
        self.elapsed() < PROGRAM_TIME_LIMIT_SEC
    }

    #[inline(always)]
    fn progress(self) -> f64 {
        (self.elapsed() / PROGRAM_TIME_LIMIT_SEC).min(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct XorShift {
    x: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        Self {
            x: if seed != 0 {
                seed
            } else {
                88_172_645_463_325_252_u64
            },
        }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.x ^= self.x << 7;
        self.x ^= self.x >> 9;
        self.x
    }

    #[inline(always)]
    fn next_int(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

fn shuffle_vec<T>(a: &mut [T], rng: &mut XorShift) {
    for i in (1..a.len()).rev() {
        let j = rng.next_int(i + 1);
        a.swap(i, j);
    }
}

#[derive(Debug, Clone, Copy)]
struct Adj {
    to: u16,
    edge: u16,
}

#[derive(Debug, Clone)]
struct Input {
    grid: [u8; CELL_COUNT],
    deg: [u8; CELL_COUNT],
    adj: [[Adj; 4]; CELL_COUNT],
    empty_cells: Vec<usize>,
    ds: [i32; CELL_COUNT],
    dg: [i32; CELL_COUNT],
}

impl Input {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            k: usize,
            rows: [Bytes; N],
        }

        assert_eq!(n, N);
        assert_eq!(m, M);
        assert_eq!(k, K);

        let mut grid = [b'#'; CELL_COUNT];
        let mut empty_cells = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), N);
            for (j, &cell) in row.iter().enumerate() {
                assert!(cell == b'.' || cell == b'#');
                let id = cell_id(i, j);
                grid[id] = cell;
                if cell == b'.' {
                    empty_cells.push(id);
                }
            }
        }

        let mut input = Self {
            grid,
            deg: [0; CELL_COUNT],
            adj: [[Adj { to: 0, edge: 0 }; 4]; CELL_COUNT],
            empty_cells,
            ds: [-1; CELL_COUNT],
            dg: [-1; CELL_COUNT],
        };

        for i in 0..N {
            for j in 0..N {
                let u = cell_id(i, j);
                if input.grid[u] != b'.' {
                    continue;
                }
                if i + 1 < N && input.grid[cell_id(i + 1, j)] == b'.' {
                    input.add_edge(u, cell_id(i + 1, j), h_edge_id(i, j));
                }
                if j + 1 < N && input.grid[cell_id(i, j + 1)] == b'.' {
                    input.add_edge(u, cell_id(i, j + 1), v_edge_id(i, j));
                }
            }
        }

        input.bfs_fill(0, true);
        input.bfs_fill(CELL_COUNT - 1, false);

        input
    }

    fn add_edge(&mut self, u: usize, v: usize, e: usize) {
        let du = self.deg[u] as usize;
        self.adj[u][du] = Adj {
            to: v as u16,
            edge: e as u16,
        };
        self.deg[u] += 1;

        let dv = self.deg[v] as usize;
        self.adj[v][dv] = Adj {
            to: u as u16,
            edge: e as u16,
        };
        self.deg[v] += 1;
    }

    fn bfs_fill(&mut self, src: usize, from_start: bool) {
        let mut dist = [-1_i32; CELL_COUNT];
        let mut que = [0_usize; CELL_COUNT];
        let mut head = 0;
        let mut tail = 0;

        dist[src] = 0;
        que[tail] = src;
        tail += 1;

        while head < tail {
            let v = que[head];
            head += 1;

            for z in 0..self.deg[v] as usize {
                let to = self.adj[v][z].to as usize;
                if dist[to] < 0 {
                    dist[to] = dist[v] + 1;
                    que[tail] = to;
                    tail += 1;
                }
            }
        }

        if from_start {
            self.ds = dist;
        } else {
            self.dg = dist;
        }
    }
}

#[inline(always)]
fn cell_id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn h_edge_id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn v_edge_id(i: usize, j: usize) -> usize {
    H_EDGE_COUNT + i * (N - 1) + j
}

#[derive(Debug, Clone)]
struct State {
    door: [u8; EDGE_COUNT],
    sw: [u8; CELL_COUNT],
    door_count: usize,
}

impl State {
    fn new() -> Self {
        Self {
            door: [NO; EDGE_COUNT],
            sw: [NO; CELL_COUNT],
            door_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Work {
    seen: Vec<i32>,
    dist: Vec<i32>,
    prev: Vec<usize>,
    que: Vec<usize>,
    seen_stamp: i32,
    edge_mask: [u16; EDGE_COUNT],
    edge_open: [u16; EDGE_COUNT],
    cand_seen: Vec<i32>,
    cand_stamp: i32,
}

impl Work {
    fn new() -> Self {
        Self {
            seen: vec![0; MAX_STATE],
            dist: vec![0; MAX_STATE],
            prev: vec![usize::MAX; MAX_STATE],
            que: vec![0; MAX_STATE],
            seen_stamp: 1,
            edge_mask: [0; EDGE_COUNT],
            edge_open: [0; EDGE_COUNT],
            cand_seen: vec![0; EDGE_COUNT * 20],
            cand_stamp: 1,
        }
    }

    fn next_seen_stamp(&mut self) {
        self.seen_stamp += 1;
        if self.seen_stamp == i32::MAX {
            self.seen.fill(0);
            self.seen_stamp = 1;
        }
    }

    fn next_cand_stamp(&mut self) {
        self.cand_stamp += 1;
        if self.cand_stamp == i32::MAX {
            self.cand_seen.fill(0);
            self.cand_stamp = 1;
        }
    }
}

fn prepare_edges(st: &State, work: &mut Work) {
    for e in 0..EDGE_COUNT {
        let g = st.door[e];
        if g == NO {
            work.edge_mask[e] = 0;
            work.edge_open[e] = 0;
        } else {
            let b = 1_u16 << (g >> 1);
            work.edge_mask[e] = b;
            work.edge_open[e] = if (g & 1) != 0 { b } else { 0 };
        }
    }
}

fn calc_t(
    in_: &Input,
    st: &State,
    mut path: Option<&mut Vec<(usize, usize)>>,
    work: &mut Work,
) -> i32 {
    prepare_edges(st, work);
    work.next_seen_stamp();

    let mut head = 0;
    let mut tail = 0;
    let stamp = work.seen_stamp;

    work.seen[0] = stamp;
    work.dist[0] = 0;
    if path.is_some() {
        work.prev[0] = usize::MAX;
    }
    work.que[tail] = 0;
    tail += 1;

    let mut goal_pack = usize::MAX;

    while head < tail {
        let pack = work.que[head];
        head += 1;

        let mask = pack >> 9;
        let v = pack & 511;
        let nd = work.dist[pack] + 1;

        if v == CELL_COUNT - 1 {
            goal_pack = pack;
            break;
        }

        for z in 0..in_.deg[v] as usize {
            let a = in_.adj[v][z];
            let e = a.edge as usize;
            if ((mask as u16) & work.edge_mask[e]) != work.edge_open[e] {
                continue;
            }

            let np = (mask << 9) | a.to as usize;
            if work.seen[np] != stamp {
                work.seen[np] = stamp;
                work.dist[np] = nd;
                if path.is_some() {
                    work.prev[np] = pack;
                }
                work.que[tail] = np;
                tail += 1;
            }
        }

        let s = st.sw[v];
        if s != NO {
            let nm = mask ^ (1_usize << s);
            let np = (nm << 9) | v;
            if work.seen[np] != stamp {
                work.seen[np] = stamp;
                work.dist[np] = nd;
                if path.is_some() {
                    work.prev[np] = pack;
                }
                work.que[tail] = np;
                tail += 1;
            }
        }
    }

    if goal_pack == usize::MAX {
        return 0;
    }

    if let Some(path) = path.as_mut() {
        path.clear();
        let mut x = goal_pack;
        while work.prev[x] != usize::MAX {
            path.push((work.prev[x], x));
            x = work.prev[x];
        }
        path.reverse();
    }

    work.dist[goal_pack]
}

#[inline(always)]
fn edge_between(in_: &Input, u: usize, v: usize) -> Option<usize> {
    for z in 0..in_.deg[u] as usize {
        let a = in_.adj[u][z];
        if a.to as usize == v {
            return Some(a.edge as usize);
        }
    }
    None
}

fn make_initial_state(in_: &Input, rng: &mut XorShift, run_id: usize) -> State {
    let mut st = State::new();
    let mut cells = Vec::with_capacity(in_.empty_cells.len());

    for &c in &in_.empty_cells {
        if c != CELL_COUNT - 1 {
            cells.push(c);
        }
    }

    match run_id % 4 {
        0 => {
            shuffle_vec(&mut cells, rng);
        }
        1 => {
            cells.sort_by(|&a, &b| {
                (in_.ds[b] + in_.dg[b])
                    .cmp(&(in_.ds[a] + in_.dg[a]))
                    .then_with(|| a.cmp(&b))
            });
            let top = cells.len().min(80);
            shuffle_vec(&mut cells[..top], rng);
        }
        2 => {
            let mut leaves = Vec::new();
            let mut rest = Vec::new();
            for c in cells {
                if in_.deg[c] <= 1 {
                    leaves.push(c);
                } else {
                    rest.push(c);
                }
            }
            shuffle_vec(&mut leaves, rng);
            shuffle_vec(&mut rest, rng);
            cells = leaves;
            cells.extend(rest);
        }
        _ => {
            cells.sort_by(|&a, &b| in_.ds[b].cmp(&in_.ds[a]).then_with(|| a.cmp(&b)));
            let top = cells.len().min(100);
            shuffle_vec(&mut cells[..top], rng);
        }
    }

    assert!(cells.len() >= K);
    for k in 0..K {
        st.sw[cells[k]] = k as u8;
    }

    st
}

#[inline(always)]
fn update_best(st: &State, t: i32, best: &mut State, best_t: &mut i32) {
    if t > *best_t {
        *best_t = t;
        *best = st.clone();
    }
}

fn greedy_run(
    in_: &Input,
    mut st: State,
    best: &mut State,
    best_t: &mut i32,
    rng: &mut XorShift,
    timer: Timer,
    work: &mut Work,
) {
    let mut current_t = calc_t(in_, &st, None, work);
    update_best(&st, current_t, best, best_t);

    let mut path = Vec::new();
    let mut candidates = Vec::with_capacity(8192);

    while st.door_count < M && timer.ok() {
        current_t = calc_t(in_, &st, Some(&mut path), work);
        if current_t <= 0 {
            break;
        }

        update_best(&st, current_t, best, best_t);
        candidates.clear();

        work.next_cand_stamp();
        let cand_stamp = work.cand_stamp;

        for &(a, b) in &path {
            let u = a & 511;
            let v = b & 511;
            if u == v {
                continue;
            }

            let Some(e) = edge_between(in_, u, v) else {
                continue;
            };
            if st.door[e] != NO {
                continue;
            }

            let mask = a >> 9;
            for k in 0..K {
                let bit = (mask >> k) & 1;
                let g = (k << 1) | (bit ^ 1);
                let code = e * 20 + g;

                if work.cand_seen[code] != cand_stamp {
                    work.cand_seen[code] = cand_stamp;
                    candidates.push(code);
                }
            }
        }

        if candidates.is_empty() {
            break;
        }

        shuffle_vec(&mut candidates, rng);

        let mut limit = 34_i32 - (16.0 * timer.progress()) as i32;
        if limit < 12 {
            limit = 12;
        }

        let eval_cap = candidates.len().min(limit as usize);
        let mut best_code = usize::MAX;
        let mut best_nt = current_t;

        for (idx, &code) in candidates.iter().enumerate() {
            if idx >= eval_cap {
                if best_code != usize::MAX {
                    break;
                }
                if idx >= limit as usize * 5 {
                    break;
                }
            }

            let e = code / 20;
            let g = code % 20;

            st.door[e] = g as u8;
            st.door_count += 1;

            let nt = calc_t(in_, &st, None, work);

            st.door[e] = NO;
            st.door_count -= 1;

            if nt > best_nt {
                best_nt = nt;
                best_code = code;
            }

            if !timer.ok() {
                break;
            }
        }

        if best_code == usize::MAX {
            break;
        }

        st.door[best_code / 20] = (best_code % 20) as u8;
        st.door_count += 1;

        current_t = best_nt;
        update_best(&st, current_t, best, best_t);
    }
}

fn print_output(st: &State) {
    let mut out = String::new();

    let mut doors = Vec::with_capacity(M);
    for e in 0..EDGE_COUNT {
        let g = st.door[e];
        if g == NO {
            continue;
        }

        if e < H_EDGE_COUNT {
            doors.push((0, e / N, e % N, g as usize));
        } else {
            let r = e - H_EDGE_COUNT;
            doors.push((1, r / (N - 1), r % (N - 1), g as usize));
        }
    }

    writeln!(&mut out, "{}", doors.len()).unwrap();
    for (d, i, j, g) in doors {
        writeln!(&mut out, "{} {} {} {}", d, i, j, g).unwrap();
    }

    let mut switches = Vec::with_capacity(K);
    for id in 0..CELL_COUNT {
        if st.sw[id] != NO {
            switches.push((id / N, id % N, st.sw[id] as usize));
        }
    }

    writeln!(&mut out, "{}", switches.len()).unwrap();
    for (i, j, s) in switches {
        writeln!(&mut out, "{} {} {}", i, j, s).unwrap();
    }

    print!("{}", out);
}

fn main() {
    let timer = Timer::new();
    let in_ = Input::read();

    let mut seed = 1_469_598_103_934_665_603_u64;
    for &cell in &in_.grid {
        seed = seed
            .wrapping_mul(1_000_003)
            .wrapping_add(cell as u64)
            .wrapping_add(97);
    }

    let mut rng = XorShift::new(seed);
    let mut work = Work::new();

    let mut best = State::new();
    let mut best_t = calc_t(&in_, &best, None, &mut work);

    let mut run = 0;
    while timer.ok() {
        let st = make_initial_state(&in_, &mut rng, run);
        run += 1;
        greedy_run(&in_, st, &mut best, &mut best_t, &mut rng, timer, &mut work);
    }

    print_output(&best);
}
