// v021_direct5_beam.rs
use std::io::{Read, Write};
use std::time::Instant;

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SID_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const TIME_LIMIT_SEC: f64 = 1.90;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Move {
    kind: usize,
    i: usize,
    j: usize,
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

#[derive(Clone, Copy, Debug)]
struct DirectPolicy {
    bucket_noise: i32,
    ready_bonus: i32,
    ordered_bonus: i32,
    block_penalty: i32,
}

impl DirectPolicy {
    fn base() -> Self {
        Self {
            bucket_noise: 0,
            ready_bonus: 32_000,
            ordered_bonus: 8_000,
            block_penalty: 4_000,
        }
    }

    fn random(rng: &mut XorShift64) -> Self {
        const BUCKET_NOISE: [i32; 6] = [0, 120, 300, 700, 1400, 2600];
        const READY_BONUS: [i32; 5] = [18_000, 24_000, 32_000, 42_000, 56_000];
        const ORDERED_BONUS: [i32; 5] = [3_000, 5_000, 8_000, 13_000, 21_000];
        const BLOCK_PENALTY: [i32; 5] = [1_000, 2_000, 4_000, 7_000, 11_000];

        Self {
            bucket_noise: BUCKET_NOISE[rng.usize(BUCKET_NOISE.len())],
            ready_bonus: READY_BONUS[rng.usize(READY_BONUS.len())],
            ordered_bonus: ORDERED_BONUS[rng.usize(ORDERED_BONUS.len())],
            block_penalty: BLOCK_PENALTY[rng.usize(BLOCK_PENALTY.len())],
        }
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct Plan {
    take: usize,
    total_skip: usize,
    chunks: Vec<usize>,
    segs: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
struct DirectCandidate {
    mv: Move,
    score: i32,
}

impl Plan {
    fn new() -> Self {
        Self {
            take: 0,
            total_skip: 0,
            chunks: Vec::new(),
            segs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct Solver {
    dep: Vec<Vec<usize>>,
    sid: Vec<Vec<usize>>,
    turns: Vec<Vec<Move>>,
    debug: bool,
}

impl Solver {
    fn new(init: Vec<Vec<usize>>) -> Self {
        Self {
            dep: init,
            sid: vec![Vec::new(); R],
            turns: Vec::new(),
            debug: false,
        }
    }

    #[inline(always)]
    fn target_line(car: usize) -> usize {
        car / INIT_LEN
    }

    #[inline(always)]
    fn target_id(r: usize, c: usize) -> usize {
        INIT_LEN * r + c
    }

    fn add_turn(&mut self, mut moves: Vec<Move>) {
        if moves.is_empty() {
            return;
        }
        moves.sort_unstable_by_key(|mv| (mv.i, mv.j));
        if self.debug {
            self.validate_turn(&moves);
        }
        for &mv in &moves {
            self.apply(mv);
        }
        self.turns.push(moves);
    }

    fn validate_turn(&self, moves: &[Move]) {
        let mut used_dep = [false; R];
        let mut used_sid = [false; R];
        let mut last_i = None;
        let mut last_j = 0usize;

        for &mv in moves {
            assert!(mv.kind == MOVE_DEP_TO_SIDING || mv.kind == MOVE_SIDING_TO_DEP);
            assert!(mv.i < R && mv.j < R && mv.k >= 1);
            assert!(!used_dep[mv.i] && !used_sid[mv.j]);
            used_dep[mv.i] = true;
            used_sid[mv.j] = true;
            if let Some(prev_i) = last_i {
                assert!(prev_i < mv.i && last_j < mv.j);
            }
            last_i = Some(mv.i);
            last_j = mv.j;
            if mv.kind == MOVE_DEP_TO_SIDING {
                assert!(self.dep[mv.i].len() >= mv.k);
                assert!(self.sid[mv.j].len() + mv.k <= SID_CAP);
            } else {
                assert!(self.sid[mv.j].len() >= mv.k);
                assert!(self.dep[mv.i].len() + mv.k <= DEP_CAP);
            }
        }
    }

    fn apply(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            let n = self.dep[mv.i].len();
            let block = self.dep[mv.i][n - mv.k..].to_vec();
            self.dep[mv.i].truncate(n - mv.k);

            let mut next = block;
            next.extend_from_slice(&self.sid[mv.j]);
            self.sid[mv.j] = next;
        } else {
            let block = self.sid[mv.j][..mv.k].to_vec();
            self.sid[mv.j].drain(..mv.k);
            self.dep[mv.i].extend(block);
        }
    }

    fn make_scan_plan(&self, r: usize) -> Plan {
        let mut plan = Plan::new();
        let pref = self.dep[r].len();
        if pref >= INIT_LEN {
            return plan;
        }

        let mut pos = [0usize; INIT_LEN];
        for (i, &car) in self.sid[r].iter().enumerate() {
            pos[car % INIT_LEN] = i;
        }

        let mut last = None::<usize>;
        while pref + plan.take < INIT_LEN {
            let p = pos[pref + plan.take];
            if last.map_or(true, |prev| p > prev) {
                last = Some(p);
                plan.take += 1;
            } else {
                break;
            }
        }

        let mut cur = 0usize;
        for t in 0..plan.take {
            let p = pos[pref + t];
            let chunk = p - cur;
            plan.total_skip += chunk;
            if t == 0 || chunk > 0 {
                plan.chunks.push(chunk);
                plan.segs.push(1);
            } else {
                *plan.segs.last_mut().unwrap() += 1;
            }
            cur = p + 1;
        }

        plan
    }

    fn assign_buffers_for(
        &self,
        active_p: &[usize],
        plans: &[Plan],
        active: &[usize],
        buffers: &mut Vec<usize>,
    ) -> bool {
        let mut is_active = [false; R];
        for &r in active {
            is_active[r] = true;
        }

        let mut cand = Vec::new();
        for (b, &active) in is_active.iter().enumerate() {
            if !active {
                cand.push(b);
            }
        }

        buffers.clear();
        let mut ci = 0usize;
        for &r in active_p {
            let mut ok = false;
            while ci < cand.len() {
                let b = cand[ci];
                ci += 1;
                if self.dep[b].len() + plans[r].total_skip <= DEP_CAP {
                    buffers.push(b);
                    ok = true;
                    break;
                }
            }
            if !ok {
                return false;
            }
        }

        true
    }

    fn execute_scan_batch(
        &mut self,
        active_p: &[usize],
        buffers: &[usize],
        active_z: &[usize],
        plans: &[Plan],
    ) {
        let a = active_p.len();
        let max_seg = active_p
            .iter()
            .map(|&r| plans[r].segs.len())
            .max()
            .unwrap_or(0);
        let mut z_done = false;

        for step in 0..max_seg {
            let mut mb = Vec::new();
            for t in 0..a {
                let r = active_p[t];
                if step < plans[r].chunks.len() && plans[r].chunks[step] > 0 {
                    mb.push(Move::siding_to_dep(buffers[t], r, plans[r].chunks[step]));
                }
            }
            self.add_turn(mb);

            let mut mt = Vec::new();
            for &r in active_p {
                if step < plans[r].segs.len() {
                    mt.push(Move::siding_to_dep(r, r, plans[r].segs[step]));
                }
            }
            if !z_done {
                for &r in active_z {
                    if plans[r].take > 0 {
                        mt.push(Move::siding_to_dep(r, r, plans[r].take));
                    }
                }
                z_done = true;
            }
            self.add_turn(mt);
        }

        if !z_done && !active_z.is_empty() {
            let mut mt = Vec::new();
            for &r in active_z {
                if plans[r].take > 0 {
                    mt.push(Move::siding_to_dep(r, r, plans[r].take));
                }
            }
            self.add_turn(mt);
        }

        let mut mr = Vec::new();
        for t in 0..a {
            let r = active_p[t];
            if plans[r].total_skip > 0 {
                mr.push(Move::dep_to_siding(buffers[t], r, plans[r].total_skip));
            }
        }
        self.add_turn(mr);
    }

    fn dep_prefix_len(&self, r: usize) -> usize {
        let mut p = 0usize;
        while p < self.dep[r].len() && p < INIT_LEN && self.dep[r][p] == Self::target_id(r, p) {
            p += 1;
        }
        p
    }

    fn dep_is_clean_prefix(&self, r: usize) -> bool {
        self.dep[r].len() == self.dep_prefix_len(r)
    }

    fn all_dep_clean_prefixes(&self) -> bool {
        (0..R).all(|r| self.dep_is_clean_prefix(r))
    }

    fn sidings_are_grouped(&self) -> bool {
        for j in 0..R {
            for &car in &self.sid[j] {
                if Self::target_line(car) != j {
                    return false;
                }
            }
        }
        true
    }

    fn block_is_target_range(&self, block: &[usize], r: usize, start_pos: usize) -> bool {
        if start_pos + block.len() > INIT_LEN {
            return false;
        }
        block
            .iter()
            .enumerate()
            .all(|(offset, &car)| car == Self::target_id(r, start_pos + offset))
    }

    fn make_direct_candidates(
        &self,
        policy: DirectPolicy,
        rng: &mut XorShift64,
    ) -> Vec<DirectCandidate> {
        let mut candidates = Vec::with_capacity(R * 4);
        let all_clean = self.all_dep_clean_prefixes();

        for r in 0..R {
            if !self.dep_is_clean_prefix(r) {
                continue;
            }
            let pref = self.dep[r].len();
            if pref >= INIT_LEN || (!all_clean && pref >= 6) || self.sid[r].is_empty() {
                continue;
            }
            if self.sid[r][0] != Self::target_id(r, pref) {
                continue;
            }

            let mut k = 0usize;
            while pref + k < INIT_LEN
                && k < self.sid[r].len()
                && self.sid[r][k] == Self::target_id(r, pref + k)
            {
                k += 1;
            }
            if k > 0 {
                candidates.push(DirectCandidate {
                    mv: Move::siding_to_dep(r, r, k),
                    score: policy.ready_bonus + 8_000 * k as i32,
                });
            }
        }

        for i in 0..R {
            let fixed = self.dep_prefix_len(i);
            let n = self.dep[i].len();
            if n <= fixed {
                continue;
            }

            let j = Self::target_line(self.dep[i][n - 1]);
            let mut max_k = 0usize;
            while max_k < n - fixed
                && Self::target_line(self.dep[i][n - 1 - max_k]) == j
                && self.sid[j].len() + max_k < SID_CAP
            {
                max_k += 1;
            }
            if max_k == 0 {
                continue;
            }

            let mut sizes = [0usize; 4];
            sizes[0] = max_k;
            sizes[1] = 1;
            sizes[2] = (max_k + 1) / 2;
            sizes[3] = if max_k >= 3 { max_k - 1 } else { max_k };

            for &k in &sizes {
                if k == 0 || self.sid[j].len() + k > SID_CAP {
                    continue;
                }

                let start = n - k;
                let block = &self.dep[i][start..n];
                let mut score = 1_400 * k as i32
                    + rng.i32_range(policy.bucket_noise)
                    + (10 - self.dep[i][n - 1] % INIT_LEN) as i32 * 70;

                let sid_head_pos = if self.sid[j].is_empty() {
                    Some(INIT_LEN)
                } else if Self::target_line(self.sid[j][0]) == j {
                    Some(self.sid[j][0] % INIT_LEN)
                } else {
                    None
                };

                let mut ordered_push = false;
                if let Some(head_pos) = sid_head_pos {
                    if head_pos >= k && self.block_is_target_range(block, j, head_pos - k) {
                        ordered_push = true;
                        score += policy.ordered_bonus + 5_000 * k as i32;
                    }
                }

                let target_pref = self.dep_prefix_len(j);
                let blocks_ready_head = self.dep_is_clean_prefix(j)
                    && target_pref < INIT_LEN
                    && !self.sid[j].is_empty()
                    && self.sid[j][0] == Self::target_id(j, target_pref);
                if blocks_ready_head && !ordered_push {
                    score -= policy.block_penalty;
                }

                candidates.push(DirectCandidate {
                    mv: Move::dep_to_siding(i, j, k),
                    score,
                });
            }
        }

        candidates
    }

    fn choose_direct5_moves(&self, policy: DirectPolicy, rng: &mut XorShift64) -> Vec<Move> {
        let candidates = self.make_direct_candidates(policy, rng);
        if candidates.is_empty() {
            return Vec::new();
        }

        let mut by_i = vec![Vec::<usize>::new(); R];
        for (idx, cand) in candidates.iter().enumerate() {
            by_i[cand.mv.i].push(idx);
        }
        for list in &mut by_i {
            list.sort_unstable_by(|&a, &b| {
                candidates[b]
                    .score
                    .cmp(&candidates[a].score)
                    .then_with(|| candidates[b].mv.k.cmp(&candidates[a].mv.k))
            });
            list.dedup_by_key(|&mut idx| {
                let mv = candidates[idx].mv;
                (mv.kind, mv.i, mv.j, mv.k)
            });
            if list.len() > 4 {
                list.truncate(4);
            }
        }

        let mut dp = vec![vec![vec![None::<(i32, Vec<usize>)>; 6]; R + 1]; R + 1];
        dp[0][0][0] = Some((0, Vec::new()));

        for i in 0..R {
            let mut next = vec![vec![None::<(i32, Vec<usize>)>; 6]; R + 1];
            for last in 0..=R {
                for cnt in 0..=5 {
                    let Some((score, path)) = dp[i][last][cnt].clone() else {
                        continue;
                    };
                    Self::relax_direct_dp(&mut next[last][cnt], score, path.clone());
                    if cnt == 5 {
                        continue;
                    }
                    for &cand_idx in &by_i[i] {
                        let cand = candidates[cand_idx];
                        let next_last = cand.mv.j + 1;
                        if next_last <= last {
                            continue;
                        }
                        let mut next_path = path.clone();
                        next_path.push(cand_idx);
                        Self::relax_direct_dp(
                            &mut next[next_last][cnt + 1],
                            score + cand.score,
                            next_path,
                        );
                    }
                }
            }
            dp[i + 1] = next;
        }

        let mut best = None::<(i32, Vec<usize>)>;
        for last in 0..=R {
            for cnt in 1..=5 {
                if let Some((score, path)) = dp[R][last][cnt].clone() {
                    let adjusted = score + 400 * cnt as i32;
                    Self::relax_direct_dp(&mut best, adjusted, path);
                }
            }
        }

        best.map_or_else(Vec::new, |(_, path)| {
            path.into_iter().map(|idx| candidates[idx].mv).collect()
        })
    }

    fn relax_direct_dp(slot: &mut Option<(i32, Vec<usize>)>, score: i32, path: Vec<usize>) {
        let replace = match slot {
            None => true,
            Some((old_score, old_path)) => {
                score > *old_score || (score == *old_score && path.len() > old_path.len())
            }
        };
        if replace {
            *slot = Some((score, path));
        }
    }

    fn target_pos_in_siding(&self, r: usize, pos: usize) -> Option<usize> {
        let target = Self::target_id(r, pos);
        self.sid[r].iter().position(|&car| car == target)
    }

    fn make_checked_scan_plan(&self, r: usize) -> Option<Plan> {
        if !self.dep_is_clean_prefix(r) {
            return None;
        }
        let pref = self.dep[r].len();
        if pref >= INIT_LEN || self.target_pos_in_siding(r, pref).is_none() {
            return None;
        }
        let plan = self.make_scan_plan(r);
        if plan.take == 0 { None } else { Some(plan) }
    }

    fn execute_one_direct_scan_batch(&mut self) -> bool {
        let mut plans = (0..R).map(|_| Plan::new()).collect::<Vec<_>>();
        let mut need_buf = Vec::new();

        for r in 0..R {
            if let Some(plan) = self.make_checked_scan_plan(r) {
                if plan.total_skip > 0 {
                    plans[r] = plan;
                    need_buf.push(r);
                }
            }
        }
        if need_buf.is_empty() {
            return false;
        }

        need_buf.sort_unstable_by(|&a, &b| {
            plans[b]
                .take
                .cmp(&plans[a].take)
                .then_with(|| plans[a].total_skip.cmp(&plans[b].total_skip))
                .then_with(|| self.dep[a].len().cmp(&self.dep[b].len()))
                .then_with(|| a.cmp(&b))
        });

        let max_a = 5usize.min(need_buf.len());
        for a in (1..=max_a).rev() {
            for start in 0..=need_buf.len() - a {
                let mut active_p = need_buf[start..start + a].to_vec();
                active_p.sort_unstable();
                let active = active_p.clone();
                let mut buffers = Vec::new();
                if self.assign_buffers_for(&active_p, &plans, &active, &mut buffers) {
                    self.execute_scan_batch(&active_p, &buffers, &[], &plans);
                    return true;
                }
            }
        }

        false
    }

    fn solve_direct5_unified(&mut self, policy: DirectPolicy, rng: &mut XorShift64) -> bool {
        let mut guard = 0usize;
        while !self.complete() {
            guard += 1;
            if guard > 260 || self.turns.len() > MAX_TURNS - 200 || !self.sidings_are_grouped() {
                return false;
            }

            let mut progressed = false;
            let moves = self.choose_direct5_moves(policy, rng);
            if !moves.is_empty() {
                self.add_turn(moves);
                progressed = true;
                if self.complete() {
                    break;
                }
            }

            if self.execute_one_direct_scan_batch() {
                progressed = true;
            }

            if !progressed {
                return false;
            }
        }

        self.complete()
    }

    fn complete(&self) -> bool {
        for r in 0..R {
            if self.dep[r].len() != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] != Self::target_id(r, c) {
                    return false;
                }
            }
        }
        true
    }

    fn solve(&mut self) {
        let start = Instant::now();
        let initial_dep = self.dep.clone();

        let mut rng = XorShift64::new(input_seed(&initial_dep));
        let mut best = None::<(Vec<Vec<usize>>, Vec<Vec<usize>>, Vec<Vec<Move>>)>;
        let mut attempt = 0usize;

        while attempt == 0 || start.elapsed().as_secs_f64() < TIME_LIMIT_SEC {
            attempt += 1;
            let mut candidate = Solver::new(initial_dep.clone());
            let policy = if attempt == 1 {
                DirectPolicy::base()
            } else {
                DirectPolicy::random(&mut rng)
            };

            if !candidate.solve_direct5_unified(policy, &mut rng) {
                continue;
            }

            if best
                .as_ref()
                .map_or(true, |(_, _, turns)| candidate.turns.len() < turns.len())
            {
                best = Some((candidate.dep, candidate.sid, candidate.turns));
            }
        }

        let (best_dep, best_sid, best_turns) =
            best.unwrap_or_else(|| panic!("direct5 unified search failed to complete"));
        self.dep = best_dep;
        self.sid = best_sid;
        self.turns = best_turns;
        assert!(self.complete());
        assert!(self.turns.len() <= MAX_TURNS);
    }
}

fn input_seed(initial: &[Vec<usize>]) -> u64 {
    let mut seed = 0x243f_6a88_85a3_08d3_u64;
    for row in initial.iter().take(R) {
        for &car in row.iter().take(INIT_LEN) {
            let v = car as u64 + 1;
            seed ^= v.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            seed = seed.rotate_left(11);
        }
    }
    seed
}

fn read_input() -> Vec<Vec<usize>> {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let r_in = it.next().unwrap().parse::<usize>().unwrap();
    assert_eq!(r_in, R);

    let mut init = vec![vec![0usize; INIT_LEN]; R];
    for row in init.iter_mut().take(R) {
        for cell in row.iter_mut().take(INIT_LEN) {
            *cell = it.next().unwrap().parse::<usize>().unwrap();
        }
    }
    init
}

fn print_turns(turns: &[Vec<Move>]) {
    let move_count = turns.iter().map(Vec::len).sum::<usize>();
    let mut out = String::with_capacity(16 + turns.len() * 4 + move_count * 16);
    out.push_str(&format!("{}\n", turns.len()));
    for moves in turns {
        out.push_str(&format!("{}\n", moves.len()));
        for mv in moves {
            out.push_str(&format!("{} {} {} {}\n", mv.kind, mv.i, mv.j, mv.k));
        }
    }

    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    stdout.write_all(out.as_bytes()).unwrap();
}

fn main() {
    let init = read_input();
    let mut solver = Solver::new(init);
    solver.solve();
    print_turns(&solver.turns);
}
