use proconio::input;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BLOCK_SIDE: usize = 8;
const BASE_CLUSTER_VARIANTS: usize = 6;
const PER_ORDER_VARIANTS: usize = 24;
const INTERMEDIATE_VARIANTS: usize = 24;
const BASE_PATH_SWEEP_PASSES: usize = 10;
const LOCAL_SWEEP_PASSES_5: usize = 10;
const FINAL_BEAM_WIDTH: usize = 6000;
const ETA_LIST: [f64; 2] = [0.50, 0.90];
const NEG_INF: f64 = -1.0e300;

#[derive(Clone)]
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

#[derive(Clone, Copy)]
struct BeamState {
    score: f64,
    mask: u32,
    last_pos: u8,
    last_option: u8,
    prev_idx: usize,
}

#[derive(Clone, Copy)]
struct VariantSpec {
    first_idx: u16,
    last_idx: u16,
    score: f64,
    head: usize,
    tail: usize,
}

fn main() {
    input! {
        n: usize,
        a_grid: [[i64; n]; n],
    }

    assert!(n % BLOCK_SIDE == 0);
    let b = n / BLOCK_SIDE;
    assert_eq!(b, 25);

    let raw_values = flatten_values(n, &a_grid);
    let rank: Vec<f64> = raw_values.iter().map(|&v| (v - 1) as f64).collect();
    let cell_coords = build_cell_coords(n);
    let local_orders_5 = build_local_orders(5);

    let mut best = evaluate_path(
        &build_hierarchical_path(n, b, &rank, &cell_coords, &local_orders_5),
        &raw_values,
    );
    let mut target = positions_to_target(&best.positions);

    for &eta in &ETA_LIST {
        let blended = blend_weights(&rank, &target, eta);
        let candidate = evaluate_path(
            &build_hierarchical_path(n, b, &blended, &cell_coords, &local_orders_5),
            &raw_values,
        );
        if candidate.raw_score > best.raw_score {
            best = candidate;
            target = positions_to_target(&best.positions);
        }
    }

    if std::env::var_os("V302_DEBUG").is_some() {
        eprintln!(
            "block_side={} stage_variants={} final_beam={} raw_score={}",
            BLOCK_SIDE, INTERMEDIATE_VARIANTS, FINAL_BEAM_WIDTH, best.raw_score
        );
    }

    for &cell in &best.path {
        let (r, c) = cell_coords[cell];
        println!("{} {}", r, c);
    }
}

fn build_hierarchical_path(
    n: usize,
    b: usize,
    weights: &[f64],
    cell_coords: &[(u16, u16)],
    local_orders_5: &[Vec<u8>],
) -> Vec<usize> {
    let base_clusters = build_base_clusters(n, b, weights);
    let mid_clusters = merge_stage_5(n, b, &base_clusters, cell_coords, local_orders_5);
    assert_eq!(mid_clusters.len(), 25);

    let child_refs: Vec<&Cluster> = mid_clusters.iter().collect();
    let final_cluster = beam_merge_final(&child_refs, cell_coords);
    let path = final_cluster.variants[0].path.clone();
    validate_path(&path, n, cell_coords);
    path
}

fn build_base_clusters(n: usize, b: usize, weights: &[f64]) -> Vec<Cluster> {
    let local_coords = build_local_coords(BLOCK_SIDE);
    let templates = build_base_templates(BLOCK_SIDE);
    let mut clusters = Vec::with_capacity(b * b);

    for br in 0..b {
        for bc in 0..b {
            let top = br * BLOCK_SIDE;
            let left = bc * BLOCK_SIDE;
            let block_cells = build_block_cells(top, left, n, BLOCK_SIDE);
            let local_weights: Vec<f64> = block_cells.iter().map(|&cell| weights[cell]).collect();
            let sum_weight = local_weights.iter().sum();

            let mut best_by_pair: Vec<Option<ClusterVariant>> = vec![None; BASE_CLUSTER_VARIANTS];
            for template in &templates {
                let mut path = template.clone();
                optimize_local_adj_swaps_fixed_ends(
                    &mut path,
                    &local_coords,
                    &local_weights,
                    BASE_PATH_SWEEP_PASSES,
                );
                consider_base_template(
                    &path,
                    &block_cells,
                    &local_weights,
                    sum_weight,
                    &mut best_by_pair,
                );
            }

            let mut variants = Vec::with_capacity(BASE_CLUSTER_VARIANTS);
            for entry in best_by_pair {
                if let Some(variant) = entry {
                    variants.push(variant);
                }
            }
            assert_eq!(
                variants.len(),
                BASE_CLUSTER_VARIANTS,
                "missing 8x8 base variants at block ({}, {})",
                br,
                bc
            );
            variants.sort_by(|a, b| b.score.total_cmp(&a.score));
            clusters.push(Cluster {
                sum_weight,
                variants,
            });
        }
    }

    clusters
}

fn consider_base_template(
    local_path: &[u8],
    block_cells: &[usize],
    local_weights: &[f64],
    sum_weight: f64,
    best_by_pair: &mut [Option<ClusterVariant>],
) {
    let head = local_path[0] as usize;
    let tail = local_path[local_path.len() - 1] as usize;
    let Some(slot) = corner_pair_slot(head, tail, BLOCK_SIDE) else {
        return;
    };

    let forward_score = score_local_path(local_path, local_weights);
    let reverse_score = (local_path.len() as f64 - 1.0) * sum_weight - forward_score;

    let mut path = Vec::with_capacity(local_path.len());
    let score = if reverse_score > forward_score {
        for &idx in local_path.iter().rev() {
            path.push(block_cells[idx as usize]);
        }
        reverse_score
    } else {
        for &idx in local_path {
            path.push(block_cells[idx as usize]);
        }
        forward_score
    };

    let variant = ClusterVariant {
        head: path[0],
        tail: path[path.len() - 1],
        score,
        path,
    };

    let replace = match &best_by_pair[slot] {
        None => true,
        Some(current) => variant.score > current.score,
    };
    if replace {
        best_by_pair[slot] = Some(variant);
    }
}

fn merge_stage_5(
    n: usize,
    side: usize,
    clusters: &[Cluster],
    cell_coords: &[(u16, u16)],
    local_orders_5: &[Vec<u8>],
) -> Vec<Cluster> {
    assert_eq!(side % 5, 0);
    let next_side = side / 5;
    let mut next = Vec::with_capacity(next_side * next_side);

    for gr in 0..next_side {
        for gc in 0..next_side {
            let mut tile_children = Vec::with_capacity(25);
            for lr in 0..5 {
                for lc in 0..5 {
                    let idx = (gr * 5 + lr) * side + (gc * 5 + lc);
                    tile_children.push(&clusters[idx]);
                }
            }

            let child_span = int_sqrt_exact(tile_children[0].variants[0].path.len());
            let span = child_span * 5;
            let top = gr * span;
            let left = gc * span;
            next.push(merge_tile(
                n,
                &tile_children,
                cell_coords,
                local_orders_5,
                top,
                left,
                span,
            ));
        }
    }

    next
}

fn merge_tile(
    _n: usize,
    children: &[&Cluster],
    cell_coords: &[(u16, u16)],
    local_orders_5: &[Vec<u8>],
    top: usize,
    left: usize,
    span: usize,
) -> Cluster {
    let local_coords = build_local_coords(5);
    let sum_weight = children.iter().map(|child| child.sum_weight).sum();
    let child_weights: Vec<f64> = children.iter().map(|child| child.sum_weight).collect();
    let child_options: Vec<Vec<ChildOption>> = children
        .iter()
        .map(|child| build_child_options(child))
        .collect();

    let mut candidates = Vec::new();
    let mut seen_orders = HashSet::new();
    for base_order in local_orders_5 {
        if seen_orders.insert(hash_u8_path(base_order)) {
            candidates.extend(evaluate_child_order(
                base_order,
                children,
                &child_options,
                cell_coords,
                top,
                left,
                span,
            ));
        }

        let mut improved = base_order.clone();
        optimize_local_adj_swaps(
            &mut improved,
            &local_coords,
            &child_weights,
            LOCAL_SWEEP_PASSES_5,
        );
        if seen_orders.insert(hash_u8_path(&improved)) {
            candidates.extend(evaluate_child_order(
                &improved,
                children,
                &child_options,
                cell_coords,
                top,
                left,
                span,
            ));
        }
    }

    assert!(
        !candidates.is_empty(),
        "merge_tile produced no candidates for top={} left={} span={}",
        top,
        left,
        span
    );

    Cluster {
        sum_weight,
        variants: compress_variants_boundary(
            candidates,
            cell_coords,
            top,
            left,
            span,
            INTERMEDIATE_VARIANTS,
        ),
    }
}

fn evaluate_child_order(
    order: &[u8],
    children: &[&Cluster],
    child_options: &[Vec<ChildOption>],
    cell_coords: &[(u16, u16)],
    top: usize,
    left: usize,
    span: usize,
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
    let mut specs = Vec::new();
    for first_idx in 0..first_count {
        for last_idx in 0..last_options.len() {
            let final_score = dp_prev[first_idx][last_idx];
            if final_score <= NEG_INF / 2.0 {
                continue;
            }
            specs.push(VariantSpec {
                first_idx: first_idx as u16,
                last_idx: last_idx as u16,
                score: final_score,
                head: first_options[first_idx].head,
                tail: last_options[last_idx].tail,
            });
        }
    }

    let selected_specs =
        select_variant_specs_boundary(specs, cell_coords, top, left, span, PER_ORDER_VARIANTS);
    let mut variants = Vec::with_capacity(selected_specs.len());
    for spec in selected_specs {
        if let Some(variant) = reconstruct_cluster_variant(
            order,
            children,
            child_options,
            &parents,
            spec.first_idx as usize,
            spec.last_idx as usize,
            spec.score,
        ) {
            variants.push(variant);
        }
    }
    variants
}

fn reconstruct_cluster_variant(
    order: &[u8],
    children: &[&Cluster],
    child_options: &[Vec<ChildOption>],
    parents: &[Vec<Vec<u16>>],
    first_idx: usize,
    last_idx: usize,
    score: f64,
) -> Option<ClusterVariant> {
    let q = order.len();
    let mut selected = vec![0usize; q];
    selected[q - 1] = last_idx;
    for k in (1..q).rev() {
        let prev_idx = parents[k][first_idx][selected[k]];
        if prev_idx == u16::MAX {
            return None;
        }
        selected[k - 1] = prev_idx as usize;
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

    Some(ClusterVariant {
        head: path[0],
        tail: path[path.len() - 1],
        score,
        path,
    })
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

    Cluster {
        sum_weight,
        variants: vec![ClusterVariant {
            head: path[0],
            tail: path[path.len() - 1],
            score: final_layer[best_idx].score,
            path,
        }],
    }
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

fn select_variant_specs_boundary(
    mut specs: Vec<VariantSpec>,
    cell_coords: &[(u16, u16)],
    top: usize,
    left: usize,
    span: usize,
    limit: usize,
) -> Vec<VariantSpec> {
    specs.sort_by(|a, b| b.score.total_cmp(&a.score));
    let bottom = top + span - 1;
    let right = left + span - 1;

    let mut selected = Vec::new();
    let mut seen_side_pairs = HashSet::new();
    let mut used_pairs = HashSet::new();

    for spec in &specs {
        let head_mask = boundary_mask(spec.head, cell_coords, top, bottom, left, right);
        let tail_mask = boundary_mask(spec.tail, cell_coords, top, bottom, left, right);
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
                    used_pairs.insert((spec.head, spec.tail));
                    selected.push(*spec);
                    inserted = true;
                    break;
                }
            }
            if inserted {
                break;
            }
        }
        if selected.len() == limit {
            return selected;
        }
    }

    for spec in &specs {
        if selected.len() == limit {
            return selected;
        }
        let head_mask = boundary_mask(spec.head, cell_coords, top, bottom, left, right);
        let tail_mask = boundary_mask(spec.tail, cell_coords, top, bottom, left, right);
        if head_mask == 0 || tail_mask == 0 {
            continue;
        }
        if used_pairs.insert((spec.head, spec.tail)) {
            selected.push(*spec);
        }
    }

    let mut seen_pairs = HashSet::new();
    for spec in &selected {
        seen_pairs.insert((spec.head, spec.tail));
    }
    for spec in specs {
        if selected.len() == limit {
            break;
        }
        if seen_pairs.insert((spec.head, spec.tail)) {
            selected.push(spec);
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
            return selected;
        }
    }

    for variant in &variants {
        if selected.len() == limit {
            return selected;
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

    let mut seen_pairs = HashSet::new();
    for variant in &selected {
        seen_pairs.insert((variant.head, variant.tail));
    }
    for variant in variants {
        if selected.len() == limit {
            break;
        }
        if seen_pairs.insert((variant.head, variant.tail)) {
            selected.push(variant);
        }
    }
    selected.sort_by(|a, b| b.score.total_cmp(&a.score));
    selected
}

fn build_base_templates(side: usize) -> Vec<Vec<u8>> {
    let base_patterns = vec![
        row_snake_coords(side),
        pair_weave_coords(side),
        diag_snake_coords(side),
    ];
    let mut templates = Vec::new();
    let mut seen = HashSet::new();

    for base in base_patterns {
        for sym in 0..8 {
            let mut path = Vec::with_capacity(side * side);
            for &(r, c) in &base {
                let (tr, tc) = apply_symmetry(r, c, side, sym);
                path.push((tr * side + tc) as u8);
            }
            let hash = hash_u8_path(&path);
            if seen.insert(hash) {
                templates.push(path);
            }
        }
    }

    templates
}

fn build_local_orders(side: usize) -> Vec<Vec<u8>> {
    let mut orders = Vec::new();
    let mut seen = HashSet::new();
    let base_patterns = vec![
        row_snake_coords(side),
        diag_snake_coords(side),
        spiral_coords(side),
    ];

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
    assert_eq!(side % 2, 0);
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

fn build_local_coords(side: usize) -> Vec<(i16, i16)> {
    let mut coords = Vec::with_capacity(side * side);
    for r in 0..side {
        for c in 0..side {
            coords.push((r as i16, c as i16));
        }
    }
    coords
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

fn optimize_local_adj_swaps_fixed_ends(
    path: &mut [u8],
    coords: &[(i16, i16)],
    weights: &[f64],
    passes: usize,
) {
    if path.len() < 4 {
        return;
    }
    for _ in 0..passes {
        let mut changed = false;
        for i in 1..(path.len() - 2) {
            let left = path[i] as usize;
            let right = path[i + 1] as usize;
            if weights[left] <= weights[right] {
                continue;
            }
            let prev = path[i - 1] as usize;
            let next = path[i + 2] as usize;
            if !coords_adj(coords[prev], coords[right]) {
                continue;
            }
            if !coords_adj(coords[left], coords[next]) {
                continue;
            }
            path.swap(i, i + 1);
            changed = true;
        }
        if !changed {
            break;
        }
    }
}

fn score_local_path(path: &[u8], weights: &[f64]) -> f64 {
    let mut score = 0.0f64;
    for (idx, &cell) in path.iter().enumerate() {
        score += idx as f64 * weights[cell as usize];
    }
    score
}

fn corner_pair_slot(head: usize, tail: usize, side: usize) -> Option<usize> {
    let tl = 0usize;
    let tr = side - 1;
    let bl = side * (side - 1);
    let br = side * side - 1;
    let a = head.min(tail);
    let b = head.max(tail);
    match (a, b) {
        (x, y) if x == tl && y == tr => Some(0),
        (x, y) if x == tl && y == bl => Some(1),
        (x, y) if x == tl && y == br => Some(2),
        (x, y) if x == tr && y == bl => Some(3),
        (x, y) if x == tr && y == br => Some(4),
        (x, y) if x == bl && y == br => Some(5),
        _ => None,
    }
}

fn positions_to_target(positions: &[u32]) -> Vec<f64> {
    positions.iter().map(|&p| p as f64).collect()
}

fn blend_weights(rank: &[f64], target: &[f64], eta: f64) -> Vec<f64> {
    rank.iter()
        .zip(target.iter())
        .map(|(&r, &t)| r + eta * (t - r))
        .collect()
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

fn build_block_cells(top: usize, left: usize, n: usize, side: usize) -> Vec<usize> {
    let mut cells = Vec::with_capacity(side * side);
    for r in 0..side {
        for c in 0..side {
            cells.push(cell_id(top + r, left + c, n));
        }
    }
    cells
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

fn int_sqrt_exact(x: usize) -> usize {
    let s = (x as f64).sqrt() as usize;
    assert_eq!(s * s, x);
    s
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

fn hash_u8_path(path: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
