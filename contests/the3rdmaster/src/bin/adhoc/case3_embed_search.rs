// case3_embed_search.rs
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const N: usize = 32;
const M: usize = N * N;
const WORDS: usize = M / 64;

type Color = u8;
type Grid = [[Color; N]; N];

#[derive(Debug, Clone)]
struct Input {
    goal: Grid,
}

impl Input {
    fn read_from_path(path: &Path) -> Self {
        let src = fs::read_to_string(path).expect("failed to read input");
        let mut tokens = src.split_ascii_whitespace();
        let n: usize = tokens.next().unwrap().parse().unwrap();
        let _k_layers: usize = tokens.next().unwrap().parse().unwrap();
        let _color_count: usize = tokens.next().unwrap().parse().unwrap();
        assert_eq!(n, N);

        let mut goal = [[0; N]; N];
        for row in &mut goal {
            for cell in row {
                *cell = tokens.next().unwrap().parse().unwrap();
            }
        }
        Self { goal }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Bits {
    w: [u64; WORDS],
}

impl Bits {
    fn empty() -> Self {
        Self { w: [0; WORDS] }
    }

    fn set(&mut self, p: usize) {
        self.w[p >> 6] |= 1_u64 << (p & 63);
    }

    fn or_assign(&mut self, other: &Self) {
        for i in 0..WORDS {
            self.w[i] |= other.w[i];
        }
    }

    fn and_not_count(&self, covered: &Self) -> usize {
        let mut res = 0_usize;
        for i in 0..WORDS {
            res += (self.w[i] & !covered.w[i]).count_ones() as usize;
        }
        res
    }

    fn positions(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for block in 0..WORDS {
            let mut bits = self.w[block];
            while bits != 0 {
                let tz = bits.trailing_zeros() as usize;
                out.push(block * 64 + tz);
                bits &= bits - 1;
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PatternKey {
    cells: Vec<(u8, u8, Color)>,
}

impl PatternKey {
    fn build_cost(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Debug, Clone)]
struct Placement {
    rot: u8,
    anchor_i: u8,
    anchor_j: u8,
    cover: Bits,
}

#[derive(Debug, Clone)]
struct CandidateType {
    key: PatternKey,
    placements: Vec<Placement>,
}

#[derive(Debug, Clone, Copy)]
enum ScoreMode {
    Delta,
    Ratio,
}

#[derive(Debug, Clone, Copy)]
struct Config {
    max_len: usize,
    max_painted: usize,
    score_mode: ScoreMode,
}

#[derive(Debug, Clone)]
struct SelectedType {
    type_idx: usize,
    placement_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct SearchResult {
    background: Color,
    config: Config,
    total_ops: usize,
    selected_types: Vec<SelectedType>,
    covered: Bits,
    candidate_types: Vec<CandidateType>,
}

#[derive(Debug, Clone)]
struct TypePlan {
    placement_indices: Vec<usize>,
    gain: usize,
    incremental_cost: usize,
    score_num: usize,
    score_den: usize,
}

fn main() {
    let input_path = Path::new("src/make_input/case3_random_input.txt");
    let best_path = Path::new("src/make_input/case3_random_best_output.txt");
    let search_out_path = Path::new("src/make_input/case3_random_search_output.txt");

    let input = Input::read_from_path(input_path);
    let prev_best_count = read_action_count(best_path).unwrap_or(usize::MAX);

    let configs = [
        Config {
            max_len: 4,
            max_painted: 3,
            score_mode: ScoreMode::Delta,
        },
        Config {
            max_len: 6,
            max_painted: 3,
            score_mode: ScoreMode::Delta,
        },
        Config {
            max_len: 8,
            max_painted: 3,
            score_mode: ScoreMode::Delta,
        },
        Config {
            max_len: 6,
            max_painted: 4,
            score_mode: ScoreMode::Delta,
        },
        Config {
            max_len: 8,
            max_painted: 4,
            score_mode: ScoreMode::Delta,
        },
        Config {
            max_len: 6,
            max_painted: 3,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 8,
            max_painted: 3,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 8,
            max_painted: 4,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 12,
            max_painted: 2,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 12,
            max_painted: 3,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 16,
            max_painted: 2,
            score_mode: ScoreMode::Ratio,
        },
        Config {
            max_len: 16,
            max_painted: 3,
            score_mode: ScoreMode::Ratio,
        },
    ];

    let mut best_result: Option<SearchResult> = None;
    for background in 1..=4_u8 {
        for &config in &configs {
            let candidate_types = generate_candidate_types(&input, background, config);
            let mut result = greedy_search(&input, background, config, candidate_types);
            prune_result(&input, &mut result);
            eprintln!(
                "background={} max_len={} max_painted={} mode={:?} types={} ops={}",
                background,
                config.max_len,
                config.max_painted,
                config.score_mode,
                result.selected_types.len(),
                result.total_ops,
            );
            if best_result
                .as_ref()
                .is_none_or(|best| result.total_ops < best.total_ops)
            {
                best_result = Some(result);
            }
        }
    }

    let best_result = best_result.expect("no search result");
    let output_text = render_solution(&input, &best_result);
    let action_count = output_text.lines().filter(|line| !line.is_empty()).count();
    assert_eq!(action_count, best_result.total_ops);
    verify_solution(&input, &output_text);

    fs::write(search_out_path, &output_text).expect("failed to write search output");
    eprintln!("generated {}", search_out_path.display());

    if action_count < prev_best_count {
        fs::write(best_path, &output_text).expect("failed to update best output");
        eprintln!(
            "updated {}: {} -> {}",
            best_path.display(),
            prev_best_count,
            action_count
        );
    } else {
        eprintln!(
            "best unchanged: current={} candidate={}",
            prev_best_count, action_count
        );
    }
}

fn read_action_count(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.lines().filter(|line| !line.trim().is_empty()).count())
}

fn generate_candidate_types(input: &Input, background: Color, config: Config) -> Vec<CandidateType> {
    let mut map: HashMap<PatternKey, Vec<Placement>> = HashMap::new();
    let mut seen: HashMap<PatternKey, HashSet<(u8, u8, u8)>> = HashMap::new();

    for len in 2..=config.max_len {
        for i in 0..N {
            for j in 0..=N - len {
                let mut offsets = Vec::new();
                let mut colors = Vec::new();
                for off in 0..len {
                    let color = input.goal[i][j + off];
                    if color != background {
                        offsets.push(off as u8);
                        colors.push(color);
                    }
                }
                if offsets.len() < 2 || offsets.len() > config.max_painted {
                    continue;
                }
                let shift = offsets[0];
                for off in &mut offsets {
                    *off -= shift;
                }
                let anchor_j = (j + shift as usize) as u8;
                let key = PatternKey {
                    cells: offsets
                        .iter()
                        .zip(colors.iter())
                        .map(|(&off, &color)| (0, off, color))
                        .collect(),
                };
                let placement_key = (0, i as u8, anchor_j);
                if seen.entry(key.clone()).or_default().insert(placement_key) {
                    let mut cover = Bits::empty();
                    for &(pi, pj, _) in &key.cells {
                        cover.set((i + usize::from(pi)) * N + usize::from(anchor_j) + usize::from(pj));
                    }
                    map.entry(key).or_default().push(Placement {
                        rot: 0,
                        anchor_i: i as u8,
                        anchor_j,
                        cover,
                    });
                }
            }
        }

        for i in 0..=N - len {
            for j in 0..N {
                let mut offsets = Vec::new();
                let mut colors = Vec::new();
                for off in 0..len {
                    let color = input.goal[i + off][j];
                    if color != background {
                        offsets.push(off as u8);
                        colors.push(color);
                    }
                }
                if offsets.len() < 2 || offsets.len() > config.max_painted {
                    continue;
                }
                let shift = offsets[0];
                for off in &mut offsets {
                    *off -= shift;
                }
                let anchor_i = (i + shift as usize) as u8;
                let key = PatternKey {
                    cells: offsets
                        .iter()
                        .zip(colors.iter())
                        .map(|(&off, &color)| (0, off, color))
                        .collect(),
                };
                let placement_key = (1, anchor_i, j as u8);
                if seen.entry(key.clone()).or_default().insert(placement_key) {
                    let mut cover = Bits::empty();
                    for &(pi, pj, _) in &key.cells {
                        cover.set((usize::from(anchor_i) + usize::from(pj)) * N + j + usize::from(pi));
                    }
                    map.entry(key).or_default().push(Placement {
                        rot: 1,
                        anchor_i,
                        anchor_j: j as u8,
                        cover,
                    });
                }
            }
        }
    }

    for height in 2..=4 {
        for width in 2..=4 {
            for i in 0..=N - height {
                for j in 0..=N - width {
                    let mut cells = Vec::new();
                    let mut min_r = height;
                    let mut min_c = width;
                    for di in 0..height {
                        for dj in 0..width {
                            let color = input.goal[i + di][j + dj];
                            if color == background {
                                continue;
                            }
                            cells.push((di as u8, dj as u8, color));
                            min_r = min_r.min(di);
                            min_c = min_c.min(dj);
                        }
                    }
                    if cells.len() < 2 || cells.len() > config.max_painted {
                        continue;
                    }
                    let same_row = cells.iter().all(|&(di, _, _)| di == cells[0].0);
                    let same_col = cells.iter().all(|&(_, dj, _)| dj == cells[0].1);
                    if same_row || same_col {
                        continue;
                    }
                    for cell in &mut cells {
                        cell.0 -= min_r as u8;
                        cell.1 -= min_c as u8;
                    }
                    cells.sort_unstable();
                    let key = PatternKey { cells };
                    let anchor_i = (i + min_r) as u8;
                    let anchor_j = (j + min_c) as u8;
                    let placement_key = (0, anchor_i, anchor_j);
                    if seen.entry(key.clone()).or_default().insert(placement_key) {
                        let mut cover = Bits::empty();
                        for &(pi, pj, _) in &key.cells {
                            cover.set(
                                (usize::from(anchor_i) + usize::from(pi)) * N
                                    + usize::from(anchor_j)
                                    + usize::from(pj),
                            );
                        }
                        map.entry(key).or_default().push(Placement {
                            rot: 0,
                            anchor_i,
                            anchor_j,
                            cover,
                        });
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for (key, placements) in map {
        if placements.len() >= 2 {
            out.push(CandidateType { key, placements });
        }
    }
    out.sort_by_key(|cand| (cand.key.build_cost(), usize::MAX - cand.placements.len()));
    out
}

fn greedy_search(
    input: &Input,
    background: Color,
    config: Config,
    candidate_types: Vec<CandidateType>,
) -> SearchResult {
    let target = target_non_background_bits(input, background);
    let mut covered = Bits::empty();
    let mut selected_types = Vec::new();
    let mut used = vec![false; candidate_types.len()];

    loop {
        let clear_cost = usize::from(!selected_types.is_empty());
        let mut best_idx = None;
        let mut best_plan = None;

        for (idx, cand) in candidate_types.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let plan = best_plan_for_type(cand, &covered, clear_cost, config.score_mode);
            if plan.gain == 0 {
                continue;
            }
            if plan.gain <= plan.incremental_cost {
                continue;
            }
            let better = match &best_plan {
                None => true,
                Some(curr) => compare_plan(&plan, curr, config.score_mode),
            };
            if better {
                best_idx = Some(idx);
                best_plan = Some(plan);
            }
        }

        let Some(type_idx) = best_idx else {
            break;
        };
        let plan = best_plan.unwrap();
        used[type_idx] = true;
        let cand = &candidate_types[type_idx];
        for &placement_idx in &plan.placement_indices {
            covered.or_assign(&cand.placements[placement_idx].cover);
        }
        selected_types.push(SelectedType {
            type_idx,
            placement_indices: plan.placement_indices,
        });
        if covered.and_not_count(&target) == 0 && target.and_not_count(&covered) == 0 {
            break;
        }
    }

    let total_ops = evaluate_total_ops(input, background, &candidate_types, &selected_types);
    SearchResult {
        background,
        config,
        total_ops,
        selected_types,
        covered,
        candidate_types,
    }
}

fn best_plan_for_type(
    cand: &CandidateType,
    covered: &Bits,
    clear_cost: usize,
    score_mode: ScoreMode,
) -> TypePlan {
    let mut local = *covered;
    let mut used = vec![false; cand.placements.len()];
    let mut chosen = Vec::new();
    let mut gain = 0_usize;

    loop {
        let mut best_idx = None;
        let mut best_gain = 0_usize;
        for (idx, placement) in cand.placements.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let g = placement.cover.and_not_count(&local);
            if g > best_gain {
                best_gain = g;
                best_idx = Some(idx);
            }
        }
        if best_gain < 2 {
            break;
        }
        let idx = best_idx.unwrap();
        used[idx] = true;
        chosen.push(idx);
        gain += best_gain;
        local.or_assign(&cand.placements[idx].cover);
    }

    let incremental_cost = cand.key.build_cost() + clear_cost + chosen.len();
    let (score_num, score_den) = match score_mode {
        ScoreMode::Delta => (gain.saturating_sub(incremental_cost), 1),
        ScoreMode::Ratio => (gain, incremental_cost.max(1)),
    };
    TypePlan {
        placement_indices: chosen,
        gain,
        incremental_cost,
        score_num,
        score_den,
    }
}

fn compare_plan(a: &TypePlan, b: &TypePlan, score_mode: ScoreMode) -> bool {
    match score_mode {
        ScoreMode::Delta => {
            (a.score_num, a.gain, usize::MAX - a.incremental_cost)
                > (b.score_num, b.gain, usize::MAX - b.incremental_cost)
        }
        ScoreMode::Ratio => {
            let lhs = (a.score_num as u128) * (b.score_den as u128);
            let rhs = (b.score_num as u128) * (a.score_den as u128);
            if lhs != rhs {
                lhs > rhs
            } else {
                (a.gain, usize::MAX - a.incremental_cost)
                    > (b.gain, usize::MAX - b.incremental_cost)
            }
        }
    }
}

fn prune_result(input: &Input, result: &mut SearchResult) {
    loop {
        let before = result.total_ops;
        let mut changed = false;

        let mut occupied = Bits::empty();
        for selected in &result.selected_types {
            let cand = &result.candidate_types[selected.type_idx];
            for &placement_idx in &selected.placement_indices {
                occupied.or_assign(&cand.placements[placement_idx].cover);
            }
        }

        'outer: for type_pos in 0..result.selected_types.len() {
            let type_idx = result.selected_types[type_pos].type_idx;
            let cand = &result.candidate_types[type_idx];
            for placement_pos in 0..result.selected_types[type_pos].placement_indices.len() {
                let placement_idx = result.selected_types[type_pos].placement_indices[placement_pos];
                let cover = cand.placements[placement_idx].cover;
                let mut without = Bits::empty();
                for (other_type_pos, selected) in result.selected_types.iter().enumerate() {
                    let other_cand = &result.candidate_types[selected.type_idx];
                    for (other_pos, &other_idx) in selected.placement_indices.iter().enumerate() {
                        if other_type_pos == type_pos && other_pos == placement_pos {
                            continue;
                        }
                        without.or_assign(&other_cand.placements[other_idx].cover);
                    }
                }
                let unique_gain = cover.and_not_count(&without);
                if unique_gain <= 1 {
                    result.selected_types[type_pos]
                        .placement_indices
                        .remove(placement_pos);
                    if result.selected_types[type_pos].placement_indices.is_empty() {
                        result.selected_types.remove(type_pos);
                    }
                    changed = true;
                    break 'outer;
                }
            }
        }

        if !changed {
            for type_pos in 0..result.selected_types.len() {
                let mut trial = result.selected_types.clone();
                trial.remove(type_pos);
                let trial_ops =
                    evaluate_total_ops(input, result.background, &result.candidate_types, &trial);
                if trial_ops <= result.total_ops {
                    result.selected_types = trial;
                    changed = true;
                    break;
                }
            }
        }

        result.total_ops = evaluate_total_ops(
            input,
            result.background,
            &result.candidate_types,
            &result.selected_types,
        );
        result.covered = covered_bits(&result.candidate_types, &result.selected_types);
        if !changed || result.total_ops > before {
            break;
        }
    }
}

fn target_non_background_bits(input: &Input, background: Color) -> Bits {
    let mut bits = Bits::empty();
    for i in 0..N {
        for j in 0..N {
            if input.goal[i][j] != background {
                bits.set(i * N + j);
            }
        }
    }
    bits
}

fn covered_bits(candidate_types: &[CandidateType], selected_types: &[SelectedType]) -> Bits {
    let mut bits = Bits::empty();
    for selected in selected_types {
        let cand = &candidate_types[selected.type_idx];
        for &placement_idx in &selected.placement_indices {
            bits.or_assign(&cand.placements[placement_idx].cover);
        }
    }
    bits
}

fn evaluate_total_ops(
    input: &Input,
    background: Color,
    candidate_types: &[CandidateType],
    selected_types: &[SelectedType],
) -> usize {
    let mut total = build_full_background_ops(background).len();
    let mut used_type_count = 0_usize;
    let mut covered = Bits::empty();
    for selected in selected_types {
        let cand = &candidate_types[selected.type_idx];
        if used_type_count > 0 {
            total += 1; // clear layer 1
        }
        used_type_count += 1;
        total += cand.key.build_cost();
        total += selected.placement_indices.len();
        for &placement_idx in &selected.placement_indices {
            covered.or_assign(&cand.placements[placement_idx].cover);
        }
    }
    for p in target_non_background_bits(input, background).positions() {
        if (covered.w[p >> 6] >> (p & 63)) & 1 == 0 {
            total += 1;
        }
    }
    total
}

fn render_solution(input: &Input, result: &SearchResult) -> String {
    let mut ops = Vec::<String>::new();
    for line in build_full_background_ops(result.background) {
        ops.push(line);
    }

    let mut first = true;
    for selected in &result.selected_types {
        let cand = &result.candidate_types[selected.type_idx];
        if !first {
            ops.push("2 1".to_string());
        }
        first = false;
        for &(pi, pj, color) in &cand.key.cells {
            ops.push(format!("0 1 {} {} {}", pi, pj, color));
        }
        for &placement_idx in &selected.placement_indices {
            let placement = &cand.placements[placement_idx];
            let (di, dj) = match placement.rot {
                0 => (placement.anchor_i as isize, placement.anchor_j as isize),
                1 => (placement.anchor_i as isize, placement.anchor_j as isize - (N as isize - 1)),
                _ => unreachable!(),
            };
            ops.push(format!("1 0 1 {} {} {}", placement.rot, di, dj));
        }
    }

    let covered = covered_bits(&result.candidate_types, &result.selected_types);
    for p in target_non_background_bits(input, result.background).positions() {
        if ((covered.w[p >> 6] >> (p & 63)) & 1) != 0 {
            continue;
        }
        let i = p / N;
        let j = p % N;
        ops.push(format!("0 0 {} {} {}", i, j, input.goal[i][j]));
    }

    let mut out = String::new();
    for line in ops {
        let _ = writeln!(out, "{line}");
    }
    out
}

fn grow_deltas(target: usize) -> Vec<usize> {
    let mut curr = 1_usize;
    let mut deltas = Vec::new();
    while curr < target {
        let delta = curr.min(target - curr);
        deltas.push(delta);
        curr += delta;
    }
    deltas
}

fn build_full_background_ops(color: Color) -> Vec<String> {
    let mut ops = Vec::new();
    ops.push(format!("0 0 0 0 {}", color));
    for dj in grow_deltas(N) {
        ops.push(format!("1 0 0 0 0 {}", dj));
    }
    for di in grow_deltas(N) {
        ops.push(format!("1 0 0 0 {} 0", di));
    }
    ops
}

fn verify_solution(input: &Input, output_text: &str) {
    let mut layers = vec![[[0_u8; N]; N]; 2];
    for raw in output_text.lines() {
        if raw.trim().is_empty() {
            continue;
        }
        let vals = raw
            .split_ascii_whitespace()
            .map(|s| s.parse::<isize>().unwrap())
            .collect::<Vec<_>>();
        match vals[0] {
            0 => {
                let k = vals[1] as usize;
                let i = vals[2] as usize;
                let j = vals[3] as usize;
                let color = vals[4] as u8;
                layers[k][i][j] = color;
            }
            1 => {
                let k = vals[1] as usize;
                let h = vals[2] as usize;
                let rot = vals[3] as usize;
                let di = vals[4];
                let dj = vals[5];
                let src = layers[h];
                for i in 0..N {
                    for j in 0..N {
                        let color = src[i][j];
                        if color == 0 {
                            continue;
                        }
                        let (ri, rj) = rotate((i, j), rot);
                        let ni = ri as isize + di;
                        let nj = rj as isize + dj;
                        assert!((0..N as isize).contains(&ni));
                        assert!((0..N as isize).contains(&nj));
                        layers[k][ni as usize][nj as usize] = color;
                    }
                }
            }
            2 => {
                let k = vals[1] as usize;
                layers[k] = [[0_u8; N]; N];
            }
            _ => panic!("unknown op"),
        }
    }
    assert_eq!(layers[0], input.goal);
}

fn rotate((i, j): (usize, usize), rot: usize) -> (usize, usize) {
    match rot & 3 {
        0 => (i, j),
        1 => (j, N - 1 - i),
        2 => (N - 1 - i, N - 1 - j),
        3 => (N - 1 - j, i),
        _ => unreachable!(),
    }
}
