// v403_block_greedy.rs
use std::collections::VecDeque;
use std::io::{self, Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SIDING_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

#[derive(Clone)]
struct Input {
    initial: [[usize; INIT_LEN]; R],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
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

    #[inline]
    fn target_id(r: usize, c: usize) -> usize {
        r * INIT_LEN + c
    }

    #[inline]
    fn target_line(car: usize) -> usize {
        car / INIT_LEN
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
    #[inline]
    fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    #[inline]
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

    fn print(&self) {
        let mut out = io::BufWriter::new(io::stdout().lock());
        writeln!(out, "{}", self.turns.len()).unwrap();
        for turn in &self.turns {
            writeln!(out, "{}", turn.len()).unwrap();
            for mv in turn {
                writeln!(out, "{} {} {} {}", mv.kind, mv.i, mv.j, mv.k).unwrap();
            }
        }
    }
}

#[derive(Clone)]
struct State {
    dep: Vec<VecDeque<usize>>,
    sid: Vec<VecDeque<usize>>,
}

impl State {
    fn new(input: &Input) -> Self {
        let mut dep = vec![VecDeque::new(); R];
        for r in 0..R {
            dep[r] = input.initial[r].iter().copied().collect();
        }
        Self {
            dep,
            sid: vec![VecDeque::new(); R],
        }
    }

    fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            let mut block = Vec::with_capacity(mv.k);
            for _ in 0..mv.k {
                block.push(self.dep[mv.i].pop_back().unwrap());
            }
            block.reverse();
            for &car in block.iter().rev() {
                self.sid[mv.j].push_front(car);
            }
        } else {
            for _ in 0..mv.k {
                let car = self.sid[mv.j].pop_front().unwrap();
                self.dep[mv.i].push_back(car);
            }
        }
    }

    fn prefix_len(&self, r: usize) -> usize {
        let mut len = 0;
        while len < INIT_LEN
            && len < self.dep[r].len()
            && self.dep[r][len] == Input::target_id(r, len)
        {
            len += 1;
        }
        len
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
        }
        true
    }
}

#[derive(Debug, Clone, Copy)]
struct EdgeBlock {
    start: usize,
    end: usize,
    len: usize,
}

#[derive(Debug, Clone, Copy)]
enum Area {
    Dep,
    Sid,
}

#[derive(Debug, Clone, Copy)]
struct Location {
    area: Area,
    line: usize,
    pos: usize,
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    mv: Move,
    weight: i64,
}

#[inline]
fn same_target_line(a: usize, b: usize) -> bool {
    Input::target_line(a) == Input::target_line(b)
}

#[inline]
fn is_next(a: usize, b: usize) -> bool {
    a + 1 == b && same_target_line(a, b)
}

fn dep_tail_block(state: &State, i: usize) -> Option<EdgeBlock> {
    let line = &state.dep[i];
    if line.is_empty() {
        return None;
    }
    let mut l = line.len() - 1;
    while l > 0 && is_next(line[l - 1], line[l]) {
        l -= 1;
    }
    Some(EdgeBlock {
        start: line[l],
        end: *line.back().unwrap(),
        len: line.len() - l,
    })
}

fn sid_head_block(state: &State, j: usize) -> Option<EdgeBlock> {
    let line = &state.sid[j];
    if line.is_empty() {
        return None;
    }
    let mut r = 1;
    while r < line.len() && is_next(line[r - 1], line[r]) {
        r += 1;
    }
    Some(EdgeBlock {
        start: line[0],
        end: line[r - 1],
        len: r,
    })
}

fn locate_car(state: &State, car: usize) -> Location {
    for i in 0..R {
        for (pos, &x) in state.dep[i].iter().enumerate() {
            if x == car {
                return Location {
                    area: Area::Dep,
                    line: i,
                    pos,
                };
            }
        }
    }
    for j in 0..R {
        for (pos, &x) in state.sid[j].iter().enumerate() {
            if x == car {
                return Location {
                    area: Area::Sid,
                    line: j,
                    pos,
                };
            }
        }
    }
    panic!("car not found: {}", car);
}

fn forward_block_len(state: &State, loc: Location, limit: usize) -> usize {
    let line = match loc.area {
        Area::Dep => &state.dep[loc.line],
        Area::Sid => &state.sid[loc.line],
    };
    let mut len = 1;
    while len < limit
        && loc.pos + len < line.len()
        && is_next(line[loc.pos + len - 1], line[loc.pos + len])
    {
        len += 1;
    }
    len
}

fn line_block_potential(line: &VecDeque<usize>) -> i64 {
    if line.is_empty() {
        return 0;
    }
    let mut sum = 0;
    let mut len = 1;
    for p in 1..line.len() {
        if is_next(line[p - 1], line[p]) {
            len += 1;
        } else {
            sum += (len * len) as i64;
            len = 1;
        }
    }
    sum + (len * len) as i64
}

fn block_potential(state: &State) -> i64 {
    let mut sum = 0;
    for i in 0..R {
        sum += line_block_potential(&state.dep[i]);
        sum += line_block_potential(&state.sid[i]);
    }
    sum
}

fn valid_move(state: &State, mv: Move) -> bool {
    if mv.k == 0 || mv.i >= R || mv.j >= R {
        return false;
    }
    if mv.kind == MOVE_DEP_TO_SIDING {
        state.dep[mv.i].len() >= mv.k && state.sid[mv.j].len() + mv.k <= SIDING_CAP
    } else {
        state.sid[mv.j].len() >= mv.k && state.dep[mv.i].len() + mv.k <= DEP_CAP
    }
}

fn add_candidate(cands: &mut Vec<Candidate>, state: &State, mv: Move, weight: i64) {
    if weight > 0 && valid_move(state, mv) {
        cands.push(Candidate { mv, weight });
    }
}

fn select_batch(cands: &[Candidate]) -> Vec<Move> {
    let mut best: [[Option<Candidate>; R]; R] = [[None; R]; R];
    for &cand in cands {
        let cell = &mut best[cand.mv.i][cand.mv.j];
        if cell.map_or(true, |old| cand.weight > old.weight) {
            *cell = Some(cand);
        }
    }

    let mut dp = [[0_i64; R + 1]; R + 1];
    let mut parent = [[0_u8; R + 1]; R + 1];
    for i in 1..=R {
        for j in 1..=R {
            let mut value = dp[i - 1][j];
            let mut par = 1_u8;
            if dp[i][j - 1] > value {
                value = dp[i][j - 1];
                par = 2;
            }
            if let Some(cand) = best[i - 1][j - 1] {
                let diag = dp[i - 1][j - 1] + cand.weight;
                if diag > value {
                    value = diag;
                    par = 3;
                }
            }
            dp[i][j] = value;
            parent[i][j] = par;
        }
    }

    let mut moves = Vec::new();
    let mut i = R;
    let mut j = R;
    while i > 0 && j > 0 {
        match parent[i][j] {
            3 => {
                moves.push(best[i - 1][j - 1].unwrap().mv);
                i -= 1;
                j -= 1;
            }
            2 => j -= 1,
            _ => i -= 1,
        }
    }
    moves.sort_by_key(|mv| (mv.i, mv.j));
    moves
}

fn try_push_turn_batch(state: &mut State, output: &mut Output, mut moves: Vec<Move>) -> bool {
    if moves.is_empty() || output.turns.len() >= MAX_TURNS {
        return false;
    }
    moves.sort_by_key(|mv| (mv.i, mv.j));

    let mut used_i = [false; R];
    let mut used_j = [false; R];
    let mut prev: Option<(usize, usize)> = None;
    for &mv in &moves {
        if used_i[mv.i] || used_j[mv.j] || !valid_move(state, mv) {
            return false;
        }
        used_i[mv.i] = true;
        used_j[mv.j] = true;
        if let Some((pi, pj)) = prev {
            if !(pi < mv.i && pj < mv.j) {
                return false;
            }
        }
        prev = Some((mv.i, mv.j));
    }

    for &mv in &moves {
        state.apply_move(mv);
    }
    output.turns.push(moves);
    true
}

fn generate_phase1_candidates(state: &State) -> Vec<Candidate> {
    let mut cands = Vec::new();
    let mut dep_blocks = [None; R];
    let mut sid_blocks = [None; R];
    for i in 0..R {
        dep_blocks[i] = dep_tail_block(state, i);
        sid_blocks[i] = sid_head_block(state, i);
    }

    for i in 0..R {
        if let Some(a) = dep_blocks[i] {
            for j in 0..R {
                if let Some(b) = sid_blocks[j] {
                    if a.end + 1 == b.start && same_target_line(a.end, b.start) {
                        let result_len = a.len + b.len;
                        let weight = 10000 + 500 * result_len as i64 + 50 * a.len as i64;
                        add_candidate(
                            &mut cands,
                            state,
                            Move::dep_to_siding(i, j, a.len),
                            weight,
                        );
                    }
                }
            }
            if a.len >= 2 {
                for j in 0..R {
                    if state.sid[j].is_empty() {
                        let weight = if a.len >= 3 {
                            800 + 120 * a.len as i64
                        } else {
                            250
                        };
                        add_candidate(
                            &mut cands,
                            state,
                            Move::dep_to_siding(i, j, a.len),
                            weight,
                        );
                    }
                }
            }
        }
    }

    for j in 0..R {
        if let Some(a) = sid_blocks[j] {
            for i in 0..R {
                if let Some(b) = dep_blocks[i] {
                    if b.end + 1 == a.start && same_target_line(b.end, a.start) {
                        let result_len = a.len + b.len;
                        let weight = 10000 + 500 * result_len as i64 + 50 * a.len as i64;
                        add_candidate(
                            &mut cands,
                            state,
                            Move::siding_to_dep(i, j, a.len),
                            weight,
                        );
                    }
                }
            }
            if a.len >= 2 {
                for i in 0..R {
                    if state.dep[i].is_empty() {
                        let weight = if a.len >= 3 {
                            800 + 120 * a.len as i64
                        } else {
                            250
                        };
                        add_candidate(
                            &mut cands,
                            state,
                            Move::siding_to_dep(i, j, a.len),
                            weight,
                        );
                    }
                }
            }
        }
    }
    cands
}

fn run_phase1(state: &mut State, output: &mut Output) {
    let mut best_potential = block_potential(state);
    let mut stagnant = 0;
    for _ in 0..1200 {
        if output.turns.len() >= MAX_TURNS - 50 {
            break;
        }
        let cands = generate_phase1_candidates(state);
        let batch = select_batch(&cands);
        if batch.is_empty() {
            break;
        }
        if !try_push_turn_batch(state, output, batch) {
            break;
        }
        let now = block_potential(state);
        if now > best_potential {
            best_potential = now;
            stagnant = 0;
        } else {
            stagnant += 1;
            if stagnant >= 80 {
                break;
            }
        }
    }
}

fn refresh_fixed(state: &State, fixed: &mut [usize; R]) {
    for r in 0..R {
        fixed[r] = state.prefix_len(r);
    }
}

fn cleanup_k_from_dep(state: &State, i: usize, fixed: &[usize; R]) -> usize {
    if state.dep[i].len() <= fixed[i] {
        return 0;
    }
    let extra = state.dep[i].len() - fixed[i];
    let tail = dep_tail_block(state, i).map_or(1, |b| b.len);
    tail.min(extra)
}

fn run_completion(state: &mut State, output: &mut Output) {
    collect_all_dep_to_sid(state, output);
    build_from_sid_parallel(state, output);
}

fn collect_all_dep_to_sid(state: &mut State, output: &mut Output) {
    while state.dep.iter().any(|line| !line.is_empty()) && output.turns.len() < MAX_TURNS {
        let mut cands = Vec::new();
        for i in 0..R {
            if state.dep[i].is_empty() {
                continue;
            }
            for j in 0..R {
                let cap = SIDING_CAP - state.sid[j].len();
                if cap == 0 {
                    continue;
                }
                let k = state.dep[i].len().min(cap);
                let weight = 150000 + 200 * k as i64 - i.abs_diff(j) as i64;
                add_candidate(
                    &mut cands,
                    state,
                    Move::dep_to_siding(i, j, k),
                    weight,
                );
            }
        }
        let mut batch = select_batch(&cands);
        if batch.is_empty() {
            'outer: for i in 0..R {
                if state.dep[i].is_empty() {
                    continue;
                }
                for j in 0..R {
                    let cap = SIDING_CAP - state.sid[j].len();
                    if cap > 0 {
                        batch.push(Move::dep_to_siding(i, j, state.dep[i].len().min(cap)));
                        break 'outer;
                    }
                }
            }
        }
        if !try_push_turn_batch(state, output, batch) {
            break;
        }
    }
}

fn build_from_sid_parallel(state: &mut State, output: &mut Output) {
    let mut fixed = [0_usize; R];
    refresh_fixed(state, &mut fixed);

    while !state.is_complete() && output.turns.len() < MAX_TURNS {
        flush_dep_extras_batched(state, output, &fixed, &[None; R]);
        refresh_fixed(state, &mut fixed);

        let mut appended = false;
        loop {
            let batch = direct_append_batch(state, &fixed);
            if batch.is_empty() {
                break;
            }
            if !try_push_turn_batch(state, output, batch) {
                break;
            }
            appended = true;
            refresh_fixed(state, &mut fixed);
            flush_dep_extras_batched(state, output, &fixed, &[None; R]);
            refresh_fixed(state, &mut fixed);
            if state.is_complete() || output.turns.len() >= MAX_TURNS {
                break;
            }
        }
        if appended {
            continue;
        }

        let Some(target) = choose_next_target(state, &fixed) else {
            break;
        };
        if !expose_target_to_sid_head(state, output, &mut fixed, target) {
            break;
        }
    }
}

fn direct_append_batch(state: &State, fixed: &[usize; R]) -> Vec<Move> {
    let mut cands = Vec::new();
    for r in 0..R {
        if fixed[r] >= INIT_LEN || state.dep[r].len() != fixed[r] {
            continue;
        }
        let target = Input::target_id(r, fixed[r]);
        let loc = locate_car(state, target);
        if let Area::Sid = loc.area {
            if loc.pos == 0 {
                let block_len = forward_block_len(state, loc, INIT_LEN - fixed[r]);
                let weight = 300000 + 3000 * block_len as i64;
                add_candidate(
                    &mut cands,
                    state,
                    Move::siding_to_dep(r, loc.line, block_len),
                    weight,
                );
            }
        }
    }
    select_batch(&cands)
}

fn choose_next_target(state: &State, fixed: &[usize; R]) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for r in 0..R {
        if fixed[r] >= INIT_LEN || state.dep[r].len() != fixed[r] {
            continue;
        }
        let target = Input::target_id(r, fixed[r]);
        let loc = locate_car(state, target);
        let cost = match loc.area {
            Area::Sid => loc.pos,
            Area::Dep => 100 + state.dep[loc.line].len().saturating_sub(loc.pos + 1),
        };
        if best.map_or(true, |(old_cost, _)| cost < old_cost) {
            best = Some((cost, target));
        }
    }
    best.map(|(_, target)| target)
}

fn flush_dep_extras_batched(
    state: &mut State,
    output: &mut Output,
    fixed: &[usize; R],
    avoid_sid: &[Option<usize>; R],
) -> bool {
    let mut moved_any = false;
    while state
        .dep
        .iter()
        .enumerate()
        .any(|(i, line)| line.len() > fixed[i])
        && output.turns.len() < MAX_TURNS
    {
        let mut cands = Vec::new();
        for i in 0..R {
            if state.dep[i].len() <= fixed[i] {
                continue;
            }
            let base_k = cleanup_k_from_dep(state, i, fixed).max(1);
            for j in 0..R {
                let cap = SIDING_CAP - state.sid[j].len();
                if cap == 0 {
                    continue;
                }
                let k = base_k.min(cap);
                let avoid_penalty = if avoid_sid[i] == Some(j) { 50000 } else { 0 };
                let weight = 200000 + 200 * k as i64 - avoid_penalty;
                add_candidate(
                    &mut cands,
                    state,
                    Move::dep_to_siding(i, j, k),
                    weight,
                );
            }
        }
        let mut batch = select_batch(&cands);
        if batch.is_empty() {
            'outer: for i in 0..R {
                if state.dep[i].len() <= fixed[i] {
                    continue;
                }
                let base_k = cleanup_k_from_dep(state, i, fixed).max(1);
                for pass in 0..2 {
                    for j in 0..R {
                        if pass == 0 && avoid_sid[i] == Some(j) {
                            continue;
                        }
                        let cap = SIDING_CAP - state.sid[j].len();
                        if cap > 0 {
                            batch.push(Move::dep_to_siding(i, j, base_k.min(cap)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        if !try_push_turn_batch(state, output, batch) {
            break;
        }
        moved_any = true;
    }
    moved_any
}

fn expose_target_to_sid_head(
    state: &mut State,
    output: &mut Output,
    fixed: &mut [usize; R],
    target: usize,
) -> bool {
    loop {
        refresh_fixed(state, fixed);
        flush_dep_extras_batched(state, output, fixed, &[None; R]);
        refresh_fixed(state, fixed);
        if output.turns.len() >= MAX_TURNS {
            return false;
        }

        let loc = locate_car(state, target);
        match loc.area {
            Area::Sid => {
                if loc.pos == 0 {
                    return true;
                }
                let source = loc.line;
                let max_slack = (0..R)
                    .map(|i| DEP_CAP - state.dep[i].len())
                    .max()
                    .unwrap_or(0);
                if max_slack == 0 {
                    return false;
                }
                let mut dest = None;
                let mut best_cap = 0;
                for j in 0..R {
                    if j == source {
                        continue;
                    }
                    let cap = SIDING_CAP - state.sid[j].len();
                    if cap > best_cap {
                        best_cap = cap;
                        dest = Some(j);
                    }
                }
                let Some(dest) = dest else {
                    return false;
                };
                if best_cap == 0 {
                    return false;
                }
                let head_len = sid_head_block(state, source).map_or(1, |b| b.len);
                let mut k = loc.pos.min(head_len).min(max_slack).min(best_cap).max(1);
                let temp = (0..R)
                    .max_by_key(|&i| DEP_CAP - state.dep[i].len())
                    .unwrap();
                k = k.min(DEP_CAP - state.dep[temp].len());
                if k == 0 {
                    return false;
                }
                if !try_push_turn_batch(
                    state,
                    output,
                    vec![Move::siding_to_dep(temp, source, k)],
                ) {
                    return false;
                }

                if !try_push_turn_batch(state, output, vec![Move::dep_to_siding(temp, dest, k)]) {
                    return false;
                }
            }
            Area::Dep => {
                let mut avoid = [None; R];
                avoid[loc.line] = None;
                if !flush_dep_extras_batched(state, output, fixed, &avoid) {
                    return false;
                }
            }
        }
    }
}

fn main() {
    let input = Input::read();
    let mut state = State::new(&input);
    let mut output = Output::new();

    run_phase1(&mut state, &mut output);
    run_completion(&mut state, &mut output);

    if output.turns.len() > MAX_TURNS {
        output.turns.truncate(MAX_TURNS);
    }
    output.print();
}
