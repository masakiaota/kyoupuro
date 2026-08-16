// check_long_stay_veto.rs
#![allow(dead_code)]

#[path = "../../../src/bin/v067_posterior_long_stay_veto.rs"]
mod solver;

fn main() {
    solver::verify_long_stay_veto();
    println!("long-stay veto checks passed");
}
