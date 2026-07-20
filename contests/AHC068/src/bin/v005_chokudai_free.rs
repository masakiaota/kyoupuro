// v005_chokudai_free.rs
// 自由 δ(1..=10)の帯 Kadane 候補を深さ別キューの chokudai サーチで展開し、
// E の記録を大きく更新したノードだけに葉剥がし tail を実走して、prefix+tail の
// 総手数が最小の完成列を anytime に保持する。開始時に初期盤面へ tail を実走する
// ため、どの時点で打ち切っても E=0 の出力を持つ(fallback 分岐ではなく初期化)。
// 中心アイデア: 階層固定(v002/v003)を捨て、大小の操作を全深さで同時に候補化し、
// 時間を使い切る探索で単手貪欲の停滞と粗密固定の終盤押し込みを回避する。

use std::collections::HashSet;
use std::io::{self, BufWriter, Read, Write};
use std::time::Instant;

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_SHIFT: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

// 探索パラメタ。solver 実行後の調整はユーザー指示を要するため、初版は固定値で登録する。
/// 深さ別キューの容量。
const BEAM_CAPACITY: usize = 64;
/// 1 ノード展開で生成する子の数(帯 Kadane 候補の上位)。
const EXPAND_CHILDREN: usize = 12;
/// prefix(探索部)の最大手数。tail 上界 79,800 と合わせても 10^5 に収まる。
const MAX_PREFIX_DEPTH: usize = 320;
/// E がこの幅以上で記録更新したときだけ tail を実走する(tail の回数を E/GAP 回に抑える)。
const TAIL_GAP: i32 = 8;
/// 全時間のうち chokudai 探索に使う割合。残りは終端 tail・検証・出力に確保する。
const SEARCH_TIME_RATIO: f64 = 0.93;

// ---------- 入出力 ----------

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

/// 操作は 21bit に詰めて arena と操作列で持つ。dir(1) | r(5) | c(5) | h(5) | w(5)。
#[inline]
fn encode_op(is_vertical: bool, r: usize, c: usize, h: usize, w: usize) -> u32 {
    ((is_vertical as u32) << 20) | ((r as u32) << 15) | ((c as u32) << 10) | ((h as u32) << 5) | (w as u32)
}

#[inline]
fn decode_op(op: u32) -> (bool, usize, usize, usize, usize) {
    (
        (op >> 20) & 1 == 1,
        ((op >> 15) & 31) as usize,
        ((op >> 10) & 31) as usize,
        ((op >> 5) & 31) as usize,
        (op & 31) as usize,
    )
}

fn write_output(operations: &[u32]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for &op in operations {
        let (is_v, r, c, h, w) = decode_op(op);
        writeln!(writer, "{} {} {} {} {}", if is_v { 'V' } else { 'H' }, r, c, h, w).unwrap();
    }
}

// ---------- トレース ----------

#[cfg(feature = "local")]
#[derive(Debug, Default)]
struct TraceStats {
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }
    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }
    fn summary(&self) {
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {key}={value}");
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {key}={value:.3}");
        }
    }
}

#[cfg(feature = "local")]
macro_rules! local {
    ($($body:tt)*) => {{ $($body)* }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

// ---------- 時間管理(v000_template と同じ) ----------

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

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

// ---------- 乱数 ----------

struct XorShift64 {
    s: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { s: seed.max(1) }
    }
    #[inline]
    fn next(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x << 7;
        x ^= x >> 9;
        self.s = x;
        x
    }
}

// ---------- 壁の前計算 ----------

/// 帯(細長長方形)の合法性を O(1) で判定するための累積和。
/// 添字はすべて「区間 [a,b) の壁数 = prefix[b] - prefix[a]」の形で使う。
struct Walls {
    /// hp_col[j][i] = Σ_{i'<i} H[i'][j](列 j を縦に走る H 壁の累積)
    hp_col: [[u16; N]; N],
    /// vp_col[j][i] = Σ_{i'<i} V[i'][j](列境界 j|j+1 を縦に走る V 壁の累積)
    vp_col: [[u16; N + 1]; N - 1],
    /// vp_row[i][j] = Σ_{j'<j} V[i][j'](行 i を横に走る V 壁の累積)
    vp_row: [[u16; N]; N],
    /// hp_row[i][j] = Σ_{j'<j} H[i][j'](行境界 i|i+1 を横に走る H 壁の累積)
    hp_row: [[u16; N + 1]; N - 1],
}

impl Walls {
    fn new(input: &Input) -> Self {
        let mut hp_col = [[0u16; N]; N];
        for j in 0..N {
            for i in 0..N - 1 {
                hp_col[j][i + 1] = hp_col[j][i] + input.horizontal_walls[i][j] as u16;
            }
        }
        let mut vp_col = [[0u16; N + 1]; N - 1];
        for j in 0..N - 1 {
            for i in 0..N {
                vp_col[j][i + 1] = vp_col[j][i] + input.vertical_walls[i][j] as u16;
            }
        }
        let mut vp_row = [[0u16; N]; N];
        for i in 0..N {
            for j in 0..N - 1 {
                vp_row[i][j + 1] = vp_row[i][j] + input.vertical_walls[i][j] as u16;
            }
        }
        let mut hp_row = [[0u16; N + 1]; N - 1];
        for i in 0..N - 1 {
            for j in 0..N {
                hp_row[i][j + 1] = hp_row[i][j] + input.horizontal_walls[i][j] as u16;
            }
        }
        Self {
            hp_col,
            vp_col,
            vp_row,
            hp_row,
        }
    }

    /// 列 j が縦帯 [r, r+hgt) の柱として使えるか(内部 H 壁ゼロ)。
    #[inline]
    fn v_pillar_ok(&self, j: usize, r: usize, hgt: usize) -> bool {
        self.hp_col[j][r + hgt - 1] == self.hp_col[j][r]
    }
    /// 列境界 j|j+1 が縦帯 [r, r+hgt) 内で通行可か(V 壁ゼロ)。
    #[inline]
    fn v_boundary_ok(&self, j: usize, r: usize, hgt: usize) -> bool {
        self.vp_col[j][r + hgt] == self.vp_col[j][r]
    }
    /// 行 i が横帯 [c, c+wid) の梁として使えるか(内部 V 壁ゼロ)。
    #[inline]
    fn h_beam_ok(&self, i: usize, c: usize, wid: usize) -> bool {
        self.vp_row[i][c + wid - 1] == self.vp_row[i][c]
    }
    /// 行境界 i|i+1 が横帯 [c, c+wid) 内で通行可か(H 壁ゼロ)。
    #[inline]
    fn h_boundary_ok(&self, i: usize, c: usize, wid: usize) -> bool {
        self.hp_row[i][c + wid] == self.hp_row[i][c]
    }
}

// ---------- 盤面状態 ----------

#[inline]
fn needs_of(d: i32) -> i32 {
    if d == 0 {
        0
    } else if d.abs() <= MAX_SHIFT as i32 {
        1
    } else {
        2
    }
}

#[inline]
fn row_delta(card: usize, cell: usize) -> i32 {
    (card / N) as i32 - (cell / N) as i32
}

#[inline]
fn col_delta(card: usize, cell: usize) -> i32 {
    (card % N) as i32 - (cell % N) as i32
}

/// 探索ノードが持つ最小状態。needs は「残り必要シフト回数の総和」で、
/// 行 needs は縦操作だけが、列 needs は横操作だけが変化させる(OBS-C8)。
#[derive(Clone)]
struct Board {
    cells: [u16; CELLS],
    row_needs: i32,
    col_needs: i32,
    e: i32,
    hash: u64,
}

impl Board {
    fn new(initial_board: &[usize; CELLS], zobrist: &[u64]) -> Self {
        let cells: [u16; CELLS] = std::array::from_fn(|cell| initial_board[cell] as u16);
        let mut row_needs = 0;
        let mut col_needs = 0;
        let mut e = 0;
        let mut hash = 0u64;
        for cell in 0..CELLS {
            let card = cells[cell] as usize;
            row_needs += needs_of(row_delta(card, cell));
            col_needs += needs_of(col_delta(card, cell));
            e += (card != cell) as i32;
            hash ^= zobrist[card * CELLS + cell];
        }
        Self {
            cells,
            row_needs,
            col_needs,
            e,
            hash,
        }
    }

    #[inline]
    fn needs_sum(&self) -> i32 {
        self.row_needs + self.col_needs
    }

    /// 1 ペア (a,b) のカードを交換し、needs・E・hash を差分更新する。
    /// axis_row=true なら行 needs、false なら列 needs のみ変わる。
    #[inline]
    fn swap_pair(&mut self, a: usize, b: usize, axis_row: bool, zobrist: &[u64]) {
        let ca = self.cells[a] as usize;
        let cb = self.cells[b] as usize;
        if axis_row {
            self.row_needs -= needs_of(row_delta(ca, a)) + needs_of(row_delta(cb, b));
            self.row_needs += needs_of(row_delta(ca, b)) + needs_of(row_delta(cb, a));
        } else {
            self.col_needs -= needs_of(col_delta(ca, a)) + needs_of(col_delta(cb, b));
            self.col_needs += needs_of(col_delta(ca, b)) + needs_of(col_delta(cb, a));
        }
        self.e -= (ca != a) as i32 + (cb != b) as i32;
        self.e += (ca != b) as i32 + (cb != a) as i32;
        self.hash ^= zobrist[ca * CELLS + a]
            ^ zobrist[ca * CELLS + b]
            ^ zobrist[cb * CELLS + b]
            ^ zobrist[cb * CELLS + a];
        self.cells.swap(a, b);
    }

    fn apply(&mut self, op: u32, zobrist: &[u64]) {
        let (is_v, r, c, h, w) = decode_op(op);
        if is_v {
            let half = h / 2;
            for x in 0..half {
                for j in c..c + w {
                    let a = (r + x) * N + j;
                    self.swap_pair(a, a + half * N, true, zobrist);
                }
            }
        } else {
            let half = w / 2;
            for i in r..r + h {
                for y in 0..half {
                    let a = i * N + c + y;
                    self.swap_pair(a, a + half, false, zobrist);
                }
            }
        }
    }
}

// ---------- 候補生成(帯ごとの Kadane) ----------

#[derive(Clone, Copy)]
struct Candidate {
    score: i32,
    op: u32,
    tie: u32,
}

/// 縦帯・横帯 200 本それぞれについて、needs 純益(+E 純益の微小項)を最大化する
/// (c,w) または (r,h) を非空 Kadane で求め、pool に帯ごとの最良候補を積む。
/// 純益が負の帯も最小劣化の候補として残す(停滞脱出は探索側が判断する)。
fn generate_candidates(board: &Board, walls: &Walls, rng: &mut XorShift64, pool: &mut Vec<Candidate>) {
    pool.clear();
    let cells = &board.cells;

    // 縦帯: シフト量 delta、上端 r。列 j の柱スコア = 帯内 delta ペアの複合益の和。
    for delta in 1..=MAX_SHIFT {
        let hgt = 2 * delta;
        for r in 0..=(N - hgt) {
            let mut best_score = i32::MIN;
            let mut best_c = 0;
            let mut best_w = 0;
            let mut cur_score = 0;
            let mut cur_start = 0;
            let mut in_segment = false;
            for j in 0..N {
                // 柱が違法なら区間が切れる。境界壁でも切れる。
                let pillar = walls.v_pillar_ok(j, r, hgt);
                let connected = in_segment && j > 0 && walls.v_boundary_ok(j - 1, r, hgt);
                if !pillar {
                    in_segment = false;
                    continue;
                }
                // 複合益: 行 needs 純益を主、E 純益を従(16:1)で 1 列ぶん集計する。
                let mut col_score = 0;
                for x in 0..delta {
                    let a = (r + x) * N + j;
                    let b = a + delta * N;
                    let ca = cells[a] as usize;
                    let cb = cells[b] as usize;
                    let needs_gain = needs_of(row_delta(ca, a)) + needs_of(row_delta(cb, b))
                        - needs_of(row_delta(ca, b))
                        - needs_of(row_delta(cb, a));
                    let e_gain = (ca != a) as i32 + (cb != b) as i32
                        - (ca != b) as i32
                        - (cb != a) as i32;
                    col_score += needs_gain * 16 + e_gain;
                }
                // 非空 Kadane: 直前区間に繋げるか、この列から新規に始めるか。
                if connected && cur_score > 0 {
                    cur_score += col_score;
                } else {
                    cur_score = col_score;
                    cur_start = j;
                }
                in_segment = true;
                if cur_score > best_score {
                    best_score = cur_score;
                    best_c = cur_start;
                    best_w = j - cur_start + 1;
                }
            }
            if best_score > i32::MIN {
                pool.push(Candidate {
                    score: best_score,
                    op: encode_op(true, r, best_c, hgt, best_w),
                    tie: rng.next() as u32,
                });
            }
        }
    }

    // 横帯: シフト量 delta、左端 c。行 i の梁スコアで対称に。
    for delta in 1..=MAX_SHIFT {
        let wid = 2 * delta;
        for c in 0..=(N - wid) {
            let mut best_score = i32::MIN;
            let mut best_r = 0;
            let mut best_h = 0;
            let mut cur_score = 0;
            let mut cur_start = 0;
            let mut in_segment = false;
            for i in 0..N {
                let beam = walls.h_beam_ok(i, c, wid);
                let connected = in_segment && i > 0 && walls.h_boundary_ok(i - 1, c, wid);
                if !beam {
                    in_segment = false;
                    continue;
                }
                let mut row_score = 0;
                for y in 0..delta {
                    let a = i * N + c + y;
                    let b = a + delta;
                    let ca = cells[a] as usize;
                    let cb = cells[b] as usize;
                    let needs_gain = needs_of(col_delta(ca, a)) + needs_of(col_delta(cb, b))
                        - needs_of(col_delta(ca, b))
                        - needs_of(col_delta(cb, a));
                    let e_gain = (ca != a) as i32 + (cb != b) as i32
                        - (ca != b) as i32
                        - (cb != a) as i32;
                    row_score += needs_gain * 16 + e_gain;
                }
                if connected && cur_score > 0 {
                    cur_score += row_score;
                } else {
                    cur_score = row_score;
                    cur_start = i;
                }
                in_segment = true;
                if cur_score > best_score {
                    best_score = cur_score;
                    best_r = cur_start;
                    best_h = i - cur_start + 1;
                }
            }
            if best_score > i32::MIN {
                pool.push(Candidate {
                    score: best_score,
                    op: encode_op(false, best_r, c, best_h, wid),
                    tie: rng.next() as u32,
                });
            }
        }
    }

    // 上位 EXPAND_CHILDREN 件に絞る。score 降順、同点は乱数で多様化する。
    pool.sort_unstable_by(|a, b| b.score.cmp(&a.score).then(a.tie.cmp(&b.tie)));
    pool.truncate(EXPAND_CHILDREN);
}

// ---------- 葉剥がし tail(v001_baseline の部品化) ----------

/// 全域木と削除順は壁のみに依存するため 1 回だけ前計算する。
struct Tail {
    parent: Vec<usize>,
    /// BFS 発見順。逆順に処理すると各対象が残りの木で葉になる。
    order: Vec<usize>,
}

impl Tail {
    fn new(input: &Input) -> Self {
        let mut graph = vec![Vec::new(); CELLS];
        for r in 0..N {
            for c in 0..N - 1 {
                if !input.vertical_walls[r][c] {
                    graph[r * N + c].push(r * N + c + 1);
                    graph[r * N + c + 1].push(r * N + c);
                }
            }
        }
        for r in 0..N - 1 {
            for c in 0..N {
                if !input.horizontal_walls[r][c] {
                    graph[r * N + c].push((r + 1) * N + c);
                    graph[(r + 1) * N + c].push(r * N + c);
                }
            }
        }
        let mut parent = vec![usize::MAX; CELLS];
        let mut order = Vec::with_capacity(CELLS);
        parent[0] = 0;
        order.push(0);
        let mut head = 0;
        while head < order.len() {
            let v = order[head];
            head += 1;
            for &to in &graph[v] {
                if parent[to] == usize::MAX {
                    parent[to] = v;
                    order.push(to);
                }
            }
        }
        assert_eq!(order.len(), CELLS, "盤面の隣接グラフが連結でない");
        Self { parent, order }
    }

    /// 任意の盤面から完成までの葉剥がし操作列を ops へ追記する。
    fn run(&self, start_cells: &[u16; CELLS], ops: &mut Vec<u32>) {
        let mut board: [u16; CELLS] = *start_cells;
        let mut position = [0u16; CELLS];
        for cell in 0..CELLS {
            position[board[cell] as usize] = cell as u16;
        }
        // 全域木上の経路復元用バッファは使い回す。
        let mut from_index = [usize::MAX; CELLS];
        let mut from_chain: Vec<usize> = Vec::with_capacity(64);
        let mut to_chain: Vec<usize> = Vec::with_capacity(64);

        for &target in self.order.iter().rev() {
            if target == 0 {
                break;
            }
            let source = position[target] as usize;
            if source == target {
                continue;
            }
            // start -> root のチェーンを張り、goal 側から LCA まで遡って経路を得る。
            from_chain.clear();
            to_chain.clear();
            let mut v = source;
            loop {
                from_index[v] = from_chain.len();
                from_chain.push(v);
                if self.parent[v] == v {
                    break;
                }
                v = self.parent[v];
            }
            let mut v = target;
            while from_index[v] == usize::MAX {
                to_chain.push(v);
                v = self.parent[v];
            }
            let lca_pos = from_index[v];
            // 経路に沿って隣接 swap を並べる。
            let mut prev = source;
            let mut emit = |u: usize, w: usize, ops: &mut Vec<u32>| {
                let (ru, cu) = (u / N, u % N);
                let (rw, cw) = (w / N, w % N);
                let op = if ru == rw {
                    encode_op(false, ru, cu.min(cw), 1, 2)
                } else {
                    encode_op(true, ru.min(rw), cu, 2, 1)
                };
                ops.push(op);
                let card_u = board[u];
                let card_w = board[w];
                board.swap(u, w);
                position[card_u as usize] = w as u16;
                position[card_w as usize] = u as u16;
            };
            for step in 1..=lca_pos {
                let next = from_chain[step];
                emit(prev, next, ops);
                prev = next;
            }
            for &next in to_chain.iter().rev() {
                emit(prev, next, ops);
                prev = next;
            }
            debug_assert_eq!(board[target] as usize, target);
            // from_index を汚したまま次の反復に入らないよう掃除する。
            for &u in &from_chain {
                from_index[u] = usize::MAX;
            }
        }
        debug_assert!((0..CELLS).all(|cell| board[cell] as usize == cell));
    }
}

// ---------- chokudai サーチ ----------

struct Node {
    board: Board,
    arena: u32,
    tie: u32,
}

#[inline]
fn node_key(n: &Node) -> (i32, i32, u32) {
    (n.board.needs_sum(), n.board.e, n.tie)
}

fn pop_best(queue: &mut Vec<Node>) -> Option<Node> {
    if queue.is_empty() {
        return None;
    }
    let mut best = 0;
    for i in 1..queue.len() {
        if node_key(&queue[i]) < node_key(&queue[best]) {
            best = i;
        }
    }
    Some(queue.swap_remove(best))
}

fn push_capped(queue: &mut Vec<Node>, node: Node) -> bool {
    if queue.len() < BEAM_CAPACITY {
        queue.push(node);
        return true;
    }
    let mut worst = 0;
    for i in 1..queue.len() {
        if node_key(&queue[i]) > node_key(&queue[worst]) {
            worst = i;
        }
    }
    if node_key(&node) < node_key(&queue[worst]) {
        queue[worst] = node;
        true
    } else {
        false
    }
}

fn reconstruct_prefix(arena: &[(u32, u32)], mut idx: u32) -> Vec<u32> {
    let mut ops = Vec::new();
    while idx != 0 {
        let (parent, op) = arena[idx as usize];
        ops.push(op);
        idx = parent;
    }
    ops.reverse();
    ops
}

fn main() {
    let time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 6);
    let input = Input::read();
    let walls = Walls::new(&input);
    let tail = Tail::new(&input);

    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();

    // Zobrist 表(card, cell)。シードは固定して再現性を保つ。
    let mut rng = XorShift64::new(0x51ce_b00c_5eed_2026);
    let zobrist: Vec<u64> = (0..CELLS * CELLS).map(|_| rng.next()).collect();

    let root = Board::new(&input.initial_board, &zobrist);

    // anytime の初期化: 初期盤面から葉剥がしを実走し、完成列を必ず 1 本持つ。
    let mut best_ops: Vec<u32> = Vec::new();
    tail.run(&root.cells, &mut best_ops);
    // best の prefix 手数。local のトレース専用だが、cfg 分岐を避けるため常に更新する。
    #[allow(unused_assignments)]
    let mut best_prefix_len = 0usize;
    local! {
        trace.count_by("tail_runs", 1);
        trace.count_by("initial_tail_len", best_ops.len() as i64);
    }

    // 深さ別キュー・重複排除・親リンク arena。
    let mut queues: Vec<Vec<Node>> = (0..=MAX_PREFIX_DEPTH).map(|_| Vec::new()).collect();
    let mut seen: Vec<HashSet<u64>> = (0..=MAX_PREFIX_DEPTH).map(|_| HashSet::new()).collect();
    let mut arena: Vec<(u32, u32)> = vec![(0, 0)]; // 0 番は root の番兵。

    seen[0].insert(root.hash);
    let root_e = root.e;
    queues[0].push(Node {
        board: root,
        arena: 0,
        tie: rng.next() as u32,
    });

    let mut tail_gate = root_e;
    let mut tk = time_keeper;
    let search_limit = PROGRAM_TIME_LIMIT_SEC * SEARCH_TIME_RATIO;
    let mut pool: Vec<Candidate> = Vec::with_capacity(256);

    let mut running = true;
    while running {
        let mut progressed = false;
        for t in 0..MAX_PREFIX_DEPTH {
            if !tk.step() || tk.exact_elapsed_sec() >= search_limit {
                running = false;
                break;
            }
            let Some(node) = pop_best(&mut queues[t]) else {
                continue;
            };
            progressed = true;
            local! { trace.count_by("expanded", 1); }

            generate_candidates(&node.board, &walls, &mut rng, &mut pool);
            for cand in pool.iter() {
                let mut child = node.board.clone();
                child.apply(cand.op, &zobrist);
                if !seen[t + 1].insert(child.hash) {
                    local! { trace.count_by("dup_hits", 1); }
                    continue;
                }
                let arena_idx = arena.len() as u32;
                arena.push((node.arena, cand.op));

                if child.e == 0 {
                    // 探索単独で完成。prefix そのものが完成列になる。
                    let ops = reconstruct_prefix(&arena, arena_idx);
                    if ops.len() < best_ops.len() {
                        best_prefix_len = ops.len();
                        best_ops = ops;
                        local! {
                            trace.count_by("best_updates", 1);
                            trace.count_by("complete_by_search", 1);
                        }
                    }
                    continue;
                }

                // E の記録を大きく更新したノードにだけ tail を実走し、総手数でベストを争う。
                if child.e + TAIL_GAP <= tail_gate {
                    tail_gate = child.e;
                    let mut ops = reconstruct_prefix(&arena, arena_idx);
                    tail.run(&child.cells, &mut ops);
                    local! { trace.count_by("tail_runs", 1); }
                    if ops.len() < best_ops.len() {
                        best_prefix_len = (t + 1).min(ops.len());
                        best_ops = ops;
                        local! { trace.count_by("best_updates", 1); }
                    }
                }

                push_capped(
                    &mut queues[t + 1],
                    Node {
                        board: child,
                        arena: arena_idx,
                        tie: rng.next() as u32,
                    },
                );
            }
        }
        if !progressed {
            break;
        }
        local! { trace.count_by("rounds", 1); }
    }

    // 打ち切り時: キューに残った E 最良ノード 1 つに最終 tail を当て、取りこぼしを防ぐ。
    let mut final_best: Option<(i32, i32, usize, u32)> = None;
    for t in 0..=MAX_PREFIX_DEPTH {
        for node in &queues[t] {
            let key = (node.board.e, node.board.needs_sum());
            if final_best.is_none_or(|(be, bn, _, _)| key < (be, bn)) {
                final_best = Some((node.board.e, node.board.needs_sum(), t, node.arena));
                // cells が要るので arena とは別に board を引けるよう t 内 index でも良いが、
                // ここでは直後にもう一度走査して盤面を取り出す。
            }
        }
    }
    if let Some((be, _bn, bt, barena)) = final_best {
        if be + 1 <= tail_gate {
            if let Some(node) = queues[bt].iter().find(|n| n.arena == barena) {
                let mut ops = reconstruct_prefix(&arena, barena);
                tail.run(&node.board.cells, &mut ops);
                local! { trace.count_by("tail_runs", 1); }
                if ops.len() < best_ops.len() {
                    best_prefix_len = bt.min(ops.len());
                    best_ops = ops;
                    local! { trace.count_by("best_updates", 1); }
                }
            }
        }
    }

    assert!(best_ops.len() <= MAX_OPERATIONS);
    let _ = best_prefix_len;

    // local 時のみ、出力列を初期盤面へ再生して合法性と完成を検証する(機構確認)。
    local! {
        let mut replay: [u16; CELLS] = std::array::from_fn(|cell| input.initial_board[cell] as u16);
        for &op in &best_ops {
            let (is_v, r, c, h, w) = decode_op(op);
            assert!(h >= 1 && w >= 1 && r + h <= N && c + w <= N, "長方形が盤面外");
            if is_v {
                assert!(h % 2 == 0, "縦操作の高さが奇数");
            } else {
                assert!(w % 2 == 0, "横操作の幅が奇数");
            }
            // 内部壁ゼロの確認(細長判定の合成で全長方形を検査できる)。
            for j in c..c + w {
                assert!(walls.hp_col[j][r + h - 1] == walls.hp_col[j][r], "内部に H 壁");
            }
            for j in c..c + w - 1 {
                assert!(walls.vp_col[j][r + h] == walls.vp_col[j][r], "内部に V 壁");
            }
            if is_v {
                for x in 0..h / 2 {
                    for j in c..c + w {
                        let a = (r + x) * N + j;
                        replay.swap(a, a + (h / 2) * N);
                    }
                }
            } else {
                for i in r..r + h {
                    for y in 0..w / 2 {
                        let a = i * N + c + y;
                        replay.swap(a, a + w / 2);
                    }
                }
            }
        }
        let final_e = (0..CELLS).filter(|&cell| replay[cell] as usize != cell).count();
        assert_eq!(final_e, 0, "再生後に E != 0");
        trace.count_by("final_e", final_e as i64);
        trace.count_by("best_total", best_ops.len() as i64);
        trace.count_by("best_prefix_len", best_prefix_len as i64);
        trace.count_by("best_tail_len", (best_ops.len() - best_prefix_len) as i64);
        trace.count_by("tail_gate_final", tail_gate as i64);
        trace.add_time_ms("total", tk.exact_elapsed_sec() * 1000.0);
        trace.summary();
    }

    write_output(&best_ops);
}
