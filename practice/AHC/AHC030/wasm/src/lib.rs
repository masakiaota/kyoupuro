use wasm_bindgen::prelude::*;

mod impl_vis;

#[wasm_bindgen(getter_with_clone)]
pub struct Ret {
    pub score: i64,
    pub err: String,
    pub svg: String,
    pub graph_svg: String,
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
        Ok((score, err, svg, graph_svg)) => Ok(Ret {
            score,
            err,
            svg,
            graph_svg,
        }),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}
