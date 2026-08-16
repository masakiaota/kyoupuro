// bench_cell_set.rs
#![allow(non_snake_case)]

use std::env;
use std::fs;
use std::hint::black_box;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const N: usize = 50;
const M: usize = 1_000;
const CELL_COUNT: usize = N * N;
const WORD_BITS: usize = u64::BITS as usize;
const WORD_COUNT: usize = CELL_COUNT.div_ceil(WORD_BITS);
const ROW_MASK: u64 = (1_u64 << N) - 1;
const NO_OWNER: u16 = u16::MAX;
const BENCH_ROUNDS: usize = 9;
const QUERY_COUNT: usize = 1 << 18;
const SET_PAIR_COUNT: usize = 1 << 12;
const CANDIDATE_COUNT: usize = 1 << 10;
const BOARD_SAMPLE_STRIDE: usize = 131;
const REGION_SAMPLE_STRIDE: usize = 97;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CellSet {
    words: [u64; WORD_COUNT],
}

impl Default for CellSet {
    fn default() -> Self {
        Self {
            words: [0; WORD_COUNT],
        }
    }
}

impl CellSet {
    #[inline]
    fn contains_id(&self, id: usize) -> bool {
        self.words[id >> 6] & (1_u64 << (id & 63)) != 0
    }

    #[inline]
    fn insert_id(&mut self, id: usize) {
        self.words[id >> 6] |= 1_u64 << (id & 63);
    }

    #[inline]
    fn remove_id(&mut self, id: usize) {
        self.words[id >> 6] &= !(1_u64 << (id & 63));
    }

    #[inline]
    fn row_bits(&self, x: usize) -> u64 {
        let start = x * N;
        let word_index = start >> 6;
        let offset = start & 63;
        let mut row = self.words[word_index] >> offset;
        if offset + N > WORD_BITS {
            row |= self.words[word_index + 1] << (WORD_BITS - offset);
        }
        row & ROW_MASK
    }

    #[inline]
    fn is_disjoint(&self, other: &Self) -> bool {
        self.words
            .iter()
            .zip(&other.words)
            .all(|(&a, &b)| a & b == 0)
    }

    #[inline]
    fn union_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] |= other.words[k];
        }
    }

    #[inline]
    fn difference_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] &= !other.words[k];
        }
    }

    #[inline]
    fn intersection_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] &= other.words[k];
        }
    }

    #[inline]
    fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoolSet {
    cells: [bool; CELL_COUNT],
}

impl Default for BoolSet {
    fn default() -> Self {
        Self {
            cells: [false; CELL_COUNT],
        }
    }
}

impl BoolSet {
    fn from_cell_set(set: &CellSet) -> Self {
        let mut result = Self::default();
        for id in 0..CELL_COUNT {
            result.cells[id] = set.contains_id(id);
        }
        result
    }

    #[inline]
    fn contains_id(&self, id: usize) -> bool {
        self.cells[id]
    }

    #[inline]
    fn insert_id(&mut self, id: usize) {
        self.cells[id] = true;
    }

    #[inline]
    fn remove_id(&mut self, id: usize) {
        self.cells[id] = false;
    }

    #[inline]
    fn row_bits(&self, x: usize) -> u64 {
        let mut result = 0_u64;
        let start = x * N;
        for y in 0..N {
            result |= (self.cells[start + y] as u64) << y;
        }
        result
    }

    #[inline]
    fn is_disjoint(&self, other: &Self) -> bool {
        self.cells.iter().zip(&other.cells).all(|(&a, &b)| !a || !b)
    }

    #[inline]
    fn union_with(&mut self, other: &Self) {
        for id in 0..CELL_COUNT {
            self.cells[id] |= other.cells[id];
        }
    }

    #[inline]
    fn difference_with(&mut self, other: &Self) {
        for id in 0..CELL_COUNT {
            self.cells[id] &= !other.cells[id];
        }
    }

    #[inline]
    fn intersection_with(&mut self, other: &Self) {
        for id in 0..CELL_COUNT {
            self.cells[id] &= other.cells[id];
        }
    }

    #[inline]
    fn count(&self) -> usize {
        self.cells.iter().map(|&cell| cell as usize).sum()
    }
}

#[derive(Clone)]
struct SetPair {
    bits: CellSet,
    bools: BoolSet,
}

impl SetPair {
    fn new(bits: CellSet) -> Self {
        let bools = BoolSet::from_cell_set(&bits);
        Self { bits, bools }
    }
}

#[derive(Clone)]
struct CandidateSample {
    board: SetPair,
    region: SetPair,
}

struct Samples {
    grass: SetPair,
    boards: Vec<SetPair>,
    regions: Vec<SetPair>,
    candidates: Vec<CandidateSample>,
    queries: Vec<(u16, u16)>,
    set_pairs: Vec<(u16, u16)>,
    total_turns: usize,
    total_regions: usize,
    total_region_cells: usize,
}

#[derive(Clone)]
struct BitStatePayload {
    occupied: CellSet,
    owner: [u16; CELL_COUNT],
    active_index_by_group: [u16; M],
    current_X: i64,
}

#[derive(Clone)]
struct BoolStatePayload {
    occupied: BoolSet,
    owner: [u16; CELL_COUNT],
    active_index_by_group: [u16; M],
    current_X: i64,
}

struct SmallRng(u64);

impl SmallRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.0 = x;
        x
    }

    #[inline]
    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn input_paths(input_dir: &Path, case_limit: usize) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(input_dir)
        .expect("read input directory")
        .map(|entry| entry.expect("read input entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(case_limit);
    paths
}

fn occupancy_in_frame(frame: &tools::Frame) -> CellSet {
    let mut result = CellSet::default();
    for x in 0..N {
        for y in 0..N {
            if frame.grid[x][y] != usize::MAX {
                result.insert_id(x * N + y);
            }
        }
    }
    result
}

fn region_in_frame(frame: &tools::Frame, owner: usize) -> CellSet {
    let mut result = CellSet::default();
    for x in 0..N {
        for y in 0..N {
            if frame.grid[x][y] == owner {
                result.insert_id(x * N + y);
            }
        }
    }
    result
}

fn grass_set(input: &tools::Input) -> CellSet {
    let mut result = CellSet::default();
    for x in 0..N {
        for y in 0..N {
            if input.grass[x][y] {
                result.insert_id(x * N + y);
            }
        }
    }
    result
}

fn load_samples(input_dir: &Path, output_dir: &Path, case_limit: usize) -> Samples {
    let mut grass = None;
    let mut boards = Vec::new();
    let mut regions = Vec::new();
    let mut total_turns = 0_usize;
    let mut total_regions = 0_usize;
    let mut total_region_cells = 0_usize;

    for input_path in input_paths(input_dir, case_limit) {
        let output_path = output_dir.join(input_path.file_name().expect("input basename"));
        let input_text = fs::read_to_string(&input_path).expect("read input");
        let output_text = fs::read_to_string(&output_path).expect("read output");
        let input = tools::parse_input(&input_text);
        let output = tools::parse_output(&input, &output_text);
        assert!(
            output.error.is_none(),
            "{}: {:?}",
            output_path.display(),
            output.error
        );
        if grass.is_none() {
            grass = Some(SetPair::new(grass_set(&input)));
        }

        for frame in output.frames.iter().skip(1) {
            if frame.arrival.is_none() {
                continue;
            }
            total_turns += 1;
            if total_turns.is_multiple_of(BOARD_SAMPLE_STRIDE) {
                boards.push(SetPair::new(occupancy_in_frame(frame)));
            }

            let mut placed_ids = frame.moved.clone();
            if let Some(i) = frame.accepted_arrival() {
                placed_ids.push(i);
            }
            for owner in placed_ids {
                total_regions += 1;
                if total_regions.is_multiple_of(REGION_SAMPLE_STRIDE) {
                    let region = region_in_frame(frame, owner);
                    assert_eq!(region.count(), input.groups[owner].p);
                    total_region_cells += region.count();
                    regions.push(SetPair::new(region));
                }
            }
        }
    }

    assert!(!boards.is_empty());
    assert!(!regions.is_empty());
    let mut rng = SmallRng::new(0x50a1_50c0_ffee_1234);
    let mut candidates = Vec::with_capacity(CANDIDATE_COUNT);
    for i in 0..CANDIDATE_COUNT {
        let region = regions[rng.usize(regions.len())].clone();
        let source = &boards[rng.usize(boards.len())];
        let mut board_bits = source.bits.clone();
        if i % 2 == 0 {
            board_bits.difference_with(&region.bits);
        } else {
            board_bits.union_with(&region.bits);
        }
        candidates.push(CandidateSample {
            board: SetPair::new(board_bits),
            region,
        });
    }

    let queries = (0..QUERY_COUNT)
        .map(|_| (rng.usize(boards.len()) as u16, rng.usize(CELL_COUNT) as u16))
        .collect::<Vec<_>>();
    let set_pairs = (0..SET_PAIR_COUNT)
        .map(|_| {
            (
                rng.usize(regions.len()) as u16,
                rng.usize(regions.len()) as u16,
            )
        })
        .collect::<Vec<_>>();

    Samples {
        grass: grass.unwrap(),
        boards,
        regions,
        candidates,
        queries,
        set_pairs,
        total_turns,
        total_regions,
        total_region_cells,
    }
}

fn assert_same(bits: &CellSet, bools: &BoolSet) {
    assert_eq!(bits.count(), bools.count());
    for x in 0..N {
        assert_eq!(bits.row_bits(x), bools.row_bits(x));
    }
    for id in 0..CELL_COUNT {
        assert_eq!(bits.contains_id(id), bools.contains_id(id));
    }
}

fn perimeter_bits(region: &CellSet) -> usize {
    let P = region.count();
    let mut adjacent = 0;
    let mut previous = 0_u64;
    for x in 0..N {
        let row = region.row_bits(x);
        adjacent += (row & (row >> 1)).count_ones() as usize;
        adjacent += (row & previous).count_ones() as usize;
        previous = row;
    }
    4 * P - 2 * adjacent
}

fn perimeter_bools(region: &BoolSet) -> usize {
    let mut P = 0;
    let mut adjacent = 0;
    for x in 0..N {
        for y in 0..N {
            let id = x * N + y;
            if !region.cells[id] {
                continue;
            }
            P += 1;
            if y > 0 && region.cells[id - 1] {
                adjacent += 1;
            }
            if x > 0 && region.cells[id - N] {
                adjacent += 1;
            }
        }
    }
    4 * P - 2 * adjacent
}

fn run_correctness_checks(samples: &Samples) {
    assert_same(&samples.grass.bits, &samples.grass.bools);
    for sample in samples.boards.iter().chain(&samples.regions) {
        assert_same(&sample.bits, &sample.bools);
    }
    for candidate in &samples.candidates {
        assert_same(&candidate.board.bits, &candidate.board.bools);
        assert_same(&candidate.region.bits, &candidate.region.bools);
        assert_eq!(
            candidate.board.bits.is_disjoint(&candidate.region.bits),
            candidate.board.bools.is_disjoint(&candidate.region.bools)
        );
    }
    for region in &samples.regions {
        assert_eq!(perimeter_bits(&region.bits), perimeter_bools(&region.bools));
    }

    for &(a, b) in samples.set_pairs.iter().take(512) {
        let a = a as usize;
        let b = b as usize;
        let mut bit_value = samples.regions[a].bits.clone();
        let mut bool_value = samples.regions[a].bools.clone();
        bit_value.union_with(&samples.regions[b].bits);
        bool_value.union_with(&samples.regions[b].bools);
        assert_same(&bit_value, &bool_value);
        bit_value = samples.regions[a].bits.clone();
        bool_value = samples.regions[a].bools.clone();
        bit_value.difference_with(&samples.regions[b].bits);
        bool_value.difference_with(&samples.regions[b].bools);
        assert_same(&bit_value, &bool_value);
        bit_value = samples.regions[a].bits.clone();
        bool_value = samples.regions[a].bools.clone();
        bit_value.intersection_with(&samples.regions[b].bits);
        bool_value.intersection_with(&samples.regions[b].bools);
        assert_same(&bit_value, &bool_value);
    }

    let mut rng = SmallRng::new(0x1234_5678_9abc_def0);
    let mut bits = CellSet::default();
    let mut bools = BoolSet::default();
    for step in 0..100_000 {
        let id = rng.usize(CELL_COUNT);
        if rng.next_u64() & 1 == 0 {
            bits.insert_id(id);
            bools.insert_id(id);
        } else {
            bits.remove_id(id);
            bools.remove_id(id);
        }
        if step % 1_000 == 0 {
            assert_same(&bits, &bools);
        }
    }
    assert_same(&bits, &bools);
    println!("[check] all CellSet/BoolSet operations agree");
}

fn contains_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(set, id) in &samples.queries {
        checksum += samples.boards[set as usize].bits.contains_id(id as usize) as u64;
    }
    black_box(checksum)
}

fn contains_hot_bits(samples: &Samples) -> u64 {
    let board = &samples.boards[0].bits;
    let mut checksum = 0_u64;
    for &(_, id) in &samples.queries {
        checksum += board.contains_id(id as usize) as u64;
    }
    black_box(checksum)
}

fn contains_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(set, id) in &samples.queries {
        checksum += samples.boards[set as usize].bools.contains_id(id as usize) as u64;
    }
    black_box(checksum)
}

fn contains_hot_bools(samples: &Samples) -> u64 {
    let board = &samples.boards[0].bools;
    let mut checksum = 0_u64;
    for &(_, id) in &samples.queries {
        checksum += board.contains_id(id as usize) as u64;
    }
    black_box(checksum)
}

fn is_free_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(set, id) in &samples.queries {
        let id = id as usize;
        checksum += (samples.grass.bits.contains_id(id)
            && !samples.boards[set as usize].bits.contains_id(id)) as u64;
    }
    black_box(checksum)
}

fn is_free_hot_bits(samples: &Samples) -> u64 {
    let occupied = &samples.boards[0].bits;
    let mut checksum = 0_u64;
    for &(_, id) in &samples.queries {
        let id = id as usize;
        checksum += (samples.grass.bits.contains_id(id) && !occupied.contains_id(id)) as u64;
    }
    black_box(checksum)
}

fn is_free_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(set, id) in &samples.queries {
        let id = id as usize;
        checksum += (samples.grass.bools.contains_id(id)
            && !samples.boards[set as usize].bools.contains_id(id)) as u64;
    }
    black_box(checksum)
}

fn is_free_hot_bools(samples: &Samples) -> u64 {
    let occupied = &samples.boards[0].bools;
    let mut checksum = 0_u64;
    for &(_, id) in &samples.queries {
        let id = id as usize;
        checksum += (samples.grass.bools.contains_id(id) && !occupied.contains_id(id)) as u64;
    }
    black_box(checksum)
}

fn toggle_bits(queries: &[(u16, u16)]) -> u64 {
    let mut value = CellSet::default();
    let mut checksum = 0_u64;
    for &(set, id) in queries {
        let id = id as usize;
        if value.contains_id(id) {
            value.remove_id(id);
        } else {
            value.insert_id(id);
        }
        checksum =
            checksum.wrapping_add(value.contains_id((id + set as usize + 1) % CELL_COUNT) as u64);
    }
    black_box(checksum + value.count() as u64)
}

fn toggle_bools(queries: &[(u16, u16)]) -> u64 {
    let mut value = BoolSet::default();
    let mut checksum = 0_u64;
    for &(set, id) in queries {
        let id = id as usize;
        if value.contains_id(id) {
            value.remove_id(id);
        } else {
            value.insert_id(id);
        }
        checksum =
            checksum.wrapping_add(value.contains_id((id + set as usize + 1) % CELL_COUNT) as u64);
    }
    black_box(checksum + value.count() as u64)
}

fn disjoint_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        checksum += samples.regions[a as usize]
            .bits
            .is_disjoint(&samples.regions[b as usize].bits) as u64;
    }
    black_box(checksum)
}

fn disjoint_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        checksum += samples.regions[a as usize]
            .bools
            .is_disjoint(&samples.regions[b as usize].bools) as u64;
    }
    black_box(checksum)
}

fn union_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bits.clone();
        value.union_with(&samples.regions[b as usize].bits);
        black_box(&value);
        checksum ^= value.words[(a as usize + b as usize) % WORD_COUNT];
    }
    black_box(checksum)
}

fn union_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bools.clone();
        value.union_with(&samples.regions[b as usize].bools);
        black_box(&value);
        checksum ^= value.cells[(a as usize * 31 + b as usize) % CELL_COUNT] as u64;
    }
    black_box(checksum)
}

fn difference_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bits.clone();
        value.difference_with(&samples.regions[b as usize].bits);
        black_box(&value);
        checksum ^= value.words[(a as usize + b as usize) % WORD_COUNT];
    }
    black_box(checksum)
}

fn difference_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bools.clone();
        value.difference_with(&samples.regions[b as usize].bools);
        black_box(&value);
        checksum ^= value.cells[(a as usize * 31 + b as usize) % CELL_COUNT] as u64;
    }
    black_box(checksum)
}

fn intersection_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bits.clone();
        value.intersection_with(&samples.regions[b as usize].bits);
        black_box(&value);
        checksum ^= value.words[(a as usize + b as usize) % WORD_COUNT];
    }
    black_box(checksum)
}

fn intersection_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for &(a, b) in &samples.set_pairs {
        let mut value = samples.regions[a as usize].bools.clone();
        value.intersection_with(&samples.regions[b as usize].bools);
        black_box(&value);
        checksum ^= value.cells[(a as usize * 31 + b as usize) % CELL_COUNT] as u64;
    }
    black_box(checksum)
}

fn count_bits(samples: &Samples) -> u64 {
    black_box(
        samples
            .boards
            .iter()
            .map(|sample| sample.bits.count() as u64)
            .sum(),
    )
}

fn count_bools(samples: &Samples) -> u64 {
    black_box(
        samples
            .boards
            .iter()
            .map(|sample| sample.bools.count() as u64)
            .sum(),
    )
}

fn row_bits_bits(samples: &Samples) -> u64 {
    let mut checksum = 0;
    for sample in &samples.regions {
        for x in 0..N {
            checksum ^= sample.bits.row_bits(x).rotate_left(x as u32);
        }
    }
    black_box(checksum)
}

fn row_bits_bools(samples: &Samples) -> u64 {
    let mut checksum = 0;
    for sample in &samples.regions {
        for x in 0..N {
            checksum ^= sample.bools.row_bits(x).rotate_left(x as u32);
        }
    }
    black_box(checksum)
}

fn compactness_bits(samples: &Samples) -> u64 {
    black_box(
        samples
            .regions
            .iter()
            .map(|sample| perimeter_bits(&sample.bits) as u64)
            .sum(),
    )
}

fn compactness_bools(samples: &Samples) -> u64 {
    black_box(
        samples
            .regions
            .iter()
            .map(|sample| perimeter_bools(&sample.bools) as u64)
            .sum(),
    )
}

fn iterate_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for sample in &samples.regions {
        for word_index in 0..WORD_COUNT {
            let mut bits = sample.bits.words[word_index];
            while bits != 0 {
                checksum = checksum
                    .wrapping_add((word_index * WORD_BITS + bits.trailing_zeros() as usize) as u64);
                bits &= bits - 1;
            }
        }
    }
    black_box(checksum)
}

fn iterate_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for sample in &samples.regions {
        for (id, &present) in sample.bools.cells.iter().enumerate() {
            if present {
                checksum = checksum.wrapping_add(id as u64);
            }
        }
    }
    black_box(checksum)
}

fn place_regions_bits(samples: &Samples) -> u64 {
    let mut occupied = CellSet::default();
    let mut owner = [NO_OWNER; CELL_COUNT];
    for (i, sample) in samples.regions.iter().enumerate() {
        for word_index in 0..WORD_COUNT {
            let mut bits = sample.bits.words[word_index];
            while bits != 0 {
                let id = word_index * WORD_BITS + bits.trailing_zeros() as usize;
                owner[id] = (i % M) as u16;
                bits &= bits - 1;
            }
        }
        occupied.union_with(&sample.bits);
    }
    black_box(owner[occupied.count() % CELL_COUNT] as u64 + occupied.count() as u64)
}

fn place_regions_bools(samples: &Samples) -> u64 {
    let mut occupied = BoolSet::default();
    let mut owner = [NO_OWNER; CELL_COUNT];
    for (i, sample) in samples.regions.iter().enumerate() {
        for id in 0..CELL_COUNT {
            if sample.bools.cells[id] {
                owner[id] = (i % M) as u16;
            }
        }
        occupied.union_with(&sample.bools);
    }
    black_box(owner[occupied.count() % CELL_COUNT] as u64 + occupied.count() as u64)
}

fn mixed_candidate_bits(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for (index, sample) in samples.candidates.iter().enumerate() {
        let mut child = sample.board.bits.clone();
        let free = child.is_disjoint(&sample.region.bits);
        if free {
            child.union_with(&sample.region.bits);
        }
        checksum = checksum
            .wrapping_add(child.count() as u64)
            .wrapping_add(child.row_bits(index % N))
            .wrapping_add(free as u64);
        black_box(&child);
    }
    black_box(checksum)
}

fn mixed_candidate_bools(samples: &Samples) -> u64 {
    let mut checksum = 0_u64;
    for (index, sample) in samples.candidates.iter().enumerate() {
        let mut child = sample.board.bools.clone();
        let free = child.is_disjoint(&sample.region.bools);
        if free {
            child.union_with(&sample.region.bools);
        }
        checksum = checksum
            .wrapping_add(child.count() as u64)
            .wrapping_add(child.row_bits(index % N))
            .wrapping_add(free as u64);
        black_box(&child);
    }
    black_box(checksum)
}

fn clone_cell_sets(samples: &Samples, sink: &mut Vec<CellSet>) -> u64 {
    sink.clear();
    sink.extend(samples.boards.iter().map(|sample| sample.bits.clone()));
    black_box(&*sink);
    black_box(sink.len() as u64 + sink.last().unwrap().words[0])
}

fn clone_bool_sets(samples: &Samples, sink: &mut Vec<BoolSet>) -> u64 {
    sink.clear();
    sink.extend(samples.boards.iter().map(|sample| sample.bools.clone()));
    black_box(&*sink);
    black_box(sink.len() as u64 + sink.last().unwrap().cells[0] as u64)
}

fn build_state_payloads(samples: &Samples) -> (Vec<BitStatePayload>, Vec<BoolStatePayload>) {
    let mut bits = Vec::with_capacity(samples.boards.len());
    let mut bools = Vec::with_capacity(samples.boards.len());
    for (sample_index, board) in samples.boards.iter().enumerate() {
        let mut owner = [NO_OWNER; CELL_COUNT];
        for (id, slot) in owner.iter_mut().enumerate() {
            if board.bits.contains_id(id) {
                *slot = (id % M) as u16;
            }
        }
        let mut active_index_by_group = [NO_OWNER; M];
        for (i, slot) in active_index_by_group.iter_mut().enumerate() {
            *slot = ((i + sample_index) % M) as u16;
        }
        bits.push(BitStatePayload {
            occupied: board.bits.clone(),
            owner,
            active_index_by_group,
            current_X: sample_index as i64,
        });
        bools.push(BoolStatePayload {
            occupied: board.bools.clone(),
            owner,
            active_index_by_group,
            current_X: sample_index as i64,
        });
    }
    (bits, bools)
}

fn clone_bit_states(samples: &[BitStatePayload], sink: &mut Vec<BitStatePayload>) -> u64 {
    sink.clear();
    sink.extend_from_slice(samples);
    black_box(&*sink);
    let last = sink.last().unwrap();
    black_box(
        sink.len() as u64
            + last.occupied.words[0]
            + last.owner[0] as u64
            + last.active_index_by_group[0] as u64
            + last.current_X as u64,
    )
}

fn clone_bool_states(samples: &[BoolStatePayload], sink: &mut Vec<BoolStatePayload>) -> u64 {
    sink.clear();
    sink.extend_from_slice(samples);
    black_box(&*sink);
    let last = sink.last().unwrap();
    black_box(
        sink.len() as u64
            + last.occupied.cells[0] as u64
            + last.owner[0] as u64
            + last.active_index_by_group[0] as u64
            + last.current_X as u64,
    )
}

fn calibrated_repeats<F>(f: &mut F) -> usize
where
    F: FnMut() -> u64,
{
    let start = Instant::now();
    black_box(f());
    let elapsed = start.elapsed().max(Duration::from_nanos(1));
    let target = Duration::from_millis(40);
    ((target.as_nanos() / elapsed.as_nanos())
        .max(1)
        .min(1_000_000)) as usize
}

fn measure<F>(f: &mut F, repeats: usize) -> Duration
where
    F: FnMut() -> u64,
{
    let start = Instant::now();
    let mut checksum = 0_u64;
    for _ in 0..repeats {
        checksum ^= f();
    }
    black_box(checksum);
    start.elapsed().div_f64(repeats as f64)
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn benchmark_pair<FB, FD>(label: &str, operations: usize, mut bits: FB, mut bools: FD)
where
    FB: FnMut() -> u64,
    FD: FnMut() -> u64,
{
    let bit_repeats = calibrated_repeats(&mut bits);
    let bool_repeats = calibrated_repeats(&mut bools);
    let mut bit_samples = Vec::with_capacity(BENCH_ROUNDS);
    let mut bool_samples = Vec::with_capacity(BENCH_ROUNDS);
    for round in 0..BENCH_ROUNDS {
        if round % 2 == 0 {
            bit_samples.push(measure(&mut bits, bit_repeats));
            bool_samples.push(measure(&mut bools, bool_repeats));
        } else {
            bool_samples.push(measure(&mut bools, bool_repeats));
            bit_samples.push(measure(&mut bits, bit_repeats));
        }
    }
    let bit_time = median(&mut bit_samples);
    let bool_time = median(&mut bool_samples);
    let bit_ns = bit_time.as_secs_f64() * 1e9 / operations as f64;
    let bool_ns = bool_time.as_secs_f64() * 1e9 / operations as f64;
    println!("[bench] {label}");
    println!("  CellSet {:10.3} ns/op", bit_ns);
    println!("  bool[]  {:10.3} ns/op", bool_ns);
    println!(
        "  bool/CellSet = {:.3}x ({})",
        bool_ns / bit_ns,
        if bit_ns <= bool_ns {
            "CellSet faster"
        } else {
            "bool[] faster"
        }
    );
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let input_dir = Path::new(args.get(1).map_or("tools/in", String::as_str));
    let output_dir = Path::new(
        args.get(2)
            .map_or("results/out/v053_posterior_rollout", String::as_str),
    );
    let case_limit = args.get(3).map_or(100, |arg| {
        arg.parse::<usize>().expect("case limit must be usize")
    });

    println!("[load] input={}", input_dir.display());
    println!("[load] output={}", output_dir.display());
    println!("[load] case_limit={case_limit}");
    let samples = load_samples(input_dir, output_dir, case_limit);
    run_correctness_checks(&samples);

    let average_region_cells = samples
        .regions
        .iter()
        .map(|sample| sample.bits.count())
        .sum::<usize>() as f64
        / samples.regions.len() as f64;
    let disjoint_candidates = samples
        .candidates
        .iter()
        .filter(|sample| sample.board.bits.is_disjoint(&sample.region.bits))
        .count();
    println!("[sample] source_turns={}", samples.total_turns);
    println!("[sample] source_regions={}", samples.total_regions);
    println!("[sample] boards={}", samples.boards.len());
    println!("[sample] regions={}", samples.regions.len());
    println!("[sample] avg_region_cells={average_region_cells:.3}");
    println!(
        "[sample] candidate_disjoint={}/{}",
        disjoint_candidates,
        samples.candidates.len()
    );
    println!(
        "[sample] sampled_region_cells_during_load={}",
        samples.total_region_cells
    );
    println!("[size] CellSet={} bytes", size_of::<CellSet>());
    println!("[size] BoolSet={} bytes", size_of::<BoolSet>());
    println!(
        "[size] V000 fixed payload CellSet={} bytes BoolSet={} bytes",
        size_of::<BitStatePayload>(),
        size_of::<BoolStatePayload>()
    );

    benchmark_pair(
        "contains on one hot board",
        samples.queries.len(),
        || contains_hot_bits(&samples),
        || contains_hot_bools(&samples),
    );
    benchmark_pair(
        "random contains across many State samples",
        samples.queries.len(),
        || contains_bits(&samples),
        || contains_bools(&samples),
    );
    benchmark_pair(
        "is_free on one hot board",
        samples.queries.len(),
        || is_free_hot_bits(&samples),
        || is_free_hot_bools(&samples),
    );
    benchmark_pair(
        "is_free across many State samples",
        samples.queries.len(),
        || is_free_bits(&samples),
        || is_free_bools(&samples),
    );
    benchmark_pair(
        "single-cell contains + toggle",
        samples.queries.len(),
        || toggle_bits(&samples.queries),
        || toggle_bools(&samples.queries),
    );
    benchmark_pair(
        "is_disjoint on real regions",
        samples.set_pairs.len(),
        || disjoint_bits(&samples),
        || disjoint_bools(&samples),
    );
    benchmark_pair(
        "clone + union",
        samples.set_pairs.len(),
        || union_bits(&samples),
        || union_bools(&samples),
    );
    benchmark_pair(
        "clone + difference",
        samples.set_pairs.len(),
        || difference_bits(&samples),
        || difference_bools(&samples),
    );
    benchmark_pair(
        "clone + intersection",
        samples.set_pairs.len(),
        || intersection_bits(&samples),
        || intersection_bools(&samples),
    );
    benchmark_pair(
        "count occupied cells",
        samples.boards.len(),
        || count_bits(&samples),
        || count_bools(&samples),
    );
    benchmark_pair(
        "row_bits extraction",
        samples.regions.len() * N,
        || row_bits_bits(&samples),
        || row_bits_bools(&samples),
    );
    benchmark_pair(
        "region perimeter/compactness preparation",
        samples.regions.len(),
        || compactness_bits(&samples),
        || compactness_bools(&samples),
    );
    benchmark_pair(
        "iterate present cells for output/owner update",
        samples.regions.len(),
        || iterate_bits(&samples),
        || iterate_bools(&samples),
    );
    benchmark_pair(
        "place regions into owner + occupied",
        samples.regions.len(),
        || place_regions_bits(&samples),
        || place_regions_bools(&samples),
    );
    benchmark_pair(
        "V000-like clone + disjoint + conditional union + count + row_bits",
        samples.candidates.len(),
        || mixed_candidate_bits(&samples),
        || mixed_candidate_bools(&samples),
    );

    let mut bit_sink = Vec::with_capacity(samples.boards.len());
    let mut bool_sink = Vec::with_capacity(samples.boards.len());
    benchmark_pair(
        "CellSet/BoolSet clone",
        samples.boards.len(),
        || clone_cell_sets(&samples, &mut bit_sink),
        || clone_bool_sets(&samples, &mut bool_sink),
    );

    let (bit_states, bool_states) = build_state_payloads(&samples);
    let mut bit_state_sink = Vec::with_capacity(bit_states.len());
    let mut bool_state_sink = Vec::with_capacity(bool_states.len());
    benchmark_pair(
        "V000 fixed State payload clone",
        samples.boards.len(),
        || clone_bit_states(&bit_states, &mut bit_state_sink),
        || clone_bool_states(&bool_states, &mut bool_state_sink),
    );
}
