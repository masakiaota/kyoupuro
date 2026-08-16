// v000_template.rs
#![allow(non_snake_case)] // 問題文の `N`, `M` などを対応づけたまま使う。
#![allow(dead_code)] // v000 以外ではdead_codeは許容しない

use std::io::{BufRead, Write};
use std::time::Instant;

/// 公園は常に `N × N = 50 × 50` マスである。
const N: usize = 50;
/// 到着するグループ数は常に 1000 個である。
const M: usize = 1000;
/// グループの人数は `P_MIN..=P_MAX` の範囲である。
const P_MIN: usize = 4;
const P_MAX: usize = 150;
/// 到着・退去時刻の上限である。
const T_MAX: usize = 100_000;

/// `N × N` 盤面の総マス数である。
const CELL_COUNT: usize = N * N;
const WORD_BITS: usize = u64::BITS as usize;
const WORD_COUNT: usize = (CELL_COUNT + WORD_BITS - 1) / WORD_BITS;
const ROW_MASK: u64 = (1_u64 << N) - 1;

/// 座標 `(x, y)` を `x * N + y` で一次元化したマス番号。
type CellId = usize;

// AtCoder から与えられる入力は制約を満たすため、入力値の範囲検証や
// 不正入力に対するエラー処理は行わない。

/// インタラクティブ入力を一行ずつ補充し、入力全体の終了を待たずに読み進める。
struct InputReader<R> {
    reader: R,
    line: String,
}

impl<R: BufRead> InputReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            line: String::new(),
        }
    }

    fn read_line(&mut self) -> &str {
        self.line.clear();
        self.reader.read_line(&mut self.line).unwrap();
        &self.line
    }
}

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
const JUDGE_TIME_LIMIT_SEC: f64 = 1.90;
/// local feature 時はローカル実行の速度差を見込んで探索時間を短くする。
const LOCAL_TIME_RATIO: f64 = 0.80;

const PROGRAM_TIME_LIMIT_SEC: f64 = if cfg!(feature = "local") {
    JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO
} else {
    JUDGE_TIME_LIMIT_SEC
};

/// 盤面上の任意のマス集合を、`CellId = x * N + y` の順に詰めた bitset で表す。
/// 最後の `u64` は下位 `CELL_COUNT % WORD_BITS` bit だけを使う。
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// 座標 `(x, y)` のマスが集合に含まれるかを返す。
    /// `x < N`, `y < N` を前提とする。
    #[inline]
    fn contains(&self, x: usize, y: usize) -> bool {
        self.contains_id(x * N + y)
    }

    /// 座標 `(x, y)` のマスを集合へ追加する。
    /// `x < N`, `y < N` を前提とする。
    #[inline]
    fn insert(&mut self, x: usize, y: usize) {
        self.insert_id(x * N + y);
    }

    /// 座標 `(x, y)` のマスを集合から取り除く。
    /// `x < N`, `y < N` を前提とする。
    #[inline]
    fn remove(&mut self, x: usize, y: usize) {
        self.remove_id(x * N + y);
    }

    /// 行 `x` に含まれる `N` マスを、下位 `N` bitへ抽出する。
    /// 返り値のbit `y` は `contains(x, y)` が真のときに限り1となる。
    /// `x < N` を前提とする。
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

    /// `self` と `other` に共通するマスが一つもないかを返す。
    fn is_disjoint(&self, other: &Self) -> bool {
        self.words
            .iter()
            .zip(&other.words)
            .all(|(&a, &b)| a & b == 0)
    }

    /// `other` の全マスを `self` へ追加し、`self` を和集合へ更新する。
    fn union_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] |= other.words[k];
        }
    }

    /// `other` に含まれる全マスを `self` から取り除き、`self` を差集合へ更新する。
    fn difference_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] &= !other.words[k];
        }
    }

    /// `other` にも含まれるマスだけを残し、`self` を積集合へ更新する。
    fn intersection_with(&mut self, other: &Self) {
        for k in 0..WORD_COUNT {
            self.words[k] &= other.words[k];
        }
    }

    /// 集合に含まれるマス数を返す。
    fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    #[inline]
    fn contains_id(&self, id: CellId) -> bool {
        self.words[id >> 6] & (1_u64 << (id & 63)) != 0
    }

    #[inline]
    fn insert_id(&mut self, id: CellId) {
        self.words[id >> 6] |= 1_u64 << (id & 63);
    }

    #[inline]
    fn remove_id(&mut self, id: CellId) {
        self.words[id >> 6] &= !(1_u64 << (id & 63));
    }
}

#[derive(Debug, Clone)]
struct InitialInput {
    R: f64,
    /// 芝生マスの集合。
    grass_cells: CellSet,
}

impl InitialInput {
    /// 最初に与えられる `N M R` と公園の形状を読み込む。
    fn read<R: BufRead>(input: &mut InputReader<R>) -> Self {
        let R = {
            let mut tokens = input.read_line().split_ascii_whitespace();
            let _N: usize = tokens.next().unwrap().parse().unwrap();
            let _M: usize = tokens.next().unwrap().parse().unwrap();
            tokens.next().unwrap().parse().unwrap()
        };

        let mut grass_cells = CellSet::default();
        for x in 0..N {
            let line = input.read_line().as_bytes();
            for (y, &cell) in line.iter().take(N).enumerate() {
                if cell == b'.' {
                    grass_cells.insert(x, y);
                }
            }
        }

        Self { R, grass_cells }
    }
}

#[derive(Debug, Clone, Copy)]
struct GroupInput {
    i: usize,
    S: usize,
    T: usize,
    P: usize,
    V: i64,
}

/// 到着済みグループの入力を、グループ番号順に保持する。
struct InputHistory {
    groups: Vec<GroupInput>,
}

impl InputHistory {
    fn new() -> Self {
        Self {
            groups: Vec::with_capacity(M),
        }
    }
}

impl GroupInput {
    /// 次に到着した1グループ分を読み込み、`history.groups[i]` へ追加する。
    /// 次のグループを読む前に、現在のターンの出力を flush する必要がある。
    fn read<R: BufRead>(input: &mut InputReader<R>, history: &mut InputHistory) -> Self {
        let mut tokens = input.read_line().split_ascii_whitespace();
        let group = Self {
            i: tokens.next().unwrap().parse().unwrap(),
            S: tokens.next().unwrap().parse().unwrap(),
            T: tokens.next().unwrap().parse().unwrap(),
            P: tokens.next().unwrap().parse().unwrap(),
            V: tokens.next().unwrap().parse().unwrap(),
        };
        history.groups.push(group);
        group
    }

    #[inline]
    fn D(&self) -> usize {
        self.T - self.S
    }
}

#[derive(Debug)]
struct Relocation {
    j: usize,
    region: CellSet,
}

#[derive(Debug)]
enum ArrivalDecision {
    Accept { region: CellSet },
    Reject,
}

#[derive(Debug)]
struct TurnOutput {
    relocations: Vec<Relocation>,
    decision: ArrivalDecision,
}

impl TurnOutput {
    /// 1ターン分をインタラクティブ形式で出力し、最後に必ず flush する。
    fn write_and_flush<W: Write>(&self, output: &mut W) {
        writeln!(output, "{}", self.relocations.len()).unwrap();

        for relocation in &self.relocations {
            writeln!(output, "{}", relocation.j).unwrap();
            Self::write_region(output, &relocation.region);
        }

        match &self.decision {
            ArrivalDecision::Accept { region } => {
                writeln!(output, "Yes").unwrap();
                Self::write_region(output, region);
            }
            ArrivalDecision::Reject => {
                writeln!(output, "No").unwrap();
            }
        }

        output.flush().unwrap();
    }

    fn write_region<W: Write>(output: &mut W, region: &CellSet) {
        for x in 0..N {
            let mut bits = region.row_bits(x);
            while bits != 0 {
                let y = bits.trailing_zeros() as usize;
                writeln!(output, "{x} {y}").unwrap();
                bits &= bits - 1;
            }
        }
    }
}

const NO_OWNER: u16 = u16::MAX;
const NO_ACTIVE_INDEX: u16 = u16::MAX;
const GROUP_WORD_COUNT: usize = (M + WORD_BITS - 1) / WORD_BITS;

#[derive(Debug, Clone)]
struct GroupState {
    i: usize,
    compactness: f64,
    /// 退去時の利用料 `round(V[i] * min_compactness)` の計算に使う。
    min_compactness: f64,
}

#[derive(Debug, Clone)]
struct State {
    /// `owner` から導出できるが、頻繁な重複判定と集合演算のためにキャッシュする。
    occupied: CellSet,

    /// `owner[id]` はマス `id` の所有グループ番号。空きマスは `NO_OWNER` とする。
    owner: [u16; CELL_COUNT],

    /// 現在利用中のグループだけを密に保持する。
    active_groups: Vec<GroupState>,

    /// `active_index_by_group[i]` は `active_groups` 内の位置。
    /// グループ `i` が利用中でなければ `NO_ACTIVE_INDEX` とする。
    active_index_by_group: [u16; M],

    /// 現時点の所持金 `X`。退去時の利用料で増え、再配置コストで減る。
    current_X: i64,
}

impl State {
    fn new() -> Self {
        Self {
            occupied: CellSet::default(),
            owner: [NO_OWNER; CELL_COUNT],
            active_groups: Vec::with_capacity(M),
            active_index_by_group: [NO_ACTIVE_INDEX; M],
            current_X: 0,
        }
    }

    #[inline]
    fn owner_at(&self, x: usize, y: usize) -> Option<usize> {
        let owner = self.owner[x * N + y];
        if owner == NO_OWNER {
            None
        } else {
            Some(owner as usize)
        }
    }

    #[inline]
    fn active_group(&self, i: usize) -> Option<&GroupState> {
        let index = self.active_index_by_group[i];
        if index == NO_ACTIVE_INDEX {
            None
        } else {
            Some(&self.active_groups[index as usize])
        }
    }

    #[inline]
    fn is_free(&self, grass_cells: &CellSet, x: usize, y: usize) -> bool {
        grass_cells.contains(x, y) && !self.occupied.contains(x, y)
    }

    #[inline]
    fn occupied_count(&self) -> usize {
        self.occupied.count()
    }

    #[inline]
    fn abs_score(&self) -> i64 {
        self.current_X.max(0)
    }

    /// 最新の到着時刻より前に退去した全グループを処理する。
    /// 対象が複数でも `owner` の走査は一度だけ行う。
    fn process_departures(&mut self, history: &InputHistory) {
        let current_S = history.groups.last().unwrap().S;
        let mut departed = [0_u64; GROUP_WORD_COUNT];
        let mut has_departures = false;
        let mut index = 0;

        while index < self.active_groups.len() {
            let group = &self.active_groups[index];
            let input = &history.groups[group.i];
            if input.T >= current_S {
                index += 1;
                continue;
            }

            self.current_X += ((input.V as f64) * group.min_compactness).round() as i64;
            departed[group.i >> 6] |= 1_u64 << (group.i & 63);
            has_departures = true;
            self.remove_active_group_at(index);
        }

        if has_departures {
            self.clear_groups_from_board(&departed);
        }
    }

    /// 合法な1ターン分の出力を、再配置の同時性を保って反映する。
    fn apply_output(
        &mut self,
        initial: &InitialInput,
        history: &InputHistory,
        output: &TurnOutput,
    ) {
        if !output.relocations.is_empty() {
            let mut relocated = [0_u64; GROUP_WORD_COUNT];
            for relocation in &output.relocations {
                relocated[relocation.j >> 6] |= 1_u64 << (relocation.j & 63);
            }
            self.clear_groups_from_board(&relocated);

            for relocation in &output.relocations {
                let j = relocation.j;
                let input = &history.groups[j];
                let compactness = Self::region_compactness(&relocation.region, input.P);
                let active_index = self.active_index_by_group[j] as usize;
                let group = &mut self.active_groups[active_index];
                group.compactness = compactness;
                group.min_compactness = group.min_compactness.min(compactness);

                self.current_X -= (((input.V as f64) * initial.R).round() as i64).max(1);
                self.place_region(j, &relocation.region);
            }
        }

        if let ArrivalDecision::Accept { region } = &output.decision {
            let i = history.groups.len() - 1;
            let compactness = Self::region_compactness(region, history.groups[i].P);
            let active_index = self.active_groups.len();
            self.active_groups.push(GroupState {
                i,
                compactness,
                min_compactness: compactness,
            });
            self.active_index_by_group[i] = active_index as u16;
            self.place_region(i, region);
        }
    }

    /// 最終ターン後の全グループを退去させ、利用料を確定する。
    fn process_final_departures(&mut self, history: &InputHistory) {
        let mut income = 0_i64;
        for group in &self.active_groups {
            let input = &history.groups[group.i];
            income += ((input.V as f64) * group.min_compactness).round() as i64;
            self.active_index_by_group[group.i] = NO_ACTIVE_INDEX;
        }
        self.current_X += income;
        self.active_groups.clear();
        self.owner.fill(NO_OWNER);
        self.occupied = CellSet::default();
    }

    #[inline]
    fn remove_active_group_at(&mut self, index: usize) {
        let removed_i = self.active_groups[index].i;
        self.active_groups.swap_remove(index);
        self.active_index_by_group[removed_i] = NO_ACTIVE_INDEX;

        if index < self.active_groups.len() {
            let moved_i = self.active_groups[index].i;
            self.active_index_by_group[moved_i] = index as u16;
        }
    }

    fn clear_groups_from_board(&mut self, groups: &[u64; GROUP_WORD_COUNT]) {
        for id in 0..CELL_COUNT {
            let owner = self.owner[id];
            if owner == NO_OWNER {
                continue;
            }
            let i = owner as usize;
            if groups[i >> 6] & (1_u64 << (i & 63)) != 0 {
                self.owner[id] = NO_OWNER;
                self.occupied.remove_id(id);
            }
        }
    }

    fn place_region(&mut self, i: usize, region: &CellSet) {
        let owner = i as u16;
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

    fn region_compactness(region: &CellSet, P: usize) -> f64 {
        let mut adjacent_edges = 0_usize;
        let mut previous_row = 0_u64;
        for x in 0..N {
            let row = region.row_bits(x);
            adjacent_edges += (row & (row >> 1)).count_ones() as usize;
            adjacent_edges += (row & previous_row).count_ones() as usize;
            previous_row = row;
        }
        let perimeter = 4 * P - 2 * adjacent_edges;
        4.0 * (P as f64).sqrt() / (perimeter as f64)
    }
}

#[cfg(feature = "local")]
#[derive(Debug, Default, Clone)]
struct TraceStats {
    fallback_count: usize,
    counts: std::collections::BTreeMap<&'static str, i64>,
    times_ms: std::collections::BTreeMap<&'static str, f64>,
}

#[cfg(feature = "local")]
impl TraceStats {
    fn mark_fallback(&mut self) {
        self.fallback_count += 1;
    }

    fn count(&mut self, key: &'static str) {
        self.count_by(key, 1);
    }

    fn count_by(&mut self, key: &'static str, delta: i64) {
        *self.counts.entry(key).or_insert(0) += delta;
    }

    fn add_time_ms(&mut self, key: &'static str, ms: f64) {
        *self.times_ms.entry(key).or_insert(0.0) += ms;
    }

    fn summary(&self) {
        eprintln!("[summary] fallback_count={}", self.fallback_count);
        for (key, value) in &self.counts {
            eprintln!("[summary.count] {}={}", key, value);
        }
        for (key, value) in &self.times_ms {
            eprintln!("[summary.time_ms] {}={:.3}", key, value);
        }
    }
}

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {{
        $($body)*
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local {
    ($($body:tt)*) => {};
}

#[cfg(feature = "local")]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{
        let __local_time_start = std::time::Instant::now();
        let __local_time_result = { $body };
        $trace.add_time_ms($key, __local_time_start.elapsed().as_secs_f64() * 1000.0);
        __local_time_result
    }};
}

#[cfg(not(feature = "local"))]
#[allow(unused_macros)]
macro_rules! local_time {
    ($trace:expr, $key:expr, $body:block) => {{ $body }};
}

#[derive(Debug, Clone)]
struct TimeKeeper {
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
    fn new(time_limit_sec: f64, check_interval_log2: u32) -> Self {
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
    fn step(&mut self) -> bool {
        self.iter += 1;
        if (self.iter & self.check_mask) == 0 {
            self.force_update();
        }
        !self.is_over
    }

    /// 明示的に時計を更新したいときに使う
    #[inline(always)]
    fn force_update(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.elapsed_sec = elapsed;
        self.progress = (elapsed / self.time_limit_sec).clamp(0.0, 1.0);
        self.is_over = elapsed >= self.time_limit_sec;
    }

    /// batched な経過時間
    #[inline(always)]
    fn elapsed_sec(&self) -> f64 {
        self.elapsed_sec
    }

    /// batched な進捗率 [0, 1]
    #[inline(always)]
    fn progress(&self) -> f64 {
        self.progress
    }

    /// batched な時間切れ判定
    #[inline(always)]
    fn is_time_over(&self) -> bool {
        self.is_over
    }

    /// ログ用の正確な経過時間
    #[inline]
    fn exact_elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// ログ用の正確な残り時間
    #[inline]
    fn exact_remaining_sec(&self) -> f64 {
        (self.time_limit_sec - self.exact_elapsed_sec()).max(0.0)
    }
}

fn main() {
    // TimeKeeper は main 開始直後に作り、探索打ち切りには PROGRAM_TIME_LIMIT_SEC を使う。
    // フェーズ切替などの時間系パラメータは PROGRAM_TIME_LIMIT_SEC に対する割合で指定する。
    let _time_keeper = TimeKeeper::new(PROGRAM_TIME_LIMIT_SEC, 8);

    // ここに入力処理、各ターンの意思決定、出力など、version 固有の solver ロジックを実装する。
}
