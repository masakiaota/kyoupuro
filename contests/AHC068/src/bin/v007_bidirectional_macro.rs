// v007_bidirectional_macro.rs
use std::{
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

#[cfg(feature = "local")]
use std::collections::BTreeMap;

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_SINGLE_AXIS_SHIFT: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;
const LEAF_FIXING_UPPER_BOUND: usize = CELLS * (CELLS - 1) / 2;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};
/// 出力と local 検証の余裕を残し、探索には時間上限の 95% を使う。
const SEARCH_DEADLINE_RATIO: f64 = 0.95;

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

        let input_n: usize = tokens.next().unwrap().parse().unwrap();
        assert_eq!(input_n, N);
        let initial_board: [usize; CELLS] =
            std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            assert_eq!(row.len(), N - 1);
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            assert_eq!(row.len(), N);
            std::array::from_fn(|j| row[j] == b'1')
        });

        let mut seen = [false; CELLS];
        for &card in &initial_board {
            assert!(card < CELLS && !seen[card]);
            seen[card] = true;
        }

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

#[derive(Clone)]
struct State {
    board: [u16; CELLS],
    position: [u16; CELLS],
}

impl State {
    fn new(initial_board: &[usize; CELLS]) -> Self {
        let board = std::array::from_fn(|cell| initial_board[cell] as u16);
        let mut position = [0_u16; CELLS];
        for cell in 0..CELLS {
            position[board[cell] as usize] = cell as u16;
        }
        Self { board, position }
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
    fn swap_cells(&mut self, lhs: usize, rhs: usize) {
        let lhs_card = self.board[lhs];
        let rhs_card = self.board[rhs];
        self.board.swap(lhs, rhs);
        self.position[lhs_card as usize] = rhs as u16;
        self.position[rhs_card as usize] = lhs as u16;
    }

    fn apply(&mut self, op: Operation) {
        match op.direction {
            Direction::Vertical => {
                let shift = op.h / 2;
                for x in 0..shift {
                    for y in 0..op.w {
                        let upper = (op.r + x) * N + op.c + y;
                        let lower = (op.r + shift + x) * N + op.c + y;
                        self.swap_cells(upper, lower);
                    }
                }
            }
            Direction::Horizontal => {
                let shift = op.w / 2;
                for x in 0..op.h {
                    for y in 0..shift {
                        let left = (op.r + x) * N + op.c + y;
                        let right = (op.r + x) * N + op.c + shift + y;
                        self.swap_cells(left, right);
                    }
                }
            }
        }
    }

    fn is_identity(&self) -> bool {
        (0..CELLS).all(|cell| self.card_at(cell) == cell)
    }

    #[cfg(feature = "local")]
    fn assert_consistent(&self) {
        for cell in 0..CELLS {
            let card = self.card_at(cell);
            assert_eq!(self.position_of(card), cell);
        }
    }
}

#[derive(Clone, Copy)]
struct MoveEdge {
    to: u16,
    operation_id: u16,
}

struct CatalogOperation {
    operation: Operation,
}

struct Geometry {
    open_adjacency: Vec<Vec<usize>>,
    operations: Vec<CatalogOperation>,
    moves_from: Vec<Vec<MoveEdge>>,
    operations_containing: Vec<Vec<u16>>,
}

impl Geometry {
    fn new(input: &Input) -> Self {
        let mut geometry = Self {
            open_adjacency: Self::build_open_adjacency(input),
            operations: Vec::with_capacity(4_000),
            moves_from: (0..CELLS).map(|_| Vec::new()).collect(),
            operations_containing: (0..CELLS).map(|_| Vec::new()).collect(),
        };

        for r in 0..N {
            for shift in 1..=MAX_SINGLE_AXIS_SHIFT {
                let w = 2 * shift;
                for c in 0..=N - w {
                    let op = Operation {
                        direction: Direction::Horizontal,
                        r,
                        c,
                        h: 1,
                        w,
                    };
                    if is_operation_legal(input, op) {
                        geometry.add_operation(op);
                    }
                }
            }
        }
        for c in 0..N {
            for shift in 1..=MAX_SINGLE_AXIS_SHIFT {
                let h = 2 * shift;
                for r in 0..=N - h {
                    let op = Operation {
                        direction: Direction::Vertical,
                        r,
                        c,
                        h,
                        w: 1,
                    };
                    if is_operation_legal(input, op) {
                        geometry.add_operation(op);
                    }
                }
            }
        }

        assert!(geometry.operations.len() <= u16::MAX as usize);
        geometry
    }

    fn build_open_adjacency(input: &Input) -> Vec<Vec<usize>> {
        let mut adjacency: Vec<Vec<usize>> = (0..CELLS).map(|_| Vec::new()).collect();
        for i in 0..N {
            for j in 0..N {
                let cell = i * N + j;
                if j + 1 < N && !input.vertical_walls[i][j] {
                    let right = cell + 1;
                    adjacency[cell].push(right);
                    adjacency[right].push(cell);
                }
                if i + 1 < N && !input.horizontal_walls[i][j] {
                    let down = cell + N;
                    adjacency[cell].push(down);
                    adjacency[down].push(cell);
                }
            }
        }
        adjacency
    }

    fn add_operation(&mut self, operation: Operation) {
        let operation_id = self.operations.len() as u16;
        for i in operation.r..operation.r + operation.h {
            for j in operation.c..operation.c + operation.w {
                self.operations_containing[i * N + j].push(operation_id);
            }
        }

        match operation.direction {
            Direction::Horizontal => {
                let shift = operation.w / 2;
                for y in 0..shift {
                    let left = operation.r * N + operation.c + y;
                    let right = operation.r * N + operation.c + shift + y;
                    self.moves_from[left].push(MoveEdge {
                        to: right as u16,
                        operation_id,
                    });
                    self.moves_from[right].push(MoveEdge {
                        to: left as u16,
                        operation_id,
                    });
                }
            }
            Direction::Vertical => {
                let shift = operation.h / 2;
                for x in 0..shift {
                    let upper = (operation.r + x) * N + operation.c;
                    let lower = (operation.r + shift + x) * N + operation.c;
                    self.moves_from[upper].push(MoveEdge {
                        to: lower as u16,
                        operation_id,
                    });
                    self.moves_from[lower].push(MoveEdge {
                        to: upper as u16,
                        operation_id,
                    });
                }
            }
        }
        self.operations.push(CatalogOperation { operation });
    }
}

fn is_operation_legal(input: &Input, op: Operation) -> bool {
    if op.h == 0 || op.w == 0 || op.r + op.h > N || op.c + op.w > N {
        return false;
    }
    match op.direction {
        Direction::Vertical if op.h % 2 != 0 => return false,
        Direction::Horizontal if op.w % 2 != 0 => return false,
        _ => {}
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

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn index(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next_u64() % upper as u64) as usize
    }
}

fn random_spanning_tree(geometry: &Geometry, rng: &mut SplitMix64) -> Vec<Vec<usize>> {
    let mut tree: Vec<Vec<usize>> = (0..CELLS).map(|_| Vec::new()).collect();
    let mut visited = [false; CELLS];
    let root = rng.index(CELLS);
    visited[root] = true;
    let mut visited_count = 1;
    let mut frontier = Vec::<(usize, usize)>::new();
    for &next in &geometry.open_adjacency[root] {
        frontier.push((root, next));
    }

    while visited_count < CELLS {
        assert!(!frontier.is_empty(), "open cell graph must be connected");
        let index = rng.index(frontier.len());
        let (from, to) = frontier.swap_remove(index);
        if visited[to] {
            continue;
        }
        assert!(visited[from]);
        visited[to] = true;
        visited_count += 1;
        tree[from].push(to);
        tree[to].push(from);
        for &next in &geometry.open_adjacency[to] {
            if !visited[next] {
                frontier.push((to, next));
            }
        }
    }

    assert_eq!(tree.iter().map(Vec::len).sum::<usize>(), 2 * (CELLS - 1));
    tree
}

fn choose_leaf(
    state: &State,
    active: &[bool; CELLS],
    degree: &[u8; CELLS],
    rng: &mut SplitMix64,
) -> usize {
    let mut best_score = u64::MAX;
    let mut best_leaf = CELLS;
    for leaf in 0..CELLS {
        if !active[leaf] || degree[leaf] > 1 {
            continue;
        }
        let source = state.position_of(leaf);
        assert!(active[source]);
        let row_distance = (source / N).abs_diff(leaf / N);
        let col_distance = (source % N).abs_diff(leaf % N);
        let macro_lower_bound = row_distance.div_ceil(MAX_SINGLE_AXIS_SHIFT)
            + col_distance.div_ceil(MAX_SINGLE_AXIS_SHIFT);
        let manhattan = row_distance + col_distance;
        // 大きな一枚搬送距離の差は保ち、近い葉の間だけ乱択して計画を多様化する。
        let noise = rng.next_u64() & 255;
        let score = (macro_lower_bound as u64) * 4096 + (manhattan as u64) * 64 + noise;
        if score < best_score {
            best_score = score;
            best_leaf = leaf;
        }
    }
    assert!(best_leaf < CELLS);
    best_leaf
}

fn shortest_macro_steps(
    source: usize,
    target: usize,
    blocked_operation_count: &[u8],
    geometry: &Geometry,
    rng: &mut SplitMix64,
) -> Vec<(u16, usize)> {
    if source == target {
        return Vec::new();
    }

    const UNVISITED: u16 = u16::MAX;
    let mut parent = [UNVISITED; CELLS];
    let mut parent_operation = [UNVISITED; CELLS];
    let mut queue = [0_usize; CELLS];
    let mut head = 0;
    let mut tail = 1;
    queue[0] = source;
    parent[source] = source as u16;

    while head < tail && parent[target] == UNVISITED {
        let cell = queue[head];
        head += 1;
        let edges = &geometry.moves_from[cell];
        let start = if edges.is_empty() {
            0
        } else {
            rng.index(edges.len())
        };
        for offset in 0..edges.len() {
            let edge = edges[(start + offset) % edges.len()];
            if blocked_operation_count[edge.operation_id as usize] != 0 {
                continue;
            }
            let next = edge.to as usize;
            if parent[next] != UNVISITED {
                continue;
            }
            parent[next] = cell as u16;
            parent_operation[next] = edge.operation_id;
            queue[tail] = next;
            tail += 1;
            if next == target {
                break;
            }
        }
    }
    assert_ne!(
        parent[target], UNVISITED,
        "active macro graph must stay connected"
    );

    let mut reversed_steps = Vec::new();
    let mut current = target;
    while current != source {
        let operation_id = parent_operation[current];
        assert_ne!(operation_id, UNVISITED);
        reversed_steps.push((operation_id, current));
        current = parent[current] as usize;
    }
    reversed_steps.reverse();
    reversed_steps
}

fn construct_complete_plan(
    initial_board: &[usize; CELLS],
    geometry: &Geometry,
    seed: u64,
    timer: &TimeKeeper,
    deadline_sec: Option<f64>,
) -> Option<Vec<Operation>> {
    let mut rng = SplitMix64::new(seed);
    let tree = random_spanning_tree(geometry, &mut rng);
    let mut degree: [u8; CELLS] = std::array::from_fn(|cell| tree[cell].len() as u8);
    let mut active = [true; CELLS];
    let mut blocked_operation_count = vec![0_u8; geometry.operations.len()];
    let mut state = State::new(initial_board);
    let mut operations = Vec::with_capacity(4_000);

    for _remaining in (2..=CELLS).rev() {
        if deadline_sec.is_some_and(|deadline| timer.exact_elapsed_sec() >= deadline) {
            return None;
        }

        let leaf = choose_leaf(&state, &active, &degree, &mut rng);
        assert_eq!(degree[leaf], 1);
        let source = state.position_of(leaf);
        assert!(active[source]);
        let steps =
            shortest_macro_steps(source, leaf, &blocked_operation_count, geometry, &mut rng);
        let mut expected_position = source;
        for (operation_id, next_position) in steps {
            let catalog_operation = &geometry.operations[operation_id as usize];
            assert_eq!(blocked_operation_count[operation_id as usize], 0);
            state.apply(catalog_operation.operation);
            operations.push(catalog_operation.operation);
            expected_position = next_position;
            assert_eq!(state.position_of(leaf), expected_position);
        }
        assert_eq!(expected_position, leaf);
        assert_eq!(state.card_at(leaf), leaf);

        active[leaf] = false;
        for &operation_id in &geometry.operations_containing[leaf] {
            let count = &mut blocked_operation_count[operation_id as usize];
            *count = count.checked_add(1).unwrap();
        }
        let parent = tree[leaf]
            .iter()
            .copied()
            .find(|&neighbor| active[neighbor])
            .unwrap();
        degree[leaf] = 0;
        degree[parent] -= 1;

        if operations.len() > LEAF_FIXING_UPPER_BOUND {
            panic!("leaf-fixing upper bound was violated");
        }
    }

    assert!(state.is_identity());
    assert!(operations.len() <= LEAF_FIXING_UPPER_BOUND);
    assert!(operations.len() <= MAX_OPERATIONS);
    Some(operations)
}

fn inverse_board(initial_board: &[usize; CELLS]) -> [usize; CELLS] {
    let mut inverse = [0_usize; CELLS];
    for cell in 0..CELLS {
        inverse[initial_board[cell]] = cell;
    }
    for cell in 0..CELLS {
        assert_eq!(inverse[initial_board[cell]], cell);
        assert_eq!(initial_board[inverse[cell]], cell);
    }
    inverse
}

fn input_seed(input: &Input) -> u64 {
    let mut hash = 0x243f_6a88_85a3_08d3_u64;
    for (cell, &card) in input.initial_board.iter().enumerate() {
        hash ^= (card as u64)
            .wrapping_add((cell as u64) << 32)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15);
        hash = hash.rotate_left(17).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    }
    hash
}

#[cfg(feature = "local")]
#[derive(Default)]
struct TraceStats {
    fallback_count: usize,
    counts: BTreeMap<&'static str, i64>,
    times_ms: BTreeMap<&'static str, f64>,
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
    ($($body:tt)*) => {{ $($body)* }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

struct TimeKeeper {
    start: Instant,
}

impl TimeKeeper {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

#[cfg(feature = "local")]
fn replay_and_validate(input: &Input, operations: &[Operation]) {
    let mut board = input.initial_board;
    for &op in operations {
        assert!(is_operation_legal(input, op));
        match op.direction {
            Direction::Vertical => {
                let shift = op.h / 2;
                for x in 0..shift {
                    for y in 0..op.w {
                        board.swap((op.r + x) * N + op.c + y, (op.r + shift + x) * N + op.c + y);
                    }
                }
            }
            Direction::Horizontal => {
                let shift = op.w / 2;
                for x in 0..op.h {
                    for y in 0..shift {
                        board.swap((op.r + x) * N + op.c + y, (op.r + x) * N + op.c + shift + y);
                    }
                }
            }
        }
    }
    assert!((0..CELLS).all(|cell| board[cell] == cell));
}

fn main() {
    // 入力と前処理も同じ時間予算に含めるため、main の先頭で作る。
    let timer = TimeKeeper::new();
    let input = Input::read();
    let geometry = Geometry::new(&input);
    let reverse_initial_board = inverse_board(&input.initial_board);
    let seed = input_seed(&input);

    let mut forward_seed_rng = SplitMix64::new(seed ^ 0xa409_3822_299f_31d0);
    let mut reverse_seed_rng = SplitMix64::new(seed ^ 0x082e_fa98_ec4e_6c89);

    // 制限時間付き探索の前に、両方向で完成済み incumbent を一本ずつ作る。
    let mut best_forward = construct_complete_plan(
        &input.initial_board,
        &geometry,
        forward_seed_rng.next_u64(),
        &timer,
        None,
    )
    .unwrap();
    let mut best_reverse = construct_complete_plan(
        &reverse_initial_board,
        &geometry,
        reverse_seed_rng.next_u64(),
        &timer,
        None,
    )
    .unwrap();
    let mut forward_trials = 1_usize;
    let mut reverse_trials = 1_usize;

    let search_start = timer.exact_elapsed_sec();
    let search_deadline = PROGRAM_TIME_LIMIT_SEC * SEARCH_DEADLINE_RATIO;
    let forward_deadline = search_start + (search_deadline - search_start).max(0.0) * 0.5;

    while timer.exact_elapsed_sec() < forward_deadline {
        let candidate = construct_complete_plan(
            &input.initial_board,
            &geometry,
            forward_seed_rng.next_u64(),
            &timer,
            Some(forward_deadline),
        );
        let Some(candidate) = candidate else {
            break;
        };
        forward_trials += 1;
        if candidate.len() < best_forward.len() {
            best_forward = candidate;
        }
    }
    while timer.exact_elapsed_sec() < search_deadline {
        let candidate = construct_complete_plan(
            &reverse_initial_board,
            &geometry,
            reverse_seed_rng.next_u64(),
            &timer,
            Some(search_deadline),
        );
        let Some(candidate) = candidate else {
            break;
        };
        reverse_trials += 1;
        if candidate.len() < best_reverse.len() {
            best_reverse = candidate;
        }
    }

    let reverse_selected = best_reverse.len() < best_forward.len();
    let forward_best_t = best_forward.len();
    let reverse_best_t = best_reverse.len();
    #[cfg(feature = "local")]
    let forward_solution_for_validation = best_forward.clone();
    #[cfg(feature = "local")]
    let reverse_solution_for_validation = {
        let mut operations = best_reverse.clone();
        operations.reverse();
        operations
    };
    let operations = if reverse_selected {
        best_reverse.reverse();
        best_reverse
    } else {
        best_forward
    };
    assert!(forward_trials >= 1 && reverse_trials >= 1);
    assert_eq!(operations.len(), forward_best_t.min(reverse_best_t));
    assert!(operations.len() <= LEAF_FIXING_UPPER_BOUND);
    assert!(operations.len() <= MAX_OPERATIONS);

    local! {
        let mut trace = TraceStats::default();
        let complete_plans = forward_trials + reverse_trials;
        let non_adjacent_ops = operations
            .iter()
            .filter(|op| match op.direction {
                Direction::Vertical => op.h / 2 > 1,
                Direction::Horizontal => op.w / 2 > 1,
            })
            .count();
        trace.count_by("complete_plans", complete_plans as i64);
        trace.count_by("forward_trials", forward_trials as i64);
        trace.count_by("reverse_trials", reverse_trials as i64);
        trace.count_by("forward_complete_plans", forward_trials as i64);
        trace.count_by("reverse_complete_plans", reverse_trials as i64);
        trace.count_by("non_adjacent_ops", non_adjacent_ops as i64);
        trace.count_by("forward_best_t", forward_best_t as i64);
        trace.count_by("reverse_best_t", reverse_best_t as i64);
        trace.count_by("portfolio_t", operations.len() as i64);
        trace.count_by("reverse_selected", reverse_selected as i64);
        trace.add_time_ms("search_elapsed_ms", timer.exact_elapsed_sec() * 1000.0);

        let replay_state = State::new(&input.initial_board);
        replay_state.assert_consistent();
        replay_and_validate(&input, &forward_solution_for_validation);
        replay_and_validate(&input, &reverse_solution_for_validation);
        replay_and_validate(&input, &operations);
        trace.summary();
    }

    write_output(&operations);
}
