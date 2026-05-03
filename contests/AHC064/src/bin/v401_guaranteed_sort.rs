// v401_guaranteed_sort.rs
pub const R: usize = 10;
pub const INIT_LEN: usize = 10;
pub const DEP_CAP: usize = 15;
pub const SIDING_CAP: usize = 20;
pub const MAX_TURNS: usize = 4000;
pub const CAR_COUNT: usize = R * INIT_LEN;

pub const MOVE_DEP_TO_SIDING: usize = 0;
pub const MOVE_SIDING_TO_DEP: usize = 1;

pub const AREA_DEP: u8 = 0;
pub const AREA_SIDING: u8 = 1;

pub type CarId = usize;
pub type LineIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarPos {
    pub r: LineIdx,
    pub c: usize,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub initial: [[CarId; INIT_LEN]; R],
    pub initial_pos: [CarPos; CAR_COUNT],
}

impl Input {
    pub fn read() -> Self {
        use std::io::Read;

        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        Self::from_str(&s)
    }

    pub fn from_str(s: &str) -> Self {
        let mut it = s.split_whitespace();
        let _r = it.next().unwrap().parse::<usize>().unwrap();

        let mut initial = [[0; INIT_LEN]; R];
        let mut initial_pos = [CarPos { r: 0, c: 0 }; CAR_COUNT];

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = it.next().unwrap().parse::<usize>().unwrap();
                initial[r][c] = car;
                initial_pos[car] = CarPos { r, c };
            }
        }

        Self {
            initial,
            initial_pos,
        }
    }

    #[inline(always)]
    pub fn target_id(r: usize, c: usize) -> CarId {
        r * INIT_LEN + c
    }

    #[inline(always)]
    pub fn target_line(car: CarId) -> usize {
        car / INIT_LEN
    }

    #[inline(always)]
    pub fn target_pos(car: CarId) -> usize {
        car % INIT_LEN
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub kind: usize,
    pub i: LineIdx,
    pub j: LineIdx,
    pub k: usize,
}

impl Move {
    #[inline(always)]
    pub fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    #[inline(always)]
    pub fn siding_to_dep(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_SIDING_TO_DEP,
            i,
            j,
            k,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    pub turns: Vec<Vec<Move>>,
}

impl Output {
    #[inline(always)]
    pub fn new() -> Self {
        Self { turns: Vec::new() }
    }

    #[inline(always)]
    pub fn push_turn(&mut self, moves: Vec<Move>) {
        debug_assert!(!moves.is_empty());
        self.turns.push(moves);
    }

    pub fn to_output_string(&self) -> String {
        use std::fmt::Write;

        let move_count: usize = self.turns.iter().map(Vec::len).sum();
        let mut s = String::with_capacity(16 + self.turns.len() * 4 + move_count * 16);

        writeln!(&mut s, "{}", self.turns.len()).unwrap();
        for moves in &self.turns {
            writeln!(&mut s, "{}", moves.len()).unwrap();
            for mv in moves {
                writeln!(&mut s, "{} {} {} {}", mv.kind, mv.i, mv.j, mv.k).unwrap();
            }
        }

        s
    }

    pub fn print(&self) {
        use std::io::Write;

        let s = self.to_output_string();
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        out.write_all(s.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub dep: [[u8; DEP_CAP]; R],
    pub dep_len: [u8; R],

    pub sid: [[u8; SIDING_CAP]; R],
    pub sid_head: [u8; R],
    pub sid_len: [u8; R],

    pub car_area: [u8; CAR_COUNT],
    pub car_line: [u8; CAR_COUNT],
    pub car_slot: [u8; CAR_COUNT],

    pub partial_score: i32,
}

impl State {
    pub fn new(input: &Input) -> Self {
        let mut dep = [[0; DEP_CAP]; R];
        let dep_len = [INIT_LEN as u8; R];
        let sid = [[0; SIDING_CAP]; R];
        let sid_head = [0; R];
        let sid_len = [0; R];
        let mut car_area = [AREA_DEP; CAR_COUNT];
        let mut car_line = [0; CAR_COUNT];
        let mut car_slot = [0; CAR_COUNT];
        let mut partial_score = 0;

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = input.initial[r][c];
                dep[r][c] = car as u8;
                car_area[car] = AREA_DEP;
                car_line[car] = r as u8;
                car_slot[car] = c as u8;
                partial_score += Self::dep_score_piece(car, r, c);
            }
        }

        Self {
            dep,
            dep_len,
            sid,
            sid_head,
            sid_len,
            car_area,
            car_line,
            car_slot,
            partial_score,
        }
    }

    #[inline(always)]
    pub fn dep_score_piece(car: usize, r: usize, c: usize) -> i32 {
        if Input::target_line(car) != r {
            0
        } else if Input::target_pos(car) == c {
            10
        } else {
            1
        }
    }

    #[inline(always)]
    pub fn sid_slot(head: usize, offset: usize) -> usize {
        let slot = head + offset;
        if slot >= SIDING_CAP {
            slot - SIDING_CAP
        } else {
            slot
        }
    }

    #[inline(always)]
    pub fn sid_head_after_pop(head: usize, k: usize) -> usize {
        Self::sid_slot(head, k)
    }

    #[inline(always)]
    pub fn sid_head_after_push(head: usize, k: usize) -> usize {
        if head >= k {
            head - k
        } else {
            head + SIDING_CAP - k
        }
    }

    #[inline(always)]
    pub fn dep_car(&self, i: usize, pos: usize) -> usize {
        self.dep[i][pos] as usize
    }

    #[inline(always)]
    pub fn sid_car(&self, j: usize, pos: usize) -> usize {
        let slot = Self::sid_slot(self.sid_head[j] as usize, pos);
        self.sid[j][slot] as usize
    }

    #[inline(always)]
    pub fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            self.move_dep_to_siding(mv.i, mv.j, mv.k);
        } else {
            self.move_siding_to_dep(mv.i, mv.j, mv.k);
        }
    }

    #[inline(always)]
    pub fn apply_turn(&mut self, moves: &[Move]) {
        for &mv in moves {
            self.apply_move(mv);
        }
    }

    #[inline(always)]
    pub fn move_dep_to_siding(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len[i] as usize;
        let new_dep_len = old_dep_len - k;
        let new_sid_head = Self::sid_head_after_push(self.sid_head[j] as usize, k);

        for offset in 0..k {
            let dep_pos = new_dep_len + offset;
            let car = self.dep[i][dep_pos];
            let car_idx = car as usize;
            let sid_slot = Self::sid_slot(new_sid_head, offset);

            self.partial_score -= Self::dep_score_piece(car_idx, i, dep_pos);
            self.sid[j][sid_slot] = car;
            self.car_area[car_idx] = AREA_SIDING;
            self.car_line[car_idx] = j as u8;
            self.car_slot[car_idx] = sid_slot as u8;
        }

        self.dep_len[i] = new_dep_len as u8;
        self.sid_head[j] = new_sid_head as u8;
        self.sid_len[j] += k as u8;
    }

    #[inline(always)]
    pub fn move_siding_to_dep(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len[i] as usize;
        let old_sid_head = self.sid_head[j] as usize;

        for offset in 0..k {
            let sid_slot = Self::sid_slot(old_sid_head, offset);
            let car = self.sid[j][sid_slot];
            let car_idx = car as usize;
            let dep_pos = old_dep_len + offset;

            self.dep[i][dep_pos] = car;
            self.partial_score += Self::dep_score_piece(car_idx, i, dep_pos);
            self.car_area[car_idx] = AREA_DEP;
            self.car_line[car_idx] = i as u8;
            self.car_slot[car_idx] = dep_pos as u8;
        }

        self.dep_len[i] = (old_dep_len + k) as u8;
        self.sid_head[j] = Self::sid_head_after_pop(old_sid_head, k) as u8;
        self.sid_len[j] -= k as u8;
    }

    pub fn is_complete(&self) -> bool {
        for r in 0..R {
            if self.dep_len[r] as usize != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] as usize != Input::target_id(r, c) {
                    return false;
                }
            }
        }
        true
    }
}

#[inline(always)]
fn bucket(car: usize, scratch_sid: usize, extra_sid: usize) -> usize {
    let target = Input::target_line(car);
    if target == scratch_sid {
        extra_sid
    } else {
        target
    }
}

fn is_empty_dep(state: &State) -> bool {
    state.dep_len.iter().all(|&len| len == 0)
}

fn tail_bucket_block(
    state: &State,
    i: usize,
    scratch_sid: usize,
    extra_sid: usize,
) -> Option<Move> {
    let len = state.dep_len[i] as usize;
    if len == 0 {
        return None;
    }

    let j = bucket(state.dep_car(i, len - 1), scratch_sid, extra_sid);
    let mut k = 1;
    while k < len && bucket(state.dep_car(i, len - 1 - k), scratch_sid, extra_sid) == j {
        k += 1;
    }

    if state.sid_len[j] as usize + k <= SIDING_CAP {
        Some(Move::dep_to_siding(i, j, k))
    } else {
        None
    }
}

fn valid_together(moves: &[Move]) -> bool {
    let mut used_dep = [false; R];
    let mut used_sid = [false; R];
    for mv in moves {
        if used_dep[mv.i] || used_sid[mv.j] {
            return false;
        }
        used_dep[mv.i] = true;
        used_sid[mv.j] = true;
    }
    for a in 0..moves.len() {
        for b in a + 1..moves.len() {
            let ma = moves[a];
            let mb = moves[b];
            if (ma.i < mb.i && ma.j >= mb.j) || (ma.i > mb.i && ma.j <= mb.j) {
                return false;
            }
        }
    }
    true
}

fn best_initial_turn(candidates: &[Move]) -> Vec<Move> {
    let n = candidates.len();
    let mut best_mask = 0_usize;
    let mut best_count = 0_usize;
    let mut best_cars = 0_usize;

    for mask in 1_usize..(1_usize << n) {
        let mut moves = Vec::new();
        let mut cars = 0;
        for (idx, &mv) in candidates.iter().enumerate() {
            if ((mask >> idx) & 1) != 0 {
                cars += mv.k;
                moves.push(mv);
            }
        }
        if !valid_together(&moves) {
            continue;
        }

        let count = moves.len();
        if count > best_count || (count == best_count && cars > best_cars) {
            best_mask = mask;
            best_count = count;
            best_cars = cars;
        }
    }

    let mut best = Vec::new();
    for (idx, &mv) in candidates.iter().enumerate() {
        if ((best_mask >> idx) & 1) != 0 {
            best.push(mv);
        }
    }
    best
}

fn distribute_to_sidings(
    state: &mut State,
    out: &mut Output,
    scratch_sid: usize,
    extra_sid: usize,
) {
    while !is_empty_dep(state) {
        let mut candidates = Vec::new();
        for i in 0..R {
            if let Some(mv) = tail_bucket_block(state, i, scratch_sid, extra_sid) {
                candidates.push(mv);
            }
        }

        let moves = best_initial_turn(&candidates);
        assert!(!moves.is_empty());
        state.apply_turn(&moves);
        out.push_turn(moves);
    }

    assert_eq!(state.sid_len[scratch_sid], 0);
}

#[inline(always)]
fn push_single_move(state: &mut State, out: &mut Output, mv: Move) {
    state.apply_move(mv);
    out.push_turn(vec![mv]);
}

fn move_siding_prefix(
    state: &mut State,
    out: &mut Output,
    buffer_dep: usize,
    from_sid: usize,
    to_sid: usize,
    mut len: usize,
    max_chunk: usize,
) {
    assert!(max_chunk > 0);
    while len > 0 {
        let k = len.min(max_chunk);
        push_single_move(state, out, Move::siding_to_dep(buffer_dep, from_sid, k));
        push_single_move(state, out, Move::dep_to_siding(buffer_dep, to_sid, k));
        len -= k;
    }
}

fn find_car_in_sidings(state: &State, car: usize, scratch_sid: usize) -> (usize, usize) {
    for j in 0..R {
        if j == scratch_sid {
            continue;
        }
        for pos in 0..state.sid_len[j] as usize {
            if state.sid_car(j, pos) == car {
                return (j, pos);
            }
        }
    }
    panic!("car {} is not in a non-scratch siding", car);
}

fn place_target_line(state: &mut State, out: &mut Output, r: usize, scratch_sid: usize) {
    let mut fixed = 0;
    while fixed < INIT_LEN {
        let target = Input::target_id(r, fixed);
        let (src_sid, blockers) = find_car_in_sidings(state, target, scratch_sid);

        if blockers > 0 {
            let max_before = DEP_CAP - state.dep_len[r] as usize;
            move_siding_prefix(state, out, r, src_sid, scratch_sid, blockers, max_before);
            assert_eq!(state.sid_len[scratch_sid] as usize, blockers);
        }

        let mut run = 0;
        while fixed + run < INIT_LEN
            && run < state.sid_len[src_sid] as usize
            && state.sid_car(src_sid, run) == Input::target_id(r, fixed + run)
        {
            run += 1;
        }
        assert!(run > 0);

        push_single_move(state, out, Move::siding_to_dep(r, src_sid, run));
        fixed += run;

        if blockers > 0 {
            let max_after = DEP_CAP - state.dep_len[r] as usize;
            move_siding_prefix(state, out, r, scratch_sid, src_sid, blockers, max_after);
            assert_eq!(state.sid_len[scratch_sid], 0);
        }
    }
}

fn target_order(scratch_sid: usize, extra_sid: usize, scratch_first: bool) -> Vec<usize> {
    let mut order = Vec::with_capacity(R);
    for r in 0..R {
        if r != scratch_sid && r != extra_sid {
            order.push(r);
        }
    }
    if scratch_first {
        order.push(scratch_sid);
        order.push(extra_sid);
    } else {
        order.push(extra_sid);
        order.push(scratch_sid);
    }
    order
}

fn solve_variant(
    input: &Input,
    scratch_sid: usize,
    extra_sid: usize,
    scratch_first: bool,
) -> Output {
    let mut state = State::new(input);
    let mut out = Output::new();

    distribute_to_sidings(&mut state, &mut out, scratch_sid, extra_sid);

    for r in target_order(scratch_sid, extra_sid, scratch_first) {
        place_target_line(&mut state, &mut out, r, scratch_sid);
    }

    assert!(state.is_complete());
    assert!(out.turns.len() <= MAX_TURNS);
    out
}

fn solve(input: &Input) -> Output {
    let mut best: Option<Output> = None;

    for scratch_sid in 0..R {
        for extra_sid in 0..R {
            if extra_sid == scratch_sid {
                continue;
            }
            for scratch_first in [false, true] {
                let out = solve_variant(input, scratch_sid, extra_sid, scratch_first);
                if best
                    .as_ref()
                    .map_or(true, |current| out.turns.len() < current.turns.len())
                {
                    best = Some(out);
                }
            }
        }
    }

    best.unwrap()
}

fn main() {
    let input = Input::read();
    let out = solve(&input);
    out.print();
}
