#![allow(non_snake_case)]

use std::collections::BinaryHeap;
use std::hint::black_box;
use std::mem::MaybeUninit;
use std::time::Instant;

const MAX_N: usize = 50;
const MAX_P: usize = 150;
const MAX_C: usize = MAX_N * MAX_N;
type Rows = [u64; MAX_N];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 7;
        x ^= x >> 9;
        self.0 = x;
        x
    }
}

#[inline]
fn is_free(grass: &Rows, occ: &Rows, r: usize, c: usize) -> bool {
    ((grass[r] >> c) & 1) != 0 && ((occ[r] >> c) & 1) == 0
}

#[inline]
fn fit_probability(size: usize) -> f64 {
    if size < 4 {
        0.0
    } else if size >= MAX_P {
        1.0
    } else {
        ((size - 3) as f64 / (MAX_P - 3) as f64).sqrt()
    }
}

fn old_fragment(N: usize, grass: &Rows, occ: &Rows) -> f64 {
    let mut seen = [false; MAX_C];
    let mut queue = [0_usize; MAX_C];
    let mut sizes = [0_usize; MAX_C];
    let mut component_count = 0;
    let mut dead_ends = 0;
    const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for r in 0..N {
        for c in 0..N {
            if !is_free(grass, occ, r, c) {
                continue;
            }
            let id = r * N + c;
            let mut degree = 0;
            for (dr, dc) in DIRS {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0
                    && nr < N as isize
                    && nc >= 0
                    && nc < N as isize
                    && is_free(grass, occ, nr as usize, nc as usize)
                {
                    degree += 1;
                }
            }
            if degree <= 1 {
                dead_ends += 1;
            }
            if seen[id] {
                continue;
            }
            let mut head = 0;
            let mut tail = 1;
            queue[0] = id;
            seen[id] = true;
            while head < tail {
                let v = queue[head];
                head += 1;
                let vr = v / N;
                let vc = v % N;
                for (dr, dc) in DIRS {
                    let nr = vr as isize + dr;
                    let nc = vc as isize + dc;
                    if nr < 0 || nr >= N as isize || nc < 0 || nc >= N as isize {
                        continue;
                    }
                    let next = nr as usize * N + nc as usize;
                    if seen[next] || !is_free(grass, occ, nr as usize, nc as usize) {
                        continue;
                    }
                    seen[next] = true;
                    queue[tail] = next;
                    tail += 1;
                }
            }
            sizes[component_count] = tail;
            component_count += 1;
        }
    }
    metric_from_sizes(dead_ends, &sizes[..component_count])
}

fn metric_from_sizes(dead_ends: usize, sizes: &[usize]) -> f64 {
    let mut metric = 4.0 * dead_ends as f64;
    for &size in sizes {
        if size < 4 {
            metric += 100.0 * size as f64;
        } else {
            metric += 18.0 + 3.0 * (size as f64).sqrt();
            if size < MAX_P {
                metric += 30.0 * (1.0 - fit_probability(size));
            }
        }
    }
    metric
}

#[inline(always)]
fn horizontal_closure(mut bits: u64, available: u64) -> u64 {
    loop {
        let expanded = (bits | (bits << 1) | (bits >> 1)) & available;
        if expanded == bits {
            return bits;
        }
        bits = expanded;
    }
}

fn new_fragment(N: usize, grass: &Rows, occ: &Rows) -> f64 {
    let mut unvisited = [0_u64; MAX_N];
    for r in 0..N {
        unvisited[r] = grass[r] & !occ[r];
    }
    let mut dead_ends = 0;
    for r in 0..N {
        let here = unvisited[r];
        let up = if r > 0 { unvisited[r - 1] } else { 0 };
        let down = if r + 1 < N { unvisited[r + 1] } else { 0 };
        let left = here << 1;
        let right = here >> 1;
        let at_least_two = (up & down)
            | (up & left)
            | (up & right)
            | (down & left)
            | (down & right)
            | (left & right);
        dead_ends += (here & !at_least_two).count_ones() as usize;
    }
    let mut metric = 4.0 * dead_ends as f64;
    let mut row_queue = [0_usize; MAX_C];
    let mut bits_queue = [0_u64; MAX_C];
    let mut first_row = 0;
    loop {
        while first_row < N && unvisited[first_row] == 0 {
            first_row += 1;
        }
        if first_row == N {
            break;
        }
        let available = unvisited[first_row];
        let segment = horizontal_closure(available & available.wrapping_neg(), available);
        unvisited[first_row] &= !segment;
        row_queue[0] = first_row;
        bits_queue[0] = segment;
        let mut head = 0;
        let mut tail = 1;
        let mut size = segment.count_ones() as usize;
        while head < tail {
            let r = row_queue[head];
            let bits = bits_queue[head];
            head += 1;
            for nr in [r.checked_sub(1), (r + 1 < N).then_some(r + 1)]
                .into_iter()
                .flatten()
            {
                let available = unvisited[nr];
                let seeds = bits & available;
                if seeds == 0 {
                    continue;
                }
                let next = horizontal_closure(seeds, available);
                unvisited[nr] &= !next;
                row_queue[tail] = nr;
                bits_queue[tail] = next;
                tail += 1;
                size += next.count_ones() as usize;
            }
        }
        if size < 4 {
            metric += 100.0 * size as f64;
        } else {
            metric += 18.0 + 3.0 * (size as f64).sqrt();
            if size < MAX_P {
                metric += 30.0 * (1.0 - fit_probability(size));
            }
        }
    }
    metric
}

#[derive(PartialEq, Eq, Debug)]
struct FreeSummary {
    component: [i16; MAX_C],
    sizes: Vec<usize>,
    cells: Vec<Vec<usize>>,
    free_count: usize,
    dead_ends: usize,
}

fn old_free_info(N: usize, grass: &Rows, occ: &Rows) -> FreeSummary {
    let mut out = FreeSummary {
        component: [-1; MAX_C],
        sizes: Vec::new(),
        cells: Vec::new(),
        free_count: 0,
        dead_ends: 0,
    };
    let mut queue = [0_usize; MAX_C];
    const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut component_id = 0_i16;
    for r in 0..N {
        for c in 0..N {
            if !is_free(grass, occ, r, c) {
                continue;
            }
            let id = r * N + c;
            out.free_count += 1;
            let mut degree = 0;
            for (dr, dc) in DIRS {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0
                    && nr < N as isize
                    && nc >= 0
                    && nc < N as isize
                    && is_free(grass, occ, nr as usize, nc as usize)
                {
                    degree += 1;
                }
            }
            out.dead_ends += usize::from(degree <= 1);
            if out.component[id] >= 0 {
                continue;
            }
            let mut head = 0;
            let mut tail = 1;
            queue[0] = id;
            out.component[id] = component_id;
            while head < tail {
                let v = queue[head];
                head += 1;
                let vr = v / N;
                let vc = v % N;
                for (dr, dc) in DIRS {
                    let nr = vr as isize + dr;
                    let nc = vc as isize + dc;
                    if nr < 0 || nr >= N as isize || nc < 0 || nc >= N as isize {
                        continue;
                    }
                    let next = nr as usize * N + nc as usize;
                    if out.component[next] < 0 && is_free(grass, occ, nr as usize, nc as usize) {
                        out.component[next] = component_id;
                        queue[tail] = next;
                        tail += 1;
                    }
                }
            }
            out.sizes.push(tail);
            out.cells.push(queue[..tail].to_vec());
            component_id += 1;
        }
    }
    out
}

fn new_free_info(N: usize, grass: &Rows, occ: &Rows) -> FreeSummary {
    let mut out = FreeSummary {
        component: [-1; MAX_C],
        sizes: Vec::new(),
        cells: Vec::new(),
        free_count: 0,
        dead_ends: 0,
    };
    let mut free = [0_u64; MAX_N];
    for r in 0..N {
        free[r] = grass[r] & !occ[r];
        out.free_count += free[r].count_ones() as usize;
    }
    for r in 0..N {
        let here = free[r];
        let up = if r > 0 { free[r - 1] } else { 0 };
        let down = if r + 1 < N { free[r + 1] } else { 0 };
        let left = here << 1;
        let right = here >> 1;
        let at_least_two = (up & down)
            | (up & left)
            | (up & right)
            | (down & left)
            | (down & right)
            | (left & right);
        out.dead_ends += (here & !at_least_two).count_ones() as usize;
    }
    let mut queue = [0_usize; MAX_C];
    let mut component_id = 0_i16;
    for r in 0..N {
        for c in 0..N {
            if ((free[r] >> c) & 1) == 0 {
                continue;
            }
            let id = r * N + c;
            if out.component[id] >= 0 {
                continue;
            }
            let mut head = 0;
            let mut tail = 1;
            queue[0] = id;
            out.component[id] = component_id;
            while head < tail {
                let v = queue[head];
                head += 1;
                let vr = v / N;
                let vc = v % N;
                let mut push = |next: usize, nr: usize, nc: usize| {
                    if out.component[next] < 0 && ((free[nr] >> nc) & 1) != 0 {
                        out.component[next] = component_id;
                        queue[tail] = next;
                        tail += 1;
                    }
                };
                if vr > 0 {
                    push(v - N, vr - 1, vc);
                }
                if vr + 1 < N {
                    push(v + N, vr + 1, vc);
                }
                if vc > 0 {
                    push(v - 1, vr, vc - 1);
                }
                if vc + 1 < N {
                    push(v + 1, vr, vc + 1);
                }
            }
            out.sizes.push(tail);
            out.cells.push(queue[..tail].to_vec());
            component_id += 1;
        }
    }
    out
}

fn old_run(mask: u64, len: usize) -> u64 {
    let mut run = mask;
    for shift in 1..len {
        run &= mask >> shift;
    }
    run
}

fn new_run(mask: u64, len: usize) -> u64 {
    let mut run = mask;
    let mut covered = 1;
    while covered < len {
        let shift = covered.min(len - covered);
        run &= run >> shift;
        covered += shift;
    }
    run
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn verify_region_set(rng: &mut Rng) {
    for _ in 0..2_000 {
        let mut old_keys = [0_u64; 512];
        let mut old_used = [false; 512];
        let mut new_keys = [MaybeUninit::<u64>::uninit(); 512];
        let mut new_used = [0_u64; 8];
        let mut history = [0_u64; 256];
        let mut history_len = 0;
        for step in 0..400 {
            let value = if step % 5 == 0 && history_len > 0 {
                history[rng.next() as usize % history_len]
            } else {
                let value = rng.next();
                if history_len < history.len() {
                    history[history_len] = value;
                    history_len += 1;
                }
                value
            };
            let mut old_pos = splitmix64(value) as usize & 511;
            while old_used[old_pos] && old_keys[old_pos] != value {
                old_pos = (old_pos + 1) & 511;
            }
            let old_inserted = !old_used[old_pos];
            if old_inserted {
                old_used[old_pos] = true;
                old_keys[old_pos] = value;
            }
            let mut new_pos = splitmix64(value) as usize & 511;
            while ((new_used[new_pos >> 6] >> (new_pos & 63)) & 1) != 0
                && unsafe { new_keys[new_pos].assume_init() } != value
            {
                new_pos = (new_pos + 1) & 511;
            }
            let new_inserted = ((new_used[new_pos >> 6] >> (new_pos & 63)) & 1) == 0;
            if new_inserted {
                new_used[new_pos >> 6] |= 1_u64 << (new_pos & 63);
                new_keys[new_pos].write(value);
            }
            assert_eq!(old_inserted, new_inserted);
            assert_eq!(old_pos, new_pos);
        }
    }
}

fn old_selected_is_connected(
    N: usize,
    selected: &[bool; MAX_C],
    start: usize,
    expected: usize,
) -> bool {
    let mut seen = [false; MAX_C];
    let mut stack = [0_usize; MAX_C];
    let mut top = 1;
    stack[0] = start;
    seen[start] = true;
    let mut reached = 0;
    while top > 0 {
        top -= 1;
        let id = stack[top];
        reached += 1;
        let r = id / N;
        let c = id % N;
        for next in [
            (r > 0).then_some(id.wrapping_sub(N)),
            (r + 1 < N).then_some(id + N),
            (c > 0).then_some(id.wrapping_sub(1)),
            (c + 1 < N).then_some(id + 1),
        ]
        .into_iter()
        .flatten()
        {
            if selected[next] && !seen[next] {
                seen[next] = true;
                stack[top] = next;
                top += 1;
            }
        }
    }
    reached == expected
}

fn new_selected_is_connected(
    N: usize,
    selected: &[bool; MAX_C],
    start: usize,
    expected: usize,
    seen: &mut [u16; MAX_C],
    stamp: u16,
) -> bool {
    let mut stack = [MaybeUninit::<usize>::uninit(); MAX_P];
    let mut top = 1;
    stack[0].write(start);
    seen[start] = stamp;
    let mut reached = 0;
    while top > 0 {
        top -= 1;
        let id = unsafe { stack.get_unchecked(top).assume_init() };
        reached += 1;
        let r = id / N;
        let c = id % N;
        for next in [
            (r > 0).then_some(id.wrapping_sub(N)),
            (r + 1 < N).then_some(id + N),
            (c > 0).then_some(id.wrapping_sub(1)),
            (c + 1 < N).then_some(id + 1),
        ]
        .into_iter()
        .flatten()
        {
            if selected[next] && seen[next] != stamp {
                seen[next] = stamp;
                unsafe {
                    stack.get_unchecked_mut(top).write(next);
                }
                top += 1;
            }
        }
    }
    reached == expected
}

fn main() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut boards = Vec::with_capacity(2_048);
    for case in 0..2_048 {
        let N = 20 + case % 31;
        let mask = (1_u64 << N) - 1;
        let mut grass = [0_u64; MAX_N];
        let mut occ = [0_u64; MAX_N];
        for r in 0..N {
            grass[r] = (rng.next() | rng.next()) & mask;
            occ[r] = rng.next() & rng.next() & grass[r];
        }
        boards.push((N, grass, occ));
    }

    for (index, (N, grass, occ)) in boards.iter().enumerate() {
        let old = old_fragment(*N, grass, occ);
        let new = new_fragment(*N, grass, occ);
        assert_eq!(old.to_bits(), new.to_bits(), "fragment case {index}");
        assert_eq!(old_free_info(*N, grass, occ), new_free_info(*N, grass, occ));
    }
    for _ in 0..100_000 {
        let mask = rng.next();
        for len in 1..=MAX_N {
            assert_eq!(old_run(mask, len), new_run(mask, len));
        }
    }
    // roughの全件sort→固定長top-k変換を、同点を多く含む有限scoreで照合する。
    for _ in 0..20_000 {
        let len = 1 + (rng.next() as usize % 200);
        let limit = 1 + (rng.next() as usize % 16);
        let mut all = Vec::with_capacity(len);
        for id in 0..len {
            all.push(((rng.next() % 23) as f64 * 0.25, id));
        }
        let compare =
            |a: &(f64, usize), b: &(f64, usize)| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1));
        let mut expected = all.clone();
        expected.sort_by(compare);
        expected.truncate(limit);
        let mut fixed = [(0.0_f64, 0_usize); 16];
        let mut count = 0;
        for item in all {
            if count < limit {
                fixed[count] = item;
                count += 1;
            } else {
                let mut worst = 0;
                for i in 1..count {
                    if compare(&fixed[worst], &fixed[i]).is_lt() {
                        worst = i;
                    }
                }
                if compare(&item, &fixed[worst]).is_lt() {
                    fixed[worst] = item;
                }
            }
        }
        fixed[..count].sort_by(compare);
        assert_eq!(expected, fixed[..count]);
    }
    // BinaryHeap::into_iterと再利用heapのdrainが同じ内部順を返すことも照合する。
    let mut reused = BinaryHeap::with_capacity(64);
    for _ in 0..20_000 {
        let limit = 1 + rng.next() as usize % 48;
        let mut fresh = BinaryHeap::with_capacity(64);
        reused.clear();
        for _ in 0..200 {
            let item = (
                (rng.next() % 97) as i64,
                rng.next() as usize % 50,
                rng.next() as usize % 50,
                rng.next() as usize % 100,
            );
            for heap in [&mut fresh, &mut reused] {
                if heap.len() < limit {
                    heap.push(item);
                } else if item.0 < heap.peek().unwrap().0 {
                    heap.pop();
                    heap.push(item);
                }
            }
        }
        let expected: Vec<_> = fresh.into_iter().collect();
        let actual: Vec<_> = reused.drain().collect();
        assert_eq!(expected, actual);
    }
    verify_region_set(&mut rng);

    // biased swap の大配列初期化を stamp に替えた連結判定を旧実装と照合する。
    let mut connectivity_seen = [0_u16; MAX_C];
    let mut connectivity_stamp = 0_u16;
    for case in 0..20_000 {
        let N = 20 + rng.next() as usize % 31;
        let selected_count = 1 + rng.next() as usize % MAX_P;
        let mut selected = [false; MAX_C];
        let mut cells = [0_usize; MAX_P];
        let mut count = 0;
        while count < selected_count {
            let id = rng.next() as usize % (N * N);
            if !selected[id] {
                selected[id] = true;
                cells[count] = id;
                count += 1;
            }
        }
        connectivity_stamp = connectivity_stamp.wrapping_add(1);
        if connectivity_stamp == 0 {
            connectivity_seen.fill(0);
            connectivity_stamp = 1;
        }
        let start = cells[rng.next() as usize % selected_count];
        let old = old_selected_is_connected(N, &selected, start, selected_count);
        let new = new_selected_is_connected(
            N,
            &selected,
            start,
            selected_count,
            &mut connectivity_seen,
            connectivity_stamp,
        );
        assert_eq!(old, new, "connectivity case {case}");
    }

    let rounds = 24;
    let start = Instant::now();
    let mut old_sum = 0_u64;
    for _ in 0..rounds {
        for (N, grass, occ) in &boards {
            old_sum ^= black_box(old_fragment(*N, grass, occ).to_bits());
        }
    }
    let old_elapsed = start.elapsed();
    let start = Instant::now();
    let mut new_sum = 0_u64;
    for _ in 0..rounds {
        for (N, grass, occ) in &boards {
            new_sum ^= black_box(new_fragment(*N, grass, occ).to_bits());
        }
    }
    let new_elapsed = start.elapsed();
    assert_eq!(old_sum, new_sum);
    let calls = (rounds * boards.len()) as f64;
    println!("verified_boards={}", boards.len());
    println!("verified_run_masks={}", 100_000 * MAX_N);
    println!("verified_fixed_topk={}", 20_000);
    println!("verified_heap_reuse={}", 20_000);
    println!("verified_region_set_batches={}", 2_000);
    println!("verified_connectivity={}", 20_000);
    println!(
        "old_fragment_ns_per_call={:.1}",
        old_elapsed.as_nanos() as f64 / calls
    );
    println!(
        "new_fragment_ns_per_call={:.1}",
        new_elapsed.as_nanos() as f64 / calls
    );
    println!(
        "fragment_speedup={:.3}",
        old_elapsed.as_secs_f64() / new_elapsed.as_secs_f64()
    );
}
