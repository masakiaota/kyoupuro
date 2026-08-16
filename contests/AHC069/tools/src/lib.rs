#![allow(non_snake_case, unused_macros)]

use proconio::input;
use rand::prelude::*;
use rand_distr::{Distribution, Exp, Normal};
use std::collections::{HashMap, HashSet};
use std::io::prelude::*;
use std::io::BufReader;
use std::ops::RangeBounds;
use svg::node::{
    element::{Group, Path, Rectangle, Style, Title},
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

/// Park side length
pub const N: usize = 50;
/// Number of groups
pub const M: usize = 1000;

/// One group's info
#[derive(Clone, Copy, Debug)]
pub struct GroupInfo {
    /// Group index i (arrival order)
    pub i: usize,
    /// Arrival time S_i
    pub s: i64,
    /// Departure time T_i
    pub t: i64,
    /// People P_i
    pub p: usize,
    /// Base payment V_i
    pub v: i64,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub n: usize,
    pub m: usize,
    /// Move cost coefficient R
    pub r: f64,
    /// true = grass, false = pond
    pub grass: Vec<Vec<bool>>,
    /// Groups in arrival order
    pub groups: Vec<GroupInfo>,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {:.3}", self.n, self.m, self.r)?;
        for row in &self.grass {
            let line: String = row.iter().map(|&g| if g { '.' } else { '#' }).collect();
            writeln!(f, "{}", line)?;
        }
        for g in &self.groups {
            writeln!(f, "{} {} {} {} {}", g.i, g.s, g.t, g.p, g.v)?;
        }
        Ok(())
    }
}

pub fn parse_input(f: &str) -> Input {
    let f = proconio::source::once::OnceSource::from(f);
    input! {
        from f,
        n: usize,
        m: usize,
        r: f64,
        rows: [String; n],
        gs: [(usize, i64, i64, usize, i64); m],
    }
    let grass = rows
        .iter()
        .map(|row| row.chars().map(|c| c == '.').collect())
        .collect();
    let groups = gs
        .into_iter()
        .map(|(i, s, t, p, v)| GroupInfo { i, s, t, p, v })
        .collect();
    Input {
        n,
        m,
        r,
        grass,
        groups,
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

/// One group in use at one frame, as shown in the visualizer's panel.
#[derive(Clone, Copy, Debug)]
pub struct ActiveInfo {
    /// Group index
    pub id: usize,
    /// People P (= number of occupied cells)
    pub p: usize,
    /// Base payment V
    pub v: i64,
    /// Departure time T
    pub t: i64,
    /// Boundary length L of the region right now
    pub l: i64,
    /// Largest L so far, which is the one the fee is computed from
    pub max_l: i64,
    /// Fee the group would pay if it departed now
    pub fee: i64,
}

impl ActiveInfo {
    /// Compactness of the current region
    pub fn c_now(&self) -> f64 {
        4.0 * (self.p as f64).sqrt() / self.l as f64
    }
    /// Smallest compactness so far, the one the fee follows
    pub fn c_min(&self) -> f64 {
        4.0 * (self.p as f64).sqrt() / self.max_l as f64
    }
}

/// One-turn snapshot for the visualizer.
pub struct Frame {
    /// grid[x][y] = index of the group occupying the cell (!0 if empty)
    pub grid: Vec<Vec<usize>>,
    /// Money at this frame
    pub money: i64,
    /// Current time (the arrival time S of the group of this turn)
    pub now: i64,
    /// The group that arrived on this turn and whether it was taken in.
    /// None on the first frame and on the frame after the last departure.
    pub arrival: Option<(usize, bool)>,
    /// Groups moved on this turn, in ascending index order
    pub moved: Vec<usize>,
    /// Groups that departed just before this turn's arrival, as (group index, fee paid),
    /// sorted by index
    pub departed: Vec<(usize, i64)>,
    /// Groups in use, sorted by index
    pub actives: Vec<ActiveInfo>,
    /// Number of occupied cells
    pub cells_used: usize,
    /// Cumulative number of groups taken in and turned away
    pub accepted: usize,
    pub rejected: usize,
    /// Cumulative fees earned and move costs paid
    pub total_fee: i64,
    pub total_move_cost: i64,
}

impl Frame {
    /// The group whose region should be highlighted as newly placed, if any.
    pub fn accepted_arrival(&self) -> Option<usize> {
        self.arrival.filter(|&(_, ok)| ok).map(|(i, _)| i)
    }
}

pub struct Output {
    /// Per-turn snapshots (first is the initial state, last is after all departures)
    pub frames: Vec<Frame>,
    /// Comment lines (`#` lines) emitted by the solution for each frame; same length as frames.
    pub comments: Vec<String>,
    /// Final score max(money, 0), or 0 when `error` is set, since an output the rules reject
    /// scores nothing.
    pub score: i64,
    /// Why the replay stopped short of the last turn, if it did. The frames up to that point
    /// were replayed under the same rules as the judge and are still worth drawing.
    pub error: Option<String>,
}

/// The state before the first group arrives: an empty park and no money spent.
fn initial_frame(state: &State) -> Frame {
    Frame {
        grid: state.occupied.clone(),
        money: state.money,
        now: 0,
        arrival: None,
        moved: vec![],
        departed: vec![],
        actives: vec![],
        cells_used: 0,
        accepted: 0,
        rejected: 0,
        total_fee: 0,
        total_move_cost: 0,
    }
}

/// Replays the solution's output log from the start and records the state at each turn.
/// Uses the same `apply_turn` as the judge (`exec`), so the rules match exactly.
///
/// A log the rules reject, or one cut short because the solution crashed or ran out of time,
/// still yields every turn before that point. Those are the turns worth looking at when working
/// out what went wrong, so the error is carried in the `Output` rather than replacing it.
pub fn parse_output(input: &Input, f: &str) -> Output {
    let mut reader = Reader::new(BufReader::new(f.as_bytes()), false, true);
    let mut state = State::new(input);
    let mut frames = vec![initial_frame(&state)];
    let mut comments = vec![String::new()];
    let mut accepted = 0;
    let mut rejected = 0;
    let mut error = None;
    for i in 0..input.m {
        let g = input.groups[i];
        // The fee only lands in the money inside `depart`, so note each group's share before the
        // departures happen.
        let departed = state.fees_of(state.leaving_before(g.s));
        state.depart_before(g.s);
        // A failed `apply_turn` can leave the state half-updated (`apply_moves` clears the moved
        // regions before it validates them), so stop without recording a frame for this turn.
        // The last frame kept is the one from the previous turn, which is consistent.
        let log = match apply_turn(&mut reader, &mut state, input, i) {
            Ok(log) => log,
            Err(err) => {
                // Name the turn: on its own, a message like "Allocated region is not connected"
                // gives no clue where in the log to look.
                error = Some(format!("turn {} (group {}): {}", i + 1, i, err));
                break;
            }
        };
        if log.arrived.is_some() {
            accepted += 1;
        } else {
            rejected += 1;
        }
        let actives = state.active_infos();
        frames.push(Frame {
            grid: state.occupied.clone(),
            money: state.money,
            now: g.s,
            arrival: Some((i, log.arrived.is_some())),
            moved: log.moved,
            departed,
            cells_used: actives.iter().map(|a| a.p).sum(),
            actives,
            accepted,
            rejected,
            total_fee: state.total_fee,
            total_move_cost: state.total_move_cost,
        });
        comments.push(reader.take_comment());
    }
    // Only a log that ran to the last turn ends with every group departing, so the final frame
    // and the score belong to that case alone.
    let score = if error.is_some() {
        0
    } else {
        let departed = state.fees_of(state.active.keys().copied().collect());
        state.depart_all();
        frames.push(Frame {
            grid: state.occupied.clone(),
            money: state.money,
            now: input.groups.iter().map(|g| g.t).max().unwrap_or(0),
            arrival: None,
            moved: vec![],
            departed,
            actives: vec![],
            cells_used: 0,
            accepted,
            rejected,
            total_fee: state.total_fee,
            total_move_cost: state.total_move_cost,
        });
        comments.push(reader.take_comment());
        state.money.max(0)
    };
    Output {
        frames,
        comments,
        score,
        error,
    }
}

pub fn gen(seed: u64) -> Input {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);

    // R = rand(1, 100) * 0.001
    let r = rng.gen_range(1..=100) as f64 * 0.001;

    // Park layout: start with all grass
    let mut grass = vec![vec![true; N]; N];

    // num_cluster = round(2^randf(1,8)) random cells -> ponds
    let num_cluster = 2f64.powf(rng.gen_range(1.0..8.0)).round() as usize;
    let mut cells: Vec<(usize, usize)> = (0..N).flat_map(|i| (0..N).map(move |j| (i, j))).collect();
    cells.shuffle(&mut rng);
    for &(i, j) in cells.iter().take(num_cluster) {
        grass[i][j] = false;
    }

    // num_pond = rand(0, 900-num_cluster) grass cells adjacent to ponds -> ponds
    // Cast bounds through i32 so results match the wasm (32-bit) visualizer
    let num_pond = rng.gen_range(0..=(900 - num_cluster) as i32) as usize;
    let dxy: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    // frontier: grass cells adjacent to a pond
    let mut in_frontier = vec![vec![false; N]; N];
    let mut frontier: Vec<(usize, usize)> = vec![];
    let is_pond_neighbor = |grass: &Vec<Vec<bool>>, i: usize, j: usize| -> bool {
        dxy.iter().any(|&(dx, dy)| {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32 && !grass[ni as usize][nj as usize]
        })
    };
    for i in 0..N {
        for j in 0..N {
            if grass[i][j] && is_pond_neighbor(&grass, i, j) {
                in_frontier[i][j] = true;
                frontier.push((i, j));
            }
        }
    }
    for _ in 0..num_pond {
        if frontier.is_empty() {
            break;
        }
        let idx = rng.gen_range(0..frontier.len() as i32) as usize;
        let (i, j) = frontier.swap_remove(idx);
        in_frontier[i][j] = false;
        grass[i][j] = false;
        // add newly pond-adjacent grass cells to the frontier
        for &(dx, dy) in &dxy {
            let (ni, nj) = (i as i32 + dx, j as i32 + dy);
            if ni >= 0 && ni < N as i32 && nj >= 0 && nj < N as i32 {
                let (ni, nj) = (ni as usize, nj as usize);
                if grass[ni][nj] && !in_frontier[ni][nj] {
                    in_frontier[ni][nj] = true;
                    frontier.push((ni, nj));
                }
            }
        }
    }

    // --- Generate groups ---
    // S_i, T_i
    let theta = rng.gen_range(2000..=8000) as f64;
    let exp = Exp::new(1.0 / theta).unwrap();
    let mut used: HashSet<i64> = HashSet::new();
    let mut st: Vec<(i64, i64)> = Vec::with_capacity(M);
    for _ in 0..M {
        loop {
            // l_i = round(Exp(1/theta)); redo if l_i >= 100000
            let l = exp.sample(&mut rng).round() as i64;
            if l >= 100000 {
                continue;
            }
            // s_i = rand(0, 100000-1-l_i), t_i = s_i + l_i + 1 (so t_i <= 100000)
            let s = rng.gen_range(0..=((100000 - 1 - l) as i32)) as i64;
            let t = s + l + 1;
            // redo if s/t already used
            if used.contains(&s) || used.contains(&t) {
                continue;
            }
            used.insert(s);
            used.insert(t);
            st.push((s, t));
            break;
        }
    }
    // sort by s ascending (all s distinct)
    st.sort();

    // P_i, V_i
    let sqrt150 = 150f64.sqrt();
    let normal = Normal::new(0.0, 0.8).unwrap();
    let mut groups = Vec::with_capacity(M);
    for (i, &(s, t)) in st.iter().enumerate() {
        // P_i = round(randf(2.0, sqrt(150.0))^2)
        let p = rng.gen_range(2.0..sqrt150).powi(2).round() as usize;
        // V_i = round(P_i * (T_i - S_i)^0.9 * 2^gauss(0.0, 0.8))
        let mut v = ((p as f64) * ((t - s) as f64).powf(0.9) * 2f64.powf(normal.sample(&mut rng)))
            .round() as i64;
        if v == 0 {
            v = 1;
        }
        if v > 100_000_000 {
            v = 100_000_000;
        }
        groups.push(GroupInfo { i, s, t, p, v });
    }

    Input {
        n: N,
        m: M,
        r,
        grass,
        groups,
    }
}

pub fn compute_score(_input: &Input, out: &Output) -> (i64, String) {
    (out.score, out.error.clone().unwrap_or_default())
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

/// Picks an easily distinguishable color from a group index. Rotating the hue by the
/// golden angle keeps colors of neighboring indices from clashing.
fn group_color(id: usize) -> String {
    let h = (id as f64 * 137.508) % 360.0;
    hsl_to_rgb(h, GROUP_S, GROUP_L)
}

/// Saturation and lightness of every region fill. The web visualizer's index.html repeats these,
/// and the hues below, so that the swatches in its group list match the board.
const GROUP_S: f64 = 0.36;
const GROUP_L: f64 = 0.66;

/// Hues the `ColorMode::Time` and `ColorMode::Compact` scale runs through. Swept in one
/// direction, so no hue but these is crossed twice.
const VALUE_HUES: [f64; 3] = [210.0, 90.0, -11.0];

/// Divergent scale from blue through green to red for a `val` in [0, 1], out of range values
/// taking the nearest end. Held at the saturation and lightness of the group colors, so the
/// board stays equally pale whichever color mode paints it.
fn value_color(mut val: f64) -> String {
    val.setmin(1.0);
    val.setmax(0.0);
    let [lo, mid, hi] = VALUE_HUES;
    let h = if val < 0.5 {
        lo + (mid - lo) * val * 2.0
    } else {
        mid + (hi - mid) * (val * 2.0 - 1.0)
    };
    hsl_to_rgb(h.rem_euclid(360.0), GROUP_S, GROUP_L)
}

/// A `#rrggbb` color scaled towards black by `f` (0 = black, 1 = unchanged).
fn darken(hex: &str, f: f64) -> String {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    let ch = |shift: u32| (((v >> shift) & 0xff) as f64 * f).round() as u32;
    format!("#{:02x}{:02x}{:02x}", ch(16), ch(8), ch(0))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f64| ((v + m) * 255.0).round() as i32;
    format!("#{:02x}{:02x}{:02x}", to(r), to(g), to(b))
}

/// How the occupied regions are colored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorMode {
    /// A distinct hue per group index
    Group,
    /// Time left until departure: blue = stays long, red = leaves soon
    Time,
    /// Compactness C: blue = close to a square, red = sprawling
    Compact,
}

impl ColorMode {
    /// Parses `group` / `t` / `c` (case-insensitive); an empty string means `Group`.
    pub fn parse(s: &str) -> Option<ColorMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "g" | "group" | "id" => Some(ColorMode::Group),
            "t" | "time" => Some(ColorMode::Time),
            "c" | "compact" | "compactness" => Some(ColorMode::Compact),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ColorMode::Group => "group id",
            ColorMode::Time => "T - now (time left)",
            ColorMode::Compact => "C = 4*sqrt(P)/L",
        }
    }
}

/// Drawing options for `vis`.
#[derive(Clone, Copy, Debug)]
pub struct VisOpt {
    /// How the occupied regions are colored
    pub color: ColorMode,
    /// Draw the status panel to the right of the board. Turn it off where the page around the
    /// SVG already shows the same information.
    pub panel: bool,
}

impl Default for VisOpt {
    fn default() -> Self {
        VisOpt {
            color: ColorMode::Group,
            panel: true,
        }
    }
}

/// The replay's error as the empty string when there is none, which is what the callers hand
/// back to the page and to the command line.
fn err_of(out: &Output) -> String {
    out.error.clone().unwrap_or_default()
}

pub fn vis_default(input: &Input, out: &Output, opt: VisOpt) -> (i64, String, String) {
    // A complete replay ends with an empty park (every group departed), so step back one to land
    // on the final arrival turn, the most crowded state. A replay that stopped early has no such
    // frame, and its last one is already the turn worth showing.
    let back = if out.error.is_some() { 1 } else { 2 };
    let t = out.frames.len().saturating_sub(back);
    vis(input, out, t, opt)
}

/// Left-aligned SVG text node at (x, y).
fn text(x: i32, y: i32, size: i32, fill: &str, s: impl Into<String>) -> svg::node::element::Text {
    svg::node::element::Text::new()
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("fill", fill)
        .add(Text::new(s.into()))
}

/// Small color chip vertically centered on the text baseline at y.
fn swatch(x: i32, y: i32, fill: &str) -> Rectangle {
    rect(x as usize, (y - 6) as usize, 12, 12, fill)
        .set("stroke", "#666666")
        .set("stroke-width", 1)
}

/// `"3, 7, 12, +5 more"`: group indices, cut off after `max` of them.
fn ids_str(ids: &[usize], max: usize) -> String {
    let head: Vec<String> = ids.iter().take(max).map(|i| i.to_string()).collect();
    let mut s = head.join(", ");
    if ids.len() > max {
        s += &format!(", +{} more", ids.len() - max);
    }
    s
}

/// Compactness mapped to the hot end of the gradient in `ColorMode::Compact`. A region has to
/// sprawl badly to fall this low, so the scale would waste its cold half on a smaller bound.
/// `value_color` clamps, so anything below simply takes the hottest color.
pub const C_MIN: f64 = 0.35;

/// Accent for the group that arrived on the drawn turn
const ARRIVED_COLOR: &str = "#ff0033";
/// Accent for the groups that moved on the drawn turn
const MOVED_COLOR: &str = "#101010";
/// Accent for the group picked from the group list beside the board
const SELECTED_COLOR: &str = "#ff8c00";
/// Outline width of the arrived and picked regions, the two worth spotting at a glance
const FOCUS_WIDTH: f64 = 3.2;
/// Outline width of the moved regions, of which a turn can have many
const MOVED_WIDTH: f64 = 2.0;
/// How much wider the white halo is than the accent line it sits under, so half of this shows
/// on each side of the line
const HALO_EXTRA: f64 = 2.6;

/// Percentile of the stays that the `ColorMode::Time` scale spans. A percentile and not the longest
/// stay, which is an extreme value that swings far more widely between inputs than the stays
/// themselves do, so the same time left would land on a different color from input to input.
const TIME_PERCENTILE: usize = 90;

/// Time left that paints a group with the coldest color, i.e. the far end of the
/// `ColorMode::Time` scale. Derived from the input rather than from the groups currently on the
/// board, so a group keeps the same color as the board fills and empties around it. `color` clamps,
/// so the longer stays above the end simply take the coldest color.
pub fn time_scale_end(input: &Input) -> i64 {
    let mut stays: Vec<i64> = input.groups.iter().map(|g| g.t - g.s).collect();
    if stays.is_empty() {
        return 1;
    }
    stays.sort_unstable();
    stays[stays.len() * TIME_PERCENTILE / 100].max(1)
}

/// Draws the park state at frame t as SVG.
/// Ponds are dark, empty grass is white, and occupied regions are colored per `opt.color`.
pub fn vis(input: &Input, output: &Output, t: usize, opt: VisOpt) -> (i64, String, String) {
    let n = input.n;
    let d = 600 / n; // cell size in pixels
    let bw = d * n; // board width
    let h_svg = 600usize;
    // Total canvas width: the board alone, or the board plus the status panel
    let w_svg = if opt.panel { 920 } else { bw };
    let t = t.min(output.frames.len() - 1);
    let frame = &output.frames[t];
    let mode = opt.color;

    // Cells of each group present at this frame; the rest of its numbers are in `frame.actives`.
    let mut cells_of: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    for x in 0..n {
        for y in 0..n {
            let gid = frame.grid[x][y];
            if gid != !0 {
                cells_of.entry(gid).or_default().push((x, y));
            }
        }
    }
    let info_of: HashMap<usize, &ActiveInfo> = frame.actives.iter().map(|a| (a.id, a)).collect();
    let gids: Vec<usize> = frame.actives.iter().map(|a| a.id).collect();

    let cold_at = time_scale_end(input);
    let fill_of = |gid: usize| -> String {
        let a = info_of[&gid];
        match mode {
            ColorMode::Group => group_color(gid),
            ColorMode::Time => value_color(1.0 - (a.t - frame.now).max(0) as f64 / cold_at as f64),
            // C stays above C_MIN in practice, so [C_MIN, 1] is stretched over the gradient.
            ColorMode::Compact => value_color((1.0 - a.c_now()) / (1.0 - C_MIN)),
        }
    };

    let mut doc = svg::Document::new()
        .set("id", "vis")
        .set(
            "viewBox",
            (-5i32, -5i32, (w_svg + 10) as i32, (h_svg + 10) as i32),
        )
        .set("width", w_svg + 10)
        .set("height", h_svg + 10)
        .set("style", "background-color:white");
    // To link the board with the group list beside it, the page appends a copy of a region's
    // outline carrying `.sel`. These rules turn that copy into an accent exactly like the
    // arrived / moved ones below: the halo is revealed and the line takes the accent color and
    // width. Presentation attributes lose to these rules, so the copy overrides whatever colors
    // and widths the region it came from carries as attributes.
    doc = doc.add(Style::new(format!(
        "text{{dominant-baseline:central;font-family:sans-serif}}\
         .c:hover rect{{stroke:red;stroke-width:2}}\
         .r.sel .h{{stroke-opacity:1;stroke-width:{:.1}}}\
         .r.sel .l{{stroke:{};stroke-width:{:.1}}}",
        FOCUS_WIDTH + HALO_EXTRA,
        SELECTED_COLOR,
        FOCUS_WIDTH
    )));

    // ---- Board ----
    let mut board = Group::new();
    board = board.add(rect(0, 0, bw, bw, "#ffffff"));
    for x in 0..n {
        for y in 0..n {
            let (sx, sy) = (y * d, x * d);
            let gid = frame.grid[x][y];
            if !input.grass[x][y] {
                board = board.add(
                    rect(sx, sy, d, d, "#4a6070")
                        .set("stroke", "#43596a")
                        .set("stroke-width", 1),
                );
            } else if gid == !0 {
                board = board.add(
                    rect(sx, sy, d, d, "#ffffff")
                        .set("stroke", "#e8e8e8")
                        .set("stroke-width", 1),
                );
            } else {
                // Faint white grid inside a region; the region outline below carries the shape.
                let cell = rect(sx, sy, d, d, &fill_of(gid))
                    .set("stroke", "#ffffff")
                    .set("stroke-opacity", 0.3)
                    .set("stroke-width", 1);
                let a = info_of[&gid];
                board = board.add(
                    group(format!(
                        "group {} (P={}, V={}, T={}, L={}, C(now)={:.3}, C(min)={:.3}, fee={})",
                        gid,
                        a.p,
                        a.v,
                        a.t,
                        a.l,
                        a.c_now(),
                        a.c_min(),
                        a.fee
                    ))
                    .set("class", "c")
                    .set("data-gid", gid)
                    .add(cell),
                );
            }
        }
    }

    // ---- Region outlines ----
    // A darkened shade of the fill separates touching groups without adding a heavy grid.
    // The group that arrived and the ones that moved on this turn get an accent ring on top
    // of a white halo, so they stand out over any fill color.
    let moved: HashSet<usize> = frame.moved.iter().copied().collect();
    let arrived = frame.accepted_arrival();
    // Neighboring regions share a boundary, so whichever is drawn last covers the other's outline.
    // Accented regions are held back and drawn over the plain ones to keep their color and halo
    // intact all the way around.
    let mut accented = vec![];
    for &gid in &gids {
        let cells = &cells_of[&gid];
        let set: HashSet<(usize, usize)> = cells.iter().copied().collect();
        let mut path = String::new();
        for &(x, y) in cells {
            let (sx, sy, dd) = ((y * d) as i32, (x * d) as i32, d as i32);
            let mut edge = |x1: i32, y1: i32, x2: i32, y2: i32| {
                path += &format!("M{} {}L{} {}", x1, y1, x2, y2);
            };
            let same = |dx: i32, dy: i32| {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                nx >= 0 && ny >= 0 && set.contains(&(nx as usize, ny as usize))
            };
            if !same(-1, 0) {
                edge(sx, sy, sx + dd, sy);
            }
            if !same(1, 0) {
                edge(sx, sy + dd, sx + dd, sy + dd);
            }
            if !same(0, -1) {
                edge(sx, sy, sx, sy + dd);
            }
            if !same(0, 1) {
                edge(sx + dd, sy, sx + dd, sy + dd);
            }
        }
        let outline = |class: &str, stroke: &str, width: f64, opacity: f64| {
            Path::new()
                .set("class", class)
                .set("d", path.clone())
                .set("fill", "none")
                .set("stroke", stroke)
                .set("stroke-width", format!("{:.1}", width))
                .set("stroke-opacity", opacity)
                .set("stroke-linecap", "square")
                .set("pointer-events", "none")
        };
        let accent = if arrived == Some(gid) {
            Some((ARRIVED_COLOR, FOCUS_WIDTH))
        } else if moved.contains(&gid) {
            Some((MOVED_COLOR, MOVED_WIDTH))
        } else {
            None
        };
        // The halo is drawn under the line and only shows for an accented or selected group.
        let (line, width, halo) = match accent {
            Some((a, w)) => (a.to_owned(), w, 1.0),
            None => (darken(&fill_of(gid), 0.5), 1.4, 0.0),
        };
        let region = Group::new()
            .set("class", "r")
            .set("data-gid", gid)
            .add(outline("h", "#ffffff", width + HALO_EXTRA, halo))
            .add(outline("l", &line, width, 1.0));
        if accent.is_some() {
            accented.push(region);
        } else {
            board = board.add(region);
        }
    }
    for region in accented {
        board = board.add(region);
    }

    // Board frame
    board = board.add(
        Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", bw)
            .set("height", bw)
            .set("fill", "none")
            .set("stroke", "#000000")
            .set("stroke-width", 2),
    );
    doc = doc.add(board);

    if !opt.panel {
        return (output.score, err_of(output), doc.to_string());
    }

    // ---- Right panel ----
    let rx = (bw + 20) as i32;
    let panel_w = w_svg as i32 - rx - 5;
    let x0 = rx + 10;
    let mut panel = Group::new();
    panel = panel.add(
        rect(rx as usize - 1, 0, panel_w as usize + 2, h_svg, "#fafafa")
            .set("stroke", "#bbbbbb")
            .set("stroke-width", 1),
    );

    // Status header
    let mut y = 24i32;
    panel = panel.add(text(
        x0,
        y,
        20,
        "#000000",
        format!("frame {} / {}", t, output.frames.len() - 1),
    ));
    y += 28;
    panel = panel.add(text(
        x0,
        y,
        16,
        "#000000",
        format!("score = {}", output.score),
    ));
    y += 22;
    panel = panel.add(text(
        x0,
        y,
        16,
        "#000000",
        format!("money = {}", frame.money),
    ));
    y += 22;
    panel = panel.add(text(x0, y, 13, "#666666", format!("now = {}", frame.now)));
    y += 30;

    // What happened on this turn
    if let Some((gid, ok)) = frame.arrival {
        let g = input.groups[gid];
        let (verdict, fill) = if ok {
            ("accepted", "#1a7f37")
        } else {
            ("rejected", "#b02020")
        };
        if ok {
            panel = panel.add(swatch(x0, y, ARRIVED_COLOR));
        }
        panel = panel.add(text(
            x0 + 18,
            y,
            14,
            fill,
            format!("group {} {}", gid, verdict),
        ));
        y += 18;
        panel = panel.add(text(
            x0 + 18,
            y,
            12,
            "#444444",
            format!("T={} P={} V={}", g.t, g.p, g.v),
        ));
        y += 24;
    }
    if !frame.moved.is_empty() {
        panel = panel.add(swatch(x0, y, MOVED_COLOR));
        panel = panel.add(text(
            x0 + 18,
            y,
            14,
            "#000000",
            format!("moved: {}", frame.moved.len()),
        ));
        y += 18;
        panel = panel.add(text(x0 + 18, y, 12, "#444444", ids_str(&frame.moved, 6)));
        y += 24;
    }
    if !frame.departed.is_empty() {
        let fee: i64 = frame.departed.iter().map(|&(_, f)| f).sum();
        let ids: Vec<usize> = frame.departed.iter().map(|&(j, _)| j).collect();
        panel = panel.add(text(
            x0,
            y,
            14,
            "#000000",
            format!("departed: {} (+{})", frame.departed.len(), fee),
        ));
        y += 18;
        panel = panel.add(text(x0 + 18, y, 12, "#444444", ids_str(&ids, 6)));
        y += 24;
    }

    // Occupancy and running totals
    let grass: usize = input.grass.iter().flatten().filter(|&&g| g).count();
    y += 6;
    panel = panel.add(text(
        x0,
        y,
        14,
        "#000000",
        format!("active groups: {}", gids.len()),
    ));
    y += 20;
    panel = panel.add(text(
        x0,
        y,
        13,
        "#666666",
        format!(
            "used {}/{} ({:.1}%)",
            frame.cells_used,
            grass,
            100.0 * frame.cells_used as f64 / grass as f64
        ),
    ));
    y += 20;
    panel = panel.add(text(
        x0,
        y,
        13,
        "#666666",
        format!("{} accepted / {} rejected", frame.accepted, frame.rejected),
    ));
    y += 20;
    panel = panel.add(text(
        x0,
        y,
        13,
        "#666666",
        format!("fee +{} / move -{}", frame.total_fee, frame.total_move_cost),
    ));

    // Comment (`#` lines) emitted by the solution, if any
    let comment = output.comments.get(t).map(String::as_str).unwrap_or("");
    if !comment.trim().is_empty() {
        y += 28;
        for line in comment.trim().lines().take(8) {
            // The svg crate writes text verbatim, so escape what the solution printed.
            let line = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            panel = panel.add(text(x0, y, 12, "#2266aa", format!("# {}", line)));
            y += 16;
        }
    }

    // ---- Color legend ----
    let ly = h_svg as i32 - 68;
    panel = panel.add(text(
        x0,
        ly,
        13,
        "#000000",
        format!("color: {}", mode.label()),
    ));
    if mode == ColorMode::Group {
        panel = panel.add(text(x0, ly + 22, 12, "#666666", "hue = group index"));
    } else {
        let bar_w = panel_w - 20;
        let bar_y = ly + 16;
        let steps = 40i32;
        for i in 0..steps {
            let sx = x0 + bar_w * i / steps;
            let ex = x0 + bar_w * (i + 1) / steps;
            let v = i as f64 / (steps - 1) as f64;
            panel = panel.add(rect(
                sx as usize,
                bar_y as usize,
                (ex - sx).max(1) as usize,
                12,
                &value_color(v),
            ));
        }
        panel = panel.add(
            rect(x0 as usize, bar_y as usize, bar_w as usize, 12, "none")
                .set("stroke", "#888888")
                .set("stroke-width", 1),
        );
        let (lo, hi) = match mode {
            ColorMode::Time => (format!("+{} or more", cold_at), "leaves next".to_owned()),
            _ => (
                "C = 1 (square)".to_owned(),
                format!("C = {} or less", C_MIN),
            ),
        };
        panel = panel.add(text(x0, bar_y + 24, 11, "#444444", lo));
        panel =
            panel.add(text(x0 + bar_w, bar_y + 24, 11, "#444444", hi).set("text-anchor", "end"));
    }

    doc = doc.add(panel);
    (output.score, err_of(output), doc.to_string())
}

/// Offsets for the 4 directions (up/down/left/right)
const DXY: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

/// A group currently in use, occupying a region of the park.
struct Active {
    /// Cells currently occupied
    cells: Vec<(usize, usize)>,
    /// Maximum boundary length L over all positions occupied so far.
    /// P is constant, so the position with the largest L corresponds to the smallest
    /// compactness C = 4*sqrt(P)/L.
    max_l: i64,
    /// Base payment V
    v: i64,
    /// Departure time T
    t: i64,
}

/// Simulation state. Holds money, occupancy, and active groups, and applies
/// departures, moves, and allocations.
struct State {
    /// Park side length N
    n: usize,
    /// Move cost coefficient R as an integer in thousandths (R = r_milli / 1000, r_milli in 1..=100)
    r_milli: i64,
    /// Current money
    money: i64,
    /// occupied[x][y] = index of the group occupying the cell (!0 if empty)
    occupied: Vec<Vec<usize>>,
    /// Active groups (group index -> state)
    active: HashMap<usize, Active>,
    /// Cumulative usage fees earned. Accumulated in `depart`, where the fee is computed.
    total_fee: i64,
    /// Cumulative move costs paid. Accumulated in `apply_moves`.
    total_move_cost: i64,
}

impl State {
    fn new(input: &Input) -> Self {
        State {
            n: input.n,
            r_milli: (input.r * 1000.0).round() as i64,
            money: 0,
            occupied: vec![vec![!0usize; input.n]; input.n],
            active: HashMap::new(),
            total_fee: 0,
            total_move_cost: 0,
        }
    }

    /// Groups whose departure time has passed by time s.
    fn leaving_before(&self, s: i64) -> Vec<usize> {
        self.active
            .iter()
            .filter(|(_, a)| a.t < s)
            .map(|(&j, _)| j)
            .collect()
    }

    /// Pairs each group with the fee it would pay if it departed now, sorted by index.
    fn fees_of(&self, js: Vec<usize>) -> Vec<(usize, i64)> {
        let mut fees: Vec<(usize, i64)> = js
            .into_iter()
            .map(|j| {
                let a = &self.active[&j];
                (j, round_payment(a.v, a.cells.len(), a.max_l))
            })
            .collect();
        fees.sort();
        fees
    }

    /// Snapshot of every group in use, sorted by index.
    fn active_infos(&self) -> Vec<ActiveInfo> {
        let mut infos: Vec<ActiveInfo> = self
            .active
            .iter()
            .map(|(&id, a)| {
                let p = a.cells.len();
                ActiveInfo {
                    id,
                    p,
                    v: a.v,
                    t: a.t,
                    l: boundary_len(&a.cells, self.n),
                    max_l: a.max_l,
                    fee: round_payment(a.v, p, a.max_l),
                }
            })
            .collect();
        infos.sort_by_key(|a| a.id);
        infos
    }

    /// Whether group j is currently in use
    fn is_active(&self, j: usize) -> bool {
        self.active.contains_key(&j)
    }

    /// Number of cells occupied by group j (= P_j)
    fn region_size(&self, j: usize) -> usize {
        self.active[&j].cells.len()
    }

    /// Departs every group that left before time s.
    fn depart_before(&mut self, s: i64) {
        for j in self.leaving_before(s) {
            self.depart(j);
        }
    }

    /// Departs all remaining active groups.
    fn depart_all(&mut self) {
        let js: Vec<usize> = self.active.keys().copied().collect();
        for j in js {
            self.depart(j);
        }
    }

    /// Departs group j and adds the usage fee round(V_j * C_j) to the money.
    /// Since C_j = 4*sqrt(P_j)/L_j, the rounding is done exactly in integers, not via f64.
    fn depart(&mut self, j: usize) {
        let a = self.active.remove(&j).unwrap();
        let fee = round_payment(a.v, a.cells.len(), a.max_l);
        self.money += fee;
        self.total_fee += fee;
        for &(x, y) in &a.cells {
            self.occupied[x][y] = !0;
        }
    }

    /// Applies simultaneous moves of active groups.
    /// `moves` is a sequence of (group index, new region). The caller has already checked
    /// that each region is in range, on grass, and made of distinct P_j cells, and that
    /// each group is active and not specified more than once. Here we check connectivity
    /// and overlapping occupancy.
    fn apply_moves(&mut self, moves: Vec<(usize, Vec<(usize, usize)>)>) -> Result<(), String> {
        // First clear the regions of all groups being moved
        for &(j, _) in &moves {
            for &(x, y) in &self.active[&j].cells {
                self.occupied[x][y] = !0;
            }
        }
        // Place them while checking connectivity and overlap with other groups
        for (j, cells) in &moves {
            if !connected(cells, self.n) {
                return Err(format!("Group {}'s moved region is not connected", j));
            }
            for &(x, y) in cells {
                if self.occupied[x][y] != !0 {
                    return Err(format!(
                        "Cell ({}, {}) overlaps another group while moving group {}",
                        x, y, j
                    ));
                }
                self.occupied[x][y] = *j;
            }
        }
        // Subtract the move cost from the money and update the maximum boundary length L
        for (j, cells) in moves {
            let l = boundary_len(&cells, self.n);
            let cost = move_cost(self.active[&j].v, self.r_milli);
            self.money -= cost;
            self.total_move_cost += cost;
            let a = self.active.get_mut(&j).unwrap();
            a.cells = cells;
            a.max_l.setmax(l);
        }
        Ok(())
    }

    /// Allocates region `cells` to the arriving group i.
    /// The caller has already checked that `cells` is in range, on grass, and made of
    /// distinct P_i cells.
    fn apply_alloc(
        &mut self,
        i: usize,
        g: GroupInfo,
        cells: Vec<(usize, usize)>,
    ) -> Result<(), String> {
        for &(x, y) in &cells {
            if self.occupied[x][y] != !0 {
                return Err(format!("Cell ({}, {}) is already occupied", x, y));
            }
        }
        if !connected(&cells, self.n) {
            return Err("Allocated region is not connected".to_owned());
        }
        let l = boundary_len(&cells, self.n);
        for &(x, y) in &cells {
            self.occupied[x][y] = i;
        }
        self.active.insert(
            i,
            Active {
                cells,
                max_l: l,
                v: g.v,
                t: g.t,
            },
        );
        Ok(())
    }
}

/// Reads whitespace-separated tokens. Blank lines are skipped, and comment lines starting
/// with `#` are accumulated into `comment` (so the visualizer can show them per turn).
/// The source can be any `BufRead`, so the same logic works for the judge (the child
/// process's stdout) and the visualizer (the output log as a string).
struct Reader<B: BufRead> {
    src: B,
    local: bool,
    /// Whether `#` lines are worth keeping. A solution can print them without bound, so the
    /// judge, which never reads them back, leaves them off rather than holding every one of
    /// them for the length of the run.
    keep_comments: bool,
    /// The line being tokenized and how far into it the tokens have been handed out. A turn that
    /// moves every group in use carries thousands of coordinates, so tokens are read out of this
    /// buffer in place rather than each becoming a `String` of its own.
    line: String,
    pos: usize,
    /// Accumulated comment lines (starting with `#`) read recently; cleared when taken.
    comment: String,
}

impl<B: BufRead> Reader<B> {
    fn new(src: B, local: bool, keep_comments: bool) -> Self {
        Reader {
            src,
            local,
            keep_comments,
            line: String::new(),
            pos: 0,
            comment: String::new(),
        }
    }

    /// Runs `f` on the next whitespace-separated token. The token is lent out rather than
    /// returned so that it can stay a slice of the line buffer.
    fn with_token<T>(&mut self, f: impl FnOnce(&str) -> T) -> Result<T, String> {
        loop {
            // Where the next token of the current line starts and ends, if it has one left.
            let span = {
                let rest = &self.line[self.pos..];
                let lead = rest.len() - rest.trim_start().len();
                if lead == rest.len() {
                    None
                } else {
                    let from = self.pos + lead;
                    let to = self.line[from..]
                        .find(char::is_whitespace)
                        .map_or(self.line.len(), |at| from + at);
                    Some((from, to))
                }
            };
            if let Some((from, to)) = span {
                self.pos = to;
                return Ok(f(&self.line[from..to]));
            }
            self.line.clear();
            self.pos = 0;
            match self.src.read_line(&mut self.line) {
                Ok(0) | Err(_) => return Err("Your program has terminated unexpectedly".to_owned()),
                _ => {}
            }
            if self.local {
                print!("{}", self.line);
            }
            // A comment line holds no tokens, so drop it and read on. A blank line needs no
            // handling of its own: the next pass finds no token in it and reads the one after.
            if let Some(c) = self.line.trim().strip_prefix('#') {
                if self.keep_comments {
                    self.comment += c.trim_start();
                    self.comment.push('\n');
                }
                self.line.clear();
            }
        }
    }

    /// Takes the accumulated comments (empties them afterwards).
    fn take_comment(&mut self) -> String {
        std::mem::take(&mut self.comment)
    }
}

/// Reads one token and validates that it lies within `range`. `what` names the value, so a
/// rejected token says which of the several numbers a turn carries was the bad one.
fn read_val<T, R, B>(reader: &mut Reader<B>, what: &str, range: R) -> Result<T, String>
where
    T: Copy + PartialOrd + std::fmt::Display + std::str::FromStr,
    R: RangeBounds<T>,
    B: BufRead,
{
    reader
        .with_token(|tok| read(Some(tok), range))?
        .map_err(|e| format!("{}: {}", what, e))
}

/// Reads `p` cell coordinates and checks that each cell is in range, on grass, and distinct.
fn read_region<B: BufRead>(
    reader: &mut Reader<B>,
    input: &Input,
    p: usize,
) -> Result<Vec<(usize, usize)>, String> {
    let n = input.n;
    let mut cells = Vec::with_capacity(p);
    let mut seen = vec![false; n * n];
    for _ in 0..p {
        let x: usize = read_val(reader, "cell x", 0..n)?;
        let y: usize = read_val(reader, "cell y", 0..n)?;
        if !input.grass[x][y] {
            return Err(format!("Cell ({}, {}) is not grass", x, y));
        }
        if std::mem::replace(&mut seen[x * n + y], true) {
            return Err(format!(
                "Cell ({}, {}) appears multiple times in a region",
                x, y
            ));
        }
        cells.push((x, y));
    }
    Ok(cells)
}

/// Boundary length L between the region and the outside (other cells or outside the park).
/// Compactness is defined as C = 4*sqrt(P) / L.
fn boundary_len(cells: &[(usize, usize)], n: usize) -> i64 {
    let in_region = region_mask(cells, n);
    let mut l = 0i64;
    for &(x, y) in cells {
        for &(dx, dy) in &DXY {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0
                || nx >= n as i32
                || ny < 0
                || ny >= n as i32
                || !in_region[nx as usize * n + ny as usize]
            {
                l += 1;
            }
        }
    }
    l
}

/// The region marked out on a park-sized grid, indexed by `x * n + y`. Every cell of a region is
/// looked at four times over, once per neighbor, so a flat grid beats hashing the coordinates.
fn region_mask(cells: &[(usize, usize)], n: usize) -> Vec<bool> {
    let mut mask = vec![false; n * n];
    for &(x, y) in cells {
        mask[x * n + y] = true;
    }
    mask
}

/// Computes round(V * C) = round(V * 4*sqrt(P) / L) exactly in integers, without f64.
/// In f64, when V*C lands exactly on an x.5 boundary (which can happen as a rational when
/// P is a perfect square) the rounding can be off by 1, so we get an approximate answer
/// with f64 and then correct it via integer comparison.
/// A fractional part of 0.5 is rounded up to match the problem's round (values are >= 0,
/// so this is not banker's rounding).
fn round_payment(v: i64, p: usize, l: i64) -> i64 {
    // An empty region has L = 0, which no bound below can ever satisfy, so `upper_ok` would
    // never turn true and the correction loop would spin forever. P >= 4 rules this out for
    // generated inputs, but a hand-written one can carry P = 0.
    if l <= 0 {
        return 0;
    }
    // n is the answer <=> (2n-1)*L <= 8*V*sqrt(P) < (2n+1)*L.
    // Each side is non-negative, so square and compare. (8*V*sqrt(P))^2 = 64*V^2*P.
    let sq = 64i128 * (v as i128) * (v as i128) * (p as i128);
    let li = l as i128;
    let lower_ok = |n: i64| {
        let t = (2 * n as i128 - 1) * li;
        t <= 0 || t * t <= sq
    };
    let upper_ok = |n: i64| {
        let t = (2 * n as i128 + 1) * li;
        sq < t * t
    };
    // Get an approximate answer with f64, then correct within +-1 to satisfy the bounds.
    let mut n = ((v as f64) * 4.0 * (p as f64).sqrt() / (l as f64)).round() as i64;
    n = n.max(0);
    while !lower_ok(n) {
        n -= 1;
    }
    while !upper_ok(n) {
        n += 1;
    }
    n
}

/// Computes the move cost max(round(V * R), 1) exactly in integers, without f64.
/// Since R = r_milli / 1000 (r_milli is an integer), R is not exactly representable in
/// binary f64 either, so rounding V*R can be off by 1 when it is exactly x.5.
/// round(V * r_milli / 1000) = (2*V*r_milli + 1000) / 2000, with 0.5 rounded up.
fn move_cost(v: i64, r_milli: i64) -> i64 {
    let cost = (2 * v as i128 * r_milli as i128 + 1000) / 2000;
    (cost as i64).max(1)
}

/// Whether a set of distinct cells forms a single 4-connected region.
fn connected(cells: &[(usize, usize)], n: usize) -> bool {
    if cells.is_empty() {
        return true;
    }
    let in_region = region_mask(cells, n);
    let mut seen = vec![false; n * n];
    let mut stack = vec![cells[0]];
    seen[cells[0].0 * n + cells[0].1] = true;
    let mut reached = 1usize;
    while let Some((x, y)) = stack.pop() {
        for &(dx, dy) in &DXY {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            // The flat index folds x and y together, so both have to be in range before it is
            // built or a cell past the edge of a row would read as one on the next row.
            if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                continue;
            }
            let k = nx as usize * n + ny as usize;
            if in_region[k] && !seen[k] {
                seen[k] = true;
                reached += 1;
                stack.push((nx as usize, ny as usize));
            }
        }
    }
    reached == cells.len()
}

/// What the solution did on one turn, for the visualizer.
struct TurnLog {
    /// Group allocated on this turn (None if it was rejected)
    arrived: Option<usize>,
    /// Groups moved on this turn
    moved: Vec<usize>,
}

/// Reads one turn of the solution's output (step 2 moves and step 3 allocation) and
/// applies it to the state. Shared judging logic for the judge (`exec`) and the
/// visualizer (`parse_output`).
fn apply_turn<B: BufRead>(
    reader: &mut Reader<B>,
    state: &mut State,
    input: &Input,
    i: usize,
) -> Result<TurnLog, String> {
    let g = input.groups[i];

    // Step 2: move active groups (optional)
    let a_i: usize = read_val(
        reader,
        "A_i (number of groups to move)",
        0..=state.active.len(),
    )?;
    let mut moves: Vec<(usize, Vec<(usize, usize)>)> = Vec::with_capacity(a_i);
    let mut moved_set: HashSet<usize> = HashSet::new();
    for _ in 0..a_i {
        let j: usize = read_val(reader, "group index to move", 0..input.m)?;
        if !state.is_active(j) {
            return Err(format!("Group {} is not currently in use", j));
        }
        if !moved_set.insert(j) {
            return Err(format!("Group {} is moved multiple times", j));
        }
        let cells = read_region(reader, input, state.region_size(j))?;
        moves.push((j, cells));
    }
    let mut moved: Vec<usize> = moves.iter().map(|&(j, _)| j).collect();
    moved.sort();
    state.apply_moves(moves)?;

    // Step 3: allocate the arriving group (Yes) or reject it (No)
    let taken = reader.with_token(|tok| match tok {
        "Yes" => Ok(true),
        "No" => Ok(false),
        _ => Err(format!("Expected `Yes` or `No`, got `{}`", tok)),
    })??;
    let mut arrived = None;
    if taken {
        let cells = read_region(reader, input, g.p)?;
        state.apply_alloc(i, g, cells)?;
        arrived = Some(i);
    }
    Ok(TurnLog { arrived, moved })
}

pub fn exec(p: &mut std::process::Child, local: bool) -> Result<i64, String> {
    // Read the input
    let mut input_str = String::new();
    std::io::stdin().read_to_string(&mut input_str).unwrap();
    let input = parse_input(&input_str);

    let mut stdin = std::io::BufWriter::new(p.stdin.take().unwrap());
    let mut reader = Reader::new(BufReader::new(p.stdout.take().unwrap()), local, false);

    // Send the park parameters and layout to the solution program
    let _ = writeln!(stdin, "{} {} {:.3}", input.n, input.m, input.r);
    for row in &input.grass {
        let line: String = row.iter().map(|&g| if g { '.' } else { '#' }).collect();
        let _ = writeln!(stdin, "{}", line);
    }
    let _ = stdin.flush();

    let mut state = State::new(&input);

    for i in 0..input.m {
        let g = input.groups[i];

        // Step 1: depart groups that left before this arrival
        state.depart_before(g.s);

        // Send the arriving group's information
        let _ = writeln!(stdin, "{} {} {} {} {}", i, g.s, g.t, g.p, g.v);
        let _ = stdin.flush();

        apply_turn(&mut reader, &mut state, &input, i)
            .map_err(|e| format!("turn {} (group {}): {}", i + 1, i, e))?;
    }

    // After the last group, depart all remaining active groups
    state.depart_all();

    // Close the pipe before waiting. `p.stdin` was taken, so `wait` cannot close it for us, and
    // a solution that reads its input to EOF would otherwise wait on us while we wait on it.
    drop(stdin);
    p.wait().unwrap();
    Ok(state.money.max(0))
}
