// v000_template.rs
use proconio::{input, marker::Chars};
use std::time::Instant;

const MAX_N: usize = 20;
const MAX_M: usize = 40;
const MAX_CELLS: usize = MAX_N * MAX_N;
const NONE: u8 = 255;
const OP_F: u8 = 0;
const OP_R: u8 = 1;
const OP_L: u8 = 2;
const OP_S: u8 = 3;
const BTN_F: u8 = OP_F;
const BTN_R: u8 = OP_R;
const BTN_L: u8 = OP_L;
const BTN_S: u8 = OP_S;
const BTN_M: u8 = 4;
const BTN_P: u8 = 5;

#[derive(Debug, Clone)]
struct Input {
    n: usize,
    m: usize,
    t_limit: usize,

    // cell -> 壁がある方向 bitmask。外周は含めない。
    // dir: 0=上, 1=右, 2=下, 3=左。
    wall_mask: [u8; MAX_CELLS],

    // cell -> 初期状態で置かれているボール種類。ボールなしは NONE。
    init_ball_at: [u8; MAX_CELLS],

    // cell -> そのマスにあるかご種類。かごなしは NONE。
    basket_at: [u8; MAX_CELLS],

    // k -> 種類 k のボール初期 cell。
    ball_pos: [u16; MAX_M],

    // k -> 種類 k のかご cell。
    basket_pos: [u16; MAX_M],
}

impl Input {
    fn read() -> Self {
        input! {
            n: usize,
            m: usize,
            t_limit: usize,
            wall_v_raw: [Chars; n],
            wall_h_raw: [Chars; n - 1],
            bcde: [(usize, usize, usize, usize); m],
        }

        let cell = |i: usize, j: usize| -> usize { i * n + j };

        let mut wall_mask = [0u8; MAX_CELLS];
        for i in 0..n {
            for j in 0..n - 1 {
                if wall_v_raw[i][j] == '1' {
                    let left = cell(i, j);
                    let right = cell(i, j + 1);
                    wall_mask[left] |= 1 << 1;
                    wall_mask[right] |= 1 << 3;
                }
            }
        }
        for i in 0..n - 1 {
            for j in 0..n {
                if wall_h_raw[i][j] == '1' {
                    let up = cell(i, j);
                    let down = cell(i + 1, j);
                    wall_mask[up] |= 1 << 2;
                    wall_mask[down] |= 1 << 0;
                }
            }
        }

        let mut init_ball_at = [NONE; MAX_CELLS];
        let mut basket_at = [NONE; MAX_CELLS];
        let mut ball_pos = [0u16; MAX_M];
        let mut basket_pos = [0u16; MAX_M];
        for (k, &(b, c, d, e)) in bcde.iter().enumerate() {
            let ball_cell = cell(b, c);
            let basket_cell = cell(d, e);
            init_ball_at[ball_cell] = k as u8;
            basket_at[basket_cell] = k as u8;
            ball_pos[k] = ball_cell as u16;
            basket_pos[k] = basket_cell as u16;
        }

        Self {
            n,
            m,
            t_limit,
            wall_mask,
            init_ball_at,
            basket_at,
            ball_pos,
            basket_pos,
        }
    }
}

#[derive(Debug, Clone)]
struct Grid {
    n: usize,
    cell_count: usize,

    // dir: 0=上, 1=右, 2=下, 3=左。
    // cell に足す 1 次元差分。
    dir_delta: [i16; 4],

    // cell -> 外周に当たらず進める方向 bitmask。壁は考慮しない。
    edge_mask: [u8; MAX_CELLS],

    // cell -> 壁がある方向 bitmask。外周は含めない。
    wall_mask: [u8; MAX_CELLS],

    // cell -> 外周・壁を両方考慮して進める方向 bitmask。
    move_mask: [u8; MAX_CELLS],

    // cell, dir -> 1 手前進後の cell。外周または壁で進めない場合は同じ cell。
    next_cell: [[u16; 4]; MAX_CELLS],
}

impl Grid {
    fn new(input: &Input) -> Self {
        let n = input.n;
        let cell_count = n * n;
        let dir_delta = [-(n as i16), 1, n as i16, -1];

        let mut edge_mask = [0u8; MAX_CELLS];
        let wall_mask = input.wall_mask;
        let mut move_mask = [0u8; MAX_CELLS];
        let mut next_cell = [[0u16; 4]; MAX_CELLS];

        for cell in 0..cell_count {
            let i = cell / n;
            let j = cell % n;

            let mut edge = 0u8;
            if i > 0 {
                edge |= 1 << 0;
            }
            if j + 1 < n {
                edge |= 1 << 1;
            }
            if i + 1 < n {
                edge |= 1 << 2;
            }
            if j > 0 {
                edge |= 1 << 3;
            }

            edge_mask[cell] = edge;
            move_mask[cell] = edge & !wall_mask[cell];

            for dir in 0..4 {
                next_cell[cell][dir] = if move_mask[cell] & (1 << dir) != 0 {
                    (cell as i16 + dir_delta[dir]) as u16
                } else {
                    cell as u16
                };
            }
        }

        Self {
            n,
            cell_count,
            dir_delta,
            edge_mask,
            wall_mask,
            move_mask,
            next_cell,
        }
    }

    #[inline(always)]
    fn cell(&self, i: usize, j: usize) -> usize {
        i * self.n + j
    }

    #[inline(always)]
    fn ij(&self, cell: usize) -> (usize, usize) {
        (cell / self.n, cell % self.n)
    }

    #[inline(always)]
    fn moved_unchecked(&self, cell: usize, dir: usize) -> u16 {
        (cell as i16 + self.dir_delta[dir]) as u16
    }

    #[inline(always)]
    fn hits_edge(&self, cell: usize, dir: usize) -> bool {
        self.edge_mask[cell] & (1 << dir) == 0
    }

    #[inline(always)]
    fn has_wall(&self, cell: usize, dir: usize) -> bool {
        self.wall_mask[cell] & (1 << dir) != 0
    }

    #[inline(always)]
    fn can_move(&self, cell: usize, dir: usize) -> bool {
        self.move_mask[cell] & (1 << dir) != 0
    }

    #[inline(always)]
    fn next(&self, cell: usize, dir: usize) -> u16 {
        self.next_cell[cell][dir]
    }
}

#[derive(Debug, Clone)]
struct State {
    // ロボットの現在 cell。
    pos: u16,

    // dir: 0=上, 1=右, 2=下, 3=左。
    dir: u8,

    // 手に持っているボール種類。持っていない場合は NONE。
    held: u8,

    // 対応するかご上にあるボール数。
    matched: u8,

    // cell -> そのマスに置かれているボール種類。ボールなしは NONE。
    cell_ball: [u8; MAX_CELLS],

    // マクロ展開後に実行済みの基本操作数。
    basic_count: usize,

    // 現在マクロを記録中か。
    recording: bool,

    // 最後に登録が完了したマクロ。要素は OP_F, OP_R, OP_L, OP_S のみ。
    last_macro: Vec<u8>,

    // 現在記録中のマクロ。要素は OP_F, OP_R, OP_L, OP_S のみ。
    cur_macro: Vec<u8>,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut matched = 0u8;
        for k in 0..input.m {
            let cell = input.basket_pos[k] as usize;
            if input.init_ball_at[cell] == k as u8 {
                matched += 1;
            }
        }

        Self {
            pos: 0,
            dir: 1,
            held: NONE,
            matched,
            cell_ball: input.init_ball_at,
            basic_count: 0,
            recording: false,
            last_macro: Vec::new(),
            cur_macro: Vec::new(),
        }
    }

    fn press_button(&mut self, input: &Input, grid: &Grid, button: u8) {
        if self.basic_count >= input.t_limit {
            return;
        }

        match button {
            OP_F | OP_R | OP_L | OP_S => {
                self.execute_basic(input, grid, button);
            }
            BTN_M => {
                self.toggle_recording();
            }
            BTN_P => {
                self.replay_last_macro(input, grid);
            }
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn toggle_recording(&mut self) {
        if self.recording {
            std::mem::swap(&mut self.last_macro, &mut self.cur_macro);
            self.cur_macro.clear();
            self.recording = false;
        } else {
            self.cur_macro.clear();
            self.recording = true;
        }
    }

    fn replay_last_macro(&mut self, input: &Input, grid: &Grid) {
        let mut idx = 0;
        while idx < self.last_macro.len() {
            let op = self.last_macro[idx];
            if !self.execute_basic(input, grid, op) {
                break;
            }
            idx += 1;
        }
    }

    #[inline(always)]
    fn execute_basic(&mut self, input: &Input, grid: &Grid, op: u8) -> bool {
        if self.basic_count >= input.t_limit {
            return false;
        }

        self.apply_basic(input, grid, op);
        self.basic_count += 1;

        if self.recording {
            self.cur_macro.push(op);
        }

        true
    }

    #[inline(always)]
    fn apply_basic(&mut self, input: &Input, grid: &Grid, op: u8) {
        match op {
            OP_F => {
                self.pos = grid.next(self.pos as usize, self.dir as usize);
            }
            OP_R => {
                self.dir = (self.dir + 1) & 3;
            }
            OP_L => {
                self.dir = (self.dir + 3) & 3;
            }
            OP_S => {
                self.apply_swap(input);
            }
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn apply_swap(&mut self, input: &Input) {
        let cell = self.pos as usize;
        let old = self.cell_ball[cell];
        let new = self.held;

        if is_correct_at(input, cell, old) {
            self.matched -= 1;
        }

        self.cell_ball[cell] = new;
        self.held = old;

        if is_correct_at(input, cell, new) {
            self.matched += 1;
        }
    }
}

#[inline(always)]
fn is_correct_at(input: &Input, cell: usize, ball: u8) -> bool {
    ball != NONE && input.basket_at[cell] == ball
}

#[inline(always)]
fn absolute_score(input: &Input, state: &State, answer_len: usize) -> usize {
    if state.matched as usize == input.m {
        answer_len
    } else {
        input.t_limit * (input.m - state.matched as usize)
    }
}

#[derive(Debug, Clone, Default)]
struct ButtonSeq {
    buttons: Vec<u8>,
}

impl ButtonSeq {
    fn new() -> Self {
        Self {
            buttons: Vec::new(),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            buttons: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    fn push(&mut self, button: u8) {
        self.buttons.push(button);
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<u8> {
        self.buttons.pop()
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.buttons.len()
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.buttons.is_empty()
    }

    fn clear(&mut self) {
        self.buttons.clear();
    }

    fn simulate(&self, input: &Input, grid: &Grid) -> State {
        let mut state = State::new(input);
        for &button in &self.buttons {
            state.press_button(input, grid, button);
        }
        state
    }

    fn absolute_score(&self, input: &Input, grid: &Grid) -> usize {
        let state = self.simulate(input, grid);
        absolute_score(input, &state, self.len())
    }

    fn to_output_string(&self) -> String {
        let mut out = String::with_capacity(self.buttons.len() * 2);
        for &button in &self.buttons {
            out.push(button_to_char(button));
            out.push('\n');
        }
        out
    }

    fn print(&self) {
        print!("{}", self.to_output_string());
    }
}

#[inline(always)]
fn button_to_char(button: u8) -> char {
    match button {
        BTN_F => 'F',
        BTN_R => 'R',
        BTN_L => 'L',
        BTN_S => 'S',
        BTN_M => 'M',
        BTN_P => 'P',
        _ => unreachable!(),
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

fn main() {}
