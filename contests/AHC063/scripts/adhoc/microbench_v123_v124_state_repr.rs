use std::hint::black_box;
use std::mem::size_of;
use std::time::Instant;

const FIXED_CAP: usize = 16 * 16;
const POS_HASH_BASE1: u64 = 0x9E37_79B1_85EB_CA87;
const POS_HASH_BASE2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const COLOR_HASH_BASE1: u64 = 0x1656_67B1_9E37_79F9;
const COLOR_HASH_BASE2: u64 = 0x27D4_EB2F_C2B2_AE63;

const fn build_hash_pows(base: u64) -> [u64; FIXED_CAP + 1] {
    let mut pows = [0_u64; FIXED_CAP + 1];
    pows[0] = 1;
    let mut i = 1;
    while i <= FIXED_CAP {
        pows[i] = pows[i - 1].wrapping_mul(base);
        i += 1;
    }
    pows
}

const POS_HASH_POW1: [u64; FIXED_CAP + 1] = build_hash_pows(POS_HASH_BASE1);
const POS_HASH_POW2: [u64; FIXED_CAP + 1] = build_hash_pows(POS_HASH_BASE2);
const COLOR_HASH_POW1: [u64; FIXED_CAP + 1] = build_hash_pows(COLOR_HASH_BASE1);
const COLOR_HASH_POW2: [u64; FIXED_CAP + 1] = build_hash_pows(COLOR_HASH_BASE2);

#[inline]
fn pos_hash_token(cell: u16) -> u64 {
    cell as u64 + 1
}

#[inline]
fn color_hash_token(color: u8) -> u64 {
    color as u64 + 1
}

#[derive(Clone)]
struct StateVec {
    n: usize,
    food: Vec<u8>,
    pos: Vec<u16>,
    colors: Vec<u8>,
}

#[derive(Clone)]
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [u16; FIXED_CAP],
    hash1: u64,
    hash2: u64,
}

impl InternalPosDeque {
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [0; FIXED_CAP],
            hash1: 0,
            hash2: 0,
        }
    }

    fn from_slice(cells: &[u16]) -> Self {
        let mut out = Self::new();
        out.buf[..cells.len()].copy_from_slice(cells);
        out.len = cells.len();
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &cell in cells {
            let x = pos_hash_token(cell);
            out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
            out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(POS_HASH_BASE1);
            pow2 = pow2.wrapping_mul(POS_HASH_BASE2);
        }
        out
    }

    fn len(&self) -> usize {
        self.len
    }

    fn physical_index(&self, idx: usize) -> usize {
        let raw = self.head + idx;
        if raw < FIXED_CAP { raw } else { raw - FIXED_CAP }
    }

    fn push_front(&mut self, cell: u16) {
        let x = pos_hash_token(cell);
        self.head = (self.head + FIXED_CAP - 1) % FIXED_CAP;
        self.buf[self.head] = cell;
        self.hash1 = x.wrapping_add(self.hash1.wrapping_mul(POS_HASH_BASE1));
        self.hash2 = x.wrapping_add(self.hash2.wrapping_mul(POS_HASH_BASE2));
        self.len += 1;
    }

    fn pop_back(&mut self) -> Option<u16> {
        if self.len == 0 {
            return None;
        }
        let idx = self.physical_index(self.len - 1);
        let cell = self.buf[idx];
        let x = pos_hash_token(cell);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(POS_HASH_POW1[self.len - 1]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(POS_HASH_POW2[self.len - 1]));
        self.len -= 1;
        Some(cell)
    }

    fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.len).map(move |i| self.buf[self.physical_index(i)])
    }
}

impl std::ops::Index<usize> for InternalPosDeque {
    type Output = u16;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.buf[self.physical_index(idx)]
    }
}

#[derive(Clone)]
struct InternalColors {
    buf: [u8; FIXED_CAP],
    len: u16,
    hash1: u64,
    hash2: u64,
}

impl InternalColors {
    fn new() -> Self {
        Self {
            buf: [0; FIXED_CAP],
            len: 0,
            hash1: 0,
            hash2: 0,
        }
    }

    fn from_slice(colors: &[u8]) -> Self {
        let mut out = Self::new();
        out.buf[..colors.len()].copy_from_slice(colors);
        out.len = colors.len() as u16;
        let mut pow1 = 1_u64;
        let mut pow2 = 1_u64;
        for &color in colors {
            let x = color_hash_token(color);
            out.hash1 = out.hash1.wrapping_add(x.wrapping_mul(pow1));
            out.hash2 = out.hash2.wrapping_add(x.wrapping_mul(pow2));
            pow1 = pow1.wrapping_mul(COLOR_HASH_BASE1);
            pow2 = pow2.wrapping_mul(COLOR_HASH_BASE2);
        }
        out
    }

    fn push(&mut self, color: u8) {
        let idx = self.len as usize;
        self.buf[idx] = color;
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_add(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_add(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let idx = self.len as usize - 1;
        let color = self.buf[idx];
        let x = color_hash_token(color);
        self.hash1 = self.hash1.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW1[idx]));
        self.hash2 = self.hash2.wrapping_sub(x.wrapping_mul(COLOR_HASH_POW2[idx]));
        self.len -= 1;
        Some(color)
    }

    fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

#[derive(Clone)]
struct StateFixed {
    n: usize,
    food: [u8; FIXED_CAP],
    food_hash: u64,
    pos: InternalPosDeque,
    colors: InternalColors,
}

#[derive(Clone)]
struct NodeVec {
    state: StateVec,
    parent: Option<usize>,
    move_seg: String,
}

#[derive(Clone)]
struct NodeFixed {
    state: StateFixed,
    parent: Option<usize>,
    move_seg: String,
}

fn make_state_vec(n: usize, len: usize) -> StateVec {
    let mut food = vec![0_u8; FIXED_CAP];
    for i in 0..FIXED_CAP {
        if i % 7 == 0 {
            food[i] = (i % 5 + 1) as u8;
        }
    }
    let pos = (0..len).map(|i| i as u16).collect::<Vec<_>>();
    let colors = (0..len).map(|i| (i % 7 + 1) as u8).collect::<Vec<_>>();
    StateVec { n, food, pos, colors }
}

fn make_state_fixed(n: usize, len: usize) -> StateFixed {
    let src = make_state_vec(n, len);
    let mut food = [0_u8; FIXED_CAP];
    let mut food_hash = 0_u64;
    for (i, &x) in src.food.iter().enumerate() {
        food[i] = x;
        if x != 0 {
            food_hash ^= ((i as u64) << 8) ^ x as u64;
        }
    }
    StateFixed {
        n,
        food,
        food_hash,
        pos: InternalPosDeque::from_slice(&src.pos),
        colors: InternalColors::from_slice(&src.colors),
    }
}

fn encode_key_vec(st: &StateVec) -> usize {
    let mut food = Vec::with_capacity(st.colors.len() + 16);
    for (idx, &col) in st.food.iter().enumerate() {
        if col != 0 {
            food.push((idx as u16, col));
        }
    }
    black_box((st.pos.clone(), st.colors.clone(), food)).0.len()
}

fn encode_key_fixed(st: &StateFixed) -> usize {
    let mut food = Vec::with_capacity(st.colors.as_slice().len() + 16);
    for (idx, &col) in st.food.iter().enumerate() {
        if col != 0 {
            food.push((idx as u16, col));
        }
    }
    let pos = st.pos.iter().collect::<Vec<_>>();
    let colors = st.colors.as_slice().to_vec();
    black_box((pos, colors, food)).0.len()
}

fn step_vec_checksum(st: &StateVec, nh: u16, eat: bool) -> usize {
    let old_len = st.pos.len();
    let mut food = st.food.clone();
    let mut new_pos = Vec::with_capacity(old_len + 1);
    new_pos.push(nh);
    new_pos.extend_from_slice(&st.pos[..old_len - 1]);
    let mut colors = st.colors.clone();
    if eat {
        food[nh as usize] = 0;
        new_pos.push(st.pos[old_len - 1]);
        colors.push(1);
    }
    let tail = *new_pos.last().unwrap_or(&0) as usize;
    let head = new_pos[0] as usize;
    head ^ tail ^ food[nh as usize] as usize ^ colors.len()
}

fn step_fixed_checksum(st: &StateFixed, nh: u16, eat: bool) -> usize {
    let mut ns = st.clone();
    if eat {
        ns.food[nh as usize] = 0;
        ns.colors.push(1);
    } else {
        ns.pos.pop_back().unwrap();
    }
    ns.pos.push_front(nh);
    let tail = ns.pos[ns.pos.len() - 1] as usize;
    let head = ns.pos[0] as usize;
    head ^ tail ^ ns.food[nh as usize] as usize ^ ns.colors.as_slice().len()
}

fn bench<F: FnMut() -> usize>(name: &str, iters: usize, mut f: F) {
    let t0 = Instant::now();
    let mut acc = 0usize;
    for _ in 0..iters {
        acc ^= black_box(f());
    }
    let dt = t0.elapsed().as_secs_f64() * 1e9 / iters as f64;
    println!("{name:28} {:9.2} ns acc={acc}", dt);
}

fn main() {
    println!("size_of<StateVec>={}", size_of::<StateVec>());
    println!("size_of<StateFixed>={}", size_of::<StateFixed>());
    println!("size_of<NodeVec>={}", size_of::<NodeVec>());
    println!("size_of<NodeFixed>={}", size_of::<NodeFixed>());

    for &len in &[16usize, 48, 96, 160] {
        let sv = make_state_vec(16, len);
        let sf = make_state_fixed(16, len);
        let nh = len as u16 + 1;
        println!("\nlen={len}");
        bench("clone_state_vec", 200_000, || {
            let c = black_box(black_box(&sv).clone());
            c.pos[0] as usize ^ c.food[0] as usize ^ c.colors.len()
        });
        bench("clone_state_fixed", 200_000, || {
            let c = black_box(black_box(&sf).clone());
            c.pos[0] as usize ^ c.food[0] as usize ^ c.colors.as_slice().len()
        });

        bench("node_push_vec", 20_000, || {
            let mut v = Vec::with_capacity(16);
            for i in 0..16 {
                v.push(NodeVec {
                    state: black_box(&sv).clone(),
                    parent: Some(i),
                    move_seg: String::new(),
                });
            }
            let last = black_box(v.pop().unwrap());
            last.state.pos[0] as usize ^ last.state.colors.len()
        });
        bench("node_push_fixed", 20_000, || {
            let mut v = Vec::with_capacity(16);
            for i in 0..16 {
                v.push(NodeFixed {
                    state: black_box(&sf).clone(),
                    parent: Some(i),
                    move_seg: String::new(),
                });
            }
            let last = black_box(v.pop().unwrap());
            last.state.pos[0] as usize ^ last.state.colors.as_slice().len()
        });

        bench("step_vec_empty", 100_000, || {
            step_vec_checksum(black_box(&sv), black_box(nh), false)
        });
        bench("step_fixed_empty", 100_000, || {
            step_fixed_checksum(black_box(&sf), black_box(nh), false)
        });
        bench("step_vec_eat", 100_000, || {
            step_vec_checksum(black_box(&sv), black_box(nh), true)
        });
        bench("step_fixed_eat", 100_000, || {
            step_fixed_checksum(black_box(&sf), black_box(nh), true)
        });

        bench("encode_key_vec", 100_000, || encode_key_vec(black_box(&sv)));
        bench("encode_key_fixed", 100_000, || encode_key_fixed(black_box(&sf)));
    }
}
