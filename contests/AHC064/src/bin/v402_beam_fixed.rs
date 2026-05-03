// v402_beam_fixed.rs
use std::collections::HashMap;
use std::time::Instant;

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

fn fallback_solve(input: &Input) -> Output {
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

const BEAM_WIDTH: usize = 90;
const CAND_LIMIT: usize = 18;
const TURN_LIMIT: usize = 24;
const TURN_ENUM_VISIT_LIMIT: usize = 3500;
const BEAM_TIME_LIMIT_SEC: f64 = 0.45;

#[derive(Debug, Clone, Copy)]
struct CandMove {
    mv: Move,
    score: i32,
}

#[derive(Debug, Clone)]
struct BeamItem {
    state: State,
    fixed: [u8; R],
    rec: usize,
    score: i64,
}

#[derive(Debug, Clone)]
struct CandidateItem {
    state: State,
    fixed: [u8; R],
    parent_rec: usize,
    turn: Vec<Move>,
    score: i64,
    hash: u64,
}

#[derive(Debug, Clone)]
struct Record {
    parent: usize,
    turn: Vec<Move>,
}

fn recompute_fixed(state: &State) -> [u8; R] {
    let mut fixed = [0_u8; R];
    for r in 0..R {
        let len = state.dep_len[r] as usize;
        let mut f = 0;
        while f < INIT_LEN && f < len && state.dep_car(r, f) == Input::target_id(r, f) {
            f += 1;
        }
        fixed[r] = f as u8;
    }
    fixed
}

#[inline(always)]
fn dirty_len(state: &State, fixed: &[u8; R], r: usize) -> usize {
    state.dep_len[r] as usize - fixed[r] as usize
}

#[inline(always)]
fn dep_is_clean(state: &State, fixed: &[u8; R], r: usize) -> bool {
    dirty_len(state, fixed, r) == 0
}

#[inline(always)]
fn consecutive(a: usize, b: usize) -> bool {
    Input::target_line(a) == Input::target_line(b)
        && Input::target_pos(a) + 1 == Input::target_pos(b)
}

fn find_car_in_sid(state: &State, target: usize) -> Option<(usize, usize)> {
    for j in 0..R {
        for pos in 0..state.sid_len[j] as usize {
            if state.sid_car(j, pos) == target {
                return Some((j, pos));
            }
        }
    }
    None
}

fn add_candidate(candidates: &mut Vec<CandMove>, cand: CandMove) {
    for existing in candidates.iter_mut() {
        if existing.mv == cand.mv {
            if cand.score > existing.score {
                existing.score = cand.score;
            }
            return;
        }
    }
    candidates.push(cand);
}

fn gen_commit_candidates(state: &State, fixed: &[u8; R], candidates: &mut Vec<CandMove>) {
    for j in 0..R {
        if state.sid_len[j] == 0 {
            continue;
        }
        let head = state.sid_car(j, 0);
        let r = Input::target_line(head);
        let f = fixed[r] as usize;
        if f >= INIT_LEN || Input::target_pos(head) != f || !dep_is_clean(state, fixed, r) {
            continue;
        }

        let mut k = 0;
        while f + k < INIT_LEN
            && k < state.sid_len[j] as usize
            && state.sid_car(j, k) == Input::target_id(r, f + k)
        {
            k += 1;
        }

        if k > 0 && state.dep_len[r] as usize + k <= DEP_CAP {
            add_candidate(
                candidates,
                CandMove {
                    mv: Move::siding_to_dep(r, j, k),
                    score: 120_000 + 4_000 * k as i32,
                },
            );
        }
    }
}

fn gen_expose_candidates(state: &State, fixed: &[u8; R], candidates: &mut Vec<CandMove>) {
    for r in 0..R {
        let f = fixed[r] as usize;
        if f >= INIT_LEN || !dep_is_clean(state, fixed, r) {
            continue;
        }

        let target = Input::target_id(r, f);
        let Some((src_sid, depth)) = find_car_in_sid(state, target) else {
            continue;
        };
        if depth == 0 {
            continue;
        }

        let mut buffers = Vec::new();
        for b in 0..R {
            if !dep_is_clean(state, fixed, b) {
                continue;
            }
            let cap = DEP_CAP - state.dep_len[b] as usize;
            if cap == 0 {
                continue;
            }
            let q = depth.min(cap);
            let mut score = 44_000 - 450 * depth as i32 + 70 * q as i32;
            if q == depth {
                score += 10_000;
            }
            if b == r {
                score -= 3_000;
            }
            if fixed[b] as usize == INIT_LEN {
                score -= 1_500;
            }
            score -= 20 * fixed[b] as i32;
            buffers.push((score, b, q));
        }

        buffers.sort_by(|a, b| b.0.cmp(&a.0));
        for &(score, b, q) in buffers.iter().take(3) {
            add_candidate(
                candidates,
                CandMove {
                    mv: Move::siding_to_dep(b, src_sid, q),
                    score,
                },
            );
        }
    }
}

fn tail_same_target_len(state: &State, fixed: &[u8; R], r: usize) -> usize {
    let len = state.dep_len[r] as usize;
    let f = fixed[r] as usize;
    let tail = state.dep_car(r, len - 1);
    let target_line = Input::target_line(tail);
    let mut k = 1;
    while len >= f + k + 1 {
        let car = state.dep_car(r, len - k - 1);
        if Input::target_line(car) != target_line {
            break;
        }
        k += 1;
    }
    k
}

fn tail_consecutive_len(state: &State, fixed: &[u8; R], r: usize) -> usize {
    let len = state.dep_len[r] as usize;
    let f = fixed[r] as usize;
    let mut start = len - 1;
    while start > f {
        let prev = state.dep_car(r, start - 1);
        let cur = state.dep_car(r, start);
        if !consecutive(prev, cur) {
            break;
        }
        start -= 1;
    }
    len - start
}

fn flush_move_score(state: &State, fixed: &[u8; R], b: usize, j: usize, k: usize) -> i32 {
    let len = state.dep_len[b] as usize;
    let start = len - k;
    let first = state.dep_car(b, start);
    let last = state.dep_car(b, len - 1);
    let mut score = 9_000 + 140 * k as i32 - 35 * state.sid_len[j] as i32;

    for p in start..len - 1 {
        if consecutive(state.dep_car(b, p), state.dep_car(b, p + 1)) {
            score += 900;
        }
    }

    if state.sid_len[j] > 0 && consecutive(last, state.sid_car(j, 0)) {
        score += 1_400;
    }

    let first_line = Input::target_line(first);
    if fixed[first_line] as usize == Input::target_pos(first)
        && dep_is_clean(state, fixed, first_line)
    {
        score += 4_000;
        if first_line == b && len - k == fixed[b] as usize {
            score += 4_000;
        }
    }

    score
}

fn gen_flush_candidates(state: &State, fixed: &[u8; R], candidates: &mut Vec<CandMove>) {
    for b in 0..R {
        let d = dirty_len(state, fixed, b);
        if d == 0 {
            continue;
        }

        let mut ks = vec![
            1,
            d,
            tail_same_target_len(state, fixed, b),
            tail_consecutive_len(state, fixed, b),
        ];
        ks.sort_unstable();
        ks.dedup();

        for k in ks {
            if k == 0 || k > d {
                continue;
            }
            let mut sid_scores = Vec::new();
            for j in 0..R {
                if state.sid_len[j] as usize + k > SIDING_CAP {
                    continue;
                }
                sid_scores.push((flush_move_score(state, fixed, b, j, k), j));
            }
            sid_scores.sort_by(|a, b| b.0.cmp(&a.0));
            for &(score, j) in sid_scores.iter().take(3) {
                add_candidate(
                    candidates,
                    CandMove {
                        mv: Move::dep_to_siding(b, j, k),
                        score,
                    },
                );
            }
        }
    }
}

fn gen_candidates(state: &State, fixed: &[u8; R]) -> Vec<CandMove> {
    let mut candidates = Vec::new();
    gen_commit_candidates(state, fixed, &mut candidates);
    gen_expose_candidates(state, fixed, &mut candidates);
    gen_flush_candidates(state, fixed, &mut candidates);

    candidates.sort_by(|a, b| b.score.cmp(&a.score));
    candidates.truncate(CAND_LIMIT);
    candidates
}

fn push_turn_candidate(best: &mut Vec<(i32, Vec<Move>)>, score: i32, moves: &[Move]) {
    if moves.is_empty() {
        return;
    }
    best.push((score + 120 * moves.len() as i32, moves.to_vec()));
    best.sort_by(|a, b| b.0.cmp(&a.0));
    best.dedup_by(|a, b| a.1 == b.1);
    if best.len() > TURN_LIMIT {
        best.truncate(TURN_LIMIT);
    }
}

fn dfs_turn_candidates(
    sorted: &[CandMove],
    pos: usize,
    last_j: i32,
    used_dep: u16,
    used_sid: u16,
    current_score: i32,
    current: &mut Vec<Move>,
    best: &mut Vec<(i32, Vec<Move>)>,
    visits: &mut usize,
) {
    *visits += 1;
    if *visits > TURN_ENUM_VISIT_LIMIT {
        return;
    }
    push_turn_candidate(best, current_score, current);

    for idx in pos..sorted.len() {
        let cand = sorted[idx];
        let mv = cand.mv;
        let dep_bit = 1_u16 << mv.i;
        let sid_bit = 1_u16 << mv.j;
        if (used_dep & dep_bit) != 0 || (used_sid & sid_bit) != 0 {
            continue;
        }
        if mv.j as i32 <= last_j {
            continue;
        }

        current.push(mv);
        dfs_turn_candidates(
            sorted,
            idx + 1,
            mv.j as i32,
            used_dep | dep_bit,
            used_sid | sid_bit,
            current_score + cand.score + 10 * mv.k as i32,
            current,
            best,
            visits,
        );
        current.pop();
        if *visits > TURN_ENUM_VISIT_LIMIT {
            return;
        }
    }
}

fn gen_turn_candidates(candidates: &[CandMove]) -> Vec<Vec<Move>> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|a, b| {
        a.mv.i
            .cmp(&b.mv.i)
            .then(a.mv.j.cmp(&b.mv.j))
            .then(b.score.cmp(&a.score))
    });

    let mut best = Vec::new();
    let mut current = Vec::new();
    let mut visits = 0;
    dfs_turn_candidates(
        &sorted,
        0,
        -1,
        0,
        0,
        0,
        &mut current,
        &mut best,
        &mut visits,
    );

    best.sort_by(|a, b| b.0.cmp(&a.0));
    best.truncate(TURN_LIMIT);
    best.into_iter().map(|(_, moves)| moves).collect()
}

fn ready_commit_len_sum(state: &State, fixed: &[u8; R]) -> (usize, usize) {
    let mut sum = 0;
    let mut count = 0;
    for j in 0..R {
        if state.sid_len[j] == 0 {
            continue;
        }
        let head = state.sid_car(j, 0);
        let r = Input::target_line(head);
        let f = fixed[r] as usize;
        if f >= INIT_LEN || Input::target_pos(head) != f || !dep_is_clean(state, fixed, r) {
            continue;
        }
        let mut k = 0;
        while f + k < INIT_LEN
            && k < state.sid_len[j] as usize
            && state.sid_car(j, k) == Input::target_id(r, f + k)
        {
            k += 1;
        }
        if k > 0 {
            sum += k;
            count += 1;
        }
    }
    (sum, count)
}

fn next_target_depth_sum(state: &State, fixed: &[u8; R]) -> i32 {
    let mut depth = 0;
    for r in 0..R {
        let f = fixed[r] as usize;
        if f >= INIT_LEN {
            continue;
        }
        let target = Input::target_id(r, f);
        let mut found = false;
        for j in 0..R {
            for pos in 0..state.sid_len[j] as usize {
                if state.sid_car(j, pos) == target {
                    depth += pos as i32;
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if found {
            continue;
        }
        for b in 0..R {
            let start = fixed[b] as usize;
            for pos in start..state.dep_len[b] as usize {
                if state.dep_car(b, pos) == target {
                    depth += 18 + (state.dep_len[b] as usize - 1 - pos) as i32;
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
    }
    depth
}

fn sid_run_score(state: &State) -> i32 {
    let mut score = 0;
    for j in 0..R {
        let len = state.sid_len[j] as usize;
        for p in 0..len.saturating_sub(1) {
            if consecutive(state.sid_car(j, p), state.sid_car(j, p + 1)) {
                score += 1;
            }
        }
    }
    score
}

fn eval_beam_state(state: &State, fixed: &[u8; R], turns: usize) -> i64 {
    if state.is_complete() {
        return 1_000_000_000_i64 - turns as i64;
    }

    let fixed_sum: i32 = fixed.iter().map(|&x| x as i32).sum();
    let dirty_sum: i32 = (0..R).map(|r| dirty_len(state, fixed, r) as i32).sum();
    let (ready_len, ready_count) = ready_commit_len_sum(state, fixed);
    let depth_sum = next_target_depth_sum(state, fixed);
    let run_score = sid_run_score(state);
    let sid_load_penalty: i32 = state
        .sid_len
        .iter()
        .map(|&len| {
            let len = len as i32;
            if len <= 15 {
                0
            } else {
                (len - 15) * (len - 15)
            }
        })
        .sum();

    22_000_i64 * fixed_sum as i64
        + 1_000_i64 * ready_len as i64
        + 700_i64 * ready_count as i64
        + 120_i64 * run_score as i64
        - 260_i64 * depth_sum as i64
        - 180_i64 * dirty_sum as i64
        - 80_i64 * sid_load_penalty as i64
        - 4_i64 * turns as i64
}

#[inline(always)]
fn mix_hash(mut h: u64, x: u64) -> u64 {
    h ^= x
        .wrapping_add(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(h << 6)
        .wrapping_add(h >> 2);
    h
}

fn hash_beam_state(state: &State, fixed: &[u8; R]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for r in 0..R {
        h = mix_hash(h, fixed[r] as u64);
        h = mix_hash(h, state.dep_len[r] as u64);
        for p in 0..state.dep_len[r] as usize {
            h = mix_hash(h, state.dep_car(r, p) as u64 + 1);
        }
    }
    for j in 0..R {
        h = mix_hash(h, state.sid_len[j] as u64);
        for p in 0..state.sid_len[j] as usize {
            h = mix_hash(h, state.sid_car(j, p) as u64 + 101);
        }
    }
    h
}

fn reconstruct_output(records: &[Record], mut rec: usize) -> Output {
    let mut turns = Vec::new();
    while rec != 0 {
        turns.push(records[rec].turn.clone());
        rec = records[rec].parent;
    }
    turns.reverse();
    Output { turns }
}

fn beam_solve(input: &Input, fallback_turns: usize) -> Option<Output> {
    let started = Instant::now();
    let root_state = State::new(input);
    let root_fixed = recompute_fixed(&root_state);
    let root_score = eval_beam_state(&root_state, &root_fixed, 0);

    let mut records = vec![Record {
        parent: usize::MAX,
        turn: Vec::new(),
    }];
    let mut beam = vec![BeamItem {
        state: root_state,
        fixed: root_fixed,
        rec: 0,
        score: root_score,
    }];

    let mut best_rec = None;
    let mut best_turns = fallback_turns;

    for depth in 0..fallback_turns.saturating_sub(1) {
        if started.elapsed().as_secs_f64() >= BEAM_TIME_LIMIT_SEC {
            break;
        }

        let mut next_candidates = Vec::new();
        for item in &beam {
            let candidates = gen_candidates(&item.state, &item.fixed);
            if candidates.is_empty() {
                continue;
            }
            let turns = gen_turn_candidates(&candidates);
            for turn in turns {
                let mut next_state = item.state.clone();
                next_state.apply_turn(&turn);
                let next_fixed = recompute_fixed(&next_state);
                let score = eval_beam_state(&next_state, &next_fixed, depth + 1);
                let hash = hash_beam_state(&next_state, &next_fixed);
                next_candidates.push(CandidateItem {
                    state: next_state,
                    fixed: next_fixed,
                    parent_rec: item.rec,
                    turn,
                    score,
                    hash,
                });
            }
        }

        if next_candidates.is_empty() {
            break;
        }

        next_candidates.sort_by(|a, b| b.score.cmp(&a.score));
        let mut seen = HashMap::new();
        let mut unique = Vec::new();
        for cand in next_candidates {
            if seen.insert(cand.hash, ()).is_none() {
                unique.push(cand);
                if unique.len() >= BEAM_WIDTH * 4 {
                    break;
                }
            }
        }

        unique.sort_by(|a, b| b.score.cmp(&a.score));
        unique.truncate(BEAM_WIDTH);

        let mut next_beam = Vec::with_capacity(unique.len());
        for cand in unique {
            let rec = records.len();
            records.push(Record {
                parent: cand.parent_rec,
                turn: cand.turn,
            });
            if cand.state.is_complete() && depth + 1 < best_turns {
                best_turns = depth + 1;
                best_rec = Some(rec);
            }
            if depth + 1 < best_turns {
                next_beam.push(BeamItem {
                    state: cand.state,
                    fixed: cand.fixed,
                    rec,
                    score: cand.score,
                });
            }
        }

        if next_beam.is_empty() {
            break;
        }
        next_beam.sort_by(|a, b| b.score.cmp(&a.score));
        beam = next_beam;
    }

    best_rec.map(|rec| reconstruct_output(&records, rec))
}

fn solve(input: &Input) -> Output {
    let fallback = fallback_solve(input);
    if let Some(beam) = beam_solve(input, fallback.turns.len()) {
        if beam.turns.len() < fallback.turns.len() {
            return beam;
        }
    }
    fallback
}

fn main() {
    let input = Input::read();
    let out = solve(&input);
    out.print();
}
