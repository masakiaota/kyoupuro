// v000_template.rs
use proconio::{input, marker::Bytes};
use std::fmt::Write as _;
use std::time::Instant;

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
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
const DOOR_KIND_COUNT: usize = 2 * K;
const H_DOOR_COUNT: usize = (N - 1) * N;
const V_DOOR_COUNT: usize = N * (N - 1);
const EDGE_COUNT: usize = H_DOOR_COUNT + V_DOOR_COUNT;
const MASK_COUNT: usize = 1 << K;
const HERO_STATE_COUNT: usize = MASK_COUNT * CELL_COUNT;

const START_ID: usize = 0;
const GOAL_ID: usize = CELL_COUNT - 1;

const NO_DOOR: u8 = u8::MAX;
const NO_SWITCH: u8 = u8::MAX;
const UNREACHED: i32 = -1;
const SCORE_SCALE: f64 = 1_000_000.0;

const DIRECTIONS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[inline(always)]
fn h_edge_id(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn v_edge_id(i: usize, j: usize) -> usize {
    H_DOOR_COUNT + i * (N - 1) + j
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdjEdge {
    to: u16,
    edge_id: u16,
}

impl AdjEdge {
    const INVALID: Self = Self {
        to: u16::MAX,
        edge_id: u16::MAX,
    };
}

#[derive(Debug, Clone)]
struct Input {
    grid: [u8; CELL_COUNT],
    empty_ids: Vec<usize>,
    neighbors: [[usize; 4]; CELL_COUNT],
    adj_edges: [[AdjEdge; 4]; CELL_COUNT],
    neighbor_count: [u8; CELL_COUNT],
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
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), N);
            for (j, &cell) in row.iter().enumerate() {
                assert!(cell == b'.' || cell == b'#');
                grid[Self::id(i, j)] = cell;
            }
        }

        Self::new(grid)
    }

    fn new(grid: [u8; CELL_COUNT]) -> Self {
        assert_eq!(grid[START_ID], b'.');
        assert_eq!(grid[GOAL_ID], b'.');

        let mut empty_ids = Vec::new();
        for id in 0..CELL_COUNT {
            if grid[id] == b'.' {
                empty_ids.push(id);
            }
        }

        let mut neighbors = [[usize::MAX; 4]; CELL_COUNT];
        let mut adj_edges = [[AdjEdge::INVALID; 4]; CELL_COUNT];
        let mut neighbor_count = [0_u8; CELL_COUNT];
        for id in 0..CELL_COUNT {
            if grid[id] != b'.' {
                continue;
            }
            let (i, j) = Self::ij(id);
            for &(di, dj) in &DIRECTIONS {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if ni < 0 || ni >= N as isize || nj < 0 || nj >= N as isize {
                    continue;
                }
                let nid = Self::id(ni as usize, nj as usize);
                if grid[nid] != b'.' {
                    continue;
                }
                let idx = neighbor_count[id] as usize;
                let edge_id = match (di, dj) {
                    (1, 0) => h_edge_id(i, j),
                    (-1, 0) => h_edge_id(ni as usize, nj as usize),
                    (0, 1) => v_edge_id(i, j),
                    (0, -1) => v_edge_id(ni as usize, nj as usize),
                    _ => unreachable!(),
                };
                neighbors[id][idx] = nid;
                adj_edges[id][idx] = AdjEdge {
                    to: nid as u16,
                    edge_id: edge_id as u16,
                };
                neighbor_count[id] += 1;
            }
        }

        Self {
            grid,
            empty_ids,
            neighbors,
            adj_edges,
            neighbor_count,
        }
    }

    #[inline(always)]
    fn id(i: usize, j: usize) -> usize {
        i * N + j
    }

    #[inline(always)]
    fn ij(id: usize) -> (usize, usize) {
        (id / N, id % N)
    }

    #[inline(always)]
    fn is_empty_id(&self, id: usize) -> bool {
        self.grid[id] == b'.'
    }

    #[inline(always)]
    fn is_empty(&self, i: usize, j: usize) -> bool {
        self.is_empty_id(Self::id(i, j))
    }

    #[inline(always)]
    fn neighbor_ids(&self, id: usize) -> &[usize] {
        &self.neighbors[id][..self.neighbor_count[id] as usize]
    }

    #[inline(always)]
    fn adj_edges(&self, id: usize) -> &[AdjEdge] {
        &self.adj_edges[id][..self.neighbor_count[id] as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoorDir {
    Down,
    Right,
}

impl DoorDir {
    #[inline(always)]
    fn output_value(self) -> usize {
        match self {
            DoorDir::Down => 0,
            DoorDir::Right => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Door {
    dir: DoorDir,
    i: usize,
    j: usize,
    g: usize,
}

impl Door {
    fn new(dir: DoorDir, i: usize, j: usize, g: usize) -> Self {
        Self { dir, i, j, g }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SwitchPlacement {
    i: usize,
    j: usize,
    s: usize,
}

impl SwitchPlacement {
    fn new(i: usize, j: usize, s: usize) -> Self {
        Self { i, j, s }
    }
}

#[derive(Debug, Clone)]
struct Output {
    doors: Vec<Door>,
    switches: Vec<SwitchPlacement>,
}

impl Output {
    fn empty() -> Self {
        Self {
            doors: Vec::new(),
            switches: Vec::new(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.doors.len() > M {
            return Err(format!("too many doors: {} > {}", self.doors.len(), M));
        }
        if self.switches.len() > CELL_COUNT {
            return Err(format!(
                "too many switches: {} > {}",
                self.switches.len(),
                CELL_COUNT
            ));
        }

        let mut seen_h = [false; H_DOOR_COUNT];
        let mut seen_v = [false; V_DOOR_COUNT];
        for &door in &self.doors {
            Self::validate_door(door)?;
            match door.dir {
                DoorDir::Down => {
                    let idx = Self::h_door_index(door.i, door.j);
                    if seen_h[idx] {
                        return Err(format!(
                            "duplicated down door edge at ({},{})",
                            door.i, door.j
                        ));
                    }
                    seen_h[idx] = true;
                }
                DoorDir::Right => {
                    let idx = Self::v_door_index(door.i, door.j);
                    if seen_v[idx] {
                        return Err(format!(
                            "duplicated right door edge at ({},{})",
                            door.i, door.j
                        ));
                    }
                    seen_v[idx] = true;
                }
            }
        }

        let mut seen_switch = [false; CELL_COUNT];
        for &sw in &self.switches {
            Self::validate_switch(sw)?;
            let id = Input::id(sw.i, sw.j);
            if seen_switch[id] {
                return Err(format!("duplicated switch cell at ({},{})", sw.i, sw.j));
            }
            seen_switch[id] = true;
        }

        Ok(())
    }

    fn door_grids(&self) -> Result<DoorGrids, String> {
        let mut grids = DoorGrids::empty();
        let mut seen_h = [false; H_DOOR_COUNT];
        let mut seen_v = [false; V_DOOR_COUNT];

        for &door in &self.doors {
            Self::validate_door(door)?;
            match door.dir {
                DoorDir::Down => {
                    let idx = Self::h_door_index(door.i, door.j);
                    if seen_h[idx] {
                        return Err(format!(
                            "duplicated down door edge at ({},{})",
                            door.i, door.j
                        ));
                    }
                    seen_h[idx] = true;
                    grids.h[idx] = door.g as i8;
                }
                DoorDir::Right => {
                    let idx = Self::v_door_index(door.i, door.j);
                    if seen_v[idx] {
                        return Err(format!(
                            "duplicated right door edge at ({},{})",
                            door.i, door.j
                        ));
                    }
                    seen_v[idx] = true;
                    grids.v[idx] = door.g as i8;
                }
            }
        }

        Ok(grids)
    }

    fn switch_grid(&self) -> Result<[i8; CELL_COUNT], String> {
        let mut grid = [-1_i8; CELL_COUNT];

        for &sw in &self.switches {
            Self::validate_switch(sw)?;
            let id = Input::id(sw.i, sw.j);
            if grid[id] != -1 {
                return Err(format!("duplicated switch cell at ({},{})", sw.i, sw.j));
            }
            grid[id] = sw.s as i8;
        }

        Ok(grid)
    }

    fn to_output_string(&self) -> String {
        let mut out = String::new();

        writeln!(&mut out, "{}", self.doors.len()).unwrap();
        for door in &self.doors {
            writeln!(
                &mut out,
                "{} {} {} {}",
                door.dir.output_value(),
                door.i,
                door.j,
                door.g
            )
            .unwrap();
        }

        writeln!(&mut out, "{}", self.switches.len()).unwrap();
        for sw in &self.switches {
            writeln!(&mut out, "{} {} {}", sw.i, sw.j, sw.s).unwrap();
        }

        out
    }

    fn print(&self) {
        print!("{}", self.to_output_string());
    }

    #[inline(always)]
    fn h_door_index(i: usize, j: usize) -> usize {
        i * N + j
    }

    #[inline(always)]
    fn v_door_index(i: usize, j: usize) -> usize {
        i * (N - 1) + j
    }

    fn validate_door(door: Door) -> Result<(), String> {
        if door.g >= DOOR_KIND_COUNT {
            return Err(format!(
                "invalid door type at ({},{}): {}",
                door.i, door.j, door.g
            ));
        }

        match door.dir {
            DoorDir::Down => {
                if door.i >= N - 1 || door.j >= N {
                    return Err(format!(
                        "invalid down door coordinate: ({},{})",
                        door.i, door.j
                    ));
                }
            }
            DoorDir::Right => {
                if door.i >= N || door.j >= N - 1 {
                    return Err(format!(
                        "invalid right door coordinate: ({},{})",
                        door.i, door.j
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_switch(sw: SwitchPlacement) -> Result<(), String> {
        if sw.i >= N || sw.j >= N {
            return Err(format!("invalid switch coordinate: ({},{})", sw.i, sw.j));
        }
        if sw.s >= K {
            return Err(format!(
                "invalid switch type at ({},{}): {}",
                sw.i, sw.j, sw.s
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DoorGrids {
    h: [i8; H_DOOR_COUNT],
    v: [i8; V_DOOR_COUNT],
}

impl DoorGrids {
    fn empty() -> Self {
        Self {
            h: [-1_i8; H_DOOR_COUNT],
            v: [-1_i8; V_DOOR_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CutoffEvalResult {
    Reached { t: usize },
    NotReachedWithinCutoff,
}

#[derive(Debug, Clone)]
struct CastlePlan {
    door_code_by_edge: [u8; EDGE_COUNT],
    switch_by_cell: [u8; CELL_COUNT],
    door_count: u8,
    switch_count: u16,
}

impl CastlePlan {
    fn empty() -> Self {
        Self {
            door_code_by_edge: [NO_DOOR; EDGE_COUNT],
            switch_by_cell: [NO_SWITCH; CELL_COUNT],
            door_count: 0,
            switch_count: 0,
        }
    }

    fn from_output(output: &Output) -> Result<Self, String> {
        output.validate()?;

        let mut plan = Self::empty();
        plan.door_count = output.doors.len() as u8;
        plan.switch_count = output.switches.len() as u16;

        for &door in &output.doors {
            let edge_id = match door.dir {
                DoorDir::Down => h_edge_id(door.i, door.j),
                DoorDir::Right => v_edge_id(door.i, door.j),
            };
            plan.door_code_by_edge[edge_id] = door.g as u8;
        }

        for &sw in &output.switches {
            let id = Input::id(sw.i, sw.j);
            plan.switch_by_cell[id] = sw.s as u8;
        }

        Ok(plan)
    }

    fn to_output(&self) -> Output {
        let mut doors = Vec::with_capacity(self.door_count as usize);
        for edge_id in 0..H_DOOR_COUNT {
            let g = self.door_code_by_edge[edge_id];
            if g != NO_DOOR {
                doors.push(Door::new(
                    DoorDir::Down,
                    edge_id / N,
                    edge_id % N,
                    g as usize,
                ));
            }
        }
        for edge_id in H_DOOR_COUNT..EDGE_COUNT {
            let g = self.door_code_by_edge[edge_id];
            if g != NO_DOOR {
                let rem = edge_id - H_DOOR_COUNT;
                doors.push(Door::new(
                    DoorDir::Right,
                    rem / (N - 1),
                    rem % (N - 1),
                    g as usize,
                ));
            }
        }

        let mut switches = Vec::with_capacity(self.switch_count as usize);
        for id in 0..CELL_COUNT {
            let s = self.switch_by_cell[id];
            if s != NO_SWITCH {
                let (i, j) = Input::ij(id);
                switches.push(SwitchPlacement::new(i, j, s as usize));
            }
        }

        Output { doors, switches }
    }

    fn calc_t(&self, input: &Input) -> usize {
        let mut scratch = EvalScratch::new();
        self.calc_t_with_scratch(input, &mut scratch)
    }

    fn calc_t_with_scratch(&self, input: &Input, scratch: &mut EvalScratch) -> usize {
        self.prepare_eval(scratch);
        scratch.clear_bfs();

        let edge_mask = &scratch.edge_mask;
        let edge_open = &scratch.edge_open;
        let dist = &mut scratch.dist;
        let queue = &mut scratch.queue;

        dist[START_ID] = 0;
        queue.push(Self::pack_hero_node(0, START_ID));

        let mut head = 0;
        while head < queue.len() {
            let packed = queue[head];
            head += 1;

            let mask = (packed >> 16) as usize;
            let id = (packed & 0xffff) as usize;
            let dist_idx = mask * CELL_COUNT + id;
            let d = dist[dist_idx];

            if id == GOAL_ID {
                return d as usize;
            }

            for &adj in input.adj_edges(id) {
                let edge_id = adj.edge_id as usize;
                if ((mask as u16) & edge_mask[edge_id]) != edge_open[edge_id] {
                    continue;
                }

                let to = adj.to as usize;
                let next_idx = mask * CELL_COUNT + to;
                if dist[next_idx] == UNREACHED {
                    dist[next_idx] = d + 1;
                    queue.push(Self::pack_hero_node(mask, to));
                }
            }

            let s = self.switch_by_cell[id];
            if s != NO_SWITCH {
                let next_mask = mask ^ (1_usize << s as usize);
                let next_idx = next_mask * CELL_COUNT + id;
                if dist[next_idx] == UNREACHED {
                    dist[next_idx] = d + 1;
                    queue.push(Self::pack_hero_node(next_mask, id));
                }
            }
        }

        0
    }

    fn calc_t_with_cutoff(
        &self,
        input: &Input,
        scratch: &mut EvalScratch,
        cutoff: usize,
    ) -> CutoffEvalResult {
        self.prepare_eval(scratch);
        scratch.clear_bfs();

        let edge_mask = &scratch.edge_mask;
        let edge_open = &scratch.edge_open;
        let dist = &mut scratch.dist;
        let queue = &mut scratch.queue;
        let cutoff = cutoff as i32;

        dist[START_ID] = 0;
        queue.push(Self::pack_hero_node(0, START_ID));

        let mut head = 0;
        while head < queue.len() {
            let packed = queue[head];
            head += 1;

            let mask = (packed >> 16) as usize;
            let id = (packed & 0xffff) as usize;
            let dist_idx = mask * CELL_COUNT + id;
            let d = dist[dist_idx];

            if id == GOAL_ID {
                return CutoffEvalResult::Reached { t: d as usize };
            }
            if d >= cutoff {
                continue;
            }

            for &adj in input.adj_edges(id) {
                let edge_id = adj.edge_id as usize;
                if ((mask as u16) & edge_mask[edge_id]) != edge_open[edge_id] {
                    continue;
                }

                let to = adj.to as usize;
                let next_idx = mask * CELL_COUNT + to;
                if dist[next_idx] == UNREACHED {
                    dist[next_idx] = d + 1;
                    queue.push(Self::pack_hero_node(mask, to));
                }
            }

            let s = self.switch_by_cell[id];
            if s != NO_SWITCH {
                let next_mask = mask ^ (1_usize << s as usize);
                let next_idx = next_mask * CELL_COUNT + id;
                if dist[next_idx] == UNREACHED {
                    dist[next_idx] = d + 1;
                    queue.push(Self::pack_hero_node(next_mask, id));
                }
            }
        }

        CutoffEvalResult::NotReachedWithinCutoff
    }

    fn score(&self, input: &Input) -> i64 {
        let mut scratch = EvalScratch::new();
        self.score_with_scratch(input, &mut scratch)
    }

    fn score_with_scratch(&self, input: &Input, scratch: &mut EvalScratch) -> i64 {
        Self::score_from_t(self.calc_t_with_scratch(input, scratch))
    }

    fn score_from_t(t: usize) -> i64 {
        if t == 0 {
            1
        } else {
            (SCORE_SCALE * (t as f64 / N as f64).log2()).round() as i64
        }
    }

    fn prepare_eval(&self, scratch: &mut EvalScratch) {
        for edge_id in 0..EDGE_COUNT {
            let g = self.door_code_by_edge[edge_id];
            if g == NO_DOOR {
                scratch.edge_mask[edge_id] = 0;
                scratch.edge_open[edge_id] = 0;
            } else {
                let bit = 1_u16 << (g as usize / 2);
                scratch.edge_mask[edge_id] = bit;
                scratch.edge_open[edge_id] = if (g & 1) == 1 { bit } else { 0 };
            }
        }
    }

    #[inline(always)]
    fn pack_hero_node(mask: usize, id: usize) -> u32 {
        ((mask as u32) << 16) | id as u32
    }
}

#[derive(Debug, Clone)]
struct EvalScratch {
    edge_mask: [u16; EDGE_COUNT],
    edge_open: [u16; EDGE_COUNT],
    dist: Vec<i32>,
    queue: Vec<u32>,
}

impl EvalScratch {
    fn new() -> Self {
        Self {
            edge_mask: [0; EDGE_COUNT],
            edge_open: [0; EDGE_COUNT],
            dist: vec![UNREACHED; HERO_STATE_COUNT],
            queue: Vec::with_capacity(HERO_STATE_COUNT),
        }
    }

    fn clear_bfs(&mut self) {
        self.dist.fill(UNREACHED);
        self.queue.clear();
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn mark_fallback(&mut self) {
        self.fallback_count += 1;
    }

    fn count(&mut self, key: &'static str) {
        self.count_by(key, 1);
    }

    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }

    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }

    fn summary(&self) {
        eprintln!("[summary] fallback_count={}", self.fallback_count);
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {}={}", key, value);
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {}={:.3}", key, value);
        }
    }
}

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let __local_time_start = std::time::Instant::now();
        let __local_time_result = { $body };
        $trace.add_time_ms($key, __local_time_start.elapsed().as_secs_f64() * 1000.0);
        __local_time_result
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,

    iter: u64,
    check_mask: u64,

    elapsed_sec: f64,
    progress: f64,
    is_over: bool,
}

impl TimeKeeper {
    /// `check_interval_log2 = 8` なら 2^8 = 256 反復ごとに時計更新
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
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

    /// ホットループではこれだけ呼ぶ
    /// true: 継続, false: 打ち切り
    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    /// 明示的に時計を更新したいときに使う
    #[inline(always)]
    fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    /// batched な経過時間
    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    /// batched な進捗率 [0, 1]
    #[inline(always)]
    fn progress(&self) -> f64 {
        self.progress
    }

    /// batched な時間切れ判定
    #[inline(always)]
    fn is_time_over(&self) -> bool {
        self.is_over
    }

    /// ログ用の正確な経過時間
    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// ログ用の正確な残り時間
    #[inline]
    fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn main() {
    // TimeKeeper は main 開始直後に作り、探索打ち切りには PROGRAM_TIME_LIMIT_SEC を使う。
    // フェーズ切替などの時間系パラメータは PROGRAM_TIME_LIMIT_SEC に対する割合で指定する。
    let _time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);
}
