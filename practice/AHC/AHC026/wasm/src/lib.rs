use wasm_bindgen::prelude::*;

mod impl_vis;

#[wasm_bindgen(getter_with_clone)]
pub struct Ret {
    pub score: i64,
    /// 体力消費量 V (= 10000 - score、ただし clamp 前の生値)。turn 時点までの累積。
    pub cost: i64,
    pub err: String,
    pub svg: String,
}

#[wasm_bindgen]
pub fn gen(seed: i32) -> String {
    impl_vis::generate(seed)
}

#[wasm_bindgen]
pub fn get_max_turn(input: &str, output: &str) -> i32 {
    impl_vis::calc_max_turn(input, output) as i32
}

#[wasm_bindgen]
pub fn vis(input: &str, output: &str, turn: i32) -> Result<Ret, JsValue> {
    match impl_vis::visualize(input, output, turn.max(0) as usize) {
        Ok((score, cost, err, svg)) => Ok(Ret {
            score,
            cost,
            err,
            svg,
        }),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}
