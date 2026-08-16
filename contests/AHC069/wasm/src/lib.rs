#![allow(non_snake_case)]

use wasm_bindgen::prelude::*;

mod impl_vis;

#[wasm_bindgen(getter_with_clone)]
pub struct PrepareRet {
    pub N: u32,
    pub max_turn: u32,
    pub default_turn: u32,
    pub score: f64,
    pub error: String,
    pub grass_cells: u32,
    pub max_time_left: f64,
    grass: Vec<u8>,
}

#[wasm_bindgen]
impl PrepareRet {
    #[wasm_bindgen(getter)]
    pub fn grass(&self) -> Vec<u8> {
        self.grass.clone()
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct FrameRet {
    pub turn: u32,
    pub money: f64,
    pub now: f64,
    pub arrival_id: i32,
    pub arrival_accepted: bool,
    pub arrival_s: f64,
    pub arrival_t: f64,
    pub arrival_p: u32,
    pub arrival_v: f64,
    pub cells_used: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub total_fee: f64,
    pub total_move_cost: f64,
    pub comment: String,
    grid: Vec<u16>,
    moved: Vec<u16>,
    move_sources: Vec<u16>,
    departed: Vec<f64>,
    actives: Vec<f64>,
}

#[wasm_bindgen]
impl FrameRet {
    #[wasm_bindgen(getter)]
    pub fn grid(&self) -> Vec<u16> {
        self.grid.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn moved(&self) -> Vec<u16> {
        self.moved.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn move_sources(&self) -> Vec<u16> {
        self.move_sources.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn departed(&self) -> Vec<f64> {
        self.departed.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn actives(&self) -> Vec<f64> {
        self.actives.clone()
    }
}

#[wasm_bindgen]
pub fn gen(seed: i32) -> String {
    impl_vis::generate(seed)
}

#[wasm_bindgen]
pub fn prepare_case(input: &str, output: &str) -> Result<PrepareRet, JsValue> {
    impl_vis::prepare(input, output)
        .map(|prepared| PrepareRet {
            N: prepared.N as u32,
            max_turn: prepared.max_turn as u32,
            default_turn: prepared.default_turn as u32,
            score: prepared.score as f64,
            error: prepared.error,
            grass_cells: prepared.grass_cells as u32,
            max_time_left: prepared.max_time_left as f64,
            grass: prepared.grass,
        })
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn get_frame(turn: i32) -> Result<FrameRet, JsValue> {
    impl_vis::frame(turn.max(0) as usize)
        .map(|frame| FrameRet {
            turn: frame.turn as u32,
            money: frame.money as f64,
            now: frame.now as f64,
            arrival_id: frame.arrival_id,
            arrival_accepted: frame.arrival_accepted,
            arrival_s: frame.arrival_s as f64,
            arrival_t: frame.arrival_t as f64,
            arrival_p: frame.arrival_p as u32,
            arrival_v: frame.arrival_v as f64,
            cells_used: frame.cells_used as u32,
            accepted: frame.accepted as u32,
            rejected: frame.rejected as u32,
            total_fee: frame.total_fee as f64,
            total_move_cost: frame.total_move_cost as f64,
            comment: frame.comment,
            grid: frame.grid,
            moved: frame.moved,
            move_sources: frame.move_sources,
            departed: frame.departed,
            actives: frame.actives,
        })
        .map_err(|error| JsValue::from_str(&error))
}
