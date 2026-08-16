// check_size_position_bias.rs
#![allow(dead_code)]

#[path = "../../../src/bin/v063_no_move_size_gradient.rs"]
mod solver;

fn main() {
    solver::verify_size_position_bias();
    println!("size position bias checks passed");
}
