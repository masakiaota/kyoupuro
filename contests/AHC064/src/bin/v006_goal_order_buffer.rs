// v006_goal_order_buffer.rs
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const CAR_COUNT: usize = R * INIT_LEN;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

const TEMP_SIDING_SOFT_CAP: usize = 13;

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

    #[inline(always)]
    fn target_line(car: CarId) -> usize {
        car / INIT_LEN
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
    dep: [[u8; DEP_CAP]; R],
    dep_len: [u8; R],
    sid: [[u8; SIDING_CAP]; R],
    sid_head: [u8; R],
    sid_len: [u8; R],
}

impl State {
    fn new(input: &Input) -> Self {
        let mut dep = [[0; DEP_CAP]; R];
        let dep_len = [INIT_LEN as u8; R];
        let sid = [[0; SIDING_CAP]; R];
        let sid_head = [0; R];
        let sid_len = [0; R];

        for r in 0..R {
            for c in 0..INIT_LEN {
                dep[r][c] = input.initial[r][c] as u8;
            }
        }

        Self {
            dep,
            dep_len,
            sid,
            sid_head,
            sid_len,
        }
    }

    #[inline(always)]
    fn dep_len(&self, i: usize) -> usize {
        self.dep_len[i] as usize
    }

    #[inline(always)]
    fn sid_len(&self, j: usize) -> usize {
        self.sid_len[j] as usize
    }

    #[inline(always)]
    fn has_dep_cars(&self) -> bool {
        self.dep_len.iter().any(|&len| len > 0)
    }

    #[inline(always)]
    fn dep_car(&self, i: usize, pos: usize) -> CarId {
        self.dep[i][pos] as usize
    }

    #[inline(always)]
    fn dep_last(&self, i: usize) -> CarId {
        self.dep_car(i, self.dep_len(i) - 1)
    }

    #[inline(always)]
    fn sid_slot(head: usize, offset: usize) -> usize {
        let slot = head + offset;
        if slot >= SIDING_CAP {
            slot - SIDING_CAP
        } else {
            slot
        }
    }

    #[inline(always)]
    fn sid_head_after_push(head: usize, k: usize) -> usize {
        if head >= k {
            head - k
        } else {
            head + SIDING_CAP - k
        }
    }

    #[inline(always)]
    fn sid_car(&self, j: usize, pos: usize) -> CarId {
        let slot = Self::sid_slot(self.sid_head[j] as usize, pos);
        self.sid[j][slot] as usize
    }

    fn locate_sid(&self, target: CarId) -> (LineIdx, PosIdx) {
        for j in 0..R {
            for pos in 0..self.sid_len(j) {
                if self.sid_car(j, pos) == target {
                    return (j, pos);
                }
            }
        }
        panic!("target car {} is not in any siding", target);
    }

    fn suffix_run_len(&self, i: usize) -> usize {
        let len = self.dep_len(i);
        let j = Input::target_line(self.dep_last(i));
        let mut k = 0;
        while k < len {
            let car = self.dep_car(i, len - 1 - k);
            if Input::target_line(car) != j {
                break;
            }
            k += 1;
        }
        k
    }

    #[inline(always)]
    fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            self.move_dep_to_siding(mv.i, mv.j, mv.k);
        } else {
            self.move_siding_to_dep(mv.i, mv.j, mv.k);
        }
    }

    #[inline(always)]
    fn apply_turn(&mut self, moves: &[Move]) {
        for &mv in moves {
            self.apply_move(mv);
        }
    }

    #[inline(always)]
    fn move_dep_to_siding(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len(i);
        let new_dep_len = old_dep_len - k;
        let new_sid_head = Self::sid_head_after_push(self.sid_head[j] as usize, k);

        debug_assert!(k >= 1);
        debug_assert!(k <= old_dep_len);
        debug_assert!(self.sid_len(j) + k <= SIDING_CAP);

        for offset in 0..k {
            let car = self.dep[i][new_dep_len + offset];
            let sid_slot = Self::sid_slot(new_sid_head, offset);
            self.sid[j][sid_slot] = car;
        }

        self.dep_len[i] = new_dep_len as u8;
        self.sid_head[j] = new_sid_head as u8;
        self.sid_len[j] += k as u8;
    }

    #[inline(always)]
    fn move_siding_to_dep(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len(i);
        let old_sid_head = self.sid_head[j] as usize;

        debug_assert!(k >= 1);
        debug_assert!(k <= self.sid_len(j));
        debug_assert!(old_dep_len + k <= DEP_CAP);

        for offset in 0..k {
            let sid_slot = Self::sid_slot(old_sid_head, offset);
            self.dep[i][old_dep_len + offset] = self.sid[j][sid_slot];
        }

        self.dep_len[i] = (old_dep_len + k) as u8;
        self.sid_head[j] = Self::sid_slot(old_sid_head, k) as u8;
        self.sid_len[j] -= k as u8;
    }

    fn is_complete(&self) -> bool {
        for r in 0..R {
            if self.dep_len(r) != INIT_LEN || self.sid_len(r) != 0 {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep_car(r, c) != Input::target_id(r, c) {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy)]
struct BucketCandidate {
    i: LineIdx,
    target: LineIdx,
    k: usize,
}

#[derive(Debug, Clone, Copy)]
struct TargetInfo {
    r: LineIdx,
    source: LineIdx,
    pos: PosIdx,
}

#[derive(Debug, Clone, Copy)]
struct TmpPair {
    source: LineIdx,
    tmp: LineIdx,
    k: usize,
}

#[derive(Debug, Clone)]
struct DeliveryGroupPlan {
    targets: Vec<TargetInfo>,
    tmp_pairs: Vec<TmpPair>,
    cost: usize,
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

fn bucket_candidates(state: &State) -> Vec<BucketCandidate> {
    let mut candidates = Vec::with_capacity(R);

    for i in 0..R {
        if state.dep_len(i) == 0 {
            continue;
        }
        let target = Input::target_line(state.dep_last(i));
        candidates.push(BucketCandidate {
            i,
            target,
            k: state.suffix_run_len(i),
        });
    }

    candidates
}

fn bucket_score(state: &State, cand: BucketCandidate, j: usize) -> Option<i32> {
    if state.sid_len(j) + cand.k > SIDING_CAP {
        return None;
    }

    if j == cand.target {
        return Some(cand.k as i32 * 4000 + 2000);
    }

    if state.sid_len(j) + cand.k > TEMP_SIDING_SOFT_CAP {
        return None;
    }

    let dist = cand.target.abs_diff(j) as i32;
    if cand.k < 4 || dist > 1 {
        return None;
    }

    let score = cand.k as i32 * 700 - dist * 250 - state.sid_len(j) as i32 * 30 - 1800;
    if score > 0 { Some(score) } else { None }
}

fn choose_bucket_moves(state: &State) -> Vec<Move> {
    let candidates = bucket_candidates(state);
    let n = candidates.len();
    debug_assert!(n > 0);

    let mut dp = vec![0usize; n];
    let mut prev = vec![usize::MAX; n];
    let mut best = 0;

    for a in 0..n {
        dp[a] = candidates[a].k;
        for b in 0..a {
            if candidates[b].target < candidates[a].target && dp[b] + candidates[a].k > dp[a] {
                dp[a] = dp[b] + candidates[a].k;
                prev[a] = b;
            }
        }
        if dp[a] > dp[best] {
            best = a;
        }
    }

    let mut base_indices = Vec::new();
    let mut cur = best;
    loop {
        base_indices.push(cur);
        if prev[cur] == usize::MAX {
            break;
        }
        cur = prev[cur];
    }
    base_indices.reverse();

    let mut assigned = [None; R];
    let mut is_base = [false; R];
    for &idx in &base_indices {
        assigned[idx] = Some(candidates[idx].target);
        is_base[idx] = true;
    }

    let mut boundaries = Vec::with_capacity(base_indices.len() + 2);
    boundaries.push((usize::MAX, usize::MAX));
    for &idx in &base_indices {
        boundaries.push((idx, candidates[idx].target));
    }
    boundaries.push((n, R));

    for w in boundaries.windows(2) {
        let (left_idx, left_j) = w[0];
        let (right_idx, right_j) = w[1];
        let start = if left_idx == usize::MAX {
            0
        } else {
            left_idx + 1
        };
        let end = right_idx;
        let gap_min_j = if left_idx == usize::MAX {
            0
        } else {
            left_j + 1
        };
        if start >= end || gap_min_j >= right_j {
            continue;
        }

        fill_bucket_gap(
            state,
            &candidates,
            &is_base,
            &mut assigned,
            start,
            end,
            gap_min_j,
            right_j,
        );
    }

    let mut moves = Vec::new();
    for (idx, cand) in candidates.iter().enumerate() {
        if let Some(j) = assigned[idx] {
            moves.push(Move::dep_to_siding(cand.i, j, cand.k));
        }
    }

    moves
}

fn fill_bucket_gap(
    state: &State,
    candidates: &[BucketCandidate],
    is_base: &[bool; R],
    assigned: &mut [Option<usize>; R],
    start: usize,
    end: usize,
    min_j: usize,
    max_j_exclusive: usize,
) {
    let items = (start..end)
        .filter(|&idx| !is_base[idx])
        .collect::<Vec<_>>();
    if items.is_empty() {
        return;
    }

    let mut memo = [[i32::MIN; R + 1]; R + 1];
    let mut choice = [[usize::MAX; R + 1]; R + 1];

    fn dfs_gap(
        item_pos: usize,
        min_j: usize,
        max_j_exclusive: usize,
        items: &[usize],
        candidates: &[BucketCandidate],
        state: &State,
        memo: &mut [[i32; R + 1]; R + 1],
        choice: &mut [[usize; R + 1]; R + 1],
    ) -> i32 {
        if item_pos == items.len() {
            return 0;
        }
        if memo[item_pos][min_j] != i32::MIN {
            return memo[item_pos][min_j];
        }

        let idx = items[item_pos];
        let mut best = dfs_gap(
            item_pos + 1,
            min_j,
            max_j_exclusive,
            items,
            candidates,
            state,
            memo,
            choice,
        );
        choice[item_pos][min_j] = R;

        for j in min_j..max_j_exclusive {
            if let Some(score) = bucket_score(state, candidates[idx], j) {
                let next = score
                    + dfs_gap(
                        item_pos + 1,
                        j + 1,
                        max_j_exclusive,
                        items,
                        candidates,
                        state,
                        memo,
                        choice,
                    );
                if next > best {
                    best = next;
                    choice[item_pos][min_j] = j;
                }
            }
        }

        memo[item_pos][min_j] = best;
        best
    }

    dfs_gap(
        0,
        min_j,
        max_j_exclusive,
        &items,
        candidates,
        state,
        &mut memo,
        &mut choice,
    );

    let mut item_pos = 0;
    let mut next_min_j = min_j;
    while item_pos < items.len() {
        let j = choice[item_pos][next_min_j];
        if j == R {
            item_pos += 1;
        } else {
            assigned[items[item_pos]] = Some(j);
            item_pos += 1;
            next_min_j = j + 1;
        }
    }
}

fn collect_target_info(state: &State, c: usize, done: &[bool; R]) -> [Option<TargetInfo>; R] {
    let mut info = [None; R];
    for r in 0..R {
        if done[r] {
            continue;
        }
        let target = Input::target_id(r, c);
        let (source, pos) = state.locate_sid(target);
        info[r] = Some(TargetInfo { r, source, pos });
    }
    info
}

fn make_delivery_group_plan(
    state: &State,
    info: &[Option<TargetInfo>; R],
    mask: usize,
) -> Option<DeliveryGroupPlan> {
    let mut source_used = [false; R];
    let mut targets = Vec::new();
    let mut target_pairs = Vec::new();
    let mut blockers = Vec::new();
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
        target_pairs.push((target.r, target.source));
        if target.pos > 0 {
            blockers.push((target.source, target.pos));
            blocker_sum += target.pos;
        }
        targets.push(target);
    }

    target_pairs.sort_unstable();
    for w in target_pairs.windows(2) {
        if w[0].1 >= w[1].1 {
            return None;
        }
    }

    blockers.sort_unstable();
    let mut tmp_pairs = Vec::new();
    let mut last_tmp = None;

    for (source, k) in blockers {
        let mut found = None;
        for tmp in 0..R {
            if (mask >> tmp) & 1 == 1 {
                continue;
            }
            if last_tmp.is_some_and(|last| tmp <= last) {
                continue;
            }
            if state.dep_len(tmp) + k <= DEP_CAP {
                found = Some(tmp);
                break;
            }
        }
        let tmp = found?;
        last_tmp = Some(tmp);
        tmp_pairs.push(TmpPair { source, tmp, k });
    }

    let cost = if tmp_pairs.is_empty() { 1 } else { 3 };
    Some(DeliveryGroupPlan {
        targets,
        tmp_pairs,
        cost,
        blocker_sum,
    })
}

fn better_group(a: &DeliveryGroupPlan, b: &DeliveryGroupPlan) -> bool {
    let a_key = (
        a.targets.len() * 100 / a.cost,
        a.targets.len(),
        3 - a.cost,
        1000usize.saturating_sub(a.blocker_sum),
    );
    let b_key = (
        b.targets.len() * 100 / b.cost,
        b.targets.len(),
        3 - b.cost,
        1000usize.saturating_sub(b.blocker_sum),
    );
    a_key > b_key
}

fn best_delivery_group(
    state: &State,
    info: &[Option<TargetInfo>; R],
    active_mask: usize,
) -> Option<DeliveryGroupPlan> {
    let mut best = None;
    let mut sub = active_mask;

    while sub > 0 {
        if let Some(plan) = make_delivery_group_plan(state, info, sub) {
            if best
                .as_ref()
                .is_none_or(|current| better_group(&plan, current))
            {
                best = Some(plan);
            }
        }
        sub = (sub - 1) & active_mask;
    }

    best
}

fn apply_delivery_group(
    output: &mut Output,
    state: &mut State,
    done: &mut [bool; R],
    plan: DeliveryGroupPlan,
) {
    if !plan.tmp_pairs.is_empty() {
        let moves = plan
            .tmp_pairs
            .iter()
            .map(|pair| Move::siding_to_dep(pair.tmp, pair.source, pair.k))
            .collect::<Vec<_>>();
        emit_turn(output, state, moves);
    }

    let moves = plan
        .targets
        .iter()
        .map(|target| Move::siding_to_dep(target.r, target.source, 1))
        .collect::<Vec<_>>();
    emit_turn(output, state, moves);

    for target in &plan.targets {
        done[target.r] = true;
    }

    if !plan.tmp_pairs.is_empty() {
        let moves = plan
            .tmp_pairs
            .iter()
            .map(|pair| Move::dep_to_siding(pair.tmp, pair.source, pair.k))
            .collect::<Vec<_>>();
        emit_turn(output, state, moves);
    }
}

fn apply_single_delivery(output: &mut Output, state: &mut State, r: usize, c: usize) {
    let target = Input::target_id(r, c);
    let (source, pos) = state.locate_sid(target);
    let mut chunks = Vec::new();
    let mut remaining = pos;

    while remaining > 0 {
        let mut best_tmp = None;
        let mut best_spare = 0;
        for tmp in 0..R {
            if tmp == r {
                continue;
            }
            let spare = DEP_CAP - state.dep_len(tmp);
            if spare > best_spare {
                best_spare = spare;
                best_tmp = Some(tmp);
            }
        }

        let tmp = best_tmp.expect("temporary departure line was not found");
        let k = remaining.min(best_spare);
        debug_assert!(k > 0);
        emit_turn(output, state, vec![Move::siding_to_dep(tmp, source, k)]);
        chunks.push((tmp, source, k));
        remaining -= k;
    }

    emit_turn(output, state, vec![Move::siding_to_dep(r, source, 1)]);

    for &(tmp, source, k) in chunks.iter().rev() {
        emit_turn(output, state, vec![Move::dep_to_siding(tmp, source, k)]);
    }
}

fn solve(input: &Input) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(180);

    while state.has_dep_cars() {
        let moves = choose_bucket_moves(&state);
        emit_turn(&mut output, &mut state, moves);
    }

    for c in 0..INIT_LEN {
        let mut done = [false; R];
        let mut done_count = 0;

        while done_count < R {
            let mut active_mask = 0usize;
            for r in 0..R {
                if !done[r] {
                    active_mask |= 1usize << r;
                }
            }

            let info = collect_target_info(&state, c, &done);
            if let Some(plan) = best_delivery_group(&state, &info, active_mask) {
                let progressed = plan.targets.len();
                apply_delivery_group(&mut output, &mut state, &mut done, plan);
                done_count += progressed;
            } else {
                let mut fallback = None;
                let mut best_pos = usize::MAX;
                for r in 0..R {
                    if done[r] {
                        continue;
                    }
                    let target = Input::target_id(r, c);
                    let (_, pos) = state.locate_sid(target);
                    if pos < best_pos {
                        best_pos = pos;
                        fallback = Some(r);
                    }
                }
                let r = fallback.unwrap();
                apply_single_delivery(&mut output, &mut state, r, c);
                done[r] = true;
                done_count += 1;
            }
        }

        for r in 0..R {
            debug_assert_eq!(state.dep_len(r), c + 1);
        }
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
