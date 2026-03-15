use proconio::input;
use std::env;
use std::time::{Duration, Instant};

const NEG_INF: i64 = i64::MIN / 4;
const ALPHA: [i64; 5] = [1, 2, 3, 4, 5];
const SEARCH_ROWS_TOP: usize = 7;
const SEARCH_ROWS_BOTTOM: usize = 192;
const MAX_SEGMENT_SPAN: usize = 240;
const TIME_LIMIT_MS: u64 = 2_800;
const SA_START_TEMP: f64 = 20_000_000.0;
const SA_END_TEMP: f64 = 1_000.0;

#[derive(Clone, Copy)]
struct Transform {
    transpose: bool,
    flip_row: bool,
    flip_col: bool,
}

struct XorShift64 {
    state: u64,
}

#[derive(Clone, Copy, Default)]
struct BucketStats {
    attempts: u64,
    accepted_uphill: u64,
    accepted_downhill: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        self.state = x;
        x
    }

    fn gen_range(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next_u64() % (upper as u64)) as usize
    }

    fn gen_f64(&mut self) -> f64 {
        const SCALE: f64 = (1u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / SCALE
    }
}

fn main() {
    let start = Instant::now();
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    assert_eq!(n, 200, "this construction assumes n = 200");

    let perms = generate_permutations5();
    let id_state = perms
        .iter()
        .position(|p| *p == [0, 1, 2, 3, 4])
        .expect("identity permutation must exist");
    let rev_state = perms
        .iter()
        .position(|p| *p == [4, 3, 2, 1, 0])
        .expect("reverse permutation must exist");
    let valid = build_transition_table(&perms);

    let mut best_score = i64::MIN;
    let mut best_path = Vec::new();
    for tr in all_transforms() {
        let transformed = transform_board(&a, tr);
        let path = solve_single(&transformed, &perms, &valid, [id_state, rev_state]);
        let original_path = map_path_to_original(&path, n, tr);
        let score = compute_score(&a, &original_path);
        if score > best_score {
            best_score = score;
            best_path = original_path;
        }
    }

    let initial_path = best_path.clone();
    let initial_score = best_score;
    let deadline = start + Duration::from_millis(TIME_LIMIT_MS);
    anneal_path(&mut best_path, &a, deadline, initial_score, &initial_path);

    assert_eq!(best_path.len(), n * n, "path length must be n^2");
    validate_path(n, &best_path);

    for (r, c) in best_path {
        println!("{r} {c}");
    }
}

fn all_transforms() -> [Transform; 8] {
    [
        Transform {
            transpose: false,
            flip_row: false,
            flip_col: false,
        },
        Transform {
            transpose: false,
            flip_row: true,
            flip_col: false,
        },
        Transform {
            transpose: false,
            flip_row: false,
            flip_col: true,
        },
        Transform {
            transpose: false,
            flip_row: true,
            flip_col: true,
        },
        Transform {
            transpose: true,
            flip_row: false,
            flip_col: false,
        },
        Transform {
            transpose: true,
            flip_row: true,
            flip_col: false,
        },
        Transform {
            transpose: true,
            flip_row: false,
            flip_col: true,
        },
        Transform {
            transpose: true,
            flip_row: true,
            flip_col: true,
        },
    ]
}

fn solve_single(
    a: &[Vec<i64>],
    perms: &[[usize; 5]],
    valid: &[Vec<bool>],
    endpoint_state: [usize; 2],
) -> Vec<(usize, usize)> {
    let routes = build_band_routes(a, perms, valid, endpoint_state);
    let path = build_full_path(&routes);
    validate_path(a.len(), &path);
    path
}

fn anneal_path(
    path: &mut [(usize, usize)],
    a: &[Vec<i64>],
    deadline: Instant,
    initial_score: i64,
    initial_path: &[(usize, usize)],
) {
    const BUCKETS: usize = 10;
    let start_temp = env::var("SA_START_TEMP")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(SA_START_TEMP);
    let end_temp = env::var("SA_END_TEMP")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(SA_END_TEMP);
    let anneal_start = Instant::now();
    if anneal_start >= deadline {
        return;
    }
    let total_secs = (deadline - anneal_start).as_secs_f64().max(1e-9);
    let n = path.len();
    let mut values = vec![0i64; n];
    for i in 0..n {
        let (r, c) = path[i];
        values[i] = a[r][c];
    }
    let mut current_score = score_from_values(&values);
    let mut best_score = current_score;
    let mut best_path = path.to_vec();
    let seed = (current_score as u64)
        ^ ((values[0] as u64) << 1)
        ^ ((values[n - 1] as u64) << 17)
        ^ ((n as u64) << 33)
        ^ 0xA076_1D64_78BD_642F;
    let mut rng = XorShift64::new(seed);
    let mut candidates = collect_candidate_indices(path);
    if candidates.len() < 2 {
        return;
    }
    let mut right_bounds = build_right_bounds(&candidates);
    let (mut pref_a, mut pref_ia) = build_prefix_sums(&values);
    let mut stats = [BucketStats::default(); BUCKETS];

    while Instant::now() < deadline {
        let progress = ((Instant::now() - anneal_start).as_secs_f64() / total_secs).clamp(0.0, 1.0);
        let bucket = ((progress * BUCKETS as f64) as usize).min(BUCKETS - 1);
        let temp = start_temp * (end_temp / start_temp).powf(progress);
        let left_idx = rng.gen_range(candidates.len() - 1);
        let right_exclusive = right_bounds[left_idx];
        if right_exclusive <= left_idx + 1 {
            continue;
        }
        let right_idx = left_idx + 1 + rng.gen_range(right_exclusive - left_idx - 1);
        let l = candidates[left_idx];
        let r = candidates[right_idx];
        if r <= l + 1 || !can_reverse(path, l, r) {
            continue;
        }
        stats[bucket].attempts += 1;
        let delta = reversal_delta(&pref_a, &pref_ia, l, r);
        let accept = if delta >= 0 {
            true
        } else {
            rng.gen_f64() < ((delta as f64) / temp).exp()
        };
        if !accept {
            continue;
        }

        path[l..=r].reverse();
        values[l..=r].reverse();
        current_score += delta;
        if delta >= 0 {
            stats[bucket].accepted_uphill += 1;
        } else {
            stats[bucket].accepted_downhill += 1;
        }
        if current_score > best_score {
            best_score = current_score;
            best_path.clone_from_slice(path);
        }
        candidates = collect_candidate_indices(path);
        if candidates.len() < 2 {
            break;
        }
        right_bounds = build_right_bounds(&candidates);
        (pref_a, pref_ia) = build_prefix_sums(&values);
    }

    if best_score > current_score {
        path.clone_from_slice(&best_path);
    }

    let final_score = compute_score(a, path);
    let best_diff = path_diff_count(initial_path, &best_path);
    let final_diff = path_diff_count(initial_path, path);
    eprintln!(
        "anneal_summary start_temp={} end_temp={} initial={} best={} final={} gain_best={} gain_final={} best_diff={} final_diff={}",
        start_temp,
        end_temp,
        initial_score,
        best_score,
        final_score,
        best_score - initial_score,
        final_score - initial_score,
        best_diff,
        final_diff
    );
    for (idx, bucket) in stats.iter().enumerate() {
        if bucket.attempts == 0 {
            eprintln!("bucket {} attempts=0 uphill=0 downhill=0", idx);
        } else {
            eprintln!(
                "bucket {} attempts={} uphill={} downhill={} downhill_rate={:.4}",
                idx,
                bucket.attempts,
                bucket.accepted_uphill,
                bucket.accepted_downhill,
                bucket.accepted_downhill as f64 / bucket.attempts as f64
            );
        }
    }
}

fn collect_candidate_indices(path: &[(usize, usize)]) -> Vec<usize> {
    let mut indices = Vec::new();
    for (idx, &(r, _)) in path.iter().enumerate() {
        if r <= SEARCH_ROWS_TOP || r >= SEARCH_ROWS_BOTTOM {
            indices.push(idx);
        }
    }
    indices
}

fn build_prefix_sums(values: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let mut pref_a = vec![0i64; values.len() + 1];
    let mut pref_ia = vec![0i64; values.len() + 1];
    for i in 0..values.len() {
        pref_a[i + 1] = pref_a[i] + values[i];
        pref_ia[i + 1] = pref_ia[i] + (i as i64) * values[i];
    }
    (pref_a, pref_ia)
}

fn build_right_bounds(candidates: &[usize]) -> Vec<usize> {
    let mut right_bounds = vec![0usize; candidates.len()];
    let mut r = 0usize;
    for l in 0..candidates.len() {
        if r < l + 1 {
            r = l + 1;
        }
        while r < candidates.len() && candidates[r] - candidates[l] <= MAX_SEGMENT_SPAN {
            r += 1;
        }
        right_bounds[l] = r;
    }
    right_bounds
}

fn reversal_delta(pref_a: &[i64], pref_ia: &[i64], l: usize, r: usize) -> i64 {
    let sum_a = pref_a[r + 1] - pref_a[l];
    let sum_ia = pref_ia[r + 1] - pref_ia[l];
    ((l + r) as i64) * sum_a - 2 * sum_ia
}

fn can_reverse(path: &[(usize, usize)], l: usize, r: usize) -> bool {
    if l > 0 && !is_adjacent(path[l - 1], path[r]) {
        return false;
    }
    if r + 1 < path.len() && !is_adjacent(path[l], path[r + 1]) {
        return false;
    }
    true
}

fn is_adjacent(a: (usize, usize), b: (usize, usize)) -> bool {
    let dr = a.0.abs_diff(b.0);
    let dc = a.1.abs_diff(b.1);
    dr <= 1 && dc <= 1 && (dr != 0 || dc != 0)
}

fn transform_board(a: &[Vec<i64>], tr: Transform) -> Vec<Vec<i64>> {
    let n = a.len();
    let mut out = vec![vec![0i64; n]; n];
    for (r, row) in a.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            let (nr, nc) = to_transformed((r, c), n, tr);
            out[nr][nc] = v;
        }
    }
    out
}

fn map_path_to_original(path: &[(usize, usize)], n: usize, tr: Transform) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(path.len());
    for &p in path {
        out.push(to_original(p, n, tr));
    }
    out
}

fn to_transformed((r, c): (usize, usize), n: usize, tr: Transform) -> (usize, usize) {
    let (mut x, mut y) = if tr.transpose { (c, r) } else { (r, c) };
    if tr.flip_row {
        x = n - 1 - x;
    }
    if tr.flip_col {
        y = n - 1 - y;
    }
    (x, y)
}

fn to_original((r, c): (usize, usize), n: usize, tr: Transform) -> (usize, usize) {
    let mut x = r;
    let mut y = c;
    if tr.flip_row {
        x = n - 1 - x;
    }
    if tr.flip_col {
        y = n - 1 - y;
    }
    if tr.transpose { (y, x) } else { (x, y) }
}

fn compute_score(a: &[Vec<i64>], path: &[(usize, usize)]) -> i64 {
    let mut score = 0i64;
    for (idx, &(r, c)) in path.iter().enumerate() {
        score += (idx as i64) * a[r][c];
    }
    score
}

fn score_from_values(values: &[i64]) -> i64 {
    let mut score = 0i64;
    for (idx, &value) in values.iter().enumerate() {
        score += (idx as i64) * value;
    }
    score
}

fn path_diff_count(a: &[(usize, usize)], b: &[(usize, usize)]) -> usize {
    let mut diff = 0usize;
    for i in 0..a.len() {
        if a[i] != b[i] {
            diff += 1;
        }
    }
    diff
}

fn build_band_routes(
    a: &[Vec<i64>],
    perms: &[[usize; 5]],
    valid: &[Vec<bool>],
    endpoint_state: [usize; 2],
) -> Vec<Vec<Vec<usize>>> {
    let band_count = 40;
    let core_h = 190;
    let state_count = perms.len();
    let mut routes = vec![vec![vec![0usize; core_h]; 5]; band_count];

    for k in 0..band_count {
        let base = 5 * k;
        let end_state = endpoint_state[k % 2];
        let mut parent = vec![vec![usize::MAX; state_count]; core_h];
        let mut cur = vec![NEG_INF; state_count];
        cur[end_state] = row_gain(a, 5, base, &perms[end_state]);

        for step in 0..(core_h - 1) {
            let row = 6 + step;
            let mut next = vec![NEG_INF; state_count];
            for s in 0..state_count {
                let score = cur[s];
                if score <= NEG_INF / 2 {
                    continue;
                }
                for t in 0..state_count {
                    if !valid[s][t] {
                        continue;
                    }
                    let cand = score + row_gain(a, row, base, &perms[t]);
                    if cand > next[t] {
                        next[t] = cand;
                        parent[step + 1][t] = s;
                    }
                }
            }
            cur = next;
        }

        assert!(cur[end_state] > NEG_INF / 2, "band DP is unreachable");

        let mut state_on_row = vec![0usize; core_h];
        state_on_row[core_h - 1] = end_state;
        for rev in 0..(core_h - 1) {
            let idx = core_h - 1 - rev;
            state_on_row[idx - 1] = parent[idx][state_on_row[idx]];
        }
        assert_eq!(state_on_row[0], end_state, "top state must be fixed");

        for idx in 0..core_h {
            let state = state_on_row[idx];
            for lane in 0..5 {
                routes[k][lane][idx] = base + perms[state][lane];
            }
        }
    }

    routes
}

fn build_full_path(routes: &[Vec<Vec<usize>>]) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(40000);

    append_points(&mut path, &left_end_start());
    append_round_forward(&mut path, routes, 0);
    append_points(&mut path, &right_end_pair_01());
    append_round_backward(&mut path, routes, 1);
    append_points(&mut path, &left_end_pair_12());
    append_round_forward(&mut path, routes, 2);
    append_points(&mut path, &right_end_pair_23());
    append_round_backward(&mut path, routes, 3);
    append_points(&mut path, &left_end_pair_34());
    append_round_forward(&mut path, routes, 4);
    append_points(&mut path, &right_end_finish());
    path
}

fn append_round_forward(path: &mut Vec<(usize, usize)>, routes: &[Vec<Vec<usize>>], lane: usize) {
    append_core(path, routes, 0, lane, true);
    for boundary in 0..39 {
        let left_col = if boundary % 2 == 0 { lane } else { 4 - lane };
        if boundary % 2 == 0 {
            append_points(path, &bottom_boundary_path(boundary, left_col, false));
        } else {
            append_points(path, &top_boundary_path(boundary, left_col, false));
        }
        append_core(path, routes, boundary + 1, lane, (boundary + 1) % 2 == 0);
    }
}

fn append_round_backward(path: &mut Vec<(usize, usize)>, routes: &[Vec<Vec<usize>>], lane: usize) {
    append_core(path, routes, 39, lane, true);
    for boundary_rev in 0..39 {
        let boundary = 38 - boundary_rev;
        let left_col = if boundary % 2 == 0 { lane } else { 4 - lane };
        if boundary % 2 == 0 {
            append_points(path, &bottom_boundary_path(boundary, left_col, true));
        } else {
            append_points(path, &top_boundary_path(boundary, left_col, true));
        }
        append_core(path, routes, boundary, lane, boundary % 2 == 1);
    }
}

fn append_core(
    path: &mut Vec<(usize, usize)>,
    routes: &[Vec<Vec<usize>>],
    band: usize,
    lane: usize,
    top_to_bottom: bool,
) {
    if top_to_bottom {
        for idx in 0..190 {
            path.push((5 + idx, routes[band][lane][idx]));
        }
    } else {
        for rev in 0..190 {
            let idx = 189 - rev;
            path.push((5 + idx, routes[band][lane][idx]));
        }
    }
}

fn top_boundary_path(boundary: usize, left_lane: usize, reverse: bool) -> Vec<(usize, usize)> {
    let base = 5 * boundary;
    let local = match left_lane {
        0 => vec![
            (4, 0),
            (3, 0),
            (2, 0),
            (1, 0),
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (0, 7),
            (0, 8),
            (0, 9),
            (1, 9),
            (2, 9),
            (3, 9),
            (4, 9),
        ],
        1 => vec![
            (4, 1),
            (3, 1),
            (2, 1),
            (1, 1),
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            (1, 7),
            (1, 8),
            (2, 8),
            (3, 8),
            (4, 8),
        ],
        2 => vec![
            (4, 2),
            (3, 2),
            (2, 2),
            (2, 3),
            (2, 4),
            (2, 5),
            (2, 6),
            (2, 7),
            (3, 7),
            (4, 7),
        ],
        3 => vec![(4, 3), (3, 3), (3, 4), (3, 5), (3, 6), (4, 6)],
        4 => vec![(4, 4), (4, 5)],
        _ => unreachable!(),
    };
    let mut global = local
        .into_iter()
        .map(|(r, dc)| (r, base + dc))
        .collect::<Vec<_>>();
    if reverse {
        global.reverse();
    }
    global
}

fn bottom_boundary_path(boundary: usize, left_lane: usize, reverse: bool) -> Vec<(usize, usize)> {
    let base = 5 * boundary;
    let local = match left_lane {
        0 => vec![
            (195, 0),
            (196, 0),
            (197, 0),
            (198, 0),
            (199, 0),
            (199, 1),
            (199, 2),
            (199, 3),
            (199, 4),
            (199, 5),
            (199, 6),
            (199, 7),
            (199, 8),
            (199, 9),
            (198, 9),
            (197, 9),
            (196, 9),
            (195, 9),
        ],
        1 => vec![
            (195, 1),
            (196, 1),
            (197, 1),
            (198, 1),
            (198, 2),
            (198, 3),
            (198, 4),
            (198, 5),
            (198, 6),
            (198, 7),
            (198, 8),
            (197, 8),
            (196, 8),
            (195, 8),
        ],
        2 => vec![
            (195, 2),
            (196, 2),
            (197, 2),
            (197, 3),
            (197, 4),
            (197, 5),
            (197, 6),
            (197, 7),
            (196, 7),
            (195, 7),
        ],
        3 => vec![(195, 3), (196, 3), (196, 4), (196, 5), (196, 6), (195, 6)],
        4 => vec![(195, 4), (195, 5)],
        _ => unreachable!(),
    };
    let mut global = local
        .into_iter()
        .map(|(r, dc)| (r, base + dc))
        .collect::<Vec<_>>();
    if reverse {
        global.reverse();
    }
    global
}

fn left_end_start() -> Vec<(usize, usize)> {
    vec![
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (1, 4),
        (1, 3),
        (1, 2),
        (1, 1),
        (1, 0),
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
        (2, 4),
        (3, 4),
        (3, 3),
        (3, 2),
        (3, 1),
        (3, 0),
        (4, 0),
    ]
}

fn left_end_pair_12() -> Vec<(usize, usize)> {
    vec![(4, 1), (4, 2)]
}

fn left_end_pair_34() -> Vec<(usize, usize)> {
    vec![(4, 3), (4, 4)]
}

fn right_end_pair_01() -> Vec<(usize, usize)> {
    let base = 195;
    vec![(4, 4), (4, 3)]
        .into_iter()
        .map(|(r, dc)| (r, base + dc))
        .collect()
}

fn right_end_pair_23() -> Vec<(usize, usize)> {
    let base = 195;
    vec![(4, 2), (4, 1)]
        .into_iter()
        .map(|(r, dc)| (r, base + dc))
        .collect()
}

fn right_end_finish() -> Vec<(usize, usize)> {
    let base = 195;
    vec![
        (4, 0),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
        (2, 4),
        (2, 3),
        (2, 2),
        (2, 1),
        (2, 0),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (1, 4),
        (0, 4),
        (0, 3),
        (0, 2),
        (0, 1),
        (0, 0),
    ]
    .into_iter()
    .map(|(r, dc)| (r, base + dc))
    .collect()
}

fn append_points(path: &mut Vec<(usize, usize)>, points: &[(usize, usize)]) {
    path.extend_from_slice(points);
}

fn row_gain(a: &[Vec<i64>], row: usize, base: usize, perm: &[usize; 5]) -> i64 {
    let mut gain = 0;
    for lane in 0..5 {
        gain += ALPHA[lane] * a[row][base + perm[lane]];
    }
    gain
}

fn generate_permutations5() -> Vec<[usize; 5]> {
    let mut arr = [0usize, 1, 2, 3, 4];
    let mut perms = vec![arr];
    while next_permutation(&mut arr) {
        perms.push(arr);
    }
    perms
}

fn next_permutation(a: &mut [usize; 5]) -> bool {
    let mut i = a.len() - 1;
    while i > 0 && a[i - 1] >= a[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let pivot = i - 1;
    let mut j = a.len() - 1;
    while a[j] <= a[pivot] {
        j -= 1;
    }
    a.swap(pivot, j);
    a[i..].reverse();
    true
}

fn build_transition_table(perms: &[[usize; 5]]) -> Vec<Vec<bool>> {
    let m = perms.len();
    let mut valid = vec![vec![false; m]; m];
    for i in 0..m {
        for j in 0..m {
            let mut ok = true;
            for lane in 0..5 {
                if perms[i][lane].abs_diff(perms[j][lane]) > 1 {
                    ok = false;
                    break;
                }
            }
            valid[i][j] = ok;
        }
    }
    valid
}

fn validate_path(n: usize, path: &[(usize, usize)]) {
    let mut seen = vec![false; n * n];
    for idx in 0..path.len() {
        let (r, c) = path[idx];
        assert!(r < n && c < n, "out of board");
        let id = r * n + c;
        assert!(!seen[id], "duplicated cell: ({r}, {c})");
        seen[id] = true;
        if idx + 1 < path.len() {
            let (nr, nc) = path[idx + 1];
            let dr = r.abs_diff(nr);
            let dc = c.abs_diff(nc);
            assert!(dr <= 1 && dc <= 1 && (dr != 0 || dc != 0), "invalid move");
        }
    }
}
