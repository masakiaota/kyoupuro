// check_dynamic_gap_relocation.rs
#![allow(non_snake_case)]

use std::collections::VecDeque;

fn components(free: &[bool], N: usize) -> Vec<Vec<usize>> {
    let mut seen = vec![false; N * N];
    let mut result = Vec::new();
    for start in 0..N * N {
        if !free[start] || seen[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut cells = Vec::new();
        seen[start] = true;
        while let Some(id) = queue.pop_front() {
            cells.push(id);
            let r = id / N;
            let c = id % N;
            for next in [
                (r > 0).then_some(id - N),
                (r + 1 < N).then_some(id + N),
                (c > 0).then_some(id - 1),
                (c + 1 < N).then_some(id + 1),
            ]
            .into_iter()
            .flatten()
            {
                if free[next] && !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
        result.push(cells);
    }
    result
}

fn connected(cells: &[usize], N: usize) -> bool {
    let mut selected = vec![false; N * N];
    for &id in cells {
        selected[id] = true;
    }
    let mut seen = vec![false; N * N];
    let mut queue = VecDeque::from([cells[0]]);
    seen[cells[0]] = true;
    let mut count = 0;
    while let Some(id) = queue.pop_front() {
        count += 1;
        let r = id / N;
        let c = id % N;
        for next in [
            (r > 0).then_some(id - N),
            (r + 1 < N).then_some(id + N),
            (c > 0).then_some(id - 1),
            (c + 1 < N).then_some(id + 1),
        ]
        .into_iter()
        .flatten()
        {
            if selected[next] && !seen[next] {
                seen[next] = true;
                queue.push_back(next);
            }
        }
    }
    count == cells.len()
}

fn perimeter(cells: &[usize], N: usize) -> usize {
    let mut selected = vec![false; N * N];
    for &id in cells {
        selected[id] = true;
    }
    let mut result = 0;
    for &id in cells {
        let r = id / N;
        let c = id % N;
        result += usize::from(r == 0 || !selected[id - N]);
        result += usize::from(r + 1 == N || !selected[id + N]);
        result += usize::from(c == 0 || !selected[id - 1]);
        result += usize::from(c + 1 == N || !selected[id + 1]);
    }
    result
}

fn main() {
    const N: usize = 7;
    let main_free = [8, 9, 15, 16];
    let gap = [12, 19, 26, 33];
    let old_group = [10, 17, 23, 24];
    let P = old_group.len();

    let mut before = vec![false; N * N];
    for &id in main_free.iter().chain(&gap) {
        before[id] = true;
    }
    let before_components = components(&before, N);
    assert_eq!(before_components.len(), 2);
    assert_eq!(before_components.iter().map(Vec::len).max(), Some(4));
    assert!(gap.iter().all(|&id| before[id]));
    assert!(old_group.iter().all(|&id| !before[id]));

    let mut after = before.clone();
    for &id in &gap {
        after[id] = false;
    }
    for &id in &old_group {
        after[id] = true;
    }
    let after_components = components(&after, N);

    assert_eq!(before.iter().filter(|&&cell| cell).count(), 2 * P);
    assert_eq!(after.iter().filter(|&&cell| cell).count(), 2 * P);
    assert_eq!(after_components.iter().map(Vec::len).max(), Some(8));
    assert!(connected(&old_group, N));
    assert!(connected(&gap, N));
    assert!(perimeter(&gap, N) <= perimeter(&old_group, N));

    println!("dynamic gap relocation invariants: ok");
}
