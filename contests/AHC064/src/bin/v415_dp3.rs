// v415_dp3.rs
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

    fn push_turn(&mut self, moves: Vec<Move>) {
        if !moves.is_empty() {
            self.turns.push(moves);
        }
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

struct XorShift {
    state: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        self.state = x;
        x
    }

    fn gen_range(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn input_seed(input: &Input) -> u64 {
    let mut seed = 1469598103934665603u64;
    for r in 0..R {
        for c in 0..INIT_LEN {
            seed ^= input.initial[r][c] as u64;
            seed = seed.wrapping_mul(1099511628211u64);
        }
    }
    seed
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

    fn sid_slot(head: usize, offset: usize) -> usize {
        let slot = head + offset;
        if slot >= SIDING_CAP {
            slot - SIDING_CAP
        } else {
            slot
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

fn choose_random_zero_gain_move(
    state: &State,
    rng: &mut XorShift,
    i: usize,
    j: usize,
    kind: usize,
) -> Option<Move> {
    let mut selected = None;
    let mut count = 0usize;
    if kind == MOVE_DEP_TO_SIDING {
        let max_k = state.dep_len(i).min(SIDING_CAP - state.sid_len(j));
        for k in 1..=max_k {
            let mv = Move::dep_to_siding(i, j, k);
            if move_delta_p(state, mv) == 0 {
                count += 1;
                if rng.gen_range(count) == 0 {
                    selected = Some(mv);
                }
            }
        }
    } else {
        let max_k = state.sid_len(j).min(DEP_CAP - state.dep_len(i));
        for k in 1..=max_k {
            let mv = Move::siding_to_dep(i, j, k);
            if move_delta_p(state, mv) == 0 {
                count += 1;
                if rng.gen_range(count) == 0 {
                    selected = Some(mv);
                }
            }
        }
    }
    selected
}

fn shuffle_moves(moves: &mut [Move], rng: &mut XorShift) {
    for i in (1..moves.len()).rev() {
        let j = rng.gen_range(i + 1);
        moves.swap(i, j);
    }
}

fn random_plateau_turn(state: &State, rng: &mut XorShift) -> Vec<Move> {
    let mut candidates = Vec::new();
    for i in 0..R {
        for j in 0..R {
            if let Some(mv) = choose_random_zero_gain_move(state, rng, i, j, MOVE_DEP_TO_SIDING) {
                candidates.push(mv);
            }
            if let Some(mv) = choose_random_zero_gain_move(state, rng, i, j, MOVE_SIDING_TO_DEP) {
                candidates.push(mv);
            }
        }
    }

    shuffle_moves(&mut candidates, rng);

    let mut turn = Vec::new();
    for mv in candidates {
        if can_add_to_turn(&turn, mv) {
            turn.push(mv);
        }
    }
    turn
}

fn is_goal(state: &State) -> bool {
    for j in 0..R {
        if state.sid_len(j) != 0 {
            return false;
        }
    }
    for r in 0..R {
        if state.dep_len(r) != INIT_LEN {
            return false;
        }
        for c in 0..INIT_LEN {
            if state.dep[r][c] as usize != r * INIT_LEN + c {
                return false;
            }
        }
    }
    true
}

fn solve(input: &Input) -> Output {
    let mut state = State::new(input);
    let mut output = Output::new();
    let mut rng = XorShift::new(input_seed(input));

    for _ in 0..4000 {
        if is_goal(&state) {
            break;
        }

        let mut turn = best_dp_turn(&state);
        if turn.is_empty() {
            turn = random_plateau_turn(&state, &mut rng);
        }
        if turn.is_empty() {
            break;
        }
        for &mv in &turn {
            state.apply_move(mv);
        }
        output.push_turn(turn);
    }

    output
}

fn main() {
    let input = Input::read();
    let output = solve(&input);
    output.print();
}
