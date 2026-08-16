// bench_persistent_2d_owner.rs
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
const NO_OWNER: u16 = u16::MAX;
const NULL_NODE: u32 = 0;
const BENCH_ROUNDS: usize = 9;
const SAMPLE_CASE_COUNT: usize = 5;
const SAMPLE_TURN_STRIDE: usize = 10;
const SAMPLE_PLACEMENT_STRIDE: usize = 7;

type GroupSet = [u64; GROUP_WORD_COUNT];
type NodeId = u32;

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
    fn row_bits(&self, x: usize) -> u64 {
        let start = x * N;
        let word_index = start >> 6;
        let offset = start & 63;
        let mut row = self.words[word_index] >> offset;
        if offset + N > WORD_BITS {
            row |= self.words[word_index + 1] << (WORD_BITS - offset);
        }
        row & ((1_u64 << N) - 1)
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
    fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x0: u8,
    x1: u8,
    y0: u8,
    y1: u8,
}

impl Rect {
    fn new(x0: usize, x1: usize, y0: usize, y1: usize) -> Self {
        Self {
            x0: x0 as u8,
            x1: x1 as u8,
            y0: y0 as u8,
            y1: y1 as u8,
        }
    }
}

/// 各行の同一水平runを上下へ連結し、重ならない長方形列へ正確に分解する。
fn decompose_rectangles(region: &CellSet) -> Vec<Rect> {
    let mut result = Vec::new();
    let mut active: Vec<Rect> = Vec::new();

    for x in 0..N {
        let mut bits = region.row_bits(x);
        let mut runs = Vec::new();
        while bits != 0 {
            let y0 = bits.trailing_zeros() as usize;
            let shifted = bits >> y0;
            let len = shifted.trailing_ones() as usize;
            let y1 = y0 + len;
            runs.push((y0, y1));
            let run_mask = ((1_u64 << len) - 1) << y0;
            bits &= !run_mask;
        }

        let mut next = Vec::with_capacity(runs.len());
        for (y0, y1) in runs {
            if let Some(index) = active
                .iter()
                .position(|rect| rect.y0 as usize == y0 && rect.y1 as usize == y1)
            {
                let mut rect = active.swap_remove(index);
                rect.x1 = (x + 1) as u8;
                next.push(rect);
            } else {
                next.push(Rect::new(x, x + 1, y0, y1));
            }
        }
        result.extend(active);
        active = next;
    }
    result.extend(active);
    result
}

fn rectangles_to_cell_set(rectangles: &[Rect]) -> CellSet {
    let mut result = CellSet::default();
    for rect in rectangles {
        for x in rect.x0 as usize..rect.x1 as usize {
            for y in rect.y0 as usize..rect.y1 as usize {
                assert!(!result.contains_id(x * N + y));
                result.insert_id(x * N + y);
            }
        }
    }
    result
}

#[derive(Clone, Copy, Default)]
struct OwnerTag {
    stamp: u32,
    owner: u16,
}

impl OwnerTag {
    #[inline]
    fn later(self, other: Self) -> Self {
        if other.stamp > self.stamp {
            other
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, Default)]
struct XNode {
    left: NodeId,
    right: NodeId,
    y_root: NodeId,
}

#[derive(Clone, Copy, Default)]
struct YNode {
    left: NodeId,
    right: NodeId,
    tag: OwnerTag,
}

#[derive(Clone, Copy)]
struct ArenaCheckpoint {
    x_len: usize,
    y_len: usize,
    next_stamp: u32,
}

struct PersistentOwner2D {
    x_nodes: Vec<XNode>,
    y_nodes: Vec<YNode>,
    next_stamp: u32,
}

impl PersistentOwner2D {
    fn new() -> Self {
        Self::with_capacity(1, 1)
    }

    fn with_capacity(x_capacity: usize, y_capacity: usize) -> Self {
        let mut x_nodes = Vec::with_capacity(x_capacity.max(1));
        let mut y_nodes = Vec::with_capacity(y_capacity.max(1));
        x_nodes.push(XNode::default());
        y_nodes.push(YNode::default());
        Self {
            x_nodes,
            y_nodes,
            next_stamp: 1,
        }
    }

    fn reset(&mut self) {
        self.x_nodes.clear();
        self.y_nodes.clear();
        self.x_nodes.push(XNode::default());
        self.y_nodes.push(YNode::default());
        self.next_stamp = 1;
    }

    #[inline]
    fn checkpoint(&self) -> ArenaCheckpoint {
        ArenaCheckpoint {
            x_len: self.x_nodes.len(),
            y_len: self.y_nodes.len(),
            next_stamp: self.next_stamp,
        }
    }

    fn rollback(&mut self, checkpoint: ArenaCheckpoint) {
        self.x_nodes.truncate(checkpoint.x_len);
        self.y_nodes.truncate(checkpoint.y_len);
        self.next_stamp = checkpoint.next_stamp;
    }

    #[inline]
    fn push_x(&mut self, node: XNode) -> NodeId {
        let id = self.x_nodes.len() as NodeId;
        self.x_nodes.push(node);
        id
    }

    #[inline]
    fn push_y(&mut self, node: YNode) -> NodeId {
        let id = self.y_nodes.len() as NodeId;
        self.y_nodes.push(node);
        id
    }

    fn assign_rectangles(&mut self, mut root: NodeId, rectangles: &[Rect], owner: u16) -> NodeId {
        if rectangles.is_empty() {
            return root;
        }
        let tag = OwnerTag {
            stamp: self.next_stamp,
            owner,
        };
        self.next_stamp += 1;
        for rect in rectangles {
            root = self.update_x(root, 0, N, *rect, tag);
        }
        root
    }

    fn update_x(
        &mut self,
        root: NodeId,
        left: usize,
        right: usize,
        rect: Rect,
        tag: OwnerTag,
    ) -> NodeId {
        let mut node = self.x_nodes[root as usize];
        let ql = rect.x0 as usize;
        let qr = rect.x1 as usize;
        if ql <= left && right <= qr {
            node.y_root = self.update_y(node.y_root, 0, N, rect.y0 as usize, rect.y1 as usize, tag);
            return self.push_x(node);
        }

        let middle = (left + right) / 2;
        if ql < middle {
            node.left = self.update_x(node.left, left, middle, rect, tag);
        }
        if middle < qr {
            node.right = self.update_x(node.right, middle, right, rect, tag);
        }
        self.push_x(node)
    }

    fn update_y(
        &mut self,
        root: NodeId,
        left: usize,
        right: usize,
        ql: usize,
        qr: usize,
        tag: OwnerTag,
    ) -> NodeId {
        let mut node = self.y_nodes[root as usize];
        if ql <= left && right <= qr {
            node.tag = tag;
            return self.push_y(node);
        }

        let middle = (left + right) / 2;
        if ql < middle {
            node.left = self.update_y(node.left, left, middle, ql, qr, tag);
        }
        if middle < qr {
            node.right = self.update_y(node.right, middle, right, ql, qr, tag);
        }
        self.push_y(node)
    }

    #[inline]
    fn owner_at(&self, root: NodeId, x: usize, y: usize) -> Option<u16> {
        let mut best = OwnerTag::default();
        let mut x_root = root;
        let mut left = 0;
        let mut right = N;

        loop {
            if x_root == NULL_NODE {
                break;
            }
            let node = self.x_nodes[x_root as usize];
            best = best.later(self.query_y(node.y_root, y));
            if right - left == 1 {
                break;
            }
            let middle = (left + right) / 2;
            if x < middle {
                x_root = node.left;
                right = middle;
            } else {
                x_root = node.right;
                left = middle;
            }
        }

        (best.stamp != 0).then_some(best.owner)
    }

    #[inline]
    fn query_y(&self, mut root: NodeId, y: usize) -> OwnerTag {
        let mut best = OwnerTag::default();
        let mut left = 0;
        let mut right = N;
        while root != NULL_NODE {
            let node = self.y_nodes[root as usize];
            best = best.later(node.tag);
            if right - left == 1 {
                break;
            }
            let middle = (left + right) / 2;
            if y < middle {
                root = node.left;
                right = middle;
            } else {
                root = node.right;
                left = middle;
            }
        }
        best
    }

    fn bytes_used(&self) -> usize {
        self.x_nodes.len() * size_of::<XNode>() + self.y_nodes.len() * size_of::<YNode>()
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

    fn clear_groups(&mut self, groups: &GroupSet) {
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
        self.occupied.words.fill(0);
        self.owner.fill(NO_OWNER);
    }

    fn place(&mut self, owner: u16, region: &CellSet) {
        for word_index in 0..WORD_COUNT {
            let mut bits = region.words[word_index];
            while bits != 0 {
                let bit_index = bits.trailing_zeros() as usize;
                self.owner[word_index * WORD_BITS + bit_index] = owner;
                bits &= bits - 1;
            }
        }
        self.occupied.union_with(region);
    }

    #[inline]
    fn owner_at_id(&self, id: usize) -> Option<u16> {
        let owner = self.owner[id];
        (owner != NO_OWNER).then_some(owner)
    }
}

#[derive(Clone)]
struct PersistentBoard {
    occupied: CellSet,
    owner_root: NodeId,
}

impl PersistentBoard {
    fn new() -> Self {
        Self {
            occupied: CellSet::default(),
            owner_root: NULL_NODE,
        }
    }

    fn clear_regions(&mut self, regions: &[CellSet]) {
        for region in regions {
            self.occupied.difference_with(region);
        }
    }

    fn clear_all(&mut self) {
        self.occupied.words.fill(0);
    }

    fn place(
        &mut self,
        arena: &mut PersistentOwner2D,
        owner: u16,
        region: &CellSet,
        rectangles: &[Rect],
    ) {
        self.owner_root = arena.assign_rectangles(self.owner_root, rectangles, owner);
        self.occupied.union_with(region);
    }

    #[inline]
    fn owner_at_id(&self, arena: &PersistentOwner2D, id: usize) -> Option<u16> {
        if !self.occupied.contains_id(id) {
            return None;
        }
        arena.owner_at(self.owner_root, id / N, id % N)
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
struct PersistentSearchState {
    board: PersistentBoard,
    active_groups: Vec<GroupMeta>,
    active_index_by_group: [u16; M],
    current_X: i64,
}

#[derive(Clone)]
struct Placement {
    owner: u16,
    region: CellSet,
    rectangles: Vec<Rect>,
}

struct TurnTrace {
    departed: GroupSet,
    departed_regions: Vec<CellSet>,
    moved: GroupSet,
    moved_regions: Vec<CellSet>,
    placements: Vec<Placement>,
    final_clear: bool,
}

struct CaseTrace {
    grass: CellSet,
    turns: Vec<TurnTrace>,
}

struct ReadCloneSample {
    dense: DenseSearchState,
    persistent: PersistentSearchState,
    boundary_queries: Vec<u16>,
}

#[derive(Clone)]
struct CandidateSample {
    dense: DenseSearchState,
    persistent: PersistentSearchState,
    placement: Placement,
}

#[derive(Default)]
struct TraceStats {
    cases: usize,
    turns: usize,
    placements: usize,
    placed_cells: usize,
    rectangle_count: usize,
    departed_groups: usize,
    moved_groups: usize,
    rectangle_counts: Vec<usize>,
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

fn regions_from_expected(expected: &[u16; CELL_COUNT], ids: &[usize]) -> Vec<CellSet> {
    let mut result = vec![CellSet::default(); ids.len()];
    let mut index_by_group = [u16::MAX; M];
    for (index, &id) in ids.iter().enumerate() {
        index_by_group[id] = index as u16;
    }
    for (cell, &owner) in expected.iter().enumerate() {
        if owner == NO_OWNER {
            continue;
        }
        let index = index_by_group[owner as usize];
        if index != u16::MAX {
            result[index as usize].insert_id(cell);
        }
    }
    result
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

fn grass_cell_set(input: &tools::Input) -> CellSet {
    let mut grass = CellSet::default();
    for x in 0..N {
        for y in 0..N {
            if input.grass[x][y] {
                grass.insert_id(x * N + y);
            }
        }
    }
    grass
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
) -> (Vec<CaseTrace>, TraceStats) {
    let mut cases = Vec::new();
    let mut stats = TraceStats::default();

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
        let mut turns = Vec::with_capacity(output.frames.len() - 1);
        for (frame_index, frame) in output.frames.iter().enumerate().skip(1) {
            if frame.arrival.is_none() {
                expected.fill(NO_OWNER);
                turns.push(TurnTrace {
                    departed: [0; GROUP_WORD_COUNT],
                    departed_regions: Vec::new(),
                    moved: [0; GROUP_WORD_COUNT],
                    moved_regions: Vec::new(),
                    placements: Vec::new(),
                    final_clear: true,
                });
                continue;
            }

            stats.turns += 1;
            let departed_ids = frame.departed.iter().map(|&(i, _)| i).collect::<Vec<_>>();
            let departed_regions = regions_from_expected(&expected, &departed_ids);
            let departed = group_set(departed_ids.iter().copied());
            stats.departed_groups += departed_ids.len();
            clear_expected(&mut expected, &departed);

            let moved_ids = frame.moved.clone();
            let moved_regions = regions_from_expected(&expected, &moved_ids);
            let moved = group_set(moved_ids.iter().copied());
            stats.moved_groups += moved_ids.len();
            clear_expected(&mut expected, &moved);

            let mut placed_ids = moved_ids;
            if let Some(i) = frame.accepted_arrival() {
                placed_ids.push(i);
            }
            let mut placements = Vec::with_capacity(placed_ids.len());
            for owner in placed_ids {
                let region = region_in_frame(frame, owner);
                assert_eq!(region.count(), input.groups[owner].p);
                let rectangles = decompose_rectangles(&region);
                assert_eq!(rectangles_to_cell_set(&rectangles), region);
                for word_index in 0..WORD_COUNT {
                    let mut bits = region.words[word_index];
                    while bits != 0 {
                        let bit_index = bits.trailing_zeros() as usize;
                        expected[word_index * WORD_BITS + bit_index] = owner as u16;
                        bits &= bits - 1;
                    }
                }
                stats.placements += 1;
                stats.placed_cells += region.count();
                stats.rectangle_count += rectangles.len();
                stats.rectangle_counts.push(rectangles.len());
                placements.push(Placement {
                    owner: owner as u16,
                    region,
                    rectangles,
                });
            }

            if frame_index % 100 == 0 {
                for id in 0..CELL_COUNT {
                    let frame_owner = frame.grid[id / N][id % N];
                    let frame_owner = if frame_owner == usize::MAX {
                        NO_OWNER
                    } else {
                        frame_owner as u16
                    };
                    assert_eq!(expected[id], frame_owner);
                }
            }

            turns.push(TurnTrace {
                departed,
                departed_regions,
                moved,
                moved_regions,
                placements,
                final_clear: false,
            });
        }
        cases.push(CaseTrace {
            grass: grass_cell_set(&input),
            turns,
        });
        stats.cases += 1;
    }

    (cases, stats)
}

struct SmallRng(u64);

impl SmallRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.0 = x;
        x
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn run_correctness_checks() {
    let mut rng = SmallRng::new(0x1234_5678_9abc_def0);

    for _ in 0..2_000 {
        let mut region = CellSet::default();
        let rectangle_count = 1 + rng.usize(8);
        for _ in 0..rectangle_count {
            let x0 = rng.usize(N);
            let x1 = x0 + 1 + rng.usize(N - x0);
            let y0 = rng.usize(N);
            let y1 = y0 + 1 + rng.usize(N - y0);
            for x in x0..x1 {
                for y in y0..y1 {
                    region.insert_id(x * N + y);
                }
            }
        }
        let decomposed = decompose_rectangles(&region);
        assert_eq!(rectangles_to_cell_set(&decomposed), region);
    }

    let mut arena = PersistentOwner2D::new();
    let mut versions: Vec<(NodeId, Box<[u16; CELL_COUNT]>)> = Vec::new();
    versions.push((NULL_NODE, Box::new([NO_OWNER; CELL_COUNT])));

    for step in 0..4_000 {
        let base_index = rng.usize(versions.len());
        let (base_root, base_dense) = &versions[base_index];
        let mut root = *base_root;
        let mut dense = base_dense.clone();
        let owner = rng.usize(M) as u16;
        let count = 1 + rng.usize(4);
        let mut rectangles = Vec::with_capacity(count);
        for _ in 0..count {
            let x0 = rng.usize(N);
            let x1 = x0 + 1 + rng.usize(N - x0);
            let y0 = rng.usize(N);
            let y1 = y0 + 1 + rng.usize(N - y0);
            rectangles.push(Rect::new(x0, x1, y0, y1));
            for x in x0..x1 {
                for y in y0..y1 {
                    dense[x * N + y] = owner;
                }
            }
        }
        root = arena.assign_rectangles(root, &rectangles, owner);

        for _ in 0..64 {
            let id = rng.usize(CELL_COUNT);
            let expected = (dense[id] != NO_OWNER).then_some(dense[id]);
            assert_eq!(arena.owner_at(root, id / N, id % N), expected);
        }
        if step % 200 == 0 {
            for (id, &owner) in dense.iter().enumerate() {
                let expected = (owner != NO_OWNER).then_some(owner);
                assert_eq!(arena.owner_at(root, id / N, id % N), expected);
            }
        }

        let old_index = rng.usize(versions.len());
        let (old_root, old_dense) = &versions[old_index];
        for _ in 0..16 {
            let id = rng.usize(CELL_COUNT);
            let expected = (old_dense[id] != NO_OWNER).then_some(old_dense[id]);
            assert_eq!(arena.owner_at(*old_root, id / N, id % N), expected);
        }
        versions.push((root, dense));
    }

    println!(
        "[check] random branching/decomposition passed: versions={} x_nodes={} y_nodes={}",
        versions.len(),
        arena.x_nodes.len(),
        arena.y_nodes.len()
    );
}

fn boundary_queries(grass: &CellSet, dense: &DenseBoard) -> Vec<u16> {
    let mut result = Vec::new();
    for x in 0..N {
        for y in 0..N {
            let id = x * N + y;
            if !grass.contains_id(id) || dense.owner[id] != NO_OWNER {
                continue;
            }
            if x > 0 && dense.owner[id - N] != NO_OWNER {
                result.push((id - N) as u16);
            }
            if x + 1 < N && dense.owner[id + N] != NO_OWNER {
                result.push((id + N) as u16);
            }
            if y > 0 && dense.owner[id - 1] != NO_OWNER {
                result.push((id - 1) as u16);
            }
            if y + 1 < N && dense.owner[id + 1] != NO_OWNER {
                result.push((id + 1) as u16);
            }
        }
    }
    result
}

fn common_state(owner: &[u16; CELL_COUNT]) -> (Vec<GroupMeta>, [u16; M]) {
    let mut present = [false; M];
    for &i in owner {
        if i != NO_OWNER {
            present[i as usize] = true;
        }
    }
    let mut active_groups = Vec::new();
    let mut active_index_by_group = [u16::MAX; M];
    for (i, &active) in present.iter().enumerate() {
        if active {
            active_index_by_group[i] = active_groups.len() as u16;
            active_groups.push(GroupMeta {
                i,
                compactness: 1.0,
                min_compactness: 1.0,
            });
        }
    }
    (active_groups, active_index_by_group)
}

fn make_search_states(
    dense: &DenseBoard,
    persistent: &PersistentBoard,
) -> (DenseSearchState, PersistentSearchState) {
    let (active_groups, active_index_by_group) = common_state(&dense.owner);
    (
        DenseSearchState {
            board: dense.clone(),
            active_groups: active_groups.clone(),
            active_index_by_group,
            current_X: 0,
        },
        PersistentSearchState {
            board: persistent.clone(),
            active_groups,
            active_index_by_group,
            current_X: 0,
        },
    )
}

fn verify_boards(dense: &DenseBoard, persistent: &PersistentBoard, arena: &PersistentOwner2D) {
    for id in 0..CELL_COUNT {
        assert_eq!(
            persistent.owner_at_id(arena, id),
            dense.owner_at_id(id),
            "cell {id}"
        );
    }
}

fn build_samples(
    cases: &[CaseTrace],
) -> (
    PersistentOwner2D,
    Vec<ReadCloneSample>,
    Vec<CandidateSample>,
) {
    let mut arena = PersistentOwner2D::with_capacity(1_000_000, 4_000_000);
    let mut read_samples = Vec::new();
    let mut candidate_samples = Vec::new();
    let mut global_turn = 0_usize;
    let mut global_placement = 0_usize;

    for case in cases.iter().take(SAMPLE_CASE_COUNT) {
        let mut dense = DenseBoard::new();
        let mut persistent = PersistentBoard::new();
        for (turn_index, turn) in case.turns.iter().enumerate() {
            if turn.final_clear {
                dense.clear_all();
                persistent.clear_all();
                continue;
            }
            global_turn += 1;
            if !group_set_is_empty(&turn.departed) {
                dense.clear_groups(&turn.departed);
                persistent.clear_regions(&turn.departed_regions);
            }

            if global_turn.is_multiple_of(SAMPLE_TURN_STRIDE) {
                let queries = boundary_queries(&case.grass, &dense);
                let (dense_state, persistent_state) = make_search_states(&dense, &persistent);
                read_samples.push(ReadCloneSample {
                    dense: dense_state,
                    persistent: persistent_state,
                    boundary_queries: queries,
                });
            }

            if !group_set_is_empty(&turn.moved) {
                dense.clear_groups(&turn.moved);
                persistent.clear_regions(&turn.moved_regions);
            }
            for placement in &turn.placements {
                global_placement += 1;
                if global_placement.is_multiple_of(SAMPLE_PLACEMENT_STRIDE) {
                    let (dense_state, persistent_state) = make_search_states(&dense, &persistent);
                    candidate_samples.push(CandidateSample {
                        dense: dense_state,
                        persistent: persistent_state,
                        placement: placement.clone(),
                    });
                }
                dense.place(placement.owner, &placement.region);
                persistent.place(
                    &mut arena,
                    placement.owner,
                    &placement.region,
                    &placement.rectangles,
                );
            }
            if turn_index % 100 == 0 {
                verify_boards(&dense, &persistent, &arena);
            }
        }
        verify_boards(&dense, &persistent, &arena);
    }

    assert!(!read_samples.is_empty());
    assert!(!candidate_samples.is_empty());
    (arena, read_samples, candidate_samples)
}

#[derive(Default)]
struct NodeStats {
    total_x: usize,
    total_y: usize,
    max_x: usize,
    max_y: usize,
    max_bytes: usize,
}

fn persistent_node_stats(cases: &[CaseTrace]) -> NodeStats {
    let mut stats = NodeStats::default();
    let mut arena = PersistentOwner2D::new();
    for case in cases {
        arena.reset();
        let mut board = PersistentBoard::new();
        for turn in &case.turns {
            if turn.final_clear {
                board.clear_all();
                continue;
            }
            board.clear_regions(&turn.departed_regions);
            board.clear_regions(&turn.moved_regions);
            for placement in &turn.placements {
                board.place(
                    &mut arena,
                    placement.owner,
                    &placement.region,
                    &placement.rectangles,
                );
            }
        }
        let x = arena.x_nodes.len();
        let y = arena.y_nodes.len();
        stats.total_x += x - 1;
        stats.total_y += y - 1;
        stats.max_x = stats.max_x.max(x);
        stats.max_y = stats.max_y.max(y);
        stats.max_bytes = stats.max_bytes.max(arena.bytes_used());
    }
    stats
}

fn replay_dense(cases: &[CaseTrace]) -> u64 {
    let mut checksum = 0_u64;
    for case in cases {
        let mut board = DenseBoard::new();
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
        checksum = checksum.wrapping_add(board.occupied.count() as u64);
    }
    black_box(checksum)
}

fn replay_persistent(cases: &[CaseTrace], x_capacity: usize, y_capacity: usize) -> u64 {
    let mut checksum = 0_u64;
    let mut arena = PersistentOwner2D::with_capacity(x_capacity, y_capacity);
    for case in cases {
        arena.reset();
        let mut board = PersistentBoard::new();
        for turn in &case.turns {
            if turn.final_clear {
                board.clear_all();
                continue;
            }
            board.clear_regions(&turn.departed_regions);
            board.clear_regions(&turn.moved_regions);
            for placement in &turn.placements {
                board.place(
                    &mut arena,
                    placement.owner,
                    &placement.region,
                    &placement.rectangles,
                );
            }
        }
        checksum = checksum
            .wrapping_add(board.occupied.count() as u64)
            .wrapping_add(arena.x_nodes.len() as u64)
            .wrapping_add(arena.y_nodes.len() as u64);
    }
    black_box(checksum)
}

fn replay_dense_placements(cases: &[CaseTrace]) -> u64 {
    let mut checksum = 0_u64;
    for case in cases {
        let mut board = DenseBoard::new();
        for turn in &case.turns {
            for placement in &turn.placements {
                board.place(placement.owner, &placement.region);
            }
        }
        checksum = checksum.wrapping_add(board.occupied.count() as u64);
    }
    black_box(checksum)
}

fn replay_persistent_placements(cases: &[CaseTrace], x_capacity: usize, y_capacity: usize) -> u64 {
    let mut checksum = 0_u64;
    let mut arena = PersistentOwner2D::with_capacity(x_capacity, y_capacity);
    for case in cases {
        arena.reset();
        let mut board = PersistentBoard::new();
        for turn in &case.turns {
            for placement in &turn.placements {
                board.place(
                    &mut arena,
                    placement.owner,
                    &placement.region,
                    &placement.rectangles,
                );
            }
        }
        checksum = checksum
            .wrapping_add(board.occupied.count() as u64)
            .wrapping_add(arena.x_nodes.len() as u64)
            .wrapping_add(arena.y_nodes.len() as u64);
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

fn read_boundary_persistent(
    samples: &[ReadCloneSample],
    arena: &PersistentOwner2D,
    group_values: &[u64],
) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for &id in &sample.boundary_queries {
            let owner = sample
                .persistent
                .board
                .owner_at_id(arena, id as usize)
                .unwrap() as usize;
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

fn read_sequential_persistent(
    samples: &[ReadCloneSample],
    arena: &PersistentOwner2D,
    group_values: &[u64],
) -> u64 {
    let mut checksum = 0_u64;
    let group_values = black_box(group_values);
    for sample in samples {
        for id in 0..CELL_COUNT {
            if let Some(owner) = sample.persistent.board.owner_at_id(arena, id) {
                checksum = checksum.wrapping_add(group_values[owner as usize]);
            }
        }
    }
    black_box(checksum)
}

fn clone_board_dense(samples: &[ReadCloneSample], sink: &mut Vec<DenseBoard>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.dense.board.clone()));
    black_box(&*sink);
    black_box(sink.len() as u64)
}

fn clone_board_persistent(samples: &[ReadCloneSample], sink: &mut Vec<PersistentBoard>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.persistent.board.clone()));
    black_box(&*sink);
    black_box(sink.len() as u64)
}

fn clone_state_dense(samples: &[ReadCloneSample], sink: &mut Vec<DenseSearchState>) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.dense.clone()));
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        let group = state.active_groups.first().map_or(0, |group| {
            group.i as u64 + group.compactness.to_bits() + group.min_compactness.to_bits()
        });
        acc.wrapping_add(group)
            .wrapping_add(state.active_index_by_group[0] as u64)
            .wrapping_add(state.current_X as u64)
    });
    black_box(&*sink);
    black_box(checksum)
}

fn clone_state_persistent(
    samples: &[ReadCloneSample],
    sink: &mut Vec<PersistentSearchState>,
) -> u64 {
    sink.clear();
    sink.extend(samples.iter().map(|sample| sample.persistent.clone()));
    let checksum = sink.iter().fold(0_u64, |acc, state| {
        let group = state.active_groups.first().map_or(0, |group| {
            group.i as u64 + group.compactness.to_bits() + group.min_compactness.to_bits()
        });
        acc.wrapping_add(group)
            .wrapping_add(state.active_index_by_group[0] as u64)
            .wrapping_add(state.current_X as u64)
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
    black_box(&*sink);
    black_box(sink.len() as u64)
}

fn clone_place_board_persistent(
    samples: &[CandidateSample],
    sink: &mut Vec<PersistentBoard>,
    arena: &mut PersistentOwner2D,
    checkpoint: ArenaCheckpoint,
) -> u64 {
    sink.clear();
    arena.rollback(checkpoint);
    for sample in samples {
        let mut board = sample.persistent.board.clone();
        board.place(
            arena,
            sample.placement.owner,
            &sample.placement.region,
            &sample.placement.rectangles,
        );
        sink.push(board);
    }
    black_box(&*sink);
    black_box((sink.len() + arena.x_nodes.len() + arena.y_nodes.len()) as u64)
}

fn clone_place_state_dense(samples: &[CandidateSample], sink: &mut Vec<DenseSearchState>) -> u64 {
    sink.clear();
    for sample in samples {
        let mut state = sample.dense.clone();
        state
            .board
            .place(sample.placement.owner, &sample.placement.region);
        sink.push(state);
    }
    black_box(&*sink);
    black_box(sink.len() as u64)
}

fn clone_place_state_persistent(
    samples: &[CandidateSample],
    sink: &mut Vec<PersistentSearchState>,
    arena: &mut PersistentOwner2D,
    checkpoint: ArenaCheckpoint,
) -> u64 {
    sink.clear();
    arena.rollback(checkpoint);
    for sample in samples {
        let mut state = sample.persistent.clone();
        state.board.place(
            arena,
            sample.placement.owner,
            &sample.placement.region,
            &sample.placement.rectangles,
        );
        sink.push(state);
    }
    black_box(&*sink);
    black_box((sink.len() + arena.x_nodes.len() + arena.y_nodes.len()) as u64)
}

fn decompose_workload(cases: &[CaseTrace]) -> u64 {
    let mut checksum = 0_u64;
    for case in cases {
        for turn in &case.turns {
            for placement in &turn.placements {
                let rectangles = decompose_rectangles(&placement.region);
                checksum = checksum.wrapping_add(rectangles.len() as u64);
            }
        }
    }
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

fn benchmark_pair<FD, FP>(label: &str, operations: usize, mut dense: FD, mut persistent: FP)
where
    FD: FnMut() -> u64,
    FP: FnMut() -> u64,
{
    let dense_repeats = calibrated_repeats(&mut dense);
    let persistent_repeats = calibrated_repeats(&mut persistent);
    let mut dense_samples = Vec::with_capacity(BENCH_ROUNDS);
    let mut persistent_samples = Vec::with_capacity(BENCH_ROUNDS);
    for round in 0..BENCH_ROUNDS {
        if round % 2 == 0 {
            dense_samples.push(measure(&mut dense, dense_repeats));
            persistent_samples.push(measure(&mut persistent, persistent_repeats));
        } else {
            persistent_samples.push(measure(&mut persistent, persistent_repeats));
            dense_samples.push(measure(&mut dense, dense_repeats));
        }
    }
    let dense_min = *dense_samples.iter().min().unwrap();
    let dense_max = *dense_samples.iter().max().unwrap();
    let persistent_min = *persistent_samples.iter().min().unwrap();
    let persistent_max = *persistent_samples.iter().max().unwrap();
    let dense_result = Measurement {
        median: median_duration(&mut dense_samples),
        min: dense_min,
        max: dense_max,
    };
    let persistent_result = Measurement {
        median: median_duration(&mut persistent_samples),
        min: persistent_min,
        max: persistent_max,
    };
    let dense_ns = dense_result.median.as_secs_f64() * 1e9 / operations as f64;
    let persistent_ns = persistent_result.median.as_secs_f64() * 1e9 / operations as f64;
    println!("[bench] {label}");
    println!(
        "  dense      {:10.3} ns/op median={:9.3} ms range={:.3}..{:.3} ms",
        dense_ns,
        dense_result.median.as_secs_f64() * 1e3,
        dense_result.min.as_secs_f64() * 1e3,
        dense_result.max.as_secs_f64() * 1e3,
    );
    println!(
        "  persistent {:10.3} ns/op median={:9.3} ms range={:.3}..{:.3} ms",
        persistent_ns,
        persistent_result.median.as_secs_f64() * 1e3,
        persistent_result.min.as_secs_f64() * 1e3,
        persistent_result.max.as_secs_f64() * 1e3,
    );
    println!("  dense/persistent = {:.3}x", dense_ns / persistent_ns);
}

fn benchmark_single<F>(label: &str, operations: usize, mut f: F)
where
    F: FnMut() -> u64,
{
    let repeats = calibrated_repeats(&mut f);
    let mut samples = Vec::with_capacity(BENCH_ROUNDS);
    for _ in 0..BENCH_ROUNDS {
        samples.push(measure(&mut f, repeats));
    }
    let result = median_duration(&mut samples);
    println!(
        "[bench] {label}: {:.3} ns/op median={:.3} ms",
        result.as_secs_f64() * 1e9 / operations as f64,
        result.as_secs_f64() * 1e3,
    );
}

fn percentile(sorted: &[usize], numerator: usize, denominator: usize) -> usize {
    sorted[(sorted.len() - 1) * numerator / denominator]
}

fn main() {
    run_correctness_checks();
    if env::args().any(|arg| arg == "--check-only") {
        return;
    }

    let args = env::args().collect::<Vec<_>>();
    let input_dir = Path::new(args.get(1).map_or("tools/in", String::as_str));
    let output_dir = Path::new(
        args.get(2)
            .map_or("results/out/v052_adaptive_capacity", String::as_str),
    );
    let case_limit = args.get(3).map_or(100, |s| s.parse().expect("case limit"));

    println!("[load] input={}", input_dir.display());
    println!("[load] output={}", output_dir.display());
    println!("[load] case_limit={case_limit}");
    let (cases, mut stats) = load_traces(input_dir, output_dir, case_limit);
    stats.rectangle_counts.sort_unstable();
    println!("[trace] cases={}", stats.cases);
    println!("[trace] turns={}", stats.turns);
    println!("[trace] placements={}", stats.placements);
    println!(
        "[trace] avg_cells_per_placement={:.3}",
        stats.placed_cells as f64 / stats.placements as f64
    );
    println!(
        "[trace] rectangles avg={:.3} p50={} p90={} p99={} max={}",
        stats.rectangle_count as f64 / stats.placements as f64,
        percentile(&stats.rectangle_counts, 50, 100),
        percentile(&stats.rectangle_counts, 90, 100),
        percentile(&stats.rectangle_counts, 99, 100),
        stats.rectangle_counts.last().unwrap(),
    );

    let node_stats = persistent_node_stats(&cases);
    println!(
        "[nodes] XNode={} bytes YNode={} bytes",
        size_of::<XNode>(),
        size_of::<YNode>()
    );
    println!(
        "[nodes] avg_per_placement x={:.3} y={:.3} bytes={:.1}",
        node_stats.total_x as f64 / stats.placements as f64,
        node_stats.total_y as f64 / stats.placements as f64,
        (node_stats.total_x * size_of::<XNode>() + node_stats.total_y * size_of::<YNode>()) as f64
            / stats.placements as f64,
    );
    println!(
        "[nodes] max_case_arena={:.3} MiB",
        node_stats.max_bytes as f64 / (1024.0 * 1024.0)
    );

    let (mut sample_arena, read_samples, candidate_samples) = build_samples(&cases);
    let sample_checkpoint = sample_arena.checkpoint();
    let one_rectangle_samples = candidate_samples
        .iter()
        .filter(|sample| sample.placement.rectangles.len() == 1)
        .cloned()
        .collect::<Vec<_>>();
    let large_simple_samples = candidate_samples
        .iter()
        .filter(|sample| {
            sample.placement.region.count() >= 64 && sample.placement.rectangles.len() <= 2
        })
        .cloned()
        .collect::<Vec<_>>();
    let boundary_query_count = read_samples
        .iter()
        .map(|sample| sample.boundary_queries.len())
        .sum::<usize>();
    let group_values = (0..M)
        .map(|i| (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .collect::<Vec<_>>();
    println!("[sample] read_samples={}", read_samples.len());
    println!("[sample] boundary_queries={boundary_query_count}");
    println!("[sample] candidate_samples={}", candidate_samples.len());
    println!(
        "[sample] one_rectangle_samples={}",
        one_rectangle_samples.len()
    );
    println!(
        "[sample] large_simple_samples={}",
        large_simple_samples.len()
    );

    println!("[size] DenseBoard={} bytes", size_of::<DenseBoard>());
    println!(
        "[size] PersistentBoard={} bytes + shared arena",
        size_of::<PersistentBoard>()
    );
    println!(
        "[size] DenseSearchState={} bytes + active_groups heap",
        size_of::<DenseSearchState>()
    );
    println!(
        "[size] PersistentSearchState={} bytes + active_groups heap + shared arena",
        size_of::<PersistentSearchState>()
    );

    benchmark_single("CellSet -> exact rectangles", stats.placements, || {
        decompose_workload(&cases)
    });

    benchmark_pair(
        "exact saved-output board transitions",
        stats.turns,
        || replay_dense(&cases),
        || replay_persistent(&cases, node_stats.max_x, node_stats.max_y),
    );
    benchmark_pair(
        "compact placement only (rectangles precomputed)",
        stats.placements,
        || replay_dense_placements(&cases),
        || replay_persistent_placements(&cases, node_stats.max_x, node_stats.max_y),
    );
    benchmark_pair(
        "occupied-neighbor owner_at + group metadata",
        boundary_query_count,
        || read_boundary_dense(&read_samples, &group_values),
        || read_boundary_persistent(&read_samples, &sample_arena, &group_values),
    );
    benchmark_pair(
        "sequential full-board owner_at + group metadata",
        read_samples.len() * CELL_COUNT,
        || read_sequential_dense(&read_samples, &group_values),
        || read_sequential_persistent(&read_samples, &sample_arena, &group_values),
    );

    let mut dense_board_sink = Vec::with_capacity(read_samples.len());
    let mut persistent_board_sink = Vec::with_capacity(read_samples.len());
    benchmark_pair(
        "board clone",
        read_samples.len(),
        || clone_board_dense(&read_samples, &mut dense_board_sink),
        || clone_board_persistent(&read_samples, &mut persistent_board_sink),
    );

    let mut dense_state_sink = Vec::with_capacity(read_samples.len());
    let mut persistent_state_sink = Vec::with_capacity(read_samples.len());
    benchmark_pair(
        "V000-like State clone",
        read_samples.len(),
        || clone_state_dense(&read_samples, &mut dense_state_sink),
        || clone_state_persistent(&read_samples, &mut persistent_state_sink),
    );

    let mut dense_board_candidate_sink = Vec::with_capacity(candidate_samples.len());
    let mut persistent_board_candidate_sink = Vec::with_capacity(candidate_samples.len());
    benchmark_pair(
        "board clone + compact placement",
        candidate_samples.len(),
        || clone_place_board_dense(&candidate_samples, &mut dense_board_candidate_sink),
        || {
            clone_place_board_persistent(
                &candidate_samples,
                &mut persistent_board_candidate_sink,
                &mut sample_arena,
                sample_checkpoint,
            )
        },
    );

    let mut dense_state_candidate_sink = Vec::with_capacity(candidate_samples.len());
    let mut persistent_state_candidate_sink = Vec::with_capacity(candidate_samples.len());
    benchmark_pair(
        "V000-like State clone + compact placement",
        candidate_samples.len(),
        || clone_place_state_dense(&candidate_samples, &mut dense_state_candidate_sink),
        || {
            clone_place_state_persistent(
                &candidate_samples,
                &mut persistent_state_candidate_sink,
                &mut sample_arena,
                sample_checkpoint,
            )
        },
    );

    if !one_rectangle_samples.is_empty() {
        let mut dense_sink = Vec::with_capacity(one_rectangle_samples.len());
        let mut persistent_sink = Vec::with_capacity(one_rectangle_samples.len());
        benchmark_pair(
            "V000-like State clone + one-rectangle placement",
            one_rectangle_samples.len(),
            || clone_place_state_dense(&one_rectangle_samples, &mut dense_sink),
            || {
                clone_place_state_persistent(
                    &one_rectangle_samples,
                    &mut persistent_sink,
                    &mut sample_arena,
                    sample_checkpoint,
                )
            },
        );
    }

    if !large_simple_samples.is_empty() {
        let mut dense_sink = Vec::with_capacity(large_simple_samples.len());
        let mut persistent_sink = Vec::with_capacity(large_simple_samples.len());
        benchmark_pair(
            "V000-like State clone + P>=64 and <=2-rectangle placement",
            large_simple_samples.len(),
            || clone_place_state_dense(&large_simple_samples, &mut dense_sink),
            || {
                clone_place_state_persistent(
                    &large_simple_samples,
                    &mut persistent_sink,
                    &mut sample_arena,
                    sample_checkpoint,
                )
            },
        );
    }
}
