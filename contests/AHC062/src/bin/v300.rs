use proconio::input;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BLOCK_SIDE: usize = 2;
const STATE_COUNT: usize = 12;
const INITIAL_SWEEP_PASSES: usize = 20;
const LEARNED_SWEEP_PASSES: usize = 36;
const ELITE_COUNT: usize = 16;
const SEED_COUNT: usize = 12;
const SOFTMAX_BETA: f64 = 4.0;
const TARGET_ALPHA: f64 = 0.72;
const STABILITY_TAU: f64 = 2500.0;
const ETA_LIST: [f64; 3] = [0.50, 0.80, 1.00];

#[derive(Clone)]
struct CandidateSummary {
    raw_score: i128,
    positions: Vec<u32>,
    macro_path: Vec<u16>,
    hash: u64,
}

struct EvalResult {
    raw_score: i128,
    path: Vec<usize>,
    positions: Vec<u32>,
}

fn main() {
    input! {
        n: usize,
        a_grid: [[i64; n]; n],
    }

    assert!(n % BLOCK_SIDE == 0);

    let m = n * n;
    let b = n / BLOCK_SIDE;
    let macro_len = b * b;

    let mut a = vec![0_i64; m];
    let mut cell_coords = vec![(0_u16, 0_u16); m];
    for r in 0..n {
        for c in 0..n {
            let id = cell_id(r, c, n);
            a[id] = a_grid[r][c];
            cell_coords[id] = (r as u16, c as u16);
        }
    }
    let rank: Vec<f64> = a.iter().map(|&v| (v - 1) as f64).collect();

    let block_cells = build_block_cells(n, b);
    let block_coords = build_block_coords(b);
    let states = build_states();
    let base_macro_paths = build_macro_paths(b);
    let rank_block_weights = compute_block_weights(&rank, &block_cells);

    let mut best_raw_score = i128::MIN;
    let mut best_path = Vec::new();
    let mut initial_candidates = Vec::new();

    for macro_path in &base_macro_paths {
        let direct =
            evaluate_macro_path(macro_path, &block_cells, &cell_coords, &states, &rank, &a);
        update_best(&direct, &mut best_raw_score, &mut best_path);
        initial_candidates.push(CandidateSummary {
            raw_score: direct.raw_score,
            positions: direct.positions,
            macro_path: macro_path.clone(),
            hash: hash_macro_path(macro_path),
        });

        let mut improved_macro = macro_path.clone();
        optimize_macro_adj_swaps(
            &mut improved_macro,
            &block_coords,
            &rank_block_weights,
            INITIAL_SWEEP_PASSES,
        );
        let improved = evaluate_macro_path(
            &improved_macro,
            &block_cells,
            &cell_coords,
            &states,
            &rank,
            &a,
        );
        update_best(&improved, &mut best_raw_score, &mut best_path);
        initial_candidates.push(CandidateSummary {
            raw_score: improved.raw_score,
            positions: improved.positions,
            macro_path: improved_macro.clone(),
            hash: hash_macro_path(&improved_macro),
        });
    }

    let target = learn_target(&initial_candidates, &rank, m);
    let seed_paths = select_seed_paths(&initial_candidates, SEED_COUNT);

    for &eta in &ETA_LIST {
        let blended = blend_weights(&rank, &target, eta);
        let blended_block_weights = compute_block_weights(&blended, &block_cells);
        for seed_macro in &seed_paths {
            let mut macro_path = seed_macro.clone();
            optimize_macro_adj_swaps(
                &mut macro_path,
                &block_coords,
                &blended_block_weights,
                LEARNED_SWEEP_PASSES,
            );
            let refined =
                evaluate_macro_path(&macro_path, &block_cells, &cell_coords, &states, &rank, &a);
            update_best(&refined, &mut best_raw_score, &mut best_path);
        }
    }

    validate_path(&best_path, n, &cell_coords);

    if std::env::var_os("V300_DEBUG").is_some() {
        eprintln!(
            "macro_paths={} initial={} best_raw={}",
            base_macro_paths.len(),
            initial_candidates.len(),
            best_raw_score
        );
    }

    for &cell in &best_path {
        let (r, c) = cell_coords[cell];
        println!("{} {}", r, c);
    }

    assert_eq!(best_path.len(), macro_len * 4);
}

fn cell_id(r: usize, c: usize, n: usize) -> usize {
    r * n + c
}

fn block_id(br: usize, bc: usize, b: usize) -> usize {
    br * b + bc
}

fn build_block_cells(n: usize, b: usize) -> Vec<[usize; 4]> {
    let mut blocks = vec![[0_usize; 4]; b * b];
    for br in 0..b {
        for bc in 0..b {
            let r = br * BLOCK_SIDE;
            let c = bc * BLOCK_SIDE;
            blocks[block_id(br, bc, b)] = [
                cell_id(r, c, n),
                cell_id(r, c + 1, n),
                cell_id(r + 1, c, n),
                cell_id(r + 1, c + 1, n),
            ];
        }
    }
    blocks
}

fn build_block_coords(b: usize) -> Vec<(i16, i16)> {
    let mut coords = vec![(0_i16, 0_i16); b * b];
    for br in 0..b {
        for bc in 0..b {
            coords[block_id(br, bc, b)] = (br as i16, bc as i16);
        }
    }
    coords
}

fn build_states() -> [(u8, u8); STATE_COUNT] {
    let mut states = [(0_u8, 0_u8); STATE_COUNT];
    let mut idx = 0;
    for s in 0..4 {
        for t in 0..4 {
            if s != t {
                states[idx] = (s as u8, t as u8);
                idx += 1;
            }
        }
    }
    states
}

fn build_macro_paths(b: usize) -> Vec<Vec<u16>> {
    let base_patterns = vec![
        row_snake_coords(b),
        pair_weave_coords(b),
        diag_snake_coords(b),
        spiral_coords(b),
    ];
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for base in base_patterns {
        for sym in 0..8 {
            let mut path = Vec::with_capacity(b * b);
            for &(r, c) in &base {
                let (tr, tc) = apply_symmetry(r, c, b, sym);
                path.push(block_id(tr, tc, b) as u16);
            }
            let hash = hash_macro_path(&path);
            if seen.insert(hash) {
                paths.push(path);
            }
        }
    }
    paths
}

fn row_snake_coords(b: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(b * b);
    for r in 0..b {
        if r % 2 == 0 {
            for c in 0..b {
                path.push((r, c));
            }
        } else {
            for c in (0..b).rev() {
                path.push((r, c));
            }
        }
    }
    path
}

fn pair_weave_coords(b: usize) -> Vec<(usize, usize)> {
    assert!(b % 2 == 0);
    let mut path = Vec::with_capacity(b * b);
    for pair in 0..(b / 2) {
        let r0 = pair * 2;
        let r1 = r0 + 1;
        if pair % 2 == 0 {
            for c in 0..b {
                path.push((r0, c));
                path.push((r1, c));
            }
        } else {
            for c in (0..b).rev() {
                path.push((r0, c));
                path.push((r1, c));
            }
        }
    }
    path
}

fn diag_snake_coords(b: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(b * b);
    for s in 0..=(2 * (b - 1)) {
        let r_min = s.saturating_sub(b - 1);
        let r_max = s.min(b - 1);
        if s % 2 == 0 {
            for r in (r_min..=r_max).rev() {
                path.push((r, s - r));
            }
        } else {
            for r in r_min..=r_max {
                path.push((r, s - r));
            }
        }
    }
    path
}

fn spiral_coords(b: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(b * b);
    let mut top = 0_usize;
    let mut bottom = b - 1;
    let mut left = 0_usize;
    let mut right = b - 1;

    while top <= bottom && left <= right {
        for c in left..=right {
            path.push((top, c));
        }
        if top == bottom {
            break;
        }
        top += 1;

        for r in top..=bottom {
            path.push((r, right));
        }
        if left == right {
            break;
        }
        if right == 0 {
            break;
        }
        right -= 1;

        for c in (left..=right).rev() {
            path.push((bottom, c));
        }
        if top > bottom {
            break;
        }
        if bottom == 0 {
            break;
        }
        bottom -= 1;

        for r in (top..=bottom).rev() {
            path.push((r, left));
        }
        left += 1;
    }

    path
}

fn apply_symmetry(r: usize, c: usize, b: usize, sym: usize) -> (usize, usize) {
    match sym {
        0 => (r, c),
        1 => (c, b - 1 - r),
        2 => (b - 1 - r, b - 1 - c),
        3 => (b - 1 - c, r),
        4 => (r, b - 1 - c),
        5 => (b - 1 - r, c),
        6 => (c, r),
        7 => (b - 1 - c, b - 1 - r),
        _ => unreachable!(),
    }
}

fn compute_block_weights(weights: &[f64], block_cells: &[[usize; 4]]) -> Vec<f64> {
    block_cells
        .iter()
        .map(|cells| weights[cells[0]] + weights[cells[1]] + weights[cells[2]] + weights[cells[3]])
        .collect()
}

fn optimize_macro_adj_swaps(
    path: &mut [u16],
    block_coords: &[(i16, i16)],
    block_weights: &[f64],
    passes: usize,
) {
    if path.len() < 2 {
        return;
    }
    for _ in 0..passes {
        let mut changed = false;
        for i in 0..(path.len() - 1) {
            let left = path[i] as usize;
            let right = path[i + 1] as usize;
            if block_weights[left] <= block_weights[right] {
                continue;
            }
            if i > 0 {
                let prev = path[i - 1] as usize;
                if !macro_adj(block_coords[prev], block_coords[right]) {
                    continue;
                }
            }
            if i + 2 < path.len() {
                let next = path[i + 2] as usize;
                if !macro_adj(block_coords[left], block_coords[next]) {
                    continue;
                }
            }
            path.swap(i, i + 1);
            changed = true;
        }
        if !changed {
            break;
        }
    }
}

fn macro_adj(a: (i16, i16), b: (i16, i16)) -> bool {
    let dr = (a.0 - b.0).abs();
    let dc = (a.1 - b.1).abs();
    dr <= 1 && dc <= 1 && (dr != 0 || dc != 0)
}

fn cell_adj(a: usize, b: usize, coords: &[(u16, u16)]) -> bool {
    let (ar, ac) = coords[a];
    let (br, bc) = coords[b];
    let dr = ar.abs_diff(br);
    let dc = ac.abs_diff(bc);
    dr <= 1 && dc <= 1 && (dr != 0 || dc != 0)
}

fn evaluate_macro_path(
    macro_path: &[u16],
    block_cells: &[[usize; 4]],
    cell_coords: &[(u16, u16)],
    states: &[(u8, u8); STATE_COUNT],
    order_weights: &[f64],
    raw_values: &[i64],
) -> EvalResult {
    let macro_len = macro_path.len();
    let mut parent = vec![[0_u8; STATE_COUNT]; macro_len];
    let mut dp_prev = [f64::NEG_INFINITY; STATE_COUNT];
    let mut dp_cur = [f64::NEG_INFINITY; STATE_COUNT];

    let first_cells = &block_cells[macro_path[0] as usize];
    for (state_idx, &state) in states.iter().enumerate() {
        dp_prev[state_idx] = state_contrib(first_cells, order_weights, 0, state);
    }

    for k in 1..macro_len {
        let cells = &block_cells[macro_path[k] as usize];
        let prev_cells = &block_cells[macro_path[k - 1] as usize];
        let base = k * 4;
        let mut contribs = [0.0_f64; STATE_COUNT];
        let mut starts = [0_usize; STATE_COUNT];
        let mut prev_ends = [0_usize; STATE_COUNT];

        for (state_idx, &state) in states.iter().enumerate() {
            contribs[state_idx] = state_contrib(cells, order_weights, base, state);
            starts[state_idx] = cells[state.0 as usize];
        }
        for (prev_state_idx, &state) in states.iter().enumerate() {
            prev_ends[prev_state_idx] = prev_cells[state.1 as usize];
        }

        for cur_idx in 0..STATE_COUNT {
            let mut best_score = f64::NEG_INFINITY;
            let mut best_prev = 0_u8;
            for prev_idx in 0..STATE_COUNT {
                if !cell_adj(prev_ends[prev_idx], starts[cur_idx], cell_coords) {
                    continue;
                }
                let candidate = dp_prev[prev_idx] + contribs[cur_idx];
                if candidate > best_score {
                    best_score = candidate;
                    best_prev = prev_idx as u8;
                }
            }
            dp_cur[cur_idx] = best_score;
            parent[k][cur_idx] = best_prev;
        }
        dp_prev = dp_cur;
        dp_cur = [f64::NEG_INFINITY; STATE_COUNT];
    }

    let mut last_state = 0_usize;
    for state_idx in 1..STATE_COUNT {
        if dp_prev[state_idx] > dp_prev[last_state] {
            last_state = state_idx;
        }
    }

    let mut state_seq = vec![0_u8; macro_len];
    state_seq[macro_len - 1] = last_state as u8;
    for k in (1..macro_len).rev() {
        let cur = state_seq[k] as usize;
        state_seq[k - 1] = parent[k][cur];
    }

    let mut path = Vec::with_capacity(macro_len * 4);
    for k in 0..macro_len {
        let cells = &block_cells[macro_path[k] as usize];
        let order = state_order(cells, order_weights, states[state_seq[k] as usize]);
        path.extend_from_slice(&order);
    }

    let mut positions = vec![0_u32; raw_values.len()];
    let mut raw_score = 0_i128;
    for (pos, &cell) in path.iter().enumerate() {
        positions[cell] = pos as u32;
        raw_score += pos as i128 * raw_values[cell] as i128;
    }

    EvalResult {
        raw_score,
        path,
        positions,
    }
}

fn state_order(cells: &[usize; 4], weights: &[f64], state: (u8, u8)) -> [usize; 4] {
    let s = state.0 as usize;
    let t = state.1 as usize;
    let mut mids = [0_usize; 2];
    let mut idx = 0;
    for i in 0..4 {
        if i != s && i != t {
            mids[idx] = i;
            idx += 1;
        }
    }
    let left = mids[0];
    let right = mids[1];
    let order_mid = if weight_before(cells[left], cells[right], weights) {
        [left, right]
    } else {
        [right, left]
    };
    [cells[s], cells[order_mid[0]], cells[order_mid[1]], cells[t]]
}

fn state_contrib(cells: &[usize; 4], weights: &[f64], base: usize, state: (u8, u8)) -> f64 {
    let order = state_order(cells, weights, state);
    (base as f64) * weights[order[0]]
        + ((base + 1) as f64) * weights[order[1]]
        + ((base + 2) as f64) * weights[order[2]]
        + ((base + 3) as f64) * weights[order[3]]
}

fn weight_before(left_cell: usize, right_cell: usize, weights: &[f64]) -> bool {
    let wl = weights[left_cell];
    let wr = weights[right_cell];
    if (wl - wr).abs() > 1e-12 {
        wl <= wr
    } else {
        left_cell <= right_cell
    }
}

fn update_best(result: &EvalResult, best_raw_score: &mut i128, best_path: &mut Vec<usize>) {
    if result.raw_score > *best_raw_score {
        *best_raw_score = result.raw_score;
        *best_path = result.path.clone();
    }
}

fn learn_target(candidates: &[CandidateSummary], rank: &[f64], m: usize) -> Vec<f64> {
    let elite = select_elite(candidates, ELITE_COUNT);
    let mut sum_w = 0.0_f64;
    let mut sum_pos = vec![0.0_f64; m];
    let mut sum_pos_sq = vec![0.0_f64; m];

    let best_score = elite[0].raw_score as f64;
    let worst_score = elite[elite.len() - 1].raw_score as f64;
    let denom = (best_score - worst_score).abs().max(1.0);

    for cand in elite {
        let normalized = ((cand.raw_score as f64) - worst_score) / denom;
        let weight = (SOFTMAX_BETA * normalized).exp();
        sum_w += weight;
        for cell in 0..m {
            let pos = cand.positions[cell] as f64;
            sum_pos[cell] += weight * pos;
            sum_pos_sq[cell] += weight * pos * pos;
        }
    }

    let upper = (m - 1) as f64;
    let mut target = vec![0.0_f64; m];
    for cell in 0..m {
        let mu = sum_pos[cell] / sum_w;
        let var = (sum_pos_sq[cell] / sum_w - mu * mu).max(0.0);
        let sigma = var.sqrt();
        let stability = STABILITY_TAU / (STABILITY_TAU + sigma);
        let corrected = rank[cell] + TARGET_ALPHA * stability * (mu - rank[cell]);
        target[cell] = corrected.clamp(0.0, upper);
    }
    target
}

fn select_elite<'a>(candidates: &'a [CandidateSummary], limit: usize) -> Vec<&'a CandidateSummary> {
    let mut sorted: Vec<&CandidateSummary> = candidates.iter().collect();
    sorted.sort_by(|a, b| b.raw_score.cmp(&a.raw_score));
    let mut elite = Vec::new();
    let mut seen = HashSet::new();
    for cand in sorted {
        if seen.insert(cand.hash) {
            elite.push(cand);
            if elite.len() == limit {
                break;
            }
        }
    }
    elite
}

fn select_seed_paths(candidates: &[CandidateSummary], limit: usize) -> Vec<Vec<u16>> {
    let mut sorted: Vec<&CandidateSummary> = candidates.iter().collect();
    sorted.sort_by(|a, b| b.raw_score.cmp(&a.raw_score));
    let mut seeds = Vec::new();
    let mut seen = HashSet::new();
    for cand in sorted {
        if seen.insert(cand.hash) {
            seeds.push(cand.macro_path.clone());
            if seeds.len() == limit {
                break;
            }
        }
    }
    seeds
}

fn blend_weights(rank: &[f64], target: &[f64], eta: f64) -> Vec<f64> {
    rank.iter()
        .zip(target.iter())
        .map(|(&r, &t)| r + eta * (t - r))
        .collect()
}

fn hash_macro_path(path: &[u16]) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn validate_path(path: &[usize], n: usize, coords: &[(u16, u16)]) {
    assert_eq!(path.len(), n * n);
    let mut used = vec![false; n * n];
    for &cell in path {
        assert!(cell < n * n);
        assert!(!used[cell]);
        used[cell] = true;
    }
    for i in 0..(path.len() - 1) {
        assert!(cell_adj(path[i], path[i + 1], coords));
    }
}
