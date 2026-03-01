// v017_b_wall_snake_cross_library_allmix.rs
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Read};
use std::time::{Duration, Instant};

const ACT_R: u8 = 0;
const ACT_L: u8 = 1;
const ACT_F: u8 = 2;

const DIJ: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DIR_CHARS: [char; 4] = ['U', 'R', 'D', 'L'];
const N_FIXED: usize = 20;
const N2: usize = N_FIXED * N_FIXED;
const ORIENTS: usize = N2 * 4;
const M_LIMIT: usize = 4 * N_FIXED * N_FIXED; // 1600
const BIT_WORDS: usize = 7; // 448 bits for 400 cells
const DEFAULT_SEARCH_TIME_MS: u64 = 1000;
const INITIAL_NN_TOURS: usize = 14;
const EVAL_EXTRA_SHIFTS: usize = 16;
const OFFSET_TRIAL_LIMIT: usize = 128;
const DEFAULT_TEMPLATE_TOGGLE_ITERS: usize = 8000;

#[derive(Clone)]
struct Input {
    n: usize,
    ak: i64,
    am: i64,
    aw: i64,
    wall_v: Vec<Vec<u8>>,
    wall_h: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct RouteCandidate {
    start_cell: usize,
    start_dir: usize,
    actions: Vec<u8>,
}

#[derive(Clone, Copy)]
struct AutoState {
    a0: u8,
    b0: usize,
    a1: u8,
    b1: usize,
}

#[derive(Clone)]
struct RobotPlan {
    start_cell: usize,
    start_dir: usize,
    states: Vec<AutoState>,
    wall_v_add: Vec<Vec<u8>>,
    wall_h_add: Vec<Vec<u8>>,
    w_count: usize,
    value_v: i64,
}

#[derive(Clone, Copy)]
enum PatchKind {
    B1Only,
    Both,
}

#[derive(Clone, Copy)]
struct PatchRef {
    state_id: usize,
    kind: PatchKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BitSet {
    w: [u64; BIT_WORDS],
}

impl BitSet {
    fn empty() -> Self {
        Self { w: [0; BIT_WORDS] }
    }
    fn all_400() -> Self {
        let mut w = [u64::MAX; BIT_WORDS];
        w[BIT_WORDS - 1] = (1u64 << 16) - 1;
        Self { w }
    }
    fn set_cell(&mut self, cell: usize) {
        self.w[cell >> 6] |= 1u64 << (cell & 63);
    }
    fn or_assign(&mut self, other: &Self) {
        for i in 0..BIT_WORDS {
            self.w[i] |= other.w[i];
        }
    }
    fn count_new(&self, covered: &Self) -> u32 {
        let mut s = 0u32;
        for i in 0..BIT_WORDS {
            s += (self.w[i] & !covered.w[i]).count_ones();
        }
        s
    }
}

#[derive(Clone)]
struct TemplateChoice {
    m: usize,
    start_cell: usize,
    start_dir: usize,
    a0: Vec<u8>,
    b0: Vec<usize>,
    a1: Vec<u8>,
    b1: Vec<usize>,
    cover: BitSet,
}

#[derive(Clone, Copy)]
struct TemplateDef {
    m: usize,
    rules: &'static [(u8, u8, u8, u8)],
}

struct TemplatePlan {
    cands: Vec<TemplateChoice>,
    selected: Vec<usize>,
    value_v: i64,
}

struct TemplateEnv {
    wall: Vec<bool>,
    next_o: Vec<[usize; 3]>,
}

const TEMPLATE_LIBRARY_B: &[TemplateDef] = &[
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 9),
            (ACT_F, 1, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 11),
            (ACT_L, 12, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 13),
            (ACT_F, 1, ACT_R, 10),
            (ACT_L, 10, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 13),
            (ACT_F, 1, ACT_R, 10),
            (ACT_L, 2, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_R, 5, ACT_R, 13),
            (ACT_F, 1, ACT_R, 10),
            (ACT_L, 8, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 9, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 13),
            (ACT_F, 1, ACT_R, 10),
            (ACT_R, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 13, ACT_R, 9),
            (ACT_F, 1, ACT_R, 10),
            (ACT_F, 8, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 13),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 11),
            (ACT_L, 12, ACT_L, 2),
            (ACT_R, 6, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 11, ACT_L, 13),
            (ACT_F, 12, ACT_R, 11),
            (ACT_L, 12, ACT_L, 2),
            (ACT_L, 4, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 13, ACT_R, 11),
            (ACT_L, 12, ACT_L, 2),
            (ACT_R, 10, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 13),
            (ACT_L, 12, ACT_L, 2),
            (ACT_L, 0, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 13, ACT_R, 7),
            (ACT_L, 12, ACT_L, 2),
            (ACT_F, 2, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 13),
            (ACT_L, 12, ACT_L, 2),
            (ACT_L, 9, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_R, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 13),
            (ACT_L, 12, ACT_L, 2),
            (ACT_F, 6, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 13),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 11),
            (ACT_L, 12, ACT_L, 2),
            (ACT_F, 3, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 13),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 7),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 9),
            (ACT_F, 1, ACT_R, 10),
            (ACT_R, 13, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_R, 6),
            (ACT_L, 13, ACT_R, 10),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 1, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 13),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_R, 10, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 13),
            (ACT_L, 3, ACT_L, 3),
            (ACT_F, 9, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 13),
            (ACT_L, 2, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_F, 12, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 8, ACT_L, 13),
            (ACT_L, 3, ACT_L, 3),
            (ACT_R, 11, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 13),
            (ACT_L, 2, ACT_R, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_R, 1, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 1, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_R, 1, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 1, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 1),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 4, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 11),
            (ACT_L, 3, ACT_L, 3),
            (ACT_F, 9, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_F, 8, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 9, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 13),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 7, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 9),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 13, ACT_L, 9),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 0, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_R, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_R, 13),
            (ACT_L, 3, ACT_L, 3),
            (ACT_L, 11, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 5, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 6, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 11, ACT_L, 5),
            (ACT_R, 2, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 11),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 5, ACT_L, 6),
            (ACT_L, 4, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_R, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 6, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 5, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_R, 10, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_L, 4, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 11, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 5),
            (ACT_F, 8, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 9, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_R, 9, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 11),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 3, ACT_L, 5),
            (ACT_R, 11, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_F, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_F, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 2, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 12, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_F, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 6, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 12, ACT_L, 2),
            (ACT_L, 0, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 11),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 5, ACT_R, 10),
            (ACT_L, 4, ACT_L, 7),
            (ACT_F, 3, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_F, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_R, 8),
            (ACT_F, 8, ACT_L, 0),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_R, 12, ACT_R, 10),
            (ACT_R, 9, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_L, 2, ACT_L, 9),
            (ACT_F, 3, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 11),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_L, 12),
            (ACT_R, 8, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 0),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 12),
            (ACT_L, 8, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 11, ACT_L, 5),
            (ACT_R, 2, ACT_L, 1),
            (ACT_F, 3, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 11, ACT_L, 5),
            (ACT_R, 2, ACT_L, 1),
            (ACT_F, 3, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 0),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 12, ACT_R, 11),
            (ACT_R, 5, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 11),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 3, ACT_L, 5),
            (ACT_F, 3, ACT_L, 12),
            (ACT_L, 10, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 0),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_R, 5, ACT_R, 12),
            (ACT_R, 11, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_L, 11, ACT_L, 6),
            (ACT_F, 3, ACT_R, 11),
            (ACT_F, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_R, 9, ACT_L, 9),
            (ACT_F, 3, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_L, 9),
            (ACT_F, 3, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 7, ACT_R, 9),
            (ACT_F, 3, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 12, ACT_R, 6),
            (ACT_F, 8, ACT_L, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 9, ACT_L, 3),
            (ACT_F, 8, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 9),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_R, 1, ACT_L, 5),
            (ACT_L, 7, ACT_L, 0),
            (ACT_R, 5, ACT_L, 0),
            (ACT_F, 8, ACT_L, 12),
            (ACT_R, 7, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 3, ACT_L, 11),
            (ACT_F, 4, ACT_R, 9),
            (ACT_F, 3, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 7, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 7, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 4, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 2, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 0),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 1, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 7),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 1, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 7, ACT_R, 12),
            (ACT_R, 2, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 1, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_L, 6, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_L, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 12, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 4, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 0),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 7, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_R, 12, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 6, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 11, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 12),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 1, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 5, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 2, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 6, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 6, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 12, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_L, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 0, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 1, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 7, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 9, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 3, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 7, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 11, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 10, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_L, 9),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 10, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_F, 6, ACT_L, 0),
            (ACT_R, 12, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_F, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 9, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_L, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 10, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 0, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 2, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 6),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 8, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 8),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 12, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 12, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_L, 2),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 5, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 8, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_F, 12, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 4, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_R, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 12, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 1, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_F, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_L, 2, ACT_R, 8),
            (ACT_L, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_F, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 7),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 0, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 6, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_L, 9),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 1, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_F, 10, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 8, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_L, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 9, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 12, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 0, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 12, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 10, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 6, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 5, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 8, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 8, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 5, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 12, ACT_L, 0),
            (ACT_L, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 6, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 10, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 0, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_L, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 9, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 5, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 12, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 12),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 11, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 11, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 1, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 10, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 7),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 0, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 6),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 8, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_L, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 4, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 11, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_L, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 3, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_L, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 0, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 0),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 1, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 3),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_L, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 12, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 12, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 9, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 2, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_L, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 7, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_L, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 12),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 8, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_F, 5, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_L, 9, ACT_R, 12),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 10, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_R, 11),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 5, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_F, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 9, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 9, ACT_R, 12),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 2, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 12, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_R, 1, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 3, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_R, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 1, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 12, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 9, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_R, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 12, ACT_L, 8),
            (ACT_R, 3, ACT_R, 8),
            (ACT_R, 8, ACT_R, 8),
            (ACT_F, 10, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_F, 3, ACT_L, 7),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
            (ACT_R, 2, ACT_R, 4),
            (ACT_F, 3, ACT_L, 5),
            (ACT_L, 6, ACT_L, 0),
            (ACT_R, 9, ACT_R, 10),
            (ACT_L, 11, ACT_L, 4),
            (ACT_F, 11, ACT_L, 0),
            (ACT_F, 1, ACT_R, 8),
            (ACT_R, 3, ACT_R, 12),
            (ACT_R, 8, ACT_R, 8),
            (ACT_L, 0, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_R, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 13),
            (ACT_F, 3, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 7, ACT_R, 13),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 1, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 7),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_R, 11, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 13, ACT_R, 10),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 8, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_L, 3, ACT_R, 13),
            (ACT_F, 8, ACT_R, 10),
            (ACT_R, 0, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 5, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 10, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 12),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 3),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 13),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 5, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 4, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 12, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 1, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 13, ACT_R, 6),
            (ACT_F, 12, ACT_R, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 7, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 13),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 4, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 13, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 8),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 6, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 13, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 4, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_L, 1),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 13),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 8, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_L, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 13, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 5, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 1, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 0, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 2, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 13, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 4),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 5, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 12, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_L, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 9, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 13),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 5, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 0, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 13),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 10, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_L, 11),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 0, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 0),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 9, ACT_R, 13),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 12, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 4, ACT_R, 2),
            (ACT_L, 7, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 13),
            (ACT_F, 12, ACT_L, 7),
            (ACT_R, 8, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 9, ACT_R, 5),
            (ACT_F, 4, ACT_R, 2),
            (ACT_F, 0, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 13),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 9, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 4, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 13),
            (ACT_L, 4, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 13),
            (ACT_F, 11, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_L, 9),
            (ACT_F, 8, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 13, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_R, 5, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 5, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 12, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 0, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_R, 1, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 13),
            (ACT_R, 11, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 1, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 5, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 8, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 2),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 13, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 2, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 13, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 6, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 12),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 8, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 13, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 1, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 3, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_R, 4, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_F, 11, ACT_R, 6),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 6, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 13),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 1, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 12, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 13),
            (ACT_L, 10, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_F, 1, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 12, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 12, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 4, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 7, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 13, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 2, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 2, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 13),
            (ACT_R, 11, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 3, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_L, 7, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 12, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 10, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 3),
            (ACT_R, 12, ACT_R, 13),
            (ACT_F, 8, ACT_R, 9),
            (ACT_L, 13, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 13, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_L, 3, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 13),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_R, 6, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_L, 0, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 2, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_L, 1, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_F, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 6),
            (ACT_R, 4, ACT_R, 13),
            (ACT_F, 3, ACT_R, 8),
            (ACT_F, 8, ACT_R, 9),
            (ACT_R, 3, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 11, ACT_R, 9),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 13, ACT_L, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 8, ACT_L, 1),
            (ACT_L, 3, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 0, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 8, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 7),
            (ACT_R, 12, ACT_L, 10),
            (ACT_L, 6, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 12),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 6, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 0),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 9, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 7, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 4),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 0, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 12),
            (ACT_F, 5, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 12),
            (ACT_R, 0, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 12, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 6, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_R, 10),
            (ACT_L, 0, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 6),
            (ACT_L, 4, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 10, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 6, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_L, 3, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 1, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 0),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_R, 2, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 12, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_L, 5, ACT_L, 10),
            (ACT_L, 0, ACT_R, 10),
            (ACT_L, 8, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 6),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 11, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_R, 7, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 12, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_L, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 3, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 1, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_L, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 13, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_R, 5),
            (ACT_F, 11, ACT_R, 7),
            (ACT_R, 4, ACT_R, 12),
            (ACT_R, 10, ACT_L, 9),
            (ACT_F, 12, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 12, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 0, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 1),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_L, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 13),
            (ACT_R, 9, ACT_R, 6),
            (ACT_L, 0, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 5),
            (ACT_R, 11, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_L, 6),
            (ACT_F, 9, ACT_R, 12),
            (ACT_F, 0, ACT_R, 7),
            (ACT_L, 5, ACT_L, 4),
            (ACT_F, 8, ACT_R, 4),
            (ACT_L, 11, ACT_L, 5),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 0, ACT_L, 10),
            (ACT_F, 0, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_R, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 12),
            (ACT_L, 13, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 2, ACT_R, 9),
            (ACT_L, 7, ACT_R, 1),
            (ACT_R, 10, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 11, ACT_R, 7),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 10, ACT_L, 5),
            (ACT_L, 10, ACT_L, 6),
            (ACT_R, 6, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 13),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_R, 11),
            (ACT_R, 9, ACT_R, 5),
            (ACT_F, 8, ACT_L, 11),
            (ACT_F, 0, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 12),
            (ACT_R, 9, ACT_R, 7),
            (ACT_F, 9, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 11),
            (ACT_L, 9, ACT_L, 6),
            (ACT_F, 0, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 11),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 1, ACT_R, 4),
            (ACT_R, 11, ACT_R, 7),
            (ACT_L, 5, ACT_L, 9),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 10, ACT_R, 6),
            (ACT_L, 1, ACT_R, 8),
            (ACT_R, 6, ACT_R, 12),
            (ACT_F, 2, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 11),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 10, ACT_R, 4),
            (ACT_L, 6, ACT_R, 7),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 4, ACT_L, 9),
            (ACT_L, 11, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 5),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 13),
            (ACT_F, 11, ACT_R, 10),
            (ACT_R, 12, ACT_R, 6),
            (ACT_L, 10, ACT_R, 10),
            (ACT_F, 2, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 12, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 9, ACT_L, 11),
            (ACT_F, 2, ACT_L, 4),
            (ACT_R, 6, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 13),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 3),
            (ACT_R, 12, ACT_R, 6),
            (ACT_L, 11, ACT_R, 1),
            (ACT_L, 13, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_L, 1),
            (ACT_R, 4, ACT_R, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_F, 12, ACT_R, 12),
            (ACT_L, 5, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 13),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_F, 8, ACT_R, 12),
            (ACT_F, 4, ACT_L, 0),
            (ACT_L, 13, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 12, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 10, ACT_L, 9),
            (ACT_L, 12, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 11),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 10, ACT_L, 4),
            (ACT_R, 6, ACT_R, 9),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_F, 0, ACT_L, 7),
            (ACT_F, 7, ACT_R, 11),
            (ACT_L, 11, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 13),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_R, 2),
            (ACT_R, 6, ACT_R, 4),
            (ACT_F, 10, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 0, ACT_R, 7),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 10, ACT_L, 5),
            (ACT_F, 5, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 7),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 11, ACT_L, 4),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 12),
            (ACT_F, 2, ACT_R, 10),
            (ACT_R, 5, ACT_R, 6),
            (ACT_R, 3, ACT_R, 10),
            (ACT_L, 12, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 4),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 13, ACT_L, 12),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 0),
            (ACT_F, 11, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 13),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 9),
            (ACT_R, 6, ACT_L, 2),
            (ACT_R, 13, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 9,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 1, ACT_R, 4),
            (ACT_R, 6, ACT_R, 7),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 6,
        rules: &[
            (ACT_F, 3, ACT_L, 0),
            (ACT_L, 0, ACT_R, 2),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 4, ACT_R, 0),
            (ACT_F, 1, ACT_L, 4),
            (ACT_R, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 12, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 13),
            (ACT_F, 0, ACT_R, 1),
            (ACT_R, 7, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 13),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 6, ACT_R, 5),
            (ACT_R, 9, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 12, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 13),
            (ACT_F, 0, ACT_L, 5),
            (ACT_L, 9, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 13, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_F, 11, ACT_L, 9),
            (ACT_L, 12, ACT_L, 4),
            (ACT_F, 0, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 12, ACT_R, 7),
            (ACT_L, 13, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_F, 11, ACT_R, 11),
            (ACT_F, 12, ACT_L, 9),
            (ACT_R, 6, ACT_L, 2),
            (ACT_L, 12, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 6, ACT_R, 5),
            (ACT_R, 13, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 13),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 1, ACT_L, 12),
            (ACT_L, 12, ACT_R, 7),
            (ACT_L, 6, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 13, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_L, 8, ACT_L, 12),
            (ACT_L, 12, ACT_R, 12),
            (ACT_R, 11, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_F, 2, ACT_L, 9),
            (ACT_F, 10, ACT_L, 6),
            (ACT_L, 11, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 13),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 6, ACT_R, 5),
            (ACT_L, 5, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 7, ACT_L, 5),
            (ACT_L, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_R, 12),
            (ACT_L, 9, ACT_R, 3),
            (ACT_R, 6, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 13),
            (ACT_R, 8, ACT_L, 9),
            (ACT_L, 5, ACT_R, 1),
            (ACT_R, 12, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 7, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 9),
            (ACT_R, 0, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_R, 7, ACT_L, 6),
            (ACT_L, 6, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 12, ACT_R, 0),
            (ACT_R, 6, ACT_L, 2),
            (ACT_L, 6, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 5),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 9),
            (ACT_F, 9, ACT_L, 4),
            (ACT_L, 6, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_L, 12, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_R, 6, ACT_R, 9),
            (ACT_L, 6, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 13),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 2, ACT_L, 9),
            (ACT_L, 5, ACT_R, 1),
            (ACT_L, 6, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 13, ACT_L, 5),
            (ACT_F, 2, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 13),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 2, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_L, 12, ACT_R, 4),
            (ACT_L, 7, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 13, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_R, 7, ACT_L, 6),
            (ACT_L, 0, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 13),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_F, 2, ACT_L, 9),
            (ACT_F, 10, ACT_L, 6),
            (ACT_F, 0, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 13),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 11),
            (ACT_R, 5, ACT_L, 5),
            (ACT_R, 9, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 13),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 6, ACT_R, 12),
            (ACT_L, 1, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 12, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 7),
            (ACT_R, 10, ACT_R, 13),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_R, 9, ACT_R, 6),
            (ACT_L, 5, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 13),
            (ACT_L, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 7, ACT_L, 5),
            (ACT_R, 6, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 13),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_R, 8, ACT_L, 5),
            (ACT_F, 5, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_R, 11, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 13),
            (ACT_L, 8, ACT_L, 12),
            (ACT_L, 8, ACT_L, 10),
            (ACT_R, 2, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 13, ACT_R, 1),
            (ACT_R, 12, ACT_R, 11),
            (ACT_R, 8, ACT_L, 9),
            (ACT_F, 7, ACT_L, 5),
            (ACT_L, 10, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 13),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 1),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 9),
            (ACT_R, 3, ACT_L, 4),
            (ACT_L, 9, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_L, 9, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 12),
            (ACT_R, 3, ACT_R, 11),
            (ACT_R, 13, ACT_L, 9),
            (ACT_F, 0, ACT_L, 3),
            (ACT_F, 5, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 4, ACT_L, 13),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 1, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 6, ACT_R, 4),
            (ACT_L, 5, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 7),
            (ACT_R, 7, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_R, 6, ACT_R, 1),
            (ACT_F, 0, ACT_L, 5),
            (ACT_F, 8, ACT_L, 4),
            (ACT_R, 10, ACT_R, 13),
            (ACT_R, 9, ACT_R, 11),
            (ACT_R, 8, ACT_L, 12),
            (ACT_L, 0, ACT_R, 7),
            (ACT_L, 4, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 2, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 1, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 12),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 9, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_R, 2, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 2, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_F, 8, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 0),
            (ACT_F, 2, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 1, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 10),
            (ACT_L, 10, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_L, 1, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_F, 0, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_L, 11),
            (ACT_R, 1, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 9,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 3, ACT_R, 5),
            (ACT_F, 5, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 9,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 8),
            (ACT_R, 5, ACT_L, 4),
            (ACT_L, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 2),
            (ACT_L, 0, ACT_R, 4),
            (ACT_L, 1, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 9),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 4, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 3, ACT_R, 5),
            (ACT_F, 5, ACT_L, 6),
            (ACT_R, 8, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 5),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 2),
            (ACT_L, 9, ACT_R, 0),
            (ACT_L, 7, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 7, ACT_L, 0),
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 5, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 9),
            (ACT_F, 3, ACT_R, 5),
            (ACT_F, 5, ACT_L, 6),
            (ACT_R, 9, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 6),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 2),
            (ACT_L, 6, ACT_L, 9),
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 4, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_R, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_F, 8, ACT_R, 8),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 3, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 5, ACT_L, 4),
            (ACT_L, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 8, ACT_L, 7),
            (ACT_L, 1, ACT_L, 1),
            (ACT_R, 4, ACT_L, 8),
            (ACT_F, 2, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 9, ACT_R, 5),
            (ACT_R, 3, ACT_L, 6),
            (ACT_L, 5, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 1, ACT_L, 6),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 8, ACT_R, 7),
            (ACT_L, 6, ACT_R, 1),
            (ACT_R, 3, ACT_L, 6),
            (ACT_L, 7, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 5, ACT_L, 4),
            (ACT_L, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 8),
            (ACT_R, 8, ACT_R, 1),
            (ACT_F, 0, ACT_L, 6),
            (ACT_L, 1, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 8),
            (ACT_R, 5, ACT_L, 9),
            (ACT_R, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 2),
            (ACT_L, 0, ACT_R, 4),
            (ACT_L, 1, ACT_R, 2),
            (ACT_F, 7, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 8),
            (ACT_R, 5, ACT_L, 4),
            (ACT_L, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 9),
            (ACT_R, 0, ACT_R, 4),
            (ACT_L, 1, ACT_R, 2),
            (ACT_F, 7, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 7, ACT_L, 6),
            (ACT_F, 5, ACT_R, 0),
            (ACT_R, 8, ACT_L, 3),
            (ACT_L, 2, ACT_R, 1),
            (ACT_L, 8, ACT_R, 1),
            (ACT_L, 3, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 9),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 4, ACT_L, 10),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_L, 0),
            (ACT_L, 3, ACT_R, 5),
            (ACT_F, 5, ACT_L, 6),
            (ACT_R, 8, ACT_L, 7),
            (ACT_L, 7, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 10),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 9, ACT_R, 4),
            (ACT_L, 7, ACT_L, 6),
            (ACT_F, 5, ACT_R, 0),
            (ACT_R, 8, ACT_L, 2),
            (ACT_F, 3, ACT_L, 6),
            (ACT_F, 3, ACT_L, 5),
            (ACT_F, 5, ACT_L, 1),
            (ACT_R, 10, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 10, ACT_R, 3),
            (ACT_F, 2, ACT_R, 5),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_F, 3, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_L, 6),
            (ACT_R, 8, ACT_R, 3),
            (ACT_R, 0, ACT_R, 9),
            (ACT_R, 6, ACT_L, 8),
            (ACT_F, 2, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 5),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 10),
            (ACT_R, 7, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 5),
            (ACT_R, 7, ACT_R, 9),
            (ACT_R, 9, ACT_R, 7),
            (ACT_F, 2, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 3, ACT_R, 10),
            (ACT_F, 5, ACT_L, 3),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 9, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 10, ACT_L, 0),
            (ACT_L, 3, ACT_R, 7),
            (ACT_F, 5, ACT_L, 5),
            (ACT_L, 1, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 10),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 3, ACT_R, 5),
            (ACT_F, 9, ACT_L, 5),
            (ACT_F, 9, ACT_L, 0),
            (ACT_R, 8, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 4),
            (ACT_R, 5, ACT_R, 4),
            (ACT_F, 2, ACT_L, 6),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 8),
            (ACT_R, 10, ACT_L, 4),
            (ACT_F, 0, ACT_L, 9),
            (ACT_F, 9, ACT_L, 7),
            (ACT_F, 9, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 7, ACT_L, 0),
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 5, ACT_L, 8),
            (ACT_F, 4, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 8, ACT_L, 10),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 7, ACT_L, 0),
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 5, ACT_L, 5),
            (ACT_L, 8, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 5),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 10, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 5),
            (ACT_L, 0, ACT_R, 9),
            (ACT_L, 6, ACT_R, 7),
            (ACT_L, 5, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 2, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 3, ACT_R, 9),
            (ACT_F, 10, ACT_L, 6),
            (ACT_R, 9, ACT_L, 7),
            (ACT_R, 9, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 5),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 9),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 10),
            (ACT_L, 8, ACT_R, 5),
            (ACT_L, 0, ACT_R, 0),
            (ACT_F, 7, ACT_L, 1),
            (ACT_R, 9, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 10),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 5),
            (ACT_L, 0, ACT_R, 9),
            (ACT_L, 6, ACT_R, 7),
            (ACT_R, 5, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_R, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_F, 8, ACT_R, 8),
            (ACT_F, 2, ACT_R, 10),
            (ACT_R, 3, ACT_L, 8),
            (ACT_L, 6, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 2),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 9, ACT_R, 5),
            (ACT_L, 0, ACT_R, 0),
            (ACT_R, 8, ACT_L, 10),
            (ACT_R, 10, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_R, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_F, 8, ACT_R, 8),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 3, ACT_L, 10),
            (ACT_R, 3, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 5, ACT_L, 4),
            (ACT_L, 6, ACT_L, 2),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 8, ACT_L, 7),
            (ACT_L, 1, ACT_L, 10),
            (ACT_R, 4, ACT_L, 8),
            (ACT_F, 2, ACT_R, 2),
            (ACT_R, 8, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 2, ACT_L, 6),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 7, ACT_L, 6),
            (ACT_R, 8, ACT_R, 4),
            (ACT_F, 5, ACT_R, 10),
            (ACT_R, 6, ACT_R, 0),
            (ACT_R, 3, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_L, 10),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 5),
            (ACT_L, 0, ACT_R, 9),
            (ACT_L, 7, ACT_L, 4),
            (ACT_R, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_L, 0, ACT_R, 6),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 2),
            (ACT_L, 10, ACT_L, 9),
            (ACT_L, 8, ACT_R, 0),
            (ACT_F, 4, ACT_L, 6),
            (ACT_F, 3, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_R, 8, ACT_L, 0),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_F, 8, ACT_R, 8),
            (ACT_F, 10, ACT_R, 6),
            (ACT_R, 3, ACT_L, 8),
            (ACT_F, 7, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 10),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 8, ACT_R, 5),
            (ACT_L, 0, ACT_R, 9),
            (ACT_R, 7, ACT_L, 4),
            (ACT_L, 10, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 8),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 10),
            (ACT_L, 6, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_L, 9, ACT_R, 5),
            (ACT_R, 3, ACT_L, 6),
            (ACT_L, 5, ACT_R, 8),
            (ACT_L, 4, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_R, 6, ACT_R, 3),
            (ACT_F, 2, ACT_R, 9),
            (ACT_L, 0, ACT_R, 1),
            (ACT_R, 5, ACT_R, 4),
            (ACT_R, 8, ACT_L, 7),
            (ACT_F, 5, ACT_R, 0),
            (ACT_F, 2, ACT_R, 6),
            (ACT_F, 8, ACT_R, 10),
            (ACT_F, 2, ACT_R, 6),
            (ACT_R, 3, ACT_L, 8),
            (ACT_R, 5, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 9,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 9),
            (ACT_R, 6, ACT_R, 11),
            (ACT_L, 8, ACT_L, 1),
            (ACT_F, 1, ACT_R, 10),
            (ACT_F, 0, ACT_R, 2),
            (ACT_R, 7, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 8,
        rules: &[
            (ACT_F, 5, ACT_R, 6),
            (ACT_F, 2, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 1, ACT_R, 3),
            (ACT_L, 3, ACT_R, 1),
            (ACT_L, 2, ACT_R, 6),
            (ACT_L, 7, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 8,
        rules: &[
            (ACT_F, 4, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 7, ACT_L, 1),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 5),
            (ACT_L, 5, ACT_L, 1),
            (ACT_R, 3, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 4,
        rules: &[
            (ACT_F, 1, ACT_L, 2),
            (ACT_L, 2, ACT_R, 0),
            (ACT_F, 3, ACT_L, 0),
            (ACT_R, 0, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 9,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 0),
            (ACT_R, 6, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 9),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 0, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_F, 5, ACT_R, 6),
            (ACT_F, 2, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 9, ACT_R, 3),
            (ACT_L, 3, ACT_R, 8),
            (ACT_F, 7, ACT_R, 6),
            (ACT_L, 7, ACT_L, 3),
            (ACT_L, 4, ACT_L, 5),
            (ACT_F, 2, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 9),
            (ACT_R, 6, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
            (ACT_R, 0, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 0),
            (ACT_R, 6, ACT_R, 2),
            (ACT_L, 9, ACT_L, 1),
            (ACT_L, 7, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 9),
            (ACT_F, 1, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 3),
            (ACT_R, 3, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 10,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 0),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 9),
            (ACT_R, 7, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 4, ACT_L, 10),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 5, ACT_R, 2),
            (ACT_L, 8, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_F, 6, ACT_R, 7),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 9),
            (ACT_L, 7, ACT_R, 4),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 0, ACT_L, 3),
            (ACT_R, 3, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 10),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 9),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 0, ACT_L, 3),
            (ACT_R, 3, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_F, 5, ACT_R, 6),
            (ACT_F, 2, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 9, ACT_R, 3),
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 7, ACT_L, 6),
            (ACT_L, 7, ACT_L, 3),
            (ACT_L, 4, ACT_L, 5),
            (ACT_F, 2, ACT_L, 3),
            (ACT_R, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 10, ACT_R, 9),
            (ACT_R, 0, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
            (ACT_R, 2, ACT_R, 0),
            (ACT_F, 9, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_R, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 10),
            (ACT_R, 6, ACT_R, 2),
            (ACT_L, 9, ACT_L, 1),
            (ACT_L, 7, ACT_L, 8),
            (ACT_F, 8, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 10, ACT_R, 0),
            (ACT_R, 6, ACT_R, 2),
            (ACT_L, 9, ACT_L, 1),
            (ACT_L, 7, ACT_L, 8),
            (ACT_F, 0, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 9),
            (ACT_F, 1, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 3),
            (ACT_R, 3, ACT_R, 10),
            (ACT_L, 6, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 10),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 9),
            (ACT_R, 7, ACT_L, 0),
            (ACT_F, 9, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 10),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 0),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 9),
            (ACT_R, 7, ACT_R, 8),
            (ACT_R, 7, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 0),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 9),
            (ACT_R, 7, ACT_R, 10),
            (ACT_R, 5, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_R, 6),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 8, ACT_R, 10),
            (ACT_L, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 9),
            (ACT_R, 7, ACT_R, 8),
            (ACT_L, 9, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 11,
        rules: &[
            (ACT_F, 5, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 10),
            (ACT_R, 1, ACT_R, 3),
            (ACT_R, 5, ACT_L, 7),
            (ACT_R, 4, ACT_L, 0),
            (ACT_R, 9, ACT_R, 3),
            (ACT_L, 8, ACT_L, 1),
            (ACT_R, 8, ACT_L, 8),
            (ACT_L, 5, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 10, ACT_L, 5),
            (ACT_F, 2, ACT_L, 9),
            (ACT_F, 5, ACT_R, 11),
            (ACT_F, 7, ACT_R, 9),
            (ACT_F, 3, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 10, ACT_R, 11),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 9),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 3),
            (ACT_R, 4, ACT_L, 5),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 6, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_L, 10, ACT_L, 5),
            (ACT_F, 2, ACT_L, 11),
            (ACT_F, 5, ACT_R, 6),
            (ACT_R, 4, ACT_L, 7),
            (ACT_R, 1, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 10, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 4, ACT_L, 6),
            (ACT_F, 2, ACT_L, 11),
            (ACT_F, 5, ACT_R, 2),
            (ACT_L, 4, ACT_R, 4),
            (ACT_R, 2, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 10, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 2, ACT_L, 6),
            (ACT_F, 2, ACT_L, 11),
            (ACT_F, 5, ACT_R, 2),
            (ACT_L, 4, ACT_L, 4),
            (ACT_L, 5, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_R, 8, ACT_R, 6),
            (ACT_F, 2, ACT_L, 10),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 4, ACT_L, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_F, 11, ACT_R, 10),
            (ACT_F, 4, ACT_L, 10),
            (ACT_R, 1, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 2, ACT_L, 10),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 5),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 8),
            (ACT_L, 11, ACT_L, 9),
            (ACT_R, 4, ACT_L, 7),
            (ACT_F, 2, ACT_R, 3),
            (ACT_F, 5, ACT_R, 3),
            (ACT_F, 4, ACT_L, 10),
            (ACT_L, 9, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 10, ACT_L, 11),
            (ACT_F, 2, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 9, ACT_R, 3),
            (ACT_L, 3, ACT_R, 8),
            (ACT_F, 7, ACT_R, 6),
            (ACT_R, 3, ACT_L, 3),
            (ACT_L, 4, ACT_L, 5),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 8, ACT_L, 6),
            (ACT_R, 9, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 6, ACT_R, 7),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_R, 9),
            (ACT_L, 7, ACT_L, 4),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 0, ACT_L, 3),
            (ACT_R, 11, ACT_L, 4),
            (ACT_F, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 10),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 0),
            (ACT_F, 6, ACT_L, 9),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 11, ACT_L, 3),
            (ACT_R, 3, ACT_L, 0),
            (ACT_L, 2, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 11),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 9, ACT_R, 0),
            (ACT_F, 6, ACT_L, 4),
            (ACT_F, 2, ACT_L, 3),
            (ACT_F, 5, ACT_R, 10),
            (ACT_R, 8, ACT_R, 4),
            (ACT_L, 3, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 5, ACT_R, 6),
            (ACT_F, 2, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 9, ACT_R, 3),
            (ACT_L, 3, ACT_R, 8),
            (ACT_F, 10, ACT_R, 6),
            (ACT_L, 7, ACT_L, 3),
            (ACT_L, 4, ACT_L, 5),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 11, ACT_L, 5),
            (ACT_L, 7, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 7, ACT_R, 6),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 8),
            (ACT_R, 1, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_L, 7, ACT_L, 9),
            (ACT_R, 0, ACT_R, 4),
            (ACT_L, 8, ACT_L, 1),
            (ACT_R, 11, ACT_R, 10),
            (ACT_L, 2, ACT_R, 1),
            (ACT_F, 3, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 11),
            (ACT_L, 3, ACT_R, 1),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 10, ACT_R, 6),
            (ACT_R, 5, ACT_R, 9),
            (ACT_F, 6, ACT_L, 3),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 0, ACT_L, 7),
            (ACT_R, 4, ACT_R, 11),
            (ACT_R, 3, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 4, ACT_R, 7),
            (ACT_F, 2, ACT_L, 8),
            (ACT_L, 5, ACT_R, 1),
            (ACT_F, 4, ACT_L, 11),
            (ACT_R, 8, ACT_R, 3),
            (ACT_F, 4, ACT_L, 1),
            (ACT_R, 8, ACT_R, 9),
            (ACT_F, 6, ACT_R, 10),
            (ACT_F, 2, ACT_L, 3),
            (ACT_L, 4, ACT_L, 9),
            (ACT_L, 9, ACT_L, 9),
            (ACT_L, 11, ACT_R, 9),
        ],
    },
];




#[derive(Clone, Copy)]
struct XorShift64 {
    x: u64,
}

impl XorShift64 {
    fn new(mut seed: u64) -> Self {
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        Self { x: seed }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.x;
        x ^= x << 7;
        x ^= x >> 9;
        self.x = x;
        x
    }
    fn gen_usize(&mut self, upper: usize) -> usize {
        (self.next_u64() % upper as u64) as usize
    }
}

fn act_char(a: u8) -> char {
    match a {
        ACT_R => 'R',
        ACT_L => 'L',
        ACT_F => 'F',
        _ => unreachable!(),
    }
}

const TEMPLATE_LIBRARY_A_MIX: &[TemplateDef] = &[
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 8),
            (ACT_R, 8, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_R, 11),
            (ACT_R, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 2, ACT_L, 11),
            (ACT_F, 9, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 1, ACT_L, 11),
            (ACT_L, 9, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 2),
            (ACT_R, 8, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 2),
            (ACT_R, 7, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 7, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_R, 11, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_F, 0, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 9, ACT_L, 11),
            (ACT_F, 2, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 11),
            (ACT_F, 1, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 4),
            (ACT_R, 7, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_F, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 11, ACT_L, 5),
            (ACT_F, 1, ACT_L, 6),
            (ACT_R, 7, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 11, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_L, 5),
            (ACT_R, 3, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 13),
            (ACT_F, 0, ACT_L, 6),
            (ACT_R, 9, ACT_R, 12),
            (ACT_F, 0, ACT_R, 7),
            (ACT_L, 5, ACT_L, 4),
            (ACT_F, 8, ACT_R, 4),
            (ACT_L, 11, ACT_L, 5),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 0, ACT_L, 10),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 13, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_L, 6),
            (ACT_R, 9, ACT_R, 12),
            (ACT_F, 0, ACT_R, 7),
            (ACT_L, 5, ACT_L, 4),
            (ACT_F, 8, ACT_R, 4),
            (ACT_L, 11, ACT_L, 13),
            (ACT_F, 8, ACT_R, 9),
            (ACT_F, 0, ACT_L, 10),
            (ACT_F, 0, ACT_L, 12),
            (ACT_F, 3, ACT_L, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 13, ACT_L, 12),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 0),
            (ACT_L, 10, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 13, ACT_L, 5),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 10),
            (ACT_L, 0, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 13),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 12),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 0, ACT_L, 0),
            (ACT_R, 10, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 13, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 5),
            (ACT_R, 11, ACT_R, 6),
            (ACT_L, 6, ACT_R, 10),
            (ACT_R, 11, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 13, ACT_R, 9),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 12),
            (ACT_R, 11, ACT_R, 6),
            (ACT_F, 3, ACT_L, 0),
            (ACT_L, 11, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 11),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 10, ACT_R, 7),
            (ACT_L, 5, ACT_L, 13),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 12, ACT_L, 5),
            (ACT_R, 11, ACT_R, 6),
            (ACT_R, 7, ACT_L, 6),
            (ACT_R, 13, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 0, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 5),
            (ACT_L, 3, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 0, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 12),
            (ACT_L, 2, ACT_R, 9),
            (ACT_R, 5, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 12),
            (ACT_F, 8, ACT_R, 3),
            (ACT_L, 1, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 3, ACT_R, 12),
            (ACT_L, 8, ACT_R, 9),
            (ACT_F, 11, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 12, ACT_L, 5),
            (ACT_R, 5, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_R, 12, ACT_R, 6),
            (ACT_L, 3, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_F, 12, ACT_R, 6),
            (ACT_R, 11, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 12),
            (ACT_R, 2, ACT_R, 6),
            (ACT_R, 1, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 12, ACT_R, 9),
            (ACT_L, 11, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 12, ACT_L, 5),
            (ACT_L, 2, ACT_R, 9),
            (ACT_L, 11, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 12),
            (ACT_L, 7, ACT_R, 3),
            (ACT_F, 6, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 2, ACT_R, 11),
            (ACT_R, 12, ACT_R, 2),
            (ACT_F, 10, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 1, ACT_L, 11),
            (ACT_L, 10, ACT_L, 12),
            (ACT_F, 12, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 2, ACT_R, 1),
            (ACT_L, 10, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_L, 11),
            (ACT_R, 1, ACT_R, 3),
            (ACT_L, 8, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 11, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 1, ACT_L, 12),
            (ACT_F, 0, ACT_R, 5),
            (ACT_R, 8, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 3),
            (ACT_R, 12, ACT_R, 6),
            (ACT_L, 11, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_R, 11),
            (ACT_R, 9, ACT_L, 5),
            (ACT_R, 8, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 3),
            (ACT_R, 12, ACT_R, 6),
            (ACT_L, 3, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 12),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 2, ACT_R, 9),
            (ACT_L, 7, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_L, 7, ACT_R, 12),
            (ACT_L, 9, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 9, ACT_L, 2),
            (ACT_R, 7, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_F, 9, ACT_R, 7),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 6),
            (ACT_F, 4, ACT_R, 3),
            (ACT_L, 7, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 12),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 1, ACT_R, 9),
            (ACT_F, 0, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_F, 0, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_L, 12),
            (ACT_L, 2, ACT_L, 3),
            (ACT_F, 9, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_F, 0, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_R, 11),
            (ACT_R, 5, ACT_R, 9),
            (ACT_R, 7, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 11),
            (ACT_F, 10, ACT_L, 12),
            (ACT_L, 4, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 11),
            (ACT_F, 0, ACT_L, 11),
            (ACT_R, 10, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 12),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 0),
            (ACT_F, 8, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_L, 12, ACT_L, 10),
            (ACT_L, 9, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 11, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 5),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_L, 5),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 0, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 8, ACT_R, 2),
            (ACT_L, 10, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 12),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 3, ACT_R, 11),
            (ACT_R, 1, ACT_L, 1),
            (ACT_F, 0, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_R, 10),
            (ACT_L, 10, ACT_L, 8),
            (ACT_R, 0, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 12),
            (ACT_F, 2, ACT_L, 4),
            (ACT_F, 1, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_L, 11),
            (ACT_L, 6, ACT_L, 9),
            (ACT_R, 2, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 5),
            (ACT_R, 11, ACT_L, 12),
            (ACT_R, 2, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 12),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 11),
            (ACT_L, 5, ACT_L, 0),
            (ACT_L, 5, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 8),
            (ACT_R, 12, ACT_L, 7),
            (ACT_L, 3, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 5, ACT_R, 11),
            (ACT_R, 5, ACT_R, 12),
            (ACT_R, 10, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 12),
            (ACT_L, 6, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 11),
            (ACT_L, 3, ACT_L, 0),
            (ACT_R, 8, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 11, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 2, ACT_L, 5),
            (ACT_F, 0, ACT_R, 6),
            (ACT_R, 10, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 2, ACT_L, 11),
            (ACT_R, 12, ACT_R, 2),
            (ACT_L, 8, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 12),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 8, ACT_R, 0),
            (ACT_R, 2, ACT_L, 10),
            (ACT_R, 11, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 12),
            (ACT_R, 4, ACT_L, 4),
            (ACT_R, 5, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 12),
            (ACT_L, 5, ACT_L, 11),
            (ACT_L, 5, ACT_L, 12),
            (ACT_F, 11, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 2),
            (ACT_R, 12, ACT_R, 2),
            (ACT_R, 5, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 11, ACT_L, 5),
            (ACT_F, 11, ACT_L, 12),
            (ACT_F, 12, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 11),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_R, 2),
            (ACT_R, 7, ACT_L, 10),
            (ACT_L, 7, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_L, 5),
            (ACT_R, 11, ACT_L, 12),
            (ACT_R, 1, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 7, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_R, 11, ACT_R, 5),
            (ACT_R, 12, ACT_R, 2),
            (ACT_F, 7, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_L, 12),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 7, ACT_L, 11),
            (ACT_F, 12, ACT_L, 4),
            (ACT_R, 2, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 2, ACT_R, 2),
            (ACT_F, 3, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_F, 12, ACT_L, 9),
            (ACT_R, 0, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 11, ACT_R, 10),
            (ACT_F, 12, ACT_L, 3),
            (ACT_R, 11, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_L, 11, ACT_L, 5),
            (ACT_L, 5, ACT_L, 6),
            (ACT_L, 6, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 1, ACT_L, 3),
            (ACT_F, 2, ACT_R, 1),
            (ACT_R, 4, ACT_L, 1),
            (ACT_L, 8, ACT_L, 8),
            (ACT_F, 0, ACT_R, 6),
            (ACT_F, 9, ACT_R, 4),
            (ACT_R, 9, ACT_R, 7),
            (ACT_L, 5, ACT_R, 10),
            (ACT_F, 8, ACT_L, 4),
            (ACT_L, 5, ACT_L, 5),
            (ACT_F, 12, ACT_L, 11),
            (ACT_F, 2, ACT_L, 4),
            (ACT_F, 1, ACT_L, 6),
        ],
    },
];

const TEMPLATE_LIBRARY_C_MIX: &[TemplateDef] = &[
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 10, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 9),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 6),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 6, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_R, 5),
            (ACT_R, 4, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 3, ACT_L, 10),
            (ACT_L, 8, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 5, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 12, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 11, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 8, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_R, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 5, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_R, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_R, 1, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 6, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 6, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 7, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 4),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 11, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 7, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 7, ACT_L, 10),
            (ACT_L, 0, ACT_L, 12),
            (ACT_F, 6, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 0),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 12),
            (ACT_L, 0, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 8),
            (ACT_R, 12, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 2, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 5, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 12),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 12),
            (ACT_R, 10, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 3, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 5, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 1, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 8, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 10, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 8, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 12),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 1, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 12, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 11),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 0, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_R, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 11, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 3, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 0, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_F, 0, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_L, 3),
            (ACT_R, 4, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 11, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 1, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 10, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 6, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 1, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 11),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 6, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 11, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 8, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 9),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 4, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_L, 8),
            (ACT_R, 11, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 0, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 11, ACT_R, 5),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 1, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_R, 10, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 2),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_R, 6, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 2, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 11, ACT_R, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 6, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 4, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 6, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_F, 8, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 8, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 0, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_R, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 8, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_L, 2, ACT_R, 4),
            (ACT_R, 9, ACT_L, 6),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_L, 6, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_L, 6),
            (ACT_R, 11, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 11, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 8, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 6, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 7, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 3, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 1, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 11, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 2, ACT_R, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_F, 6, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 11, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 11, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 11, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 11, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_F, 4, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_L, 7, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 5, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 7, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 11),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 9),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 5, ACT_R, 6),
            (ACT_R, 7, ACT_L, 10),
            (ACT_R, 2, ACT_L, 4),
            (ACT_L, 11, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_F, 3, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 4, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_L, 11, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 6, ACT_L, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_R, 11, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_F, 3, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_R, 4),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 3, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 11, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 5),
            (ACT_F, 6, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 2, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_R, 9, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 2, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 11, ACT_R, 6),
            (ACT_L, 9, ACT_L, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 4, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 6, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 11),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 9, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 9, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 10, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 2),
            (ACT_R, 9, ACT_L, 11),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 9, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 11, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_F, 3, ACT_R, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_F, 9, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_F, 2, ACT_R, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 10),
            (ACT_R, 11, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 4),
            (ACT_L, 7, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_L, 2),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 11),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_F, 11, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 5),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 11),
            (ACT_R, 9, ACT_R, 9),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 4, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 11),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_R, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_F, 8, ACT_L, 7),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_L, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 5),
            (ACT_R, 9, ACT_R, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_L, 6, ACT_L, 2),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 3, ACT_L, 10),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 9, ACT_R, 6),
            (ACT_R, 9, ACT_L, 0),
            (ACT_L, 10, ACT_L, 1),
            (ACT_R, 1, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 11, ACT_R, 4),
            (ACT_F, 4, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 3, ACT_L, 9),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 8),
            (ACT_L, 1, ACT_L, 8),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 6, ACT_R, 11),
            (ACT_R, 2, ACT_L, 3),
        ],
    },
    TemplateDef {
        m: 12,
        rules: &[
            (ACT_L, 1, ACT_R, 8),
            (ACT_F, 3, ACT_L, 11),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 1),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_L, 10, ACT_L, 9),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 11, ACT_R, 10),
            (ACT_L, 11, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 13,
        rules: &[
            (ACT_L, 10, ACT_R, 8),
            (ACT_F, 3, ACT_L, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 1),
            (ACT_F, 6, ACT_R, 10),
            (ACT_L, 1, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 4, ACT_R, 11),
            (ACT_F, 4, ACT_L, 7),
            (ACT_R, 8, ACT_R, 8),
            (ACT_R, 4, ACT_L, 12),
            (ACT_F, 5, ACT_R, 8),
            (ACT_L, 7, ACT_L, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_L, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 13, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 3, ACT_L, 12),
            (ACT_R, 12, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 13),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_R, 10),
            (ACT_L, 0, ACT_L, 6),
            (ACT_R, 9, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 10),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_L, 9),
            (ACT_F, 6, ACT_R, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 12),
            (ACT_L, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 1, ACT_R, 11),
            (ACT_R, 5, ACT_R, 2),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 0, ACT_L, 6),
            (ACT_F, 7, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 0),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 9, ACT_L, 9),
            (ACT_F, 7, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 3, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_R, 2, ACT_L, 1),
            (ACT_L, 7, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 7),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 8, ACT_R, 13),
            (ACT_F, 5, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 12, ACT_R, 10),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 13, ACT_L, 10),
            (ACT_L, 11, ACT_L, 8),
            (ACT_R, 4, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 5, ACT_L, 10),
            (ACT_F, 13, ACT_R, 4),
            (ACT_F, 0, ACT_R, 13),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 13),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 11),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 12, ACT_L, 1),
            (ACT_F, 12, ACT_L, 11),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 12, ACT_L, 10),
            (ACT_L, 0, ACT_L, 13),
            (ACT_L, 1, ACT_L, 9),
            (ACT_R, 11, ACT_R, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 13),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_R, 2),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 5, ACT_L, 9),
            (ACT_F, 10, ACT_L, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 13, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 0, ACT_R, 9),
            (ACT_F, 3, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 2, ACT_R, 13),
            (ACT_R, 13, ACT_R, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_R, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 13, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 5, ACT_R, 4),
            (ACT_F, 2, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 12),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 13),
            (ACT_R, 0, ACT_L, 10),
            (ACT_L, 8, ACT_R, 7),
            (ACT_F, 5, ACT_L, 4),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 3, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 13, ACT_R, 4),
            (ACT_R, 6, ACT_R, 6),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 12, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 13),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 5, ACT_L, 3),
            (ACT_L, 8, ACT_R, 3),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 12, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_L, 5, ACT_L, 10),
            (ACT_L, 13, ACT_R, 10),
            (ACT_L, 8, ACT_R, 11),
            (ACT_F, 6, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 13),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 7, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 6, ACT_L, 4),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 11, ACT_L, 6),
            (ACT_F, 6, ACT_R, 12),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 0),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 9, ACT_R, 9),
            (ACT_R, 2, ACT_L, 1),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_L, 13),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 13, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_F, 9, ACT_R, 9),
            (ACT_L, 11, ACT_R, 9),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 13),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_R, 12, ACT_L, 1),
            (ACT_L, 10, ACT_L, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_L, 12, ACT_L, 10),
            (ACT_L, 13, ACT_L, 6),
            (ACT_L, 12, ACT_L, 2),
            (ACT_R, 4, ACT_R, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 10, ACT_R, 10),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 4, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 13),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 11, ACT_L, 12),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 8, ACT_R, 4),
            (ACT_F, 5, ACT_R, 10),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 13),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 0),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 12, ACT_L, 10),
            (ACT_R, 2, ACT_L, 1),
            (ACT_F, 10, ACT_R, 0),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_L, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_L, 6, ACT_R, 13),
            (ACT_L, 7, ACT_L, 5),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_L, 10),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 2),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 13, ACT_L, 12),
            (ACT_R, 9, ACT_L, 1),
            (ACT_F, 5, ACT_L, 10),
            (ACT_L, 0, ACT_L, 10),
            (ACT_F, 1, ACT_R, 11),
            (ACT_L, 1, ACT_L, 8),
        ],
    },
    TemplateDef {
        m: 14,
        rules: &[
            (ACT_L, 8, ACT_R, 6),
            (ACT_F, 3, ACT_R, 1),
            (ACT_F, 5, ACT_L, 2),
            (ACT_R, 2, ACT_R, 9),
            (ACT_F, 6, ACT_R, 12),
            (ACT_L, 1, ACT_R, 7),
            (ACT_F, 6, ACT_R, 2),
            (ACT_R, 4, ACT_R, 6),
            (ACT_F, 10, ACT_L, 11),
            (ACT_R, 9, ACT_L, 1),
            (ACT_R, 5, ACT_L, 0),
            (ACT_L, 12, ACT_R, 4),
            (ACT_L, 13, ACT_R, 12),
            (ACT_R, 8, ACT_L, 11),
        ],
    },
];

fn parse_input() -> Input {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let n: usize = it.next().unwrap().parse().unwrap();
    let ak: i64 = it.next().unwrap().parse().unwrap();
    let am: i64 = it.next().unwrap().parse().unwrap();
    let aw: i64 = it.next().unwrap().parse().unwrap();
    let mut wall_v = vec![vec![0u8; n - 1]; n];
    for row in wall_v.iter_mut().take(n) {
        let line = it.next().unwrap().as_bytes();
        for (j, v) in row.iter_mut().enumerate().take(n - 1) {
            *v = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    let mut wall_h = vec![vec![0u8; n]; n - 1];
    for row in wall_h.iter_mut().take(n - 1) {
        let line = it.next().unwrap().as_bytes();
        for (j, v) in row.iter_mut().enumerate().take(n) {
            *v = if line[j] == b'1' { 1 } else { 0 };
        }
    }
    Input {
        n,
        ak,
        am,
        aw,
        wall_v,
        wall_h,
    }
}

fn seed_from_input(input: &Input) -> u64 {
    let mut h = 1469598103934665603u64;
    let mut mix = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(1099511628211);
    };
    mix(input.n as u64);
    mix(input.ak as u64);
    mix(input.am as u64);
    mix(input.aw as u64);
    for i in 0..input.n {
        for j in 0..input.n - 1 {
            mix(input.wall_v[i][j] as u64);
        }
    }
    for i in 0..input.n - 1 {
        for j in 0..input.n {
            mix(input.wall_h[i][j] as u64);
        }
    }
    h
}

fn has_wall(input: &Input, i: usize, j: usize, d: usize) -> bool {
    let ni = i as isize + DIJ[d].0;
    let nj = j as isize + DIJ[d].1;
    if ni < 0 || ni >= input.n as isize || nj < 0 || nj >= input.n as isize {
        return true;
    }
    let ni = ni as usize;
    let nj = nj as usize;
    if ni == i {
        input.wall_v[i][j.min(nj)] == 1
    } else {
        input.wall_h[i.min(ni)][j] == 1
    }
}

fn build_graph(input: &Input) -> Vec<Vec<(usize, usize)>> {
    let mut g = vec![Vec::<(usize, usize)>::new(); N2];
    for i in 0..input.n {
        for j in 0..input.n {
            let v = i * input.n + j;
            for d in 0..4 {
                if !has_wall(input, i, j, d) {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    g[v].push((d, ni * input.n + nj));
                }
            }
        }
    }
    g
}

fn precompute_shortest(
    g: &[Vec<(usize, usize)>],
) -> (Vec<[u16; N2]>, Vec<[u8; N2]>, Vec<[u16; 4]>) {
    let mut dist = vec![[u16::MAX; N2]; N2];
    let mut first_dir = vec![[255u8; N2]; N2];
    let mut next_by_dir = vec![[u16::MAX; 4]; N2];
    for v in 0..N2 {
        for &(d, to) in &g[v] {
            next_by_dir[v][d] = to as u16;
        }
    }
    let mut q = VecDeque::<usize>::new();
    for s in 0..N2 {
        q.clear();
        dist[s][s] = 0;
        first_dir[s][s] = 0;
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            let dv = dist[s][v];
            for &(d, to) in &g[v] {
                if dist[s][to] == u16::MAX {
                    dist[s][to] = dv + 1;
                    first_dir[s][to] = if v == s { d as u8 } else { first_dir[s][v] };
                    q.push_back(to);
                }
            }
        }
    }
    (dist, first_dir, next_by_dir)
}

fn tour_cost(order: &[usize], dist: &[[u16; N2]]) -> i64 {
    let mut c = 0i64;
    for i in 0..N2 {
        let a = order[i];
        let b = order[(i + 1) % N2];
        c += dist[a][b] as i64;
    }
    c
}

fn two_opt_once(order: &mut [usize], dist: &[[u16; N2]], rng: &mut XorShift64) -> bool {
    let n = order.len();
    let offset = rng.gen_usize(n);
    for t in 0..n {
        let l = (offset + t) % n;
        let a = order[(l + n - 1) % n];
        let b = order[l];
        let mut r = l + 2;
        while r + 1 < n {
            let c = order[r];
            let d = order[(r + 1) % n];
            let old = dist[a][b] as i64 + dist[c][d] as i64;
            let newv = dist[a][c] as i64 + dist[b][d] as i64;
            if newv < old {
                order[l..=r].reverse();
                return true;
            }
            r += 1;
        }
    }
    false
}

fn or_opt_once(order: &mut Vec<usize>, dist: &[[u16; N2]], rng: &mut XorShift64) -> bool {
    let n = order.len();
    if n <= 6 {
        return false;
    }
    let len = 1 + rng.gen_usize(3).min(n - 2);
    let l = rng.gen_usize(n - len + 1);
    let mut ins = rng.gen_usize(n - len + 1);
    if ins == l {
        return false;
    }
    if ins > l && ins < l + len {
        ins = l + len;
    }

    let old = tour_cost(order, dist);
    let mut cand = order.clone();
    let seg: Vec<usize> = cand[l..l + len].to_vec();
    cand.drain(l..l + len);
    let pos = if ins > l { ins - len } else { ins };
    for (k, &x) in seg.iter().enumerate() {
        cand.insert(pos + k, x);
    }
    let newv = tour_cost(&cand, dist);
    if newv < old {
        *order = cand;
        true
    } else {
        false
    }
}

fn kick_order(order: &mut [usize], rng: &mut XorShift64) {
    let n = order.len();
    if n < 8 {
        return;
    }
    if rng.gen_usize(100) < 70 {
        let l = rng.gen_usize(n - 3);
        let max_add = (n - 1 - l).min(48);
        let r = l + 1 + rng.gen_usize(max_add);
        order[l..=r].reverse();
    } else {
        let mut idx = [
            rng.gen_usize(n),
            rng.gen_usize(n),
            rng.gen_usize(n),
            rng.gen_usize(n),
        ];
        idx.sort_unstable();
        if idx[0] + 2 < idx[1] && idx[1] + 2 < idx[2] && idx[2] + 2 < idx[3] {
            order[idx[0]..idx[1]].reverse();
            order[idx[2]..idx[3]].reverse();
        }
    }
}

fn row_snake() -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(N2);
    for i in 0..N_FIXED {
        if i % 2 == 0 {
            for j in 0..N_FIXED {
                order.push(i * N_FIXED + j);
            }
        } else {
            for j in (0..N_FIXED).rev() {
                order.push(i * N_FIXED + j);
            }
        }
    }
    order
}

fn col_snake() -> Vec<usize> {
    let mut order = Vec::<usize>::with_capacity(N2);
    for j in 0..N_FIXED {
        if j % 2 == 0 {
            for i in 0..N_FIXED {
                order.push(i * N_FIXED + j);
            }
        } else {
            for i in (0..N_FIXED).rev() {
                order.push(i * N_FIXED + j);
            }
        }
    }
    order
}

fn nearest_neighbor_tour(start: usize, dist: &[[u16; N2]]) -> Vec<usize> {
    let mut used = [false; N2];
    let mut order = Vec::<usize>::with_capacity(N2);
    let mut cur = start;
    used[cur] = true;
    order.push(cur);
    for _ in 1..N2 {
        let mut best = usize::MAX;
        let mut best_d = u16::MAX;
        for (v, &used_v) in used.iter().enumerate() {
            if !used_v {
                let d = dist[cur][v];
                if d < best_d {
                    best_d = d;
                    best = v;
                }
            }
        }
        cur = best;
        used[cur] = true;
        order.push(cur);
    }
    order
}

fn reconstruct_moves(order: &[usize], first_dir: &[[u8; N2]], next_by_dir: &[[u16; 4]]) -> Vec<u8> {
    let mut moves = Vec::<u8>::new();
    for i in 0..N2 {
        let mut cur = order[i];
        let goal = order[(i + 1) % N2];
        while cur != goal {
            let d = first_dir[cur][goal] as usize;
            moves.push(d as u8);
            cur = next_by_dir[cur][d] as usize;
        }
    }
    moves
}

fn rot_cost(from: usize, to: usize) -> usize {
    let diff = (to + 4 - from) % 4;
    match diff {
        0 => 0,
        1 | 3 => 1,
        2 => 2,
        _ => unreachable!(),
    }
}

fn append_turns(cur: &mut usize, to: usize, actions: &mut Vec<u8>) {
    let diff = (to + 4 - *cur) % 4;
    match diff {
        0 => {}
        1 => {
            actions.push(ACT_R);
            *cur = (*cur + 1) % 4;
        }
        2 => {
            actions.push(ACT_R);
            actions.push(ACT_R);
            *cur = (*cur + 2) % 4;
        }
        3 => {
            actions.push(ACT_L);
            *cur = (*cur + 3) % 4;
        }
        _ => unreachable!(),
    }
}

fn build_route_by_order(
    order: &[usize],
    first_dir: &[[u8; N2]],
    next_by_dir: &[[u16; 4]],
) -> Option<RouteCandidate> {
    let moves = reconstruct_moves(order, first_dir, next_by_dir);
    let mut best_len = usize::MAX;
    let mut best_sd = 0usize;
    for sd in 0..4 {
        let mut turns = 0usize;
        let mut cur = sd;
        for &md in &moves {
            turns += rot_cost(cur, md as usize);
            cur = md as usize;
        }
        turns += rot_cost(cur, sd);
        let len = moves.len() + turns;
        if len < best_len {
            best_len = len;
            best_sd = sd;
        }
    }
    if best_len == usize::MAX || best_len > M_LIMIT {
        return None;
    }
    let mut actions = Vec::<u8>::with_capacity(best_len);
    let mut cur = best_sd;
    for &md in &moves {
        append_turns(&mut cur, md as usize, &mut actions);
        actions.push(ACT_F);
        cur = md as usize;
    }
    append_turns(&mut cur, best_sd, &mut actions);
    if actions.is_empty() || actions.len() > M_LIMIT {
        return None;
    }
    Some(RouteCandidate {
        start_cell: order[0],
        start_dir: best_sd,
        actions,
    })
}

fn evaluate_order_with_shifts(
    order: &[usize],
    first_dir: &[[u8; N2]],
    next_by_dir: &[[u16; 4]],
    rng: &mut XorShift64,
) -> Option<RouteCandidate> {
    let mut shifts = Vec::<usize>::new();
    shifts.push(0);
    shifts.push(N2 / 4);
    shifts.push(N2 / 2);
    shifts.push(N2 * 3 / 4);
    for _ in 0..EVAL_EXTRA_SHIFTS {
        shifts.push(rng.gen_usize(N2));
    }
    shifts.sort_unstable();
    shifts.dedup();

    let mut best_route: Option<RouteCandidate> = None;
    let mut best_m = usize::MAX;
    for &sh in &shifts {
        let mut rotated = Vec::<usize>::with_capacity(N2);
        rotated.extend_from_slice(&order[sh..]);
        rotated.extend_from_slice(&order[..sh]);
        if let Some(route) = build_route_by_order(&rotated, first_dir, next_by_dir) {
            let m = route.actions.len();
            if m < best_m {
                best_m = m;
                best_route = Some(route);
            }
        }
    }
    best_route
}

fn best_route(input: &Input) -> Option<RouteCandidate> {
    let g = build_graph(input);
    let (dist, first_dir, next_by_dir) = precompute_shortest(&g);
    let start_time = Instant::now();
    let limit_ms = std::env::var("SEARCH_TIME_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEARCH_TIME_MS);
    let limit = Duration::from_millis(limit_ms);
    let mut rng = XorShift64::new(seed_from_input(input) ^ 0xA24BAED4963EE407);

    let mut initials = Vec::<Vec<usize>>::new();
    let row = row_snake();
    let mut rev_row = row.clone();
    rev_row.reverse();
    let col = col_snake();
    let mut rev_col = col.clone();
    rev_col.reverse();
    initials.push(row);
    initials.push(rev_row);
    initials.push(col);
    initials.push(rev_col);
    for _ in 0..INITIAL_NN_TOURS {
        initials.push(nearest_neighbor_tour(rng.gen_usize(N2), &dist));
    }
    let base_orders = initials.clone();

    let mut best_order = initials[0].clone();
    let mut best_cost = tour_cost(&best_order, &dist);
    let mut best_route: Option<RouteCandidate> = None;
    let mut best_m = usize::MAX;
    let mut best_route_cost = i64::MAX;

    for init in initials {
        if start_time.elapsed() >= limit {
            break;
        }
        let mut order = init;
        let mut cost = tour_cost(&order, &dist);
        if cost < best_cost {
            best_cost = cost;
            best_order = order.clone();
        }
        if let Some(route) = evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng)
        {
            let m = route.actions.len();
            if m < best_m || (m == best_m && cost < best_route_cost) {
                best_m = m;
                best_route_cost = cost;
                best_route = Some(route);
                best_order = order.clone();
                best_cost = cost;
            }
        }

        let mut stagnation = 0usize;
        let mut improve_cnt = 0usize;
        while start_time.elapsed() < limit && stagnation < 64 {
            let improved = if rng.gen_usize(100) < 78 {
                two_opt_once(&mut order[..], &dist, &mut rng)
            } else {
                or_opt_once(&mut order, &dist, &mut rng)
            };
            if improved {
                stagnation = 0;
                improve_cnt += 1;
                cost = tour_cost(&order, &dist);
                if cost < best_cost {
                    best_cost = cost;
                    best_order = order.clone();
                }
                if improve_cnt % 10 == 0 {
                    if let Some(route) =
                        evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng)
                    {
                        let m = route.actions.len();
                        if m < best_m || (m == best_m && cost < best_route_cost) {
                            best_m = m;
                            best_route_cost = cost;
                            best_route = Some(route);
                            best_order = order.clone();
                            best_cost = cost;
                        }
                    }
                }
            } else {
                stagnation += 1;
            }
        }
        if let Some(route) = evaluate_order_with_shifts(&order, &first_dir, &next_by_dir, &mut rng)
        {
            let m = route.actions.len();
            if m < best_m || (m == best_m && cost < best_route_cost) {
                best_m = m;
                best_route_cost = cost;
                best_route = Some(route);
                best_order = order.clone();
                best_cost = cost;
            }
        }
    }

    // キック + 局所探索を繰り返し、m最小を直接追う
    let mut work = best_order.clone();
    let mut work_cost = tour_cost(&work, &dist);
    while start_time.elapsed() < limit {
        kick_order(&mut work[..], &mut rng);
        let mut no_imp = 0usize;
        while start_time.elapsed() < limit && no_imp < 80 {
            let improved = if rng.gen_usize(100) < 72 {
                two_opt_once(&mut work[..], &dist, &mut rng)
            } else {
                or_opt_once(&mut work, &dist, &mut rng)
            };
            if improved {
                work_cost = tour_cost(&work, &dist);
                no_imp = 0;
            } else {
                no_imp += 1;
            }
        }

        if let Some(route) = evaluate_order_with_shifts(&work, &first_dir, &next_by_dir, &mut rng) {
            let m = route.actions.len();
            if m < best_m || (m == best_m && work_cost < best_route_cost) {
                best_m = m;
                best_route_cost = work_cost;
                best_route = Some(route);
                best_order = work.clone();
            }
        }

        if work_cost > best_route_cost + 120 {
            work = best_order.clone();
            work_cost = best_route_cost;
        }
    }

    if best_route.is_none() {
        if let Some(route) =
            evaluate_order_with_shifts(&best_order, &first_dir, &next_by_dir, &mut rng)
                .or_else(|| build_route_by_order(&best_order, &first_dir, &next_by_dir))
        {
            best_route = Some(route);
        }
    }
    if best_route.is_none() {
        for order in &base_orders {
            if let Some(route) =
                evaluate_order_with_shifts(order, &first_dir, &next_by_dir, &mut rng)
                    .or_else(|| build_route_by_order(order, &first_dir, &next_by_dir))
            {
                best_route = Some(route);
                break;
            }
        }
    }
    best_route
}

fn rotate_dir(dir: usize, act: u8) -> usize {
    match act {
        ACT_R => (dir + 1) % 4,
        ACT_L => (dir + 3) % 4,
        ACT_F => dir,
        _ => unreachable!(),
    }
}

fn edge_slot_from_cell_dir(cell: usize, dir: usize, n: usize) -> Option<(bool, usize, usize)> {
    let i = cell / n;
    let j = cell % n;
    match dir {
        0 => {
            if i == 0 {
                None
            } else {
                Some((false, i - 1, j))
            }
        }
        1 => {
            if j + 1 >= n {
                None
            } else {
                Some((true, i, j))
            }
        }
        2 => {
            if i + 1 >= n {
                None
            } else {
                Some((false, i, j))
            }
        }
        3 => {
            if j == 0 {
                None
            } else {
                Some((true, i, j - 1))
            }
        }
        _ => unreachable!(),
    }
}

fn add_wall_if_new(
    wall_v_add: &mut [Vec<u8>],
    wall_h_add: &mut [Vec<u8>],
    is_v: bool,
    i: usize,
    j: usize,
) {
    if is_v {
        wall_v_add[i][j] = 1;
    } else {
        wall_h_add[i][j] = 1;
    }
}

fn has_wall_with_add(
    input: &Input,
    wall_v_add: &[Vec<u8>],
    wall_h_add: &[Vec<u8>],
    cell: usize,
    dir: usize,
) -> bool {
    let i = cell / input.n;
    let j = cell % input.n;
    if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(cell, dir, input.n) {
        if is_v {
            input.wall_v[ei][ej] == 1 || wall_v_add[ei][ej] == 1
        } else {
            input.wall_h[ei][ej] == 1 || wall_h_add[ei][ej] == 1
        }
    } else {
        let _ = (i, j);
        true
    }
}

fn apply_action(input: &Input, cell: usize, dir: usize, act: u8) -> (usize, usize) {
    match act {
        ACT_R | ACT_L => (cell, rotate_dir(dir, act)),
        ACT_F => {
            let i = cell / input.n;
            let j = cell % input.n;
            let ni = (i as isize + DIJ[dir].0) as usize;
            let nj = (j as isize + DIJ[dir].1) as usize;
            (ni * input.n + nj, dir)
        }
        _ => unreachable!(),
    }
}

fn rotate_route(route: &RouteCandidate, input: &Input, offset: usize) -> Option<RouteCandidate> {
    let m = route.actions.len();
    if m == 0 {
        return None;
    }
    let offset = offset % m;
    let mut cell = route.start_cell;
    let mut dir = route.start_dir;
    for t in 0..offset {
        let act = route.actions[t];
        if act == ACT_F && has_wall(input, cell / input.n, cell % input.n, dir) {
            return None;
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        cell = nc;
        dir = nd;
    }
    let mut actions = Vec::<u8>::with_capacity(m);
    actions.extend_from_slice(&route.actions[offset..]);
    actions.extend_from_slice(&route.actions[..offset]);
    Some(RouteCandidate {
        start_cell: cell,
        start_dir: dir,
        actions,
    })
}

fn compute_plan_value(input: &Input, m: usize, w_count: usize) -> i64 {
    input.am * m as i64 + input.aw * w_count as i64
}

fn simulate_plan_cover_all(input: &Input, plan: &RobotPlan) -> bool {
    let m = plan.states.len();
    if m == 0 || m > M_LIMIT {
        return false;
    }
    let total = N2 * 4 * m;
    let mut seen = vec![-1i32; total];
    let mut path = Vec::<(usize, usize, usize)>::new();

    let mut cell = plan.start_cell;
    let mut dir = plan.start_dir;
    let mut st = 0usize;

    let cycle_start: usize = loop {
        if st >= m {
            return false;
        }
        let idx = (cell * 4 + dir) * m + st;
        if seen[idx] >= 0 {
            break seen[idx] as usize;
        }
        seen[idx] = path.len() as i32;
        path.push((cell, dir, st));

        let s = plan.states[st];
        let wall = has_wall_with_add(input, &plan.wall_v_add, &plan.wall_h_add, cell, dir);
        let (act, ns) = if wall { (s.a1, s.b1) } else { (s.a0, s.b0) };
        if wall && act == ACT_F {
            return false;
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        cell = nc;
        dir = nd;
        st = ns;
    };

    let mut cover = [false; N2];
    for &(c, _, _) in path.iter().skip(cycle_start) {
        cover[c] = true;
    }
    cover.into_iter().all(|x| x)
}

fn build_explicit_plan(input: &Input, route: &RouteCandidate) -> Option<RobotPlan> {
    let m = route.actions.len();
    if m == 0 || m > M_LIMIT {
        return None;
    }
    let mut states = Vec::<AutoState>::with_capacity(m);
    for s in 0..m {
        let act = route.actions[s];
        let ns = (s + 1) % m;
        states.push(AutoState {
            a0: act,
            b0: ns,
            a1: if act == ACT_F { ACT_R } else { act },
            b1: ns,
        });
    }
    let wall_v_add = vec![vec![0u8; input.n - 1]; input.n];
    let wall_h_add = vec![vec![0u8; input.n]; input.n - 1];
    let plan = RobotPlan {
        start_cell: route.start_cell,
        start_dir: route.start_dir,
        states,
        wall_v_add,
        wall_h_add,
        w_count: 0,
        value_v: compute_plan_value(input, m, 0),
    };
    if simulate_plan_cover_all(input, &plan) {
        Some(plan)
    } else {
        None
    }
}

#[derive(Clone)]
struct SegmentInfo {
    run_start: usize,
    run_end: usize,
    turn_end: usize,
    run_len: usize,
    existing_wall: bool,
    add_edge: Option<(bool, usize, usize)>,
}

fn select_added_edges_smart(
    input: &Input,
    segments: &[SegmentInfo],
) -> HashSet<(bool, usize, usize)> {
    let mut gain_by_edge = HashMap::<(bool, usize, usize), i64>::new();
    for seg in segments {
        if seg.existing_wall || seg.run_len <= 1 {
            continue;
        }
        if let Some(edge) = seg.add_edge {
            *gain_by_edge.entry(edge).or_insert(0) += (seg.run_len - 1) as i64;
        }
    }

    let mut chosen = HashSet::<(bool, usize, usize)>::new();
    for (edge, save_states) in gain_by_edge {
        // 壁1本で得られる状態削減の総和が、壁コストを上回るときのみ採用する。
        if input.aw - input.am * save_states < 0 {
            chosen.insert(edge);
        }
    }
    chosen
}

fn build_segment_plan_for_offset(
    input: &Input,
    base_route: &RouteCandidate,
    offset: usize,
    allow_add_walls: bool,
) -> Option<RobotPlan> {
    let route = rotate_route(base_route, input, offset)?;
    if route.actions.is_empty() || route.actions[0] != ACT_F {
        return None;
    }
    let m = route.actions.len();

    let mut pre_cell = vec![0usize; m];
    let mut pre_dir = vec![0usize; m];
    let mut post_cell = vec![0usize; m];
    let mut post_dir = vec![0usize; m];

    let mut used_v = vec![vec![false; input.n - 1]; input.n];
    let mut used_h = vec![vec![false; input.n]; input.n - 1];

    let mut cell = route.start_cell;
    let mut dir = route.start_dir;
    for t in 0..m {
        pre_cell[t] = cell;
        pre_dir[t] = dir;
        let act = route.actions[t];
        if act == ACT_F {
            if has_wall(input, cell / input.n, cell % input.n, dir) {
                return None;
            }
            if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(cell, dir, input.n) {
                if is_v {
                    used_v[ei][ej] = true;
                } else {
                    used_h[ei][ej] = true;
                }
            }
        }
        let (nc, nd) = apply_action(input, cell, dir, act);
        post_cell[t] = nc;
        post_dir[t] = nd;
        cell = nc;
        dir = nd;
    }
    if cell != route.start_cell || dir != route.start_dir {
        return None;
    }

    let mut segments = Vec::<SegmentInfo>::new();
    let mut i = 0usize;
    while i < m {
        if route.actions[i] != ACT_F {
            return None;
        }
        let run_start = i;
        while i < m && route.actions[i] == ACT_F {
            i += 1;
        }
        let run_end = i;
        let turn_start = i;
        while i < m && route.actions[i] != ACT_F {
            i += 1;
        }
        let turn_end = i;
        if turn_start == turn_end {
            return None;
        }

        let run_len = run_end - run_start;
        let end_cell = post_cell[run_end - 1];
        let end_dir = post_dir[run_end - 1];
        let existing_wall = has_wall(input, end_cell / input.n, end_cell % input.n, end_dir);
        let mut add_edge = None;
        if !existing_wall && allow_add_walls {
            if let Some((is_v, ei, ej)) = edge_slot_from_cell_dir(end_cell, end_dir, input.n) {
                let used = if is_v { used_v[ei][ej] } else { used_h[ei][ej] };
                if !used {
                    add_edge = Some((is_v, ei, ej));
                }
            }
        }

        segments.push(SegmentInfo {
            run_start,
            run_end,
            turn_end,
            run_len,
            existing_wall,
            add_edge,
        });
    }
    if segments.is_empty() {
        return None;
    }

    let chosen_add_edges = if allow_add_walls {
        select_added_edges_smart(input, &segments)
    } else {
        HashSet::new()
    };

    let mut states = Vec::<AutoState>::new();
    let seg_n = segments.len();
    let mut seg_start_state = vec![usize::MAX; seg_n];
    let mut patches = vec![Vec::<PatchRef>::new(); seg_n];

    for (si, seg) in segments.iter().enumerate() {
        seg_start_state[si] = states.len();
        let turns = &route.actions[seg.run_end..seg.turn_end];
        if turns.is_empty() {
            return None;
        }
        let compressible = seg.existing_wall
            || seg
                .add_edge
                .map(|edge| chosen_add_edges.contains(&edge))
                .unwrap_or(false);

        if compressible {
            let move_id = states.len();
            states.push(AutoState {
                a0: ACT_F,
                b0: move_id,
                a1: turns[0],
                b1: 0,
            });
            if turns.len() == 1 {
                patches[si].push(PatchRef {
                    state_id: move_id,
                    kind: PatchKind::B1Only,
                });
            } else {
                let mut prev = move_id;
                for (idx, &act) in turns.iter().enumerate().skip(1) {
                    let sid = states.len();
                    states.push(AutoState {
                        a0: act,
                        b0: 0,
                        a1: act,
                        b1: 0,
                    });
                    if idx == 1 {
                        states[move_id].b1 = sid;
                    } else {
                        states[prev].b0 = sid;
                        states[prev].b1 = sid;
                    }
                    prev = sid;
                }
                patches[si].push(PatchRef {
                    state_id: prev,
                    kind: PatchKind::Both,
                });
            }
        } else {
            let seq = &route.actions[seg.run_start..seg.turn_end];
            let base = states.len();
            for (k, &act) in seq.iter().enumerate() {
                let next = if k + 1 < seq.len() { base + k + 1 } else { 0 };
                let a1 = if act == ACT_F { ACT_R } else { act };
                states.push(AutoState {
                    a0: act,
                    b0: next,
                    a1,
                    b1: next,
                });
            }
            patches[si].push(PatchRef {
                state_id: states.len() - 1,
                kind: PatchKind::Both,
            });
        }
    }

    if states.is_empty() || states.len() > M_LIMIT {
        return None;
    }

    for si in 0..seg_n {
        let next_start = seg_start_state[(si + 1) % seg_n];
        for p in &patches[si] {
            match p.kind {
                PatchKind::B1Only => {
                    states[p.state_id].b1 = next_start;
                }
                PatchKind::Both => {
                    states[p.state_id].b0 = next_start;
                    states[p.state_id].b1 = next_start;
                }
            }
        }
    }

    let mut wall_v_add = vec![vec![0u8; input.n - 1]; input.n];
    let mut wall_h_add = vec![vec![0u8; input.n]; input.n - 1];
    for &(is_v, ei, ej) in &chosen_add_edges {
        add_wall_if_new(&mut wall_v_add, &mut wall_h_add, is_v, ei, ej);
    }
    let w_count = chosen_add_edges.len();

    let plan = RobotPlan {
        start_cell: route.start_cell,
        start_dir: route.start_dir,
        states,
        wall_v_add,
        wall_h_add,
        w_count,
        value_v: compute_plan_value(input, m.min(M_LIMIT), w_count),
    };
    let mut checked = plan;
    checked.value_v = compute_plan_value(input, checked.states.len(), checked.w_count);
    if simulate_plan_cover_all(input, &checked) {
        Some(checked)
    } else {
        None
    }
}

fn collect_offset_candidates(actions: &[u8], seed: u64) -> Vec<usize> {
    if actions.is_empty() {
        return vec![0];
    }
    let m = actions.len();
    let mut offsets = Vec::<usize>::new();
    for i in 0..m {
        let prev = if i == 0 {
            actions[m - 1]
        } else {
            actions[i - 1]
        };
        if actions[i] == ACT_F && prev != ACT_F {
            offsets.push(i);
        }
    }
    if offsets.is_empty() {
        for i in 0..m {
            if actions[i] == ACT_F {
                offsets.push(i);
                break;
            }
        }
    }
    offsets.push(0);
    offsets.sort_unstable();
    offsets.dedup();

    if offsets.len() <= OFFSET_TRIAL_LIMIT {
        return offsets;
    }
    let mut rng = XorShift64::new(seed ^ 0x9E3779B97F4A7C15);
    for i in (1..offsets.len()).rev() {
        let j = rng.gen_usize(i + 1);
        offsets.swap(i, j);
    }
    offsets.truncate(OFFSET_TRIAL_LIMIT);
    offsets.sort_unstable();
    offsets
}

fn better_plan(a: &RobotPlan, b: &RobotPlan) -> bool {
    let key_a = (a.value_v, a.states.len(), a.w_count);
    let key_b = (b.value_v, b.states.len(), b.w_count);
    key_a < key_b
}

fn choose_best_plan(input: &Input, route: &RouteCandidate) -> Option<RobotPlan> {
    let mut best = build_explicit_plan(input, route)?;
    let offsets = collect_offset_candidates(&route.actions, seed_from_input(input));
    // C相当 (AW が極端に重い) では壁探索を省く。Bレンジでは壁探索を有効化する。
    let try_wall_mode = input.aw <= input.am * 32;

    for &off in &offsets {
        if let Some(plan) = build_segment_plan_for_offset(input, route, off, false) {
            if better_plan(&plan, &best) {
                best = plan;
            }
        }
        if try_wall_mode {
            if let Some(plan) = build_segment_plan_for_offset(input, route, off, true) {
                if better_plan(&plan, &best) {
                    best = plan;
                }
            }
        }
    }
    Some(best)
}

fn build_template_env(input: &Input) -> TemplateEnv {
    let mut wall = vec![false; ORIENTS];
    let mut next_o = vec![[0usize; 3]; ORIENTS];
    for i in 0..input.n {
        for j in 0..input.n {
            let cell = i * input.n + j;
            for d in 0..4 {
                let o = cell * 4 + d;
                let w = has_wall(input, i, j, d);
                wall[o] = w;
                next_o[o][ACT_R as usize] = cell * 4 + (d + 1) % 4;
                next_o[o][ACT_L as usize] = cell * 4 + (d + 3) % 4;
                if w {
                    next_o[o][ACT_F as usize] = o;
                } else {
                    let ni = (i as isize + DIJ[d].0) as usize;
                    let nj = (j as isize + DIJ[d].1) as usize;
                    next_o[o][ACT_F as usize] = (ni * input.n + nj) * 4 + d;
                }
            }
        }
    }
    TemplateEnv { wall, next_o }
}

fn collect_from_template(env: &TemplateEnv, tpl: &TemplateDef, out: &mut Vec<TemplateChoice>) {
    let m = tpl.m;
    if m == 0 || m > M_LIMIT || tpl.rules.len() != m {
        return;
    }
    let mut a0 = Vec::<u8>::with_capacity(m);
    let mut b0 = Vec::<usize>::with_capacity(m);
    let mut a1 = Vec::<u8>::with_capacity(m);
    let mut b1 = Vec::<usize>::with_capacity(m);
    for &(x0, y0, x1, y1) in tpl.rules {
        a0.push(x0);
        b0.push(y0 as usize);
        a1.push(x1);
        b1.push(y1 as usize);
    }

    let total = ORIENTS * m;
    let mut next = vec![0usize; total];
    for o in 0..ORIENTS {
        let wall = env.wall[o];
        for s in 0..m {
            let act = if wall { a1[s] } else { a0[s] };
            let ns = if wall { b1[s] } else { b0[s] };
            let no = env.next_o[o][act as usize];
            next[o * m + s] = no * m + ns;
        }
    }

    let mut mark = vec![0u8; total];
    let mut comp = vec![usize::MAX; total];
    let mut cycles = Vec::<BitSet>::new();
    let mut stack = Vec::<usize>::new();

    for st in 0..total {
        if mark[st] != 0 {
            continue;
        }
        stack.clear();
        let mut u = st;
        while mark[u] == 0 {
            mark[u] = 1;
            stack.push(u);
            u = next[u];
        }

        let cid = if mark[u] == 1 {
            let pos = stack.iter().rposition(|&x| x == u).unwrap();
            let mut bits = BitSet::empty();
            for &x in &stack[pos..] {
                bits.set_cell((x / m) / 4);
            }
            let id = cycles.len();
            cycles.push(bits);
            for &x in &stack[pos..] {
                comp[x] = id;
            }
            id
        } else {
            comp[u]
        };

        for &x in &stack {
            if comp[x] == usize::MAX {
                comp[x] = cid;
            }
            mark[x] = 2;
        }
    }

    let mut local = HashMap::<BitSet, usize>::new();
    for o in 0..ORIENTS {
        let st0 = o * m;
        let bits = cycles[comp[st0]];
        local.entry(bits).or_insert(o);
    }

    for (cover, start_o) in local {
        out.push(TemplateChoice {
            m,
            start_cell: start_o / 4,
            start_dir: start_o % 4,
            a0: a0.clone(),
            b0: b0.clone(),
            a1: a1.clone(),
            b1: b1.clone(),
            cover,
        });
    }
}

fn collect_from_raw_automaton(
    env: &TemplateEnv,
    m: usize,
    a0: &[u8],
    b0: &[usize],
    a1: &[u8],
    b1: &[usize],
    out: &mut Vec<TemplateChoice>,
) {
    if m == 0 || m > M_LIMIT {
        return;
    }
    if a0.len() != m || b0.len() != m || a1.len() != m || b1.len() != m {
        return;
    }

    let total = ORIENTS * m;
    let mut next = vec![0usize; total];
    for o in 0..ORIENTS {
        let wall = env.wall[o];
        for s in 0..m {
            let act = if wall { a1[s] } else { a0[s] };
            let ns = if wall { b1[s] } else { b0[s] };
            let no = env.next_o[o][act as usize];
            next[o * m + s] = no * m + ns;
        }
    }

    let mut mark = vec![0u8; total];
    let mut comp = vec![usize::MAX; total];
    let mut cycles = Vec::<BitSet>::new();
    let mut stack = Vec::<usize>::new();

    for st in 0..total {
        if mark[st] != 0 {
            continue;
        }
        stack.clear();
        let mut u = st;
        while mark[u] == 0 {
            mark[u] = 1;
            stack.push(u);
            u = next[u];
        }

        let cid = if mark[u] == 1 {
            let pos = stack.iter().rposition(|&x| x == u).unwrap();
            let mut bits = BitSet::empty();
            for &x in &stack[pos..] {
                bits.set_cell((x / m) / 4);
            }
            let id = cycles.len();
            cycles.push(bits);
            for &x in &stack[pos..] {
                comp[x] = id;
            }
            id
        } else {
            comp[u]
        };

        for &x in &stack {
            if comp[x] == usize::MAX {
                comp[x] = cid;
            }
            mark[x] = 2;
        }
    }

    let mut local = HashMap::<BitSet, usize>::new();
    for o in 0..ORIENTS {
        let st0 = o * m;
        let bits = cycles[comp[st0]];
        local.entry(bits).or_insert(o);
    }

    for (cover, start_o) in local {
        out.push(TemplateChoice {
            m,
            start_cell: start_o / 4,
            start_dir: start_o % 4,
            a0: a0.to_vec(),
            b0: b0.to_vec(),
            a1: a1.to_vec(),
            b1: b1.to_vec(),
            cover,
        });
    }
}

fn mirror_lr_actions(src: &[u8]) -> Vec<u8> {
    src.iter()
        .map(|&a| match a {
            ACT_L => ACT_R,
            ACT_R => ACT_L,
            _ => a,
        })
        .collect()
}

fn add_teammate_template_candidates(env: &TemplateEnv, out: &mut Vec<TemplateChoice>) {
    let push = |m: usize,
                a0: Vec<u8>,
                b0: Vec<usize>,
                a1: Vec<u8>,
                b1: Vec<usize>,
                out: &mut Vec<TemplateChoice>| {
        collect_from_raw_automaton(env, m, &a0, &b0, &a1, &b1, out);
    };

    let down_a0 = vec![
        ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_L,
    ];
    let down_b0 = vec![0, 2, 3, 3, 5, 0, 6, 8, 9, 9, 11, 6];
    let down_a1 = vec![
        ACT_L, ACT_L, ACT_L, ACT_R, ACT_R, ACT_R, ACT_R, ACT_R, ACT_R, ACT_L, ACT_L, ACT_L,
    ];
    let down_b1 = vec![1, 9, 3, 4, 6, 0, 7, 3, 9, 10, 0, 6];
    push(
        12,
        down_a0.clone(),
        down_b0.clone(),
        down_a1.clone(),
        down_b1.clone(),
        out,
    );
    push(
        12,
        mirror_lr_actions(&down_a0),
        down_b0.clone(),
        mirror_lr_actions(&down_a1),
        down_b1.clone(),
        out,
    );

    let right_a0 = vec![
        ACT_F, ACT_F, ACT_R, ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_R,
    ];
    let right_b0 = vec![0, 2, 3, 3, 5, 0, 6, 8, 9, 9, 11, 6];
    let right_a1 = vec![
        ACT_R, ACT_L, ACT_R, ACT_L, ACT_R, ACT_L, ACT_L, ACT_R, ACT_L, ACT_R, ACT_L, ACT_R,
    ];
    let right_b1 = vec![1, 6, 3, 4, 9, 0, 7, 0, 9, 10, 3, 6];
    push(
        12,
        right_a0.clone(),
        right_b0.clone(),
        right_a1.clone(),
        right_b1.clone(),
        out,
    );
    push(
        12,
        mirror_lr_actions(&right_a0),
        right_b0.clone(),
        mirror_lr_actions(&right_a1),
        right_b1.clone(),
        out,
    );

    push(
        2,
        vec![ACT_F, ACT_R],
        vec![0, 0],
        vec![ACT_R, ACT_R],
        vec![1, 0],
        out,
    );
    push(
        4,
        vec![ACT_R, ACT_F, ACT_F, ACT_F],
        vec![1, 0, 0, 0],
        vec![ACT_R, ACT_L, ACT_L, ACT_L],
        vec![1, 2, 3, 3],
        out,
    );
    push(
        4,
        vec![ACT_F, ACT_F, ACT_F, ACT_F],
        vec![1, 0, 3, 2],
        vec![ACT_R, ACT_L, ACT_R, ACT_L],
        vec![2, 3, 0, 1],
        out,
    );
    push(
        6,
        vec![ACT_F, ACT_F, ACT_L, ACT_F, ACT_F, ACT_L],
        vec![0, 2, 3, 3, 5, 0],
        vec![ACT_L, ACT_L, ACT_L, ACT_L, ACT_L, ACT_L],
        vec![1, 3, 3, 4, 0, 0],
        out,
    );
    push(
        3,
        vec![ACT_F, ACT_L, ACT_R],
        vec![1, 2, 0],
        vec![ACT_R, ACT_R, ACT_R],
        vec![0, 0, 2],
        out,
    );
    push(
        2,
        vec![ACT_L, ACT_F],
        vec![1, 0],
        vec![ACT_R, ACT_L],
        vec![1, 1],
        out,
    );

    let extras = vec![
        (
            2,
            vec![ACT_F, ACT_F],
            vec![0, 1],
            vec![ACT_R, ACT_L],
            vec![1, 0],
        ),
        (
            2,
            vec![ACT_F, ACT_R],
            vec![0, 0],
            vec![ACT_L, ACT_R],
            vec![1, 0],
        ),
        (
            2,
            vec![ACT_F, ACT_L],
            vec![0, 0],
            vec![ACT_R, ACT_L],
            vec![1, 0],
        ),
        (
            3,
            vec![ACT_F, ACT_R, ACT_F],
            vec![1, 2, 0],
            vec![ACT_R, ACT_L, ACT_L],
            vec![2, 0, 1],
        ),
        (
            3,
            vec![ACT_F, ACT_L, ACT_F],
            vec![1, 2, 0],
            vec![ACT_L, ACT_R, ACT_R],
            vec![2, 0, 1],
        ),
        (
            4,
            vec![ACT_F, ACT_F, ACT_R, ACT_L],
            vec![1, 0, 3, 2],
            vec![ACT_R, ACT_R, ACT_L, ACT_L],
            vec![2, 3, 0, 1],
        ),
        (
            4,
            vec![ACT_F, ACT_F, ACT_L, ACT_R],
            vec![1, 0, 3, 2],
            vec![ACT_L, ACT_L, ACT_R, ACT_R],
            vec![2, 3, 0, 1],
        ),
        (
            3,
            vec![ACT_F, ACT_F, ACT_F],
            vec![0, 2, 1],
            vec![ACT_R, ACT_R, ACT_L],
            vec![1, 2, 0],
        ),
        (
            3,
            vec![ACT_F, ACT_L, ACT_R],
            vec![1, 2, 0],
            vec![ACT_R, ACT_L, ACT_R],
            vec![2, 0, 1],
        ),
    ];
    for (m, a0, b0, a1, b1) in extras {
        push(m, a0, b0, a1, b1, out);
    }
}

fn eval_template_energy(
    input: &Input,
    k: usize,
    m_sum: usize,
    uncovered: usize,
    penalty_unit: i64,
) -> i64 {
    input.ak * (k.saturating_sub(1) as i64)
        + input.am * (m_sum as i64)
        + penalty_unit * (uncovered as i64)
}

fn improve_template_selection_toggle(
    input: &Input,
    cands: &[TemplateChoice],
    selected: &mut Vec<usize>,
) {
    if selected.is_empty() || cands.is_empty() {
        return;
    }
    let mut in_sel = vec![false; cands.len()];
    let mut best_in_sel = vec![false; cands.len()];
    let mut cover_count = [0u16; N2];
    let mut k = 0usize;
    let mut m_sum = 0usize;
    let mut uncovered = N2;
    for &idx in selected.iter() {
        if in_sel[idx] {
            continue;
        }
        in_sel[idx] = true;
        k += 1;
        m_sum += cands[idx].m;
        for b in 0..BIT_WORDS {
            let mut w = cands[idx].cover.w[b];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let id = (b << 6) + bit;
                if id < N2 {
                    if cover_count[id] == 0 {
                        uncovered -= 1;
                    }
                    cover_count[id] += 1;
                }
                w &= w - 1;
            }
        }
    }

    let penalty_unit = input.ak * (N2 as i64) + input.am * (M_LIMIT as i64) + 1;
    let mut cur_energy = eval_template_energy(input, k, m_sum, uncovered, penalty_unit);
    let mut best_energy = cur_energy;
    best_in_sel.clone_from(&in_sel);
    let mut rng = XorShift64::new(seed_from_input(input) ^ 0xD6E8FEB86659FD93);

    for _ in 0..DEFAULT_TEMPLATE_TOGGLE_ITERS {
        let idx = rng.gen_usize(cands.len());
        if in_sel[idx] {
            let mut next_uncovered = uncovered;
            for b in 0..BIT_WORDS {
                let mut w = cands[idx].cover.w[b];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let id = (b << 6) + bit;
                    if id < N2 && cover_count[id] == 1 {
                        next_uncovered += 1;
                    }
                    w &= w - 1;
                }
            }
            let next_k = k - 1;
            let next_m = m_sum - cands[idx].m;
            let next_energy =
                eval_template_energy(input, next_k, next_m, next_uncovered, penalty_unit);
            if next_energy < cur_energy {
                for b in 0..BIT_WORDS {
                    let mut w = cands[idx].cover.w[b];
                    while w != 0 {
                        let bit = w.trailing_zeros() as usize;
                        let id = (b << 6) + bit;
                        if id < N2 {
                            cover_count[id] -= 1;
                        }
                        w &= w - 1;
                    }
                }
                in_sel[idx] = false;
                k = next_k;
                m_sum = next_m;
                uncovered = next_uncovered;
                cur_energy = next_energy;
                if cur_energy < best_energy {
                    best_energy = cur_energy;
                    best_in_sel.clone_from(&in_sel);
                }
            }
        } else {
            let mut next_uncovered = uncovered;
            for b in 0..BIT_WORDS {
                let mut w = cands[idx].cover.w[b];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let id = (b << 6) + bit;
                    if id < N2 && cover_count[id] == 0 {
                        next_uncovered -= 1;
                    }
                    w &= w - 1;
                }
            }
            let next_k = k + 1;
            let next_m = m_sum + cands[idx].m;
            let next_energy =
                eval_template_energy(input, next_k, next_m, next_uncovered, penalty_unit);
            if next_energy < cur_energy {
                for b in 0..BIT_WORDS {
                    let mut w = cands[idx].cover.w[b];
                    while w != 0 {
                        let bit = w.trailing_zeros() as usize;
                        let id = (b << 6) + bit;
                        if id < N2 {
                            cover_count[id] += 1;
                        }
                        w &= w - 1;
                    }
                }
                in_sel[idx] = true;
                k = next_k;
                m_sum = next_m;
                uncovered = next_uncovered;
                cur_energy = next_energy;
                if cur_energy < best_energy {
                    best_energy = cur_energy;
                    best_in_sel.clone_from(&in_sel);
                }
            }
        }
    }

    selected.clear();
    for (idx, &on) in best_in_sel.iter().enumerate() {
        if on {
            selected.push(idx);
        }
    }
}

fn add_stationary_template_candidates(input: &Input, out: &mut Vec<TemplateChoice>) {
    for cell in 0..(input.n * input.n) {
        let mut cover = BitSet::empty();
        cover.set_cell(cell);
        out.push(TemplateChoice {
            m: 1,
            start_cell: cell,
            start_dir: 0,
            a0: vec![ACT_R],
            b0: vec![0],
            a1: vec![ACT_R],
            b1: vec![0],
            cover,
        });
    }
}

fn prune_template_selection(selected: &mut Vec<usize>, cands: &[TemplateChoice], all: BitSet) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0usize;
        while i < selected.len() {
            let mut cov = BitSet::empty();
            for (j, &idx) in selected.iter().enumerate() {
                if i != j {
                    cov.or_assign(&cands[idx].cover);
                }
            }
            if cov == all {
                selected.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
    }
}

fn build_template_cover_plan(input: &Input) -> Option<TemplatePlan> {
    let env = build_template_env(input);
    let mut cands = Vec::<TemplateChoice>::new();
    for tpl in TEMPLATE_LIBRARY_B {
        collect_from_template(&env, tpl, &mut cands);
    }
    for tpl in TEMPLATE_LIBRARY_A_MIX {
        collect_from_template(&env, tpl, &mut cands);
    }
    for tpl in TEMPLATE_LIBRARY_C_MIX {
        collect_from_template(&env, tpl, &mut cands);
    }
    add_teammate_template_candidates(&env, &mut cands);
    add_stationary_template_candidates(input, &mut cands);

    let all = BitSet::all_400();
    let mut covered = BitSet::empty();
    let mut selected = Vec::<usize>::new();

    while covered != all {
        let mut best_idx = None::<usize>;
        let mut best_gain = 0u32;
        let mut best_w = 1i64;
        for i in 0..cands.len() {
            let gain = cands[i].cover.count_new(&covered);
            if gain == 0 {
                continue;
            }
            let w = input.ak + input.am * cands[i].m as i64;
            let better = match best_idx {
                None => true,
                Some(_) => {
                    let lhs = gain as i128 * best_w as i128;
                    let rhs = best_gain as i128 * w as i128;
                    if lhs != rhs {
                        lhs > rhs
                    } else if w != best_w {
                        w < best_w
                    } else {
                        gain > best_gain
                    }
                }
            };
            if better {
                best_idx = Some(i);
                best_gain = gain;
                best_w = w;
            }
        }
        let idx = best_idx?;
        selected.push(idx);
        covered.or_assign(&cands[idx].cover);
    }

    prune_template_selection(&mut selected, &cands, all);
    improve_template_selection_toggle(input, &cands, &mut selected);
    prune_template_selection(&mut selected, &cands, all);
    if selected.is_empty() {
        return None;
    }
    let m_sum: usize = selected.iter().map(|&i| cands[i].m).sum();
    let k = selected.len();
    let value_v = input.ak * (k as i64 - 1) + input.am * m_sum as i64;
    Some(TemplatePlan {
        cands,
        selected,
        value_v,
    })
}

fn print_template_plan(input: &Input, plan: &TemplatePlan) {
    println!("{}", plan.selected.len());
    for &idx in &plan.selected {
        let c = &plan.cands[idx];
        println!(
            "{} {} {} {}",
            c.m,
            c.start_cell / input.n,
            c.start_cell % input.n,
            DIR_CHARS[c.start_dir]
        );
        for s in 0..c.m {
            println!(
                "{} {} {} {}",
                act_char(c.a0[s]),
                c.b0[s],
                act_char(c.a1[s]),
                c.b1[s]
            );
        }
    }
    let zeros_v = "0".repeat(input.n - 1);
    let zeros_h = "0".repeat(input.n);
    for _ in 0..input.n {
        println!("{}", zeros_v);
    }
    for _ in 0..input.n - 1 {
        println!("{}", zeros_h);
    }
}

fn print_single_robot_plan(input: &Input, plan: &RobotPlan) {
    println!("1");
    println!(
        "{} {} {} {}",
        plan.states.len(),
        plan.start_cell / input.n,
        plan.start_cell % input.n,
        DIR_CHARS[plan.start_dir]
    );
    for s in 0..plan.states.len() {
        let st = plan.states[s];
        println!(
            "{} {} {} {}",
            act_char(st.a0),
            st.b0,
            act_char(st.a1),
            st.b1
        );
    }
    for i in 0..input.n {
        let mut line = String::with_capacity(input.n - 1);
        for j in 0..input.n - 1 {
            line.push(if plan.wall_v_add[i][j] == 1 { '1' } else { '0' });
        }
        println!("{}", line);
    }
    for i in 0..input.n - 1 {
        let mut line = String::with_capacity(input.n);
        for j in 0..input.n {
            line.push(if plan.wall_h_add[i][j] == 1 { '1' } else { '0' });
        }
        println!("{}", line);
    }
}

fn print_fallback_all_stationary(input: &Input) {
    println!("{}", N2);
    for cell in 0..N2 {
        println!("1 {} {} U", cell / input.n, cell % input.n);
        println!("R 0 R 0");
    }
    let zeros_v = "0".repeat(input.n - 1);
    let zeros_h = "0".repeat(input.n);
    for _ in 0..input.n {
        println!("{}", zeros_v);
    }
    for _ in 0..input.n - 1 {
        println!("{}", zeros_h);
    }
}

fn main() {
    let input = parse_input();
    assert_eq!(input.n, N_FIXED);

    let mut route_best: Option<RobotPlan> = None;
    if let Some(route) = best_route(&input) {
        route_best = choose_best_plan(&input, &route);
    }
    let template_best = build_template_cover_plan(&input);

    match (route_best, template_best) {
        (Some(rp), Some(tp)) => {
            if rp.value_v <= tp.value_v {
                print_single_robot_plan(&input, &rp);
            } else {
                print_template_plan(&input, &tp);
            }
        }
        (Some(rp), None) => print_single_robot_plan(&input, &rp),
        (None, Some(tp)) => print_template_plan(&input, &tp),
        (None, None) => print_fallback_all_stationary(&input),
    }
}
