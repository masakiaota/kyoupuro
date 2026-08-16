// case_theta_probe.rs
#![allow(non_snake_case)]

use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::fs;

const N: usize = 50;

fn main() {
    let seed = std::env::args()
        .nth(1)
        .expect("usage: case_theta_probe <seed> <input>")
        .parse::<u64>()
        .unwrap();
    let input_path = std::env::args().nth(2).expect("missing input path");

    // 公式 generator と theta の抽選直前まで同じ乱数消費を再現する。
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let R = rng.gen_range(1..=100) as f64 * 0.001;
    let mut grass = vec![vec![true; N]; N];
    let num_cluster = 2f64.powf(rng.gen_range(1.0..8.0)).round() as usize;
    let mut cells: Vec<(usize, usize)> = (0..N)
        .flat_map(|i| (0..N).map(move |j| (i, j)))
        .collect();
    cells.shuffle(&mut rng);
    for &(i, j) in cells.iter().take(num_cluster) {
        grass[i][j] = false;
    }

    let num_pond = rng.gen_range(0..=(900 - num_cluster) as i32) as usize;
    let dxy = [(0_i32, 1_i32), (0, -1), (1, 0), (-1, 0)];
    let mut in_frontier = vec![vec![false; N]; N];
    let mut frontier = Vec::new();
    let is_pond_neighbor = |grass: &[Vec<bool>], i: usize, j: usize| {
        dxy.iter().any(|&(dx, dy)| {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32
                && !grass[ni as usize][nj as usize]
        })
    };
    for i in 0..N {
        for j in 0..N {
            if grass[i][j] && is_pond_neighbor(&grass, i, j) {
                in_frontier[i][j] = true;
                frontier.push((i, j));
            }
        }
    }
    for _ in 0..num_pond {
        if frontier.is_empty() {
            break;
        }
        let idx = rng.gen_range(0..frontier.len() as i32) as usize;
        let (i, j) = frontier.swap_remove(idx);
        in_frontier[i][j] = false;
        grass[i][j] = false;
        for &(dx, dy) in &dxy {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            if ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32 {
                let (ni, nj) = (ni as usize, nj as usize);
                if grass[ni][nj] && !in_frontier[ni][nj] {
                    in_frontier[ni][nj] = true;
                    frontier.push((ni, nj));
                }
            }
        }
    }
    let theta = rng.gen_range(2000..=8000);

    let input = fs::read_to_string(input_path).unwrap();
    let mut lines = input.lines();
    let header = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
    let input_R = header[2].parse::<f64>().unwrap();
    let layout_matches = (0..N).all(|i| {
        let expected = grass[i]
            .iter()
            .map(|&cell| if cell { '.' } else { '#' })
            .collect::<String>();
        lines.next() == Some(expected.as_str())
    });
    println!(
        "seed={seed} theta={theta} R={R:.3} input_R={input_R:.3} layout_matches={layout_matches}"
    );
}
