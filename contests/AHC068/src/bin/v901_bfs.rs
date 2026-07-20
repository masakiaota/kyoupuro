// v901_bfs.rs
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_HALF_LENGTH: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;

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

#[derive(Clone, Copy)]
struct PreviousStep {
    cell: usize,
    operation: Operation,
}

struct State {
    board: [usize; CELLS],
    position: [usize; CELLS],
}

impl State {
    fn new(initial_board: [usize; CELLS]) -> Self {
        let mut position = [0; CELLS];
        for (cell, &card) in initial_board.iter().enumerate() {
            position[card] = cell;
        }
        Self {
            board: initial_board,
            position,
        }
    }

    fn apply(&mut self, operation: Operation) {
        let half = match operation.direction {
            Direction::Vertical => operation.h / 2,
            Direction::Horizontal => operation.w / 2,
        };

        match operation.direction {
            Direction::Vertical => {
                for dr in 0..half {
                    for dc in 0..operation.w {
                        let first = (operation.r + dr) * N + operation.c + dc;
                        let second = (operation.r + half + dr) * N + operation.c + dc;
                        self.board.swap(first, second);
                        self.position[self.board[first]] = first;
                        self.position[self.board[second]] = second;
                    }
                }
            }
            Direction::Horizontal => {
                for dr in 0..operation.h {
                    for dc in 0..half {
                        let first = (operation.r + dr) * N + operation.c + dc;
                        let second = (operation.r + dr) * N + operation.c + half + dc;
                        self.board.swap(first, second);
                        self.position[self.board[first]] = first;
                        self.position[self.board[second]] = second;
                    }
                }
            }
        }
    }
}

/// 中心を根とする BFS の発見順を反転する。
/// この順に確定すれば、未確定マスには常に根へ至る BFS 木の経路が残る。
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

/// `cell` を含み、確定済みマスと壁で止まる縦・横の連続区間の長さを得る。
/// 戻り値はそれぞれ、上・下・左・右へ進めるマス数である。
fn free_runs(input: &Input, fixed: &[bool; CELLS], cell: usize) -> (usize, usize, usize, usize) {
    let r = cell / N;
    let c = cell % N;

    let mut up = 0;
    while up < r && !fixed[cell - N * (up + 1)] && !input.horizontal_walls[r - up - 1][c] {
        up += 1;
    }

    let mut down = 0;
    while r + down + 1 < N && !fixed[cell + N * (down + 1)] && !input.horizontal_walls[r + down][c]
    {
        down += 1;
    }

    let mut left = 0;
    while left < c && !fixed[cell - (left + 1)] && !input.vertical_walls[r][c - left - 1] {
        left += 1;
    }

    let mut right = 0;
    while c + right + 1 < N && !fixed[cell + right + 1] && !input.vertical_walls[r][c + right] {
        right += 1;
    }

    (up, down, left, right)
}

fn enqueue(
    queue: &mut VecDeque<usize>,
    seen: &mut [bool; CELLS],
    previous: &mut [Option<PreviousStep>; CELLS],
    from: usize,
    to: usize,
    operation: Operation,
) {
    if !seen[to] {
        seen[to] = true;
        previous[to] = Some(PreviousStep {
            cell: from,
            operation,
        });
        queue.push_back(to);
    }
}

/// 確定済みカードを動かさずに、1 枚のカードを `source` から `target` へ運ぶ最短操作列を求める。
///
/// 縦操作で注目カードが動く距離は `h / 2`、横操作では `w / 2` だけである。
/// したがって任意の合法な長方形は、注目カードを含む 1 列または 1 行へ縮めても、
/// 確定済みカードに触れず同じ行き先へ動かせる。この BFS はその縮約後の全候補を
/// 辺として持つため、この制約下での最小操作数を返す。
fn shortest_route(
    input: &Input,
    fixed: &[bool; CELLS],
    source: usize,
    target: usize,
) -> Vec<Operation> {
    assert!(!fixed[source]);
    assert!(!fixed[target]);

    let mut queue = VecDeque::new();
    let mut seen = [false; CELLS];
    let mut previous = [None; CELLS];
    seen[source] = true;
    queue.push_back(source);

    while let Some(cell) = queue.pop_front() {
        if cell == target {
            break;
        }

        let r = cell / N;
        let c = cell % N;
        let (up, down, left, right) = free_runs(input, fixed, cell);

        for k in 1..=MAX_HALF_LENGTH {
            // 下へ k マス動かす。カードは長方形の上半分のどこに置いてもよいので、
            // 固定マスを避ける配置を 1 つ選ぶ。
            let lower_offset = (2 * k).saturating_sub(down + 1);
            let upper_offset = (k - 1).min(up);
            if lower_offset <= upper_offset {
                let offset = lower_offset;
                enqueue(
                    &mut queue,
                    &mut seen,
                    &mut previous,
                    cell,
                    cell + N * k,
                    Operation {
                        direction: Direction::Vertical,
                        r: r - offset,
                        c,
                        h: 2 * k,
                        w: 1,
                    },
                );
            }

            // 上へ k マス動かす。カードを下半分のどこに置くかを選ぶ。
            if up >= k {
                let lower_offset = k.saturating_sub(down + 1);
                let upper_offset = (k - 1).min(up - k);
                if lower_offset <= upper_offset {
                    let offset = lower_offset;
                    enqueue(
                        &mut queue,
                        &mut seen,
                        &mut previous,
                        cell,
                        cell - N * k,
                        Operation {
                            direction: Direction::Vertical,
                            r: r - k - offset,
                            c,
                            h: 2 * k,
                            w: 1,
                        },
                    );
                }
            }

            // 右へ k マス動かす。
            let lower_offset = (2 * k).saturating_sub(right + 1);
            let upper_offset = (k - 1).min(left);
            if lower_offset <= upper_offset {
                let offset = lower_offset;
                enqueue(
                    &mut queue,
                    &mut seen,
                    &mut previous,
                    cell,
                    cell + k,
                    Operation {
                        direction: Direction::Horizontal,
                        r,
                        c: c - offset,
                        h: 1,
                        w: 2 * k,
                    },
                );
            }

            // 左へ k マス動かす。
            if left >= k {
                let lower_offset = k.saturating_sub(right + 1);
                let upper_offset = (k - 1).min(left - k);
                if lower_offset <= upper_offset {
                    let offset = lower_offset;
                    enqueue(
                        &mut queue,
                        &mut seen,
                        &mut previous,
                        cell,
                        cell - k,
                        Operation {
                            direction: Direction::Horizontal,
                            r,
                            c: c - k - offset,
                            h: 1,
                            w: 2 * k,
                        },
                    );
                }
            }
        }
    }

    assert!(seen[target], "the unprocessed cells must stay connected");
    let mut route = Vec::new();
    let mut cell = target;
    while cell != source {
        let step = previous[cell].unwrap();
        route.push(step.operation);
        cell = step.cell;
    }
    route.reverse();
    route
}

fn solve(input: &Input) -> Vec<Operation> {
    let mut state = State::new(input.initial_board);
    let mut fixed = [false; CELLS];
    let mut operations = Vec::new();

    for target in target_order(input) {
        let source = state.position[target];
        let route = shortest_route(input, &fixed, source, target);
        for operation in route {
            state.apply(operation);
            operations.push(operation);
        }

        assert_eq!(state.board[target], target);
        fixed[target] = true;
    }

    assert_eq!(state.board, std::array::from_fn(|cell| cell));
    assert!(operations.len() <= MAX_OPERATIONS);
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
