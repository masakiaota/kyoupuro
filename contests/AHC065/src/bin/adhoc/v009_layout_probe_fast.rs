// v009_layout_probe_fast.rs
#![allow(dead_code)]

use std::cmp::Ordering;
use std::env;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::io::{self, Read};
use std::time::Instant;

const N: usize = 20;
const NN: usize = N * N;
const EXIT_COL: usize = N / 2;
const E: (usize, usize) = (0, EXIT_COL);
const EXIT_P: usize = E.0 * N + E.1;
const EMPTY: usize = NN;
const EMPTY_BOX: i16 = -1;
const MAX_M: usize = NN;
const MAX_T: usize = 100_000;

const V_BLOCK: usize = 4;
const H_BLOCK: usize = 4;
const BEAM_WIDTH: usize = 80;
const K_LOOK: usize = 60;
const EXTRA_DEPTH: usize = 6;
const DMAX: usize = 80;
const LEN_PENALTY: f32 = 1.2;
const BEAM_TIME_LIMIT: f64 = 1.82;
const LAYOUT_SPEC_ENV: &str = "AHC065_LAYOUT_SPEC";

#[inline(always)]
fn to_p(i: usize, j: usize) -> usize {
    i * N + j
}

#[inline(always)]
fn to_ij(p: usize) -> (usize, usize) {
    (p / N, p % N)
}

#[inline(always)]
fn dist_exit(p: usize) -> usize {
    let (i, j) = to_ij(p);
    i + j.abs_diff(EXIT_COL)
}

#[derive(Debug, Clone)]
struct Input {
    a: [usize; NN],
    pos_of_box: [usize; NN],
}

impl Input {
    fn read() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n_in: usize = it.next().unwrap().parse().unwrap();
        debug_assert_eq!(n_in, N);

        let mut a = [0usize; NN];
        let mut pos_of_box = [0usize; NN];

        for p in 0..NN {
            let k: usize = it.next().unwrap().parse().unwrap();
            a[p] = k;
            pos_of_box[k] = p;
        }

        Self { a, pos_of_box }
    }

    #[inline(always)]
    fn at(&self, i: usize, j: usize) -> usize {
        self.a[i * N + j]
    }

    #[inline(always)]
    fn pos_of(&self, k: usize) -> (usize, usize) {
        to_ij(self.pos_of_box[k])
    }
}

fn read_input() -> Input {
    Input::read()
}

#[derive(Debug, Clone, Copy)]
struct Operation {
    m: usize,
    d: i8,
}

#[derive(Debug, Clone)]
struct Conveyor {
    len: usize,
    cells: [usize; NN],
}

impl Conveyor {
    fn new() -> Self {
        Self {
            len: 0,
            cells: [0; NN],
        }
    }

    fn from_slice(cells: &[usize]) -> Self {
        debug_assert!(2 <= cells.len() && cells.len() <= NN);

        let mut conveyor = Self::new();
        conveyor.len = cells.len();
        conveyor.cells[..cells.len()].copy_from_slice(cells);
        conveyor
    }

    #[inline(always)]
    fn as_slice(&self) -> &[usize] {
        &self.cells[..self.len]
    }
}

#[derive(Debug, Clone)]
struct Solution {
    conveyors: Vec<Conveyor>,
    ops: Vec<Operation>,
}

type Output = Solution;

impl Solution {
    fn new() -> Self {
        Self {
            conveyors: Vec::with_capacity(MAX_M),
            ops: Vec::with_capacity(MAX_T),
        }
    }

    fn add_conveyor(&mut self, cells: &[usize]) -> usize {
        debug_assert!(self.conveyors.len() < MAX_M);

        let m = self.conveyors.len();
        self.conveyors.push(Conveyor::from_slice(cells));
        m
    }

    #[inline(always)]
    fn add_op(&mut self, m: usize, d: i8) {
        debug_assert!(self.ops.len() < MAX_T);
        debug_assert!(m < self.conveyors.len());
        debug_assert!(d == -1 || d == 1);

        self.ops.push(Operation { m, d });
    }

    fn print(&self) {
        let mut out = String::new();

        writeln!(&mut out, "{}", self.conveyors.len()).unwrap();
        for conveyor in &self.conveyors {
            write!(&mut out, "{}", conveyor.len).unwrap();
            for &p in conveyor.as_slice() {
                write!(&mut out, " {} {}", p / N, p % N).unwrap();
            }
            out.push('\n');
        }

        writeln!(&mut out, "{}", self.ops.len()).unwrap();
        for op in &self.ops {
            writeln!(&mut out, "{} {}", op.m, op.d).unwrap();
        }

        io::stdout().write_all(out.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone)]
struct TimeKeeper {
    start: Instant,
}

impl TimeKeeper {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

#[derive(Debug, Clone, Copy)]
struct BeamState {
    eval: f32,
    target: u16,
    fp: [u16; K_LOOK],
    seq: [u8; DMAX],
    len: u8,
    last_op: i16,
}

impl BeamState {
    fn new() -> Self {
        Self {
            eval: 0.0,
            target: 0,
            fp: [0; K_LOOK],
            seq: [0; DMAX],
            len: 0,
            last_op: -1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitMode {
    AroundExit,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerOrder {
    HorizontalFirst,
    VerticalFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutFamily {
    Blocks,
    UniformComb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerticalCombSide {
    Left,
    Right,
    Alternate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalCombSide {
    Top,
    Bottom,
    Alternate,
}

#[derive(Debug, Clone, Copy)]
struct LayoutSpec {
    family: LayoutFamily,
    h_block: usize,
    v_block: usize,
    row_offset: usize,
    col_offset: usize,
    v_split: SplitMode,
    order: LayerOrder,
    reverse_rows: bool,
    reverse_cols: bool,
    vertical_comb_count: usize,
    horizontal_comb_count: usize,
    vertical_comb_side: VerticalCombSide,
    horizontal_comb_side: HorizontalCombSide,
}

impl LayoutSpec {
    fn default() -> Self {
        Self {
            family: LayoutFamily::Blocks,
            h_block: H_BLOCK,
            v_block: V_BLOCK,
            row_offset: 0,
            col_offset: 0,
            v_split: SplitMode::AroundExit,
            order: LayerOrder::HorizontalFirst,
            reverse_rows: false,
            reverse_cols: false,
            vertical_comb_count: 4,
            horizontal_comb_count: 4,
            vertical_comb_side: VerticalCombSide::Left,
            horizontal_comb_side: HorizontalCombSide::Top,
        }
    }

    fn from_env() -> Self {
        match env::var(LAYOUT_SPEC_ENV) {
            Ok(raw) if !raw.trim().is_empty() => Self::parse(&raw),
            _ => Self::default(),
        }
    }

    fn parse(raw: &str) -> Self {
        let mut spec = Self::default();
        for part in raw.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (key, value) = part
                .split_once('=')
                .unwrap_or_else(|| panic!("invalid layout token: {part}"));
            let key = key.trim();
            let value = value.trim();
            match key {
                "family" => {
                    spec.family = match value {
                        "blocks" | "block" => LayoutFamily::Blocks,
                        "comb" | "uniform_comb" => LayoutFamily::UniformComb,
                        _ => panic!("invalid {key}: {value}"),
                    };
                }
                "h" | "hb" | "h_block" => spec.h_block = parse_even_size(key, value),
                "v" | "vb" | "v_block" => spec.v_block = parse_even_size(key, value),
                "ro" | "row_offset" => spec.row_offset = parse_even_offset(key, value),
                "co" | "col_offset" => spec.col_offset = parse_even_offset(key, value),
                "vs" | "vsplit" => {
                    spec.v_split = match value {
                        "exit" | "around_exit" => SplitMode::AroundExit,
                        "grid" | "left" => SplitMode::Grid,
                        _ => panic!("invalid {key}: {value}"),
                    };
                }
                "order" => {
                    spec.order = match value {
                        "hv" | "horizontal_first" => LayerOrder::HorizontalFirst,
                        "vh" | "vertical_first" => LayerOrder::VerticalFirst,
                        _ => panic!("invalid {key}: {value}"),
                    };
                }
                "hr" | "reverse_rows" => spec.reverse_rows = parse_bool(key, value),
                "vr" | "reverse_cols" => spec.reverse_cols = parse_bool(key, value),
                "vc" | "vertical_comb_count" => {
                    spec.vertical_comb_count = parse_comb_count(key, value)
                }
                "hc" | "horizontal_comb_count" => {
                    spec.horizontal_comb_count = parse_comb_count(key, value)
                }
                "vside" | "vertical_comb_side" => {
                    spec.vertical_comb_side = match value {
                        "left" | "l" => VerticalCombSide::Left,
                        "right" | "r" => VerticalCombSide::Right,
                        "alternate" | "alt" | "lr" => VerticalCombSide::Alternate,
                        _ => panic!("invalid {key}: {value}"),
                    }
                }
                "hside" | "horizontal_comb_side" => {
                    spec.horizontal_comb_side = match value {
                        "top" | "t" => HorizontalCombSide::Top,
                        "bottom" | "b" => HorizontalCombSide::Bottom,
                        "alternate" | "alt" | "tb" => HorizontalCombSide::Alternate,
                        _ => panic!("invalid {key}: {value}"),
                    }
                }
                "val" | "vertical_arm_len" | "hal" | "horizontal_arm_len" => {
                    let arm_len: usize = value
                        .parse()
                        .unwrap_or_else(|_| panic!("invalid {key}: {value}"));
                    if arm_len != N - 2 {
                        panic!("uniform comb probe currently requires {key}={}", N - 2);
                    }
                }
                "hf" | "h_flips" | "vf" | "v_flips" | "flips" | "seed" => {
                    panic!("unsupported layout key for block-only search: {key}");
                }
                _ => panic!("unknown layout key: {key}"),
            }
        }
        spec
    }
}

fn parse_even_size(key: &str, value: &str) -> usize {
    let x: usize = value
        .parse()
        .unwrap_or_else(|_| panic!("invalid {key}: {value}"));
    if !(2..=N).contains(&x) || (x & 1) == 1 {
        panic!("{key} must be an even integer in 2..={N}: {value}");
    }
    x
}

fn parse_even_offset(key: &str, value: &str) -> usize {
    let x: usize = value
        .parse()
        .unwrap_or_else(|_| panic!("invalid {key}: {value}"));
    if x > N || (x & 1) == 1 || x == 1 {
        panic!("{key} must be 0 or an even integer in 2..={N}: {value}");
    }
    x
}

fn parse_bool(key: &str, value: &str) -> bool {
    match value {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => panic!("invalid boolean {key}: {value}"),
    }
}

fn parse_comb_count(key: &str, value: &str) -> usize {
    let x: usize = value
        .parse()
        .unwrap_or_else(|_| panic!("invalid {key}: {value}"));
    if x == 0 || x > N / 2 {
        panic!("{key} must be in 1..={}: {value}", N / 2);
    }
    x
}

struct Solver {
    solution: Solution,
    op_list: Vec<Operation>,
    op_next: Vec<[u16; NN]>,
    downhill_ops: Vec<Vec<u8>>,
    weight: [f32; K_LOOK],
    cell: [i16; NN],
    pos: [i16; NN],
    delivered: usize,
    timer: TimeKeeper,
}

impl Solver {
    fn new() -> Self {
        let mut weight = [0.0f32; K_LOOK];
        for (i, w) in weight.iter_mut().enumerate() {
            *w = 1.0 / ((i + 1) as f32).sqrt();
        }

        Self {
            solution: Solution::new(),
            op_list: Vec::new(),
            op_next: Vec::new(),
            downhill_ops: vec![Vec::new(); NN],
            weight,
            cell: [EMPTY_BOX; NN],
            pos: [EMPTY_BOX; NN],
            delivered: 0,
            timer: TimeKeeper::new(),
        }
    }

    fn rect_cycle(&self, r0: usize, h: usize, c0: usize, w: usize) -> Vec<usize> {
        let mut c = Vec::with_capacity(h * w);

        for jj in 0..w {
            c.push(to_p(r0, c0 + jj));
        }

        let mut first = true;
        let mut high = w - 1;
        while high >= 1 {
            let start = if first { 1 } else { 2 };
            first = false;
            let ch = c0 + high;

            for ii in start..h {
                c.push(to_p(r0 + ii, ch));
            }
            c.push(to_p(r0 + h - 1, ch - 1));
            for ii in (1..=(h - 2)).rev() {
                c.push(to_p(r0 + ii, ch - 1));
            }
            if high - 1 > 0 {
                c.push(to_p(r0 + 1, ch - 2));
            }

            if high < 2 {
                break;
            }
            high -= 2;
        }

        c
    }

    fn horizontal_block_cycle(&self, r0: usize, h: usize) -> Vec<usize> {
        let t = self.rect_cycle(0, N, 0, h);
        let mut c = Vec::with_capacity(N * h);
        for p in t {
            let tr = p / N;
            let tc = p % N;
            c.push(to_p(r0 + tc, tr));
        }
        c
    }

    fn vertical_comb_cycle_rect(&self, r0: usize, h: usize, c0: usize, w: usize) -> Vec<usize> {
        debug_assert!(h >= 2 && h % 2 == 0);
        debug_assert!(w >= 2);
        debug_assert!(r0 + h <= N && c0 + w <= N);

        let left = c0;
        let spine = c0 + 1;
        let arm_len = w - 2;
        let mut cells = Vec::with_capacity(h * w);

        for dr in 0..h {
            cells.push(to_p(r0 + dr, left));
        }
        cells.push(to_p(r0 + h - 1, spine));

        for upper_local in (0..h).step_by(2).rev() {
            let upper = r0 + upper_local;
            let lower = upper + 1;
            let lower_spine = to_p(lower, spine);
            if cells.last().copied() != Some(lower_spine) {
                cells.push(lower_spine);
            }

            if arm_len > 0 {
                for dc in 1..=arm_len {
                    cells.push(to_p(lower, spine + dc));
                }
                cells.push(to_p(upper, spine + arm_len));
                for dc in (1..arm_len).rev() {
                    cells.push(to_p(upper, spine + dc));
                }
            }
            cells.push(to_p(upper, spine));
        }

        debug_assert_eq!(cells.len(), h * w);
        cells
    }

    fn reflect_cols(&self, cells: &mut [usize], c0: usize, w: usize) {
        for p in cells {
            let i = *p / N;
            let j = *p % N;
            *p = to_p(i, c0 + (w - 1 - (j - c0)));
        }
    }

    fn reflect_rows(&self, cells: &mut [usize], r0: usize, h: usize) {
        for p in cells {
            let i = *p / N;
            let j = *p % N;
            *p = to_p(r0 + (h - 1 - (i - r0)), j);
        }
    }

    fn vertical_comb_cycle_rect_with_side(
        &self,
        r0: usize,
        h: usize,
        c0: usize,
        w: usize,
        side: VerticalCombSide,
        idx: usize,
    ) -> Vec<usize> {
        let mut cells = self.vertical_comb_cycle_rect(r0, h, c0, w);
        let use_right = match side {
            VerticalCombSide::Left => false,
            VerticalCombSide::Right => true,
            VerticalCombSide::Alternate => idx % 2 == 1,
        };
        if use_right {
            self.reflect_cols(&mut cells, c0, w);
        }
        cells
    }

    fn horizontal_comb_cycle_rect(&self, r0: usize, h: usize, c0: usize, w: usize) -> Vec<usize> {
        debug_assert!(h >= 2 && w >= 2 && w % 2 == 0);
        let transposed = self.vertical_comb_cycle_rect(c0, w, r0, h);
        let mut cells = Vec::with_capacity(h * w);
        for p in transposed {
            let tr = p / N;
            let tc = p % N;
            cells.push(to_p(tc, tr));
        }
        cells
    }

    fn horizontal_comb_cycle_rect_with_side(
        &self,
        r0: usize,
        h: usize,
        c0: usize,
        w: usize,
        side: HorizontalCombSide,
        idx: usize,
    ) -> Vec<usize> {
        let mut cells = self.horizontal_comb_cycle_rect(r0, h, c0, w);
        let use_bottom = match side {
            HorizontalCombSide::Top => false,
            HorizontalCombSide::Bottom => true,
            HorizontalCombSide::Alternate => idx % 2 == 1,
        };
        if use_bottom {
            self.reflect_rows(&mut cells, r0, h);
        }
        cells
    }

    fn split_even_bands(&self, count: usize) -> Vec<(usize, usize)> {
        debug_assert!(count > 0 && count <= N / 2);
        let total_pairs = N / 2;
        let base = total_pairs / count;
        let rem = total_pairs % count;
        let mut start = 0usize;
        let mut bands = Vec::with_capacity(count);

        for idx in 0..count {
            let pairs = base + usize::from(idx < rem);
            debug_assert!(pairs > 0);
            let len = pairs * 2;
            bands.push((start, len));
            start += len;
        }
        debug_assert_eq!(start, N);
        bands
    }

    fn split_line(&self, span: usize, block: usize, first_width: usize) -> Vec<usize> {
        let mut res = Vec::new();
        let mut x = 0usize;
        let mut first = true;
        while x < span {
            let mut w = if first && first_width > 0 {
                first_width.min(span - x)
            } else {
                block.min(span - x)
            };
            first = false;
            if w & 1 == 1 {
                w -= 1;
            }
            if w == 0 {
                w = 2;
            }
            debug_assert!(x + w <= span);
            res.push(w);
            x += w;
        }
        res
    }

    fn split_rows(&self, h_block: usize, row_offset: usize) -> Vec<(usize, usize)> {
        let mut r = 0usize;
        let mut res = Vec::new();
        for h in self.split_line(N, h_block, row_offset) {
            res.push((r, h));
            r += h;
        }
        res
    }

    fn split_cols_grid(&self, w_block: usize, col_offset: usize) -> Vec<(usize, usize)> {
        let mut c = 0usize;
        let mut res = Vec::new();
        for w in self.split_line(N, w_block, col_offset) {
            res.push((c, w));
            c += w;
        }
        res
    }

    fn split_cols_around_exit(&self, w_block: usize, col_offset: usize) -> Vec<(usize, usize)> {
        let mut res = Vec::new();
        let mut x = EXIT_COL;
        let mut left = Vec::new();
        for w in self.split_line(EXIT_COL, w_block, col_offset) {
            left.push((x - w, w));
            x -= w;
        }
        left.reverse();
        res.extend(left);

        x = EXIT_COL;
        for w in self.split_line(N - EXIT_COL, w_block, col_offset) {
            res.push((x, w));
            x += w;
        }

        res
    }

    fn horizontal_cycles(&self, spec: LayoutSpec) -> Vec<Vec<usize>> {
        let mut rows = self.split_rows(spec.h_block, spec.row_offset);
        if spec.reverse_rows {
            rows.reverse();
        }
        let mut cycles = Vec::with_capacity(rows.len());
        for (r, h) in rows {
            cycles.push(self.horizontal_block_cycle(r, h));
        }
        cycles
    }

    fn vertical_cycles(&self, spec: LayoutSpec) -> Vec<Vec<usize>> {
        let mut cols = match spec.v_split {
            SplitMode::AroundExit => self.split_cols_around_exit(spec.v_block, spec.col_offset),
            SplitMode::Grid => self.split_cols_grid(spec.v_block, spec.col_offset),
        };
        if spec.reverse_cols {
            cols.reverse();
        }
        let mut cycles = Vec::with_capacity(cols.len());
        for (c, w) in cols {
            cycles.push(self.rect_cycle(0, N, c, w));
        }
        cycles
    }

    fn add_cycles(&mut self, cycles: Vec<Vec<usize>>) {
        for cells in cycles {
            self.solution.add_conveyor(&cells);
        }
    }

    fn build_uniform_comb_loops(&mut self, spec: LayoutSpec) {
        for (idx, (r, h)) in self
            .split_even_bands(spec.vertical_comb_count)
            .into_iter()
            .enumerate()
        {
            let cells =
                self.vertical_comb_cycle_rect_with_side(r, h, 0, N, spec.vertical_comb_side, idx);
            self.solution.add_conveyor(&cells);
        }
        for (idx, (c, w)) in self
            .split_even_bands(spec.horizontal_comb_count)
            .into_iter()
            .enumerate()
        {
            let cells = self.horizontal_comb_cycle_rect_with_side(
                0,
                N,
                c,
                w,
                spec.horizontal_comb_side,
                idx,
            );
            self.solution.add_conveyor(&cells);
        }
    }

    fn validate_conveyors(&self) {
        let mut cover = [0u8; NN];
        let mut seen = [false; NN];
        let mut missing_downhill = 0usize;

        for conveyor in &self.solution.conveyors {
            let cells = conveyor.as_slice();
            assert!(cells.len() >= 2);
            seen.fill(false);
            for x in 0..cells.len() {
                let p = cells[x];
                let q = cells[(x + 1) % cells.len()];
                assert!(p < NN);
                assert!(!seen[p]);
                seen[p] = true;
                assert!(to_ij(p).0.abs_diff(to_ij(q).0) + to_ij(p).1.abs_diff(to_ij(q).1) == 1);
                cover[p] += 1;
                assert!(cover[p] <= 2);
            }
        }

        for p in 0..NN {
            if p == EXIT_P {
                continue;
            }
            let mut ok = false;
            for conveyor in &self.solution.conveyors {
                let cells = conveyor.as_slice();
                for x in 0..cells.len() {
                    if cells[x] != p {
                        continue;
                    }
                    let prev = cells[(x + cells.len() - 1) % cells.len()];
                    let next = cells[(x + 1) % cells.len()];
                    if dist_exit(prev) + 1 == dist_exit(p) || dist_exit(next) + 1 == dist_exit(p) {
                        ok = true;
                        break;
                    }
                }
                if ok {
                    break;
                }
            }
            if !ok {
                missing_downhill += 1;
            }
        }
        if missing_downhill > 0 {
            eprintln!("layout warning: missing_downhill={missing_downhill}");
        }
    }

    fn build_loops(&mut self) {
        self.solution.conveyors.clear();
        let spec = LayoutSpec::from_env();

        if spec.family == LayoutFamily::UniformComb {
            self.build_uniform_comb_loops(spec);
            self.validate_conveyors();
            return;
        }

        match spec.order {
            LayerOrder::HorizontalFirst => {
                self.add_cycles(self.horizontal_cycles(spec));
                self.add_cycles(self.vertical_cycles(spec));
            }
            LayerOrder::VerticalFirst => {
                self.add_cycles(self.vertical_cycles(spec));
                self.add_cycles(self.horizontal_cycles(spec));
            }
        }
        self.validate_conveyors();
    }

    fn precompute_moves(&mut self) {
        self.op_list.clear();
        self.op_next.clear();
        self.downhill_ops = vec![Vec::new(); NN];

        for (m, conveyor) in self.solution.conveyors.iter().enumerate() {
            let cells = conveyor.as_slice();
            let len = cells.len();
            for d in [-1i8, 1i8] {
                let mut next = [0u16; NN];
                for (p, q) in next.iter_mut().enumerate() {
                    *q = p as u16;
                }
                for x in 0..len {
                    let to = if d == 1 {
                        cells[(x + 1) % len]
                    } else {
                        cells[(x + len - 1) % len]
                    };
                    next[cells[x]] = to as u16;
                }

                self.op_list.push(Operation { m, d });
                self.op_next.push(next);
            }
        }

        for (oi, next) in self.op_next.iter().enumerate() {
            for p in 0..NN {
                let q = next[p] as usize;
                if dist_exit(q) + 1 == dist_exit(p) {
                    self.downhill_ops[p].push(oi as u8);
                }
            }
        }
    }

    fn init_board(&mut self, input: &Input) {
        self.cell = [EMPTY_BOX; NN];
        self.pos = [EMPTY_BOX; NN];

        for p in 0..NN {
            let k = input.a[p];
            self.cell[p] = k as i16;
            self.pos[k] = p as i16;
        }

        self.delivered = 0;
        if self.cell[EXIT_P] == 0 {
            self.cell[EXIT_P] = EMPTY_BOX;
            self.pos[0] = EMPTY_BOX;
            self.delivered = 1;
        }
        self.solution.ops.clear();
    }

    fn apply_op(&mut self, op: Operation) {
        let cells = self.solution.conveyors[op.m].as_slice();
        let len = cells.len();

        if op.d == 1 {
            let last = self.cell[cells[len - 1]];
            for x in (1..len).rev() {
                let val = self.cell[cells[x - 1]];
                self.cell[cells[x]] = val;
                if val >= 0 {
                    self.pos[val as usize] = cells[x] as i16;
                }
            }
            self.cell[cells[0]] = last;
            if last >= 0 {
                self.pos[last as usize] = cells[0] as i16;
            }
        } else {
            let first = self.cell[cells[0]];
            for x in 0..(len - 1) {
                let val = self.cell[cells[x + 1]];
                self.cell[cells[x]] = val;
                if val >= 0 {
                    self.pos[val as usize] = cells[x] as i16;
                }
            }
            self.cell[cells[len - 1]] = first;
            if first >= 0 {
                self.pos[first as usize] = cells[len - 1] as i16;
            }
        }

        self.solution.add_op(op.m, op.d);
        if self.delivered < NN && self.cell[EXIT_P] == self.delivered as i16 {
            self.cell[EXIT_P] = EMPTY_BOX;
            self.pos[self.delivered] = EMPTY_BOX;
            self.delivered += 1;
        }
    }

    #[inline(always)]
    fn future_score(&self, st: &BeamState, flen: usize) -> f32 {
        let mut s = 0.0f32;
        for i in 0..flen {
            s += self.weight[i] * dist_exit(st.fp[i] as usize) as f32;
        }
        s
    }

    fn greedy_path(&self, start: usize) -> Vec<Operation> {
        let mut seq = Vec::with_capacity(64);
        let mut p = start;

        for _ in 0..80 {
            if p == EXIT_P || self.downhill_ops[p].is_empty() {
                break;
            }

            let mut best_oi = self.downhill_ops[p][0] as usize;
            let mut best_val = i32::MAX;
            for &oi8 in &self.downhill_ops[p] {
                let oi = oi8 as usize;
                let mut val = 0i32;
                let last = NN.min(self.delivered + 21);
                for k in (self.delivered + 1)..last {
                    let pk = self.pos[k];
                    if pk >= 0 {
                        val += dist_exit(self.op_next[oi][pk as usize] as usize) as i32;
                    }
                }
                if val < best_val {
                    best_val = val;
                    best_oi = oi;
                }
            }

            seq.push(self.op_list[best_oi]);
            p = self.op_next[best_oi][p] as usize;
        }

        seq
    }

    fn beam_plan(&self, target_box: usize) -> Vec<Operation> {
        let start_i = self.pos[target_box];
        if start_i < 0 || start_i as usize == EXIT_P {
            return Vec::new();
        }

        let start = start_i as usize;
        let base = dist_exit(start);
        let max_depth = (DMAX - 1).min(base + EXTRA_DEPTH);
        let flen = K_LOOK.min(NN - (target_box + 1));

        let mut beam = Vec::with_capacity(BEAM_WIDTH + 8);
        let mut next_beam = Vec::with_capacity(BEAM_WIDTH * self.op_list.len() + 16);

        let mut init = BeamState::new();
        init.eval = 1000.0 * base as f32;
        init.target = start as u16;
        init.len = 0;
        init.last_op = -1;
        for i in 0..flen {
            let mut pp = self.pos[target_box + 1 + i];
            if pp < 0 {
                pp = EXIT_P as i16;
            }
            init.fp[i] = pp as u16;
            init.eval += self.weight[i] * dist_exit(pp as usize) as f32;
        }
        beam.push(init);

        let mut has_goal = false;
        let mut best_goal = BeamState::new();
        best_goal.eval = f32::INFINITY;

        for depth in 0..max_depth {
            if (depth & 3) == 0 && self.timer.elapsed_sec() > BEAM_TIME_LIMIT {
                break;
            }

            next_beam.clear();
            for st in &beam {
                for oi in 0..self.op_list.len() {
                    if st.last_op >= 0 {
                        let pr = self.op_list[st.last_op as usize];
                        let cu = self.op_list[oi];
                        if pr.m == cu.m && pr.d == -cu.d {
                            continue;
                        }
                    }

                    let nt = self.op_next[oi][st.target as usize] as usize;
                    let remaining = max_depth - (st.len as usize + 1);
                    if dist_exit(nt) > remaining {
                        continue;
                    }

                    let mut ns = *st;
                    ns.target = nt as u16;
                    ns.seq[st.len as usize] = oi as u8;
                    ns.len = st.len + 1;
                    ns.last_op = oi as i16;

                    let next = &self.op_next[oi];
                    for i in 0..flen {
                        ns.fp[i] = next[st.fp[i] as usize];
                    }

                    let fs = self.future_score(&ns, flen);
                    if nt == EXIT_P {
                        ns.eval = fs + LEN_PENALTY * ns.len as f32;
                        if !has_goal || ns.eval < best_goal.eval {
                            has_goal = true;
                            best_goal = ns;
                        }
                    } else {
                        ns.eval = 1000.0 * dist_exit(nt) as f32 + fs + LEN_PENALTY * ns.len as f32;
                        next_beam.push(ns);
                    }
                }
            }

            if next_beam.is_empty() {
                break;
            }
            if next_beam.len() > BEAM_WIDTH {
                next_beam.select_nth_unstable_by(BEAM_WIDTH, |a, b| {
                    a.eval.partial_cmp(&b.eval).unwrap_or(Ordering::Equal)
                });
                next_beam.truncate(BEAM_WIDTH);
            }
            std::mem::swap(&mut beam, &mut next_beam);
        }

        if !has_goal {
            return self.greedy_path(start);
        }

        let mut seq = Vec::with_capacity(best_goal.len as usize);
        for i in 0..best_goal.len as usize {
            seq.push(self.op_list[best_goal.seq[i] as usize]);
        }
        seq
    }

    fn solve(&mut self, input: &Input) {
        self.build_loops();
        self.precompute_moves();
        self.init_board(input);

        while self.delivered < NN && self.solution.ops.len() < MAX_T {
            let k = self.delivered;
            let start = self.pos[k];

            let mut seq = if self.timer.elapsed_sec() < BEAM_TIME_LIMIT {
                self.beam_plan(k)
            } else if start >= 0 {
                self.greedy_path(start as usize)
            } else {
                Vec::new()
            };

            if seq.is_empty() {
                if start >= 0 && start as usize == EXIT_P {
                    let oi = self
                        .downhill_ops
                        .get(EXIT_P)
                        .and_then(|ops| ops.first())
                        .map_or(0usize, |&oi| oi as usize);
                    seq.push(self.op_list[oi]);
                } else if start >= 0 {
                    seq = self.greedy_path(start as usize);
                }
            }

            let before = self.delivered;
            for op in seq {
                if self.solution.ops.len() >= MAX_T {
                    break;
                }
                self.apply_op(op);
                if self.delivered > before {
                    break;
                }
            }

            if self.delivered == before && self.solution.ops.len() < MAX_T {
                let p = self.pos[k];
                if p >= 0 {
                    for op in self.greedy_path(p as usize) {
                        if self.solution.ops.len() >= MAX_T {
                            break;
                        }
                        self.apply_op(op);
                        if self.delivered > before {
                            break;
                        }
                    }
                }
            }

            if self.delivered == before {
                break;
            }
        }
    }

    fn print(&self) {
        self.solution.print();
    }
}

fn main() {
    let input = read_input();
    let mut solver = Solver::new();
    solver.solve(&input);
    solver.print();
}
