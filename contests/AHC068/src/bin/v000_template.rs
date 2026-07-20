// v000_template.rs
use std::{
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

/// 盤面の一辺。全ケースで固定。
const N: usize = 20;
/// マス数・カード枚数。
const CELLS: usize = N * N;
/// 1 操作でカードが一軸方向へ移動できる最大距離。
const MAX_SINGLE_AXIS_SHIFT: usize = N / 2;
/// 出力できる最大操作数。
const MAX_OPERATIONS: usize = 100_000;

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

struct Input {
    initial_board: [usize; CELLS],
    vertical_walls: [[bool; N - 1]; N],
    horizontal_walls: [[bool; N]; N - 1],
}

impl Input {
    fn read() -> Self {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap();
        let mut tokens = source.split_whitespace();

        let _n: usize = tokens.next().unwrap().parse().unwrap();
        let initial_board = std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });

        Self {
            initial_board,
            vertical_walls,
            horizontal_walls,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Direction {
    fn as_char(self) -> char {
        match self {
            Self::Vertical => 'V',
            Self::Horizontal => 'H',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

fn write_output(operations: &[Operation]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    for op in operations {
        writeln!(
            writer,
            "{} {} {} {} {}",
            op.direction.as_char(),
            op.r,
            op.c,
            op.h,
            op.w,
        )
        .unwrap();
    }
}

/// 盤面走査だけで候補生成と評価を行う探索用の軽量状態。
/// 階層シフトや Kadane ベースの探索では、カード位置の逆引きを更新しない分だけ apply と clone が軽い。
/// 後続フェーズで特定カードの位置が必要になった時点で `into_state` を一度だけ呼ぶ。
#[derive(Clone)]
struct BoardState {
    /// `board[cell] = card`。値域は 0..400 なので、clone を軽くするため u16 で保持する。
    board: [u16; CELLS],
    /// 公式評価の E に相当する。
    misplaced_count: u16,
    /// 目標行と異なるカード数。縦操作だけが変化させる。
    row_mismatch_count: u16,
    /// 目標列と異なるカード数。横操作だけが変化させる。
    col_mismatch_count: u16,
}

impl BoardState {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        let board = std::array::from_fn(|cell| initial_board[cell] as u16);
        let mut misplaced_count = 0;
        let mut row_mismatch_count = 0;
        let mut col_mismatch_count = 0;

        for cell in 0..CELLS {
            let card = board[cell] as usize;
            misplaced_count += (card != cell) as u16;
            row_mismatch_count += (card / N != cell / N) as u16;
            col_mismatch_count += (card % N != cell % N) as u16;
        }

        Self {
            board,
            misplaced_count,
            row_mismatch_count,
            col_mismatch_count,
        }
    }

    #[inline]
    fn board(&self) -> &[u16; CELLS] {
        &self.board
    }

    #[inline]
    fn card_at(&self, cell: usize) -> usize {
        self.board[cell] as usize
    }

    /// `target_row - current_row` を返す。
    #[inline]
    fn row_delta_at(&self, cell: usize) -> i32 {
        (self.card_at(cell) / N) as i32 - (cell / N) as i32
    }

    /// `target_col - current_col` を返す。
    #[inline]
    fn col_delta_at(&self, cell: usize) -> i32 {
        (self.card_at(cell) % N) as i32 - (cell % N) as i32
    }

    #[inline]
    fn misplaced_count(&self) -> usize {
        self.misplaced_count as usize
    }

    #[inline]
    fn row_mismatch_count(&self) -> usize {
        self.row_mismatch_count as usize
    }

    #[inline]
    fn col_mismatch_count(&self) -> usize {
        self.col_mismatch_count as usize
    }

    #[inline]
    fn is_complete(&self) -> bool {
        self.misplaced_count == 0
    }

    fn into_state(self) -> State {
        State::from_board_state(self)
    }
}

/// 特定カードの現在位置を O(1) で引く必要がある探索用の状態。
/// カード駆動の候補生成、macro 搬送、終盤処理、chokudai search の各キュー要素にはこちらを使う。
/// 広いビームでは候補ごとに clone せず、親へ apply/undo して軽量な候補だけを集め、採用後に clone する。
/// 評価キャッシュ、Zobrist hash、親ポインタ、操作列は探索ごとに異なるため、この State の外側に置く。
#[derive(Clone)]
struct State {
    board_state: BoardState,
    /// `position[card] = cell`。値域は 0..400 なので u16 で保持する。
    position: [u16; CELLS],
}

impl State {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        Self::from_board_state(BoardState::new(initial_board))
    }

    fn from_board_state(board_state: BoardState) -> Self {
        let mut position = [0; CELLS];
        for cell in 0..CELLS {
            position[board_state.card_at(cell)] = cell as u16;
        }
        Self {
            board_state,
            position,
        }
    }

    #[inline]
    fn board(&self) -> &[u16; CELLS] {
        self.board_state.board()
    }

    #[inline]
    fn card_at(&self, cell: usize) -> usize {
        self.board_state.card_at(cell)
    }

    #[inline]
    fn position_of(&self, card: usize) -> usize {
        self.position[card] as usize
    }

    #[inline]
    fn row_delta_at(&self, cell: usize) -> i32 {
        self.board_state.row_delta_at(cell)
    }

    #[inline]
    fn col_delta_at(&self, cell: usize) -> i32 {
        self.board_state.col_delta_at(cell)
    }

    #[inline]
    fn misplaced_count(&self) -> usize {
        self.board_state.misplaced_count()
    }

    #[inline]
    fn row_mismatch_count(&self) -> usize {
        self.board_state.row_mismatch_count()
    }

    #[inline]
    fn col_mismatch_count(&self) -> usize {
        self.board_state.col_mismatch_count()
    }

    #[inline]
    fn is_complete(&self) -> bool {
        self.board_state.is_complete()
    }

    fn into_board_state(self) -> BoardState {
        self.board_state
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
    let input = Input::read();
    let _state = State::new(&input.initial_board);
    let operations = Vec::new();
    write_output(&operations);
}
