// a01_wall_stats.rs

use std::cmp::{max, min};
use std::collections::VecDeque;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const EXPECTED_N: usize = 20;
const EXPECTED_CASES: usize = 100;

#[derive(Clone)]
struct Board {
    n: usize,
    cards: Vec<usize>,
    // V[i][j]: (i,j) と (i,j+1) の間の壁。
    v: Vec<Vec<bool>>,
    // H[i][j]: (i,j) と (i+1,j) の間の壁。
    h: Vec<Vec<bool>>,
}

impl Board {
    fn wall_free(n: usize) -> Self {
        Self {
            n,
            cards: (0..n * n).collect(),
            v: vec![vec![false; n - 1]; n],
            h: vec![vec![false; n]; n - 1],
        }
    }
}

#[derive(Clone)]
struct CaseStats {
    case: String,
    wall_edges: usize,
    wall_segments: usize,
    legal_ops_total: usize,
    legal_ops_v: usize,
    legal_ops_h: usize,
    legal_area_max: usize,
    legal_area_p50: usize,
    legal_area_p90: usize,
    big_ops_100: usize,
    big_ops_200: usize,
    maximal_rects: usize,
    maximal_rect_area_max: usize,
    move_deg_avg: f64,
    move_dist_avg: f64,
    move_dist_diam: usize,
    move_within2_ratio: f64,
    bfs_dist_avg: f64,
    bfs_diam: usize,
    manhattan_gap_avg: f64,
    blocked_adj_pairs: usize,
    blocked_detour_sum: u64,
    blocked_detour_max: usize,
    init_total_bfs_dist: u64,
    init_cycles: usize,
    init_fixed_points: usize,
}

struct LegalStats {
    total: usize,
    vertical: usize,
    horizontal: usize,
    area_max: usize,
    area_p50: usize,
    area_p90: usize,
    big_100: usize,
    big_200: usize,
    maximal_rects: usize,
    maximal_rect_area_max: usize,
}

struct GraphStats {
    average: f64,
    diameter: usize,
    within2_ratio: f64,
}

// 長方形内部の壁の有無を O(1) で調べるための二次元累積和。
struct Prefix2D {
    cols: usize,
    data: Vec<usize>,
}

impl Prefix2D {
    fn new(values: &[Vec<bool>]) -> Self {
        let rows = values.len();
        let cols = values.first().map_or(0, Vec::len);
        let mut data = vec![0; (rows + 1) * (cols + 1)];
        for r in 0..rows {
            let mut row_sum = 0;
            for c in 0..cols {
                row_sum += usize::from(values[r][c]);
                data[(r + 1) * (cols + 1) + c + 1] = data[r * (cols + 1) + c + 1] + row_sum;
            }
        }
        Self { cols, data }
    }

    fn sum(&self, r0: usize, r1: usize, c0: usize, c1: usize) -> usize {
        if r0 == r1 || c0 == c1 {
            return 0;
        }
        let stride = self.cols + 1;
        self.data[r1 * stride + c1] + self.data[r0 * stride + c0]
            - self.data[r0 * stride + c1]
            - self.data[r1 * stride + c0]
    }
}

struct WallIndex {
    v: Prefix2D,
    h: Prefix2D,
}

impl WallIndex {
    fn new(board: &Board) -> Self {
        Self {
            v: Prefix2D::new(&board.v),
            h: Prefix2D::new(&board.h),
        }
    }

    // [r0,r1) x [c0,c1) の内部に壁がなければ true。
    fn is_clear(&self, r0: usize, r1: usize, c0: usize, c1: usize) -> bool {
        let vertical_clear = c1 - c0 <= 1 || self.v.sum(r0, r1, c0, c1 - 1) == 0;
        let horizontal_clear = r1 - r0 <= 1 || self.h.sum(r0, r1 - 1, c0, c1) == 0;
        vertical_clear && horizontal_clear
    }
}

struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn unite(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.size[ra] < self.size[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <input_dir> <output_dir>",
            args.first().map_or("a01_wall_stats", String::as_str)
        ));
    }
    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    let mut input_paths = collect_input_paths(input_dir)?;
    input_paths.sort();
    if input_paths.len() != EXPECTED_CASES {
        return Err(format!(
            "入力ケース数が {EXPECTED_CASES} ではない: {}",
            input_paths.len()
        ));
    }

    // 実データより先に人工盤面を計算し、列挙ロジックの基準値を固定する。
    let baseline_board = Board::wall_free(EXPECTED_N);
    let baseline = analyze_board("wall_free", &baseline_board)?;
    assert_eq!(baseline.legal_ops_total, 42_000);
    assert_eq!(baseline.legal_ops_v, 21_000);
    assert_eq!(baseline.legal_ops_h, 21_000);

    let mut cases = Vec::with_capacity(input_paths.len());
    for path in &input_paths {
        let board = parse_board(path)?;
        let case = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("case stem を UTF-8 として読めない: {}", path.display()))?;
        let stats = analyze_board(case, &board)?;
        assert_eq!(
            stats.blocked_adj_pairs, stats.wall_edges,
            "{case}: blocked_adj_pairs と wall_edges が不一致"
        );
        if stats.wall_edges == 0 {
            assert_eq!(
                stats.legal_ops_total, 42_000,
                "{case}: 壁なしケースの合法操作数が不一致"
            );
        }
        cases.push(stats);
    }

    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "出力ディレクトリを作成できない {}: {e}",
            output_dir.display()
        )
    })?;
    write_cases_csv(&output_dir.join("a01_cases.csv"), &cases)?;
    let summary_path = output_dir.join("a01_summary.md");
    write_summary(&summary_path, input_dir, &cases, &baseline)
        .map_err(|e| format!("summary の書き込みに失敗 {}: {e}", summary_path.display()))?;

    let zero_wall_cases = cases.iter().filter(|s| s.wall_edges == 0).count();
    println!("processed {} cases", cases.len());
    println!(
        "wall-free sanity: legal_ops={}/{}/{}, move_dist_diam={}",
        baseline.legal_ops_total,
        baseline.legal_ops_v,
        baseline.legal_ops_h,
        baseline.move_dist_diam
    );
    println!("zero-wall input cases: {zero_wall_cases}");
    println!("blocked_adj_pairs == wall_edges: all passed");
    Ok(())
}

fn collect_input_paths(input_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(input_dir)
        .map_err(|e| format!("入力ディレクトリを読めない {}: {e}", input_dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("入力ディレクトリの走査に失敗: {e}"))?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("txt")) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn parse_board(path: &Path) -> Result<Board, String> {
    let input =
        fs::read_to_string(path).map_err(|e| format!("入力を読めない {}: {e}", path.display()))?;
    let mut tokens = input.split_whitespace();
    let n: usize = tokens
        .next()
        .ok_or_else(|| format!("N がない: {}", path.display()))?
        .parse()
        .map_err(|e| format!("N が不正 {}: {e}", path.display()))?;
    if n != EXPECTED_N {
        return Err(format!(
            "N={n} は未対応（期待値 {EXPECTED_N}）: {}",
            path.display()
        ));
    }

    let mut cards = Vec::with_capacity(n * n);
    for index in 0..n * n {
        let value: usize = tokens
            .next()
            .ok_or_else(|| format!("a[{index}] がない: {}", path.display()))?
            .parse()
            .map_err(|e| format!("a[{index}] が不正 {}: {e}", path.display()))?;
        cards.push(value);
    }
    validate_permutation(&cards, path)?;

    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let token = tokens
            .next()
            .ok_or_else(|| format!("V[{i}] がない: {}", path.display()))?;
        v.push(parse_wall_row(token, n - 1, &format!("V[{i}]"), path)?);
    }
    let mut h = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let token = tokens
            .next()
            .ok_or_else(|| format!("H[{i}] がない: {}", path.display()))?;
        h.push(parse_wall_row(token, n, &format!("H[{i}]"), path)?);
    }
    if let Some(extra) = tokens.next() {
        return Err(format!(
            "入力末尾に余分な token `{extra}` がある: {}",
            path.display()
        ));
    }
    Ok(Board { n, cards, v, h })
}

fn validate_permutation(cards: &[usize], path: &Path) -> Result<(), String> {
    let mut seen = vec![false; cards.len()];
    for &card in cards {
        if card >= cards.len() || seen[card] {
            return Err(format!(
                "カード列が 0..{} の順列ではない: {}",
                cards.len() - 1,
                path.display()
            ));
        }
        seen[card] = true;
    }
    Ok(())
}

fn parse_wall_row(
    token: &str,
    expected_len: usize,
    label: &str,
    path: &Path,
) -> Result<Vec<bool>, String> {
    if token.len() != expected_len || !token.bytes().all(|b| b == b'0' || b == b'1') {
        return Err(format!(
            "{label} は長さ {expected_len} の 01 文字列ではない: {}",
            path.display()
        ));
    }
    Ok(token.bytes().map(|b| b == b'1').collect())
}

fn analyze_board(case: &str, board: &Board) -> Result<CaseStats, String> {
    let n = board.n;
    let wall_index = WallIndex::new(board);
    let wall_edges = count_wall_edges(board);
    let legal = enumerate_rectangles(board, &wall_index);

    let normal_graph = build_normal_graph(board);
    let normal_dist = all_pairs_distances(&normal_graph, "通常隣接グラフ")?;
    let normal_stats = summarize_distances(&normal_dist);

    let move_graph = build_move_graph(board, &wall_index);
    let move_dist = all_pairs_distances(&move_graph, "一枚移動グラフ")?;
    let move_stats = summarize_distances(&move_dist);
    let move_deg_avg = move_graph.iter().map(Vec::len).sum::<usize>() as f64 / (n * n) as f64;

    let mut manhattan_gap_sum = 0_u64;
    for (a, distances_from_a) in normal_dist.iter().enumerate() {
        let (ar, ac) = (a / n, a % n);
        for (b, &raw_distance) in distances_from_a.iter().enumerate().skip(a + 1) {
            let (br, bc) = (b / n, b % n);
            let manhattan = ar.abs_diff(br) + ac.abs_diff(bc);
            let distance = usize::from(raw_distance);
            debug_assert!(distance >= manhattan);
            manhattan_gap_sum += (distance - manhattan) as u64;
        }
    }
    let pair_count = (n * n * (n * n - 1) / 2) as f64;

    let mut blocked_adj_pairs = 0;
    let mut blocked_detour_sum = 0_u64;
    let mut blocked_detour_max = 0;
    for i in 0..n {
        for j in 0..n - 1 {
            if board.v[i][j] {
                let a = i * n + j;
                let b = a + 1;
                add_blocked_detour(
                    usize::from(normal_dist[a][b]),
                    &mut blocked_adj_pairs,
                    &mut blocked_detour_sum,
                    &mut blocked_detour_max,
                );
            }
        }
    }
    for i in 0..n - 1 {
        for j in 0..n {
            if board.h[i][j] {
                let a = i * n + j;
                let b = (i + 1) * n + j;
                add_blocked_detour(
                    usize::from(normal_dist[a][b]),
                    &mut blocked_adj_pairs,
                    &mut blocked_detour_sum,
                    &mut blocked_detour_max,
                );
            }
        }
    }
    if blocked_adj_pairs != wall_edges {
        return Err(format!(
            "{case}: blocked_adj_pairs={blocked_adj_pairs} != wall_edges={wall_edges}"
        ));
    }

    let init_total_bfs_dist = board
        .cards
        .iter()
        .enumerate()
        .map(|(position, &card)| u64::from(normal_dist[position][card]))
        .sum();
    let (init_cycles, init_fixed_points) = permutation_cycle_stats(&board.cards);

    Ok(CaseStats {
        case: case.to_owned(),
        wall_edges,
        wall_segments: count_wall_segments(board),
        legal_ops_total: legal.total,
        legal_ops_v: legal.vertical,
        legal_ops_h: legal.horizontal,
        legal_area_max: legal.area_max,
        legal_area_p50: legal.area_p50,
        legal_area_p90: legal.area_p90,
        big_ops_100: legal.big_100,
        big_ops_200: legal.big_200,
        maximal_rects: legal.maximal_rects,
        maximal_rect_area_max: legal.maximal_rect_area_max,
        move_deg_avg,
        move_dist_avg: move_stats.average,
        move_dist_diam: move_stats.diameter,
        move_within2_ratio: move_stats.within2_ratio,
        bfs_dist_avg: normal_stats.average,
        bfs_diam: normal_stats.diameter,
        manhattan_gap_avg: manhattan_gap_sum as f64 / pair_count,
        blocked_adj_pairs,
        blocked_detour_sum,
        blocked_detour_max,
        init_total_bfs_dist,
        init_cycles,
        init_fixed_points,
    })
}

fn count_wall_edges(board: &Board) -> usize {
    board
        .v
        .iter()
        .chain(board.h.iter())
        .flatten()
        .filter(|&&wall| wall)
        .count()
}

fn count_wall_segments(board: &Board) -> usize {
    let n = board.n;
    let mut endpoints = Vec::new();
    let vertex = |r: usize, c: usize| r * (n + 1) + c;
    for i in 0..n {
        for j in 0..n - 1 {
            if board.v[i][j] {
                endpoints.push((vertex(i, j + 1), vertex(i + 1, j + 1)));
            }
        }
    }
    for i in 0..n - 1 {
        for j in 0..n {
            if board.h[i][j] {
                endpoints.push((vertex(i + 1, j), vertex(i + 1, j + 1)));
            }
        }
    }
    if endpoints.is_empty() {
        return 0;
    }

    // 同じ格子頂点に触れる壁辺を併合すれば、壁辺集合の連結成分になる。
    let mut first_edge_at_vertex = vec![None; (n + 1) * (n + 1)];
    let mut dsu = Dsu::new(endpoints.len());
    for (edge, &(a, b)) in endpoints.iter().enumerate() {
        for endpoint in [a, b] {
            if let Some(other) = first_edge_at_vertex[endpoint] {
                dsu.unite(edge, other);
            } else {
                first_edge_at_vertex[endpoint] = Some(edge);
            }
        }
    }
    (0..endpoints.len())
        .filter(|&edge| dsu.find(edge) == edge)
        .count()
}

fn enumerate_rectangles(board: &Board, walls: &WallIndex) -> LegalStats {
    let n = board.n;
    let mut vertical = 0;
    let mut horizontal = 0;
    let mut areas = Vec::new();
    let mut maximal_rects = 0;
    let mut maximal_rect_area_max = 0;

    for r0 in 0..n {
        for r1 in r0 + 1..=n {
            for c0 in 0..n {
                for c1 in c0 + 1..=n {
                    if !walls.is_clear(r0, r1, c0, c1) {
                        continue;
                    }
                    let height = r1 - r0;
                    let width = c1 - c0;
                    let area = height * width;
                    if height % 2 == 0 {
                        vertical += 1;
                        areas.push(area);
                    }
                    if width % 2 == 0 {
                        horizontal += 1;
                        areas.push(area);
                    }

                    let can_expand = (r0 > 0 && walls.is_clear(r0 - 1, r1, c0, c1))
                        || (r1 < n && walls.is_clear(r0, r1 + 1, c0, c1))
                        || (c0 > 0 && walls.is_clear(r0, r1, c0 - 1, c1))
                        || (c1 < n && walls.is_clear(r0, r1, c0, c1 + 1));
                    if !can_expand {
                        maximal_rects += 1;
                        maximal_rect_area_max = max(maximal_rect_area_max, area);
                    }
                }
            }
        }
    }

    areas.sort_unstable();
    LegalStats {
        total: vertical + horizontal,
        vertical,
        horizontal,
        area_max: *areas.last().expect("連結盤面には合法な隣接操作がある"),
        area_p50: nearest_rank_usize(&areas, 1, 2),
        area_p90: nearest_rank_usize(&areas, 9, 10),
        big_100: areas.iter().filter(|&&area| area >= 100).count(),
        big_200: areas.iter().filter(|&&area| area >= 200).count(),
        maximal_rects,
        maximal_rect_area_max,
    }
}

fn build_normal_graph(board: &Board) -> Vec<Vec<usize>> {
    let n = board.n;
    let mut graph = vec![Vec::new(); n * n];
    for i in 0..n {
        for j in 0..n - 1 {
            if !board.v[i][j] {
                add_undirected_edge(&mut graph, i * n + j, i * n + j + 1);
            }
        }
    }
    for i in 0..n - 1 {
        for j in 0..n {
            if !board.h[i][j] {
                add_undirected_edge(&mut graph, i * n + j, (i + 1) * n + j);
            }
        }
    }
    graph
}

fn build_move_graph(board: &Board, walls: &WallIndex) -> Vec<Vec<usize>> {
    let n = board.n;
    let mut graph = vec![Vec::new(); n * n];

    // 同列の距離 d の組は、包含する高さ 2d の壁なし区間が一つでもあれば結ぶ。
    for j in 0..n {
        for i in 0..n {
            for next_i in i + 1..n {
                let d = next_i - i;
                if 2 * d > n {
                    break;
                }
                let start_lo = (i + 1).saturating_sub(d);
                let start_hi = min(i, n - 2 * d);
                let exists = start_lo <= start_hi
                    && (start_lo..=start_hi).any(|r| walls.is_clear(r, r + 2 * d, j, j + 1));
                if exists {
                    add_undirected_edge(&mut graph, i * n + j, next_i * n + j);
                }
            }
        }
    }

    // 同行についても、幅 2d の横操作を対称に列挙する。
    for i in 0..n {
        for j in 0..n {
            for next_j in j + 1..n {
                let d = next_j - j;
                if 2 * d > n {
                    break;
                }
                let start_lo = (j + 1).saturating_sub(d);
                let start_hi = min(j, n - 2 * d);
                let exists = start_lo <= start_hi
                    && (start_lo..=start_hi).any(|c| walls.is_clear(i, i + 1, c, c + 2 * d));
                if exists {
                    add_undirected_edge(&mut graph, i * n + j, i * n + next_j);
                }
            }
        }
    }
    graph
}

fn add_undirected_edge(graph: &mut [Vec<usize>], a: usize, b: usize) {
    graph[a].push(b);
    graph[b].push(a);
}

fn all_pairs_distances(graph: &[Vec<usize>], name: &str) -> Result<Vec<Vec<u16>>, String> {
    let vertex_count = graph.len();
    let mut all_dist = Vec::with_capacity(vertex_count);
    for source in 0..vertex_count {
        let mut dist = vec![u16::MAX; vertex_count];
        let mut queue = VecDeque::new();
        dist[source] = 0;
        queue.push_back(source);
        while let Some(vertex) = queue.pop_front() {
            let next_dist = dist[vertex] + 1;
            for &next in &graph[vertex] {
                if dist[next] == u16::MAX {
                    dist[next] = next_dist;
                    queue.push_back(next);
                }
            }
        }
        if dist.contains(&u16::MAX) {
            return Err(format!("{name} が非連結（始点 {source}）"));
        }
        all_dist.push(dist);
    }
    Ok(all_dist)
}

fn summarize_distances(dist: &[Vec<u16>]) -> GraphStats {
    let n = dist.len();
    let mut sum = 0_u64;
    let mut diameter = 0;
    let mut within2 = 0_u64;
    for (i, distances_from_i) in dist.iter().enumerate() {
        for &raw_distance in distances_from_i.iter().skip(i + 1) {
            let d = usize::from(raw_distance);
            sum += d as u64;
            diameter = max(diameter, d);
            if d <= 2 {
                within2 += 1;
            }
        }
    }
    let pairs = (n * (n - 1) / 2) as f64;
    GraphStats {
        average: sum as f64 / pairs,
        diameter,
        within2_ratio: within2 as f64 / pairs,
    }
}

fn add_blocked_detour(bfs_distance: usize, pairs: &mut usize, sum: &mut u64, maximum: &mut usize) {
    let cost = 2 * bfs_distance - 1;
    *pairs += 1;
    *sum += cost as u64;
    *maximum = max(*maximum, cost);
}

fn permutation_cycle_stats(permutation: &[usize]) -> (usize, usize) {
    let mut visited = vec![false; permutation.len()];
    let mut nontrivial_cycles = 0;
    let mut fixed_points = 0;
    for start in 0..permutation.len() {
        if visited[start] {
            continue;
        }
        if permutation[start] == start {
            visited[start] = true;
            fixed_points += 1;
            continue;
        }
        nontrivial_cycles += 1;
        let mut current = start;
        while !visited[current] {
            visited[current] = true;
            current = permutation[current];
        }
    }
    (nontrivial_cycles, fixed_points)
}

fn nearest_rank_usize(sorted: &[usize], numerator: usize, denominator: usize) -> usize {
    let rank = (sorted.len() * numerator).div_ceil(denominator);
    sorted[rank - 1]
}

fn nearest_rank_f64(sorted: &[f64], numerator: usize, denominator: usize) -> f64 {
    let rank = (sorted.len() * numerator).div_ceil(denominator);
    sorted[rank - 1]
}

fn write_cases_csv(path: &Path, cases: &[CaseStats]) -> Result<(), String> {
    let file =
        File::create(path).map_err(|e| format!("CSV を作成できない {}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(
        out,
        "case,wall_edges,wall_segments,legal_ops_total,legal_ops_v,legal_ops_h,legal_area_max,legal_area_p50,legal_area_p90,big_ops_100,big_ops_200,maximal_rects,maximal_rect_area_max,move_deg_avg,move_dist_avg,move_dist_diam,move_within2_ratio,bfs_dist_avg,bfs_diam,manhattan_gap_avg,blocked_adj_pairs,blocked_detour_sum,blocked_detour_max,init_total_bfs_dist,init_cycles,init_fixed_points"
    )
    .map_err(|e| format!("CSV header の書き込みに失敗: {e}"))?;
    for s in cases {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{},{:.9},{:.9},{},{:.9},{},{},{},{},{},{}",
            s.case,
            s.wall_edges,
            s.wall_segments,
            s.legal_ops_total,
            s.legal_ops_v,
            s.legal_ops_h,
            s.legal_area_max,
            s.legal_area_p50,
            s.legal_area_p90,
            s.big_ops_100,
            s.big_ops_200,
            s.maximal_rects,
            s.maximal_rect_area_max,
            s.move_deg_avg,
            s.move_dist_avg,
            s.move_dist_diam,
            s.move_within2_ratio,
            s.bfs_dist_avg,
            s.bfs_diam,
            s.manhattan_gap_avg,
            s.blocked_adj_pairs,
            s.blocked_detour_sum,
            s.blocked_detour_max,
            s.init_total_bfs_dist,
            s.init_cycles,
            s.init_fixed_points,
        )
        .map_err(|e| format!("CSV row {} の書き込みに失敗: {e}", s.case))?;
    }
    out.flush()
        .map_err(|e| format!("CSV flush に失敗 {}: {e}", path.display()))
}

fn write_summary(
    path: &Path,
    input_dir: &Path,
    cases: &[CaseStats],
    baseline: &CaseStats,
) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut out = BufWriter::new(file);

    writeln!(out, "# A-01 壁と操作容量の統計分析")?;
    writeln!(out)?;
    writeln!(
        out,
        "`{}` の {} ケースを対象に集計した。離散パーセンタイルと中央値には nearest-rank 法を使う。合法操作の面積分布では縦操作と横操作を別々に数え、両方向が合法な同一長方形は2回数える。距離平均は自己対を除く順序なし全点対で計算した。",
        input_dir.display(),
        cases.len()
    )?;
    writeln!(out)?;

    writeln!(out, "## 主要列の分布")?;
    writeln!(out)?;
    writeln!(out, "| 指標 | min | 中央値 | max |")?;
    writeln!(out, "|---|---:|---:|---:|")?;
    write_usize_distribution(
        &mut out,
        "wall_edges",
        cases.iter().map(|s| s.wall_edges).collect(),
    )?;
    write_usize_distribution(
        &mut out,
        "legal_ops_total",
        cases.iter().map(|s| s.legal_ops_total).collect(),
    )?;
    write_usize_distribution(
        &mut out,
        "legal_area_max",
        cases.iter().map(|s| s.legal_area_max).collect(),
    )?;
    write_usize_distribution(
        &mut out,
        "big_ops_100",
        cases.iter().map(|s| s.big_ops_100).collect(),
    )?;
    write_f64_distribution(
        &mut out,
        "move_dist_avg",
        cases.iter().map(|s| s.move_dist_avg).collect(),
    )?;
    write_usize_distribution(
        &mut out,
        "move_dist_diam",
        cases.iter().map(|s| s.move_dist_diam).collect(),
    )?;
    write_u64_distribution(
        &mut out,
        "blocked_detour_sum",
        cases.iter().map(|s| s.blocked_detour_sum).collect(),
    )?;
    writeln!(out)?;

    let mut legal_values: Vec<usize> = cases.iter().map(|s| s.legal_ops_total).collect();
    legal_values.sort_unstable();
    let legal_q1 = nearest_rank_usize(&legal_values, 1, 3);
    let legal_q2 = nearest_rank_usize(&legal_values, 2, 3);
    let mut move_values: Vec<f64> = cases.iter().map(|s| s.move_dist_avg).collect();
    move_values.sort_by(f64::total_cmp);
    let move_q1 = nearest_rank_f64(&move_values, 1, 3);
    let move_q2 = nearest_rank_f64(&move_values, 2, 3);
    let mut cells = [[0_usize; 3]; 3];
    for s in cases {
        cells[tertile_bin_usize(s.legal_ops_total, legal_q1, legal_q2)]
            [tertile_bin_f64(s.move_dist_avg, move_q1, move_q2)] += 1;
    }

    writeln!(out, "## 層別 3×3")?;
    writeln!(out)?;
    writeln!(
        out,
        "legal_ops_total の境界は `{legal_q1}`, `{legal_q2}`、move_dist_avg の境界は `{move_q1:.6}`, `{move_q2:.6}` である。低は第1境界以下、中は第1境界超かつ第2境界以下、高は第2境界超とした（同値があるため各層は必ずしも同数ではない）。"
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "| legal_ops_total \\ move_dist_avg | 低 | 中 | 高 | 計 |"
    )?;
    writeln!(out, "|---|---:|---:|---:|---:|")?;
    for (row, label) in ["低", "中", "高"].iter().enumerate() {
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} |",
            cells[row][0],
            cells[row][1],
            cells[row][2],
            cells[row].iter().sum::<usize>()
        )?;
    }
    writeln!(
        out,
        "| 計 | {} | {} | {} | {} |",
        cells.iter().map(|row| row[0]).sum::<usize>(),
        cells.iter().map(|row| row[1]).sum::<usize>(),
        cells.iter().map(|row| row[2]).sum::<usize>(),
        cases.len()
    )?;
    writeln!(out)?;

    writeln!(out, "## 壁なし基準値")?;
    writeln!(out)?;
    writeln!(out, "人工の N=20 壁なし盤面を同じコード経路で計算した。")?;
    writeln!(out)?;
    writeln!(out, "| 指標 | 基準値 |")?;
    writeln!(out, "|---|---:|")?;
    writeln!(out, "| legal_ops_total | {} |", baseline.legal_ops_total)?;
    writeln!(out, "| legal_ops_v | {} |", baseline.legal_ops_v)?;
    writeln!(out, "| legal_ops_h | {} |", baseline.legal_ops_h)?;
    writeln!(out, "| legal_area_max | {} |", baseline.legal_area_max)?;
    writeln!(out, "| big_ops_100 | {} |", baseline.big_ops_100)?;
    writeln!(out, "| big_ops_200 | {} |", baseline.big_ops_200)?;
    writeln!(out, "| maximal_rects | {} |", baseline.maximal_rects)?;
    writeln!(
        out,
        "| maximal_rect_area_max | {} |",
        baseline.maximal_rect_area_max
    )?;
    writeln!(out, "| move_deg_avg | {:.6} |", baseline.move_deg_avg)?;
    writeln!(out, "| move_dist_avg | {:.6} |", baseline.move_dist_avg)?;
    writeln!(out, "| move_dist_diam | {} |", baseline.move_dist_diam)?;
    writeln!(
        out,
        "| move_within2_ratio | {:.6} |",
        baseline.move_within2_ratio
    )?;
    writeln!(out, "| bfs_dist_avg | {:.6} |", baseline.bfs_dist_avg)?;
    writeln!(out, "| bfs_diam | {} |", baseline.bfs_diam)?;
    writeln!(
        out,
        "| manhattan_gap_avg | {:.6} |",
        baseline.manhattan_gap_avg
    )?;
    writeln!(out)?;

    let wall_values: Vec<f64> = cases.iter().map(|s| s.wall_edges as f64).collect();
    let move_dist_values: Vec<f64> = cases.iter().map(|s| s.move_dist_avg).collect();
    let big_values: Vec<f64> = cases.iter().map(|s| s.big_ops_100 as f64).collect();
    let corr_move = pearson(&wall_values, &move_dist_values);
    let corr_big = pearson(&wall_values, &big_values);
    let move_changes: Vec<f64> = cases
        .iter()
        .map(|s| (s.move_dist_avg / baseline.move_dist_avg - 1.0) * 100.0)
        .collect();
    let big_changes: Vec<f64> = cases
        .iter()
        .map(|s| (s.big_ops_100 as f64 / baseline.big_ops_100 as f64 - 1.0) * 100.0)
        .collect();
    let legal_changes: Vec<f64> = cases
        .iter()
        .map(|s| (s.legal_ops_total as f64 / baseline.legal_ops_total as f64 - 1.0) * 100.0)
        .collect();
    let move_change_summary = f64_distribution(move_changes);
    let big_change_summary = f64_distribution(big_changes);
    let legal_change_summary = f64_distribution(legal_changes);
    let bulk_loss = -big_change_summary.1;
    let move_growth = move_change_summary.1;
    let hypothesis_supported = bulk_loss > move_growth;

    writeln!(out, "## 仮説検証")?;
    writeln!(out)?;
    writeln!(
        out,
        "仮説: 壁は一枚移動距離よりも、大面積長方形による一括実行能力を強く悪化させる。"
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "- wall_edges と move_dist_avg の Pearson 相関係数は **{corr_move:.6}**（{}）である。",
        correlation_direction(corr_move)
    )?;
    writeln!(
        out,
        "- wall_edges と big_ops_100 の Pearson 相関係数は **{corr_big:.6}**（{}）である。",
        correlation_direction(corr_big)
    )?;
    writeln!(
        out,
        "- 壁なし基準からの move_dist_avg の変化率は min / 中央値 / max = **{:.2}% / {:+.2}% / {:+.2}%** である。",
        move_change_summary.0, move_change_summary.1, move_change_summary.2
    )?;
    writeln!(
        out,
        "- 壁なし基準からの big_ops_100 の変化率は min / 中央値 / max = **{:.2}% / {:+.2}% / {:+.2}%** である。",
        big_change_summary.0, big_change_summary.1, big_change_summary.2
    )?;
    writeln!(
        out,
        "- 参考として legal_ops_total の変化率は min / 中央値 / max = **{:.2}% / {:+.2}% / {:+.2}%** である。",
        legal_change_summary.0, legal_change_summary.1, legal_change_summary.2
    )?;
    writeln!(out)?;
    if hypothesis_supported {
        writeln!(
            out,
            "**結論: この100ケースでは仮説を支持する。** 中央値で、一枚移動距離の悪化が {move_growth:.2}% なのに対し、面積100以上の操作容量は {bulk_loss:.2}% 減少している。"
        )?;
    } else {
        writeln!(
            out,
            "**結論: この100ケースでは仮説を支持しない。** 中央値で、一枚移動距離の悪化が {move_growth:.2}%、面積100以上の操作容量の減少が {bulk_loss:.2}% であり、後者が前者を上回らない。"
        )?;
    }
    writeln!(out)?;

    let within_120 = cases
        .iter()
        .filter(|s| s.move_dist_avg <= baseline.move_dist_avg * 1.2 + 1e-12)
        .count();
    let below_half_big = cases
        .iter()
        .filter(|s| (s.big_ops_100 as f64) < baseline.big_ops_100 as f64 * 0.5)
        .count();
    writeln!(out, "## 判断材料")?;
    writeln!(out)?;
    writeln!(
        out,
        "- (a) move_dist_avg が壁なし基準の 1.2 倍以内: **{within_120}/{} ケース（{:.1}%）**",
        cases.len(),
        within_120 as f64 / cases.len() as f64 * 100.0
    )?;
    writeln!(
        out,
        "- (b) big_ops_100 が壁なし基準の 50% 未満: **{below_half_big}/{} ケース（{:.1}%）**",
        cases.len(),
        below_half_big as f64 / cases.len() as f64 * 100.0
    )?;
    writeln!(out, "- (c) blocked_detour_sum 上位10ケース:")?;
    writeln!(out)?;
    writeln!(
        out,
        "| 順位 | case | blocked_detour_sum | wall_edges | move_dist_avg | big_ops_100 |"
    )?;
    writeln!(out, "|---:|---:|---:|---:|---:|---:|")?;
    let mut stress: Vec<&CaseStats> = cases.iter().collect();
    stress.sort_by(|a, b| {
        b.blocked_detour_sum
            .cmp(&a.blocked_detour_sum)
            .then_with(|| a.case.cmp(&b.case))
    });
    for (rank, s) in stress.into_iter().take(10).enumerate() {
        writeln!(
            out,
            "| {} | {} | {} | {} | {:.6} | {} |",
            rank + 1,
            s.case,
            s.blocked_detour_sum,
            s.wall_edges,
            s.move_dist_avg,
            s.big_ops_100
        )?;
    }
    writeln!(out)?;

    let zero_wall: Vec<&str> = cases
        .iter()
        .filter(|s| s.wall_edges == 0)
        .map(|s| s.case.as_str())
        .collect();
    writeln!(out, "## Sanity check")?;
    writeln!(out)?;
    writeln!(
        out,
        "- PASS: 人工の壁なし盤面で legal_ops_total = 42,000、legal_ops_v = legal_ops_h = 21,000。"
    )?;
    writeln!(
        out,
        "- PASS: 人工の壁なし盤面の move_dist_diam は **{}**。",
        baseline.move_dist_diam
    )?;
    if zero_wall.is_empty() {
        writeln!(
            out,
            "- PASS: 入力100ケースに wall_edges = 0 のケースは存在しない（照合対象 0 件）。"
        )?;
    } else {
        writeln!(
            out,
            "- PASS: wall_edges = 0 の {} ケース（{}）はすべて legal_ops_total = 42,000。",
            zero_wall.len(),
            zero_wall.join(", ")
        )?;
    }
    writeln!(
        out,
        "- PASS: 全{}ケースで blocked_adj_pairs == wall_edges。",
        cases.len()
    )?;
    writeln!(
        out,
        "- PASS: 通常隣接グラフと一枚移動グラフは全ケースで連結し、全点対距離を取得できた。"
    )?;

    out.flush()
}

fn write_usize_distribution(
    out: &mut BufWriter<File>,
    label: &str,
    mut values: Vec<usize>,
) -> std::io::Result<()> {
    values.sort_unstable();
    writeln!(
        out,
        "| {label} | {} | {} | {} |",
        values[0],
        nearest_rank_usize(&values, 1, 2),
        values[values.len() - 1]
    )
}

fn write_u64_distribution(
    out: &mut BufWriter<File>,
    label: &str,
    mut values: Vec<u64>,
) -> std::io::Result<()> {
    values.sort_unstable();
    writeln!(
        out,
        "| {label} | {} | {} | {} |",
        values[0],
        values[(values.len() - 1) / 2],
        values[values.len() - 1]
    )
}

fn write_f64_distribution(
    out: &mut BufWriter<File>,
    label: &str,
    values: Vec<f64>,
) -> std::io::Result<()> {
    let (minimum, median, maximum) = f64_distribution(values);
    writeln!(
        out,
        "| {label} | {minimum:.6} | {median:.6} | {maximum:.6} |"
    )
}

fn f64_distribution(mut values: Vec<f64>) -> (f64, f64, f64) {
    values.sort_by(f64::total_cmp);
    (
        values[0],
        nearest_rank_f64(&values, 1, 2),
        values[values.len() - 1],
    )
}

fn tertile_bin_usize(value: usize, q1: usize, q2: usize) -> usize {
    if value <= q1 {
        0
    } else if value <= q2 {
        1
    } else {
        2
    }
}

fn tertile_bin_f64(value: f64, q1: f64, q2: f64) -> usize {
    if value <= q1 {
        0
    } else if value <= q2 {
        1
    } else {
        2
    }
}

fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    assert_eq!(xs.len(), ys.len());
    let mean_x = xs.iter().sum::<f64>() / xs.len() as f64;
    let mean_y = ys.iter().sum::<f64>() / ys.len() as f64;
    let mut covariance = 0.0;
    let mut variance_x = 0.0;
    let mut variance_y = 0.0;
    for (&x, &y) in xs.iter().zip(ys) {
        let dx = x - mean_x;
        let dy = y - mean_y;
        covariance += dx * dy;
        variance_x += dx * dx;
        variance_y += dy * dy;
    }
    covariance / (variance_x * variance_y).sqrt()
}

fn correlation_direction(value: f64) -> &'static str {
    if value >= 0.0 {
        "正の傾向"
    } else {
        "負の傾向"
    }
}
