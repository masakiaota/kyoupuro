// v003_direct_sid.rs
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

fn collect_target_info(state: &State, next_pos: &[i32; R]) -> [Option<TargetInfo>; R] {
    let mut info = [None; R];

    for r in 0..R {
        if next_pos[r] < 0 {
            continue;
        }

        let c = next_pos[r] as usize;
        let target = Input::target_id(r, c);
        let (source, pos) = state.locate_dep(target);
        let blockers = state.dep[source].len() - pos - 1;

        let mut block_len = 1;
        while block_len <= pos && c >= block_len {
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

fn apply_group(output: &mut Output, state: &mut State, next_pos: &mut [i32; R], plan: GroupPlan) {
    if !plan.tmp_pairs.is_empty() {
        let moves = plan
            .tmp_pairs
            .iter()
            .map(|pair| Move::dep_to_siding(pair.source, pair.tmp, pair.k))
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
        next_pos[target.r] -= target.block_len as i32;
    }

    if !plan.tmp_pairs.is_empty() {
        let moves = plan
            .tmp_pairs
            .iter()
            .map(|pair| Move::siding_to_dep(pair.source, pair.tmp, pair.k))
            .collect::<Vec<_>>();
        emit_turn(output, state, moves);
    }
}

fn solve(input: &Input) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(140);
    let mut next_pos = [(INIT_LEN as i32) - 1; R];

    while next_pos.iter().any(|&c| c >= 0) {
        let info = collect_target_info(&state, &next_pos);
        let mut active_mask = 0usize;
        for (r, &c) in next_pos.iter().enumerate() {
            if c >= 0 {
                active_mask |= 1usize << r;
            }
        }

        let plan = best_group(&state, &info, active_mask);
        apply_group(&mut output, &mut state, &mut next_pos, plan);
    }

    let mut finish_moves = Vec::new();
    for r in 0..R {
        debug_assert_eq!(
            state.sid[r],
            (0..INIT_LEN)
                .map(|c| Input::target_id(r, c))
                .collect::<Vec<_>>()
        );
        finish_moves.push(Move::siding_to_dep(r, r, INIT_LEN));
    }
    emit_turn(&mut output, &mut state, finish_moves);

    assert!(state.is_complete());
    assert!(output.turns.len() <= MAX_TURNS);

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
