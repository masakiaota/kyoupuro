// offline_reference.rs
#![allow(non_snake_case)]

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const MAX_N: usize = 50;
const MAX_P: usize = 150;
type Rows = [u64; MAX_N];

#[derive(Clone)]
struct Group {
    id: usize,
    S: usize,
    T: usize,
    P: usize,
    V: i64,
    max_fee: i64,
}

#[derive(Clone)]
struct Case {
    name: String,
    N: usize,
    M: usize,
    R: f64,
    grass_rows: Rows,
    groups: Vec<Group>,
}

#[derive(Clone)]
struct Shape {
    h: usize,
    w: usize,
    perimeter: usize,
    left: Vec<usize>,
    len: Vec<usize>,
}

#[derive(Clone)]
struct Region {
    rows: Rows,
    cells: Vec<u16>,
    perimeter: usize,
}

#[derive(Clone)]
struct Plan {
    regions: Vec<Option<Region>>,
    usage: Vec<u16>,
    score: i64,
    accepted: usize,
}

#[derive(Clone)]
struct Episode {
    incoming_id: usize,
    target_region: Region,
    relocations: Vec<(usize, Region)>,
    restore_turn: Option<usize>,
    estimated_gain: i64,
}

#[derive(Clone)]
struct TargetOption {
    region: Region,
    blockers: Vec<usize>,
    rank: i64,
}

struct DynamicResult {
    output: String,
    score: i64,
    total_fee: i64,
    move_cost: i64,
    accepted: usize,
    accepted_ids: Vec<usize>,
    episodes: usize,
    moves: usize,
    candidates: usize,
}

#[derive(Clone, Copy)]
struct PlacementStyle {
    reuse_weight: i64,
    contact_weight: i64,
    x_bias: i64,
    y_bias: i64,
    noise: i64,
    growth_attempts: usize,
}

struct Config {
    input: PathBuf,
    seconds: f64,
    seed: u64,
    output: Option<PathBuf>,
}

fn minimum_perimeter(P: usize) -> usize {
    2 * (2.0 * (P as f64).sqrt() - 1.0e-12).ceil() as usize
}

fn compactness(P: usize, perimeter: usize) -> f64 {
    4.0 * (P as f64).sqrt() / (perimeter as f64)
}

fn fee(V: i64, P: usize, perimeter: usize) -> i64 {
    ((V as f64) * compactness(P, perimeter)).round() as i64
}

fn move_cost(case: &Case, group: &Group) -> i64 {
    (((group.V as f64) * case.R).round() as i64).max(1)
}

fn intervals_overlap(a: &Group, b: &Group) -> bool {
    a.S < b.T && b.S < a.T
}

fn parse_case(path: &Path) -> Result<Case, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{} を読めない: {error}", path.display()))?;
    let mut tokens = text.split_whitespace();
    let N: usize = tokens
        .next()
        .ok_or_else(|| "N がない".to_string())?
        .parse()
        .map_err(|error| format!("N が不正: {error}"))?;
    let M: usize = tokens
        .next()
        .ok_or_else(|| "M がない".to_string())?
        .parse()
        .map_err(|error| format!("M が不正: {error}"))?;
    let R: f64 = tokens
        .next()
        .ok_or_else(|| "R がない".to_string())?
        .parse()
        .map_err(|error| format!("R が不正: {error}"))?;
    if N > MAX_N {
        return Err(format!("N={N} は MAX_N={MAX_N} を超える"));
    }

    let mut grass_rows = [0_u64; MAX_N];
    for row_mask in grass_rows.iter_mut().take(N) {
        let row = tokens
            .next()
            .ok_or_else(|| "盤面行が不足している".to_string())?;
        if row.len() != N {
            return Err(format!("盤面行の長さが {} である", row.len()));
        }
        for (y, byte) in row.bytes().enumerate() {
            if byte == b'.' {
                *row_mask |= 1_u64 << y;
            }
        }
    }

    let mut groups = Vec::with_capacity(M);
    for expected_id in 0..M {
        let id: usize = tokens
            .next()
            .ok_or_else(|| "group id が不足している".to_string())?
            .parse()
            .map_err(|error| format!("group id が不正: {error}"))?;
        let S: usize = tokens
            .next()
            .ok_or_else(|| "S が不足している".to_string())?
            .parse()
            .map_err(|error| format!("S が不正: {error}"))?;
        let T: usize = tokens
            .next()
            .ok_or_else(|| "T が不足している".to_string())?
            .parse()
            .map_err(|error| format!("T が不正: {error}"))?;
        let P: usize = tokens
            .next()
            .ok_or_else(|| "P が不足している".to_string())?
            .parse()
            .map_err(|error| format!("P が不正: {error}"))?;
        let V: i64 = tokens
            .next()
            .ok_or_else(|| "V が不足している".to_string())?
            .parse()
            .map_err(|error| format!("V が不正: {error}"))?;
        if id != expected_id {
            return Err(format!("group id={id}, expected={expected_id}"));
        }
        groups.push(Group {
            id,
            S,
            T,
            P,
            V,
            max_fee: fee(V, P, minimum_perimeter(P)),
        });
    }

    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("case")
        .to_string();
    Ok(Case {
        name,
        N,
        M,
        R,
        grass_rows,
        groups,
    })
}

fn shape_perimeter(shape: &Shape, P: usize) -> usize {
    let mut adjacent = 0;
    for r in 0..shape.h {
        adjacent += shape.len[r] - 1;
        if r > 0 {
            let a0 = shape.left[r - 1];
            let a1 = a0 + shape.len[r - 1];
            let b0 = shape.left[r];
            let b1 = b0 + shape.len[r];
            adjacent += a1.min(b1).saturating_sub(a0.max(b0));
        }
    }
    4 * P - 2 * adjacent
}

fn shape_key(shape: &Shape) -> Vec<u8> {
    let mut key = Vec::with_capacity(2 + 2 * shape.h);
    key.push(shape.h as u8);
    key.push(shape.w as u8);
    for r in 0..shape.h {
        key.push(shape.left[r] as u8);
        key.push(shape.len[r] as u8);
    }
    key
}

fn shape_complexity(shape: &Shape) -> usize {
    let mut result = 0;
    for r in 1..shape.h {
        result += usize::from(shape.left[r] != shape.left[r - 1]);
        result += usize::from(shape.len[r] != shape.len[r - 1]);
    }
    result
}

fn try_add_shape(
    generated: &mut Vec<Shape>,
    seen: &mut HashSet<Vec<u8>>,
    mut shape: Shape,
    P: usize,
    min_perimeter: usize,
) {
    let mut area = 0;
    for r in 0..shape.h {
        area += shape.len[r];
        if shape.len[r] == 0 || shape.left[r] + shape.len[r] > shape.w {
            return;
        }
        if r > 0 {
            let lo = shape.left[r - 1].max(shape.left[r]);
            let hi = (shape.left[r - 1] + shape.len[r - 1]).min(shape.left[r] + shape.len[r]);
            if lo >= hi {
                return;
            }
        }
    }
    if area != P {
        return;
    }
    shape.perimeter = shape_perimeter(&shape, P);
    if shape.perimeter > min_perimeter + 8 {
        return;
    }
    if seen.insert(shape_key(&shape)) {
        generated.push(shape);
    }
}

fn generate_shapes(N: usize) -> Vec<Vec<Shape>> {
    let mut result = vec![Vec::new(); MAX_P + 1];
    for (P, target) in result.iter_mut().enumerate().take(MAX_P + 1).skip(4) {
        let min_perimeter = minimum_perimeter(P);
        let mut generated = Vec::new();
        let mut seen = HashSet::new();
        for h in 1..=N.min(P) {
            let w = P.div_ceil(h);
            if w > N {
                continue;
            }
            let missing = h * w - P;
            if missing == 0 {
                try_add_shape(
                    &mut generated,
                    &mut seen,
                    Shape {
                        h,
                        w,
                        perimeter: 0,
                        left: vec![0; h],
                        len: vec![w; h],
                    },
                    P,
                    min_perimeter,
                );
                continue;
            }
            if w <= 1 {
                continue;
            }

            let mut starts = vec![
                0_isize,
                h as isize - missing as isize,
                (h as isize - missing as isize) / 2,
                (h as isize - missing as isize + 1) / 2,
            ];
            starts.sort_unstable();
            starts.dedup();
            for start in starts {
                if start < 0 || start as usize + missing > h {
                    continue;
                }
                for remove_left in 0..=1 {
                    let mut shape = Shape {
                        h,
                        w,
                        perimeter: 0,
                        left: vec![0; h],
                        len: vec![w; h],
                    };
                    for r in start as usize..start as usize + missing {
                        shape.left[r] = remove_left;
                        shape.len[r] = w - 1;
                    }
                    try_add_shape(&mut generated, &mut seen, shape, P, min_perimeter);
                }
            }

            if missing >= 2 {
                let top = missing / 2;
                let bottom = missing - top;
                for top_side in 0..=1 {
                    for bottom_side in 0..=1 {
                        let mut shape = Shape {
                            h,
                            w,
                            perimeter: 0,
                            left: vec![0; h],
                            len: vec![w; h],
                        };
                        for r in 0..top {
                            shape.left[r] = top_side;
                            shape.len[r] = w - 1;
                        }
                        for r in h - bottom..h {
                            shape.left[r] = bottom_side;
                            shape.len[r] = w - 1;
                        }
                        try_add_shape(&mut generated, &mut seen, shape, P, min_perimeter);
                    }
                }
            }
        }

        generated.sort_by(|a, b| {
            a.perimeter
                .cmp(&b.perimeter)
                .then_with(|| a.h.abs_diff(a.w).cmp(&b.h.abs_diff(b.w)))
                .then_with(|| shape_complexity(a).cmp(&shape_complexity(b)))
                .then_with(|| a.h.cmp(&b.h))
                .then_with(|| a.left.cmp(&b.left))
                .then_with(|| a.len.cmp(&b.len))
        });

        let mut kept = Vec::new();
        let mut begin = 0;
        while begin < generated.len() {
            let mut end = begin + 1;
            while end < generated.len() && generated[end].perimeter == generated[begin].perimeter {
                end += 1;
            }
            const CAP_PER_LEVEL: usize = 20;
            let count = end - begin;
            if count <= CAP_PER_LEVEL {
                kept.extend_from_slice(&generated[begin..end]);
            } else {
                for k in 0..CAP_PER_LEVEL {
                    let index = begin + k * count / CAP_PER_LEVEL;
                    kept.push(generated[index].clone());
                }
            }
            begin = end;
        }
        *target = kept;
    }
    result
}

impl Plan {
    fn empty(case: &Case) -> Self {
        Self {
            regions: vec![None; case.M],
            usage: vec![0; case.N * case.N],
            score: 0,
            accepted: 0,
        }
    }

    fn insert(&mut self, group: &Group, region: Region) {
        debug_assert!(self.regions[group.id].is_none());
        self.score += fee(group.V, group.P, region.perimeter);
        self.accepted += 1;
        for &cell in &region.cells {
            self.usage[cell as usize] += 1;
        }
        self.regions[group.id] = Some(region);
    }

    fn remove(&mut self, group: &Group) {
        let Some(region) = self.regions[group.id].take() else {
            return;
        };
        self.score -= fee(group.V, group.P, region.perimeter);
        self.accepted -= 1;
        for &cell in &region.cells {
            let value = &mut self.usage[cell as usize];
            debug_assert!(*value > 0);
            *value -= 1;
        }
    }
}

fn build_overlaps(case: &Case) -> Vec<Vec<usize>> {
    let mut overlaps = vec![Vec::new(); case.M];
    for i in 0..case.M {
        for j in 0..case.M {
            if i != j && intervals_overlap(&case.groups[i], &case.groups[j]) {
                overlaps[i].push(j);
            }
        }
    }
    overlaps
}

fn blocked_rows(plan: &Plan, overlaps: &[Vec<usize>], group_id: usize) -> Rows {
    let mut blocked = [0_u64; MAX_N];
    for &other_id in &overlaps[group_id] {
        if let Some(region) = &plan.regions[other_id] {
            for (target, &source) in blocked.iter_mut().zip(&region.rows) {
                *target |= source;
            }
        }
    }
    blocked
}

fn build_runs(free_rows: &Rows, N: usize) -> [[u64; MAX_N + 1]; MAX_N] {
    let mut runs = [[0_u64; MAX_N + 1]; MAX_N];
    for r in 0..N {
        runs[r][1] = free_rows[r];
        for len in 2..=N {
            runs[r][len] = runs[r][len - 1] & (free_rows[r] >> (len - 1));
        }
    }
    runs
}

fn position_score<R: Rng>(
    case: &Case,
    plan: &Plan,
    free_rows: &Rows,
    shape: &Shape,
    x: usize,
    y: usize,
    style: PlacementStyle,
    rng: &mut R,
) -> i64 {
    let mut reuse = 0_i64;
    let mut contact = 0_i64;
    for rr in 0..shape.h {
        let row = x + rr;
        let begin = y + shape.left[rr];
        for col in begin..begin + shape.len[rr] {
            reuse += plan.usage[row * case.N + col] as i64;
            for (dx, dy) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
                let nx = row as i32 + dx;
                let ny = col as i32 + dy;
                if nx < 0
                    || ny < 0
                    || nx >= case.N as i32
                    || ny >= case.N as i32
                    || (free_rows[nx as usize] >> ny as usize) & 1 == 0
                {
                    contact += 1;
                }
            }
        }
    }
    reuse * style.reuse_weight
        + contact * style.contact_weight
        + (x as i64) * style.x_bias
        + (y as i64) * style.y_bias
        + rng.gen_range(0..=style.noise.max(0))
}

fn materialize_shape(case: &Case, shape: &Shape, x: usize, y: usize) -> Region {
    let mut rows = [0_u64; MAX_N];
    let mut cells = Vec::new();
    for rr in 0..shape.h {
        let row = x + rr;
        let begin = y + shape.left[rr];
        let mask = ((1_u64 << shape.len[rr]) - 1) << begin;
        rows[row] |= mask;
        for col in begin..begin + shape.len[rr] {
            cells.push((row * case.N + col) as u16);
        }
    }
    Region {
        rows,
        cells,
        perimeter: shape.perimeter,
    }
}

fn neighbor_cells(cell: usize, N: usize, output: &mut [usize; 4]) -> usize {
    let x = cell / N;
    let y = cell % N;
    let mut count = 0;
    if x > 0 {
        output[count] = cell - N;
        count += 1;
    }
    if x + 1 < N {
        output[count] = cell + N;
        count += 1;
    }
    if y > 0 {
        output[count] = cell - 1;
        count += 1;
    }
    if y + 1 < N {
        output[count] = cell + 1;
        count += 1;
    }
    count
}

fn free_components(free_rows: &Rows, N: usize, minimum_size: usize) -> Vec<Vec<usize>> {
    let mut visited = vec![false; N * N];
    let mut components = Vec::new();
    for start in 0..N * N {
        if visited[start] || (free_rows[start / N] >> (start % N)) & 1 == 0 {
            continue;
        }
        visited[start] = true;
        let mut component = vec![start];
        let mut head = 0;
        while head < component.len() {
            let cell = component[head];
            head += 1;
            let mut neighbors = [0; 4];
            let count = neighbor_cells(cell, N, &mut neighbors);
            for &next in &neighbors[..count] {
                if !visited[next] && (free_rows[next / N] >> (next % N)) & 1 != 0 {
                    visited[next] = true;
                    component.push(next);
                }
            }
        }
        if component.len() >= minimum_size {
            components.push(component);
        }
    }
    components
}

fn region_perimeter(cells: &[u16], N: usize) -> usize {
    let mut selected = vec![false; N * N];
    for &cell in cells {
        selected[cell as usize] = true;
    }
    let mut perimeter = 0;
    for &cell in cells {
        let cell = cell as usize;
        let x = cell / N;
        let y = cell % N;
        perimeter += usize::from(x == 0 || !selected[cell - N]);
        perimeter += usize::from(x + 1 == N || !selected[cell + N]);
        perimeter += usize::from(y == 0 || !selected[cell - 1]);
        perimeter += usize::from(y + 1 == N || !selected[cell + 1]);
    }
    perimeter
}

fn grow_region<R: Rng>(
    case: &Case,
    plan: &Plan,
    free_rows: &Rows,
    P: usize,
    style: PlacementStyle,
    rng: &mut R,
) -> Option<Region> {
    let components = free_components(free_rows, case.N, P);
    if components.is_empty() {
        return None;
    }
    let mut best: Option<(usize, i64, Region)> = None;
    for attempt in 0..style.growth_attempts.max(1) {
        let component_index = if attempt == 0 {
            components
                .iter()
                .enumerate()
                .max_by_key(|(_, component)| component.len())
                .map(|(index, _)| index)
                .unwrap()
        } else {
            rng.gen_range(0..components.len())
        };
        let component = &components[component_index];
        let seed = if attempt == 0 {
            *component
                .iter()
                .max_by_key(|&&cell| plan.usage[cell] as i64)
                .unwrap()
        } else {
            component[rng.gen_range(0..component.len())]
        };

        let mut selected = vec![false; case.N * case.N];
        let mut in_frontier = vec![false; case.N * case.N];
        let mut chosen = Vec::with_capacity(P);
        let mut frontier = Vec::new();
        selected[seed] = true;
        chosen.push(seed as u16);
        let mut neighbors = [0; 4];
        let count = neighbor_cells(seed, case.N, &mut neighbors);
        for &next in &neighbors[..count] {
            if (free_rows[next / case.N] >> (next % case.N)) & 1 != 0 {
                in_frontier[next] = true;
                frontier.push(next);
            }
        }

        while chosen.len() < P {
            if frontier.is_empty() {
                break;
            }
            let mut best_index = 0;
            let mut best_score = i64::MIN;
            for (index, &cell) in frontier.iter().enumerate() {
                let count = neighbor_cells(cell, case.N, &mut neighbors);
                let adjacent = neighbors[..count]
                    .iter()
                    .filter(|&&next| selected[next])
                    .count() as i64;
                let unavailable = 4_i64
                    - neighbors[..count]
                        .iter()
                        .filter(|&&next| (free_rows[next / case.N] >> (next % case.N)) & 1 != 0)
                        .count() as i64;
                let score = adjacent * 1_000_000
                    + (plan.usage[cell] as i64) * style.reuse_weight
                    + unavailable * style.contact_weight
                    + rng.gen_range(0..=style.noise.max(1));
                if score > best_score {
                    best_score = score;
                    best_index = index;
                }
            }
            let cell = frontier.swap_remove(best_index);
            in_frontier[cell] = false;
            if selected[cell] {
                continue;
            }
            selected[cell] = true;
            chosen.push(cell as u16);
            let count = neighbor_cells(cell, case.N, &mut neighbors);
            for &next in &neighbors[..count] {
                if !selected[next]
                    && !in_frontier[next]
                    && (free_rows[next / case.N] >> (next % case.N)) & 1 != 0
                {
                    in_frontier[next] = true;
                    frontier.push(next);
                }
            }
        }
        if chosen.len() != P {
            continue;
        }
        let perimeter = region_perimeter(&chosen, case.N);
        let mut rows = [0_u64; MAX_N];
        let mut reuse_score = 0_i64;
        for &cell in &chosen {
            let cell = cell as usize;
            rows[cell / case.N] |= 1_u64 << (cell % case.N);
            reuse_score += plan.usage[cell] as i64;
        }
        let region = Region {
            rows,
            cells: chosen,
            perimeter,
        };
        let key = (perimeter, -reuse_score);
        if best
            .as_ref()
            .is_none_or(|(best_perimeter, best_negative_reuse, _)| {
                key < (*best_perimeter, *best_negative_reuse)
            })
        {
            best = Some((key.0, key.1, region));
        }
    }
    best.map(|(_, _, region)| region)
}

fn find_region<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    overlaps: &[Vec<usize>],
    plan: &Plan,
    group_id: usize,
    style: PlacementStyle,
    rng: &mut R,
) -> Option<Region> {
    let group = &case.groups[group_id];
    let blocked = blocked_rows(plan, overlaps, group_id);
    let mut free_rows = [0_u64; MAX_N];
    let mut free_count = 0;
    for r in 0..case.N {
        free_rows[r] = case.grass_rows[r] & !blocked[r];
        free_count += free_rows[r].count_ones() as usize;
    }
    if free_count < group.P {
        return None;
    }
    let runs = build_runs(&free_rows, case.N);
    let mut found_perimeter = None;
    let mut best: Option<(i64, usize, usize, usize)> = None;
    for (shape_index, shape) in shapes_by_p[group.P].iter().enumerate() {
        if found_perimeter.is_some_and(|value| shape.perimeter > value) {
            break;
        }
        if shape.h > case.N || shape.w > case.N {
            continue;
        }
        let y_count = case.N - shape.w + 1;
        let valid_y = (1_u64 << y_count) - 1;
        for x in 0..=case.N - shape.h {
            let mut ys = valid_y;
            for rr in 0..shape.h {
                ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                if ys == 0 {
                    break;
                }
            }
            while ys != 0 {
                let y = ys.trailing_zeros() as usize;
                ys &= ys - 1;
                found_perimeter = Some(shape.perimeter);
                let score = position_score(case, plan, &free_rows, shape, x, y, style, rng);
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _, _, _)| score > *best_score)
                {
                    best = Some((score, shape_index, x, y));
                }
            }
        }
    }
    if let Some((_, shape_index, x, y)) = best {
        return Some(materialize_shape(
            case,
            &shapes_by_p[group.P][shape_index],
            x,
            y,
        ));
    }
    grow_region(case, plan, &free_rows, group.P, style, rng)
}

fn find_region_in_free<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    plan: &Plan,
    P: usize,
    free_rows: &Rows,
    style: PlacementStyle,
    rng: &mut R,
) -> Option<Region> {
    let free_count = free_rows[..case.N]
        .iter()
        .map(|row| row.count_ones() as usize)
        .sum::<usize>();
    if free_count < P {
        return None;
    }
    let runs = build_runs(free_rows, case.N);
    let mut found_perimeter = None;
    let mut best: Option<(i64, usize, usize, usize)> = None;
    for (shape_index, shape) in shapes_by_p[P].iter().enumerate() {
        if found_perimeter.is_some_and(|value| shape.perimeter > value) {
            break;
        }
        if shape.h > case.N || shape.w > case.N {
            continue;
        }
        let y_count = case.N - shape.w + 1;
        let valid_y = (1_u64 << y_count) - 1;
        for x in 0..=case.N - shape.h {
            let mut ys = valid_y;
            for rr in 0..shape.h {
                ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                if ys == 0 {
                    break;
                }
            }
            while ys != 0 {
                let y = ys.trailing_zeros() as usize;
                ys &= ys - 1;
                found_perimeter = Some(shape.perimeter);
                let score = position_score(case, plan, free_rows, shape, x, y, style, rng);
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _, _, _)| score > *best_score)
                {
                    best = Some((score, shape_index, x, y));
                }
            }
        }
    }
    if let Some((_, shape_index, x, y)) = best {
        return Some(materialize_shape(case, &shapes_by_p[P][shape_index], x, y));
    }
    grow_region(case, plan, free_rows, P, style, rng)
}

fn random_style<R: Rng>(rng: &mut R) -> PlacementStyle {
    PlacementStyle {
        reuse_weight: rng.gen_range(50..=500),
        contact_weight: rng.gen_range(20..=250),
        x_bias: rng.gen_range(-20..=20),
        y_bias: rng.gen_range(-20..=20),
        noise: rng.gen_range(0..=500),
        growth_attempts: 6,
    }
}

fn order_groups<R: Rng>(
    case: &Case,
    ids: &[usize],
    beta: f64,
    noise: f64,
    forced_first: Option<usize>,
    rng: &mut R,
) -> Vec<usize> {
    let mut keyed = ids
        .iter()
        .map(|&id| {
            let group = &case.groups[id];
            let duration = (group.T - group.S) as f64;
            let mut key =
                (group.max_fee as f64).ln() - (group.P as f64).ln() - beta * duration.ln();
            if noise > 0.0 {
                let u = rng.gen_range(1.0e-12_f64..1.0 - 1.0e-12);
                key += noise * (-(-u.ln()).ln());
            }
            if forced_first == Some(id) {
                key += 100.0;
            }
            (id, key)
        })
        .collect::<Vec<_>>();
    keyed.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    keyed.into_iter().map(|(id, _)| id).collect()
}

fn temporal_first_order(case: &Case, priority_order: &[usize], capacity_ratio: f64) -> Vec<usize> {
    let mut times = Vec::with_capacity(2 * case.M);
    for group in &case.groups {
        times.push(group.S);
        times.push(group.T);
    }
    times.sort_unstable();
    times.dedup();
    let grass_count = case.grass_rows[..case.N]
        .iter()
        .map(|row| row.count_ones() as usize)
        .sum::<usize>();
    let capacity = ((grass_count as f64) * capacity_ratio).round() as usize;
    let mut load = vec![0_usize; times.len() - 1];
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for &group_id in priority_order {
        let group = &case.groups[group_id];
        let begin = times.binary_search(&group.S).unwrap();
        let end = times.binary_search(&group.T).unwrap();
        if load[begin..end]
            .iter()
            .all(|&value| value + group.P <= capacity)
        {
            for value in &mut load[begin..end] {
                *value += group.P;
            }
            accepted.push(group_id);
        } else {
            rejected.push(group_id);
        }
    }
    accepted.extend(rejected);
    accepted
}

fn construct_plan<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    overlaps: &[Vec<usize>],
    beta: f64,
    noise: f64,
    capacity_ratio: f64,
    style: PlacementStyle,
    rng: &mut R,
) -> Plan {
    let ids = (0..case.M).collect::<Vec<_>>();
    let priority_order = order_groups(case, &ids, beta, noise, None, rng);
    let order = if capacity_ratio > 0.0 {
        temporal_first_order(case, &priority_order, capacity_ratio)
    } else {
        priority_order
    };
    let mut plan = Plan::empty(case);
    for group_id in order {
        if let Some(region) = find_region(case, shapes_by_p, overlaps, &plan, group_id, style, rng)
        {
            plan.insert(&case.groups[group_id], region);
        }
    }
    plan
}

fn relocate_one<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    overlaps: &[Vec<usize>],
    current: &Plan,
    rng: &mut R,
) -> Plan {
    let mut selected = None;
    for _ in 0..20 {
        let group_id = rng.gen_range(0..case.M);
        let Some(region) = &current.regions[group_id] else {
            continue;
        };
        let group = &case.groups[group_id];
        let loss = group.max_fee - fee(group.V, group.P, region.perimeter);
        let key = loss as f64 * rng.gen_range(0.8..=1.2);
        if selected.is_none_or(|(_, best_key)| key > best_key) {
            selected = Some((group_id, key));
        }
    }
    let Some((group_id, _)) = selected else {
        return current.clone();
    };
    let mut candidate = current.clone();
    candidate.remove(&case.groups[group_id]);
    let style = random_style(rng);
    let Some(region) = find_region(
        case,
        shapes_by_p,
        overlaps,
        &candidate,
        group_id,
        style,
        rng,
    ) else {
        return current.clone();
    };
    candidate.insert(&case.groups[group_id], region);
    candidate
}

fn compact_repack<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    overlaps: &[Vec<usize>],
    current: &Plan,
    rng: &mut R,
) -> Plan {
    let mut selected = None;
    for _ in 0..24 {
        let group_id = rng.gen_range(0..case.M);
        let Some(region) = &current.regions[group_id] else {
            continue;
        };
        let group = &case.groups[group_id];
        let loss = group.max_fee - fee(group.V, group.P, region.perimeter);
        if loss <= 0 {
            continue;
        }
        let key = loss as f64 * rng.gen_range(0.75..=1.25);
        if selected.is_none_or(|(_, best_key)| key > best_key) {
            selected = Some((group_id, key));
        }
    }
    let Some((target_id, _)) = selected else {
        return current.clone();
    };
    let target = &case.groups[target_id];
    let old_perimeter = current.regions[target_id].as_ref().unwrap().perimeter;
    let mut base = current.clone();
    base.remove(target);

    let mut cell_blockers = vec![Vec::<usize>::new(); case.N * case.N];
    for &other_id in &overlaps[target_id] {
        if let Some(region) = &base.regions[other_id] {
            for &cell in &region.cells {
                cell_blockers[cell as usize].push(other_id);
            }
        }
    }
    let grass_runs = build_runs(&case.grass_rows, case.N);
    let mut placements = Vec::<(i64, Region, Vec<usize>)>::new();
    let mut seen = vec![false; case.M];
    let mut found_perimeter = None;
    for shape in &shapes_by_p[target.P] {
        if shape.perimeter >= old_perimeter {
            break;
        }
        if found_perimeter.is_some_and(|value| shape.perimeter > value) {
            break;
        }
        let y_count = case.N - shape.w + 1;
        let valid_y = (1_u64 << y_count) - 1;
        for x in 0..=case.N - shape.h {
            let mut ys = valid_y;
            for rr in 0..shape.h {
                ys &= grass_runs[x + rr][shape.len[rr]] >> shape.left[rr];
                if ys == 0 {
                    break;
                }
            }
            while ys != 0 {
                let y = ys.trailing_zeros() as usize;
                ys &= ys - 1;
                found_perimeter = Some(shape.perimeter);
                let mut blockers = Vec::new();
                let mut blocker_area = 0_i64;
                let mut reuse = 0_i64;
                for rr in 0..shape.h {
                    let row = x + rr;
                    let begin = y + shape.left[rr];
                    for col in begin..begin + shape.len[rr] {
                        let cell = row * case.N + col;
                        reuse += base.usage[cell] as i64;
                        for &group_id in &cell_blockers[cell] {
                            if !seen[group_id] {
                                seen[group_id] = true;
                                blocker_area += case.groups[group_id].P as i64;
                                blockers.push(group_id);
                            }
                        }
                    }
                }
                let rank = blockers.len() as i64 * 1_000_000 + blocker_area * 1_000 - reuse * 20;
                let region = materialize_shape(case, shape, x, y);
                if placements.len() < 24 {
                    placements.push((rank, region, blockers.clone()));
                } else {
                    let worst = placements
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, (value, _, _))| *value)
                        .map(|(index, _)| index)
                        .unwrap();
                    if rank < placements[worst].0 {
                        placements[worst] = (rank, region, blockers.clone());
                    }
                }
                for group_id in blockers {
                    seen[group_id] = false;
                }
            }
        }
    }
    placements.sort_by_key(|(rank, _, _)| *rank);

    let mut best = current.clone();
    for (_, target_region, blockers) in placements {
        for attempt in 0..2 {
            let mut candidate = base.clone();
            for &group_id in &blockers {
                candidate.remove(&case.groups[group_id]);
            }
            candidate.insert(target, target_region.clone());
            let mut order = blockers.clone();
            if attempt == 0 {
                order.sort_by_key(|&id| std::cmp::Reverse(case.groups[id].P));
            } else {
                let mut keyed = order
                    .into_iter()
                    .map(|id| (id, rng.r#gen::<u64>()))
                    .collect::<Vec<_>>();
                keyed.sort_by_key(|&(_, key)| key);
                order = keyed.into_iter().map(|(id, _)| id).collect();
            }
            let style = random_style(rng);
            let mut success = true;
            for group_id in order {
                let Some(region) = find_region(
                    case,
                    shapes_by_p,
                    overlaps,
                    &candidate,
                    group_id,
                    style,
                    rng,
                ) else {
                    success = false;
                    break;
                };
                candidate.insert(&case.groups[group_id], region);
            }
            if success && candidate.score > best.score {
                best = candidate;
            }
        }
    }
    best
}

fn augment_static_plan<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    overlaps: &[Vec<usize>],
    plan: &mut Plan,
    rng: &mut R,
    started: &Instant,
    deadline_sec: f64,
) -> usize {
    let mut rejected = (0..case.M)
        .filter(|&id| plan.regions[id].is_none())
        .collect::<Vec<_>>();
    rejected.sort_by(|&left, &right| {
        let a = &case.groups[left];
        let b = &case.groups[right];
        let a_key = a.max_fee as f64 / ((a.T - a.S) as f64 + 500.0);
        let b_key = b.max_fee as f64 / ((b.T - b.S) as f64 + 500.0);
        b_key.total_cmp(&a_key).then_with(|| left.cmp(&right))
    });
    let mut added = 0;
    for group_id in rejected {
        if started.elapsed().as_secs_f64() >= deadline_sec {
            break;
        }
        let style = random_style(rng);
        if let Some(region) = find_region(case, shapes_by_p, overlaps, plan, group_id, style, rng) {
            plan.insert(&case.groups[group_id], region);
            added += 1;
        }
    }
    added
}

fn restore_turn(case: &Case, incoming_id: usize) -> Option<usize> {
    let departure = case.groups[incoming_id].T;
    ((incoming_id + 1)..case.M).find(|&id| case.groups[id].S > departure)
}

fn enumerate_target_options(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    base: &Plan,
    incoming_id: usize,
    started: &Instant,
    deadline_sec: f64,
) -> Vec<TargetOption> {
    const TARGET_OPTION_LIMIT: usize = 24;
    const BLOCKER_LIMIT: usize = 6;
    let incoming = &case.groups[incoming_id];
    let restore = restore_turn(case, incoming_id);
    let restore_time = restore.map(|id| case.groups[id].S);

    // Future arrivals cannot be moved before this target arrives. Their home regions are
    // therefore reservations that the target must avoid for its whole stay.
    let mut future_blocked = [0_u64; MAX_N];
    for other_id in (incoming_id + 1)..case.M {
        if case.groups[other_id].S >= incoming.T {
            break;
        }
        let Some(region) = &base.regions[other_id] else {
            continue;
        };
        for (target, &source) in future_blocked.iter_mut().zip(&region.rows) {
            *target |= source;
        }
    }
    let mut target_free = [0_u64; MAX_N];
    for row in 0..case.N {
        target_free[row] = case.grass_rows[row] & !future_blocked[row];
    }
    let runs = build_runs(&target_free, case.N);

    let mut active_by_cell = vec![Vec::<usize>::new(); case.N * case.N];
    for other_id in 0..incoming_id {
        if case.groups[other_id].T <= incoming.S {
            continue;
        }
        let Some(region) = &base.regions[other_id] else {
            continue;
        };
        for &cell in &region.cells {
            active_by_cell[cell as usize].push(other_id);
        }
    }

    let mut options = Vec::<TargetOption>::new();
    let mut seen = vec![false; case.M];
    let mut first_perimeter = None;
    'shape_loop: for shape in &shapes_by_p[incoming.P] {
        if first_perimeter.is_some_and(|value| shape.perimeter > value + 2) {
            break;
        }
        if shape.h > case.N || shape.w > case.N {
            continue;
        }
        let y_count = case.N - shape.w + 1;
        let valid_y = (1_u64 << y_count) - 1;
        for x in 0..=case.N - shape.h {
            if started.elapsed().as_secs_f64() >= deadline_sec {
                break 'shape_loop;
            }
            let mut ys = valid_y;
            for rr in 0..shape.h {
                ys &= runs[x + rr][shape.len[rr]] >> shape.left[rr];
                if ys == 0 {
                    break;
                }
            }
            while ys != 0 {
                let y = ys.trailing_zeros() as usize;
                ys &= ys - 1;
                let region = materialize_shape(case, shape, x, y);
                let mut blockers = Vec::new();
                let mut reuse = 0_i64;
                for &cell in &region.cells {
                    let cell = cell as usize;
                    reuse += base.usage[cell] as i64;
                    for &group_id in &active_by_cell[cell] {
                        if !seen[group_id] {
                            seen[group_id] = true;
                            blockers.push(group_id);
                        }
                    }
                }
                if blockers.is_empty() || blockers.len() > BLOCKER_LIMIT {
                    for group_id in blockers {
                        seen[group_id] = false;
                    }
                    continue;
                }
                first_perimeter.get_or_insert(shape.perimeter);
                let mut relocation_cost = 0_i64;
                let mut blocker_area = 0_i64;
                for &group_id in &blockers {
                    let group = &case.groups[group_id];
                    relocation_cost += move_cost(case, group);
                    if restore_time.is_some_and(|time| group.T > time) {
                        relocation_cost += move_cost(case, group);
                    }
                    blocker_area += group.P as i64;
                }
                let rank = relocation_cost * 1_000_000
                    + blockers.len() as i64 * 100_000
                    + blocker_area * 100
                    - reuse;
                if options.len() < TARGET_OPTION_LIMIT {
                    options.push(TargetOption {
                        region,
                        blockers: blockers.clone(),
                        rank,
                    });
                } else {
                    let worst = options
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, option)| option.rank)
                        .map(|(index, _)| index)
                        .unwrap();
                    if rank < options[worst].rank {
                        options[worst] = TargetOption {
                            region,
                            blockers: blockers.clone(),
                            rank,
                        };
                    }
                }
                for group_id in blockers {
                    seen[group_id] = false;
                }
            }
        }
    }
    options.sort_by_key(|option| option.rank);
    options
}

fn build_episode_for_target<R: Rng>(
    case: &Case,
    shapes_by_p: &[Vec<Shape>],
    base: &Plan,
    incoming_id: usize,
    rng: &mut R,
    started: &Instant,
    deadline_sec: f64,
) -> Option<Episode> {
    let incoming = &case.groups[incoming_id];
    let restore = restore_turn(case, incoming_id);
    let restore_time = restore.map(|id| case.groups[id].S);
    let options =
        enumerate_target_options(case, shapes_by_p, base, incoming_id, started, deadline_sec);
    let mut best: Option<Episode> = None;
    for option in options {
        if started.elapsed().as_secs_f64() >= deadline_sec {
            break;
        }
        let mut blocker_set = vec![false; case.M];
        for &group_id in &option.blockers {
            blocker_set[group_id] = true;
        }
        for attempt in 0..3 {
            if started.elapsed().as_secs_f64() >= deadline_sec {
                break;
            }
            let mut order = option.blockers.clone();
            match attempt {
                0 => order.sort_by_key(|&id| std::cmp::Reverse(case.groups[id].P)),
                1 => order.sort_by_key(|&id| std::cmp::Reverse(move_cost(case, &case.groups[id]))),
                _ => order.shuffle(rng),
            }
            let mut relocations = Vec::<(usize, Region)>::new();
            let mut success = true;
            let style = random_style(rng);
            for group_id in order {
                let group = &case.groups[group_id];
                let end = restore_time.map_or(group.T, |time| group.T.min(time));
                let mut blocked = option.region.rows;
                for other_id in 0..case.M {
                    if blocker_set[other_id]
                        || case.groups[other_id].S >= end
                        || case.groups[other_id].T <= incoming.S
                    {
                        continue;
                    }
                    let Some(region) = &base.regions[other_id] else {
                        continue;
                    };
                    for (target, &source) in blocked.iter_mut().zip(&region.rows) {
                        *target |= source;
                    }
                }
                for (_, region) in &relocations {
                    for (target, &source) in blocked.iter_mut().zip(&region.rows) {
                        *target |= source;
                    }
                }
                let mut free_rows = [0_u64; MAX_N];
                for row in 0..case.N {
                    free_rows[row] = case.grass_rows[row] & !blocked[row];
                }
                let Some(region) =
                    find_region_in_free(case, shapes_by_p, base, group.P, &free_rows, style, rng)
                else {
                    success = false;
                    break;
                };
                relocations.push((group_id, region));
            }
            if !success {
                continue;
            }
            let mut fee_loss = 0_i64;
            let mut relocation_cost = 0_i64;
            for (group_id, alternate) in &relocations {
                let group = &case.groups[*group_id];
                let home = base.regions[*group_id].as_ref().unwrap();
                fee_loss += fee(group.V, group.P, home.perimeter)
                    - fee(group.V, group.P, home.perimeter.max(alternate.perimeter));
                relocation_cost += move_cost(case, group);
                if restore_time.is_some_and(|time| group.T > time) {
                    relocation_cost += move_cost(case, group);
                }
            }
            let estimated_gain =
                fee(incoming.V, incoming.P, option.region.perimeter) - fee_loss - relocation_cost;
            if estimated_gain > 0
                && best
                    .as_ref()
                    .is_none_or(|episode| estimated_gain > episode.estimated_gain)
            {
                best = Some(Episode {
                    incoming_id,
                    target_region: option.region.clone(),
                    relocations,
                    restore_turn: restore,
                    estimated_gain,
                });
            }
        }
    }
    best
}

fn select_episodes(case: &Case, candidates: Vec<Option<Episode>>) -> Vec<Episode> {
    let mut best_gain = vec![0_i64; case.M + 2];
    let mut take = vec![false; case.M];
    for turn in (0..case.M).rev() {
        best_gain[turn] = best_gain[turn + 1];
        let Some(episode) = &candidates[turn] else {
            continue;
        };
        let next = episode
            .restore_turn
            .map_or(case.M, |value| (value + 1).min(case.M));
        let candidate_gain = episode.estimated_gain + best_gain[next];
        if candidate_gain > best_gain[turn] {
            best_gain[turn] = candidate_gain;
            take[turn] = true;
        }
    }
    let mut selected = Vec::new();
    let mut turn = 0;
    while turn < case.M {
        if take[turn] {
            let episode = candidates[turn].as_ref().unwrap().clone();
            turn = episode
                .restore_turn
                .map_or(case.M, |value| (value + 1).min(case.M));
            selected.push(episode);
        } else {
            turn += 1;
        }
    }
    selected
}

fn append_region(output: &mut String, region: &Region, N: usize) {
    for &cell in &region.cells {
        let cell = cell as usize;
        output.push_str(&format!("{} {}\n", cell / N, cell % N));
    }
}

fn build_output_with_episodes(case: &Case, base: &Plan, episodes: &[Episode]) -> String {
    let mut start_at = vec![None::<usize>; case.M];
    let mut restore_at = vec![Vec::<usize>::new(); case.M];
    for (episode_index, episode) in episodes.iter().enumerate() {
        start_at[episode.incoming_id] = Some(episode_index);
        if let Some(turn) = episode.restore_turn {
            restore_at[turn].push(episode_index);
        }
    }

    let mut output = String::new();
    for turn in 0..case.M {
        let now = case.groups[turn].S;
        let mut moves = Vec::<(usize, &Region)>::new();
        for &episode_index in &restore_at[turn] {
            let episode = &episodes[episode_index];
            for (group_id, _) in &episode.relocations {
                if case.groups[*group_id].T > now {
                    moves.push((*group_id, base.regions[*group_id].as_ref().unwrap()));
                }
            }
        }
        if let Some(episode_index) = start_at[turn] {
            for (group_id, region) in &episodes[episode_index].relocations {
                moves.push((*group_id, region));
            }
        }
        moves.sort_by_key(|(group_id, _)| *group_id);
        output.push_str(&format!("{}\n", moves.len()));
        for (group_id, region) in moves {
            output.push_str(&format!("{group_id}\n"));
            append_region(&mut output, region, case.N);
        }

        if let Some(region) = &base.regions[turn] {
            output.push_str("Yes\n");
            append_region(&mut output, region, case.N);
        } else if let Some(episode_index) = start_at[turn] {
            output.push_str("Yes\n");
            append_region(&mut output, &episodes[episode_index].target_region, case.N);
        } else {
            output.push_str("No\n");
        }
    }
    output
}

fn replay_dynamic_output(
    input_text: &str,
    output: String,
    episodes: usize,
    candidates: usize,
) -> Result<DynamicResult, String> {
    let input = tools::parse_input(input_text);
    let replay = tools::parse_output(&input, &output);
    if let Some(error) = &replay.error {
        return Err(format!("dynamic replay失敗: {error}"));
    }
    let final_frame = replay
        .frames
        .last()
        .ok_or_else(|| "dynamic replayのframeがない".to_string())?;
    let moves = replay
        .frames
        .iter()
        .map(|frame| frame.moved.len())
        .sum::<usize>();
    let accepted_ids = replay
        .frames
        .iter()
        .filter_map(|frame| frame.arrival)
        .filter_map(|(group_id, accepted)| accepted.then_some(group_id))
        .collect::<Vec<_>>();
    Ok(DynamicResult {
        output,
        score: replay.score,
        total_fee: final_frame.total_fee,
        move_cost: final_frame.total_move_cost,
        accepted: final_frame.accepted,
        accepted_ids,
        episodes,
        moves,
        candidates,
    })
}

fn optimize_dynamic_episodes<R: Rng>(
    case: &Case,
    input_text: &str,
    shapes_by_p: &[Vec<Shape>],
    base: &Plan,
    rng: &mut R,
    started: &Instant,
    deadline_sec: f64,
) -> Result<DynamicResult, String> {
    let mut rejected = (0..case.M)
        .filter(|&id| base.regions[id].is_none())
        .collect::<Vec<_>>();
    rejected.sort_by(|&left, &right| {
        let a = &case.groups[left];
        let b = &case.groups[right];
        let a_key = a.max_fee as f64 / ((a.T - a.S) as f64 + 750.0);
        let b_key = b.max_fee as f64 / ((b.T - b.S) as f64 + 750.0);
        b_key.total_cmp(&a_key).then_with(|| left.cmp(&right))
    });
    let mut candidates = vec![None::<Episode>; case.M];
    let mut candidate_count = 0;
    for incoming_id in rejected {
        if started.elapsed().as_secs_f64() >= deadline_sec {
            break;
        }
        if let Some(episode) = build_episode_for_target(
            case,
            shapes_by_p,
            base,
            incoming_id,
            rng,
            started,
            deadline_sec,
        ) {
            candidates[incoming_id] = Some(episode);
            candidate_count += 1;
        }
    }
    let episodes = select_episodes(case, candidates);
    let output = build_output_with_episodes(case, base, &episodes);
    replay_dynamic_output(input_text, output, episodes.len(), candidate_count)
}

fn validate_plan(case: &Case, plan: &Plan) -> Result<(), String> {
    let mut expected_score = 0_i64;
    let mut expected_accepted = 0;
    for group in &case.groups {
        let Some(region) = &plan.regions[group.id] else {
            continue;
        };
        expected_accepted += 1;
        if region.cells.len() != group.P {
            return Err(format!(
                "group {}: cells={} P={}",
                group.id,
                region.cells.len(),
                group.P
            ));
        }
        let mut seen = HashSet::new();
        for &cell in &region.cells {
            let cell = cell as usize;
            if !seen.insert(cell) {
                return Err(format!("group {}: 重複セル", group.id));
            }
            if (case.grass_rows[cell / case.N] >> (cell % case.N)) & 1 == 0 {
                return Err(format!("group {}: 池または盤外", group.id));
            }
        }
        let mut reached = HashSet::new();
        let mut stack = vec![region.cells[0] as usize];
        reached.insert(region.cells[0] as usize);
        while let Some(cell) = stack.pop() {
            let mut neighbors = [0; 4];
            let count = neighbor_cells(cell, case.N, &mut neighbors);
            for &next in &neighbors[..count] {
                if seen.contains(&next) && reached.insert(next) {
                    stack.push(next);
                }
            }
        }
        if reached.len() != group.P {
            return Err(format!("group {}: 非連結", group.id));
        }
        let actual_perimeter = region_perimeter(&region.cells, case.N);
        if actual_perimeter != region.perimeter {
            return Err(format!(
                "group {}: perimeter={} actual={actual_perimeter}",
                group.id, region.perimeter
            ));
        }
        expected_score += fee(group.V, group.P, region.perimeter);
    }
    if expected_score != plan.score || expected_accepted != plan.accepted {
        return Err(format!(
            "集計不一致: score={} expected_score={expected_score}, accepted={} expected_accepted={expected_accepted}",
            plan.score, plan.accepted
        ));
    }
    for i in 0..case.M {
        let Some(a) = &plan.regions[i] else {
            continue;
        };
        for j in i + 1..case.M {
            let Some(b) = &plan.regions[j] else {
                continue;
            };
            if !intervals_overlap(&case.groups[i], &case.groups[j]) {
                continue;
            }
            for r in 0..case.N {
                if a.rows[r] & b.rows[r] != 0 {
                    return Err(format!("group {i} と {j} が重なる"));
                }
            }
        }
    }
    Ok(())
}

fn parse_args() -> Result<Config, String> {
    let mut args = env::args().skip(1);
    let input = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: offline_reference <input> [--seconds <f64>] [--seed <u64>] [--output <path>]"
            .to_string()
    })?;
    let mut seconds: f64 = 60.0;
    let mut seed = 0_u64;
    let mut output = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--seconds" => {
                seconds = args
                    .next()
                    .ok_or_else(|| "--seconds の値がない".to_string())?
                    .parse()
                    .map_err(|error| format!("--seconds が不正: {error}"))?;
            }
            "--seed" => {
                seed = args
                    .next()
                    .ok_or_else(|| "--seed の値がない".to_string())?
                    .parse()
                    .map_err(|error| format!("--seed が不正: {error}"))?;
            }
            "--output" => {
                output = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output の値がない".to_string())?,
                ));
            }
            _ => return Err(format!("未知の引数: {flag}")),
        }
    }
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("--seconds は正の有限値にする".to_string());
    }
    Ok(Config {
        input,
        seconds,
        seed,
        output,
    })
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    let started = Instant::now();
    let input_text = fs::read_to_string(&config.input)
        .map_err(|error| format!("{} を読めない: {error}", config.input.display()))?;
    let case = parse_case(&config.input)?;
    let shapes_by_p = generate_shapes(case.N);
    let overlaps = build_overlaps(&case);
    let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
    let grass_count = case.grass_rows[..case.N]
        .iter()
        .map(|row| row.count_ones() as u64)
        .sum::<u64>();
    let total_cell_time = case
        .groups
        .iter()
        .map(|group| group.P as u64 * (group.T - group.S) as u64)
        .sum::<u64>();
    let load_ratio = total_cell_time as f64 / (grass_count as f64 * 100_000.0);
    let beta_center = 0.68 + 0.35 * ((load_ratio - 0.8) / 1.4).clamp(0.0, 1.0);
    let initial_deadline = config.seconds * 0.15;
    let mut best: Option<Plan> = None;
    let mut builds = 0_usize;
    while best.is_none()
        || builds < 12
        || (builds < 40 && started.elapsed().as_secs_f64() < initial_deadline)
    {
        let beta = if builds == 0 {
            1.0
        } else if builds <= 2 {
            0.9
        } else if builds == 3 {
            beta_center
        } else if builds == 4 {
            (beta_center - 0.10).max(0.50)
        } else {
            rng.gen_range((beta_center - 0.25).max(0.45)..=(beta_center + 0.10).min(1.15))
        };
        let noise = if builds < 3 {
            0.0
        } else if builds == 3 {
            0.07
        } else if builds == 4 {
            0.16
        } else {
            rng.gen_range(0.03..=0.30)
        };
        let capacity_ratio = if builds == 0 {
            0.0
        } else if builds == 1 {
            0.0
        } else if builds == 2 {
            0.90
        } else if builds == 3 {
            0.0
        } else if builds == 4 {
            0.0
        } else if load_ratio > 1.1 && rng.gen_bool(0.25) {
            rng.gen_range(0.78..=1.0)
        } else {
            0.0
        };
        let style = if builds == 0 {
            PlacementStyle {
                reuse_weight: 250,
                contact_weight: 120,
                x_bias: -4,
                y_bias: -4,
                noise: 0,
                growth_attempts: 6,
            }
        } else if builds == 1 {
            PlacementStyle {
                reuse_weight: 5_000,
                contact_weight: 120,
                x_bias: 4,
                y_bias: 4,
                noise: 0,
                growth_attempts: 6,
            }
        } else if builds == 2 {
            PlacementStyle {
                reuse_weight: 250,
                contact_weight: 120,
                x_bias: 4,
                y_bias: -4,
                noise: 0,
                growth_attempts: 6,
            }
        } else if builds == 3 {
            PlacementStyle {
                reuse_weight: 180,
                contact_weight: 160,
                x_bias: 0,
                y_bias: 0,
                noise: 200,
                growth_attempts: 6,
            }
        } else if builds == 4 {
            PlacementStyle {
                reuse_weight: 435,
                contact_weight: 150,
                x_bias: 0,
                y_bias: 0,
                noise: 250,
                growth_attempts: 6,
            }
        } else {
            random_style(&mut rng)
        };
        let plan = construct_plan(
            &case,
            &shapes_by_p,
            &overlaps,
            beta,
            noise,
            capacity_ratio,
            style,
            &mut rng,
        );
        builds += 1;
        if best.as_ref().is_none_or(|value| plan.score > value.score) {
            eprintln!(
                "initial_best build={} score={} beta={:.3} noise={:.3} capacity_ratio={:.3} reuse={} contact={} x_bias={} y_bias={} placement_noise={}",
                builds,
                plan.score,
                beta,
                noise,
                capacity_ratio,
                style.reuse_weight,
                style.contact_weight,
                style.x_bias,
                style.y_bias,
                style.noise,
            );
            best = Some(plan);
        }
        if started.elapsed().as_secs_f64() >= config.seconds * 0.90 {
            break;
        }
    }

    let mut best = best.expect("初期解がない");
    let initial_score = best.score;
    let mut current = best.clone();
    let lns_started = started.elapsed().as_secs_f64();
    let mut iterations = 0_usize;
    let mut last_improvement = 0_usize;
    let mut neighborhood_trials = [0_usize; 2];
    let mut neighborhood_best_updates = [0_usize; 2];
    let mut neighborhood_best_gain = [0_i64; 2];
    let base_deadline = config.seconds * 0.94;
    while started.elapsed().as_secs_f64() < base_deadline {
        let elapsed = started.elapsed().as_secs_f64();
        let progress =
            ((elapsed - lns_started) / (base_deadline - lns_started).max(1.0e-9)).clamp(0.0, 1.0);
        let neighborhood = rng.gen_range(0..100);
        let kind = if progress < 0.65 || neighborhood < 80 {
            0
        } else {
            1
        };
        neighborhood_trials[kind] += 1;
        let candidate = match kind {
            0 => relocate_one(&case, &shapes_by_p, &overlaps, &current, &mut rng),
            _ => compact_repack(&case, &shapes_by_p, &overlaps, &current, &mut rng),
        };
        iterations += 1;
        let delta = candidate.score - current.score;
        let temperature = 800_000.0 * (1.0 - progress).powi(3) + 2_000.0;
        let accept =
            delta >= 0 || rng.r#gen::<f64>() < ((delta as f64) / temperature).exp().clamp(0.0, 1.0);
        if accept {
            current = candidate;
        }
        if current.score > best.score {
            neighborhood_best_updates[kind] += 1;
            neighborhood_best_gain[kind] += current.score - best.score;
            best = current.clone();
            last_improvement = iterations;
        }
        if iterations.saturating_sub(last_improvement) >= 200 {
            current = best.clone();
            last_improvement = iterations;
        }
    }

    let static_added = augment_static_plan(
        &case,
        &shapes_by_p,
        &overlaps,
        &mut best,
        &mut rng,
        &started,
        config.seconds * 0.95,
    );
    validate_plan(&case, &best)?;
    let base_score = best.score;
    let base_accepted = best.accepted;
    let dynamic = optimize_dynamic_episodes(
        &case,
        &input_text,
        &shapes_by_p,
        &best,
        &mut rng,
        &started,
        config.seconds * 0.985,
    )?;
    if dynamic.score < base_score {
        return Err(format!(
            "再配置後scoreが基礎計画を下回った: dynamic={} base={base_score}",
            dynamic.score
        ));
    }
    if let Some(path) = &config.output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("{} を作れない: {error}", parent.display()))?;
        }
        fs::write(path, &dynamic.output)
            .map_err(|error| format!("{} に書けない: {error}", path.display()))?;
    }
    let elapsed_ms = started.elapsed().as_millis();
    eprintln!(
        "neighborhood trials={:?} best_updates={:?} best_gain={:?}",
        neighborhood_trials, neighborhood_best_updates, neighborhood_best_gain
    );
    let optimistic_selected = case
        .groups
        .iter()
        .filter(|group| dynamic.accepted_ids.binary_search(&group.id).is_ok())
        .map(|group| group.max_fee)
        .sum::<i64>();
    let selected_cell_time = case
        .groups
        .iter()
        .filter(|group| dynamic.accepted_ids.binary_search(&group.id).is_ok())
        .map(|group| group.P as u64 * (group.T - group.S) as u64)
        .sum::<u64>();
    println!(
        "case,score,base_score,initial_score,optimistic_selected,shape_loss,accepted,extra_accepted,selected_cell_time,moves,move_cost,episodes,static_added,builds,lns_iterations,dynamic_candidates,elapsed_ms"
    );
    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        case.name,
        dynamic.score,
        base_score,
        initial_score,
        optimistic_selected,
        optimistic_selected - dynamic.total_fee,
        dynamic.accepted,
        dynamic.accepted - base_accepted,
        selected_cell_time,
        dynamic.moves,
        dynamic.move_cost,
        dynamic.episodes,
        static_added,
        builds,
        iterations,
        dynamic.candidates,
        elapsed_ms
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_perimeters_are_correct() {
        assert_eq!(minimum_perimeter(4), 8);
        assert_eq!(minimum_perimeter(5), 10);
        assert_eq!(minimum_perimeter(6), 10);
        assert_eq!(minimum_perimeter(9), 12);
    }

    #[test]
    fn generated_shapes_have_the_requested_area_and_perimeter() {
        let all = generate_shapes(50);
        for P in 4..=MAX_P {
            assert!(!all[P].is_empty());
            for shape in &all[P] {
                assert_eq!(shape.len.iter().sum::<usize>(), P);
                assert_eq!(shape_perimeter(shape, P), shape.perimeter);
            }
        }
    }
}
