// v101pro_token_beam_solver.rs
use rustc_hash::FxHashMap;
use std::io::{self, Read};
use std::time::Instant;

const MAX_TURNS: usize = 100_000;
const TIME_LIMIT_SEC: f64 = 1.85;
const MAX_DEPTH: usize = 12;
const BEAM_WIDTH: usize = 800;
const MAX_STATES: usize = 25_000;
const EXTRA_LIMIT: usize = 12;
const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];

#[derive(Clone)]
struct Input {
    n: usize,
    m: usize,
    d: Vec<u8>,
    food: Vec<u8>,
}

#[derive(Clone)]
struct State {
    n: usize,
    food: Vec<u8>,
    pos: Vec<u16>,
    colors: Vec<u8>,
}

#[derive(Clone, Default)]
struct MoveInfo {
    ate_color: u8,
    bite_idx: Option<usize>,
    dropped: Vec<Dropped>,
}

#[derive(Clone)]
struct Dropped {
    idx: usize,
    cell: u16,
    color: u8,
}

#[derive(Hash, Eq, PartialEq)]
struct Key {
    pos: Vec<u16>,
    colors: Vec<u8>,
    food: Vec<(u16, u8)>,
    carry: Option<u16>,
}

struct Node {
    state: State,
    carry: Option<usize>,
    parent: Option<usize>,
    action: String,
}

struct ResetCandidate {
    ops: String,
    state: State,
    newcarry: Option<usize>,
    carried_dropped: Option<(u16, u8)>,
}

impl State {
    fn initial(input: &Input) -> Self {
        let n = input.n;
        let pos = vec![
            cell_of(4, 0, n),
            cell_of(3, 0, n),
            cell_of(2, 0, n),
            cell_of(1, 0, n),
            cell_of(0, 0, n),
        ];
        Self {
            n,
            food: input.food.clone(),
            pos,
            colors: vec![1; 5],
        }
    }
}

#[inline]
fn cell_of(r: usize, c: usize, n: usize) -> u16 {
    (r * n + c) as u16
}

#[inline]
fn rc_of(cell: u16, n: usize) -> (usize, usize) {
    let x = cell as usize;
    (x / n, x % n)
}

#[inline]
fn manhattan(n: usize, a: u16, b: u16) -> usize {
    let (ar, ac) = rc_of(a, n);
    let (br, bc) = rc_of(b, n);
    ar.abs_diff(br) + ac.abs_diff(bc)
}

#[inline]
fn time_over(start: &Instant) -> bool {
    start.elapsed().as_secs_f64() >= TIME_LIMIT_SEC
}

fn read_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut d = Vec::with_capacity(m);
    for _ in 0..m {
        d.push(it.next().unwrap().parse::<u8>().unwrap());
    }

    let mut food = vec![0_u8; n * n];
    for r in 0..n {
        for c in 0..n {
            food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
        }
    }

    Input { n, m, d, food }
}

#[inline]
fn next_head_cell(st: &State, dir: usize) -> Option<u16> {
    let (dr, dc, _) = DIRS[dir];
    let (hr, hc) = rc_of(st.pos[0], st.n);
    let nr = hr as isize + dr;
    let nc = hc as isize + dc;
    if nr < 0 || nr >= st.n as isize || nc < 0 || nc >= st.n as isize {
        return None;
    }
    Some(cell_of(nr as usize, nc as usize, st.n))
}

#[inline]
fn is_legal_dir(st: &State, dir: usize) -> bool {
    let Some(nh) = next_head_cell(st, dir) else {
        return false;
    };
    if st.pos.len() >= 2 && nh == st.pos[1] {
        return false;
    }
    true
}

fn legal_dirs(st: &State) -> Vec<usize> {
    let mut out = Vec::with_capacity(4);
    for dir in 0..4 {
        if is_legal_dir(st, dir) {
            out.push(dir);
        }
    }
    out
}

fn step_info(st: &State, dir: usize) -> (State, MoveInfo) {
    let nh = next_head_cell(st, dir).unwrap();
    let old_len = st.pos.len();

    let mut food = st.food.clone();
    let mut new_pos = Vec::with_capacity(old_len + 1);
    new_pos.push(nh);
    new_pos.extend_from_slice(&st.pos[..old_len - 1]);

    let mut new_colors = st.colors.clone();
    let mut info = MoveInfo {
        ate_color: food[nh as usize],
        ..MoveInfo::default()
    };

    if info.ate_color != 0 {
        new_pos.push(st.pos[old_len - 1]);
        new_colors.push(info.ate_color);
        food[nh as usize] = 0;
    }

    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }
    info.bite_idx = bite_idx;

    if let Some(bi) = bite_idx {
        let mut dropped = Vec::with_capacity(new_pos.len().saturating_sub(bi + 1));
        for p in bi + 1..new_pos.len() {
            let cell = new_pos[p];
            let color = new_colors[p];
            food[cell as usize] = color;
            dropped.push(Dropped {
                idx: p,
                cell,
                color,
            });
        }
        new_pos.truncate(bi + 1);
        new_colors.truncate(bi + 1);
        info.dropped = dropped;
    }

    (
        State {
            n: st.n,
            food,
            pos: new_pos,
            colors: new_colors,
        },
        info,
    )
}

#[inline]
fn colors_prefix_ok(st: &State, d: &[u8], ell: usize) -> bool {
    let upto = st.colors.len().min(ell);
    st.colors[..upto] == d[..upto]
}

#[inline]
fn prefix_exact(st: &State, d: &[u8], ell: usize) -> bool {
    st.colors.len() >= ell && st.colors[..ell] == d[..ell]
}

fn encode_key(st: &State, carry: Option<usize>) -> Key {
    let mut food = Vec::with_capacity(st.colors.len() + 16);
    for (idx, &col) in st.food.iter().enumerate() {
        if col != 0 {
            food.push((idx as u16, col));
        }
    }
    Key {
        pos: st.pos.clone(),
        colors: st.colors.clone(),
        food,
        carry: carry.map(|x| x as u16),
    }
}

fn recover_from_dropped(
    ns: &State,
    dropped: &[Dropped],
    input: &Input,
    ell: usize,
) -> Option<(State, String)> {
    if ns.colors.len() > ell {
        return None;
    }

    let mut s = ns.clone();
    let mut ops = String::new();
    let need_cnt = ell - s.colors.len();

    for ent in dropped.iter().take(need_cnt) {
        let need = input.d[s.colors.len()];
        if ent.color != need {
            return None;
        }

        let mut found = false;
        for dir in legal_dirs(&s) {
            let nh = next_head_cell(&s, dir).unwrap();
            if nh != ent.cell {
                continue;
            }
            if s.food[ent.cell as usize] != need {
                continue;
            }
            let (s2, info) = step_info(&s, dir);
            if info.bite_idx.is_some() {
                return None;
            }
            s = s2;
            ops.push(DIRS[dir].2);
            found = true;
            break;
        }
        if !found {
            return None;
        }
    }

    Some((s, ops))
}

fn reset_candidates(st: &State, input: &Input, ell: usize, carry: Option<usize>) -> Vec<ResetCandidate> {
    let mut out = Vec::new();

    for dir in legal_dirs(st) {
        let (ns, info) = step_info(st, dir);
        let Some(bite_idx) = info.bite_idx else {
            continue;
        };

        let mut carried_dropped = None;
        let mut newcarry = carry;

        if let Some(c) = carry {
            if c > bite_idx {
                for ent in &info.dropped {
                    if ent.idx == c {
                        carried_dropped = Some((ent.cell, ent.color));
                        break;
                    }
                }
                newcarry = None;
            }
        }

        if !colors_prefix_ok(&ns, &input.d, ell) {
            continue;
        }

        if ns.colors.len() >= ell {
            let mut ops = String::new();
            ops.push(DIRS[dir].2);
            out.push(ResetCandidate {
                ops,
                state: ns,
                newcarry,
                carried_dropped,
            });
        } else if let Some((rs, rop)) = recover_from_dropped(&ns, &info.dropped, input, ell) {
            let mut ops = String::new();
            ops.push(DIRS[dir].2);
            ops.push_str(&rop);
            out.push(ResetCandidate {
                ops,
                state: rs,
                newcarry,
                carried_dropped,
            });
        }
    }

    out
}

fn direct_success(st: &State, input: &Input, ell: usize) -> Option<String> {
    if st.colors.len() != ell || !prefix_exact(st, &input.d, ell) {
        return None;
    }

    let target = input.d[ell];
    for dir in legal_dirs(st) {
        let nh = next_head_cell(st, dir).unwrap();
        if st.food[nh as usize] != target {
            continue;
        }
        let (ns, _) = step_info(st, dir);
        if ns.colors.len() >= ell + 1 && ns.colors[..=ell] == input.d[..=ell] {
            return Some(DIRS[dir].2.to_string());
        }
    }

    None
}

fn success_via_carry(st: &State, input: &Input, ell: usize, carry: Option<usize>) -> Option<String> {
    let Some(carry_idx) = carry else {
        return None;
    };
    if carry_idx >= st.pos.len() {
        return None;
    }

    let target = input.d[ell];
    for cand in reset_candidates(st, input, ell, carry) {
        let Some((cell, color)) = cand.carried_dropped else {
            continue;
        };
        if color != target {
            continue;
        }

        let head = cand.state.pos[0];
        if manhattan(cand.state.n, head, cell) != 1 {
            continue;
        }

        for dir in legal_dirs(&cand.state) {
            let nh = next_head_cell(&cand.state, dir).unwrap();
            if nh != cell {
                continue;
            }
            if cand.state.food[cell as usize] != target {
                continue;
            }
            let (ns, _) = step_info(&cand.state, dir);
            if ns.colors.len() >= ell + 1 && ns.colors[..=ell] == input.d[..=ell] {
                let mut ops = cand.ops;
                ops.push(DIRS[dir].2);
                return Some(ops);
            }
        }
    }

    None
}

fn heur(st: &State, input: &Input, ell: usize, carry: Option<usize>) -> (usize, usize, usize, usize) {
    if let Some(cidx) = carry {
        if cidx >= st.pos.len() {
            return (1_000_000, 0, 0, 0);
        }
        let cell = st.pos[cidx];
        let ref_pos = st.pos[st.pos.len().saturating_sub(1).min(ell.saturating_sub(1))];
        let d1 = manhattan(st.n, cell, ref_pos);
        let gap = cidx.saturating_sub(ell);
        (1, d1, gap, 0)
    } else {
        let target = input.d[ell];
        let head = st.pos[0];
        let mut best = usize::MAX;
        for (idx, &col) in st.food.iter().enumerate() {
            if col == target {
                let dist = manhattan(st.n, head, idx as u16);
                if dist < best {
                    best = dist;
                }
            }
        }
        if best == usize::MAX {
            return (1_000_000, 0, 0, 0);
        }
        let excess = st.colors.len().saturating_sub(ell);
        (0, best, excess, 0)
    }
}

fn beam_extend_with_carry(state: &State, input: &Input, ell: usize, start: &Instant) -> Option<String> {
    let mut nodes = Vec::with_capacity(BEAM_WIDTH * 8);
    nodes.push(Node {
        state: state.clone(),
        carry: None,
        parent: None,
        action: String::new(),
    });

    let mut seen = FxHashMap::<Key, usize>::default();
    seen.insert(encode_key(state, None), 0);

    let mut frontier = vec![0_usize];
    let mut goal: Option<(usize, String)> = None;

    for depth in 0..=MAX_DEPTH {
        if time_over(start) {
            break;
        }

        for &nid in &frontier {
            let st = &nodes[nid].state;
            let carry = nodes[nid].carry;

            if let Some(ds) = direct_success(st, input, ell) {
                goal = Some((nid, ds));
                break;
            }
            if let Some(cs) = success_via_carry(st, input, ell, carry) {
                goal = Some((nid, cs));
                break;
            }
        }

        if goal.is_some() || depth == MAX_DEPTH {
            break;
        }

        let mut cand = Vec::new();

        for &nid in &frontier {
            if time_over(start) || seen.len() >= MAX_STATES {
                break;
            }

            let base_state = nodes[nid].state.clone();
            let base_carry = nodes[nid].carry;

            for rc in reset_candidates(&base_state, input, ell, base_carry) {
                if rc.state.colors.len() != ell {
                    continue;
                }
                let key = encode_key(&rc.state, rc.newcarry);
                if seen.contains_key(&key) {
                    continue;
                }
                let child = nodes.len();
                nodes.push(Node {
                    state: rc.state,
                    carry: rc.newcarry,
                    parent: Some(nid),
                    action: rc.ops,
                });
                seen.insert(key, child);
                cand.push(child);
                if seen.len() >= MAX_STATES {
                    break;
                }
            }
            if seen.len() >= MAX_STATES {
                break;
            }

            for dir in legal_dirs(&base_state) {
                let mut action = String::new();
                action.push(DIRS[dir].2);

                let target = input.d[ell];
                let (mut ns, info) = step_info(&base_state, dir);

                let mut newcarry = base_carry;
                if base_carry.is_none() && info.ate_color == target {
                    newcarry = Some(base_state.colors.len());
                    if let Some(bite_idx) = info.bite_idx {
                        if newcarry.unwrap() > bite_idx {
                            newcarry = None;
                        }
                    }
                } else if let Some(c) = base_carry {
                    if let Some(bite_idx) = info.bite_idx {
                        if c > bite_idx {
                            newcarry = None;
                        }
                    }
                }

                if info.bite_idx.is_some() && ns.colors.len() < ell && colors_prefix_ok(&ns, &input.d, ell) {
                    let Some((recovered, rop)) = recover_from_dropped(&ns, &info.dropped, input, ell)
                    else {
                        continue;
                    };
                    ns = recovered;
                    action.push_str(&rop);
                }

                if !(ns.colors.len() >= ell && ns.colors[..ell] == input.d[..ell]) {
                    continue;
                }
                if ns.colors.len() > ell + EXTRA_LIMIT {
                    continue;
                }

                if let Some(cidx) = newcarry {
                    if cidx >= ns.colors.len() {
                        newcarry = None;
                    }
                }

                let key = encode_key(&ns, newcarry);
                if seen.contains_key(&key) {
                    continue;
                }

                let child = nodes.len();
                nodes.push(Node {
                    state: ns,
                    carry: newcarry,
                    parent: Some(nid),
                    action,
                });
                seen.insert(key, child);
                cand.push(child);

                if seen.len() >= MAX_STATES {
                    break;
                }
            }
        }

        if cand.is_empty() {
            break;
        }

        cand.sort_unstable_by_key(|&nid| heur(&nodes[nid].state, input, ell, nodes[nid].carry));
        if cand.len() > BEAM_WIDTH {
            cand.truncate(BEAM_WIDTH);
        }
        frontier = cand;
    }

    let (goal_id, suffix) = goal?;

    let mut parts = Vec::<String>::new();
    let mut cur = goal_id;
    while let Some(parent) = nodes[cur].parent {
        parts.push(nodes[cur].action.clone());
        cur = parent;
    }
    parts.reverse();

    let mut ops = String::new();
    for part in parts {
        ops.push_str(&part);
    }
    ops.push_str(&suffix);
    Some(ops)
}

fn solve(input: &Input) -> Vec<char> {
    let start = Instant::now();
    let mut state = State::initial(input);
    let mut ops = Vec::new();

    for ell in 5..input.m {
        if ops.len() >= MAX_TURNS || time_over(&start) {
            break;
        }
        if !prefix_exact(&state, &input.d, ell) {
            break;
        }

        let Some(plan) = beam_extend_with_carry(&state, input, ell, &start) else {
            break;
        };
        if plan.is_empty() {
            break;
        }

        for ch in plan.chars() {
            if ops.len() >= MAX_TURNS {
                break;
            }
            let dir = match ch {
                'U' => 0,
                'D' => 1,
                'L' => 2,
                'R' => 3,
                _ => continue,
            };
            if !is_legal_dir(&state, dir) {
                return ops;
            }
            let (ns, _) = step_info(&state, dir);
            state = ns;
            ops.push(ch);
        }

        if state.colors.len() < ell + 1 || state.colors[..=ell] != input.d[..=ell] {
            break;
        }
    }

    ops
}

fn main() {
    let input = read_input();
    let ans = solve(&input);

    let mut out = String::new();
    for ch in ans {
        out.push(ch);
        out.push('\n');
    }
    print!("{out}");
}
