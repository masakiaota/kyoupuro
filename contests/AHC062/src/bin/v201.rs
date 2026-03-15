use proconio::input;
use rand::prelude::*;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::time::Instant;

const PHASES: usize = 10;
const TIME_LIMIT_SEC: f64 = 1.30;
const REPAIR_SLICE_SEC: f64 = 0.05;

#[derive(Clone)]
struct Problem {
    n: usize,
    m: usize,
    value: Vec<i64>,
    coord: Vec<(i32, i32)>,
    ideal_path: Vec<usize>,
}

impl Problem {
    fn new(a: Vec<Vec<usize>>) -> Self {
        let n = a.len();
        let m = n * n;
        let mut value = vec![0_i64; m];
        let mut coord = vec![(0_i32, 0_i32); m];
        let mut ideal_path = vec![0_usize; m];
        for i in 0..n {
            for j in 0..n {
                let id = i * n + j;
                let v = a[i][j];
                value[id] = v as i64;
                coord[id] = (i as i32, j as i32);
                ideal_path[v - 1] = id;
            }
        }
        Self {
            n,
            m,
            value,
            coord,
            ideal_path,
        }
    }

    #[inline]
    fn is_adjacent(&self, u: usize, v: usize) -> bool {
        let (ux, uy) = self.coord[u];
        let (vx, vy) = self.coord[v];
        let dx = (ux - vx).abs();
        let dy = (uy - vy).abs();
        (dx.max(dy) == 1) as bool
    }

    fn ideal_violation_count(&self) -> i32 {
        let mut viol = 0_i32;
        for k in 0..self.m - 1 {
            if !self.is_adjacent(self.ideal_path[k], self.ideal_path[k + 1]) {
                viol += 1;
            }
        }
        viol
    }

    fn build_baseline_path(&self) -> Vec<usize> {
        let mut path = Vec::with_capacity(self.m);
        for strip in 0..(self.n / 2) {
            let r0 = strip * 2;
            let r1 = r0 + 1;
            let cols: Box<dyn Iterator<Item = usize>> = if strip % 2 == 0 {
                Box::new(0..self.n)
            } else {
                Box::new((0..self.n).rev())
            };
            let mut idx = 0_usize;
            for c in cols {
                let top = r0 * self.n + c;
                let bottom = r1 * self.n + c;
                let endpoint = idx == 0 || idx + 1 == self.n;
                if endpoint || self.value[top] <= self.value[bottom] {
                    path.push(top);
                    path.push(bottom);
                } else {
                    path.push(bottom);
                    path.push(top);
                }
                idx += 1;
            }
        }
        path
    }
}

#[derive(Clone)]
struct State {
    path: Vec<usize>,
    pos: Vec<usize>,
    bad: Vec<u8>,
    breakpoints: Vec<usize>,
    bp_index: Vec<usize>,
    raw_score: i64,
    viol_cnt: i32,
}

impl State {
    fn from_path(problem: &Problem, path: Vec<usize>) -> Self {
        let mut pos = vec![0_usize; problem.m];
        for (idx, &id) in path.iter().enumerate() {
            pos[id] = idx;
        }
        let mut raw_score = 0_i64;
        for (idx, &id) in path.iter().enumerate() {
            raw_score += idx as i64 * problem.value[id];
        }
        let mut bad = vec![0_u8; problem.m - 1];
        let mut breakpoints = Vec::new();
        let mut bp_index = vec![usize::MAX; problem.m - 1];
        let mut viol_cnt = 0_i32;
        for k in 0..problem.m - 1 {
            if !problem.is_adjacent(path[k], path[k + 1]) {
                bad[k] = 1;
                bp_index[k] = breakpoints.len();
                breakpoints.push(k);
                viol_cnt += 1;
            }
        }
        Self {
            path,
            pos,
            bad,
            breakpoints,
            bp_index,
            raw_score,
            viol_cnt,
        }
    }

    #[inline]
    fn rounded_score(&self, m: usize) -> i64 {
        (self.raw_score + (m as i64 / 2)) / m as i64
    }

    fn set_bad(&mut self, idx: usize, new_bad: u8) {
        let old_bad = self.bad[idx];
        if old_bad == new_bad {
            return;
        }
        if old_bad == 1 {
            self.viol_cnt -= 1;
            let remove_pos = self.bp_index[idx];
            let last = self.breakpoints.pop().unwrap();
            if remove_pos < self.breakpoints.len() {
                self.breakpoints[remove_pos] = last;
                self.bp_index[last] = remove_pos;
            }
            self.bp_index[idx] = usize::MAX;
        }
        if new_bad == 1 {
            self.viol_cnt += 1;
            self.bp_index[idx] = self.breakpoints.len();
            self.breakpoints.push(idx);
        }
        self.bad[idx] = new_bad;
    }
}

#[derive(Clone)]
struct BestFeasible {
    path: Vec<usize>,
    raw_score: i64,
}

impl BestFeasible {
    fn from_state(state: &State) -> Self {
        Self {
            path: state.path.clone(),
            raw_score: state.raw_score,
        }
    }
}

#[derive(Clone)]
struct WindowMove {
    l: usize,
    new_segment: Vec<usize>,
    delta_raw: i64,
    delta_viol: i32,
}

impl WindowMove {
    #[inline]
    fn r(&self) -> usize {
        self.l + self.new_segment.len() - 1
    }

    #[inline]
    fn new_viol(&self, state: &State) -> i32 {
        state.viol_cnt + self.delta_viol
    }
}

#[derive(Default)]
struct PhaseStats {
    attempted: usize,
    accepted: usize,
    repair_viol_drop: i32,
}

fn optimize_window_exact(
    problem: &Problem,
    state: &State,
    l: usize,
    r: usize,
    lambda: i64,
) -> Option<WindowMove> {
    let cells = state.path[l..=r].to_vec();
    let k = cells.len();
    if k <= 1 || k > 12 {
        return None;
    }
    let prev = if l > 0 { Some(state.path[l - 1]) } else { None };
    let next = if r + 1 < problem.m {
        Some(state.path[r + 1])
    } else {
        None
    };
    let size = 1_usize << k;
    let neg_inf = i64::MIN / 4;
    let mut dp = vec![neg_inf; size * k];
    let mut parent = vec![u16::MAX; size * k];
    for last in 0..k {
        let mut score = l as i64 * problem.value[cells[last]];
        if let Some(p) = prev {
            if !problem.is_adjacent(p, cells[last]) {
                score -= lambda;
            }
        }
        let idx = (1_usize << last) * k + last;
        dp[idx] = score;
        parent[idx] = k as u16;
    }
    for mask in 0..size {
        if mask == 0 {
            continue;
        }
        let step = mask.count_ones() as usize;
        if step == k {
            continue;
        }
        let pos = l + step;
        let mut rest = (!mask) & (size - 1);
        while rest != 0 {
            let bit = rest & (!rest + 1);
            let nxt = bit.trailing_zeros() as usize;
            rest ^= bit;
            for last in 0..k {
                let cur = dp[mask * k + last];
                if cur == neg_inf {
                    continue;
                }
                let mut cand = cur + pos as i64 * problem.value[cells[nxt]];
                if !problem.is_adjacent(cells[last], cells[nxt]) {
                    cand -= lambda;
                }
                let new_mask = mask | bit;
                let slot = new_mask * k + nxt;
                if cand > dp[slot] {
                    dp[slot] = cand;
                    parent[slot] = last as u16;
                }
            }
        }
    }

    let full = size - 1;
    let mut best_last = None;
    let mut best_score = neg_inf;
    for last in 0..k {
        let mut score = dp[full * k + last];
        if score == neg_inf {
            continue;
        }
        if let Some(nx) = next {
            if !problem.is_adjacent(cells[last], nx) {
                score -= lambda;
            }
        }
        if score > best_score {
            best_score = score;
            best_last = Some(last);
        }
    }
    let mut last = best_last?;
    let mut mask = full;
    let mut order_idx = vec![0_usize; k];
    for t in (0..k).rev() {
        order_idx[t] = last;
        let prev_last = parent[mask * k + last];
        mask ^= 1_usize << last;
        if prev_last == k as u16 {
            break;
        }
        last = prev_last as usize;
    }
    let mut new_segment = Vec::with_capacity(k);
    for &idx in &order_idx {
        new_segment.push(cells[idx]);
    }
    let mv = evaluate_window_move(problem, state, l, new_segment);
    (mv.delta_raw != 0 || mv.delta_viol != 0).then_some(mv)
}

fn relocate_segment(window: &[usize], a: usize, b: usize, insert_pos: usize) -> Vec<usize> {
    let len = window.len();
    let seg_len = b + 1 - a;
    let mut rest = Vec::with_capacity(len - seg_len);
    rest.extend_from_slice(&window[..a]);
    rest.extend_from_slice(&window[b + 1..]);
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&rest[..insert_pos]);
    out.extend_from_slice(&window[a..=b]);
    out.extend_from_slice(&rest[insert_pos..]);
    out
}

fn reverse_segment(window: &[usize], a: usize, b: usize) -> Vec<usize> {
    let mut out = window.to_vec();
    out[a..=b].reverse();
    out
}

fn swap_segments(window: &[usize], a: usize, b: usize, c: usize, d: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(window.len());
    out.extend_from_slice(&window[..a]);
    out.extend_from_slice(&window[c..=d]);
    out.extend_from_slice(&window[b + 1..c]);
    out.extend_from_slice(&window[a..=b]);
    out.extend_from_slice(&window[d + 1..]);
    out
}

fn evaluate_window_move(
    problem: &Problem,
    state: &State,
    l: usize,
    new_segment: Vec<usize>,
) -> WindowMove {
    let r = l + new_segment.len() - 1;
    let mut old_raw = 0_i64;
    let mut new_raw = 0_i64;
    for (off, &id) in state.path[l..=r].iter().enumerate() {
        old_raw += (l + off) as i64 * problem.value[id];
    }
    for (off, &id) in new_segment.iter().enumerate() {
        new_raw += (l + off) as i64 * problem.value[id];
    }
    let edge_l = l.saturating_sub(1);
    let edge_r = r.min(problem.m - 2);
    let mut old_viol = 0_i32;
    let mut new_viol = 0_i32;
    for e in edge_l..=edge_r {
        old_viol += state.bad[e] as i32;
        let u = if e < l {
            state.path[e]
        } else {
            new_segment[e - l]
        };
        let v_pos = e + 1;
        let v = if v_pos > r {
            state.path[v_pos]
        } else {
            new_segment[v_pos - l]
        };
        if !problem.is_adjacent(u, v) {
            new_viol += 1;
        }
    }
    WindowMove {
        l,
        new_segment,
        delta_raw: new_raw - old_raw,
        delta_viol: new_viol - old_viol,
    }
}

fn apply_window_move(problem: &Problem, state: &mut State, mv: &WindowMove) {
    let l = mv.l;
    let r = mv.r();
    for (off, &id) in mv.new_segment.iter().enumerate() {
        state.path[l + off] = id;
        state.pos[id] = l + off;
    }
    state.raw_score += mv.delta_raw;
    let edge_l = l.saturating_sub(1);
    let edge_r = r.min(problem.m - 2);
    for e in edge_l..=edge_r {
        let new_bad = (!problem.is_adjacent(state.path[e], state.path[e + 1])) as u8;
        state.set_bad(e, new_bad);
    }
}

fn update_best(problem: &Problem, state: &State, best: &mut BestFeasible) {
    if state.viol_cnt == 0 && state.raw_score > best.raw_score {
        let _ = problem;
        *best = BestFeasible::from_state(state);
    }
}

fn build_budgets(initial_viol: i32) -> [i32; PHASES] {
    let capped = ((initial_viol as f64).sqrt().round() as i32).clamp(8, 64);
    let mut budgets = [0_i32; PHASES];
    for p in 0..PHASES {
        if p + 1 == PHASES {
            budgets[p] = 0;
        } else {
            let frac = 1.0 - p as f64 / (PHASES as f64 - 1.0);
            budgets[p] = ((capped as f64) * frac * frac).round() as i32;
        }
    }
    budgets
}

fn build_lambdas(scale: i64) -> [i64; PHASES] {
    let base = (scale / 20).max(1);
    let last = (scale * 5).max(base + 1);
    let mut lambdas = [0_i64; PHASES];
    for p in 0..PHASES {
        let t = p as f64 / (PHASES as f64 - 1.0);
        lambdas[p] = ((base as f64) * (1.0 - t) + (last as f64) * t).round() as i64;
    }
    lambdas
}

fn build_phase_ratios() -> [f64; PHASES] {
    [0.12, 0.12, 0.11, 0.10, 0.10, 0.10, 0.10, 0.09, 0.08, 0.08]
}

fn phase_temperature(scale: i64, progress: f64) -> f64 {
    let t0 = scale.max(1) as f64;
    let t1 = (scale.max(1) as f64 / 30.0).max(1.0);
    t0 * (t1 / t0).powf(progress.clamp(0.0, 1.0))
}

fn accept_move(delta_obj: f64, temp: f64, rng: &mut Xoshiro256PlusPlus) -> bool {
    if delta_obj >= 0.0 {
        return true;
    }
    let prob = (delta_obj / temp).exp();
    rng.random::<f64>() < prob
}

fn move_obj_delta(mv: &WindowMove, lambda: i64) -> i64 {
    mv.delta_raw - lambda * mv.delta_viol as i64
}

fn choose_breakpoint(state: &State, rng: &mut Xoshiro256PlusPlus, m: usize) -> Option<usize> {
    if state.breakpoints.is_empty() {
        return None;
    }
    let mut best_bp = state.breakpoints[rng.random_range(0..state.breakpoints.len())];
    let mut best_priority = breakpoint_priority(state, best_bp, m);
    for _ in 0..4 {
        let bp = state.breakpoints[rng.random_range(0..state.breakpoints.len())];
        let pr = breakpoint_priority(state, bp, m);
        if pr > best_priority {
            best_priority = pr;
            best_bp = bp;
        }
    }
    Some(best_bp)
}

fn breakpoint_priority(state: &State, bp: usize, m: usize) -> i32 {
    let mut pr = 1_i32;
    if bp > 0 && state.bad[bp - 1] == 1 {
        pr += 2;
    }
    if bp + 1 < m - 1 && state.bad[bp + 1] == 1 {
        pr += 2;
    }
    if bp < 32 || bp + 32 >= m - 1 {
        pr += 2;
    }
    pr
}

fn search_hotspot_window(
    problem: &Problem,
    state: &State,
    bp: usize,
    lambda: i64,
    radius: usize,
    repair_mode: bool,
) -> Option<WindowMove> {
    let l = bp.saturating_sub(radius);
    let r = (bp + 1 + radius).min(problem.m - 1);
    let window = &state.path[l..=r];
    let len = window.len();
    if len < 2 {
        return None;
    }
    let pivot = bp - l;
    let mut best: Option<WindowMove> = None;
    let mut best_key = i64::MIN;
    let lambda_eff = if repair_mode { lambda * 3 } else { lambda };

    let exact_half = if repair_mode { 5 } else { 4 };
    let exact_l = bp.saturating_sub(exact_half);
    let exact_r = (bp + 1 + exact_half).min(problem.m - 1);
    if let Some(mv) = optimize_window_exact(problem, state, exact_l, exact_r, lambda_eff) {
        let key = move_obj_delta(&mv, lambda_eff);
        if should_take_hotspot(state, &mv, key, best_key, repair_mode) {
            best_key = key;
            best = Some(mv);
        }
    }

    let rev_left = pivot.saturating_sub(10);
    let rev_right = (pivot + 10).min(len - 1);
    for a in rev_left..=rev_right {
        let end_max = (a + 18).min(len - 1);
        for b in (a + 1)..=end_max {
            if b + 1 < pivot || a > pivot + 1 {
                continue;
            }
            let new_segment = reverse_segment(window, a, b);
            let mv = evaluate_window_move(problem, state, l, new_segment);
            let key = move_obj_delta(&mv, lambda_eff);
            if should_take_hotspot(state, &mv, key, best_key, repair_mode) {
                best_key = key;
                best = Some(mv);
            }
        }
    }

    let src_left = pivot.saturating_sub(8);
    let src_right = (pivot + 2).min(len - 1);
    for a in src_left..=src_right {
        for seg_len in 1..=8 {
            let b = a + seg_len - 1;
            if b >= len {
                break;
            }
            if b + 1 < pivot || a > pivot + 1 {
                continue;
            }
            let rest_len = len - seg_len;
            for insert_pos in 0..=rest_len {
                let source_insert_pos = if insert_pos <= a {
                    insert_pos
                } else {
                    insert_pos + seg_len
                };
                if source_insert_pos == a || source_insert_pos == b + 1 {
                    continue;
                }
                let new_segment = relocate_segment(window, a, b, insert_pos);
                let mv = evaluate_window_move(problem, state, l, new_segment);
                let key = move_obj_delta(&mv, lambda_eff);
                if should_take_hotspot(state, &mv, key, best_key, repair_mode) {
                    best_key = key;
                    best = Some(mv);
                }
            }
        }
    }

    let left_a = pivot.saturating_sub(8);
    let left_b = (pivot + 1).min(len - 3);
    for a in left_a..=left_b {
        for left_len in 1..=5 {
            let b = a + left_len - 1;
            if b >= len {
                break;
            }
            let gap_start = b + 1;
            let gap_end = (b + 9).min(len - 2);
            for c in gap_start..=gap_end {
                for right_len in 1..=5 {
                    let d = c + right_len - 1;
                    if d >= len {
                        break;
                    }
                    if d + 1 < pivot || a > pivot + 1 {
                        continue;
                    }
                    let new_segment = swap_segments(window, a, b, c, d);
                    let mv = evaluate_window_move(problem, state, l, new_segment);
                    let key = move_obj_delta(&mv, lambda_eff);
                    if should_take_hotspot(state, &mv, key, best_key, repair_mode) {
                        best_key = key;
                        best = Some(mv);
                    }
                }
            }
        }
    }

    best
}

fn should_take_hotspot(
    state: &State,
    mv: &WindowMove,
    key: i64,
    best_key: i64,
    repair_mode: bool,
) -> bool {
    if !repair_mode {
        return key > best_key;
    }
    let new_viol = mv.new_viol(state);
    let best_improves = best_key > i64::MIN;
    if new_viol < state.viol_cnt {
        return true;
    }
    if new_viol == state.viol_cnt && mv.delta_raw > 0 {
        return !best_improves || key > best_key;
    }
    !best_improves && key > best_key
}

fn sample_rank_move(
    problem: &Problem,
    state: &State,
    lambda: i64,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<WindowMove> {
    let mut best_p = rng.random_range(0..problem.m);
    let mut best_gap = rank_gap(problem, state, best_p).abs();
    for _ in 0..7 {
        let p = rng.random_range(0..problem.m);
        let gap = rank_gap(problem, state, p).abs();
        if gap > best_gap {
            best_gap = gap;
            best_p = p;
        }
    }
    if best_gap == 0 {
        return None;
    }
    let gap_signed = rank_gap(problem, state, best_p);
    let dir = if gap_signed > 0 { -1_i32 } else { 1_i32 };
    let radius = best_gap.unsigned_abs().min(48) as usize + 8;
    let l = best_p.saturating_sub(radius);
    let r = (best_p + radius).min(problem.m - 1);
    let exact_l = best_p.saturating_sub(4);
    let exact_r = (best_p + 4).min(problem.m - 1);
    let mut best = optimize_window_exact(problem, state, exact_l, exact_r, lambda);
    let mut best_key = best
        .as_ref()
        .map(|mv| move_obj_delta(mv, lambda))
        .unwrap_or(i64::MIN);
    let window = &state.path[l..=r];
    let len = window.len();
    let p_local = best_p - l;

    for seg_len in 1..=4 {
        let a = p_local.saturating_sub(seg_len - 1);
        let b = (a + seg_len - 1).min(len - 1);
        let shifts = [4_usize, 8, 12, 16, 24, 32, 40];
        for &shift in &shifts {
            let anchor = if dir < 0 {
                a.saturating_sub(shift)
            } else {
                (b + 1 + shift).min(len)
            };
            let insert_pos = if dir < 0 {
                anchor
            } else {
                anchor.saturating_sub(seg_len)
            };
            if insert_pos > len - (b + 1 - a) {
                continue;
            }
            let new_segment = relocate_segment(window, a, b, insert_pos);
            let mv = evaluate_window_move(problem, state, l, new_segment);
            let key = move_obj_delta(&mv, lambda);
            if key > best_key {
                best_key = key;
                best = Some(mv);
            }
        }
    }

    for shift in [4_usize, 8, 12, 16, 24, 32] {
        let q = if dir < 0 {
            p_local.saturating_sub(shift)
        } else {
            (p_local + shift).min(len - 1)
        };
        if q == p_local {
            continue;
        }
        let (a, b) = if q < p_local {
            (q, p_local)
        } else {
            (p_local, q)
        };
        let new_segment = reverse_segment(window, a, b);
        let mv = evaluate_window_move(problem, state, l, new_segment);
        let key = move_obj_delta(&mv, lambda);
        if key > best_key {
            best_key = key;
            best = Some(mv);
        }
    }

    best
}

fn rank_gap(problem: &Problem, state: &State, pos: usize) -> i32 {
    let id = state.path[pos];
    pos as i32 - (problem.value[id] as i32 - 1)
}

fn sample_random_move(
    problem: &Problem,
    state: &State,
    lambda: i64,
    rng: &mut Xoshiro256PlusPlus,
) -> Option<WindowMove> {
    let span = rng.random_range(18..=56);
    let l = rng.random_range(0..=problem.m - span);
    let r = l + span - 1;
    let exact_len = rng.random_range(7..=10);
    let exact_l = rng.random_range(0..=problem.m - exact_len);
    if let Some(mv) =
        optimize_window_exact(problem, state, exact_l, exact_l + exact_len - 1, lambda)
    {
        return Some(mv);
    }
    let window = &state.path[l..=r];
    let len = window.len();
    match rng.random_range(0..3) {
        0 => {
            let a = rng.random_range(0..len - 1);
            let b = rng.random_range(a + 1..len);
            let new_segment = reverse_segment(window, a, b);
            let mv = evaluate_window_move(problem, state, l, new_segment);
            (move_obj_delta(&mv, lambda) > i64::MIN).then_some(mv)
        }
        1 => {
            let a = rng.random_range(0..len - 1);
            let seg_len = rng.random_range(1..=6).min(len - a);
            let b = a + seg_len - 1;
            let rest_len = len - seg_len;
            let insert_pos = rng.random_range(0..=rest_len);
            let new_segment = relocate_segment(window, a, b, insert_pos);
            let mv = evaluate_window_move(problem, state, l, new_segment);
            (move_obj_delta(&mv, lambda) > i64::MIN).then_some(mv)
        }
        _ => {
            if len < 8 {
                return None;
            }
            let a = rng.random_range(0..=len - 6);
            let left_len = rng.random_range(1..=3).min(len - 4 - a);
            let b = a + left_len - 1;
            let c_min = b + 2;
            if c_min > len - 2 {
                return None;
            }
            let c = rng.random_range(c_min..=len - 2);
            let right_len = rng.random_range(1..=3).min(len - c);
            let d = c + right_len - 1;
            let new_segment = swap_segments(window, a, b, c, d);
            let mv = evaluate_window_move(problem, state, l, new_segment);
            (move_obj_delta(&mv, lambda) > i64::MIN).then_some(mv)
        }
    }
}

fn calibrate_scale(problem: &Problem, state: &State, rng: &mut Xoshiro256PlusPlus) -> i64 {
    let mut deltas = Vec::with_capacity(96);
    for _ in 0..96 {
        if let Some(mv) = sample_random_move(problem, state, 0, rng) {
            deltas.push(mv.delta_raw.abs());
        }
    }
    if deltas.is_empty() {
        return 1;
    }
    deltas.sort_unstable();
    deltas[deltas.len() / 2].max(1)
}

fn legal_local_pass(
    problem: &Problem,
    state: &mut State,
    best: &mut BestFeasible,
    deadline: Instant,
) {
    let legal_lambda = 1_000_000_000_000_i64;
    let width = 8_usize;
    let stride = 4_usize;
    for offset in [0_usize, 2] {
        let mut l = offset;
        while l + width <= problem.m && Instant::now() < deadline {
            if let Some(mv) = optimize_window_exact(problem, state, l, l + width - 1, legal_lambda)
            {
                if mv.new_viol(state) == 0 && mv.delta_raw > 0 {
                    apply_window_move(problem, state, &mv);
                    update_best(problem, state, best);
                }
            }
            l += stride;
        }
        if Instant::now() >= deadline {
            break;
        }
    }
}

fn sweep_repair_pass(
    problem: &Problem,
    state: &mut State,
    best: &mut BestFeasible,
    deadline: Instant,
    width: usize,
    stride: usize,
) -> bool {
    let legal_lambda = 1_000_000_000_000_i64;
    let mut improved = false;
    for offset in 0..stride.min(width) {
        let mut l = offset;
        while l + width <= problem.m && Instant::now() < deadline {
            if let Some(mv) = optimize_window_exact(problem, state, l, l + width - 1, legal_lambda)
            {
                let new_viol = mv.new_viol(state);
                if new_viol < state.viol_cnt
                    || (new_viol == 0 && state.viol_cnt == 0 && mv.delta_raw > 0)
                {
                    apply_window_move(problem, state, &mv);
                    update_best(problem, state, best);
                    improved = true;
                }
            }
            l += stride;
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    improved
}

fn force_repair(
    problem: &Problem,
    state: &mut State,
    best: &mut BestFeasible,
    lambda: i64,
    target_budget: i32,
    deadline: Instant,
    depth_limit: usize,
    stats: &mut PhaseStats,
    rng: &mut Xoshiro256PlusPlus,
) {
    let mut depth = 0_usize;
    let mut stagnation = 0_usize;
    while Instant::now() < deadline && state.viol_cnt > target_budget {
        let Some(bp) = choose_breakpoint(state, rng, problem.m) else {
            break;
        };
        let radius = 28 + depth * 16;
        let before = state.viol_cnt;
        let candidate = search_hotspot_window(problem, state, bp, lambda, radius, true);
        let Some(mv) = candidate else {
            stagnation += 1;
            if stagnation >= 4 && depth < depth_limit {
                depth += 1;
                stagnation = 0;
            }
            continue;
        };
        let new_viol = mv.new_viol(state);
        if new_viol > state.viol_cnt {
            stagnation += 1;
            continue;
        }
        if new_viol > target_budget && new_viol == state.viol_cnt && mv.delta_raw <= 0 {
            stagnation += 1;
            continue;
        }
        apply_window_move(problem, state, &mv);
        update_best(problem, state, best);
        let reduced = before - state.viol_cnt;
        if reduced > 0 {
            stats.repair_viol_drop += reduced;
            stagnation = 0;
        } else {
            stagnation += 1;
        }
        if stagnation >= 4 && depth < depth_limit {
            depth += 1;
            stagnation = 0;
        }
    }
}

fn seed_from_problem(problem: &Problem) -> u64 {
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    for (idx, &id) in problem.ideal_path.iter().take(64).enumerate() {
        seed ^= ((id as u64) << (idx % 17))
            ^ (problem.value[id] as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        seed = seed.rotate_left(13) ^ 0x94D0_49BB_1331_11EB_u64;
    }
    seed
}

fn main() {
    input! {
        n: usize,
        a: [[usize; n]; n],
    }
    let problem = Problem::new(a);
    let start = Instant::now();

    let ideal_viol = problem.ideal_violation_count();
    let baseline_path = problem.build_baseline_path();
    let mut state = State::from_path(&problem, baseline_path);
    let mut best = BestFeasible::from_state(&state);

    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed_from_problem(&problem));
    let scale = calibrate_scale(&problem, &state, &mut rng);
    let budgets = build_budgets(ideal_viol);
    let lambdas = build_lambdas(scale);
    let phase_ratios = build_phase_ratios();
    let search_budget_sec = (TIME_LIMIT_SEC - REPAIR_SLICE_SEC).max(0.20);

    let warmup_deadline = start + std::time::Duration::from_secs_f64(0.06);
    legal_local_pass(&problem, &mut state, &mut best, warmup_deadline);
    let debug_log = std::env::var_os("AHC062_DEBUG").is_some();

    for phase in 0..PHASES {
        let phase_begin_ratio = phase_ratios[..phase].iter().sum::<f64>();
        let phase_end_ratio = phase_ratios[..=phase].iter().sum::<f64>();
        let phase_end_sec = search_budget_sec * phase_end_ratio;
        let phase_end = start + std::time::Duration::from_secs_f64(phase_end_sec);
        let lambda = lambdas[phase];
        let budget = budgets[phase];
        let next_budget = if phase + 1 < PHASES {
            budgets[phase + 1]
        } else {
            0
        };
        let mut stats = PhaseStats::default();

        while Instant::now() < phase_end {
            stats.attempted += 1;
            let elapsed = start.elapsed().as_secs_f64();
            let progress = (elapsed / search_budget_sec).clamp(0.0, 1.0);
            let temp = phase_temperature(scale, progress);
            let phase_progress = ((elapsed / search_budget_sec) - phase_begin_ratio)
                / (phase_end_ratio - phase_begin_ratio).max(1e-9);
            let repair_weight = if phase >= PHASES - 3 { 0.70 } else { 0.60 };
            let rank_weight = if phase < 3 {
                0.30
            } else if phase >= PHASES - 3 {
                0.15
            } else {
                0.25
            };
            let roll = rng.random::<f64>();
            let candidate = if roll < repair_weight {
                choose_breakpoint(&state, &mut rng, problem.m)
                    .and_then(|bp| {
                        search_hotspot_window(&problem, &state, bp, lambda, 18 + phase * 3, false)
                    })
                    .or_else(|| sample_rank_move(&problem, &state, lambda, &mut rng))
            } else if roll < repair_weight + rank_weight {
                sample_rank_move(&problem, &state, lambda, &mut rng)
                    .or_else(|| sample_random_move(&problem, &state, lambda, &mut rng))
            } else {
                sample_random_move(&problem, &state, lambda, &mut rng)
                    .or_else(|| sample_rank_move(&problem, &state, lambda, &mut rng))
            };
            let Some(mv) = candidate else {
                continue;
            };
            let new_viol = mv.new_viol(&state);
            if new_viol > budget {
                continue;
            }
            let delta_obj = move_obj_delta(&mv, lambda) as f64;
            if accept_move(delta_obj, temp.max(1.0), &mut rng) {
                apply_window_move(&problem, &mut state, &mv);
                update_best(&problem, &state, &mut best);
                stats.accepted += 1;
            }
            if phase_progress > 0.85 && state.viol_cnt > next_budget {
                let repair_deadline = Instant::now()
                    + std::time::Duration::from_secs_f64(0.002 + 0.001 * phase as f64);
                force_repair(
                    &problem,
                    &mut state,
                    &mut best,
                    lambda,
                    next_budget,
                    repair_deadline.min(phase_end),
                    if phase + 1 < PHASES { 1 } else { 2 },
                    &mut stats,
                    &mut rng,
                );
            }
        }

        let repair_deadline =
            Instant::now() + std::time::Duration::from_secs_f64(0.004 + 0.002 * phase as f64);
        force_repair(
            &problem,
            &mut state,
            &mut best,
            lambda,
            next_budget,
            repair_deadline.min(start + std::time::Duration::from_secs_f64(search_budget_sec)),
            if phase < 6 { 2 } else { 4 },
            &mut stats,
            &mut rng,
        );
        let sweep_deadline =
            Instant::now() + std::time::Duration::from_secs_f64(0.003 + 0.0015 * phase as f64);
        let _ = sweep_repair_pass(
            &problem,
            &mut state,
            &mut best,
            sweep_deadline.min(start + std::time::Duration::from_secs_f64(search_budget_sec)),
            10,
            5,
        );

        let bp_density = state.viol_cnt as f64 / (problem.m - 1) as f64;
        if debug_log {
            eprintln!(
                "phase={} lambda={} budget={} best={} curr_raw={} curr_viol={} curr_obj={} accepted={}/{} bp_density={:.6} repair_drop={}",
                phase,
                lambda,
                budget,
                (best.raw_score + problem.m as i64 / 2) / problem.m as i64,
                state.rounded_score(problem.m),
                state.viol_cnt,
                state.raw_score - lambda * state.viol_cnt as i64,
                stats.accepted,
                stats.attempted,
                bp_density,
                stats.repair_viol_drop
            );
        }
    }

    let final_deadline = start + std::time::Duration::from_secs_f64(TIME_LIMIT_SEC);
    let mut final_stats = PhaseStats::default();
    force_repair(
        &problem,
        &mut state,
        &mut best,
        lambdas[PHASES - 1] * 4,
        0,
        final_deadline,
        6,
        &mut final_stats,
        &mut rng,
    );
    while Instant::now() < final_deadline {
        if !sweep_repair_pass(&problem, &mut state, &mut best, final_deadline, 10, 5) {
            break;
        }
    }
    update_best(&problem, &state, &mut best);

    let out_path = if state.viol_cnt == 0 && state.raw_score > best.raw_score {
        &state.path
    } else {
        &best.path
    };
    for &id in out_path {
        let i = id / problem.n;
        let j = id % problem.n;
        println!("{i} {j}");
    }
}
