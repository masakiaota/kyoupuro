use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    assert!(n % 2 == 0);
    let band_count = n / 2;
    assert!(band_count >= 1);

    let turn_band = band_count - 1;
    let active_band_count = turn_band;

    let mut path = Vec::with_capacity(n * n);

    for band in 0..active_band_count {
        let base_col = band * 2;
        let low_cols = build_low_cols_for_band(&a, band, base_col);
        append_band_lane(&mut path, band, base_col, &low_cols);
    }

    let turn_base_col = turn_band * 2;
    append_turn_band(&mut path, &a, turn_base_col);

    for band in (0..active_band_count).rev() {
        let base_col = band * 2;
        let low_cols = build_low_cols_for_band(&a, band, base_col);
        append_band_lane_complement(&mut path, band, base_col, &low_cols);
    }

    assert_eq!(path.len(), n * n);
    validate_path(n, &path);

    for (r, c) in path {
        println!("{r} {c}");
    }
}

fn build_low_cols_for_band(a: &[Vec<i64>], band: usize, base_col: usize) -> Vec<usize> {
    let n = a.len();
    let mut low_cols = vec![0usize; n];

    if band % 2 == 0 {
        low_cols[0] = 0;
        low_cols[n - 1] = 1;
    } else {
        low_cols[0] = 1;
        low_cols[n - 1] = 0;
    }

    for row in 1..n - 1 {
        let left = a[row][base_col];
        let right = a[row][base_col + 1];
        low_cols[row] = if left <= right { 0 } else { 1 };
    }

    low_cols
}

fn append_band_lane(
    path: &mut Vec<(usize, usize)>,
    band: usize,
    base_col: usize,
    low_cols: &[usize],
) {
    let n = low_cols.len();
    if band % 2 == 0 {
        for (row, &col) in low_cols.iter().enumerate() {
            path.push((row, base_col + col));
        }
    } else {
        for row in (0..n).rev() {
            path.push((row, base_col + low_cols[row]));
        }
    }
}

fn append_band_lane_complement(
    path: &mut Vec<(usize, usize)>,
    band: usize,
    base_col: usize,
    low_cols: &[usize],
) {
    let n = low_cols.len();
    if band % 2 == 0 {
        for (row, &col) in low_cols.iter().enumerate() {
            path.push((row, base_col + (1 - col)));
        }
    } else {
        for row in (0..n).rev() {
            path.push((row, base_col + (1 - low_cols[row])));
        }
    }
}

fn append_turn_band(path: &mut Vec<(usize, usize)>, a: &[Vec<i64>], base_col: usize) {
    let n = a.len();
    for row in (1..n).rev() {
        let left = a[row][base_col];
        let right = a[row][base_col + 1];
        if row == n - 1 {
            path.push((row, base_col));
            path.push((row, base_col + 1));
        } else if left <= right {
            path.push((row, base_col));
            path.push((row, base_col + 1));
        } else {
            path.push((row, base_col + 1));
            path.push((row, base_col));
        }
    }

    path.push((0, base_col + 1));
    path.push((0, base_col));
}

fn validate_path(n: usize, path: &[(usize, usize)]) {
    let mut seen = vec![false; n * n];
    for &(r, c) in path {
        assert!(r < n && c < n);
        let idx = r * n + c;
        assert!(!seen[idx]);
        seen[idx] = true;
    }
    for window in path.windows(2) {
        let (r0, c0) = window[0];
        let (r1, c1) = window[1];
        let dr = r0.abs_diff(r1);
        let dc = c0.abs_diff(c1);
        assert!(dr.max(dc) == 1);
    }
}
