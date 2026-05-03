// v011_bucket_search.rs
use std::io::{Read, Write};
use std::time::Instant;

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const CAR_COUNT: usize = R * INIT_LEN;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

const TIME_LIMIT_SEC: f64 = 1.95;

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

    #[inline(always)]
    fn target_pos(car: CarId) -> usize {
        car % INIT_LEN
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

    fn sid_pos(&self, j: usize, target: CarId) -> usize {
        for pos in 0..self.sid_len(j) {
            if self.sid_car(j, pos) == target {
                return pos;
            }
        }
        panic!("target car {} is not in siding {}", target, j);
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

#[derive(Debug, Clone)]
struct GroupPlan {
    mask: usize,
    tmp_pairs: Vec<(LineIdx, LineIdx)>,
}

#[derive(Debug, Clone, Copy)]
struct BucketCandidate {
    i: LineIdx,
    j: LineIdx,
    k: usize,
    pos_sum: i32,
}

#[derive(Debug, Clone, Copy)]
struct BucketPolicy {
    split_permille: u32,
    pos_bias: i32,
    noise: i32,
}

impl BucketPolicy {
    #[inline(always)]
    fn baseline() -> Self {
        Self {
            split_permille: 0,
            pos_bias: 0,
            noise: 0,
        }
    }

    fn random(rng: &mut XorShift64) -> Self {
        const SPLIT: [u32; 6] = [0, 0, 60, 120, 220, 360];
        const POS_BIAS: [i32; 9] = [-120, -40, 0, 40, 80, 140, 220, 340, 520];
        const NOISE: [i32; 7] = [0, 120, 300, 700, 1200, 2200, 3600];

        Self {
            split_permille: SPLIT[rng.usize(SPLIT.len())],
            pos_bias: POS_BIAS[rng.usize(POS_BIAS.len())],
            noise: NOISE[rng.usize(NOISE.len())],
        }
    }
}

#[derive(Debug, Clone)]
struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let x = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        };
        Self { x }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }

    #[inline(always)]
    fn usize(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n
    }

    #[inline(always)]
    fn i32_range(&mut self, width: i32) -> i32 {
        if width <= 0 {
            0
        } else {
            (self.next_u64() % ((2 * width + 1) as u64)) as i32 - width
        }
    }
}

fn input_seed(input: &Input) -> u64 {
    let mut seed = 0x243f_6a88_85a3_08d3_u64;
    for r in 0..R {
        for c in 0..INIT_LEN {
            let v = input.initial[r][c] as u64 + 1;
            seed ^= v.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            seed = seed.rotate_left(11);
        }
    }
    seed
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

fn make_bucket_candidates(
    state: &State,
    policy: BucketPolicy,
    rng: &mut XorShift64,
    randomize: bool,
) -> ([BucketCandidate; R], usize) {
    let empty = BucketCandidate {
        i: 0,
        j: 0,
        k: 0,
        pos_sum: 0,
    };
    let mut candidates = [empty; R];
    let mut n = 0;

    for i in 0..R {
        if state.dep_len(i) == 0 {
            continue;
        }

        let j = Input::target_line(state.dep_last(i));
        let max_k = state.suffix_run_len(i);
        let mut k = max_k;
        if randomize && max_k > 1 && rng.usize(1000) < policy.split_permille as usize {
            k = 1 + rng.usize(max_k);
        }

        let len = state.dep_len(i);
        let mut pos_sum = 0;
        for offset in 0..k {
            let car = state.dep_car(i, len - k + offset);
            pos_sum += Input::target_pos(car) as i32;
        }

        candidates[n] = BucketCandidate { i, j, k, pos_sum };
        n += 1;
    }

    (candidates, n)
}

fn choose_bucket_moves(
    state: &State,
    policy: BucketPolicy,
    rng: &mut XorShift64,
    randomize: bool,
) -> Vec<Move> {
    let (candidates, n) = make_bucket_candidates(state, policy, rng, randomize);
    debug_assert!(n > 0);

    let mut score = [0i32; R];
    let mut dp = [0i32; R];
    let mut prev = [usize::MAX; R];
    let mut best = 0usize;

    for a in 0..n {
        let cand = candidates[a];
        score[a] =
            cand.k as i32 * 1000 + cand.pos_sum * policy.pos_bias + rng.i32_range(policy.noise);
        dp[a] = score[a];
        prev[a] = usize::MAX;

        for b in 0..a {
            if candidates[b].j < cand.j {
                let next = dp[b] + score[a];
                if next > dp[a] || (randomize && next == dp[a] && (rng.next_u64() & 1) == 1) {
                    dp[a] = next;
                    prev[a] = b;
                }
            }
        }

        if dp[a] > dp[best] || (randomize && dp[a] == dp[best] && (rng.next_u64() & 1) == 1) {
            best = a;
        }
    }

    let mut chosen = Vec::new();
    let mut cur = best;
    loop {
        chosen.push(cur);
        if prev[cur] == usize::MAX {
            break;
        }
        cur = prev[cur];
    }
    chosen.reverse();

    chosen
        .into_iter()
        .map(|idx| {
            let cand = candidates[idx];
            Move::dep_to_siding(cand.i, cand.j, cand.k)
        })
        .collect()
}

fn make_group_plan(mask: usize, ppos: &[usize; R]) -> Option<GroupPlan> {
    let mut blocker_rs = Vec::new();
    let mut tmp_lines = Vec::new();

    for r in 0..R {
        if (mask >> r) & 1 == 1 && ppos[r] > 0 {
            blocker_rs.push(r);
        }
        if (mask >> r) & 1 == 0 {
            tmp_lines.push(r);
        }
    }

    if blocker_rs.len() > tmp_lines.len() {
        return None;
    }

    let tmp_pairs = blocker_rs.into_iter().zip(tmp_lines).collect::<Vec<_>>();

    Some(GroupPlan { mask, tmp_pairs })
}

fn plan_groups(ppos: &[usize; R]) -> Vec<GroupPlan> {
    let full = (1usize << R) - 1;
    let inf = 1_000_000usize;
    let mut group_cost = vec![inf; 1usize << R];
    let mut group_plan: Vec<Option<GroupPlan>> = vec![None; 1usize << R];

    for mask in 1..=full {
        if let Some(plan) = make_group_plan(mask, ppos) {
            group_cost[mask] = if plan.tmp_pairs.is_empty() { 1 } else { 3 };
            group_plan[mask] = Some(plan);
        }
    }

    let mut dp = vec![inf; 1usize << R];
    let mut prev_mask = vec![usize::MAX; 1usize << R];
    let mut used_group = vec![usize::MAX; 1usize << R];
    dp[0] = 0;

    for mask in 0..=full {
        if dp[mask] == inf {
            continue;
        }
        let rem = full ^ mask;
        let mut sub = rem;
        while sub > 0 {
            if group_cost[sub] < inf {
                let next = mask | sub;
                let cand = dp[mask] + group_cost[sub];
                if cand < dp[next] {
                    dp[next] = cand;
                    prev_mask[next] = mask;
                    used_group[next] = sub;
                }
            }
            sub = (sub - 1) & rem;
        }
    }

    let mut masks = Vec::new();
    let mut mask = full;
    while mask != 0 {
        let sub = used_group[mask];
        debug_assert_ne!(sub, usize::MAX);
        masks.push(sub);
        mask = prev_mask[mask];
    }
    masks.reverse();

    masks
        .into_iter()
        .map(|mask| group_plan[mask].clone().unwrap())
        .collect()
}

fn build_output(
    input: &Input,
    policy: BucketPolicy,
    rng: &mut XorShift64,
    randomize: bool,
) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(170);

    while state.has_dep_cars() {
        let moves = choose_bucket_moves(&state, policy, rng, randomize);
        emit_turn(&mut output, &mut state, moves);
    }

    for c in 0..INIT_LEN {
        for r in 0..R {
            debug_assert_eq!(state.dep_len(r), c);
        }

        let mut ppos = [0usize; R];
        for (r, slot) in ppos.iter_mut().enumerate() {
            let target = Input::target_id(r, c);
            *slot = state.sid_pos(r, target);
        }

        for plan in plan_groups(&ppos) {
            if !plan.tmp_pairs.is_empty() {
                let moves = plan
                    .tmp_pairs
                    .iter()
                    .map(|&(r, tmp)| Move::siding_to_dep(tmp, r, ppos[r]))
                    .collect::<Vec<_>>();
                emit_turn(&mut output, &mut state, moves);
            }

            let mut target_moves = Vec::new();
            for r in 0..R {
                if (plan.mask >> r) & 1 == 1 {
                    debug_assert_eq!(state.sid_car(r, 0), Input::target_id(r, c));
                    target_moves.push(Move::siding_to_dep(r, r, 1));
                }
            }
            emit_turn(&mut output, &mut state, target_moves);

            if !plan.tmp_pairs.is_empty() {
                let moves = plan
                    .tmp_pairs
                    .iter()
                    .map(|&(r, tmp)| Move::dep_to_siding(tmp, r, ppos[r]))
                    .collect::<Vec<_>>();
                emit_turn(&mut output, &mut state, moves);
            }
        }
    }

    debug_assert!(state.is_complete());
    debug_assert!(output.turns.len() <= MAX_TURNS);

    output
}

fn solve(input: &Input) -> Output {
    let mut rng = XorShift64::new(input_seed(input));
    let mut best = build_output(input, BucketPolicy::baseline(), &mut rng, false);
    let start = Instant::now();

    while start.elapsed().as_secs_f64() < TIME_LIMIT_SEC {
        let policy = BucketPolicy::random(&mut rng);
        let candidate = build_output(input, policy, &mut rng, true);
        if candidate.turns.len() < best.turns.len() {
            best = candidate;
        }
    }

    best
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
