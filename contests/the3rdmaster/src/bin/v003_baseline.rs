// v003_baseline.rs
use std::cmp::min;
use std::fmt::Write as _;
use std::io::{self, Read};

const N: usize = 32;
const M: usize = N * N;
const WORDS: usize = M / 64;

#[derive(Debug, Clone, Copy, Default)]
struct Bits {
    w: [u64; WORDS],
}

impl Bits {
    fn set(&mut self, p: usize) {
        self.w[p >> 6] |= 1_u64 << (p & 63);
    }

    fn test(&self, p: usize) -> bool {
        ((self.w[p >> 6] >> (p & 63)) & 1) != 0
    }

    fn count(&self) -> usize {
        self.w.iter().map(|x| x.count_ones() as usize).sum()
    }

    fn positions(&self) -> Vec<usize> {
        let mut res = Vec::with_capacity(self.count());
        for (block, &mut_bits) in self.w.iter().enumerate() {
            let mut bits = mut_bits;
            while bits != 0 {
                let tz = bits.trailing_zeros() as usize;
                res.push((block << 6) + tz);
                bits &= bits - 1;
            }
        }
        res
    }

    fn difference(&self, other: &Self) -> Self {
        let mut res = Self::default();
        for i in 0..WORDS {
            res.w[i] = self.w[i] & !other.w[i];
        }
        res
    }
}

#[derive(Debug, Clone)]
struct Transform {
    r: usize,
    di: isize,
    dj: isize,
    to: [i16; M],
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Paint {
        k: usize,
        i: usize,
        j: usize,
        color: u8,
    },
    Copy {
        k: usize,
        h: usize,
        r: usize,
        di: isize,
        dj: isize,
    },
}

impl Op {
    fn paint(k: usize, i: usize, j: usize, color: u8) -> Self {
        Self::Paint { k, i, j, color }
    }

    fn copy(k: usize, h: usize, r: usize, di: isize, dj: isize) -> Self {
        Self::Copy { k, h, r, di, dj }
    }

    fn write_to(self, out: &mut String) {
        match self {
            Self::Paint { k, i, j, color } => {
                let _ = writeln!(out, "0 {k} {i} {j} {color}");
            }
            Self::Copy { k, h, r, di, dj } => {
                let _ = writeln!(out, "1 {k} {h} {r} {di} {dj}");
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Plan {
    cost: usize,
    ops: Vec<Op>,
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    idx: usize,
    gain: usize,
    new_count: usize,
    src_count: usize,
}

struct Solver {
    color: [u8; M],
    transforms: Vec<Transform>,
    seen: [u32; M],
    seen_token: u32,
}

impl Solver {
    const SMALL_THRESHOLD: usize = 10;
    const MAX_DEPTH: usize = 10;
    const ROOT_WIDTH: usize = 8;
    const SECOND_WIDTH: usize = 3;
    const MIN_NEW_PIXELS: usize = 2;

    fn new() -> Self {
        Self {
            color: [0; M],
            transforms: Vec::new(),
            seen: [0; M],
            seen_token: 1,
        }
    }

    fn rotate_cw(i: usize, j: usize, r: usize) -> (usize, usize) {
        match r & 3 {
            0 => (i, j),
            1 => (j, N - 1 - i),
            2 => (N - 1 - i, N - 1 - j),
            _ => (N - 1 - j, i),
        }
    }

    fn build_transforms(&mut self) {
        let cap = 4 * (2 * N - 1) * (2 * N - 1) - 1;
        self.transforms = Vec::with_capacity(cap);

        for r in 0..4 {
            for di in -((N as isize) - 1)..=((N as isize) - 1) {
                for dj in -((N as isize) - 1)..=((N as isize) - 1) {
                    if r == 0 && di == 0 && dj == 0 {
                        continue;
                    }
                    let mut tr = Transform {
                        r,
                        di,
                        dj,
                        to: [-1; M],
                    };
                    for i in 0..N {
                        for j in 0..N {
                            let (ri, rj) = Self::rotate_cw(i, j, r);
                            let ni = ri as isize + di;
                            let nj = rj as isize + dj;
                            let p = i * N + j;
                            if (0..N as isize).contains(&ni) && (0..N as isize).contains(&nj) {
                                tr.to[p] = (ni as usize * N + nj as usize) as i16;
                            }
                        }
                    }
                    self.transforms.push(tr);
                }
            }
        }
    }

    fn paint_plan_from_positions(&self, pos: &[usize]) -> Plan {
        let mut ops = Vec::with_capacity(pos.len());
        for &p in pos {
            ops.push(Op::paint(0, p / N, p % N, self.color[p]));
        }
        Plan {
            cost: pos.len(),
            ops,
        }
    }

    fn branch_width(depth: usize) -> usize {
        match depth {
            0 => Self::ROOT_WIDTH,
            1 => Self::SECOND_WIDTH,
            _ => 1,
        }
    }

    fn enumerate_candidates(&mut self, p: &Bits, plist: &[usize]) -> Vec<Candidate> {
        let mut cand = Vec::with_capacity(256);
        let mut src = Vec::with_capacity(plist.len());

        for (idx, tr) in self.transforms.iter().enumerate() {
            self.seen_token = self.seen_token.wrapping_add(1);
            if self.seen_token == 0 {
                self.seen.fill(0);
                self.seen_token = 1;
            }
            let token = self.seen_token;

            src.clear();
            for &pix in plist {
                let q = tr.to[pix];
                if q >= 0 {
                    let q = q as usize;
                    if self.color[pix] == self.color[q] && p.test(q) {
                        src.push(pix);
                        self.seen[pix] = token;
                    }
                }
            }
            if src.is_empty() {
                continue;
            }

            let mut new_count = 0;
            for &pix in &src {
                let q = tr.to[pix] as usize;
                if self.seen[q] != token {
                    new_count += 1;
                }
            }

            if new_count >= Self::MIN_NEW_PIXELS {
                cand.push(Candidate {
                    idx,
                    gain: new_count - 1,
                    new_count,
                    src_count: src.len(),
                });
            }
        }

        cand.sort_unstable_by(|a, b| {
            b.gain
                .cmp(&a.gain)
                .then_with(|| a.src_count.cmp(&b.src_count))
                .then_with(|| a.idx.cmp(&b.idx))
        });
        if cand.len() > 32 {
            cand.truncate(32);
        }
        cand
    }

    fn make_source(&self, p: &Bits, plist: &[usize], t_idx: usize) -> (Bits, Vec<usize>) {
        let mut s = Bits::default();
        let mut src = Vec::with_capacity(plist.len());
        let tr = &self.transforms[t_idx];

        for &pix in plist {
            let q = tr.to[pix];
            if q >= 0 {
                let q = q as usize;
                if self.color[pix] == self.color[q] && p.test(q) {
                    s.set(pix);
                    src.push(pix);
                }
            }
        }

        (s, src)
    }

    fn append_paint_ops(&self, p: &Bits, ops: &mut Vec<Op>) {
        for pix in p.positions() {
            ops.push(Op::paint(0, pix / N, pix % N, self.color[pix]));
        }
    }

    fn solve_subset(&mut self, p: Bits, depth: usize) -> Plan {
        let plist = p.positions();
        let mut best = self.paint_plan_from_positions(&plist);
        let cnt = plist.len();

        if cnt == 0 || cnt <= Self::SMALL_THRESHOLD || depth >= Self::MAX_DEPTH {
            return best;
        }

        let cand = self.enumerate_candidates(&p, &plist);
        let width = min(cand.len(), Self::branch_width(depth));

        for cnd in cand.into_iter().take(width) {
            let optimistic = 1 + cnt - cnd.src_count - cnd.new_count;
            if optimistic >= best.cost {
                continue;
            }

            let (s, src) = self.make_source(&p, &plist, cnd.idx);
            let sub = self.solve_subset(s, depth + 1);

            let tr = &self.transforms[cnd.idx];
            let mut built = s;
            for pix in src {
                built.set(tr.to[pix] as usize);
            }
            let remain = p.difference(&built);
            let total_cost = sub.cost + 1 + remain.count();
            if total_cost >= best.cost {
                continue;
            }

            let mut ops = Vec::with_capacity(total_cost);
            ops.extend(sub.ops);
            ops.push(Op::copy(0, 0, tr.r, tr.di, tr.dj));
            self.append_paint_ops(&remain, &mut ops);

            if ops.len() < best.cost {
                best.cost = ops.len();
                best.ops = ops;
            }
        }

        best
    }

    fn read_input(&mut self) {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap();
        let mut it = input.split_ascii_whitespace();

        let n: usize = it.next().unwrap().parse().unwrap();
        let _k_layers: usize = it.next().unwrap().parse().unwrap();
        let _color_count: usize = it.next().unwrap().parse().unwrap();
        assert_eq!(n, N);

        for i in 0..N {
            for j in 0..N {
                let x: u8 = it.next().unwrap().parse().unwrap();
                self.color[i * N + j] = x;
            }
        }
    }

    fn solve(&mut self) -> Vec<Op> {
        self.build_transforms();

        let mut target = Bits::default();
        for p in 0..M {
            if self.color[p] != 0 {
                target.set(p);
            }
        }

        let mut ans = self.solve_subset(target, 0);
        if ans.ops.len() > M {
            ans = self.paint_plan_from_positions(&target.positions());
        }
        ans.ops
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.read_input();
    let ops = solver.solve();

    let mut out = String::new();
    for op in ops {
        op.write_to(&mut out);
    }
    print!("{out}");
}
