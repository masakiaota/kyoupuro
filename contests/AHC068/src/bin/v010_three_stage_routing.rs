// v010_three_stage_routing.rs
use std::{
    collections::VecDeque,
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
/// 全域木の葉剥がしは、各時点の残存頂点数未満の交換で一葉を確定する。
const BASELINE_MAX_OPERATIONS: usize = CELLS * (CELLS - 1) / 2;

/// AtCoder 側の基準の探索打ち切り秒数。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 出力再生の時間を確保するため、探索は全体制限より少し早く止める。
const SEARCH_END_RATIO: f64 = 0.94;

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

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
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

    #[inline(always)]
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    #[inline(always)]
    fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    #[inline(always)]
    fn progress(&self) -> f64 {
        self.progress
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

/// SplitMix64 は探索を入力・実行環境によらず再現可能にするため固定 seed で使う。
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn usize(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next_u64() as usize) % upper
    }

    fn f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.usize(i + 1));
        }
    }
}

#[derive(Clone)]
struct SwapRoute {
    operations: Vec<Operation>,
}

struct RoutingTable {
    /// row-major で (r,c)-(r,c+1) の 20*19 本。
    horizontal: Vec<SwapRoute>,
    /// row-major で (r,c)-(r+1,c) の 19*20 本。
    vertical: Vec<SwapRoute>,
}

impl RoutingTable {
    fn new(input: &Input) -> Self {
        let mut horizontal = Vec::with_capacity(N * (N - 1));
        for r in 0..N {
            for c in 0..N - 1 {
                horizontal.push(Self::make_route(input, r * N + c, r * N + c + 1));
            }
        }

        let mut vertical = Vec::with_capacity((N - 1) * N);
        for r in 0..N - 1 {
            for c in 0..N {
                vertical.push(Self::make_route(input, r * N + c, (r + 1) * N + c));
            }
        }

        Self {
            horizontal,
            vertical,
        }
    }

    fn horizontal(&self, r: usize, c: usize) -> &SwapRoute {
        &self.horizontal[r * (N - 1) + c]
    }

    fn vertical(&self, r: usize, c: usize) -> &SwapRoute {
        &self.vertical[r * N + c]
    }

    /// 論理的には隣接していても壁越しなら直接交換できない。全開辺グラフの
    /// 最短路を往復し、経路内部を復元しながら端点だけを交換する。
    fn make_route(input: &Input, source: usize, target: usize) -> SwapRoute {
        let mut previous = [usize::MAX; CELLS];
        let mut queue = VecDeque::new();
        previous[source] = source;
        queue.push_back(source);

        while let Some(cell) = queue.pop_front() {
            if cell == target {
                break;
            }
            let r = cell / N;
            let c = cell % N;
            let mut visit = |next: usize| {
                if previous[next] == usize::MAX {
                    previous[next] = cell;
                    queue.push_back(next);
                }
            };
            if r > 0 && !input.horizontal_walls[r - 1][c] {
                visit(cell - N);
            }
            if r + 1 < N && !input.horizontal_walls[r][c] {
                visit(cell + N);
            }
            if c > 0 && !input.vertical_walls[r][c - 1] {
                visit(cell - 1);
            }
            if c + 1 < N && !input.vertical_walls[r][c] {
                visit(cell + 1);
            }
        }
        assert_ne!(previous[target], usize::MAX);

        let mut path = Vec::new();
        let mut cell = target;
        path.push(cell);
        while cell != source {
            cell = previous[cell];
            path.push(cell);
        }
        path.reverse();

        let distance = path.len() - 1;
        let mut operations = Vec::with_capacity(2 * distance - 1);
        for i in 0..distance {
            operations.push(edge_operation(path[i], path[i + 1]));
        }
        // 最後の辺まで戻すと端点交換も取り消すため、その一辺だけを除く。
        for i in (0..distance - 1).rev() {
            operations.push(edge_operation(path[i], path[i + 1]));
        }
        debug_assert_eq!(operations.len(), 2 * distance - 1);
        SwapRoute { operations }
    }
}

fn edge_operation(a: usize, b: usize) -> Operation {
    debug_assert!(MAX_SINGLE_AXIS_SHIFT >= 1);
    let ar = a / N;
    let ac = a % N;
    let br = b / N;
    let bc = b % N;
    if ar == br {
        assert_eq!(ac.abs_diff(bc), 1);
        Operation {
            direction: Direction::Horizontal,
            r: ar,
            c: ac.min(bc),
            h: 1,
            w: 2,
        }
    } else {
        assert_eq!(ac, bc);
        assert_eq!(ar.abs_diff(br), 1);
        Operation {
            direction: Direction::Vertical,
            r: ar.min(br),
            c: ac,
            h: 2,
            w: 1,
        }
    }
}

/// 完全マッチング用の Kuhn DFS。target を一度だけ訪れることで多重辺も扱える。
fn augment_matching(
    source: usize,
    remaining: &[bool; CELLS],
    candidates: &[Vec<usize>],
    source_of_card: &[usize; CELLS],
    seen_target: &mut [bool; N],
    matched_card: &mut [usize; N],
) -> bool {
    for &card in &candidates[source] {
        if !remaining[card] {
            continue;
        }
        let target = card / N;
        if seen_target[target] {
            continue;
        }
        seen_target[target] = true;
        let old_card = matched_card[target];
        if old_card == usize::MAX
            || augment_matching(
                source_of_card[old_card],
                remaining,
                candidates,
                source_of_card,
                seen_target,
                matched_card,
            )
        {
            matched_card[target] = card;
            return true;
        }
    }
    false
}

/// 20 正則二部多重グラフから Kuhn 完全マッチングを順に除去する。
fn decompose_matchings(
    input: &Input,
    source_of_card: &[usize; CELLS],
    randomized: bool,
    rng: &mut Rng,
) -> [usize; CELLS] {
    let mut remaining = [true; CELLS];
    let mut color = [usize::MAX; CELLS];

    for matching_color in 0..N {
        let mut candidates: Vec<Vec<usize>> = (0..N)
            .map(|source| {
                (0..N)
                    .map(|c| input.initial_board[source * N + c])
                    .collect()
            })
            .collect();
        let mut source_order: Vec<usize> = (0..N).collect();
        if randomized {
            for row in &mut candidates {
                rng.shuffle(row);
            }
            rng.shuffle(&mut source_order);
        }

        let mut matched_card = [usize::MAX; N];
        for source in source_order {
            let mut seen_target = [false; N];
            assert!(augment_matching(
                source,
                &remaining,
                &candidates,
                source_of_card,
                &mut seen_target,
                &mut matched_card,
            ));
        }
        for &card in &matched_card {
            assert_ne!(card, usize::MAX);
            assert!(remaining[card]);
            remaining[card] = false;
            color[card] = matching_color;
        }
    }
    assert!(remaining.iter().all(|&value| !value));
    color
}

/// 二色の完全マッチングの合併は交互閉路に分解される。そのうち一閉路だけを
/// 反転すれば、source/target の双方で各色一回という制約を常に保てる。
fn mutate_alternating_cycle(
    color: &mut [usize; CELLS],
    source_of_card: &[usize; CELLS],
    a: usize,
    b: usize,
    rng: &mut Rng,
) -> Vec<usize> {
    let mut a_card_of_source = [usize::MAX; N];
    let mut b_card_of_target = [usize::MAX; N];
    for card in 0..CELLS {
        if color[card] == a {
            a_card_of_source[source_of_card[card]] = card;
        } else if color[card] == b {
            b_card_of_target[card / N] = card;
        }
    }
    debug_assert!(a_card_of_source.iter().all(|&card| card != usize::MAX));
    debug_assert!(b_card_of_target.iter().all(|&card| card != usize::MAX));

    let mut cycles = Vec::new();
    let mut visited = [false; N];
    for start in 0..N {
        if visited[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut source = start;
        loop {
            visited[source] = true;
            cycle.push(source);
            let target = a_card_of_source[source] / N;
            source = source_of_card[b_card_of_target[target]];
            if source == start {
                break;
            }
            debug_assert!(!visited[source]);
        }
        cycles.push(cycle);
    }

    let cycle = &cycles[rng.usize(cycles.len())];
    let mut changed_cards = Vec::with_capacity(2 * cycle.len());
    for &source in cycle {
        let a_card = a_card_of_source[source];
        let b_card = b_card_of_target[a_card / N];
        color[a_card] = b;
        color[b_card] = a;
        changed_cards.push(a_card);
        changed_cards.push(b_card);
    }
    changed_cards
}

/// 構築時と同じ三段階の論理盤面を再現し、選択した挿入ソート列の物理コストを返す。
/// 単純なカード変位和では、途中で逆向きに動いて戻るカードを数えられない。
fn evaluate(input: &Input, color: &[usize; CELLS], routes: &RoutingTable) -> usize {
    let mut board = input.initial_board;
    let mut cost = 0;

    for row in 0..N {
        let values = std::array::from_fn(|col| color[board[row * N + col]]);
        for col in choose_horizontal_swaps(values, row, routes) {
            cost += routes.horizontal(row, col).operations.len();
            board.swap(row * N + col, row * N + col + 1);
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(color[board[cell]], cell % N);
        }
    }

    for col in 0..N {
        let values = std::array::from_fn(|row| board[row * N + col] / N);
        for row in choose_vertical_swaps(values, col, routes) {
            cost += routes.vertical(row, col).operations.len();
            board.swap(row * N + col, (row + 1) * N + col);
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(board[cell] / N, cell / N);
        }
    }

    for row in 0..N {
        let values = std::array::from_fn(|col| board[row * N + col] % N);
        for col in choose_horizontal_swaps(values, row, routes) {
            cost += routes.horizontal(row, col).operations.len();
            board.swap(row * N + col, row * N + col + 1);
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(board[cell], cell);
        }
    }
    cost
}

fn insertion_sort_left(mut values: [usize; N]) -> Vec<usize> {
    let mut swaps = Vec::new();
    for i in 1..N {
        let mut j = i;
        while j > 0 && values[j - 1] > values[j] {
            values.swap(j - 1, j);
            swaps.push(j - 1);
            j -= 1;
        }
    }
    debug_assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
    swaps
}

fn insertion_sort_right(mut values: [usize; N]) -> Vec<usize> {
    let mut swaps = Vec::new();
    for i in (0..N - 1).rev() {
        let mut j = i;
        while j + 1 < N && values[j] > values[j + 1] {
            values.swap(j, j + 1);
            swaps.push(j);
            j += 1;
        }
    }
    debug_assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
    swaps
}

#[derive(Default)]
struct BuildStats {
    logical: [usize; 3],
    physical: [usize; 3],
    /// 壁越し経路（物理操作が2個以上）を使った論理 swap 数。
    routed_swaps: usize,
}

fn choose_horizontal_swaps(values: [usize; N], row: usize, routes: &RoutingTable) -> Vec<usize> {
    let left = insertion_sort_left(values);
    let right = insertion_sort_right(values);
    let left_cost: usize = left
        .iter()
        .map(|&c| routes.horizontal(row, c).operations.len())
        .sum();
    let right_cost: usize = right
        .iter()
        .map(|&c| routes.horizontal(row, c).operations.len())
        .sum();
    if left_cost <= right_cost {
        left
    } else {
        right
    }
}

fn choose_vertical_swaps(values: [usize; N], col: usize, routes: &RoutingTable) -> Vec<usize> {
    let top = insertion_sort_left(values);
    let bottom = insertion_sort_right(values);
    let top_cost: usize = top
        .iter()
        .map(|&r| routes.vertical(r, col).operations.len())
        .sum();
    let bottom_cost: usize = bottom
        .iter()
        .map(|&r| routes.vertical(r, col).operations.len())
        .sum();
    if top_cost <= bottom_cost {
        top
    } else {
        bottom
    }
}

fn append_horizontal_swap(
    board: &mut [usize; CELLS],
    operations: &mut Vec<Operation>,
    stats: &mut BuildStats,
    stage: usize,
    row: usize,
    col: usize,
    routes: &RoutingTable,
    emit: bool,
) {
    board.swap(row * N + col, row * N + col + 1);
    let route = routes.horizontal(row, col);
    stats.logical[stage] += 1;
    stats.physical[stage] += route.operations.len();
    stats.routed_swaps += (route.operations.len() > 1) as usize;
    if emit {
        operations.extend_from_slice(&route.operations);
    }
}

fn append_vertical_swap(
    board: &mut [usize; CELLS],
    operations: &mut Vec<Operation>,
    stats: &mut BuildStats,
    stage: usize,
    row: usize,
    col: usize,
    routes: &RoutingTable,
    emit: bool,
) {
    board.swap(row * N + col, (row + 1) * N + col);
    let route = routes.vertical(row, col);
    stats.logical[stage] += 1;
    stats.physical[stage] += route.operations.len();
    stats.routed_swaps += (route.operations.len() > 1) as usize;
    if emit {
        operations.extend_from_slice(&route.operations);
    }
}

fn construct_operations(
    input: &Input,
    color: &[usize; CELLS],
    routes: &RoutingTable,
    expected_cost: usize,
    emit: bool,
) -> (Vec<Operation>, BuildStats) {
    let mut board = input.initial_board;
    let mut operations = if emit {
        Vec::with_capacity(expected_cost)
    } else {
        Vec::new()
    };
    let mut stats = BuildStats::default();

    // 第1段: 初期行ごとに中間列 color の昇順へ並べる。
    for row in 0..N {
        let values = std::array::from_fn(|col| color[board[row * N + col]]);
        for col in choose_horizontal_swaps(values, row, routes) {
            append_horizontal_swap(
                &mut board,
                &mut operations,
                &mut stats,
                0,
                row,
                col,
                routes,
                emit,
            );
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(color[board[cell]], cell % N);
        }
    }

    // 第2段: 各中間列で目標行の昇順へ並べる。
    for col in 0..N {
        let values = std::array::from_fn(|row| board[row * N + col] / N);
        for row in choose_vertical_swaps(values, col, routes) {
            append_vertical_swap(
                &mut board,
                &mut operations,
                &mut stats,
                1,
                row,
                col,
                routes,
                emit,
            );
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(board[cell] / N, cell / N);
        }
    }

    // 第3段: 目標行内で目標列の昇順へ並べれば完成する。
    for row in 0..N {
        let values = std::array::from_fn(|col| board[row * N + col] % N);
        for col in choose_horizontal_swaps(values, row, routes) {
            append_horizontal_swap(
                &mut board,
                &mut operations,
                &mut stats,
                2,
                row,
                col,
                routes,
                emit,
            );
        }
    }
    local! {
        for cell in 0..CELLS {
            assert_eq!(board[cell], cell);
        }
        assert_eq!(stats.physical.iter().sum::<usize>(), expected_cost);
        if emit {
            assert_eq!(operations.len(), expected_cost);
        } else {
            assert!(operations.is_empty());
        }
    }
    (operations, stats)
}

/// 開辺だけから作った固定 BFS 全域木を葉から剥がす完全解。
/// 葉 v のカードは残存木内にあるため、その現在地から v まで木辺 swap で運べる。
/// v を確定後に削除すれば以後触れず、最後の根も置換の保存から自動的に確定する。
fn construct_tree_baseline(input: &Input) -> Vec<Operation> {
    let mut parent = [usize::MAX; CELLS];
    let mut bfs_order = Vec::with_capacity(CELLS);
    let mut queue = VecDeque::new();
    parent[0] = 0;
    queue.push_back(0);
    while let Some(cell) = queue.pop_front() {
        bfs_order.push(cell);
        let r = cell / N;
        let c = cell % N;
        let mut visit = |next: usize| {
            if parent[next] == usize::MAX {
                parent[next] = cell;
                queue.push_back(next);
            }
        };
        if r > 0 && !input.horizontal_walls[r - 1][c] {
            visit(cell - N);
        }
        if r + 1 < N && !input.horizontal_walls[r][c] {
            visit(cell + N);
        }
        if c > 0 && !input.vertical_walls[r][c - 1] {
            visit(cell - 1);
        }
        if c + 1 < N && !input.vertical_walls[r][c] {
            visit(cell + 1);
        }
    }
    assert_eq!(bfs_order.len(), CELLS);

    let mut tree = vec![Vec::new(); CELLS];
    for vertex in 1..CELLS {
        tree[vertex].push(parent[vertex]);
        tree[parent[vertex]].push(vertex);
    }

    let mut board = input.initial_board;
    let mut position = [0; CELLS];
    for cell in 0..CELLS {
        position[board[cell]] = cell;
    }
    let mut active = [true; CELLS];
    let mut operations = Vec::with_capacity(BASELINE_MAX_OPERATIONS);

    // BFS では親が子より先に現れるため、逆順なら対象は常に残存木の葉である。
    for &target in bfs_order[1..].iter().rev() {
        let source = position[target];
        assert!(active[source] && active[target]);

        let mut previous = [usize::MAX; CELLS];
        let mut path_queue = VecDeque::new();
        previous[source] = source;
        path_queue.push_back(source);
        while let Some(vertex) = path_queue.pop_front() {
            if vertex == target {
                break;
            }
            for &next in &tree[vertex] {
                if active[next] && previous[next] == usize::MAX {
                    previous[next] = vertex;
                    path_queue.push_back(next);
                }
            }
        }
        assert_ne!(previous[target], usize::MAX);

        let mut path = vec![target];
        while *path.last().unwrap() != source {
            path.push(previous[*path.last().unwrap()]);
        }
        path.reverse();
        for edge in path.windows(2) {
            let a = edge[0];
            let b = edge[1];
            let card_a = board[a];
            let card_b = board[b];
            board.swap(a, b);
            position[card_a] = b;
            position[card_b] = a;
            operations.push(edge_operation(a, b));
        }
        assert_eq!(position[target], target);
        active[target] = false;
    }
    debug_assert_eq!(board[0], 0);
    assert!(operations.len() <= BASELINE_MAX_OPERATIONS);
    operations
}

#[cfg(feature = "local")]
fn assert_color_decomposition(color: &[usize; CELLS], source_of_card: &[usize; CELLS]) {
    for source in 0..N {
        let mut seen = [false; N];
        for card in 0..CELLS {
            if source_of_card[card] == source {
                assert!(!seen[color[card]]);
                seen[color[card]] = true;
            }
        }
        assert!(seen.iter().all(|&value| value));
    }
    for target in 0..N {
        let mut seen = [false; N];
        for card in target * N..(target + 1) * N {
            assert!(!seen[color[card]]);
            seen[color[card]] = true;
        }
        assert!(seen.iter().all(|&value| value));
    }
}

#[cfg(feature = "local")]
fn apply_operation(board: &mut [usize; CELLS], op: Operation, input: &Input) {
    assert!(op.h > 0 && op.w > 0);
    assert!(op.r + op.h <= N && op.c + op.w <= N);
    match op.direction {
        Direction::Vertical => {
            assert_eq!(op.h % 2, 0);
            assert!(op.h / 2 <= MAX_SINGLE_AXIS_SHIFT);
        }
        Direction::Horizontal => {
            assert_eq!(op.w % 2, 0);
            assert!(op.w / 2 <= MAX_SINGLE_AXIS_SHIFT);
        }
    }
    for r in op.r..op.r + op.h {
        for c in op.c..op.c + op.w - 1 {
            assert!(!input.vertical_walls[r][c]);
        }
    }
    for r in op.r..op.r + op.h - 1 {
        for c in op.c..op.c + op.w {
            assert!(!input.horizontal_walls[r][c]);
        }
    }

    match op.direction {
        Direction::Vertical => {
            for x in 0..op.h / 2 {
                for y in 0..op.w {
                    board.swap(
                        (op.r + x) * N + op.c + y,
                        (op.r + op.h / 2 + x) * N + op.c + y,
                    );
                }
            }
        }
        Direction::Horizontal => {
            for x in 0..op.h {
                for y in 0..op.w / 2 {
                    board.swap(
                        (op.r + x) * N + op.c + y,
                        (op.r + x) * N + op.c + op.w / 2 + y,
                    );
                }
            }
        }
    }
}

#[cfg(feature = "local")]
fn assert_swap_routes(input: &Input, routes: &RoutingTable) {
    for r in 0..N {
        for c in 0..N - 1 {
            let a = r * N + c;
            let b = a + 1;
            let mut board = std::array::from_fn(|cell| cell);
            for &op in &routes.horizontal(r, c).operations {
                apply_operation(&mut board, op, input);
            }
            for cell in 0..CELLS {
                let expected = if cell == a {
                    b
                } else if cell == b {
                    a
                } else {
                    cell
                };
                assert_eq!(board[cell], expected);
            }
        }
    }
    for r in 0..N - 1 {
        for c in 0..N {
            let a = r * N + c;
            let b = a + N;
            let mut board = std::array::from_fn(|cell| cell);
            for &op in &routes.vertical(r, c).operations {
                apply_operation(&mut board, op, input);
            }
            for cell in 0..CELLS {
                let expected = if cell == a {
                    b
                } else if cell == b {
                    a
                } else {
                    cell
                };
                assert_eq!(board[cell], expected);
            }
        }
    }
}

#[cfg(feature = "local")]
fn assert_replay_complete(input: &Input, operations: &[Operation]) {
    let mut board = input.initial_board;
    for &operation in operations {
        apply_operation(&mut board, operation, input);
    }
    for cell in 0..CELLS {
        assert_eq!(board[cell], cell);
    }
}

fn main() {
    // 全処理を時間制限に含めるため main 開始直後に作る。
    // 終了後に最大 79,800 操作の構築・検証・出力が残るため、探索末尾は
    // 32 反復ごとに時計を更新して 94% 境界の超過を小さく抑える。
    let mut time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 5);
    let input = Input::read();
    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();
    local! {
        for key in [
            "matching_decompositions",
            "cycle_mutations",
            "accepted_mutations",
            "best_updates",
            "routed_swaps",
            "stage1_logical",
            "stage1_physical",
            "stage2_logical",
            "stage2_physical",
            "stage3_logical",
            "stage3_physical",
            "first_t",
            "best_t",
            "baseline_t",
            "three_stage_t",
            "three_stage_selected",
        ] {
            trace.count_by(key, 0);
        }
    }

    let mut source_of_card = [0; CELLS];
    for cell in 0..CELLS {
        let card = input.initial_board[cell];
        source_of_card[card] = cell / N;
    }

    let routes = RoutingTable::new(&input);
    local! {
        assert_swap_routes(&input, &routes);
    }
    // 上限保証候補も探索結果にかかわらず常に完成させる通常 portfolio とする。
    let baseline_operations = construct_tree_baseline(&input);
    local! {
        assert!(baseline_operations.len() <= BASELINE_MAX_OPERATIONS);
        assert_replay_complete(&input, &baseline_operations);
    }

    let mut rng = Rng::new(0x68d8_0105_5eed_f00d);
    let mut current_color = decompose_matchings(&input, &source_of_card, false, &mut rng);
    local! {
        trace.count("matching_decompositions");
        assert_color_decomposition(&current_color, &source_of_card);
    }
    let first_cost = evaluate(&input, &current_color, &routes);
    let mut current_cost = first_cost;
    let mut best_color = current_color;
    let mut best_cost = first_cost;

    let _search_start = Instant::now();
    let start_temperature = (first_cost as f64 * 0.005).max(20.0);
    let end_temperature = 0.5;
    let mut cycle_mutations = 0_u64;
    time_keeper.force_update();
    loop {
        if !time_keeper.step() || time_keeper.progress() >= SEARCH_END_RATIO {
            break;
        }

        // 定期的な再分解で、交互閉路近傍だけでは届きにくい成分へ移る。
        if cycle_mutations > 0 && cycle_mutations % 16_384 == 0 {
            current_color = decompose_matchings(&input, &source_of_card, true, &mut rng);
            current_cost = evaluate(&input, &current_color, &routes);
            local! {
                trace.count("matching_decompositions");
                assert_color_decomposition(&current_color, &source_of_card);
            }
            if current_cost < best_cost {
                best_cost = current_cost;
                best_color = current_color;
                local! { trace.count("best_updates"); }
            }
        }

        let a = rng.usize(N);
        let mut b = rng.usize(N - 1);
        if b >= a {
            b += 1;
        }
        let changed_cards =
            mutate_alternating_cycle(&mut current_color, &source_of_card, a, b, &mut rng);
        cycle_mutations += 1;
        local! { trace.count("cycle_mutations"); }

        let next_cost = evaluate(&input, &current_color, &routes);
        let progress = (time_keeper.progress() / SEARCH_END_RATIO).clamp(0.0, 1.0);
        let temperature = start_temperature * (end_temperature / start_temperature).powf(progress);
        let accepted = next_cost <= current_cost
            || rng.f64() < (-(next_cost as f64 - current_cost as f64) / temperature).exp();
        if accepted {
            current_cost = next_cost;
            local! { trace.count("accepted_mutations"); }
            if current_cost < best_cost {
                best_cost = current_cost;
                best_color = current_color;
                local! { trace.count("best_updates"); }
            }
        } else {
            // 反転は二色交換なので、同じカード集合をもう一度交換すれば undo できる。
            for card in changed_cards {
                current_color[card] = if current_color[card] == a { b } else { a };
            }
        }
    }
    time_keeper.force_update();
    local! {
        trace.add_time_ms("search_elapsed_ms", _search_start.elapsed().as_secs_f64() * 1000.0);
        trace.count_by("first_t", first_cost as i64);
        trace.count_by("best_t", best_cost as i64);
        assert_color_decomposition(&best_color, &source_of_card);
    }

    // 厳密評価値だけで採否を先に決め、非選択時は巨大になり得る物理列を展開しない。
    let three_stage_selected = best_cost <= baseline_operations.len();
    let (three_stage_operations, _build_stats) = construct_operations(
        &input,
        &best_color,
        &routes,
        best_cost,
        three_stage_selected,
    );
    local! {
        trace.count_by("routed_swaps", _build_stats.routed_swaps as i64);
        trace.count_by("stage1_logical", _build_stats.logical[0] as i64);
        trace.count_by("stage1_physical", _build_stats.physical[0] as i64);
        trace.count_by("stage2_logical", _build_stats.logical[1] as i64);
        trace.count_by("stage2_physical", _build_stats.physical[1] as i64);
        trace.count_by("stage3_logical", _build_stats.logical[2] as i64);
        trace.count_by("stage3_physical", _build_stats.physical[2] as i64);
        trace.count_by("baseline_t", baseline_operations.len() as i64);
        trace.count_by("three_stage_t", best_cost as i64);
        trace.count_by("three_stage_selected", three_stage_selected as i64);
        if three_stage_selected {
            assert_replay_complete(&input, &three_stage_operations);
        } else {
            assert!(three_stage_operations.is_empty());
        }
    }
    let operations = if three_stage_selected {
        three_stage_operations
    } else {
        baseline_operations
    };
    assert!(operations.len() <= MAX_OPERATIONS);
    local! {
        // 実際に選択した最終出力についても、初期盤面からの合法 replay を明示確認する。
        assert_replay_complete(&input, &operations);
        trace.summary();
    }
    let _ = time_keeper.exact_elapsed_sec();
    write_output(&operations);
}
