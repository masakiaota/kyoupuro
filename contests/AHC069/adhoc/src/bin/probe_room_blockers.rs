// probe_room_blockers.rs
#![allow(non_snake_case)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const EMPTY: usize = !0_usize;

#[derive(Default)]
struct Aggregate {
    cases: usize,
    high_q_fragmented: usize,
    targets_with_blocker: usize,
    blocker_links: usize,
    translated_candidates: usize,
    history_safe_candidates: usize,
    component_improving_candidates: usize,
    rescuing_candidates: usize,
    targets_with_safe_alternative: usize,
    targets_rescued: usize,
    rescued_ideal_fee: i128,
    target_bucket: [usize; 4],
    rescued_bucket: [usize; 4],
    rescued_blocker_bucket: [usize; 4],
    rescued_blocker_slack_le6: usize,
    rescued_blocker_slack_ge8: usize,
    rescued_slot_visible: usize,
    rescued_slot_blind: usize,
    rescued_projected_visible: usize,
    rescued_projected_blind: usize,
    rescued_projected_visible_fee: i128,
    rescued_projected_blind_fee: i128,
    rescued_cutloss_visible: usize,
    rescued_cutloss_blind: usize,
    rescued_cutloss_visible_fee: i128,
    rescued_compact_box: usize,
    rescued_compact_box_fee: i128,
    rescued_target_projection_visible: usize,
    rescued_blocker_D_bucket: [usize; 4],
}

#[derive(Clone, Copy)]
struct ProbeSlot {
    x: usize,
    y: usize,
    h: usize,
    w: usize,
    ready: i64,
}

fn input_paths(input_dir: &Path, case_limit: usize) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(input_dir)
        .expect("read input dir")
        .map(|entry| entry.expect("read dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    paths.sort();
    paths.truncate(case_limit);
    paths
}

fn free_components(grass: &[Vec<bool>], occupied: &[Vec<bool>]) -> (usize, usize, Vec<isize>) {
    let N = grass.len();
    let mut component = vec![-1_isize; N * N];
    let mut total = 0;
    let mut largest = 0;
    let mut next_component = 0_isize;
    let mut queue = Vec::with_capacity(N * N);
    for x in 0..N {
        for y in 0..N {
            let start = x * N + y;
            if !grass[x][y] || occupied[x][y] || component[start] >= 0 {
                continue;
            }
            queue.clear();
            queue.push(start);
            component[start] = next_component;
            let mut head = 0;
            while head < queue.len() {
                let id = queue[head];
                head += 1;
                let r = id / N;
                let c = id % N;
                for next in [
                    (r > 0).then_some(id - N),
                    (r + 1 < N).then_some(id + N),
                    (c > 0).then_some(id - 1),
                    (c + 1 < N).then_some(id + 1),
                ]
                .into_iter()
                .flatten()
                {
                    let nr = next / N;
                    let nc = next % N;
                    if grass[nr][nc] && !occupied[nr][nc] && component[next] < 0 {
                        component[next] = next_component;
                        queue.push(next);
                    }
                }
            }
            total += queue.len();
            largest = largest.max(queue.len());
            next_component += 1;
        }
    }
    (total, largest, component)
}

fn free_cut_loss(N: usize, free: &[bool]) -> Vec<usize> {
    let cell_count = N * N;
    let unvisited = usize::MAX;
    let mut timer = 0_usize;
    let mut tin = vec![unvisited; cell_count];
    let mut low = vec![0_usize; cell_count];
    let mut parent = vec![unvisited; cell_count];
    let mut subtree = vec![0_usize; cell_count];
    let mut root_of = vec![unvisited; cell_count];
    let mut component_size_at_root = vec![0_usize; cell_count];

    #[allow(clippy::too_many_arguments)]
    fn dfs(
        v: usize,
        root: usize,
        N: usize,
        free: &[bool],
        timer: &mut usize,
        tin: &mut [usize],
        low: &mut [usize],
        parent: &mut [usize],
        subtree: &mut [usize],
        root_of: &mut [usize],
    ) {
        tin[v] = *timer;
        low[v] = *timer;
        *timer += 1;
        subtree[v] = 1;
        root_of[v] = root;
        let r = v / N;
        let c = v % N;
        for to in [
            (r > 0).then_some(v.wrapping_sub(N)),
            (r + 1 < N).then_some(v + N),
            (c > 0).then_some(v.wrapping_sub(1)),
            (c + 1 < N).then_some(v + 1),
        ]
        .into_iter()
        .flatten()
        {
            if !free[to] || to == parent[v] {
                continue;
            }
            if tin[to] == usize::MAX {
                parent[to] = v;
                dfs(to, root, N, free, timer, tin, low, parent, subtree, root_of);
                subtree[v] += subtree[to];
                low[v] = low[v].min(low[to]);
            } else {
                low[v] = low[v].min(tin[to]);
            }
        }
    }

    for root in 0..cell_count {
        if free[root] && tin[root] == unvisited {
            dfs(
                root,
                root,
                N,
                free,
                &mut timer,
                &mut tin,
                &mut low,
                &mut parent,
                &mut subtree,
                &mut root_of,
            );
            component_size_at_root[root] = subtree[root];
        }
    }
    let mut result = vec![0_usize; cell_count];
    for v in 0..cell_count {
        if !free[v] {
            continue;
        }
        let component_size = component_size_at_root[root_of[v]];
        let r = v / N;
        let c = v % N;
        let mut separated_sum = 0;
        let mut largest_piece = 0;
        for child in [
            (r > 0).then_some(v.wrapping_sub(N)),
            (r + 1 < N).then_some(v + N),
            (c > 0).then_some(v.wrapping_sub(1)),
            (c + 1 < N).then_some(v + 1),
        ]
        .into_iter()
        .flatten()
        {
            if parent[child] == v && low[child] >= tin[v] {
                separated_sum += subtree[child];
                largest_piece = largest_piece.max(subtree[child]);
            }
        }
        largest_piece = largest_piece.max(component_size - 1 - separated_sum);
        result[v] = component_size - 1 - largest_piece;
    }
    result
}

fn occupied_without(frame: &tools::Frame, removed: Option<usize>) -> Vec<Vec<bool>> {
    frame
        .grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|&owner| owner != EMPTY && Some(owner) != removed)
                .collect()
        })
        .collect()
}

fn group_cells(frame: &tools::Frame, group_id: usize) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for (x, row) in frame.grid.iter().enumerate() {
        for (y, &owner) in row.iter().enumerate() {
            if owner == group_id {
                cells.push((x, y));
            }
        }
    }
    cells
}

fn minimum_perimeter(P: usize) -> usize {
    2 * (2.0 * (P as f64).sqrt() - 1e-12).ceil() as usize
}

fn ideal_fee(V: i64, P: usize) -> i64 {
    ((V as f64) * 4.0 * (P as f64).sqrt() / (minimum_perimeter(P) as f64)).round() as i64
}

fn has_compact_box(input: &tools::Input, occupied: &[Vec<bool>], P: usize) -> bool {
    let N = input.n;
    let mut prefix = vec![vec![0_usize; N + 1]; N + 1];
    for x in 0..N {
        for y in 0..N {
            let free = usize::from(input.grass[x][y] && !occupied[x][y]);
            prefix[x + 1][y + 1] = free + prefix[x][y + 1] + prefix[x + 1][y] - prefix[x][y];
        }
    }
    let min_L = minimum_perimeter(P);
    for h in 1..=N {
        for w in 1..=N {
            let area = h * w;
            if area < P || area > P + 24 || 2 * (h + w) > min_L + 2 {
                continue;
            }
            let short = h.min(w);
            let long = h.max(w);
            if 2 * long > 3 * short {
                continue;
            }
            for x in 0..=N - h {
                for y in 0..=N - w {
                    let free_count =
                        prefix[x + h][y + w] - prefix[x][y + w] - prefix[x + h][y] + prefix[x][y];
                    if free_count < P {
                        continue;
                    }
                    let mut seen = vec![false; h * w];
                    let mut queue = Vec::with_capacity(free_count);
                    for local_x in 0..h {
                        for local_y in 0..w {
                            let local = local_x * w + local_y;
                            if seen[local]
                                || !input.grass[x + local_x][y + local_y]
                                || occupied[x + local_x][y + local_y]
                            {
                                continue;
                            }
                            seen[local] = true;
                            queue.clear();
                            queue.push(local);
                            let mut head = 0;
                            while head < queue.len() {
                                let current = queue[head];
                                head += 1;
                                let r = current / w;
                                let c = current % w;
                                for next in [
                                    (r > 0).then_some(current - w),
                                    (r + 1 < h).then_some(current + w),
                                    (c > 0).then_some(current - 1),
                                    (c + 1 < w).then_some(current + 1),
                                ]
                                .into_iter()
                                .flatten()
                                {
                                    let nr = next / w;
                                    let nc = next % w;
                                    if !seen[next]
                                        && input.grass[x + nr][y + nc]
                                        && !occupied[x + nr][y + nc]
                                    {
                                        seen[next] = true;
                                        queue.push(next);
                                    }
                                }
                            }
                            if queue.len() >= P {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn size_bucket(P: usize) -> usize {
    match P {
        4..=63 => 0,
        64..=95 => 1,
        96..=127 => 2,
        _ => 3,
    }
}

fn duration_bucket(D: i64) -> usize {
    match D {
        ..=999 => 0,
        1_000..=2_999 => 1,
        3_000..=5_999 => 2,
        _ => 3,
    }
}

fn perimeter(cells: &[(usize, usize)]) -> usize {
    let selected: std::collections::HashSet<_> = cells.iter().copied().collect();
    cells
        .iter()
        .map(|&(x, y)| {
            [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ]
            .iter()
            .filter(|next| !selected.contains(next))
            .count()
        })
        .sum()
}

fn slots_overlap(a: ProbeSlot, b: ProbeSlot) -> bool {
    a.x < b.x + b.h && b.x < a.x + a.h && a.y < b.y + b.w && b.y < a.y + a.w
}

fn build_large_slots(
    input: &tools::Input,
    frame: &tools::Frame,
    removed: usize,
    now: i64,
) -> Vec<ProbeSlot> {
    const DIMS: [(usize, usize); 9] = [
        (10, 10),
        (10, 12),
        (12, 10),
        (11, 11),
        (10, 15),
        (15, 10),
        (12, 12),
        (12, 13),
        (13, 12),
    ];
    let N = input.n;
    let mut departure = vec![now; input.m];
    for active in &frame.actives {
        departure[active.id] = active.t;
    }
    let mut pool = Vec::new();
    for (h, w) in DIMS {
        for x in 0..=N - h {
            for y in 0..=N - w {
                let mut ready = now;
                let mut valid = true;
                for row in 0..h {
                    for col in 0..w {
                        let r = x + row;
                        let c = y + col;
                        if !input.grass[r][c] {
                            valid = false;
                            break;
                        }
                        let owner = frame.grid[r][c];
                        if owner != EMPTY && owner != removed {
                            ready = ready.max(departure[owner]);
                        }
                    }
                    if !valid {
                        break;
                    }
                }
                if valid {
                    pool.push(ProbeSlot { x, y, h, w, ready });
                }
            }
        }
    }
    pool.sort_unstable_by_key(|slot| {
        (
            slot.ready,
            std::cmp::Reverse(slot.h * slot.w),
            slot.x,
            slot.y,
            slot.h,
            slot.w,
        )
    });
    let mut selected = Vec::new();
    for candidate in pool {
        if selected.iter().all(|&slot| !slots_overlap(slot, candidate)) {
            selected.push(candidate);
            if selected.len() == 6 {
                break;
            }
        }
    }
    selected
}

fn slot_delay(cells: &[(usize, usize)], T: i64, slots: &[ProbeSlot]) -> i64 {
    slots
        .iter()
        .filter(|slot| {
            cells.iter().any(|&(x, y)| {
                slot.x <= x && x < slot.x + slot.h && slot.y <= y && y < slot.y + slot.w
            })
        })
        .map(|slot| (T - slot.ready).max(0))
        .sum()
}

fn projected_largest(
    input: &tools::Input,
    frame: &tools::Frame,
    removed: usize,
    candidate: &[(usize, usize)],
    at: i64,
) -> usize {
    let mut departure = vec![i64::MAX; input.m];
    for active in &frame.actives {
        departure[active.id] = active.t;
    }
    let mut occupied = vec![vec![false; input.n]; input.n];
    for (x, row) in frame.grid.iter().enumerate() {
        for (y, &owner) in row.iter().enumerate() {
            if owner != EMPTY && owner != removed && departure[owner] >= at {
                occupied[x][y] = true;
            }
        }
    }
    for &(x, y) in candidate {
        occupied[x][y] = true;
    }
    free_components(&input.grass, &occupied).1
}

fn projected_cut_loss_field(
    input: &tools::Input,
    frame: &tools::Frame,
    removed: usize,
) -> Vec<usize> {
    let blocker = input.groups[removed];
    let D = blocker.t - blocker.s;
    let mut departure = vec![i64::MAX; input.m];
    for active in &frame.actives {
        departure[active.id] = active.t;
    }
    let mut field = vec![0_usize; input.n * input.n];
    for k in [1_i64, 2, 3] {
        let at = blocker.s + D * k / 4;
        let free: Vec<bool> = (0..input.n * input.n)
            .map(|id| {
                let x = id / input.n;
                let y = id % input.n;
                if !input.grass[x][y] {
                    return false;
                }
                let owner = frame.grid[x][y];
                owner == EMPTY || owner == removed || departure[owner] < at
            })
            .collect();
        let snapshot = free_cut_loss(input.n, &free);
        for id in 0..field.len() {
            field[id] += snapshot[id];
        }
    }
    field
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_dir = Path::new(args.get(1).map_or("tools/in", String::as_str));
    let output_dir = Path::new(
        args.get(2)
            .map_or("results/out/v035_no_move_growth_cutloss", String::as_str),
    );
    let case_limit = args.get(3).map_or(5, |s| s.parse().expect("case limit"));
    let mut aggregate = Aggregate::default();
    let mut representative_count = 0;

    for input_path in input_paths(input_dir, case_limit) {
        let basename = input_path.file_name().expect("input basename");
        let output_path = output_dir.join(basename);
        let input_text = fs::read_to_string(&input_path).expect("read input");
        let output_text = fs::read_to_string(&output_path).expect("read output");
        let input = tools::parse_input(&input_text);
        let output = tools::parse_output(&input, &output_text);
        assert!(
            output.error.is_none(),
            "{}: {:?}",
            output_path.display(),
            output.error
        );
        assert_eq!(input.m + 2, output.frames.len());
        aggregate.cases += 1;

        let mut arrival_frame = vec![usize::MAX; input.m];
        for (frame_index, frame) in output.frames.iter().enumerate() {
            if let Some((group_id, _)) = frame.arrival {
                arrival_frame[group_id] = frame_index;
            }
        }

        for target_id in 0..input.m {
            let target_frame_index = arrival_frame[target_id];
            let target_frame = &output.frames[target_frame_index];
            if target_frame.arrival != Some((target_id, false)) {
                continue;
            }
            let target = input.groups[target_id];
            let D = target.t - target.s;
            let q = (target.v as f64) / ((target.p as f64) * (D as f64).powf(0.9));
            if q < 1.5 {
                continue;
            }
            let target_occupied = occupied_without(target_frame, None);
            let (total_free, base_largest, _) = free_components(&input.grass, &target_occupied);
            if total_free < target.p || base_largest >= target.p {
                continue;
            }
            aggregate.high_q_fragmented += 1;
            aggregate.target_bucket[size_bucket(target.p)] += 1;

            let mut blockers = Vec::new();
            for active in &target_frame.actives {
                let without = occupied_without(target_frame, Some(active.id));
                let (_, largest_without, _) = free_components(&input.grass, &without);
                if largest_without > base_largest {
                    blockers.push((active.id, largest_without));
                }
            }
            blockers.sort_unstable_by_key(|&(_, largest)| std::cmp::Reverse(largest));
            if !blockers.is_empty() {
                aggregate.targets_with_blocker += 1;
                aggregate.blocker_links += blockers.len();
            }

            let mut target_has_safe = false;
            let mut target_rescued = false;
            let mut target_slot_visible = false;
            let mut target_projected_visible = false;
            let mut target_cutloss_visible = false;
            let mut target_compact_box = false;
            let mut target_projection_visible = false;
            for (blocker_id, removed_largest) in blockers {
                let blocker_frame_index = arrival_frame[blocker_id];
                assert!(blocker_frame_index < target_frame_index);
                let blocker_frame = &output.frames[blocker_frame_index];
                let saved_cells = group_cells(blocker_frame, blocker_id);
                assert_eq!(saved_cells.len(), input.groups[blocker_id].p);
                let min_x = saved_cells.iter().map(|&(x, _)| x).min().unwrap();
                let min_y = saved_cells.iter().map(|&(_, y)| y).min().unwrap();
                let offsets: Vec<_> = saved_cells
                    .iter()
                    .map(|&(x, y)| (x - min_x, y - min_y))
                    .collect();
                let height = offsets.iter().map(|&(x, _)| x).max().unwrap() + 1;
                let width = offsets.iter().map(|&(_, y)| y).max().unwrap() + 1;

                let pre_occupied = occupied_without(blocker_frame, Some(blocker_id));
                let (_, _, pre_component) = free_components(&input.grass, &pre_occupied);
                let original_component =
                    pre_component[saved_cells[0].0 * input.n + saved_cells[0].1];
                assert!(original_component >= 0);

                let mut history_forbidden = vec![false; input.n * input.n];
                for frame in &output.frames[blocker_frame_index..=target_frame_index] {
                    for x in 0..input.n {
                        for y in 0..input.n {
                            let owner = frame.grid[x][y];
                            if owner != EMPTY && owner != blocker_id {
                                history_forbidden[x * input.n + y] = true;
                            }
                        }
                    }
                }

                let blocker_T = input.groups[blocker_id].t;
                let mut slot_context: Option<(Vec<ProbeSlot>, i64)> = None;
                let mut cutloss_context: Option<(Vec<usize>, usize)> = None;

                for anchor_x in 0..=input.n - height {
                    for anchor_y in 0..=input.n - width {
                        aggregate.translated_candidates += 1;
                        let candidate: Vec<_> = offsets
                            .iter()
                            .map(|&(dx, dy)| (anchor_x + dx, anchor_y + dy))
                            .collect();
                        if candidate.iter().any(|&(x, y)| {
                            !input.grass[x][y]
                                || pre_occupied[x][y]
                                || pre_component[x * input.n + y] != original_component
                                || history_forbidden[x * input.n + y]
                        }) {
                            continue;
                        }
                        aggregate.history_safe_candidates += 1;
                        target_has_safe = true;

                        let mut counter_occupied = occupied_without(target_frame, Some(blocker_id));
                        for &(x, y) in &candidate {
                            assert!(!counter_occupied[x][y]);
                            counter_occupied[x][y] = true;
                        }
                        let (_, counter_largest, _) =
                            free_components(&input.grass, &counter_occupied);
                        if counter_largest > base_largest {
                            aggregate.component_improving_candidates += 1;
                        }
                        if counter_largest >= target.p {
                            aggregate.rescuing_candidates += 1;
                            let (slots, saved_delay) = slot_context.get_or_insert_with(|| {
                                let slots = build_large_slots(
                                    &input,
                                    blocker_frame,
                                    blocker_id,
                                    input.groups[blocker_id].s,
                                );
                                let saved_delay = slot_delay(&saved_cells, blocker_T, &slots);
                                (slots, saved_delay)
                            });
                            let candidate_delay = slot_delay(&candidate, blocker_T, slots);
                            if candidate_delay < *saved_delay {
                                target_slot_visible = true;
                            }
                            let blocker = input.groups[blocker_id];
                            let blocker_D = blocker.t - blocker.s;
                            let snapshot_score = [1_i64, 2, 3]
                                .iter()
                                .map(|&k| blocker.s + blocker_D * k / 4)
                                .map(|at| {
                                    projected_largest(
                                        &input,
                                        blocker_frame,
                                        blocker_id,
                                        &candidate,
                                        at,
                                    )
                                })
                                .sum::<usize>();
                            let saved_snapshot_score = [1_i64, 2, 3]
                                .iter()
                                .map(|&k| blocker.s + blocker_D * k / 4)
                                .map(|at| {
                                    projected_largest(
                                        &input,
                                        blocker_frame,
                                        blocker_id,
                                        &saved_cells,
                                        at,
                                    )
                                })
                                .sum::<usize>();
                            if snapshot_score > saved_snapshot_score {
                                target_projected_visible = true;
                            }
                            let (cutloss_field, saved_cutloss) = cutloss_context
                                .get_or_insert_with(|| {
                                    let field =
                                        projected_cut_loss_field(&input, blocker_frame, blocker_id);
                                    let saved = saved_cells
                                        .iter()
                                        .map(|&(x, y)| field[x * input.n + y])
                                        .sum();
                                    (field, saved)
                                });
                            let candidate_cutloss: usize = candidate
                                .iter()
                                .map(|&(x, y)| cutloss_field[x * input.n + y])
                                .sum();
                            if candidate_cutloss < *saved_cutloss {
                                target_cutloss_visible = true;
                            }
                            if !target_compact_box
                                && has_compact_box(&input, &counter_occupied, target.p)
                            {
                                target_compact_box = true;
                            }
                            let candidate_target_projection = projected_largest(
                                &input,
                                blocker_frame,
                                blocker_id,
                                &candidate,
                                target.s,
                            );
                            let saved_target_projection = projected_largest(
                                &input,
                                blocker_frame,
                                blocker_id,
                                &saved_cells,
                                target.s,
                            );
                            if candidate_target_projection > saved_target_projection {
                                target_projection_visible = true;
                            }
                            if !target_rescued {
                                let blocker_P = input.groups[blocker_id].p;
                                aggregate.rescued_blocker_bucket[size_bucket(blocker_P)] += 1;
                                let blocker_D =
                                    input.groups[blocker_id].t - input.groups[blocker_id].s;
                                aggregate.rescued_blocker_D_bucket[duration_bucket(blocker_D)] += 1;
                                let slack = perimeter(&saved_cells) - minimum_perimeter(blocker_P);
                                if slack <= 6 {
                                    aggregate.rescued_blocker_slack_le6 += 1;
                                } else {
                                    aggregate.rescued_blocker_slack_ge8 += 1;
                                }
                            }
                            target_rescued = true;
                            if representative_count < 20 {
                                println!(
                                    "[rescue] case={} target={} P={} q={:.3} base_largest={} blocker={} removed_largest={} blocker_P={} anchor=({}, {}) counter_largest={}",
                                    basename.to_string_lossy(),
                                    target_id,
                                    target.p,
                                    q,
                                    base_largest,
                                    blocker_id,
                                    removed_largest,
                                    input.groups[blocker_id].p,
                                    anchor_x,
                                    anchor_y,
                                    counter_largest,
                                );
                                representative_count += 1;
                            }
                        }
                    }
                }
            }
            aggregate.targets_with_safe_alternative += usize::from(target_has_safe);
            if target_rescued {
                let target_ideal_fee = ideal_fee(target.v, target.p) as i128;
                aggregate.targets_rescued += 1;
                aggregate.rescued_ideal_fee += target_ideal_fee;
                aggregate.rescued_bucket[size_bucket(target.p)] += 1;
                if target_slot_visible {
                    aggregate.rescued_slot_visible += 1;
                } else {
                    aggregate.rescued_slot_blind += 1;
                }
                if target_projected_visible {
                    aggregate.rescued_projected_visible += 1;
                    aggregate.rescued_projected_visible_fee += target_ideal_fee;
                } else {
                    aggregate.rescued_projected_blind += 1;
                    aggregate.rescued_projected_blind_fee += target_ideal_fee;
                }
                if target_cutloss_visible {
                    aggregate.rescued_cutloss_visible += 1;
                    aggregate.rescued_cutloss_visible_fee += target_ideal_fee;
                } else {
                    aggregate.rescued_cutloss_blind += 1;
                }
                if target_compact_box {
                    aggregate.rescued_compact_box += 1;
                    aggregate.rescued_compact_box_fee += target_ideal_fee;
                }
                aggregate.rescued_target_projection_visible +=
                    usize::from(target_projection_visible);
            }
        }
    }

    println!("cases={}", aggregate.cases);
    println!(
        "high_q_fragmented={} avg_per_case={:.3}",
        aggregate.high_q_fragmented,
        aggregate.high_q_fragmented as f64 / aggregate.cases as f64
    );
    println!(
        "targets_with_blocker={} blocker_links={}",
        aggregate.targets_with_blocker, aggregate.blocker_links
    );
    println!(
        "translated_candidates={} history_safe_candidates={} component_improving_candidates={} rescuing_candidates={}",
        aggregate.translated_candidates,
        aggregate.history_safe_candidates,
        aggregate.component_improving_candidates,
        aggregate.rescuing_candidates
    );
    println!(
        "targets_with_safe_alternative={} targets_rescued={} rescued_per_case={:.3} rescued_fraction={:.6} rescued_ideal_fee={} rescued_ideal_fee_per_case={:.3}",
        aggregate.targets_with_safe_alternative,
        aggregate.targets_rescued,
        aggregate.targets_rescued as f64 / aggregate.cases as f64,
        if aggregate.high_q_fragmented == 0 {
            0.0
        } else {
            aggregate.targets_rescued as f64 / aggregate.high_q_fragmented as f64
        },
        aggregate.rescued_ideal_fee,
        aggregate.rescued_ideal_fee as f64 / aggregate.cases as f64,
    );
    println!(
        "target_bucket_4_63={} target_bucket_64_95={} target_bucket_96_127={} target_bucket_128_150={}",
        aggregate.target_bucket[0],
        aggregate.target_bucket[1],
        aggregate.target_bucket[2],
        aggregate.target_bucket[3],
    );
    println!(
        "rescued_bucket_4_63={} rescued_bucket_64_95={} rescued_bucket_96_127={} rescued_bucket_128_150={}",
        aggregate.rescued_bucket[0],
        aggregate.rescued_bucket[1],
        aggregate.rescued_bucket[2],
        aggregate.rescued_bucket[3],
    );
    println!(
        "rescued_blocker_bucket_4_63={} rescued_blocker_bucket_64_95={} rescued_blocker_bucket_96_127={} rescued_blocker_bucket_128_150={} blocker_slack_le6={} blocker_slack_ge8={}",
        aggregate.rescued_blocker_bucket[0],
        aggregate.rescued_blocker_bucket[1],
        aggregate.rescued_blocker_bucket[2],
        aggregate.rescued_blocker_bucket[3],
        aggregate.rescued_blocker_slack_le6,
        aggregate.rescued_blocker_slack_ge8,
    );
    println!(
        "rescued_slot_visible={} rescued_slot_blind={}",
        aggregate.rescued_slot_visible, aggregate.rescued_slot_blind,
    );
    println!(
        "rescued_projected_visible={} rescued_projected_blind={} visible_ideal_fee={} visible_ideal_fee_per_case={:.3} blind_ideal_fee={} rescued_target_projection_visible={}",
        aggregate.rescued_projected_visible,
        aggregate.rescued_projected_blind,
        aggregate.rescued_projected_visible_fee,
        aggregate.rescued_projected_visible_fee as f64 / aggregate.cases as f64,
        aggregate.rescued_projected_blind_fee,
        aggregate.rescued_target_projection_visible,
    );
    println!(
        "rescued_cutloss_visible={} rescued_cutloss_blind={} visible_ideal_fee={} visible_ideal_fee_per_case={:.3}",
        aggregate.rescued_cutloss_visible,
        aggregate.rescued_cutloss_blind,
        aggregate.rescued_cutloss_visible_fee,
        aggregate.rescued_cutloss_visible_fee as f64 / aggregate.cases as f64,
    );
    println!(
        "rescued_compact_box={} compact_box_ideal_fee={} compact_box_ideal_fee_per_case={:.3}",
        aggregate.rescued_compact_box,
        aggregate.rescued_compact_box_fee,
        aggregate.rescued_compact_box_fee as f64 / aggregate.cases as f64,
    );
    println!(
        "rescued_blocker_D_lt1000={} rescued_blocker_D_1000_2999={} rescued_blocker_D_3000_5999={} rescued_blocker_D_6000_plus={}",
        aggregate.rescued_blocker_D_bucket[0],
        aggregate.rescued_blocker_D_bucket[1],
        aggregate.rescued_blocker_D_bucket[2],
        aggregate.rescued_blocker_D_bucket[3],
    );
}
