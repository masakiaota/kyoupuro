// v999_champion_code.rs
use proconio::input;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::io::Write;
use std::time::Instant;

const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

const N: usize = 200;
const M: usize = 10;
const PER_PILE: usize = N / M;
const N_VAL: u16 = N as u16;

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
    time_limit_sec: f64,
}

impl TimeKeeper {
    fn new(time_limit_sec: f64) -> Self {
        Self {
            start: Instant::now(),
            time_limit_sec,
        }
    }

    #[inline]
    fn is_time_over(&self) -> bool {
        self.start.elapsed().as_secs_f64() >= self.time_limit_sec
    }

    #[cfg(feature = "local")]
    #[inline]
    fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}

#[derive(Debug, Clone, Copy)]
struct Operation {
    v: u16,
    from: u8,
    to: i8,
}

impl Operation {
    fn new(v: u16, from: usize, to: Option<usize>) -> Self {
        Self {
            v,
            from: from as u8,
            to: to.map_or(-1, |x| x as i8),
        }
    }
}

#[derive(Debug, Clone)]
struct State {
    cost: i32,
    b: Vec<Vec<u16>>,
    ops: Vec<Operation>,
    next_pop: u16,
}

impl State {
    fn new(b: Vec<Vec<u16>>) -> Self {
        Self {
            cost: 0,
            b,
            ops: Vec::with_capacity(512),
            next_pop: 0,
        }
    }

    fn calc_b_hash(&self) -> i32 {
        const MD: i64 = 998244353;
        const BASE: i64 = 9999;
        let mut res = 0_i64;
        for col in 0..M {
            for &v in &self.b[col] {
                res = (res * BASE + v as i64) % MD;
            }
            res = (res * BASE + N as i64) % MD;
        }
        res as i32
    }

    #[inline]
    fn get_top_val(&self, col: usize) -> u16 {
        self.b[col].last().copied().unwrap_or(N_VAL)
    }

    #[inline]
    fn is_done(&self) -> bool {
        self.next_pop as usize == N
    }

    fn can_pop(&self) -> bool {
        if self.is_done() {
            return false;
        }
        self.b.iter().any(|v| v.last() == Some(&self.next_pop))
    }

    fn is_sorted(&self, col: usize) -> bool {
        let v = &self.b[col];
        for i in (1..v.len()).rev() {
            if v[i - 1] < v[i] {
                return false;
            }
        }
        true
    }

    fn sorted_len(&self, col: usize) -> usize {
        let v = &self.b[col];
        if v.is_empty() {
            return 0;
        }
        let mut ret = 1;
        while ret < v.len() && v[ret - 1] > v[ret] {
            ret += 1;
        }
        ret
    }

    fn find_column(&self, val: u16) -> usize {
        for col in 0..M {
            if self.b[col].contains(&val) {
                return col;
            }
        }
        panic!("value not found: {val}");
    }

    fn move_value(&mut self, val: u16, from: usize, to: usize) {
        let mut length = 1;
        while self.b[from][self.b[from].len() - length] != val {
            length += 1;
        }
        self.move_by_count(length, from, to);
    }

    fn move_by_count(&mut self, length: usize, from: usize, to: usize) {
        debug_assert!(from < M && to < M && from != to);
        debug_assert!(self.b[from].len() >= length);

        let split_at = self.b[from].len() - length;
        let val = self.b[from][split_at];
        let moved = self.b[from].split_off(split_at);
        self.b[to].extend_from_slice(&moved);

        let add_cost = 1 + length as i32;
        if let Some(last) = self.ops.last_mut() {
            if last.v == val {
                debug_assert_eq!(last.to, from as i8);
                let original_from = last.from as usize;
                if original_from == to {
                    self.cost -= add_cost;
                    self.ops.pop();
                } else {
                    last.to = to as i8;
                }
                return;
            }
        }

        self.cost += add_cost;
        self.ops.push(Operation::new(val, from, Some(to)));
    }

    fn popnext(&mut self) {
        let mut col = 0;
        while self.b[col].last() != Some(&self.next_pop) {
            col += 1;
        }
        self.b[col].pop();
        self.ops
            .push(Operation::new(self.next_pop, col, None));
        self.next_pop += 1;
    }
}

#[derive(Debug, Clone)]
struct XorShift {
    x: u32,
    y: u32,
    z: u32,
    w: u32,
}

impl XorShift {
    fn new() -> Self {
        Self {
            x: 123456789,
            y: 362436069,
            z: 521288629,
            w: 88675123,
        }
    }

    fn next_u32(&mut self) -> u32 {
        let t = self.x ^ (self.x << 11);
        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        self.w = (self.w ^ (self.w >> 19)) ^ (t ^ (t >> 8));
        self.w
    }
}

fn erase_v(val: u16, s: &mut State) {
    debug_assert_eq!(s.next_pop, val);
    let cur = s.find_column(val);

    if s.b[cur].last() != Some(&val) {
        let c = s.b[cur].iter().position(|&x| x == val).unwrap();
        let rem_sz = c + 1;

        while s.b[cur].len() > rem_sz {
            let vec = &s.b[cur];
            let mut l = vec.len() - 1;
            while l > rem_sz && vec[l - 1] > vec[l] {
                l -= 1;
            }

            let mv_bottom = s.b[cur][l];
            let mut best_dest = usize::MAX;
            let mut best_cost = i32::MAX;
            for col in 0..M {
                if col == cur {
                    continue;
                }

                let eval = if s.b[col].is_empty() {
                    1000
                } else if s.get_top_val(col) > mv_bottom {
                    s.get_top_val(col) as i32 - mv_bottom as i32
                } else {
                    10000 + mv_bottom as i32 - s.get_top_val(col) as i32
                };

                if eval < best_cost {
                    best_cost = eval;
                    best_dest = col;
                }
            }
            s.move_value(mv_bottom, cur, best_dest);
        }
    }

    s.popnext();
}

fn erase_all(s: &mut State) {
    while !s.is_done() {
        erase_v(s.next_pop, s);
    }
}

fn eval2(a: u16, b: u16, nin: i32) -> i32 {
    let mut top_diff = b as i32 - a as i32;
    if top_diff < 0 {
        top_diff += nin;
    } else {
        top_diff -= nin;
    }
    debug_assert_ne!(top_diff, 0);

    if top_diff < 0 {
        0
    } else if top_diff < 4 {
        (1900.0 - 300.0 * top_diff as f64) as i32
    } else if top_diff < 10 {
        (1000.0 - (top_diff as f64).powf(1.2) * 50.0) as i32
    } else {
        -1000 - top_diff
    }
}

fn evaluate_state(s: &State) -> i32 {
    let num_popped = s.next_pop as i32;
    let mut total_sorted = 0_i32;
    let mut cs = vec![0_i32; N];

    for col in 0..M {
        let sorted_len = s.sorted_len(col);
        total_sorted += sorted_len as i32;
        for i in 0..sorted_len {
            cs[s.b[col][i] as usize] += 1;
        }
    }

    for i in 1..N {
        cs[i] += cs[i - 1];
    }

    let mut adj_evals = 0_i32;
    for col in 0..M {
        let sorted_len = s.sorted_len(col);
        let v = &s.b[col];
        for i in sorted_len..v.len().saturating_sub(1) {
            let b = v[i];
            let a = v[i + 1];
            let lo = a.min(b) as usize;
            let hi = a.max(b) as usize;
            adj_evals += eval2(a, b, cs[hi - 1] - cs[lo]);
        }
    }

    adj_evals + total_sorted * 2300 + num_popped * 2300 - s.cost * 1000
}

#[derive(Debug, Clone)]
struct UnionFind {
    par: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            par: (0..n).collect(),
            size: vec![1; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.par[x] == x {
            x
        } else {
            let r = self.find(self.par[x]);
            self.par[x] = r;
            r
        }
    }

    fn unite(&mut self, a: usize, b: usize) -> bool {
        let mut x = self.find(a);
        let mut y = self.find(b);
        if x == y {
            return false;
        }
        if self.size[x] < self.size[y] {
            std::mem::swap(&mut x, &mut y);
        }
        self.par[y] = x;
        self.size[x] += self.size[y];
        true
    }
}

fn pileup(s: &mut State) {
    loop {
        while s.can_pop() {
            s.popnext();
        }

        let mut sorted_cols = Vec::new();
        let mut unsorted_cols = Vec::new();
        for col in 0..M {
            if s.is_sorted(col) {
                sorted_cols.push(col);
            } else {
                unsorted_cols.push(col);
            }
        }

        let mut best_from = usize::MAX;
        let mut best_to = usize::MAX;
        let mut best_len = 0_usize;
        let mut best_eval = i32::MIN / 4;

        for &to in &sorted_cols {
            let top = s.get_top_val(to);
            let mut cands: Vec<(u16, u16, usize, usize)> = Vec::new();

            for &from in &unsorted_cols {
                let vec = &s.b[from];
                if vec.is_empty() || top < *vec.last().unwrap() {
                    continue;
                }

                let mut len = 1_usize;
                let mut x = *vec.last().unwrap();
                while len < vec.len() {
                    let y = vec[vec.len() - len - 1];
                    if x < y && y < top {
                        len += 1;
                        x = y;
                    } else {
                        break;
                    }
                }

                if len < vec.len() {
                    let y = vec[vec.len() - len - 1];
                    if y > x && y <= x + 3 {
                        continue;
                    }
                }

                cands.push((vec[vec.len() - len], *vec.last().unwrap(), len, from));
            }
            if cands.is_empty() {
                continue;
            }

            cands.sort_by(|a, b| b.1.cmp(&a.1));

            let (_, min0, len0, from0) = cands[0];
            let mut nxtidx = 1;
            while nxtidx < cands.len() && cands[nxtidx].0 > min0 {
                nxtidx += 1;
            }

            let mut nxtmax = 0_u16;
            let mut nxtmin = N_VAL;
            let mut nxtlen = 0_usize;
            if nxtidx < cands.len() {
                nxtmax = cands[nxtidx].0;
                nxtmin = cands[nxtidx].1;
                nxtlen = cands[nxtidx].2;
            }

            let mut tmpmin = min0;
            let mut tmpfrom = from0;
            let mut tmplen = len0;

            if cands.len() > 1 {
                let (_, min1, len1, from1) = cands[1];
                if min1 > nxtmax && len1 > tmplen {
                    tmpmin = min1;
                    tmpfrom = from1;
                    tmplen = len1;
                } else if min1 >= nxtmin && len1 >= len0 + nxtlen {
                    tmpmin = min1;
                    tmpfrom = from1;
                    tmplen = len1;
                }
            }

            let eval = -(top as i32 - tmpmin as i32);
            if eval > best_eval {
                best_eval = eval;
                best_from = tmpfrom;
                best_to = to;
                best_len = tmplen;
            }
        }

        if best_from == usize::MAX {
            break;
        }

        s.move_by_count(best_len, best_from, best_to);
    }
}

fn augment(
    c: &[Vec<i32>],
    f: &mut [i32],
    g: &mut [i32],
    s: usize,
    mate: &mut [isize],
    mate_inv: &mut [isize],
    fixrows: usize,
) -> i32 {
    let nr = f.len();
    let nc = g.len();
    debug_assert!(s < nr);
    debug_assert!(mate[s] < 0);

    let mut dist = vec![0_i32; nc];
    let mut prv = vec![-1_isize; nc];
    let mut done = vec![false; nc];

    for i in 0..fixrows {
        let j = mate[i];
        if j >= 0 {
            done[j as usize] = true;
        }
    }

    let h = (0..nc).find(|&j| !done[j]).unwrap();
    f[s] = c[s][h] - g[h];
    for j in h + 1..nc {
        if !done[j] {
            f[s] = f[s].min(c[s][j] - g[j]);
        }
    }

    for j in 0..nc {
        if !done[j] {
            dist[j] = c[s][j] - f[s] - g[j];
            prv[j] = -1;
        }
    }

    let mut t: isize = -1;
    while t == -1 {
        let mut j1 = usize::MAX;
        for j in 0..nc {
            if done[j] {
                continue;
            }
            if j1 == usize::MAX
                || dist[j] < dist[j1]
                || (dist[j] == dist[j1] && mate_inv[j] < 0)
            {
                j1 = j;
            }
        }

        if mate_inv[j1] < 0 {
            t = j1 as isize;
            break;
        }

        done[j1] = true;
        let mut stack = vec![j1];
        while let Some(j2) = stack.pop() {
            let i = mate_inv[j2];
            if i < 0 {
                t = j2 as isize;
                break;
            }
            let i = i as usize;

            for j in 0..nc {
                if done[j] {
                    continue;
                }

                let len = c[i][j] - f[i] - g[j];
                if dist[j] > dist[j1] + len {
                    dist[j] = dist[j1] + len;
                    prv[j] = j2 as isize;
                }

                if len == 0 {
                    stack.push(j);
                    done[j] = true;
                }
            }
        }
    }

    let len = dist[t as usize];
    f[s] += len;

    for i in 0..fixrows {
        let j = mate[i];
        if j >= 0 {
            done[j as usize] = false;
        }
    }

    for j in 0..nc {
        if done[j] {
            g[j] -= len - dist[j];
        }
    }

    for i in fixrows..nr {
        let j = mate[i];
        if j >= 0 && done[j as usize] {
            f[i] += len - dist[j as usize];
        }
    }

    let mut ret = 0_i32;
    let mut cur = t;
    while cur >= 0 {
        let nxt = prv[cur as usize];
        if nxt < 0 {
            mate_inv[cur as usize] = s as isize;
            mate[s] = cur;
            ret += c[s][cur as usize];
            break;
        }
        let i = mate_inv[nxt as usize] as usize;
        ret += c[i][cur as usize] - c[i][nxt as usize];
        mate_inv[cur as usize] = i as isize;
        mate[i] = cur;
        cur = nxt;
    }

    ret
}

fn linear_sum_assignment(c: &[Vec<i32>]) -> (i32, Vec<isize>, Vec<i32>, Vec<i32>) {
    let nr = c.len();
    let nc = if nr == 0 { 0 } else { c[0].len() };
    let mut mate = vec![-1_isize; nr];
    let mut mate_inv = vec![-1_isize; nc];
    let mut f = vec![0_i32; nr];
    let mut g = vec![0_i32; nc];

    if nr == 0 || nc == 0 {
        return (0, mate, f, g);
    }
    debug_assert!(nr <= nc);

    for i in 0..nr {
        if mate[i] < 0 {
            augment(c, &mut f, &mut g, i, &mut mate, &mut mate_inv, 0);
        }
    }

    let mut ret = 0_i32;
    for i in 0..nr {
        ret += c[i][mate[i] as usize];
    }
    (ret, mate, f, g)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AssignmentNode {
    opt: i32,
    mate: Vec<isize>,
    f: Vec<i32>,
    g: Vec<i32>,
    fixed_rows: usize,
    banned_js: Vec<usize>,
}

impl Ord for AssignmentNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .opt
            .cmp(&self.opt)
            .then_with(|| other.fixed_rows.cmp(&self.fixed_rows))
    }
}

impl PartialOrd for AssignmentNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct BestAssignments {
    nr: usize,
    nc: usize,
    inf: i32,
    c: Vec<Vec<i32>>,
    ctmp: Vec<Vec<i32>>,
    pq: BinaryHeap<AssignmentNode>,
}

impl BestAssignments {
    fn new(nr: usize, nc: usize, cost: &[Vec<i32>], inf: i32) -> Self {
        debug_assert!(nr <= nc);
        let rows = nr + usize::from(nr != nc);
        let mut c = vec![vec![0_i32; nc]; rows];
        for i in 0..nr {
            for j in 0..nc {
                c[i][j] = cost[i][j];
            }
        }
        let ctmp = c.clone();
        let (opt, mate, f, g) = linear_sum_assignment(&c);
        let mut pq = BinaryHeap::new();
        pq.push(AssignmentNode {
            opt,
            mate,
            f,
            g,
            fixed_rows: 0,
            banned_js: Vec::new(),
        });
        Self {
            nr,
            nc,
            inf,
            c,
            ctmp,
            pq,
        }
    }

    fn finished(&self) -> bool {
        self.pq.is_empty()
    }

    fn yield_assignment(&mut self) -> (i32, Vec<isize>) {
        let ret = self.pq.pop().unwrap();

        for fixed_rows in ret.fixed_rows..self.nr {
            let mut banned_js = if fixed_rows == ret.fixed_rows {
                ret.banned_js.clone()
            } else {
                Vec::new()
            };

            let s = fixed_rows;
            let old_j = ret.mate[s] as usize;
            banned_js.push(old_j);
            if banned_js.len() >= self.nc {
                continue;
            }

            let mut f = ret.f.clone();
            let mut g = ret.g.clone();
            let mut mate = ret.mate.clone();
            let mut mate_inv = vec![self.nr as isize; self.nc];
            for i in 0..self.nr {
                mate_inv[mate[i] as usize] = i as isize;
            }

            for &j in &banned_js {
                self.ctmp[s][j] = self.inf;
            }

            mate_inv[mate[s] as usize] = -1;
            mate[s] = -1;

            let aug = augment(
                &self.ctmp,
                &mut f,
                &mut g,
                s,
                &mut mate,
                &mut mate_inv,
                fixed_rows,
            );

            for j in 0..self.nc {
                if mate_inv[j] < 0 {
                    let mut gj = -f[f.len() - 1];
                    for i in fixed_rows..self.nr {
                        gj = gj.min(self.ctmp[i][j] - f[i]);
                    }
                    g[j] = gj;
                }
            }

            if self.ctmp[s][mate[s] as usize] < self.inf {
                self.pq.push(AssignmentNode {
                    opt: ret.opt + aug - self.c[s][old_j],
                    mate,
                    f,
                    g,
                    fixed_rows,
                    banned_js,
                });
            }

            for &j in &ret.banned_js {
                self.ctmp[s][j] = self.c[s][j];
            }
            self.ctmp[s][old_j] = self.c[s][old_j];
        }

        (ret.opt, ret.mate)
    }
}

fn build_matching_inputs(
    s: &State,
    col: usize,
) -> (Vec<u16>, Vec<usize>, Vec<bool>, usize, usize) {
    let v = &s.b[col];
    let mut ndel = 0_usize;
    let mut pos2vals = Vec::new();
    let mut pos2cols = Vec::new();
    let mut next_pops = Vec::new();

    for m in 0..M {
        if m == col || s.b[m].is_empty() {
            continue;
        }
        pos2vals.push(*s.b[m].last().unwrap());
        pos2cols.push(m);
        next_pops.push(false);
    }

    for i in 0..v.len() {
        let a = v[v.len() - 1 - i];
        if s.next_pop as usize <= a as usize && a as usize <= s.next_pop as usize + ndel {
            ndel += 1;
            if let Some(last) = next_pops.last_mut() {
                *last = true;
            }
        } else {
            pos2vals.push(a);
            pos2cols.push(col);
            next_pops.push(false);
        }
    }

    let del_ub = s.next_pop as usize + ndel;
    let ndown = pos2vals.len();

    for m in 0..M {
        let mut a = N_VAL;
        if m != col && s.b[m].len() > 1 {
            a = s.b[m][s.b[m].len() - 2];
        }
        pos2vals.push(a);
        pos2cols.push(m);
        next_pops.push(false);
    }

    (pos2vals, pos2cols, next_pops, ndown, del_ub)
}

fn build_matching_cost(
    s: &State,
    col: usize,
    pos2vals: &[u16],
    pos2cols: &[usize],
    next_pops: &[bool],
    ndown: usize,
    md: i32,
    rng: &mut XorShift,
) -> Vec<Vec<i32>> {
    const INF: i32 = 1 << 20;
    let nup = pos2vals.len();
    let mut cost = vec![vec![INF; nup]; ndown];
    let mut cscnt = vec![0_i32; N + 1];

    for m in 0..M {
        if m == col {
            continue;
        }
        let sorted_len = s.sorted_len(m);
        if sorted_len < 5 {
            continue;
        }
        for i in 0..sorted_len {
            cscnt[s.b[m][i] as usize] += 1;
        }
    }
    for i in 1..=N {
        cscnt[i] += cscnt[i - 1];
    }

    for i in 0..ndown {
        let a = pos2vals[i];
        let from_col = pos2cols[i];

        for j in 0..nup {
            let b = pos2vals[j];
            let to_col = pos2cols[j];
            if i == j {
                continue;
            }
            if from_col == col && to_col == col && i + 1 < j && j < ndown {
                continue;
            }
            if i + 1 == j && next_pops[i] {
                continue;
            }
            if b == N_VAL && to_col == col {
                continue;
            }
            if from_col != col && from_col != to_col && !(a < b && b < N_VAL) {
                continue;
            }
            if from_col != col && to_col != from_col && j >= ndown {
                continue;
            }
            if from_col != col && to_col == col {
                continue;
            }

            let lo = a.min(b) as usize;
            let hi = a.max(b) as usize;
            let mut eval = eval2(a, b, cscnt[hi - 1] - cscnt[lo]);
            if from_col != col && from_col == to_col {
                eval += 2010;
            }
            if from_col == col && from_col == to_col && j == i + 1 {
                eval += 1000;
            }

            let noise = if md > 1 {
                (rng.next_u32() % md as u32) as i32
            } else {
                0
            };
            cost[i][j] = -eval + noise;
        }
    }

    cost
}

fn apply_matching_moves(
    s: &mut State,
    col: usize,
    mate: &[isize],
    pos2vals: &[u16],
    pos2cols: &[usize],
    ndown: usize,
    del_ub: Option<usize>,
    md: i32,
    rng: &mut XorShift,
) {
    let mut other_moves = Vec::new();
    for i in 0..ndown {
        let j = mate[i] as usize;
        let from_val = pos2vals[i];
        let from_col = pos2cols[i];
        let to_val = pos2vals[j];
        let to_col = pos2cols[j];
        if from_col == col || from_col == to_col {
            continue;
        }
        other_moves.push((from_val, from_col, to_val, to_col));
    }

    while !other_moves.is_empty() {
        let mut moved = false;
        for i in 0..other_moves.len() {
            let (from_val, from_col, to_val, _) = other_moves[i];
            let nxtcol = s.find_column(to_val);
            if s.b[nxtcol].last() == Some(&to_val) {
                s.move_value(from_val, from_col, nxtcol);
                other_moves.remove(i);
                moved = true;
                break;
            }
        }
        debug_assert!(moved);
        if !moved {
            break;
        }
    }

    for i in 0..ndown {
        if s.can_pop() {
            while s.can_pop() {
                s.popnext();
            }
            if let Some(del_ub) = del_ub {
                if s.next_pop as usize > del_ub {
                    pushout_matching(s, col, md, rng);
                    break;
                }
            } else {
                break;
            }
        }

        let from_col = pos2cols[i];
        if from_col != col {
            continue;
        }
        let j = mate[i] as usize;
        let dest_val = pos2vals[j];
        let dest_col = if dest_val == N_VAL {
            pos2cols[j]
        } else {
            s.find_column(dest_val)
        };

        if from_col != dest_col {
            s.move_value(pos2vals[i], from_col, dest_col);
        }
    }
}

fn pushout_matching(s: &mut State, col: usize, md: i32, rng: &mut XorShift) {
    loop {
        if s.can_pop() {
            s.popnext();
            continue;
        }
        if s.b[col].is_empty() {
            break;
        }

        let (pos2vals, pos2cols, next_pops, ndown, _) = build_matching_inputs(s, col);
        if ndown == 0 {
            break;
        }
        let cost = build_matching_cost(s, col, &pos2vals, &pos2cols, &next_pops, ndown, md, rng);
        let (_, mate, _, _) = linear_sum_assignment(&cost);

        let mut uf = UnionFind::new(ndown);
        let mut has_cycle = false;
        for i in 0..ndown {
            if mate[i] < 0 || mate[i] as usize >= ndown {
                continue;
            }
            if !uf.unite(i, mate[i] as usize) {
                has_cycle = true;
                break;
            }
        }
        if has_cycle {
            pushout(s, col);
            continue;
        }

        apply_matching_moves(
            s, col, &mate, &pos2vals, &pos2cols, ndown, None, md, rng,
        );
    }
}

fn pushout(s: &mut State, col: usize) {
    if s.can_pop() {
        s.popnext();
        return;
    }
    if s.b[col].is_empty() {
        return;
    }

    let v = &s.b[col];
    let mut poplen = 1_usize;
    while poplen < v.len() {
        let nxt = v[v.len() - poplen - 1];
        if nxt == s.next_pop {
            break;
        }
        let diff = v[v.len() - poplen] as i32 - nxt as i32;
        if diff > 0 || diff > -10 {
            poplen += 1;
        } else {
            break;
        }
    }

    let bottom = s.b[col][s.b[col].len() - poplen];
    let mut best_dest = usize::MAX;
    let mut best_eval = i32::MIN / 4;
    for m in 0..M {
        if m == col {
            continue;
        }
        let top_diff = s.get_top_val(m) as i32 - bottom as i32;
        let eval = if top_diff < 0 {
            1000 + top_diff
        } else if top_diff < 10 {
            10000 - top_diff
        } else {
            top_diff
        };
        if eval > best_eval {
            best_eval = eval;
            best_dest = m;
        }
    }

    s.move_by_count(poplen, col, best_dest);
}

fn pushout_matching_multi(
    mut s0: State,
    col: usize,
    md: i32,
    ncand: usize,
    rng: &mut XorShift,
) -> Vec<State> {
    while s0.can_pop() {
        s0.popnext();
    }
    if s0.b[col].is_empty() {
        return vec![s0];
    }

    let (pos2vals, pos2cols, next_pops, ndown, del_ub) = build_matching_inputs(&s0, col);
    if ndown == 0 {
        return vec![s0];
    }
    let cost = build_matching_cost(
        &s0,
        col,
        &pos2vals,
        &pos2cols,
        &next_pops,
        ndown,
        md,
        rng,
    );

    let mut mates = Vec::new();
    {
        const INF: i32 = 1 << 20;
        let mut ba = BestAssignments::new(ndown, pos2vals.len(), &cost, INF * 10);
        let mut min_cost = 0_i32;
        for trial in 0..ncand {
            if ba.finished() {
                break;
            }
            let (v, mate) = ba.yield_assignment();
            if trial == 0 {
                min_cost = v;
            } else if v > min_cost + 2000 {
                break;
            }

            let mut uf = UnionFind::new(ndown);
            let mut has_cycle = false;
            for i in 0..ndown {
                if mate[i] < 0 || mate[i] as usize >= ndown {
                    continue;
                }
                if !uf.unite(i, mate[i] as usize) {
                    has_cycle = true;
                    break;
                }
            }
            if !has_cycle {
                mates.push(mate);
            }
        }
    }

    let mut res = Vec::new();
    for mate in mates {
        let mut s = s0.clone();
        apply_matching_moves(
            &mut s,
            col,
            &mate,
            &pos2vals,
            &pos2cols,
            ndown,
            Some(del_ub),
            md,
            rng,
        );
        res.push(s);
    }

    if res.is_empty() {
        pushout_matching(&mut s0, col, md, rng);
        return vec![s0];
    }

    res.sort_by_key(|s| s.cost);
    let mut res2 = Vec::new();
    let mut hashes = Vec::new();
    for s in res {
        let h = s.calc_b_hash();
        if !hashes.contains(&h) {
            hashes.push(h);
            res2.push(s);
        }
    }
    res2
}

fn presort_column(
    col: usize,
    s: &State,
    md: i32,
    ncand: usize,
    rng: &mut XorShift,
) -> Vec<State> {
    let mut res = pushout_matching_multi(s.clone(), col, md, ncand, rng);
    for s in &mut res {
        pileup(s);
    }
    res
}

#[derive(Debug, Clone, Copy)]
struct QueueSummary {
    maxeval: i32,
    mineval: i32,
    maxpos: usize,
    minpos: usize,
    maxstateinfo: usize,
}

impl QueueSummary {
    const INF: i32 = 1 << 28;

    fn empty() -> Self {
        Self {
            maxeval: -Self::INF,
            mineval: -Self::INF,
            maxpos: usize::MAX,
            minpos: usize::MAX,
            maxstateinfo: usize::MAX,
        }
    }

    fn make(eval: i32, pos: usize, stateinfo: usize) -> Self {
        Self {
            maxeval: eval,
            mineval: eval,
            maxpos: pos,
            minpos: pos,
            maxstateinfo: stateinfo,
        }
    }
}

fn queue_op(a: QueueSummary, b: QueueSummary) -> QueueSummary {
    let maxeval = a.maxeval.max(b.maxeval);
    let mineval = a.mineval.min(b.mineval);
    QueueSummary {
        maxeval,
        mineval,
        maxpos: if a.maxeval == maxeval {
            a.maxpos
        } else {
            b.maxpos
        },
        minpos: if a.mineval == mineval {
            a.minpos
        } else {
            b.minpos
        },
        maxstateinfo: if a.maxeval == maxeval {
            a.maxstateinfo
        } else {
            b.maxstateinfo
        },
    }
}

#[derive(Debug, Clone)]
struct SegmentQueue {
    size: usize,
    data: Vec<QueueSummary>,
}

impl SegmentQueue {
    fn new(width: usize) -> Self {
        debug_assert!(width.is_power_of_two());
        let size = width;
        let mut data = vec![QueueSummary::empty(); size * 2];
        for i in 0..width {
            data[size + i].maxpos = i;
            data[size + i].minpos = i;
        }
        for i in (1..size).rev() {
            data[i] = queue_op(data[i << 1], data[i << 1 | 1]);
        }
        Self { size, data }
    }

    fn set(&mut self, mut pos: usize, value: QueueSummary) {
        pos += self.size;
        self.data[pos] = value;
        pos >>= 1;
        while pos > 0 {
            self.data[pos] = queue_op(self.data[pos << 1], self.data[pos << 1 | 1]);
            pos >>= 1;
        }
    }

    #[inline]
    fn all_prod(&self) -> QueueSummary {
        self.data[1]
    }
}

fn solve(init_state: State) -> State {
    let timer = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC);
    let mut rng = XorShift::new();
    let mut min_turn = 490_i32;

    let mut dp = Vec::with_capacity(M);
    for phase in 0..M {
        let width = if phase == 0 { 1 } else { 2048 * 2 };
        dp.push(SegmentQueue::new(width));
    }

    let mut hash2mincost: HashMap<i32, i32> = HashMap::new();
    dp[0].set(0, QueueSummary::make(0, 0, 0));

    let mut best_state = init_state.clone();
    let mut seqs_memo: Vec<Vec<usize>> = vec![Vec::new()];
    let mut states_memo = vec![init_state];

    'search: for _round in 0..1000 {
        for phase in 0..M {
            for _iter in 0..=phase {
                if timer.is_time_over() {
                    break 'search;
                }

                let p = dp[phase].all_prod();
                if p.maxeval == -QueueSummary::INF || p.maxstateinfo == usize::MAX {
                    continue;
                }
                dp[phase].set(
                    p.maxpos,
                    QueueSummary::make(-QueueSummary::INF, p.maxpos, usize::MAX),
                );

                if states_memo[p.maxstateinfo].cost + 1 >= min_turn {
                    continue;
                }

                let seq = seqs_memo[p.maxstateinfo].clone();
                let parent = states_memo[p.maxstateinfo].clone();

                for col in 0..M {
                    if seq.contains(&col) {
                        continue;
                    }
                    let mut seq2 = seq.clone();
                    seq2.push(col);

                    let candidates = presort_column(col, &parent, 1, (M - phase) * 3, &mut rng);
                    for mut s in candidates {
                        if s.cost >= min_turn {
                            continue;
                        }

                        let h = s.calc_b_hash();
                        if let Some(prev) = hash2mincost.get_mut(&h) {
                            if s.cost < *prev {
                                *prev = s.cost;
                            } else {
                                continue;
                            }
                        } else {
                            hash2mincost.insert(h, s.cost);
                        }

                        if seq.len() == M - 1 {
                            erase_all(&mut s);
                        }

                        if s.is_done() {
                            if s.cost < min_turn {
                                min_turn = s.cost;
                                best_state = s;
                            }
                        } else if phase + 1 < M {
                            let e = evaluate_state(&s);
                            let nxt = dp[phase + 1].all_prod();
                            if nxt.mineval < e {
                                let state_idx = states_memo.len();
                                dp[phase + 1]
                                    .set(nxt.minpos, QueueSummary::make(e, nxt.minpos, state_idx));
                                seqs_memo.push(seq2.clone());
                                states_memo.push(s);
                            }
                        }
                    }
                }
            }
        }
    }

    if !best_state.is_done() {
        erase_all(&mut best_state);
    }

    #[cfg(feature = "local")]
    {
        eprintln!("[summary.count] states={}", states_memo.len());
        eprintln!("[summary.count] cost={}", best_state.cost);
        eprintln!("[summary.count] ops={}", best_state.ops.len());
        eprintln!("[summary.time_ms] elapsed={}", timer.elapsed_ms());
    }

    best_state
}

fn main() {
    input! {
        n_: usize,
        m_: usize,
        rows: [[usize; PER_PILE]; M],
    }
    debug_assert_eq!(n_, N);
    debug_assert_eq!(m_, M);

    let mut b = vec![Vec::with_capacity(PER_PILE); M];
    for i in 0..M {
        for j in 0..PER_PILE {
            b[i].push((rows[i][j] - 1) as u16);
        }
    }

    let best_state = solve(State::new(b));

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for op in best_state.ops {
        let to = if op.to >= 0 { op.to as usize + 1 } else { 0 };
        writeln!(out, "{} {}", op.v as usize + 1, to).unwrap();
    }
}
