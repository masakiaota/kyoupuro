// tmp_prefix_repair_check.rs
use std::collections::VecDeque;

const DIRS: [(isize, isize, char); 4] = [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const CELL_CAPACITY: usize = 16 * 16;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dropped {
    cell: Cell,
    color: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    food: Vec<u8>,
    pos: VecDeque<Cell>,
    colors: Vec<u8>,
    pos_occupancy: [u8; CELL_CAPACITY],
}

impl State {
    fn initial(n: usize, food: Vec<u8>) -> Self {
        let pos = VecDeque::from([
            Grid::cell(n, 4, 0),
            Grid::cell(n, 3, 0),
            Grid::cell(n, 2, 0),
            Grid::cell(n, 1, 0),
            Grid::cell(n, 0, 0),
        ]);
        Self::from_parts(food, pos, vec![1; 5])
    }

    fn from_parts(food: Vec<u8>, pos: VecDeque<Cell>, colors: Vec<u8>) -> Self {
        assert_eq!(pos.len(), colors.len());
        let mut pos_occupancy = [0_u8; CELL_CAPACITY];
        for &cell in &pos {
            pos_occupancy[Grid::index(cell)] += 1;
        }
        Self {
            food,
            pos,
            colors,
            pos_occupancy,
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

    fn legal_dirs(&self, n: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for dir in 0..DIRS.len() {
            if self.is_legal_dir(n, dir) {
                out.push(dir);
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StepResult {
    state: State,
    ate: Option<u8>,
    bite_idx: Option<usize>,
    dropped: Vec<Dropped>,
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

fn find_bite_idx(pos: &VecDeque<Cell>) -> Option<usize> {
    let head = pos[0];
    (1..pos.len().saturating_sub(1)).find(|&idx| pos[idx] == head)
}

fn step(state: &State, n: usize, dir: usize) -> StepResult {
    debug_assert!(state.is_legal_dir(n, dir));
    let next_head = Grid::next_cell(n, state.head(), dir);

    let mut food = state.food.clone();
    let mut pos = state.pos.clone();
    let mut colors = state.colors.clone();
    let mut occ = state.pos_occupancy;
    let mut ate = None;

    let eat_idx = Grid::index(next_head);
    if food[eat_idx] != 0 {
        let color = food[eat_idx];
        food[eat_idx] = 0;
        colors.push(color);
        ate = Some(color);
    } else {
        let tail = pos.pop_back().unwrap();
        occ[Grid::index(tail)] -= 1;
    }

    let excluded_tail = pos.back().copied();
    let tail_bias = u8::from(excluded_tail == Some(next_head));
    let bite = occ[Grid::index(next_head)] > tail_bias;

    occ[Grid::index(next_head)] += 1;
    pos.push_front(next_head);
    let bite_idx = if bite { find_bite_idx(&pos) } else { None };
    debug_assert!(ate.is_none() || bite_idx.is_none());

    let mut dropped = Vec::new();
    if let Some(h) = bite_idx {
        let mut dropped_rev = Vec::new();
        while pos.len() > h + 1 {
            let cell = pos.pop_back().unwrap();
            occ[Grid::index(cell)] -= 1;
            let color = colors.pop().unwrap();
            food[Grid::index(cell)] = color;
            dropped_rev.push(Dropped { cell, color });
        }
        dropped_rev.reverse();
        dropped = dropped_rev;
    }

    StepResult {
        state: State {
            food,
            pos,
            colors,
            pos_occupancy: occ,
        },
        ate,
        bite_idx,
        dropped,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixRepairResult {
    state: State,
    ops: Vec<usize>,
    repaired: bool,
}

fn repair_prefix_after_bite(
    st_after: &State,
    n: usize,
    prefix_target: &[u8],
    dropped: &[Dropped],
) -> PrefixRepairResult {
    let keep = st_after.colors.len().min(prefix_target.len());
    debug_assert!(matches_prefix_len(&st_after.colors, prefix_target, keep));

    let need = prefix_target.len().saturating_sub(st_after.colors.len());
    if need == 0 {
        return PrefixRepairResult {
            state: st_after.clone(),
            ops: Vec::new(),
            repaired: false,
        };
    }

    debug_assert!(dropped.len() >= need);
    let mut food = st_after.food.clone();
    let mut pos = st_after.pos.clone();
    let mut occ = st_after.pos_occupancy;
    let mut ops = Vec::with_capacity(need);
    let mut prev = st_after.head();

    for (t, ent) in dropped.iter().take(need).enumerate() {
        debug_assert_eq!(food[Grid::index(ent.cell)], ent.color);
        debug_assert_eq!(ent.color, prefix_target[st_after.colors.len() + t]);
        let dir = Grid::dir_between_cells(n, prev, ent.cell);
        ops.push(dir);
        food[Grid::index(ent.cell)] = 0;
        pos.push_front(ent.cell);
        occ[Grid::index(ent.cell)] += 1;
        prev = ent.cell;
    }

    PrefixRepairResult {
        state: State {
            food,
            pos,
            colors: prefix_target.to_vec(),
            pos_occupancy: occ,
        },
        ops,
        repaired: true,
    }
}

fn repair_prefix_after_bite_reference(
    st_after: &State,
    n: usize,
    prefix_target: &[u8],
    dropped: &[Dropped],
) -> PrefixRepairResult {
    let need = prefix_target.len().saturating_sub(st_after.colors.len());
    if need == 0 {
        return PrefixRepairResult {
            state: st_after.clone(),
            ops: Vec::new(),
            repaired: false,
        };
    }

    let mut state = st_after.clone();
    let mut ops = Vec::with_capacity(need);
    for ent in dropped.iter().take(need) {
        let dir = Grid::dir_between_cells(n, state.head(), ent.cell);
        let res = step(&state, n, dir);
        assert_eq!(res.ate, Some(ent.color));
        assert!(res.bite_idx.is_none());
        assert!(res.dropped.is_empty());
        ops.push(dir);
        state = res.state;
    }

    PrefixRepairResult {
        state,
        ops,
        repaired: true,
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256PlusPlus;

    fn make_state(n: usize, food: Vec<u8>, pos_ij: &[(usize, usize)], colors: &[u8]) -> State {
        let pos = pos_ij
            .iter()
            .map(|&(i, j)| Grid::cell(n, i, j))
            .collect::<VecDeque<_>>();
        State::from_parts(food, pos, colors.to_vec())
    }

    fn assert_states_eq(actual: &State, expected: &State) {
        assert_eq!(actual.food, expected.food);
        assert_eq!(actual.pos, expected.pos);
        assert_eq!(actual.colors, expected.colors);
        assert_eq!(actual.pos_occupancy, expected.pos_occupancy);
    }

    #[test]
    fn repair_is_noop_when_prefix_still_remains() {
        let n = 6;
        let state = make_state(
            n,
            vec![0; n * n],
            &[
                (2, 2),
                (2, 1),
                (1, 1),
                (1, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (3, 2),
                (3, 1),
            ],
            &[1, 2, 3, 4, 5, 6, 7, 1, 2],
        );
        let res = step(&state, n, 3);
        assert_eq!(res.bite_idx, Some(6));
        let prefix_target = state.colors[..6].to_vec();

        let repaired =
            repair_prefix_after_bite(&res.state, n, &prefix_target, &res.dropped);
        assert!(!repaired.repaired);
        assert!(repaired.ops.is_empty());
        assert_states_eq(&repaired.state, &res.state);
    }

    #[test]
    fn repair_one_cell_matches_reference() {
        let n = 6;
        let state = make_state(
            n,
            vec![0; n * n],
            &[
                (2, 2),
                (2, 1),
                (1, 1),
                (1, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (3, 2),
                (3, 1),
            ],
            &[1, 2, 3, 4, 5, 6, 7, 1, 2],
        );
        let bite = step(&state, n, 3);
        let prefix_target = state.colors[..8].to_vec();

        let actual = repair_prefix_after_bite(&bite.state, n, &prefix_target, &bite.dropped);
        let expected =
            repair_prefix_after_bite_reference(&bite.state, n, &prefix_target, &bite.dropped);

        assert!(actual.repaired);
        assert_eq!(actual.ops, expected.ops);
        assert_states_eq(&actual.state, &expected.state);
    }

    #[test]
    fn repair_multiple_cells_matches_reference() {
        let n = 6;
        let state = make_state(
            n,
            vec![0; n * n],
            &[
                (2, 2),
                (2, 1),
                (1, 1),
                (1, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (3, 2),
                (3, 1),
            ],
            &[1, 2, 3, 4, 5, 6, 7, 1, 2],
        );
        let bite = step(&state, n, 3);
        let prefix_target = state.colors.clone();

        let actual = repair_prefix_after_bite(&bite.state, n, &prefix_target, &bite.dropped);
        let expected =
            repair_prefix_after_bite_reference(&bite.state, n, &prefix_target, &bite.dropped);

        assert!(actual.repaired);
        assert_eq!(actual.ops, expected.ops);
        assert_states_eq(&actual.state, &expected.state);
    }

    #[test]
    fn random_bite_repairs_match_reference() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
        let n = 8;
        let mut food = vec![0; n * n];
        for i in 0..n {
            for j in 0..n {
                if j == 0 && i <= 4 {
                    continue;
                }
                if rng.random_range(0..100) < 30 {
                    food[Grid::index(Grid::cell(n, i, j))] = rng.random_range(1..=3);
                }
            }
        }

        let mut state = State::initial(n, food);
        let mut checked = 0usize;

        for _ in 0..800 {
            let dirs = state.legal_dirs(n);
            let bite_dirs = dirs
                .iter()
                .copied()
                .filter(|&dir| step(&state, n, dir).bite_idx.is_some())
                .collect::<Vec<_>>();

            if !bite_dirs.is_empty() {
                let dir = bite_dirs[rng.random_range(0..bite_dirs.len())];
                let before = state.clone();
                let bite = step(&state, n, dir);
                let post_len = bite.state.colors.len();
                let ell = rng.random_range(post_len..=before.colors.len());
                let prefix_target = before.colors[..ell].to_vec();

                let actual =
                    repair_prefix_after_bite(&bite.state, n, &prefix_target, &bite.dropped);
                let expected = repair_prefix_after_bite_reference(
                    &bite.state,
                    n,
                    &prefix_target,
                    &bite.dropped,
                );

                assert_eq!(actual.repaired, expected.repaired);
                assert_eq!(actual.ops, expected.ops);
                assert_states_eq(&actual.state, &expected.state);
                assert!(matches_prefix_len(
                    &actual.state.colors,
                    &prefix_target,
                    prefix_target.len()
                ));

                checked += 1;
                state = bite.state;
            } else {
                let dir = dirs[rng.random_range(0..dirs.len())];
                state = step(&state, n, dir).state;
            }
        }

        assert!(checked >= 10, "bite case が十分集まっていない: checked={checked}");
    }
}
