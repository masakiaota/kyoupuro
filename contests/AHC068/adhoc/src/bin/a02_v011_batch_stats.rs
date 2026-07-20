// a02_v011_batch_stats.rs

use std::collections::VecDeque;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const EXPECTED_N: usize = 20;
const EXPECTED_CASES: usize = 100;
const STRESS_CASES: [&str; 5] = ["0037", "0039", "0092", "0089", "0070"];

#[derive(Clone)]
struct Input {
    n: usize,
    board: Vec<usize>,
    vertical_walls: Vec<Vec<bool>>,
    horizontal_walls: Vec<Vec<bool>>,
}

#[derive(Clone, Copy)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy)]
struct Operation {
    direction: Direction,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

#[derive(Default, Clone)]
struct Totals {
    cases: u64,
    operations: u64,
    swap_pairs: u64,
    tree_beneficiaries: u64,
    extra_tree_beneficiaries: u64,
    axis_beneficiaries: u64,
    opposing_pairs: u64,
    opposing_operations: u64,
    multi_tree_operations: u64,
    supported_operations: u64,
    supported_opposing_operations: u64,
    tree_gain: i64,
    initial_tree_sum: u64,
    beneficiary_bins: [u64; 7],
}

impl Totals {
    fn add(&mut self, other: &Self) {
        self.cases += other.cases;
        self.operations += other.operations;
        self.swap_pairs += other.swap_pairs;
        self.tree_beneficiaries += other.tree_beneficiaries;
        self.extra_tree_beneficiaries += other.extra_tree_beneficiaries;
        self.axis_beneficiaries += other.axis_beneficiaries;
        self.opposing_pairs += other.opposing_pairs;
        self.opposing_operations += other.opposing_operations;
        self.multi_tree_operations += other.multi_tree_operations;
        self.supported_operations += other.supported_operations;
        self.supported_opposing_operations += other.supported_opposing_operations;
        self.tree_gain += other.tree_gain;
        self.initial_tree_sum += other.initial_tree_sum;
        for (to, from) in self
            .beneficiary_bins
            .iter_mut()
            .zip(other.beneficiary_bins.iter())
        {
            *to += from;
        }
    }
}

struct CaseStats {
    case: String,
    wall_edges: usize,
    totals: Totals,
    max_tree_beneficiaries: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!(
            "usage: {} <input_dir> <v011_output_dir> <analysis_output_dir>",
            args.first().map_or("a02_v011_batch_stats", String::as_str)
        ));
    }
    let input_dir = Path::new(&args[1]);
    let solver_output_dir = Path::new(&args[2]);
    let analysis_output_dir = Path::new(&args[3]);

    let mut input_paths = collect_txt_files(input_dir)?;
    input_paths.sort();
    if input_paths.len() != EXPECTED_CASES {
        return Err(format!(
            "入力ケース数が {EXPECTED_CASES} ではない: {}",
            input_paths.len()
        ));
    }

    let mut cases = Vec::with_capacity(input_paths.len());
    for input_path in &input_paths {
        let case = input_path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("case名を読めない: {}", input_path.display()))?;
        let output_path = solver_output_dir.join(format!("{case}.txt"));
        let input = parse_input(input_path)?;
        let operations = parse_operations(&output_path, input.n)?;
        cases.push(analyze_case(case, &input, &operations)?);
    }

    let wall_free_count = cases.iter().filter(|case| case.wall_edges == 0).count();
    if wall_free_count != 9 {
        return Err(format!("壁なしケース数が9ではない: {wall_free_count}"));
    }
    let stress_count = cases
        .iter()
        .filter(|case| STRESS_CASES.contains(&case.case.as_str()))
        .count();
    if stress_count != STRESS_CASES.len() {
        return Err(format!("stressケース数が5ではない: {stress_count}"));
    }

    fs::create_dir_all(analysis_output_dir).map_err(|error| {
        format!(
            "分析出力ディレクトリを作成できない {}: {error}",
            analysis_output_dir.display()
        )
    })?;
    write_cases_csv(&analysis_output_dir.join("a02_cases.csv"), &cases)?;
    write_summary(
        &analysis_output_dir.join("a02_summary.md"),
        input_dir,
        solver_output_dir,
        &cases,
    )?;

    let all = aggregate(cases.iter());
    println!("processed {} cases", cases.len());
    println!("legal replay and final E=0: all passed");
    println!("wall-free cases: {wall_free_count}, stress cases: {stress_count}");
    println!(
        "opposing-op rate={:.3}%, extra tree beneficiaries/op={:.3}",
        ratio(all.opposing_operations, all.operations) * 100.0,
        ratio(all.extra_tree_beneficiaries, all.operations)
    );
    Ok(())
}

fn collect_txt_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("ディレクトリを読めない {}: {error}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("ディレクトリ走査に失敗: {error}"))?
            .path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("txt") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn parse_input(path: &Path) -> Result<Input, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("入力を読めない {}: {error}", path.display()))?;
    let mut tokens = source.split_whitespace();
    let n = parse_usize(tokens.next(), path, "N")?;
    if n != EXPECTED_N {
        return Err(format!("{}: N={n}", path.display()));
    }
    let mut board = Vec::with_capacity(n * n);
    for index in 0..n * n {
        board.push(parse_usize(tokens.next(), path, &format!("a[{index}]"))?);
    }
    let mut vertical_walls = Vec::with_capacity(n);
    for i in 0..n {
        let row = tokens
            .next()
            .ok_or_else(|| format!("{}: V[{i}] がない", path.display()))?;
        if row.len() != n - 1 || !row.bytes().all(|value| value == b'0' || value == b'1') {
            return Err(format!("{}: V[{i}] が不正", path.display()));
        }
        vertical_walls.push(row.bytes().map(|value| value == b'1').collect());
    }
    let mut horizontal_walls = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let row = tokens
            .next()
            .ok_or_else(|| format!("{}: H[{i}] がない", path.display()))?;
        if row.len() != n || !row.bytes().all(|value| value == b'0' || value == b'1') {
            return Err(format!("{}: H[{i}] が不正", path.display()));
        }
        horizontal_walls.push(row.bytes().map(|value| value == b'1').collect());
    }
    if tokens.next().is_some() {
        return Err(format!("{}: 余分な入力token", path.display()));
    }
    Ok(Input {
        n,
        board,
        vertical_walls,
        horizontal_walls,
    })
}

fn parse_usize(token: Option<&str>, path: &Path, label: &str) -> Result<usize, String> {
    token
        .ok_or_else(|| format!("{}: {label} がない", path.display()))?
        .parse()
        .map_err(|error| format!("{}: {label} を読めない: {error}", path.display()))
}

fn parse_operations(path: &Path, n: usize) -> Result<Vec<Operation>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("解答列を読めない {}: {error}", path.display()))?;
    let mut operations = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!(
                "{}:{}: フィールド数が5ではない",
                path.display(),
                line_index + 1
            ));
        }
        let direction = match fields[0] {
            "V" => Direction::Vertical,
            "H" => Direction::Horizontal,
            other => {
                return Err(format!(
                    "{}:{}: direction={other}",
                    path.display(),
                    line_index + 1
                ));
            }
        };
        let values = fields[1..]
            .iter()
            .map(|field| field.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                format!(
                    "{}:{}: 座標を読めない: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
        let op = Operation {
            direction,
            r: values[0],
            c: values[1],
            h: values[2],
            w: values[3],
        };
        if op.r + op.h > n || op.c + op.w > n || op.h == 0 || op.w == 0 {
            return Err(format!("{}:{}: 範囲外操作", path.display(), line_index + 1));
        }
        operations.push(op);
    }
    Ok(operations)
}

fn analyze_case(case: &str, input: &Input, operations: &[Operation]) -> Result<CaseStats, String> {
    let distance = build_tree_distances(input)?;
    let mut board = input.board.clone();
    let initial_tree_sum = board
        .iter()
        .enumerate()
        .map(|(cell, &card)| distance[cell * input.n * input.n + card] as u64)
        .sum::<u64>();
    let mut totals = Totals {
        cases: 1,
        initial_tree_sum,
        ..Totals::default()
    };
    let mut max_tree_beneficiaries = 0;

    for (operation_index, &op) in operations.iter().enumerate() {
        if !is_legal(input, op) {
            return Err(format!("{case}: operation {operation_index} が非合法"));
        }
        let pairs = operation_pairs(input.n, op);
        let supported = match op.direction {
            Direction::Horizontal => op.h > 1,
            Direction::Vertical => op.w > 1,
        };
        let mut tree_beneficiaries = 0;
        let mut axis_beneficiaries = 0;
        let mut opposing_pairs = 0;
        let mut operation_tree_gain = 0_i64;

        for &(first, second) in &pairs {
            let first_card = board[first];
            let second_card = board[second];
            let old_first = distance[first * input.n * input.n + first_card] as i64;
            let new_first = distance[second * input.n * input.n + first_card] as i64;
            let old_second = distance[second * input.n * input.n + second_card] as i64;
            let new_second = distance[first * input.n * input.n + second_card] as i64;
            tree_beneficiaries += usize::from(new_first < old_first);
            tree_beneficiaries += usize::from(new_second < old_second);
            operation_tree_gain += old_first + old_second - new_first - new_second;

            let first_axis_gain = axis_distance(input.n, first_card, first, op.direction)
                - axis_distance(input.n, first_card, second, op.direction);
            let second_axis_gain = axis_distance(input.n, second_card, second, op.direction)
                - axis_distance(input.n, second_card, first, op.direction);
            axis_beneficiaries += usize::from(first_axis_gain > 0);
            axis_beneficiaries += usize::from(second_axis_gain > 0);
            opposing_pairs += usize::from(first_axis_gain > 0 && second_axis_gain > 0);
        }
        if tree_beneficiaries == 0 {
            return Err(format!(
                "{case}: operation {operation_index} に木距離改善カードがない"
            ));
        }
        let bin = beneficiary_bin(tree_beneficiaries);
        totals.beneficiary_bins[bin] += 1;
        totals.operations += 1;
        totals.swap_pairs += pairs.len() as u64;
        totals.tree_beneficiaries += tree_beneficiaries as u64;
        totals.extra_tree_beneficiaries += tree_beneficiaries.saturating_sub(1) as u64;
        totals.axis_beneficiaries += axis_beneficiaries as u64;
        totals.opposing_pairs += opposing_pairs as u64;
        totals.opposing_operations += u64::from(opposing_pairs > 0);
        totals.multi_tree_operations += u64::from(tree_beneficiaries >= 2);
        totals.supported_operations += u64::from(supported);
        totals.supported_opposing_operations += u64::from(supported && opposing_pairs > 0);
        totals.tree_gain += operation_tree_gain;
        max_tree_beneficiaries = max_tree_beneficiaries.max(tree_beneficiaries);
        for &(first, second) in &pairs {
            board.swap(first, second);
        }
    }

    if board.iter().enumerate().any(|(cell, &card)| cell != card) {
        let misplaced = board
            .iter()
            .enumerate()
            .filter(|&(cell, card)| cell != *card)
            .count();
        return Err(format!("{case}: final E={misplaced}"));
    }
    if totals.tree_gain != totals.initial_tree_sum as i64 {
        return Err(format!(
            "{case}: 木距離差分不一致 gain={} initial={}",
            totals.tree_gain, totals.initial_tree_sum
        ));
    }

    let wall_edges = input
        .vertical_walls
        .iter()
        .flatten()
        .chain(input.horizontal_walls.iter().flatten())
        .filter(|&&wall| wall)
        .count();
    Ok(CaseStats {
        case: case.to_owned(),
        wall_edges,
        totals,
        max_tree_beneficiaries,
    })
}

fn axis_distance(n: usize, card: usize, cell: usize, direction: Direction) -> i32 {
    let (target, current) = match direction {
        Direction::Vertical => (card / n, cell / n),
        Direction::Horizontal => (card % n, cell % n),
    };
    target.abs_diff(current) as i32
}

fn beneficiary_bin(count: usize) -> usize {
    match count {
        0 => 0,
        1 => 1,
        2 => 2,
        3..=4 => 3,
        5..=8 => 4,
        9..=16 => 5,
        _ => 6,
    }
}

fn operation_pairs(n: usize, op: Operation) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    match op.direction {
        Direction::Vertical => {
            let half = op.h / 2;
            for x in 0..half {
                for y in 0..op.w {
                    pairs.push(((op.r + x) * n + op.c + y, (op.r + half + x) * n + op.c + y));
                }
            }
        }
        Direction::Horizontal => {
            let half = op.w / 2;
            for x in 0..op.h {
                for y in 0..half {
                    pairs.push(((op.r + x) * n + op.c + y, (op.r + x) * n + op.c + half + y));
                }
            }
        }
    }
    pairs
}

fn is_legal(input: &Input, op: Operation) -> bool {
    if op.h == 0 || op.w == 0 || op.r + op.h > input.n || op.c + op.w > input.n {
        return false;
    }
    match op.direction {
        Direction::Vertical if op.h % 2 != 0 => return false,
        Direction::Horizontal if op.w % 2 != 0 => return false,
        _ => {}
    }
    for i in op.r..op.r + op.h {
        for j in op.c..op.c + op.w.saturating_sub(1) {
            if input.vertical_walls[i][j] {
                return false;
            }
        }
    }
    for i in op.r..op.r + op.h.saturating_sub(1) {
        for j in op.c..op.c + op.w {
            if input.horizontal_walls[i][j] {
                return false;
            }
        }
    }
    true
}

fn build_tree_distances(input: &Input) -> Result<Vec<u16>, String> {
    let cells = input.n * input.n;
    let root = (input.n / 2) * input.n + input.n / 2;
    let mut parent = vec![cells; cells];
    parent[root] = root;
    let mut queue = VecDeque::from([root]);
    while let Some(cell) = queue.pop_front() {
        for next in open_neighbors(input, cell) {
            if parent[next] == cells {
                parent[next] = cell;
                queue.push_back(next);
            }
        }
    }
    if parent.iter().any(|&value| value == cells) {
        return Err("開辺グラフが非連結".to_owned());
    }
    let mut tree = vec![Vec::new(); cells];
    for cell in 0..cells {
        if cell != root {
            tree[cell].push(parent[cell]);
            tree[parent[cell]].push(cell);
        }
    }
    let mut distance = vec![u16::MAX; cells * cells];
    for source in 0..cells {
        let mut queue = VecDeque::from([source]);
        distance[source * cells + source] = 0;
        while let Some(cell) = queue.pop_front() {
            let next_distance = distance[source * cells + cell] + 1;
            for &next in &tree[cell] {
                if distance[source * cells + next] == u16::MAX {
                    distance[source * cells + next] = next_distance;
                    queue.push_back(next);
                }
            }
        }
    }
    Ok(distance)
}

fn open_neighbors(input: &Input, cell: usize) -> Vec<usize> {
    let i = cell / input.n;
    let j = cell % input.n;
    let mut result = Vec::with_capacity(4);
    // v011 と同じ順序で BFS 木を固定する。
    if i > 0 && !input.horizontal_walls[i - 1][j] {
        result.push(cell - input.n);
    }
    if j > 0 && !input.vertical_walls[i][j - 1] {
        result.push(cell - 1);
    }
    if j + 1 < input.n && !input.vertical_walls[i][j] {
        result.push(cell + 1);
    }
    if i + 1 < input.n && !input.horizontal_walls[i][j] {
        result.push(cell + input.n);
    }
    result
}

fn aggregate<'a>(cases: impl Iterator<Item = &'a CaseStats>) -> Totals {
    let mut result = Totals::default();
    for case in cases {
        result.add(&case.totals);
    }
    result
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn write_cases_csv(path: &Path, cases: &[CaseStats]) -> Result<(), String> {
    let file =
        File::create(path).map_err(|error| format!("CSVを作れない {}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "case,wall_edges,T,swap_pairs,tree_beneficiaries,extra_tree_beneficiaries,extra_per_op,multi_tree_op_rate,opposing_pairs,opposing_op_rate,supported_ops,supported_opposing_rate,max_tree_beneficiaries,initial_tree_sum")
        .map_err(|error| format!("CSV headerを書けない: {error}"))?;
    for case in cases {
        let totals = &case.totals;
        writeln!(
            writer,
            "{},{},{},{},{},{},{:.6},{:.6},{},{:.6},{},{:.6},{},{}",
            case.case,
            case.wall_edges,
            totals.operations,
            totals.swap_pairs,
            totals.tree_beneficiaries,
            totals.extra_tree_beneficiaries,
            ratio(totals.extra_tree_beneficiaries, totals.operations),
            ratio(totals.multi_tree_operations, totals.operations),
            totals.opposing_pairs,
            ratio(totals.opposing_operations, totals.operations),
            totals.supported_operations,
            ratio(
                totals.supported_opposing_operations,
                totals.supported_operations
            ),
            case.max_tree_beneficiaries,
            totals.initial_tree_sum,
        )
        .map_err(|error| format!("CSV rowを書けない: {error}"))?;
    }
    Ok(())
}

fn write_summary(
    path: &Path,
    input_dir: &Path,
    solver_output_dir: &Path,
    cases: &[CaseStats],
) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|error| format!("summaryを作れない {}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let all = aggregate(cases.iter());
    let wall_free = aggregate(cases.iter().filter(|case| case.wall_edges == 0));
    let stress = aggregate(
        cases
            .iter()
            .filter(|case| STRESS_CASES.contains(&case.case.as_str())),
    );

    writeln!(writer, "# A-02 v011 巻き添え益・対向需要分析").map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(writer, "- 入力: `{}`", input_dir.display()).map_err(|error| error.to_string())?;
    writeln!(writer, "- 解答列: `{}`", solver_output_dir.display())
        .map_err(|error| error.to_string())?;
    writeln!(
        writer,
        "- 全100列で操作合法性、最終 `E=0`、木距離差分累積一致を確認した。"
    )
    .map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(writer, "## 層別集計").map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(writer, "| 層 | cases | T | 追加木距離改善枚数/op | 複数改善op率 | 対向需要op率 | 対向需要pair率 | 支持拡大op率 |")
        .map_err(|error| error.to_string())?;
    writeln!(writer, "|---|---:|---:|---:|---:|---:|---:|---:|")
        .map_err(|error| error.to_string())?;
    for (label, totals) in [("全体", &all), ("壁なし", &wall_free), ("stress", &stress)] {
        writeln!(
            writer,
            "| {label} | {} | {} | {:.3} | {:.2}% | {:.2}% | {:.2}% | {:.2}% |",
            totals.cases,
            totals.operations,
            ratio(totals.extra_tree_beneficiaries, totals.operations),
            ratio(totals.multi_tree_operations, totals.operations) * 100.0,
            ratio(totals.opposing_operations, totals.operations) * 100.0,
            ratio(totals.opposing_pairs, totals.swap_pairs) * 100.0,
            ratio(totals.supported_operations, totals.operations) * 100.0,
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(writer, "## 1操作の木距離改善カード数分布").map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(writer, "| 改善枚数 | operations | 比率 |").map_err(|error| error.to_string())?;
    writeln!(writer, "|---|---:|---:|").map_err(|error| error.to_string())?;
    let labels = ["0", "1", "2", "3–4", "5–8", "9–16", "17以上"];
    for (label, &count) in labels.iter().zip(all.beneficiary_bins.iter()) {
        writeln!(
            writer,
            "| {label} | {count} | {:.2}% |",
            ratio(count, all.operations) * 100.0
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(writer).map_err(|error| error.to_string())?;
    let direct_gate = ratio(all.opposing_operations, all.operations) >= 0.10
        && ratio(all.extra_tree_beneficiaries, all.operations) >= 1.0;
    writeln!(writer, "## 事前登録 gate").map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writeln!(
        writer,
        "- 対向需要op率 >= 10%: **{}** ({:.2}%)",
        if ratio(all.opposing_operations, all.operations) >= 0.10 {
            "達成"
        } else {
            "未達"
        },
        ratio(all.opposing_operations, all.operations) * 100.0
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        writer,
        "- 追加木距離改善枚数/op >= 1: **{}** ({:.3})",
        if ratio(all.extra_tree_beneficiaries, all.operations) >= 1.0 {
            "達成"
        } else {
            "未達"
        },
        ratio(all.extra_tree_beneficiaries, all.operations)
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        writer,
        "- 機械的な次候補: **{}**",
        if direct_gate {
            "IDEA-C17 帯位置・支持区間同時最適化"
        } else {
            "IDEA-C4 直交整地を含む需要被覆"
        }
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
