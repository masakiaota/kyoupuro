// v994_from_explanation.rs
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
const LOCAL_TIME_RATIO: f64 = 0.80;
const EMERGENCY_JUDGE_SEC: f64 = 1.60;
const EMERGENCY_START_SEC: f64 = if cfg!(feature = "local") {
    EMERGENCY_JUDGE_SEC * LOCAL_TIME_RATIO
} else {
    EMERGENCY_JUDGE_SEC
};

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
    fn can_move(&self, from: usize, to: usize) -> bool {
        let (r0, c0) = (from / N, from % N);
        let (r1, c1) = (to / N, to % N);
        if r0 == r1 {
            if c0 + 1 == c1 {
                !self.vertical_walls[r0][c0]
            } else if c1 + 1 == c0 {
                !self.vertical_walls[r0][c1]
            } else {
                false
            }
        } else if c0 == c1 {
            if r0 + 1 == r1 {
                !self.horizontal_walls[r0][c0]
            } else if r1 + 1 == r0 {
                !self.horizontal_walls[r1][c0]
            } else {
                false
            }
        } else {
            false
        }
    }
}

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
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
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
        let slides = self.slide_r as u32 - self.slide_l as u32 + 1;
        let left = self.must_l as u32 - self.outer_l as u32 + 1;
        let right = self.outer_r as u32 - self.must_r as u32 + 1;
        slides * left * right - 1
    }

    fn representative(self) -> Operation {
        let k = self.k as usize;
        match self.direction {
            Direction::Vertical => Operation {
                direction: self.direction,
                r: self.slide_l as usize,
                c: self.outer_l as usize,
                h: 2 * k,
                w: (self.outer_r - self.outer_l) as usize,
            },
            Direction::Horizontal => Operation {
                direction: self.direction,
                r: self.outer_l as usize,
                c: self.slide_l as usize,
                h: (self.outer_r - self.outer_l) as usize,
                w: 2 * k,
            },
        }
    }

    fn fixed(self) -> Self {
        let op = self.representative();
        match op.direction {
            Direction::Vertical => Self {
                direction: op.direction,
                k: (op.h / 2) as u8,
                slide_l: op.r as u8,
                slide_r: op.r as u8,
                must_l: op.c as u8,
                must_r: (op.c + op.w) as u8,
                outer_l: op.c as u8,
                outer_r: (op.c + op.w) as u8,
            },
            Direction::Horizontal => Self {
                direction: op.direction,
                k: (op.w / 2) as u8,
                slide_l: op.c as u8,
                slide_r: op.c as u8,
                must_l: op.r as u8,
                must_r: (op.r + op.h) as u8,
                outer_l: op.r as u8,
                outer_r: (op.r + op.h) as u8,
            },
        }
    }
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
    fn set(&mut self, cell: usize) {
        self.words[cell >> 6] |= 1_u64 << (cell & 63);
    }

    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        let va = self.get(a);
        let vb = self.get(b);
        if va != vb {
            self.words[a >> 6] ^= 1_u64 << (a & 63);
            self.words[b >> 6] ^= 1_u64 << (b & 63);
        }
    }
}

fn apply_family_to_mask(mask: &mut Mask, family: Family) {
    let op = family.representative();
    match op.direction {
        Direction::Vertical => {
            let k = op.h / 2;
            for dr in 0..k {
                for dc in 0..op.w {
                    let a = (op.r + dr) * N + op.c + dc;
                    mask.swap(a, a + k * N);
                }
            }
        }
        Direction::Horizontal => {
            let k = op.w / 2;
            for dr in 0..op.h {
                for dc in 0..k {
                    let a = (op.r + dr) * N + op.c + dc;
                    mask.swap(a, a + k);
                }
            }
        }
    }
}

fn apply_family_to_position(family: Family, cell: usize) -> usize {
    let op = family.representative();
    let (r, c) = (cell / N, cell % N);
    if r < op.r || r >= op.r + op.h || c < op.c || c >= op.c + op.w {
        return cell;
    }
    match op.direction {
        Direction::Vertical => {
            let k = op.h / 2;
            if r < op.r + k {
                cell + k * N
            } else {
                cell - k * N
            }
        }
        Direction::Horizontal => {
            let k = op.w / 2;
            if c < op.c + k { cell + k } else { cell - k }
        }
    }
}

fn processing_order(input: &Input) -> Vec<usize> {
    let root = (N / 2) * N + N / 2;
    let mut seen = [false; CELLS];
    let mut order = Vec::with_capacity(CELLS);
    let mut queue = VecDeque::new();
    seen[root] = true;
    queue.push_back(root);
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        let (r, c) = (cell / N, cell % N);
        let candidates = [
            (r > 0).then_some(cell.wrapping_sub(N)),
            (r + 1 < N).then_some(cell + N),
            (c > 0).then_some(cell.wrapping_sub(1)),
            (c + 1 < N).then_some(cell + 1),
        ];
        for next in candidates.into_iter().flatten() {
            if !seen[next] && input.can_move(cell, next) {
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
    std::array::from_fn(|cell| {
        let (r, c) = (cell / N, cell % N);
        let mut result = [0_u8; 4];
        let mut rr = r;
        while rr > 0 && !input.horizontal_walls[rr - 1][c] {
            result[0] += 1;
            rr -= 1;
        }
        rr = r;
        while rr + 1 < N && !input.horizontal_walls[rr][c] {
            result[1] += 1;
            rr += 1;
        }
        let mut cc = c;
        while cc > 0 && !input.vertical_walls[r][cc - 1] {
            result[2] += 1;
            cc -= 1;
        }
        cc = c;
        while cc + 1 < N && !input.vertical_walls[r][cc] {
            result[3] += 1;
            cc += 1;
        }
        result
    })
}

fn protected_timeline(processed_initial: Mask, operations: &[Family]) -> Vec<Mask> {
    let mut timeline = Vec::with_capacity(operations.len() + 1);
    let mut mask = processed_initial;
    timeline.push(mask);
    for &family in operations {
        apply_family_to_mask(&mut mask, family);
        timeline.push(mask);
    }
    timeline
}

fn intersect_slide(mut family: Family, low: i32, high: i32) -> Option<Family> {
    let low = low.max(family.slide_l as i32);
    let high = high.min(family.slide_r as i32);
    if low > high {
        return None;
    }
    family.slide_l = low as u8;
    family.slide_r = high as u8;
    Some(family)
}

fn existing_transitions(family: Family, cell: usize, output: &mut Vec<(usize, Family, u32)>) {
    output.clear();
    let (x, y) = match family.direction {
        Direction::Vertical => (cell / N, cell % N),
        Direction::Horizontal => (cell % N, cell / N),
    };
    let x = x as i32;
    let y = y as u8;
    let k = family.k as i32;
    let old_freedom = family.freedom();

    if y >= family.outer_l && y < family.outer_r {
        if let Some(mut next) = intersect_slide(family, x - 2 * k + 1, x - k) {
            next.must_l = next.must_l.min(y);
            next.must_r = next.must_r.max(y + 1);
            let next_cell = match family.direction {
                Direction::Vertical => cell - family.k as usize * N,
                Direction::Horizontal => cell - family.k as usize,
            };
            output.push((next_cell, next, old_freedom - next.freedom()));
        }
        if let Some(mut next) = intersect_slide(family, x - k + 1, x) {
            next.must_l = next.must_l.min(y);
            next.must_r = next.must_r.max(y + 1);
            let next_cell = match family.direction {
                Direction::Vertical => cell + family.k as usize * N,
                Direction::Horizontal => cell + family.k as usize,
            };
            output.push((next_cell, next, old_freedom - next.freedom()));
        }

        if let Some(next) = intersect_slide(family, i32::MIN / 2, x - 2 * k) {
            output.push((cell, next, old_freedom - next.freedom()));
        }
        if let Some(next) = intersect_slide(family, x + 1, i32::MAX / 2) {
            output.push((cell, next, old_freedom - next.freedom()));
        }
        if y < family.must_l {
            let mut next = family;
            next.outer_l = next.outer_l.max(y + 1);
            output.push((cell, next, old_freedom - next.freedom()));
        } else if y >= family.must_r {
            let mut next = family;
            next.outer_r = next.outer_r.min(y);
            output.push((cell, next, old_freedom - next.freedom()));
        }
    } else {
        output.push((cell, family, 0));
    }

    output.sort_unstable_by_key(|&(_, constrained, _)| constrained.freedom());
}

#[derive(Clone, Copy, Debug)]
struct NewEdge {
    direction: Direction,
    k: u8,
    to: usize,
}

fn has_slide_start(x: usize, neg: usize, pos: usize, k: usize, positive: bool) -> bool {
    let move_low = if positive {
        x as i32 - k as i32 + 1
    } else {
        x as i32 - 2 * k as i32 + 1
    };
    let move_high = if positive {
        x as i32
    } else {
        x as i32 - k as i32
    };
    let low = 0_i32.max(x as i32 - neg as i32).max(move_low);
    let high = ((N - 2 * k) as i32)
        .min(x as i32 + pos as i32 - 2 * k as i32)
        .min(move_high);
    low <= high
}

fn free_axis_span(
    mask: Mask,
    cell: usize,
    wall_dist: &[[u8; 4]; CELLS],
    direction: Direction,
) -> (usize, usize) {
    let (r, c) = (cell / N, cell % N);
    let (negative_index, positive_index) = match direction {
        Direction::Vertical => (0, 1),
        Direction::Horizontal => (2, 3),
    };
    let mut neg = wall_dist[cell][negative_index] as usize;
    let mut pos = wall_dist[cell][positive_index] as usize;
    for distance in 1..=neg {
        let next = match direction {
            Direction::Vertical => (r - distance) * N + c,
            Direction::Horizontal => r * N + c - distance,
        };
        if mask.get(next) {
            neg = distance - 1;
            break;
        }
    }
    for distance in 1..=pos {
        let next = match direction {
            Direction::Vertical => (r + distance) * N + c,
            Direction::Horizontal => r * N + c + distance,
        };
        if mask.get(next) {
            pos = distance - 1;
            break;
        }
    }
    (neg, pos)
}

fn new_transitions(
    mask: Mask,
    cell: usize,
    wall_dist: &[[u8; 4]; CELLS],
    output: &mut Vec<NewEdge>,
) {
    output.clear();
    for direction in [Direction::Vertical, Direction::Horizontal] {
        let x = match direction {
            Direction::Vertical => cell / N,
            Direction::Horizontal => cell % N,
        };
        let (neg, pos) = free_axis_span(mask, cell, wall_dist, direction);
        for k in 1..=MAX_SHIFT {
            if x >= k && has_slide_start(x, neg, pos, k, false) {
                let to = match direction {
                    Direction::Vertical => cell - k * N,
                    Direction::Horizontal => cell - k,
                };
                output.push(NewEdge {
                    direction,
                    k: k as u8,
                    to,
                });
            }
            if x + k < N && has_slide_start(x, neg, pos, k, true) {
                let to = match direction {
                    Direction::Vertical => cell + k * N,
                    Direction::Horizontal => cell + k,
                };
                output.push(NewEdge {
                    direction,
                    k: k as u8,
                    to,
                });
            }
        }
    }
}

fn rectangle_valid(
    input: &Input,
    mask: Mask,
    direction: Direction,
    k: usize,
    slide: usize,
    orth_l: usize,
    orth_r: usize,
) -> bool {
    let (r, c, h, w) = match direction {
        Direction::Vertical => (slide, orth_l, 2 * k, orth_r - orth_l),
        Direction::Horizontal => (orth_l, slide, orth_r - orth_l, 2 * k),
    };
    if h == 0 || w == 0 || r + h > N || c + w > N {
        return false;
    }
    for rr in r..r + h {
        for cc in c..c + w {
            if mask.get(rr * N + cc) {
                return false;
            }
        }
    }
    for rr in r..r + h {
        for cc in c..c + w - 1 {
            if input.vertical_walls[rr][cc] {
                return false;
            }
        }
    }
    for rr in r..r + h - 1 {
        for cc in c..c + w {
            if input.horizontal_walls[rr][cc] {
                return false;
            }
        }
    }
    true
}

fn moved_axis(slide: usize, k: usize, x: usize) -> usize {
    if x >= slide && x < slide + k {
        x + k
    } else if x >= slide + k && x < slide + 2 * k {
        x - k
    } else {
        x
    }
}

fn maximize_new_family(
    input: &Input,
    mask: Mask,
    from: usize,
    to: usize,
    direction: Direction,
    k: usize,
) -> Family {
    let (x, core, target_x) = match direction {
        Direction::Vertical => (from / N, from % N, to / N),
        Direction::Horizontal => (from % N, from / N, to % N),
    };
    let mut slides = Vec::with_capacity(N);
    for slide in 0..=N - 2 * k {
        if moved_axis(slide, k, x) != target_x
            || !rectangle_valid(input, mask, direction, k, slide, core, core + 1)
        {
            continue;
        }
        let mut outer_l = core;
        while outer_l > 0
            && rectangle_valid(input, mask, direction, k, slide, outer_l - 1, core + 1)
        {
            outer_l -= 1;
        }
        let mut outer_r = core + 1;
        while outer_r < N && rectangle_valid(input, mask, direction, k, slide, outer_l, outer_r + 1)
        {
            outer_r += 1;
        }
        slides.push((slide, outer_l, outer_r));
    }
    assert!(!slides.is_empty());

    let mut best: Option<(usize, usize, usize, Family)> = None;
    for begin in 0..slides.len() {
        let mut common_l = slides[begin].1;
        let mut common_r = slides[begin].2;
        for end in begin..slides.len() {
            if end > begin && slides[end].0 != slides[end - 1].0 + 1 {
                break;
            }
            common_l = common_l.max(slides[end].1);
            common_r = common_r.min(slides[end].2);
            if !(common_l <= core && core < common_r) {
                break;
            }
            let slide_count = end - begin + 1;
            let choices = slide_count * (core - common_l + 1) * (common_r - core);
            let width = common_r - common_l;
            let family = Family {
                direction,
                k: k as u8,
                slide_l: slides[begin].0 as u8,
                slide_r: slides[end].0 as u8,
                must_l: core as u8,
                must_r: (core + 1) as u8,
                outer_l: common_l as u8,
                outer_r: common_r as u8,
            };
            let replace =
                best.as_ref()
                    .is_none_or(|&(best_choices, best_slides, best_width, _)| {
                        (choices, slide_count, width) > (best_choices, best_slides, best_width)
                    });
            if replace {
                best = Some((choices, slide_count, width, family));
            }
        }
    }
    best.unwrap().3
}

#[derive(Clone, Copy, Debug)]
enum Edge {
    None,
    Existing(Family),
    New { direction: Direction, k: u8 },
}

#[derive(Clone, Copy, Debug)]
struct Parent {
    previous: u32,
    edge: Edge,
}

impl Default for Parent {
    fn default() -> Self {
        Self {
            previous: u32::MAX,
            edge: Edge::None,
        }
    }
}

#[inline]
fn state_id(time: usize, cell: usize) -> usize {
    time * CELLS + cell
}

fn add_card_normal(
    input: &Input,
    wall_dist: &[[u8; 4]; CELLS],
    operations: &[Family],
    timeline: &[Mask],
    start: usize,
    target: usize,
) -> Vec<Family> {
    let length = operations.len();
    let state_count = (length + 1) * CELLS;
    let start_id = state_id(0, start);
    let goal_id = state_id(length, target);
    let mut distance = vec![INF; state_count];
    let mut parent = vec![Parent::default(); state_count];
    let mut deque = VecDeque::new();
    distance[start_id] = 0;
    deque.push_back((start_id as u32, 0_u64));
    let mut best_new_count: Option<u64> = None;
    let mut existing_buffer = Vec::with_capacity(5);
    let mut new_buffer = Vec::with_capacity(4 * MAX_SHIFT);

    while let Some((id_u32, queued_cost)) = deque.pop_front() {
        let id = id_u32 as usize;
        if distance[id] != queued_cost {
            continue;
        }
        let new_count = queued_cost >> 32;
        if best_new_count.is_some_and(|best| new_count > best) {
            break;
        }
        if id == goal_id && best_new_count.is_none() {
            // The first completed path fixes the best number of inserted operations.
            // From this point onward no new-operation transition is generated.
            best_new_count = Some(new_count);
        }

        let time = id / CELLS;
        let cell = id % CELLS;
        if time < length {
            existing_transitions(operations[time], cell, &mut existing_buffer);
            for &(next_cell, constrained, loss) in &existing_buffer {
                let next_id = state_id(time + 1, next_cell);
                let next_cost = queued_cost + loss as u64;
                if next_cost < distance[next_id] {
                    distance[next_id] = next_cost;
                    parent[next_id] = Parent {
                        previous: id as u32,
                        edge: Edge::Existing(constrained),
                    };
                    // Branches are sorted narrow-to-wide; push_front reverses that order.
                    deque.push_front((next_id as u32, next_cost));
                }
            }
        }

        if best_new_count.is_none() {
            new_transitions(timeline[time], cell, wall_dist, &mut new_buffer);
            for edge in &new_buffer {
                let next_id = state_id(time, edge.to);
                let next_cost = queued_cost + NEW_OPERATION_COST;
                if next_cost < distance[next_id] {
                    distance[next_id] = next_cost;
                    parent[next_id] = Parent {
                        previous: id as u32,
                        edge: Edge::New {
                            direction: edge.direction,
                            k: edge.k,
                        },
                    };
                    deque.push_back((next_id as u32, next_cost));
                }
            }
        }
    }
    assert_ne!(distance[goal_id], INF);

    let mut reversed = Vec::new();
    let mut current = goal_id;
    while current != start_id {
        let step = parent[current];
        assert_ne!(step.previous, u32::MAX);
        reversed.push((step.previous as usize, current, step.edge));
        current = step.previous as usize;
    }
    reversed.reverse();

    let mut updated = Vec::with_capacity(length + (distance[goal_id] >> 32) as usize);
    let mut consumed = 0;
    for (previous, current, edge) in reversed {
        match edge {
            Edge::Existing(family) => {
                updated.push(family);
                consumed += 1;
            }
            Edge::New { direction, k } => {
                let from = previous % CELLS;
                let to = current % CELLS;
                updated.push(maximize_new_family(
                    input,
                    timeline[consumed],
                    from,
                    to,
                    direction,
                    k as usize,
                ));
            }
            Edge::None => unreachable!(),
        }
    }
    assert_eq!(consumed, length);
    updated
}

fn emergency_path(input: &Input, blocked: Mask, start: usize, target: usize) -> Vec<usize> {
    let mut parent = [usize::MAX; CELLS];
    let mut queue = VecDeque::new();
    parent[start] = start;
    queue.push_back(start);
    while let Some(cell) = queue.pop_front() {
        if cell == target {
            break;
        }
        let (r, c) = (cell / N, cell % N);
        let candidates = [
            (r > 0).then_some(cell.wrapping_sub(N)),
            (r + 1 < N).then_some(cell + N),
            (c > 0).then_some(cell.wrapping_sub(1)),
            (c + 1 < N).then_some(cell + 1),
        ];
        for next in candidates.into_iter().flatten() {
            if parent[next] == usize::MAX && !blocked.get(next) && input.can_move(cell, next) {
                parent[next] = cell;
                queue.push_back(next);
            }
        }
    }
    assert_ne!(parent[target], usize::MAX);
    let mut reversed = Vec::new();
    let mut cell = target;
    while cell != start {
        reversed.push(cell);
        cell = parent[cell];
    }
    reversed.reverse();
    reversed
}

fn exact_adjacent_family(from: usize, to: usize) -> Family {
    let (r0, c0) = (from / N, from % N);
    let (r1, c1) = (to / N, to % N);
    if c0 == c1 {
        let slide = r0.min(r1) as u8;
        Family {
            direction: Direction::Vertical,
            k: 1,
            slide_l: slide,
            slide_r: slide,
            must_l: c0 as u8,
            must_r: (c0 + 1) as u8,
            outer_l: c0 as u8,
            outer_r: (c0 + 1) as u8,
        }
    } else {
        let slide = c0.min(c1) as u8;
        Family {
            direction: Direction::Horizontal,
            k: 1,
            slide_l: slide,
            slide_r: slide,
            must_l: r0 as u8,
            must_r: (r0 + 1) as u8,
            outer_l: r0 as u8,
            outer_r: (r0 + 1) as u8,
        }
    }
}

fn add_card_emergency(
    input: &Input,
    operations: &mut Vec<Family>,
    processed_targets: Mask,
    start: usize,
    target: usize,
) {
    // Emergency mode uses one fixed representative of every old family. Thus the
    // current card consumes the complete old sequence before any operation is appended.
    let mut current = start;
    for &family in operations.iter() {
        current = apply_family_to_position(family, current);
    }
    for next in emergency_path(input, processed_targets, current, target) {
        operations.push(exact_adjacent_family(current, next));
        current = next;
    }
    assert_eq!(current, target);
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
    // The contest clock includes parsing and output preparation.
    let start_time = Instant::now();
    let input = Input::read();
    let order = processing_order(&input);
    let wall_dist = wall_distances(&input);
    let mut initial_position = [0_usize; CELLS];
    for cell in 0..CELLS {
        initial_position[input.initial_board[cell]] = cell;
    }

    let mut operations: Vec<Family> = Vec::new();
    let mut processed_initial = Mask::default();
    let mut processed_targets = Mask::default();
    let mut emergency = false;

    for target in order {
        if !emergency && start_time.elapsed().as_secs_f64() > EMERGENCY_START_SEC {
            emergency = true;
            // Once the deadline is crossed, retain only the representatives that will
            // actually be output. The remaining work is linear simulation plus BFS paths.
            for family in &mut operations {
                *family = family.fixed();
            }
        }
        let card = target;
        let card_start = initial_position[card];
        if emergency {
            add_card_emergency(
                &input,
                &mut operations,
                processed_targets,
                card_start,
                target,
            );
        } else {
            let timeline = protected_timeline(processed_initial, &operations);
            operations = add_card_normal(
                &input,
                &wall_dist,
                &operations,
                &timeline,
                card_start,
                target,
            );
        }
        processed_initial.set(card_start);
        processed_targets.set(target);
    }

    write_output(&operations);
}
