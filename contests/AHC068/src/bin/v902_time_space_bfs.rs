// v902_time_space_bfs.rs
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_HALF_LENGTH: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;
const MASK_WORDS: usize = CELLS.div_ceil(64);
const UNREACHED: u16 = u16::MAX;
const NO_PARENT: u32 = u32::MAX;

type Mask = [u64; MASK_WORDS];

struct Input {
    initial_board: [usize; CELLS],
    /// `vertical_walls[r][c]` は `(r, c)` と `(r, c + 1)` の間の壁。
    vertical_walls: [[bool; N - 1]; N],
    /// `horizontal_walls[r][c]` は `(r, c)` と `(r + 1, c)` の間の壁。
    horizontal_walls: [[bool; N]; N - 1],
}

impl Input {
    fn read() -> Self {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap();
        let mut tokens = source.split_whitespace();

        let input_n: usize = tokens.next().unwrap().parse().unwrap();
        assert_eq!(input_n, N);
        let initial_board = std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|c| row[c] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|c| row[c] == b'1')
        });

        Self {
            initial_board,
            vertical_walls,
            horizontal_walls,
        }
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

/// 親辺に保存する新規操作を 1 word に収める。
/// 0 は既存操作を通過した辺、非 0 は挿入した操作を表す。
fn pack_operation(operation: Operation) -> u32 {
    let direction = match operation.direction {
        Direction::Vertical => 0,
        Direction::Horizontal => 1,
    };
    (direction << 20)
        | ((operation.r as u32) << 15)
        | ((operation.c as u32) << 10)
        | ((operation.h as u32) << 5)
        | operation.w as u32
}

fn unpack_operation(packed: u32) -> Operation {
    assert_ne!(packed, 0);
    Operation {
        direction: if (packed >> 20) & 1 == 0 {
            Direction::Vertical
        } else {
            Direction::Horizontal
        },
        r: ((packed >> 15) & 31) as usize,
        c: ((packed >> 10) & 31) as usize,
        h: ((packed >> 5) & 31) as usize,
        w: (packed & 31) as usize,
    }
}

#[inline]
fn mask_contains(mask: &Mask, cell: usize) -> bool {
    (mask[cell / 64] >> (cell % 64)) & 1 != 0
}

#[inline]
fn mask_set(mask: &mut Mask, cell: usize) {
    mask[cell / 64] |= 1_u64 << (cell % 64);
}

#[inline]
fn mask_swap(mask: &mut Mask, first: usize, second: usize) {
    if mask_contains(mask, first) != mask_contains(mask, second) {
        mask[first / 64] ^= 1_u64 << (first % 64);
        mask[second / 64] ^= 1_u64 << (second % 64);
    }
}

/// 操作をカード位置へ適用する。長方形の外側なら位置は変わらない。
#[inline]
fn apply_to_cell(operation: Operation, cell: usize) -> usize {
    let r = cell / N;
    let c = cell % N;
    if r < operation.r
        || operation.r + operation.h <= r
        || c < operation.c
        || operation.c + operation.w <= c
    {
        return cell;
    }

    match operation.direction {
        Direction::Vertical => {
            let half = operation.h / 2;
            if r < operation.r + half {
                cell + half * N
            } else {
                cell - half * N
            }
        }
        Direction::Horizontal => {
            let half = operation.w / 2;
            if c < operation.c + half {
                cell + half
            } else {
                cell - half
            }
        }
    }
}

fn apply_to_mask(mask: &mut Mask, operation: Operation) {
    match operation.direction {
        Direction::Vertical => {
            let half = operation.h / 2;
            for dr in 0..half {
                for dc in 0..operation.w {
                    let first = (operation.r + dr) * N + operation.c + dc;
                    let second = (operation.r + half + dr) * N + operation.c + dc;
                    mask_swap(mask, first, second);
                }
            }
        }
        Direction::Horizontal => {
            let half = operation.w / 2;
            for dr in 0..operation.h {
                for dc in 0..half {
                    let first = (operation.r + dr) * N + operation.c + dc;
                    let second = (operation.r + dr) * N + operation.c + half + dc;
                    mask_swap(mask, first, second);
                }
            }
        }
    }
}

struct BoardState {
    board: [usize; CELLS],
}

impl BoardState {
    fn new(initial_board: [usize; CELLS]) -> Self {
        Self {
            board: initial_board,
        }
    }

    fn apply(&mut self, operation: Operation) {
        match operation.direction {
            Direction::Vertical => {
                let half = operation.h / 2;
                for dr in 0..half {
                    for dc in 0..operation.w {
                        let first = (operation.r + dr) * N + operation.c + dc;
                        let second = (operation.r + half + dr) * N + operation.c + dc;
                        self.board.swap(first, second);
                    }
                }
            }
            Direction::Horizontal => {
                let half = operation.w / 2;
                for dr in 0..operation.h {
                    for dc in 0..half {
                        let first = (operation.r + dr) * N + operation.c + dc;
                        let second = (operation.r + dr) * N + operation.c + half + dc;
                        self.board.swap(first, second);
                    }
                }
            }
        }
    }
}

/// 中心を根とする BFS の発見順を反転する。
fn target_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut queue = VecDeque::new();
    let mut visited = [false; CELLS];
    let mut order = Vec::with_capacity(CELLS);

    visited[root] = true;
    queue.push_back(root);
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        let r = cell / N;
        let c = cell % N;

        if r > 0 {
            let next = cell - N;
            if !input.horizontal_walls[r - 1][c] && !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
        if r + 1 < N {
            let next = cell + N;
            if !input.horizontal_walls[r][c] && !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
        if c > 0 {
            let next = cell - 1;
            if !input.vertical_walls[r][c - 1] && !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
        if c + 1 < N {
            let next = cell + 1;
            if !input.vertical_walls[r][c] && !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }

    assert_eq!(order.len(), CELLS);
    order.reverse();
    order
}

/// `cell` を含み、保護マスクと壁で止まる縦・横の連続区間を調べる。
/// 戻り値は上・下・左・右へ進めるマス数。
fn free_runs(input: &Input, protected: &Mask, cell: usize) -> (usize, usize, usize, usize) {
    let r = cell / N;
    let c = cell % N;

    let mut up = 0;
    while up < r
        && !mask_contains(protected, cell - N * (up + 1))
        && !input.horizontal_walls[r - up - 1][c]
    {
        up += 1;
    }

    let mut down = 0;
    while r + down + 1 < N
        && !mask_contains(protected, cell + N * (down + 1))
        && !input.horizontal_walls[r + down][c]
    {
        down += 1;
    }

    let mut left = 0;
    while left < c
        && !mask_contains(protected, cell - (left + 1))
        && !input.vertical_walls[r][c - left - 1]
    {
        left += 1;
    }

    let mut right = 0;
    while c + right + 1 < N
        && !mask_contains(protected, cell + right + 1)
        && !input.vertical_walls[r][c + right]
    {
        right += 1;
    }

    (up, down, left, right)
}

/// `dist` が改善したとき、コスト 0 の辺なら deque の先頭、コスト 1 の辺なら末尾へ入れる。
#[inline]
fn relax(
    deque: &mut VecDeque<(usize, u16)>,
    dist: &mut [u16],
    parent: &mut [u32],
    action: &mut [u32],
    from: usize,
    to: usize,
    new_dist: u16,
    packed_action: u32,
) {
    if new_dist < dist[to] {
        dist[to] = new_dist;
        parent[to] = from as u32;
        action[to] = packed_action;
        if packed_action == 0 {
            deque.push_front((to, new_dist));
        } else {
            deque.push_back((to, new_dist));
        }
    }
}

/// カード `target` を、現在の既存操作列 `operations` を壊さずに完成させる。
///
/// ノード `(cell, t)` は「カード target が基準フレーム t で cell にいる」状態である。
/// 既存操作の通過はコスト0、新しい細長い長方形の挿入はコスト1として 0-1 BFS を行う。
fn insert_route_for_card(
    input: &Input,
    operations: &[Operation],
    source: usize,
    target: usize,
    initial_protected: Mask,
    #[cfg(feature = "local")] trace: &mut Trace,
) -> Vec<Operation> {
    let operation_count = operations.len();
    let state_count = (operation_count + 1) * CELLS;

    // `protected_at[t]` は既存操作を t 個通過した直後の完成済みカード位置である。
    let mut protected_at = Vec::with_capacity(operation_count + 1);
    protected_at.push(initial_protected);
    for &operation in operations {
        let mut next = *protected_at.last().unwrap();
        apply_to_mask(&mut next, operation);
        protected_at.push(next);
    }

    let start = source;
    let goal = operation_count * CELLS + target;
    let mut dist = vec![UNREACHED; state_count];
    let mut parent = vec![NO_PARENT; state_count];
    let mut action = vec![0_u32; state_count];
    let mut deque = VecDeque::new();
    dist[start] = 0;
    deque.push_back((start, 0));

    let goal_dist = loop {
        let (state, current_dist) = deque
            .pop_front()
            .expect("the unprotected cells keep a connected fallback route");
        if current_dist != dist[state] {
            continue;
        }
        if state == goal {
            break current_dist;
        }

        let t = state / CELLS;
        let cell = state % CELLS;
        debug_assert!(!mask_contains(&protected_at[t], cell));

        // 既存操作 O_t を通過する。操作の外にいるカードは同じマスへ進む。
        if t < operation_count {
            let next_cell = apply_to_cell(operations[t], cell);
            let next_state = (t + 1) * CELLS + next_cell;
            relax(
                &mut deque,
                &mut dist,
                &mut parent,
                &mut action,
                state,
                next_state,
                current_dist,
                0,
            );
        }

        // この基準フレームへ挿入できる、新しい細長い長方形を列挙する。
        let r = cell / N;
        let c = cell % N;
        let (up, down, left, right) = free_runs(input, &protected_at[t], cell);
        for k in 1..=MAX_HALF_LENGTH {
            // 下へ k マス。カードを縦長方形の上半分のどこへ置くかも選べる。
            let lower_offset = (2 * k).saturating_sub(down + 1);
            let upper_offset = (k - 1).min(up);
            if lower_offset <= upper_offset {
                let operation = Operation {
                    direction: Direction::Vertical,
                    r: r - lower_offset,
                    c,
                    h: 2 * k,
                    w: 1,
                };
                relax(
                    &mut deque,
                    &mut dist,
                    &mut parent,
                    &mut action,
                    state,
                    t * CELLS + cell + N * k,
                    current_dist + 1,
                    pack_operation(operation),
                );
            }

            // 上へ k マス。
            if up >= k {
                let lower_offset = k.saturating_sub(down + 1);
                let upper_offset = (k - 1).min(up - k);
                if lower_offset <= upper_offset {
                    let operation = Operation {
                        direction: Direction::Vertical,
                        r: r - k - lower_offset,
                        c,
                        h: 2 * k,
                        w: 1,
                    };
                    relax(
                        &mut deque,
                        &mut dist,
                        &mut parent,
                        &mut action,
                        state,
                        t * CELLS + cell - N * k,
                        current_dist + 1,
                        pack_operation(operation),
                    );
                }
            }

            // 右へ k マス。
            let lower_offset = (2 * k).saturating_sub(right + 1);
            let upper_offset = (k - 1).min(left);
            if lower_offset <= upper_offset {
                let operation = Operation {
                    direction: Direction::Horizontal,
                    r,
                    c: c - lower_offset,
                    h: 1,
                    w: 2 * k,
                };
                relax(
                    &mut deque,
                    &mut dist,
                    &mut parent,
                    &mut action,
                    state,
                    t * CELLS + cell + k,
                    current_dist + 1,
                    pack_operation(operation),
                );
            }

            // 左へ k マス。
            if left >= k {
                let lower_offset = k.saturating_sub(right + 1);
                let upper_offset = (k - 1).min(left - k);
                if lower_offset <= upper_offset {
                    let operation = Operation {
                        direction: Direction::Horizontal,
                        r,
                        c: c - k - lower_offset,
                        h: 1,
                        w: 2 * k,
                    };
                    relax(
                        &mut deque,
                        &mut dist,
                        &mut parent,
                        &mut action,
                        state,
                        t * CELLS + cell - k,
                        current_dist + 1,
                        pack_operation(operation),
                    );
                }
            }
        }
    };

    // ゴールから逆順に、挿入操作と既存操作通過の列を復元する。
    let mut reverse_actions = Vec::new();
    let mut state = goal;
    while state != start {
        reverse_actions.push(action[state]);
        state = parent[state] as usize;
    }
    reverse_actions.reverse();

    let mut rebuilt = Vec::with_capacity(operation_count + goal_dist as usize);
    let mut existing_index = 0;
    #[cfg(feature = "local")]
    let mut inserted_middle = 0;
    #[cfg(feature = "local")]
    let mut inserted_end = 0;
    for packed_action in reverse_actions {
        if packed_action == 0 {
            rebuilt.push(operations[existing_index]);
            existing_index += 1;
        } else {
            #[cfg(feature = "local")]
            {
                if existing_index < operation_count {
                    inserted_middle += 1;
                } else {
                    inserted_end += 1;
                }
            }
            rebuilt.push(unpack_operation(packed_action));
        }
    }
    assert_eq!(existing_index, operation_count);
    assert_eq!(rebuilt.len(), operation_count + goal_dist as usize);

    #[cfg(feature = "local")]
    {
        trace.existing_passes += operation_count;
        trace.insertions += goal_dist as usize;
        trace.middle_insertions += inserted_middle;
        trace.end_insertions += inserted_end;
        trace.goal_reached += 1;
    }

    rebuilt
}

#[cfg(feature = "local")]
#[derive(Default)]
struct Trace {
    existing_passes: usize,
    insertions: usize,
    middle_insertions: usize,
    end_insertions: usize,
    goal_reached: usize,
}

fn solve(input: &Input) -> Vec<Operation> {
    let mut source_of = [0; CELLS];
    for (cell, &card) in input.initial_board.iter().enumerate() {
        source_of[card] = cell;
    }

    let mut done_card = [false; CELLS];
    let mut operations = Vec::new();
    #[cfg(feature = "local")]
    let mut trace = Trace::default();

    for target in target_order(input) {
        // 基準フレーム0で、既完成カードが初期盤面のどこにいるかを作る。
        let mut initial_protected = [0; MASK_WORDS];
        for (cell, &card) in input.initial_board.iter().enumerate() {
            if done_card[card] {
                mask_set(&mut initial_protected, cell);
            }
        }

        operations = insert_route_for_card(
            input,
            &operations,
            source_of[target],
            target,
            initial_protected,
            #[cfg(feature = "local")]
            &mut trace,
        );
        assert!(operations.len() <= MAX_OPERATIONS);

        // 今回のカードは、初期位置から新しい操作列を通過した末尾で目標にいる。
        let mut cell = source_of[target];
        for &operation in &operations {
            cell = apply_to_cell(operation, cell);
        }
        assert_eq!(cell, target);
        done_card[target] = true;
    }

    let mut final_board = BoardState::new(input.initial_board);
    for &operation in &operations {
        final_board.apply(operation);
    }
    assert_eq!(final_board.board, std::array::from_fn(|cell| cell));

    #[cfg(feature = "local")]
    eprintln!(
        "[v902] goals={} existing_passes={} insertions={} middle_insertions={} end_insertions={}",
        trace.goal_reached,
        trace.existing_passes,
        trace.insertions,
        trace.middle_insertions,
        trace.end_insertions,
    );

    operations
}

fn write_output(operations: &[Operation]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for operation in operations {
        writeln!(
            writer,
            "{} {} {} {} {}",
            operation.direction.as_char(),
            operation.r,
            operation.c,
            operation.h,
            operation.w,
        )
        .unwrap();
    }
}

fn main() {
    let input = Input::read();
    let operations = solve(&input);
    write_output(&operations);
}
