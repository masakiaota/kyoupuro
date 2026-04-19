// v001_sample.rs
use std::io::{self, Read};

fn main() {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();

    let n: usize = it.next().unwrap().parse().unwrap();
    let m: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();

    for _ in 0..m {
        let _ = it.next();
    }
    for _ in 0..n * n {
        let _ = it.next();
    }

    let mut ans = Vec::new();

    for _ in 4..(n - 1) {
        ans.push('D');
    }

    for col in 1..n {
        ans.push('R');
        if col % 2 == 1 {
            for _ in 0..(n - 1) {
                ans.push('U');
            }
        } else {
            for _ in 0..(n - 1) {
                ans.push('D');
            }
        }
    }

    let out = ans
        .into_iter()
        .map(|ch| ch.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    println!("{out}");
}
