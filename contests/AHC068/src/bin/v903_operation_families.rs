// v903_operation_families.rs
#![allow(non_snake_case)]

use proconio::input;
use proconio::marker::Chars;
use smallvec::SmallVec;
use std::collections::VecDeque;

const N: usize = 20;
const N2: usize = N * N;
const DIJ: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];

fn main() {
    get_time();
    let input = read_input();
    // 各方向について、壁にぶつかるまでの距離を計算しておく。
    let mut range = vec![[0; 4]; N2];
    for i in 0..N {
        for j in 0..N {
            for dir in 0..4 {
                for k in 0.. {
                    let i2 = i + DIJ[dir].0 * k;
                    let j2 = j + DIJ[dir].1 * k;
                    let i3 = i2 + DIJ[dir].0;
                    let j3 = j2 + DIJ[dir].1;
                    if i3 >= N || j3 >= N || input.has_wall(i2, j2, i3, j3) {
                        range[i * N + j][dir] = k;
                        break;
                    }
                }
            }
        }
    }
    let mut source_of = vec![0; N2];
    for s in 0..N2 {
        source_of[input.a[s]] = s;
    }
    let order = get_order(&input);
    let mut out = vec![];
    let mut done = vec![false; N2];
    for p in order {
        // ng[t][p] := out の先頭 t 操作適用後にマス p に処理済みカードがあるかどうか。
        // FlexRect 内の全候補は処理済みカードへ同じ作用をするため、代表元でよい。
        let mut ng = vec![done.clone()];
        for &op in &out {
            let mut a2 = ng.last().unwrap().clone();
            apply(&mut a2, representative(op));
            ng.push(a2);
        }
        let mut dist = vec![u64::MAX; N2 * (out.len() + 1)];
        let s = source_of[p];
        let mut trace = Trace::new();
        let mut que = VecDeque::new();
        que.push_back((s, 0, 0, !0));
        dist[s] = 0;
        let hurry = get_time() > 1.9;
        let mut goal_id = !0;
        let mut goal_dist = u64::MAX;
        while let Some((q, t, d, id)) = que.pop_front() {
            if dist[t * N2 + q] != d {
                continue;
            }
            if goal_id != !0 && (d >> 32) > (goal_dist >> 32) {
                break;
            }
            if q == p && t == out.len() {
                if goal_dist.setmin(d) {
                    goal_id = id;
                }
                continue;
            }
            // 既存操作を実行して時刻を進める。
            // スライドと太さの各自由度を、q の行き先ごとに安全な直積部分へ絞って分岐する。
            if t < out.len() {
                let op = out[t];
                let old_score = op.freedom_score();
                let mut branches = existing_branches(op, q);
                // push_front した際に自由度損失が小さいものから処理される順序にする。
                branches.sort_by_key(|branch| branch.op.freedom_score());

                for branch in branches.iter() {
                    let new_score = branch.op.freedom_score();
                    let nd = d + (old_score - new_score) as u64;
                    let ni = (t + 1) * N2 + branch.to;
                    if dist[ni].setmin(nd) {
                        let id2 = trace.add(TraceStep::Existing(branch.decision), id);
                        que.push_front((branch.to, t + 1, nd, id2));
                    }
                }

                if hurry {
                    continue;
                }
            }

            if goal_id != !0 {
                continue;
            }

            // 現在位置から各方向に、壁または処理済みカードに
            // ぶつかるまで何マス進めるかを求める。
            let mut up = 0;
            while up + 1 <= range[q][0] && !ng[t][q - N * (up + 1)] {
                up += 1;
            }
            let mut down = 0;
            while down + 1 <= range[q][1] && !ng[t][q + N * (down + 1)] {
                down += 1;
            }
            let mut left = 0;
            while left + 1 <= range[q][2] && !ng[t][q - (left + 1)] {
                left += 1;
            }
            let mut right = 0;
            while right + 1 <= range[q][3] && !ng[t][q + (right + 1)] {
                right += 1;
            }

            // 縦操作を新たに挿入する。
            for k in 1..=(up + down + 1) / 2 {
                let sum = 2 * k - 1;

                // 上方向へ k マス。
                let u = k.max(sum.saturating_sub(down));
                if u <= up {
                    let to = q - N * k;
                    let mv = NewMove::new(0, k, q, to);
                    let nd = d + (1 << 32);
                    let ni = t * N2 + to;
                    if dist[ni].setmin(nd) {
                        let id2 = trace.add(TraceStep::New(mv), id);
                        que.push_back((to, t, nd, id2));
                    }
                }

                // 下方向へ k マス。
                let u = sum.saturating_sub(down);
                if u <= up && u < k {
                    let to = q + N * k;
                    let mv = NewMove::new(0, k, q, to);
                    let nd = d + (1 << 32);
                    let ni = t * N2 + to;
                    if dist[ni].setmin(nd) {
                        let id2 = trace.add(TraceStep::New(mv), id);
                        que.push_back((to, t, nd, id2));
                    }
                }
            }

            // 横操作を新たに挿入する。
            for k in 1..=(left + right + 1) / 2 {
                let sum = 2 * k - 1;

                // 左方向へ k マス。
                let l = k.max(sum.saturating_sub(right));
                if l <= left {
                    let to = q - k;
                    let mv = NewMove::new(1, k, q, to);
                    let nd = d + (1 << 32);
                    let ni = t * N2 + to;
                    if dist[ni].setmin(nd) {
                        let id2 = trace.add(TraceStep::New(mv), id);
                        que.push_back((to, t, nd, id2));
                    }
                }

                // 右方向へ k マス。
                let l = sum.saturating_sub(right);
                if l <= left && l < k {
                    let to = q + k;
                    let mv = NewMove::new(1, k, q, to);
                    let nd = d + (1 << 32);
                    let ni = t * N2 + to;
                    if dist[ni].setmin(nd) {
                        let id2 = trace.add(TraceStep::New(mv), id);
                        que.push_back((to, t, nd, id2));
                    }
                }
            }
        }

        let steps = trace.get(goal_id);
        let mut old_t = 0;
        let mut new_out = Vec::with_capacity(steps.len());

        for step in steps {
            match step {
                TraceStep::New(mv) => {
                    let op = build_flex_rect(mv, &ng[old_t], &input);
                    new_out.push(op);
                }
                TraceStep::Existing(decision) => {
                    let mut op = out[old_t];
                    apply_decision(&mut op, decision);
                    new_out.push(op);
                    old_t += 1;
                }
            }
        }
        out = new_out;
        done[s] = true;
    }

    for op in out {
        let rect = representative(op);
        println!(
            "{} {} {} {} {}",
            if rect.d == 0 { 'V' } else { 'H' },
            rect.i,
            rect.j,
            rect.h,
            rect.w
        );
    }
    eprintln!("Time = {:.3}", get_time());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    d: u8,
    i: usize,
    j: usize,
    h: usize,
    w: usize,
}

// d=V: slide は上端行、[min_l,min_r),[max_l,max_r) は列区間。
// d=H: slide は左端列、各区間は行区間。
// slide_lo..=slide_hi と、min を含み max に含まれる任意の直交区間との
// 直積に含まれるすべての長方形が合法で、確定済みカードへ同じ作用をする。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FlexRect {
    d: u8,
    k: u8,
    slide_lo: u8,
    slide_hi: u8,
    min_l: u8,
    min_r: u8,
    max_l: u8,
    max_r: u8,
}

impl FlexRect {
    #[inline]
    fn k(self) -> usize {
        self.k as usize
    }

    #[inline]
    fn freedom_score(self) -> u32 {
        let slides = (self.slide_hi - self.slide_lo + 1) as u32;
        let left_choices = (self.min_l - self.max_l + 1) as u32;
        let right_choices = (self.max_r - self.min_r + 1) as u32;
        slides * left_choices * right_choices - 1
    }
}

#[derive(Clone, Copy, Debug)]
struct NewMove {
    d: u8,
    k: u8,
    from: u16,
    to: u16,
}

impl NewMove {
    fn new(d: u8, k: usize, from: usize, to: usize) -> Self {
        Self {
            d,
            k: k as u8,
            from: from as u16,
            to: to as u16,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExistingDecision {
    Keep,
    MoveTo { pos: u16, to: u16 },
    StayPerp { pos: u16 },
    StaySlide { lo: u8, hi: u8 },
}

#[derive(Clone, Copy, Debug)]
enum TraceStep {
    Existing(ExistingDecision),
    New(NewMove),
}

#[derive(Clone, Copy, Debug)]
struct Branch {
    to: usize,
    decision: ExistingDecision,
    op: FlexRect,
}

type BranchList = SmallVec<[Branch; 5]>;

#[inline]
fn representative(op: FlexRect) -> Rect {
    make_rect(
        op,
        op.slide_lo as usize,
        op.max_l as usize,
        op.max_r as usize,
    )
}

#[inline]
fn make_rect(op: FlexRect, slide: usize, l: usize, r: usize) -> Rect {
    if op.d == 0 {
        Rect {
            d: 0,
            i: slide,
            j: l,
            h: 2 * op.k(),
            w: r - l,
        }
    } else {
        Rect {
            d: 1,
            i: l,
            j: slide,
            h: r - l,
            w: 2 * op.k(),
        }
    }
}

#[inline]
fn axis_perp(op: FlexRect, p: usize) -> (usize, usize) {
    let (x, y) = (p / N, p % N);
    if op.d == 0 { (x, y) } else { (y, x) }
}

fn intersect_signed(lo: usize, hi: usize, a: isize, b: isize) -> Option<(usize, usize)> {
    let l = (lo as isize).max(a).max(0);
    let r = (hi as isize).min(b).min((N - 1) as isize);
    if l <= r {
        Some((l as usize, r as usize))
    } else {
        None
    }
}

fn include_perp(op: &mut FlexRect, perp: usize) -> bool {
    if perp < op.max_l as usize || perp >= op.max_r as usize {
        return false;
    }
    op.min_l.setmin(perp as u8);
    op.min_r.setmax((perp + 1) as u8);
    true
}

fn exclude_perp(op: &mut FlexRect, perp: usize) -> bool {
    if perp < op.max_l as usize || perp >= op.max_r as usize {
        return true;
    }
    if perp < op.min_l as usize {
        op.max_l.setmax((perp + 1) as u8);
        true
    } else if perp >= op.min_r as usize {
        op.max_r.setmin(perp as u8);
        true
    } else {
        false
    }
}

fn restrict_move(mut op: FlexRect, pos: usize, to: usize) -> Option<FlexRect> {
    let (axis, perp) = axis_perp(op, pos);
    if !include_perp(&mut op, perp) {
        return None;
    }
    let k = op.k();
    let negative = if op.d == 0 {
        to + N * k == pos
    } else {
        to + k == pos
    };
    let positive = if op.d == 0 {
        pos + N * k == to
    } else {
        pos + k == to
    };
    if !negative && !positive {
        return None;
    }
    let (a, b) = if negative {
        (
            axis as isize - 2 * k as isize + 1,
            axis as isize - k as isize,
        )
    } else {
        (axis as isize - k as isize + 1, axis as isize)
    };
    let (lo, hi) = intersect_signed(op.slide_lo as usize, op.slide_hi as usize, a, b)?;
    op.slide_lo = lo as u8;
    op.slide_hi = hi as u8;
    Some(op)
}

fn apply_decision(op: &mut FlexRect, decision: ExistingDecision) {
    match decision {
        ExistingDecision::Keep => {}
        ExistingDecision::MoveTo { pos, to } => {
            *op = restrict_move(*op, pos as usize, to as usize).unwrap();
        }
        ExistingDecision::StayPerp { pos } => {
            let (_, perp) = axis_perp(*op, pos as usize);
            exclude_perp(op, perp);
        }
        ExistingDecision::StaySlide { lo, hi } => {
            op.slide_lo = lo;
            op.slide_hi = hi;
        }
    }
}

fn existing_branches(op: FlexRect, pos: usize) -> BranchList {
    let (axis, perp) = axis_perp(op, pos);
    let k = op.k();
    let mut out = SmallVec::<[Branch; 5]>::new();

    // 直交方向で最大範囲外なら、どのスライドでも必ず不動。
    if perp < op.max_l as usize || perp >= op.max_r as usize {
        out.push(Branch {
            to: pos,
            decision: ExistingDecision::Keep,
            op,
        });
        return out;
    }

    // 負方向（上・左）へ移動。
    if axis >= k {
        let neg_to = if op.d == 0 { pos - N * k } else { pos - k };
        if let Some(moved) = restrict_move(op, pos, neg_to) {
            let decision = if moved == op {
                ExistingDecision::Keep
            } else {
                ExistingDecision::MoveTo {
                    pos: pos as u16,
                    to: neg_to as u16,
                }
            };
            out.push(Branch {
                to: neg_to,
                decision,
                op: moved,
            });
        }
    }

    // 正方向（下・右）へ移動。
    if axis + k < N {
        let pos_to = if op.d == 0 { pos + N * k } else { pos + k };
        if let Some(moved) = restrict_move(op, pos, pos_to) {
            let decision = if moved == op {
                ExistingDecision::Keep
            } else {
                ExistingDecision::MoveTo {
                    pos: pos as u16,
                    to: pos_to as u16,
                }
            };
            out.push(Branch {
                to: pos_to,
                decision,
                op: moved,
            });
        }
    }

    // スライド範囲を、カードが軸区間の手前にある部分へ限定して不動にする。
    if let Some((lo, hi)) = intersect_signed(
        op.slide_lo as usize,
        op.slide_hi as usize,
        isize::MIN / 4,
        axis as isize - 2 * k as isize,
    ) {
        let mut stayed = op;
        stayed.slide_lo = lo as u8;
        stayed.slide_hi = hi as u8;
        let decision = if stayed == op {
            ExistingDecision::Keep
        } else {
            ExistingDecision::StaySlide {
                lo: lo as u8,
                hi: hi as u8,
            }
        };
        out.push(Branch {
            to: pos,
            decision,
            op: stayed,
        });
    }

    // スライド範囲を、カードが軸区間の後ろにある部分へ限定して不動にする。
    if let Some((lo, hi)) = intersect_signed(
        op.slide_lo as usize,
        op.slide_hi as usize,
        axis as isize + 1,
        isize::MAX / 4,
    ) {
        let mut stayed = op;
        stayed.slide_lo = lo as u8;
        stayed.slide_hi = hi as u8;
        let decision = if stayed == op {
            ExistingDecision::Keep
        } else {
            ExistingDecision::StaySlide {
                lo: lo as u8,
                hi: hi as u8,
            }
        };
        out.push(Branch {
            to: pos,
            decision,
            op: stayed,
        });
    }

    // 太さ方向でカードを除外して不動にする。
    let mut stayed = op;
    if exclude_perp(&mut stayed, perp) {
        let decision = if stayed == op {
            ExistingDecision::Keep
        } else {
            ExistingDecision::StayPerp { pos: pos as u16 }
        };
        out.push(Branch {
            to: pos,
            decision,
            op: stayed,
        });
    }
    out
}

fn desired_slide_bounds(mv: NewMove) -> (usize, usize) {
    let from = mv.from as usize;
    let to = mv.to as usize;
    let k = mv.k as usize;
    let axis = if mv.d == 0 { from / N } else { from % N };
    let negative = if mv.d == 0 {
        to + N * k == from
    } else {
        to + k == from
    };
    let (a, b) = if negative {
        (
            axis as isize - 2 * k as isize + 1,
            axis as isize - k as isize,
        )
    } else {
        (axis as isize - k as isize + 1, axis as isize)
    };
    let max_start = N - 2 * k;
    let lo = a.max(0) as usize;
    let hi = b.min(max_start as isize) as usize;
    (lo, hi)
}

fn rect_legal_blocked(rect: Rect, blocked: &[bool], input: &Input) -> bool {
    for i in rect.i..rect.i + rect.h {
        for j in rect.j..rect.j + rect.w {
            if blocked[i * N + j] {
                return false;
            }
        }
    }
    rect_legal(rect, input)
}

fn can_add_perp(
    d: u8,
    slide: usize,
    k: usize,
    coord: usize,
    adjacent: usize,
    blocked: &[bool],
    input: &Input,
) -> bool {
    if d == 0 {
        for i in slide..slide + 2 * k {
            if blocked[i * N + coord] || input.has_wall(i, coord, i, adjacent) {
                return false;
            }
            if i + 1 < slide + 2 * k && input.has_wall(i, coord, i + 1, coord) {
                return false;
            }
        }
    } else {
        for j in slide..slide + 2 * k {
            if blocked[coord * N + j] || input.has_wall(coord, j, adjacent, j) {
                return false;
            }
            if j + 1 < slide + 2 * k && input.has_wall(coord, j, coord, j + 1) {
                return false;
            }
        }
    }
    true
}

fn max_perp_bounds(
    d: u8,
    slide: usize,
    k: usize,
    core: usize,
    blocked: &[bool],
    input: &Input,
) -> Option<(usize, usize)> {
    let base = if d == 0 {
        Rect {
            d,
            i: slide,
            j: core,
            h: 2 * k,
            w: 1,
        }
    } else {
        Rect {
            d,
            i: core,
            j: slide,
            h: 1,
            w: 2 * k,
        }
    };
    if !rect_legal_blocked(base, blocked, input) {
        return None;
    }
    let mut l = core;
    while l > 0 && can_add_perp(d, slide, k, l - 1, l, blocked, input) {
        l -= 1;
    }
    let mut r = core + 1;
    while r < N && can_add_perp(d, slide, k, r, r - 1, blocked, input) {
        r += 1;
    }
    Some((l, r))
}

fn build_flex_rect(mv: NewMove, blocked: &[bool], input: &Input) -> FlexRect {
    let k = mv.k as usize;
    let from = mv.from as usize;
    let core = if mv.d == 0 { from % N } else { from / N };
    let (start_lo, start_hi) = desired_slide_bounds(mv);
    let mut options: Vec<Option<(usize, usize)>> = Vec::with_capacity(start_hi - start_lo + 1);
    for slide in start_lo..=start_hi {
        options.push(max_perp_bounds(mv.d, slide, k, core, blocked, input));
    }
    let mut best_key: Option<(u32, usize, usize)> = None;
    let mut best: Option<(usize, usize, usize, usize)> = None;
    for a in 0..options.len() {
        let Some((mut common_l, mut common_r)) = options[a] else {
            continue;
        };
        for b in a..options.len() {
            let Some((l, r)) = options[b] else { break };
            common_l.setmax(l);
            common_r.setmin(r);
            if !(common_l <= core && core < common_r) {
                break;
            }
            let op = FlexRect {
                d: mv.d,
                k: mv.k,
                slide_lo: (start_lo + a) as u8,
                slide_hi: (start_lo + b) as u8,
                min_l: core as u8,
                min_r: (core + 1) as u8,
                max_l: common_l as u8,
                max_r: common_r as u8,
            };
            let key = (op.freedom_score(), b - a + 1, common_r - common_l);
            if best_key.setmax(Some(key)) {
                best = Some((a, b, common_l, common_r));
            }
        }
    }

    let (a, b, common_l, common_r) = best.expect("new move must have a legal rectangle");
    FlexRect {
        d: mv.d,
        k: mv.k,
        slide_lo: (start_lo + a) as u8,
        slide_hi: (start_lo + b) as u8,
        min_l: core as u8,
        min_r: (core + 1) as u8,
        max_l: common_l as u8,
        max_r: common_r as u8,
    }
}

fn rect_legal(rect: Rect, input: &Input) -> bool {
    for i in rect.i..rect.i + rect.h {
        for j in rect.j..rect.j + rect.w.saturating_sub(1) {
            if input.has_wall(i, j, i, j + 1) {
                return false;
            }
        }
    }
    for i in rect.i..rect.i + rect.h.saturating_sub(1) {
        for j in rect.j..rect.j + rect.w {
            if input.has_wall(i, j, i + 1, j) {
                return false;
            }
        }
    }
    true
}

fn apply(a: &mut [bool], rect: Rect) {
    if rect.d == 0 {
        for x in 0..rect.h / 2 {
            for y in 0..rect.w {
                let p = (rect.i + x) * N + rect.j + y;
                let q = (rect.i + rect.h / 2 + x) * N + rect.j + y;
                a.swap(p, q);
            }
        }
    } else {
        for x in 0..rect.h {
            for y in 0..rect.w / 2 {
                let p = (rect.i + x) * N + rect.j + y;
                let q = (rect.i + x) * N + rect.j + rect.w / 2 + y;
                a.swap(p, q);
            }
        }
    }
}

// BFSで順番を決める。
fn get_order(input: &Input) -> Vec<usize> {
    let mut que = VecDeque::new();
    que.push_back((N / 2, N / 2));
    let mut visited = vec![false; N2];
    visited[N / 2 * N + N / 2] = true;
    let mut order = vec![];
    while let Some((i, j)) = que.pop_front() {
        order.push(i * N + j);
        for (di, dj) in DIJ {
            let i2 = i + di;
            let j2 = j + dj;
            if i2 < N && j2 < N && !input.has_wall(i, j, i2, j2) {
                let next = i2 * N + j2;
                if visited[next].setmax(true) {
                    que.push_back((i2, j2));
                }
            }
        }
    }
    order.reverse();
    order
}

struct Input {
    a: Vec<usize>,
    wall_v: Vec<bool>,
    wall_h: Vec<bool>,
}

impl Input {
    #[inline]
    fn has_wall(&self, i: usize, j: usize, i2: usize, j2: usize) -> bool {
        if i == i2 {
            self.wall_v[i * (N - 1) + j.min(j2)]
        } else {
            self.wall_h[i.min(i2) * N + j]
        }
    }
}

fn read_input() -> Input {
    input! {
        _N: usize,
        a: [[usize; N]; N],
        wall_v: [Chars; N],
        wall_h: [Chars; N - 1],
    }
    Input {
        a: a.into_iter().flatten().collect(),
        wall_v: wall_v
            .into_iter()
            .map(|s| s.into_iter().map(|c| c == '1'))
            .flatten()
            .collect(),
        wall_h: wall_h
            .into_iter()
            .map(|s| s.into_iter().map(|c| c == '1'))
            .flatten()
            .collect(),
    }
}

// ここからライブラリ

pub trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}

impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }

    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

pub fn get_time() -> f64 {
    static mut STIME: f64 = -1.0;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let ms = t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9;
    unsafe {
        if STIME < 0.0 {
            STIME = ms;
        }
        // ローカル環境とジャッジ環境の実行速度差はget_timeで吸収しておくと便利
        #[cfg(feature = "local")]
        {
            (ms - STIME) * 0.9
        }
        #[cfg(not(feature = "local"))]
        {
            ms - STIME
        }
    }
}

pub struct Trace<T: Copy> {
    log: Vec<(T, u32)>,
}

impl<T: Copy> Trace<T> {
    pub fn new() -> Self {
        Self { log: vec![] }
    }
    pub fn add(&mut self, c: T, p: u32) -> u32 {
        self.log.push((c, p));
        self.log.len() as u32 - 1
    }
    pub fn get(&self, mut i: u32) -> Vec<T> {
        let mut out = vec![];
        while i != !0 {
            out.push(self.log[i as usize].0);
            i = self.log[i as usize].1;
        }
        out.reverse();
        out
    }
}

