// v073_from_scratch.rs
#![allow(non_snake_case)] // 問題文の `N`, `M` などを対応づけたまま使う。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::io::{self, BufRead, BufWriter, Write};
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

/// prefix 補正付き MAP 推定で使う、正規化前の時刻上限である。
const PREFIX_H: f64 = T_MAX as f64;
/// `theta in [2000, 8000]` を `beta = H / theta` へ写した探索区間である。
const PREFIX_BETA_MIN: f64 = 12.5;
const PREFIX_BETA_MAX: f64 = 50.0;

/// `N × N` 盤面の総マス数である。
const CELL_COUNT: usize = N * N;
const WORD_BITS: usize = u64::BITS as usize;
const WORD_COUNT: usize = (CELL_COUNT + WORD_BITS - 1) / WORD_BITS;
const ROW_MASK: u64 = (1_u64 << N) - 1;
const NO_COMPONENT: u16 = u16::MAX;
const MAX_WINDOW_SHORTLIST: usize = 64;
const MAX_COMPONENT_CANDIDATES: usize = 12;
const WINDOW_AREA_EXTRA_MIN: usize = 24;
const WINDOW_PERIMETER_SLACK: usize = 12;
const MAX_POLISH_STEPS: usize = 12;

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

#[derive(Debug, Clone, Copy)]
struct PrefixMapEval {
    value: f64,
    first: f64,
    second: f64,
}

/// 到着順 prefix の未到着情報を含めて、ケース固有の theta を逐次推定する。
///
/// 保持する状態は、観測数、`Y_n = sum(D[i] - 1)`、直前の
/// `beta = H / theta` の3つだけである。
struct PrefixThetaEstimator {
    duration_minus_one_sum: u64,
    duration_count: usize,
    prefix_beta_hat: f64,
    #[cfg(feature = "local")]
    trace: TraceStats,
}

impl PrefixThetaEstimator {
    fn new() -> Self {
        Self {
            duration_minus_one_sum: 0,
            duration_count: 0,
            prefix_beta_hat: PREFIX_H / 5_000.0,
            #[cfg(feature = "local")]
            trace: TraceStats::default(),
        }
    }

    /// 1グループ分の観測を取り込み、そのprefixに対するthetaのMAP推定値を返す。
    fn observe(&mut self, group: &GroupInput) -> f64 {
        #[cfg(feature = "local")]
        let theta_start = Instant::now();

        self.duration_minus_one_sum += (group.D() - 1) as u64;
        self.duration_count += 1;

        let use_interval_search = self.duration_count <= 32;
        let (beta, _guarded_steps) = if use_interval_search {
            (self.prefix_interval_map(group.S), 0)
        } else {
            self.prefix_newton_map(group.S)
        };
        assert!(beta.is_finite() && (PREFIX_BETA_MIN..=PREFIX_BETA_MAX).contains(&beta));
        self.prefix_beta_hat = beta;

        let theta = PREFIX_H / beta;
        #[cfg(feature = "local")]
        {
            self.trace.add_time_ms(
                "theta_estimation",
                theta_start.elapsed().as_secs_f64() * 1_000.0,
            );
            self.trace.count("theta_prefix_map_turn");
            if use_interval_search {
                self.trace.count("theta_prefix_interval_turn");
            } else {
                self.trace.count("theta_prefix_newton_turn");
            }
            self.trace
                .count_by("theta_prefix_guarded_step", _guarded_steps as i64);
            self.trace.count_by(
                "theta_prefix_map_value_milli",
                (theta * 1_000.0).round() as i64,
            );

            let raw_theta = ((self.duration_minus_one_sum as f64) / (self.duration_count as f64))
                .clamp(2_000.0, 8_000.0);
            let delta_milli = ((theta - raw_theta) * 1_000.0).round() as i64;
            self.trace
                .count_by("theta_prefix_minus_raw_milli", delta_milli);
            if delta_milli < 0 {
                self.trace.count("theta_prefix_downshift_turn");
            }
        }
        theta
    }

    /// `u_k(beta,a) = integral_0^a z^k beta exp(-beta z) dz` を返す。
    /// `beta*a` が小さい領域では、漸化式の差し引きによる桁落ちを級数で避ける。
    fn prefix_moments(beta: f64, a: f64, max_k: usize) -> [f64; 10] {
        let mut u = [0.0; 10];
        let x = beta * a;
        if x < 8.0 {
            let mut a_power = a;
            for k in 0..=max_k {
                let mut term = 1.0 / ((k + 1) as f64);
                let mut sum = term;
                for j in 1..=128 {
                    term *= -x / (j as f64) * ((k + j) as f64) / ((k + j + 1) as f64);
                    sum += term;
                    if term.abs() <= 1e-17 * sum.abs().max(1.0) {
                        break;
                    }
                }
                u[k] = beta * a_power * sum;
                a_power *= a;
            }
        } else {
            let exp_neg = (-x).exp();
            u[0] = -(-x).exp_m1();
            let mut a_power = a;
            for k in 1..=max_k {
                u[k] = (k as f64) * u[k - 1] / beta - a_power * exp_neg;
                a_power *= a;
            }
        }
        u
    }

    /// theta側の事後密度へ`beta = H / theta`を代入した目的関数とbeta微分。
    /// 変数変換のヤコビアンは掛けず、theta密度のMAPを求める。
    fn prefix_map_eval(beta: f64, n: usize, Y: u64, S: usize, derivatives: bool) -> PrefixMapEval {
        let r = M - n;
        let mut result = PrefixMapEval {
            value: (n as f64) * beta.ln() - (Y as f64 / PREFIX_H) * beta,
            first: (n as f64) / beta - Y as f64 / PREFIX_H,
            second: -(n as f64) / (beta * beta),
        };
        if r == 0 {
            return result;
        }

        let a = (PREFIX_H - S as f64) / PREFIX_H;
        let elapsed_fraction = S as f64 / PREFIX_H;
        let u = Self::prefix_moments(beta, a, if derivatives { 9 } else { 7 });
        let q = u[0] - elapsed_fraction * u[..=7].iter().sum::<f64>();
        assert!(
            q > 0.0 && q.is_finite(),
            "invalid prefix survival probability"
        );
        result.value += (r as f64) * q.ln();

        if derivatives {
            let q_first = (u[0] / beta - u[1])
                - elapsed_fraction * (0..=7).map(|k| u[k] / beta - u[k + 1]).sum::<f64>();
            let q_second = (u[2] - 2.0 * u[1] / beta)
                - elapsed_fraction
                    * (0..=7)
                        .map(|k| u[k + 2] - 2.0 * u[k + 1] / beta)
                        .sum::<f64>();
            let ratio = q_first / q;
            result.first += (r as f64) * ratio;
            result.second += (r as f64) * (q_second / q - ratio * ratio);
        }
        result
    }

    /// 最初の32グループでは、固定12回の目的関数評価で区間探索する。
    fn prefix_interval_map(&self, S: usize) -> f64 {
        const INV_PHI: f64 = 0.618_033_988_749_894_9;
        let mut lo = PREFIX_BETA_MIN;
        let mut hi = PREFIX_BETA_MAX;
        let mut x1 = hi - INV_PHI * (hi - lo);
        let mut x2 = lo + INV_PHI * (hi - lo);
        let mut f1 = Self::prefix_map_eval(
            x1,
            self.duration_count,
            self.duration_minus_one_sum,
            S,
            false,
        )
        .value;
        let mut f2 = Self::prefix_map_eval(
            x2,
            self.duration_count,
            self.duration_minus_one_sum,
            S,
            false,
        )
        .value;

        for _ in 2..12 {
            if f1 < f2 {
                lo = x1;
                x1 = x2;
                f1 = f2;
                x2 = lo + INV_PHI * (hi - lo);
                f2 = Self::prefix_map_eval(
                    x2,
                    self.duration_count,
                    self.duration_minus_one_sum,
                    S,
                    false,
                )
                .value;
            } else {
                hi = x2;
                x2 = x1;
                f2 = f1;
                x1 = hi - INV_PHI * (hi - lo);
                f1 = Self::prefix_map_eval(
                    x1,
                    self.duration_count,
                    self.duration_minus_one_sum,
                    S,
                    false,
                )
                .value;
            }
        }
        if f1 >= f2 {
            x1
        } else {
            x2
        }
    }

    /// 33グループ目以降は、直前のMAPから保護付きNewton更新を2回行う。
    fn prefix_newton_map(&self, S: usize) -> (f64, usize) {
        let mut beta = self.prefix_beta_hat.clamp(PREFIX_BETA_MIN, PREFIX_BETA_MAX);
        let mut lo = PREFIX_BETA_MIN;
        let mut hi = PREFIX_BETA_MAX;
        let mut guarded_steps = 0;

        for _ in 0..2 {
            let value = Self::prefix_map_eval(
                beta,
                self.duration_count,
                self.duration_minus_one_sum,
                S,
                true,
            );
            assert!(
                value.value.is_finite() && value.first.is_finite() && value.second.is_finite(),
                "invalid prefix MAP derivatives"
            );
            if value.first > 0.0 {
                lo = lo.max(beta);
            } else {
                hi = hi.min(beta);
            }
            if hi <= lo {
                beta = lo;
                continue;
            }

            let raw = beta - value.first / value.second;
            if value.second < 0.0 && raw.is_finite() && raw >= lo && raw <= hi {
                beta = raw;
            } else {
                guarded_steps += 1;
                beta = if value.second < 0.0 && raw.is_finite() {
                    raw.clamp(lo, hi)
                } else {
                    0.5 * (lo + hi)
                };
            }
        }
        (beta, guarded_steps)
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
        let perimeter = Self::region_perimeter(region, P);
        4.0 * (P as f64).sqrt() / (perimeter as f64)
    }

    fn region_perimeter(region: &CellSet, P: usize) -> usize {
        let mut adjacent_edges = 0_usize;
        let mut previous_row = 0_u64;
        for x in 0..N {
            let row = region.row_bits(x);
            adjacent_edges += (row & (row >> 1)).count_ones() as usize;
            adjacent_edges += (row & previous_row).count_ones() as usize;
            previous_row = row;
        }
        4 * P - 2 * adjacent_edges
    }
}

/// 現在の空き芝生を連結成分に分ける。
/// `component_by_cell[id]` と `component_sizes` の組が、各空きマスの連結度合を表す。
struct FreeComponents {
    free_cells: CellSet,
    component_by_cell: [u16; CELL_COUNT],
    component_sizes: Vec<usize>,
    component_cells: Vec<Vec<CellId>>,
    connectivity_potential: u64,
}

impl FreeComponents {
    fn build(initial: &InitialInput, state: &State) -> Self {
        let mut free_cells = initial.grass_cells.clone();
        free_cells.difference_with(&state.occupied);

        let mut component_by_cell = [NO_COMPONENT; CELL_COUNT];
        let mut component_sizes = Vec::new();
        let mut component_cells = Vec::new();
        let mut stack = Vec::with_capacity(CELL_COUNT);

        for start in 0..CELL_COUNT {
            if !free_cells.contains_id(start) || component_by_cell[start] != NO_COMPONENT {
                continue;
            }

            let component_id = component_sizes.len() as u16;
            let mut cells = Vec::new();
            component_by_cell[start] = component_id;
            stack.push(start);

            while let Some(id) = stack.pop() {
                cells.push(id);
                for next in neighbor_ids(id) {
                    if next < CELL_COUNT
                        && free_cells.contains_id(next)
                        && component_by_cell[next] == NO_COMPONENT
                    {
                        component_by_cell[next] = component_id;
                        stack.push(next);
                    }
                }
            }

            component_sizes.push(cells.len());
            component_cells.push(cells);
        }

        let connectivity_potential = component_sizes
            .iter()
            .map(|&size| (size * size) as u64)
            .sum();

        Self {
            free_cells,
            component_by_cell,
            component_sizes,
            component_cells,
            connectivity_potential,
        }
    }
}

struct PlacementCandidate {
    region: CellSet,
    component_size: usize,
    perimeter: usize,
    compactness: f64,
    fee: i64,
    connectivity_loss: u64,
    fragmentation_loss: u64,
    residual_component_count: usize,
    temporal_contact: f64,
}

struct PlacementSearch {
    choice: Option<PlacementCandidate>,
    free_component_count: usize,
    eligible_component_count: usize,
    candidate_count: usize,
    shortlisted_window_count: usize,
    polish_swap_count: usize,
    connectivity_potential: u64,
}

#[inline]
fn neighbor_ids(id: CellId) -> [CellId; 4] {
    let x = id / N;
    let y = id % N;
    [
        if x > 0 { id - N } else { CELL_COUNT },
        if x + 1 < N { id + N } else { CELL_COUNT },
        if y > 0 { id - 1 } else { CELL_COUNT },
        if y + 1 < N { id + 1 } else { CELL_COUNT },
    ]
}

#[inline]
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[inline]
fn cell_hash(group_i: usize, id: CellId) -> u64 {
    mix64((group_i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ id as u64)
}

#[inline]
fn region_neighbor_count(region: &CellSet, id: CellId) -> usize {
    neighbor_ids(id)
        .into_iter()
        .filter(|&next| next < CELL_COUNT && region.contains_id(next))
        .count()
}

fn temporal_weights(
    state: &State,
    history: &InputHistory,
    group: &GroupInput,
    theta: f64,
) -> Vec<f64> {
    let mut weights = vec![0.0; M];
    for active in &state.active_groups {
        let other_T = history.groups[active.i].T;
        let delta_T = group.T.abs_diff(other_T) as f64;
        weights[active.i] = (-delta_T / theta).exp();
    }
    weights
}

fn cell_temporal_contact(state: &State, weights: &[f64], id: CellId) -> f64 {
    neighbor_ids(id)
        .into_iter()
        .filter(|&next| next < CELL_COUNT)
        .map(|next| {
            let owner = state.owner[next];
            if owner == NO_OWNER {
                0.0
            } else {
                weights[owner as usize]
            }
        })
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RectWindow {
    x0: usize,
    y0: usize,
    h: usize,
    w: usize,
}

#[inline]
fn prefix_sum(prefix: &[i64], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
    let stride = N + 1;
    prefix[x1 * stride + y1] - prefix[x0 * stride + y1] - prefix[x1 * stride + y0]
        + prefix[x0 * stride + y0]
}

fn build_window_prefixes(
    components: &FreeComponents,
    state: &State,
    weights: &[f64],
) -> (Vec<i64>, Vec<i64>) {
    let stride = N + 1;
    let mut free_prefix = vec![0_i64; stride * stride];
    let mut contact_prefix = vec![0_i64; stride * stride];

    for x in 0..N {
        for y in 0..N {
            let id = x * N + y;
            let is_free = components.free_cells.contains_id(id);
            let free_value = is_free as i64;
            let contact_value = if is_free {
                (cell_temporal_contact(state, weights, id) * 1_000.0).round() as i64
            } else {
                0
            };
            let index = (x + 1) * stride + y + 1;
            free_prefix[index] =
                free_value + free_prefix[x * stride + y + 1] + free_prefix[(x + 1) * stride + y]
                    - free_prefix[x * stride + y];
            contact_prefix[index] = contact_value
                + contact_prefix[x * stride + y + 1]
                + contact_prefix[(x + 1) * stride + y]
                - contact_prefix[x * stride + y];
        }
    }

    (free_prefix, contact_prefix)
}

fn minimum_rectangle_perimeter(P: usize) -> usize {
    (1..=N)
        .filter_map(|h| {
            let w = (P + h - 1) / h;
            (w <= N).then_some(2 * (h + w))
        })
        .min()
        .unwrap()
}

fn shortlist_windows(
    components: &FreeComponents,
    state: &State,
    weights: &[f64],
    group: &GroupInput,
) -> Vec<RectWindow> {
    type WindowEntry = (usize, usize, Reverse<i64>, usize, u64, RectWindow);

    let (free_prefix, contact_prefix) = build_window_prefixes(components, state, weights);
    let min_perimeter = minimum_rectangle_perimeter(group.P);
    let max_area = group.P + WINDOW_AREA_EXTRA_MIN.max(group.P / 2);
    let mut heap = BinaryHeap::<WindowEntry>::new();

    for h in 1..=N {
        for w in 1..=N {
            let area = h * w;
            let box_perimeter = 2 * (h + w);
            if area < group.P
                || area > max_area
                || box_perimeter > min_perimeter + WINDOW_PERIMETER_SLACK
            {
                continue;
            }

            for x0 in 0..=N - h {
                for y0 in 0..=N - w {
                    let x1 = x0 + h;
                    let y1 = y0 + w;
                    let free_count = prefix_sum(&free_prefix, x0, y0, x1, y1) as usize;
                    if free_count < group.P {
                        continue;
                    }
                    let contact = prefix_sum(&contact_prefix, x0, y0, x1, y1);
                    let window = RectWindow { x0, y0, h, w };
                    let tie_hash =
                        cell_hash(group.i, x0 * N + y0) ^ mix64(((h as u64) << 32) | w as u64);
                    let entry = (
                        box_perimeter,
                        free_count - group.P,
                        Reverse(contact),
                        area - free_count,
                        tie_hash,
                        window,
                    );

                    if heap.len() < MAX_WINDOW_SHORTLIST {
                        heap.push(entry);
                    } else if entry < *heap.peek().unwrap() {
                        heap.pop();
                        heap.push(entry);
                    }
                }
            }
        }
    }

    let mut entries = heap.into_vec();
    entries.sort_unstable();
    entries
        .into_iter()
        .map(|(_, _, _, _, _, window)| window)
        .collect()
}

fn connected_pieces_in_window(
    components: &FreeComponents,
    window: RectWindow,
    P: usize,
) -> Vec<Vec<CellId>> {
    let mut visited = CellSet::default();
    let mut stack = Vec::new();
    let mut pieces = Vec::new();
    let x1 = window.x0 + window.h;
    let y1 = window.y0 + window.w;

    for x in window.x0..x1 {
        for y in window.y0..y1 {
            let start = x * N + y;
            if !components.free_cells.contains_id(start) || visited.contains_id(start) {
                continue;
            }

            let mut cells = Vec::new();
            visited.insert_id(start);
            stack.push(start);
            while let Some(id) = stack.pop() {
                cells.push(id);
                for next in neighbor_ids(id) {
                    if next >= CELL_COUNT {
                        continue;
                    }
                    let nx = next / N;
                    let ny = next % N;
                    if nx >= window.x0
                        && nx < x1
                        && ny >= window.y0
                        && ny < y1
                        && components.free_cells.contains_id(next)
                        && !visited.contains_id(next)
                    {
                        visited.insert_id(next);
                        stack.push(next);
                    }
                }
            }

            if cells.len() >= P {
                pieces.push(cells);
            }
        }
    }

    pieces
}

#[inline]
fn distance_from_center(id: CellId, center_x2: isize, center_y2: isize) -> usize {
    let x2 = 2 * (id / N) as isize;
    let y2 = 2 * (id % N) as isize;
    (x2 - center_x2).unsigned_abs() + (y2 - center_y2).unsigned_abs()
}

fn bounding_center(cells: &[CellId]) -> (isize, isize) {
    let mut min_x = N;
    let mut max_x = 0;
    let mut min_y = N;
    let mut max_y = 0;
    for &id in cells {
        let x = id / N;
        let y = id % N;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    ((min_x + max_x) as isize, (min_y + max_y) as isize)
}

/// 窓内の連結領域から全域木の葉を取り除き、連結性を保ったまま `P` マスへ削る。
/// 低次数の突起を先に落とし、同次数なら窓の中心から遠いマスを先に落とす。
fn peel_connected_region(
    cells: &[CellId],
    P: usize,
    group_i: usize,
    center_x2: isize,
    center_y2: isize,
) -> (CellSet, usize) {
    type LeafEntry = (Reverse<u8>, usize, Reverse<u64>, Reverse<CellId>);

    let mut region = CellSet::default();
    for &id in cells {
        region.insert_id(id);
    }
    let mut perimeter = State::region_perimeter(&region, cells.len());
    if cells.len() == P {
        return (region, perimeter);
    }

    let root = *cells
        .iter()
        .min_by_key(|&&id| {
            (
                distance_from_center(id, center_x2, center_y2),
                cell_hash(group_i, id),
            )
        })
        .unwrap();
    let mut parent = [CELL_COUNT; CELL_COUNT];
    let mut child_count = [0_u8; CELL_COUNT];
    let mut queue = VecDeque::new();
    parent[root] = root;
    queue.push_back(root);
    while let Some(id) = queue.pop_front() {
        for next in neighbor_ids(id) {
            if next < CELL_COUNT && region.contains_id(next) && parent[next] == CELL_COUNT {
                parent[next] = id;
                child_count[id] += 1;
                queue.push_back(next);
            }
        }
    }

    let mut heap = BinaryHeap::<LeafEntry>::new();
    let push_leaf = |id: CellId, region: &CellSet, heap: &mut BinaryHeap<LeafEntry>| {
        let degree = region_neighbor_count(region, id) as u8;
        heap.push((
            Reverse(degree),
            distance_from_center(id, center_x2, center_y2),
            Reverse(cell_hash(group_i, id)),
            Reverse(id),
        ));
    };
    for &id in cells {
        if id != root && child_count[id] == 0 {
            push_leaf(id, &region, &mut heap);
        }
    }

    let mut region_size = cells.len();
    while region_size > P {
        let (Reverse(stored_degree), _, _, Reverse(id)) = heap.pop().unwrap();
        if !region.contains_id(id) || id == root || child_count[id] != 0 {
            continue;
        }
        let degree = region_neighbor_count(&region, id);
        if degree != stored_degree as usize {
            push_leaf(id, &region, &mut heap);
            continue;
        }

        region.remove_id(id);
        region_size -= 1;
        perimeter = (perimeter as isize - 4 + 2 * degree as isize) as usize;

        let p = parent[id];
        child_count[p] -= 1;
        if p != root && child_count[p] == 0 {
            push_leaf(p, &region, &mut heap);
        }

        for next in neighbor_ids(id) {
            if next < CELL_COUNT
                && region.contains_id(next)
                && next != root
                && child_count[next] == 0
            {
                push_leaf(next, &region, &mut heap);
            }
        }
    }

    (region, perimeter)
}

fn region_is_connected(region: &CellSet, P: usize) -> bool {
    let first_word = region.words.iter().position(|&word| word != 0).unwrap();
    let start = first_word * WORD_BITS + region.words[first_word].trailing_zeros() as usize;
    let mut visited = CellSet::default();
    let mut stack = vec![start];
    visited.insert_id(start);
    let mut count = 0;
    while let Some(id) = stack.pop() {
        count += 1;
        for next in neighbor_ids(id) {
            if next < CELL_COUNT && region.contains_id(next) && !visited.contains_id(next) {
                visited.insert_id(next);
                stack.push(next);
            }
        }
    }
    count == P
}

/// 完成候補について、境界1マスを別の境界へ移す交換だけを試す。
/// 元候補も残すため、連結性指標を悪化させた改善形は最終比較で自然に落ちる。
fn polish_region(
    components: &FreeComponents,
    group: &GroupInput,
    mut region: CellSet,
    mut perimeter: usize,
) -> (CellSet, usize, usize) {
    let mut improvement_count = 0;

    for _ in 0..MAX_POLISH_STEPS {
        let mut region_cells = Vec::with_capacity(group.P);
        let mut frontier = CellSet::default();
        for word_index in 0..WORD_COUNT {
            let mut bits = region.words[word_index];
            while bits != 0 {
                let id = word_index * WORD_BITS + bits.trailing_zeros() as usize;
                region_cells.push(id);
                for next in neighbor_ids(id) {
                    if next < CELL_COUNT
                        && components.free_cells.contains_id(next)
                        && !region.contains_id(next)
                    {
                        frontier.insert_id(next);
                    }
                }
                bits &= bits - 1;
            }
        }

        let mut frontier_cells = Vec::new();
        for word_index in 0..WORD_COUNT {
            let mut bits = frontier.words[word_index];
            while bits != 0 {
                let id = word_index * WORD_BITS + bits.trailing_zeros() as usize;
                frontier_cells.push((id, region_neighbor_count(&region, id)));
                bits &= bits - 1;
            }
        }

        let mut moves = Vec::new();
        for &removed in &region_cells {
            let removed_degree = region_neighbor_count(&region, removed);
            if removed_degree == 4 {
                continue;
            }
            for &(added, base_added_degree) in &frontier_cells {
                let touches_removed = neighbor_ids(added).contains(&removed) as usize;
                let added_degree = base_added_degree - touches_removed;
                if added_degree > removed_degree {
                    let new_perimeter = perimeter + 2 * removed_degree - 2 * added_degree;
                    let tie_hash = mix64(((removed as u64) << 32) | added as u64);
                    moves.push((new_perimeter, tie_hash, removed, added));
                }
            }
        }
        moves.sort_unstable();

        let mut improved = false;
        for (new_perimeter, _, removed, added) in moves {
            let mut next_region = region.clone();
            next_region.remove_id(removed);
            next_region.insert_id(added);
            if region_is_connected(&next_region, group.P) {
                region = next_region;
                perimeter = new_perimeter;
                improvement_count += 1;
                improved = true;
                break;
            }
        }
        if !improved {
            break;
        }
    }

    (region, perimeter, improvement_count)
}

fn residual_connectivity(
    components: &FreeComponents,
    component_id: usize,
    region: &CellSet,
) -> (u64, usize) {
    let mut visited = CellSet::default();
    let mut stack = Vec::new();
    let mut residual_potential = 0_u64;
    let mut residual_component_count = 0_usize;

    for &start in &components.component_cells[component_id] {
        if region.contains_id(start) || visited.contains_id(start) {
            continue;
        }

        let mut size = 0_usize;
        visited.insert_id(start);
        stack.push(start);
        while let Some(id) = stack.pop() {
            size += 1;
            for next in neighbor_ids(id) {
                if next < CELL_COUNT
                    && components.component_by_cell[next] as usize == component_id
                    && !region.contains_id(next)
                    && !visited.contains_id(next)
                {
                    visited.insert_id(next);
                    stack.push(next);
                }
            }
        }

        residual_potential += (size * size) as u64;
        residual_component_count += 1;
    }

    (residual_potential, residual_component_count)
}

fn evaluate_candidate(
    components: &FreeComponents,
    state: &State,
    group: &GroupInput,
    weights: &[f64],
    region: CellSet,
    perimeter: usize,
) -> PlacementCandidate {
    let first_word = region.words.iter().position(|&word| word != 0).unwrap();
    let first_id = first_word * WORD_BITS + region.words[first_word].trailing_zeros() as usize;
    let component_id = components.component_by_cell[first_id] as usize;
    let component_size = components.component_sizes[component_id];
    let (residual_potential, residual_component_count) =
        residual_connectivity(components, component_id, &region);
    let connectivity_loss = (component_size * component_size) as u64 - residual_potential;
    let remaining_size = component_size - group.P;
    let fragmentation_loss = (remaining_size * remaining_size) as u64 - residual_potential;

    let mut temporal_contact = 0.0;
    for word_index in 0..WORD_COUNT {
        let mut bits = region.words[word_index];
        while bits != 0 {
            let id = word_index * WORD_BITS + bits.trailing_zeros() as usize;
            temporal_contact += cell_temporal_contact(state, weights, id);
            bits &= bits - 1;
        }
    }

    let compactness = 4.0 * (group.P as f64).sqrt() / (perimeter as f64);
    let fee = ((group.V as f64) * compactness).round() as i64;
    PlacementCandidate {
        region,
        component_size,
        perimeter,
        compactness,
        fee,
        connectivity_loss,
        fragmentation_loss,
        residual_component_count,
        temporal_contact,
    }
}

fn candidate_is_better(candidate: &PlacementCandidate, current: &PlacementCandidate) -> bool {
    if candidate.connectivity_loss != current.connectivity_loss {
        return candidate.connectivity_loss < current.connectivity_loss;
    }
    if candidate.temporal_contact != current.temporal_contact {
        return candidate.temporal_contact > current.temporal_contact;
    }
    if candidate.fee != current.fee {
        return candidate.fee > current.fee;
    }
    candidate.perimeter < current.perimeter
}

fn select_candidate_index(
    candidates: &[PlacementCandidate],
    initial: &InitialInput,
    group: &GroupInput,
) -> usize {
    let best_fee = candidates
        .iter()
        .map(|candidate| candidate.fee)
        .max()
        .unwrap();
    let fee_loss_limit = (((group.V as f64) * initial.R).round() as i64).max(1);
    let mut choice_index = None;
    for (index, candidate) in candidates.iter().enumerate() {
        if best_fee - candidate.fee > fee_loss_limit {
            continue;
        }
        match choice_index {
            None => choice_index = Some(index),
            Some(current_index) => {
                if candidate_is_better(candidate, &candidates[current_index]) {
                    choice_index = Some(index);
                }
            }
        }
    }
    choice_index.unwrap()
}

fn search_greedy_placement(
    initial: &InitialInput,
    state: &State,
    history: &InputHistory,
    group: &GroupInput,
    theta: f64,
) -> PlacementSearch {
    let components = FreeComponents::build(initial, state);
    let mut eligible_components: Vec<usize> = components
        .component_sizes
        .iter()
        .enumerate()
        .filter_map(|(component_id, &size)| (size >= group.P).then_some(component_id))
        .collect();
    eligible_components.sort_unstable_by_key(|&component_id| {
        (
            components.component_sizes[component_id] - group.P,
            component_id,
        )
    });

    let free_component_count = components.component_sizes.len();
    let eligible_component_count = eligible_components.len();
    let connectivity_potential = components.connectivity_potential;
    if eligible_components.is_empty() {
        return PlacementSearch {
            choice: None,
            free_component_count,
            eligible_component_count,
            candidate_count: 0,
            shortlisted_window_count: 0,
            polish_swap_count: 0,
            connectivity_potential,
        };
    }

    let weights = temporal_weights(state, history, group, theta);
    let windows = shortlist_windows(&components, state, &weights, group);
    let shortlisted_window_count = windows.len();
    let mut raw_candidates = Vec::<(CellSet, usize)>::new();

    // 各小成分そのものからも1候補を削り出す。これにより矩形窓が一つも
    // 成立しない複雑な成分でも、置けるなら必ず合法候補を一つ保持できる。
    for &component_id in eligible_components.iter().take(MAX_COMPONENT_CANDIDATES) {
        let cells = &components.component_cells[component_id];
        let (center_x2, center_y2) = bounding_center(cells);
        let candidate = peel_connected_region(cells, group.P, group.i, center_x2, center_y2);
        if !raw_candidates
            .iter()
            .any(|(region, _)| *region == candidate.0)
        {
            raw_candidates.push(candidate);
        }
    }

    for window in windows {
        let center_x2 = (2 * window.x0 + window.h - 1) as isize;
        let center_y2 = (2 * window.y0 + window.w - 1) as isize;
        for cells in connected_pieces_in_window(&components, window, group.P) {
            let candidate = peel_connected_region(&cells, group.P, group.i, center_x2, center_y2);
            if !raw_candidates
                .iter()
                .any(|(region, _)| *region == candidate.0)
            {
                raw_candidates.push(candidate);
            }
        }
    }

    let mut candidates = Vec::with_capacity(raw_candidates.len() + 2);
    for (region, perimeter) in raw_candidates {
        candidates.push(evaluate_candidate(
            &components,
            state,
            group,
            &weights,
            region,
            perimeter,
        ));
    }

    // 連結性優先の暫定選択と、純粋な最高利用料候補の双方を局所整形する。
    // 元候補も残し、整形による将来連結性の悪化は最終比較に判断させる。
    let preliminary_choice = select_candidate_index(&candidates, initial, group);
    let best_fee_index = candidates
        .iter()
        .enumerate()
        .max_by_key(|(_, candidate)| candidate.fee)
        .map(|(index, _)| index)
        .unwrap();
    let mut polish_indices = vec![preliminary_choice];
    if best_fee_index != preliminary_choice {
        polish_indices.push(best_fee_index);
    }
    let polish_sources: Vec<(CellSet, usize)> = polish_indices
        .into_iter()
        .map(|index| {
            (
                candidates[index].region.clone(),
                candidates[index].perimeter,
            )
        })
        .collect();
    let mut polish_swap_count = 0;
    for (region, perimeter) in polish_sources {
        let (polished, polished_perimeter, swap_count) =
            polish_region(&components, group, region, perimeter);
        polish_swap_count += swap_count;
        if swap_count == 0
            || candidates
                .iter()
                .any(|candidate| candidate.region == polished)
        {
            continue;
        }
        candidates.push(evaluate_candidate(
            &components,
            state,
            group,
            &weights,
            polished,
            polished_perimeter,
        ));
    }

    let candidate_count = candidates.len();
    let choice_index = select_candidate_index(&candidates, initial, group);
    PlacementSearch {
        choice: Some(candidates.swap_remove(choice_index)),
        free_component_count,
        eligible_component_count,
        candidate_count,
        shortlisted_window_count,
        polish_swap_count,
        connectivity_potential,
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
    ($trace:expr, $key:expr, $body:block) => {{
        $body
    }};
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

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = InputReader::new(stdin.lock());
    let mut output = BufWriter::new(stdout.lock());

    let initial = InitialInput::read(&mut input);
    let mut history = InputHistory::new();
    let mut theta_estimator = PrefixThetaEstimator::new();
    let mut state = State::new();

    for _ in 0..M {
        let group = GroupInput::read(&mut input, &mut history);
        let theta = theta_estimator.observe(&group);
        state.process_departures(&history);

        let search = local_time!(theta_estimator.trace, "placement", {
            search_greedy_placement(&initial, &state, &history, &group, theta)
        });

        local! {
            theta_estimator.trace.count_by(
                "free_component_count_sum",
                search.free_component_count as i64,
            );
            theta_estimator.trace.count_by(
                "eligible_component_count_sum",
                search.eligible_component_count as i64,
            );
            theta_estimator.trace.count_by(
                "placement_candidate_count",
                search.candidate_count as i64,
            );
            theta_estimator.trace.count_by(
                "shortlisted_window_count",
                search.shortlisted_window_count as i64,
            );
            theta_estimator.trace.count_by(
                "polish_swap_count",
                search.polish_swap_count as i64,
            );
            theta_estimator.trace.count_by(
                "connectivity_potential_sum",
                search.connectivity_potential as i64,
            );
            if group.i == 65 {
                theta_estimator.trace.count_by(
                    "group65_candidate_count",
                    search.candidate_count as i64,
                );
                theta_estimator.trace.count_by(
                    "group65_window_count",
                    search.shortlisted_window_count as i64,
                );
            }
        }

        let decision = match search.choice {
            Some(candidate) => {
                local! {
                    theta_estimator.trace.count("greedy_accept");
                    theta_estimator.trace.count_by(
                        "accepted_component_slack_sum",
                        (candidate.component_size - group.P) as i64,
                    );
                    theta_estimator.trace.count_by(
                        "accepted_perimeter_sum",
                        candidate.perimeter as i64,
                    );
                    theta_estimator.trace.count_by(
                        "accepted_compactness_milli_sum",
                        (candidate.compactness * 1_000.0).round() as i64,
                    );
                    theta_estimator.trace.count_by("accepted_fee_sum", candidate.fee);
                    theta_estimator.trace.count_by(
                        "accepted_connectivity_loss_sum",
                        candidate.connectivity_loss as i64,
                    );
                    theta_estimator.trace.count_by(
                        "accepted_fragmentation_loss_sum",
                        candidate.fragmentation_loss as i64,
                    );
                    theta_estimator.trace.count_by(
                        "accepted_residual_component_count_sum",
                        candidate.residual_component_count as i64,
                    );
                    theta_estimator.trace.count_by(
                        "accepted_temporal_contact_milli_sum",
                        (candidate.temporal_contact * 1_000.0).round() as i64,
                    );
                    if candidate.component_size == group.P {
                        theta_estimator.trace.count("exact_fit_accept");
                    }
                    if candidate.fragmentation_loss > 0 {
                        theta_estimator.trace.count("fragmenting_accept");
                    }
                    if group.i == 65 {
                        theta_estimator.trace.count_by(
                            "group65_perimeter",
                            candidate.perimeter as i64,
                        );
                        theta_estimator.trace.count_by("group65_fee", candidate.fee);
                        theta_estimator.trace.count_by(
                            "group65_connectivity_loss",
                            candidate.connectivity_loss as i64,
                        );
                    }
                }
                ArrivalDecision::Accept {
                    region: candidate.region,
                }
            }
            None => {
                local! {
                    theta_estimator.trace.count("greedy_reject");
                }
                ArrivalDecision::Reject
            }
        };

        let turn_output = TurnOutput {
            relocations: Vec::new(),
            decision,
        };
        state.apply_output(&initial, &history, &turn_output);
        turn_output.write_and_flush(&mut output);
    }

    state.process_final_departures(&history);

    local! {
        theta_estimator
            .trace
            .count_by("final_abs_score", state.abs_score());
        theta_estimator.trace.summary();
    }
}
