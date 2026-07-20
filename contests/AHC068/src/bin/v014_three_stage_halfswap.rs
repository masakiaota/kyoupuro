// v014_three_stage_halfswap.rs
//
// v010_three_stage_routing 棄却の再開条件 (IDEA-D9) を実現する後継実験。
// 横→縦→横の三段階配送で、各段階の一次元整列を隣接交換から区間内の
// 等長ブロックスワップ(半交換)へ置換し、中間列割当を壁跨ぎ費用付きの
// König 辺彩色にして開辺端点交換 (2d-1 手) の発生自体を減らす。
// v000_template だけを土台に新設し、v010 のコードは参照していない。
//
// 完成保証の構造 (tail・fallback なし):
//   1. 需要二部多重グラフ(現在行→目標行、各頂点次数 N)の彩色指数は N
//      (König)。Kempe 交互路により全カードへ必ず中間列を割り当てられる。
//   2. 各フェーズは行(列)内の順列実現。壁区間を跨ぐ移動は開辺最短路の
//      端点交換(2d-1 隣接交換、中間カード復元)で厳密に行い、区間内は
//      半交換 greedy(距離和厳密減)+停滞時の最左確定搬送で必ず完了する。
//   3. フェーズ 1 で各カードは彩色列へ、フェーズ 2 で目標行へ、フェーズ 3 で
//      目標列へ移り、3 フェーズ完了で盤面は完成する(構成的)。

use std::{
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

/// 盤面の一辺。全ケースで固定。
const N: usize = 20;
/// マス数・カード枚数。
const CELLS: usize = N * N;
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

/// リスタート探索に使う割合。残りは出力と local 検証の余裕にする。
const SEARCH_TIME_RATIO: f64 = 0.94;

/// 彩色費用: 壁区間を 1 回跨ぐことのペナルティ。端点交換 2d-1 手の実費が
/// 距離項 (最大 2*19*10=380) より常に重くなるスケールに置く。
const CROSS_COST: u64 = 10_000;
/// 彩色費用: 中間列と現在列・目標列の距離 1 あたりの費用。
const DIST_COST: u64 = 10;
/// 乱択リスタート時に費用へ足すノイズ上限。距離項スケールで揺らす。
const NOISE_COST: u64 = 240;

const NO_CARD: usize = usize::MAX;

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

/// カード位置の逆引きを持つ状態。半交換・端点交換の適用はすべて swap_cells 経由。
#[derive(Clone)]
struct State {
    /// `board[cell] = card`
    board: [u16; CELLS],
    /// `position[card] = cell`
    position: [u16; CELLS],
    misplaced_count: u16,
}

impl State {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        let board: [u16; CELLS] = std::array::from_fn(|cell| initial_board[cell] as u16);
        let mut position = [0u16; CELLS];
        let mut misplaced_count = 0;
        for cell in 0..CELLS {
            position[board[cell] as usize] = cell as u16;
            misplaced_count += (board[cell] as usize != cell) as u16;
        }
        Self {
            board,
            position,
            misplaced_count,
        }
    }

    #[inline]
    fn card_at(&self, cell: usize) -> usize {
        self.board[cell] as usize
    }

    #[inline]
    fn position_of(&self, card: usize) -> usize {
        self.position[card] as usize
    }

    #[inline]
    fn is_complete(&self) -> bool {
        self.misplaced_count == 0
    }

    #[inline]
    fn swap_cells(&mut self, p: usize, q: usize) {
        let a = self.board[p];
        let b = self.board[q];
        self.misplaced_count -= (a as usize != p) as u16 + (b as usize != q) as u16;
        self.misplaced_count += (b as usize != p) as u16 + (a as usize != q) as u16;
        self.board[p] = b;
        self.board[q] = a;
        self.position[a as usize] = q as u16;
        self.position[b as usize] = p as u16;
    }

    /// 長方形半交換を定義通りに適用する。
    fn apply_operation(&mut self, op: &Operation) {
        match op.direction {
            Direction::Vertical => {
                let half = op.h / 2;
                for x in 0..half {
                    for y in 0..op.w {
                        let p = (op.r + x) * N + op.c + y;
                        let q = (op.r + half + x) * N + op.c + y;
                        self.swap_cells(p, q);
                    }
                }
            }
            Direction::Horizontal => {
                let half = op.w / 2;
                for x in 0..op.h {
                    for y in 0..half {
                        let p = (op.r + x) * N + op.c + y;
                        let q = (op.r + x) * N + op.c + half + y;
                        self.swap_cells(p, q);
                    }
                }
            }
        }
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
    #[allow(dead_code)]
    fn mark_fallback(&mut self) {
        self.fallback_count += 1;
    }

    #[allow(dead_code)]
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
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    #[inline]
    fn search_deadline_passed(&self) -> bool {
        self.exact_elapsed_sec() >= self.time_limit_sec * SEARCH_TIME_RATIO
    }
}

struct XorShift64 {
    s: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            s: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.s = x;
        x
    }

    #[inline]
    fn next_range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// 壁から決まる静的情報。
struct Precomp {
    /// row_seg[r][c]: 行 r 内で縦壁に区切られた横支持区間の id。
    row_seg: [[u8; N]; N],
    /// col_seg[r][c]: 列 c 内で横壁に区切られた縦支持区間の id。
    col_seg: [[u8; N]; N],
    /// 各行の横支持区間 [lo, hi] の一覧。
    row_segments: Vec<Vec<(usize, usize)>>,
    /// 各列の縦支持区間 [lo, hi] の一覧。
    col_segments: Vec<Vec<(usize, usize)>>,
    /// 開辺グラフの隣接リスト。
    adj: [[u16; 4]; CELLS],
    adj_len: [u8; CELLS],
}

impl Precomp {
    fn new(input: &Input) -> Self {
        let mut row_seg = [[0u8; N]; N];
        let mut row_segments = vec![Vec::new(); N];
        for r in 0..N {
            let mut seg = 0u8;
            let mut lo = 0usize;
            for c in 0..N {
                row_seg[r][c] = seg;
                let cut = c + 1 == N || input.vertical_walls[r][c];
                if cut {
                    row_segments[r].push((lo, c));
                    seg += 1;
                    lo = c + 1;
                }
            }
        }

        let mut col_seg = [[0u8; N]; N];
        let mut col_segments = vec![Vec::new(); N];
        for c in 0..N {
            let mut seg = 0u8;
            let mut lo = 0usize;
            for r in 0..N {
                col_seg[r][c] = seg;
                let cut = r + 1 == N || input.horizontal_walls[r][c];
                if cut {
                    col_segments[c].push((lo, r));
                    seg += 1;
                    lo = r + 1;
                }
            }
        }

        let mut adj = [[0u16; 4]; CELLS];
        let mut adj_len = [0u8; CELLS];
        for r in 0..N {
            for c in 0..N {
                let cell = r * N + c;
                let mut push = |to: usize| {
                    adj[cell][adj_len[cell] as usize] = to as u16;
                    adj_len[cell] += 1;
                };
                if c + 1 < N && !input.vertical_walls[r][c] {
                    push(cell + 1);
                }
                if c > 0 && !input.vertical_walls[r][c - 1] {
                    push(cell - 1);
                }
                if r + 1 < N && !input.horizontal_walls[r][c] {
                    push(cell + N);
                }
                if r > 0 && !input.horizontal_walls[r - 1][c] {
                    push(cell - N);
                }
            }
        }

        Self {
            row_seg,
            col_seg,
            row_segments,
            col_segments,
            adj,
            adj_len,
        }
    }
}

/// 一次元整列の対象となる直線。Row は行 r(main = 列)、Col は列 c(main = 行)。
#[derive(Clone, Copy)]
enum Axis {
    Row(usize),
    Col(usize),
}

impl Axis {
    #[inline]
    fn cell(&self, main: usize) -> usize {
        match *self {
            Axis::Row(r) => r * N + main,
            Axis::Col(c) => main * N + c,
        }
    }

    #[inline]
    fn seg_id(&self, pre: &Precomp, main: usize) -> u8 {
        match *self {
            Axis::Row(r) => pre.row_seg[r][main],
            Axis::Col(c) => pre.col_seg[main][c],
        }
    }

    fn segments<'a>(&self, pre: &'a Precomp) -> &'a [(usize, usize)] {
        match *self {
            Axis::Row(r) => &pre.row_segments[r],
            Axis::Col(c) => &pre.col_segments[c],
        }
    }

    /// 区間 [off, off+2*len) の前半と後半を交換する半交換操作。
    #[inline]
    fn block_op(&self, off: usize, len: usize) -> Operation {
        match *self {
            Axis::Row(r) => Operation {
                direction: Direction::Horizontal,
                r,
                c: off,
                h: 1,
                w: 2 * len,
            },
            Axis::Col(c) => Operation {
                direction: Direction::Vertical,
                r: off,
                c,
                h: 2 * len,
                w: 1,
            },
        }
    }
}

/// 1 計画分の統計。best 計画のものを TraceStats へ流す。
#[derive(Default, Clone, Copy)]
struct PlanStats {
    cross_swaps: usize,
    cross_ops: usize,
    halfswap_ops: usize,
    forced_ops: usize,
    stalls: usize,
    phase_ops: [usize; 3],
    kempe_chains: usize,
    recolor_moves: usize,
    cross_assigned: usize,
}

/// 需要二部多重グラフ(現在行→目標行)の N 色 Kempe 彩色。
/// 返り値 color[card] = 中間列。必ず全カードに割り当てが付く(König)。
fn make_coloring(
    state0: &State,
    pre: &Precomp,
    rng: &mut XorShift64,
    randomize: bool,
    stats: &mut PlanStats,
) -> [usize; CELLS] {
    let mut color = [usize::MAX; CELLS];
    // col_a[r0][c]: 現在行 r0 側で色 c を使うカード。col_b[r1][c]: 目標行側。
    let mut col_a = [[NO_CARD; N]; N];
    let mut col_b = [[NO_CARD; N]; N];

    let cur_row = |card: usize| state0.position_of(card) / N;
    let cur_col = |card: usize| state0.position_of(card) % N;
    let tgt_row = |card: usize| card / N;
    let tgt_col = |card: usize| card % N;

    // 中間列 c を選ぶ費用。跨ぎ費用は3箇所:
    //   フェーズ1(行 r0 内 c0→c)、フェーズ2(列 c 内 r0→r1)、フェーズ3(行 r1 内 c→c1)。
    let cost_of = |card: usize, c: usize, rng: &mut XorShift64, randomize: bool| -> u64 {
        let r0 = cur_row(card);
        let c0 = cur_col(card);
        let r1 = tgt_row(card);
        let c1 = tgt_col(card);
        let mut cost = 0u64;
        if pre.row_seg[r0][c] != pre.row_seg[r0][c0] {
            cost += CROSS_COST;
        }
        if pre.col_seg[r0][c] != pre.col_seg[r1][c] {
            cost += CROSS_COST;
        }
        if pre.row_seg[r1][c] != pre.row_seg[r1][c1] {
            cost += CROSS_COST;
        }
        cost += (c.abs_diff(c0) + c.abs_diff(c1)) as u64 * DIST_COST;
        if randomize {
            cost += rng.next_u64() % NOISE_COST;
        }
        cost
    };

    let mut order: Vec<usize> = (0..CELLS).collect();
    if randomize {
        for i in (1..CELLS).rev() {
            let j = rng.next_range(i + 1);
            order.swap(i, j);
        }
    }

    for &card in &order {
        let a = cur_row(card);
        let b = tgt_row(card);

        // 両側空きの色から費用最小を選ぶ。
        let mut best: Option<(u64, usize)> = None;
        for c in 0..N {
            if col_a[a][c] == NO_CARD && col_b[b][c] == NO_CARD {
                let cost = cost_of(card, c, rng, randomize);
                if best.is_none() || cost < best.unwrap().0 {
                    best = Some((cost, c));
                }
            }
        }

        let chosen = if let Some((_, c)) = best {
            c
        } else {
            // Kempe 交互路: α = a 側空きの費用最小色、β = b 側空きの任意色。
            // 二部性より α/β 交互路は a に戻らないため、反転後に α が両側で空く。
            let mut alpha_best: Option<(u64, usize)> = None;
            for c in 0..N {
                if col_a[a][c] == NO_CARD {
                    let cost = cost_of(card, c, rng, randomize);
                    if alpha_best.is_none() || cost < alpha_best.unwrap().0 {
                        alpha_best = Some((cost, c));
                    }
                }
            }
            let alpha = alpha_best.expect("degree < N guarantees a free color on side a").1;
            let beta = (0..N)
                .find(|&c| col_b[b][c] == NO_CARD)
                .expect("degree < N guarantees a free color on side b");

            // b から α で始まる交互路を収集する。α/β 部分グラフは最大次数 2 の
            // path/cycle 和で、b は次数 1 の path 端点なので必ず有限で終わる。
            let mut chain: Vec<usize> = Vec::new();
            let mut on_b_side = true;
            let mut vertex = b;
            let mut want = alpha;
            loop {
                let e = if on_b_side {
                    col_b[vertex][want]
                } else {
                    col_a[vertex][want]
                };
                if e == NO_CARD {
                    break;
                }
                assert!(chain.len() <= 2 * CELLS, "Kempe chain must terminate");
                chain.push(e);
                vertex = if on_b_side { cur_row(e) } else { tgt_row(e) };
                on_b_side = !on_b_side;
                want = if want == alpha { beta } else { alpha };
            }
            // 色 α↔β を chain 上で入れ替える。
            for &e in &chain {
                let ec = color[e];
                col_a[cur_row(e)][ec] = NO_CARD;
                col_b[tgt_row(e)][ec] = NO_CARD;
            }
            for &e in &chain {
                let nc = if color[e] == alpha { beta } else { alpha };
                color[e] = nc;
                col_a[cur_row(e)][nc] = e;
                col_b[tgt_row(e)][nc] = e;
            }
            stats.kempe_chains += 1;
            debug_assert!(col_a[a][alpha] == NO_CARD && col_b[b][alpha] == NO_CARD);
            alpha
        };

        color[card] = chosen;
        col_a[a][chosen] = card;
        col_b[b][chosen] = card;
    }

    // 単色改善: 両側空きでより安い色があれば移す。跨ぎ数の削減が主目的。
    for _round in 0..3 {
        let mut moved = 0usize;
        for card in 0..CELLS {
            let a = cur_row(card);
            let b = tgt_row(card);
            let cur = color[card];
            let cur_cost = cost_of(card, cur, rng, false);
            let mut best: Option<(u64, usize)> = None;
            for c in 0..N {
                if c != cur && col_a[a][c] == NO_CARD && col_b[b][c] == NO_CARD {
                    let cost = cost_of(card, c, rng, false);
                    if cost < cur_cost && (best.is_none() || cost < best.unwrap().0) {
                        best = Some((cost, c));
                    }
                }
            }
            if let Some((_, c)) = best {
                col_a[a][cur] = NO_CARD;
                col_b[b][cur] = NO_CARD;
                color[card] = c;
                col_a[a][c] = card;
                col_b[b][c] = card;
                moved += 1;
            }
        }
        stats.recolor_moves += moved;
        if moved == 0 {
            break;
        }
    }

    // 跨ぎ割当数(実費見積もりの機構統計)。
    for card in 0..CELLS {
        let c = color[card];
        let r0 = cur_row(card);
        let c0 = cur_col(card);
        let r1 = tgt_row(card);
        let c1 = tgt_col(card);
        if pre.row_seg[r0][c] != pre.row_seg[r0][c0]
            || pre.col_seg[r0][c] != pre.col_seg[r1][c]
            || pre.row_seg[r1][c] != pre.row_seg[r1][c1]
        {
            stats.cross_assigned += 1;
        }
    }

    local! {
        // 彩色の完全性: 各現在行・各目標行で色が重複しないこと。
        for r in 0..N {
            let mut seen_a = [false; N];
            let mut seen_b = [false; N];
            for c in 0..N {
                if col_a[r][c] != NO_CARD {
                    assert!(!seen_a[c]);
                    seen_a[c] = true;
                }
                if col_b[r][c] != NO_CARD {
                    assert!(!seen_b[c]);
                    seen_b[c] = true;
                }
            }
        }
        for card in 0..CELLS {
            assert!(color[card] < N, "coloring must assign every card");
        }
    }

    color
}

/// 隣接 2 セルを 1x2 / 2x1 の半交換で交換し、列へ記録する。
#[inline]
fn adjacent_swap(state: &mut State, ops: &mut Vec<Operation>, x: usize, y: usize) {
    let (rx, cx) = (x / N, x % N);
    let (ry, cy) = (y / N, y % N);
    let op = if rx == ry {
        Operation {
            direction: Direction::Horizontal,
            r: rx,
            c: cx.min(cy),
            h: 1,
            w: 2,
        }
    } else {
        Operation {
            direction: Direction::Vertical,
            r: rx.min(ry),
            c: cx,
            h: 2,
            w: 1,
        }
    };
    state.apply_operation(&op);
    ops.push(op);
}

/// 開辺最短路の端点交換: セル p, q のカードだけを交換し、中間カードは復元する。
/// 経路長 d に対して 2d-1 手。important_properties の証明済み部品。
fn endpoint_swap(
    state: &mut State,
    pre: &Precomp,
    ops: &mut Vec<Operation>,
    p: usize,
    q: usize,
    stats: &mut PlanStats,
) {
    if p == q {
        return;
    }
    #[cfg(feature = "local")]
    let before = state.board;

    // BFS で p→q の最短路を取る。全マス連結保証より必ず届く。
    let mut prev = [u16::MAX; CELLS];
    let mut queue = [0u16; CELLS];
    let (mut head, mut tail) = (0usize, 0usize);
    prev[p] = p as u16;
    queue[tail] = p as u16;
    tail += 1;
    while head < tail {
        let x = queue[head] as usize;
        head += 1;
        if x == q {
            break;
        }
        for k in 0..pre.adj_len[x] as usize {
            let n = pre.adj[x][k] as usize;
            if prev[n] == u16::MAX {
                prev[n] = x as u16;
                queue[tail] = n as u16;
                tail += 1;
            }
        }
    }
    debug_assert!(prev[q] != u16::MAX);

    let mut path = vec![q];
    let mut cur = q;
    while cur != p {
        cur = prev[cur] as usize;
        path.push(cur);
    }
    path.reverse();

    let d = path.len() - 1;
    let ops_before = ops.len();
    for i in 0..d {
        adjacent_swap(state, ops, path[i], path[i + 1]);
    }
    for i in (0..d.saturating_sub(1)).rev() {
        adjacent_swap(state, ops, path[i], path[i + 1]);
    }
    stats.cross_ops += ops.len() - ops_before;

    #[cfg(feature = "local")]
    {
        // 端点だけが交換され、中間カードが復元されていることを確認する。
        let after = &state.board;
        for cell in 0..CELLS {
            if cell == p {
                assert_eq!(after[cell], before[q]);
            } else if cell == q {
                assert_eq!(after[cell], before[p]);
            } else {
                assert_eq!(after[cell], before[cell], "endpoint swap must restore middles");
            }
        }
    }
}

/// 支持区間 [lo, hi] の中を半交換 greedy で整列する。
/// target(card) は区間内で閉じた main 座標の順列を返す(呼び出し側が保証)。
///
/// 停止性: greedy 段は距離和を厳密に減らし(gain >= 1)、停滞したら
/// 最左不一致 main を確定する選択搬送で確定フロンティア lo_fixed を厳密に増やす。
fn sort_segment<F>(
    state: &mut State,
    axis: Axis,
    lo: usize,
    hi: usize,
    target: &F,
    ops: &mut Vec<Operation>,
    rng: &mut XorShift64,
    randomize: bool,
    stats: &mut PlanStats,
) where
    F: Fn(usize) -> usize,
{
    if lo >= hi {
        return;
    }
    let mut lo_fixed = lo;

    // 現在の main にあるカードの目標 main。
    let tgt_at = |state: &State, m: usize| -> usize { target(state.card_at(axis.cell(m))) };

    loop {
        // 先頭から一致している main を確定扱いで進める。
        while lo_fixed <= hi && tgt_at(state, lo_fixed) == lo_fixed {
            lo_fixed += 1;
        }
        if lo_fixed >= hi {
            break;
        }

        // 距離和 greedy: [lo_fixed, hi] 内の全ブロックスワップから
        // 距離減少 gain > 0 の最良を適用する。タイは一致増、幅小、乱択で破る。
        loop {
            let span = hi - lo_fixed + 1;
            let mut best_gain = 0i64;
            let mut best_score = i64::MIN;
            let mut best: Option<(usize, usize)> = None;
            let mut tie_count = 0usize;
            for len in 1..=span / 2 {
                for off in lo_fixed..=hi + 1 - 2 * len {
                    let mut gain = 0i64;
                    let mut match_delta = 0i64;
                    for i in 0..len {
                        let m1 = off + i;
                        let m2 = off + len + i;
                        let t1 = tgt_at(state, m1) as i64;
                        let t2 = tgt_at(state, m2) as i64;
                        let (m1i, m2i) = (m1 as i64, m2 as i64);
                        gain += (m1i - t1).abs() + (m2i - t2).abs()
                            - (m2i - t1).abs()
                            - (m1i - t2).abs();
                        match_delta += (m2i == t1) as i64 + (m1i == t2) as i64
                            - (m1i == t1) as i64
                            - (m2i == t2) as i64;
                    }
                    if gain <= 0 {
                        continue;
                    }
                    let score = gain * 1000 + match_delta * 10 - len as i64;
                    if score > best_score {
                        best_score = score;
                        best_gain = gain;
                        best = Some((off, len));
                        tie_count = 1;
                    } else if score == best_score && randomize {
                        // 同点は reservoir sampling で乱択し、リスタートの多様性にする。
                        tie_count += 1;
                        if rng.next_range(tie_count) == 0 {
                            best = Some((off, len));
                        }
                    }
                }
            }
            let Some((off, len)) = best else { break };
            debug_assert!(best_gain > 0);
            let op = axis.block_op(off, len);
            state.apply_operation(&op);
            ops.push(op);
            stats.halfswap_ops += 1;
        }

        while lo_fixed <= hi && tgt_at(state, lo_fixed) == lo_fixed {
            lo_fixed += 1;
        }
        if lo_fixed >= hi {
            break;
        }

        // 停滞: 最左の未確定 main に来るべきカードをブロックスワップで搬送して確定する。
        stats.stalls += 1;
        let mut p = usize::MAX;
        for m in lo_fixed + 1..=hi {
            if tgt_at(state, m) == lo_fixed {
                p = m;
                break;
            }
        }
        debug_assert!(p != usize::MAX, "target card must exist inside the segment");
        while p > lo_fixed {
            let len = (p - lo_fixed).min(hi + 1 - p);
            let op = axis.block_op(p - len, len);
            state.apply_operation(&op);
            ops.push(op);
            stats.forced_ops += 1;
            p -= len;
        }
        lo_fixed += 1;
    }

    local! {
        for m in lo..=hi {
            assert_eq!(tgt_at(state, m), m, "segment must be sorted");
        }
    }
}

/// 1 本の行(列)を目標順列へ整列する。
/// 手順: (1) 壁区間を跨ぐカードを端点交換のサイクル解消で正しい区間へ入れ、
/// (2) 各区間内を半交換 greedy で整列する。
fn realize_line<F>(
    state: &mut State,
    pre: &Precomp,
    axis: Axis,
    target: &F,
    ops: &mut Vec<Operation>,
    rng: &mut XorShift64,
    randomize: bool,
    stats: &mut PlanStats,
) where
    F: Fn(usize) -> usize,
{
    local! {
        // target は line 上の main の順列でなければならない。
        let mut seen = [false; N];
        for m in 0..N {
            let t = target(state.card_at(axis.cell(m)));
            assert!(t < N && !seen[t], "target must be a permutation on the line");
            seen[t] = true;
        }
    }

    // (1) 区間跨ぎ解消。毎交換で少なくとも 1 枚が正しい区間へ確定するため停止する。
    loop {
        let mut viol: Vec<(usize, usize)> = Vec::new(); // (main, tgt_main)
        for m in 0..N {
            let t = target(state.card_at(axis.cell(m)));
            if axis.seg_id(pre, m) != axis.seg_id(pre, t) {
                viol.push((m, t));
            }
        }
        if viol.is_empty() {
            break;
        }
        let (um, ut) = viol[0];
        let target_seg = axis.seg_id(pre, ut);
        let source_seg = axis.seg_id(pre, um);
        // 相互交換(目標区間が互いの現区間)できる相手を優先する。
        let mut pick = usize::MAX;
        for &(vm, vt) in viol.iter().skip(1) {
            if axis.seg_id(pre, vm) == target_seg {
                if axis.seg_id(pre, vt) == source_seg {
                    pick = vm;
                    break;
                }
                if pick == usize::MAX {
                    pick = vm;
                }
            }
        }
        assert!(
            pick != usize::MAX,
            "a violating partner must exist in the destination segment"
        );
        endpoint_swap(state, pre, ops, axis.cell(um), axis.cell(pick), stats);
        stats.cross_swaps += 1;
    }

    // (2) 区間内整列。
    for &(lo, hi) in axis.segments(pre) {
        sort_segment(state, axis, lo, hi, target, ops, rng, randomize, stats);
    }

    local! {
        for m in 0..N {
            assert_eq!(target(state.card_at(axis.cell(m))), m, "line must be realized");
        }
    }
}

/// 三段階配送の完全計画を 1 本生成する。
fn plan_once(
    input: &Input,
    pre: &Precomp,
    seed: u64,
    randomize: bool,
) -> (Vec<Operation>, PlanStats) {
    let mut rng = XorShift64::new(seed);
    let mut stats = PlanStats::default();
    let mut state = State::new(&input.initial_board);
    let mut ops: Vec<Operation> = Vec::with_capacity(4096);

    let color = make_coloring(&state, pre, &mut rng, randomize, &mut stats);

    // フェーズ 1(横): 各行内で、各カードを彩色された中間列へ。
    let ops_mark0 = ops.len();
    for r in 0..N {
        let target = |card: usize| color[card];
        realize_line(
            &mut state, pre, Axis::Row(r), &target, &mut ops, &mut rng, randomize, &mut stats,
        );
    }
    stats.phase_ops[0] = ops.len() - ops_mark0;
    local! {
        for cell in 0..CELLS {
            assert_eq!(color[state.card_at(cell)], cell % N, "phase1 must place cards on colors");
        }
    }

    // フェーズ 2(縦): 各列内で、各カードを目標行へ。彩色のスロット制約
    // (各 (列, 目標行) 高々 1 枚)が順列性を保証する。
    let ops_mark1 = ops.len();
    for c in 0..N {
        let target = |card: usize| card / N;
        realize_line(
            &mut state, pre, Axis::Col(c), &target, &mut ops, &mut rng, randomize, &mut stats,
        );
    }
    stats.phase_ops[1] = ops.len() - ops_mark1;
    local! {
        for cell in 0..CELLS {
            assert_eq!(state.card_at(cell) / N, cell / N, "phase2 must fix rows");
        }
    }

    // フェーズ 3(横): 各行内で、各カードを目標列へ。行が確定済みなので順列。
    let ops_mark2 = ops.len();
    for r in 0..N {
        let target = |card: usize| card % N;
        realize_line(
            &mut state, pre, Axis::Row(r), &target, &mut ops, &mut rng, randomize, &mut stats,
        );
    }
    stats.phase_ops[2] = ops.len() - ops_mark2;

    assert!(state.is_complete(), "three-stage delivery must complete the board");
    assert!(ops.len() <= MAX_OPERATIONS);

    (ops, stats)
}

#[cfg(feature = "local")]
fn op_is_legal(input: &Input, op: &Operation) -> bool {
    if op.h == 0 || op.w == 0 || op.r + op.h > N || op.c + op.w > N {
        return false;
    }
    match op.direction {
        Direction::Vertical => {
            if op.h % 2 != 0 {
                return false;
            }
        }
        Direction::Horizontal => {
            if op.w % 2 != 0 {
                return false;
            }
        }
    }
    for i in op.r..op.r + op.h {
        for j in op.c..op.c + op.w - 1 {
            if input.vertical_walls[i][j] {
                return false;
            }
        }
    }
    for i in op.r..op.r + op.h - 1 {
        for j in op.c..op.c + op.w {
            if input.horizontal_walls[i][j] {
                return false;
            }
        }
    }
    true
}

// best_stats / best_updates は local 検証時のみ読まれるため、本番ビルドの
// 未使用警告だけを抑止する。
#[cfg_attr(not(feature = "local"), allow(unused_variables, unused_assignments))]
fn main() {
    let time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let input = Input::read();
    let pre = Precomp::new(&input);

    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();

    let mut best_ops: Option<Vec<Operation>> = None;
    let mut best_stats = PlanStats::default();
    let mut plans = 0u64;
    let mut best_updates = 0i64;

    // 1 本目は決定的な計画で完全解を確保し、以後は彩色・整列の乱択リスタートで
    // 時間いっぱい最短計画を探す。
    loop {
        let randomize = plans > 0;
        let (ops, stats) = plan_once(&input, &pre, plans + 1, randomize);
        plans += 1;
        let better = best_ops.as_ref().is_none_or(|b| ops.len() < b.len());
        if better {
            best_ops = Some(ops);
            best_stats = stats;
            best_updates += 1;
        }
        if time_keeper.search_deadline_passed() {
            break;
        }
    }

    let best = best_ops.expect("at least one plan is always produced");

    local! {
        // 出力列の再生検証: 全操作の合法性と最終盤面の完成。
        let mut replay = State::new(&input.initial_board);
        for op in &best {
            assert!(op_is_legal(&input, op), "illegal operation in output");
            replay.apply_operation(op);
        }
        assert!(replay.is_complete(), "replayed output must complete the board");

        trace.count_by("plans", plans as i64);
        trace.count_by("best_updates", best_updates);
        trace.count_by("best_t", best.len() as i64);
        trace.count_by("phase1_ops", best_stats.phase_ops[0] as i64);
        trace.count_by("phase2_ops", best_stats.phase_ops[1] as i64);
        trace.count_by("phase3_ops", best_stats.phase_ops[2] as i64);
        trace.count_by("halfswap_ops", best_stats.halfswap_ops as i64);
        trace.count_by("forced_ops", best_stats.forced_ops as i64);
        trace.count_by("stalls", best_stats.stalls as i64);
        trace.count_by("cross_assigned", best_stats.cross_assigned as i64);
        trace.count_by("cross_swaps", best_stats.cross_swaps as i64);
        trace.count_by("cross_ops", best_stats.cross_ops as i64);
        trace.count_by("kempe_chains", best_stats.kempe_chains as i64);
        trace.count_by("recolor_moves", best_stats.recolor_moves as i64);
        trace.add_time_ms("search", time_keeper.exact_elapsed_sec() * 1000.0);
        trace.summary();
    }

    write_output(&best);
}
