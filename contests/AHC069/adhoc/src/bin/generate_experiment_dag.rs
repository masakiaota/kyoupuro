// generate_experiment_dag.rs

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const JOURNAL_PATH: &str = "notes/journal.md";
const BACKLOG_PATH: &str = "notes/backlog.md";
const DAG_PATH: &str = "notes/experiment_dag.md";

const STATUSES: [&str; 5] = [
    "現行採用",
    "後続への統合",
    "知見のみ有効",
    "条件付き再検討",
    "未決着",
];
const SERIES: [&str; 4] = ["foundation", "no_move", "current", "auxiliary"];
const VERDICTS: [&str; 4] = ["未判定", "採用", "棄却", "中断"];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Experiment {
    name: String,
    status: String,
    summary: String,
    series: String,
    base: Option<String>,
    imports: Vec<String>,
    verdict: String,
    field_counts: BTreeMap<String, usize>,
    non_empty_body_lines: usize,
}

#[derive(Debug)]
struct EntryBuilder {
    name: String,
    status: String,
    summary: String,
    lineage: Option<(String, Option<String>, Vec<String>)>,
    verdict: Option<String>,
    field_counts: BTreeMap<String, usize>,
    non_empty_body_lines: usize,
}

impl EntryBuilder {
    fn finish(self) -> Result<Experiment, String> {
        let (series, base, imports) = self
            .lineage
            .ok_or_else(|| format!("{} に系譜がない", self.name))?;
        let verdict = self
            .verdict
            .ok_or_else(|| format!("{} に当時の判定がない", self.name))?;
        Ok(Experiment {
            name: self.name,
            status: self.status,
            summary: self.summary,
            series,
            base,
            imports,
            verdict,
            field_counts: self.field_counts,
            non_empty_body_lines: self.non_empty_body_lines,
        })
    }
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "adhoc の親ディレクトリを取得できない".to_string())
}

fn parse_heading(line: &str) -> Result<Option<(String, String, String)>, String> {
    let Some(rest) = line.strip_prefix("## ") else {
        return Ok(None);
    };
    let Some((name, tail)) = rest.split_once(" — ") else {
        return Ok(None);
    };
    let Some((status, summary)) = tail.split_once(": ") else {
        return Err(format!("実験見出しの形式が不正: {line}"));
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(format!("実験名の形式が不正: {name}"));
    }
    if summary.is_empty() {
        return Err(format!("現在の位置づけが空: {name}"));
    }
    Ok(Some((
        name.to_string(),
        status.to_string(),
        summary.to_string(),
    )))
}

fn parse_lineage(line: &str) -> Result<(String, Option<String>, Vec<String>), String> {
    let rest = line
        .strip_prefix("系譜: ")
        .ok_or_else(|| format!("系譜の形式が不正: {line}"))?;
    let parts: Vec<_> = rest.split("; ").collect();
    if parts.len() != 3 {
        return Err(format!(
            "系譜は series, base, imports の3項目とする: {line}"
        ));
    }
    let series = parts[0]
        .strip_prefix("series=")
        .ok_or_else(|| format!("series がない: {line}"))?
        .to_string();
    let base_text = parts[1]
        .strip_prefix("base=")
        .ok_or_else(|| format!("base がない: {line}"))?;
    let base = if base_text == "-" {
        None
    } else if base_text.is_empty() {
        return Err(format!("base が空: {line}"));
    } else {
        Some(base_text.to_string())
    };
    let imports_text = parts[2]
        .strip_prefix("imports=[")
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("imports の形式が不正: {line}"))?;
    let imports = if imports_text.is_empty() {
        Vec::new()
    } else {
        imports_text.split(", ").map(str::to_string).collect()
    };
    Ok((series, base, imports))
}

fn parse_journal(input: &str) -> Result<Vec<Experiment>, String> {
    let mut experiments = Vec::new();
    let mut current: Option<EntryBuilder> = None;
    let required_fields = ["仮説", "変更", "機構確認", "採否基準", "結果", "学び"];

    for line in input.lines() {
        if let Some((name, status, summary)) = parse_heading(line)? {
            if let Some(builder) = current.take() {
                experiments.push(builder.finish()?);
            }
            current = Some(EntryBuilder {
                name,
                status,
                summary,
                lineage: None,
                verdict: None,
                field_counts: BTreeMap::new(),
                non_empty_body_lines: 0,
            });
            continue;
        }

        let Some(builder) = current.as_mut() else {
            continue;
        };
        if line.is_empty() {
            continue;
        }
        builder.non_empty_body_lines += 1;
        if line.starts_with("系譜: ") {
            if builder.lineage.is_some() {
                return Err(format!("{} に系譜が複数ある", builder.name));
            }
            builder.lineage = Some(parse_lineage(line)?);
            continue;
        }
        if let Some(value) = line.strip_prefix("当時の判定: ") {
            if builder.verdict.is_some() {
                return Err(format!("{} に当時の判定が複数ある", builder.name));
            }
            let verdict = value
                .strip_suffix('。')
                .ok_or_else(|| format!("当時の判定は句点で終える: {line}"))?;
            builder.verdict = Some(verdict.to_string());
            continue;
        }
        for field in required_fields {
            if line.starts_with(&format!("{field}:")) {
                *builder.field_counts.entry(field.to_string()).or_default() += 1;
                break;
            }
        }
    }
    if let Some(builder) = current {
        experiments.push(builder.finish()?);
    }
    if experiments.is_empty() {
        return Err("journal に実験エントリがない".to_string());
    }
    Ok(experiments)
}

#[derive(Debug)]
struct Backlog {
    states: HashMap<String, String>,
    ids: Vec<String>,
}

fn parse_backlog(input: &str) -> Result<Backlog, String> {
    let mut states = HashMap::new();
    let mut ids = Vec::new();
    for line in input.lines() {
        if let Some(rest) = line.strip_prefix("- **[")
            && let Some((id, _)) = rest.split_once(']')
        {
            if id.starts_with("B-") {
                ids.push(id.to_string());
            }
        }
        let Some((_, rhs)) = line.split_once("→ ") else {
            continue;
        };
        if rhs.starts_with("取り下げ:") {
            continue;
        }
        let Some((name, rest)) = rhs.split_once(' ') else {
            return Err(format!("backlog の実験参照形式が不正: {line}"));
        };
        let Some((status, _)) = rest.split_once(':') else {
            return Err(format!("backlog の現在状態がない: {line}"));
        };
        if states
            .insert(name.to_string(), status.to_string())
            .is_some()
        {
            return Err(format!("backlog に実験参照が重複: {name}"));
        }
    }
    Ok(Backlog { states, ids })
}

fn validate_fields(experiment: &Experiment) -> Result<(), String> {
    for field in ["仮説", "変更", "機構確認", "採否基準", "結果", "学び"] {
        if experiment.field_counts.get(field).copied().unwrap_or(0) != 1 {
            return Err(format!("{} の {field} は1行必要", experiment.name));
        }
    }
    if experiment.non_empty_body_lines != 8 {
        return Err(format!("{} は見出しの後を8行とする", experiment.name));
    }
    if experiment.status == "未決着" {
        if experiment.verdict != "未判定" {
            return Err(format!(
                "未決着の {} は当時の判定を未判定とする",
                experiment.name
            ));
        }
    } else {
        if experiment.verdict == "未判定" {
            return Err(format!("決着済みの {} は未判定にできない", experiment.name));
        }
    }
    Ok(())
}

fn validate_cycle(
    experiments: &[Experiment],
    indices: &HashMap<String, usize>,
) -> Result<(), String> {
    fn visit(
        index: usize,
        experiments: &[Experiment],
        indices: &HashMap<String, usize>,
        colors: &mut [u8],
        stack: &mut Vec<String>,
    ) -> Result<(), String> {
        if colors[index] == 2 {
            return Ok(());
        }
        if colors[index] == 1 {
            stack.push(experiments[index].name.clone());
            return Err(format!("実験DAGに循環がある: {}", stack.join(" -> ")));
        }
        colors[index] = 1;
        stack.push(experiments[index].name.clone());
        let mut dependencies = Vec::new();
        if let Some(base) = &experiments[index].base {
            dependencies.push(base);
        }
        dependencies.extend(&experiments[index].imports);
        for dependency in dependencies {
            visit(indices[dependency], experiments, indices, colors, stack)?;
        }
        stack.pop();
        colors[index] = 2;
        Ok(())
    }

    let mut colors = vec![0_u8; experiments.len()];
    for index in 0..experiments.len() {
        if colors[index] == 0 {
            visit(index, experiments, indices, &mut colors, &mut Vec::new())?;
        }
    }
    Ok(())
}

fn validate_integrated_reachability(
    experiments: &[Experiment],
    indices: &HashMap<String, usize>,
    current_solver: &str,
) -> Result<(), String> {
    let mut children = vec![Vec::new(); experiments.len()];
    for (child, experiment) in experiments.iter().enumerate() {
        if let Some(base) = &experiment.base {
            children[indices[base]].push(child);
        }
        for imported in &experiment.imports {
            children[indices[imported]].push(child);
        }
    }
    let target = indices[current_solver];
    for (start, experiment) in experiments.iter().enumerate() {
        if experiment.status != "後続への統合" {
            continue;
        }
        let mut seen = vec![false; experiments.len()];
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(index) = stack.pop() {
            for &child in &children[index] {
                if !seen[child] {
                    seen[child] = true;
                    stack.push(child);
                }
            }
        }
        if !seen[target] {
            return Err(format!(
                "{} は後続への統合だが現行solverへ至る系譜がない",
                experiment.name
            ));
        }
    }
    Ok(())
}

fn validate(experiments: &[Experiment], backlog: &Backlog) -> Result<(), String> {
    let mut indices = HashMap::new();
    for (index, experiment) in experiments.iter().enumerate() {
        if indices.insert(experiment.name.clone(), index).is_some() {
            return Err(format!("journal に実験名が重複: {}", experiment.name));
        }
        if !STATUSES.contains(&experiment.status.as_str()) {
            return Err(format!(
                "{} の現在状態が未定義: {}",
                experiment.name, experiment.status
            ));
        }
        if !SERIES.contains(&experiment.series.as_str()) {
            return Err(format!(
                "{} のseriesが未定義: {}",
                experiment.name, experiment.series
            ));
        }
        if !VERDICTS.contains(&experiment.verdict.as_str()) {
            return Err(format!(
                "{} の当時の判定が未定義: {}",
                experiment.name, experiment.verdict
            ));
        }
        validate_fields(experiment)?;
    }

    let mut ids = HashSet::new();
    for id in &backlog.ids {
        if !ids.insert(id) {
            return Err(format!("backlog IDが重複: {id}"));
        }
    }

    for experiment in experiments {
        if let Some(base) = &experiment.base {
            if base == &experiment.name {
                return Err(format!("{} が自身をbaseにしている", experiment.name));
            }
            if !indices.contains_key(base) {
                return Err(format!("{} のbase参照が不明: {base}", experiment.name));
            }
        }
        let mut imports = HashSet::new();
        for imported in &experiment.imports {
            if imported == &experiment.name {
                return Err(format!("{} が自身をimportsに含めている", experiment.name));
            }
            if !indices.contains_key(imported) {
                return Err(format!(
                    "{} のimports参照が不明: {imported}",
                    experiment.name
                ));
            }
            if !imports.insert(imported) {
                return Err(format!("{} のimportsが重複: {imported}", experiment.name));
            }
        }
        match backlog.states.get(&experiment.name) {
            Some(status) if status == &experiment.status => {}
            Some(status) => {
                return Err(format!(
                    "{} の現在状態がjournal={}、backlog={}で不一致",
                    experiment.name, experiment.status, status
                ));
            }
            None => return Err(format!("{} がbacklogにない", experiment.name)),
        }
    }
    for name in backlog.states.keys() {
        if !indices.contains_key(name) {
            return Err(format!("backlogの{name}がjournalにない"));
        }
    }

    validate_cycle(experiments, &indices)?;
    let current_solvers: Vec<_> = experiments
        .iter()
        .filter(|experiment| experiment.status == "現行採用" && experiment.series != "auxiliary")
        .collect();
    if current_solvers.len() != 1 {
        return Err(format!(
            "非補助系列の現行solverは1件必要だが{}件ある",
            current_solvers.len()
        ));
    }
    validate_integrated_reachability(experiments, &indices, &current_solvers[0].name)?;
    Ok(())
}

fn mermaid_class(status: &str) -> &'static str {
    match status {
        "現行採用" => "current",
        "後続への統合" => "integrated",
        "知見のみ有効" => "knowledge",
        "条件付き再検討" => "conditional",
        "未決着" => "unresolved",
        _ => unreachable!(),
    }
}

fn push_mermaid_styles(output: &mut String, nodes: &[&Experiment], external: &BTreeSet<String>) {
    output.push_str("  classDef current fill:#dcfce7,stroke:#15803d,stroke-width:3px;\n");
    output.push_str("  classDef integrated fill:#dbeafe,stroke:#2563eb,stroke-width:2px;\n");
    output.push_str("  classDef knowledge fill:#f3f4f6,stroke:#6b7280;\n");
    output.push_str("  classDef conditional fill:#fef3c7,stroke:#d97706,stroke-width:2px;\n");
    output.push_str("  classDef unresolved fill:#ffedd5,stroke:#ea580c,stroke-width:2px;\n");
    output.push_str("  classDef external fill:#ffffff,stroke:#9ca3af,stroke-dasharray: 4 3;\n");
    for status in STATUSES {
        let names: Vec<_> = nodes
            .iter()
            .filter(|experiment| experiment.status == status)
            .map(|experiment| experiment.name.as_str())
            .collect();
        if !names.is_empty() {
            output.push_str(&format!(
                "  class {} {};\n",
                names.join(","),
                mermaid_class(status)
            ));
        }
    }
    if !external.is_empty() {
        output.push_str(&format!(
            "  class {} external;\n",
            external.iter().cloned().collect::<Vec<_>>().join(",")
        ));
    }
}

fn render_mermaid(
    experiments: &[Experiment],
    included: &HashSet<String>,
    show_external: bool,
) -> String {
    let by_name: HashMap<_, _> = experiments
        .iter()
        .map(|experiment| (experiment.name.as_str(), experiment))
        .collect();
    let nodes: Vec<_> = experiments
        .iter()
        .filter(|experiment| included.contains(&experiment.name))
        .collect();
    let mut external = BTreeSet::new();
    if show_external {
        for experiment in &nodes {
            if let Some(base) = &experiment.base
                && !included.contains(base)
            {
                external.insert(base.clone());
            }
            for imported in &experiment.imports {
                if !included.contains(imported) {
                    external.insert(imported.clone());
                }
            }
        }
    }

    let mut output = String::from("```mermaid\nflowchart LR\n");
    for experiment in &nodes {
        output.push_str(&format!(
            "  {}[\"{}<br/>{}\"]\n",
            experiment.name, experiment.name, experiment.status
        ));
    }
    for name in &external {
        let experiment = by_name[name.as_str()];
        output.push_str(&format!(
            "  {}[\"{}<br/>別系列参照\"]\n",
            experiment.name, experiment.name
        ));
    }
    for experiment in &nodes {
        if let Some(base) = &experiment.base
            && (included.contains(base) || external.contains(base))
        {
            output.push_str(&format!("  {base} -->|base| {}\n", experiment.name));
        }
        for imported in &experiment.imports {
            if included.contains(imported) || external.contains(imported) {
                output.push_str(&format!("  {imported} -.->|imports| {}\n", experiment.name));
            }
        }
    }
    push_mermaid_styles(&mut output, &nodes, &external);
    output.push_str("```\n");
    output
}

fn render(experiments: &[Experiment]) -> String {
    let mut counts = BTreeMap::new();
    for status in STATUSES {
        counts.insert(status, 0_usize);
    }
    for experiment in experiments {
        *counts.get_mut(experiment.status.as_str()).unwrap() += 1;
    }

    let mut output = String::new();
    output.push_str("# 実験DAG\n\n");
    output.push_str("このファイルは `notes/journal.md` から自動生成する。\n");
    output.push_str(
        "手作業では編集せず、`generate_experiment_dag` の `--write` モードで更新する。\n\n",
    );
    output.push_str("solid矢印は実装上の基盤となる `base`、破線矢印は別実験から中心機構を取り込む `imports` を表す。\n");
    output.push_str("ノードの色は現在状態を表し、評価時点の採否は `notes/journal.md` の「当時の判定」で確認する。\n\n");
    output.push_str("## 現行系統\n\n");
    output.push_str("現行採用または後続への統合に分類した実験だけを表示する。\n\n");
    let current_lineage: HashSet<_> = experiments
        .iter()
        .filter(|experiment| experiment.status == "現行採用" || experiment.status == "後続への統合")
        .map(|experiment| experiment.name.clone())
        .collect();
    output.push_str(&render_mermaid(experiments, &current_lineage, false));

    output.push_str("\n## 現在状態の件数\n\n");
    output.push_str("| 現在状態 | 件数 |\n|---|---:|\n");
    for status in STATUSES {
        output.push_str(&format!("| {status} | {} |\n", counts[status]));
    }

    let sections = [
        ("foundation", "初期および主力形成系列"),
        ("no_move", "再移動なし系列"),
        ("current", "現行主力から派生した系列"),
        ("auxiliary", "補助検証系列"),
    ];
    for (series, title) in sections {
        output.push_str(&format!("\n## {title}\n\n"));
        let included: HashSet<_> = experiments
            .iter()
            .filter(|experiment| experiment.series == series)
            .map(|experiment| experiment.name.clone())
            .collect();
        output.push_str(&render_mermaid(experiments, &included, true));
    }

    output.push_str("\n## 実験一覧\n\n");
    output.push_str("| 実験 | 現在状態 | series | base | imports |\n");
    output.push_str("|---|---|---|---|---|\n");
    for experiment in experiments {
        let base = experiment.base.as_deref().unwrap_or("-");
        let imports = if experiment.imports.is_empty() {
            "-".to_string()
        } else {
            experiment.imports.join(", ")
        };
        output.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | {} |\n",
            experiment.name, experiment.status, experiment.series, base, imports
        ));
    }
    output
}

fn load_and_validate(root: &Path) -> Result<Vec<Experiment>, String> {
    let journal = fs::read_to_string(root.join(JOURNAL_PATH))
        .map_err(|error| format!("{JOURNAL_PATH} を読めない: {error}"))?;
    let backlog = fs::read_to_string(root.join(BACKLOG_PATH))
        .map_err(|error| format!("{BACKLOG_PATH} を読めない: {error}"))?;
    let experiments = parse_journal(&journal)?;
    let backlog = parse_backlog(&backlog)?;
    validate(&experiments, &backlog)?;
    Ok(experiments)
}

fn run() -> Result<(), String> {
    let root = project_root()?;
    let experiments = load_and_validate(&root)?;
    let generated = render(&experiments);
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            print!("{generated}");
            Ok(())
        }
        [flag] if flag == "--write" => fs::write(root.join(DAG_PATH), generated)
            .map_err(|error| format!("{DAG_PATH} を書けない: {error}")),
        [flag] if flag == "--check" => {
            let actual = fs::read_to_string(root.join(DAG_PATH))
                .map_err(|error| format!("{DAG_PATH} を読めない: {error}"))?;
            if actual != generated {
                return Err(format!("{DAG_PATH} が古い。--write で再生成する必要がある"));
            }
            Ok(())
        }
        _ => Err("引数は省略、--write、--check のいずれかとする".to_string()),
    }
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

    fn entry(name: &str, status: &str, series: &str, base: &str, imports: &str) -> String {
        let verdict = if status == "未決着" {
            "未判定"
        } else {
            "採用"
        };
        let mut result = format!(
            "## {name} — {status}: test\n\n系譜: series={series}; base={base}; imports=[{imports}]\n当時の判定: {verdict}。\n仮説: x\n変更: x\n機構確認: x\n採否基準: x\n"
        );
        let (result_text, learning_text) = if status == "未決着" {
            ("未評価。", "未確定。")
        } else {
            ("x", "x")
        };
        result.push_str(&format!("結果: {result_text}\n学び: {learning_text}\n"));
        result
    }

    fn backlog(entries: &[(&str, &str, &str)]) -> String {
        entries
            .iter()
            .map(|(id, name, status)| format!("- **[{id}] x** → {name} {status}: x\n"))
            .collect()
    }

    #[test]
    fn valid_graph_passes() {
        let journal = format!(
            "{}\n{}",
            entry("v001_base", "後続への統合", "foundation", "-", ""),
            entry("v002_current", "現行採用", "current", "v001_base", "")
        );
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[
            ("B-001", "v001_base", "後続への統合"),
            ("B-002", "v002_current", "現行採用"),
        ]))
        .unwrap();
        validate(&experiments, &backlog).unwrap();
    }

    #[test]
    fn rejects_unknown_status() {
        let journal = entry("v001_current", "採用", "current", "-", "");
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[("B-001", "v001_current", "採用")])).unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("現在状態が未定義")
        );
    }

    #[test]
    fn rejects_unknown_series() {
        let journal = entry("v001_current", "現行採用", "unknown", "-", "");
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[("B-001", "v001_current", "現行採用")])).unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("seriesが未定義")
        );
    }

    #[test]
    fn rejects_unknown_reference() {
        let journal = entry("v001_current", "現行採用", "current", "v999_missing", "");
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[("B-001", "v001_current", "現行採用")])).unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("base参照が不明")
        );
    }

    #[test]
    fn rejects_cycle() {
        let journal = format!(
            "{}\n{}",
            entry("v001_a", "知見のみ有効", "foundation", "v002_current", ""),
            entry("v002_current", "現行採用", "current", "v001_a", "")
        );
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[
            ("B-001", "v001_a", "知見のみ有効"),
            ("B-002", "v002_current", "現行採用"),
        ]))
        .unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("循環がある")
        );
    }

    #[test]
    fn rejects_duplicate_experiment() {
        let journal = format!(
            "{}\n{}",
            entry("v001_current", "現行採用", "current", "-", ""),
            entry("v001_current", "現行採用", "current", "-", "")
        );
        let experiments = parse_journal(&journal).unwrap();
        let backlog = parse_backlog(&backlog(&[("B-001", "v001_current", "現行採用")])).unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("実験名が重複")
        );
    }

    #[test]
    fn rejects_duplicate_backlog_id() {
        let journal = entry("v001_current", "現行採用", "current", "-", "");
        let experiments = parse_journal(&journal).unwrap();
        let source =
            format!("- **[B-001] x** → v001_current 現行採用: x\n- **[B-001] y**: 未着手\n");
        let backlog = parse_backlog(&source).unwrap();
        assert!(
            validate(&experiments, &backlog)
                .unwrap_err()
                .contains("backlog IDが重複")
        );
    }
}
