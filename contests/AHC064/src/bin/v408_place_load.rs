// v408_place_load.rs
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;

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

#[derive(Debug, Clone, Copy)]
struct CandidateMove {
    mv: Move,
    weight: i32,
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
    dep: [Vec<usize>; R],
    sid: [Vec<usize>; R],
}

impl State {
    fn new(input: &Input) -> Self {
        let dep = std::array::from_fn(|r| input.initial[r].to_vec());
        let sid = std::array::from_fn(|_| Vec::new());
        Self { dep, sid }
    }

    fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            let len = self.dep[mv.i].len();
            let block = self.dep[mv.i].split_off(len - mv.k);
            let old = std::mem::take(&mut self.sid[mv.j]);
            self.sid[mv.j] = block.into_iter().chain(old).collect();
        } else {
            let moved: Vec<usize> = self.sid[mv.j].drain(0..mv.k).collect();
            self.dep[mv.i].extend(moved);
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

fn scan_line(cars: &[usize], area: usize, line: usize, blocks: &mut Vec<Block>) {
    let mut start = 0;
    while start < cars.len() {
        let r = target_line(cars[start]);
        let l = target_pos(cars[start]);
        let mut end = start + 1;
        while end < cars.len() && target_line(cars[end]) == r && cars[end] == cars[end - 1] + 1 {
            end += 1;
        }
        blocks.push(Block {
            area,
            line,
            start,
            len: end - start,
            r,
            l,
            u: target_pos(cars[end - 1]),
        });
        start = end;
    }
}

fn collect_blocks(state: &State) -> Vec<Block> {
    let mut blocks = Vec::with_capacity(100);
    for i in 0..R {
        scan_line(&state.dep[i], AREA_DEP, i, &mut blocks);
    }
    for j in 0..R {
        scan_line(&state.sid[j], AREA_SID, j, &mut blocks);
    }
    blocks
}

fn find_block(state: &State, iv: Interval) -> Option<Block> {
    collect_blocks(state)
        .into_iter()
        .find(|block| block.contains(iv))
}

fn interval_completed(state: &State, iv: Interval) -> bool {
    find_block(state, iv).is_some()
}

fn is_completed_full_block(block: Block) -> bool {
    block.l == 0 && block.u == INIT_LEN - 1 && block.len == INIT_LEN
}

fn dep_tail_block(state: &State, i: usize) -> Option<Block> {
    let mut blocks = Vec::new();
    scan_line(&state.dep[i], AREA_DEP, i, &mut blocks);
    blocks.pop()
}

fn sid_head_block(state: &State, j: usize) -> Option<Block> {
    let mut blocks = Vec::new();
    scan_line(&state.sid[j], AREA_SID, j, &mut blocks);
    blocks.into_iter().next()
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
        if avoid_sid[j] || state.sid[j].len() + block.len > SIDING_CAP {
            continue;
        }

        let mut score = (SIDING_CAP - state.sid[j].len()) as i32;
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

fn block_contains_other_protected(block: Block, protected: &[Interval], allowed: Interval) -> bool {
    protected
        .iter()
        .any(|&iv| iv != allowed && block.contains(iv))
}

fn is_valid_move(state: &State, mv: Move) -> bool {
    if mv.kind == MOVE_DEP_TO_SIDING {
        mv.k > 0 && state.dep[mv.i].len() >= mv.k && state.sid[mv.j].len() + mv.k <= SIDING_CAP
    } else {
        mv.k > 0 && state.sid[mv.j].len() >= mv.k && state.dep[mv.i].len() + mv.k <= DEP_CAP
    }
}

fn moved_completed_full(state: &State, mv: Move) -> Option<usize> {
    let cars = if mv.kind == MOVE_DEP_TO_SIDING {
        let len = state.dep[mv.i].len();
        &state.dep[mv.i][len - mv.k..]
    } else {
        &state.sid[mv.j][..mv.k]
    };
    if cars.len() != INIT_LEN || target_pos(cars[0]) != 0 {
        return None;
    }
    let r = target_line(cars[0]);
    for w in cars.windows(2) {
        if w[1] != w[0] + 1 || target_line(w[1]) != r {
            return None;
        }
    }
    Some(r)
}

fn staging_dep_line(iv: Interval) -> usize {
    if iv.l == 0 {
        iv.r
    } else {
        (iv.r + iv.l) % R
    }
}

fn staging_sid_line(iv: Interval) -> usize {
    (iv.r + iv.l) % R
}

fn push_candidate(candidates: &mut Vec<CandidateMove>, state: &State, mv: Move, bonus: i32) {
    if !is_valid_move(state, mv) {
        return;
    }
    let mut weight = move_weight(state, mv) + bonus;
    if let Some(r) = moved_completed_full(state, mv) {
        if mv.kind == MOVE_SIDING_TO_DEP && mv.i == r && state.dep[r].is_empty() {
            weight += 7_000;
        } else {
            weight -= 7_000;
        }
    }
    weight = weight.max(1);
    if let Some(pos) = candidates.iter().position(|cand| cand.mv == mv) {
        if candidates[pos].weight < weight {
            candidates[pos].weight = weight;
        }
    } else {
        candidates.push(CandidateMove { mv, weight });
    }
}

fn push_dep_to_sidings(
    state: &State,
    candidates: &mut Vec<CandidateMove>,
    i: usize,
    block: Block,
    avoid_sid: &[bool; R],
    preferred_sid: Option<usize>,
) {
    for j in 0..R {
        if avoid_sid[j] {
            continue;
        }
        let mut bonus = 0;
        if preferred_sid == Some(j) {
            bonus += 2_200;
        }
        bonus += ((SIDING_CAP - state.sid[j].len()) as i32) * 70;
        if state.sid[j].is_empty() {
            bonus += 250;
        }
        if i.abs_diff(j) <= 1 {
            bonus += 120;
        }
        push_candidate(
            candidates,
            state,
            Move::dep_to_siding(i, j, block.len),
            bonus,
        );
    }
}

fn push_siding_to_deps(
    state: &State,
    candidates: &mut Vec<CandidateMove>,
    j: usize,
    block: Block,
    avoid_dep: &[bool; R],
    preferred_dep: Option<usize>,
) {
    for i in 0..R {
        if avoid_dep[i] {
            continue;
        }
        let mut bonus = 0;
        if preferred_dep == Some(i) {
            bonus += 2_200;
        }
        bonus += ((DEP_CAP - state.dep[i].len()) as i32) * 70;
        if state.dep[i].is_empty() {
            bonus += 250;
        }
        if i.abs_diff(j) <= 1 {
            bonus += 120;
        }
        push_candidate(
            candidates,
            state,
            Move::siding_to_dep(i, j, block.len),
            bonus,
        );
    }
}

fn expose_dep_tail_candidates(
    state: &State,
    iv: Interval,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Option<(bool, Vec<CandidateMove>)> {
    let block = find_block(state, iv)?;
    let mut candidates = Vec::new();
    if block.area == AREA_DEP {
        if block.end_pos() == state.dep[block.line].len() {
            return Some((true, candidates));
        }
        let tail = dep_tail_block(state, block.line)?;
        if block_contains_other_protected(tail, protected, iv) {
            return Some((false, candidates));
        }
        push_dep_to_sidings(state, &mut candidates, block.line, tail, avoid_sid, None);
    } else if block.start == 0 {
        push_siding_to_deps(
            state,
            &mut candidates,
            block.line,
            block,
            avoid_dep,
            Some(staging_dep_line(iv)),
        );
    } else {
        let head = sid_head_block(state, block.line)?;
        if block_contains_other_protected(head, protected, iv) {
            return Some((false, candidates));
        }
        push_siding_to_deps(state, &mut candidates, block.line, head, avoid_dep, None);
    }
    Some((false, candidates))
}

fn expose_sid_head_candidates(
    state: &State,
    iv: Interval,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Option<(bool, Vec<CandidateMove>)> {
    let block = find_block(state, iv)?;
    let mut candidates = Vec::new();
    if block.area == AREA_SID {
        if block.start == 0 {
            return Some((true, candidates));
        }
        let head = sid_head_block(state, block.line)?;
        if block_contains_other_protected(head, protected, iv) {
            return Some((false, candidates));
        }
        push_siding_to_deps(state, &mut candidates, block.line, head, avoid_dep, None);
    } else if block.end_pos() == state.dep[block.line].len() {
        push_dep_to_sidings(
            state,
            &mut candidates,
            block.line,
            block,
            avoid_sid,
            Some(staging_sid_line(iv)),
        );
    } else {
        let tail = dep_tail_block(state, block.line)?;
        if block_contains_other_protected(tail, protected, iv) {
            return Some((false, candidates));
        }
        push_dep_to_sidings(state, &mut candidates, block.line, tail, avoid_sid, None);
    }
    Some((false, candidates))
}

fn merged_interval(job: MergeJob) -> Interval {
    Interval {
        r: job.a.r,
        l: job.a.l,
        u: job.b.u,
    }
}

fn job_step_candidates(
    state: &State,
    job: MergeJob,
    avoid_dep: &[bool; R],
    avoid_sid: &[bool; R],
    protected: &[Interval],
) -> Vec<CandidateMove> {
    if interval_completed(state, merged_interval(job)) {
        return Vec::new();
    }

    if let (Some(a_probe), Some(b_probe)) = (find_block(state, job.a), find_block(state, job.b)) {
        if a_probe.area == AREA_SID
            && b_probe.area == AREA_SID
            && a_probe.line == b_probe.line
            && b_probe.end_pos() <= a_probe.start
        {
            let mut candidates = Vec::new();
            let head = sid_head_block(state, a_probe.line).unwrap();
            push_siding_to_deps(state, &mut candidates, a_probe.line, head, avoid_dep, None);
            return candidates;
        }
    }

    let Some((a_ready, a_moves)) =
        expose_dep_tail_candidates(state, job.a, avoid_dep, avoid_sid, protected)
    else {
        return Vec::new();
    };
    if !a_ready {
        return a_moves;
    }

    let Some(a_block) = find_block(state, job.a) else {
        return Vec::new();
    };
    if a_block.area != AREA_DEP || a_block.end_pos() != state.dep[a_block.line].len() {
        return Vec::new();
    }

    if let Some(b_probe) = find_block(state, job.b) {
        if b_probe.area == AREA_DEP
            && b_probe.line == a_block.line
            && b_probe.end_pos() <= a_block.start
        {
            let mut candidates = Vec::new();
            push_dep_to_sidings(
                state,
                &mut candidates,
                a_block.line,
                a_block,
                avoid_sid,
                Some(staging_sid_line(job.a)),
            );
            return candidates;
        }
    }

    let mut avoid_dep_for_b = *avoid_dep;
    avoid_dep_for_b[a_block.line] = true;
    let Some((b_ready, b_moves)) =
        expose_sid_head_candidates(state, job.b, &avoid_dep_for_b, avoid_sid, protected)
    else {
        return Vec::new();
    };
    if !b_ready {
        return b_moves;
    }

    let Some(b_block) = find_block(state, job.b) else {
        return Vec::new();
    };
    if b_block.area != AREA_SID || b_block.start != 0 || a_block.r != b_block.r {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let prefer_dep = job.a.l == 0 && (job.b.u != INIT_LEN - 1 || a_block.line == job.a.r);
    if prefer_dep {
        push_candidate(
            &mut candidates,
            state,
            Move::siding_to_dep(a_block.line, b_block.line, b_block.len),
            3_000,
        );
        push_candidate(
            &mut candidates,
            state,
            Move::dep_to_siding(a_block.line, b_block.line, a_block.len),
            400,
        );
    } else {
        push_candidate(
            &mut candidates,
            state,
            Move::dep_to_siding(a_block.line, b_block.line, a_block.len),
            3_000,
        );
        push_candidate(
            &mut candidates,
            state,
            Move::siding_to_dep(a_block.line, b_block.line, b_block.len),
            400,
        );
    }
    candidates
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

fn select_non_crossing_moves(state: &State, candidates: &[CandidateMove]) -> Vec<Move> {
    let mut edge: [[Option<(i32, Move)>; R]; R] = [[None; R]; R];
    for &cand in candidates {
        if !is_valid_move(state, cand.mv) {
            continue;
        }
        match edge[cand.mv.i][cand.mv.j] {
            Some((old, _)) if old >= cand.weight => {}
            _ => edge[cand.mv.i][cand.mv.j] = Some((cand.weight, cand.mv)),
        }
    }

    let mut dp = [[0i32; R + 1]; R + 1];
    for i in (0..R).rev() {
        for j in (0..R).rev() {
            let mut best = dp[i + 1][j].max(dp[i][j + 1]);
            if let Some((weight, _)) = edge[i][j] {
                best = best.max(weight + dp[i + 1][j + 1]);
            }
            dp[i][j] = best;
        }
    }

    let mut selected = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < R && j < R {
        if let Some((weight, mv)) = edge[i][j] {
            if dp[i][j] == weight + dp[i + 1][j + 1] {
                selected.push(mv);
                i += 1;
                j += 1;
                continue;
            }
        }
        if dp[i][j] == dp[i + 1][j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    selected
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
            candidates.extend(job_step_candidates(
                &st, job, &avoid_dep, &avoid_sid, &protected,
            ));
        }
        if candidates.is_empty() {
            break;
        }
        candidates.sort_by_key(|cand| -cand.weight);
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
    const MAX_ACTIVE_JOBS: usize = 9;
    const CAND_LIMIT: usize = 36;
    const BEAM_WIDTH: usize = 128;
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
            candidates.extend(job_step_candidates(
                state, job, &avoid_dep, &avoid_sid, &protected,
            ));
        }

        if candidates.is_empty() {
            return completed_any;
        }
        candidates.sort_by_key(|cand| -cand.weight);

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
        if is_completed_full_block(block) {
            done[block.r] = true;
        }
    }
    done
}

fn place_completed_blocks(state: &mut State, output: &mut Output) {
    let mut filled = [false; R];
    for i in 0..R {
        if let Some(block) = dep_tail_block(state, i) {
            if state.dep[i].len() == INIT_LEN
                && block.l == 0
                && block.u == INIT_LEN - 1
                && block.r == i
            {
                filled[i] = true;
            }
        }
    }

    for i in 0..R {
        while !state.dep[i].is_empty() && !filled[i] {
            let block = dep_tail_block(state, i).unwrap();
            let j = choose_sid_for_block(state, block, None).unwrap();
            let mv = Move::dep_to_siding(i, j, block.len);
            state.apply_move(mv);
            output.push_move(mv);
        }
    }

    let mut remaining = filled.iter().filter(|&&x| !x).count();
    while remaining > 0 {
        let mut progressed = false;
        for j in 0..R {
            if state.sid[j].is_empty() {
                continue;
            }
            let block = sid_head_block(state, j).unwrap();
            if block.l == 0
                && block.u == INIT_LEN - 1
                && !filled[block.r]
                && state.dep[block.r].is_empty()
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
    let debug = std::env::var_os("V408_DEBUG").is_some();

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
    }

    place_completed_blocks(&mut state, &mut output);
    output.compact_turns();

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
