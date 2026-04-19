// v000_template.rs
use std::io::{self, Read};
use std::str::{FromStr, SplitAsciiWhitespace};
use std::time::Instant;

pub const N: usize = 32;
pub type Color = u8;
pub type Grid = [[Color; N]; N];
pub type Coord = (usize, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub k_layers: usize,
    pub color_count: usize,
    pub goal: Grid,
}

impl Input {
    pub fn read() -> Self {
        let mut src = String::new();
        io::stdin()
            .read_to_string(&mut src)
            .expect("failed to read stdin");
        Self::from_str(&src)
    }

    pub fn from_str(src: &str) -> Self {
        let mut tokens = src.split_ascii_whitespace();

        let _: usize = read_value(&mut tokens);
        let k_layers = read_value(&mut tokens);
        let color_count = read_value(&mut tokens);

        let mut goal = [[0; N]; N];
        for row in &mut goal {
            for cell in row {
                *cell = read_value(&mut tokens);
            }
        }

        Self {
            k_layers,
            color_count,
            goal,
        }
    }

    pub fn nonzero_goal_count(&self) -> usize {
        self.goal
            .iter()
            .flatten()
            .filter(|&&color| color != 0)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub layers: Vec<Grid>,
    pub op_count: usize,
    pub layer0_mismatch_count: usize,
    pub goal_nonzero_count: usize,
}

impl State {
    pub fn new(input: &Input) -> Self {
        let goal_nonzero_count = input.nonzero_goal_count();
        Self {
            layers: vec![[[0; N]; N]; input.k_layers],
            op_count: 0,
            layer0_mismatch_count: goal_nonzero_count,
            goal_nonzero_count,
        }
    }

    #[inline(always)]
    pub fn is_goal(&self) -> bool {
        self.layer0_mismatch_count == 0
    }

    #[inline]
    pub fn official_score(&self) -> i64 {
        debug_assert!(self.op_count > 0);
        let ratio = self.goal_nonzero_count as f64 / self.op_count as f64;
        (1_000_000.0 * (1.0 + ratio.log2())).round() as i64
    }
}

#[inline(always)]
pub fn paint(state: &mut State, goal: &Grid, k: usize, i: usize, j: usize, color: Color) {
    debug_assert!(k < state.layers.len());
    debug_assert!(i < N);
    debug_assert!(j < N);

    if k == 0 {
        replace_layer0_cell(
            &mut state.layers[0],
            goal,
            &mut state.layer0_mismatch_count,
            i,
            j,
            color,
        );
    } else {
        state.layers[k][i][j] = color;
    }
    state.op_count += 1;
}

#[inline]
pub fn copy(state: &mut State, goal: &Grid, k: usize, h: usize, rot: usize, di: isize, dj: isize) {
    debug_assert!(k < state.layers.len());
    debug_assert!(h < state.layers.len());

    let src = state.layers[h];
    if k == 0 {
        let mut dst = state.layers[0];
        let mut mismatch_count = state.layer0_mismatch_count;
        for (i, row) in src.iter().enumerate() {
            for (j, &color) in row.iter().enumerate() {
                if color == 0 {
                    continue;
                }
                let (ri, rj) = rotate_coord((i, j), rot);
                let ni = ri as isize + di;
                let nj = rj as isize + dj;
                debug_assert!((0..N as isize).contains(&ni));
                debug_assert!((0..N as isize).contains(&nj));
                replace_layer0_cell(
                    &mut dst,
                    goal,
                    &mut mismatch_count,
                    ni as usize,
                    nj as usize,
                    color,
                );
            }
        }
        state.layers[0] = dst;
        state.layer0_mismatch_count = mismatch_count;
    } else {
        let dst = &mut state.layers[k];
        for (i, row) in src.iter().enumerate() {
            for (j, &color) in row.iter().enumerate() {
                if color == 0 {
                    continue;
                }
                let (ri, rj) = rotate_coord((i, j), rot);
                let ni = ri as isize + di;
                let nj = rj as isize + dj;
                debug_assert!((0..N as isize).contains(&ni));
                debug_assert!((0..N as isize).contains(&nj));
                dst[ni as usize][nj as usize] = color;
            }
        }
    }
    state.op_count += 1;
}

#[inline(always)]
pub fn clear(state: &mut State, k: usize) {
    debug_assert!(k < state.layers.len());

    state.layers[k] = [[0; N]; N];
    if k == 0 {
        state.layer0_mismatch_count = state.goal_nonzero_count;
    }
    state.op_count += 1;
}

#[inline(always)]
fn replace_layer0_cell(
    layer0: &mut Grid,
    goal: &Grid,
    mismatch_count: &mut usize,
    i: usize,
    j: usize,
    color: Color,
) {
    let goal_color = goal[i][j];
    *mismatch_count -= usize::from(layer0[i][j] != goal_color);
    *mismatch_count += usize::from(color != goal_color);
    layer0[i][j] = color;
}

#[inline(always)]
fn rotate_coord((i, j): Coord, rot: usize) -> Coord {
    match rot & 3 {
        0 => (i, j),
        1 => (j, N - 1 - i),
        2 => (N - 1 - i, N - 1 - j),
        3 => (N - 1 - j, i),
        _ => unreachable!(),
    }
}

#[inline(always)]
fn read_value<T>(tokens: &mut SplitAsciiWhitespace<'_>) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    tokens.next().unwrap().parse().unwrap()
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

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> String {
        let mut src = format!("{N} 3 4\n");
        for i in 0..N {
            for j in 0..N {
                let color = if i == 0 && j < 4 { (j + 1) as Color } else { 0 };
                src.push_str(&format!("{color} "));
            }
            src.push('\n');
        }
        src
    }

    #[test]
    fn from_str_parses_valid_input() {
        let input = Input::from_str(&valid_input());
        assert_eq!(input.k_layers, 3);
        assert_eq!(input.color_count, 4);
        assert_eq!(input.goal[0][0], 1);
        assert_eq!(input.goal[0][3], 4);
        assert_eq!(input.goal[1][0], 0);
        assert_eq!(input.nonzero_goal_count(), 4);
    }

    #[test]
    fn state_new_initializes_empty_layers() {
        let input = Input::from_str(&valid_input());
        let state = State::new(&input);
        assert_eq!(state.layers.len(), 3);
        assert_eq!(state.op_count, 0);
        assert_eq!(state.goal_nonzero_count, 4);
        assert_eq!(state.layer0_mismatch_count, 4);
        assert!(!state.is_goal());
        assert_eq!(state.layers[0], [[0; N]; N]);
    }

    #[test]
    fn paint_updates_layer0_mismatch_count() {
        let input = Input::from_str(&valid_input());
        let mut state = State::new(&input);
        paint(&mut state, &input.goal, 0, 0, 0, 1);
        assert_eq!(state.op_count, 1);
        assert_eq!(state.layer0_mismatch_count, 3);
        assert_eq!(state.layers[0][0][0], 1);
    }

    #[test]
    fn paint_on_nonzero_layer_does_not_touch_layer0_score_state() {
        let input = Input::from_str(&valid_input());
        let mut state = State::new(&input);
        paint(&mut state, &input.goal, 1, 0, 0, 1);
        assert_eq!(state.op_count, 1);
        assert_eq!(state.layer0_mismatch_count, 4);
        assert_eq!(state.layers[1][0][0], 1);
    }

    #[test]
    fn copy_updates_layer0_mismatch_count() {
        let input = Input::from_str(&valid_input());
        let mut state = State::new(&input);
        state.layers[1][0][0] = 1;
        state.layers[1][0][1] = 2;
        copy(&mut state, &input.goal, 0, 1, 0, 0, 0);
        assert_eq!(state.op_count, 1);
        assert_eq!(state.layer0_mismatch_count, 2);
        assert_eq!(state.layers[0][0][0], 1);
        assert_eq!(state.layers[0][0][1], 2);
    }

    #[test]
    fn clear_layer0_resets_mismatch_count() {
        let input = Input::from_str(&valid_input());
        let mut state = State::new(&input);
        state.layers[0] = input.goal;
        state.layer0_mismatch_count = 0;
        clear(&mut state, 0);
        assert_eq!(state.op_count, 1);
        assert_eq!(state.layer0_mismatch_count, 4);
        assert_eq!(state.layers[0], [[0; N]; N]);
    }

    #[test]
    fn state_official_score_uses_goal_nonzero_count_and_op_count() {
        let input = Input::from_str(&valid_input());
        let mut state = State::new(&input);
        state.layers[0] = input.goal;
        state.layer0_mismatch_count = 0;
        state.op_count = 4;
        assert!(state.is_goal());
        assert_eq!(state.official_score(), 1_000_000);
    }

    #[test]
    fn rotate_coord_matches_problem_definition() {
        assert_eq!(rotate_coord((1, 2), 0), (1, 2));
        assert_eq!(rotate_coord((1, 2), 1), (2, N - 1 - 1));
        assert_eq!(rotate_coord((1, 2), 2), (N - 1 - 1, N - 1 - 2));
        assert_eq!(rotate_coord((1, 2), 3), (N - 1 - 2, 1));
    }
}
