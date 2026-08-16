use std::collections::HashSet;
#![allow(non_snake_case)]

use std::hint::black_box;
use std::time::Instant;

const MAX_N: usize = 50;
const MAX_P: usize = 150;
const MAX_LEN: usize = 25;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CompactSpec {
    a: usize,
    full: usize,
    rem: usize,
    off: usize,
    side: usize,
    rot: usize,
    perimeter: usize,
}

struct BoardData {
    prefix: [[i32; MAX_N + 1]; MAX_N + 1],
    runs: [[u64; MAX_LEN + 1]; MAX_N],
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 7;
        self.state ^= self.state >> 9;
        self.state
    }
}

fn minimum_perimeter(P: usize) -> usize {
    let mut best = usize::MAX;
    for a in 1..=(P as f64).sqrt() as usize + 2 {
        best = best.min(2 * (a + P.div_ceil(a)));
    }
    best
}

fn dimensions(spec: CompactSpec) -> (usize, usize) {
    let base_width = spec.full + usize::from(spec.rem > 0);
    if spec.rot != 0 {
        (base_width, spec.a)
    } else {
        (spec.a, base_width)
    }
}

fn generate_specs(N: usize) -> Vec<CompactSpec> {
    let mut result = Vec::new();
    for P in 4..=MAX_P {
        let minimum = minimum_perimeter(P);
        let mut used = HashSet::new();
        for a in 2..=25.min(P) {
            let full = P / a;
            let rem = P % a;
            if full == 0 {
                continue;
            }
            let base_width = full + usize::from(rem > 0);
            if !(2..=25).contains(&base_width) {
                continue;
            }
            let perimeter = 2 * (a + base_width);
            if perimeter > minimum + 4 || a.max(base_width) * 10 > a.min(base_width) * 28 {
                continue;
            }
            let mut add = |off: usize, side: usize, rot: usize| {
                if !used.insert((a, full, rem, off, side, rot)) {
                    return;
                }
                let spec = CompactSpec {
                    a,
                    full,
                    rem,
                    off,
                    side,
                    rot,
                    perimeter,
                };
                let (height, width) = dimensions(spec);
                if height <= N && width <= N {
                    result.push(spec);
                }
            };
            if rem == 0 {
                add(0, 0, 0);
                if a != full {
                    add(0, 0, 1);
                }
            } else {
                let mut offsets = [0, a - rem, (a - rem) / 2];
                offsets.sort_unstable();
                let mut previous = usize::MAX;
                for off in offsets {
                    if off == previous {
                        continue;
                    }
                    previous = off;
                    for side in 0..2 {
                        for rot in 0..2 {
                            add(off, side, rot);
                        }
                    }
                }
            }
        }
    }
    result
}

fn build_board(N: usize, free_rows: &[u64; MAX_N]) -> BoardData {
    let mut board = BoardData {
        prefix: [[0; MAX_N + 1]; MAX_N + 1],
        runs: [[0; MAX_LEN + 1]; MAX_N],
    };
    for x in 0..N {
        let mut row_sum = 0_i32;
        for y in 0..N {
            row_sum += ((free_rows[x] >> y) & 1) as i32;
            board.prefix[x + 1][y + 1] = board.prefix[x][y + 1] + row_sum;
        }
        board.runs[x][0] = !0;
        board.runs[x][1] = free_rows[x];
        for len in 2..=MAX_LEN {
            board.runs[x][len] =
                board.runs[x][len - 1] & (free_rows[x] >> (len - 1));
        }
    }
    board
}

fn rectangle_count(
    prefix: &[[i32; MAX_N + 1]; MAX_N + 1],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
) -> usize {
    (prefix[x1][y1] - prefix[x0][y1] - prefix[x1][y0] + prefix[x0][y0]) as usize
}

fn old_is_free(
    N: usize,
    board: &BoardData,
    spec: CompactSpec,
    x0: usize,
    y0: usize,
) -> bool {
    let (height, width) = dimensions(spec);
    if x0 + height > N || y0 + width > N {
        return false;
    }
    if spec.rem == 0 {
        return rectangle_count(&board.prefix, x0, y0, x0 + height, y0 + width)
            == spec.a * spec.full;
    }
    if spec.rot == 0 {
        let base_y = y0 + usize::from(spec.side == 0);
        if rectangle_count(
            &board.prefix,
            x0,
            base_y,
            x0 + spec.a,
            base_y + spec.full,
        ) != spec.a * spec.full
        {
            return false;
        }
        let partial_y = y0 + if spec.side != 0 { spec.full } else { 0 };
        rectangle_count(
            &board.prefix,
            x0 + spec.off,
            partial_y,
            x0 + spec.off + spec.rem,
            partial_y + 1,
        ) == spec.rem
    } else {
        let base_x = x0 + usize::from(spec.side == 0);
        if rectangle_count(
            &board.prefix,
            base_x,
            y0,
            base_x + spec.full,
            y0 + spec.a,
        ) != spec.a * spec.full
        {
            return false;
        }
        let partial_x = x0 + if spec.side != 0 { spec.full } else { 0 };
        rectangle_count(
            &board.prefix,
            partial_x,
            y0 + spec.off,
            partial_x + 1,
            y0 + spec.off + spec.rem,
        ) == spec.rem
    }
}

fn old_valid_y_mask(
    N: usize,
    board: &BoardData,
    spec: CompactSpec,
    x0: usize,
) -> u64 {
    let (_, width) = dimensions(spec);
    let mut mask = 0_u64;
    for y0 in 0..=N - width {
        if old_is_free(N, board, spec, x0, y0) {
            mask |= 1_u64 << y0;
        }
    }
    mask
}

fn new_valid_y_mask(
    N: usize,
    board: &BoardData,
    spec: CompactSpec,
    x0: usize,
) -> u64 {
    let (height, width) = dimensions(spec);
    let valid_y = (1_u64 << (N - width + 1)) - 1;
    let mut ys = valid_y;
    if spec.rem == 0 {
        for row_offset in 0..height {
            ys &= board.runs[x0 + row_offset][width];
            if ys == 0 {
                break;
            }
        }
    } else if spec.rot == 0 {
        for row_offset in 0..spec.a {
            let partial = spec.off <= row_offset && row_offset < spec.off + spec.rem;
            let len = spec.full + usize::from(partial);
            let left = usize::from(spec.side == 0 && !partial);
            ys &= board.runs[x0 + row_offset][len] >> left;
            if ys == 0 {
                break;
            }
        }
    } else {
        let partial_row = if spec.side != 0 { spec.full } else { 0 };
        for row_offset in 0..height {
            let partial = row_offset == partial_row;
            let len = if partial { spec.rem } else { spec.a };
            let left = if partial { spec.off } else { 0 };
            ys &= board.runs[x0 + row_offset][len] >> left;
            if ys == 0 {
                break;
            }
        }
    }
    ys & valid_y
}

fn make_free_rows(N: usize, case_index: usize, rng: &mut XorShift64) -> [u64; MAX_N] {
    let mut rows = [0_u64; MAX_N];
    let valid = (1_u64 << N) - 1;
    match case_index {
        0 => rows[..N].fill(valid),
        1 => {}
        2 => {
            for (x, row) in rows[..N].iter_mut().enumerate() {
                *row = if x & 1 == 0 { 0x5555_5555_5555_5555 } else { 0xAAAA_AAAA_AAAA_AAAA } & valid;
            }
        }
        _ => {
            let threshold = [12_u64, 28, 45, 62, 80, 94][(case_index - 3) % 6];
            for row in &mut rows[..N] {
                let mut mask = 0_u64;
                for y in 0..N {
                    if rng.next_u64() % 100 < threshold {
                        mask |= 1_u64 << y;
                    }
                }
                *row = mask;
            }
        }
    }
    rows
}

fn scan_checksum(
    N: usize,
    board: &BoardData,
    specs: &[CompactSpec],
    use_bitset: bool,
) -> (u64, usize) {
    let mut checksum = 0x1234_5678_9ABC_DEF0_u64;
    let mut positions = 0_usize;
    for (spec_index, &spec) in specs.iter().enumerate() {
        let (height, _) = dimensions(spec);
        for x0 in 0..=N - height {
            let mut ys = if use_bitset {
                new_valid_y_mask(N, board, spec, x0)
            } else {
                old_valid_y_mask(N, board, spec, x0)
            };
            while ys != 0 {
                let y0 = ys.trailing_zeros() as usize;
                ys &= ys - 1;
                positions += 1;
                checksum = checksum
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add(((spec_index * MAX_N + x0) * MAX_N + y0) as u64);
            }
        }
    }
    (checksum, positions)
}

fn main() {
    let mut rng = XorShift64::new(0xC0DE_9069_5EED);
    let mut mask_checks = 0_usize;
    let mut valid_positions = 0_usize;
    let mut spec_total = 0_usize;
    for N in [8_usize, 13, 25, 37, 50] {
        let specs = generate_specs(N);
        spec_total += specs.len();
        for case_index in 0..11 {
            let rows = make_free_rows(N, case_index, &mut rng);
            let board = build_board(N, &rows);
            for &spec in &specs {
                let (height, _) = dimensions(spec);
                for x0 in 0..=N - height {
                    let old = old_valid_y_mask(N, &board, spec, x0);
                    let new = new_valid_y_mask(N, &board, spec, x0);
                    assert_eq!(old, new, "N={N} spec={spec:?} x={x0}");
                    mask_checks += 1;
                    valid_positions += old.count_ones() as usize;
                }
            }
        }
    }

    let N = 50;
    let specs = generate_specs(N);
    let mut boards = Vec::new();
    for case_index in 3..15 {
        let rows = make_free_rows(N, case_index, &mut rng);
        boards.push(build_board(N, &rows));
    }
    let old_begin = Instant::now();
    let mut old_result = (0_u64, 0_usize);
    for board in &boards {
        let result = scan_checksum(N, black_box(board), black_box(&specs), false);
        old_result.0 ^= result.0;
        old_result.1 += result.1;
    }
    let old_elapsed = old_begin.elapsed();
    let new_begin = Instant::now();
    let mut new_result = (0_u64, 0_usize);
    for board in &boards {
        let result = scan_checksum(N, black_box(board), black_box(&specs), true);
        new_result.0 ^= result.0;
        new_result.1 += result.1;
    }
    let new_elapsed = new_begin.elapsed();
    assert_eq!(old_result, new_result);

    println!(
        "verified specs={} masks={} valid_positions={}",
        spec_total, mask_checks, valid_positions
    );
    println!(
        "bench old_ms={:.3} new_ms={:.3} speedup={:.3} positions={}",
        old_elapsed.as_secs_f64() * 1_000.0,
        new_elapsed.as_secs_f64() * 1_000.0,
        old_elapsed.as_secs_f64() / new_elapsed.as_secs_f64(),
        old_result.1
    );
}
