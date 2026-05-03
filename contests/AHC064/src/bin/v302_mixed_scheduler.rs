// v302_mixed_scheduler.rs
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};

const R: usize = 10;
const INIT_LEN: usize = 10;
const DEP_CAP: usize = 15;
const SID_CAP: usize = 20;
const MAX_TURNS: usize = 4000;
const TIME_LIMIT_SEC: f64 = 1.95;

const MOVE_DEP_TO_SIDING: usize = 0;
const MOVE_SIDING_TO_DEP: usize = 1;

const OP_A: u8 = b'A';
const OP_B: u8 = b'B';
const OP_T: u8 = b'T';

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AbsOp {
    t: u8,
    k: usize,
}

#[derive(Clone, Debug)]
struct Sim {
    dep: Vec<Vec<usize>>,
    sid: Vec<Vec<usize>>,
    turns: Vec<Vec<Move>>,
    debug: bool,
}

impl Sim {
    fn new(init: &[Vec<usize>]) -> Self {
        Self {
            dep: init.to_vec(),
            sid: vec![Vec::new(); R],
            turns: Vec::new(),
            debug: false,
        }
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
        let mut prev: Option<(usize, usize)> = None;

        for &mv in moves {
            assert!(mv.kind == MOVE_DEP_TO_SIDING || mv.kind == MOVE_SIDING_TO_DEP);
            assert!(mv.i < R && mv.j < R && mv.k > 0);
            assert!(!used_dep[mv.i] && !used_sid[mv.j]);
            used_dep[mv.i] = true;
            used_sid[mv.j] = true;
            if let Some((pi, pj)) = prev {
                assert!(pi < mv.i && pj < mv.j);
            }
            prev = Some((mv.i, mv.j));
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
            let mut block = self.dep[mv.i].split_off(n - mv.k);
            block.extend_from_slice(&self.sid[mv.j]);
            self.sid[mv.j] = block;
        } else {
            let block = self.sid[mv.j][..mv.k].to_vec();
            self.sid[mv.j].drain(..mv.k);
            self.dep[mv.i].extend(block);
        }
    }

    fn is_goal(&self) -> bool {
        for r in 0..R {
            if self.dep[r].len() != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] != target_id(r, c) {
                    return false;
                }
            }
        }
        true
    }
}

#[inline(always)]
fn target_id(r: usize, c: usize) -> usize {
    r * INIT_LEN + c
}

#[inline(always)]
fn route_of(x: usize) -> usize {
    x / INIT_LEN
}

#[inline(always)]
fn pos_of(x: usize) -> usize {
    x % INIT_LEN
}

fn encode_prog_key(s: &[u8], a: &[u8], l: usize) -> u64 {
    let mut key = l as u64;
    key |= (s.len() as u64) << 4;
    key |= (a.len() as u64) << 8;
    let mut shift = 12;
    for &x in s {
        key |= (x as u64) << shift;
        shift += 4;
    }
    for &x in a {
        key |= (x as u64) << shift;
        shift += 4;
    }
    key
}

fn decode_prog_key(mut key: u64, s: &mut Vec<u8>, a: &mut Vec<u8>) -> usize {
    let l = (key & 15) as usize;
    let sl = ((key >> 4) & 15) as usize;
    let al = ((key >> 8) & 15) as usize;
    key >>= 12;
    s.clear();
    a.clear();
    s.reserve(sl);
    a.reserve(al);
    for _ in 0..sl {
        s.push((key & 15) as u8);
        key >>= 4;
    }
    for _ in 0..al {
        a.push((key & 15) as u8);
        key >>= 4;
    }
    l
}

fn perm_code_from_vec(p: &[usize]) -> usize {
    let mut code = 0usize;
    for &x in p {
        code = code * 10 + x;
    }
    code
}

fn get_prog_for_perm(p: &[usize], memo: &mut HashMap<usize, Vec<AbsOp>>) -> Vec<AbsOp> {
    let pcode = perm_code_from_vec(p);
    if let Some(ops) = memo.get(&pcode) {
        return ops.clone();
    }

    let start_s = p.iter().map(|&x| x as u8).collect::<Vec<_>>();
    let start = encode_prog_key(&start_s, &[], 0);
    let mut queue = VecDeque::new();
    let mut visited = HashSet::with_capacity(10000);
    let mut parent: HashMap<u64, (u64, AbsOp)> = HashMap::with_capacity(10000);

    queue.push_back(start);
    visited.insert(start);

    let mut s = Vec::with_capacity(INIT_LEN);
    let mut a = Vec::with_capacity(INIT_LEN);
    while let Some(cur) = queue.pop_front() {
        let l = decode_prog_key(cur, &mut s, &mut a);
        if l == INIT_LEN && s.is_empty() && a.is_empty() {
            let mut ops = Vec::new();
            let mut x = cur;
            while x != start {
                let &(px, op) = parent.get(&x).unwrap();
                ops.push(op);
                x = px;
            }
            ops.reverse();
            memo.insert(pcode, ops.clone());
            return ops;
        }

        let mut maxk = 0usize;
        for (i, &x) in s.iter().enumerate() {
            if x as usize == l + i {
                maxk = i + 1;
            } else {
                break;
            }
        }
        if maxk > 0 {
            let k = maxk;
            let ns = encode_prog_key(&s[k..], &a, l + k);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_T, k }));
                queue.push_back(ns);
            }
        }

        for k in 1..=s.len() {
            if a.len() + k > DEP_CAP {
                continue;
            }
            let mut na = a.clone();
            na.extend_from_slice(&s[..k]);
            let ns = encode_prog_key(&s[k..], &na, l);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_A, k }));
                queue.push_back(ns);
            }
        }

        for k in 1..=a.len() {
            let na = &a[..a.len() - k];
            let mut ns_vec = Vec::with_capacity(k + s.len());
            ns_vec.extend_from_slice(&a[a.len() - k..]);
            ns_vec.extend_from_slice(&s);
            let ns = encode_prog_key(&ns_vec, na, l);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_B, k }));
                queue.push_back(ns);
            }
        }
    }

    Vec::new()
}

fn noncrossing_pairs(pairs: &[(usize, usize)]) -> bool {
    let mut v = pairs.to_vec();
    v.sort_unstable();
    for i in 1..v.len() {
        if v[i - 1].1 >= v[i].1 {
            return false;
        }
    }
    true
}

fn schedule_group_est(group: &[usize], aux_set: &[usize], progs: &[Vec<AbsOp>]) -> i32 {
    let n = group.len();
    let mut g = group.to_vec();
    let mut a = aux_set.to_vec();
    g.sort_unstable();
    a.sort_unstable();

    let mut aux = [usize::MAX; R];
    for t in 0..n {
        aux[g[t]] = a[t];
    }

    let mut idx = [0usize; R];
    let mut turns = 0i32;
    loop {
        let mut done = true;
        for &r in &g {
            if idx[r] < progs[r].len() {
                done = false;
            }
        }
        if done {
            return turns;
        }

        #[derive(Clone, Copy)]
        struct Av {
            i: usize,
            j: usize,
            t: u8,
            k: usize,
        }

        let mut av = vec![None; n];
        for z in 0..n {
            let r = g[z];
            if idx[r] < progs[r].len() {
                let op = progs[r][idx[r]];
                let ii = if op.t == OP_T { r } else { aux[r] };
                av[z] = Some(Av {
                    i: ii,
                    j: r,
                    t: op.t,
                    k: op.k,
                });
            }
        }

        let mut best_mask = 0usize;
        let mut best_score = -1i32;
        for mask in 1usize..(1usize << n) {
            let mut pairs = Vec::new();
            let mut ok = true;
            let mut cnt = 0i32;
            let mut tc = 0i32;
            let mut ab = 0i32;
            let mut ksum = 0i32;
            let mut used = 0usize;
            for z in 0..n {
                if (mask >> z) & 1 == 0 {
                    continue;
                }
                let Some(x) = av[z] else {
                    ok = false;
                    break;
                };
                if (used >> x.i) & 1 == 1 {
                    ok = false;
                    break;
                }
                used |= 1usize << x.i;
                pairs.push((x.i, x.j));
                cnt += 1;
                ksum += x.k as i32;
                if x.t == OP_T {
                    tc += 1;
                } else {
                    ab += 1;
                }
            }
            if !ok || !noncrossing_pairs(&pairs) {
                continue;
            }
            let score = cnt * 10000 + tc * 100 + ab * 10 + ksum;
            if score > best_score {
                best_score = score;
                best_mask = mask;
            }
        }
        if best_mask == 0 {
            return 1_000_000;
        }
        for z in 0..n {
            if (best_mask >> z) & 1 == 1 {
                idx[g[z]] += 1;
            }
        }
        turns += 1;
    }
}

#[derive(Clone, Debug)]
struct SortPlanChoice {
    first_group: Vec<usize>,
    est_turns: i32,
}

fn build_sort_progs(
    sroute: &[Vec<usize>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> Vec<Vec<AbsOp>> {
    let mut progs = vec![Vec::new(); R];
    for r in 0..R {
        let p = sroute[r].iter().map(|&x| pos_of(x)).collect::<Vec<_>>();
        progs[r] = get_prog_for_perm(&p, memo);
    }
    progs
}

fn choose_sort_partition(progs: &[Vec<AbsOp>]) -> SortPlanChoice {
    let mut best = SortPlanChoice {
        first_group: Vec::new(),
        est_turns: i32::MAX,
    };

    for mask in 0usize..(1usize << R) {
        if mask.count_ones() != 5 {
            continue;
        }
        let mut g = Vec::new();
        let mut c = Vec::new();
        for r in 0..R {
            if (mask >> r) & 1 == 1 {
                g.push(r);
            } else {
                c.push(r);
            }
        }
        let t1 = schedule_group_est(&g, &c, progs);
        let t2 = schedule_group_est(&c, &g, progs);
        let est = t1 + 1 + t2 + 1;
        if est < best.est_turns {
            best.est_turns = est;
            best.first_group = g;
        }
    }
    best
}

fn execute_group_sort(sim: &mut Sim, group: &[usize], aux_set: &[usize], progs: &[Vec<AbsOp>]) {
    let n = group.len();
    let mut g = group.to_vec();
    let mut a = aux_set.to_vec();
    g.sort_unstable();
    a.sort_unstable();

    let mut aux = [usize::MAX; R];
    for t in 0..n {
        aux[g[t]] = a[t];
    }

    let mut idx = [0usize; R];
    loop {
        let mut done = true;
        for &r in &g {
            if idx[r] < progs[r].len() {
                done = false;
            }
        }
        if done {
            break;
        }

        #[derive(Clone, Copy)]
        struct Av {
            i: usize,
            j: usize,
            t: u8,
            k: usize,
            mv: Move,
        }

        let mut av = vec![None; n];
        for z in 0..n {
            let r = g[z];
            if idx[r] < progs[r].len() {
                let op = progs[r][idx[r]];
                let mv = if op.t == OP_A {
                    Move::siding_to_dep(aux[r], r, op.k)
                } else if op.t == OP_B {
                    Move::dep_to_siding(aux[r], r, op.k)
                } else {
                    Move::siding_to_dep(r, r, op.k)
                };
                av[z] = Some(Av {
                    i: mv.i,
                    j: mv.j,
                    t: op.t,
                    k: op.k,
                    mv,
                });
            }
        }

        let mut best_mask = 0usize;
        let mut best_score = -1i32;
        for mask in 1usize..(1usize << n) {
            let mut pairs = Vec::new();
            let mut ok = true;
            let mut cnt = 0i32;
            let mut tc = 0i32;
            let mut ab = 0i32;
            let mut ksum = 0i32;
            let mut used = 0usize;
            for z in 0..n {
                if (mask >> z) & 1 == 0 {
                    continue;
                }
                let Some(x) = av[z] else {
                    ok = false;
                    break;
                };
                if (used >> x.i) & 1 == 1 {
                    ok = false;
                    break;
                }
                used |= 1usize << x.i;
                pairs.push((x.i, x.j));
                cnt += 1;
                ksum += x.k as i32;
                if x.t == OP_T {
                    tc += 1;
                } else {
                    ab += 1;
                }
            }
            if !ok || !noncrossing_pairs(&pairs) {
                continue;
            }
            let score = cnt * 10000 + tc * 100 + ab * 10 + ksum;
            if score > best_score {
                best_score = score;
                best_mask = mask;
            }
        }

        assert!(best_mask != 0);
        let mut ops = Vec::new();
        for z in 0..n {
            if (best_mask >> z) & 1 == 1 {
                ops.push(av[z].unwrap().mv);
                idx[g[z]] += 1;
            }
        }
        sim.add_turn(ops);
    }
}

fn sort_route_buckets_to_goal(
    sim: &mut Sim,
    sroute: &[Vec<usize>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) {
    let progs = build_sort_progs(sroute, memo);
    let choice = choose_sort_partition(&progs);
    let g = choice.first_group;
    let mut in_g = 0usize;
    for &r in &g {
        in_g |= 1usize << r;
    }
    let mut c = Vec::new();
    for r in 0..R {
        if (in_g >> r) & 1 == 0 {
            c.push(r);
        }
    }

    execute_group_sort(sim, &g, &c, &progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::dep_to_siding(r, r, INIT_LEN))
            .collect(),
    );
    execute_group_sort(sim, &c, &g, &progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::siding_to_dep(r, r, INIT_LEN))
            .collect(),
    );
}

fn remaining_prog_key(p: &[usize], start_l: usize, aux_cap: usize) -> u64 {
    let mut key = start_l as u64;
    key |= (aux_cap as u64) << 4;
    key |= (p.len() as u64) << 8;
    let mut shift = 12;
    for &x in p {
        key |= (x as u64) << shift;
        shift += 4;
    }
    key
}

fn get_prog_for_remaining_positions(
    p: &[usize],
    start_l: usize,
    aux_cap: usize,
    memo: &mut HashMap<u64, Vec<AbsOp>>,
) -> Vec<AbsOp> {
    let pkey = remaining_prog_key(p, start_l, aux_cap);
    if let Some(ops) = memo.get(&pkey) {
        return ops.clone();
    }

    let start_s = p.iter().map(|&x| x as u8).collect::<Vec<_>>();
    let start = encode_prog_key(&start_s, &[], start_l);
    let mut queue = VecDeque::new();
    let mut visited = HashSet::with_capacity(10000);
    let mut parent: HashMap<u64, (u64, AbsOp)> = HashMap::with_capacity(10000);

    queue.push_back(start);
    visited.insert(start);

    let mut s = Vec::with_capacity(INIT_LEN);
    let mut a = Vec::with_capacity(INIT_LEN);
    while let Some(cur) = queue.pop_front() {
        let l = decode_prog_key(cur, &mut s, &mut a);
        if l == INIT_LEN && s.is_empty() && a.is_empty() {
            let mut ops = Vec::new();
            let mut x = cur;
            while x != start {
                let &(px, op) = parent.get(&x).unwrap();
                ops.push(op);
                x = px;
            }
            ops.reverse();
            memo.insert(pkey, ops.clone());
            return ops;
        }

        let mut maxk = 0usize;
        for (i, &x) in s.iter().enumerate() {
            if x as usize == l + i {
                maxk = i + 1;
            } else {
                break;
            }
        }
        if maxk > 0 {
            let k = maxk;
            let ns = encode_prog_key(&s[k..], &a, l + k);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_T, k }));
                queue.push_back(ns);
            }
        }

        for k in 1..=s.len() {
            if a.len() + k > aux_cap {
                continue;
            }
            let mut na = a.clone();
            na.extend_from_slice(&s[..k]);
            let ns = encode_prog_key(&s[k..], &na, l);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_A, k }));
                queue.push_back(ns);
            }
        }

        for k in 1..=a.len() {
            let na = &a[..a.len() - k];
            let mut ns_vec = Vec::with_capacity(k + s.len());
            ns_vec.extend_from_slice(&a[a.len() - k..]);
            ns_vec.extend_from_slice(&s);
            let ns = encode_prog_key(&ns_vec, na, l);
            if visited.insert(ns) {
                parent.insert(ns, (cur, AbsOp { t: OP_B, k }));
                queue.push_back(ns);
            }
        }
    }

    Vec::new()
}

fn build_prefixed_sort_progs(
    sroute: &[Vec<usize>],
    fixed: &[usize; R],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
    rem_memo: &mut HashMap<u64, Vec<AbsOp>>,
) -> Vec<Vec<AbsOp>> {
    let mut progs = vec![Vec::new(); R];
    for r in 0..R {
        let p = sroute[r].iter().map(|&x| pos_of(x)).collect::<Vec<_>>();
        if fixed[r] == 0 {
            progs[r] = get_prog_for_perm(&p, memo);
        } else {
            debug_assert!(p.iter().all(|&x| x >= fixed[r]));
            progs[r] = get_prog_for_remaining_positions(&p, fixed[r], DEP_CAP, rem_memo);
        }
    }
    progs
}

fn build_prefixed_sort_progs_with_caps(
    sroute: &[Vec<usize>],
    fixed: &[usize; R],
    aux_cap: &[usize; R],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
    rem_memo: &mut HashMap<u64, Vec<AbsOp>>,
) -> Option<Vec<Vec<AbsOp>>> {
    let mut progs = vec![Vec::new(); R];
    for r in 0..R {
        let p = sroute[r].iter().map(|&x| pos_of(x)).collect::<Vec<_>>();
        if p.len() + fixed[r] != INIT_LEN {
            return None;
        }
        if p.iter().any(|&x| x < fixed[r]) {
            return None;
        }
        if p.is_empty() {
            continue;
        }

        let prog = if fixed[r] == 0 && aux_cap[r] == DEP_CAP {
            get_prog_for_perm(&p, memo)
        } else {
            get_prog_for_remaining_positions(&p, fixed[r], aux_cap[r], rem_memo)
        };
        if prog.is_empty() {
            return None;
        }
        progs[r] = prog;
    }
    Some(progs)
}

fn sort_route_buckets_to_goal_with_prefix(
    sim: &mut Sim,
    first_group: &[usize],
    fixed: &[usize; R],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
    rem_memo: &mut HashMap<u64, Vec<AbsOp>>,
) {
    let sroute = sim.sid.clone();
    let progs = build_prefixed_sort_progs(&sroute, fixed, memo, rem_memo);
    let mut g = first_group.to_vec();
    g.sort_unstable();

    let mut in_g = 0usize;
    for &r in &g {
        in_g |= 1usize << r;
    }
    let mut c = Vec::new();
    for r in 0..R {
        if (in_g >> r) & 1 == 0 {
            debug_assert_eq!(fixed[r], 0);
            c.push(r);
        }
    }

    execute_group_sort(sim, &g, &c, &progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::dep_to_siding(r, r, INIT_LEN))
            .collect(),
    );
    execute_group_sort(sim, &c, &g, &progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::siding_to_dep(r, r, INIT_LEN))
            .collect(),
    );
}

fn sort_route_buckets_to_goal_with_prefix_caps(
    sim: &mut Sim,
    first_group: &[usize],
    fixed: &[usize; R],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
    rem_memo: &mut HashMap<u64, Vec<AbsOp>>,
) -> bool {
    let sroute = sim.sid.clone();
    let mut g = first_group.to_vec();
    g.sort_unstable();

    let mut in_g = 0usize;
    for &r in &g {
        in_g |= 1usize << r;
    }
    let mut c = Vec::new();
    for r in 0..R {
        if (in_g >> r) & 1 == 0 {
            c.push(r);
        }
    }

    let mut first_aux_cap = [DEP_CAP; R];
    for t in 0..g.len() {
        first_aux_cap[g[t]] = DEP_CAP - fixed[c[t]];
    }
    let Some(first_progs) =
        build_prefixed_sort_progs_with_caps(&sroute, fixed, &first_aux_cap, memo, rem_memo)
    else {
        return false;
    };

    let full_aux_cap = [DEP_CAP; R];
    let Some(second_progs) =
        build_prefixed_sort_progs_with_caps(&sroute, fixed, &full_aux_cap, memo, rem_memo)
    else {
        return false;
    };

    execute_group_sort(sim, &g, &c, &first_progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::dep_to_siding(r, r, INIT_LEN))
            .collect(),
    );
    execute_group_sort(sim, &c, &g, &second_progs);
    sim.add_turn(
        g.iter()
            .map(|&r| Move::siding_to_dep(r, r, INIT_LEN))
            .collect(),
    );
    true
}

#[derive(Clone, Copy, Debug)]
struct DistNode {
    p: [u8; R],
    seen_mask: [u16; R],
    g: i32,
    bad: i32,
    score: i32,
    code: u64,
    parent: isize,
    pmask: usize,
}

impl DistNode {
    fn new_start() -> Self {
        let p = [0u8; R];
        Self {
            p,
            seen_mask: [0u16; R],
            g: 0,
            bad: 0,
            score: 0,
            code: encode_pos(&p),
            parent: -1,
            pmask: 0,
        }
    }
}

fn encode_pos(p: &[u8; R]) -> u64 {
    let mut x = 0u64;
    for i in (0..R).rev() {
        x = x * 11 + p[i] as u64;
    }
    x
}

#[derive(Clone, Debug)]
struct DistBuilder {
    car: [[usize; INIT_LEN]; R],
    rt: [[usize; INIT_LEN]; R],
    ps: [[usize; INIT_LEN]; R],
}

impl DistBuilder {
    fn new(d0: &[Vec<usize>]) -> Self {
        let mut db = Self {
            car: [[0; INIT_LEN]; R],
            rt: [[0; INIT_LEN]; R],
            ps: [[0; INIT_LEN]; R],
        };
        for (i, row) in d0.iter().enumerate().take(R) {
            for q in 0..INIT_LEN {
                let x = row[INIT_LEN - 1 - q];
                db.car[i][q] = x;
                db.rt[i][q] = route_of(x);
                db.ps[i][q] = pos_of(x);
            }
        }
        db
    }

    fn run_len(&self, i: usize, p: usize) -> usize {
        if p >= INIT_LEN {
            return 0;
        }
        let j = self.rt[i][p];
        let mut q = p;
        while q < INIT_LEN && self.rt[i][q] == j {
            q += 1;
        }
        q - p
    }

    fn lower_bound_runs(&self, p: &[u8; R]) -> i32 {
        let mut src_lb = 0i32;
        let mut target_runs = [0i32; R];
        for i in 0..R {
            let mut runs = 0i32;
            let mut last = usize::MAX;
            for q in p[i] as usize..INIT_LEN {
                let j = self.rt[i][q];
                if j != last {
                    runs += 1;
                    target_runs[j] += 1;
                    last = j;
                }
            }
            src_lb = src_lb.max(runs);
        }
        let mut lb = src_lb;
        for &runs in &target_runs {
            lb = lb.max(runs);
        }
        lb
    }

    fn bad_inc_run(&self, i: usize, p: usize, len: usize, seen: &[u16; R]) -> i32 {
        let j = self.rt[i][p];
        let mut inc = 0i32;
        let mut block = Vec::with_capacity(len);
        for q in (p..p + len).rev() {
            block.push(self.ps[i][q]);
        }

        let old = seen[j] as u32;
        for a in 0..block.len() {
            let pp = block[a];
            inc += (old & ((1u32 << pp) - 1)).count_ones() as i32;
            for b in 0..a {
                if block[b] > pp {
                    inc += 1;
                }
            }
        }
        inc
    }

    fn gen_masks_rec(
        i: usize,
        prev: isize,
        top: &[isize; R],
        res: &mut Vec<(usize, usize)>,
        mask: usize,
        cnt: usize,
    ) {
        if i == R {
            if mask != 0 {
                res.push((mask, cnt));
            }
            return;
        }
        Self::gen_masks_rec(i + 1, prev, top, res, mask, cnt);
        let j = top[i];
        if j >= 0 && j > prev {
            Self::gen_masks_rec(i + 1, j, top, res, mask | (1usize << i), cnt + 1);
        }
    }

    fn legal_masks(&self, top: &[isize; R]) -> Vec<(usize, usize)> {
        let mut res = Vec::with_capacity(200);
        Self::gen_masks_rec(0, -1, top, &mut res, 0, 0);
        res
    }

    fn build_turns_from_node(&self, nodes: &[DistNode], mut id: usize) -> Vec<Vec<Move>> {
        let mut path = Vec::new();
        while nodes[id].parent != -1 {
            path.push(id);
            id = nodes[id].parent as usize;
        }
        path.reverse();

        let mut pp = [0usize; R];
        let mut turns = Vec::new();
        for nid in path {
            let mask = nodes[nid].pmask;
            let mut ops = Vec::new();
            for i in 0..R {
                if (mask >> i) & 1 == 1 {
                    let p = pp[i];
                    let len = self.run_len(i, p);
                    let j = self.rt[i][p];
                    ops.push(Move::dep_to_siding(i, j, len));
                    pp[i] += len;
                }
            }
            turns.push(ops);
        }
        turns
    }

    fn greedy_final_node(&self, nodes: &mut Vec<DistNode>, order_penalty: i32) -> usize {
        nodes.clear();
        nodes.push(DistNode::new_start());
        let mut cur_id = 0usize;

        loop {
            let nd = nodes[cur_id];
            let mut done = true;
            let mut top = [-1isize; R];
            for i in 0..R {
                if nd.p[i] < INIT_LEN as u8 {
                    done = false;
                    top[i] = self.rt[i][nd.p[i] as usize] as isize;
                }
            }
            if done {
                return cur_id;
            }

            let masks = self.legal_masks(&top);
            let mut best_mask = 0usize;
            let mut best_score = i32::MIN;
            let mut best_bad = 0i32;
            for (mask, cnt) in masks {
                let mut cars = 0i32;
                let mut bi = 0i32;
                for i in 0..R {
                    if (mask >> i) & 1 == 1 {
                        let len = self.run_len(i, nd.p[i] as usize);
                        cars += len as i32;
                        bi += self.bad_inc_run(i, nd.p[i] as usize, len, &nd.seen_mask);
                    }
                }
                let score = cnt as i32 * 100000 + cars * 100 - order_penalty * bi;
                if score > best_score {
                    best_score = score;
                    best_mask = mask;
                    best_bad = bi;
                }
            }

            let mut nn = nd;
            nn.parent = cur_id as isize;
            nn.pmask = best_mask;
            nn.g = nd.g + 1;
            nn.bad = nd.bad + best_bad;
            for i in 0..R {
                if (best_mask >> i) & 1 == 1 {
                    let p = nd.p[i] as usize;
                    let len = self.run_len(i, p);
                    let j = self.rt[i][p];
                    for q in p..p + len {
                        nn.seen_mask[j] |= 1u16 << self.ps[i][q];
                    }
                    nn.p[i] += len as u8;
                }
            }
            nn.code = encode_pos(&nn.p);
            nodes.push(nn);
            cur_id = nodes.len() - 1;
        }
    }

    fn beam_final_nodes(
        &self,
        nodes: &mut Vec<DistNode>,
        width: usize,
        max_expand: usize,
        bad_weight: i32,
        extra_depth: i32,
        final_keep: usize,
    ) -> Vec<usize> {
        nodes.clear();
        nodes.reserve(width * 80);
        nodes.push(DistNode::new_start());

        let mut beam = vec![0usize];
        let mut finals = Vec::new();
        let mut first_final = 1_000_000_000i32;

        for depth in 0..90i32 {
            let mut cand = Vec::with_capacity(width * max_expand);
            for &id in &beam {
                let nd = nodes[id];
                let mut done = true;
                let mut top = [-1isize; R];
                for i in 0..R {
                    if nd.p[i] < INIT_LEN as u8 {
                        done = false;
                        top[i] = self.rt[i][nd.p[i] as usize] as isize;
                    }
                }
                if done {
                    first_final = first_final.min(nd.g);
                    finals.push(id);
                    continue;
                }
                if nd.g >= first_final + extra_depth {
                    continue;
                }

                #[derive(Clone, Copy, Debug)]
                struct MaskMove {
                    mask: usize,
                    cnt: usize,
                    cars: i32,
                    bad: i32,
                }

                let masks = self.legal_masks(&top);
                let mut mv = Vec::with_capacity(masks.len());
                for (mask, cnt) in masks {
                    let mut cars = 0i32;
                    let mut bi = 0i32;
                    for i in 0..R {
                        if (mask >> i) & 1 == 1 {
                            let len = self.run_len(i, nd.p[i] as usize);
                            cars += len as i32;
                            bi += self.bad_inc_run(i, nd.p[i] as usize, len, &nd.seen_mask);
                        }
                    }
                    mv.push(MaskMove {
                        mask,
                        cnt,
                        cars,
                        bad: bi,
                    });
                }
                mv.sort_unstable_by(|x, y| {
                    let sx = x.cnt as i32 * 100000 + x.cars * 100 - x.bad * 5;
                    let sy = y.cnt as i32 * 100000 + y.cars * 100 - y.bad * 5;
                    sy.cmp(&sx)
                });
                if mv.len() > max_expand {
                    mv.truncate(max_expand);
                }

                for m in mv {
                    let mut nn = nd;
                    nn.parent = id as isize;
                    nn.pmask = m.mask;
                    nn.g = nd.g + 1;
                    nn.bad = nd.bad + m.bad;
                    for i in 0..R {
                        if (m.mask >> i) & 1 == 1 {
                            let p = nd.p[i] as usize;
                            let len = self.run_len(i, p);
                            let j = self.rt[i][p];
                            for q in p..p + len {
                                nn.seen_mask[j] |= 1u16 << self.ps[i][q];
                            }
                            nn.p[i] += len as u8;
                        }
                    }
                    nn.code = encode_pos(&nn.p);
                    let processed = nn.p.iter().map(|&x| x as i32).sum::<i32>();
                    let lb = self.lower_bound_runs(&nn.p);
                    nn.score = (nn.g + lb) * 1_000_000 + nn.bad * bad_weight - processed * 2000;
                    nodes.push(nn);
                    cand.push(nodes.len() - 1);
                }
            }

            if !finals.is_empty() && depth >= first_final + extra_depth {
                break;
            }

            cand.sort_unstable_by(|&a, &b| {
                let na = nodes[a];
                let nb = nodes[b];
                na.score
                    .cmp(&nb.score)
                    .then_with(|| na.bad.cmp(&nb.bad))
                    .then_with(|| na.code.cmp(&nb.code))
            });

            beam.clear();
            let mut used = HashSet::with_capacity(width * 2 + 10);
            for id in cand {
                let code = nodes[id].code;
                if used.insert(code) {
                    beam.push(id);
                    if beam.len() >= width {
                        break;
                    }
                }
            }
            if beam.is_empty() {
                break;
            }
        }

        finals.sort_unstable_by(|&a, &b| {
            let na = nodes[a];
            let nb = nodes[b];
            na.g.cmp(&nb.g)
                .then_with(|| na.bad.cmp(&nb.bad))
                .then_with(|| na.code.cmp(&nb.code))
        });
        if finals.len() > final_keep {
            finals.truncate(final_keep);
        }
        finals
    }
}

#[derive(Clone, Debug)]
struct CandidateSolution {
    turns: Vec<Vec<Move>>,
    turn_count: usize,
}

impl CandidateSolution {
    fn empty() -> Self {
        Self {
            turns: Vec::new(),
            turn_count: usize::MAX,
        }
    }
}

fn can_add_move_to_turn(moves: &[Move], cand: Move) -> bool {
    for &mv in moves {
        if mv.i == cand.i || mv.j == cand.j {
            return false;
        }
        if (mv.i < cand.i && mv.j >= cand.j) || (cand.i < mv.i && cand.j >= mv.j) {
            return false;
        }
    }
    true
}

fn ready_commit_len(
    sim: &Sim,
    r: usize,
    fixed: &[usize; R],
    src_remaining: &[usize; R],
    commit_mask: usize,
) -> usize {
    if ((commit_mask >> r) & 1 == 0) || src_remaining[r] != 0 || fixed[r] >= INIT_LEN {
        return 0;
    }
    if sim.dep[r].len() != fixed[r] {
        return 0;
    }

    let mut len = 0usize;
    while fixed[r] + len < INIT_LEN
        && len < sim.sid[r].len()
        && sim.sid[r][len] == target_id(r, fixed[r] + len)
        && sim.dep[r].len() + len < DEP_CAP
    {
        len += 1;
    }
    len
}

fn choose_commit_group_for_distribution(
    init: &[Vec<usize>],
    dturns: &[Vec<Move>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> Vec<usize> {
    let mut dry = Sim::new(init);
    for ops in dturns {
        dry.add_turn(ops.clone());
    }
    let sroute = dry.sid.clone();
    let progs = build_sort_progs(&sroute, memo);
    choose_sort_partition(&progs).first_group
}

fn make_solution_from_distribution(
    db: &DistBuilder,
    nodes: &[DistNode],
    final_id: usize,
    init: &[Vec<usize>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> CandidateSolution {
    let mut sim = Sim::new(init);
    let dturns = db.build_turns_from_node(nodes, final_id);
    for ops in dturns {
        sim.add_turn(ops);
    }
    let sroute = sim.sid.clone();
    sort_route_buckets_to_goal(&mut sim, &sroute, memo);
    assert!(sim.is_goal());

    CandidateSolution {
        turn_count: sim.turns.len(),
        turns: sim.turns,
    }
}

fn make_solution_from_distribution_mixed_b(
    db: &DistBuilder,
    nodes: &[DistNode],
    final_id: usize,
    init: &[Vec<usize>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> CandidateSolution {
    let dturns = db.build_turns_from_node(nodes, final_id);
    let commit_group = choose_commit_group_for_distribution(init, &dturns, memo);
    let mut commit_mask = 0usize;
    for &r in &commit_group {
        commit_mask |= 1usize << r;
    }

    let mut sim = Sim::new(init);
    let mut fixed = [0usize; R];
    let mut src_remaining = [INIT_LEN; R];

    for dist_ops in dturns {
        let mut ops = dist_ops.clone();
        let mut commits = Vec::new();
        for r in 0..R {
            let len = ready_commit_len(&sim, r, &fixed, &src_remaining, commit_mask);
            if len == 0 {
                continue;
            }
            let mv = Move::siding_to_dep(r, r, len);
            if can_add_move_to_turn(&ops, mv) {
                ops.push(mv);
                commits.push(mv);
            }
        }

        sim.add_turn(ops);
        for mv in dist_ops {
            src_remaining[mv.i] -= mv.k;
        }
        for mv in commits {
            fixed[mv.i] += mv.k;
        }
    }

    let mut rem_memo = HashMap::new();
    sort_route_buckets_to_goal_with_prefix(&mut sim, &commit_group, &fixed, memo, &mut rem_memo);
    assert!(sim.is_goal());

    CandidateSolution {
        turn_count: sim.turns.len(),
        turns: sim.turns,
    }
}

fn make_solution_from_distribution_mixed_all(
    db: &DistBuilder,
    nodes: &[DistNode],
    final_id: usize,
    init: &[Vec<usize>],
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> Option<CandidateSolution> {
    let dturns = db.build_turns_from_node(nodes, final_id);
    let first_group = choose_commit_group_for_distribution(init, &dturns, memo);

    let mut sim = Sim::new(init);
    let mut fixed = [0usize; R];
    let mut src_remaining = [INIT_LEN; R];

    for dist_ops in dturns {
        let mut ops = dist_ops.clone();
        let mut commits = Vec::new();
        for r in 0..R {
            let len = ready_commit_len(&sim, r, &fixed, &src_remaining, (1usize << R) - 1);
            if len == 0 {
                continue;
            }
            let mv = Move::siding_to_dep(r, r, len);
            if can_add_move_to_turn(&ops, mv) {
                ops.push(mv);
                commits.push(mv);
            }
        }

        sim.add_turn(ops);
        for mv in dist_ops {
            src_remaining[mv.i] -= mv.k;
        }
        for mv in commits {
            fixed[mv.i] += mv.k;
        }
    }

    let mut rem_memo = HashMap::new();
    if !sort_route_buckets_to_goal_with_prefix_caps(
        &mut sim,
        &first_group,
        &fixed,
        memo,
        &mut rem_memo,
    ) {
        return None;
    }
    assert!(sim.is_goal());

    Some(CandidateSolution {
        turn_count: sim.turns.len(),
        turns: sim.turns,
    })
}

fn solve_route_strategy(
    init: &[Vec<usize>],
    mode: usize,
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> CandidateSolution {
    let db = DistBuilder::new(init);
    let mut nodes = Vec::new();
    let final_ids = if mode == 1 {
        let id = db.greedy_final_node(&mut nodes, 0);
        vec![id]
    } else if mode == 2 {
        db.beam_final_nodes(&mut nodes, 1200, 80, 0, 1, 3)
    } else {
        db.beam_final_nodes(&mut nodes, 3000, 100, 0, 2, 12)
    };

    let mut best = CandidateSolution::empty();
    for id in final_ids {
        let sol = make_solution_from_distribution(&db, &nodes, id, init, memo);
        if sol.turn_count < best.turn_count {
            best = sol;
        }
        let mixed = make_solution_from_distribution_mixed_b(&db, &nodes, id, init, memo);
        if mixed.turn_count < best.turn_count {
            best = mixed;
        }
        if let Some(mixed_all) =
            make_solution_from_distribution_mixed_all(&db, &nodes, id, init, memo)
        {
            if mixed_all.turn_count < best.turn_count {
                best = mixed_all;
            }
        }
    }

    if mode == 3 {
        for pen in [10, 50] {
            let mut greedy_nodes = Vec::new();
            let gid = db.greedy_final_node(&mut greedy_nodes, pen);
            let sol = make_solution_from_distribution(&db, &greedy_nodes, gid, init, memo);
            if sol.turn_count < best.turn_count {
                best = sol;
            }
            let mixed =
                make_solution_from_distribution_mixed_b(&db, &greedy_nodes, gid, init, memo);
            if mixed.turn_count < best.turn_count {
                best = mixed;
            }
            if let Some(mixed_all) =
                make_solution_from_distribution_mixed_all(&db, &greedy_nodes, gid, init, memo)
            {
                if mixed_all.turn_count < best.turn_count {
                    best = mixed_all;
                }
            }
        }
    }

    best
}

fn old_distribute_to_route_sidings(sim: &mut Sim) {
    loop {
        let mut any = false;
        let mut cj = [usize::MAX; R];
        let mut clen = [0usize; R];
        for i in 0..R {
            if sim.dep[i].is_empty() {
                continue;
            }
            any = true;
            let j = route_of(*sim.dep[i].last().unwrap());
            let mut len = 0usize;
            for &x in sim.dep[i].iter().rev() {
                if route_of(x) == j {
                    len += 1;
                } else {
                    break;
                }
            }
            cj[i] = j;
            clen[i] = len;
        }
        if !any {
            break;
        }

        let mut best_mask = 0usize;
        let mut best_score = -1i32;
        for mask in 1usize..(1usize << R) {
            let mut prev = usize::MAX;
            let mut used = 0usize;
            let mut cnt = 0i32;
            let mut lsum = 0i32;
            let mut ok = true;
            for i in 0..R {
                if (mask >> i) & 1 == 0 {
                    continue;
                }
                if clen[i] == 0 {
                    ok = false;
                    break;
                }
                let j = cj[i];
                if (prev != usize::MAX && j <= prev) || ((used >> j) & 1 == 1) {
                    ok = false;
                    break;
                }
                prev = j;
                used |= 1usize << j;
                cnt += 1;
                lsum += clen[i] as i32;
            }
            if ok {
                let sc = cnt * 1000 + lsum;
                if sc > best_score {
                    best_score = sc;
                    best_mask = mask;
                }
            }
        }

        let mut ops = Vec::new();
        for i in 0..R {
            if (best_mask >> i) & 1 == 1 {
                ops.push(Move::dep_to_siding(i, cj[i], clen[i]));
            }
        }
        sim.add_turn(ops);
    }
}

fn old_find_in_siding(sim: &Sim, j: usize, car: usize) -> Option<usize> {
    sim.sid[j].iter().position(|&x| x == car)
}

fn old_run_len_from_top(sim: &Sim, r: usize) -> usize {
    let k = sim.dep[r].len();
    let mut len = 0usize;
    while k + len < INIT_LEN && len < sim.sid[r].len() && sim.sid[r][len] == target_id(r, k + len) {
        len += 1;
    }
    len
}

fn old_seq_extract(sim: &mut Sim, r: usize, use_run: bool) {
    let k = sim.dep[r].len();
    let car = target_id(r, k);
    let pos = old_find_in_siding(sim, r, car).unwrap();
    let mut chunks = Vec::new();
    let mut rem = pos;
    while rem > 0 {
        let mut best_t = usize::MAX;
        let mut best_sp = 0usize;
        for t in 0..R {
            if t != r {
                let sp = DEP_CAP - sim.dep[t].len();
                if sp > best_sp {
                    best_sp = sp;
                    best_t = t;
                }
            }
        }
        let take = rem.min(best_sp);
        assert!(take > 0 && best_t != usize::MAX);
        sim.add_turn(vec![Move::siding_to_dep(best_t, r, take)]);
        chunks.push((best_t, take));
        rem -= take;
    }

    let len = if use_run {
        old_run_len_from_top(sim, r)
    } else {
        1
    };
    sim.add_turn(vec![Move::siding_to_dep(r, r, len)]);

    for &(t, take) in chunks.iter().rev() {
        sim.add_turn(vec![Move::dep_to_siding(t, r, take)]);
    }
}

fn old_dynamic_sort(sim: &mut Sim, use_run: bool) {
    loop {
        let mut all = true;
        let mut ready = Vec::new();
        let mut blocked = Vec::new();
        let mut pos_next = [usize::MAX; R];

        for r in 0..R {
            if sim.dep[r].len() == INIT_LEN {
                continue;
            }
            all = false;
            let car = target_id(r, sim.dep[r].len());
            let p = old_find_in_siding(sim, r, car).unwrap();
            pos_next[r] = p;
            if p == 0 {
                ready.push(r);
            } else {
                blocked.push(r);
            }
        }
        if all {
            break;
        }

        if blocked.is_empty() {
            let mut ops = Vec::new();
            for r in ready {
                let len = if use_run {
                    old_run_len_from_top(sim, r)
                } else {
                    1
                };
                ops.push(Move::siding_to_dep(r, r, len));
            }
            sim.add_turn(ops);
            continue;
        }

        let start = if blocked[0] < 5 { 0 } else { 5 };
        let mut b = Vec::new();
        for r in start..R.min(start + 5) {
            if sim.dep[r].len() < INIT_LEN && pos_next[r] > 0 {
                b.push(r);
            }
        }
        if b.is_empty() {
            b.push(blocked[0]);
        }

        let mut bmask = 0usize;
        for &r in &b {
            bmask |= 1usize << r;
        }
        let mut tlines = Vec::new();
        for t in 0..R {
            if (bmask >> t) & 1 == 0 {
                tlines.push(t);
            }
        }
        while tlines.len() > b.len() {
            tlines.pop();
        }

        let mut ok = true;
        for z in 0..b.len() {
            if sim.dep[tlines[z]].len() + pos_next[b[z]] > DEP_CAP {
                ok = false;
            }
        }
        if !ok {
            old_seq_extract(sim, blocked[0], use_run);
            continue;
        }

        let mut out = Vec::new();
        for z in 0..b.len() {
            out.push(Move::siding_to_dep(tlines[z], b[z], pos_next[b[z]]));
        }
        sim.add_turn(out);

        let mut mid = Vec::new();
        let mut tmask = 0usize;
        for &x in &tlines {
            tmask |= 1usize << x;
        }
        for &r in &b {
            let len = if use_run {
                old_run_len_from_top(sim, r)
            } else {
                1
            };
            mid.push(Move::siding_to_dep(r, r, len));
        }
        for r in ready {
            if ((bmask >> r) & 1 == 0) && ((tmask >> r) & 1 == 0) {
                let len = if use_run {
                    old_run_len_from_top(sim, r)
                } else {
                    1
                };
                mid.push(Move::siding_to_dep(r, r, len));
            }
        }
        sim.add_turn(mid);

        let mut back = Vec::new();
        for z in 0..b.len() {
            back.push(Move::dep_to_siding(tlines[z], b[z], pos_next[b[z]]));
        }
        sim.add_turn(back);
    }
}

fn solve_old_fallback(init: &[Vec<usize>]) -> CandidateSolution {
    let mut sim = Sim::new(init);
    old_distribute_to_route_sidings(&mut sim);
    old_dynamic_sort(&mut sim, true);
    assert!(sim.is_goal());

    CandidateSolution {
        turn_count: sim.turns.len(),
        turns: sim.turns,
    }
}

fn solve(init: &[Vec<usize>]) -> CandidateSolution {
    let _time_limit_sec = TIME_LIMIT_SEC;
    let mut memo = HashMap::new();

    let mut best = solve_route_strategy(init, 3, &mut memo);
    let old = solve_old_fallback(init);
    if old.turn_count < best.turn_count {
        best = old;
    }

    assert!(best.turn_count <= MAX_TURNS);
    best
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
        let mut ops = moves.clone();
        ops.sort_unstable_by_key(|mv| (mv.i, mv.j));
        for mv in ops {
            out.push_str(&format!("{} {} {} {}\n", mv.kind, mv.i, mv.j, mv.k));
        }
    }

    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    stdout.write_all(out.as_bytes()).unwrap();
}

fn main() {
    let init = read_input();
    let sol = solve(&init);
    print_turns(&sol.turns);
}
