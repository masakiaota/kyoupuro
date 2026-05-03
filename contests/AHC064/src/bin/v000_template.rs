// v000_template.rs
use std::time::Instant;

pub const R: usize = 10;
pub const INIT_LEN: usize = 10;
pub const DEP_CAP: usize = 15;
pub const SIDING_CAP: usize = 20;
pub const MAX_TURNS: usize = 4000;
pub const CAR_COUNT: usize = R * INIT_LEN;

pub const MOVE_DEP_TO_SIDING: usize = 0;
pub const MOVE_SIDING_TO_DEP: usize = 1;

pub const AREA_DEP: u8 = 0;
pub const AREA_SIDING: u8 = 1;

pub type CarId = usize;
pub type LineIdx = usize;
pub type PosIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarPos {
    pub r: LineIdx,
    pub c: PosIdx,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub initial: [[CarId; INIT_LEN]; R],
    pub initial_pos: [CarPos; CAR_COUNT],
}

impl Input {
    pub fn read() -> Self {
        use std::io::Read;

        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        Self::from_str(&s)
    }

    pub fn from_str(s: &str) -> Self {
        let mut it = s.split_whitespace();
        let _r = it.next().unwrap().parse::<usize>().unwrap();

        let mut initial = [[0; INIT_LEN]; R];
        let mut initial_pos = [CarPos { r: 0, c: 0 }; CAR_COUNT];

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = it.next().unwrap().parse::<usize>().unwrap();
                initial[r][c] = car;
                initial_pos[car] = CarPos { r, c };
            }
        }

        Self {
            initial,
            initial_pos,
        }
    }

    #[inline(always)]
    pub fn target_id(r: usize, c: usize) -> CarId {
        r * INIT_LEN + c
    }

    #[inline(always)]
    pub fn target_line(car: CarId) -> usize {
        car / INIT_LEN
    }

    #[inline(always)]
    pub fn target_pos(car: CarId) -> usize {
        car % INIT_LEN
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub kind: usize,
    pub i: LineIdx,
    pub j: LineIdx,
    pub k: usize,
}

impl Move {
    #[inline(always)]
    pub fn dep_to_siding(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_DEP_TO_SIDING,
            i,
            j,
            k,
        }
    }

    #[inline(always)]
    pub fn siding_to_dep(i: usize, j: usize, k: usize) -> Self {
        Self {
            kind: MOVE_SIDING_TO_DEP,
            i,
            j,
            k,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    pub turns: Vec<Vec<Move>>,
}

impl Output {
    #[inline(always)]
    pub fn new() -> Self {
        Self { turns: Vec::new() }
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            turns: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn push_turn(&mut self, moves: Vec<Move>) {
        self.turns.push(moves);
    }

    pub fn to_output_string(&self) -> String {
        use std::fmt::Write;

        let move_count: usize = self.turns.iter().map(Vec::len).sum();
        let mut s = String::with_capacity(16 + self.turns.len() * 4 + move_count * 16);

        writeln!(&mut s, "{}", self.turns.len()).unwrap();
        for moves in &self.turns {
            writeln!(&mut s, "{}", moves.len()).unwrap();
            for mv in moves {
                writeln!(&mut s, "{} {} {} {}", mv.kind, mv.i, mv.j, mv.k).unwrap();
            }
        }

        s
    }

    pub fn print(&self) {
        use std::io::Write;

        let s = self.to_output_string();
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        out.write_all(s.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub dep: [[u8; DEP_CAP]; R],
    pub dep_len: [u8; R],

    pub sid: [[u8; SIDING_CAP]; R],
    pub sid_head: [u8; R],
    pub sid_len: [u8; R],

    pub car_area: [u8; CAR_COUNT],
    pub car_line: [u8; CAR_COUNT],
    pub car_slot: [u8; CAR_COUNT],

    pub partial_score: i32,
}

impl State {
    pub fn new(input: &Input) -> Self {
        let mut dep = [[0; DEP_CAP]; R];
        let dep_len = [INIT_LEN as u8; R];
        let sid = [[0; SIDING_CAP]; R];
        let sid_head = [0; R];
        let sid_len = [0; R];
        let mut car_area = [AREA_DEP; CAR_COUNT];
        let mut car_line = [0; CAR_COUNT];
        let mut car_slot = [0; CAR_COUNT];
        let mut partial_score = 0;

        for r in 0..R {
            for c in 0..INIT_LEN {
                let car = input.initial[r][c];
                dep[r][c] = car as u8;
                car_area[car] = AREA_DEP;
                car_line[car] = r as u8;
                car_slot[car] = c as u8;
                partial_score += Self::dep_score_piece(car, r, c);
            }
        }

        Self {
            dep,
            dep_len,
            sid,
            sid_head,
            sid_len,
            car_area,
            car_line,
            car_slot,
            partial_score,
        }
    }

    #[inline(always)]
    pub fn dep_score_piece(car: usize, r: usize, c: usize) -> i32 {
        if Input::target_line(car) != r {
            0
        } else if Input::target_pos(car) == c {
            10
        } else {
            1
        }
    }

    #[inline(always)]
    pub fn sid_slot(head: usize, offset: usize) -> usize {
        let slot = head + offset;
        if slot >= SIDING_CAP {
            slot - SIDING_CAP
        } else {
            slot
        }
    }

    #[inline(always)]
    pub fn sid_head_after_pop(head: usize, k: usize) -> usize {
        Self::sid_slot(head, k)
    }

    #[inline(always)]
    pub fn sid_head_after_push(head: usize, k: usize) -> usize {
        if head >= k {
            head - k
        } else {
            head + SIDING_CAP - k
        }
    }

    #[inline(always)]
    pub fn dep_car(&self, i: usize, pos: usize) -> usize {
        self.dep[i][pos] as usize
    }

    #[inline(always)]
    pub fn sid_car(&self, j: usize, pos: usize) -> usize {
        let slot = Self::sid_slot(self.sid_head[j] as usize, pos);
        self.sid[j][slot] as usize
    }

    #[inline(always)]
    pub fn apply_move(&mut self, mv: Move) {
        if mv.kind == MOVE_DEP_TO_SIDING {
            self.move_dep_to_siding(mv.i, mv.j, mv.k);
        } else {
            self.move_siding_to_dep(mv.i, mv.j, mv.k);
        }
    }

    #[inline(always)]
    pub fn apply_turn(&mut self, moves: &[Move]) {
        for &mv in moves {
            self.apply_move(mv);
        }
    }

    #[inline(always)]
    pub fn move_dep_to_siding(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len[i] as usize;
        let new_dep_len = old_dep_len - k;
        let new_sid_head = Self::sid_head_after_push(self.sid_head[j] as usize, k);

        for offset in 0..k {
            let dep_pos = new_dep_len + offset;
            let car = self.dep[i][dep_pos];
            let car_idx = car as usize;
            let sid_slot = Self::sid_slot(new_sid_head, offset);

            self.partial_score -= Self::dep_score_piece(car_idx, i, dep_pos);
            self.sid[j][sid_slot] = car;
            self.car_area[car_idx] = AREA_SIDING;
            self.car_line[car_idx] = j as u8;
            self.car_slot[car_idx] = sid_slot as u8;
        }

        self.dep_len[i] = new_dep_len as u8;
        self.sid_head[j] = new_sid_head as u8;
        self.sid_len[j] += k as u8;
    }

    #[inline(always)]
    pub fn move_siding_to_dep(&mut self, i: usize, j: usize, k: usize) {
        let old_dep_len = self.dep_len[i] as usize;
        let old_sid_head = self.sid_head[j] as usize;

        for offset in 0..k {
            let sid_slot = Self::sid_slot(old_sid_head, offset);
            let car = self.sid[j][sid_slot];
            let car_idx = car as usize;
            let dep_pos = old_dep_len + offset;

            self.dep[i][dep_pos] = car;
            self.partial_score += Self::dep_score_piece(car_idx, i, dep_pos);
            self.car_area[car_idx] = AREA_DEP;
            self.car_line[car_idx] = i as u8;
            self.car_slot[car_idx] = dep_pos as u8;
        }

        self.dep_len[i] = (old_dep_len + k) as u8;
        self.sid_head[j] = Self::sid_head_after_pop(old_sid_head, k) as u8;
        self.sid_len[j] -= k as u8;
    }

    pub fn is_complete(&self) -> bool {
        for r in 0..R {
            if self.dep_len[r] as usize != INIT_LEN {
                return false;
            }
            for c in 0..INIT_LEN {
                if self.dep[r][c] as usize != Input::target_id(r, c) {
                    return false;
                }
            }
        }
        true
    }

    pub fn judge_score(&self, turn_count: usize) -> i32 {
        if self.is_complete() {
            (100 * R + MAX_TURNS - turn_count) as i32
        } else {
            self.partial_score
        }
    }
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
