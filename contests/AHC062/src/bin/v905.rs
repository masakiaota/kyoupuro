use proconio::input;

const NEG_INF: i64 = i64::MIN / 4;
const ALPHA: [i64; 4] = [1, 2, 3, 4];
fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    assert_eq!(n, 200, "this construction assumes n = 200");

    let perms = generate_permutations4();
    let id_state = perms
        .iter()
        .position(|p| *p == [0, 1, 2, 3])
        .expect("identity permutation must exist");
    let rev_state = perms
        .iter()
        .position(|p| *p == [3, 2, 1, 0])
        .expect("reverse permutation must exist");
    let valid = build_transition_table(&perms);

    let routes = build_band_routes(&a, &perms, &valid, [id_state, rev_state]);
    let path = build_full_path(&routes);

    assert_eq!(path.len(), n * n, "path length must be n^2");
    validate_path(n, &path);

    for (r, c) in path {
        println!("{r} {c}");
    }
}

fn build_band_routes(
    a: &[Vec<i64>],
    perms: &[[usize; 4]],
    valid: &[Vec<bool>],
    endpoint_state: [usize; 2],
) -> Vec<Vec<Vec<usize>>> {
    let band_count = 50;
    let core_h = 192;
    let state_count = perms.len();
    let mut routes = vec![vec![vec![0usize; core_h]; 4]; band_count];

    for k in 0..band_count {
        let base = 4 * k;
        let end_state = endpoint_state[k % 2];
        let mut parent = vec![vec![usize::MAX; state_count]; core_h];
        let mut cur = vec![NEG_INF; state_count];
        cur[end_state] = row_gain(a, 4, base, &perms[end_state]);

        for step in 0..(core_h - 1) {
            let row = 5 + step;
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
            for lane in 0..4 {
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
    append_points(&mut path, &right_end_pair_high());
    append_round_backward(&mut path, routes, 1);
    append_points(&mut path, &left_end_middle());
    append_round_forward(&mut path, routes, 2);
    append_points(&mut path, &right_end_pair_low());
    append_round_backward(&mut path, routes, 3);
    append_points(&mut path, &left_end_finish());
    path
}

fn append_round_forward(path: &mut Vec<(usize, usize)>, routes: &[Vec<Vec<usize>>], lane: usize) {
    append_core(path, routes, 0, lane, true);
    for boundary in 0..49 {
        let left_col = if boundary % 2 == 0 { lane } else { 3 - lane };
        if boundary % 2 == 0 {
            append_points(path, &bottom_boundary_path(boundary, left_col, false));
        } else {
            append_points(path, &top_boundary_path(boundary, left_col, false));
        }
        append_core(path, routes, boundary + 1, lane, (boundary + 1) % 2 == 0);
    }
}

fn append_round_backward(path: &mut Vec<(usize, usize)>, routes: &[Vec<Vec<usize>>], lane: usize) {
    append_core(path, routes, 49, lane, true);
    for boundary_rev in 0..49 {
        let boundary = 48 - boundary_rev;
        let left_col = if boundary % 2 == 0 { lane } else { 3 - lane };
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
        for idx in 0..192 {
            path.push((4 + idx, routes[band][lane][idx]));
        }
    } else {
        for rev in 0..192 {
            let idx = 191 - rev;
            path.push((4 + idx, routes[band][lane][idx]));
        }
    }
}

fn top_boundary_path(boundary: usize, left_lane: usize, reverse: bool) -> Vec<(usize, usize)> {
    let base = 4 * boundary;
    let local = match left_lane {
        0 => vec![
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
            (1, 7),
            (2, 7),
            (3, 7),
        ],
        1 => vec![(3, 1), (2, 1), (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (2, 6), (3, 6)],
        2 => vec![(3, 2), (2, 2), (2, 3), (2, 4), (2, 5), (3, 5)],
        3 => vec![(3, 3), (3, 4)],
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
    let base = 4 * boundary;
    let local = match left_lane {
        0 => vec![
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
            (198, 7),
            (197, 7),
            (196, 7),
        ],
        1 => vec![(196, 1), (197, 1), (198, 1), (198, 2), (198, 3), (198, 4), (198, 5), (198, 6), (197, 6), (196, 6)],
        2 => vec![(196, 2), (197, 2), (197, 3), (197, 4), (197, 5), (196, 5)],
        3 => vec![(196, 3), (196, 4)],
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
    vec![(0, 0), (1, 0), (1, 1), (1, 2), (2, 2), (2, 1), (2, 0), (3, 0)]
}

fn left_end_middle() -> Vec<(usize, usize)> {
    vec![(3, 1), (3, 2)]
}

fn left_end_finish() -> Vec<(usize, usize)> {
    vec![(3, 3), (2, 3), (1, 3), (0, 3), (0, 2), (0, 1)]
}

fn right_end_pair_high() -> Vec<(usize, usize)> {
    let base = 196;
    vec![(3, 3), (3, 2)]
        .into_iter()
        .map(|(r, dc)| (r, base + dc))
        .collect()
}

fn right_end_pair_low() -> Vec<(usize, usize)> {
    let base = 196;
    vec![
        (3, 1),
        (2, 1),
        (2, 2),
        (2, 3),
        (1, 3),
        (0, 3),
        (0, 2),
        (1, 2),
        (1, 1),
        (0, 1),
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
    ]
    .into_iter()
    .map(|(r, dc)| (r, base + dc))
    .collect()
}

fn append_points(path: &mut Vec<(usize, usize)>, points: &[(usize, usize)]) {
    path.extend_from_slice(points);
}

fn row_gain(a: &[Vec<i64>], row: usize, base: usize, perm: &[usize; 4]) -> i64 {
    let mut gain = 0;
    for lane in 0..4 {
        gain += ALPHA[lane] * a[row][base + perm[lane]];
    }
    gain
}

fn generate_permutations4() -> Vec<[usize; 4]> {
    let mut arr = [0usize, 1, 2, 3];
    let mut perms = vec![arr];
    while next_permutation(&mut arr) {
        perms.push(arr);
    }
    perms
}

fn next_permutation(a: &mut [usize; 4]) -> bool {
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

fn build_transition_table(perms: &[[usize; 4]]) -> Vec<Vec<bool>> {
    let m = perms.len();
    let mut valid = vec![vec![false; m]; m];
    for i in 0..m {
        for j in 0..m {
            let mut ok = true;
            for lane in 0..4 {
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
