// v001_baseline.rs
use std::io::{self, BufWriter, Read, Write};

#[cfg(feature = "local")]
#[derive(Debug, Default)]
struct TraceStats {
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }

    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }

    fn summary(&self) {
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {key}={value}");
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {key}={value:.3}");
        }
    }
}

#[cfg(feature = "local")]
macro_rules! local {
    ($($body:tt)*) => {{ $($body)* }};
}

#[cfg(not(feature = "local"))]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[cfg(feature = "local")]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let start = std::time::Instant::now();
        let result = $body;
        $trace.add_time_ms($key, start.elapsed().as_secs_f64() * 1000.0);
        result
    }};
}

#[cfg(not(feature = "local"))]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
}

#[derive(Clone, Copy)]
struct Operation {
    direction: char,
    r: usize,
    c: usize,
    h: usize,
    w: usize,
}

fn add_edge(graph: &mut [Vec<usize>], u: usize, v: usize) {
    graph[u].push(v);
    graph[v].push(u);
}

/// 連結な隣接グラフから root=0 の全域木を作る。
/// `order` の逆順は、各頂点が残りの木の葉になる削除順である。
fn build_spanning_tree(graph: &[Vec<usize>]) -> (Vec<usize>, Vec<usize>) {
    let total = graph.len();
    let mut parent = vec![usize::MAX; total];
    let mut order = Vec::with_capacity(total);
    parent[0] = 0;
    order.push(0);

    let mut head = 0;
    while head < order.len() {
        let v = order[head];
        head += 1;
        for &to in &graph[v] {
            if parent[to] == usize::MAX {
                parent[to] = v;
                order.push(to);
            }
        }
    }
    assert_eq!(order.len(), total, "盤面の隣接グラフが連結でない");
    (parent, order)
}

/// 全域木における start から goal への唯一の経路を返す。
fn path_in_tree(start: usize, goal: usize, parent: &[usize]) -> Vec<usize> {
    let total = parent.len();
    let mut from_chain = Vec::new();
    let mut from_index = vec![usize::MAX; total];
    let mut v = start;
    loop {
        from_index[v] = from_chain.len();
        from_chain.push(v);
        if parent[v] == v {
            break;
        }
        v = parent[v];
    }

    let mut to_chain = Vec::new();
    let mut v = goal;
    while from_index[v] == usize::MAX {
        to_chain.push(v);
        v = parent[v];
    }
    let lca = v;

    let mut path = from_chain[..=from_index[lca]].to_vec();
    to_chain.reverse();
    path.extend(to_chain);
    path
}

/// 盤面上で隣接する u, v のカードを交換し、その操作を出力列へ記録する。
fn swap_adjacent(
    u: usize,
    v: usize,
    n: usize,
    board: &mut [usize],
    position: &mut [usize],
    operations: &mut Vec<Operation>,
) {
    let (ru, cu) = (u / n, u % n);
    let (rv, cv) = (v / n, v % n);
    let operation = if ru == rv {
        assert_eq!(cu.abs_diff(cv), 1);
        Operation {
            direction: 'H',
            r: ru,
            c: cu.min(cv),
            h: 1,
            w: 2,
        }
    } else {
        assert_eq!(cu, cv);
        assert_eq!(ru.abs_diff(rv), 1);
        Operation {
            direction: 'V',
            r: ru.min(rv),
            c: cu,
            h: 2,
            w: 1,
        }
    };

    let card_u = board[u];
    let card_v = board[v];
    board.swap(u, v);
    position[card_u] = v;
    position[card_v] = u;
    operations.push(operation);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_whitespace();

    let n: usize = tokens.next().unwrap().parse().unwrap();
    let total = n * n;
    let mut board = Vec::with_capacity(total);
    let mut position = vec![usize::MAX; total];
    for cell in 0..total {
        let card: usize = tokens.next().unwrap().parse().unwrap();
        assert!(card < total);
        assert_eq!(position[card], usize::MAX, "カード番号が重複している");
        board.push(card);
        position[card] = cell;
    }

    let mut graph = vec![Vec::new(); total];
    // V[i][j] は (i,j) と (i,j+1) を隔てる縦壁である。
    for r in 0..n {
        let walls = tokens.next().unwrap().as_bytes();
        assert_eq!(walls.len(), n - 1);
        for c in 0..n - 1 {
            if walls[c] == b'0' {
                add_edge(&mut graph, r * n + c, r * n + c + 1);
            }
        }
    }
    // H[i][j] は (i,j) と (i+1,j) を隔てる横壁である。
    for r in 0..n - 1 {
        let walls = tokens.next().unwrap().as_bytes();
        assert_eq!(walls.len(), n);
        for c in 0..n {
            if walls[c] == b'0' {
                add_edge(&mut graph, r * n + c, (r + 1) * n + c);
            }
        }
    }

    #[cfg(feature = "local")]
    let mut trace = TraceStats::default();
    let (parent, order) = local_time!(trace, "build_spanning_tree", {
        build_spanning_tree(&graph)
    });

    let mut operations = Vec::new();
    let mut fixed = vec![false; total];
    // 発見順の逆順で処理すると、対象頂点の子孫はすべて確定済みである。
    // よって残った全域木で対象は葉になり、その頂点を固定しても連結性を失わない。
    local_time!(trace, "route_cards", {
        for target in order.into_iter().rev().filter(|&v| v != 0) {
            let source = position[target];
            assert!(!fixed[source], "固定済みマスに別のカードがある");

            let path = path_in_tree(source, target, &parent);
            for edge in path.windows(2) {
                assert!(!fixed[edge[0]] && !fixed[edge[1]]);
                swap_adjacent(
                    edge[0],
                    edge[1],
                    n,
                    &mut board,
                    &mut position,
                    &mut operations,
                );
            }
            assert_eq!(board[target], target);
            fixed[target] = true;
            local! { trace.count_by("fixed_leaves", 1); }
        }
    });
    assert_eq!(board[0], 0);
    assert!(operations.len() <= 100_000);

    local! {
        trace.count_by("tree_edge_swaps", operations.len() as i64);
        trace.summary();
    }

    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    for operation in operations {
        writeln!(
            output,
            "{} {} {} {} {}",
            operation.direction, operation.r, operation.c, operation.h, operation.w
        )
        .unwrap();
    }
}
