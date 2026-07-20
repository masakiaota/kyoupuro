// v990_from_explanation.rs
use smallvec::SmallVec;
use std::{
    collections::VecDeque,
    io::{self, BufWriter, Read, Write},
    time::Instant,
};

const N: usize = 20;
const CELLS: usize = N * N;
const MAX_SINGLE_AXIS_SHIFT: usize = N / 2;
const MAX_OPERATIONS: usize = 100_000;
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;
const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};
const TRACE_NONE: u32 = u32::MAX;
const INF: u64 = u64::MAX;
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

    #[inline]
    fn can_step(&self, cell: usize, direction: usize) -> bool {
        let r = cell / N;
        let c = cell % N;
        match direction {
            0 => r > 0 && !self.horizontal_walls[r - 1][c],
            1 => r + 1 < N && !self.horizontal_walls[r][c],
            2 => c > 0 && !self.vertical_walls[r][c - 1],
            3 => c + 1 < N && !self.vertical_walls[r][c],
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Direction {
    #[inline]
    fn as_char(self) -> char {
        match self {
            Self::Vertical => 'V',
            Self::Horizontal => 'H',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OperationFamily {
    direction: Direction,
    k: u8,
    slide_l: u8,
    slide_r: u8,
    must_l: u8,
    must_r: u8,
    outer_l: u8,
    outer_r: u8,
}

impl OperationFamily {
    #[inline]
    fn freedom(self) -> u32 {
        let slides = (self.slide_r - self.slide_l + 1) as u32;
        let lefts = (self.must_l - self.outer_l + 1) as u32;
        let rights = (self.outer_r - self.must_r + 1) as u32;
        slides * lefts * rights - 1
    }

    #[inline]
    fn axis_and_perp(self, cell: usize) -> (i32, u8) {
        let r = cell / N;
        let c = cell % N;
        match self.direction {
            Direction::Vertical => (r as i32, c as u8),
            Direction::Horizontal => (c as i32, r as u8),
        }
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
}

#[derive(Clone, Copy)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

#[derive(Clone, Copy, Default)]
struct Mask {
    words: [u64; 7],
}

impl Mask {
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

#[derive(Clone, Copy, Debug)]
enum ExistingDiff {
    Keep,
    MoveTo { from: u16, to: u16 },
    StayPerp { at: u16 },
    StaySlide { slide_l: u8, slide_r: u8 },
}

#[derive(Clone, Copy, Debug)]
struct NewMove {
    direction: Direction,
    k: u8,
    from: u16,
    to: u16,
}

#[derive(Clone, Copy, Debug)]
enum TraceStep {
    Existing(ExistingDiff),
    New(NewMove),
}

#[derive(Clone, Copy)]
struct TraceNode {
    step: TraceStep,
    parent: u32,
}

#[derive(Clone, Copy)]
struct QueueEntry {
    cost: u64,
    time: u32,
    trace_id: u32,
    pos: u16,
}

#[derive(Clone, Copy)]
struct ExistingBranch {
    destination: u16,
    diff: ExistingDiff,
    constrained: OperationFamily,
}

#[derive(Default)]
struct SearchWorkspace {
    dist: Vec<u64>,
    queue: VecDeque<QueueEntry>,
    trace: Vec<TraceNode>,
}

struct SearchResult {
    steps: Vec<TraceStep>,
    trace_nodes: usize,
}

fn processing_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut seen = [false; CELLS];
    let mut queue = VecDeque::new();
    let mut order = Vec::with_capacity(CELLS);
    seen[root] = true;
    queue.push_back(root);
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        let r = cell / N;
        let c = cell % N;
        let neighbors = [
            r.checked_sub(1).map(|nr| nr * N + c),
            (r + 1 < N).then_some((r + 1) * N + c),
            c.checked_sub(1).map(|nc| r * N + nc),
            (c + 1 < N).then_some(r * N + c + 1),
        ];
        for direction in 0..4 {
            if let Some(next) = neighbors[direction]
                && !seen[next]
                && input.can_step(cell, direction)
            {
                seen[next] = true;
                queue.push_back(next);
            }
        }
    }
    assert_eq!(order.len(), CELLS);
    order.reverse();
    order
}

fn precompute_open_distances(input: &Input) -> [[u8; 4]; CELLS] {
    std::array::from_fn(|cell| {
        std::array::from_fn(|direction| {
            let mut current = cell;
            let mut distance = 0;
            while input.can_step(current, direction) {
                current = match direction {
                    0 => current - N,
                    1 => current + N,
                    2 => current - 1,
                    3 => current + 1,
                    _ => unreachable!(),
                };
                distance += 1;
            }
            distance
        })
    })
}

#[inline]
fn apply_to_mask(mask: &mut Mask, family: OperationFamily) {
    let op = family.representative();
    match op.direction {
        Direction::Vertical => {
            let k = op.h / 2;
            for dr in 0..k {
                for dc in 0..op.w {
                    mask.swap((op.r + dr) * N + op.c + dc, (op.r + k + dr) * N + op.c + dc);
                }
            }
        }
        Direction::Horizontal => {
            let k = op.w / 2;
            for dr in 0..op.h {
                for dc in 0..k {
                    mask.swap((op.r + dr) * N + op.c + dc, (op.r + dr) * N + op.c + k + dc);
                }
            }
        }
    }
}

fn protection_trajectory(initial: Mask, operations: &[OperationFamily]) -> Vec<Mask> {
    let mut masks = Vec::with_capacity(operations.len() + 1);
    let mut current = initial;
    masks.push(current);
    for &family in operations {
        apply_to_mask(&mut current, family);
        masks.push(current);
    }
    masks
}

#[inline]
fn constrain_move(family: OperationFamily, from: usize, to: usize) -> Option<OperationFamily> {
    let (x, y) = family.axis_and_perp(from);
    let (to_x, to_y) = family.axis_and_perp(to);
    if y != to_y || y < family.outer_l || y >= family.outer_r {
        return None;
    }
    let k = family.k as i32;
    let (allowed_l, allowed_r) = if to_x == x - k {
        (x - 2 * k + 1, x - k)
    } else if to_x == x + k {
        (x - k + 1, x)
    } else {
        return None;
    };
    let slide_l = (family.slide_l as i32).max(allowed_l);
    let slide_r = (family.slide_r as i32).min(allowed_r);
    if slide_l > slide_r {
        return None;
    }
    let mut result = family;
    result.slide_l = slide_l as u8;
    result.slide_r = slide_r as u8;
    result.must_l = result.must_l.min(y);
    result.must_r = result.must_r.max(y + 1);
    (result.outer_l <= result.must_l && result.must_r <= result.outer_r).then_some(result)
}

#[inline]
fn constrain_stay_perp(family: OperationFamily, at: usize) -> Option<OperationFamily> {
    let (_, y) = family.axis_and_perp(at);
    let mut result = family;
    if y < family.must_l {
        result.outer_l = result.outer_l.max(y + 1);
    } else if y >= family.must_r {
        result.outer_r = result.outer_r.min(y);
    } else {
        return None;
    }
    (result.outer_l <= result.must_l && result.must_r <= result.outer_r).then_some(result)
}

fn existing_branches(family: OperationFamily, pos: usize) -> SmallVec<[ExistingBranch; 5]> {
    let mut branches = SmallVec::<[ExistingBranch; 5]>::new();
    let (x, y) = family.axis_and_perp(pos);
    if y < family.outer_l || y >= family.outer_r {
        branches.push(ExistingBranch {
            destination: pos as u16,
            diff: ExistingDiff::Keep,
            constrained: family,
        });
        return branches;
    }

    let k = family.k as i32;
    for to_x in [x - k, x + k] {
        if (0..N as i32).contains(&to_x) {
            let to = match family.direction {
                Direction::Vertical => to_x as usize * N + y as usize,
                Direction::Horizontal => y as usize * N + to_x as usize,
            };
            if let Some(constrained) = constrain_move(family, pos, to) {
                branches.push(ExistingBranch {
                    destination: to as u16,
                    diff: ExistingDiff::MoveTo {
                        from: pos as u16,
                        to: to as u16,
                    },
                    constrained,
                });
            }
        }
    }

    let before_r = (x - 2 * k).min(family.slide_r as i32);
    if family.slide_l as i32 <= before_r {
        let mut constrained = family;
        constrained.slide_r = before_r as u8;
        branches.push(ExistingBranch {
            destination: pos as u16,
            diff: ExistingDiff::StaySlide {
                slide_l: constrained.slide_l,
                slide_r: constrained.slide_r,
            },
            constrained,
        });
    }

    let after_l = (x + 1).max(family.slide_l as i32);
    if after_l <= family.slide_r as i32 {
        let mut constrained = family;
        constrained.slide_l = after_l as u8;
        branches.push(ExistingBranch {
            destination: pos as u16,
            diff: ExistingDiff::StaySlide {
                slide_l: constrained.slide_l,
                slide_r: constrained.slide_r,
            },
            constrained,
        });
    }

    if let Some(constrained) = constrain_stay_perp(family, pos) {
        branches.push(ExistingBranch {
            destination: pos as u16,
            diff: ExistingDiff::StayPerp { at: pos as u16 },
            constrained,
        });
    }
    debug_assert!(branches.len() <= 5);

    // 先頭へ順に積むので、小さい自由度から安定に並べると広い分岐が先に処理される。
    for i in 1..branches.len() {
        let key = branches[i];
        let key_freedom = key.constrained.freedom();
        let mut j = i;
        while j > 0 && branches[j - 1].constrained.freedom() > key_freedom {
            branches[j] = branches[j - 1];
            j -= 1;
        }
        branches[j] = key;
    }
    branches
}

#[inline]
fn limited_distances(pos: usize, mask: Mask, open: &[[u8; 4]; CELLS]) -> [usize; 4] {
    let mut result = [0; 4];
    for direction in 0..4 {
        let limit = open[pos][direction] as usize;
        for distance in 1..=limit {
            let cell = match direction {
                0 => pos - distance * N,
                1 => pos + distance * N,
                2 => pos - distance,
                3 => pos + distance,
                _ => unreachable!(),
            };
            if mask.get(cell) {
                break;
            }
            result[direction] = distance;
        }
    }
    result
}

#[inline]
fn relax(
    workspace: &mut SearchWorkspace,
    pos: usize,
    time: usize,
    cost: u64,
    parent: u32,
    step: TraceStep,
    front: bool,
) {
    let index = time * CELLS + pos;
    if cost >= workspace.dist[index] {
        return;
    }
    workspace.dist[index] = cost;
    assert!(workspace.trace.len() < u32::MAX as usize);
    let trace_id = workspace.trace.len() as u32;
    workspace.trace.push(TraceNode { step, parent });
    let entry = QueueEntry {
        cost,
        time: time as u32,
        trace_id,
        pos: pos as u16,
    };
    if front {
        workspace.queue.push_front(entry);
    } else {
        workspace.queue.push_back(entry);
    }
}

fn search_card(
    source: usize,
    target: usize,
    operations: &[OperationFamily],
    masks: &[Mask],
    open: &[[u8; 4]; CELLS],
    emergency: bool,
    workspace: &mut SearchWorkspace,
) -> SearchResult {
    let operation_count = operations.len();
    let state_count = (operation_count + 1) * CELLS;
    workspace.dist.clear();
    workspace.dist.resize(state_count, INF);
    workspace.queue.clear();
    workspace.trace.clear();
    workspace.dist[source] = 0;
    workspace.queue.push_back(QueueEntry {
        cost: 0,
        time: 0,
        trace_id: TRACE_NONE,
        pos: source as u16,
    });

    let mut best_added = None;
    let mut best_cost = INF;
    let mut best_trace = TRACE_NONE;

    while let Some(entry) = workspace.queue.pop_front() {
        let pos = entry.pos as usize;
        let time = entry.time as usize;
        let index = time * CELLS + pos;
        if workspace.dist[index] != entry.cost {
            continue;
        }
        let added = (entry.cost >> 32) as u32;
        if best_added.is_some_and(|best| added > best) {
            break;
        }
        if pos == target && time == operation_count {
            if best_added.is_none() {
                best_added = Some(added);
            }
            if entry.cost < best_cost {
                best_cost = entry.cost;
                best_trace = entry.trace_id;
            }
            continue;
        }

        if time < operation_count {
            let family = operations[time];
            let old_freedom = family.freedom();
            let branches = existing_branches(family, pos);
            for branch in branches {
                let freedom_loss = old_freedom - branch.constrained.freedom();
                relax(
                    workspace,
                    branch.destination as usize,
                    time + 1,
                    entry.cost + freedom_loss as u64,
                    entry.trace_id,
                    TraceStep::Existing(branch.diff),
                    true,
                );
            }
        }

        if best_added.is_some() || (emergency && time != operation_count) {
            continue;
        }
        let distances = limited_distances(pos, masks[time], open);
        for direction in [Direction::Vertical, Direction::Horizontal] {
            let (neg, positive, axis_stride) = match direction {
                Direction::Vertical => (distances[0], distances[1], N),
                Direction::Horizontal => (distances[2], distances[3], 1),
            };
            let max_k = MAX_SINGLE_AXIS_SHIFT.min((neg + positive + 1) / 2);
            for k in 1..=max_k {
                let sum = 2 * k - 1;
                if k.max(sum.saturating_sub(positive)) <= neg {
                    let to = pos - k * axis_stride;
                    relax(
                        workspace,
                        to,
                        time,
                        entry.cost + NEW_OPERATION_COST,
                        entry.trace_id,
                        TraceStep::New(NewMove {
                            direction,
                            k: k as u8,
                            from: pos as u16,
                            to: to as u16,
                        }),
                        false,
                    );
                }
                let needed_neg = sum.saturating_sub(positive);
                if needed_neg <= neg && needed_neg < k {
                    let to = pos + k * axis_stride;
                    relax(
                        workspace,
                        to,
                        time,
                        entry.cost + NEW_OPERATION_COST,
                        entry.trace_id,
                        TraceStep::New(NewMove {
                            direction,
                            k: k as u8,
                            from: pos as u16,
                            to: to as u16,
                        }),
                        false,
                    );
                }
            }
        }
    }

    assert!(
        best_cost != INF,
        "the connected unprocessed region must provide a path"
    );
    let mut steps = Vec::new();
    let mut trace_id = best_trace;
    while trace_id != TRACE_NONE {
        let node = workspace.trace[trace_id as usize];
        steps.push(node.step);
        trace_id = node.parent;
    }
    steps.reverse();
    SearchResult {
        steps,
        trace_nodes: workspace.trace.len(),
    }
}

fn apply_existing_diff(family: OperationFamily, diff: ExistingDiff) -> OperationFamily {
    match diff {
        ExistingDiff::Keep => family,
        ExistingDiff::MoveTo { from, to } => {
            constrain_move(family, from as usize, to as usize).unwrap()
        }
        ExistingDiff::StayPerp { at } => constrain_stay_perp(family, at as usize).unwrap(),
        ExistingDiff::StaySlide { slide_l, slide_r } => OperationFamily {
            slide_l,
            slide_r,
            ..family
        },
    }
}

fn base_strip_valid(
    input: &Input,
    mask: Mask,
    direction: Direction,
    slide: usize,
    k: usize,
    perp: usize,
) -> bool {
    match direction {
        Direction::Vertical => {
            for r in slide..slide + 2 * k {
                if mask.get(r * N + perp) {
                    return false;
                }
            }
            for r in slide..slide + 2 * k - 1 {
                if input.horizontal_walls[r][perp] {
                    return false;
                }
            }
        }
        Direction::Horizontal => {
            for c in slide..slide + 2 * k {
                if mask.get(perp * N + c) {
                    return false;
                }
            }
            for c in slide..slide + 2 * k - 1 {
                if input.vertical_walls[perp][c] {
                    return false;
                }
            }
        }
    }
    true
}

fn can_add_perp_band(
    input: &Input,
    mask: Mask,
    direction: Direction,
    slide: usize,
    k: usize,
    perp: usize,
    toward_negative: bool,
) -> bool {
    match direction {
        Direction::Vertical => {
            for r in slide..slide + 2 * k {
                if mask.get(r * N + perp) {
                    return false;
                }
            }
            for r in slide..slide + 2 * k - 1 {
                if input.horizontal_walls[r][perp] {
                    return false;
                }
            }
            let boundary = if toward_negative { perp } else { perp - 1 };
            for r in slide..slide + 2 * k {
                if input.vertical_walls[r][boundary] {
                    return false;
                }
            }
        }
        Direction::Horizontal => {
            for c in slide..slide + 2 * k {
                if mask.get(perp * N + c) {
                    return false;
                }
            }
            for c in slide..slide + 2 * k - 1 {
                if input.vertical_walls[perp][c] {
                    return false;
                }
            }
            let boundary = if toward_negative { perp } else { perp - 1 };
            for c in slide..slide + 2 * k {
                if input.horizontal_walls[boundary][c] {
                    return false;
                }
            }
        }
    }
    true
}

fn make_new_family(input: &Input, mask: Mask, movement: NewMove) -> OperationFamily {
    let from = movement.from as usize;
    let to = movement.to as usize;
    let k = movement.k as usize;
    let (x, core, to_x) = match movement.direction {
        Direction::Vertical => (from / N, from % N, to / N),
        Direction::Horizontal => (from % N, from / N, to % N),
    };
    let open = precompute_limited_axis(input, mask, from, movement.direction);
    let neg = open.0;
    let positive = open.1;
    let negative_move = to_x < x;
    let (lo, hi) = if negative_move {
        (
            0_i32
                .max(x as i32 - neg as i32)
                .max(x as i32 - 2 * k as i32 + 1),
            (N - 2 * k).min(x + positive + 1 - 2 * k).min(x - k) as i32,
        )
    } else {
        (
            0_i32
                .max(x as i32 - neg as i32)
                .max(x as i32 - k as i32 + 1),
            (N - 2 * k).min(x + positive + 1 - 2 * k).min(x) as i32,
        )
    };
    assert!(lo <= hi);

    let mut maxima = Vec::<(u8, u8)>::with_capacity((hi - lo + 1) as usize);
    for slide in lo as usize..=hi as usize {
        assert!(base_strip_valid(
            input,
            mask,
            movement.direction,
            slide,
            k,
            core
        ));
        let mut outer_l = core;
        while outer_l > 0
            && can_add_perp_band(input, mask, movement.direction, slide, k, outer_l - 1, true)
        {
            outer_l -= 1;
        }
        let mut outer_r = core + 1;
        while outer_r < N
            && can_add_perp_band(input, mask, movement.direction, slide, k, outer_r, false)
        {
            outer_r += 1;
        }
        maxima.push((outer_l as u8, outer_r as u8));
    }

    let mut best_metric = (0_u32, 0_u32, 0_u32);
    let mut best = None;
    for left_index in 0..maxima.len() {
        let mut common_l = 0_u8;
        let mut common_r = N as u8;
        for right_index in left_index..maxima.len() {
            common_l = common_l.max(maxima[right_index].0);
            common_r = common_r.min(maxima[right_index].1);
            if !(common_l <= core as u8 && (core as u8) < common_r) {
                continue;
            }
            let slide_count = (right_index - left_index + 1) as u32;
            let width = (common_r - common_l) as u32;
            let choices =
                slide_count * (core as u32 - common_l as u32 + 1) * (common_r as u32 - core as u32);
            let metric = (choices, slide_count, width);
            if metric > best_metric {
                best_metric = metric;
                best = Some(OperationFamily {
                    direction: movement.direction,
                    k: movement.k,
                    slide_l: lo as u8 + left_index as u8,
                    slide_r: lo as u8 + right_index as u8,
                    must_l: core as u8,
                    must_r: core as u8 + 1,
                    outer_l: common_l,
                    outer_r: common_r,
                });
            }
        }
    }
    best.unwrap()
}

fn precompute_limited_axis(
    input: &Input,
    mask: Mask,
    pos: usize,
    direction: Direction,
) -> (usize, usize) {
    let directions = match direction {
        Direction::Vertical => (0, 1),
        Direction::Horizontal => (2, 3),
    };
    let mut result = [0; 2];
    for (index, step_direction) in [directions.0, directions.1].into_iter().enumerate() {
        let mut current = pos;
        while input.can_step(current, step_direction) {
            current = match step_direction {
                0 => current - N,
                1 => current + N,
                2 => current - 1,
                3 => current + 1,
                _ => unreachable!(),
            };
            if mask.get(current) {
                break;
            }
            result[index] += 1;
        }
    }
    (result[0], result[1])
}

fn rebuild_operations(
    input: &Input,
    old_operations: &[OperationFamily],
    masks: &[Mask],
    steps: &[TraceStep],
) -> Vec<OperationFamily> {
    let new_count = steps
        .iter()
        .filter(|step| matches!(step, TraceStep::New(_)))
        .count();
    let mut result = Vec::with_capacity(old_operations.len() + new_count);
    let mut time = 0;
    for &step in steps {
        match step {
            TraceStep::Existing(diff) => {
                result.push(apply_existing_diff(old_operations[time], diff));
                time += 1;
            }
            TraceStep::New(movement) => {
                result.push(make_new_family(input, masks[time], movement));
            }
        }
    }
    assert_eq!(time, old_operations.len());
    result
}

fn write_output(families: &[OperationFamily]) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    for &family in families {
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
    let order = processing_order(&input);
    let open = precompute_open_distances(&input);
    let mut initial_position = [0; CELLS];
    for cell in 0..CELLS {
        initial_position[input.initial_board[cell]] = cell;
    }

    let mut processed_initial = Mask::default();
    let mut operations = Vec::<OperationFamily>::new();
    let mut workspace = SearchWorkspace::default();
    let mut total_trace_nodes = 0_usize;
    let mut maximum_trace_nodes = 0_usize;
    let mut emergency_cards = 0_usize;

    for target in order {
        let source = initial_position[target];
        let masks = protection_trajectory(processed_initial, &operations);
        // 時刻確認はカード探索の開始時だけ行い、探索中の経路を時間で変えない。
        let emergency = start.elapsed().as_secs_f64() > PROGRAM_TIME_LIMIT_SEC;
        emergency_cards += emergency as usize;
        let search = search_card(
            source,
            target,
            &operations,
            &masks,
            &open,
            emergency,
            &mut workspace,
        );
        total_trace_nodes += search.trace_nodes;
        maximum_trace_nodes = maximum_trace_nodes.max(search.trace_nodes);
        operations = rebuild_operations(&input, &operations, &masks, &search.steps);
        assert!(operations.len() <= MAX_OPERATIONS);
        processed_initial.set(source, true);
    }

    #[cfg(feature = "local")]
    {
        eprintln!("[summary.count] operations={}", operations.len());
        eprintln!("[summary.count] trace_nodes={total_trace_nodes}");
        eprintln!("[summary.count] max_trace_nodes={maximum_trace_nodes}");
        eprintln!("[summary.count] emergency_cards={emergency_cards}");
        eprintln!(
            "[summary.time_ms] total={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    #[cfg(not(feature = "local"))]
    let _ = (total_trace_nodes, maximum_trace_nodes, emergency_cards);
    write_output(&operations);
}
