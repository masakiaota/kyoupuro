use proconio::input;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BLOCK_SIDE: usize = 2;
const BASE_CLUSTER_VARIANTS: usize = 6;
const CORNER_CLUSTER_VARIANTS: usize = 12;
const FINAL_CHILD_VARIANTS: usize = 128;
const LOCAL_SWEEP_PASSES_2: usize = 4;
const LOCAL_SWEEP_PASSES_5: usize = 10;
const ELITE_COUNT: usize = 6;
const ETA_LIST: [f64; 0] = [];
const FINAL_BEAM_WIDTH: usize = 100000;
const SCHEDULES: [[usize; 4]; 1] = [[2, 2, 5, 5]];
const NEG_INF: f64 = -1.0e300;

#[derive(Clone)]
struct CandidateSummary {
    raw_score: i128,
    positions: Vec<u32>,
    hash: u64,
    schedule_idx: usize,
}

struct EvalResult {
    raw_score: i128,
    path: Vec<usize>,
    positions: Vec<u32>,
}

#[derive(Clone)]
struct ClusterVariant {
    path: Vec<usize>,
    score: f64,
    head: usize,
    tail: usize,
}

#[derive(Clone)]
struct Cluster {
    sum_weight: f64,
    variants: Vec<ClusterVariant>,
}

#[derive(Clone, Copy)]
struct ChildOption {
    variant_idx: usize,
    reversed: bool,
    head: usize,
    tail: usize,
    score: f64,
}

#[derive(Clone)]
struct BeamState {
    score: f64,
    mask: u32,
    last_pos: u8,
    last_option: u8,
    prev_idx: usize,
}

fn main() {
    input! {
        n: usize,
        a_grid: [[i64; n]; n],
    }

    assert!(n % BLOCK_SIDE == 0);

    let m = n * n;
    let b = n / BLOCK_SIDE;
    let cell_coords = build_cell_coords(n);
    let raw_values = flatten_values(n, &a_grid);
    let rank: Vec<f64> = raw_values.iter().map(|&v| (v - 1) as f64).collect();
    let local_orders_2 = build_local_orders(2);
    let local_orders_5 = build_local_orders(5);

    let mut best_raw_score = i128::MIN;
    let mut best_path = Vec::new();
    let mut initial_candidates = Vec::new();

    for (schedule_idx, schedule) in SCHEDULES.iter().enumerate() {
        let path = build_hierarchical_path(
            n,
            b,
            &rank,
            &cell_coords,
            &local_orders_2,
            &local_orders_5,
            schedule,
        );
        let result = evaluate_path(&path, &raw_values);
        update_best(&result, &mut best_raw_score, &mut best_path);
        initial_candidates.push(CandidateSummary {
            raw_score: result.raw_score,
            positions: result.positions,
            hash: hash_usize_path(&path),
            schedule_idx,
        });
    }

    let target = learn_target(&initial_candidates, &rank, m);
    let seed_schedules = select_seed_schedules(&initial_candidates, 3);
    for &eta in &ETA_LIST {
        let blended = blend_weights(&rank, &target, eta);
        for &schedule_idx in &seed_schedules {
            let path = build_hierarchical_path(
                n,
                b,
                &blended,
                &cell_coords,
                &local_orders_2,
                &local_orders_5,
                &SCHEDULES[schedule_idx],
            );
            let result = evaluate_path(&path, &raw_values);
            update_best(&result, &mut best_raw_score, &mut best_path);
        }
    }

    validate_path(&best_path, n, &cell_coords);

    if std::env::var_os("V301_DEBUG").is_some() {
        eprintln!(
            "initial_candidates={} seeds={} best_raw={}",
            initial_candidates.len(),
            seed_schedules.len(),
            best_raw_score
        );
    }

    for &cell in &best_path {
        let (r, c) = cell_coords[cell];
        println!("{} {}", r, c);
    }
}

fn build_hierarchical_path(
    n: usize,
    b: usize,
    weights: &[f64],
    cell_coords: &[(u16, u16)],
    local_orders_2: &[Vec<u8>],
    local_orders_5: &[Vec<u8>],
    schedule: &[usize; 4],
) -> Vec<usize> {
    let mut clusters = build_base_clusters(n, b, weights);
    let mut side = b;
    for &factor in schedule {
        assert_eq!(side % factor, 0);
        clusters = merge_clusters(
            n,
            side,
            factor,
            &clusters,
            cell_coords,
            if factor == 2 {
                local_orders_2
            } else {
                local_orders_5
            },
            if factor == 2 {
                LOCAL_SWEEP_PASSES_2
            } else {
                LOCAL_SWEEP_PASSES_5
            },
        );
        side /= factor;
    }
    assert_eq!(side, 1);
    assert_eq!(clusters.len(), 1);
    clusters.pop().unwrap().variants.remove(0).path
}

fn build_base_clusters(n: usize, b: usize, weights: &[f64]) -> Vec<Cluster> {
    let mut clusters = Vec::with_capacity(b * b);
    for br in 0..b {
        for bc in 0..b {
            let r = br * BLOCK_SIDE;
            let c = bc * BLOCK_SIDE;
            let cells = [
                cell_id(r, c, n),
                cell_id(r, c + 1, n),
                cell_id(r + 1, c, n),
                cell_id(r + 1, c + 1, n),
            ];
            let sum_weight =
                weights[cells[0]] + weights[cells[1]] + weights[cells[2]] + weights[cells[3]];
            let mut variants = Vec::with_capacity(BASE_CLUSTER_VARIANTS);
            for s in 0..4 {
                for t in (s + 1)..4 {
                    let forward = build_base_variant(&cells, weights, s, t);
                    let backward = build_base_variant(&cells, weights, t, s);
                    if forward.score >= backward.score {
                        variants.push(forward);
                    } else {
                        variants.push(backward);
                    }
                }
            }
            variants.sort_by(|a, b| b.score.total_cmp(&a.score));
            clusters.push(Cluster {
                sum_weight,
                variants,
            });
        }
    }
    clusters
}

fn merge_clusters(
    n: usize,
    side: usize,
    factor: usize,
    clusters: &[Cluster],
    cell_coords: &[(u16, u16)],
    local_orders: &[Vec<u8>],
    sweep_passes: usize,
) -> Vec<Cluster> {
    let next_side = side / factor;
    let local_coords = build_local_coords(factor);
    let mut next = Vec::with_capacity(next_side * next_side);
    let keep_wide_boundary = next_side == 5;

    for gr in 0..next_side {
        for gc in 0..next_side {
            let mut tile_children = Vec::with_capacity(factor * factor);
            for lr in 0..factor {
                for lc in 0..factor {
                    let idx = (gr * factor + lr) * side + (gc * factor + lc);
                    tile_children.push(&clusters[idx]);
                }
            }
            let child_span = int_sqrt_exact(tile_children[0].variants[0].path.len());
            let merged_span = child_span * factor;
            let top = gr * merged_span;
            let left = gc * merged_span;
            let corners = [
                cell_id(top, left, n),
                cell_id(top, left + merged_span - 1, n),
                cell_id(top + merged_span - 1, left, n),
                cell_id(top + merged_span - 1, left + merged_span - 1, n),
            ];
            next.push(merge_tile(
                &tile_children,
                &local_coords,
                local_orders,
                sweep_passes,
                cell_coords,
                side,
                factor,
                gr,
                gc,
                &corners,
                keep_wide_boundary,
                top,
                left,
                merged_span,
            ));
        }
    }

    next
}

fn merge_tile(
    children: &[&Cluster],
    local_coords: &[(i16, i16)],
    local_orders: &[Vec<u8>],
    sweep_passes: usize,
    cell_coords: &[(u16, u16)],
    side: usize,
    factor: usize,
    gr: usize,
    gc: usize,
    corners: &[usize; 4],
    keep_wide_boundary: bool,
    top: usize,
    left: usize,
    span: usize,
) -> Cluster {
    if factor == 5 && side == 5 {
        return beam_merge_final(children, cell_coords);
    }

    let sum_weight = children.iter().map(|child| child.sum_weight).sum();
    let child_weights: Vec<f64> = children.iter().map(|child| child.sum_weight).collect();
    let child_options: Vec<Vec<ChildOption>> = children
        .iter()
        .map(|child| build_child_options(child))
        .collect();

    let mut candidates = Vec::new();
    let mut seen_orders = HashSet::new();
    for base_order in local_orders {
        let base = base_order.clone();
        consider_local_order(
            base,
            children,
            &child_options,
            &child_weights,
            local_coords,
            sweep_passes,
            cell_coords,
            &mut seen_orders,
            &mut candidates,
        );
        let mut rev = base_order.clone();
        rev.reverse();
        consider_local_order(
            rev,
            children,
            &child_options,
            &child_weights,
            local_coords,
            sweep_passes,
            cell_coords,
            &mut seen_orders,
            &mut candidates,
        );
    }

    if candidates.is_empty() {
        panic!(
            "merge_tile produced no feasible variant: side={} factor={} tile=({}, {}) child_len={} child_variants={}",
            side,
            factor,
            gr,
            gc,
            children[0].variants[0].path.len(),
            children[0].variants.len()
        );
    }

    let variants = if keep_wide_boundary {
        compress_variants_boundary(
            candidates,
            cell_coords,
            top,
            left,
            span,
            FINAL_CHILD_VARIANTS,
        )
    } else {
        compress_variants(candidates, corners)
    };
    Cluster {
        sum_weight,
        variants,
    }
}

fn beam_merge_final(children: &[&Cluster], cell_coords: &[(u16, u16)]) -> Cluster {
    let sum_weight = children.iter().map(|child| child.sum_weight).sum();
    let child_options: Vec<Vec<ChildOption>> = children
        .iter()
        .map(|child| build_child_options(child))
        .collect();
    let child_len = children[0].variants[0].path.len() as f64;
    let neighbors = build_local_neighbors_king(5);

    let mut layers: Vec<Vec<BeamState>> = Vec::with_capacity(children.len());
    let mut initial = Vec::new();
    for pos in 0..children.len() {
        for option_idx in 0..child_options[pos].len() {
            initial.push(BeamState {
                score: child_options[pos][option_idx].score,
                mask: 1u32 << pos,
                last_pos: pos as u8,
                last_option: option_idx as u8,
                prev_idx: usize::MAX,
            });
        }
    }
    initial.sort_by(|a, b| b.score.total_cmp(&a.score));
    if initial.len() > FINAL_BEAM_WIDTH {
        initial.truncate(FINAL_BEAM_WIDTH);
    }
    layers.push(initial);

    for step in 1..children.len() {
        let prev_layer = &layers[step - 1];
        let mut next_layer = Vec::new();
        for (state_idx, state) in prev_layer.iter().enumerate() {
            let last_pos = state.last_pos as usize;
            let last_opt = child_options[last_pos][state.last_option as usize];
            for &next_pos in &neighbors[last_pos] {
                if (state.mask & (1u32 << next_pos)) != 0 {
                    continue;
                }
                let base_offset = step as f64 * child_len;
                for (option_idx, option) in child_options[next_pos].iter().enumerate() {
                    if !cell_adj(last_opt.tail, option.head, cell_coords) {
                        continue;
                    }
                    next_layer.push(BeamState {
                        score: state.score
                            + option.score
                            + base_offset * children[next_pos].sum_weight,
                        mask: state.mask | (1u32 << next_pos),
                        last_pos: next_pos as u8,
                        last_option: option_idx as u8,
                        prev_idx: state_idx,
                    });
                }
            }
        }
        next_layer.sort_by(|a, b| b.score.total_cmp(&a.score));
        next_layer = dedup_beam_layer(next_layer, &child_options);
        if next_layer.len() > FINAL_BEAM_WIDTH {
            next_layer.truncate(FINAL_BEAM_WIDTH);
        }
        assert!(
            !next_layer.is_empty(),
            "final beam exhausted at step {}",
            step
        );
        layers.push(next_layer);
    }

    let final_layer = layers.last().unwrap();
    let best_idx = 0usize;
    let mut chosen_pos = vec![0usize; children.len()];
    let mut chosen_option = vec![0usize; children.len()];
    let mut state_idx = best_idx;
    for step in (0..children.len()).rev() {
        let state = &layers[step][state_idx];
        chosen_pos[step] = state.last_pos as usize;
        chosen_option[step] = state.last_option as usize;
        if step > 0 {
            state_idx = state.prev_idx;
        }
    }

    let total_len: usize = children
        .iter()
        .map(|child| child.variants[0].path.len())
        .sum();
    let mut path = Vec::with_capacity(total_len);
    for step in 0..children.len() {
        let pos = chosen_pos[step];
        let option = child_options[pos][chosen_option[step]];
        let variant = &children[pos].variants[option.variant_idx];
        if option.reversed {
            for &cell in variant.path.iter().rev() {
                path.push(cell);
            }
        } else {
            path.extend_from_slice(&variant.path);
        }
    }

    let best = &final_layer[best_idx];
    Cluster {
        sum_weight,
        variants: vec![ClusterVariant {
            head: path[0],
            tail: *path.last().unwrap(),
            score: best.score,
            path,
        }],
    }
}

fn build_local_neighbors_king(side: usize) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); side * side];
    for r in 0..side {
        for c in 0..side {
            let idx = r * side + c;
            for dr in -1isize..=1 {
                for dc in -1isize..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= side as isize || nc < 0 || nc >= side as isize {
                        continue;
                    }
                    neighbors[idx].push(nr as usize * side + nc as usize);
                }
            }
        }
    }
    neighbors
}

fn dedup_beam_layer(
    mut layer: Vec<BeamState>,
    child_options: &[Vec<ChildOption>],
) -> Vec<BeamState> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for state in layer.drain(..) {
        let opt = child_options[state.last_pos as usize][state.last_option as usize];
        if seen.insert((state.mask, state.last_pos, opt.tail)) {
            out.push(state);
        }
    }
    out
}

fn consider_local_order(
    mut order: Vec<u8>,
    children: &[&Cluster],
    child_options: &[Vec<ChildOption>],
    child_weights: &[f64],
    local_coords: &[(i16, i16)],
    sweep_passes: usize,
    cell_coords: &[(u16, u16)],
    seen_orders: &mut HashSet<u64>,
    candidates: &mut Vec<ClusterVariant>,
) {
    for improved in [false, true] {
        if improved {
            optimize_local_adj_swaps(&mut order, local_coords, child_weights, sweep_passes);
        }
        let hash = hash_u8_path(&order);
        if !seen_orders.insert(hash) {
            continue;
        }
        candidates.extend(evaluate_child_order(
            &order,
            children,
            child_options,
            cell_coords,
        ));
    }
}

fn build_child_options(cluster: &Cluster) -> Vec<ChildOption> {
    let mut options = Vec::with_capacity(cluster.variants.len() * 2);
    for (variant_idx, variant) in cluster.variants.iter().enumerate() {
        let len = variant.path.len() as f64;
        options.push(ChildOption {
            variant_idx,
            reversed: false,
            head: variant.head,
            tail: variant.tail,
            score: variant.score,
        });
        options.push(ChildOption {
            variant_idx,
            reversed: true,
            head: variant.tail,
            tail: variant.head,
            score: (len - 1.0) * cluster.sum_weight - variant.score,
        });
    }
    options
}

fn evaluate_child_order(
    order: &[u8],
    children: &[&Cluster],
    child_options: &[Vec<ChildOption>],
    cell_coords: &[(u16, u16)],
) -> Vec<ClusterVariant> {
    let q = order.len();
    let first_child = order[0] as usize;
    let first_options = &child_options[first_child];
    let first_count = first_options.len();

    let mut parents: Vec<Vec<Vec<u16>>> = Vec::with_capacity(q);
    parents.push(Vec::new());

    let mut dp_prev = vec![vec![NEG_INF; first_count]; first_count];
    for first_idx in 0..first_count {
        dp_prev[first_idx][first_idx] = first_options[first_idx].score;
    }

    for k in 1..q {
        let prev_child = order[k - 1] as usize;
        let cur_child = order[k] as usize;
        let prev_options = &child_options[prev_child];
        let cur_options = &child_options[cur_child];
        let base_offset = (k * children[cur_child].variants[0].path.len()) as f64;

        let mut dp_cur = vec![vec![NEG_INF; cur_options.len()]; first_count];
        let mut parent_k = vec![vec![u16::MAX; cur_options.len()]; first_count];

        for first_idx in 0..first_count {
            for cur_idx in 0..cur_options.len() {
                let contrib =
                    cur_options[cur_idx].score + base_offset * children[cur_child].sum_weight;
                let mut best_score = NEG_INF;
                let mut best_prev = u16::MAX;
                for prev_idx in 0..prev_options.len() {
                    let prev_score = dp_prev[first_idx][prev_idx];
                    if prev_score <= NEG_INF / 2.0 {
                        continue;
                    }
                    if !cell_adj(
                        prev_options[prev_idx].tail,
                        cur_options[cur_idx].head,
                        cell_coords,
                    ) {
                        continue;
                    }
                    let candidate = prev_score + contrib;
                    if candidate > best_score {
                        best_score = candidate;
                        best_prev = prev_idx as u16;
                    }
                }
                dp_cur[first_idx][cur_idx] = best_score;
                parent_k[first_idx][cur_idx] = best_prev;
            }
        }

        parents.push(parent_k);
        dp_prev = dp_cur;
    }

    let last_child = order[q - 1] as usize;
    let last_options = &child_options[last_child];
    let mut variants = Vec::new();

    for first_idx in 0..first_count {
        for last_idx in 0..last_options.len() {
            let final_score = dp_prev[first_idx][last_idx];
            if final_score <= NEG_INF / 2.0 {
                continue;
            }

            let mut selected = vec![0usize; q];
            selected[q - 1] = last_idx;
            let mut valid = true;
            for k in (1..q).rev() {
                let prev_idx = parents[k][first_idx][selected[k]];
                if prev_idx == u16::MAX {
                    valid = false;
                    break;
                }
                selected[k - 1] = prev_idx as usize;
            }
            if !valid {
                continue;
            }
            selected[0] = first_idx;

            let total_len: usize = order
                .iter()
                .map(|&child_idx| children[child_idx as usize].variants[0].path.len())
                .sum();
            let mut path = Vec::with_capacity(total_len);
            for (k, &child_idx) in order.iter().enumerate() {
                let option = child_options[child_idx as usize][selected[k]];
                let variant = &children[child_idx as usize].variants[option.variant_idx];
                if option.reversed {
                    for &cell in variant.path.iter().rev() {
                        path.push(cell);
                    }
                } else {
                    path.extend_from_slice(&variant.path);
                }
            }

            variants.push(ClusterVariant {
                head: first_options[first_idx].head,
                tail: last_options[last_idx].tail,
                score: final_score,
                path,
            });
        }
    }

    variants
}

fn compress_variants(
    mut variants: Vec<ClusterVariant>,
    corners: &[usize; 4],
) -> Vec<ClusterVariant> {
    variants.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut best_by_corner_pair: Vec<Option<ClusterVariant>> = vec![None; 16];
    for variant in &variants {
        let Some(head_idx) = corners.iter().position(|&cell| cell == variant.head) else {
            continue;
        };
        let Some(tail_idx) = corners.iter().position(|&cell| cell == variant.tail) else {
            continue;
        };
        if head_idx == tail_idx {
            continue;
        }
        let slot = head_idx * 4 + tail_idx;
        if best_by_corner_pair[slot]
            .as_ref()
            .is_none_or(|current| variant.score > current.score)
        {
            best_by_corner_pair[slot] = Some(variant.clone());
        }
    }

    let mut selected = Vec::new();
    let mut seen_ends = HashSet::new();
    let mut used_pairs = HashSet::new();

    for variant in best_by_corner_pair.into_iter().flatten() {
        if selected.len() == CORNER_CLUSTER_VARIANTS {
            break;
        }
        seen_ends.insert(variant.head);
        seen_ends.insert(variant.tail);
        used_pairs.insert((variant.head, variant.tail));
        selected.push(variant);
    }

    for variant in &variants {
        if selected.len() == CORNER_CLUSTER_VARIANTS {
            break;
        }
        if seen_ends.insert(variant.head) || seen_ends.insert(variant.tail) {
            used_pairs.insert((variant.head, variant.tail));
            selected.push(variant.clone());
        }
    }
    for variant in variants {
        if selected.len() == CORNER_CLUSTER_VARIANTS {
            break;
        }
        if used_pairs.insert((variant.head, variant.tail)) {
            selected.push(variant);
        }
    }
    selected.sort_by(|a, b| b.score.total_cmp(&a.score));
    selected
}

fn compress_variants_general(
    mut variants: Vec<ClusterVariant>,
    limit: usize,
) -> Vec<ClusterVariant> {
    variants.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut selected = Vec::new();
    let mut seen_ends = HashSet::new();
    let mut used_pairs = HashSet::new();

    for variant in &variants {
        if selected.len() == limit {
            break;
        }
        if seen_ends.insert(variant.head) || seen_ends.insert(variant.tail) {
            used_pairs.insert((variant.head, variant.tail));
            selected.push(variant.clone());
        }
    }
    for variant in variants {
        if selected.len() == limit {
            break;
        }
        if used_pairs.insert((variant.head, variant.tail)) {
            selected.push(variant);
        }
    }
    selected.sort_by(|a, b| b.score.total_cmp(&a.score));
    selected
}

fn compress_variants_boundary(
    mut variants: Vec<ClusterVariant>,
    cell_coords: &[(u16, u16)],
    top: usize,
    left: usize,
    span: usize,
    limit: usize,
) -> Vec<ClusterVariant> {
    variants.sort_by(|a, b| b.score.total_cmp(&a.score));
    let bottom = top + span - 1;
    let right = left + span - 1;

    let mut selected = Vec::new();
    let mut seen_side_pairs = HashSet::new();
    let mut used_pairs = HashSet::new();

    for variant in &variants {
        let head_mask = boundary_mask(variant.head, cell_coords, top, bottom, left, right);
        let tail_mask = boundary_mask(variant.tail, cell_coords, top, bottom, left, right);
        if head_mask == 0 || tail_mask == 0 {
            continue;
        }
        let mut inserted = false;
        for hs in 0..4 {
            if (head_mask & (1 << hs)) == 0 {
                continue;
            }
            for ts in 0..4 {
                if (tail_mask & (1 << ts)) == 0 {
                    continue;
                }
                if seen_side_pairs.insert((hs, ts)) {
                    used_pairs.insert((variant.head, variant.tail));
                    selected.push(variant.clone());
                    inserted = true;
                    break;
                }
            }
            if inserted {
                break;
            }
        }
        if selected.len() == limit {
            break;
        }
    }

    for variant in &variants {
        if selected.len() == limit {
            break;
        }
        let head_mask = boundary_mask(variant.head, cell_coords, top, bottom, left, right);
        let tail_mask = boundary_mask(variant.tail, cell_coords, top, bottom, left, right);
        if head_mask == 0 || tail_mask == 0 {
            continue;
        }
        if used_pairs.insert((variant.head, variant.tail)) {
            selected.push(variant.clone());
        }
    }

    if selected.len() < limit {
        let remainder = compress_variants_general(variants, limit - selected.len());
        for variant in remainder {
            if selected.len() == limit {
                break;
            }
            if used_pairs.insert((variant.head, variant.tail)) {
                selected.push(variant);
            }
        }
    }

    selected.sort_by(|a, b| b.score.total_cmp(&a.score));
    selected
}

fn build_local_orders(side: usize) -> Vec<Vec<u8>> {
    if side == 2 {
        return build_all_permutations(4);
    }

    let mut orders = Vec::new();
    let mut seen = HashSet::new();

    let mut base_patterns = vec![
        row_snake_coords(side),
        diag_snake_coords(side),
        spiral_coords(side),
    ];
    if side % 2 == 0 {
        base_patterns.push(pair_weave_coords(side));
    }

    for base in base_patterns {
        for sym in 0..8 {
            let mut order = Vec::with_capacity(side * side);
            for &(r, c) in &base {
                let (tr, tc) = apply_symmetry(r, c, side, sym);
                order.push((tr * side + tc) as u8);
            }
            let hash = hash_u8_path(&order);
            if seen.insert(hash) {
                orders.push(order);
            }
        }
    }

    orders
}

fn build_all_permutations(len: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur = Vec::with_capacity(len);
    let mut used = vec![false; len];
    dfs_permutations(len, &mut used, &mut cur, &mut out);
    out
}

fn dfs_permutations(len: usize, used: &mut [bool], cur: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
    if cur.len() == len {
        out.push(cur.clone());
        return;
    }
    for v in 0..len {
        if used[v] {
            continue;
        }
        used[v] = true;
        cur.push(v as u8);
        dfs_permutations(len, used, cur, out);
        cur.pop();
        used[v] = false;
    }
}

fn build_local_coords(side: usize) -> Vec<(i16, i16)> {
    let mut coords = Vec::with_capacity(side * side);
    for r in 0..side {
        for c in 0..side {
            coords.push((r as i16, c as i16));
        }
    }
    coords
}

fn row_snake_coords(side: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(side * side);
    for r in 0..side {
        if r % 2 == 0 {
            for c in 0..side {
                path.push((r, c));
            }
        } else {
            for c in (0..side).rev() {
                path.push((r, c));
            }
        }
    }
    path
}

fn pair_weave_coords(side: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(side * side);
    for pair in 0..(side / 2) {
        let r0 = pair * 2;
        let r1 = r0 + 1;
        if pair % 2 == 0 {
            for c in 0..side {
                path.push((r0, c));
                path.push((r1, c));
            }
        } else {
            for c in (0..side).rev() {
                path.push((r0, c));
                path.push((r1, c));
            }
        }
    }
    path
}

fn diag_snake_coords(side: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(side * side);
    for s in 0..=(2 * (side - 1)) {
        let r_min = s.saturating_sub(side - 1);
        let r_max = s.min(side - 1);
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

fn spiral_coords(side: usize) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(side * side);
    let mut top = 0usize;
    let mut bottom = side - 1;
    let mut left = 0usize;
    let mut right = side - 1;

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

fn apply_symmetry(r: usize, c: usize, side: usize, sym: usize) -> (usize, usize) {
    match sym {
        0 => (r, c),
        1 => (c, side - 1 - r),
        2 => (side - 1 - r, side - 1 - c),
        3 => (side - 1 - c, r),
        4 => (r, side - 1 - c),
        5 => (side - 1 - r, c),
        6 => (c, r),
        7 => (side - 1 - c, side - 1 - r),
        _ => unreachable!(),
    }
}

fn optimize_local_adj_swaps(
    path: &mut [u8],
    coords: &[(i16, i16)],
    weights: &[f64],
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
            if weights[left] <= weights[right] {
                continue;
            }
            if i > 0 {
                let prev = path[i - 1] as usize;
                if !coords_adj(coords[prev], coords[right]) {
                    continue;
                }
            }
            if i + 2 < path.len() {
                let next = path[i + 2] as usize;
                if !coords_adj(coords[left], coords[next]) {
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

fn evaluate_path(path: &[usize], raw_values: &[i64]) -> EvalResult {
    let mut positions = vec![0u32; raw_values.len()];
    let mut raw_score = 0i128;
    for (pos, &cell) in path.iter().enumerate() {
        positions[cell] = pos as u32;
        raw_score += pos as i128 * raw_values[cell] as i128;
    }
    EvalResult {
        raw_score,
        path: path.to_vec(),
        positions,
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
    let mut sum_w = 0.0f64;
    let mut sum_pos = vec![0.0f64; m];

    let best_score = elite[0].raw_score as f64;
    let worst_score = elite[elite.len() - 1].raw_score as f64;
    let denom = (best_score - worst_score).abs().max(1.0);

    for cand in elite {
        let normalized = ((cand.raw_score as f64) - worst_score) / denom;
        let weight = (4.0 * normalized).exp();
        sum_w += weight;
        for cell in 0..m {
            sum_pos[cell] += weight * cand.positions[cell] as f64;
        }
    }

    let upper = (m - 1) as f64;
    let mut target = vec![0.0f64; m];
    for cell in 0..m {
        let mu = sum_pos[cell] / sum_w;
        target[cell] = (0.35 * rank[cell] + 0.65 * mu).clamp(0.0, upper);
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
            if elite.len() == limit.min(candidates.len()) {
                break;
            }
        }
    }
    elite
}

fn select_seed_schedules(candidates: &[CandidateSummary], limit: usize) -> Vec<usize> {
    let mut sorted: Vec<&CandidateSummary> = candidates.iter().collect();
    sorted.sort_by(|a, b| b.raw_score.cmp(&a.raw_score));
    let mut seeds = Vec::new();
    let mut seen = HashSet::new();
    for cand in sorted {
        if seen.insert(cand.schedule_idx) {
            seeds.push(cand.schedule_idx);
            if seeds.len() == limit.min(SCHEDULES.len()) {
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

fn flatten_values(n: usize, a_grid: &[Vec<i64>]) -> Vec<i64> {
    let mut values = vec![0i64; n * n];
    for r in 0..n {
        for c in 0..n {
            values[cell_id(r, c, n)] = a_grid[r][c];
        }
    }
    values
}

fn build_cell_coords(n: usize) -> Vec<(u16, u16)> {
    let mut coords = vec![(0u16, 0u16); n * n];
    for r in 0..n {
        for c in 0..n {
            coords[cell_id(r, c, n)] = (r as u16, c as u16);
        }
    }
    coords
}

fn int_sqrt_exact(x: usize) -> usize {
    let s = (x as f64).sqrt() as usize;
    assert_eq!(s * s, x);
    s
}

fn boundary_mask(
    cell: usize,
    cell_coords: &[(u16, u16)],
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
) -> u8 {
    let (r, c) = cell_coords[cell];
    let r = r as usize;
    let c = c as usize;
    let mut mask = 0u8;
    if r == top {
        mask |= 1 << 0;
    }
    if r == bottom {
        mask |= 1 << 1;
    }
    if c == left {
        mask |= 1 << 2;
    }
    if c == right {
        mask |= 1 << 3;
    }
    mask
}

fn build_base_variant(cells: &[usize; 4], weights: &[f64], s: usize, t: usize) -> ClusterVariant {
    let mut mids = [0usize; 2];
    let mut idx = 0;
    for i in 0..4 {
        if i != s && i != t {
            mids[idx] = i;
            idx += 1;
        }
    }
    let mid_order = if weight_before(cells[mids[0]], cells[mids[1]], weights) {
        [mids[0], mids[1]]
    } else {
        [mids[1], mids[0]]
    };
    let path = vec![cells[s], cells[mid_order[0]], cells[mid_order[1]], cells[t]];
    let score = score_path(&path, weights);
    ClusterVariant {
        head: path[0],
        tail: path[3],
        score,
        path,
    }
}

fn score_path(path: &[usize], weights: &[f64]) -> f64 {
    let mut score = 0.0f64;
    for (idx, &cell) in path.iter().enumerate() {
        score += idx as f64 * weights[cell];
    }
    score
}

fn weight_before(left: usize, right: usize, weights: &[f64]) -> bool {
    let wl = weights[left];
    let wr = weights[right];
    if (wl - wr).abs() > 1e-12 {
        wl <= wr
    } else {
        left <= right
    }
}

fn cell_id(r: usize, c: usize, n: usize) -> usize {
    r * n + c
}

fn coords_adj(a: (i16, i16), b: (i16, i16)) -> bool {
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

fn hash_u8_path(path: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn hash_usize_path(path: &[usize]) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
