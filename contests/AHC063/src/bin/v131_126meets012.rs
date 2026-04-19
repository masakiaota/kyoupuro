// v131_126meets012.rs
mod solver {
    #![allow(dead_code)]
    #![allow(unused_imports)]
    // v000_template.rs
    use std::collections::VecDeque;
    use std::fmt;
    use std::hash::{Hash, Hasher};
    use std::io::{self, Read};
    use std::ops::Deref;
    use std::ops::Index;
    use std::time::Instant;

    const INTERNAL_COLOR_CAPACITY: usize = 16 * 16;
    const INTERNAL_COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
    const INTERNAL_COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;
    const INTERNAL_POS_DEQUE_CAPACITY: usize = 16 * 16;
    const INTERNAL_POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
    const INTERNAL_POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;
    const MAX_TURNS: usize = 100_000;
    const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Cell(u16);

    struct Grid;

    impl Grid {
        #[inline]
        fn cell(n: usize, i: usize, j: usize) -> Cell {
            debug_assert!(i < n && j < n);
            Cell((i * n + j) as u16)
        }

        #[inline]
        fn index(cell: Cell) -> usize {
            cell.0 as usize
        }

        #[inline]
        fn i(n: usize, cell: Cell) -> usize {
            Self::index(cell) / n
        }

        #[inline]
        fn j(n: usize, cell: Cell) -> usize {
            Self::index(cell) % n
        }

        #[inline]
        fn ij(n: usize, cell: Cell) -> (usize, usize) {
            (Self::i(n, cell), Self::j(n, cell))
        }

        #[inline]
        fn can_move(n: usize, cell: Cell, dir: usize) -> bool {
            if dir >= DIRS.len() {
                return false;
            }
            let idx = Self::index(cell);
            match dir {
                0 => idx >= n,
                1 => idx + n < n * n,
                2 => idx % n != 0,
                3 => idx % n + 1 < n,
                _ => false,
            }
        }

        #[inline]
        fn next_cell(n: usize, cell: Cell, dir: usize) -> Cell {
            debug_assert!(Self::can_move(n, cell, dir));
            let idx = Self::index(cell);
            match dir {
                0 => Cell((idx - n) as u16),
                1 => Cell((idx + n) as u16),
                2 => Cell((idx - 1) as u16),
                3 => Cell((idx + 1) as u16),
                _ => unreachable!("invalid dir: {dir}"),
            }
        }

        #[inline]
        fn dir_between_cells(n: usize, from: Cell, to: Cell) -> usize {
            let from_idx = Self::index(from);
            let to_idx = Self::index(to);
            if to_idx + n == from_idx {
                0
            } else if from_idx + n == to_idx {
                1
            } else if to_idx + 1 == from_idx && from_idx / n == to_idx / n {
                2
            } else if from_idx + 1 == to_idx && from_idx / n == to_idx / n {
                3
            } else {
                unreachable!("cells are not adjacent: from={from_idx}, to={to_idx}");
            }
        }
    }

    #[derive(Debug, Clone)]
    struct ManhattanTable {
        cell_count: usize,
        dist: Vec<u8>,
    }

    impl ManhattanTable {
        fn new(n: usize) -> Self {
            let cell_count = n * n;
            let mut dist = vec![0_u8; cell_count * cell_count];
            for a in 0..cell_count {
                let ai = a / n;
                let aj = a % n;
                for b in a..cell_count {
                    let bi = b / n;
                    let bj = b % n;
                    let d = (ai.abs_diff(bi) + aj.abs_diff(bj)) as u8;
                    dist[a * cell_count + b] = d;
                    dist[b * cell_count + a] = d;
                }
            }
            Self { cell_count, dist }
        }

        #[inline]
        fn get(&self, a: Cell, b: Cell) -> usize {
            self.dist[Grid::index(a) * self.cell_count + Grid::index(b)] as usize
        }
    }

    const fn build_internal_pos_hash_pows(base: u64) -> [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] {
        let mut pows = [0_u64; INTERNAL_POS_DEQUE_CAPACITY + 1];
        pows[0] = 1;
        let mut i = 1;
        while i <= INTERNAL_POS_DEQUE_CAPACITY {
            pows[i] = pows[i - 1].wrapping_mul(base);
            i += 1;
        }
        pows
    }

    const INTERNAL_POS_HASH_POW1: [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] =
        build_internal_pos_hash_pows(INTERNAL_POS_HASH_BASE1);
    const INTERNAL_POS_HASH_POW2: [u64; INTERNAL_POS_DEQUE_CAPACITY + 1] =
        build_internal_pos_hash_pows(INTERNAL_POS_HASH_BASE2);

    #[inline]
    fn encode_internal_pos_hash(cell: Cell) -> u64 {
        cell.0 as u64 + 1
    }

    #[derive(Clone)]
    struct InternalPosDeque {
        head: usize,
        len: usize,
        buf: [Cell; INTERNAL_POS_DEQUE_CAPACITY],
        hash1: u64,
        hash2: u64,
    }

    impl InternalPosDeque {
        // hash化が遅いかもしれない
        #[inline]
        fn new() -> Self {
            Self {
                head: 0,
                len: 0,
                buf: [Cell(0); INTERNAL_POS_DEQUE_CAPACITY],
                hash1: 0,
                hash2: 0,
            }
        }

        #[inline]
        fn from_slice(cells: &[Cell]) -> Self {
            debug_assert!(cells.len() <= INTERNAL_POS_DEQUE_CAPACITY);
            let mut deque = Self::new();
            deque.buf[..cells.len()].copy_from_slice(cells);
            deque.len = cells.len();
            let mut pow1 = 1_u64;
            let mut pow2 = 1_u64;
            for &cell in cells {
                let x = encode_internal_pos_hash(cell);
                deque.hash1 = deque.hash1.wrapping_add(x.wrapping_mul(pow1));
                deque.hash2 = deque.hash2.wrapping_add(x.wrapping_mul(pow2));
                pow1 = pow1.wrapping_mul(INTERNAL_POS_HASH_BASE1);
                pow2 = pow2.wrapping_mul(INTERNAL_POS_HASH_BASE2);
            }
            deque
        }

        #[inline]
        fn len(&self) -> usize {
            self.len
        }

        #[inline]
        fn is_empty(&self) -> bool {
            self.len == 0
        }

        #[inline]
        fn physical_index(&self, idx: usize) -> usize {
            debug_assert!(idx < self.len);
            let raw = self.head + idx;
            if raw < INTERNAL_POS_DEQUE_CAPACITY {
                raw
            } else {
                raw - INTERNAL_POS_DEQUE_CAPACITY
            }
        }

        #[inline]
        fn push_front(&mut self, cell: Cell) {
            debug_assert!(self.len < INTERNAL_POS_DEQUE_CAPACITY);
            let x = encode_internal_pos_hash(cell);
            self.head = (self.head + INTERNAL_POS_DEQUE_CAPACITY - 1) % INTERNAL_POS_DEQUE_CAPACITY;
            self.buf[self.head] = cell;
            self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(INTERNAL_POS_HASH_BASE1));
            self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(INTERNAL_POS_HASH_BASE2));
            self.len += 1;
        }

        #[inline]
        fn pop_back(&mut self) -> Option<Cell> {
            if self.is_empty() {
                return None;
            }
            let idx = self.physical_index(self.len - 1);
            let cell = self.buf[idx];
            let x = encode_internal_pos_hash(cell);
            self.hash1 = self
                .hash1
                .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW1[self.len - 1]));
            self.hash2 = self
                .hash2
                .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW2[self.len - 1]));
            self.len -= 1;
            Some(cell)
        }

        #[inline]
        fn iter(&self) -> InternalPosDequeIter<'_> {
            InternalPosDequeIter {
                deque: self,
                idx: 0,
            }
        }

        #[inline]
        fn back(&self) -> Option<Cell> {
            if self.is_empty() {
                None
            } else {
                Some(self[self.len - 1])
            }
        }

        #[inline]
        fn rolling_hash_pair(&self) -> (u64, u64) {
            (self.hash1, self.hash2)
        }
    }

    impl PartialEq for InternalPosDeque {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.len == other.len
                && self.hash1 == other.hash1
                && self.hash2 == other.hash2
                && self.iter().zip(other.iter()).all(|(a, b)| a == b)
        }
    }

    impl Eq for InternalPosDeque {}

    impl Hash for InternalPosDeque {
        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.len.hash(state);
            self.hash1.hash(state);
            self.hash2.hash(state);
        }
    }

    impl fmt::Debug for InternalPosDeque {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.iter()).finish()
        }
    }

    impl Index<usize> for InternalPosDeque {
        type Output = Cell;

        #[inline]
        fn index(&self, idx: usize) -> &Self::Output {
            let physical_idx = self.physical_index(idx);
            &self.buf[physical_idx]
        }
    }

    struct InternalPosDequeIter<'a> {
        deque: &'a InternalPosDeque,
        idx: usize,
    }

    impl Iterator for InternalPosDequeIter<'_> {
        type Item = Cell;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.idx == self.deque.len() {
                None
            } else {
                let cell = self.deque[self.idx];
                self.idx += 1;
                Some(cell)
            }
        }
    }

    const fn build_internal_color_hash_pows(base: u64) -> [u64; INTERNAL_COLOR_CAPACITY + 1] {
        let mut pows = [0_u64; INTERNAL_COLOR_CAPACITY + 1];
        pows[0] = 1;
        let mut i = 1;
        while i <= INTERNAL_COLOR_CAPACITY {
            pows[i] = pows[i - 1].wrapping_mul(base);
            i += 1;
        }
        pows
    }

    const INTERNAL_COLOR_HASH_POW1: [u64; INTERNAL_COLOR_CAPACITY + 1] =
        build_internal_color_hash_pows(INTERNAL_COLOR_HASH_BASE1);
    const INTERNAL_COLOR_HASH_POW2: [u64; INTERNAL_COLOR_CAPACITY + 1] =
        build_internal_color_hash_pows(INTERNAL_COLOR_HASH_BASE2);

    #[inline]
    fn encode_internal_color_hash(color: u8) -> u64 {
        color as u64 + 1
    }

    #[derive(Clone)]
    struct InternalColors {
        buf: [u8; INTERNAL_COLOR_CAPACITY],
        len: u16,
        hash1: u64,
        hash2: u64,
    }

    impl InternalColors {
        #[inline]
        fn new() -> Self {
            Self {
                buf: [0; INTERNAL_COLOR_CAPACITY],
                len: 0,
                hash1: 0,
                hash2: 0,
            }
        }

        #[inline]
        fn from_slice(colors: &[u8]) -> Self {
            debug_assert!(colors.len() <= INTERNAL_COLOR_CAPACITY);
            let mut out = Self::new();
            out.buf[..colors.len()].copy_from_slice(colors);
            out.len = colors.len() as u16;
            let mut pow1 = 1_u64;
            let mut pow2 = 1_u64;
            for &color in colors {
                let x = encode_internal_color_hash(color);
                out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
                out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
                pow1 = pow1.wrapping_mul(INTERNAL_COLOR_HASH_BASE1);
                pow2 = pow2.wrapping_mul(INTERNAL_COLOR_HASH_BASE2);
            }
            out
        }

        #[inline]
        fn len(&self) -> usize {
            self.len as usize
        }

        #[inline]
        fn is_empty(&self) -> bool {
            self.len == 0
        }

        #[inline]
        fn push(&mut self, color: u8) {
            debug_assert!(self.len() < INTERNAL_COLOR_CAPACITY);
            let idx = self.len();
            self.buf[idx] = color;
            let x = encode_internal_color_hash(color);
            self.hash1 = self
                .hash1
                .wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW1[idx]));
            self.hash2 = self
                .hash2
                .wrapping_add(x.wrapping_mul(INTERNAL_COLOR_HASH_POW2[idx]));
            self.len += 1;
        }

        #[inline]
        fn pop(&mut self) -> Option<u8> {
            if self.is_empty() {
                return None;
            }
            let idx = self.len() - 1;
            let color = self.buf[idx];
            let x = encode_internal_color_hash(color);
            self.hash1 = self
                .hash1
                .wrapping_sub(x.wrapping_mul(INTERNAL_COLOR_HASH_POW1[idx]));
            self.hash2 = self
                .hash2
                .wrapping_sub(x.wrapping_mul(INTERNAL_COLOR_HASH_POW2[idx]));
            self.len -= 1;
            Some(color)
        }

        #[inline]
        fn rolling_hash_pair(&self) -> (u64, u64) {
            (self.hash1, self.hash2)
        }
    }

    impl Deref for InternalColors {
        type Target = [u8];

        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.buf[..self.len()]
        }
    }

    impl PartialEq for InternalColors {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.len == other.len
                && self.hash1 == other.hash1
                && self.hash2 == other.hash2
                && &self[..] == &other[..]
        }
    }

    impl Eq for InternalColors {}

    impl Hash for InternalColors {
        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.len.hash(state);
            self.hash1.hash(state);
            self.hash2.hash(state);
        }
    }

    impl fmt::Debug for InternalColors {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.iter().copied()).finish()
        }
    }

    #[inline]
    fn find_internal_bite_idx(pos: &InternalPosDeque) -> Option<usize> {
        debug_assert!(!pos.is_empty());
        let head = pos[0];
        (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct InternalPosOccupancy {
        cnt: [u8; INTERNAL_POS_DEQUE_CAPACITY],
    }

    impl InternalPosOccupancy {
        #[inline]
        fn new() -> Self {
            Self {
                cnt: [0; INTERNAL_POS_DEQUE_CAPACITY],
            }
        }

        #[inline]
        fn from_pos(pos: &InternalPosDeque) -> Self {
            let mut out = Self::new();
            for cell in pos.iter() {
                out.inc(cell);
            }
            out
        }

        #[inline]
        fn count(&self, cell: Cell) -> u8 {
            self.cnt[Grid::index(cell)]
        }

        #[inline]
        fn inc(&mut self, cell: Cell) {
            self.cnt[Grid::index(cell)] += 1;
        }

        #[inline]
        fn dec(&mut self, cell: Cell) {
            self.cnt[Grid::index(cell)] -= 1;
        }
    }

    // 高速化メモ:
    // beam などで State::clone や state hash が支配的になった場合は、
    // food / pos / colors を fixed-size 配列に寄せるとかなり速くなる可能性が高い。
    // その場合はこの State 定義を起点に見直す。
    #[derive(Debug, Clone)]
    struct State {
        // 現在盤面の餌配置。0 は餌なし
        food: Vec<u8>,
        // 蛇の座標列。pos[0] が頭
        pos: InternalPosDeque,
        // 蛇の色列。colors[p] は pos[p] に対応する
        colors: InternalColors,
        // 各マスに何個 segment があるかを持つ占有数
        pos_occupancy: InternalPosOccupancy,
    }

    impl State {
        #[inline]
        fn initial(input: &Input) -> Self {
            let pos = InternalPosDeque::from_slice(&[
                Grid::cell(input.n, 4, 0),
                Grid::cell(input.n, 3, 0),
                Grid::cell(input.n, 2, 0),
                Grid::cell(input.n, 1, 0),
                Grid::cell(input.n, 0, 0),
            ]);
            Self {
                food: input.food.clone(),
                pos: pos.clone(),
                colors: InternalColors::from_slice(&[1; 5]),
                pos_occupancy: InternalPosOccupancy::from_pos(&pos),
            }
        }

        #[inline]
        fn len(&self) -> usize {
            self.pos.len()
        }

        #[inline]
        fn head(&self) -> Cell {
            self.pos[0]
        }

        #[inline]
        fn tail_idx(&self) -> usize {
            self.pos.len() - 1
        }

        #[inline]
        fn tail(&self) -> Cell {
            self.pos[self.tail_idx()]
        }

        #[inline]
        fn is_legal_dir(&self, n: usize, dir: usize) -> bool {
            if !Grid::can_move(n, self.head(), dir) {
                return false;
            }

            if self.len() >= 2 {
                let next = Grid::next_cell(n, self.head(), dir);
                if next == self.pos[1] {
                    return false;
                }
            }

            true
        }

        #[inline]
        fn legal_dirs(&self, n: usize) -> Vec<usize> {
            let mut dirs = Vec::with_capacity(DIRS.len());
            for dir in 0..DIRS.len() {
                if self.is_legal_dir(n, dir) {
                    dirs.push(dir);
                }
            }
            dirs
        }

        #[inline]
        fn legal_dir_count(&self, n: usize) -> usize {
            let mut count = 0;
            for dir in 0..DIRS.len() {
                if self.is_legal_dir(n, dir) {
                    count += 1;
                }
            }
            count
        }
    }

    #[derive(Debug, Clone)]
    struct CellSearchResult {
        // 探索の始点
        start: Cell,
        // 空マスへの最短到達手数。未到達は usize::MAX
        dist: Vec<usize>,
        // 最短路木での直前マス。始点や未到達は None
        prev: Vec<Option<Cell>>,
    }

    /// 初期 `pos` を固定しつつ、初期胴体マスが後ろから 1 手ごとに 1 マスずつ解放される
    /// とみなした簡易 BFS を行い、空マスへの最短到達手数と復元用親ポインタを返す。
    ///
    /// - `food` はすべて永久壁として扱う
    /// - 初期胴体マスは、その Cell を占有する最前の segment が `excluded_tail`
    ///   になる時刻以降に通行可とみなす
    ///   - したがって旧 tail や `pos[len - 2]` は 1 手後に通れることがある
    /// - 返り値 `dist[idx]` は head からその空マスへの最短到達手数で、未到達は `usize::MAX`
    /// - `prev[idx]` はその最短路木における直前 Cell を表す
    ///
    /// この探索は「Cell ごとの最短到着時刻」しか保持しない簡易化なので、
    /// 同じ Cell に遅い時刻で再到達して release を待つ必要があるケースを落とすことがある
    /// （false negative がある）。
    ///
    /// したがって exact な到達可能性判定ではなく、軽量な到達可能性フィルタとして使う。
    fn compute_body_release_dist(state: &State, n: usize) -> CellSearchResult {
        let cell_count = n * n;
        let inf = usize::MAX;

        // release[idx] = そのマスに最短で入ってよい時刻
        let mut front_idx = vec![None; cell_count];
        for j in 0..state.pos.len() {
            let cell = state.pos[j];
            let idx = Grid::index(cell);
            if front_idx[idx].is_none() {
                front_idx[idx] = Some(j);
            }
        }
        let mut release = vec![0usize; cell_count];
        for idx in 0..cell_count {
            if let Some(j) = front_idx[idx] {
                release[idx] = (state.pos.len() - 1 - j).max(1);
            }
        }

        let mut dist = vec![inf; cell_count];
        let mut prev = vec![None; cell_count];
        let mut q = VecDeque::new();

        let head = state.head();
        let head_idx = Grid::index(head);
        dist[head_idx] = 0;
        q.push_back(head);

        while let Some(cur) = q.pop_front() {
            let cur_idx = Grid::index(cur);
            let cur_dist = dist[cur_idx];

            for dir in 0..DIRS.len() {
                if !Grid::can_move(n, cur, dir) {
                    continue;
                }

                let nxt = Grid::next_cell(n, cur, dir);
                let nxt_idx = Grid::index(nxt);
                let nd = cur_dist + 1;

                if dist[nxt_idx] != inf {
                    continue;
                }
                if state.food[nxt_idx] != 0 {
                    continue;
                }
                if nd < release[nxt_idx] {
                    continue;
                }

                dist[nxt_idx] = nd;
                prev[nxt_idx] = Some(cur);
                q.push_back(nxt);
            }
        }

        CellSearchResult {
            start: head,
            dist,
            prev,
        }
    }

    /// `compute_body_release_dist` の結果を用いて、各 Cell について
    /// 「その空マスへ到達する / その food を食べる」最短手数と復元用親ポインタを返す。
    ///
    /// - 空マス `cell` については `bfs` と同じ `dist/prev` を返す
    /// - food マス `cell` については、隣接 gate のうち到達可能なものから
    ///   `bfs.dist[gate] + 1` の最小値を `dist[cell]` に入れる
    /// - 到達可能な food マス `cell` については `prev[cell]` に最後の gate を入れる
    /// - 未到達は `usize::MAX` / `None`
    ///
    /// この値も `compute_body_release_dist` と同じ簡易化に基づくため false negative がある。
    fn body_release_eat_dist(bfs: &CellSearchResult, state: &State, n: usize) -> CellSearchResult {
        let inf = usize::MAX;
        let mut dist = bfs.dist.clone();
        let mut prev = bfs.prev.clone();

        for idx in 0..(n * n) {
            if state.food[idx] == 0 {
                continue;
            }

            let cell = Cell(idx as u16);
            let mut best = inf;
            let mut best_gate = None;
            for dir in 0..DIRS.len() {
                if !Grid::can_move(n, cell, dir) {
                    continue;
                }
                let gate = Grid::next_cell(n, cell, dir);
                let gate_idx = Grid::index(gate);
                if bfs.dist[gate_idx] == inf {
                    continue;
                }
                let cand = bfs.dist[gate_idx].saturating_add(1);
                if cand < best {
                    best = cand;
                    best_gate = Some(gate);
                }
            }
            dist[idx] = best;
            prev[idx] = best_gate;
        }

        CellSearchResult {
            start: bfs.start,
            dist,
            prev,
        }
    }

    /// `CellSearchResult` から `goal` までの Cell 列を復元する。
    ///
    /// - `goal` が空マスならその空マスへの経路
    /// - `goal` が food マスならその food を食べる経路
    /// を同じ形式で返す。
    /// 返り値は始点 `start` と終点 `goal` をともに含む。
    fn reconstruct_cell_search_path(result: &CellSearchResult, goal: Cell) -> Option<Vec<Cell>> {
        let goal_idx = Grid::index(goal);
        if result.dist[goal_idx] == usize::MAX {
            return None;
        }

        let mut rev = Vec::with_capacity(result.dist[goal_idx].saturating_add(1));
        let mut cur = goal;
        loop {
            rev.push(cur);
            if cur == result.start {
                break;
            }
            cur = result.prev[Grid::index(cur)]?;
        }
        rev.reverse();
        Some(rev)
    }

    #[inline]
    fn step(state: &State, n: usize, dir: usize) -> StepResult {
        debug_assert!(
            state.is_legal_dir(n, dir),
            "illegal dir: dir={dir}, head={:?}",
            state.head()
        );

        let next_head = Grid::next_cell(n, state.head(), dir);

        let mut food = state.food.clone();
        let mut new_pos = state.pos.clone();
        let mut new_colors = state.colors.clone();
        let mut new_pos_occupancy = state.pos_occupancy.clone();
        let mut ate = None;

        let eat_idx = Grid::index(next_head);
        if food[eat_idx] != 0 {
            let food_color = food[eat_idx];
            food[eat_idx] = 0;
            new_colors.push(food_color);
            ate = Some(food_color);
        } else {
            let old_tail = new_pos.pop_back().unwrap();
            new_pos_occupancy.dec(old_tail);
        }

        let excluded_tail = new_pos.back();
        let tail_bias = u8::from(excluded_tail == Some(next_head));
        let bite = new_pos_occupancy.count(next_head) > tail_bias;

        new_pos_occupancy.inc(next_head);
        new_pos.push_front(next_head);
        let bite_idx = if bite {
            find_internal_bite_idx(&new_pos)
        } else {
            None
        };
        debug_assert!(
            ate.is_none() || bite_idx.is_none(),
            "eat and bite must not happen simultaneously"
        );

        let mut dropped = Vec::new();
        if let Some(h) = bite_idx {
            let drop_len = new_pos.len().saturating_sub(h + 1);
            dropped.reserve(drop_len);
            let mut dropped_rev = Vec::with_capacity(drop_len);
            while new_pos.len() > h + 1 {
                let cell = new_pos.pop_back().unwrap();
                new_pos_occupancy.dec(cell);
                let color = new_colors.pop().unwrap();
                food[Grid::index(cell)] = color;
                dropped_rev.push(Dropped { cell, color });
            }
            dropped_rev.reverse();
            dropped = dropped_rev;
        }

        StepResult {
            state: State {
                food,
                pos: new_pos,
                colors: new_colors,
                pos_occupancy: new_pos_occupancy,
            },
            ate,
            bite_idx,
            dropped,
        }
    }

    #[inline]
    fn lcp(a: &[u8], b: &[u8]) -> usize {
        let lim = a.len().min(b.len());
        let mut i = 0;
        while i < lim && a[i] == b[i] {
            i += 1;
        }
        i
    }

    #[inline]
    fn matches_prefix_len(a: &[u8], b: &[u8], len: usize) -> bool {
        if a.len() < len || b.len() < len {
            return false;
        }
        let mut i = 0;
        while i < len {
            if a[i] != b[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    #[derive(Debug, Clone, Copy)]
    struct Dropped {
        // 噛みちぎりで盤面に落ちたマス
        cell: Cell,
        // そのマスに落ちた餌の色
        color: u8,
    }

    #[derive(Debug, Clone)]
    struct StepResult {
        // 1 手進めた後の状態
        state: State,
        // その手で食べた色。食べていなければ None
        ate: Option<u8>,
        // 噛みちぎりが起きたときの接触先 index h
        bite_idx: Option<usize>,
        // 噛みちぎりで落ちた餌列
        dropped: Vec<Dropped>,
    }

    #[derive(Debug, Clone)]
    struct Input {
        // 盤面サイズ N
        n: usize,
        // 目標色列の長さ M
        m: usize,
        // 色数 C
        color_count: usize,
        // 目標色列 d[0..M)
        d: Vec<u8>,
        // 初期盤面の餌配置。0 は餌なし
        food: Vec<u8>,
    }

    #[derive(Debug, Clone)]
    pub struct TimeKeeper {
        start: Instant,
        time_limit_sec: f64,

        iter: u64,
        check_mask: u64,

        elapsed_sec: f64,
        progress: f64,
        is_over: bool,
    }

    impl TimeKeeper {
        /// `check_interval_log2 = 8` なら 2^8 = 256 反復ごとに時計更新
        pub fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
            assert!(time_limit_sec > 0.0);
            assert!(check_interval_log2 < 63);

            let check_mask = if check_interval_log2 == 0 {
                0
            } else {
                (1_u64 << check_interval_log2) - 1
            };

            let mut tk = Self {
                start: Instant::now(),
                time_limit_sec,
                iter: 0,
                check_mask,
                elapsed_sec: 0.0,
                progress: 0.0,
                is_over: false,
            };
            tk.force_update();
            tk
        }

        /// ホットループではこれだけ呼ぶ
        /// true: 継続, false: 打ち切り
        #[inline(always)]
        pub fn step(&mut self) -> bool {
            self.iter += 1;
            if (self.iter & self.check_mask) == 0 {
                self.force_update();
            }
            !self.is_over
        }

        /// 明示的に時計を更新したいときに使う
        #[inline(always)]
        pub fn force_update(&mut self) {
            let elapsed = self.start.elapsed().as_secs_f64();
            self.elapsed_sec = elapsed;
            self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
            self.is_over = elapsed >= self.time_limit_sec;
        }

        /// batched な経過時間
        #[inline(always)]
        pub fn elapsed_sec(&self) -> f64 {
            self.elapsed_sec
        }

        /// batched な進捗率 [0, 1]
        #[inline(always)]
        pub fn progress(&self) -> f64 {
            self.progress
        }

        /// batched な時間切れ判定
        #[inline(always)]
        pub fn is_time_over(&self) -> bool {
            self.is_over
        }

        /// ログ用の正確な経過時間
        #[inline]
        pub fn exact_elapsed_sec(&self) -> f64 {
            self.start.elapsed().as_secs_f64()
        }

        /// ログ用の正確な残り時間
        #[inline]
        pub fn exact_remaining_sec(&self) -> f64 {
            (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
        }
    }

    fn read_input() -> Input {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap();
        let mut it = s.split_whitespace();

        let n: usize = it.next().unwrap().parse().unwrap();
        let m: usize = it.next().unwrap().parse().unwrap();
        let color_count: usize = it.next().unwrap().parse().unwrap();

        let mut d = vec![0_u8; m];
        for x in &mut d {
            *x = it.next().unwrap().parse::<u8>().unwrap();
        }

        let mut food = vec![0_u8; n * n];
        for r in 0..n {
            for c in 0..n {
                food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
            }
        }

        Input {
            n,
            m,
            color_count,
            d,
            food,
        }
    }

    fn main() {
        let input = read_input();
        let _state = State::initial(&input);
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rand::{Rng, SeedableRng};
        use rand_xoshiro::Xoshiro256PlusPlus;
        use std::collections::VecDeque;

        fn make_state(n: usize, food: Vec<u8>, pos_ij: &[(usize, usize)], colors: &[u8]) -> State {
            let pos = pos_ij
                .iter()
                .map(|&(i, j)| Grid::cell(n, i, j))
                .collect::<Vec<_>>();
            State {
                food,
                pos: InternalPosDeque::from_slice(&pos),
                colors: InternalColors::from_slice(colors),
                pos_occupancy: InternalPosOccupancy::from_pos(&InternalPosDeque::from_slice(&pos)),
            }
        }

        fn calc_internal_pos_hash_pair(cells: &[Cell]) -> (u64, u64) {
            let mut hash1 = 0_u64;
            let mut hash2 = 0_u64;
            let mut pow1 = 1_u64;
            let mut pow2 = 1_u64;
            for &cell in cells {
                let x = encode_internal_pos_hash(cell);
                hash1 = hash1.wrapping_add(x.wrapping_mul(pow1));
                hash2 = hash2.wrapping_add(x.wrapping_mul(pow2));
                pow1 = pow1.wrapping_mul(INTERNAL_POS_HASH_BASE1);
                pow2 = pow2.wrapping_mul(INTERNAL_POS_HASH_BASE2);
            }
            (hash1, hash2)
        }

        fn calc_internal_color_hash_pair(colors: &[u8]) -> (u64, u64) {
            let mut hash1 = 0_u64;
            let mut hash2 = 0_u64;
            let mut pow1 = 1_u64;
            let mut pow2 = 1_u64;
            for &color in colors {
                let x = encode_internal_color_hash(color);
                hash1 = hash1.wrapping_add(x.wrapping_mul(pow1));
                hash2 = hash2.wrapping_add(x.wrapping_mul(pow2));
                pow1 = pow1.wrapping_mul(INTERNAL_COLOR_HASH_BASE1);
                pow2 = pow2.wrapping_mul(INTERNAL_COLOR_HASH_BASE2);
            }
            (hash1, hash2)
        }

        #[test]
        fn lcp_works_for_vec_and_internal_colors() {
            assert_eq!(lcp(&[1, 1, 2, 3], &[1, 1, 2, 4, 5]), 3);
            assert_eq!(lcp(&[1, 2], &[1, 2]), 2);
            assert_eq!(lcp(&[1, 2], &[2, 1]), 0);
            assert_eq!(lcp(&[] as &[u8], &[1, 2, 3]), 0);

            let colors = InternalColors::from_slice(&[1, 1, 2, 5]);
            let target = vec![1, 1, 2, 4, 3];
            assert_eq!(lcp(&colors, &target), 3);
        }

        #[test]
        fn matches_prefix_len_works_for_vec_and_internal_colors() {
            assert!(matches_prefix_len(&[1, 1, 2, 3], &[1, 1, 2, 4, 5], 3));
            assert!(!matches_prefix_len(&[1, 1, 2, 3], &[1, 1, 2, 4, 5], 4));
            assert!(matches_prefix_len(&[1, 2], &[1, 2], 2));
            assert!(!matches_prefix_len(&[1, 2], &[1, 2], 3));

            let colors = InternalColors::from_slice(&[1, 1, 2, 5]);
            let target = vec![1, 1, 2, 4, 3];
            assert!(matches_prefix_len(&colors, &target, 3));
            assert!(!matches_prefix_len(&colors, &target, 4));
        }

        fn assert_state_consistent(state: &State) {
            assert_eq!(state.pos.len(), state.colors.len());
            let cells = state.pos.iter().collect::<Vec<_>>();
            assert_eq!(
                state.pos.rolling_hash_pair(),
                calc_internal_pos_hash_pair(&cells)
            );
            assert_eq!(
                state.colors.rolling_hash_pair(),
                calc_internal_color_hash_pair(&state.colors)
            );
            let mut cnt = [0_u8; INTERNAL_POS_DEQUE_CAPACITY];
            for &cell in &cells {
                cnt[Grid::index(cell)] += 1;
            }
            assert_eq!(state.pos_occupancy.cnt, cnt);
        }

        fn assert_step_result_eq(actual: &StepResult, expected: &StepResult) {
            assert_eq!(actual.ate, expected.ate);
            assert_eq!(actual.bite_idx, expected.bite_idx);
            assert_eq!(actual.dropped.len(), expected.dropped.len());
            for (a, e) in actual.dropped.iter().zip(expected.dropped.iter()) {
                assert_eq!(a.cell, e.cell);
                assert_eq!(a.color, e.color);
            }
            assert_eq!(actual.state.food, expected.state.food);
            assert_eq!(actual.state.colors, expected.state.colors);
            assert_eq!(actual.state.pos, expected.state.pos);
            assert_eq!(actual.state.pos_occupancy, expected.state.pos_occupancy);
            assert_state_consistent(&actual.state);
        }

        fn reference_step(state: &State, n: usize, dir: usize) -> StepResult {
            assert!(state.is_legal_dir(n, dir));

            let next_head = Grid::next_cell(n, state.head(), dir);

            let mut food = state.food.clone();
            let mut pos = state.pos.iter().collect::<VecDeque<_>>();
            let mut colors = state.colors.clone();
            let mut ate = None;

            let eat_idx = Grid::index(next_head);
            if food[eat_idx] != 0 {
                let color = food[eat_idx];
                food[eat_idx] = 0;
                colors.push(color);
                ate = Some(color);
            } else {
                pos.pop_back();
            }
            pos.push_front(next_head);

            let mut bite_idx = None;
            for idx in 1..pos.len().saturating_sub(1) {
                if pos[idx] == next_head {
                    bite_idx = Some(idx);
                    break;
                }
            }

            let mut dropped = Vec::new();
            if let Some(h) = bite_idx {
                let mut dropped_rev = Vec::new();
                while pos.len() > h + 1 {
                    let cell = pos.pop_back().unwrap();
                    let color = colors.pop().unwrap();
                    food[Grid::index(cell)] = color;
                    dropped_rev.push(Dropped { cell, color });
                }
                dropped_rev.reverse();
                dropped = dropped_rev;
            }

            let pos = pos.iter().copied().collect::<Vec<_>>();
            StepResult {
                state: State {
                    food,
                    pos: InternalPosDeque::from_slice(&pos),
                    colors,
                    pos_occupancy: InternalPosOccupancy::from_pos(&InternalPosDeque::from_slice(
                        &pos,
                    )),
                },
                ate,
                bite_idx,
                dropped,
            }
        }

        #[test]
        fn internal_pos_deque_push_front_and_pop_back() {
            let mut pos = InternalPosDeque::from_slice(&[
                Grid::cell(8, 4, 0),
                Grid::cell(8, 3, 0),
                Grid::cell(8, 2, 0),
            ]);
            assert_eq!(
                pos.rolling_hash_pair(),
                calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
            );
            pos.push_front(Grid::cell(8, 5, 0));
            assert_eq!(
                pos.iter().collect::<Vec<_>>(),
                vec![
                    Grid::cell(8, 5, 0),
                    Grid::cell(8, 4, 0),
                    Grid::cell(8, 3, 0),
                    Grid::cell(8, 2, 0)
                ]
            );
            assert_eq!(
                pos.rolling_hash_pair(),
                calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
            );
            assert_eq!(pos.pop_back(), Some(Grid::cell(8, 2, 0)));
            assert_eq!(
                pos.iter().collect::<Vec<_>>(),
                vec![
                    Grid::cell(8, 5, 0),
                    Grid::cell(8, 4, 0),
                    Grid::cell(8, 3, 0)
                ]
            );
            assert_eq!(
                pos.rolling_hash_pair(),
                calc_internal_pos_hash_pair(&pos.iter().collect::<Vec<_>>())
            );
        }

        #[test]
        fn grid_methods_work() {
            let n = 8;
            let center = Grid::cell(n, 3, 4);
            assert!(Grid::can_move(n, center, 0));
            assert!(Grid::can_move(n, center, 1));
            assert!(Grid::can_move(n, center, 2));
            assert!(Grid::can_move(n, center, 3));
            assert!(!Grid::can_move(n, Grid::cell(n, 0, 0), 0));
            assert!(!Grid::can_move(n, Grid::cell(n, 0, 0), 2));

            assert_eq!(Grid::next_cell(n, center, 0), Grid::cell(n, 2, 4));
            assert_eq!(Grid::next_cell(n, center, 1), Grid::cell(n, 4, 4));
            assert_eq!(Grid::next_cell(n, center, 2), Grid::cell(n, 3, 3));
            assert_eq!(Grid::next_cell(n, center, 3), Grid::cell(n, 3, 5));

            assert_eq!(
                Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 2, 4)),
                0
            );
            assert_eq!(
                Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 4, 4)),
                1
            );
            assert_eq!(
                Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 3, 3)),
                2
            );
            assert_eq!(
                Grid::dir_between_cells(n, Grid::cell(n, 3, 4), Grid::cell(n, 3, 5)),
                3
            );
        }

        #[test]
        fn manhattan_table_works() {
            let n = 8;
            let table = ManhattanTable::new(n);
            let a = Grid::cell(n, 1, 2);
            let b = Grid::cell(n, 4, 6);
            let c = Grid::cell(n, 1, 2);
            assert_eq!(table.get(a, b), 7);
            assert_eq!(table.get(b, a), 7);
            assert_eq!(table.get(a, c), 0);
        }

        #[test]
        fn step_matches_reference_without_eat() {
            let n = 8;
            let input = Input {
                n,
                m: 5,
                color_count: 3,
                d: vec![1; 5],
                food: vec![0; n * n],
            };
            let state = State::initial(&input);
            let actual = step(&state, n, 3);
            let expected = reference_step(&state, n, 3);
            assert_step_result_eq(&actual, &expected);
        }

        #[test]
        fn step_matches_reference_with_eat() {
            let n = 8;
            let mut food = vec![0; n * n];
            food[Grid::index(Grid::cell(n, 4, 1))] = 2;
            let input = Input {
                n,
                m: 6,
                color_count: 3,
                d: vec![1; 6],
                food,
            };
            let state = State::initial(&input);
            let actual = step(&state, n, 3);
            let expected = reference_step(&state, n, 3);
            assert_step_result_eq(&actual, &expected);
            assert_eq!(actual.state.len(), 6);
            assert_eq!(actual.ate, Some(2));
        }

        #[test]
        fn step_matches_reference_with_bite() {
            let n = 6;
            let food = vec![0; n * n];
            let pos_ij = [
                (2, 2),
                (2, 1),
                (1, 1),
                (1, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (3, 2),
                (3, 1),
            ];
            let colors = [1, 2, 3, 4, 5, 6, 7, 1, 2];
            let state = make_state(n, food, &pos_ij, &colors);
            let actual = step(&state, n, 3);
            let expected = reference_step(&state, n, 3);
            assert_step_result_eq(&actual, &expected);
            assert_eq!(actual.bite_idx, Some(6));
            assert_eq!(actual.dropped.len(), 2);
        }

        #[test]
        fn body_release_dist_allows_late_opening_initial_body_cells() {
            let n = 6;
            let input = Input {
                n,
                m: 5,
                color_count: 3,
                d: vec![1; 5],
                food: vec![0; n * n],
            };
            let state = State::initial(&input);
            let bfs = compute_body_release_dist(&state, n);

            assert_eq!(bfs.start, Grid::cell(n, 4, 0));
            assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 2, 0))], 4);
            assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 3, 0))], 3);
            assert_eq!(
                bfs.prev[Grid::index(Grid::cell(n, 2, 0))],
                Some(Grid::cell(n, 2, 1))
            );
        }

        #[test]
        fn body_release_eat_dist_turns_adjacent_food_into_one_step_goal() {
            let n = 8;
            let mut food = vec![0; n * n];
            food[Grid::index(Grid::cell(n, 4, 1))] = 2;
            let input = Input {
                n,
                m: 6,
                color_count: 3,
                d: vec![1; 6],
                food,
            };
            let state = State::initial(&input);
            let bfs = compute_body_release_dist(&state, n);
            let eat_result = body_release_eat_dist(&bfs, &state, n);

            assert_eq!(bfs.dist[Grid::index(Grid::cell(n, 4, 1))], usize::MAX);
            assert_eq!(eat_result.dist[Grid::index(Grid::cell(n, 4, 1))], 1);
            assert_eq!(
                eat_result.prev[Grid::index(Grid::cell(n, 4, 1))],
                Some(Grid::cell(n, 4, 0))
            );
            assert_eq!(
                reconstruct_cell_search_path(&eat_result, Grid::cell(n, 4, 1)),
                Some(vec![Grid::cell(n, 4, 0), Grid::cell(n, 4, 1)])
            );
        }

        #[test]
        fn random_walk_matches_reference() {
            let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
            let n = 8;
            let mut food = vec![0; n * n];
            for i in 0..n {
                for j in 0..n {
                    if j == 0 && i <= 4 {
                        continue;
                    }
                    if rng.random_range(0..100) < 25 {
                        food[Grid::index(Grid::cell(n, i, j))] = rng.random_range(1..=3);
                    }
                }
            }
            let input = Input {
                n,
                m: 40,
                color_count: 3,
                d: vec![1; 40],
                food,
            };
            let mut state = State::initial(&input);
            assert_state_consistent(&state);

            for _turn in 0..300 {
                let dirs = state.legal_dirs(n);
                let dir = dirs[rng.random_range(0..dirs.len())];
                let actual = step(&state, n, dir);
                let expected = reference_step(&state, n, dir);
                assert_eq!(
                    actual.ate, expected.ate,
                    "ate mismatch: dir={dir}, state={state:?}, actual={actual:?}, expected={expected:?}"
                );
                assert_eq!(
                    actual.bite_idx, expected.bite_idx,
                    "bite_idx mismatch: dir={dir}, state={state:?}, actual={actual:?}, expected={expected:?}"
                );
                assert_step_result_eq(&actual, &expected);
                state = actual.state;
            }
        }
    }

    mod v126raw {
        #![allow(dead_code)]
        #![allow(unused_imports)]

        use std::cmp::Reverse;
        use std::collections::{BinaryHeap, HashMap, HashSet};
        use std::hash::{BuildHasherDefault, Hash, Hasher};
        use std::io::{self, Read};
        use std::time::Instant;

        macro_rules! prof_inc {
            ($($tt:tt)*) => {};
        }

        macro_rules! prof_set {
            ($($tt:tt)*) => {};
        }

        mod profile_counts {
            pub fn dump(_: &str) {}
        }

        const MAX_N: usize = 16;
        const MAX_CELLS: usize = MAX_N * MAX_N;
        const MAX_LEN: usize = MAX_CELLS;
        const MAX_TURNS: usize = 100_000;
        const TIME_LIMIT_SEC: f64 = 1.88;
        const STAGE_BEAM: usize = 5;
        const MAX_TARGETS_PER_STAGE: usize = 10;
        const MAX_TARGETS_ENDGAME: usize = 24;
        const MAX_TARGETS_RESCUE: usize = 28;
        const VISIT_REPEAT_LIMIT: usize = 12;
        const DIRS: [(isize, isize, char); 4] =
            [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
        const BUDGETS_NORMAL: [(usize, usize); 3] = [(2_000, 20), (8_000, 20), (25_000, 24)];
        const BUDGETS_LATE: [(usize, usize); 3] = [(4_000, 20), (12_000, 24), (40_000, 28)];
        const BUDGETS_ENDGAME_LIGHT: [(usize, usize); 2] = [(800, 16), (2_500, 20)];
        const BUDGETS_RESCUE: [(usize, usize); 2] = [(16_000, 24), (60_000, 32)];
        const ENDGAME_REMAINING_FOOD: usize = 18;
        const ENDGAME_ELL_LEFT: usize = 24;
        const LOOKAHEAD_HORIZON: usize = 6;
        const SUFFIX_OPT_WINDOWS: [usize; 4] = [8, 12, 16, 20];
        const SUFFIX_STAGE_BEAM: usize = 5;
        const SUFFIX_OPT_TARGETS: usize = 12;
        const SUFFIX_OPT_MIN_LEFT_SEC: f64 = 0.18;
        const EMPTY_PATH_DEPTH_LIMIT: usize = 64;
        const EMPTY_PATH_EXPANSION_CAP: usize = 140_000;
        const EMPTY_PATH_REMAINING_LIMIT: usize = 12;
        const EMPTY_PATH_MIN_LEFT_SEC: f64 = 0.10;
        const FASTLANE_MIN_LEFT_SEC: f64 = 0.28;
        const FAST_SAFE_DEPTH_LIMIT: u8 = 10;
        const FAST_SAFE_NODE_LIMIT: usize = 2_500;
        const FAST_RESCUE_DEPTH_LIMIT: u8 = 16;
        const FAST_RESCUE_NODE_LIMIT: usize = 9_000;
        const FAST_FALLBACK_TARGETS: usize = 8;

        type Cell = u16;

        type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
        type FxHashSet<T> = HashSet<T, BuildHasherDefault<FxHasher>>;

        #[derive(Default)]
        struct FxHasher {
            hash: u64,
        }

        impl FxHasher {
            #[inline]
            fn mix(&mut self, x: u64) {
                self.hash ^= x;
                self.hash = self.hash.rotate_left(5).wrapping_mul(0x517c_c1b7_2722_0a95);
            }
        }

        impl Hasher for FxHasher {
            #[inline]
            fn finish(&self) -> u64 {
                self.hash
            }

            #[inline]
            fn write(&mut self, bytes: &[u8]) {
                let mut idx = 0usize;
                while idx + 8 <= bytes.len() {
                    let mut chunk = [0u8; 8];
                    chunk.copy_from_slice(&bytes[idx..idx + 8]);
                    self.mix(u64::from_le_bytes(chunk));
                    idx += 8;
                }
                if idx < bytes.len() {
                    let mut tail = [0u8; 8];
                    tail[..bytes.len() - idx].copy_from_slice(&bytes[idx..]);
                    self.mix(u64::from_le_bytes(tail));
                }
            }

            #[inline]
            fn write_u8(&mut self, i: u8) {
                self.mix(i as u64);
            }

            #[inline]
            fn write_u16(&mut self, i: u16) {
                self.mix(i as u64);
            }

            #[inline]
            fn write_u32(&mut self, i: u32) {
                self.mix(i as u64);
            }

            #[inline]
            fn write_u64(&mut self, i: u64) {
                self.mix(i);
            }

            #[inline]
            fn write_u128(&mut self, i: u128) {
                self.mix(i as u64);
                self.mix((i >> 64) as u64);
            }

            #[inline]
            fn write_usize(&mut self, i: usize) {
                self.mix(i as u64);
            }

            #[inline]
            fn write_i8(&mut self, i: i8) {
                self.mix(i as i64 as u64);
            }

            #[inline]
            fn write_i16(&mut self, i: i16) {
                self.mix(i as i64 as u64);
            }

            #[inline]
            fn write_i32(&mut self, i: i32) {
                self.mix(i as i64 as u64);
            }

            #[inline]
            fn write_i64(&mut self, i: i64) {
                self.mix(i as u64);
            }

            #[inline]
            fn write_i128(&mut self, i: i128) {
                self.mix(i as u64);
                self.mix((i >> 64) as u64);
            }

            #[inline]
            fn write_isize(&mut self, i: isize) {
                self.mix(i as i64 as u64);
            }
        }

        const fn splitmix64(mut x: u64) -> u64 {
            x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            x ^ (x >> 31)
        }

        const INTERNAL_POS_HASH_BASE1: u64 = 0x94d0_49bb_1331_11eb;
        const INTERNAL_POS_HASH_BASE2: u64 = 0xbf58_476d_1ce4_e5b9;
        const COLOR_HASH_BASE1: u64 = 0x9e37_79b9_7f4a_7c15;
        const COLOR_HASH_BASE2: u64 = 0x517c_c1b7_2722_0a95;

        const fn build_pow(base: u64) -> [u64; MAX_LEN + 1] {
            let mut out = [0_u64; MAX_LEN + 1];
            let mut idx = 0usize;
            let mut cur = 1_u64;
            while idx <= MAX_LEN {
                out[idx] = cur;
                cur = cur.wrapping_mul(base);
                idx += 1;
            }
            out
        }

        const fn build_food_hash_table() -> [[u64; 8]; MAX_CELLS] {
            let mut out = [[0_u64; 8]; MAX_CELLS];
            let mut cell = 0usize;
            while cell < MAX_CELLS {
                let mut color = 1usize;
                while color < 8 {
                    out[cell][color] =
                        splitmix64(((cell as u64) << 8) ^ color as u64 ^ 0x1234_5678_9abc_def0);
                    color += 1;
                }
                cell += 1;
            }
            out
        }

        const INTERNAL_POS_HASH_POW1: [u64; MAX_LEN + 1] = build_pow(INTERNAL_POS_HASH_BASE1);
        const INTERNAL_POS_HASH_POW2: [u64; MAX_LEN + 1] = build_pow(INTERNAL_POS_HASH_BASE2);
        const COLOR_HASH_POW1: [u64; MAX_LEN + 1] = build_pow(COLOR_HASH_BASE1);
        const COLOR_HASH_POW2: [u64; MAX_LEN + 1] = build_pow(COLOR_HASH_BASE2);
        const FOOD_HASH_TABLE: [[u64; 8]; MAX_CELLS] = build_food_hash_table();

        #[inline]
        const fn encode_internal_pos_hash(cell: Cell) -> u64 {
            splitmix64(cell as u64 ^ 0x243f_6a88_85a3_08d3)
        }

        #[inline]
        const fn encode_color_hash(color: u8) -> u64 {
            splitmix64(color as u64 ^ 0x1319_8a2e_0370_7344)
        }

        #[derive(Clone)]
        struct Input {
            n: usize,
            m: usize,
            d: [u8; MAX_LEN],
            food: [u8; MAX_CELLS],
        }

        #[derive(Clone)]
        struct State {
            n: usize,
            food: [u8; MAX_CELLS],
            remaining_food: usize,
            food_hash: u64,
            pos: InternalPosDeque,
            colors: [u8; MAX_LEN],
            color_hash1: u64,
            color_hash2: u64,
        }

        #[derive(Clone)]
        struct BeamState {
            state: State,
            ops: String,
        }

        #[derive(Clone, Copy, Default)]
        struct Dropped {
            cell: Cell,
            color: u8,
        }

        #[derive(Clone)]
        struct DroppedBuf {
            len: usize,
            entries: [Dropped; MAX_LEN],
        }

        #[derive(Clone)]
        struct DroppedQueue {
            head: usize,
            len: usize,
            buf: [Dropped; MAX_LEN],
        }

        #[derive(Hash, Eq, PartialEq)]
        struct VisitKey {
            head: Cell,
            neck: Cell,
            len: u16,
            goal: Cell,
            restore_len: u16,
        }

        struct Node {
            state: State,
            parent: Option<usize>,
            move_seg: String,
        }

        struct PosNode {
            pos: InternalPosDeque,
            parent: Option<usize>,
            mv: u8,
        }

        #[derive(Clone, Copy)]
        struct QuickPlanConfig {
            depth_limit: u8,
            node_limit: usize,
            non_target_limit: u8,
            bite_limit: u8,
        }

        #[derive(Clone)]
        struct QuickSearchNode {
            state: State,
            parent: usize,
            dir: u8,
            depth: u8,
            non_target: u8,
            bite: u8,
        }

        #[derive(Clone)]
        struct QuickSeenKey {
            state: State,
            non_target: u8,
            bite: u8,
        }

        #[derive(Clone)]
        struct CellList {
            len: usize,
            buf: [Cell; MAX_CELLS],
        }

        #[derive(Clone, Copy)]
        struct SmallDirList {
            len: usize,
            dirs: [u8; 4],
        }

        #[derive(Clone, Copy)]
        struct SmallCellList {
            len: usize,
            cells: [Cell; 4],
        }

        #[derive(Clone)]
        struct InternalPosDeque {
            head: usize,
            len: usize,
            buf: [Cell; MAX_LEN],
            hash1: u64,
            hash2: u64,
        }

        impl DroppedBuf {
            #[inline]
            fn new() -> Self {
                Self {
                    len: 0,
                    entries: [Dropped::default(); MAX_LEN],
                }
            }

            #[inline]
            fn clear(&mut self) {
                self.len = 0;
            }

            #[inline]
            fn push(&mut self, cell: Cell, color: u8) {
                self.entries[self.len] = Dropped { cell, color };
                self.len += 1;
            }

            #[inline]
            fn as_slice(&self) -> &[Dropped] {
                &self.entries[..self.len]
            }
        }

        impl DroppedQueue {
            #[inline]
            fn new() -> Self {
                Self {
                    head: 0,
                    len: 0,
                    buf: [Dropped::default(); MAX_LEN],
                }
            }

            #[inline]
            fn is_empty(&self) -> bool {
                self.len == 0
            }

            #[inline]
            fn push_back(&mut self, x: Dropped) {
                let idx = self.head + self.len;
                let idx = if idx < MAX_LEN { idx } else { idx - MAX_LEN };
                self.buf[idx] = x;
                self.len += 1;
            }

            #[inline]
            fn pop_front(&mut self) -> Option<Dropped> {
                if self.len == 0 {
                    return None;
                }
                let out = self.buf[self.head];
                self.head += 1;
                if self.head == MAX_LEN {
                    self.head = 0;
                }
                self.len -= 1;
                Some(out)
            }

            #[inline]
            fn front(&self) -> Option<&Dropped> {
                if self.len == 0 {
                    None
                } else {
                    Some(&self.buf[self.head])
                }
            }
        }

        impl CellList {
            #[inline]
            fn new() -> Self {
                Self {
                    len: 0,
                    buf: [0; MAX_CELLS],
                }
            }

            #[inline]
            fn push(&mut self, cell: Cell) {
                self.buf[self.len] = cell;
                self.len += 1;
            }

            #[inline]
            fn is_empty(&self) -> bool {
                self.len == 0
            }

            #[inline]
            fn len(&self) -> usize {
                self.len
            }

            #[inline]
            fn truncate(&mut self, new_len: usize) {
                self.len = self.len.min(new_len);
            }

            #[inline]
            fn as_slice(&self) -> &[Cell] {
                &self.buf[..self.len]
            }

            #[inline]
            fn as_mut_slice(&mut self) -> &mut [Cell] {
                &mut self.buf[..self.len]
            }
        }

        impl SmallDirList {
            #[inline]
            fn new() -> Self {
                Self {
                    len: 0,
                    dirs: [0; 4],
                }
            }

            #[inline]
            fn push(&mut self, dir: usize) {
                self.dirs[self.len] = dir as u8;
                self.len += 1;
            }

            #[inline]
            fn as_slice(&self) -> &[u8] {
                &self.dirs[..self.len]
            }
        }

        impl SmallCellList {
            #[inline]
            fn new() -> Self {
                Self {
                    len: 0,
                    cells: [0; 4],
                }
            }

            #[inline]
            fn push(&mut self, cell: Cell) {
                self.cells[self.len] = cell;
                self.len += 1;
            }

            #[inline]
            fn as_slice(&self) -> &[Cell] {
                &self.cells[..self.len]
            }

            #[inline]
            fn as_mut_slice(&mut self) -> &mut [Cell] {
                &mut self.cells[..self.len]
            }
        }

        impl InternalPosDeque {
            #[inline]
            fn new() -> Self {
                Self {
                    head: 0,
                    len: 0,
                    buf: [0; MAX_LEN],
                    hash1: 0,
                    hash2: 0,
                }
            }

            #[inline]
            fn from_slice(cells: &[Cell]) -> Self {
                let mut deque = Self::new();
                deque.buf[..cells.len()].copy_from_slice(cells);
                deque.len = cells.len();
                let mut pow1 = 1_u64;
                let mut pow2 = 1_u64;
                for &cell in cells {
                    let x = encode_internal_pos_hash(cell);
                    deque.hash1 = deque.hash1.wrapping_add(x.wrapping_mul(pow1));
                    deque.hash2 = deque.hash2.wrapping_add(x.wrapping_mul(pow2));
                    pow1 = pow1.wrapping_mul(INTERNAL_POS_HASH_BASE1);
                    pow2 = pow2.wrapping_mul(INTERNAL_POS_HASH_BASE2);
                }
                deque
            }

            #[inline]
            fn len(&self) -> usize {
                self.len
            }

            #[inline]
            fn physical_index(&self, idx: usize) -> usize {
                let raw = self.head + idx;
                if raw < MAX_LEN { raw } else { raw - MAX_LEN }
            }

            #[inline]
            fn head(&self) -> Cell {
                self.buf[self.head]
            }

            #[inline]
            fn back(&self) -> Option<Cell> {
                if self.len == 0 {
                    None
                } else {
                    Some(self[self.len - 1])
                }
            }

            #[inline]
            fn push_front_grow(&mut self, cell: Cell) {
                let x = encode_internal_pos_hash(cell);
                self.head = if self.head == 0 {
                    MAX_LEN - 1
                } else {
                    self.head - 1
                };
                self.buf[self.head] = cell;
                self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(INTERNAL_POS_HASH_BASE1));
                self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(INTERNAL_POS_HASH_BASE2));
                self.len += 1;
            }

            #[inline]
            fn push_front_pop_back(&mut self, cell: Cell) {
                let len = self.len;
                let tail = self[len - 1];
                let tail_hash = encode_internal_pos_hash(tail);
                let x = encode_internal_pos_hash(cell);

                self.head = if self.head == 0 {
                    MAX_LEN - 1
                } else {
                    self.head - 1
                };
                self.buf[self.head] = cell;

                self.hash1 = x.wrapping_add(
                    self.hash1
                        .wrapping_sub(tail_hash.wrapping_mul(INTERNAL_POS_HASH_POW1[len - 1]))
                        .wrapping_mul(INTERNAL_POS_HASH_BASE1),
                );
                self.hash2 = x.wrapping_add(
                    self.hash2
                        .wrapping_sub(tail_hash.wrapping_mul(INTERNAL_POS_HASH_POW2[len - 1]))
                        .wrapping_mul(INTERNAL_POS_HASH_BASE2),
                );
            }

            #[inline]
            fn truncate(&mut self, new_len: usize) {
                if new_len >= self.len {
                    return;
                }
                for idx in new_len..self.len {
                    let x = encode_internal_pos_hash(self[idx]);
                    self.hash1 = self
                        .hash1
                        .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW1[idx]));
                    self.hash2 = self
                        .hash2
                        .wrapping_sub(x.wrapping_mul(INTERNAL_POS_HASH_POW2[idx]));
                }
                self.len = new_len;
            }
        }

        impl std::ops::Index<usize> for InternalPosDeque {
            type Output = Cell;

            #[inline]
            fn index(&self, idx: usize) -> &Self::Output {
                &self.buf[self.physical_index(idx)]
            }
        }

        impl PartialEq for InternalPosDeque {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                if self.len != other.len || self.hash1 != other.hash1 || self.hash2 != other.hash2 {
                    return false;
                }
                for idx in 0..self.len {
                    if self[idx] != other[idx] {
                        return false;
                    }
                }
                true
            }
        }

        impl Eq for InternalPosDeque {}

        impl Hash for InternalPosDeque {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.len.hash(state);
                self.hash1.hash(state);
                self.hash2.hash(state);
            }
        }

        impl std::fmt::Debug for InternalPosDeque {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut list = f.debug_list();
                for idx in 0..self.len {
                    list.entry(&self[idx]);
                }
                list.finish()
            }
        }

        impl State {
            fn initial(input: &Input) -> Self {
                let food = input.food;
                let mut food_hash = 0_u64;
                let mut remaining_food = 0usize;
                for idx in 0..input.n * input.n {
                    let color = food[idx];
                    if color != 0 {
                        food_hash ^= FOOD_HASH_TABLE[idx][color as usize];
                        remaining_food += 1;
                    }
                }

                let pos = InternalPosDeque::from_slice(&[
                    cell_of(4, 0, input.n),
                    cell_of(3, 0, input.n),
                    cell_of(2, 0, input.n),
                    cell_of(1, 0, input.n),
                    cell_of(0, 0, input.n),
                ]);

                let mut colors = [0_u8; MAX_LEN];
                let mut color_hash1 = 0_u64;
                let mut color_hash2 = 0_u64;
                for idx in 0..5 {
                    colors[idx] = 1;
                    let x = encode_color_hash(1);
                    color_hash1 = color_hash1.wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
                    color_hash2 = color_hash2.wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
                }

                Self {
                    n: input.n,
                    food,
                    remaining_food,
                    food_hash,
                    pos,
                    colors,
                    color_hash1,
                    color_hash2,
                }
            }

            #[inline]
            fn len(&self) -> usize {
                self.pos.len()
            }

            #[inline]
            fn head(&self) -> Cell {
                self.pos.head()
            }

            #[inline]
            fn neck(&self) -> Cell {
                if self.len() >= 2 {
                    self.pos[1]
                } else {
                    self.head()
                }
            }

            #[inline]
            fn set_food(&mut self, cell: Cell, new_color: u8) {
                let idx = cell as usize;
                let old_color = self.food[idx];
                if old_color == new_color {
                    return;
                }
                if old_color != 0 {
                    self.food_hash ^= FOOD_HASH_TABLE[idx][old_color as usize];
                    self.remaining_food -= 1;
                }
                if new_color != 0 {
                    self.food_hash ^= FOOD_HASH_TABLE[idx][new_color as usize];
                    self.remaining_food += 1;
                }
                self.food[idx] = new_color;
            }

            #[inline]
            fn append_color_at(&mut self, idx: usize, color: u8) {
                self.colors[idx] = color;
                let x = encode_color_hash(color);
                self.color_hash1 = self
                    .color_hash1
                    .wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
                self.color_hash2 = self
                    .color_hash2
                    .wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
            }

            #[inline]
            fn truncate_colors(&mut self, new_len: usize, old_len: usize) {
                for idx in new_len..old_len {
                    let x = encode_color_hash(self.colors[idx]);
                    self.color_hash1 = self
                        .color_hash1
                        .wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
                    self.color_hash2 = self
                        .color_hash2
                        .wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
                }
            }
        }

        impl PartialEq for State {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                if self.n != other.n
                    || self.remaining_food != other.remaining_food
                    || self.food_hash != other.food_hash
                    || self.pos != other.pos
                    || self.color_hash1 != other.color_hash1
                    || self.color_hash2 != other.color_hash2
                {
                    return false;
                }
                if self.food != other.food {
                    return false;
                }
                let len = self.len();
                for idx in 0..len {
                    if self.colors[idx] != other.colors[idx] {
                        return false;
                    }
                }
                true
            }
        }

        impl Eq for State {}

        impl Hash for State {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.n.hash(state);
                self.remaining_food.hash(state);
                self.food_hash.hash(state);
                self.pos.hash(state);
                self.color_hash1.hash(state);
                self.color_hash2.hash(state);
            }
        }

        impl PartialEq for QuickSeenKey {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.non_target == other.non_target
                    && self.bite == other.bite
                    && self.state == other.state
            }
        }

        impl Eq for QuickSeenKey {}

        impl Hash for QuickSeenKey {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.state.hash(state);
                self.non_target.hash(state);
                self.bite.hash(state);
            }
        }

        #[inline]
        fn cell_of(r: usize, c: usize, n: usize) -> Cell {
            (r * n + c) as Cell
        }

        #[inline]
        fn rc_of(cell: Cell, n: usize) -> (usize, usize) {
            let x = cell as usize;
            (x / n, x % n)
        }

        #[inline]
        fn manhattan(n: usize, a: Cell, b: Cell) -> usize {
            let (ar, ac) = rc_of(a, n);
            let (br, bc) = rc_of(b, n);
            ar.abs_diff(br) + ac.abs_diff(bc)
        }

        #[inline]
        fn time_over(started: &Instant) -> bool {
            let over = started.elapsed().as_secs_f64() >= TIME_LIMIT_SEC;
            if over {
                prof_inc!(time_over_hits);
            }
            over
        }

        #[inline]
        fn time_left(started: &Instant) -> f64 {
            (TIME_LIMIT_SEC - started.elapsed().as_secs_f64()).max(0.0)
        }

        #[inline]
        fn dir_of_char(ch: u8) -> Option<usize> {
            match ch {
                b'U' => Some(0),
                b'D' => Some(1),
                b'L' => Some(2),
                b'R' => Some(3),
                _ => None,
            }
        }

        fn read_input() -> Input {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).unwrap();
            let mut it = s.split_whitespace();

            let n: usize = it.next().unwrap().parse().unwrap();
            let m: usize = it.next().unwrap().parse().unwrap();
            let _c: usize = it.next().unwrap().parse().unwrap();

            let mut d = [0_u8; MAX_LEN];
            for x in d.iter_mut().take(m) {
                *x = it.next().unwrap().parse::<u8>().unwrap();
            }

            let mut food = [0_u8; MAX_CELLS];
            for r in 0..n {
                for c in 0..n {
                    food[r * n + c] = it.next().unwrap().parse::<u8>().unwrap();
                }
            }

            Input { n, m, d, food }
        }

        #[inline]
        fn dir_between_cells(n: usize, a: Cell, b: Cell) -> Option<usize> {
            let (ar, ac) = rc_of(a, n);
            let (br, bc) = rc_of(b, n);
            if ar == br + 1 && ac == bc {
                return Some(0);
            }
            if ar + 1 == br && ac == bc {
                return Some(1);
            }
            if ar == br && ac == bc + 1 {
                return Some(2);
            }
            if ar == br && ac + 1 == bc {
                return Some(3);
            }
            None
        }

        #[inline]
        fn next_head_cell(st: &State, dir: usize) -> Option<Cell> {
            let head = st.head() as usize;
            match dir {
                0 => (head >= st.n).then_some((head - st.n) as Cell),
                1 => (head + st.n < st.n * st.n).then_some((head + st.n) as Cell),
                2 => (head % st.n != 0).then_some((head - 1) as Cell),
                3 => (head % st.n + 1 < st.n).then_some((head + 1) as Cell),
                _ => None,
            }
        }

        #[inline]
        fn next_head_cell_pos(n: usize, pos: &InternalPosDeque, dir: usize) -> Option<Cell> {
            let head = pos.head() as usize;
            match dir {
                0 => (head >= n).then_some((head - n) as Cell),
                1 => (head + n < n * n).then_some((head + n) as Cell),
                2 => (head % n != 0).then_some((head - 1) as Cell),
                3 => (head % n + 1 < n).then_some((head + 1) as Cell),
                _ => None,
            }
        }

        #[inline]
        fn is_legal_dir(st: &State, dir: usize) -> bool {
            let Some(nh) = next_head_cell(st, dir) else {
                return false;
            };
            st.len() < 2 || nh != st.neck()
        }

        #[inline]
        fn legal_dirs(st: &State) -> SmallDirList {
            let mut out = SmallDirList::new();
            for dir in 0..4 {
                if is_legal_dir(st, dir) {
                    out.push(dir);
                }
            }
            out
        }

        #[inline]
        fn legal_dir_count(st: &State) -> usize {
            let mut cnt = 0usize;
            for dir in 0..4 {
                if is_legal_dir(st, dir) {
                    cnt += 1;
                }
            }
            cnt
        }

        #[inline]
        fn find_internal_bite_idx(pos: &InternalPosDeque) -> Option<usize> {
            if pos.len() <= 2 {
                return None;
            }
            let head = pos[0];
            for idx in 1..pos.len() - 1 {
                if pos[idx] == head {
                    return Some(idx);
                }
            }
            None
        }

        fn step_impl(
            st: &State,
            dir: usize,
            mut dropped: Option<&mut DroppedBuf>,
        ) -> (State, u8, Option<usize>) {
            prof_inc!(step_calls);
            let nh = next_head_cell(st, dir).unwrap();
            let old_len = st.len();
            let mut ns = st.clone();

            let ate = ns.food[nh as usize];
            if ate != 0 {
                prof_inc!(step_ate);
                ns.set_food(nh, 0);
                ns.pos.push_front_grow(nh);
                ns.append_color_at(old_len, ate);
            } else {
                ns.pos.push_front_pop_back(nh);
            }

            let bite_idx = find_internal_bite_idx(&ns.pos);
            if let Some(bi) = bite_idx {
                prof_inc!(step_bite);
                if let Some(buf) = &mut dropped {
                    buf.clear();
                }
                let cur_len = ns.len();
                for p in bi + 1..cur_len {
                    let cell = ns.pos[p];
                    let color = ns.colors[p];
                    ns.set_food(cell, color);
                    if let Some(buf) = &mut dropped {
                        buf.push(cell, color);
                    }
                }
                ns.truncate_colors(bi + 1, cur_len);
                ns.pos.truncate(bi + 1);
            } else if let Some(buf) = &mut dropped {
                buf.clear();
            }

            (ns, ate, bite_idx)
        }

        #[inline]
        fn step(st: &State, dir: usize) -> (State, u8, Option<usize>) {
            step_impl(st, dir, None)
        }

        #[inline]
        fn step_with_dropped(
            st: &State,
            dir: usize,
            dropped: &mut DroppedBuf,
        ) -> (State, u8, Option<usize>) {
            step_impl(st, dir, Some(dropped))
        }

        #[inline]
        fn lcp_state(st: &State, input: &Input) -> usize {
            let mut i = 0usize;
            let m = st.len().min(input.m);
            while i < m && st.colors[i] == input.d[i] {
                i += 1;
            }
            i
        }

        #[inline]
        fn prefix_ok(st: &State, input: &Input, ell: usize) -> bool {
            let keep = st.len().min(ell);
            for idx in 0..keep {
                if st.colors[idx] != input.d[idx] {
                    return false;
                }
            }
            true
        }

        #[inline]
        fn exact_prefix(st: &State, input: &Input, ell: usize) -> bool {
            st.len() == ell && prefix_ok(st, input, ell)
        }

        #[inline]
        fn remaining_food_count(st: &State) -> usize {
            st.remaining_food
        }

        fn nearest_food_dist(st: &State, color: u8) -> (usize, usize) {
            let head = st.head();
            let mut best = usize::MAX;
            let mut cnt = 0usize;
            for idx in 0..st.n * st.n {
                if st.food[idx] == color {
                    cnt += 1;
                    let dist = manhattan(st.n, head, idx as Cell);
                    if dist < best {
                        best = dist;
                    }
                }
            }
            if best == usize::MAX {
                (1_000_000_000, cnt)
            } else {
                (best, cnt)
            }
        }

        fn target_adjacent(st: &State, target: u8) -> Option<usize> {
            let neck = st.neck();
            for dir in 0..4 {
                let Some(nh) = next_head_cell(st, dir) else {
                    continue;
                };
                if nh == neck {
                    continue;
                }
                if st.food[nh as usize] == target {
                    return Some(dir);
                }
            }
            None
        }

        fn target_suffix_info(st: &State, ell: usize, target: u8) -> Option<(usize, usize)> {
            let head = st.head();
            let len = st.len();
            let mut best: Option<(usize, usize)> = None;
            for idx in ell..len {
                if st.colors[idx] != target {
                    continue;
                }
                let prev = st.pos[idx - 1];
                let cand = (manhattan(st.n, head, prev), idx);
                if best.is_none() || cand < best.unwrap() {
                    best = Some(cand);
                }
            }
            best
        }

        fn local_score(
            st: &State,
            input: &Input,
            ell: usize,
        ) -> (usize, usize, usize, usize, usize) {
            let target = input.d[ell];
            if exact_prefix(st, input, ell) {
                let (dist, _) = nearest_food_dist(st, target);
                let adj = target_adjacent(st, target).is_some();
                return (0, if adj { 0 } else { 1 }, dist, 0, st.len() - ell);
            }

            if let Some((dist, idx)) = target_suffix_info(st, ell, target) {
                return (1, 0, dist, idx - ell, st.len() - ell);
            }

            let (dist, _) = nearest_food_dist(st, target);
            (2, 0, dist, 0, st.len().saturating_sub(ell))
        }

        fn next_stage_rank(st: &State, input: &Input, ellp1: usize) -> (usize, usize, usize) {
            if ellp1 >= input.m {
                return (0, 0, 0);
            }
            let (dist, _) = nearest_food_dist(st, input.d[ellp1]);
            let (hr, hc) = rc_of(st.head(), st.n);
            let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
            (dist, center, 0)
        }

        fn greedy_future_lb_from_cell(
            st: &State,
            input: &Input,
            start_cell: Cell,
            start_ell: usize,
            horizon: usize,
            banned: Option<Cell>,
        ) -> (usize, usize, usize) {
            let mut cur = start_cell;
            let end = (start_ell + horizon).min(input.m);
            let mut used = CellList::new();
            if let Some(b) = banned {
                used.push(b);
            }

            let mut miss = 0usize;
            let mut first = 0usize;
            let mut total = 0usize;

            for idx in start_ell..end {
                let color = input.d[idx];
                let mut best: Option<(usize, Cell)> = None;
                for cell_idx in 0..st.n * st.n {
                    if st.food[cell_idx] != color {
                        continue;
                    }
                    let cell = cell_idx as Cell;
                    if used.as_slice().iter().any(|&x| x == cell) {
                        continue;
                    }
                    let dist = manhattan(st.n, cur, cell);
                    if best.is_none() || dist < best.unwrap().0 {
                        best = Some((dist, cell));
                    }
                }

                if let Some((dist, cell)) = best {
                    if idx == start_ell {
                        first = dist;
                    }
                    total += dist;
                    cur = cell;
                    used.push(cell);
                } else {
                    miss += 1;
                    let penalty = st.n * st.n;
                    if idx == start_ell {
                        first = penalty;
                    }
                    total += penalty;
                }
            }

            (miss, first, total)
        }

        fn turn_focus_next_stage_rank(
            st: &State,
            input: &Input,
            ellp1: usize,
        ) -> (usize, usize, usize, usize, usize) {
            if ellp1 >= input.m {
                return (0, 0, 0, 0, 0);
            }
            let (miss, first, total) =
                greedy_future_lb_from_cell(st, input, st.head(), ellp1, LOOKAHEAD_HORIZON, None);
            let adj = target_adjacent(st, input.d[ellp1]).is_some();
            let (hr, hc) = rc_of(st.head(), st.n);
            let center = hr.abs_diff(st.n / 2) + hc.abs_diff(st.n / 2);
            let mobility_penalty = 4usize.saturating_sub(legal_dir_count(st));
            (
                miss,
                usize::from(!adj),
                total,
                first,
                center + mobility_penalty,
            )
        }

        fn target_candidate_rank(
            st: &State,
            input: &Input,
            ell: usize,
            target: Cell,
        ) -> (usize, usize, usize, usize, usize) {
            let head = st.head();
            let capture_lb = manhattan(st.n, head, target);
            let (miss, first, total) = greedy_future_lb_from_cell(
                st,
                input,
                target,
                ell + 1,
                LOOKAHEAD_HORIZON,
                Some(target),
            );

            let mut goal_blocked = 1usize;
            for &nb in neighbors(st.n, target).as_slice() {
                if st.food[nb as usize] == 0 || nb == head {
                    goal_blocked = 0;
                    break;
                }
            }

            (miss, capture_lb + total, first, goal_blocked, capture_lb)
        }

        fn final_rank(bs: &BeamState, input: &Input) -> (usize, Reverse<usize>, Reverse<usize>) {
            (
                lcp_state(&bs.state, input),
                Reverse(remaining_food_count(&bs.state)),
                Reverse(bs.ops.len()),
            )
        }

        fn reconstruct_plan(nodes: &[Node], mut idx: usize) -> String {
            let mut rev = Vec::new();
            while let Some(parent) = nodes[idx].parent {
                rev.push(nodes[idx].move_seg.clone());
                idx = parent;
            }
            rev.reverse();

            let mut out = String::new();
            for seg in rev {
                out.push_str(&seg);
            }
            out
        }

        fn reconstruct_quick_plan(nodes: &[QuickSearchNode], mut idx: usize) -> String {
            let mut rev = Vec::new();
            while nodes[idx].parent != usize::MAX {
                rev.push(DIRS[nodes[idx].dir as usize].2);
                idx = nodes[idx].parent;
            }
            rev.reverse();
            rev.into_iter().collect()
        }

        fn plan_color_goal_quick(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target_color: u8,
            cfg: QuickPlanConfig,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(plan_quick_calls);
            let root = QuickSearchNode {
                state: bs.state.clone(),
                parent: usize::MAX,
                dir: 0,
                depth: 0,
                non_target: 0,
                bite: 0,
            };
            let mut nodes = vec![root];
            let mut q = Vec::with_capacity(cfg.node_limit.min(16_384));
            let mut q_head = 0usize;
            q.push(0usize);

            let mut seen = FxHashMap::<QuickSeenKey, u8>::default();
            seen.insert(
                QuickSeenKey {
                    state: nodes[0].state.clone(),
                    non_target: 0,
                    bite: 0,
                },
                0,
            );

            while q_head < q.len() {
                let cur_idx = q[q_head];
                q_head += 1;

                prof_inc!(plan_quick_expansions);
                if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
                    return None;
                }
                let cur_depth = nodes[cur_idx].depth;
                if cur_depth >= cfg.depth_limit {
                    continue;
                }

                let dirs = legal_dirs(&nodes[cur_idx].state);
                for &dir_u8 in dirs.as_slice() {
                    let dir = dir_u8 as usize;
                    let (sim, ate, bite_idx) = step(&nodes[cur_idx].state, dir);

                    let keep = sim.len().min(ell);
                    let mut ok = true;
                    for idx in 0..keep {
                        if sim.colors[idx] != input.d[idx] {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        continue;
                    }

                    let mut non_target = nodes[cur_idx].non_target;
                    if ate != 0 && ate != target_color {
                        if non_target >= cfg.non_target_limit {
                            continue;
                        }
                        non_target += 1;
                    }

                    let mut bite = nodes[cur_idx].bite;
                    if bite_idx.is_some() {
                        if bite >= cfg.bite_limit {
                            continue;
                        }
                        bite += 1;
                    }

                    let next_depth = cur_depth + 1;
                    let child = QuickSearchNode {
                        state: sim,
                        parent: cur_idx,
                        dir: dir as u8,
                        depth: next_depth,
                        non_target,
                        bite,
                    };

                    if child.state.len() >= ell + 1 {
                        let mut good = true;
                        for idx in 0..=ell {
                            if child.state.colors[idx] != input.d[idx] {
                                good = false;
                                break;
                            }
                        }
                        if good {
                            nodes.push(child);
                            let goal_idx = nodes.len() - 1;
                            let mut ops = bs.ops.clone();
                            ops.push_str(&reconstruct_quick_plan(&nodes, goal_idx));
                            prof_inc!(plan_quick_success);
                            return Some(BeamState {
                                state: nodes[goal_idx].state.clone(),
                                ops,
                            });
                        }
                    }

                    if nodes.len() >= cfg.node_limit {
                        return None;
                    }

                    let key = QuickSeenKey {
                        state: child.state.clone(),
                        non_target,
                        bite,
                    };
                    if seen
                        .get(&key)
                        .is_some_and(|&best_depth| best_depth <= next_depth)
                    {
                        continue;
                    }
                    seen.insert(key, next_depth);
                    nodes.push(child);
                    q.push(nodes.len() - 1);
                }
            }

            None
        }

        fn extend_fastlane_one(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(fastlane_calls);
            if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
                return None;
            }

            let target_color = input.d[ell];
            if collect_food_cells(&bs.state, target_color).is_empty() {
                return None;
            }

            let safe_cfg = QuickPlanConfig {
                depth_limit: FAST_SAFE_DEPTH_LIMIT,
                node_limit: FAST_SAFE_NODE_LIMIT,
                non_target_limit: 0,
                bite_limit: 0,
            };
            if let Some(sol) =
                plan_color_goal_quick(bs, input, ell, target_color, safe_cfg, started)
            {
                prof_inc!(fastlane_success);
                return Some(sol);
            }

            if time_over(started) || time_left(started) < FASTLANE_MIN_LEFT_SEC {
                return None;
            }

            let rescue_cfg = QuickPlanConfig {
                depth_limit: FAST_RESCUE_DEPTH_LIMIT,
                node_limit: FAST_RESCUE_NODE_LIMIT,
                non_target_limit: 6,
                bite_limit: 2,
            };
            if let Some(sol) =
                plan_color_goal_quick(bs, input, ell, target_color, rescue_cfg, started)
            {
                prof_inc!(fastlane_success);
                return Some(sol);
            }

            let sols = collect_exact_solutions(
                bs,
                input,
                ell,
                target_color,
                FAST_FALLBACK_TARGETS,
                started,
            );
            let out = sols.into_iter().min_by_key(|cand| cand.ops.len());
            if out.is_some() {
                prof_inc!(fastlane_success);
            }
            out
        }

        fn try_recover_exact(
            st: &State,
            input: &Input,
            ell: usize,
            dropped: &DroppedBuf,
        ) -> Option<(State, String)> {
            prof_inc!(try_recover_exact_calls);
            let mut s = st.clone();
            let need_cnt = ell.checked_sub(s.len())?;
            if dropped.len < need_cnt {
                return None;
            }

            let mut ops = String::with_capacity(need_cnt);
            for ent in dropped.as_slice().iter().take(need_cnt) {
                let need = input.d[s.len()];
                if ent.color != need {
                    return None;
                }
                let dir = dir_between_cells(s.n, s.head(), ent.cell)?;
                if s.len() >= 2 && ent.cell == s.neck() {
                    return None;
                }
                if s.food[ent.cell as usize] != need {
                    return None;
                }

                let (ns, ate, bite_idx) = step(&s, dir);
                if ate != need || bite_idx.is_some() {
                    return None;
                }
                s = ns;
                ops.push(DIRS[dir].2);
                prof_inc!(try_recover_exact_steps);
            }

            if exact_prefix(&s, input, ell) {
                prof_inc!(try_recover_exact_success);
                Some((s, ops))
            } else {
                None
            }
        }

        fn stage_search_bestfirst(
            start_bs: &BeamState,
            input: &Input,
            ell: usize,
            budgets: &[(usize, usize)],
            keep_solutions: usize,
            started: &Instant,
        ) -> Vec<BeamState> {
            prof_inc!(stage_search_calls);
            if budgets.is_empty() {
                return Vec::new();
            }
            let start = start_bs.state.clone();

            let max_expansions = budgets[budgets.len() - 1].0;
            let mut nodes = Vec::with_capacity(max_expansions.min(30_000) + 8);
            nodes.push(Node {
                state: start.clone(),
                parent: None,
                move_seg: String::new(),
            });

            let mut uid = 0usize;
            let mut pq = BinaryHeap::new();
            pq.push(Reverse((
                local_score(&start, input, ell),
                0usize,
                uid,
                0usize,
            )));
            uid += 1;

            let mut seen = FxHashMap::<State, usize>::default();
            seen.insert(start, 0);

            let mut sols: Vec<BeamState> = Vec::new();
            let mut solkeys: FxHashSet<State> = FxHashSet::default();
            let mut expansions = 0usize;
            let mut stage_idx = 0usize;
            let mut stage_limit = budgets[0].0;
            let mut extra_limit = budgets[0].1;
            let final_limit = budgets[budgets.len() - 1].0;
            let mut dropped1 = DroppedBuf::new();
            let mut dropped2 = DroppedBuf::new();

            while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
                prof_inc!(stage_search_expansions);
                if expansions >= final_limit || sols.len() >= keep_solutions || time_over(started) {
                    break;
                }

                expansions += 1;
                let st = nodes[idx].state.clone();

                if exact_prefix(&st, input, ell) {
                    if let Some(dir2) = target_adjacent(&st, input.d[ell]) {
                        let (ns2, _, bite2) = step(&st, dir2);
                        if bite2.is_none() && exact_prefix(&ns2, input, ell + 1) {
                            if solkeys.insert(ns2.clone()) {
                                let mut plan = reconstruct_plan(&nodes, idx);
                                plan.push(DIRS[dir2].2);

                                let mut ops = start_bs.ops.clone();
                                ops.push_str(&plan);
                                sols.push(BeamState { state: ns2, ops });
                                prof_inc!(stage_search_solution_hits);
                                if sols.len() >= keep_solutions {
                                    break;
                                }
                            }
                        }
                    }
                }
                if sols.len() >= keep_solutions {
                    break;
                }

                {
                    let mut prefix_plan: Option<String> = None;
                    let dirs1 = legal_dirs(&st);
                    for &dir1_u8 in dirs1.as_slice() {
                        let dir1 = dir1_u8 as usize;
                        let (ns1, _, bite1) = step_with_dropped(&st, dir1, &mut dropped1);
                        if bite1.is_none() || !prefix_ok(&ns1, input, ell) {
                            continue;
                        }

                        let mut rs = ns1;
                        let mut recover_ops = String::new();
                        if rs.len() < ell {
                            let Some((rec_state, rec_ops)) =
                                try_recover_exact(&rs, input, ell, &dropped1)
                            else {
                                continue;
                            };
                            rs = rec_state;
                            recover_ops = rec_ops;
                        }

                        if !exact_prefix(&rs, input, ell) {
                            continue;
                        }

                        let dirs2 = legal_dirs(&rs);
                        for &dir2_u8 in dirs2.as_slice() {
                            let dir2 = dir2_u8 as usize;
                            let nh = next_head_cell(&rs, dir2).unwrap();
                            if rs.food[nh as usize] != input.d[ell] {
                                continue;
                            }

                            let (ns2, _, bite2) = step(&rs, dir2);
                            if bite2.is_some() || !exact_prefix(&ns2, input, ell + 1) {
                                continue;
                            }

                            if !solkeys.insert(ns2.clone()) {
                                continue;
                            }

                            let mut plan = prefix_plan.clone().unwrap_or_else(|| {
                                let s = reconstruct_plan(&nodes, idx);
                                prefix_plan = Some(s.clone());
                                s
                            });
                            plan.push(DIRS[dir1].2);
                            plan.push_str(&recover_ops);
                            plan.push(DIRS[dir2].2);

                            let mut ops = start_bs.ops.clone();
                            ops.push_str(&plan);
                            sols.push(BeamState { state: ns2, ops });
                            prof_inc!(stage_search_solution_hits);
                            if sols.len() >= keep_solutions {
                                break;
                            }
                        }
                        if sols.len() >= keep_solutions {
                            break;
                        }
                    }
                }
                if sols.len() >= keep_solutions {
                    break;
                }

                let dirs = legal_dirs(&st);
                for &dir_u8 in dirs.as_slice() {
                    let dir = dir_u8 as usize;
                    let (mut ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped2);
                    let mut seg = String::new();
                    seg.push(DIRS[dir].2);

                    if bite_idx.is_some() && ns.len() < ell {
                        if !prefix_ok(&ns, input, ell) {
                            continue;
                        }
                        let Some((rec_state, rec_ops)) =
                            try_recover_exact(&ns, input, ell, &dropped2)
                        else {
                            continue;
                        };
                        ns = rec_state;
                        seg.push_str(&rec_ops);
                    }

                    if !prefix_ok(&ns, input, ell) {
                        continue;
                    }
                    if ns.len() > ell + extra_limit {
                        continue;
                    }

                    let nd = depth + seg.len();
                    if seen.get(&ns).copied().unwrap_or(usize::MAX) <= nd {
                        continue;
                    }
                    seen.insert(ns.clone(), nd);

                    let child = nodes.len();
                    nodes.push(Node {
                        state: ns.clone(),
                        parent: Some(idx),
                        move_seg: seg,
                    });
                    pq.push(Reverse((local_score(&ns, input, ell), nd, uid, child)));
                    uid += 1;
                }

                if !exact_prefix(&st, input, ell) {
                    let dirs = legal_dirs(&st);
                    for &dir_u8 in dirs.as_slice() {
                        let dir = dir_u8 as usize;
                        let (ns, _, bite_idx) = step_with_dropped(&st, dir, &mut dropped1);
                        if bite_idx.is_none() || !prefix_ok(&ns, input, ell) {
                            continue;
                        }

                        let mut rs = ns;
                        let mut seg = String::new();
                        seg.push(DIRS[dir].2);

                        if rs.len() < ell {
                            let Some((rec_state, rec_ops)) =
                                try_recover_exact(&rs, input, ell, &dropped1)
                            else {
                                continue;
                            };
                            rs = rec_state;
                            seg.push_str(&rec_ops);
                        }

                        if !exact_prefix(&rs, input, ell) {
                            continue;
                        }

                        let nd = depth + seg.len();
                        if seen.get(&rs).copied().unwrap_or(usize::MAX) <= nd {
                            continue;
                        }
                        seen.insert(rs.clone(), nd);

                        let child = nodes.len();
                        nodes.push(Node {
                            state: rs.clone(),
                            parent: Some(idx),
                            move_seg: seg,
                        });
                        pq.push(Reverse((local_score(&rs, input, ell), nd, uid, child)));
                        uid += 1;
                    }
                }

                if expansions >= stage_limit {
                    if !sols.is_empty() {
                        break;
                    }
                    if stage_idx + 1 < budgets.len() {
                        stage_idx += 1;
                        stage_limit = budgets[stage_idx].0;
                        extra_limit = budgets[stage_idx].1;
                    }
                }
            }

            sols.sort_unstable_by_key(|bs| {
                (next_stage_rank(&bs.state, input, ell + 1), bs.ops.len())
            });

            let mut out = Vec::with_capacity(keep_solutions);
            let mut seen2: FxHashSet<State> = FxHashSet::default();
            for bs in sols {
                if seen2.insert(bs.state.clone()) {
                    out.push(bs);
                    if out.len() >= keep_solutions {
                        break;
                    }
                }
            }
            out
        }

        fn collect_food_cells(st: &State, color: u8) -> CellList {
            let mut out = CellList::new();
            for idx in 0..st.n * st.n {
                if st.food[idx] == color {
                    out.push(idx as Cell);
                }
            }
            out
        }

        fn neighbors(n: usize, cid: Cell) -> SmallCellList {
            let (r, c) = rc_of(cid, n);
            let mut out = SmallCellList::new();
            for (dr, dc, _) in DIRS {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0 && nr < n as isize && nc >= 0 && nc < n as isize {
                    out.push(cell_of(nr as usize, nc as usize, n));
                }
            }
            out
        }

        #[inline]
        fn can_reach_target_next_pos(n: usize, pos: &InternalPosDeque, target: Cell) -> bool {
            dir_between_cells(n, pos[0], target).is_some() && (pos.len() < 2 || target != pos[1])
        }

        fn empty_step_pos(
            n: usize,
            food: &[u8; MAX_CELLS],
            pos: &InternalPosDeque,
            dir: usize,
            target: Cell,
        ) -> Option<InternalPosDeque> {
            let nh = next_head_cell_pos(n, pos, dir)?;
            if pos.len() >= 2 && nh == pos[1] {
                return None;
            }
            if nh == target || food[nh as usize] != 0 {
                return None;
            }

            let mut new_pos = pos.clone();
            new_pos.push_front_pop_back(nh);
            if find_internal_bite_idx(&new_pos).is_some() {
                return None;
            }
            Some(new_pos)
        }

        fn reachable_goal_neighbor_count_pos(
            n: usize,
            pos: &InternalPosDeque,
            target: Cell,
        ) -> usize {
            let mut blocked = [false; MAX_CELLS];
            if pos.len() >= 3 {
                for idx in 1..pos.len() - 1 {
                    blocked[pos[idx] as usize] = true;
                }
            }

            let start = pos[0];
            let mut seen = [false; MAX_CELLS];
            let mut q = [0_u16; MAX_CELLS];
            let mut q_head = 0usize;
            let mut q_tail = 0usize;

            seen[start as usize] = true;
            q[q_tail] = start;
            q_tail += 1;

            while q_head < q_tail {
                let cur = q[q_head];
                q_head += 1;
                let (r, c) = rc_of(cur, n);
                for (dr, dc, _) in DIRS {
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                        continue;
                    }
                    let nxt = cell_of(nr as usize, nc as usize, n);
                    let idx = nxt as usize;
                    if blocked[idx] || seen[idx] {
                        continue;
                    }
                    seen[idx] = true;
                    q[q_tail] = nxt;
                    q_tail += 1;
                }
            }

            let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
            let mut cnt = 0usize;
            for &nb in neighbors(n, target).as_slice() {
                if nb != neck && seen[nb as usize] {
                    cnt += 1;
                }
            }
            cnt
        }

        fn legal_dir_count_pos(n: usize, pos: &InternalPosDeque) -> usize {
            let (hr, hc) = rc_of(pos[0], n);
            let neck = if pos.len() >= 2 { pos[1] } else { u16::MAX };
            let mut cnt = 0usize;
            for (dr, dc, _) in DIRS {
                let nr = hr as isize + dr;
                let nc = hc as isize + dc;
                if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                    continue;
                }
                let nh = cell_of(nr as usize, nc as usize, n);
                if nh != neck {
                    cnt += 1;
                }
            }
            cnt
        }

        #[inline]
        fn empty_path_rank(
            n: usize,
            pos: &InternalPosDeque,
            target: Cell,
        ) -> (usize, usize, usize, usize) {
            (
                usize::from(!can_reach_target_next_pos(n, pos, target)),
                manhattan(n, pos[0], target),
                4usize.saturating_sub(reachable_goal_neighbor_count_pos(n, pos, target)),
                4usize.saturating_sub(legal_dir_count_pos(n, pos)),
            )
        }

        #[inline]
        fn can_reach_target_next(st: &State, target: Cell) -> bool {
            let Some(_) = dir_between_cells(st.n, st.head(), target) else {
                return false;
            };
            st.len() < 2 || target != st.neck()
        }

        fn bfs_next_dir(
            st: &State,
            goal: Cell,
            target: Cell,
            avoid_food: bool,
            strict_body: bool,
        ) -> Option<usize> {
            prof_inc!(bfs_calls);
            let n = st.n;
            let cell_count = n * n;
            let start = st.head();
            if start == goal {
                return None;
            }

            let mut blocked = [false; MAX_CELLS];
            if avoid_food {
                for idx in 0..cell_count {
                    let cell = idx as Cell;
                    if st.food[idx] != 0 && cell != goal && cell != target {
                        blocked[idx] = true;
                    }
                }
            }

            if strict_body && st.len() >= 3 {
                for idx in 1..st.len() - 1 {
                    blocked[st.pos[idx] as usize] = true;
                }
            }

            blocked[start as usize] = false;
            if blocked[goal as usize] {
                return None;
            }

            let mut dist = [u16::MAX; MAX_CELLS];
            let mut first = [u8::MAX; MAX_CELLS];
            let mut q = [0_u16; MAX_CELLS];
            let mut q_head = 0usize;
            let mut q_tail = 0usize;

            let dirs = legal_dirs(st);
            for &dir_u8 in dirs.as_slice() {
                let dir = dir_u8 as usize;
                let nid = next_head_cell(st, dir).unwrap();
                let idx = nid as usize;
                if blocked[idx] || dist[idx] != u16::MAX {
                    continue;
                }
                dist[idx] = 1;
                first[idx] = dir as u8;
                q[q_tail] = nid;
                q_tail += 1;
            }

            while q_head < q_tail {
                let cid = q[q_head];
                q_head += 1;
                prof_inc!(bfs_pops);
                if cid == goal {
                    prof_inc!(bfs_success);
                    return Some(first[cid as usize] as usize);
                }
                let (r, c) = rc_of(cid, n);
                for (dr, dc, _) in DIRS {
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                        continue;
                    }
                    let nid = cell_of(nr as usize, nc as usize, n);
                    let idx = nid as usize;
                    if blocked[idx] || dist[idx] != u16::MAX {
                        continue;
                    }
                    dist[idx] = dist[cid as usize] + 1;
                    first[idx] = first[cid as usize];
                    q[q_tail] = nid;
                    q_tail += 1;
                }
            }

            None
        }

        #[inline]
        fn make_visit_key(st: &State, goal: Cell, restore_len: usize) -> VisitKey {
            let neck = if st.len() >= 2 { st.neck() } else { st.head() };
            VisitKey {
                head: st.head(),
                neck,
                len: st.len() as u16,
                goal,
                restore_len: restore_len as u16,
            }
        }

        fn advance_with_restore_queue(
            st: &State,
            dir: usize,
            target: Cell,
            ell: usize,
            restore_queue: &mut DroppedQueue,
            dropped: &mut DroppedBuf,
        ) -> Option<(State, Option<usize>)> {
            let (ns, _, bite_idx) = step_with_dropped(st, dir, dropped);
            if ns.food[target as usize] == 0 {
                return None;
            }

            if !restore_queue.is_empty() {
                restore_queue.pop_front();
                return Some((ns, bite_idx));
            }

            if bite_idx.is_some() && ns.len() < ell {
                let need = ell - ns.len();
                for ent in dropped.as_slice().iter().take(need) {
                    restore_queue.push_back(*ent);
                }
            }
            Some((ns, bite_idx))
        }

        fn navigate_to_goal_safe(
            bs: &BeamState,
            goal: Cell,
            target: Cell,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(navigate_safe_calls);
            let mut st = bs.state.clone();
            let mut ops = bs.ops.clone();
            let mut seen = FxHashMap::<VisitKey, usize>::default();
            let mut guard = 0usize;

            while st.head() != goal {
                if time_over(started) {
                    return None;
                }
                guard += 1;
                if guard > st.n * st.n * 30 {
                    return None;
                }

                let key = make_visit_key(&st, goal, 0);
                let cnt = seen.entry(key).or_insert(0);
                *cnt += 1;
                if *cnt > VISIT_REPEAT_LIMIT {
                    return None;
                }

                let dir = bfs_next_dir(&st, goal, target, true, true)?;
                let (ns, ate, bite_idx) = step(&st, dir);
                if ns.food[target as usize] == 0 {
                    return None;
                }
                if bite_idx.is_some() || ate != 0 {
                    return None;
                }

                st = ns;
                ops.push(DIRS[dir].2);
                prof_inc!(navigate_safe_steps);
            }

            prof_inc!(navigate_safe_success);
            Some(BeamState { state: st, ops })
        }

        fn navigate_to_goal_loose(
            bs: &BeamState,
            goal: Cell,
            target: Cell,
            ell: usize,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(navigate_loose_calls);
            let mut st = bs.state.clone();
            let mut ops = bs.ops.clone();
            let mut restore_queue = DroppedQueue::new();
            let mut seen = FxHashMap::<VisitKey, usize>::default();
            let mut bite_count = 0usize;
            let bite_limit = st.n * st.n * 4;
            let mut guard = 0usize;
            let mut dropped = DroppedBuf::new();

            while st.head() != goal || !restore_queue.is_empty() {
                if time_over(started) {
                    return None;
                }
                guard += 1;
                if guard > st.n * st.n * 80 {
                    return None;
                }

                let key = make_visit_key(&st, goal, restore_queue.len);
                let cnt = seen.entry(key).or_insert(0);
                *cnt += 1;
                if *cnt > VISIT_REPEAT_LIMIT {
                    return None;
                }

                let dir = if let Some(front) = restore_queue.front() {
                    let dir = dir_between_cells(st.n, st.head(), front.cell)?;
                    if st.len() >= 2 && front.cell == st.neck() {
                        return None;
                    }
                    dir
                } else if let Some(dir) = bfs_next_dir(&st, goal, target, true, false) {
                    dir
                } else {
                    bfs_next_dir(&st, goal, target, false, false)?
                };

                let (ns, bite_idx) = advance_with_restore_queue(
                    &st,
                    dir,
                    target,
                    ell,
                    &mut restore_queue,
                    &mut dropped,
                )?;
                if bite_idx.is_some() {
                    bite_count += 1;
                    if bite_count > bite_limit {
                        return None;
                    }
                }

                st = ns;
                ops.push(DIRS[dir].2);
                prof_inc!(navigate_loose_steps);
            }

            if st.len() < ell {
                return None;
            }

            prof_inc!(navigate_loose_success);
            Some(BeamState { state: st, ops })
        }

        fn choose_shrink_dir(st: &State, input: &Input, ell: usize, target: Cell) -> Option<usize> {
            let anchor_idx = (ell.saturating_sub(1)).min(st.len() - 1);
            let anchor = st.pos[anchor_idx];

            let mut best_bite: Option<((usize, usize, usize, usize), usize)> = None;
            let mut best_move: Option<((usize, usize, usize, usize), usize)> = None;
            let mut dropped = DroppedBuf::new();

            let dirs = legal_dirs(st);
            for &dir_u8 in dirs.as_slice() {
                let dir = dir_u8 as usize;
                let nh = next_head_cell(st, dir).unwrap();
                if nh == target {
                    continue;
                }

                let (sim, ate, bite_idx) = step_with_dropped(st, dir, &mut dropped);
                if sim.food[target as usize] == 0 {
                    continue;
                }

                if !prefix_ok(&sim, input, ell) {
                    continue;
                }

                let target_dist = manhattan(sim.n, sim.head(), target);
                let anchor_dist = manhattan(sim.n, sim.head(), anchor);

                if bite_idx.is_some() {
                    let under = usize::from(sim.len() < ell);
                    let dist_len = sim.len().abs_diff(ell);
                    let not_ready = usize::from(!can_reach_target_next(&sim, target));
                    let key = (under, dist_len, not_ready, target_dist + anchor_dist);
                    if best_bite.as_ref().map_or(true, |(k, _)| key < *k) {
                        best_bite = Some((key, dir));
                    }
                } else {
                    let len_gap = sim.len().abs_diff(ell);
                    let not_ready = usize::from(!can_reach_target_next(&sim, target));
                    let ate_penalty = usize::from(ate != 0);
                    let key = (len_gap, not_ready, target_dist + anchor_dist, ate_penalty);
                    if best_move.as_ref().map_or(true, |(k, _)| key < *k) {
                        best_move = Some((key, dir));
                    }
                }
            }

            if let Some((_, dir)) = best_bite {
                return Some(dir);
            }
            if let Some((_, dir)) = best_move {
                return Some(dir);
            }
            None
        }

        fn shrink_to_ell(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target: Cell,
            target_color: u8,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(shrink_calls);
            let mut st = bs.state.clone();
            let mut ops = bs.ops.clone();

            if st.len() == ell {
                return can_reach_target_next(&st, target).then_some(BeamState { state: st, ops });
            }

            let mut restore_queue = DroppedQueue::new();
            let mut seen = FxHashMap::<VisitKey, usize>::default();
            let mut bite_count = 0usize;
            let bite_limit = st.n * st.n * 3;
            let mut guard = 0usize;
            let mut dropped = DroppedBuf::new();

            while st.len() != ell
                || !restore_queue.is_empty()
                || !can_reach_target_next(&st, target)
            {
                if time_over(started) {
                    return None;
                }
                guard += 1;
                if guard > st.n * st.n * 60 {
                    return None;
                }

                let key = make_visit_key(&st, target, restore_queue.len);
                let cnt = seen.entry(key).or_insert(0);
                *cnt += 1;
                if *cnt > VISIT_REPEAT_LIMIT {
                    return None;
                }

                let dir = if let Some(front) = restore_queue.front() {
                    let dir = dir_between_cells(st.n, st.head(), front.cell)?;
                    if st.len() >= 2 && front.cell == st.neck() {
                        return None;
                    }
                    dir
                } else {
                    choose_shrink_dir(&st, input, ell, target)?
                };

                let (ns, bite_idx) = advance_with_restore_queue(
                    &st,
                    dir,
                    target,
                    ell,
                    &mut restore_queue,
                    &mut dropped,
                )?;
                if bite_idx.is_some() {
                    bite_count += 1;
                    if bite_count > bite_limit {
                        return None;
                    }
                }

                st = ns;
                ops.push(DIRS[dir].2);
                prof_inc!(shrink_steps);
            }

            if st.len() == ell
                && st.food[target as usize] == target_color
                && can_reach_target_next(&st, target)
            {
                prof_inc!(shrink_success);
                Some(BeamState { state: st, ops })
            } else {
                None
            }
        }

        fn finish_eat_target(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target: Cell,
        ) -> Option<BeamState> {
            let st = &bs.state;
            let mut ops = bs.ops.clone();
            let dir = dir_between_cells(st.n, st.head(), target)?;
            if st.len() >= 2 && target == st.neck() {
                return None;
            }

            let (ns, _, bite_idx) = step(st, dir);
            if bite_idx.is_some() {
                return None;
            }
            if ns.len() >= ell + 1 && exact_prefix(&ns, input, ell + 1) {
                ops.push(DIRS[dir].2);
                Some(BeamState { state: ns, ops })
            } else {
                None
            }
        }

        fn try_target_empty_path(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target: Cell,
            started: &Instant,
        ) -> Option<BeamState> {
            prof_inc!(try_target_empty_path_calls);
            let st = &bs.state;
            if !exact_prefix(st, input, ell) {
                return None;
            }
            if st.food[target as usize] != input.d[ell] {
                return None;
            }
            if remaining_food_count(st) > EMPTY_PATH_REMAINING_LIMIT
                || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
            {
                return None;
            }

            if can_reach_target_next(st, target) {
                return finish_eat_target(bs, input, ell, target);
            }
            if reachable_goal_neighbor_count_pos(st.n, &st.pos, target) > 0 {
                return None;
            }
            if collect_food_cells(st, input.d[ell]).len() != 1 {
                return None;
            }

            let mut nodes = Vec::with_capacity(EMPTY_PATH_EXPANSION_CAP.min(120_000) + 8);
            nodes.push(PosNode {
                pos: st.pos.clone(),
                parent: None,
                mv: 0,
            });

            let mut uid = 0usize;
            let mut pq = BinaryHeap::new();
            pq.push(Reverse((
                empty_path_rank(st.n, &st.pos, target),
                0usize,
                uid,
                0usize,
            )));
            uid += 1;

            let mut seen = FxHashMap::<InternalPosDeque, usize>::default();
            seen.insert(st.pos.clone(), 0);
            let mut expansions = 0usize;

            while let Some(Reverse((_, depth, _, idx))) = pq.pop() {
                prof_inc!(try_target_empty_path_expansions);
                if expansions >= EMPTY_PATH_EXPANSION_CAP
                    || time_over(started)
                    || time_left(started) < EMPTY_PATH_MIN_LEFT_SEC
                {
                    break;
                }
                expansions += 1;
                let pos = nodes[idx].pos.clone();

                if can_reach_target_next_pos(st.n, &pos, target) {
                    let mut rev = Vec::new();
                    let mut cur = idx;
                    while let Some(parent) = nodes[cur].parent {
                        rev.push(nodes[cur].mv);
                        cur = parent;
                    }
                    rev.reverse();

                    let mut state = st.clone();
                    let mut ops = bs.ops.clone();
                    for dir_u8 in rev {
                        let dir = dir_u8 as usize;
                        let (ns, ate, bite_idx) = step(&state, dir);
                        if ate != 0 || bite_idx.is_some() {
                            return None;
                        }
                        state = ns;
                        ops.push(DIRS[dir].2);
                    }
                    let gate_bs = BeamState { state, ops };
                    let out = finish_eat_target(&gate_bs, input, ell, target);
                    if out.is_some() {
                        prof_inc!(try_target_empty_path_success);
                    }
                    return out;
                }

                if depth >= EMPTY_PATH_DEPTH_LIMIT {
                    continue;
                }

                for dir in 0..4 {
                    let Some(next_pos) = empty_step_pos(st.n, &st.food, &pos, dir, target) else {
                        continue;
                    };
                    let nd = depth + 1;
                    if seen.get(&next_pos).copied().unwrap_or(usize::MAX) <= nd {
                        continue;
                    }
                    seen.insert(next_pos.clone(), nd);
                    let child = nodes.len();
                    nodes.push(PosNode {
                        pos: next_pos.clone(),
                        parent: Some(idx),
                        mv: dir as u8,
                    });
                    pq.push(Reverse((
                        empty_path_rank(st.n, &next_pos, target),
                        nd,
                        uid,
                        child,
                    )));
                    uid += 1;
                }
            }

            None
        }

        fn try_target_exact(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target: Cell,
            target_color: u8,
            started: &Instant,
        ) -> Vec<BeamState> {
            prof_inc!(try_target_exact_calls);
            let head = bs.state.head();
            let mut cand = neighbors(bs.state.n, target);
            cand.as_mut_slice().sort_unstable_by_key(|&cid| {
                (
                    usize::from(bs.state.food[cid as usize] > 0),
                    manhattan(bs.state.n, head, cid),
                )
            });

            let mut sols = Vec::new();

            for &goal in cand.as_slice() {
                if time_over(started) {
                    break;
                }

                if let Some(b1) = navigate_to_goal_safe(bs, goal, target, started) {
                    if let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started)
                    {
                        if let Some(b3) = finish_eat_target(&b2, input, ell, target) {
                            sols.push(b3);
                        }
                    }
                }

                if let Some(b1) = navigate_to_goal_loose(bs, goal, target, ell, started) {
                    if let Some(b2) = shrink_to_ell(&b1, input, ell, target, target_color, started)
                    {
                        if let Some(b3) = finish_eat_target(&b2, input, ell, target) {
                            sols.push(b3);
                        }
                    }
                }
            }

            if sols.is_empty() && is_endgame_mode(&bs.state, input, ell) {
                if let Some(sol) = try_target_empty_path(bs, input, ell, target, started) {
                    sols.push(sol);
                }
            }

            sols.sort_unstable_by_key(|x| x.ops.len());
            let mut out = Vec::new();
            let mut seen: FxHashSet<State> = FxHashSet::default();
            for s in sols {
                if seen.insert(s.state.clone()) {
                    out.push(s);
                }
            }
            prof_inc!(try_target_exact_success, out.len());
            out
        }

        #[inline]
        fn is_endgame_mode(st: &State, input: &Input, ell: usize) -> bool {
            input.m - ell <= ENDGAME_ELL_LEFT && remaining_food_count(st) <= ENDGAME_REMAINING_FOOD
        }

        fn collect_exact_solutions(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target_color: u8,
            max_targets: usize,
            started: &Instant,
        ) -> Vec<BeamState> {
            prof_inc!(collect_exact_calls);
            let mut sols = Vec::new();
            let mut targets = collect_food_cells(&bs.state, target_color);
            targets
                .as_mut_slice()
                .sort_unstable_by_key(|&cid| manhattan(bs.state.n, bs.state.head(), cid));
            if targets.len() > max_targets {
                targets.truncate(max_targets);
            }
            prof_inc!(collect_exact_targets, targets.len());
            for &target in targets.as_slice() {
                if time_over(started) {
                    break;
                }
                let cand = try_target_exact(bs, input, ell, target, target_color, started);
                for s in cand {
                    sols.push(s);
                }
                if sols.len() >= STAGE_BEAM {
                    break;
                }
            }
            prof_inc!(collect_exact_returned, sols.len());
            sols
        }

        fn collect_exact_solutions_turn_focused(
            bs: &BeamState,
            input: &Input,
            ell: usize,
            target_color: u8,
            max_targets: usize,
            started: &Instant,
        ) -> Vec<BeamState> {
            prof_inc!(collect_exact_turn_calls);
            let mut sols = Vec::new();
            let mut targets = collect_food_cells(&bs.state, target_color);
            targets
                .as_mut_slice()
                .sort_unstable_by_key(|&cid| target_candidate_rank(&bs.state, input, ell, cid));
            if targets.len() > max_targets {
                targets.truncate(max_targets);
            }
            prof_inc!(collect_exact_turn_targets, targets.len());

            for &target in targets.as_slice() {
                if time_over(started) {
                    break;
                }
                let cand = try_target_exact(bs, input, ell, target, target_color, started);
                for s in cand {
                    sols.push(s);
                }
                if sols.len() >= SUFFIX_STAGE_BEAM * 2 {
                    break;
                }
            }

            sols.sort_unstable_by_key(|bs| {
                (
                    turn_focus_next_stage_rank(&bs.state, input, ell + 1),
                    bs.ops.len(),
                )
            });

            let mut out = Vec::with_capacity(SUFFIX_STAGE_BEAM);
            let mut seen: FxHashSet<State> = FxHashSet::default();
            for bs in sols {
                if seen.insert(bs.state.clone()) {
                    out.push(bs);
                    if out.len() >= SUFFIX_STAGE_BEAM {
                        break;
                    }
                }
            }
            prof_inc!(collect_exact_turn_returned, out.len());
            out
        }

        #[inline]
        fn insert_best_plan(map: &mut FxHashMap<State, String>, state: State, ops: String) {
            if let Some(prev_ops) = map.get_mut(&state) {
                if ops.len() < prev_ops.len() {
                    *prev_ops = ops;
                }
            } else {
                map.insert(state, ops);
            }
        }

        #[inline]
        fn map_into_beamstates(map: FxHashMap<State, String>) -> Vec<BeamState> {
            map.into_iter()
                .map(|(state, ops)| BeamState { state, ops })
                .collect()
        }

        fn rescue_stage(
            beam: &[BeamState],
            input: &Input,
            ell: usize,
            target_color: u8,
            started: &Instant,
        ) -> Vec<BeamState> {
            prof_inc!(rescue_stage_calls);
            prof_inc!(rescue_stage_beam_inputs, beam.len());
            let mut order: Vec<usize> = (0..beam.len()).collect();
            order.sort_unstable_by_key(|&idx| {
                (
                    local_score(&beam[idx].state, input, ell),
                    beam[idx].ops.len(),
                )
            });

            let mut rescue_map: FxHashMap<State, String> = FxHashMap::default();
            for &idx in &order {
                if time_over(started) {
                    break;
                }

                let bs = &beam[idx];
                let endgame_mode = is_endgame_mode(&bs.state, input, ell);

                let mut sols = if endgame_mode {
                    collect_exact_solutions(
                        bs,
                        input,
                        ell,
                        target_color,
                        MAX_TARGETS_RESCUE,
                        started,
                    )
                } else {
                    stage_search_bestfirst(bs, input, ell, &BUDGETS_RESCUE, STAGE_BEAM, started)
                };

                if sols.is_empty() && !time_over(started) {
                    if endgame_mode {
                        sols = stage_search_bestfirst(
                            bs,
                            input,
                            ell,
                            &BUDGETS_ENDGAME_LIGHT,
                            STAGE_BEAM,
                            started,
                        );
                    } else {
                        sols = collect_exact_solutions(
                            bs,
                            input,
                            ell,
                            target_color,
                            MAX_TARGETS_RESCUE,
                            started,
                        );
                    }
                }

                for s in sols {
                    if s.ops.len() > MAX_TURNS {
                        continue;
                    }
                    insert_best_plan(&mut rescue_map, s.state, s.ops);
                }

                if rescue_map.len() >= STAGE_BEAM * 2 {
                    break;
                }
            }

            let mut out = map_into_beamstates(rescue_map);
            out.sort_unstable_by_key(|bs| {
                (next_stage_rank(&bs.state, input, ell + 1), bs.ops.len())
            });
            if out.len() > STAGE_BEAM {
                out.truncate(STAGE_BEAM);
            }
            prof_inc!(rescue_stage_returned, out.len());
            out
        }

        fn trim_stage_beam(
            cands: Vec<BeamState>,
            input: &Input,
            next_ell: usize,
            short_lane: Option<&BeamState>,
        ) -> Vec<BeamState> {
            prof_inc!(trim_stage_beam_calls);
            prof_inc!(trim_stage_beam_inputs, cands.len());

            let mut order: Vec<usize> = (0..cands.len()).collect();
            order.sort_unstable_by_key(|&idx| {
                (
                    next_stage_rank(&cands[idx].state, input, next_ell),
                    cands[idx].ops.len(),
                )
            });

            let best_short = cands.iter().min_by_key(|bs| bs.ops.len()).cloned();

            let mut out = Vec::with_capacity(STAGE_BEAM);
            let mut seen: FxHashSet<State> = FxHashSet::default();

            if let Some(bs) = short_lane {
                if seen.insert(bs.state.clone()) {
                    out.push(bs.clone());
                }
            }

            if let Some(bs) = best_short {
                if seen.insert(bs.state.clone()) {
                    out.push(bs);
                }
            }

            for idx in order {
                let bs = &cands[idx];
                if seen.insert(bs.state.clone()) {
                    out.push(bs.clone());
                    if out.len() >= STAGE_BEAM {
                        break;
                    }
                }
            }

            prof_inc!(trim_stage_beam_returned, out.len());
            out
        }

        fn solve_base(input: &Input, started: &Instant) -> BeamState {
            let init = BeamState {
                state: State::initial(input),
                ops: String::new(),
            };
            let mut beam = vec![init];

            for ell in 5..input.m {
                prof_inc!(solve_base_iters);
                prof_inc!(solve_base_beam_inputs, beam.len());
                if time_over(started) {
                    break;
                }

                let target_color = input.d[ell];
                let budgets: &[(usize, usize)] = if input.m - ell < 10 {
                    &BUDGETS_LATE
                } else {
                    &BUDGETS_NORMAL
                };

                let short_seed = beam.iter().min_by_key(|bs| bs.ops.len()).cloned();
                let quick_short = short_seed
                    .as_ref()
                    .and_then(|bs| extend_fastlane_one(bs, input, ell, started));

                let mut new_map: FxHashMap<State, String> = FxHashMap::default();
                if let Some(sol) = quick_short.clone() {
                    insert_best_plan(&mut new_map, sol.state, sol.ops);
                }

                for bs in &beam {
                    if time_over(started) {
                        break;
                    }

                    let endgame_mode = is_endgame_mode(&bs.state, input, ell);
                    let mut sols = Vec::new();

                    if !time_over(started) {
                        if endgame_mode {
                            sols = collect_exact_solutions(
                                bs,
                                input,
                                ell,
                                target_color,
                                MAX_TARGETS_ENDGAME,
                                started,
                            );
                        } else {
                            sols = stage_search_bestfirst(
                                bs, input, ell, budgets, STAGE_BEAM, started,
                            );
                        }
                    }

                    if sols.is_empty() && !time_over(started) {
                        if endgame_mode {
                            sols = stage_search_bestfirst(
                                bs,
                                input,
                                ell,
                                &BUDGETS_ENDGAME_LIGHT,
                                STAGE_BEAM,
                                started,
                            );
                        } else {
                            sols = collect_exact_solutions(
                                bs,
                                input,
                                ell,
                                target_color,
                                MAX_TARGETS_PER_STAGE,
                                started,
                            );
                        }
                    }

                    for s in sols {
                        if s.ops.len() > MAX_TURNS {
                            continue;
                        }
                        insert_best_plan(&mut new_map, s.state, s.ops);
                    }
                }

                if new_map.is_empty() && !time_over(started) {
                    prof_inc!(solve_base_rescue_calls);
                    let rescue = rescue_stage(&beam, input, ell, target_color, started);
                    for s in rescue {
                        insert_best_plan(&mut new_map, s.state, s.ops);
                    }
                }

                if new_map.is_empty() {
                    break;
                }
                prof_inc!(solve_base_new_map_size_sum, new_map.len());

                let new_beam = trim_stage_beam(
                    map_into_beamstates(new_map),
                    input,
                    ell + 1,
                    quick_short.as_ref(),
                );
                beam = new_beam;
            }

            if beam.is_empty() {
                return BeamState {
                    state: State::initial(input),
                    ops: String::new(),
                };
            }

            beam.sort_unstable_by_key(|bs| final_rank(bs, input));
            let mut best = beam.pop().unwrap();
            if best.ops.len() > MAX_TURNS {
                best.ops.truncate(MAX_TURNS);
            }
            best
        }

        fn reconstruct_exact_checkpoints(input: &Input, ops: &str) -> Vec<Option<(usize, State)>> {
            let mut checkpoints = vec![None; input.m + 1];
            let mut st = State::initial(input);
            checkpoints[5] = Some((0, st.clone()));
            let mut ell = 5usize;

            for (t, ch) in ops.bytes().enumerate() {
                let Some(dir) = dir_of_char(ch) else {
                    break;
                };
                if !is_legal_dir(&st, dir) {
                    break;
                }
                let (ns, _, _) = step(&st, dir);
                st = ns;
                if ell < input.m && exact_prefix(&st, input, ell + 1) {
                    ell += 1;
                    checkpoints[ell] = Some((t + 1, st.clone()));
                    if ell == input.m {
                        break;
                    }
                }
            }

            checkpoints
        }

        fn is_complete_exact(bs: &BeamState, input: &Input) -> bool {
            bs.state.len() == input.m
                && exact_prefix(&bs.state, input, input.m)
                && remaining_food_count(&bs.state) == 0
        }

        fn solve_suffix_turn_focused(
            input: &Input,
            start_bs: BeamState,
            start_ell: usize,
            started: &Instant,
        ) -> BeamState {
            let mut beam = vec![start_bs.clone()];

            for ell in start_ell..input.m {
                prof_inc!(solve_suffix_iters);
                prof_inc!(solve_suffix_beam_inputs, beam.len());
                if time_over(started) || time_left(started) < 0.02 {
                    break;
                }

                let target_color = input.d[ell];
                let mut new_map: FxHashMap<State, String> = FxHashMap::default();

                for bs in &beam {
                    if time_over(started) {
                        break;
                    }

                    let mut sols = collect_exact_solutions_turn_focused(
                        bs,
                        input,
                        ell,
                        target_color,
                        SUFFIX_OPT_TARGETS,
                        started,
                    );

                    if sols.is_empty() && !time_over(started) {
                        sols = stage_search_bestfirst(
                            bs,
                            input,
                            ell,
                            &BUDGETS_ENDGAME_LIGHT,
                            SUFFIX_STAGE_BEAM,
                            started,
                        );
                    }
                    if sols.is_empty() && !time_over(started) {
                        sols = stage_search_bestfirst(
                            bs,
                            input,
                            ell,
                            &BUDGETS_LATE,
                            SUFFIX_STAGE_BEAM,
                            started,
                        );
                    }
                    if sols.is_empty() && !time_over(started) {
                        sols = collect_exact_solutions(
                            bs,
                            input,
                            ell,
                            target_color,
                            MAX_TARGETS_ENDGAME,
                            started,
                        );
                    }

                    for s in sols {
                        if s.ops.len() > MAX_TURNS {
                            continue;
                        }
                        insert_best_plan(&mut new_map, s.state, s.ops);
                    }
                }

                if new_map.is_empty() {
                    break;
                }
                prof_inc!(solve_suffix_new_map_size_sum, new_map.len());

                let mut new_beam = map_into_beamstates(new_map);
                new_beam.sort_unstable_by_key(|bs| {
                    (
                        turn_focus_next_stage_rank(&bs.state, input, ell + 1),
                        bs.ops.len(),
                    )
                });
                if new_beam.len() > SUFFIX_STAGE_BEAM {
                    new_beam.truncate(SUFFIX_STAGE_BEAM);
                }
                beam = new_beam;
            }

            if beam.is_empty() {
                return start_bs;
            }

            beam.sort_unstable_by_key(|bs| final_rank(bs, input));
            beam.pop().unwrap()
        }

        fn optimize_exact_suffix(input: &Input, base: BeamState, started: &Instant) -> BeamState {
            if !is_complete_exact(&base, input) {
                return base;
            }

            let mut best = base;

            for &window in &SUFFIX_OPT_WINDOWS {
                prof_inc!(optimize_suffix_windows);
                if time_left(started) < SUFFIX_OPT_MIN_LEFT_SEC {
                    break;
                }

                let checkpoints = reconstruct_exact_checkpoints(input, &best.ops);
                let start_ell = input.m.saturating_sub(window).max(5);
                let Some((prefix_turns, st)) = checkpoints[start_ell].clone() else {
                    continue;
                };

                let prefix_ops = best.ops[..prefix_turns].to_string();
                let start_bs = BeamState {
                    state: st,
                    ops: prefix_ops,
                };

                let cand = solve_suffix_turn_focused(input, start_bs, start_ell, started);
                if is_complete_exact(&cand, input) && cand.ops.len() < best.ops.len() {
                    best = cand;
                }
            }

            best
        }

        fn solve(input: &Input) -> String {
            let started = Instant::now();
            let base = solve_base(input, &started);
            let mut best = optimize_exact_suffix(input, base, &started);
            if best.ops.len() > MAX_TURNS {
                best.ops.truncate(MAX_TURNS);
            }
            prof_set!(final_turns, best.ops.len());
            best.ops
        }

        fn main() {
            let input = read_input();
            let ans = solve(&input);

            let mut out = String::new();
            for ch in ans.chars() {
                out.push(ch);
                out.push('\n');
            }
            print!("{out}");
            profile_counts::dump("v126pro_faster123");
        }

        pub(super) fn solve_with_started(input: &super::Input, started: &Instant) -> String {
            let mut d = [0_u8; MAX_LEN];
            for (idx, &color) in input.d.iter().enumerate() {
                d[idx] = color;
            }

            let mut food = [0_u8; MAX_CELLS];
            for (idx, &color) in input.food.iter().enumerate() {
                food[idx] = color;
            }

            let raw_input = Input {
                n: input.n,
                m: input.m,
                d,
                food,
            };
            let base = solve_base(&raw_input, started);
            let mut best = optimize_exact_suffix(&raw_input, base, started);
            if best.ops.len() > MAX_TURNS {
                best.ops.truncate(MAX_TURNS);
            }
            prof_set!(final_turns, best.ops.len());
            profile_counts::dump("v131_126meets012");
            best.ops
        }
    }

    const SIMPLE_BEAM_WIDTH: usize = 96;
    const SIMPLE_CANDIDATE_WIDTH: usize = 8;
    const FOOD_HASH_COLOR_CAPACITY: usize = 8;
    const FOOD_HASH_CELL_CAPACITY: usize = 16 * 16;

    const fn splitmix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    const fn build_food_hash_table() -> [[u64; FOOD_HASH_COLOR_CAPACITY]; FOOD_HASH_CELL_CAPACITY] {
        let mut out = [[0_u64; FOOD_HASH_COLOR_CAPACITY]; FOOD_HASH_CELL_CAPACITY];
        let mut cell = 0usize;
        while cell < FOOD_HASH_CELL_CAPACITY {
            let mut color = 1usize;
            while color < FOOD_HASH_COLOR_CAPACITY {
                out[cell][color] =
                    splitmix64(((cell as u64) << 8) ^ color as u64 ^ 0x1234_5678_9abc_def0);
                color += 1;
            }
            cell += 1;
        }
        out
    }

    const FOOD_HASH_TABLE: [[u64; FOOD_HASH_COLOR_CAPACITY]; FOOD_HASH_CELL_CAPACITY] =
        build_food_hash_table();

    #[inline]
    fn calc_food_hash(food: &[u8]) -> u64 {
        let mut hash = 0_u64;
        for (idx, &color) in food.iter().enumerate() {
            if color != 0 {
                hash ^= FOOD_HASH_TABLE[idx][color as usize];
            }
        }
        hash
    }

    #[inline]
    fn count_remaining_food(food: &[u8]) -> u16 {
        food.iter().filter(|&&color| color != 0).count() as u16
    }

    #[derive(Debug, Clone)]
    struct SolverState {
        inner: State,
        remaining_food: u16,
        food_hash: u64,
    }

    impl PartialEq for SolverState {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.remaining_food == other.remaining_food
                && self.food_hash == other.food_hash
                && self.inner.pos == other.inner.pos
                && self.inner.colors == other.inner.colors
                && self.inner.food == other.inner.food
        }
    }

    impl Eq for SolverState {}

    impl Hash for SolverState {
        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.remaining_food.hash(state);
            self.food_hash.hash(state);
            self.inner.pos.hash(state);
            self.inner.colors.hash(state);
        }
    }

    impl SolverState {
        #[inline]
        fn initial(input: &Input) -> Self {
            let inner = State::initial(input);
            let remaining_food = count_remaining_food(&inner.food);
            let food_hash = calc_food_hash(&inner.food);
            Self {
                inner,
                remaining_food,
                food_hash,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct SolverStepResult {
        state: SolverState,
        ate: Option<u8>,
        bite_idx: Option<usize>,
        dropped: Vec<Dropped>,
    }

    #[inline]
    fn solver_step(state: &SolverState, n: usize, dir: usize) -> SolverStepResult {
        let next_head = Grid::next_cell(n, state.inner.head(), dir);
        let eaten_color = state.inner.food[Grid::index(next_head)];
        let base = step(&state.inner, n, dir);

        let mut remaining_food = state.remaining_food;
        let mut food_hash = state.food_hash;
        if eaten_color != 0 {
            remaining_food -= 1;
            food_hash ^= FOOD_HASH_TABLE[Grid::index(next_head)][eaten_color as usize];
        }
        for dropped in &base.dropped {
            remaining_food += 1;
            food_hash ^= FOOD_HASH_TABLE[Grid::index(dropped.cell)][dropped.color as usize];
        }

        let state = SolverState {
            inner: base.state,
            remaining_food,
            food_hash,
        };
        debug_assert_eq!(
            state.remaining_food,
            count_remaining_food(&state.inner.food)
        );
        debug_assert_eq!(state.food_hash, calc_food_hash(&state.inner.food));

        SolverStepResult {
            state,
            ate: base.ate,
            bite_idx: base.bite_idx,
            dropped: base.dropped,
        }
    }

    #[inline]
    fn remaining_food_count(state: &SolverState) -> usize {
        state.remaining_food as usize
    }

    #[inline]
    fn is_complete_exact(state: &SolverState, input: &Input) -> bool {
        state.inner.len() == input.m
            && matches_prefix_len(&state.inner.colors, &input.d, input.m)
            && remaining_food_count(state) == 0
    }

    #[derive(Clone)]
    struct BeamNode {
        state: SolverState,
        ops: String,
    }

    type Score = (usize, usize, usize);

    struct ScoredNode {
        node: BeamNode,
        score: Score,
    }

    #[derive(Debug, Clone)]
    struct SimpleBeamAttempt {
        ops: String,
        state: SolverState,
        reached_ell: usize,
        completed: bool,
    }

    #[inline]
    fn finalize_simple_beam_attempt(
        ops: String,
        state: SolverState,
        reached_ell: usize,
        input: &Input,
    ) -> SimpleBeamAttempt {
        let completed = reached_ell == input.m && is_complete_exact(&state, input);
        SimpleBeamAttempt {
            ops,
            state,
            reached_ell,
            completed,
        }
    }

    fn collect_top_k_target_paths(
        state: &SolverState,
        n: usize,
        target_color: u8,
        k: usize,
    ) -> Vec<Vec<Cell>> {
        let bfs = compute_body_release_dist(&state.inner, n);
        let eat_dist = body_release_eat_dist(&bfs, &state.inner, n);

        let mut foods = Vec::new();
        for idx in 0..(n * n) {
            if state.inner.food[idx] != target_color {
                continue;
            }
            let dist = eat_dist.dist[idx];
            if dist == usize::MAX {
                continue;
            }
            foods.push((dist, idx));
        }

        foods.sort_by_key(|&(dist, idx)| (dist, idx));
        if foods.len() > k {
            foods.truncate(k);
        }

        let mut paths = Vec::with_capacity(foods.len());
        for (_, idx) in foods {
            if let Some(path) = reconstruct_cell_search_path(&eat_dist, Cell(idx as u16)) {
                paths.push(path);
            }
        }
        paths
    }

    fn find_path_to_nearest_target_food(
        state: &SolverState,
        n: usize,
        target_color: u8,
    ) -> Option<Vec<Cell>> {
        collect_top_k_target_paths(state, n, target_color, 1)
            .into_iter()
            .next()
    }

    fn evaluate_node(state: &SolverState, input: &Input, next_ell: usize, ops_len: usize) -> Score {
        if next_ell == input.m {
            return (
                usize::from(!is_complete_exact(state, input)),
                ops_len,
                remaining_food_count(state),
            );
        }

        let next_color = input.d[next_ell];
        let Some(path) = find_path_to_nearest_target_food(state, input.n, next_color) else {
            return (1, ops_len, usize::MAX);
        };
        (0, ops_len, path.len().saturating_sub(1))
    }

    fn apply_path_with_turn_cap(
        state: &mut SolverState,
        out: &mut String,
        n: usize,
        path: &[Cell],
        turn_cap: usize,
    ) -> bool {
        for pair in path.windows(2) {
            if out.len() == turn_cap {
                return false;
            }
            let dir = Grid::dir_between_cells(n, pair[0], pair[1]);
            let step_result = solver_step(state, n, dir);
            *state = step_result.state;
            out.push(DIRS[dir].2);
        }
        true
    }

    fn build_child_node(parent: &BeamNode, n: usize, path: &[Cell]) -> Option<BeamNode> {
        let mut child_state = parent.state.clone();
        let mut child_ops = parent.ops.clone();
        if !apply_path_with_turn_cap(&mut child_state, &mut child_ops, n, path, MAX_TURNS) {
            return None;
        }

        Some(BeamNode {
            state: child_state,
            ops: child_ops,
        })
    }

    fn expand_one_node(parent: &BeamNode, input: &Input, ell: usize) -> Vec<ScoredNode> {
        let target_color = input.d[ell];
        let paths = collect_top_k_target_paths(
            &parent.state,
            input.n,
            target_color,
            SIMPLE_CANDIDATE_WIDTH,
        );

        let mut children = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(child) = build_child_node(parent, input.n, &path) else {
                continue;
            };
            let score = evaluate_node(&child.state, input, ell + 1, child.ops.len());
            children.push(ScoredNode { node: child, score });
        }
        children
    }

    fn select_best_node<'a>(beam: &'a [BeamNode], input: &Input, ell: usize) -> &'a BeamNode {
        debug_assert!(!beam.is_empty());

        let mut best_idx = 0usize;
        let mut best_score = evaluate_node(&beam[0].state, input, ell, beam[0].ops.len());
        for (idx, node) in beam.iter().enumerate().skip(1) {
            let score = evaluate_node(&node.state, input, ell, node.ops.len());
            if score < best_score {
                best_score = score;
                best_idx = idx;
            }
        }
        &beam[best_idx]
    }

    fn try_simple_beam_first(input: &Input) -> SimpleBeamAttempt {
        let mut beam = vec![BeamNode {
            state: SolverState::initial(input),
            ops: String::new(),
        }];
        let mut ell = 5usize;

        while ell < input.m {
            let mut all_children = Vec::new();
            for parent in &beam {
                all_children.extend(expand_one_node(parent, input, ell));
            }
            if all_children.is_empty() {
                break;
            }

            all_children.sort_by(|a, b| a.score.cmp(&b.score));
            if all_children.len() > SIMPLE_BEAM_WIDTH {
                all_children.truncate(SIMPLE_BEAM_WIDTH);
            }

            beam = all_children.into_iter().map(|scored| scored.node).collect();
            ell += 1;
        }

        let best = select_best_node(&beam, input, ell);
        finalize_simple_beam_attempt(best.ops.clone(), best.state.clone(), ell, input)
    }

    pub(super) fn run() {
        let input = read_input();
        let time_keeper = TimeKeeper::new(1.85, 8);
        let attempt = try_simple_beam_first(&input);

        let ans = if attempt.completed {
            attempt.ops
        } else {
            v126raw::solve_with_started(&input, &time_keeper.start)
        };

        let mut out = String::new();
        for ch in ans.chars() {
            out.push(ch);
            out.push('\n');
        }
        print!("{out}");
    }

    #[cfg(test)]
    mod v131_tests {
        use super::*;

        fn make_solver_state(
            n: usize,
            food: Vec<u8>,
            pos_ij: &[(usize, usize)],
            colors: &[u8],
        ) -> SolverState {
            let pos = pos_ij
                .iter()
                .map(|&(i, j)| Grid::cell(n, i, j))
                .collect::<Vec<_>>();
            let inner = State {
                food,
                pos: InternalPosDeque::from_slice(&pos),
                colors: InternalColors::from_slice(colors),
                pos_occupancy: InternalPosOccupancy::from_pos(&InternalPosDeque::from_slice(&pos)),
            };
            SolverState {
                remaining_food: count_remaining_food(&inner.food),
                food_hash: calc_food_hash(&inner.food),
                inner,
            }
        }

        fn assert_solver_state_consistent(state: &SolverState) {
            assert_eq!(
                state.remaining_food,
                count_remaining_food(&state.inner.food)
            );
            assert_eq!(state.food_hash, calc_food_hash(&state.inner.food));
            assert_eq!(state.inner.pos.len(), state.inner.colors.len());
            let mut cnt = [0_u8; INTERNAL_POS_DEQUE_CAPACITY];
            for cell in state.inner.pos.iter() {
                cnt[Grid::index(cell)] += 1;
            }
            assert_eq!(state.inner.pos_occupancy.cnt, cnt);
        }

        #[test]
        fn solver_step_tracks_remaining_food_and_food_hash() {
            let n = 8;
            let mut food = vec![0; n * n];
            food[Grid::index(Grid::cell(n, 4, 1))] = 2;
            let input = Input {
                n,
                m: 6,
                color_count: 3,
                d: vec![1, 1, 1, 1, 1, 2],
                food,
            };
            let state = SolverState::initial(&input);

            let result = solver_step(&state, n, 3);
            assert_eq!(result.ate, Some(2));
            assert_eq!(result.bite_idx, None);
            assert_eq!(result.dropped.len(), 0);
            assert_eq!(remaining_food_count(&result.state), 0);
            assert_solver_state_consistent(&result.state);
        }

        #[test]
        fn simple_beam_completion_requires_exact_and_no_remaining_food() {
            let n = 8;
            let mut food = vec![0; n * n];
            food[Grid::index(Grid::cell(n, 4, 1))] = 2;
            let input = Input {
                n,
                m: 6,
                color_count: 3,
                d: vec![1, 1, 1, 1, 1, 2],
                food,
            };

            let initial = SolverState::initial(&input);
            let incomplete =
                finalize_simple_beam_attempt(String::new(), initial.clone(), input.m, &input);
            assert!(!incomplete.completed);

            let eaten = solver_step(&initial, n, 3).state;
            let complete = finalize_simple_beam_attempt("R".to_string(), eaten, input.m, &input);
            assert!(complete.completed);
        }
    }
}

fn main() {
    solver::run();
}
