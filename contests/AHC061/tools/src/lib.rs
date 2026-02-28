#![allow(non_snake_case, unused_macros)]

use proconio::input;
use rand::prelude::*;
use std::io::prelude::*;
use std::io::BufReader;
use std::ops::RangeBounds;
use std::process::ChildStdout;
use svg::node::{
    element::{Circle, Group, Rectangle, Style, Title},
    Text,
};

pub trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}
impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }
    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

#[macro_export]
macro_rules! mat {
	($($e:expr),*) => { Vec::from(vec![$($e),*]) };
	($($e:expr,)*) => { Vec::from(vec![$($e),*]) };
	($e:expr; $d:expr) => { Vec::from(vec![$e; $d]) };
	($e:expr; $d:expr $(; $ds:expr)+) => { Vec::from(vec![mat![$e $(; $ds)*]; $d]) };
}

#[derive(Clone, Debug)]
pub struct Input {
    pub N: usize,
    pub M: usize,
    pub T: usize,
    pub U: usize,
    pub V: Vec<Vec<usize>>,
    pub xy: Vec<(usize, usize)>, // 各プレイヤーの初期位置
    pub wa: Vec<f64>,
    pub wb: Vec<f64>,
    pub wc: Vec<f64>,
    pub wd: Vec<f64>,
    pub eps: Vec<f64>,
    pub r: Vec<Vec<f64>>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {} {}", self.N, self.M, self.T, self.U)?;
        for i in 0..self.N {
            for j in 0..self.N {
                write!(f, "{}", self.V[i][j])?;
                if j < self.N - 1 {
                    write!(f, " ")?;
                }
            }
            writeln!(f)?;
        }
        for i in 0..self.M {
            writeln!(f, "{} {}", self.xy[i].0, self.xy[i].1)?;
        }
        for i in 0..(self.M - 1) {
            writeln!(
                f,
                "{} {} {} {} {}",
                self.wa[i], self.wb[i], self.wc[i], self.wd[i], self.eps[i]
            )?;
        }
        for t in 0..self.T {
            for i in 0..(self.M - 1) {
                writeln!(f, "{} {}", self.r[i][2 * t], self.r[i][2 * t + 1])?;
            }
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let mut f = proconio::source::once::OnceSource::from(f);
    input! {
        from &mut f,
        N: usize, M: usize, T: usize, U: usize,
        V: [[usize; N]; N],
        xy: [(usize, usize); M],
    }
    let mut wa = vec![0.0; M - 1];
    let mut wb = vec![0.0; M - 1];
    let mut wc = vec![0.0; M - 1];
    let mut wd = vec![0.0; M - 1];
    let mut eps = vec![0.0; M - 1];
    for i in 0..(M - 1) {
        input! {
            from &mut f,
            w: (f64, f64, f64, f64, f64),
        }
        wa[i] = w.0;
        wb[i] = w.1;
        wc[i] = w.2;
        wd[i] = w.3;
        eps[i] = w.4;
    }
    let mut r = vec![vec![0.0; 2 * T]; M - 1];
    for tt in 0..T {
        for i in 0..(M - 1) {
            input! {
                from &mut f,
                val1: f64,
                val2: f64,
            }
            r[i][2 * tt] = val1;
            r[i][2 * tt + 1] = val2;
        }
    }
    Input {
        N,
        M,
        T,
        U,
        V,
        xy,
        wa,
        wb,
        wc,
        wd,
        eps,
        r,
    }
}

pub fn read<T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr, R: RangeBounds<T>>(
    token: Option<&str>,
    range: R,
) -> Result<T, String> {
    if let Some(v) = token {
        if let Ok(v) = v.parse::<T>() {
            if !range.contains(&v) {
                Err(format!("Out of range: {}", v))
            } else {
                Ok(v)
            }
        } else {
            Err(format!("Parse error: {}", v))
        }
    } else {
        Err("Unexpected EOF".to_owned())
    }
}

// ゲームの状態を表す構造体
#[derive(Clone, Debug)]
pub struct State {
    pub pos: Vec<(usize, usize)>, // 各プレイヤーの駒の位置（ターン開始時の位置）
    pub owner: Vec<Vec<i32>>,     // 各マスの所有者 (-1: 無所有)
    pub level: Vec<Vec<usize>>,   // 各マスのレベル
    pub selected: Vec<(usize, usize)>, // 各プレイヤーが選択したマス
}

impl State {
    pub fn new(input: &Input) -> Self {
        let mut owner = vec![vec![-1; input.N]; input.N];
        let mut level = vec![vec![0; input.N]; input.N];
        let pos = input.xy.clone();
        let selected = pos.clone();

        // 初期位置を各プレイヤーの領土として設定（レベル1）
        for i in 0..input.M {
            owner[pos[i].0][pos[i].1] = i as i32;
            level[pos[i].0][pos[i].1] = 1;
        }

        State {
            pos,
            owner,
            level,
            selected,
        }
    }
}

pub struct Output {
    pub out: Vec<State>, // 各ターンの状態
}

pub fn parse_output(input: &Input, f: &str) -> Result<Output, String> {
    let (out, err) = parse_output_partial(input, f);
    if err.is_empty() {
        Ok(out)
    } else {
        Err(err)
    }
}

pub fn parse_output_partial(input: &Input, f: &str) -> (Output, String) {
    let mut state = State::new(input);
    let mut out = vec![state.clone()];

    for line in f.lines() {
        if out.len() > input.T {
            return (Output { out }, "Too many moves".to_owned());
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut tokens = line.split_whitespace();
        let x = match read(tokens.next(), 0..input.N) {
            Ok(v) => v,
            Err(e) => return (Output { out }, e),
        };
        let y = match read(tokens.next(), 0..input.N) {
            Ok(v) => v,
            Err(e) => return (Output { out }, e),
        };
        if tokens.next().is_some() {
            return (Output { out }, format!("Too many tokens in line: {}", line));
        }

        // プレイヤー0の行動を決定
        let player0_move = (x, y);

        // AI の行動を決定
        let mut moves = vec![player0_move];
        for i in 1..input.M {
            let ai_move = decide_ai_move(input, &state, i - 1, out.len() - 1);
            moves.push(ai_move);
        }

        // 状態を更新
        match update_state(input, &state, &moves) {
            Ok(new_state) => {
                state = new_state;
                out.push(state.clone());
            }
            Err(e) => return (Output { out }, e),
        }
    }

    // 途中経過でもビジュアライザで表示できるようにするため、
    // ターン数不足はエラーにしない（スコア計算時にチェック）
    (Output { out }, String::new())
}

// AI の行動を決定する関数
fn decide_ai_move(input: &Input, state: &State, ai_idx: usize, turn: usize) -> (usize, usize) {
    let player_id = ai_idx + 1;

    // 到達可能な領土を取得
    let candidates = get_candidates(input, state, player_id);

    // 各候補の評価値を計算
    let mut scores = Vec::new();
    for &(x, y) in &candidates {
        let owner = state.owner[x][y];
        let level = state.level[x][y];
        let value = input.V[x][y] as f64;

        let score = if owner == -1 {
            // 誰の土地でもない
            value * input.wa[ai_idx]
        } else if owner == player_id as i32 {
            // 自分の土地
            if level < input.U {
                value * input.wb[ai_idx]
            } else {
                0.0
            }
        } else {
            if level == 1 {
                // 他人の土地（レベル1）
                value * input.wc[ai_idx]
            } else {
                // 他人の土地（レベル2以上）
                value * input.wd[ai_idx]
            }
        };
        scores.push(score);
    }

    // ε-greedy で選択
    let eps = input.eps[ai_idx];
    let r1 = input.r[ai_idx][2 * (turn % input.T)]; // ε判定用
    let r2 = input.r[ai_idx][2 * (turn % input.T) + 1]; // 行動選択用

    if r1 < eps {
        // ランダム行動: 全候補から一様ランダムに選択
        let idx = (r2 * candidates.len() as f64).floor() as usize;
        let idx = idx.min(candidates.len() - 1);
        candidates[idx]
    } else {
        // greedy: 評価値最大の候補を選択（浮動小数点誤差を考慮したタイ判定）
        let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let tolerance = 1e-9 * max_score.abs().max(1.0);
        let best: Vec<usize> = (0..candidates.len())
            .filter(|&i| scores[i] >= max_score - tolerance)
            .collect();
        let idx = (r2 * best.len() as f64).floor() as usize;
        let idx = idx.min(best.len() - 1);
        candidates[best[idx]]
    }
}

// ゲーム状態を更新する関数
fn update_state(input: &Input, state: &State, moves: &[(usize, usize)]) -> Result<State, String> {
    let mut new_state = state.clone();

    // 選択されたマスを記録
    new_state.selected = moves.to_vec();

    // ステップ1: 移動の検証
    for i in 0..input.M {
        let target = moves[i];
        if !is_valid_move(input, state, i, target) {
            let err_msg = if target.0 >= input.N || target.1 >= input.N {
                format!(
                    "Player {} attempted to move to out-of-bounds position ({}, {})",
                    i, target.0, target.1
                )
            } else {
                format!(
                    "Player {} attempted invalid move from ({}, {}) to ({}, {}).",
                    i, state.pos[i].0, state.pos[i].1, target.0, target.1
                )
            };
            return Err(err_msg);
        }
    }

    // ステップ2: 駒を移動し、各マスの駒の数をカウント
    let mut temp_pos = moves.to_vec();
    let mut move_counts = std::collections::HashMap::new();
    for i in 0..input.M {
        *move_counts.entry(moves[i]).or_insert(0) += 1;
    }

    // ステップ3: 衝突チェック。2人以上いる場合、その領土の所有者がいれば所有者だけ残す
    let mut collected = vec![false; input.M];
    for i in 0..input.M {
        let target_pos = temp_pos[i];
        if move_counts[&target_pos] >= 2 {
            // その土地の所有者を確認
            let owner = new_state.owner[target_pos.0][target_pos.1];

            // 所有者以外を回収
            if i as i32 != owner {
                collected[i] = true;
            }
        }
    }

    // ステップ4: 回収されなかった駒について、行動処理
    for i in 0..input.M {
        if collected[i] {
            continue;
        }

        let (x, y) = temp_pos[i];
        let owner = new_state.owner[x][y];

        if owner == -1 {
            // 誰の領土でもない → 占領
            new_state.owner[x][y] = i as i32;
            new_state.level[x][y] = 1;
        } else if owner == i as i32 {
            // 自分の領土 → 強化
            if new_state.level[x][y] < input.U {
                new_state.level[x][y] += 1;
            }
        } else {
            // 他人の領土 → 攻撃
            new_state.level[x][y] -= 1;
            if new_state.level[x][y] == 0 {
                // レベルが0になったら即座に占領
                new_state.owner[x][y] = i as i32;
                new_state.level[x][y] = 1;
                // この場合、駒は回収しない
            } else {
                // レベルが0にならなかった場合は駒を回収
                collected[i] = true;
            }
        }
    }

    // ステップ5: 回収された駒を元の位置に戻す
    for i in 0..input.M {
        if collected[i] {
            temp_pos[i] = state.pos[i];
        }
    }

    // 新しい状態のposを更新（ターン終了時の位置）
    new_state.pos = temp_pos;

    Ok(new_state)
}

// 移動可能な土地を列挙
fn get_candidates(input: &Input, state: &State, player: usize) -> Vec<(usize, usize)> {
    let mut reachable = vec![];
    let mut visited = vec![vec![false; input.N]; input.N];
    let mut queue = std::collections::VecDeque::new();

    let start = state.pos[player];
    queue.push_back(start);
    visited[start.0][start.1] = true;

    while let Some((x, y)) = queue.pop_front() {
        let mut ok = true;
        for i in 0..input.M {
            if i != player && state.pos[i] == (x, y) {
                ok = false;
                break;
            }
        }
        if ok {
            reachable.push((x, y));
        }
        if state.owner[x][y] == player as i32 {
            // 隣接する4方向を探索
            let dirs = [(0, 1), (1, 0), (0, !0), (!0, 0)];
            for &(dx, dy) in &dirs {
                let nx = x.wrapping_add(dx);
                let ny = y.wrapping_add(dy);
                if nx < input.N && ny < input.N && !visited[nx][ny] {
                    visited[nx][ny] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }

    reachable
}

// 移動が有効かチェック
fn is_valid_move(input: &Input, state: &State, player: usize, target: (usize, usize)) -> bool {
    // グリッド範囲外チェック
    if target.0 >= input.N || target.1 >= input.N {
        return false;
    }
    // 他のプレイヤーの駒があるかチェック
    for i in 0..input.M {
        if i != player && state.pos[i] == target {
            return false;
        }
    }

    // 到達可能な領土を取得
    let candidates = get_candidates(input, state, player);

    // 到達可能な領土または隣接するマスなら移動可能
    for &(rx, ry) in &candidates {
        if (rx, ry) == target {
            return true;
        }
    }

    false
}

pub fn gen(seed: u64, M_opt: Option<usize>, U_opt: Option<usize>) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);

    let N = 10;
    let mut M = rng.gen_range(2i32..=8i32) as usize;
    let T = 100;
    let mut U = rng.gen_range(1i32..=5i32) as usize;

    let mut K = rng.gen_range(0i32..=2i32) as usize;
    let mut a = rng.gen_range(0.0f64..3.0f64);

    // seed 0 はシンプルなのがいいよね
    if seed == 0 {
        M = 4;
        U = 3;
        K = 0;
        a = 1.0;
    }

    if let Some(M_given) = M_opt {
        M = M_given;
    }
    if let Some(U_given) = U_opt {
        U = U_given;
    }

    let mut V = vec![vec![0.0f64; N]; N];
    for i in 0..N {
        for j in 0..N {
            V[i][j] = rng.gen_range(0.5f64..1.0f64).powf(a);
        }
    }

    for _ in 0..K {
        let x = rng.gen_range(0..N as i32) as usize;
        let y = rng.gen_range(0..N as i32) as usize;
        let b = rng.gen_range(1.0f64..4.0f64);
        let m = rng.gen_range(0i32..=4i32) as usize;
        let R = rng.gen_range(1.0f64..5.0f64);
        for i in 0..N {
            for j in 0..N {
                let dx = (i as i32 - x as i32) as f64;
                let dy = (j as i32 - y as i32) as f64;
                if m == 0 {
                    // ガウシアン
                    V[i][j] += b * (-(dx * dx + dy * dy) / (2.0 * R * R)).exp();
                } else if m == 1 {
                    // 逆距離（ユークリッド）
                    V[i][j] += b / (1.0 + (dx * dx + dy * dy).sqrt() / R);
                } else if m == 2 {
                    // 一様（ユークリッド）
                    if dx * dx + dy * dy <= R * R {
                        V[i][j] += b / 4.0;
                    }
                } else if m == 3 {
                    //逆距離（マンハッタン）
                    V[i][j] += b / (1.0 + (dx.abs() + dy.abs()) / R);
                } else {
                    // 一様（マンハッタン）
                    if dx.abs() + dy.abs() <= R {
                        V[i][j] += b / 4.0;
                    }
                }
            }
        }
    }

    // 総和を計算
    let total: f64 = V.iter().flat_map(|row| row.iter()).sum();
    let target_sum = 1000 * N * N;

    // 各マスをスケールして整数化
    let mut V_int = vec![vec![0usize; N]; N];
    for i in 0..N {
        for j in 0..N {
            V_int[i][j] = (V[i][j] * target_sum as f64 / total).ceil() as usize;
        }
    }

    // 総和の調整
    let current_sum: usize = V_int.iter().flat_map(|row| row.iter()).sum();
    let mut remaining = current_sum - target_sum;
    while remaining > 0 {
        let i = rng.gen_range(0..N as i32) as usize;
        let j = rng.gen_range(0..N as i32) as usize;
        if V_int[i][j] > 1 {
            V_int[i][j] -= 1;
            remaining -= 1;
        }
    }

    let V = V_int;

    // 各プレイヤーの初期位置を重複しないように生成
    let mut xy = vec![(0, 0); M];
    let mut used = std::collections::HashSet::new();
    for i in 0..M {
        loop {
            let x = rng.gen_range(0..N as i32) as usize;
            let y = rng.gen_range(0..N as i32) as usize;
            if !used.contains(&(x, y)) {
                xy[i] = (x, y);
                used.insert((x, y));
                break;
            }
        }
    }

    // AIパラメータを生成
    let mut wa = vec![0.0; M - 1];
    let mut wb = vec![0.0; M - 1];
    let mut wc = vec![0.0; M - 1];
    let mut wd = vec![0.0; M - 1];

    let mut eps = vec![0.0; M - 1];
    for i in 0..(M - 1) {
        wa[i] = rng.gen_range(0.3f64..1.0f64);
        wb[i] = rng.gen_range(0.3f64..1.0f64);
        wc[i] = rng.gen_range(0.3f64..1.0f64);
        wd[i] = rng.gen_range(0.3f64..1.0f64);
        eps[i] = rng.gen_range(0.1f64..0.5f64);
    }

    // 乱数r1_t,i, r2_t,iを生成（ε判定用と行動選択用）
    let mut r = vec![vec![0.0; 2 * T]; M - 1];
    for i in 0..(M - 1) {
        for j in 0..(2 * T) {
            r[i][j] = rng.gen_range(0.0f64..1.0f64);
        }
    }

    Input {
        N,
        M,
        T,
        U,
        V,
        xy,
        wa,
        wb,
        wc,
        wd,
        eps,
        r,
    }
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String) {
    let (mut score, err, _) = compute_score_details(input, &out.out);
    if !err.is_empty() {
        score = 0;
    }
    (score, err)
}

pub fn compute_score_details(input: &Input, out: &[State]) -> (i64, String, Vec<i64>) {
    if out.is_empty() {
        return (0, "empty output".to_owned(), vec![]);
    }
    if out.len() > input.T + 1 {
        return (0, "too many moves".to_owned(), vec![]);
    }

    // 最終ターンの状態を取得
    let final_state = &out[out.len() - 1];

    // 各プレイヤーのスコアを計算
    let mut scores = vec![0i64; input.M];
    for i in 0..input.N {
        for j in 0..input.N {
            let owner = final_state.owner[i][j];
            if owner >= 0 {
                let value = input.V[i][j] as i64;
                let level = final_state.level[i][j] as i64;
                scores[owner as usize] += value * level;
            }
        }
    }

    // プレイヤー0のスコアと最大のAIスコアからスコアを計算
    let player0_score = scores[0];
    let mut max_ai_score = 0i64;
    for i in 1..input.M {
        max_ai_score = max_ai_score.max(scores[i]);
    }

    let score = (1e5 * (1.0 + player0_score as f64 / max_ai_score as f64).log2()).round() as i64;

    (score, String::new(), scores)
}

/// 0 <= val <= 1
pub fn color(mut val: f64) -> String {
    val.setmin(1.0);
    val.setmax(0.0);
    let (r, g, b) = if val < 0.5 {
        let x = val * 2.0;
        (
            30. * (1.0 - x) + 144. * x,
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
        )
    } else {
        let x = val * 2.0 - 1.0;
        (
            144. * (1.0 - x) + 255. * x,
            255. * (1.0 - x) + 30. * x,
            30. * (1.0 - x) + 70. * x,
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        r.round() as i32,
        g.round() as i32,
        b.round() as i32
    )
}

pub fn rect(x: usize, y: usize, w: usize, h: usize, fill: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
}

pub fn group(title: String) -> Group {
    Group::new().add(Title::new().add(Text::new(title)))
}

pub fn vis_default(input: &Input, out: &Output) -> (i64, String, String) {
    let (mut score, err, svg) = vis(input, &out.out, out.out.len() - 1, 0);
    if !err.is_empty() {
        score = 0;
    }
    (score, err, svg)
}

pub fn vis(input: &Input, out: &[State], turn: usize, color_mode: usize) -> (i64, String, String) {
    let W = 600;
    let H = 600;

    if out.is_empty() {
        return (0, "empty output".to_owned(), String::new());
    }
    if out.len() > input.T + 1 {
        return (0, "too many moves".to_owned(), String::new());
    }

    let state = if turn < out.len() {
        &out[turn]
    } else {
        &out[out.len() - 1]
    };

    // 現在のターンの時点でのスコアを計算
    let mut player_scores = vec![0i64; input.M];
    for i in 0..input.N {
        for j in 0..input.N {
            let owner = state.owner[i][j];
            if owner >= 0 {
                let value = input.V[i][j] as i64;
                let level = state.level[i][j] as i64;
                player_scores[owner as usize] += value * level;
            }
        }
    }

    let player0_score = player_scores[0];
    let mut max_ai_score = 0i64;
    for i in 1..input.M {
        max_ai_score = max_ai_score.max(player_scores[i]);
    }

    // スコアを計算
    let s = (1e5 * (1.0 + player0_score as f64 / max_ai_score as f64).log2()).round() as i64;

    // 最終スコアはTターン完了時のみ計算、それ以外は暫定値
    let (score, err) = if out.len() > input.T {
        (s, String::new())
    } else {
        let e = format!("Incomplete: {}/{} turns", out.len() - 1, input.T);
        (s, e)
    };

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set("viewBox", (-5, -15, W + 210, H + 20))
        .set("width", W + 210)
        .set("height", H + 10)
        .set("style", "background-color:white");

    doc = doc.add(Style::new(
        "text {text-anchor: middle;dominant-baseline: central;}
         .grid-cell {cursor: pointer;}
         .grid-cell:hover rect {stroke: #ff0000; stroke-width: 2;}"
            .to_string(),
    ));

    let cell_size = W / input.N;

    // プレイヤーの色を定義
    let player_colors = [
        "#3498db", // 青 (プレイヤー0)
        "#e74c3c", // 赤
        "#2ecc71", // 緑
        "#f39c12", // オレンジ
        "#9b59b6", // 紫
        "#34495e", // 濃い灰色
        "#c0392b", // 濃い赤
        "#95a5a6", // 灰色
    ];

    // プレイヤー0の移動可能なマスを計算
    let mut valid_moves = std::collections::HashSet::new();
    if turn < out.len() {
        let candidates = get_candidates(input, state, 0);

        // 到達可能な領土の隣接マスを追加
        let dirs = [(0, 1), (1, 0), (0, !0), (!0, 0)];
        for &(rx, ry) in &candidates {
            for &(dx, dy) in &dirs {
                let nx = rx.wrapping_add(dx);
                let ny = ry.wrapping_add(dy);
                if nx < input.N && ny < input.N {
                    if is_valid_move(input, state, 0, (nx, ny)) {
                        valid_moves.insert((nx, ny));
                    }
                }
            }
        }
    }

    // グリッドの描画
    for i in 0..input.N {
        for j in 0..input.N {
            let x = j * cell_size;
            let y = i * cell_size;
            let owner = state.owner[i][j];
            let level = state.level[i][j];

            // 土地の色
            let (fill, fill_opacity) = if owner >= 0 {
                let base_color = player_colors[owner as usize % player_colors.len()];
                let alpha = if color_mode == 1 {
                    // V*L モード: alpha = 0.9 * V*L / (Vmax * U)
                    let vl = input.V[i][j] as f64 * level as f64;
                    let vmax = input
                        .V
                        .iter()
                        .flat_map(|row| row.iter())
                        .copied()
                        .max()
                        .unwrap_or(1) as f64;
                    let max_vl = vmax * input.U as f64;
                    0.6 * vl / max_vl + 0.3
                } else {
                    // L モード (デフォルト)
                    0.3 + (level as f64 / input.U as f64) * 0.6
                };
                (base_color.to_string(), alpha)
            } else {
                ("#ffffff".to_string(), 1.0)
            };

            let mut g = group(format!(
                "({}, {}) V={} Owner={} Level={}",
                i, j, input.V[i][j], owner, level
            ))
            .set("class", "grid-cell")
            .set("data-x", i)
            .set("data-y", j)
            .set(
                "data-valid-move",
                if valid_moves.contains(&(i, j)) {
                    "true"
                } else {
                    "false"
                },
            );

            g = g.add(
                rect(x, y, cell_size, cell_size, &fill)
                    .set("fill-opacity", fill_opacity)
                    .set("stroke", "black")
                    .set("stroke-width", 1),
            );

            // 移動可能マスのハイライト用（内側にオフセットした矩形）
            let inset = 3;
            g = g.add(
                rect(
                    x + inset,
                    y + inset,
                    cell_size - 2 * inset,
                    cell_size - 2 * inset,
                    "none",
                )
                .set("class", "highlight-rect")
                .set("stroke", "#ff0000")
                .set("stroke-width", 1)
                .set("visibility", "hidden"),
            );

            // 価値を表示
            g = g.add(
                svg::node::element::Text::new()
                    .set("x", x + cell_size / 4)
                    .set("y", y + cell_size / 6)
                    .set("font-size", cell_size / 5)
                    .set("fill", "black")
                    .add(svg::node::Text::new(format!("{}", input.V[i][j]))),
            );

            // レベルを表示
            if owner >= 0 {
                g = g.add(
                    svg::node::element::Text::new()
                        .set("x", x + cell_size * 3 / 4 - 1)
                        .set("y", y + cell_size / 6 - 1)
                        .set("font-size", cell_size / 4)
                        .set("fill", "white")
                        .set("font-weight", "bold")
                        .add(svg::node::Text::new(format!("Lv.{}", level))),
                );
            }

            doc = doc.add(g);
        }
    }

    // プレイヤーの駒を描画
    for p in 0..input.M {
        let (i, j) = state.pos[p];
        let x = (j * cell_size + cell_size / 2) as f64;
        let y = (i * cell_size + cell_size / 2) as f64;
        let radius = cell_size as f64 / 5.0;

        let color = player_colors[p % player_colors.len()];

        let mut g = group(format!("Player {}", p))
            .set("class", "grid-cell")
            .set("data-x", i)
            .set("data-y", j)
            .set(
                "data-valid-move",
                if valid_moves.contains(&(i, j)) {
                    "true"
                } else {
                    "false"
                },
            );
        g = g.add(
            Circle::new()
                .set("cx", x)
                .set("cy", y)
                .set("r", radius)
                .set("fill", color)
                .set("stroke", "white")
                .set("stroke-width", 2),
        );

        g = g.add(
            svg::node::element::Text::new()
                .set("x", x)
                .set("y", y)
                .set("font-size", cell_size / 5)
                .set("fill", "white")
                .set("font-weight", "bold")
                .add(svg::node::Text::new(format!("{}", p))),
        );

        doc = doc.add(g);
    }

    // 各プレイヤーが選択したマスを表示
    for p in 0..input.M {
        let (i, j) = state.selected[p];

        let x = (j * cell_size + cell_size * (p + 1) / 9) as f64;
        let y = (i * cell_size + cell_size * 5 / 6) as f64;

        let color = player_colors[p % player_colors.len()];
        let mut g = group(format!("Player {} selected ({}, {})", p, i, j))
            .set("class", "grid-cell")
            .set("data-x", i)
            .set("data-y", j)
            .set(
                "data-valid-move",
                if valid_moves.contains(&(i, j)) {
                    "true"
                } else {
                    "false"
                },
            );

        g = g.add(
            svg::node::element::Text::new()
                .set("x", x)
                .set("y", y)
                .set("font-size", cell_size / 5)
                .set("fill", color)
                .set("font-weight", "bold")
                .add(svg::node::Text::new(format!("{}", p))),
        );

        doc = doc.add(g);
    }

    // スコア表示
    for p in 0..input.M {
        if p < player_scores.len() {
            let score_text = format!("P{}:{}", p, player_scores[p]);
            let color = player_colors[p % player_colors.len()];
            doc = doc.add(
                svg::node::element::Text::new()
                    .set("x", W + 70)
                    .set("y", 12 + p * 36)
                    .set("font-size", 24)
                    .set("fill", color)
                    .add(svg::node::Text::new(score_text)),
            );
        }
    }

    (score, err, doc.to_string())
}

fn read_line(stdout: &mut BufReader<ChildStdout>, local: bool) -> Result<String, String> {
    loop {
        let mut out = String::new();
        match stdout.read_line(&mut out) {
            Ok(0) | Err(_) => {
                return Err("Your program has terminated unexpectedly".to_string());
            }
            _ => (),
        }
        if local {
            print!("{}", out);
        }
        let v = out.trim();
        if !v.is_empty() && !v.starts_with('#') {
            return Ok(v.to_owned());
        }
    }
}

pub fn exec(p: &mut std::process::Child, local: bool) -> Result<i64, String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let input = parse_input(&input);

    let mut stdin = std::io::BufWriter::new(p.stdin.take().unwrap());
    let mut stdout = std::io::BufReader::new(p.stdout.take().unwrap());

    // 初期入力を送信
    let _ = writeln!(stdin, "{} {} {} {}", input.N, input.M, input.T, input.U);
    for i in 0..input.N {
        for j in 0..input.N {
            if j > 0 {
                let _ = write!(stdin, " ");
            }
            let _ = write!(stdin, "{}", input.V[i][j]);
        }
        let _ = writeln!(stdin);
    }
    // 初期位置を送信
    for i in 0..input.M {
        let _ = writeln!(stdin, "{} {}", input.xy[i].0, input.xy[i].1);
    }
    let _ = stdin.flush();

    let mut state = State::new(&input);

    // ゲームループ
    for turn in 0..input.T {
        // プレイヤー0の出力を読み取る
        let line = read_line(&mut stdout, local)?;
        let mut tokens = line.split_whitespace();
        let x = read(tokens.next(), 0..input.N)?;
        let y = read(tokens.next(), 0..input.N)?;
        if tokens.next().is_some() {
            return Err(format!("Too many tokens in line: {}", line));
        }

        // 全プレイヤーの行動を決定
        let mut moves = vec![(x, y)];
        for i in 1..input.M {
            let ai_move = decide_ai_move(&input, &state, i - 1, turn);
            moves.push(ai_move);
        }

        // 状態を更新
        state = update_state(&input, &state, &moves)?;

        // 選択した移動先を送信
        for p in 0..input.M {
            let _ = writeln!(stdin, "{} {}", state.selected[p].0, state.selected[p].1);
        }
        // 現在の状態を送信
        for p in 0..input.M {
            let _ = writeln!(stdin, "{} {}", state.pos[p].0, state.pos[p].1);
        }
        for i in 0..input.N {
            for j in 0..input.N {
                if j > 0 {
                    let _ = write!(stdin, " ");
                }
                let _ = write!(stdin, "{}", state.owner[i][j]);
            }
            let _ = writeln!(stdin);
        }
        for i in 0..input.N {
            for j in 0..input.N {
                if j > 0 {
                    let _ = write!(stdin, " ");
                }
                let _ = write!(stdin, "{}", state.level[i][j]);
            }
            let _ = writeln!(stdin);
        }
        let _ = stdin.flush();
    }

    // 最終スコアを計算
    let mut scores = vec![0i64; input.M];
    for i in 0..input.N {
        for j in 0..input.N {
            let owner = state.owner[i][j];
            if owner >= 0 {
                let value = input.V[i][j] as i64;
                let level = state.level[i][j] as i64;
                scores[owner as usize] += value * level;
            }
        }
    }

    let player0_score = scores[0];
    let mut max_ai_score = 0i64;
    for i in 1..input.M {
        max_ai_score = max_ai_score.max(scores[i]);
    }

    let score = (1e5 * (1.0 + player0_score as f64 / max_ai_score as f64).log2()).round() as i64;

    p.wait().unwrap();
    Ok(score)
}
