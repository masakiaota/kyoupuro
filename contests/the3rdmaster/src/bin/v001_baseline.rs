// v001_baseline.rs
use std::fmt::Write as _;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut it = input.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let _k: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    let mut out = String::new();
    for i in 0..n {
        for j in 0..n {
            let color: i32 = it.next().unwrap().parse().unwrap();
            if color == 0 {
                continue;
            }
            writeln!(&mut out, "0 0 {i} {j} {color}").unwrap();
        }
    }

    print!("{out}");
}
