use serde_json::{json, Value};
use tools::{Input, Operation, State};
use wasm_bindgen::prelude::*;

fn error(code: &str, message: impl AsRef<str>) -> String {
    json!({"code": code, "message": message.as_ref()}).to_string()
}

fn rule_error(message: String) -> String {
    let (code, ja) = match message.as_str() {
        "Source is empty or k is not below height" => (
            "invalid_source",
            "出発点が空、または k が塔の高さ以上である",
        ),
        "Jump length exceeds k + 1" => ("jump_too_far", "飛距離 l は k + 1 以下で指定する"),
        "Jump leaves board" => ("outside_board", "ジャンプが盤面の外に出る"),
        "Jump crosses a wall" => ("wall_on_path", "ジャンプの途中または着地点に壁がある"),
        "Landing height exceeds 8 before receiving" => {
            ("height_limit_exceeded", "帰巣前の着地直後の高さが8を超える")
        }
        _ => ("invalid_action", message.as_str()),
    };
    error(code, ja)
}

fn action_json(input: &Input, op: &Operation) -> Value {
    json!({"i": op.p / input.n, "j": op.p % input.n, "k": op.k,
        "d": (["U", "D", "L", "R"][op.d]), "l": op.l})
}

fn parse_action(input: &Input, text: &str) -> Result<Operation, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| error("invalid_action", "操作はJSONオブジェクトで指定する"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| error("invalid_action", "操作はJSONオブジェクトで指定する"))?;
    if obj.len() != 5
        || obj
            .keys()
            .any(|key| !["i", "j", "k", "d", "l"].contains(&key.as_str()))
    {
        return Err(error(
            "invalid_action",
            "操作には i, j, k, d, l の5項目を指定する",
        ));
    }
    let number = |key: &str, max: usize| -> Result<usize, String> {
        obj.get(key)
            .and_then(Value::as_u64)
            .filter(|v| *v <= max as u64)
            .map(|v| v as usize)
            .ok_or_else(|| {
                error(
                    "invalid_action",
                    format!("{key} は 0〜{max} の整数で指定する"),
                )
            })
    };
    let i = number("i", input.n - 1)?;
    let j = number("j", input.n - 1)?;
    let k = number("k", 7)?;
    let l = number("l", 8)?;
    if l == 0 {
        return Err(error("invalid_action", "飛距離 l は1以上で指定する"));
    }
    let d = match obj.get("d").and_then(Value::as_str) {
        Some("U") => 0,
        Some("D") => 1,
        Some("L") => 2,
        Some("R") => 3,
        _ => {
            return Err(error(
                "invalid_action",
                "方向 d は U, D, L, R のいずれかで指定する",
            ))
        }
    };
    Ok(Operation {
        p: i * input.n + j,
        k,
        d,
        l,
    })
}

fn colors(stack: &[usize]) -> Vec<usize> {
    stack.iter().map(|c| c - 1).collect()
}

struct Core {
    input: Input,
    state: State,
    last: Option<Operation>,
}

impl Core {
    fn new(text: &str) -> Result<Self, String> {
        let input = crate::impl_vis::parse_input(text).map_err(|e| error("invalid_input", e))?;
        let state = State::new(&input);
        Ok(Self {
            input,
            state,
            last: None,
        })
    }

    fn metrics(&self, state: &State) -> Value {
        let remaining = self.input.b.iter().sum::<usize>() - state.received.iter().sum::<usize>();
        json!({"T": state.t, "E": remaining, "S": state.score(&self.input), "completed": remaining == 0})
    }

    fn snapshot(&self) -> Value {
        let input = &self.input;
        let mut result = self.metrics(&self.state);
        let obj = result.as_object_mut().unwrap();
        obj.insert("N".into(), json!(input.n));
        obj.insert("K".into(), json!(input.k));
        obj.insert("M".into(), json!(input.b.iter().sum::<usize>()));
        obj.insert(
            "walls".into(),
            json!(input
                .floor
                .iter()
                .enumerate()
                .filter(|(_, f)| !**f)
                .map(|(p, _)| [p / input.n, p % input.n])
                .collect::<Vec<_>>()),
        );
        obj.insert(
            "nests".into(),
            json!(input
                .plates
                .iter()
                .enumerate()
                .map(|(c, p)| json!({"i": p / input.n, "j": p % input.n, "color": c}))
                .collect::<Vec<_>>()),
        );
        obj.insert(
            "towers".into(),
            json!(self
                .state
                .stacks
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.is_empty())
                .map(|(p, s)| json!({"i": p / input.n, "j": p % input.n, "colors": colors(s)}))
                .collect::<Vec<_>>()),
        );
        obj.insert("received".into(), json!(self.state.received));
        obj.insert("initial_counts".into(), json!(input.b));
        result
    }

    fn trial(&self, op: &Operation) -> Result<State, String> {
        if self.state.t >= 100000 {
            return Err(error("operation_limit", "操作数の上限100000に達している"));
        }
        let mut next = self.state.clone();
        next.apply(&self.input, op).map_err(rule_error)?;
        Ok(next)
    }

    fn effect(&self, op: &Operation, next: &State) -> Value {
        let input = &self.input;
        let q = tools::neighbor(input.n, op.p, op.d, op.l).unwrap();
        let before = &self.state;
        let mut landing = before.stacks[q].clone();
        landing.extend(before.stacks[op.p][op.k..].iter().rev());
        let source_before_home = &before.stacks[op.p][..op.k];
        let returned_source = colors(&source_before_home[next.stacks[op.p].len()..]);
        let returned_destination = colors(&landing[next.stacks[q].len()..]);
        let mut result = self.metrics(next);
        let obj = result.as_object_mut().unwrap();
        obj.insert("action".into(), action_json(input, op));
        obj.insert(
            "destination".into(),
            json!({"i": q / input.n, "j": q % input.n}),
        );
        obj.insert("moved".into(), json!(before.stacks[op.p].len() - op.k));
        obj.insert(
            "returned".into(),
            json!({
            "source": {"count": returned_source.len(), "colors": returned_source},
            "destination": {"count": returned_destination.len(), "colors": returned_destination}}),
        );
        obj.insert(
            "changed_cells".into(),
            json!([op.p, q].map(|p| json!({
            "i": p / input.n, "j": p % input.n,
            "nest": input.plates.iter().position(|v| *v == p),
            "before": colors(&before.stacks[p]), "after": colors(&next.stacks[p])}))),
        );
        obj.insert(
            "delta".into(),
            json!({"T": 1,
            "E": -(returned_source.len() as i64) - returned_destination.len() as i64,
            "S": next.score(input) - before.score(input)}),
        );
        result
    }
}

/// ブラウザ用・Node用で同じ公式Stateを使い、JSON境界だけをここで定義する。
#[wasm_bindgen]
pub struct PlayGame {
    core: Core,
}

#[wasm_bindgen]
impl PlayGame {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Result<PlayGame, JsValue> {
        Core::new(input)
            .map(|core| PlayGame { core })
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn snapshot(&self) -> String {
        self.core.snapshot().to_string()
    }

    pub fn preview(&self, action: &str) -> Result<String, JsValue> {
        let op = parse_action(&self.core.input, action).map_err(|e| JsValue::from_str(&e))?;
        let next = self.core.trial(&op).map_err(|e| JsValue::from_str(&e))?;
        Ok(self.core.effect(&op, &next).to_string())
    }

    pub fn apply(&mut self, action: &str) -> Result<String, JsValue> {
        let op = parse_action(&self.core.input, action).map_err(|e| JsValue::from_str(&e))?;
        let next = self.core.trial(&op).map_err(|e| JsValue::from_str(&e))?;
        let result = self.core.effect(&op, &next).to_string();
        self.core.state = next;
        self.core.last = Some(op);
        Ok(result)
    }

    pub fn legal(&self, i: usize, j: usize, k: i32) -> Result<String, JsValue> {
        let input = &self.core.input;
        if i >= input.n || j >= input.n || !(-1..=7).contains(&k) {
            return Err(JsValue::from_str(&error(
                "invalid_query",
                "盤面内の座標と k=-1（全候補）または0〜7を指定する",
            )));
        }
        let p = i * input.n + j;
        let mut actions = vec![];
        for left in 0..self.core.state.stacks[p].len() {
            if k >= 0 && left != k as usize {
                continue;
            }
            for d in 0..4 {
                for l in 1..=left + 1 {
                    let op = Operation { p, k: left, d, l };
                    if self.core.trial(&op).is_ok() {
                        actions.push(action_json(input, &op));
                    }
                }
            }
        }
        Ok(json!({"actions": actions}).to_string())
    }

    // 再生は一時状態で完了させ、不正な出力で現在の局面が途中まで変わることを防ぐ。
    pub fn replay(&mut self, output: &str) -> Result<String, JsValue> {
        let parsed = tools::parse_output(&self.core.input, output)
            .map_err(|e| JsValue::from_str(&error("invalid_output", e)))?;
        let mut state = State::new(&self.core.input);
        for (t, op) in parsed.ops.iter().enumerate() {
            state.apply(&self.core.input, op).map_err(|e| {
                JsValue::from_str(&error("invalid_output", format!("操作 {}: {e}", t + 1)))
            })?;
        }
        let actions = parsed
            .ops
            .iter()
            .map(|o| action_json(&self.core.input, o))
            .collect::<Vec<_>>();
        self.core.state = state;
        self.core.last = parsed.ops.last().cloned();
        Ok(json!({"actions": actions}).to_string())
    }

    pub fn svg(&self) -> String {
        tools::vis::draw_state(&self.core.input, &self.core.state, self.core.last.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> Core {
        let input = tools::generate(0, &tools::GenOption::default()).to_string();
        Core::new(&input).unwrap()
    }

    #[test]
    fn flip_and_receive_at_both_ends() {
        let mut game = core();
        game.input.floor.fill(true);
        game.input.plates = vec![0, 3, 20, 21];
        game.state.stacks.iter_mut().for_each(Vec::clear);
        game.state.stacks[0] = vec![1, 1, 2];
        let op = Operation {
            p: 0,
            k: 2,
            d: 3,
            l: 3,
        };
        let next = game.trial(&op).unwrap();
        assert!(next.stacks[0].is_empty() && next.stacks[3].is_empty());
        let effect = game.effect(&op, &next);
        assert_eq!(effect["returned"]["source"]["colors"], json!([0, 0]));
        assert_eq!(effect["returned"]["destination"]["colors"], json!([1]));
        assert_eq!(game.state.t, 0);
        game.state.stacks[10] = vec![1, 2, 3, 4];
        game.state.stacks[11] = vec![2];
        let next = game
            .trial(&Operation {
                p: 10,
                k: 1,
                d: 3,
                l: 1,
            })
            .unwrap();
        assert_eq!(next.stacks[10], vec![1]);
        assert_eq!(next.stacks[11], vec![2, 4, 3, 2]);
    }

    #[test]
    fn rejects_height_before_home_and_wall_and_zero_distance() {
        let mut game = core();
        game.input.floor.fill(true);
        game.input.plates[0] = 1;
        game.state.stacks[0] = vec![1, 1];
        game.state.stacks[1] = vec![1; 7];
        assert!(game
            .trial(&Operation {
                p: 0,
                k: 0,
                d: 3,
                l: 1
            })
            .unwrap_err()
            .contains("height_limit_exceeded"));
        game.input.floor[1] = false;
        assert!(game
            .trial(&Operation {
                p: 0,
                k: 1,
                d: 3,
                l: 2
            })
            .unwrap_err()
            .contains("wall_on_path"));
        assert!(parse_action(&game.input, r#"{"i":0,"j":0,"k":0,"d":"R","l":0}"#).is_err());
        game.state.t = 100000;
        assert!(game
            .trial(&Operation {
                p: 0,
                k: 0,
                d: 3,
                l: 1
            })
            .unwrap_err()
            .contains("operation_limit"));
    }
}
