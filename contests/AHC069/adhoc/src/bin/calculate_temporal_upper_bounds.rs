// calculate_temporal_upper_bounds.rs
#![allow(non_snake_case)]

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const HORIZON: i64 = 100_000;
const INF: f64 = 1.0e100;

#[derive(Clone, Debug)]
struct Group {
    S: i64,
    T: i64,
    P: i64,
    max_fee: i64,
}

#[derive(Clone, Debug)]
struct Case {
    name: String,
    R: f64,
    grass_count: i64,
    groups: Vec<Group>,
}

#[derive(Clone, Debug)]
struct Edge {
    to: usize,
    rev: usize,
    cap: i64,
    original_cap: i64,
    cost: f64,
}

#[derive(Copy, Clone, Debug)]
struct HeapState {
    distance: f64,
    node: usize,
}

impl PartialEq for HeapState {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.distance.to_bits() == other.distance.to_bits()
    }
}

impl Eq for HeapState {}

impl PartialOrd for HeapState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .distance
            .total_cmp(&self.distance)
            .then_with(|| other.node.cmp(&self.node))
    }
}

#[derive(Clone, Debug)]
struct Analysis {
    theta_estimate: f64,
    total_cell_time: i64,
    max_active_demand: i64,
    overflow_cell_time: i64,
    clipped_cell_time: i64,
    temporal_capacity_upper: f64,
    partial_groups: usize,
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "adhoc の親ディレクトリを取得できない".to_string())
}

fn max_compactness(P: i64) -> f64 {
    let perimeter = 2.0 * (2.0 * (P as f64).sqrt()).ceil();
    4.0 * (P as f64).sqrt() / perimeter
}

fn parse_case(path: &Path) -> Result<Case, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{} を読めない: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} の先頭行がない", path.display()))?;
    let mut header_tokens = header.split_whitespace();
    let N: usize = header_tokens
        .next()
        .ok_or_else(|| "N がない".to_string())?
        .parse()
        .map_err(|error| format!("N が不正: {error}"))?;
    let M: usize = header_tokens
        .next()
        .ok_or_else(|| "M がない".to_string())?
        .parse()
        .map_err(|error| format!("M が不正: {error}"))?;
    let R: f64 = header_tokens
        .next()
        .ok_or_else(|| "R がない".to_string())?
        .parse()
        .map_err(|error| format!("R が不正: {error}"))?;

    let mut grass_count = 0_i64;
    for _ in 0..N {
        let row = lines
            .next()
            .ok_or_else(|| format!("{} の盤面行が不足", path.display()))?;
        grass_count += row.bytes().filter(|&cell| cell == b'.').count() as i64;
    }

    let mut groups = Vec::with_capacity(M);
    for expected_i in 0..M {
        let line = lines
            .next()
            .ok_or_else(|| format!("{} のgroup行が不足", path.display()))?;
        let values = line
            .split_whitespace()
            .map(|token| token.parse::<i64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("group行が不正: {line}: {error}"))?;
        if values.len() != 5 || values[0] != expected_i as i64 {
            return Err(format!("group行の形式または番号が不正: {line}"));
        }
        let S = values[1];
        let T = values[2];
        let P = values[3];
        let V = values[4];
        let max_fee = ((V as f64) * max_compactness(P)).round() as i64;
        groups.push(Group { S, T, P, max_fee });
    }

    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("case名を取得できない: {}", path.display()))?
        .to_string();
    Ok(Case {
        name,
        R,
        grass_count,
        groups,
    })
}

fn add_edge(graph: &mut [Vec<Edge>], from: usize, to: usize, cap: i64, cost: f64) -> usize {
    let forward_index = graph[from].len();
    let reverse_index = graph[to].len();
    graph[from].push(Edge {
        to,
        rev: reverse_index,
        cap,
        original_cap: cap,
        cost,
    });
    graph[to].push(Edge {
        to: from,
        rev: forward_index,
        cap: 0,
        original_cap: 0,
        cost: -cost,
    });
    forward_index
}

fn solve_temporal_capacity_upper(
    groups: &[Group],
    grass_count: i64,
) -> Result<(f64, Vec<i64>, Vec<i64>), String> {
    let mut times = Vec::with_capacity(2 * groups.len() + 2);
    times.push(0);
    times.push(HORIZON);
    for group in groups {
        times.push(group.S);
        times.push(group.T);
    }
    times.sort_unstable();
    times.dedup();

    let mut graph = vec![Vec::<Edge>::new(); times.len()];
    for node in 0..times.len() - 1 {
        add_edge(&mut graph, node, node + 1, grass_count, 0.0);
    }

    let mut group_edges = Vec::with_capacity(groups.len());
    for group in groups {
        let from = times
            .binary_search(&group.S)
            .map_err(|_| format!("開始時刻 {} が見つからない", group.S))?;
        let to = times
            .binary_search(&group.T)
            .map_err(|_| format!("退去時刻 {} が見つからない", group.T))?;
        let edge_index = add_edge(
            &mut graph,
            from,
            to,
            group.P,
            -(group.max_fee as f64) / (group.P as f64),
        );
        group_edges.push((from, edge_index));
    }

    // 初期残余グラフは前向きDAGなので、負辺を含む最短距離を時刻順に求められる。
    let mut potential = vec![INF; times.len()];
    potential[0] = 0.0;
    for from in 0..times.len() {
        if potential[from] >= INF / 2.0 {
            continue;
        }
        for edge in &graph[from] {
            if edge.cap > 0 && edge.to > from {
                let candidate = potential[from] + edge.cost;
                if candidate < potential[edge.to] {
                    potential[edge.to] = candidate;
                }
            }
        }
    }
    if potential.iter().any(|&value| value >= INF / 2.0) {
        return Err("初期残余グラフに到達不能な時刻点がある".to_string());
    }

    let sink = times.len() - 1;
    let mut flow = 0_i64;
    let mut min_cost = 0.0_f64;
    while flow < grass_count {
        let mut distance = vec![INF; times.len()];
        let mut previous = vec![None::<(usize, usize)>; times.len()];
        let mut heap = BinaryHeap::new();
        distance[0] = 0.0;
        heap.push(HeapState {
            distance: 0.0,
            node: 0,
        });

        while let Some(state) = heap.pop() {
            if state.distance > distance[state.node] + 1.0e-9 {
                continue;
            }
            for (edge_index, edge) in graph[state.node].iter().enumerate() {
                if edge.cap <= 0 {
                    continue;
                }
                let mut reduced_cost = edge.cost + potential[state.node] - potential[edge.to];
                if reduced_cost < 0.0 && reduced_cost > -1.0e-7 {
                    reduced_cost = 0.0;
                }
                if reduced_cost < -1.0e-7 {
                    return Err(format!(
                        "負の換算費用が残った: from={} to={} cost={reduced_cost}",
                        state.node, edge.to
                    ));
                }
                let candidate = state.distance + reduced_cost;
                if candidate < distance[edge.to] - 1.0e-10 {
                    distance[edge.to] = candidate;
                    previous[edge.to] = Some((state.node, edge_index));
                    heap.push(HeapState {
                        distance: candidate,
                        node: edge.to,
                    });
                }
            }
        }

        if previous[sink].is_none() {
            return Err("終点まで必要流量を送れない".to_string());
        }
        for node in 0..times.len() {
            if distance[node] < INF / 2.0 {
                potential[node] += distance[node];
            }
        }

        let mut add_flow = grass_count - flow;
        let mut node = sink;
        let mut path_cost = 0.0;
        while node != 0 {
            let (from, edge_index) =
                previous[node].ok_or_else(|| "最短路の復元に失敗".to_string())?;
            let edge = &graph[from][edge_index];
            add_flow = add_flow.min(edge.cap);
            path_cost += edge.cost;
            node = from;
        }

        let mut node = sink;
        while node != 0 {
            let (from, edge_index) =
                previous[node].ok_or_else(|| "最短路の更新に失敗".to_string())?;
            let to = graph[from][edge_index].to;
            let reverse_index = graph[from][edge_index].rev;
            graph[from][edge_index].cap -= add_flow;
            graph[to][reverse_index].cap += add_flow;
            node = from;
        }
        flow += add_flow;
        min_cost += (add_flow as f64) * path_cost;
    }

    let mut used_cells = Vec::with_capacity(groups.len());
    for &(from, edge_index) in &group_edges {
        let edge = &graph[from][edge_index];
        used_cells.push(edge.original_cap - edge.cap);
    }

    let objective = groups
        .iter()
        .zip(&used_cells)
        .map(|(group, &used)| (used as f64) * (group.max_fee as f64) / (group.P as f64))
        .sum::<f64>();
    if (objective + min_cost).abs() > 1.0e-5 * objective.max(1.0) {
        return Err(format!(
            "目的値と最小費用が一致しない: objective={objective}, cost={min_cost}"
        ));
    }
    Ok((objective, used_cells, times))
}

fn analyze(case: &Case) -> Result<Analysis, String> {
    let total_cell_time = case
        .groups
        .iter()
        .map(|group| group.P * (group.T - group.S))
        .sum::<i64>();
    let max_fee_sum = case.groups.iter().map(|group| group.max_fee).sum::<i64>();

    let (temporal_capacity_upper, used_cells, times) =
        solve_temporal_capacity_upper(&case.groups, case.grass_count)?;
    let mut demand_difference = vec![0_i64; times.len() + 1];
    let mut relaxed_difference = vec![0_i64; times.len() + 1];
    for (group, &used) in case.groups.iter().zip(&used_cells) {
        let from = times
            .binary_search(&group.S)
            .map_err(|_| "需要の開始時刻が見つからない".to_string())?;
        let to = times
            .binary_search(&group.T)
            .map_err(|_| "需要の退去時刻が見つからない".to_string())?;
        demand_difference[from] += group.P;
        demand_difference[to] -= group.P;
        relaxed_difference[from] += used;
        relaxed_difference[to] -= used;
    }

    let mut active_demand = 0_i64;
    let mut relaxed_occupancy = 0_i64;
    let mut max_active_demand = 0_i64;
    let mut overflow_cell_time = 0_i64;
    let mut clipped_cell_time = 0_i64;
    for segment in 0..times.len() - 1 {
        active_demand += demand_difference[segment];
        relaxed_occupancy += relaxed_difference[segment];
        if relaxed_occupancy > case.grass_count {
            return Err(format!(
                "{}: 緩和解の占有量が容量を超えた: segment={segment}, occupancy={relaxed_occupancy}",
                case.name
            ));
        }
        max_active_demand = max_active_demand.max(active_demand);
        let duration = times[segment + 1] - times[segment];
        overflow_cell_time += (active_demand - case.grass_count).max(0) * duration;
        clipped_cell_time += active_demand.min(case.grass_count) * duration;
    }
    if temporal_capacity_upper > max_fee_sum as f64 + 1.0e-5 {
        return Err(format!(
            "{}: 時間容量上限が内部検証用の最大値を超えた: upper={temporal_capacity_upper}, check={max_fee_sum}",
            case.name
        ));
    }
    let expected_clipped = total_cell_time - overflow_cell_time;
    if clipped_cell_time != expected_clipped {
        return Err(format!(
            "{}: 時刻別セル時間が一致しない: clipped={clipped_cell_time}, expected={expected_clipped}",
            case.name
        ));
    }
    let partial_groups = case
        .groups
        .iter()
        .zip(&used_cells)
        .filter(|(group, used)| **used > 0 && **used < group.P)
        .count();

    let theta_estimate = case
        .groups
        .iter()
        .map(|group| group.T - group.S - 1)
        .sum::<i64>() as f64
        / case.groups.len() as f64;

    Ok(Analysis {
        theta_estimate,
        total_cell_time,
        max_active_demand,
        overflow_cell_time,
        clipped_cell_time,
        temporal_capacity_upper,
        partial_groups,
    })
}

fn input_paths(root: &Path, requested_case: Option<&str>) -> Result<Vec<PathBuf>, String> {
    let input_dir = root.join("tools/in");
    if let Some(case_name) = requested_case {
        let file_name = if case_name.ends_with(".txt") {
            case_name.to_string()
        } else {
            format!("{case_name}.txt")
        };
        return Ok(vec![input_dir.join(file_name)]);
    }
    let mut paths = fs::read_dir(&input_dir)
        .map_err(|error| format!("{} を読めない: {error}", input_dir.display()))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("入力一覧を読めない: {error}"))?;
    paths.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("txt"));
    paths.sort();
    Ok(paths)
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let requested_case = match args.as_slice() {
        [] => None,
        [flag, case_name] if flag == "--case" => Some(case_name.as_str()),
        _ => return Err("usage: calculate_temporal_upper_bounds [--case <case_name>]".to_string()),
    };
    let root = project_root()?;
    let paths = input_paths(&root, requested_case)?;
    println!(
        "case,R,grass,theta_estimate,total_cell_time,load,max_active_demand,peak_load,overflow_cell_time,clipped_cell_time,temporal_capacity_upper,partial_groups"
    );
    for path in paths {
        let case = parse_case(&path)?;
        let analysis = analyze(&case)?;
        let capacity = case.grass_count * HORIZON;
        println!(
            "{},{:.3},{},{:.3},{},{:.9},{},{:.9},{},{},{},{}",
            case.name,
            case.R,
            case.grass_count,
            analysis.theta_estimate,
            analysis.total_cell_time,
            analysis.total_cell_time as f64 / capacity as f64,
            analysis.max_active_demand,
            analysis.max_active_demand as f64 / case.grass_count as f64,
            analysis.overflow_cell_time,
            analysis.clipped_cell_time,
            analysis.temporal_capacity_upper.round() as i64,
            analysis.partial_groups,
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(S: i64, T: i64, P: i64, max_fee: i64) -> Group {
        Group { S, T, P, max_fee }
    }

    #[test]
    fn compactness_matches_small_shapes() {
        assert!((max_compactness(4) - 1.0).abs() < 1.0e-12);
        assert!((max_compactness(5) - 4.0 * 5.0_f64.sqrt() / 10.0).abs() < 1.0e-12);
        assert!((max_compactness(6) - 4.0 * 6.0_f64.sqrt() / 10.0).abs() < 1.0e-12);
    }

    #[test]
    fn temporal_capacity_selects_the_better_overlap() {
        let groups = vec![group(0, 10, 2, 20), group(0, 10, 2, 30)];
        let (objective, used, _) = solve_temporal_capacity_upper(&groups, 2).unwrap();
        assert!((objective - 30.0).abs() < 1.0e-9);
        assert_eq!(used, vec![0, 2]);
    }

    #[test]
    fn temporal_capacity_can_split_a_group_by_cells() {
        let groups = vec![group(0, 10, 3, 30), group(0, 10, 2, 30)];
        let (objective, used, _) = solve_temporal_capacity_upper(&groups, 4).unwrap();
        assert!((objective - 50.0).abs() < 1.0e-9);
        assert_eq!(used, vec![2, 2]);
    }

    #[test]
    fn non_overlapping_groups_share_capacity() {
        let groups = vec![group(0, 5, 2, 20), group(5, 10, 2, 30)];
        let (objective, used, _) = solve_temporal_capacity_upper(&groups, 2).unwrap();
        assert!((objective - 50.0).abs() < 1.0e-9);
        assert_eq!(used, vec![2, 2]);
    }

    #[test]
    fn temporal_capacity_matches_exhaustive_cell_assignment() {
        let groups = vec![
            group(0, 4, 2, 7),
            group(2, 6, 3, 11),
            group(4, 8, 2, 8),
            group(1, 7, 1, 5),
        ];
        let grass_count = 3;
        let (objective, _, times) = solve_temporal_capacity_upper(&groups, grass_count).unwrap();
        let mut exhaustive_best = 0.0_f64;
        for y0 in 0..=groups[0].P {
            for y1 in 0..=groups[1].P {
                for y2 in 0..=groups[2].P {
                    for y3 in 0..=groups[3].P {
                        let used = [y0, y1, y2, y3];
                        let feasible = (0..times.len() - 1).all(|segment| {
                            groups
                                .iter()
                                .zip(used)
                                .filter(|(group, _)| {
                                    group.S <= times[segment] && times[segment] < group.T
                                })
                                .map(|(_, value)| value)
                                .sum::<i64>()
                                <= grass_count
                        });
                        if feasible {
                            let value = groups
                                .iter()
                                .zip(used)
                                .map(|(group, used)| {
                                    used as f64 * group.max_fee as f64 / group.P as f64
                                })
                                .sum::<f64>();
                            exhaustive_best = exhaustive_best.max(value);
                        }
                    }
                }
            }
        }
        assert!((objective - exhaustive_best).abs() < 1.0e-9);
    }
}
