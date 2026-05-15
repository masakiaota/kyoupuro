// v000_template.rs
use std::io::BufRead;
use std::io::Write;
use std::time::Instant;

const N: usize = 30;
const Q: usize = 1000;
const H_EDGE_ROWS: usize = N;
const H_EDGE_COLS: usize = N - 1;
const V_EDGE_ROWS: usize = N - 1;
const V_EDGE_COLS: usize = N;
const H_EDGE_COUNT: usize = H_EDGE_ROWS * H_EDGE_COLS;
const V_EDGE_COUNT: usize = V_EDGE_ROWS * V_EDGE_COLS;
const EDGE_COUNT: usize = H_EDGE_COUNT + V_EDGE_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryInput {
    s: Point,
    t: Point,
}

impl QueryInput {
    fn read<R: BufRead>(reader: &mut R) -> Self {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let mut it = line.split_whitespace();
        let si = it.next().unwrap().parse::<usize>().unwrap();
        let sj = it.next().unwrap().parse::<usize>().unwrap();
        let ti = it.next().unwrap().parse::<usize>().unwrap();
        let tj = it.next().unwrap().parse::<usize>().unwrap();

        Self {
            s: Point::new(si, sj),
            t: Point::new(ti, tj),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservedLength {
    value: i64,
}

impl ObservedLength {
    fn read<R: BufRead>(reader: &mut R) -> Self {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let value = line.trim().parse::<i64>().unwrap();

        Self { value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    U,
    D,
    L,
    R,
}

impl Dir {
    const ALL: [Dir; 4] = [Dir::U, Dir::D, Dir::L, Dir::R];

    fn delta(self) -> (isize, isize) {
        match self {
            Dir::U => (-1, 0),
            Dir::D => (1, 0),
            Dir::L => (0, -1),
            Dir::R => (0, 1),
        }
    }

    fn to_char(self) -> char {
        match self {
            Dir::U => 'U',
            Dir::D => 'D',
            Dir::L => 'L',
            Dir::R => 'R',
        }
    }

    fn reverse(self) -> Self {
        match self {
            Dir::U => Dir::D,
            Dir::D => Dir::U,
            Dir::L => Dir::R,
            Dir::R => Dir::L,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Output {
    dirs: Vec<Dir>,
}

impl Output {
    fn new(dirs: Vec<Dir>) -> Self {
        Self { dirs }
    }

    fn write<W: Write>(&self, writer: &mut W) {
        for &dir in &self.dirs {
            write!(writer, "{}", dir.to_char()).unwrap();
        }
        writeln!(writer).unwrap();
        writer.flush().unwrap();
    }

    fn to_string(&self) -> String {
        self.dirs.iter().map(|&dir| dir.to_char()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    i: usize,
    j: usize,
}

impl Point {
    fn new(i: usize, j: usize) -> Self {
        debug_assert!(i < N);
        debug_assert!(j < N);
        Self { i, j }
    }
}

#[derive(Debug, Clone)]
struct State {
    /// 0-indexed の現在クエリ番号。
    /// `QueryInput` は `k` を持たないので、オンライン進行状態としてここで管理する。
    turn: usize,
    // 未確定: 各横辺 h[i][j] の推定値。
    // estimated_h: [[f64; H_EDGE_COLS]; H_EDGE_ROWS],
    //
    // 未確定: 各縦辺 v[i][j] の推定値。
    // estimated_v: [[f64; V_EDGE_COLS]; V_EDGE_ROWS],
    //
    // 未確定: 観測済みクエリ履歴。
    // history: Vec<Observation>,
    //
    // 未確定: 生成分布のゆらぎ幅 D の推定。
    // estimated_d: f64,
    //
    // 未確定: M=1/2 の推定。
    // estimated_m: usize,
    //
    // 未確定: M=2 の場合の横辺分割位置 x_i の推定。
    // estimated_x: [usize; H_EDGE_ROWS],
    //
    // 未確定: M=2 の場合の縦辺分割位置 y_j の推定。
    // estimated_y: [usize; V_EDGE_COLS],
    //
    // 未確定: 行ごとの横辺基準値 H_i,p の推定。
    // estimated_h_base: [[f64; 2]; H_EDGE_ROWS],
    //
    // 未確定: 列ごとの縦辺基準値 V_j,p の推定。
    // estimated_v_base: [[f64; 2]; V_EDGE_COLS],
}

impl State {
    fn new() -> Self {
        Self {
            turn: 0,
            // estimated_h: [[5000.0; H_EDGE_COLS]; H_EDGE_ROWS],
            // estimated_v: [[5000.0; V_EDGE_COLS]; V_EDGE_ROWS],
            // history: Vec::with_capacity(Q),
            // estimated_d: 1000.0,
            // estimated_m: 1,
            // estimated_x: [H_EDGE_COLS / 2; H_EDGE_ROWS],
            // estimated_y: [V_EDGE_ROWS / 2; V_EDGE_COLS],
            // estimated_h_base: [[5000.0; 2]; H_EDGE_ROWS],
            // estimated_v_base: [[5000.0; 2]; V_EDGE_COLS],
        }
    }

    fn update(&mut self, _input: QueryInput, _output: &Output, _observed: ObservedLength) {
        // 未確定: 観測値 round(b_k * e_k) を使って推定状態を更新する。
        // 直接辺推定・行列基準値推定・履歴からの再推定などの戦略をここに入れる。

        self.turn += 1;
    }
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
