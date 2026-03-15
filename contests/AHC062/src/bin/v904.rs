use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [[i64; n]; n],
    }

    assert!(n >= 4, "n must be at least 4");
    assert!(n % 2 == 0, "n must be even for this construction");

    let band_count = n / 2;
    let bridge_band = band_count - 1;
    let active_bands = bridge_band;

    // active_bands(=0..bridge_band-1) では、low ルートを先に訪問し、
    // その補集合(high ルート)を最後に逆順で訪問する。
    let mut low_choice = vec![vec![0usize; n]; active_bands];

    for k in 0..active_bands {
        let left = 2 * k;
        let right = left + 1;
        for r in 0..n {
            low_choice[k][r] = if a[r][left] <= a[r][right] { 0 } else { 1 };
        }

        // 帯間接続のため、端点だけは強制する。
        if k % 2 == 0 {
            // top -> bottom
            low_choice[k][0] = 0;
            low_choice[k][n - 1] = 1;
        } else {
            // bottom -> top
            low_choice[k][0] = 1;
            low_choice[k][n - 1] = 0;
        }
    }

    let mut path = Vec::with_capacity(n * n);

    // Phase 1: low ルートを左から右へ。
    for k in 0..active_bands {
        let left = 2 * k;
        if k % 2 == 0 {
            for r in 0..n {
                let c = left + low_choice[k][r];
                path.push((r, c));
            }
        } else {
            for rr in 0..n {
                let r = n - 1 - rr;
                let c = left + low_choice[k][r];
                path.push((r, c));
            }
        }
    }

    // Phase 2 (bridge): 右端 1 帯を全訪問して折り返す。
    // bottom-left -> top-left で 2xN の蛇行路を作る。
    let bridge_left = 2 * bridge_band;
    let bridge_right = bridge_left + 1;
    for t in 0..n {
        let r = n - 1 - t;
        if t % 2 == 0 {
            path.push((r, bridge_left));
            path.push((r, bridge_right));
        } else {
            path.push((r, bridge_right));
            path.push((r, bridge_left));
        }
    }

    // Phase 3: high ルートを右から左へ。
    for rev in 0..active_bands {
        let k = active_bands - 1 - rev;
        let left = 2 * k;
        if k % 2 == 0 {
            // top-right から入り bottom-left へ抜ける
            for r in 0..n {
                let c = left + (1 - low_choice[k][r]);
                path.push((r, c));
            }
        } else {
            // bottom-right から入り top-left へ抜ける
            for rr in 0..n {
                let r = n - 1 - rr;
                let c = left + (1 - low_choice[k][r]);
                path.push((r, c));
            }
        }
    }

    assert_eq!(path.len(), n * n, "path length must be n^2");
    validate_path(n, &path);

    for (r, c) in path {
        println!("{r} {c}");
    }
}

fn validate_path(n: usize, path: &[(usize, usize)]) {
    let mut seen = vec![false; n * n];
    for (idx, &(r, c)) in path.iter().enumerate() {
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
