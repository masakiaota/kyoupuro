// v995_from_explanation.rs
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_SHIFT: usize = N / 2;
const INF: u64 = u64::MAX;
const NEW_OPERATION_COST: u64 = 1_u64 << 32;
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

#[derive(Clone)]
struct Input {
    board: [u16; CELLS],
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
        let board = std::array::from_fn(|_| tokens.next().unwrap().parse().unwrap());
        let vertical_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        let horizontal_walls = std::array::from_fn(|_| {
            let row = tokens.next().unwrap().as_bytes();
            std::array::from_fn(|j| row[j] == b'1')
        });
        Self {
            board,
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
    #[inline]
    fn freedom(self) -> u32 {
        let slides = usize::from(self.slide_r - self.slide_l + 1);
        let left_choices = usize::from(self.must_l - self.outer_l + 1);
        let right_choices = usize::from(self.outer_r - self.must_r + 1);
        (slides * left_choices * right_choices - 1) as u32
    }

    fn representative(self) -> Operation {
        match self.direction {
            Direction::Vertical => Operation {
                direction: self.direction,
                r: self.slide_l,
                c: self.outer_l,
                h: 2 * self.k,
                w: self.outer_r - self.outer_l,
            },
            Direction::Horizontal => Operation {
                direction: self.direction,
                r: self.outer_l,
                c: self.slide_l,
                h: self.outer_r - self.outer_l,
                w: 2 * self.k,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Operation {
    direction: Direction,
    r: u8,
    c: u8,
    h: u8,
    w: u8,
}

#[derive(Clone, Copy, Default)]
struct ProtectionMask {
    words: [u64; 7],
}

impl ProtectionMask {
    #[inline]
    fn get(self, cell: usize) -> bool {
        ((self.words[cell >> 6] >> (cell & 63)) & 1) != 0
    }

    #[inline]
    fn set(&mut self, cell: usize, value: bool) {
        let bit = 1_u64 << (cell & 63);
        if value {
            self.words[cell >> 6] |= bit;
        } else {
            self.words[cell >> 6] &= !bit;
        }
    }

    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        let va = self.get(a);
        let vb = self.get(b);
        if va != vb {
            self.set(a, vb);
            self.set(b, va);
        }
    }
}

#[derive(Clone, Copy)]
struct Parent {
    previous: u32,
    action: u8,
}

impl Default for Parent {
    fn default() -> Self {
        Self {
            previous: u32::MAX,
            action: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct ExistingOption {
    next_cell: usize,
    family: Family,
    action: u8,
}

const EXISTING_MOVE_NEGATIVE: u8 = 0;
const EXISTING_MOVE_POSITIVE: u8 = 1;
const EXISTING_BEFORE_AXIS: u8 = 2;
const EXISTING_AFTER_AXIS: u8 = 3;
const EXISTING_EXCLUDE_LOW: u8 = 4;
const EXISTING_EXCLUDE_HIGH: u8 = 5;
const EXISTING_OUTSIDE_ORTHOGONAL: u8 = 6;
const NEW_ACTION_FLAG: u8 = 0x80;

#[inline]
fn encode_new_action(direction: Direction, positive: bool, k: usize) -> u8 {
    NEW_ACTION_FLAG
        | if direction == Direction::Horizontal {
            0x40
        } else {
            0
        }
        | if positive { 0x20 } else { 0 }
        | (k as u8 - 1)
}

#[inline]
fn decode_new_action(action: u8) -> (Direction, bool, usize) {
    let direction = if action & 0x40 != 0 {
        Direction::Horizontal
    } else {
        Direction::Vertical
    };
    (
        direction,
        action & 0x20 != 0,
        usize::from(action & 0x1f) + 1,
    )
}

fn processing_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut discovered = [false; CELLS];
    let mut queue = VecDeque::with_capacity(CELLS);
    let mut order = Vec::with_capacity(CELLS);
    discovered[root] = true;
    queue.push_back(root);

    // 同じ深さの順序は説明で指定されていないため、上・左・下・右に固定する。
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        let row = cell / N;
        let col = cell % N;
        let mut add = |next: usize, passable: bool| {
            if passable && !discovered[next] {
                discovered[next] = true;
                queue.push_back(next);
            }
        };
        if row > 0 {
            add(cell - N, !input.horizontal_walls[row - 1][col]);
        }
        if col > 0 {
            add(cell - 1, !input.vertical_walls[row][col - 1]);
        }
        if row + 1 < N {
            add(cell + N, !input.horizontal_walls[row][col]);
        }
        if col + 1 < N {
            add(cell + 1, !input.vertical_walls[row][col]);
        }
    }
    assert_eq!(order.len(), CELLS);
    order.reverse();
    order
}

#[inline]
fn intersect_slide(family: Family, low: i32, high: i32) -> Option<Family> {
    let low = low.max(i32::from(family.slide_l));
    let high = high.min(i32::from(family.slide_r));
    if low > high {
        return None;
    }
    let mut constrained = family;
    constrained.slide_l = low as u8;
    constrained.slide_r = high as u8;
    Some(constrained)
}

#[inline]
fn include_orthogonal(mut family: Family, y: usize) -> Option<Family> {
    if y < usize::from(family.outer_l) || y >= usize::from(family.outer_r) {
        return None;
    }
    family.must_l = family.must_l.min(y as u8);
    family.must_r = family.must_r.max(y as u8 + 1);
    Some(family)
}

fn constrain_existing(family: Family, cell: usize, action: u8) -> Option<ExistingOption> {
    let row = cell / N;
    let col = cell % N;
    let (x, y) = match family.direction {
        Direction::Vertical => (row, col),
        Direction::Horizontal => (col, row),
    };
    let k = usize::from(family.k);
    let mut next_x = x;
    let constrained = match action {
        EXISTING_MOVE_NEGATIVE => {
            let family = intersect_slide(family, x as i32 - 2 * k as i32 + 1, x as i32 - k as i32)?;
            next_x = x - k;
            include_orthogonal(family, y)?
        }
        EXISTING_MOVE_POSITIVE => {
            let family = intersect_slide(family, x as i32 - k as i32 + 1, x as i32)?;
            next_x = x + k;
            include_orthogonal(family, y)?
        }
        EXISTING_BEFORE_AXIS => intersect_slide(family, i32::MIN / 2, x as i32 - 2 * k as i32)?,
        EXISTING_AFTER_AXIS => intersect_slide(family, x as i32 + 1, i32::MAX / 2)?,
        EXISTING_EXCLUDE_LOW => {
            if y >= usize::from(family.must_l) {
                return None;
            }
            let mut family = family;
            family.outer_l = family.outer_l.max(y as u8 + 1);
            if family.outer_l > family.must_l {
                return None;
            }
            family
        }
        EXISTING_EXCLUDE_HIGH => {
            if y < usize::from(family.must_r) {
                return None;
            }
            let mut family = family;
            family.outer_r = family.outer_r.min(y as u8);
            if family.outer_r < family.must_r {
                return None;
            }
            family
        }
        EXISTING_OUTSIDE_ORTHOGONAL => {
            if y >= usize::from(family.outer_l) && y < usize::from(family.outer_r) {
                return None;
            }
            family
        }
        _ => unreachable!(),
    };
    let next_cell = match family.direction {
        Direction::Vertical => next_x * N + y,
        Direction::Horizontal => y * N + next_x,
    };
    Some(ExistingOption {
        next_cell,
        family: constrained,
        action,
    })
}

fn existing_options(family: Family, cell: usize) -> Vec<ExistingOption> {
    let (_, y) = match family.direction {
        Direction::Vertical => (cell / N, cell % N),
        Direction::Horizontal => (cell % N, cell / N),
    };
    let mut options = Vec::with_capacity(6);
    for action in [EXISTING_MOVE_NEGATIVE, EXISTING_MOVE_POSITIVE] {
        if let Some(option) = constrain_existing(family, cell, action) {
            options.push(option);
        }
    }
    if y < usize::from(family.outer_l) || y >= usize::from(family.outer_r) {
        options.push(constrain_existing(family, cell, EXISTING_OUTSIDE_ORTHOGONAL).unwrap());
    } else {
        for action in [
            EXISTING_BEFORE_AXIS,
            EXISTING_AFTER_AXIS,
            EXISTING_EXCLUDE_LOW,
            EXISTING_EXCLUDE_HIGH,
        ] {
            if let Some(option) = constrain_existing(family, cell, action) {
                options.push(option);
            }
        }
    }
    options.sort_unstable_by_key(|option| option.family.freedom());
    options
}

fn apply_operation_to_mask(mask: &mut ProtectionMask, operation: Operation) {
    let r = usize::from(operation.r);
    let c = usize::from(operation.c);
    let h = usize::from(operation.h);
    let w = usize::from(operation.w);
    match operation.direction {
        Direction::Vertical => {
            for di in 0..h / 2 {
                for dj in 0..w {
                    mask.swap((r + di) * N + c + dj, (r + h / 2 + di) * N + c + dj);
                }
            }
        }
        Direction::Horizontal => {
            for di in 0..h {
                for dj in 0..w / 2 {
                    mask.swap((r + di) * N + c + dj, (r + di) * N + c + w / 2 + dj);
                }
            }
        }
    }
}

fn protection_masks(initial: ProtectionMask, families: &[Family]) -> Vec<ProtectionMask> {
    let mut masks = Vec::with_capacity(families.len() + 1);
    let mut current = initial;
    masks.push(current);
    for &family in families {
        apply_operation_to_mask(&mut current, family.representative());
        masks.push(current);
    }
    masks
}

fn free_axis_interval(
    input: &Input,
    mask: ProtectionMask,
    cell: usize,
    direction: Direction,
) -> (usize, usize) {
    let row = cell / N;
    let col = cell % N;
    let x = if direction == Direction::Vertical {
        row
    } else {
        col
    };
    let mut low = x;
    while low > 0 {
        let (next_row, next_col, passable) = match direction {
            Direction::Vertical => (low - 1, col, !input.horizontal_walls[low - 1][col]),
            Direction::Horizontal => (row, low - 1, !input.vertical_walls[row][low - 1]),
        };
        if !passable || mask.get(next_row * N + next_col) {
            break;
        }
        low -= 1;
    }
    let mut high = x;
    while high + 1 < N {
        let (next_row, next_col, passable) = match direction {
            Direction::Vertical => (high + 1, col, !input.horizontal_walls[high][col]),
            Direction::Horizontal => (row, high + 1, !input.vertical_walls[row][high]),
        };
        if !passable || mask.get(next_row * N + next_col) {
            break;
        }
        high += 1;
    }
    (low, high)
}

fn movement_slide_bounds(
    input: &Input,
    mask: ProtectionMask,
    cell: usize,
    direction: Direction,
    positive: bool,
    k: usize,
) -> Option<(usize, usize)> {
    let x = if direction == Direction::Vertical {
        cell / N
    } else {
        cell % N
    };
    let (free_low, free_high) = free_axis_interval(input, mask, cell, direction);
    movement_slide_bounds_from_interval(x, free_low, free_high, positive, k)
}

#[inline]
fn movement_slide_bounds_from_interval(
    x: usize,
    free_low: usize,
    free_high: usize,
    positive: bool,
    k: usize,
) -> Option<(usize, usize)> {
    let movement_low = if positive {
        x as i32 - k as i32 + 1
    } else {
        x as i32 - 2 * k as i32 + 1
    };
    let movement_high = if positive {
        x as i32
    } else {
        x as i32 - k as i32
    };
    let low = 0_i32.max(free_low as i32).max(movement_low);
    let high = ((N - 2 * k) as i32)
        .min(free_high as i32 - 2 * k as i32 + 1)
        .min(movement_high);
    (low <= high).then_some((low as usize, high as usize))
}

fn new_transitions(input: &Input, mask: ProtectionMask, cell: usize) -> Vec<(usize, u8)> {
    let row = cell / N;
    let col = cell % N;
    let mut transitions = Vec::with_capacity(4 * MAX_SHIFT);
    for direction in [Direction::Vertical, Direction::Horizontal] {
        let x = if direction == Direction::Vertical {
            row
        } else {
            col
        };
        let (free_low, free_high) = free_axis_interval(input, mask, cell, direction);
        for k in 1..=MAX_SHIFT {
            for positive in [false, true] {
                if movement_slide_bounds_from_interval(x, free_low, free_high, positive, k)
                    .is_none()
                {
                    continue;
                }
                let next_cell = match direction {
                    Direction::Vertical => {
                        let next_row = if positive { row + k } else { row - k };
                        next_row * N + col
                    }
                    Direction::Horizontal => {
                        let next_col = if positive { col + k } else { col - k };
                        row * N + next_col
                    }
                };
                transitions.push((next_cell, encode_new_action(direction, positive, k)));
            }
        }
    }
    transitions
}

fn vertical_band_is_free(
    input: &Input,
    mask: ProtectionMask,
    slide: usize,
    k: usize,
    col: usize,
) -> bool {
    for row in slide..slide + 2 * k {
        if mask.get(row * N + col) {
            return false;
        }
        if row + 1 < slide + 2 * k && input.horizontal_walls[row][col] {
            return false;
        }
    }
    true
}

fn horizontal_band_is_free(
    input: &Input,
    mask: ProtectionMask,
    slide: usize,
    k: usize,
    row: usize,
) -> bool {
    for col in slide..slide + 2 * k {
        if mask.get(row * N + col) {
            return false;
        }
        if col + 1 < slide + 2 * k && input.vertical_walls[row][col] {
            return false;
        }
    }
    true
}

fn maximal_outer_interval(
    input: &Input,
    mask: ProtectionMask,
    direction: Direction,
    slide: usize,
    k: usize,
    core: usize,
) -> Option<(usize, usize)> {
    match direction {
        Direction::Vertical => {
            if !vertical_band_is_free(input, mask, slide, k, core) {
                return None;
            }
            let mut left = core;
            while left > 0 {
                let next = left - 1;
                let band_free = vertical_band_is_free(input, mask, slide, k, next);
                let boundary_free =
                    (slide..slide + 2 * k).all(|row| !input.vertical_walls[row][next]);
                if !band_free || !boundary_free {
                    break;
                }
                left = next;
            }
            let mut right = core + 1;
            while right < N {
                let band_free = vertical_band_is_free(input, mask, slide, k, right);
                let boundary_free =
                    (slide..slide + 2 * k).all(|row| !input.vertical_walls[row][right - 1]);
                if !band_free || !boundary_free {
                    break;
                }
                right += 1;
            }
            Some((left, right))
        }
        Direction::Horizontal => {
            if !horizontal_band_is_free(input, mask, slide, k, core) {
                return None;
            }
            let mut top = core;
            while top > 0 {
                let next = top - 1;
                let band_free = horizontal_band_is_free(input, mask, slide, k, next);
                let boundary_free =
                    (slide..slide + 2 * k).all(|col| !input.horizontal_walls[next][col]);
                if !band_free || !boundary_free {
                    break;
                }
                top = next;
            }
            let mut bottom = core + 1;
            while bottom < N {
                let band_free = horizontal_band_is_free(input, mask, slide, k, bottom);
                let boundary_free =
                    (slide..slide + 2 * k).all(|col| !input.horizontal_walls[bottom - 1][col]);
                if !band_free || !boundary_free {
                    break;
                }
                bottom += 1;
            }
            Some((top, bottom))
        }
    }
}

fn make_new_family(input: &Input, mask: ProtectionMask, cell: usize, action: u8) -> Family {
    let (direction, positive, k) = decode_new_action(action);
    let core = if direction == Direction::Vertical {
        cell % N
    } else {
        cell / N
    };
    let (slide_low, slide_high) =
        movement_slide_bounds(input, mask, cell, direction, positive, k).unwrap();
    let mut intervals = Vec::with_capacity(slide_high - slide_low + 1);
    for slide in slide_low..=slide_high {
        intervals.push(maximal_outer_interval(input, mask, direction, slide, k, core).unwrap());
    }

    let mut best: Option<(usize, usize, usize, usize, usize)> = None;
    for begin_index in 0..intervals.len() {
        let mut common_l = 0;
        let mut common_r = N;
        for end_index in begin_index..intervals.len() {
            common_l = common_l.max(intervals[end_index].0);
            common_r = common_r.min(intervals[end_index].1);
            if common_l > core || common_r <= core {
                break;
            }
            let slide_count = end_index - begin_index + 1;
            let width = common_r - common_l;
            let concrete_count = slide_count * (core - common_l + 1) * (common_r - core);
            let key = (concrete_count, slide_count, width);
            if best
                .map(|candidate| key > (candidate.0, candidate.1, candidate.2))
                .unwrap_or(true)
            {
                best = Some((concrete_count, slide_count, width, begin_index, end_index));
            }
        }
    }
    let (_, _, _, begin_index, end_index) = best.unwrap();
    let mut outer_l = 0;
    let mut outer_r = N;
    for &(left, right) in &intervals[begin_index..=end_index] {
        outer_l = outer_l.max(left);
        outer_r = outer_r.min(right);
    }
    Family {
        direction,
        k: k as u8,
        slide_l: (slide_low + begin_index) as u8,
        slide_r: (slide_low + end_index) as u8,
        must_l: core as u8,
        must_r: core as u8 + 1,
        outer_l: outer_l as u8,
        outer_r: outer_r as u8,
    }
}

fn insert_card(
    input: &Input,
    families: &[Family],
    masks: &[ProtectionMask],
    start_cell: usize,
    target_cell: usize,
    emergency: bool,
) -> Vec<Family> {
    let operation_count = families.len();
    let state_count = (operation_count + 1) * CELLS;
    let start_state = start_cell;
    let goal_state = operation_count * CELLS + target_cell;
    let mut distance = vec![INF; state_count];
    let mut parents = vec![Parent::default(); state_count];
    let mut queue = VecDeque::new();
    distance[start_state] = 0;
    queue.push_back((start_state as u32, 0_u64));
    let mut best_goal = INF;

    while let Some((state_u32, queued_cost)) = queue.pop_front() {
        let state = state_u32 as usize;
        if distance[state] != queued_cost {
            continue;
        }
        if best_goal != INF && (queued_cost >> 32) > (best_goal >> 32) {
            break;
        }
        if state == goal_state {
            best_goal = best_goal.min(queued_cost);
            continue;
        }
        let time = state / CELLS;
        let cell = state % CELLS;

        if time < operation_count {
            let old_freedom = families[time].freedom();
            for option in existing_options(families[time], cell) {
                let next_state = (time + 1) * CELLS + option.next_cell;
                let loss = u64::from(old_freedom - option.family.freedom());
                let next_cost = queued_cost + loss;
                if next_cost < distance[next_state] {
                    distance[next_state] = next_cost;
                    parents[next_state] = Parent {
                        previous: state_u32,
                        action: option.action,
                    };
                    queue.push_front((next_state as u32, next_cost));
                }
            }
        }

        if !emergency || time == operation_count {
            for (next_cell, action) in new_transitions(input, masks[time], cell) {
                let next_state = time * CELLS + next_cell;
                let next_cost = queued_cost + NEW_OPERATION_COST;
                if next_cost < distance[next_state] {
                    distance[next_state] = next_cost;
                    parents[next_state] = Parent {
                        previous: state_u32,
                        action,
                    };
                    queue.push_back((next_state as u32, next_cost));
                }
            }
        }
    }
    assert!(
        best_goal != INF,
        "the connected unfinished region must provide a path"
    );

    let mut edges = Vec::new();
    let mut state = goal_state;
    while state != start_state {
        let parent = parents[state];
        assert_ne!(parent.previous, u32::MAX);
        edges.push((parent.previous as usize, parent.action));
        state = parent.previous as usize;
    }
    edges.reverse();

    let mut updated = Vec::with_capacity(operation_count + (best_goal >> 32) as usize);
    let mut consumed = 0;
    for (source_state, action) in edges {
        let time = source_state / CELLS;
        let cell = source_state % CELLS;
        assert_eq!(time, consumed);
        if action & NEW_ACTION_FLAG != 0 {
            updated.push(make_new_family(input, masks[time], cell, action));
        } else {
            let option = constrain_existing(families[time], cell, action).unwrap();
            updated.push(option.family);
            consumed += 1;
        }
    }
    assert_eq!(consumed, operation_count);
    updated
}

fn solve(input: &Input, start_time: Instant) -> Vec<Family> {
    let order = processing_order(input);
    let mut initial_position = [0_usize; CELLS];
    for (cell, &card) in input.board.iter().enumerate() {
        initial_position[usize::from(card)] = cell;
    }

    let mut families = Vec::new();
    let mut protected_initial_cells = ProtectionMask::default();
    for target_cell in order {
        let card = target_cell;
        let start_cell = initial_position[card];
        let masks = protection_masks(protected_initial_cells, &families);
        let emergency = start_time.elapsed().as_secs_f64() >= PROGRAM_TIME_LIMIT_SEC;
        families = insert_card(input, &families, &masks, start_cell, target_cell, emergency);
        protected_initial_cells.set(start_cell, true);
    }
    families
}

#[cfg(feature = "local")]
fn apply_operation_to_board(board: &mut [u16; CELLS], operation: Operation) {
    let r = usize::from(operation.r);
    let c = usize::from(operation.c);
    let h = usize::from(operation.h);
    let w = usize::from(operation.w);
    match operation.direction {
        Direction::Vertical => {
            for di in 0..h / 2 {
                for dj in 0..w {
                    board.swap((r + di) * N + c + dj, (r + h / 2 + di) * N + c + dj);
                }
            }
        }
        Direction::Horizontal => {
            for di in 0..h {
                for dj in 0..w / 2 {
                    board.swap((r + di) * N + c + dj, (r + di) * N + c + w / 2 + dj);
                }
            }
        }
    }
}

#[cfg(feature = "local")]
fn input_hash(input: &Input) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &card in &input.board {
        hash ^= u64::from(card);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for row in &input.vertical_walls {
        for &wall in row {
            hash ^= wall as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    for row in &input.horizontal_walls {
        for &wall in row {
            hash ^= wall as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[cfg(feature = "local")]
fn report_local(input: &Input, operations: &[Operation], elapsed: f64) {
    let mut board = input.board;
    for &operation in operations {
        apply_operation_to_board(&mut board, operation);
    }
    let errors = board
        .iter()
        .enumerate()
        .filter(|&(cell, &card)| cell != usize::from(card))
        .count();
    // eval.py は stderr をケース別ファイルへ送るため、検証時だけ制御端末へ集計値を出す。
    if let Ok(mut terminal) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = writeln!(
            terminal,
            "[v995] hash={:016x} T={} E={} internal_elapsed_ms={:.3}",
            input_hash(input),
            operations.len(),
            errors,
            elapsed * 1000.0,
        );
    }
}

#[cfg(not(feature = "local"))]
fn report_local(_input: &Input, _operations: &[Operation], _elapsed: f64) {}

fn write_output(operations: &[Operation]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for operation in operations {
        let direction = match operation.direction {
            Direction::Vertical => 'V',
            Direction::Horizontal => 'H',
        };
        writeln!(
            writer,
            "{} {} {} {} {}",
            direction, operation.r, operation.c, operation.h, operation.w
        )
        .unwrap();
    }
}

fn main() {
    let start_time = Instant::now();
    let input = Input::read();
    let families = solve(&input, start_time);
    let operations: Vec<_> = families.into_iter().map(Family::representative).collect();
    report_local(&input, &operations, start_time.elapsed().as_secs_f64());
    write_output(&operations);
}
