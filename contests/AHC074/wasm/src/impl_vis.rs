#![allow(non_snake_case)]
use std::cell::RefCell;
use tools::vis::{VisData, VisOption};

struct CachedCase {
    input_text: String,
    output_text: String,
    input: tools::Input,
    data: VisData,
    // 公式の VisData は手の一覧を公開しないため、移動の重ね描きに使う分をここで持つ。
    moves: Vec<SheetMove>,
}

/// 1 手の移動。from は移動前の順位、to は移動後の順位 (= 操作の d) である。
#[derive(Clone, Copy, Debug, PartialEq)]
struct SheetMove {
    k: usize,
    from: usize,
    to: usize,
}

fn collect_moves(input: &tools::Input, output: &tools::Output) -> Vec<SheetMove> {
    let mut state = tools::State::new(input);
    let mut moves = Vec::new();
    for a in &output.actions {
        if let tools::Action::Move(k, d) = a {
            let from = state.order.iter().position(|v| v == k).unwrap();
            moves.push(SheetMove { k: *k, from, to: *d });
        }
        // parse_output は検証済みの手だけを actions に入れるので、適用は失敗しない。
        state.apply(input, a).unwrap();
    }
    moves
}

thread_local! {
    // 再生中は同じ出力を繰り返し描くため、公式のチェックポイントを再利用する。
    static CACHE: RefCell<Option<CachedCase>> = const { RefCell::new(None) };
}

fn parse_input(text: &str) -> Result<tools::Input, String> {
    // 公式 parser は不正入力で panic する。編集中のテキストは先に検証し、WASM の停止を防ぐ。
    let mut tokens = text.split_whitespace();
    let mut sizes = [0; 4];
    for ((size, name), limit) in sizes
        .iter_mut()
        .zip(["N", "K", "P", "R"])
        .zip([64, 192, 256, 1024])
    {
        *size = tokens
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|v| (1..=limit).contains(v))
            .ok_or_else(|| format!("{name} は 1〜{limit} の整数で指定する"))?;
    }
    // 公式の固定制約に加え、問題文の小さなサンプルも読み込める。
    let [N, _, P, R] = sizes;
    for p in 0..P {
        let barcode = tokens
            .next()
            .ok_or_else(|| format!("A[{p}] が不足している"))?;
        if barcode.len() != N || !barcode.bytes().all(|b| matches!(b, b'0' | b'1')) {
            return Err(format!("A[{p}] は長さ N の 0/1 列で指定する"));
        }
    }
    for r in 0..R {
        tokens
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&a| a < P)
            .ok_or_else(|| format!("a[{r}] が不足、または 0〜P-1 の範囲外である"))?;
    }
    if tokens.next().is_some() {
        return Err("最後の注文の後に余分な入力がある".into());
    }
    Ok(tools::parse_input(text))
}

fn with_case<T>(input: &str, output: &str, f: impl FnOnce(&CachedCase) -> T) -> Result<T, String> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache
            .as_ref()
            .is_some_and(|c| c.input_text == input && c.output_text == output)
        {
            let parsed = parse_input(input)?;
            let parsed_output = tools::parse_output(&parsed, output)?;
            let moves = collect_moves(&parsed, &parsed_output);
            let data = VisData::new(&parsed, parsed_output);
            *cache = Some(CachedCase {
                input_text: input.to_owned(),
                output_text: output.to_owned(),
                input: parsed,
                data,
                moves,
            });
        }
        Ok(f(cache.as_ref().unwrap()))
    })
}

pub fn generate(seed: i32) -> String {
    tools::generate(seed.max(0) as u64, &tools::GenOption::default()).to_string()
}

pub fn calc_max_turn(input: &str, output: &str) -> usize {
    // get_max_turn の戻り値は整数だけなので、入力エラーの詳細は visualize で返す。
    with_case(input, output, |c| {
        tools::vis::get_max_turn(&c.input, &c.data, &VisOption::default())
    })
    .unwrap_or(0)
}

/// 各注文を処理し終えた時点の turn (= それまでの総手数) を、処理済みの注文の数だけ返す。
pub fn calc_order_ends(input: &str, output: &str) -> Vec<usize> {
    with_case(input, output, |c| {
        c.data.orders.iter().map(|o| o.start + o.moves).collect()
    })
    .unwrap_or_default()
}

pub fn visualize(
    input: &str,
    output: &str,
    turn: usize,
    move_start: usize,
) -> Result<(i64, String, String), String> {
    with_case(input, output, |c| {
        let option = VisOption { t: turn };
        // 出力をまだ読み込んでいないときは、未設計の初期画面だけを表示する。
        if output.trim().is_empty() {
            return (
                0,
                String::new(),
                tools::vis::draw_parse_error(&c.input, "", &option),
            );
        }
        let max = tools::vis::get_max_turn(&c.input, &c.data, &option);
        let (score, error) = if turn >= max {
            match c.data.verdict() {
                Ok(score) => (*score, String::new()),
                Err(error) => (0, error.clone()),
            }
        } else {
            let state = c.data.state_at(&c.input, turn);
            (
                tools::score(c.input.K, state.mistakes, state.moves),
                String::new(),
            )
        };
        let svg = tools::vis::vis(&c.input, &c.data, &option).svg;
        let state = c.data.state_at(&c.input, turn.min(max));
        let svg = add_visible_outline(svg, c.input.N, &state);
        // 公式 vis と同じく、turn t の画面は t 手目 (0-indexed で t-1) の直後を表す。
        // そのため重ね描きするのは c.moves[move_start..t] である。
        let t = turn.min(max);
        let moves = &c.moves[move_start.min(t)..t];
        (score, error, add_move_overlay(svg, c.input.K, moves))
    })
}

// 公式 vis.rs の Stack 描画と同じ配置定数。山は rows 行ずつ 3 列に折り返される。
const STACK_COL_X: i32 = 16;
const STACK_COL_STEP: i32 = 340;
const STACK_ROW_Y: i32 = 334;
const STACK_ROW_STEP: i32 = 8;
const STACK_BAR_W: i32 = 320;
const STACK_BAR_H: f64 = 6.;
// 公式で「見えている紙」の印 (左端の小さな棒) に使われている色にそろえる。
const VISIBLE_COLOR: &str = "#e59132";
const MOVE_UP_COLOR: &str = "#287de0";
const MOVE_DOWN_COLOR: &str = "#8e44c9";

fn stack_pos(K: usize, rank: usize) -> (i32, i32, usize) {
    let rows = K.div_ceil(3).max(1);
    let col = rank / rows;
    (
        STACK_COL_X + col as i32 * STACK_COL_STEP,
        STACK_ROW_Y + (rank % rows) as i32 * STACK_ROW_STEP,
        col,
    )
}

fn insert_before_end(svg: String, overlay: &str) -> String {
    match svg.rfind("</svg>") {
        Some(end) => svg[..end].to_owned() + overlay + "</svg>",
        None => svg,
    }
}

/// 山の中で、窓から実際に見えているマスを枠で囲む。
/// 公式の色分けでは隠れた白と見えている白がほぼ同じ色で、区別できないため。
/// 枠はマス単位ではなく連続区間ごとに描き、細い棒の中でも読めるようにする。
fn add_visible_outline(svg: String, N: usize, state: &tools::State) -> String {
    // 設計の途中で出力が不正になった場合は、公式と同じく紙の描画自体がないので枠も付けない。
    if state.plates.len() < state.order.len() {
        return svg;
    }
    let (_, exposed) = state.display();
    let K = state.order.len();
    let cell = STACK_BAR_W as f64 / N as f64;
    let mut path = String::new();
    for (rank, &k) in state.order.iter().enumerate() {
        let bits = exposed[k];
        if bits == 0 {
            continue;
        }
        let (x, y, _) = stack_pos(K, rank);
        let mut j = 0;
        while j < N {
            if bits >> j & 1 == 0 {
                j += 1;
                continue;
            }
            let start = j;
            while j < N && bits >> j & 1 != 0 {
                j += 1;
            }
            let x0 = x as f64 + start as f64 * cell;
            let w = (j - start) as f64 * cell;
            let y0 = y as f64 - STACK_BAR_H / 2. - 0.5;
            path += &format!("M{x0} {y0}h{w}v{}h{}z", STACK_BAR_H + 1., -w);
        }
    }
    if path.is_empty() {
        return svg;
    }
    let overlay = format!(
        r#"<path id="visible-outline" d="{path}" fill="none" stroke="{VISIBLE_COLOR}" stroke-width="1.2" pointer-events="none"/>"#
    );
    insert_before_end(svg, &overlay)
}

fn move_color(m: &SheetMove) -> &'static str {
    if m.to < m.from {
        MOVE_UP_COLOR
    } else {
        MOVE_DOWN_COLOR
    }
}

/// 移動元の順位から移動先の順位への矢印の (path, 矢先の x, 矢先の y) を返す。
fn move_path(K: usize, m: &SheetMove) -> (String, i32, i32) {
    let (sx, sy, scol) = stack_pos(K, m.from);
    let (ex, ey, ecol) = stack_pos(K, m.to);
    // 同じ列なら棒の右端どうしを右へ膨らむ弧で結ぶ。列をまたぐときは、
    // 進む向きの側の端から出て、相手の列の手前側の端へ入る。
    let (x1, x2, c1, c2) = if scol == ecol {
        let right = sx + STACK_BAR_W + 3;
        let bulge = 14 + (sy - ey).abs() / 4;
        (right, right, right + bulge, right + bulge)
    } else if scol < ecol {
        let (a, b) = (sx + STACK_BAR_W + 3, ex - 5);
        let h = ((b - a) / 2).max(20);
        (a, b, a + h, b - h)
    } else {
        let (a, b) = (sx - 3, ex + STACK_BAR_W + 5);
        let h = ((a - b) / 2).max(20);
        (a, b, a - h, b + h)
    };
    (format!("M{x1} {sy}C{c1} {sy} {c2} {ey} {x2} {ey}"), x2, ey)
}

/// 公式の画面に、moves の各手の移動元・移動先・矢印を重ねる。
/// 上げる手 (to < from) と下げる手 (to > from) を色で分ける。
/// 複数手を重ねるときは、各手の from / to はその手を打った時点の順位なので、
/// 今の山と位置がずれる。どの手かを読めるよう、矢先に注文内の手番号を付ける。
fn add_move_overlay(svg: String, K: usize, moves: &[SheetMove]) -> String {
    let Some(last) = moves.last() else {
        return svg;
    };
    let numbered = moves.len() > 1;
    let mut body = String::new();
    // 移動元: 今は別の紙がいる位置なので、破線の枠だけを残す。
    for m in moves {
        let (sx, sy, _) = stack_pos(K, m.from);
        body += &format!(
            r#"<rect x="{}" y="{}" width="322" height="8" fill="none" stroke="{}" stroke-width="1.5" stroke-dasharray="3 2"/>"#,
            sx - 1,
            sy - 4,
            move_color(m),
        );
    }
    // 移動先: 公式の青枠を、向きの色の太い枠で上書きする。
    for m in moves {
        let (ex, ey, _) = stack_pos(K, m.to);
        let color = move_color(m);
        body += &format!(
            r#"<rect x="{}" y="{}" width="322" height="8" fill="{color}" fill-opacity="0.12" stroke="{color}" stroke-width="2"/>"#,
            ex - 1,
            ey - 4,
        );
    }
    for (i, m) in moves.iter().enumerate() {
        let color = move_color(m);
        let marker = if m.to < m.from { "move-arrow-up" } else { "move-arrow-down" };
        let (path, tx, ty) = move_path(K, m);
        body += &format!(
            concat!(
                r#"<path d="{path}" fill="none" stroke="white" stroke-opacity="0.85" stroke-width="5"/>"#,
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="2" marker-end="url(#{marker})"/>"#,
            ),
            path = path,
            color = color,
            marker = marker,
        );
        if numbered {
            body += &format!(
                r#"<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{color}" stroke="white" stroke-width="2.5" paint-order="stroke">{}</text>"#,
                tx + 6,
                ty - 3,
                i + 1,
            );
        }
    }
    let label = if numbered {
        let up = moves.iter().filter(|m| m.to < m.from).count();
        format!("{} 手 (↑{} ↓{})", moves.len(), up, moves.len() - up)
    } else {
        let up = last.to < last.from;
        format!(
            "{} {} {}段",
            if up { "↑" } else { "↓" },
            if up { "上げ" } else { "下げ" },
            last.from.abs_diff(last.to),
        )
    };
    let label_color = if numbered { "#263241" } else { move_color(last) };
    let overlay = format!(
        concat!(
            r#"<g id="move-overlay" pointer-events="none">"#,
            r#"<defs>"#,
            r#"<marker id="move-arrow-up" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="{up}"/></marker>"#,
            r#"<marker id="move-arrow-down" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="{down}"/></marker>"#,
            r#"</defs>{body}"#,
            r#"<text x="770" y="302" font-size="15" font-weight="bold" fill="{label_color}">{label}</text>"#,
            "</g>"
        ),
        up = MOVE_UP_COLOR,
        down = MOVE_DOWN_COLOR,
        body = body,
        label_color = label_color,
        label = label,
    );
    // 公式の「Paper k: rank a → b」の文字は常に青なので、最後の手の向きの色にそろえる。
    // この色は公式 vis.rs でこの文字にしか使われていない。
    let svg = svg.replace(r##"fill="#216bc0""##, &format!(r#"fill="{}""#, move_color(last)));
    insert_before_end(svg, &overlay)
}

#[cfg(test)]
mod tests {
    use super::*;
    const INPUT: &str = "4 3 4 3\n1001\n1100\n0011\n0110\n0\n1\n2\n";
    const OUTPUT: &str = "10.1\n1100\n0.1.\n0\n1\n0 2\n1\n1 2\n";

    #[test]
    fn sample_frames_match_official_renderer() {
        let input = tools::parse_input(INPUT);
        let data = VisData::new(&input, tools::parse_output(&input, OUTPUT).unwrap());
        assert_eq!(calc_max_turn(INPUT, OUTPUT), 2);
        for turn in 0..=2 {
            let (score, error, svg) = visualize(INPUT, OUTPUT, turn, turn.saturating_sub(1)).unwrap();
            assert_eq!(score, turn as i64);
            assert!(error.is_empty());
            // 公式の描画はそのまま残し、移動の重ね描きは末尾に足すだけにする。
            let official = tools::vis::vis(&input, &data, &VisOption { t: turn }).svg;
            let expected = if turn > 0 {
                official.replace("#216bc0", MOVE_DOWN_COLOR)
            } else {
                official
            };
            assert!(svg.starts_with(expected.strip_suffix("</svg>").unwrap()));
            assert!(svg.ends_with("</svg>"));
            // このサンプルではどの時点でも見えているマスがある。
            assert!(svg.contains(r#"id="visible-outline""#));
            if turn > 0 {
                assert!(svg.contains(r#"id="move-overlay""#));
                assert!(svg.contains("↓ 下げ 2段"));
            }
        }
        assert_eq!(
            visualize(INPUT, OUTPUT, 99, 1).unwrap(),
            visualize(INPUT, OUTPUT, 2, 1).unwrap()
        );
    }

    #[test]
    fn overlay_colors_follow_move_direction() {
        let up = add_move_overlay("<svg></svg>".into(), 192, &[SheetMove { k: 5, from: 100, to: 0 }]);
        assert!(up.contains("↑ 上げ 100段"));
        let down = add_move_overlay("<svg></svg>".into(), 192, &[SheetMove { k: 5, from: 3, to: 150 }]);
        assert!(down.contains("↓ 下げ 147段"));
        assert_eq!(add_move_overlay("<svg></svg>".into(), 192, &[]), "<svg></svg>");
    }

    #[test]
    fn order_frames_overlay_all_moves_of_the_order() {
        // サンプルは注文 0 が 0 手、注文 1 と 2 が 1 手ずつ。
        assert_eq!(calc_order_ends(INPUT, OUTPUT), vec![0, 1, 2]);
        let two = "10.1\n1100\n0.1.\n0\n2\n1 2\n2 0\n0\n";
        assert_eq!(calc_order_ends(INPUT, two), vec![0, 2, 2]);
        let svg = visualize(INPUT, two, 2, 0).unwrap().2;
        assert!(svg.contains("2 手 (↑1 ↓1)"));
        assert_eq!(svg.matches(r#"marker-end="#).count(), 2);
        // 0 手の注文では重ね描きしない。
        assert!(!visualize(INPUT, two, 2, 2).unwrap().2.contains("move-overlay"));
    }

    #[test]
    fn zero_moves_and_invalid_output_preserve_official_verdict() {
        let output = "10.1\n1100\n0.1.\n0\n0\n0\n";
        assert_eq!(calc_max_turn(INPUT, output), 0);
        assert_eq!(visualize(INPUT, output, 0, 0).unwrap().0, 12);
        let invalid = "10.1\n1100\n0.1.\n1\n2 0\n";
        assert!(visualize(INPUT, invalid, 0, 0)
            .unwrap()
            .1
            .contains("not visible"));
        let extra = format!("{OUTPUT}0\n");
        assert!(visualize(INPUT, &extra, 2, 1)
            .unwrap()
            .1
            .contains("Extra output"));
    }

    #[test]
    fn editable_input_errors_do_not_panic_or_poison_cache() {
        assert!(visualize(INPUT, "", 0, 0).unwrap().1.is_empty());
        for invalid in [
            "",
            "0 3 4 3",
            "65 3 4 3",
            "4 3 4 0",
            "4 3 4 3\n1001",
            "4 1 1 1\n1001\n1",
        ] {
            assert_eq!(calc_max_turn(invalid, OUTPUT), 0);
            assert!(visualize(invalid, OUTPUT, 0, 0).is_err());
        }
        assert_eq!(visualize(INPUT, OUTPUT, 2, 1).unwrap().0, 2);
    }

    #[test]
    fn generated_seed_zero_matches_official_input() {
        assert_eq!(generate(0), include_str!("../../tools/in/0000.txt"));
    }
}
