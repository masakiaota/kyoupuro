// v002_parallel_tmp.rs
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

#[derive(Debug, Clone)]
struct GroupPlan {
    mask: usize,
    tmp_pairs: Vec<(LineIdx, LineIdx)>,
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

fn best_bucket_moves(state: &State) -> Vec<Move> {
    let mut candidates = Vec::new();

    for i in 0..R {
        if state.dep[i].is_empty() {
            continue;
        }

        let j = Input::target_line(*state.dep[i].last().unwrap());
        let mut k = 0;
        for &car in state.dep[i].iter().rev() {
            if Input::target_line(car) == j {
                k += 1;
            } else {
                break;
            }
        }
        candidates.push((i, j, k));
    }

    let n = candidates.len();
    let mut dp = vec![0usize; n];
    let mut prev = vec![usize::MAX; n];
    let mut best = 0;

    for a in 0..n {
        dp[a] = candidates[a].2;
        for b in 0..a {
            if candidates[b].1 < candidates[a].1 && dp[b] + candidates[a].2 > dp[a] {
                dp[a] = dp[b] + candidates[a].2;
                prev[a] = b;
            }
        }
        if dp[a] > dp[best] {
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
            let (i, j, k) = candidates[idx];
            Move::dep_to_siding(i, j, k)
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

fn solve(input: &Input) -> Output {
    let _ = input.initial_pos[0];
    let mut state = State::new(input);
    let mut output = Output::with_capacity(140);

    while state.dep.iter().any(|line| !line.is_empty()) {
        let moves = best_bucket_moves(&state);
        emit_turn(&mut output, &mut state, moves);
    }

    for c in 0..INIT_LEN {
        for r in 0..R {
            debug_assert_eq!(state.dep[r].len(), c);
        }

        let mut ppos = [0usize; R];
        for (r, slot) in ppos.iter_mut().enumerate() {
            let target = Input::target_id(r, c);
            *slot = state.sid[r].iter().position(|&car| car == target).unwrap();
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
                    debug_assert_eq!(state.sid[r][0], Input::target_id(r, c));
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

    assert!(state.is_complete());
    assert!(output.turns.len() <= MAX_TURNS);

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
