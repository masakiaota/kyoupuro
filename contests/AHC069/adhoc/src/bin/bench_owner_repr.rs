// bench_owner_repr.rs
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
const GROUP_WORD_COUNT: usize = M.div_ceil(WORD_BITS);
const OWNER_BITS: usize = 10;
const NO_OWNER: u16 = u16::MAX;
const SAMPLE_TURN_STRIDE: usize = 197;
const SAMPLE_PLACEMENT_STRIDE: usize = 67;
const BENCH_ROUNDS: usize = 9;

type GroupSet = [u64; GROUP_WORD_COUNT];

#[derive(Clone, PartialEq, Eq)]
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
    fn insert_id(&mut self, id: usize) {
        self.words[id >> 6] |= 1_u64 << (id & 63);
    }

    #[inline]
    fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    #[inline]
    fn nonzero_word_count(&self) -> usize {
        self.words.iter().filter(|&&word| word != 0).count()
    }
}

#[derive(Clone)]
struct DenseBoard {
    occupied: CellSet,
    owner: [u16; CELL_COUNT],
}

impl DenseBoard {
    fn new() -> Self {
        Self {
            occupied: CellSet::default(),
            owner: [NO_OWNER; CELL_COUNT],
        }
    }
}

#[derive(Clone)]
struct BitSlicedBoard {
    occupied: CellSet,
    // 論理的には10個のCellSetだが、cloneを10回に分割させないため連続配列にする。
    owner_bits: OwnerPlanes,
}

#[derive(Clone, Copy)]
struct OwnerPlanes {
    words: [[u64; WORD_COUNT]; OWNER_BITS],
}

impl Default for OwnerPlanes {
    fn default() -> Self {
        Self {
            words: [[0; WORD_COUNT]; OWNER_BITS],
        }
    }
}

impl BitSlicedBoard {
    fn new() -> Self {
        Self {
            occupied: CellSet::default(),
            owner_bits: OwnerPlanes::default(),
        }
    }

    #[inline]
    fn occupied_owner_at_id(&self, id: usize) -> u16 {
        let word_index = id >> 6;
        let bit = 1_u64 << (id & 63);
        let mut owner = 0_u16;
        for b in 0..OWNER_BITS {
            owner |= (u16::from(self.owner_bits.words[b][word_index] & bit != 0)) << b;
        }
        owner
    }

    fn clear_groups_by_equality(&mut self, groups: &GroupSet) {
        let mut removed = [0_u64; WORD_COUNT];
        for (group_word_index, &group_word) in groups.iter().enumerate() {
            let mut group_bits = group_word;
            while group_bits != 0 {
                let bit_index = group_bits.trailing_zeros() as usize;
                let owner = group_word_index * WORD_BITS + bit_index;
                for (word_index, removed_word) in removed.iter_mut().enumerate() {
                    let mut matched = self.occupied.words[word_index];
                    for b in 0..OWNER_BITS {
                        let plane = self.owner_bits.words[b][word_index];
                        matched &= if owner & (1 << b) != 0 { plane } else { !plane };
                    }
                    *removed_word |= matched;
                }
                group_bits &= group_bits - 1;
            }
        }
        for (occupied, removed) in self.occupied.words.iter_mut().zip(removed) {
            *occupied &= !removed;
        }
    }

    fn clear_groups_by_occupied_scan(&mut self, groups: &GroupSet) {
        for word_index in 0..WORD_COUNT {
            let mut bits = self.occupied.words[word_index];
            let mut removed = 0_u64;
            while bits != 0 {
                let bit_index = bits.trailing_zeros() as usize;
                let id = word_index * WORD_BITS + bit_index;
                let owner = self.occupied_owner_at_id(id) as usize;
                if groups[owner >> 6] & (1_u64 << (owner & 63)) != 0 {
                    removed |= 1_u64 << bit_index;
                }
                bits &= bits - 1;
            }
            self.occupied.words[word_index] &= !removed;
        }
    }
}

trait OwnerBoard: Clone {
    fn new() -> Self;
    fn clear_groups(&mut self, groups: &GroupSet);
    fn clear_all(&mut self);
    fn place(&mut self, owner: u16, region: &CellSet);
    fn owner_at_id(&self, id: usize) -> Option<u16>;
    fn occupied_count(&self) -> usize;
}

impl OwnerBoard for DenseBoard {
    fn new() -> Self {
        Self::new()
    }

    fn clear_groups(&mut self, groups: &GroupSet) {
        // v000_template.rs の現在の実装と同じく、盤面全体を一度走査する。
        for id in 0..CELL_COUNT {
            let owner = self.owner[id];
            if owner == NO_OWNER {
                continue;
            }
            let owner = owner as usize;
            if groups[owner >> 6] & (1_u64 << (owner & 63)) != 0 {
                self.owner[id] = NO_OWNER;
                self.occupied.words[id >> 6] &= !(1_u64 << (id & 63));
            }
        }
    }

    fn clear_all(&mut self) {
        self.owner.fill(NO_OWNER);
        self.occupied.words.fill(0);
    }

    fn place(&mut self, owner: u16, region: &CellSet) {
        // v000_template.rs の現在の実装と同じく、region の立っているbitを列挙する。
        for word_index in 0..WORD_COUNT {
            let mut bits = region.words[word_index];
            while bits != 0 {
                let bit_index = bits.trailing_zeros() as usize;
                self.owner[word_index * WORD_BITS + bit_index] = owner;
                bits &= bits - 1;
            }
            self.occupied.words[word_index] |= region.words[word_index];
        }
    }

    #[inline]
    fn owner_at_id(&self, id: usize) -> Option<u16> {
        let owner = self.owner[id];
        (owner != NO_OWNER).then_some(owner)
    }

    #[inline]
    fn occupied_count(&self) -> usize {
        self.occupied.count()
    }
}

impl OwnerBoard for BitSlicedBoard {
    fn new() -> Self {
        Self::new()
    }

    fn clear_groups(&mut self, groups: &GroupSet) {
        let group_count = groups
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum::<usize>();
        let occupied_count = self.occupied_count();

        // 少数groupなら10枚のbit planeから一致集合を直接作る。大量退去時は
        // 各occupied cellを一度だけdecodeし、group数に比例する走査を避ける。
        if group_count * WORD_COUNT <= occupied_count {
            self.clear_groups_by_equality(groups);
        } else {
            self.clear_groups_by_occupied_scan(groups);
        }
    }

    fn clear_all(&mut self) {
        // 空きマスのowner bitは参照されず、次の配置時に全10bitを上書きする。
        self.occupied.words.fill(0);
    }

    fn place(&mut self, owner: u16, region: &CellSet) {
        for word_index in 0..WORD_COUNT {
            let mask = region.words[word_index];
            if mask == 0 {
                continue;
            }
            for b in 0..OWNER_BITS {
                if owner & (1 << b) != 0 {
                    self.owner_bits.words[b][word_index] |= mask;
                } else {
                    self.owner_bits.words[b][word_index] &= !mask;
                }
            }
            self.occupied.words[word_index] |= mask;
        }
    }

    #[inline]
    fn owner_at_id(&self, id: usize) -> Option<u16> {
        if self.occupied.words[id >> 6] & (1_u64 << (id & 63)) == 0 {
            return None;
        }
        Some(self.occupied_owner_at_id(id))
    }

    #[inline]
    fn occupied_count(&self) -> usize {
        self.occupied.count()
    }
}

#[derive(Clone)]
struct GroupMeta {
    i: usize,
    compactness: f64,
    min_compactness: f64,
}

#[derive(Clone)]
struct DenseSearchState {
    board: DenseBoard,
    active_groups: Vec<GroupMeta>,
    active_index_by_group: [u16; M],
    current_X: i64,
}

#[derive(Clone)]
struct BitSlicedSearchState {
    board: BitSlicedBoard,
    active_groups: Vec<GroupMeta>,
    active_index_by_group: [u16; M],
    current_X: i64,
}

#[derive(Clone)]
struct Placement {
    owner: u16,
    region: CellSet,
}

struct TurnTrace {
    departed: GroupSet,
    moved: GroupSet,
    placements: Vec<Placement>,
    final_clear: bool,
}

struct CaseTrace {
    turns: Vec<TurnTrace>,
}

struct ReadCloneSample {
    dense: DenseSearchState,
    bit_sliced: BitSlicedSearchState,
    boundary_queries: Vec<u16>,
}

struct CandidateSample {
    dense: DenseSearchState,
    bit_sliced: BitSlicedSearchState,
    placement: Placement,
}

#[derive(Default)]
struct TraceStats {
    cases: usize,
    turns: usize,
    placements: usize,
    placed_cells: usize,
    placement_words: usize,
    departed_groups: usize,
    moved_groups: usize,
    active_group_sum: usize,
}

fn group_set(ids: impl IntoIterator<Item = usize>) -> GroupSet {
    let mut result = [0_u64; GROUP_WORD_COUNT];
    for id in ids {
        result[id >> 6] |= 1_u64 << (id & 63);
    }
    result
}

#[inline]
fn group_set_is_empty(groups: &GroupSet) -> bool {
    groups.iter().all(|&word| word == 0)
}

fn clear_expected(expected: &mut [u16; CELL_COUNT], groups: &GroupSet) {
    for owner in expected {
        if *owner == NO_OWNER {
            continue;
        }
        let i = *owner as usize;
        if groups[i >> 6] & (1_u64 << (i & 63)) != 0 {
            *owner = NO_OWNER;
        }
    }
}

fn region_in_frame(frame: &tools::Frame, owner: usize) -> CellSet {
    let mut region = CellSet::default();
    for x in 0..N {
        for y in 0..N {
            if frame.grid[x][y] == owner {
                region.insert_id(x * N + y);
            }
        }
    }
    region
}

fn boundary_queries(input: &tools::Input, expected: &[u16; CELL_COUNT]) -> Vec<u16> {
    let mut result = Vec::new();
    for x in 0..N {
        for y in 0..N {
            let id = x * N + y;
            if !input.grass[x][y] || expected[id] != NO_OWNER {
                continue;
            }
            if x > 0 && expected[id - N] != NO_OWNER {
                result.push((id - N) as u16);
            }
            if x + 1 < N && expected[id + N] != NO_OWNER {
                result.push((id + N) as u16);
            }
            if y > 0 && expected[id - 1] != NO_OWNER {
                result.push((id - 1) as u16);
            }
            if y + 1 < N && expected[id + 1] != NO_OWNER {
                result.push((id + 1) as u16);
            }
        }
    }
    result
}

fn common_state(expected: &[u16; CELL_COUNT]) -> (Vec<GroupMeta>, [u16; M]) {
    let mut present = [false; M];
    for &owner in expected {
        if owner != NO_OWNER {
            present[owner as usize] = true;
        }
    }

    let mut active_groups = Vec::new();
    let mut active_index_by_group = [u16::MAX; M];
    for (i, &is_present) in present.iter().enumerate() {
        if !is_present {
            continue;
        }
        active_index_by_group[i] = active_groups.len() as u16;
        active_groups.push(GroupMeta {
            i,
            compactness: 1.0,
            min_compactness: 1.0,
        });
    }
    (active_groups, active_index_by_group)
}

fn make_search_states(
    dense: &DenseBoard,
    bit_sliced: &BitSlicedBoard,
    expected: &[u16; CELL_COUNT],
) -> (DenseSearchState, BitSlicedSearchState) {
    let (active_groups, active_index_by_group) = common_state(expected);
    (
        DenseSearchState {
            board: dense.clone(),
            active_groups: active_groups.clone(),
            active_index_by_group,
            current_X: 0,
        },
        BitSlicedSearchState {
            board: bit_sliced.clone(),
            active_groups,
            active_index_by_group,
            current_X: 0,
        },
    )
}

fn verify_board<B: OwnerBoard>(board: &B, expected: &[u16; CELL_COUNT]) {
    for (id, &owner) in expected.iter().enumerate() {
        let expected_owner = (owner != NO_OWNER).then_some(owner);
        assert_eq!(board.owner_at_id(id), expected_owner, "cell {id}");
    }
}

fn input_paths(input_dir: &Path, case_limit: usize) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(input_dir)
        .expect("read input dir")
        .map(|entry| entry.expect("read input entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    paths.sort();
    paths.truncate(case_limit);
    paths
}

fn load_traces(
    input_dir: &Path,
    output_dir: &Path,
    case_limit: usize,
) -> (
    Vec<CaseTrace>,
    Vec<ReadCloneSample>,
    Vec<CandidateSample>,
    TraceStats,
) {
    let mut cases = Vec::new();
    let mut read_samples = Vec::new();
    let mut candidate_samples = Vec::new();
    let mut stats = TraceStats::default();
    let mut global_turn = 0_usize;
    let mut global_placement = 0_usize;

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

        let mut expected = [NO_OWNER; CELL_COUNT];
        let mut dense = DenseBoard::new();
        let mut bit_sliced = BitSlicedBoard::new();
        let mut turns = Vec::with_capacity(output.frames.len() - 1);

        for (frame_index, frame) in output.frames.iter().enumerate().skip(1) {
            if frame.arrival.is_none() {
                dense.clear_all();
                bit_sliced.clear_all();
                expected.fill(NO_OWNER);
                turns.push(TurnTrace {
                    departed: [0; GROUP_WORD_COUNT],
                    moved: [0; GROUP_WORD_COUNT],
                    placements: Vec::new(),
                    final_clear: true,
                });
                verify_board(&dense, &expected);
                verify_board(&bit_sliced, &expected);
                continue;
            }

            global_turn += 1;
            stats.turns += 1;
            stats.active_group_sum += frame.actives.len();

            let departed = group_set(frame.departed.iter().map(|&(i, _)| i));
            stats.departed_groups += frame.departed.len();
            if !group_set_is_empty(&departed) {
                dense.clear_groups(&departed);
                bit_sliced.clear_groups(&departed);
                clear_expected(&mut expected, &departed);
            }

            if global_turn.is_multiple_of(SAMPLE_TURN_STRIDE) {
                let queries = boundary_queries(&input, &expected);
                let (dense_state, bit_sliced_state) =
                    make_search_states(&dense, &bit_sliced, &expected);
                read_samples.push(ReadCloneSample {
                    dense: dense_state,
                    bit_sliced: bit_sliced_state,
                    boundary_queries: queries,
                });
            }

            let moved = group_set(frame.moved.iter().copied());
            stats.moved_groups += frame.moved.len();
            if !group_set_is_empty(&moved) {
                dense.clear_groups(&moved);
                bit_sliced.clear_groups(&moved);
                clear_expected(&mut expected, &moved);
            }

            let mut placed_ids = frame.moved.clone();
            if let Some(i) = frame.accepted_arrival() {
                placed_ids.push(i);
            }
            let mut placements = Vec::with_capacity(placed_ids.len());
            for owner in placed_ids {
                let region = region_in_frame(frame, owner);
                assert_eq!(region.count(), input.groups[owner].p);
                let placement = Placement {
                    owner: owner as u16,
                    region,
                };

                global_placement += 1;
                if global_placement.is_multiple_of(SAMPLE_PLACEMENT_STRIDE) {
                    let (dense_state, bit_sliced_state) =
                        make_search_states(&dense, &bit_sliced, &expected);
                    candidate_samples.push(CandidateSample {
                        dense: dense_state,
                        bit_sliced: bit_sliced_state,
                        placement: placement.clone(),
                    });
                }

                dense.place(placement.owner, &placement.region);
                bit_sliced.place(placement.owner, &placement.region);
                for word_index in 0..WORD_COUNT {
                    let mut bits = placement.region.words[word_index];
                    while bits != 0 {
                        let bit_index = bits.trailing_zeros() as usize;
                        expected[word_index * WORD_BITS + bit_index] = placement.owner;
                        bits &= bits - 1;
                    }
                }

                stats.placements += 1;
                stats.placed_cells += placement.region.count();
                stats.placement_words += placement.region.nonzero_word_count();
                placements.push(placement);
            }

            if frame_index % 100 == 0 {
                verify_board(&dense, &expected);
                verify_board(&bit_sliced, &expected);
                for id in 0..CELL_COUNT {
                    let frame_owner = frame.grid[id / N][id % N];
                    let expected_owner = if frame_owner == usize::MAX {
                        NO_OWNER
                    } else {
                        frame_owner as u16
                    };
                    assert_eq!(expected[id], expected_owner);
                }
            }

            turns.push(TurnTrace {
                departed,
                moved,
                placements,
                final_clear: false,
            });
        }

        stats.cases += 1;
        cases.push(CaseTrace { turns });
    }

    assert!(!read_samples.is_empty());
    assert!(!candidate_samples.is_empty());
    (cases, read_samples, candidate_samples, stats)
}

fn replay_updates<B: OwnerBoard>(cases: &[CaseTrace]) -> u64 {
    let mut checksum = 0_u64;
    for case in cases {
        let mut board = B::new();
        for turn in &case.turns {
            if turn.final_clear {
                board.clear_all();
                continue;
            }
            if !group_set_is_empty(&turn.departed) {
                board.clear_groups(&turn.departed);
            }
            if !group_set_is_empty(&turn.moved) {
                board.clear_groups(&turn.moved);
            }
            for placement in &turn.placements {
                board.place(placement.owner, &placement.region);
            }
        }
        checksum = checksum
            .wrapping_mul(1_000_003)
            .wrapping_add(board.occupied_count() as u64);
    }
    black_box(checksum)
}

fn replay_placements<B: OwnerBoard>(cases: &[CaseTrace]) -> u64 {
    let mut checksum = 0_u64;
    for case in cases {
        let mut board = B::new();
        for turn in &case.turns {
            for placement in &turn.placements {
                board.place(placement.owner, &placement.region);
            }
        }
        checksum = checksum
            .wrapping_mul(1_000_003)
            .wrapping_add(board.occupied_count() as u64);
    }
    black_box(checksum)
}

fn read_boundary_dense(samples: &[ReadCloneSample], group_values: &[u64]) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for &id in &sample.boundary_queries {
            let owner = sample.dense.board.owner_at_id(id as usize).unwrap() as usize;
            checksum = checksum.wrapping_add(group_values[owner]);
        }
    }
    black_box(checksum)
}

fn read_boundary_bit_sliced(samples: &[ReadCloneSample], group_values: &[u64]) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for &id in &sample.boundary_queries {
            let owner = sample.bit_sliced.board.owner_at_id(id as usize).unwrap() as usize;
            checksum = checksum.wrapping_add(group_values[owner]);
        }
    }
    black_box(checksum)
}

fn read_sequential_dense(samples: &[ReadCloneSample], group_values: &[u64]) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for id in 0..CELL_COUNT {
            if let Some(owner) = sample.dense.board.owner_at_id(id) {
                checksum = checksum.wrapping_add(group_values[owner as usize]);
            }
        }
    }
    black_box(checksum)
}

fn read_sequential_bit_sliced(samples: &[ReadCloneSample], group_values: &[u64]) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for id in 0..CELL_COUNT {
            if let Some(owner) = sample.bit_sliced.board.owner_at_id(id) {
                checksum = checksum.wrapping_add(group_values[owner as usize]);
            }
        }
    }
    black_box(checksum)
}

fn clone_board_dense(samples: &[ReadCloneSample], sink: &mut Vec<DenseBoard>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.dense.board.clone()));
    let checksum = sink
        .iter()
        .fold(0_u64, |acc, board| acc + board.occupied_count() as u64);
    black_box(&*sink);
    black_box(checksum)
}

fn clone_board_bit_sliced(samples: &[ReadCloneSample], sink: &mut Vec<BitSlicedBoard>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.bit_sliced.board.clone()));
    let checksum = sink
        .iter()
        .fold(0_u64, |acc, board| acc + board.occupied_count() as u64);
    black_box(&*sink);
    black_box(checksum)
}

fn clone_state_dense(samples: &[ReadCloneSample], sink: &mut Vec<DenseSearchState>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.dense.clone()));
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        let group_checksum = state.active_groups.first().map_or(0, |group| {
            group.i as u64 + group.compactness.to_bits() + group.min_compactness.to_bits()
        });
        acc.wrapping_add(state.board.occupied_count() as u64)
            .wrapping_add(state.active_groups.len() as u64)
            .wrapping_add(state.active_index_by_group[0] as u64)
            .wrapping_add(state.current_X as u64)
            .wrapping_add(group_checksum)
    });
    black_box(&*sink);
    black_box(checksum)
}

fn clone_state_bit_sliced(
    samples: &[ReadCloneSample],
    sink: &mut Vec<BitSlicedSearchState>,
) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.bit_sliced.clone()));
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        let group_checksum = state.active_groups.first().map_or(0, |group| {
            group.i as u64 + group.compactness.to_bits() + group.min_compactness.to_bits()
        });
        acc.wrapping_add(state.board.occupied_count() as u64)
            .wrapping_add(state.active_groups.len() as u64)
            .wrapping_add(state.active_index_by_group[0] as u64)
            .wrapping_add(state.current_X as u64)
            .wrapping_add(group_checksum)
    });
    black_box(&*sink);
    black_box(checksum)
}

fn clone_place_board_dense(samples: &[CandidateSample], sink: &mut Vec<DenseBoard>) -> u64 {
    sink.clear();
    for sample in samples {
        let mut board = sample.dense.board.clone();
        board.place(sample.placement.owner, &sample.placement.region);
        sink.push(board);
    }
    let checksum = sink
        .iter()
        .fold(0_u64, |acc, board| acc + board.occupied_count() as u64);
    black_box(&*sink);
    black_box(checksum)
}

fn clone_place_board_bit_sliced(
    samples: &[CandidateSample],
    sink: &mut Vec<BitSlicedBoard>,
) -> u64 {
    sink.clear();
    for sample in samples {
        let mut board = sample.bit_sliced.board.clone();
        board.place(sample.placement.owner, &sample.placement.region);
        sink.push(board);
    }
    let checksum = sink
        .iter()
        .fold(0_u64, |acc, board| acc + board.occupied_count() as u64);
    black_box(&*sink);
    black_box(checksum)
}

fn clone_place_dense(samples: &[CandidateSample], sink: &mut Vec<DenseSearchState>) -> u64 {
    sink.clear();
    for sample in samples {
        let mut state = sample.dense.clone();
        state
            .board
            .place(sample.placement.owner, &sample.placement.region);
        sink.push(state);
    }
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        acc.wrapping_add(state.board.occupied_count() as u64)
    });
    black_box(&*sink);
    black_box(checksum)
}

fn clone_place_bit_sliced(
    samples: &[CandidateSample],
    sink: &mut Vec<BitSlicedSearchState>,
) -> u64 {
    sink.clear();
    for sample in samples {
        let mut state = sample.bit_sliced.clone();
        state
            .board
            .place(sample.placement.owner, &sample.placement.region);
        sink.push(state);
    }
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        acc.wrapping_add(state.board.occupied_count() as u64)
    });
    black_box(&*sink);
    black_box(checksum)
}

#[derive(Clone, Copy)]
struct Measurement {
    median: Duration,
    min: Duration,
    max: Duration,
}

fn median_duration(values: &mut [Duration]) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn calibrated_repeats<F>(f: &mut F) -> usize
where
    F: FnMut() -> u64,
{
    black_box(f());
    let start = Instant::now();
    black_box(f());
    let elapsed = start.elapsed().max(Duration::from_nanos(1));
    let target = Duration::from_millis(80);
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

fn benchmark_pair<FD, FB>(label: &str, operations: usize, mut dense: FD, mut bit_sliced: FB)
where
    FD: FnMut() -> u64,
    FB: FnMut() -> u64,
{
    let dense_repeats = calibrated_repeats(&mut dense);
    let bit_repeats = calibrated_repeats(&mut bit_sliced);
    let mut dense_samples = Vec::with_capacity(BENCH_ROUNDS);
    let mut bit_samples = Vec::with_capacity(BENCH_ROUNDS);

    for round in 0..BENCH_ROUNDS {
        if round % 2 == 0 {
            dense_samples.push(measure(&mut dense, dense_repeats));
            bit_samples.push(measure(&mut bit_sliced, bit_repeats));
        } else {
            bit_samples.push(measure(&mut bit_sliced, bit_repeats));
            dense_samples.push(measure(&mut dense, dense_repeats));
        }
    }

    let dense_min = *dense_samples.iter().min().unwrap();
    let dense_max = *dense_samples.iter().max().unwrap();
    let bit_min = *bit_samples.iter().min().unwrap();
    let bit_max = *bit_samples.iter().max().unwrap();
    let dense_result = Measurement {
        median: median_duration(&mut dense_samples),
        min: dense_min,
        max: dense_max,
    };
    let bit_result = Measurement {
        median: median_duration(&mut bit_samples),
        min: bit_min,
        max: bit_max,
    };

    let dense_ns = dense_result.median.as_secs_f64() * 1e9 / operations as f64;
    let bit_ns = bit_result.median.as_secs_f64() * 1e9 / operations as f64;
    let speedup = dense_ns / bit_ns;
    println!("[bench] {label}");
    println!(
        "  dense      {:9.3} ns/op  median={:8.3} ms  range={:.3}..{:.3} ms",
        dense_ns,
        dense_result.median.as_secs_f64() * 1e3,
        dense_result.min.as_secs_f64() * 1e3,
        dense_result.max.as_secs_f64() * 1e3,
    );
    println!(
        "  bit-sliced {:9.3} ns/op  median={:8.3} ms  range={:.3}..{:.3} ms",
        bit_ns,
        bit_result.median.as_secs_f64() * 1e3,
        bit_result.min.as_secs_f64() * 1e3,
        bit_result.max.as_secs_f64() * 1e3,
    );
    println!("  dense/bit-sliced = {speedup:.3}x");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_dir = Path::new(args.get(1).map_or("tools/in", String::as_str));
    let output_dir = Path::new(
        args.get(2)
            .map_or("results/out/v052_adaptive_capacity", String::as_str),
    );
    let case_limit = args.get(3).map_or(100, |s| s.parse().expect("case limit"));

    println!("[load] input={}", input_dir.display());
    println!("[load] output={}", output_dir.display());
    println!("[load] case_limit={case_limit}");
    let (cases, read_samples, candidate_samples, stats) =
        load_traces(input_dir, output_dir, case_limit);

    let boundary_query_count = read_samples
        .iter()
        .map(|sample| sample.boundary_queries.len())
        .sum::<usize>();
    let group_values = (0..M)
        .map(|i| (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .collect::<Vec<_>>();
    println!("[trace] cases={}", stats.cases);
    println!("[trace] turns={}", stats.turns);
    println!("[trace] placements={}", stats.placements);
    println!("[trace] departed_groups={}", stats.departed_groups);
    println!("[trace] moved_groups={}", stats.moved_groups);
    println!(
        "[trace] avg_cells_per_placement={:.3}",
        stats.placed_cells as f64 / stats.placements as f64
    );
    println!(
        "[trace] avg_words_per_placement={:.3}",
        stats.placement_words as f64 / stats.placements as f64
    );
    println!(
        "[trace] avg_active_groups={:.3}",
        stats.active_group_sum as f64 / stats.turns as f64
    );
    println!("[trace] read_clone_samples={}", read_samples.len());
    println!("[trace] boundary_queries={boundary_query_count}");
    println!("[trace] candidate_samples={}", candidate_samples.len());

    println!("[size] CellSet={} bytes", size_of::<CellSet>());
    println!(
        "[size] dense owner payload={} bytes",
        size_of::<[u16; CELL_COUNT]>()
    );
    println!(
        "[size] bit-sliced owner payload={} bytes",
        size_of::<OwnerPlanes>()
    );
    println!("[size] DenseBoard={} bytes", size_of::<DenseBoard>());
    println!(
        "[size] BitSlicedBoard={} bytes",
        size_of::<BitSlicedBoard>()
    );
    println!(
        "[size] DenseSearchState={} bytes + active_groups heap",
        size_of::<DenseSearchState>()
    );
    println!(
        "[size] BitSlicedSearchState={} bytes + active_groups heap",
        size_of::<BitSlicedSearchState>()
    );

    benchmark_pair(
        "exact v052 board transitions",
        stats.turns,
        || replay_updates::<DenseBoard>(&cases),
        || replay_updates::<BitSlicedBoard>(&cases),
    );

    benchmark_pair(
        "compact placement only",
        stats.placements,
        || replay_placements::<DenseBoard>(&cases),
        || replay_placements::<BitSlicedBoard>(&cases),
    );

    benchmark_pair(
        "occupied-neighbor owner_at + group metadata",
        boundary_query_count,
        || read_boundary_dense(&read_samples, &group_values),
        || read_boundary_bit_sliced(&read_samples, &group_values),
    );

    benchmark_pair(
        "sequential full-board owner_at + group metadata",
        read_samples.len() * CELL_COUNT,
        || read_sequential_dense(&read_samples, &group_values),
        || read_sequential_bit_sliced(&read_samples, &group_values),
    );

    let mut dense_board_sink = Vec::with_capacity(read_samples.len());
    let mut bit_board_sink = Vec::with_capacity(read_samples.len());
    benchmark_pair(
        "board clone",
        read_samples.len(),
        || clone_board_dense(&read_samples, &mut dense_board_sink),
        || clone_board_bit_sliced(&read_samples, &mut bit_board_sink),
    );

    let mut dense_state_sink = Vec::with_capacity(read_samples.len());
    let mut bit_state_sink = Vec::with_capacity(read_samples.len());
    benchmark_pair(
        "V000-like State clone",
        read_samples.len(),
        || clone_state_dense(&read_samples, &mut dense_state_sink),
        || clone_state_bit_sliced(&read_samples, &mut bit_state_sink),
    );

    let mut dense_board_candidate_sink = Vec::with_capacity(candidate_samples.len());
    let mut bit_board_candidate_sink = Vec::with_capacity(candidate_samples.len());
    benchmark_pair(
        "board clone + compact placement",
        candidate_samples.len(),
        || clone_place_board_dense(&candidate_samples, &mut dense_board_candidate_sink),
        || clone_place_board_bit_sliced(&candidate_samples, &mut bit_board_candidate_sink),
    );

    let mut dense_candidate_sink = Vec::with_capacity(candidate_samples.len());
    let mut bit_candidate_sink = Vec::with_capacity(candidate_samples.len());
    benchmark_pair(
        "V000-like State clone + compact placement",
        candidate_samples.len(),
        || clone_place_dense(&candidate_samples, &mut dense_candidate_sink),
        || clone_place_bit_sliced(&candidate_samples, &mut bit_candidate_sink),
    );
}
