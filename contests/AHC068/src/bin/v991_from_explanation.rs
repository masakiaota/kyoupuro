// v991_from_explanation.rs
use std::collections::VecDeque;
use std::io::{self, BufWriter, Read, Write};
use std::time::Instant;

const N: usize = 20;
const CELLS: usize = N * N;
const WORDS: usize = CELLS.div_ceil(64);
const MAX_SHIFT: usize = N / 2;
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};
const NEW_OPERATION_COST: u64 = 1_u64 << 32;
const INF: u64 = u64::MAX;
const NO_TRACE: usize = usize::MAX;

#[derive(Clone)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
struct Family {
    direction: Direction,
    k: u8,
    slide_l: u8,
    slide_r: u8,
    must_l: u8,
    must_r: u8,
    outer_l: u8,
    outer_r: u8,
}

impl Family {
    fn freedom(self) -> u32 {
        let slides = (self.slide_r - self.slide_l + 1) as u32;
        let left_choices = (self.must_l - self.outer_l + 1) as u32;
        let right_choices = (self.outer_r - self.must_r + 1) as u32;
        slides * left_choices * right_choices - 1
    }

    fn representative(self) -> Operation {
        match self.direction {
            Direction::Vertical => Operation {
                direction: self.direction,
                r: self.slide_l as usize,
                c: self.outer_l as usize,
                h: 2 * self.k as usize,
                w: (self.outer_r - self.outer_l) as usize,
            },
            Direction::Horizontal => Operation {
                direction: self.direction,
                r: self.outer_l as usize,
                c: self.slide_l as usize,
                h: (self.outer_r - self.outer_l) as usize,
                w: 2 * self.k as usize,
            },
        }
    }

    fn moved_position(self, position: usize) -> usize {
        let op = self.representative();
        let row = position / N;
        let col = position % N;
        if row < op.r || row >= op.r + op.h || col < op.c || col >= op.c + op.w {
            return position;
        }
        match self.direction {
            Direction::Vertical => {
                if row < op.r + self.k as usize {
                    position + self.k as usize * N
                } else {
                    position - self.k as usize * N
                }
            }
            Direction::Horizontal => {
                if col < op.c + self.k as usize {
                    position + self.k as usize
                } else {
                    position - self.k as usize
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

impl Operation {
    fn direction_char(self) -> char {
        match self.direction {
            Direction::Vertical => 'V',
            Direction::Horizontal => 'H',
        }
    }
}

type Mask = [u64; WORDS];

#[inline]
fn mask_contains(mask: &Mask, position: usize) -> bool {
    ((mask[position >> 6] >> (position & 63)) & 1) != 0
}

#[inline]
fn mask_insert(mask: &mut Mask, position: usize) {
    mask[position >> 6] |= 1_u64 << (position & 63);
}

#[inline]
fn mask_swap(mask: &mut Mask, a: usize, b: usize) {
    let a_set = mask_contains(mask, a);
    let b_set = mask_contains(mask, b);
    if a_set != b_set {
        mask[a >> 6] ^= 1_u64 << (a & 63);
        mask[b >> 6] ^= 1_u64 << (b & 63);
    }
}

fn apply_operation_to_mask(mask: &mut Mask, operation: Operation) {
    match operation.direction {
        Direction::Vertical => {
            let half = operation.h / 2;
            for dr in 0..half {
                for dc in 0..operation.w {
                    let a = (operation.r + dr) * N + operation.c + dc;
                    let b = a + half * N;
                    mask_swap(mask, a, b);
                }
            }
        }
        Direction::Horizontal => {
            let half = operation.w / 2;
            for dr in 0..operation.h {
                for dc in 0..half {
                    let a = (operation.r + dr) * N + operation.c + dc;
                    let b = a + half;
                    mask_swap(mask, a, b);
                }
            }
        }
    }
}

fn build_protection_masks(initial_mask: Mask, operations: &[Family]) -> Vec<Mask> {
    let mut masks = Vec::with_capacity(operations.len() + 1);
    masks.push(initial_mask);
    for &family in operations {
        let mut next = *masks.last().unwrap();
        apply_operation_to_mask(&mut next, family.representative());
        masks.push(next);
    }
    masks
}

fn connected(a: usize, b: usize, input: &Input) -> bool {
    let ar = a / N;
    let ac = a % N;
    let br = b / N;
    let bc = b % N;
    if ar == br {
        let left = ac.min(bc);
        !input.vertical_walls[ar][left]
    } else {
        let top = ar.min(br);
        !input.horizontal_walls[top][ac]
    }
}

fn target_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut seen = [false; CELLS];
    let mut queue = VecDeque::with_capacity(CELLS);
    let mut order = Vec::with_capacity(CELLS);
    seen[root] = true;
    queue.push_back(root);
    while let Some(position) = queue.pop_front() {
        order.push(position);
        let row = position / N;
        let col = position % N;
        let mut neighbors = [usize::MAX; 4];
        if row > 0 {
            neighbors[0] = position - N;
        }
        if row + 1 < N {
            neighbors[1] = position + N;
        }
        if col > 0 {
            neighbors[2] = position - 1;
        }
        if col + 1 < N {
            neighbors[3] = position + 1;
        }
        for next in neighbors {
            if next != usize::MAX && !seen[next] && connected(position, next, input) {
                seen[next] = true;
                queue.push_back(next);
            }
        }
    }
    assert_eq!(order.len(), CELLS);
    order.reverse();
    order
}

fn wall_distances(input: &Input) -> [[u8; 4]; CELLS] {
    let mut result = [[0_u8; 4]; CELLS];
    for position in 0..CELLS {
        let row = position / N;
        let col = position % N;
        let mut distance = 0;
        while row > distance && !input.horizontal_walls[row - distance - 1][col] {
            distance += 1;
        }
        result[position][0] = distance as u8;
        distance = 0;
        while row + distance + 1 < N && !input.horizontal_walls[row + distance][col] {
            distance += 1;
        }
        result[position][1] = distance as u8;
        distance = 0;
        while col > distance && !input.vertical_walls[row][col - distance - 1] {
            distance += 1;
        }
        result[position][2] = distance as u8;
        distance = 0;
        while col + distance + 1 < N && !input.vertical_walls[row][col + distance] {
            distance += 1;
        }
        result[position][3] = distance as u8;
    }
    result
}

fn free_distance(position: usize, direction: usize, wall_distance: u8, mask: &Mask) -> usize {
    let mut available = wall_distance as usize;
    for distance in 1..=available {
        let next = match direction {
            0 => position - distance * N,
            1 => position + distance * N,
            2 => position - distance,
            3 => position + distance,
            _ => unreachable!(),
        };
        if mask_contains(mask, next) {
            available = distance - 1;
            break;
        }
    }
    available
}

fn intersect_slide(mut family: Family, lower: i32, upper: i32) -> Option<Family> {
    let lower = lower.max(0) as usize;
    let upper = upper.min((N - 2 * family.k as usize) as i32);
    if upper < 0 {
        return None;
    }
    let new_l = (family.slide_l as usize).max(lower);
    let new_r = (family.slide_r as usize).min(upper as usize);
    if new_l > new_r {
        return None;
    }
    family.slide_l = new_l as u8;
    family.slide_r = new_r as u8;
    Some(family)
}

fn include_orthogonal(mut family: Family, coordinate: usize) -> Option<Family> {
    if coordinate < family.outer_l as usize || coordinate >= family.outer_r as usize {
        return None;
    }
    family.must_l = (family.must_l as usize).min(coordinate) as u8;
    family.must_r = (family.must_r as usize).max(coordinate + 1) as u8;
    Some(family)
}

fn existing_branches(family: Family, position: usize) -> ([Family; 5], usize) {
    let row = position / N;
    let col = position % N;
    let (axis, orthogonal) = match family.direction {
        Direction::Vertical => (row, col),
        Direction::Horizontal => (col, row),
    };
    let mut result = [family; 5];
    let mut len = 0;

    // 直交方向で外側区間の外なら、どの具体化でもカードは動かない。
    if orthogonal < family.outer_l as usize || orthogonal >= family.outer_r as usize {
        result[0] = family;
        return (result, 1);
    }

    let k = family.k as i32;
    let x = axis as i32;
    if let Some(constrained) = intersect_slide(family, x - 2 * k + 1, x - k)
        .and_then(|f| include_orthogonal(f, orthogonal))
    {
        result[len] = constrained;
        len += 1;
    }
    if let Some(constrained) =
        intersect_slide(family, x - k + 1, x).and_then(|f| include_orthogonal(f, orthogonal))
    {
        result[len] = constrained;
        len += 1;
    }
    if let Some(constrained) = intersect_slide(family, 0, x - 2 * k) {
        result[len] = constrained;
        len += 1;
    }
    if let Some(constrained) = intersect_slide(family, x + 1, (N - 2 * k as usize) as i32) {
        result[len] = constrained;
        len += 1;
    }
    if orthogonal < family.must_l as usize {
        let mut constrained = family;
        constrained.outer_l = constrained.outer_l.max((orthogonal + 1) as u8);
        result[len] = constrained;
        len += 1;
    } else if orthogonal >= family.must_r as usize {
        let mut constrained = family;
        constrained.outer_r = constrained.outer_r.min(orthogonal as u8);
        result[len] = constrained;
        len += 1;
    }
    (result, len)
}

fn movement_start_interval(
    axis: usize,
    negative_free: usize,
    positive_free: usize,
    k: usize,
    negative: bool,
) -> Option<(usize, usize)> {
    let x = axis as i32;
    let k = k as i32;
    let negative_free = negative_free as i32;
    let positive_free = positive_free as i32;
    let board_upper = (N as i32) - 2 * k;
    let (lower, upper) = if negative {
        (
            0.max(x - negative_free).max(x - 2 * k + 1),
            board_upper.min(x + positive_free - 2 * k + 1).min(x - k),
        )
    } else {
        (
            0.max(x - negative_free).max(x - k + 1),
            board_upper.min(x + positive_free - 2 * k + 1).min(x),
        )
    };
    if lower <= upper {
        Some((lower as usize, upper as usize))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug)]
enum TraceAction {
    Existing(Family),
    New {
        direction: Direction,
        k: u8,
        from: u16,
        to: u16,
    },
}

#[derive(Clone, Copy, Debug)]
struct TraceNode {
    parent: usize,
    action: TraceAction,
}

#[derive(Clone, Copy, Debug)]
struct QueueEntry {
    state: usize,
    cost: u64,
    trace: usize,
}

#[inline]
fn relax(
    state: usize,
    cost: u64,
    parent_trace: usize,
    action: TraceAction,
    dist: &mut [u64],
    traces: &mut Vec<TraceNode>,
    queue: &mut VecDeque<QueueEntry>,
    front: bool,
) {
    if cost >= dist[state] {
        return;
    }
    dist[state] = cost;
    let trace = traces.len();
    traces.push(TraceNode {
        parent: parent_trace,
        action,
    });
    let entry = QueueEntry { state, cost, trace };
    if front {
        queue.push_front(entry);
    } else {
        queue.push_back(entry);
    }
}

fn find_path(
    start: usize,
    target: usize,
    operations: &[Family],
    protection_masks: &[Mask],
    wall_distances: &[[u8; 4]; CELLS],
    emergency: bool,
) -> Vec<TraceAction> {
    let operation_count = operations.len();
    let state_count = (operation_count + 1) * CELLS;
    let start_state = start;
    let target_state = operation_count * CELLS + target;
    let mut dist = vec![INF; state_count];
    let mut queue = VecDeque::with_capacity(state_count);
    let mut traces = Vec::with_capacity(state_count);
    dist[start_state] = 0;
    queue.push_back(QueueEntry {
        state: start_state,
        cost: 0,
        trace: NO_TRACE,
    });

    let mut best_added_operations: Option<u32> = None;
    let mut best_end_cost = INF;
    let mut best_end_trace = NO_TRACE;

    while let Some(entry) = queue.pop_front() {
        if entry.cost != dist[entry.state] {
            continue;
        }
        let added_operations = (entry.cost >> 32) as u32;
        if best_added_operations.is_some_and(|best| added_operations > best) {
            break;
        }
        if entry.state == target_state {
            if best_added_operations.is_none() {
                best_added_operations = Some(added_operations);
            }
            if entry.cost < best_end_cost {
                best_end_cost = entry.cost;
                best_end_trace = entry.trace;
            }
            continue;
        }

        let time = entry.state / CELLS;
        let position = entry.state % CELLS;
        if time < operation_count {
            let family = operations[time];
            let old_freedom = family.freedom();
            let (mut branches, branch_count) = existing_branches(family, position);
            branches[..branch_count].sort_by_key(|candidate| candidate.freedom());
            for constrained in branches[..branch_count].iter().copied() {
                let next_position = constrained.moved_position(position);
                let next_state = (time + 1) * CELLS + next_position;
                let freedom_loss = old_freedom - constrained.freedom();
                relax(
                    next_state,
                    entry.cost + freedom_loss as u64,
                    entry.trace,
                    TraceAction::Existing(constrained),
                    &mut dist,
                    &mut traces,
                    &mut queue,
                    true,
                );
            }
        }

        if best_added_operations.is_some() || (emergency && time < operation_count) {
            continue;
        }
        let mask = &protection_masks[time];
        let up = free_distance(position, 0, wall_distances[position][0], mask);
        let down = free_distance(position, 1, wall_distances[position][1], mask);
        let left = free_distance(position, 2, wall_distances[position][2], mask);
        let right = free_distance(position, 3, wall_distances[position][3], mask);

        let row = position / N;
        let col = position % N;
        for (direction, axis, negative_free, positive_free) in [
            (Direction::Vertical, row, up, down),
            (Direction::Horizontal, col, left, right),
        ] {
            let max_k = MAX_SHIFT.min((negative_free + positive_free + 1) / 2);
            for k in 1..=max_k {
                if movement_start_interval(axis, negative_free, positive_free, k, true).is_some() {
                    let next_position = match direction {
                        Direction::Vertical => position - k * N,
                        Direction::Horizontal => position - k,
                    };
                    relax(
                        time * CELLS + next_position,
                        entry.cost + NEW_OPERATION_COST,
                        entry.trace,
                        TraceAction::New {
                            direction,
                            k: k as u8,
                            from: position as u16,
                            to: next_position as u16,
                        },
                        &mut dist,
                        &mut traces,
                        &mut queue,
                        false,
                    );
                }
                if movement_start_interval(axis, negative_free, positive_free, k, false).is_some() {
                    let next_position = match direction {
                        Direction::Vertical => position + k * N,
                        Direction::Horizontal => position + k,
                    };
                    relax(
                        time * CELLS + next_position,
                        entry.cost + NEW_OPERATION_COST,
                        entry.trace,
                        TraceAction::New {
                            direction,
                            k: k as u8,
                            from: position as u16,
                            to: next_position as u16,
                        },
                        &mut dist,
                        &mut traces,
                        &mut queue,
                        false,
                    );
                }
            }
        }
    }

    assert!(best_added_operations.is_some());
    let mut reversed = Vec::new();
    let mut trace = best_end_trace;
    while trace != NO_TRACE {
        let node = traces[trace];
        reversed.push(node.action);
        trace = node.parent;
    }
    reversed.reverse();
    reversed
}

fn maximal_orthogonal_interval(
    input: &Input,
    protection: &Mask,
    direction: Direction,
    k: usize,
    slide: usize,
    core: usize,
) -> Option<(usize, usize)> {
    match direction {
        Direction::Vertical => {
            let row_end = slide + 2 * k;
            for row in slide..row_end {
                if mask_contains(protection, row * N + core) {
                    return None;
                }
            }
            for row in slide..row_end - 1 {
                if input.horizontal_walls[row][core] {
                    return None;
                }
            }
            let mut left = core;
            while left > 0 {
                let new_col = left - 1;
                let mut legal = true;
                for row in slide..row_end {
                    if mask_contains(protection, row * N + new_col)
                        || input.vertical_walls[row][new_col]
                    {
                        legal = false;
                        break;
                    }
                }
                if legal {
                    for row in slide..row_end - 1 {
                        if input.horizontal_walls[row][new_col] {
                            legal = false;
                            break;
                        }
                    }
                }
                if !legal {
                    break;
                }
                left = new_col;
            }
            let mut right = core + 1;
            while right < N {
                let new_col = right;
                let mut legal = true;
                for row in slide..row_end {
                    if mask_contains(protection, row * N + new_col)
                        || input.vertical_walls[row][new_col - 1]
                    {
                        legal = false;
                        break;
                    }
                }
                if legal {
                    for row in slide..row_end - 1 {
                        if input.horizontal_walls[row][new_col] {
                            legal = false;
                            break;
                        }
                    }
                }
                if !legal {
                    break;
                }
                right += 1;
            }
            Some((left, right))
        }
        Direction::Horizontal => {
            let col_end = slide + 2 * k;
            for col in slide..col_end {
                if mask_contains(protection, core * N + col) {
                    return None;
                }
            }
            for col in slide..col_end - 1 {
                if input.vertical_walls[core][col] {
                    return None;
                }
            }
            let mut top = core;
            while top > 0 {
                let new_row = top - 1;
                let mut legal = true;
                for col in slide..col_end {
                    if mask_contains(protection, new_row * N + col)
                        || input.horizontal_walls[new_row][col]
                    {
                        legal = false;
                        break;
                    }
                }
                if legal {
                    for col in slide..col_end - 1 {
                        if input.vertical_walls[new_row][col] {
                            legal = false;
                            break;
                        }
                    }
                }
                if !legal {
                    break;
                }
                top = new_row;
            }
            let mut bottom = core + 1;
            while bottom < N {
                let new_row = bottom;
                let mut legal = true;
                for col in slide..col_end {
                    if mask_contains(protection, new_row * N + col)
                        || input.horizontal_walls[new_row - 1][col]
                    {
                        legal = false;
                        break;
                    }
                }
                if legal {
                    for col in slide..col_end - 1 {
                        if input.vertical_walls[new_row][col] {
                            legal = false;
                            break;
                        }
                    }
                }
                if !legal {
                    break;
                }
                bottom += 1;
            }
            Some((top, bottom))
        }
    }
}

fn make_new_family(
    input: &Input,
    protection: &Mask,
    direction: Direction,
    k: usize,
    from: usize,
    to: usize,
) -> Family {
    let from_row = from / N;
    let from_col = from % N;
    let to_row = to / N;
    let to_col = to % N;
    let (axis, core, negative) = match direction {
        Direction::Vertical => (from_row, from_col, to_row < from_row),
        Direction::Horizontal => (from_col, from_row, to_col < from_col),
    };
    let (start_l, start_r) = if negative {
        (
            (axis as i32 - 2 * k as i32 + 1).max(0) as usize,
            (axis - k).min(N - 2 * k),
        )
    } else {
        (
            (axis as i32 - k as i32 + 1).max(0) as usize,
            axis.min(N - 2 * k),
        )
    };

    let mut intervals = Vec::with_capacity(start_r - start_l + 1);
    for slide in start_l..=start_r {
        if let Some((outer_l, outer_r)) =
            maximal_orthogonal_interval(input, protection, direction, k, slide, core)
        {
            intervals.push((slide, outer_l, outer_r));
        }
    }
    assert!(!intervals.is_empty());

    let mut best: Option<(usize, usize, usize, Family)> = None;
    for first in 0..intervals.len() {
        let mut common_l = 0;
        let mut common_r = N;
        for last in first..intervals.len() {
            if last > first && intervals[last].0 != intervals[last - 1].0 + 1 {
                break;
            }
            common_l = common_l.max(intervals[last].1);
            common_r = common_r.min(intervals[last].2);
            if common_l > core || common_r <= core {
                break;
            }
            let slide_count = last - first + 1;
            let outer_width = common_r - common_l;
            let concrete_count = slide_count * (core - common_l + 1) * (common_r - core);
            let family = Family {
                direction,
                k: k as u8,
                slide_l: intervals[first].0 as u8,
                slide_r: intervals[last].0 as u8,
                must_l: core as u8,
                must_r: (core + 1) as u8,
                outer_l: common_l as u8,
                outer_r: common_r as u8,
            };
            let key = (concrete_count, slide_count, outer_width);
            if best.as_ref().is_none_or(|&(a, b, c, _)| key > (a, b, c)) {
                best = Some((key.0, key.1, key.2, family));
            }
        }
    }
    best.unwrap().3
}

fn rebuild_operations(
    input: &Input,
    old_operations: &[Family],
    protection_masks: &[Mask],
    path: &[TraceAction],
) -> Vec<Family> {
    let mut result = Vec::with_capacity(path.len());
    let mut time = 0;
    for &action in path {
        match action {
            TraceAction::Existing(family) => {
                result.push(family);
                time += 1;
            }
            TraceAction::New {
                direction,
                k,
                from,
                to,
            } => {
                result.push(make_new_family(
                    input,
                    &protection_masks[time],
                    direction,
                    k as usize,
                    from as usize,
                    to as usize,
                ));
            }
        }
    }
    assert_eq!(time, old_operations.len());
    result
}

#[cfg(feature = "local")]
fn verify_solution(input: &Input, operations: &[Operation]) {
    let mut board = input.initial_board;
    for &operation in operations {
        match operation.direction {
            Direction::Vertical => {
                let half = operation.h / 2;
                for dr in 0..half {
                    for dc in 0..operation.w {
                        let a = (operation.r + dr) * N + operation.c + dc;
                        let b = a + half * N;
                        board.swap(a, b);
                    }
                }
            }
            Direction::Horizontal => {
                let half = operation.w / 2;
                for dr in 0..operation.h {
                    for dc in 0..half {
                        let a = (operation.r + dr) * N + operation.c + dc;
                        let b = a + half;
                        board.swap(a, b);
                    }
                }
            }
        }
    }
    let errors = board
        .iter()
        .enumerate()
        .filter(|&(i, &card)| i != card)
        .count();
    eprintln!(
        "[summary] operations={} errors={}",
        operations.len(),
        errors
    );
    assert_eq!(errors, 0);
}

fn main() {
    let start_time = Instant::now();
    let input = Input::read();
    let order = target_order(&input);
    let wall_distances = wall_distances(&input);
    let mut initial_positions = [0_usize; CELLS];
    for (position, &card) in input.initial_board.iter().enumerate() {
        initial_positions[card] = position;
    }

    let mut processed_initial_mask = [0_u64; WORDS];
    let mut families: Vec<Family> = Vec::new();
    for target in order {
        // 判定は各カードの探索開始時に一度だけ行い、緊急時も同じ探索と復元を使う。
        let emergency = start_time.elapsed().as_secs_f64() > PROGRAM_TIME_LIMIT_SEC;
        let protection_masks = build_protection_masks(processed_initial_mask, &families);
        let path = find_path(
            initial_positions[target],
            target,
            &families,
            &protection_masks,
            &wall_distances,
            emergency,
        );
        families = rebuild_operations(&input, &families, &protection_masks, &path);
        mask_insert(&mut processed_initial_mask, initial_positions[target]);
    }

    let operations: Vec<_> = families
        .iter()
        .copied()
        .map(Family::representative)
        .collect();
    assert!(operations.len() <= 100_000);
    #[cfg(feature = "local")]
    verify_solution(&input, &operations);

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for operation in operations {
        writeln!(
            writer,
            "{} {} {} {} {}",
            operation.direction_char(),
            operation.r,
            operation.c,
            operation.h,
            operation.w,
        )
        .unwrap();
    }
}
