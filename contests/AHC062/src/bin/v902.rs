use proconio::input;

const STRIP_W: usize = 5;
const MARGIN: usize = 3;
const NEG_INF: i64 = i64::MIN / 4;

#[derive(Clone, Copy)]
struct Symmetry {
    flip_row: bool,
    flip_col: bool,
}

const SYMMETRIES: [Symmetry; 4] = [
    Symmetry {
        flip_row: false,
        flip_col: false,
    },
    Symmetry {
        flip_row: true,
        flip_col: false,
    },
    Symmetry {
        flip_row: false,
        flip_col: true,
    },
    Symmetry {
        flip_row: true,
        flip_col: true,
    },
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
}

#[derive(Clone, Copy)]
struct Segment {
    c0: usize,
    c1: usize,
}

#[derive(Clone)]
struct ConnectorPlan {
    p: usize,
    from_port: usize,
    to_port: usize,
    cells: Vec<usize>,
}

#[derive(Clone)]
struct SidePlan {
    suffix_cells: Vec<usize>,
    suffix_start_port: Option<usize>,
    connectors: Vec<ConnectorPlan>,
}

#[derive(Clone)]
enum TaskKind {
    Connector {
        p: usize,
        from_strip: usize,
        to_strip: usize,
    },
}

#[derive(Clone)]
struct Task {
    kind: TaskKind,
}

#[derive(Clone)]
struct StripSolution {
    rows: Vec<[u8; STRIP_W]>,
}

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    let weights = flatten_weights(n, &a);
    let perms = generate_permutations();
    let transitions = build_transitions(&perms);

    let strip_count = n / STRIP_W;
    let strip_seq_down =
        find_feasible_strip_sequence(n, strip_count, true).unwrap_or_else(|| build_strip_sequence(strip_count));
    let strip_seq_up =
        find_feasible_strip_sequence(n, strip_count, false).unwrap_or_else(|| build_strip_sequence(strip_count));

    let mut best_route = Vec::new();
    let mut best_score = NEG_INF;

    for symmetry in SYMMETRIES {
        let transformed = transform_weights(n, &weights, symmetry);
        for &start_down in &[true, false] {
            let strip_seq = if start_down {
                &strip_seq_down
            } else {
                &strip_seq_up
            };
            if let Some(route_tr) = build_route(
                n,
                &transformed,
                strip_seq,
                start_down,
                &perms,
                &transitions,
            ) {
                let score = compute_raw_score(&route_tr, &transformed);
                if score > best_score {
                    best_score = score;
                    best_route = map_route_back(&route_tr, n, symmetry);
                }
            }
        }
    }

    if best_route.is_empty() {
        eprintln!("v902: fallback");
        best_route = fallback_route(n);
    }

    for cell in best_route {
        println!("{} {}", cell / n, cell % n);
    }
}

fn flatten_weights(n: usize, a: &[Vec<i64>]) -> Vec<i64> {
    let mut weights = vec![0; n * n];
    for i in 0..n {
        for j in 0..n {
            weights[i * n + j] = a[i][j];
        }
    }
    weights
}

fn build_route(
    n: usize,
    weights: &[i64],
    strip_seq: &[usize],
    start_down: bool,
    perms: &[[usize; STRIP_W]],
    transitions: &[Vec<usize>],
) -> Option<Vec<usize>> {
    if n % STRIP_W != 0 || n < 2 * MARGIN + 1 || n % 2 == 1 {
        return None;
    }

    let strip_count = n / STRIP_W;
    let pass_count = strip_seq.len();
    if pass_count != strip_count * STRIP_W {
        return None;
    }
    for p in 0..pass_count.saturating_sub(1) {
        if strip_seq[p].abs_diff(strip_seq[p + 1]) != 1 {
            return None;
        }
    }

    let mut pass_dir_down = vec![false; pass_count];
    for (p, dir) in pass_dir_down.iter_mut().enumerate() {
        *dir = if p % 2 == 0 { start_down } else { !start_down };
    }

    let mut pass_occ = vec![0usize; pass_count];
    let mut occ_to_pass = vec![[usize::MAX; STRIP_W]; strip_count];
    let mut occ_counter = vec![0usize; strip_count];
    for p in 0..pass_count {
        let s = strip_seq[p];
        let occ = occ_counter[s];
        if occ >= STRIP_W {
            return None;
        }
        pass_occ[p] = occ;
        occ_to_pass[s][occ] = p;
        occ_counter[s] += 1;
    }
    if occ_counter.iter().any(|&c| c != STRIP_W) {
        return None;
    }

    let suffix_side = if start_down { Side::Top } else { Side::Bottom };
    let top_plan = match plan_side(
        n,
        Side::Top,
        strip_seq,
        &pass_dir_down,
        suffix_side == Side::Top,
    ) {
        Some(v) => v,
        None => return None,
    };
    let bottom_plan = match plan_side(
        n,
        Side::Bottom,
        strip_seq,
        &pass_dir_down,
        suffix_side == Side::Bottom,
    ) {
        Some(v) => v,
        None => return None,
    };

    let mut top_port = vec![None; pass_count];
    let mut bottom_port = vec![None; pass_count];
    let mut connector_cells = vec![Vec::<usize>::new(); pass_count.saturating_sub(1)];

    for c in &top_plan.connectors {
        if !set_port(&mut top_port, c.p, c.from_port) {
            return None;
        }
        if !set_port(&mut top_port, c.p + 1, c.to_port) {
            return None;
        }
        connector_cells[c.p] = c.cells.clone();
    }
    for c in &bottom_plan.connectors {
        if !set_port(&mut bottom_port, c.p, c.from_port) {
            return None;
        }
        if !set_port(&mut bottom_port, c.p + 1, c.to_port) {
            return None;
        }
        connector_cells[c.p] = c.cells.clone();
    }
    if connector_cells.iter().any(|cells| cells.is_empty()) {
        return None;
    }

    let last_pass = pass_count - 1;
    let suffix_cells = if suffix_side == Side::Top {
        let port = match top_plan.suffix_start_port {
            Some(v) => v,
            None => return None,
        };
        if !set_port(&mut top_port, last_pass, port) {
            return None;
        }
        top_plan.suffix_cells.clone()
    } else {
        let port = match bottom_plan.suffix_start_port {
            Some(v) => v,
            None => return None,
        };
        if !set_port(&mut bottom_port, last_pass, port) {
            return None;
        }
        bottom_plan.suffix_cells.clone()
    };

    let core_h = n - 2 * MARGIN;
    let core_top = MARGIN;
    let mut pass_start = vec![0usize; pass_count];
    pass_start[0] = 0;
    for p in 0..pass_count - 1 {
        pass_start[p + 1] = pass_start[p] + core_h + connector_cells[p].len();
    }

    let mut strip_sol = Vec::with_capacity(strip_count);
    for (s, _) in occ_to_pass.iter().enumerate().take(strip_count) {
        let sol = match solve_strip_paths(
            n,
            weights,
            s,
            &occ_to_pass[s],
            &pass_start,
            &pass_dir_down,
            &top_port,
            &bottom_port,
            perms,
            transitions,
        ) {
            Some(v) => v,
            None => return None,
        };
        strip_sol.push(sol);
    }

    let mut route = Vec::with_capacity(n * n);

    for p in 0..pass_count {
        let s = strip_seq[p];
        let occ = pass_occ[p];
        if pass_dir_down[p] {
            for r in 0..core_h {
                let local_col = strip_sol[s].rows[r][occ] as usize;
                route.push((core_top + r) * n + s * STRIP_W + local_col);
            }
        } else {
            for r in (0..core_h).rev() {
                let local_col = strip_sol[s].rows[r][occ] as usize;
                route.push((core_top + r) * n + s * STRIP_W + local_col);
            }
        }
        if p + 1 < pass_count {
            route.extend(connector_cells[p].iter().copied());
        }
    }
    route.extend(suffix_cells.iter().copied());

    if !is_valid_route(&route, n) {
        return None;
    }
    Some(route)
}

fn set_port(ports: &mut [Option<usize>], idx: usize, v: usize) -> bool {
    match ports[idx] {
        None => {
            ports[idx] = Some(v);
            true
        }
        Some(cur) => cur == v,
    }
}

fn plan_side(
    n: usize,
    side: Side,
    strip_seq: &[usize],
    pass_dir_down: &[bool],
    needs_suffix: bool,
) -> Option<SidePlan> {
    let pass_count = strip_seq.len();
    let segment_count = n / 2;

    let mut connector_indices = Vec::new();
    for (p, &dir) in pass_dir_down
        .iter()
        .enumerate()
        .take(pass_count.saturating_sub(1))
    {
        let connector_side = if dir { Side::Bottom } else { Side::Top };
        if connector_side == side {
            connector_indices.push(p);
        }
    }

    let mut tasks = Vec::new();
    for &p in &connector_indices {
        tasks.push(Task {
            kind: TaskKind::Connector {
                p,
                from_strip: strip_seq[p],
                to_strip: strip_seq[p + 1],
            },
        });
    }

    let mut segments = Vec::with_capacity(segment_count);
    for k in 0..segment_count {
        segments.push(Segment {
            c0: 2 * k,
            c1: 2 * k + 1,
        });
    }

    let last_strip = strip_seq[pass_count - 1];
    let suffix_candidates: Vec<Option<usize>> = if needs_suffix {
        (0..segment_count).map(Some).collect()
    } else {
        vec![None]
    };

    for excluded_suffix in suffix_candidates {
        let task_to_seg = match build_segment_matching(n, &tasks, &segments, excluded_suffix) {
            Some(v) => v,
            None => continue,
        };

        let mut connector_meta = Vec::with_capacity(tasks.len());
        for ti in 0..tasks.len() {
            let seg = segments[task_to_seg[ti]];
            let TaskKind::Connector {
                p,
                from_strip,
                to_strip,
            } = tasks[ti].kind;
            let (from_ep, to_ep) =
                match choose_connector_orientation_row1(n, seg, from_strip, to_strip) {
                    Some(v) => v,
                    None => {
                        connector_meta.clear();
                        break;
                    }
                };
            connector_meta.push((p, from_strip, to_strip, from_ep, to_ep));
        }
        if connector_meta.len() != tasks.len() {
            continue;
        }

        let mut suffix_ep = None;
        if let Some(si) = excluded_suffix {
            let seg = segments[si];
            suffix_ep = choose_suffix_orientation_row1(n, seg, last_strip);
            if suffix_ep.is_none() {
                continue;
            }
        }

        // event: (pass_idx, strip_opt, row1_endpoint_col)
        let mut event_pass = Vec::<Option<usize>>::new();
        let mut event_strip = Vec::<Option<usize>>::new();
        let mut event_ep = Vec::<usize>::new();
        let mut connector_event_idx = Vec::<(usize, usize)>::new();
        for &(p, from_strip, to_strip, from_ep, to_ep) in &connector_meta {
            let e_from = event_pass.len();
            event_pass.push(Some(p));
            event_strip.push(Some(from_strip));
            event_ep.push(from_ep);

            let e_to = event_pass.len();
            event_pass.push(Some(p + 1));
            event_strip.push(Some(to_strip));
            event_ep.push(to_ep);

            connector_event_idx.push((e_from, e_to));
        }

        let mut suffix_start_event = None;
        let mut suffix_terminal_event = None;
        if let Some((suf_start_ep, suf_end_ep)) = suffix_ep {
            let e_start = event_pass.len();
            event_pass.push(Some(pass_count - 1));
            event_strip.push(Some(last_strip));
            event_ep.push(suf_start_ep);

            let e_term = event_pass.len();
            event_pass.push(None);
            event_strip.push(None);
            event_ep.push(suf_end_ep);

            suffix_start_event = Some(e_start);
            suffix_terminal_event = Some(e_term);
        }

        if event_pass.len() != n {
            continue;
        }

        let event_to_row2 = match build_row2_matching(n, &event_strip, &event_ep) {
            Some(v) => v,
            None => continue,
        };

        let mut connectors = Vec::with_capacity(connector_meta.len());
        for (idx, &(p, _fs, _ts, from_ep, to_ep)) in connector_meta.iter().enumerate() {
            let (e_from, e_to) = connector_event_idx[idx];
            let from_row2 = event_to_row2[e_from];
            let to_row2 = event_to_row2[e_to];
            let cells = build_connector_cells(n, side, from_row2, from_ep, to_ep, to_row2);
            connectors.push(ConnectorPlan {
                p,
                from_port: from_row2,
                to_port: to_row2,
                cells,
            });
        }

        let (suffix_cells, suffix_start_port) = if let (Some(e_start), Some(e_term), Some((suf_start_ep, suf_end_ep))) =
            (suffix_start_event, suffix_terminal_event, suffix_ep)
        {
            let start_row2 = event_to_row2[e_start];
            let end_row2 = event_to_row2[e_term];
            (
                build_connector_cells(n, side, start_row2, suf_start_ep, suf_end_ep, end_row2),
                Some(start_row2),
            )
        } else {
            (Vec::new(), None)
        };

        return Some(SidePlan {
            suffix_cells,
            suffix_start_port,
            connectors,
        });
    }

    None
}

fn build_segment_matching(
    n: usize,
    tasks: &[Task],
    segments: &[Segment],
    excluded_seg: Option<usize>,
) -> Option<Vec<usize>> {
    let mut candidates = vec![Vec::<usize>::new(); tasks.len()];
    for ti in 0..tasks.len() {
        for (si, &seg) in segments.iter().enumerate() {
            if excluded_seg == Some(si) {
                continue;
            }
            if task_can_use_segment_row1(n, &tasks[ti], seg) {
                candidates[ti].push(si);
            }
        }
        if candidates[ti].is_empty() {
            return None;
        }
    }

    let mut order: Vec<usize> = (0..tasks.len()).collect();
    order.sort_by_key(|&ti| candidates[ti].len());

    let mut seg_match = vec![None; segments.len()];
    for ti in order {
        let mut seen = vec![false; segments.len()];
        if !augment_match(ti, &candidates, &mut seg_match, &mut seen) {
            return None;
        }
    }

    let mut task_to_seg = vec![usize::MAX; tasks.len()];
    for (si, &mt) in seg_match.iter().enumerate() {
        if let Some(ti) = mt {
            task_to_seg[ti] = si;
        }
    }
    if task_to_seg.contains(&usize::MAX) {
        return None;
    }
    Some(task_to_seg)
}

fn build_row2_matching(
    n: usize,
    event_strip: &[Option<usize>],
    event_ep: &[usize],
) -> Option<Vec<usize>> {
    let event_count = event_strip.len();
    let mut candidates = vec![Vec::<usize>::new(); event_count];
    for ei in 0..event_count {
        for col in 0..n {
            if col.abs_diff(event_ep[ei]) > 1 {
                continue;
            }
            if let Some(strip) = event_strip[ei] && !in_strip_range(col, strip, n) {
                continue;
            }
            candidates[ei].push(col);
        }
        if candidates[ei].is_empty() {
            return None;
        }
    }

    let mut order: Vec<usize> = (0..event_count).collect();
    order.sort_by_key(|&ei| candidates[ei].len());

    let mut col_match = vec![None; n];
    for ei in order {
        let mut seen = vec![false; n];
        if !augment_match(ei, &candidates, &mut col_match, &mut seen) {
            return None;
        }
    }

    let mut event_to_col = vec![usize::MAX; event_count];
    for (col, &me) in col_match.iter().enumerate() {
        if let Some(ei) = me {
            event_to_col[ei] = col;
        }
    }
    if event_to_col.contains(&usize::MAX) {
        return None;
    }
    Some(event_to_col)
}

fn augment_match(
    ti: usize,
    candidates: &[Vec<usize>],
    seg_match: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for &si in &candidates[ti] {
        if seen[si] {
            continue;
        }
        seen[si] = true;
        if seg_match[si].is_none() || augment_match(seg_match[si].unwrap(), candidates, seg_match, seen)
        {
            seg_match[si] = Some(ti);
            return true;
        }
    }
    false
}

fn task_can_use_segment_row1(n: usize, task: &Task, seg: Segment) -> bool {
    let TaskKind::Connector {
        from_strip,
        to_strip,
        ..
    } = task.kind;
    (in_row1_range(seg.c0, from_strip, n) && in_row1_range(seg.c1, to_strip, n))
        || (in_row1_range(seg.c1, from_strip, n) && in_row1_range(seg.c0, to_strip, n))
}

fn choose_suffix_orientation_row1(n: usize, seg: Segment, strip: usize) -> Option<(usize, usize)> {
    let mut cand = Vec::new();
    let center = strip_center(strip) as i32;
    if in_row1_range(seg.c0, strip, n) {
        let pen = (seg.c0 as i32 - center).abs() as i64;
        cand.push((pen, seg.c0, seg.c1));
    }
    if in_row1_range(seg.c1, strip, n) {
        let pen = (seg.c1 as i32 - center).abs() as i64;
        cand.push((pen, seg.c1, seg.c0));
    }
    cand.sort_by_key(|x| x.0);
    cand.first().map(|&(_, s, e)| (s, e))
}

fn choose_connector_orientation_row1(
    n: usize,
    seg: Segment,
    from_strip: usize,
    to_strip: usize,
) -> Option<(usize, usize)> {
    let mut cand = Vec::new();
    let c_from = strip_center(from_strip) as i32;
    let c_to = strip_center(to_strip) as i32;

    if in_row1_range(seg.c0, from_strip, n) && in_row1_range(seg.c1, to_strip, n) {
        let pen = (seg.c0 as i32 - c_from).abs() as i64 + (seg.c1 as i32 - c_to).abs() as i64;
        cand.push((pen, seg.c0, seg.c1));
    }
    if in_row1_range(seg.c1, from_strip, n) && in_row1_range(seg.c0, to_strip, n) {
        let pen = (seg.c1 as i32 - c_from).abs() as i64 + (seg.c0 as i32 - c_to).abs() as i64;
        cand.push((pen, seg.c1, seg.c0));
    }

    cand.sort_by_key(|x| x.0);
    cand.first().map(|&(_, s, e)| (s, e))
}

fn build_connector_cells(
    n: usize,
    side: Side,
    start_row2_col: usize,
    start_row1_col: usize,
    end_row1_col: usize,
    end_row2_col: usize,
) -> Vec<usize> {
    let (left, right, forward, from_row2, to_row2) = if start_row1_col + 1 == end_row1_col {
        (
            start_row1_col,
            end_row1_col,
            true,
            start_row2_col,
            end_row2_col,
        )
    } else if end_row1_col + 1 == start_row1_col {
        (
            end_row1_col,
            start_row1_col,
            false,
            end_row2_col,
            start_row2_col,
        )
    } else {
        panic!("segment endpoints must be adjacent");
    };

    let cells = match side {
        Side::Top => {
            let rr2 = MARGIN - 1;
            let rr1 = MARGIN - 2;
            let rr0 = MARGIN - 3;
            vec![
                rr2 * n + from_row2,
                rr1 * n + left,
                rr0 * n + left,
                rr0 * n + right,
                rr1 * n + right,
                rr2 * n + to_row2,
            ]
        }
        Side::Bottom => {
            let rr2 = n - MARGIN;
            let rr1 = n - MARGIN + 1;
            let rr0 = n - 1;
            vec![
                rr2 * n + from_row2,
                rr1 * n + left,
                rr0 * n + left,
                rr0 * n + right,
                rr1 * n + right,
                rr2 * n + to_row2,
            ]
        }
    };

    if forward {
        cells
    } else {
        let mut rev = cells;
        rev.reverse();
        rev
    }
}

fn strip_center(strip: usize) -> usize {
    strip * STRIP_W + STRIP_W / 2
}

fn in_row1_range(col: usize, strip: usize, n: usize) -> bool {
    let base = strip * STRIP_W;
    let lo = base.saturating_sub(2);
    let hi = (base + STRIP_W + 1).min(n - 1);
    lo <= col && col <= hi
}

fn in_strip_range(col: usize, strip: usize, n: usize) -> bool {
    let base = strip * STRIP_W;
    let lo = base.saturating_sub(1);
    let hi = (base + STRIP_W).min(n - 1);
    lo <= col && col <= hi
}

fn solve_strip_paths(
    n: usize,
    weights: &[i64],
    strip: usize,
    occ_to_pass: &[usize; STRIP_W],
    pass_start: &[usize],
    pass_dir_down: &[bool],
    top_port: &[Option<usize>],
    bottom_port: &[Option<usize>],
    perms: &[[usize; STRIP_W]],
    transitions: &[Vec<usize>],
) -> Option<StripSolution> {
    let state_count = perms.len();
    let core_top = MARGIN;
    let core_h = n - 2 * MARGIN;
    let strip_col0 = strip * STRIP_W;

    let mut coeff = vec![[0i64; STRIP_W]; core_h];
    for (occ, &pass) in occ_to_pass.iter().enumerate().take(STRIP_W) {
        let base = pass_start[pass] as i64;
        if pass_dir_down[pass] {
            for (r, coeff_row) in coeff.iter_mut().enumerate().take(core_h) {
                coeff_row[occ] = base + r as i64;
            }
        } else {
            for (r, coeff_row) in coeff.iter_mut().enumerate().take(core_h) {
                coeff_row[occ] = base + (core_h - 1 - r) as i64;
            }
        }
    }

    let mut row_score = vec![vec![NEG_INF; state_count]; core_h];
    for r in 0..core_h {
        let global_row = core_top + r;
        for (idx, perm) in perms.iter().enumerate() {
            let mut value = 0i64;
            let mut ok = true;
            for occ in 0..STRIP_W {
                let col = perm[occ];
                let global_col = strip_col0 + col;
                let pass = occ_to_pass[occ];

                if r == 0 && let Some(port) = top_port[pass] && global_col.abs_diff(port) > 1 {
                    ok = false;
                    break;
                }
                if r + 1 == core_h
                    && let Some(port) = bottom_port[pass]
                    && global_col.abs_diff(port) > 1
                {
                    ok = false;
                    break;
                }

                value += coeff[r][occ] * weights[global_row * n + global_col];
            }
            if ok {
                row_score[r][idx] = value;
            }
        }
    }

    let mut dp_prev = vec![NEG_INF; state_count];
    let mut dp_cur = vec![NEG_INF; state_count];
    let mut parent = vec![vec![u16::MAX; state_count]; core_h];
    dp_prev[..state_count].copy_from_slice(&row_score[0][..state_count]);

    for r in 1..core_h {
        dp_cur.fill(NEG_INF);
        for cur in 0..state_count {
            if row_score[r][cur] == NEG_INF {
                continue;
            }
            let mut best_val = NEG_INF;
            let mut best_prev = u16::MAX;
            for &prev in &transitions[cur] {
                if dp_prev[prev] == NEG_INF {
                    continue;
                }
                let cand = dp_prev[prev] + row_score[r][cur];
                if cand > best_val {
                    best_val = cand;
                    best_prev = prev as u16;
                }
            }
            dp_cur[cur] = best_val;
            parent[r][cur] = best_prev;
        }
        std::mem::swap(&mut dp_prev, &mut dp_cur);
    }

    let mut best_last = usize::MAX;
    let mut best_val = NEG_INF;
    for (idx, &v) in dp_prev.iter().enumerate().take(state_count) {
        if v > best_val {
            best_val = v;
            best_last = idx;
        }
    }
    if best_last == usize::MAX || best_val == NEG_INF {
        return None;
    }

    let mut rows = vec![[0u8; STRIP_W]; core_h];
    let mut cur = best_last;
    for r in (0..core_h).rev() {
        for occ in 0..STRIP_W {
            rows[r][occ] = perms[cur][occ] as u8;
        }
        if r > 0 {
            let prev = parent[r][cur];
            if prev == u16::MAX {
                return None;
            }
            cur = prev as usize;
        }
    }

    Some(StripSolution { rows })
}

fn build_strip_sequence(strip_count: usize) -> Vec<usize> {
    build_euler_like_strip_sequence(strip_count, STRIP_W)
}

fn find_feasible_strip_sequence(
    n: usize,
    strip_count: usize,
    start_down: bool,
) -> Option<Vec<usize>> {
    for seed in 0..512u64 {
        let seq = build_euler_like_strip_sequence_with_seed(strip_count, STRIP_W, seed + 1);
        if !is_strip_sequence_feasible(n, &seq, start_down) {
            continue;
        }
        return Some(seq);
    }

    for seed in 1..=200u64 {
        let seq = match build_random_balanced_sequence(strip_count, STRIP_W, seed) {
            Some(v) => v,
            None => continue,
        };
        if is_strip_sequence_feasible(n, &seq, start_down) {
            return Some(seq);
        }
    }
    None
}

fn is_strip_sequence_feasible(n: usize, strip_seq: &[usize], start_down: bool) -> bool {
    let pass_count = strip_seq.len();
    if pass_count == 0 || pass_count % STRIP_W != 0 {
        return false;
    }
    for p in 0..pass_count - 1 {
        if strip_seq[p].abs_diff(strip_seq[p + 1]) != 1 {
            return false;
        }
    }

    let mut pass_dir_down = vec![false; pass_count];
    for (p, dir) in pass_dir_down.iter_mut().enumerate() {
        *dir = if p % 2 == 0 { start_down } else { !start_down };
    }
    let suffix_side = if start_down { Side::Top } else { Side::Bottom };
    let top_ok = plan_side(n, Side::Top, strip_seq, &pass_dir_down, suffix_side == Side::Top)
        .is_some();
    let bottom_ok = plan_side(
        n,
        Side::Bottom,
        strip_seq,
        &pass_dir_down,
        suffix_side == Side::Bottom,
    )
    .is_some();
    top_ok && bottom_ok
}

fn build_euler_like_strip_sequence(strip_count: usize, repeats: usize) -> Vec<usize> {
    let start = 0usize;
    let end = strip_count - 1;

    let mut degree = vec![(2 * repeats) as i32; strip_count];
    degree[start] -= 1;
    degree[end] -= 1;

    let mut mult = vec![0i32; strip_count - 1];
    mult[0] = degree[0];
    for i in 1..strip_count - 1 {
        mult[i] = degree[i] - mult[i - 1];
    }

    let mut flow = vec![0i32; strip_count - 1];
    for x in flow.iter_mut().take(end).skip(start) {
        *x = 1;
    }

    let mut right = vec![0i32; strip_count - 1];
    let mut left = vec![0i32; strip_count - 1];
    for i in 0..strip_count - 1 {
        right[i] = (mult[i] + flow[i]) / 2;
        left[i] = (mult[i] - flow[i]) / 2;
    }

    let mut stack = vec![start];
    let mut out = Vec::with_capacity(strip_count * repeats);
    while let Some(&v) = stack.last() {
        let mut moved = false;
        if v > 0 && left[v - 1] > 0 {
            left[v - 1] -= 1;
            stack.push(v - 1);
            moved = true;
        } else if v + 1 < strip_count && right[v] > 0 {
            right[v] -= 1;
            stack.push(v + 1);
            moved = true;
        }
        if !moved {
            out.push(v);
            stack.pop();
        }
    }
    out.reverse();
    out
}

fn build_euler_like_strip_sequence_with_seed(
    strip_count: usize,
    repeats: usize,
    seed: u64,
) -> Vec<usize> {
    let start = 0usize;
    let end = strip_count - 1;

    let mut degree = vec![(2 * repeats) as i32; strip_count];
    degree[start] -= 1;
    degree[end] -= 1;

    let mut mult = vec![0i32; strip_count - 1];
    mult[0] = degree[0];
    for i in 1..strip_count - 1 {
        mult[i] = degree[i] - mult[i - 1];
    }

    let mut flow = vec![0i32; strip_count - 1];
    for x in flow.iter_mut().take(end).skip(start) {
        *x = 1;
    }

    let mut right = vec![0i32; strip_count - 1];
    let mut left = vec![0i32; strip_count - 1];
    for i in 0..strip_count - 1 {
        right[i] = (mult[i] + flow[i]) / 2;
        left[i] = (mult[i] - flow[i]) / 2;
    }

    let mut rng_state = seed.max(1);
    let mut stack = vec![start];
    let mut out = Vec::with_capacity(strip_count * repeats);
    while let Some(&v) = stack.last() {
        let can_left = v > 0 && left[v - 1] > 0;
        let can_right = v + 1 < strip_count && right[v] > 0;

        if can_left && can_right {
            if next_rand_bit(&mut rng_state) == 0 {
                left[v - 1] -= 1;
                stack.push(v - 1);
            } else {
                right[v] -= 1;
                stack.push(v + 1);
            }
            continue;
        }
        if can_left {
            left[v - 1] -= 1;
            stack.push(v - 1);
            continue;
        }
        if can_right {
            right[v] -= 1;
            stack.push(v + 1);
            continue;
        }

        out.push(v);
        stack.pop();
    }
    out.reverse();
    out
}

fn next_rand_bit(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 7;
    x ^= x >> 9;
    x ^= x << 8;
    if x == 0 {
        x = 0x9E37_79B9_7F4A_7C15;
    }
    *state = x;
    x & 1
}

fn build_random_balanced_sequence(
    strip_count: usize,
    repeats: usize,
    seed: u64,
) -> Option<Vec<usize>> {
    let total = strip_count * repeats;
    let mut counts = vec![repeats as i32; strip_count];
    let mut rng = seed.max(1);
    let start = (next_rand_u64(&mut rng) as usize) % strip_count;
    counts[start] -= 1;
    let mut seq = Vec::with_capacity(total);
    seq.push(start);

    while seq.len() < total {
        let cur = *seq.last().unwrap();
        let mut cand = Vec::with_capacity(2);
        if cur > 0 && counts[cur - 1] > 0 {
            cand.push(cur - 1);
        }
        if cur + 1 < strip_count && counts[cur + 1] > 0 {
            cand.push(cur + 1);
        }
        if cand.is_empty() {
            return None;
        }

        let nx = if cand.len() == 1 {
            cand[0]
        } else {
            let a = cand[0];
            let b = cand[1];
            let sa = counts[a] * 4 + (next_rand_u64(&mut rng) as i32 & 3);
            let sb = counts[b] * 4 + (next_rand_u64(&mut rng) as i32 & 3);
            if sb > sa { b } else { a }
        };

        counts[nx] -= 1;
        seq.push(nx);
    }
    if counts.iter().all(|&x| x == 0) {
        Some(seq)
    } else {
        None
    }
}

fn next_rand_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 7;
    x ^= x >> 9;
    x ^= x << 8;
    if x == 0 {
        x = 0x9E37_79B9_7F4A_7C15;
    }
    *state = x;
    x
}

fn generate_permutations() -> Vec<[usize; STRIP_W]> {
    let mut perms = Vec::new();
    let mut cur = [0usize; STRIP_W];
    let mut used = [false; STRIP_W];
    dfs_permutations(0, &mut used, &mut cur, &mut perms);
    perms
}

fn dfs_permutations(
    depth: usize,
    used: &mut [bool; STRIP_W],
    cur: &mut [usize; STRIP_W],
    perms: &mut Vec<[usize; STRIP_W]>,
) {
    if depth == STRIP_W {
        perms.push(*cur);
        return;
    }
    for col in 0..STRIP_W {
        if used[col] {
            continue;
        }
        used[col] = true;
        cur[depth] = col;
        dfs_permutations(depth + 1, used, cur, perms);
        used[col] = false;
    }
}

fn build_transitions(perms: &[[usize; STRIP_W]]) -> Vec<Vec<usize>> {
    let state_count = perms.len();
    let mut transitions = vec![Vec::new(); state_count];
    for cur in 0..state_count {
        for prev in 0..state_count {
            if is_transition_valid(&perms[prev], &perms[cur]) {
                transitions[cur].push(prev);
            }
        }
    }
    transitions
}

fn is_transition_valid(prev: &[usize; STRIP_W], cur: &[usize; STRIP_W]) -> bool {
    for occ in 0..STRIP_W {
        if prev[occ].abs_diff(cur[occ]) > 1 {
            return false;
        }
    }
    true
}

fn transform_weights(n: usize, weights: &[i64], symmetry: Symmetry) -> Vec<i64> {
    let mut transformed = vec![0; n * n];
    for row in 0..n {
        for col in 0..n {
            let (orig_row, orig_col) = map_from_transformed(row, col, n, symmetry);
            transformed[row * n + col] = weights[orig_row * n + orig_col];
        }
    }
    transformed
}

fn map_route_back(route: &[usize], n: usize, symmetry: Symmetry) -> Vec<usize> {
    route
        .iter()
        .map(|&cell| {
            let row = cell / n;
            let col = cell % n;
            let (orig_row, orig_col) = map_from_transformed(row, col, n, symmetry);
            orig_row * n + orig_col
        })
        .collect()
}

fn map_from_transformed(row: usize, col: usize, n: usize, symmetry: Symmetry) -> (usize, usize) {
    let mapped_row = if symmetry.flip_row { n - 1 - row } else { row };
    let mapped_col = if symmetry.flip_col { n - 1 - col } else { col };
    (mapped_row, mapped_col)
}

fn compute_raw_score(route: &[usize], weights: &[i64]) -> i64 {
    route
        .iter()
        .enumerate()
        .map(|(idx, &cell)| idx as i64 * weights[cell])
        .sum()
}

fn is_valid_route(route: &[usize], n: usize) -> bool {
    let len = route.len();
    if len != n * n {
        return false;
    }
    let mut seen = vec![false; len];
    for &cell in route {
        if cell >= len || seen[cell] {
            return false;
        }
        seen[cell] = true;
    }
    for i in 0..len - 1 {
        if !is_adj(route[i], route[i + 1], n) {
            return false;
        }
    }
    true
}

fn is_adj(a: usize, b: usize, n: usize) -> bool {
    let ai = a / n;
    let aj = a % n;
    let bi = b / n;
    let bj = b % n;
    ai.abs_diff(bi).max(aj.abs_diff(bj)) == 1
}

fn fallback_route(n: usize) -> Vec<usize> {
    let mut route = Vec::with_capacity(n * n);
    for i in 0..n {
        if i % 2 == 0 {
            for j in 0..n {
                route.push(i * n + j);
            }
        } else {
            for j in (0..n).rev() {
                route.push(i * n + j);
            }
        }
    }
    route
}
