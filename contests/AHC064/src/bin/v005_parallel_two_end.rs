// v005_parallel_two_end.rs
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const CAR_COUNT: usize = R * INIT_LEN;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

type CarId = usize;
type LineIdx = usize;
type PosIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CarPos {
    r: LineIdx,
    c: PosIdx,
}

#[derive(Debug, Clone)]
struct Input {
    initial: [[CarId; INIT_LEN]; R],
    initial_pos: [CarPos; CAR_COUNT],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        Self::from_str(&s)
    }

    fn from_str(s: &str) -> Self {
        let mut it = s.split_whitespace();
        let r_count = it.next().unwrap().parse::<usize>().unwrap();
        assert_eq!(r_count, R);

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
    fn target_id(r: usize, c: usize) -> CarId {
        r * INIT_LEN + c
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Move {
    kind: usize,
    i: LineIdx,
    j: LineIdx,
    k: usize,
}

impl Move {
    #[inline(always)]
    fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    #[inline(always)]
    fn siding_to_dep(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_SIDING_TO_DEP,
            i,
            j,
            k,
        }
    }
}

#[derive(Debug, Clone)]
struct Output {
    turns: Vec<Vec<Move>>,
}

impl Output {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            turns: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    fn push_turn(&mut self, moves: Vec<Move>) {
        self.turns.push(moves);
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

#[derive(Debug, Clone)]
struct State {
    dep: Vec<Vec<CarId>>,
    sid: Vec<Vec<CarId>>,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut dep = vec![Vec::with_capacity(DEP_CAP); R];
        let sid = vec![Vec::with_capacity(SIDING_CAP); R];

        for r in 0..R {
            dep[r].extend_from_slice(&input.initial[r]);
        }

        Self { dep, sid }
    }

    fn apply_move(&mut self, mv: Move) {
        assert!(mv.i < R);
        assert!(mv.j < R);
        assert!(mv.k >= 1);

        if mv.kind == MOVE_DEP_TO_SIDING {
            let old_dep_len = self.dep[mv.i].len();
            assert!(mv.k <= old_dep_len);
            assert!(self.sid[mv.j].len() + mv.k <= SIDING_CAP);

            let block = self.dep[mv.i].split_off(old_dep_len - mv.k);
            self.sid[mv.j].splice(0..0, block);
        } else {
            assert_eq!(mv.kind, MOVE_SIDING_TO_DEP);
            assert!(mv.k <= self.sid[mv.j].len());
            assert!(self.dep[mv.i].len() + mv.k <= DEP_CAP);

            let block: Vec<_> = self.sid[mv.j].drain(0..mv.k).collect();
            self.dep[mv.i].extend(block);
        }
    }

    fn apply_turn(&mut self, moves: &[Move]) {
        debug_assert!(is_non_crossing(moves));
        for &mv in moves {
            self.apply_move(mv);
        }
    }

    fn locate_dep(&self, target: CarId) -> (LineIdx, PosIdx) {
        for i in 0..R {
            if let Some(pos) = self.dep[i].iter().position(|&car| car == target) {
                return (i, pos);
            }
        }
        panic!("target car is not in departure lines: {}", target);
    }

    fn is_complete(&self) -> bool {
        for r in 0..R {
            if self.dep[r].len() != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] != Input::target_id(r, c) {
                    return false;
                }
            }
            if !self.sid[r].is_empty() {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetInfo {
    r: LineIdx,
    source: LineIdx,
    pos: PosIdx,
    blockers: usize,
    block_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TmpPair {
    r: LineIdx,
    source: LineIdx,
    tmp: LineIdx,
    k: usize,
}

#[derive(Debug, Clone)]
struct GroupPlan {
    targets: Vec<TargetInfo>,
    tmp_pairs: Vec<TmpPair>,
    cost: usize,
    progress: usize,
    blocker_sum: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrontCandidate {
    r: LineIdx,
    tmp: LineIdx,
    k: usize,
}

fn is_non_crossing(moves: &[Move]) -> bool {
    let mut used_dep = [false; R];
    let mut used_sid = [false; R];
    let mut pairs = Vec::with_capacity(moves.len());

    for mv in moves {
        if used_dep[mv.i] || used_sid[mv.j] {
            return false;
        }
        used_dep[mv.i] = true;
        used_sid[mv.j] = true;
        pairs.push((mv.i, mv.j));
    }

    pairs.sort_unstable();
    for w in pairs.windows(2) {
        if w[0].1 >= w[1].1 {
            return false;
        }
    }
    true
}

fn emit_turn(output: &mut Output, state: &mut State, mut moves: Vec<Move>) {
    moves.sort_unstable_by_key(|mv| (mv.i, mv.j));
    debug_assert!(!moves.is_empty());
    debug_assert!(is_non_crossing(&moves));
    state.apply_turn(&moves);
    output.push_turn(moves);
}

fn normalize_prefix(state: &State, front_pos: &mut [usize; R], back_pos: &[i32; R]) {
    for r in 0..R {
        while (front_pos[r] as i32) <= back_pos[r]
            && front_pos[r] < state.dep[r].len()
            && state.dep[r][front_pos[r]] == Input::target_id(r, front_pos[r])
        {
            front_pos[r] += 1;
        }
    }
}

fn collect_target_info(
    state: &State,
    front_pos: &[usize; R],
    back_pos: &[i32; R],
    active: &[bool; R],
) -> [Option<TargetInfo>; R] {
    let mut info = [None; R];

    for r in 0..R {
        if !active[r] {
            continue;
        }

        let c = back_pos[r] as usize;
        let target = Input::target_id(r, c);
        let (source, pos) = state.locate_dep(target);
        let blockers = state.dep[source].len() - pos - 1;

        let mut block_len = 1;
        while block_len <= pos && c >= front_pos[r] + block_len {
            let prev_target = Input::target_id(r, c - block_len);
            if state.dep[source][pos - block_len] != prev_target {
                break;
            }
            block_len += 1;
        }

        info[r] = Some(TargetInfo {
            r,
            source,
            pos,
            blockers,
            block_len,
        });
    }

    info
}

fn make_group_plan(
    state: &State,
    info: &[Option<TargetInfo>; R],
    mask: usize,
) -> Option<GroupPlan> {
    let mut source_used = [false; R];
    let mut targets = Vec::new();
    let mut target_pairs = Vec::new();
    let mut blocker_items = Vec::new();
    let mut progress = 0;
    let mut blocker_sum = 0;

    for (r, item) in info.iter().enumerate() {
        if (mask >> r) & 1 == 0 {
            continue;
        }

        let target = item.as_ref().copied()?;
        if source_used[target.source] {
            return None;
        }
        source_used[target.source] = true;

        target_pairs.push((target.source, r));
        if target.blockers > 0 {
            blocker_items.push((target.source, r, target.blockers));
            blocker_sum += target.blockers;
        }
        progress += target.block_len;
        targets.push(target);
    }

    target_pairs.sort_unstable();
    for w in target_pairs.windows(2) {
        if w[0].1 >= w[1].1 {
            return None;
        }
    }

    let mut outside = Vec::new();
    for j in 0..R {
        if (mask >> j) & 1 == 0 {
            outside.push(j);
        }
    }

    blocker_items.sort_unstable();
    if blocker_items.len() > outside.len() {
        return None;
    }

    let mut tmp_pairs = Vec::new();
    let mut outside_idx = 0;
    let mut last_tmp = None;

    for (source, r, k) in blocker_items {
        let mut found = None;
        while outside_idx < outside.len() {
            let tmp = outside[outside_idx];
            outside_idx += 1;
            if last_tmp.is_none_or(|last| last < tmp) && state.sid[tmp].len() + k <= SIDING_CAP {
                found = Some(tmp);
                break;
            }
        }

        let tmp = found?;
        last_tmp = Some(tmp);
        tmp_pairs.push(TmpPair { r, source, tmp, k });
    }

    let cost = if tmp_pairs.is_empty() { 1 } else { 3 };

    Some(GroupPlan {
        targets,
        tmp_pairs,
        cost,
        progress,
        blocker_sum,
    })
}

fn better_group(a: &GroupPlan, b: &GroupPlan) -> bool {
    let a_key = (
        a.progress * 100 / a.cost,
        a.progress,
        a.targets.len(),
        3 - a.cost,
        1000 - a.blocker_sum,
    );
    let b_key = (
        b.progress * 100 / b.cost,
        b.progress,
        b.targets.len(),
        3 - b.cost,
        1000 - b.blocker_sum,
    );
    a_key > b_key
}

fn best_group(state: &State, info: &[Option<TargetInfo>; R], active_mask: usize) -> GroupPlan {
    let mut best = None;
    let mut sub = active_mask;

    while sub > 0 {
        if let Some(plan) = make_group_plan(state, info, sub) {
            if best
                .as_ref()
                .is_none_or(|current| better_group(&plan, current))
            {
                best = Some(plan);
            }
        }
        sub = (sub - 1) & active_mask;
    }

    best.unwrap()
}

fn front_block_len(
    state: &State,
    tmp: usize,
    inserted_remaining: usize,
    front_pos: &[usize; R],
    back_pos: &[i32; R],
) -> Option<FrontCandidate> {
    if inserted_remaining == 0 || state.sid[tmp].is_empty() {
        return None;
    }

    let car = state.sid[tmp][0];
    let r = car / INIT_LEN;
    let c = car % INIT_LEN;
    if (front_pos[r] as i32) > back_pos[r] || c != front_pos[r] {
        return None;
    }
    if state.dep[r].len() != front_pos[r] {
        return None;
    }

    let limit = inserted_remaining.min(state.sid[tmp].len());
    let mut k = 1;
    while k < limit
        && (front_pos[r] + k) as i32 <= back_pos[r]
        && state.sid[tmp][k] == Input::target_id(r, front_pos[r] + k)
    {
        k += 1;
    }

    if state.dep[r].len() + k <= DEP_CAP {
        Some(FrontCandidate { r, tmp, k })
    } else {
        None
    }
}

fn best_front_moves(
    state: &State,
    front_pos: &[usize; R],
    back_pos: &[i32; R],
    tmp_pairs: &[TmpPair],
    inserted_remaining: &[usize; R],
) -> Vec<FrontCandidate> {
    let mut candidates = Vec::new();
    for pair in tmp_pairs {
        if let Some(candidate) = front_block_len(
            state,
            pair.tmp,
            inserted_remaining[pair.tmp],
            front_pos,
            back_pos,
        ) {
            candidates.push(candidate);
        }
    }

    let n = candidates.len();
    let mut best_mask = 0usize;
    let mut best_progress = 0usize;
    for mask in 1..(1usize << n) {
        let mut moves = Vec::new();
        let mut progress = 0;
        for (idx, candidate) in candidates.iter().enumerate() {
            if (mask >> idx) & 1 == 1 {
                moves.push(Move::siding_to_dep(candidate.r, candidate.tmp, candidate.k));
                progress += candidate.k;
            }
        }
        if progress > best_progress && is_non_crossing(&moves) {
            best_progress = progress;
            best_mask = mask;
        }
    }

    let mut selected = Vec::new();
    for (idx, &candidate) in candidates.iter().enumerate() {
        if (best_mask >> idx) & 1 == 1 {
            selected.push(candidate);
        }
    }
    selected
}

fn apply_group(
    output: &mut Output,
    state: &mut State,
    front_pos: &mut [usize; R],
    back_pos: &mut [i32; R],
    active: &mut [bool; R],
    plan: GroupPlan,
) {
    let mut inserted_remaining = [0usize; R];

    if !plan.tmp_pairs.is_empty() {
        let moves = plan
            .tmp_pairs
            .iter()
            .map(|pair| {
                inserted_remaining[pair.tmp] = pair.k;
                Move::dep_to_siding(pair.source, pair.tmp, pair.k)
            })
            .collect::<Vec<_>>();
        emit_turn(output, state, moves);
    }

    let target_moves = plan
        .targets
        .iter()
        .map(|target| Move::dep_to_siding(target.source, target.r, target.block_len))
        .collect::<Vec<_>>();
    emit_turn(output, state, target_moves);

    for target in &plan.targets {
        back_pos[target.r] -= target.block_len as i32;
    }

    normalize_prefix(state, front_pos, back_pos);

    if !plan.tmp_pairs.is_empty() {
        let front_moves = best_front_moves(
            state,
            front_pos,
            back_pos,
            &plan.tmp_pairs,
            &inserted_remaining,
        );
        if !front_moves.is_empty() {
            let moves = front_moves
                .iter()
                .map(|candidate| Move::siding_to_dep(candidate.r, candidate.tmp, candidate.k))
                .collect::<Vec<_>>();
            emit_turn(output, state, moves);

            for candidate in front_moves {
                front_pos[candidate.r] += candidate.k;
                inserted_remaining[candidate.tmp] -= candidate.k;
            }
            normalize_prefix(state, front_pos, back_pos);
        }

        let moves = plan
            .tmp_pairs
            .iter()
            .filter_map(|pair| {
                let k = inserted_remaining[pair.tmp];
                if k == 0 {
                    None
                } else {
                    Some(Move::siding_to_dep(pair.source, pair.tmp, k))
                }
            })
            .collect::<Vec<_>>();
        if !moves.is_empty() {
            emit_turn(output, state, moves);
        }
    }

    normalize_prefix(state, front_pos, back_pos);
    for r in 0..R {
        active[r] = (front_pos[r] as i32) <= back_pos[r];
    }
}

fn solve(input: &Input) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(140);
    let mut front_pos = [0usize; R];
    let mut back_pos = [(INIT_LEN as i32) - 1; R];
    let mut active = [true; R];

    normalize_prefix(&state, &mut front_pos, &back_pos);
    for r in 0..R {
        active[r] = (front_pos[r] as i32) <= back_pos[r];
    }

    while active.iter().any(|&is_active| is_active) {
        let info = collect_target_info(&state, &front_pos, &back_pos, &active);
        let mut active_mask = 0usize;
        for (r, &is_active) in active.iter().enumerate() {
            if is_active {
                active_mask |= 1usize << r;
            }
        }

        let plan = best_group(&state, &info, active_mask);
        apply_group(
            &mut output,
            &mut state,
            &mut front_pos,
            &mut back_pos,
            &mut active,
            plan,
        );
    }

    let mut finish_moves = Vec::new();
    for (r, &front) in front_pos.iter().enumerate() {
        debug_assert_eq!(state.dep[r].len(), front);
        debug_assert_eq!(
            state.sid[r],
            (front..INIT_LEN)
                .map(|c| Input::target_id(r, c))
                .collect::<Vec<_>>()
        );
        if !state.sid[r].is_empty() {
            finish_moves.push(Move::siding_to_dep(r, r, state.sid[r].len()));
        }
    }
    if !finish_moves.is_empty() {
        emit_turn(&mut output, &mut state, finish_moves);
    }

    assert!(state.is_complete());
    assert!(output.turns.len() <= MAX_TURNS);

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
