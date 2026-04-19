use std::collections::VecDeque;
use std::hint::black_box;
use std::hash::{Hash, Hasher};
use std::time::Instant;

const N: usize = 16;
const CELL_COUNT: usize = N * N;
const INTERNAL_POS_DEQUE_CAPACITY: usize = 16 * 16;

#[derive(Clone, Copy, Default)]
struct BodyOcc {
    bits: [u64; 4],
}

impl BodyOcc {
    #[inline]
    fn set(&mut self, cell: u16) {
        let idx = cell as usize;
        self.bits[idx >> 6] |= 1_u64 << (idx & 63);
    }

    #[inline]
    fn contains(&self, cell: u16) -> bool {
        let idx = cell as usize;
        ((self.bits[idx >> 6] >> (idx & 63)) & 1) != 0
    }
}

#[derive(Clone)]
struct FixtureVec {
    food: Vec<u8>,
    pos: Vec<u16>,
    colors: Vec<u8>,
}

#[derive(Clone)]
struct FixtureDeque {
    food: Vec<u8>,
    pos: VecDeque<u16>,
    colors: Vec<u8>,
    body_occ: BodyOcc,
}

#[derive(Clone)]
struct InternalPosDeque {
    head: usize,
    len: usize,
    buf: [u16; INTERNAL_POS_DEQUE_CAPACITY],
}

impl InternalPosDeque {
    #[inline]
    fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [0; INTERNAL_POS_DEQUE_CAPACITY],
        }
    }

    #[inline]
    fn from_slice(cells: &[u16]) -> Self {
        let mut deque = Self::new();
        deque.buf[..cells.len()].copy_from_slice(cells);
        deque.len = cells.len();
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
        let raw = self.head + idx;
        if raw < INTERNAL_POS_DEQUE_CAPACITY {
            raw
        } else {
            raw - INTERNAL_POS_DEQUE_CAPACITY
        }
    }

    #[inline]
    fn push_front(&mut self, cell: u16) {
        self.head = (self.head + INTERNAL_POS_DEQUE_CAPACITY - 1) % INTERNAL_POS_DEQUE_CAPACITY;
        self.buf[self.head] = cell;
        self.len += 1;
    }

    #[inline]
    fn pop_back(&mut self) -> Option<u16> {
        if self.is_empty() {
            return None;
        }
        let idx = self.physical_index(self.len - 1);
        self.len -= 1;
        Some(self.buf[idx])
    }

    #[inline]
    fn iter(&self) -> InternalPosDequeIter<'_> {
        InternalPosDequeIter {
            deque: self,
            idx: 0,
        }
    }

    #[inline]
    fn as_slices(&self) -> (&[u16], &[u16]) {
        let tail = self.head + self.len;
        if tail <= INTERNAL_POS_DEQUE_CAPACITY {
            (&self.buf[self.head..tail], &[])
        } else {
            (
                &self.buf[self.head..],
                &self.buf[..tail - INTERNAL_POS_DEQUE_CAPACITY],
            )
        }
    }
}

impl std::ops::Index<usize> for InternalPosDeque {
    type Output = u16;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.buf[self.physical_index(idx)]
    }
}

impl PartialEq for InternalPosDeque {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        let (a1, a2) = self.as_slices();
        let (b1, b2) = other.as_slices();
        eq_u16_seq2(a1, a2, b1, b2)
    }
}

impl Eq for InternalPosDeque {}

impl Hash for InternalPosDeque {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        let (front, back) = self.as_slices();
        hash_u16_slice(front, state);
        hash_u16_slice(back, state);
    }
}

#[inline]
fn eq_u16_seq2(mut a1: &[u16], mut a2: &[u16], mut b1: &[u16], mut b2: &[u16]) -> bool {
    loop {
        let aslice = if !a1.is_empty() { a1 } else { a2 };
        let bslice = if !b1.is_empty() { b1 } else { b2 };
        if aslice.is_empty() || bslice.is_empty() {
            return aslice.is_empty() && bslice.is_empty();
        }
        let take = aslice.len().min(bslice.len());
        if aslice[..take] != bslice[..take] {
            return false;
        }
        if !a1.is_empty() {
            a1 = &a1[take..];
        } else {
            a2 = &a2[take..];
        }
        if !b1.is_empty() {
            b1 = &b1[take..];
        } else {
            b2 = &b2[take..];
        }
    }
}

#[inline]
fn hash_u16_slice<H: Hasher>(slice: &[u16], state: &mut H) {
    let bytes = unsafe {
        std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), std::mem::size_of_val(slice))
    };
    state.write(bytes);
}

struct InternalPosDequeIter<'a> {
    deque: &'a InternalPosDeque,
    idx: usize,
}

impl Iterator for InternalPosDequeIter<'_> {
    type Item = u16;

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

#[derive(Clone)]
struct FixtureCustom {
    food: Vec<u8>,
    pos: InternalPosDeque,
    colors: Vec<u8>,
}

#[inline]
fn build_body_occ(pos: &VecDeque<u16>) -> BodyOcc {
    let mut occ = BodyOcc::default();
    for (idx, &cell) in pos.iter().enumerate() {
        if idx + 1 == pos.len() {
            break;
        }
        occ.set(cell);
    }
    occ
}

fn make_path(len: usize) -> Vec<u16> {
    let mut path = Vec::with_capacity(len);
    for r in 0..N {
        if r % 2 == 0 {
            for c in 0..N {
                path.push((r * N + c) as u16);
                if path.len() == len {
                    return path;
                }
            }
        } else {
            for c in (0..N).rev() {
                path.push((r * N + c) as u16);
                if path.len() == len {
                    return path;
                }
            }
        }
    }
    path
}

fn make_empty_fixtures(len: usize) -> (FixtureVec, FixtureDeque, FixtureCustom, u16) {
    let mut pos_vec = make_path(len);
    pos_vec.reverse();
    let head = pos_vec[0] as usize;
    let nh = (head + 1) as u16;
    debug_assert!(pos_vec.len() < CELL_COUNT);
    debug_assert!(!pos_vec.contains(&nh));

    let mut pos_deque: VecDeque<u16> = pos_vec.iter().copied().collect();
    for _ in 0..17 {
        let x = pos_deque.pop_back().unwrap();
        pos_deque.push_front(x);
        let y = pos_deque.pop_front().unwrap();
        pos_deque.push_back(y);
    }

    let food = vec![0_u8; CELL_COUNT];
    let colors = vec![1_u8; len];
    let fixture_vec = FixtureVec {
        food: food.clone(),
        pos: pos_vec,
        colors: colors.clone(),
    };
    let fixture_deque = FixtureDeque {
        food,
        pos: pos_deque.clone(),
        colors,
        body_occ: build_body_occ(&pos_deque),
    };
    let fixture_custom = FixtureCustom {
        food: fixture_vec.food.clone(),
        pos: InternalPosDeque::from_slice(&fixture_vec.pos),
        colors: fixture_vec.colors.clone(),
    };
    (fixture_vec, fixture_deque, fixture_custom, nh)
}

fn make_eat_fixtures(len: usize) -> (FixtureVec, FixtureDeque, FixtureCustom, u16) {
    let (mut fv, mut fd, mut fc, nh) = make_empty_fixtures(len);
    fv.food[nh as usize] = 3;
    fd.food[nh as usize] = 3;
    fc.food[nh as usize] = 3;
    (fv, fd, fc, nh)
}

#[inline]
fn v123_pos_empty(pos: &[u16], nh: u16) -> Vec<u16> {
    let old_len = pos.len();
    let mut new_pos = Vec::with_capacity(old_len + 1);
    new_pos.push(nh);
    new_pos.extend_from_slice(&pos[..old_len - 1]);
    new_pos
}

#[inline]
fn v125_pos_empty(pos: &VecDeque<u16>, nh: u16) -> VecDeque<u16> {
    let mut new_pos = pos.clone();
    new_pos.pop_back();
    new_pos.push_front(nh);
    new_pos
}

#[inline]
fn v125_custom_pos_empty(pos: &InternalPosDeque, nh: u16) -> InternalPosDeque {
    let mut new_pos = pos.clone();
    new_pos.pop_back();
    new_pos.push_front(nh);
    new_pos
}

#[inline]
fn v123_step_like_empty(fx: &FixtureVec, nh: u16) -> usize {
    let food = black_box(fx.food.clone());
    let new_pos = black_box(v123_pos_empty(&fx.pos, nh));
    let new_colors = black_box(fx.colors.clone());
    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }
    black_box(food[0] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn v125_step_like_empty(fx: &FixtureDeque, nh: u16) -> usize {
    let food = black_box(fx.food.clone());
    let mut new_pos = black_box(fx.pos.clone());
    let new_colors = black_box(fx.colors.clone());
    let body_occ = fx.body_occ;
    new_pos.pop_back();
    let occupied_except_tail = body_occ.contains(nh);
    new_pos.push_front(nh);
    let bite_idx = if occupied_except_tail {
        (1..new_pos.len().saturating_sub(1)).find(|&idx| new_pos[idx] == nh)
    } else {
        None
    };
    black_box(food[0] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn v123_step_like_eat(fx: &FixtureVec, nh: u16) -> usize {
    let mut food = black_box(fx.food.clone());
    let old_len = fx.pos.len();
    let mut new_pos = black_box(Vec::with_capacity(old_len + 1));
    new_pos.push(nh);
    new_pos.extend_from_slice(&fx.pos[..old_len - 1]);
    let mut new_colors = black_box(fx.colors.clone());
    let ate = food[nh as usize];
    if ate != 0 {
        food[nh as usize] = 0;
        new_pos.push(fx.pos[old_len - 1]);
        new_colors.push(ate);
    }
    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }
    black_box(food[nh as usize] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn v125_step_like_eat(fx: &FixtureDeque, nh: u16) -> usize {
    let mut food = black_box(fx.food.clone());
    let mut new_pos = black_box(fx.pos.clone());
    let mut new_colors = black_box(fx.colors.clone());
    let body_occ = fx.body_occ;
    let ate = food[nh as usize];
    if ate != 0 {
        food[nh as usize] = 0;
        new_colors.push(ate);
    }
    let occupied_except_tail = body_occ.contains(nh);
    new_pos.push_front(nh);
    let bite_idx = if occupied_except_tail {
        (1..new_pos.len().saturating_sub(1)).find(|&idx| new_pos[idx] == nh)
    } else {
        None
    };
    black_box(food[nh as usize] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn v125_custom_step_like_empty(fx: &FixtureCustom, nh: u16) -> usize {
    let food = black_box(fx.food.clone());
    let mut new_pos = black_box(fx.pos.clone());
    let new_colors = black_box(fx.colors.clone());
    new_pos.pop_back();
    new_pos.push_front(nh);
    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }
    black_box(food[0] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn v125_custom_step_like_eat(fx: &FixtureCustom, nh: u16) -> usize {
    let mut food = black_box(fx.food.clone());
    let mut new_pos = black_box(fx.pos.clone());
    let mut new_colors = black_box(fx.colors.clone());
    let ate = food[nh as usize];
    if ate != 0 {
        food[nh as usize] = 0;
        new_colors.push(ate);
    }
    new_pos.push_front(nh);
    let mut bite_idx = None;
    for idx in 1..new_pos.len().saturating_sub(1) {
        if new_pos[idx] == nh {
            bite_idx = Some(idx);
            break;
        }
    }
    black_box(food[nh as usize] as usize + new_pos[0] as usize + new_colors[0] as usize + bite_idx.unwrap_or(0))
}

#[inline]
fn state_hash64_vec(food: &[u8], pos: &[u16], colors: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325_u64;
    let p = 0x100000001b3_u64;
    h ^= pos.len() as u64;
    h = h.wrapping_mul(p);
    h ^= colors.len() as u64;
    h = h.wrapping_mul(p);
    for &x in food {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    for &cell in pos {
        h ^= cell as u64;
        h = h.wrapping_mul(p);
    }
    for &x in colors {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    h
}

#[inline]
fn state_hash64_custom(food: &[u8], pos: &InternalPosDeque, colors: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325_u64;
    let p = 0x100000001b3_u64;
    h ^= pos.len() as u64;
    h = h.wrapping_mul(p);
    h ^= colors.len() as u64;
    h = h.wrapping_mul(p);
    for &x in food {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    for cell in pos.iter() {
        h ^= cell as u64;
        h = h.wrapping_mul(p);
    }
    for &x in colors {
        h ^= x as u64;
        h = h.wrapping_mul(p);
    }
    h
}

#[inline]
fn std_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn bench<T>(name: &str, iters: usize, mut f: impl FnMut() -> T) {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    let elapsed = start.elapsed();
    let ns_per_iter = elapsed.as_nanos() as f64 / iters as f64;
    println!("{name:32} {iters:>10} iters  {ns_per_iter:>10.2} ns/iter");
}

fn main() {
    println!("microbench: v123 vs v125 step components");
    println!("board_size={N}x{N}");
    println!();

    let occ_vec = vec![1_u8; CELL_COUNT];
    let body_occ = BodyOcc {
        bits: [u64::MAX; 4],
    };
    bench("clone Vec<u8>(256)", 5_000_000, || occ_vec.clone());
    bench("copy BodyOcc(32B)", 20_000_000, || body_occ);
    let deque = InternalPosDeque::from_slice(&make_path(96));
    bench("clone InternalPosDeque(256)", 5_000_000, || deque.clone());
    println!();

    for &len in &[32_usize, 96, 160] {
        let (fv_empty, fd_empty, fc_empty, nh_empty) = make_empty_fixtures(len);
        let (fv_eat, fd_eat, fc_eat, nh_eat) = make_eat_fixtures(len);
        println!("len={len}");
        bench("v123 pos empty", 2_000_000, || v123_pos_empty(&fv_empty.pos, nh_empty));
        bench("v125 pos empty", 2_000_000, || v125_pos_empty(&fd_empty.pos, nh_empty));
        bench("v125 custom pos empty", 2_000_000, || {
            v125_custom_pos_empty(&fc_empty.pos, nh_empty)
        });
        bench("v123 step-like empty", 500_000, || {
            v123_step_like_empty(&fv_empty, nh_empty)
        });
        bench("v125 step-like empty", 500_000, || {
            v125_step_like_empty(&fd_empty, nh_empty)
        });
        bench("v125 custom step empty", 500_000, || {
            v125_custom_step_like_empty(&fc_empty, nh_empty)
        });
        bench("v123 step-like eat", 500_000, || {
            v123_step_like_eat(&fv_eat, nh_eat)
        });
        bench("v125 step-like eat", 500_000, || {
            v125_step_like_eat(&fd_eat, nh_eat)
        });
        bench("v125 custom step eat", 500_000, || {
            v125_custom_step_like_eat(&fc_eat, nh_eat)
        });
        bench("state_hash64 vec", 1_000_000, || {
            state_hash64_vec(&fv_empty.food, &fv_empty.pos, &fv_empty.colors)
        });
        bench("state_hash64 custom", 1_000_000, || {
            state_hash64_custom(&fc_empty.food, &fc_empty.pos, &fc_empty.colors)
        });
        bench("std hash vec pos", 1_000_000, || std_hash(&fv_empty.pos));
        bench("std hash custom pos", 1_000_000, || std_hash(&fc_empty.pos));
        bench("eq vec pos", 2_000_000, || black_box(fv_empty.pos == fv_empty.pos));
        bench("eq custom pos", 2_000_000, || {
            black_box(fc_empty.pos == fc_empty.pos)
        });
        println!();
    }
}
