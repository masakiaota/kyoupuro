// check_deadline_shelves.rs
#![allow(dead_code)]

#[path = "../../../src/bin/v062_no_move_deadline_shelves.rs"]
mod solver;

fn main() {
    solver::verify_deadline_shelves();
    println!("deadline shelf checks passed");
}
