// v001_greedy_bfs.rs
use proconio::input;

const N: usize = 50;
const CELL_COUNT: usize = N * N;
const INVALID: usize = usize::MAX;
const DIRS: [(isize, isize, u8); 4] = [(-1, 0, b'U'), (1, 0, b'D'), (0, -1, b'L'), (0, 1, b'R')];

#[derive(Clone, Copy)]
struct Choice {
    next: usize,
    dir: u8,
    reachable_tiles: usize,
    next_degree: usize,
    immediate_score: i32,
    dir_rank: usize,
}

impl Choice {
    fn is_better_than(&self, other: &Self) -> bool {
        if self.reachable_tiles != other.reachable_tiles {
            return self.reachable_tiles > other.reachable_tiles;
        }
        if self.next_degree != other.next_degree {
            return self.next_degree < other.next_degree;
        }
        if self.immediate_score != other.immediate_score {
            return self.immediate_score > other.immediate_score;
        }
        self.dir_rank < other.dir_rank
    }
}

struct Evaluator {
    seen_cells: Vec<u32>,
    seen_tiles: Vec<u32>,
    queue: Vec<usize>,
    stamp: u32,
}

impl Evaluator {
    fn new(tile_count: usize) -> Self {
        Self {
            seen_cells: vec![0; CELL_COUNT],
            seen_tiles: vec![0; tile_count],
            queue: Vec::with_capacity(CELL_COUNT),
            stamp: 0,
        }
    }

    // 次の 1 手を打ったあと、まだ到達可能な未使用タイル数を数える。
    fn reachable_tiles_after_move(
        &mut self,
        start: usize,
        start_tile: usize,
        used_tiles: &[bool],
        tiles: &[usize],
        adj: &[[usize; 4]],
    ) -> usize {
        self.stamp += 1;
        let token = self.stamp;
        self.queue.clear();
        self.queue.push(start);
        self.seen_cells[start] = token;

        let mut reachable_tiles = 0;
        let mut head = 0;
        while head < self.queue.len() {
            let v = self.queue[head];
            head += 1;

            for &to in &adj[v] {
                if to == INVALID {
                    continue;
                }
                let tile = tiles[to];
                if tile == start_tile || used_tiles[tile] {
                    continue;
                }
                if self.seen_cells[to] == token {
                    continue;
                }
                self.seen_cells[to] = token;
                self.queue.push(to);
                if self.seen_tiles[tile] != token {
                    self.seen_tiles[tile] = token;
                    reachable_tiles += 1;
                }
            }
        }
        reachable_tiles
    }
}

fn count_next_degree(pos: usize, used_tiles: &[bool], tiles: &[usize], adj: &[[usize; 4]]) -> usize {
    adj[pos]
        .iter()
        .filter(|&&to| to != INVALID && !used_tiles[tiles[to]])
        .count()
}

fn main() {
    input! {
        si: usize,
        sj: usize,
        t: [[usize; N]; N],
        p: [[i32; N]; N],
    }

    let mut tiles = vec![0; CELL_COUNT];
    let mut scores = vec![0; CELL_COUNT];
    let mut max_tile = 0;
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            tiles[idx] = t[i][j];
            scores[idx] = p[i][j];
            max_tile = max_tile.max(t[i][j]);
        }
    }
    let tile_count = max_tile + 1;

    let mut adj = vec![[INVALID; 4]; CELL_COUNT];
    for i in 0..N {
        for j in 0..N {
            let idx = i * N + j;
            for (dir, &(di, dj, _)) in DIRS.iter().enumerate() {
                let ni = i as isize + di;
                let nj = j as isize + dj;
                if !(0..N as isize).contains(&ni) || !(0..N as isize).contains(&nj) {
                    continue;
                }
                let ni = ni as usize;
                let nj = nj as usize;
                if t[i][j] == t[ni][nj] {
                    continue;
                }
                adj[idx][dir] = ni * N + nj;
            }
        }
    }

    let mut used_tiles = vec![false; tile_count];
    let mut cur = si * N + sj;
    used_tiles[tiles[cur]] = true;

    let mut evaluator = Evaluator::new(tile_count);
    let mut path = Vec::with_capacity(CELL_COUNT);

    loop {
        let mut best: Option<Choice> = None;

        for (dir_rank, &(_, _, dir_char)) in DIRS.iter().enumerate() {
            let next = adj[cur][dir_rank];
            if next == INVALID {
                continue;
            }

            let next_tile = tiles[next];
            if used_tiles[next_tile] {
                continue;
            }

            let candidate = Choice {
                next,
                dir: dir_char,
                reachable_tiles: evaluator.reachable_tiles_after_move(
                    next,
                    next_tile,
                    &used_tiles,
                    &tiles,
                    &adj,
                ),
                next_degree: count_next_degree(next, &used_tiles, &tiles, &adj),
                immediate_score: scores[next],
                dir_rank,
            };

            match best {
                Some(current_best) if !candidate.is_better_than(&current_best) => {}
                _ => best = Some(candidate),
            }
        }

        let Some(choice) = best else {
            break;
        };
        cur = choice.next;
        used_tiles[tiles[cur]] = true;
        path.push(choice.dir);
    }

    println!("{}", String::from_utf8(path).unwrap());
}
