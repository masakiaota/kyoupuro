// v993_from_explanation.rs
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
const MASK_WORDS: usize = CELLS.div_ceil(64);
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};
const NEW_OPERATION_COST: u64 = 1_u64 << 32;

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

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    const DUMMY: Self = Self {
        direction: Direction::Vertical,
        k: 0,
        slide_l: 0,
        slide_r: 0,
        must_l: 0,
        must_r: 0,
        outer_l: 0,
        outer_r: 0,
    };

    fn freedom(self) -> u64 {
        let slides = usize::from(self.slide_r - self.slide_l + 1);
        let left_choices = usize::from(self.must_l - self.outer_l + 1);
        let right_choices = usize::from(self.outer_r - self.must_r + 1);
        (slides * left_choices * right_choices - 1) as u64
    }

    fn representative(self) -> Operation {
        let slide = usize::from(self.slide_l);
        let outer_l = usize::from(self.outer_l);
        let outer_len = usize::from(self.outer_r - self.outer_l);
        let axis_len = 2 * usize::from(self.k);
        match self.direction {
            Direction::Vertical => Operation {
                direction: self.direction,
                r: slide,
                c: outer_l,
                h: axis_len,
                w: outer_len,
            },
            Direction::Horizontal => Operation {
                direction: self.direction,
                r: outer_l,
                c: slide,
                h: outer_len,
                w: axis_len,
            },
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

#[derive(Clone, Copy)]
struct Branch {
    cell: u16,
    family: Family,
}

impl Branch {
    const DUMMY: Self = Self {
        cell: 0,
        family: Family::DUMMY,
    };
}

#[derive(Clone, Copy)]
struct Parent {
    prev: u32,
    family: Family,
    kind: u8,
    new_direction: Direction,
    new_k: u8,
}

impl Parent {
    const NONE: Self = Self {
        prev: u32::MAX,
        family: Family::DUMMY,
        kind: 0,
        new_direction: Direction::Vertical,
        new_k: 0,
    };

    fn existing(prev: usize, family: Family) -> Self {
        Self {
            prev: prev as u32,
            family,
            kind: 1,
            new_direction: Direction::Vertical,
            new_k: 0,
        }
    }

    fn new(prev: usize, direction: Direction, k: usize) -> Self {
        Self {
            prev: prev as u32,
            family: Family::DUMMY,
            kind: 2,
            new_direction: direction,
            new_k: k as u8,
        }
    }
}

#[derive(Clone, Copy)]
struct QueueEntry {
    state: u32,
    cost: u64,
}

type Mask = [u64; MASK_WORDS];

#[inline]
fn mask_get(mask: &Mask, cell: usize) -> bool {
    ((mask[cell >> 6] >> (cell & 63)) & 1) != 0
}

#[inline]
fn mask_set(mask: &mut Mask, cell: usize, value: bool) {
    let bit = 1_u64 << (cell & 63);
    if value {
        mask[cell >> 6] |= bit;
    } else {
        mask[cell >> 6] &= !bit;
    }
}

#[inline]
fn mask_swap(mask: &mut Mask, a: usize, b: usize) {
    let va = mask_get(mask, a);
    let vb = mask_get(mask, b);
    if va != vb {
        mask_set(mask, a, vb);
        mask_set(mask, b, va);
    }
}

fn apply_operation_to_mask(mask: &mut Mask, op: Operation) {
    match op.direction {
        Direction::Vertical => {
            let k = op.h / 2;
            for di in 0..k {
                for dj in 0..op.w {
                    let a = (op.r + di) * N + op.c + dj;
                    let b = (op.r + k + di) * N + op.c + dj;
                    mask_swap(mask, a, b);
                }
            }
        }
        Direction::Horizontal => {
            let k = op.w / 2;
            for di in 0..op.h {
                for dj in 0..k {
                    let a = (op.r + di) * N + op.c + dj;
                    let b = (op.r + di) * N + op.c + k + dj;
                    mask_swap(mask, a, b);
                }
            }
        }
    }
}

fn protected_masks(initial_mask: Mask, operations: &[Family]) -> Vec<Mask> {
    let mut masks = Vec::with_capacity(operations.len() + 1);
    let mut mask = initial_mask;
    masks.push(mask);
    for &family in operations {
        apply_operation_to_mask(&mut mask, family.representative());
        masks.push(mask);
    }
    masks
}

fn bfs_processing_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut seen = [false; CELLS];
    let mut queue = VecDeque::new();
    let mut order = Vec::with_capacity(CELLS);
    seen[root] = true;
    queue.push_back(root);
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        let i = cell / N;
        let j = cell % N;
        let mut visit = |next: usize| {
            if !seen[next] {
                seen[next] = true;
                queue.push_back(next);
            }
        };
        if i > 0 && !input.horizontal_walls[i - 1][j] {
            visit(cell - N);
        }
        if i + 1 < N && !input.horizontal_walls[i][j] {
            visit(cell + N);
        }
        if j > 0 && !input.vertical_walls[i][j - 1] {
            visit(cell - 1);
        }
        if j + 1 < N && !input.vertical_walls[i][j] {
            visit(cell + 1);
        }
    }
    assert_eq!(order.len(), CELLS);
    order.reverse();
    order
}

/// 上、下、左、右へ、壁または盤面端まで何歩進めるか。
fn precompute_wall_distances(input: &Input) -> [[u8; 4]; CELLS] {
    let mut result = [[0_u8; 4]; CELLS];
    for i in 0..N {
        for j in 0..N {
            let cell = i * N + j;
            let mut r = i;
            while r > 0 && !input.horizontal_walls[r - 1][j] {
                result[cell][0] += 1;
                r -= 1;
            }
            r = i;
            while r + 1 < N && !input.horizontal_walls[r][j] {
                result[cell][1] += 1;
                r += 1;
            }
            let mut c = j;
            while c > 0 && !input.vertical_walls[i][c - 1] {
                result[cell][2] += 1;
                c -= 1;
            }
            c = j;
            while c + 1 < N && !input.vertical_walls[i][c] {
                result[cell][3] += 1;
                c += 1;
            }
        }
    }
    result
}

fn distance_before_protected(
    cell: usize,
    direction_index: usize,
    wall_distance: usize,
    mask: &Mask,
) -> usize {
    let mut available = 0;
    for step in 1..=wall_distance {
        let next = match direction_index {
            0 => cell - step * N,
            1 => cell + step * N,
            2 => cell - step,
            3 => cell + step,
            _ => unreachable!(),
        };
        if mask_get(mask, next) {
            break;
        }
        available = step;
    }
    available
}

fn intersect_slide(family: &mut Family, lo: i32, hi: i32) -> bool {
    let lo = lo.max(i32::from(family.slide_l));
    let hi = hi.min(i32::from(family.slide_r));
    if lo > hi {
        return false;
    }
    family.slide_l = lo as u8;
    family.slide_r = hi as u8;
    true
}

fn include_perpendicular(family: &mut Family, y: usize) -> bool {
    if y < usize::from(family.outer_l) || y >= usize::from(family.outer_r) {
        return false;
    }
    family.must_l = family.must_l.min(y as u8);
    family.must_r = family.must_r.max((y + 1) as u8);
    family.outer_l <= family.must_l
        && family.must_l < family.must_r
        && family.must_r <= family.outer_r
}

fn existing_branches(family: Family, cell: usize, output: &mut [Branch; 5]) -> usize {
    let (x, y) = match family.direction {
        Direction::Vertical => (cell / N, cell % N),
        Direction::Horizontal => (cell % N, cell / N),
    };
    let k = usize::from(family.k);
    let mut len = 0;

    if y < usize::from(family.outer_l) || y >= usize::from(family.outer_r) {
        output[0] = Branch {
            cell: cell as u16,
            family,
        };
        return 1;
    }

    let mut constrained = family;
    if intersect_slide(
        &mut constrained,
        x as i32 - 2 * k as i32 + 1,
        x as i32 - k as i32,
    ) && include_perpendicular(&mut constrained, y)
    {
        let moved = match family.direction {
            Direction::Vertical => cell - k * N,
            Direction::Horizontal => cell - k,
        };
        output[len] = Branch {
            cell: moved as u16,
            family: constrained,
        };
        len += 1;
    }

    constrained = family;
    if intersect_slide(&mut constrained, x as i32 - k as i32 + 1, x as i32)
        && include_perpendicular(&mut constrained, y)
    {
        let moved = match family.direction {
            Direction::Vertical => cell + k * N,
            Direction::Horizontal => cell + k,
        };
        output[len] = Branch {
            cell: moved as u16,
            family: constrained,
        };
        len += 1;
    }

    constrained = family;
    if intersect_slide(&mut constrained, 0, x as i32 - 2 * k as i32) {
        output[len] = Branch {
            cell: cell as u16,
            family: constrained,
        };
        len += 1;
    }

    constrained = family;
    if intersect_slide(&mut constrained, x as i32 + 1, (N - 1) as i32) {
        output[len] = Branch {
            cell: cell as u16,
            family: constrained,
        };
        len += 1;
    }

    constrained = family;
    let perpendicular_exclusion_exists = if y < usize::from(family.must_l) {
        constrained.outer_l = constrained.outer_l.max((y + 1) as u8);
        constrained.outer_l <= constrained.must_l
    } else if y >= usize::from(family.must_r) {
        constrained.outer_r = constrained.outer_r.min(y as u8);
        constrained.must_r <= constrained.outer_r
    } else {
        false
    };
    if perpendicular_exclusion_exists {
        output[len] = Branch {
            cell: cell as u16,
            family: constrained,
        };
        len += 1;
    }

    len
}

fn base_band_valid(
    input: &Input,
    mask: &Mask,
    direction: Direction,
    slide: usize,
    k: usize,
    core: usize,
) -> bool {
    match direction {
        Direction::Vertical => {
            for i in slide..slide + 2 * k {
                if mask_get(mask, i * N + core) {
                    return false;
                }
                if i + 1 < slide + 2 * k && input.horizontal_walls[i][core] {
                    return false;
                }
            }
        }
        Direction::Horizontal => {
            for j in slide..slide + 2 * k {
                if mask_get(mask, core * N + j) {
                    return false;
                }
                if j + 1 < slide + 2 * k && input.vertical_walls[core][j] {
                    return false;
                }
            }
        }
    }
    true
}

fn can_extend_band(
    input: &Input,
    mask: &Mask,
    direction: Direction,
    slide: usize,
    k: usize,
    perpendicular: usize,
    boundary: usize,
) -> bool {
    match direction {
        Direction::Vertical => {
            for i in slide..slide + 2 * k {
                if mask_get(mask, i * N + perpendicular)
                    || input.vertical_walls[i][boundary]
                    || (i + 1 < slide + 2 * k && input.horizontal_walls[i][perpendicular])
                {
                    return false;
                }
            }
        }
        Direction::Horizontal => {
            for j in slide..slide + 2 * k {
                if mask_get(mask, perpendicular * N + j)
                    || input.horizontal_walls[boundary][j]
                    || (j + 1 < slide + 2 * k && input.vertical_walls[perpendicular][j])
                {
                    return false;
                }
            }
        }
    }
    true
}

fn maximal_outer_interval(
    input: &Input,
    mask: &Mask,
    direction: Direction,
    slide: usize,
    k: usize,
    core: usize,
) -> Option<(usize, usize)> {
    if !base_band_valid(input, mask, direction, slide, k, core) {
        return None;
    }
    let mut left = core;
    while left > 0 && can_extend_band(input, mask, direction, slide, k, left - 1, left - 1) {
        left -= 1;
    }
    let mut right = core + 1;
    while right < N && can_extend_band(input, mask, direction, slide, k, right, right - 1) {
        right += 1;
    }
    Some((left, right))
}

fn maximize_new_family(
    input: &Input,
    mask: &Mask,
    direction: Direction,
    k: usize,
    from: usize,
    to: usize,
) -> Family {
    let (x, core, to_x) = match direction {
        Direction::Vertical => (from / N, from % N, to / N),
        Direction::Horizontal => (from % N, from / N, to % N),
    };
    let negative = to_x < x;
    let effect_l = if negative {
        x as i32 - 2 * k as i32 + 1
    } else {
        x as i32 - k as i32 + 1
    };
    let effect_r = if negative {
        x as i32 - k as i32
    } else {
        x as i32
    };
    let max_slide = N - 2 * k;
    let mut outers = [None; N];
    for slide in effect_l.max(0) as usize..=effect_r.min(max_slide as i32) as usize {
        outers[slide] = maximal_outer_interval(input, mask, direction, slide, k, core);
    }

    let mut best: Option<(usize, usize, usize, usize, usize)> = None;
    for slide_l in 0..=max_slide {
        let Some((mut common_l, mut common_r)) = outers[slide_l] else {
            continue;
        };
        for (slide_r, outer) in outers.iter().enumerate().take(max_slide + 1).skip(slide_l) {
            let Some((outer_l, outer_r)) = *outer else {
                break;
            };
            common_l = common_l.max(outer_l);
            common_r = common_r.min(outer_r);
            if !(common_l <= core && core < common_r) {
                break;
            }
            let slide_count = slide_r - slide_l + 1;
            let width = common_r - common_l;
            let product = slide_count * (core - common_l + 1) * (common_r - core);
            let candidate_key = (product, slide_count, width);
            let should_update = best
                .map(|(p, s, w, _, _)| candidate_key > (p, s, w))
                .unwrap_or(true);
            if should_update {
                best = Some((product, slide_count, width, slide_l, slide_r));
                // common_l/common_r are recovered below by re-intersecting this short range.
            }
        }
    }

    let (_, _, _, best_slide_l, best_slide_r) = best.expect("new transition must be realizable");
    let mut outer_l = 0;
    let mut outer_r = N;
    for outer in &outers[best_slide_l..=best_slide_r] {
        let (l, r) = outer.unwrap();
        outer_l = outer_l.max(l);
        outer_r = outer_r.min(r);
    }
    Family {
        direction,
        k: k as u8,
        slide_l: best_slide_l as u8,
        slide_r: best_slide_r as u8,
        must_l: core as u8,
        must_r: (core + 1) as u8,
        outer_l: outer_l as u8,
        outer_r: outer_r as u8,
    }
}

fn relax_new_transition(
    direction: Direction,
    k: usize,
    target: usize,
    state: usize,
    time: usize,
    cost: u64,
    distances: &mut [u64],
    parents: &mut [Parent],
    queue: &mut VecDeque<QueueEntry>,
) {
    let next_state = time * CELLS + target;
    let next_cost = cost + NEW_OPERATION_COST;
    if next_cost < distances[next_state] {
        distances[next_state] = next_cost;
        parents[next_state] = Parent::new(state, direction, k);
        queue.push_back(QueueEntry {
            state: next_state as u32,
            cost: next_cost,
        });
    }
}

fn generate_new_transitions(
    cell: usize,
    time: usize,
    state: usize,
    cost: u64,
    mask: &Mask,
    wall_distances: &[[u8; 4]; CELLS],
    distances: &mut [u64],
    parents: &mut [Parent],
    queue: &mut VecDeque<QueueEntry>,
) {
    for (direction, neg_index, pos_index) in
        [(Direction::Vertical, 0, 1), (Direction::Horizontal, 2, 3)]
    {
        let neg = distance_before_protected(
            cell,
            neg_index,
            usize::from(wall_distances[cell][neg_index]),
            mask,
        );
        let pos = distance_before_protected(
            cell,
            pos_index,
            usize::from(wall_distances[cell][pos_index]),
            mask,
        );
        let x = match direction {
            Direction::Vertical => cell / N,
            Direction::Horizontal => cell % N,
        };
        let max_k = (N / 2).min((neg + pos + 1) / 2);
        for k in 1..=max_k {
            let negative_l = 0_i32
                .max(x as i32 - neg as i32)
                .max(x as i32 - 2 * k as i32 + 1);
            let negative_r = (N - 2 * k)
                .min(x + pos + 1 - 2 * k)
                .min(x.saturating_sub(k)) as i32;
            if x >= k && negative_l <= negative_r {
                let target = match direction {
                    Direction::Vertical => cell - k * N,
                    Direction::Horizontal => cell - k,
                };
                relax_new_transition(
                    direction, k, target, state, time, cost, distances, parents, queue,
                );
            }

            let positive_l = 0_i32
                .max(x as i32 - neg as i32)
                .max(x as i32 - k as i32 + 1);
            let positive_r = (N - 2 * k).min(x + pos + 1 - 2 * k).min(x) as i32;
            if x + k < N && positive_l <= positive_r {
                let target = match direction {
                    Direction::Vertical => cell + k * N,
                    Direction::Horizontal => cell + k,
                };
                relax_new_transition(
                    direction, k, target, state, time, cost, distances, parents, queue,
                );
            }
        }
    }
}

fn add_card_to_operations(
    input: &Input,
    wall_distances: &[[u8; 4]; CELLS],
    operations: &[Family],
    protected_initial: Mask,
    start_cell: usize,
    target_cell: usize,
    emergency: bool,
) -> Vec<Family> {
    let old_len = operations.len();
    let masks = protected_masks(protected_initial, operations);
    let state_count = (old_len + 1) * CELLS;
    let start_state = start_cell;
    let goal_state = old_len * CELLS + target_cell;
    let mut distances = vec![u64::MAX; state_count];
    let mut parents = vec![Parent::NONE; state_count];
    let mut queue = VecDeque::new();
    distances[start_state] = 0;
    queue.push_back(QueueEntry {
        state: start_state as u32,
        cost: 0,
    });
    let mut best_added: Option<u64> = None;
    let mut branch_buffer = [Branch::DUMMY; 5];

    while let Some(entry) = queue.pop_front() {
        let state = entry.state as usize;
        if distances[state] != entry.cost {
            continue;
        }
        let added = entry.cost >> 32;
        if best_added.is_some_and(|best| added > best) {
            break;
        }
        if state == goal_state {
            best_added.get_or_insert(added);
            continue;
        }
        let time = state / CELLS;
        let cell = state % CELLS;

        if time < old_len {
            let original = operations[time];
            let branch_count = existing_branches(original, cell, &mut branch_buffer);
            branch_buffer[..branch_count].sort_by_key(|branch| branch.family.freedom());
            let original_freedom = original.freedom();
            for branch in &branch_buffer[..branch_count] {
                let loss = original_freedom - branch.family.freedom();
                let next_state = (time + 1) * CELLS + usize::from(branch.cell);
                let next_cost = entry.cost + loss;
                if next_cost < distances[next_state] {
                    distances[next_state] = next_cost;
                    parents[next_state] = Parent::existing(state, branch.family);
                    queue.push_front(QueueEntry {
                        state: next_state as u32,
                        cost: next_cost,
                    });
                }
            }
        }

        if best_added.is_none() && (!emergency || time == old_len) {
            generate_new_transitions(
                cell,
                time,
                state,
                entry.cost,
                &masks[time],
                wall_distances,
                &mut distances,
                &mut parents,
                &mut queue,
            );
        }
    }
    assert!(distances[goal_state] != u64::MAX);

    let mut reverse_states = Vec::new();
    let mut state = goal_state;
    while state != start_state {
        reverse_states.push(state);
        state = parents[state].prev as usize;
    }
    reverse_states.reverse();

    let mut result = Vec::with_capacity(old_len + (distances[goal_state] >> 32) as usize);
    let mut previous_state = start_state;
    let mut old_time = 0;
    for state in reverse_states {
        let parent = parents[state];
        assert_eq!(parent.prev as usize, previous_state);
        match parent.kind {
            1 => {
                result.push(parent.family);
                old_time += 1;
            }
            2 => {
                let from = previous_state % CELLS;
                let to = state % CELLS;
                result.push(maximize_new_family(
                    input,
                    &masks[old_time],
                    parent.new_direction,
                    usize::from(parent.new_k),
                    from,
                    to,
                ));
            }
            _ => unreachable!(),
        }
        previous_state = state;
    }
    assert_eq!(old_time, old_len);
    result
}

fn write_output(operations: &[Family]) {
    assert!(operations.len() <= 100_000);
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for &family in operations {
        let op = family.representative();
        writeln!(
            writer,
            "{} {} {} {} {}",
            op.direction.as_char(),
            op.r,
            op.c,
            op.h,
            op.w
        )
        .unwrap();
    }
}

fn main() {
    let start = Instant::now();
    let input = Input::read();
    let processing_order = bfs_processing_order(&input);
    let wall_distances = precompute_wall_distances(&input);
    let mut initial_position = [0_usize; CELLS];
    for (cell, &card) in input.initial_board.iter().enumerate() {
        initial_position[card] = cell;
    }

    let mut operations = Vec::new();
    let mut protected_initial = [0_u64; MASK_WORDS];
    for target_cell in processing_order {
        let emergency = start.elapsed().as_secs_f64() > PROGRAM_TIME_LIMIT_SEC;
        let start_cell = initial_position[target_cell];
        operations = add_card_to_operations(
            &input,
            &wall_distances,
            &operations,
            protected_initial,
            start_cell,
            target_cell,
            emergency,
        );
        mask_set(&mut protected_initial, start_cell, true);
    }
    write_output(&operations);
}
