// v101pro_best.rs
use std::collections::HashMap;
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

#[derive(Clone, Debug)]
struct Plan {
    take: usize,
    total_skip: usize,
    chunks: Vec<usize>,
    segs: Vec<usize>,
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

    fn greedy_group_masks(&self, runs: &[Vec<(usize, usize)>]) -> Vec<usize> {
        let mut idx = [0usize; R];
        let mut path = Vec::new();

        loop {
            let mut js = [usize::MAX; R];
            let mut ks = [0usize; R];
            let mut any = false;
            for i in 0..R {
                if idx[i] < runs[i].len() {
                    any = true;
                    js[i] = runs[i][idx[i]].0;
                    ks[i] = runs[i][idx[i]].1;
                }
            }
            if !any {
                break;
            }

            let mut best_mask = 0usize;
            let mut best_cnt = 0usize;
            let mut best_len = 0usize;
            for mask in 1usize..(1usize << R) {
                let mut last_j = usize::MAX;
                let mut cnt = 0usize;
                let mut len = 0usize;
                let mut ok = true;
                for i in 0..R {
                    if (mask >> i) & 1 == 0 {
                        continue;
                    }
                    if js[i] == usize::MAX || (last_j != usize::MAX && js[i] <= last_j) {
                        ok = false;
                        break;
                    }
                    last_j = js[i];
                    cnt += 1;
                    len += ks[i];
                }
                if !ok {
                    continue;
                }
                if cnt > best_cnt || (cnt == best_cnt && len > best_len) {
                    best_cnt = cnt;
                    best_len = len;
                    best_mask = mask;
                }
            }

            assert!(best_mask != 0);
            for (i, slot) in idx.iter_mut().enumerate() {
                if (best_mask >> i) & 1 == 1 {
                    *slot += 1;
                }
            }
            path.push(best_mask);
        }

        path
    }

    fn initial_grouping(&mut self) {
        let mut runs = vec![Vec::<(usize, usize)>::new(); R];
        let mut run_weight = vec![Vec::<i32>::new(); R];

        for i in 0..R {
            let mut p = self.dep[i].len();
            while p > 0 {
                let j = Self::target_line(self.dep[i][p - 1]);
                let mut k = 0usize;
                let mut w = 0i32;
                while p > 0 && Self::target_line(self.dep[i][p - 1]) == j {
                    w += (INIT_LEN - (self.dep[i][p - 1] % INIT_LEN)) as i32;
                    k += 1;
                    p -= 1;
                }
                runs[i].push((j, k));
                run_weight[i].push(w);
            }
        }

        let mut best_path = self.greedy_group_masks(&runs);
        let greedy_turns = best_path.len();

        #[derive(Clone, Copy)]
        struct Node {
            idx: [u8; R],
            parent: Option<usize>,
            mask: u16,
            depth: u8,
            h: u8,
            rem: u8,
            ord: i32,
        }

        fn encode(idx: &[u8; R]) -> u64 {
            let mut code = 0u64;
            for &v in idx {
                code = code * 11 + v as u64;
            }
            code
        }

        fn heuristic(runs: &[Vec<(usize, usize)>], idx: &[u8; R]) -> (usize, usize) {
            let mut rem = 0usize;
            let mut max_line = 0usize;
            let mut cnt_target = [0usize; R];
            for i in 0..R {
                let ii = idx[i] as usize;
                let left = runs[i].len() - ii;
                rem += left;
                max_line = max_line.max(left);
                for t in ii..runs[i].len() {
                    cnt_target[runs[i][t].0] += 1;
                }
            }
            let max_target = cnt_target.into_iter().max().unwrap();
            let h = ((rem + R - 1) / R).max(max_line).max(max_target);
            (h, rem)
        }

        let mut nodes = Vec::with_capacity(200_000);
        let (h0, rem0) = heuristic(&runs, &[0; R]);
        nodes.push(Node {
            idx: [0; R],
            parent: None,
            mask: 0,
            depth: 0,
            h: h0 as u8,
            rem: rem0 as u8,
            ord: 0,
        });

        let mut cur = vec![0usize];
        let mut found = None::<usize>;
        const BEAM_WIDTH: usize = 2000;
        let start = Instant::now();

        for depth in 0..greedy_turns {
            if found.is_some() {
                break;
            }

            let mut nxt = Vec::with_capacity(BEAM_WIDTH * 8);
            let mut seen = HashMap::<u64, usize>::with_capacity(BEAM_WIDTH * 32);

            for &id in &cur {
                let mut js = [usize::MAX; R];
                let mut ks = [0usize; R];
                for i in 0..R {
                    let ii = nodes[id].idx[i] as usize;
                    if ii < runs[i].len() {
                        js[i] = runs[i][ii].0;
                        ks[i] = runs[i][ii].1;
                    }
                }

                let mut valid = [false; 1 << R];
                for (mask, slot) in valid.iter_mut().enumerate().skip(1) {
                    let mut last_j = usize::MAX;
                    let mut ok = true;
                    for (i, &j) in js.iter().enumerate() {
                        if (mask >> i) & 1 == 0 {
                            continue;
                        }
                        if j == usize::MAX || (last_j != usize::MAX && j <= last_j) {
                            ok = false;
                            break;
                        }
                        last_j = j;
                    }
                    *slot = ok;
                }

                for mask in 1usize..(1usize << R) {
                    if !valid[mask] {
                        continue;
                    }
                    let mut maximal = true;
                    for (i, &j) in js.iter().enumerate() {
                        if (mask >> i) & 1 == 0 && j != usize::MAX && valid[mask | (1usize << i)] {
                            maximal = false;
                            break;
                        }
                    }
                    if !maximal {
                        continue;
                    }

                    let mut ni = nodes[id].idx;
                    for (i, slot) in ni.iter_mut().enumerate() {
                        if (mask >> i) & 1 == 1 {
                            *slot += 1;
                        }
                    }
                    let code = encode(&ni);
                    if seen.contains_key(&code) {
                        continue;
                    }

                    let (h, rem) = heuristic(&runs, &ni);
                    let mut ord = nodes[id].ord;
                    for ii in 0..R {
                        if (mask >> ii) & 1 == 1 {
                            ord += (depth as i32 + 1) * run_weight[ii][nodes[id].idx[ii] as usize];
                        }
                    }

                    let nid = nodes.len();
                    nodes.push(Node {
                        idx: ni,
                        parent: Some(id),
                        mask: mask as u16,
                        depth: (depth + 1) as u8,
                        h: h as u8,
                        rem: rem as u8,
                        ord,
                    });
                    seen.insert(code, nid);
                    nxt.push(nid);

                    if rem == 0 && found.map_or(true, |old| nodes[nid].ord > nodes[old].ord) {
                        found = Some(nid);
                    }
                }
            }

            if found.is_some() {
                break;
            }

            nxt.sort_unstable_by(|&a, &b| {
                let a_node = nodes[a];
                let b_node = nodes[b];
                let ea = a_node.depth as usize + a_node.h as usize;
                let eb = b_node.depth as usize + b_node.h as usize;
                ea.cmp(&eb)
                    .then_with(|| a_node.h.cmp(&b_node.h))
                    .then_with(|| a_node.rem.cmp(&b_node.rem))
                    .then_with(|| b_node.ord.cmp(&a_node.ord))
                    .then_with(|| b_node.mask.cmp(&a_node.mask))
            });
            if nxt.len() > BEAM_WIDTH {
                nxt.truncate(BEAM_WIDTH);
            }
            cur = nxt;
            if cur.is_empty() {
                break;
            }
            if depth >= 4 && start.elapsed().as_secs_f64() > TIME_LIMIT_SEC {
                break;
            }
        }

        if let Some(mut id) = found {
            let mut path = Vec::new();
            while let Some(parent) = nodes[id].parent {
                path.push(nodes[id].mask as usize);
                id = parent;
            }
            path.reverse();
            if path.len() <= best_path.len() {
                best_path = path;
            }
        }

        let mut idx = [0usize; R];
        for mask in best_path {
            let mut moves = Vec::new();
            for i in 0..R {
                if (mask >> i) & 1 == 1 {
                    let (j, k) = runs[i][idx[i]];
                    moves.push(Move::dep_to_siding(i, j, k));
                    idx[i] += 1;
                }
            }
            self.add_turn(moves);
        }
    }

    fn find_in_siding(&self, r: usize, car: usize) -> usize {
        self.sid[r]
            .iter()
            .position(|&x| x == car)
            .unwrap_or_else(|| panic!("car {} is not in siding {}", car, r))
    }

    fn sort_sidings_selection(&mut self) {
        for c in 0..INIT_LEN {
            let mut p = [0usize; R];
            let mut pos_groups = Vec::new();
            let mut zero_groups = Vec::new();

            for r in 0..R {
                p[r] = self.find_in_siding(r, Self::target_id(r, c));
                if p[r] == 0 {
                    zero_groups.push(r);
                } else {
                    pos_groups.push(r);
                }
            }

            let mut ip = 0usize;
            let mut iz = 0usize;
            while ip < pos_groups.len() {
                let rem = pos_groups.len() - ip;
                let a = 5usize.min(rem);
                let active_p = pos_groups[ip..ip + a].to_vec();
                ip += a;

                let zcap = R.saturating_sub(2 * a);
                let mut active_z = Vec::new();
                while iz < zero_groups.len() && active_z.len() < zcap {
                    active_z.push(zero_groups[iz]);
                    iz += 1;
                }

                let mut is_active = [false; R];
                for &r in &active_p {
                    is_active[r] = true;
                }
                for &r in &active_z {
                    is_active[r] = true;
                }

                let mut cand_buf = Vec::new();
                for (b, &active) in is_active.iter().enumerate() {
                    if !active {
                        cand_buf.push(b);
                    }
                }
                assert!(cand_buf.len() >= a);

                let buffers = cand_buf[..a].to_vec();
                for t in 0..a {
                    let r = active_p[t];
                    let b = buffers[t];
                    assert!(self.dep[b].len() + p[r] <= DEP_CAP);
                }

                let mut moves1 = Vec::new();
                for t in 0..a {
                    moves1.push(Move::siding_to_dep(buffers[t], active_p[t], p[active_p[t]]));
                }
                self.add_turn(moves1);

                let mut active = active_p.clone();
                active.extend_from_slice(&active_z);
                active.sort_unstable();
                let mut moves2 = Vec::new();
                for r in active {
                    moves2.push(Move::siding_to_dep(r, r, 1));
                }
                self.add_turn(moves2);

                let mut moves3 = Vec::new();
                for t in 0..a {
                    moves3.push(Move::dep_to_siding(buffers[t], active_p[t], p[active_p[t]]));
                }
                self.add_turn(moves3);
            }

            if iz < zero_groups.len() {
                let mut moves = Vec::new();
                while iz < zero_groups.len() {
                    let r = zero_groups[iz];
                    moves.push(Move::siding_to_dep(r, r, 1));
                    iz += 1;
                }
                self.add_turn(moves);
            }
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

    fn sort_sidings_scan(&mut self) {
        let mut guard = 0usize;
        loop {
            let mut done = true;
            for r in 0..R {
                if self.dep[r].len() < INIT_LEN {
                    done = false;
                    break;
                }
            }
            if done {
                break;
            }

            guard += 1;
            assert!(guard < 100);

            let mut plans = (0..R).map(|_| Plan::new()).collect::<Vec<_>>();
            let mut need_buf = Vec::new();
            let mut no_buf = Vec::new();

            for r in 0..R {
                if self.dep[r].len() < INIT_LEN {
                    plans[r] = self.make_scan_plan(r);
                    assert!(plans[r].take > 0);
                    if plans[r].total_skip > 0 {
                        need_buf.push(r);
                    } else {
                        no_buf.push(r);
                    }
                }
            }

            need_buf.sort_unstable_by(|&a, &b| {
                self.dep[a]
                    .len()
                    .cmp(&self.dep[b].len())
                    .then_with(|| plans[b].total_skip.cmp(&plans[a].total_skip))
                    .then_with(|| a.cmp(&b))
            });

            let mut ptr = 0usize;
            while ptr < need_buf.len() {
                let mut found = false;
                let mut best_a = 0usize;
                let mut best_active_p = Vec::new();
                let mut best_buffers = Vec::new();

                let max_a = 5usize.min(need_buf.len() - ptr);
                for a in (1..=max_a).rev() {
                    let mut active_p = need_buf[ptr..ptr + a].to_vec();
                    active_p.sort_unstable();
                    let active = active_p.clone();
                    let mut buffers = Vec::new();
                    if self.assign_buffers_for(&active_p, &plans, &active, &mut buffers) {
                        best_a = a;
                        best_active_p = active_p;
                        best_buffers = buffers;
                        found = true;
                        break;
                    }
                }

                if !found {
                    let r = need_buf[ptr];
                    let car = Self::target_id(r, self.dep[r].len());
                    let p = self.find_in_siding(r, car);
                    let mut b = None::<usize>;
                    for x in 0..R {
                        if x != r && self.dep[x].len() + p <= DEP_CAP {
                            b = Some(x);
                            break;
                        }
                    }
                    let b = b.unwrap();
                    if p > 0 {
                        self.add_turn(vec![Move::siding_to_dep(b, r, p)]);
                    }
                    self.add_turn(vec![Move::siding_to_dep(r, r, 1)]);
                    if p > 0 {
                        self.add_turn(vec![Move::dep_to_siding(b, r, p)]);
                    }
                    ptr += 1;
                    continue;
                }

                self.execute_scan_batch(&best_active_p, &best_buffers, &[], &plans);
                ptr += best_a;
            }

            let mut mz = Vec::new();
            for r in no_buf {
                if self.dep[r].len() < INIT_LEN {
                    let plan = self.make_scan_plan(r);
                    if plan.total_skip == 0 && plan.take > 0 {
                        mz.push(Move::siding_to_dep(r, r, plan.take));
                    }
                }
            }
            self.add_turn(mz);
        }
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
        self.initial_grouping();

        for j in 0..R {
            assert_eq!(self.sid[j].len(), INIT_LEN);
            for &x in &self.sid[j] {
                assert_eq!(Self::target_line(x), j);
            }
        }

        let base_dep = self.dep.clone();
        let base_sid = self.sid.clone();
        let base_turns = self.turns.clone();

        self.sort_sidings_selection();
        let best_dep = self.dep.clone();
        let best_sid = self.sid.clone();
        let mut best_turns = self.turns.clone();
        assert!(self.complete());

        self.dep = base_dep;
        self.sid = base_sid;
        self.turns = base_turns;
        self.sort_sidings_scan();
        assert!(self.complete());

        if self.turns.len() < best_turns.len() {
            best_turns = self.turns.clone();
        } else {
            self.dep = best_dep;
            self.sid = best_sid;
        }
        self.turns = best_turns;

        assert!(self.complete());
        assert!(self.turns.len() <= MAX_TURNS);
    }
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
