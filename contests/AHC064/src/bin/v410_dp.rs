// v410_dp.rs
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const CAR_COUNT: usize = R * INIT_LEN;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

const AREA_DEP: usize = 0;
const AREA_SID: usize = 1;

#[derive(Clone)]
struct Input {
    initial: [[usize; INIT_LEN]; R],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();
        let _r = it.next().unwrap().parse::<usize>().unwrap();

        let mut initial = [[0; INIT_LEN]; R];
        for r in 0..R {
            for c in 0..INIT_LEN {
                initial[r][c] = it.next().unwrap().parse::<usize>().unwrap();
            }
        }

        Self { initial }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Move {
    kind: usize,
    i: usize,
    j: usize,
    k: usize,
}

impl Move {
    fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    fn siding_to_dep(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_SIDING_TO_DEP,
            i,
            j,
            k,
        }
    }
}

struct Output {
    turns: Vec<Vec<Move>>,
}

impl Output {
    fn new() -> Self {
        Self { turns: Vec::new() }
    }

    fn push_move(&mut self, mv: Move) {
        if let Some(last) = self.turns.last_mut() {
            if can_add_to_turn(last, mv) {
                last.push(mv);
                return;
            }
        }
        self.turns.push(vec![mv]);
    }

    fn push_turn(&mut self, moves: Vec<Move>) {
        if !moves.is_empty() {
            self.turns.push(moves);
        }
    }

    fn compact_turns(&mut self) {
        let moves: Vec<Move> = self.turns.iter().flatten().copied().collect();
        let mut turns: Vec<Vec<Move>> = Vec::new();
        let mut dep_next = [0usize; R];
        let mut sid_next = [0usize; R];

        for mv in moves {
            let mut t = dep_next[mv.i].max(sid_next[mv.j]);
            while t < turns.len() && !can_add_to_turn(&turns[t], mv) {
                t += 1;
            }
            if t == turns.len() {
                turns.push(Vec::new());
            }
            turns[t].push(mv);
            dep_next[mv.i] = t + 1;
            sid_next[mv.j] = t + 1;
        }

        self.turns = turns;
    }

    fn print(&self) {
        let move_count: usize = self.turns.iter().map(Vec::len).sum();
        let mut s = String::with_capacity(16 + self.turns.len() * 4 + move_count * 16);
        s.push_str(&format!("{}\n", self.turns.len()));
        for moves in &self.turns {
            s.push_str(&format!("{}\n", moves.len()));
            for mv in moves {
                s.push_str(&format!("{} {} {} {}\n", mv.kind, mv.i, mv.j, mv.k));
            }
        }

        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        out.write_all(s.as_bytes()).unwrap();
    }
}

fn can_add_to_turn(moves: &[Move], mv: Move) -> bool {
    for &other in moves {
        if other.i == mv.i || other.j == mv.j {
            return false;
        }
        if (other.i < mv.i && other.j >= mv.j) || (mv.i < other.i && mv.j >= other.j) {
            return false;
        }
    }
    true
}

#[derive(Clone)]
struct State {
    dep: [[u8; DEP_CAP]; R],
    dep_len: [u8; R],
    sid: [[u8; SIDING_CAP]; R],
    sid_head: [u8; R],
    sid_len: [u8; R],
    car_area: [u8; CAR_COUNT],
    car_line: [u8; CAR_COUNT],
    car_slot: [u8; CAR_COUNT],
}

impl State {
    fn new(input: &Input) -> Self {
        let mut dep = [[0u8; DEP_CAP]; R];
        let dep_len = [INIT_LEN as u8; R];
        let sid = [[0u8; SIDING_CAP]; R];
        let sid_head = [0u8; R];
        let sid_len = [0u8; R];
        let mut car_area = [AREA_DEP as u8; CAR_COUNT];
        let mut car_line = [0u8; CAR_COUNT];
        let mut car_slot = [0u8; CAR_COUNT];

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = input.initial[r][c];
                dep[r][c] = car as u8;
                car_area[car] = AREA_DEP as u8;
                car_line[car] = r as u8;
                car_slot[car] = c as u8;
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
        }
    }

    fn dep_len(&self, i: usize) -> usize {
        self.dep_len[i] as usize
    }

    fn sid_len(&self, j: usize) -> usize {
        self.sid_len[j] as usize
    }

    fn dep_car(&self, i: usize, pos: usize) -> usize {
        self.dep[i][pos] as usize
    }

    fn sid_slot(head: usize, offset: usize) -> usize {
        let slot = head + offset;
        if slot >= SIDING_CAP {
            slot - SIDING_CAP
        } else {
            slot
        }
    }

    fn sid_offset_from_slot(head: usize, slot: usize) -> usize {
        if slot >= head {
            slot - head
        } else {
            slot + SIDING_CAP - head
        }
    }

    fn sid_head_after_pop(head: usize, k: usize) -> usize {
        Self::sid_slot(head, k)
    }

    fn sid_head_after_push(head: usize, k: usize) -> usize {
        if head >= k {
            head - k
        } else {
            head + SIDING_CAP - k
        }
    }

    fn sid_car(&self, j: usize, pos: usize) -> usize {
        let slot = Self::sid_slot(self.sid_head[j] as usize, pos);
        self.sid[j][slot] as usize
    }

    fn line_len(&self, area: usize, line: usize) -> usize {
        if area == AREA_DEP {
            self.dep_len(line)
        } else {
            self.sid_len(line)
        }
    }

    fn line_car(&self, area: usize, line: usize, pos: usize) -> usize {
        if area == AREA_DEP {
            self.dep_car(line, pos)
        } else {
            self.sid_car(line, pos)
        }
    }

    fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            let old_dep_len = self.dep_len(mv.i);
            let new_dep_len = old_dep_len - mv.k;
            let new_sid_head = Self::sid_head_after_push(self.sid_head[mv.j] as usize, mv.k);

            for offset in 0..mv.k {
                let dep_pos = new_dep_len + offset;
                let car = self.dep[mv.i][dep_pos];
                let car_idx = car as usize;
                let sid_slot = Self::sid_slot(new_sid_head, offset);
                self.sid[mv.j][sid_slot] = car;
                self.car_area[car_idx] = AREA_SID as u8;
                self.car_line[car_idx] = mv.j as u8;
                self.car_slot[car_idx] = sid_slot as u8;
            }

            self.dep_len[mv.i] = new_dep_len as u8;
            self.sid_head[mv.j] = new_sid_head as u8;
            self.sid_len[mv.j] += mv.k as u8;
        } else {
            let old_dep_len = self.dep_len(mv.i);
            let old_sid_head = self.sid_head[mv.j] as usize;

            for offset in 0..mv.k {
                let sid_slot = Self::sid_slot(old_sid_head, offset);
                let car = self.sid[mv.j][sid_slot];
                let car_idx = car as usize;
                let dep_pos = old_dep_len + offset;
                self.dep[mv.i][dep_pos] = car;
                self.car_area[car_idx] = AREA_DEP as u8;
                self.car_line[car_idx] = mv.i as u8;
                self.car_slot[car_idx] = dep_pos as u8;
            }

            self.dep_len[mv.i] = (old_dep_len + mv.k) as u8;
            self.sid_head[mv.j] = Self::sid_head_after_pop(old_sid_head, mv.k) as u8;
            self.sid_len[mv.j] -= mv.k as u8;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Block {
    area: usize,
    line: usize,
    start: usize,
    len: usize,
    r: usize,
    l: usize,
    u: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Interval {
    r: usize,
    l: usize,
    u: usize,
}

#[derive(Debug, Clone, Copy)]
struct MergeJob {
    a: Interval,
    b: Interval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Done,
    Ready,
    Move(Move),
}

#[derive(Clone)]
struct BeamNode {
    jobs: Vec<MergeJob>,
    used: [u16; R],
    gain: i32,
}

impl Block {
    fn end_pos(&self) -> usize {
        self.start + self.len
    }

    fn contains(&self, iv: Interval) -> bool {
        self.r == iv.r && self.l <= iv.l && iv.u <= self.u
    }
}

fn target_line(car: usize) -> usize {
    car / INIT_LEN
}

fn target_pos(car: usize) -> usize {
    car % INIT_LEN
}

fn push_block_from_run(
    blocks: &mut Vec<Block>,
    area: usize,
    line: usize,
    start: usize,
    len: usize,
    first: usize,
    last: usize,
) {
    blocks.push(Block {
        area,
        line,
        start,
        len,
        r: target_line(first),
        l: target_pos(first),
        u: target_pos(last),
    });
}

fn scan_dep_line(state: &State, i: usize, blocks: &mut Vec<Block>) {
    let line_len = state.dep_len(i);
    if line_len == 0 {
        return;
    }

    let mut start = 0;
    let mut first = state.dep_car(i, 0);
    let mut prev = first;
    let mut len = 1;

    for pos in 1..line_len {
        let car = state.dep_car(i, pos);
        if target_line(car) == target_line(prev) && car == prev + 1 {
            len += 1;
        } else {
            push_block_from_run(blocks, AREA_DEP, i, start, len, first, prev);
            start += len;
            first = car;
            len = 1;
        }
        prev = car;
    }

    push_block_from_run(blocks, AREA_DEP, i, start, len, first, prev);
}

fn scan_sid_line(state: &State, j: usize, blocks: &mut Vec<Block>) {
    let line_len = state.sid_len(j);
    if line_len == 0 {
        return;
    }

    let mut start = 0;
    let mut first = state.sid_car(j, 0);
    let mut prev = first;
    let mut len = 1;

    for pos in 1..line_len {
        let car = state.sid_car(j, pos);
        if target_line(car) == target_line(prev) && car == prev + 1 {
            len += 1;
        } else {
            push_block_from_run(blocks, AREA_SID, j, start, len, first, prev);
            start += len;
            first = car;
            len = 1;
        }
        prev = car;
    }

    push_block_from_run(blocks, AREA_SID, j, start, len, first, prev);
}

fn collect_blocks(state: &State) -> Vec<Block> {
    let mut blocks = Vec::with_capacity(100);
    for i in 0..R {
        scan_dep_line(state, i, &mut blocks);
    }
    for j in 0..R {
        scan_sid_line(state, j, &mut blocks);
    }
    blocks
}

fn find_block(state: &State, iv: Interval) -> Option<Block> {
    let first_id = iv.r * INIT_LEN + iv.l;
    let area = state.car_area[first_id] as usize;
    let line = state.car_line[first_id] as usize;
    let start_pos = if area == AREA_DEP {
        state.car_slot[first_id] as usize
    } else {
        State::sid_offset_from_slot(
            state.sid_head[line] as usize,
            state.car_slot[first_id] as usize,
        )
    };
    let line_len = state.line_len(area, line);
    let iv_len = iv.u - iv.l + 1;
    if start_pos + iv_len > line_len {
        return None;
    }

    for offset in 0..iv_len {
        if state.line_car(area, line, start_pos + offset) != first_id + offset {
            return None;
        }
    }

    let mut block_start = start_pos;
    let mut l = iv.l;
    while block_start > 0 && l > 0 {
        let prev = state.line_car(area, line, block_start - 1);
        if prev + 1 != iv.r * INIT_LEN + l || target_line(prev) != iv.r {
            break;
        }
        block_start -= 1;
        l -= 1;
    }

    let mut u = iv.u;
    while start_pos + (u - iv.l + 1) < line_len && u + 1 < INIT_LEN {
        let next_pos = start_pos + (u - iv.l + 1);
        let next = state.line_car(area, line, next_pos);
        if next != iv.r * INIT_LEN + u + 1 {
            break;
        }
        u += 1;
    }

    Some(Block {
        area,
        line,
        start: block_start,
        len: u - l + 1,
        r: iv.r,
        l,
        u,
    })
}

fn interval_completed(state: &State, iv: Interval) -> bool {
    find_block(state, iv).is_some()
}

fn dep_tail_block(state: &State, i: usize) -> Option<Block> {
    let line_len = state.dep_len(i);
    if line_len == 0 {
        return None;
    }
    let mut start = line_len - 1;
    let last = state.dep_car(i, start);
    let r = target_line(last);
    let mut first = last;
    while start > 0 {
        let prev = state.dep_car(i, start - 1);
        if target_line(prev) != r || prev + 1 != first {
            break;
        }
        start -= 1;
        first = prev;
    }
    Some(Block {
        area: AREA_DEP,
        line: i,
        start,
        len: line_len - start,
        r,
        l: target_pos(first),
        u: target_pos(last),
    })
}

fn sid_head_block(state: &State, j: usize) -> Option<Block> {
    let line_len = state.sid_len(j);
    if line_len == 0 {
        return None;
    }
    let first = state.sid_car(j, 0);
    let r = target_line(first);
    let mut last = first;
    let mut len = 1;
    while len < line_len {
        let next = state.sid_car(j, len);
        if target_line(next) != r || next != last + 1 {
            break;
        }
        last = next;
        len += 1;
    }
    Some(Block {
        area: AREA_SID,
        line: j,
        start: 0,
        len,
        r,
        l: target_pos(first),
        u: target_pos(last),
    })
}

fn choose_sid_for_block(
    state: &State,
    block: Block,
    forbidden_sid: Option<usize>,
) -> Option<usize> {
    let mut avoid = [false; R];
    if let Some(j) = forbidden_sid {
        avoid[j] = true;
    }
    choose_sid_avoiding(state, block, &avoid)
}

fn choose_sid_avoiding(state: &State, block: Block, avoid_sid: &[bool; R]) -> Option<usize> {
    let mut best = None;
    let mut best_score = i32::MIN;
    for j in 0..R {
        if avoid_sid[j] || state.sid_len(j) + block.len > SIDING_CAP {
            continue;
        }

        let mut score = (SIDING_CAP - state.sid_len(j)) as i32;
        if let Some(head) = sid_head_block(state, j) {
            if head.r == block.r && block.u + 1 == head.l {
                score += 1000;
            }
        }
        if score > best_score {
            best_score = score;
            best = Some(j);
        }
    }
    best
}

fn choose_dep_avoiding(state: &State, block: Block, avoid_dep: &[bool; R]) -> Option<usize> {
    let mut best = None;
    let mut best_score = i32::MIN;
    for i in 0..R {
        if avoid_dep[i] || state.dep_len(i) + block.len > DEP_CAP {
            continue;
        }

        let mut score = (DEP_CAP - state.dep_len(i)) as i32;
        if let Some(tail) = dep_tail_block(state, i) {
            if tail.r == block.r && tail.u + 1 == block.l {
                score += 1000;
            }
        }
        if score > best_score {
            best_score = score;
            best = Some(i);
        }
    }
    best
}

fn block_contains_other_protected(block: Block, protected: &[Interval], allowed: Interval) -> bool {
    protected
        .iter()
        .any(|&iv| iv != allowed && block.contains(iv))
}

fn is_valid_move(state: &State, mv: Move) -> bool {
    if mv.kind == MOVE_DEP_TO_SIDING {
        mv.k > 0 && state.dep_len(mv.i) >= mv.k && state.sid_len(mv.j) + mv.k <= SIDING_CAP
    } else {
        mv.k > 0 && state.sid_len(mv.j) >= mv.k && state.dep_len(mv.i) + mv.k <= DEP_CAP
    }
}

fn step_expose_dep_tail(
    state: &State,
    iv: Interval,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Option<Step> {
    let block = find_block(state, iv)?;
    if block.area == AREA_DEP {
        if block.end_pos() == state.dep_len(block.line) {
            return Some(Step::Ready);
        }
        let tail = dep_tail_block(state, block.line)?;
        if block_contains_other_protected(tail, protected, iv) {
            return None;
        }
        let j = choose_sid_avoiding(state, tail, avoid_sid)?;
        let mv = Move::dep_to_siding(block.line, j, tail.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    } else if block.start == 0 {
        let i = choose_dep_avoiding(state, block, avoid_dep)?;
        let mv = Move::siding_to_dep(i, block.line, block.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    } else {
        let head = sid_head_block(state, block.line)?;
        if block_contains_other_protected(head, protected, iv) {
            return None;
        }
        let i = choose_dep_avoiding(state, head, avoid_dep)?;
        let mv = Move::siding_to_dep(i, block.line, head.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    }
}

fn step_expose_sid_head(
    state: &State,
    iv: Interval,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Option<Step> {
    let block = find_block(state, iv)?;
    if block.area == AREA_SID {
        if block.start == 0 {
            return Some(Step::Ready);
        }
        let head = sid_head_block(state, block.line)?;
        if block_contains_other_protected(head, protected, iv) {
            return None;
        }
        let i = choose_dep_avoiding(state, head, avoid_dep)?;
        let mv = Move::siding_to_dep(i, block.line, head.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    } else if block.end_pos() == state.dep_len(block.line) {
        let j = choose_sid_avoiding(state, block, avoid_sid)?;
        let mv = Move::dep_to_siding(block.line, j, block.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    } else {
        let tail = dep_tail_block(state, block.line)?;
        if block_contains_other_protected(tail, protected, iv) {
            return None;
        }
        let j = choose_sid_avoiding(state, tail, avoid_sid)?;
        let mv = Move::dep_to_siding(block.line, j, tail.len);
        if is_valid_move(state, mv) {
            Some(Step::Move(mv))
        } else {
            None
        }
    }
}

fn merged_interval(job: MergeJob) -> Interval {
    Interval {
        r: job.a.r,
        l: job.a.l,
        u: job.b.u,
    }
}

fn next_job_step(
    state: &State,
    job: MergeJob,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Option<Step> {
    if interval_completed(state, merged_interval(job)) {
        return Some(Step::Done);
    }

    let a_step = step_expose_dep_tail(state, job.a, avoid_dep, avoid_sid, protected)?;
    if a_step != Step::Ready {
        return Some(a_step);
    }

    let a_block = find_block(state, job.a)?;
    if a_block.area != AREA_DEP || a_block.end_pos() != state.dep_len(a_block.line) {
        return None;
    }

    if let Some(b_probe) = find_block(state, job.b) {
        if b_probe.area == AREA_DEP
            && b_probe.line == a_block.line
            && b_probe.end_pos() <= a_block.start
        {
            let j = choose_sid_avoiding(state, a_block, avoid_sid)?;
            let mv = Move::dep_to_siding(a_block.line, j, a_block.len);
            if is_valid_move(state, mv) {
                return Some(Step::Move(mv));
            }
        }
    }

    let mut avoid_dep_for_b = *avoid_dep;
    avoid_dep_for_b[a_block.line] = true;
    let b_step = step_expose_sid_head(state, job.b, &avoid_dep_for_b, avoid_sid, protected)?;
    if b_step != Step::Ready {
        return Some(b_step);
    }

    let b_block = find_block(state, job.b)?;
    if b_block.area != AREA_SID || b_block.start != 0 || a_block.r != b_block.r {
        return None;
    }

    if state.dep_len(a_block.line) + b_block.len <= DEP_CAP {
        Some(Step::Move(Move::siding_to_dep(
            a_block.line,
            b_block.line,
            b_block.len,
        )))
    } else if state.sid_len(b_block.line) + a_block.len <= SIDING_CAP {
        Some(Step::Move(Move::dep_to_siding(
            a_block.line,
            b_block.line,
            a_block.len,
        )))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
struct DpEdge {
    gain: i32,
    mv: Move,
}

fn adjacent_pair_score_buf(cars: &[u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut score = 0;
    let mut prev = cars[0] as usize;
    for &car_u8 in cars.iter().take(len).skip(1) {
        let car = car_u8 as usize;
        if prev % INIT_LEN + 1 < INIT_LEN && car == prev + 1 {
            score += 1;
        }
        prev = car;
    }
    score
}

fn dep_line_score_buf(i: usize, cars: &[u8], len: usize) -> i32 {
    let anchor = if len > 0 && cars[0] as usize == i * INIT_LEN {
        1
    } else {
        0
    };
    adjacent_pair_score_buf(cars, len) + anchor
}

fn sid_line_score_buf(cars: &[u8], len: usize) -> i32 {
    adjacent_pair_score_buf(cars, len)
}

fn dep_line_score_state(state: &State, i: usize) -> i32 {
    dep_line_score_buf(i, &state.dep[i], state.dep_len(i))
}

fn sid_line_score_state(state: &State, j: usize) -> i32 {
    let len = state.sid_len(j);
    if len == 0 {
        return 0;
    }
    let mut score = 0;
    let mut prev = state.sid_car(j, 0);
    for pos in 1..len {
        let car = state.sid_car(j, pos);
        if prev % INIT_LEN + 1 < INIT_LEN && car == prev + 1 {
            score += 1;
        }
        prev = car;
    }
    score
}

fn pair_score(state: &State, i: usize, j: usize) -> i32 {
    dep_line_score_state(state, i) + sid_line_score_state(state, j)
}

fn move_delta_p(state: &State, mv: Move) -> i32 {
    let before = pair_score(state, mv.i, mv.j);
    let dep_len = state.dep_len(mv.i);
    let sid_len = state.sid_len(mv.j);
    let mut dep_after = [0u8; DEP_CAP];
    let mut sid_after = [0u8; SIDING_CAP];
    let dep_after_len;
    let sid_after_len;

    if mv.kind == MOVE_DEP_TO_SIDING {
        dep_after_len = dep_len - mv.k;
        sid_after_len = sid_len + mv.k;
        for pos in 0..dep_after_len {
            dep_after[pos] = state.dep[mv.i][pos];
        }
        for pos in 0..mv.k {
            sid_after[pos] = state.dep[mv.i][dep_after_len + pos];
        }
        for pos in 0..sid_len {
            sid_after[mv.k + pos] = state.sid_car(mv.j, pos) as u8;
        }
    } else {
        dep_after_len = dep_len + mv.k;
        sid_after_len = sid_len - mv.k;
        for pos in 0..dep_len {
            dep_after[pos] = state.dep[mv.i][pos];
        }
        for pos in 0..mv.k {
            dep_after[dep_len + pos] = state.sid_car(mv.j, pos) as u8;
        }
        for pos in mv.k..sid_len {
            sid_after[pos - mv.k] = state.sid_car(mv.j, pos) as u8;
        }
    }

    dep_line_score_buf(mv.i, &dep_after, dep_after_len)
        + sid_line_score_buf(&sid_after, sid_after_len)
        - before
}

fn best_move_for_pair(state: &State, i: usize, j: usize) -> Option<DpEdge> {
    let mut best: Option<DpEdge> = None;

    let max_dep_to_sid = state.dep_len(i).min(SIDING_CAP - state.sid_len(j));
    for k in 1..=max_dep_to_sid {
        let mv = Move::dep_to_siding(i, j, k);
        let gain = move_delta_p(state, mv);
        if gain > 0
            && best
                .map(|edge| gain > edge.gain || (gain == edge.gain && k < edge.mv.k))
                .unwrap_or(true)
        {
            best = Some(DpEdge { gain, mv });
        }
    }

    let max_sid_to_dep = state.sid_len(j).min(DEP_CAP - state.dep_len(i));
    for k in 1..=max_sid_to_dep {
        let mv = Move::siding_to_dep(i, j, k);
        let gain = move_delta_p(state, mv);
        if gain > 0
            && best
                .map(|edge| gain > edge.gain || (gain == edge.gain && k < edge.mv.k))
                .unwrap_or(true)
        {
            best = Some(DpEdge { gain, mv });
        }
    }

    best
}

fn best_dp_turn(state: &State) -> Vec<Move> {
    let mut edge = [[None; R]; R];
    for i in 0..R {
        for j in 0..R {
            edge[i][j] = best_move_for_pair(state, i, j);
        }
    }

    let mut dp = [[0i32; R + 1]; R + 1];
    let mut prev = [[0u8; R + 1]; R + 1];
    for i in 0..R {
        for j in 0..R {
            let mut best = dp[i][j + 1];
            let mut choice = 0u8;

            if dp[i + 1][j] > best {
                best = dp[i + 1][j];
                choice = 1;
            }

            if let Some(e) = edge[i][j] {
                let use_score = dp[i][j] + e.gain;
                if use_score > best {
                    best = use_score;
                    choice = 2;
                }
            }

            dp[i + 1][j + 1] = best;
            prev[i + 1][j + 1] = choice;
        }
    }

    let mut moves = Vec::new();
    let mut i = R;
    let mut j = R;
    while i > 0 && j > 0 {
        match prev[i][j] {
            2 => {
                moves.push(edge[i - 1][j - 1].unwrap().mv);
                i -= 1;
                j -= 1;
            }
            1 => j -= 1,
            _ => i -= 1,
        }
    }
    moves.reverse();
    moves
}

fn execute_dp_improvements(state: &mut State, output: &mut Output) {
    for _ in 0..200 {
        let turn = best_dp_turn(state);
        if turn.is_empty() {
            break;
        }
        for &mv in &turn {
            state.apply_move(mv);
        }
        output.push_turn(turn);
    }
}

fn move_weight(state: &State, mv: Move) -> i32 {
    let mut weight = 100 + mv.k as i32;
    if mv.kind == MOVE_SIDING_TO_DEP {
        if let Some(head) = sid_head_block(state, mv.j) {
            if let Some(tail) = dep_tail_block(state, mv.i) {
                if tail.r == head.r && tail.u + 1 == head.l {
                    weight += 5000 + ((tail.len + head.len) * (tail.len + head.len)) as i32;
                }
            }
        }
    } else if let Some(tail) = dep_tail_block(state, mv.i) {
        if let Some(head) = sid_head_block(state, mv.j) {
            if tail.r == head.r && tail.u + 1 == head.l {
                weight += 5000 + ((tail.len + head.len) * (tail.len + head.len)) as i32;
            }
        }
    }
    weight
}

fn select_non_crossing_moves(state: &State, candidates: &[Move]) -> Vec<Move> {
    let n = candidates.len();
    let mut best_score = i32::MIN;
    let mut best = Vec::new();
    for mask in 1usize..(1usize << n) {
        let mut moves = Vec::new();
        let mut ok = true;
        let mut score = 0;
        for (idx, &mv) in candidates.iter().enumerate() {
            if ((mask >> idx) & 1) == 0 {
                continue;
            }
            if !is_valid_move(state, mv) || !can_add_to_turn(&moves, mv) {
                ok = false;
                break;
            }
            score += move_weight(state, mv);
            moves.push(mv);
        }
        if ok && score > best_score {
            best_score = score;
            best = moves;
        }
    }
    best
}

fn interval_bits(a: Block, b: Block) -> u16 {
    let mut bits = 0u16;
    for p in a.l..=a.u {
        bits |= 1u16 << p;
    }
    for p in b.l..=b.u {
        bits |= 1u16 << p;
    }
    bits
}

fn simulate_subset_score(state: &State, jobs: &[MergeJob], gain: i32) -> Option<i32> {
    let mut st = state.clone();
    let mut active = jobs.to_vec();
    let mut completed = 0usize;
    let mut turns = 0usize;
    let mut moved = 0usize;

    for _ in 0..120 {
        let before_len = active.len();
        active.retain(|&job| !interval_completed(&st, merged_interval(job)));
        completed += before_len - active.len();
        if active.is_empty() {
            break;
        }

        let (avoid_dep, avoid_sid, protected) = protected_lines(&st, &active);
        let mut candidates = Vec::new();
        for &job in &active {
            if let Some(Step::Move(mv)) =
                next_job_step(&st, job, &avoid_dep, &avoid_sid, &protected)
            {
                if !candidates.iter().any(|&x| x == mv) {
                    candidates.push(mv);
                }
            }
        }
        if candidates.is_empty() {
            break;
        }
        candidates.sort_by_key(|&mv| -move_weight(&st, mv));
        candidates.truncate(R);
        let turn = select_non_crossing_moves(&st, &candidates);
        if turn.is_empty() {
            break;
        }
        for &mv in &turn {
            moved += mv.k;
            st.apply_move(mv);
        }
        turns += 1;
    }

    if completed == 0 {
        None
    } else {
        Some(
            (completed as i32) * 1_000_000 + (jobs.len() as i32) * 8_000 + gain * 80
                - (turns as i32) * 2_500
                - (moved as i32) * 5,
        )
    }
}

fn choose_wave_jobs(state: &State) -> Vec<MergeJob> {
    const MAX_ACTIVE_JOBS: usize = 10;
    const CAND_LIMIT: usize = 36;
    const BEAM_WIDTH: usize = 96;
    let mut blocks_by_r: [Vec<Block>; R] = std::array::from_fn(|_| Vec::new());
    for block in collect_blocks(state) {
        if block.len < INIT_LEN {
            blocks_by_r[block.r].push(block);
        }
    }

    let mut cands = Vec::new();
    for blocks in &mut blocks_by_r {
        blocks.sort_by_key(|block| block.l);
        for w in blocks.windows(2) {
            let a = w[0];
            let b = w[1];
            if a.u + 1 == b.l {
                let gain =
                    ((a.len + b.len) * (a.len + b.len) - a.len * a.len - b.len * b.len) as i32;
                cands.push((gain, a, b));
            }
        }
    }

    cands.sort_by(|a, b| b.0.cmp(&a.0));
    cands.truncate(CAND_LIMIT);

    let mut beam = vec![BeamNode {
        jobs: Vec::new(),
        used: [0; R],
        gain: 0,
    }];

    for &(gain, a, b) in &cands {
        let job = MergeJob {
            a: Interval {
                r: a.r,
                l: a.l,
                u: a.u,
            },
            b: Interval {
                r: b.r,
                l: b.l,
                u: b.u,
            },
        };
        let bits = interval_bits(a, b);
        let mut next = beam.clone();
        for node in &beam {
            if node.jobs.len() >= MAX_ACTIVE_JOBS || (node.used[a.r] & bits) != 0 {
                continue;
            }
            let mut added = node.clone();
            added.jobs.push(job);
            added.used[a.r] |= bits;
            added.gain += gain;
            next.push(added);
        }

        next.sort_by_key(|node| -((node.jobs.len() as i32) * 20_000 + node.gain * 100));
        next.truncate(BEAM_WIDTH);
        beam = next;
    }

    let mut best_jobs = Vec::new();
    let mut best_score = i32::MIN;
    for node in beam {
        if node.jobs.is_empty() {
            continue;
        }
        if let Some(score) = simulate_subset_score(state, &node.jobs, node.gain) {
            if score > best_score {
                best_score = score;
                best_jobs = node.jobs;
            }
        }
    }
    best_jobs
}

fn protected_lines(state: &State, jobs: &[MergeJob]) -> ([bool; R], [bool; R], Vec<Interval>) {
    let mut avoid_dep = [false; R];
    let mut avoid_sid = [false; R];
    let mut protected = Vec::with_capacity(jobs.len() * 2);
    for &job in jobs {
        for iv in [job.a, job.b] {
            protected.push(iv);
            if let Some(block) = find_block(state, iv) {
                if block.area == AREA_DEP {
                    avoid_dep[block.line] = true;
                } else {
                    avoid_sid[block.line] = true;
                }
            }
        }
    }
    (avoid_dep, avoid_sid, protected)
}

fn execute_wave(state: &mut State, output: &mut Output, mut jobs: Vec<MergeJob>) -> bool {
    let mut completed_any = false;
    for _ in 0..120 {
        let before_len = jobs.len();
        jobs.retain(|&job| !interval_completed(state, merged_interval(job)));
        if jobs.len() < before_len {
            completed_any = true;
        }
        if jobs.is_empty() {
            return completed_any;
        }

        let (avoid_dep, avoid_sid, protected) = protected_lines(state, &jobs);

        let mut candidates = Vec::new();
        for &job in &jobs {
            if let Some(Step::Move(mv)) =
                next_job_step(state, job, &avoid_dep, &avoid_sid, &protected)
            {
                if !candidates.iter().any(|&x| x == mv) {
                    candidates.push(mv);
                }
            }
        }

        if candidates.is_empty() {
            return completed_any;
        }
        candidates.sort_by_key(|&mv| -move_weight(state, mv));
        candidates.truncate(R);

        let turn = select_non_crossing_moves(state, &candidates);
        if turn.is_empty() {
            return completed_any;
        }
        for &mv in &turn {
            state.apply_move(mv);
        }
        output.push_turn(turn);
    }
    completed_any
}

fn completed(state: &State) -> [bool; R] {
    let mut done = [false; R];
    for block in collect_blocks(state) {
        if block.l == 0 && block.u == INIT_LEN - 1 {
            done[block.r] = true;
        }
    }
    done
}

fn evacuate_dep_to_sid(state: &mut State, output: &mut Output) {
    for i in 0..R {
        while state.dep_len(i) > 0 {
            let block = dep_tail_block(state, i).unwrap();
            let j = choose_sid_for_block(state, block, None).unwrap();
            let mv = Move::dep_to_siding(i, j, block.len);
            state.apply_move(mv);
            output.push_move(mv);
        }
    }
}

fn deliver_sid_to_dep(state: &mut State, output: &mut Output) {
    let mut filled = [false; R];
    let mut remaining = R;
    while remaining > 0 {
        let mut progressed = false;
        for j in 0..R {
            if state.sid_len(j) == 0 {
                continue;
            }
            let block = sid_head_block(state, j).unwrap();
            if block.l == 0
                && block.u == INIT_LEN - 1
                && !filled[block.r]
                && state.dep_len(block.r) == 0
            {
                let mv = Move::siding_to_dep(block.r, j, block.len);
                state.apply_move(mv);
                output.push_move(mv);
                filled[block.r] = true;
                remaining -= 1;
                progressed = true;
            }
        }
        assert!(progressed);
    }
}

fn solve(input: &Input) -> Output {
    let mut state = State::new(input);
    let mut output = Output::new();
    let debug = std::env::var_os("V410_DEBUG").is_some();

    execute_dp_improvements(&mut state, &mut output);

    while completed(&state).iter().any(|&done| !done) {
        let jobs = choose_wave_jobs(&state);
        if jobs.is_empty() {
            if debug {
                let done = completed(&state).iter().filter(|&&x| x).count();
                eprintln!("no jobs done={done} turns={}", output.turns.len());
                for block in collect_blocks(&state) {
                    eprintln!(
                        "block area={} line={} start={} len={} r={} [{},{}]",
                        block.area, block.line, block.start, block.len, block.r, block.l, block.u
                    );
                }
            }
            panic!("no merge jobs");
        }
        if !execute_wave(&mut state, &mut output, jobs) {
            if debug {
                let done = completed(&state).iter().filter(|&&x| x).count();
                eprintln!("stalled done={done} turns={}", output.turns.len());
                for block in collect_blocks(&state) {
                    eprintln!(
                        "block area={} line={} start={} len={} r={} [{},{}]",
                        block.area, block.line, block.start, block.len, block.r, block.l, block.u
                    );
                }
                let jobs = choose_wave_jobs(&state);
                eprintln!("jobs={}", jobs.len());
                for job in jobs {
                    eprintln!(
                        "job r={} [{},{}]+[{},{}]",
                        job.a.r, job.a.l, job.a.u, job.b.l, job.b.u
                    );
                }
            }
            panic!("parallel wave stalled");
        }
        execute_dp_improvements(&mut state, &mut output);
    }

    evacuate_dep_to_sid(&mut state, &mut output);
    deliver_sid_to_dep(&mut state, &mut output);
    output.compact_turns();

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
