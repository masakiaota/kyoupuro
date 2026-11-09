use wasm_bindgen::prelude::*;

mod impl_vis;

#[wasm_bindgen(getter_with_clone)]
pub struct Ret {
    pub score: i64,
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
pub fn get_order_ends(input: &str, output: &str) -> Vec<u32> {
    impl_vis::calc_order_ends(input, output)
        .into_iter()
        .map(|t| t as u32)
        .collect()
}

/// move_start..turn の手を重ね描きする。1 手ずつ見るときは turn - 1、注文単位では注文の開始 turn を渡す。
#[wasm_bindgen]
pub fn vis(input: &str, output: &str, turn: i32, move_start: i32) -> Result<Ret, JsValue> {
    match impl_vis::visualize(
        input,
        output,
        turn.max(0) as usize,
        move_start.max(0) as usize,
    ) {
        Ok((score, err, svg)) => Ok(Ret { score, err, svg }),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}
