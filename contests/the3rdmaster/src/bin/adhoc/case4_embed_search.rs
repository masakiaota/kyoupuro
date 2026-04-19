// case4_embed_search.rs
use std::fs;
use std::path::Path;

const N: usize = 32;
const SCRATCH: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Paint {
        k: usize,
        i: usize,
        j: usize,
        color: u8,
    },
    Copy {
        k: usize,
        h: usize,
        rot: usize,
        di: isize,
        dj: isize,
    },
    Clear {
        k: usize,
    },
}

#[derive(Clone, Debug)]
struct Input {
    goal: [[u8; N]; N],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComponentKind {
    Frame,
    Solid,
}

#[derive(Clone, Copy, Debug)]
struct Component {
    start: usize,
    side: usize,
    color: u8,
    kind: ComponentKind,
}

fn main() {
    let input_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src/make_input/case4_concentric_input.txt".to_string());
    let input = read_input(Path::new(&input_path));

    let components = [
        Component {
            start: 0,
            side: 32,
            color: 1,
            kind: ComponentKind::Frame,
        },
        Component {
            start: 3,
            side: 26,
            color: 2,
            kind: ComponentKind::Frame,
        },
        Component {
            start: 6,
            side: 20,
            color: 3,
            kind: ComponentKind::Frame,
        },
        Component {
            start: 9,
            side: 14,
            color: 4,
            kind: ComponentKind::Frame,
        },
        Component {
            start: 12,
            side: 8,
            color: 1,
            kind: ComponentKind::Frame,
        },
        Component {
            start: 15,
            side: 2,
            color: 2,
            kind: ComponentKind::Solid,
        },
    ];

    let mut best_ops = current_builder_ops(&components);
    assert!(simulate_goal(&input, &best_ops));
    eprintln!("baseline={}", best_ops.len());

    let all_batches = enumerate_ordered_batches(components.len());
    for batches in all_batches {
        let ops = build_ops_from_batches(&components, &batches);
        if ops.len() >= best_ops.len() {
            continue;
        }
        if simulate_goal(&input, &ops) {
            eprintln!("improved {} -> {}", best_ops.len(), ops.len());
            eprintln!("batches={batches:?}");
            best_ops = ops;
        }
    }

    println!("best_actions={}", best_ops.len());
    for op in best_ops {
        match op {
            Op::Paint { k, i, j, color } => println!("0 {k} {i} {j} {color}"),
            Op::Copy { k, h, rot, di, dj } => println!("1 {k} {h} {rot} {di} {dj}"),
            Op::Clear { k } => println!("2 {k}"),
        }
    }
}

fn read_input(path: &Path) -> Input {
    let text = fs::read_to_string(path).expect("failed to read input");
    let mut it = text.split_ascii_whitespace();
    let n: usize = it.next().unwrap().parse().unwrap();
    let _k: usize = it.next().unwrap().parse().unwrap();
    let _c: usize = it.next().unwrap().parse().unwrap();
    assert_eq!(n, N);
    let mut goal = [[0_u8; N]; N];
    for row in &mut goal {
        for cell in row {
            *cell = it.next().unwrap().parse().unwrap();
        }
    }
    Input { goal }
}

fn current_builder_ops(components: &[Component]) -> Vec<Op> {
    let mut ops = Vec::new();
    for (idx, &component) in components.iter().enumerate() {
        if idx > 0 {
            ops.push(Op::Clear { k: SCRATCH });
        }
        match component.kind {
            ComponentKind::Frame => {
                ops.extend(build_rectangle_at(
                    SCRATCH,
                    component.color,
                    2,
                    component.side,
                    0,
                    0,
                ));
                ops.push(Op::Copy {
                    k: 0,
                    h: SCRATCH,
                    rot: 0,
                    di: component.start as isize,
                    dj: component.start as isize,
                });
                ops.push(Op::Copy {
                    k: 0,
                    h: SCRATCH,
                    rot: 0,
                    di: (N - 2 - component.start) as isize,
                    dj: component.start as isize,
                });
                ops.push(Op::Copy {
                    k: 0,
                    h: SCRATCH,
                    rot: 1,
                    di: component.start as isize,
                    dj: component.start as isize - (N as isize - 2),
                });
                ops.push(Op::Copy {
                    k: 0,
                    h: SCRATCH,
                    rot: 1,
                    di: component.start as isize,
                    dj: -(component.start as isize),
                });
            }
            ComponentKind::Solid => {
                ops.extend(build_rectangle_at(
                    SCRATCH,
                    component.color,
                    component.side,
                    component.side,
                    0,
                    0,
                ));
                ops.push(Op::Copy {
                    k: 0,
                    h: SCRATCH,
                    rot: 0,
                    di: component.start as isize,
                    dj: component.start as isize,
                });
            }
        }
    }
    ops
}

fn build_ops_from_batches(components: &[Component], batches: &[Vec<usize>]) -> Vec<Op> {
    let mut ops = Vec::new();
    for (idx, batch) in batches.iter().enumerate() {
        if idx > 0 {
            ops.push(Op::Clear { k: SCRATCH });
        }
        for &idx in batch {
            add_component_to_scratch(&mut ops, components[idx]);
        }
        ops.push(Op::Copy {
            k: 0,
            h: SCRATCH,
            rot: 0,
            di: 0,
            dj: 0,
        });
    }
    ops
}

fn add_component_to_scratch(ops: &mut Vec<Op>, component: Component) {
    match component.kind {
        ComponentKind::Frame => {
            ops.extend(build_rectangle_at(
                SCRATCH,
                component.color,
                2,
                component.side,
                component.start,
                component.start,
            ));
            ops.push(Op::Copy {
                k: SCRATCH,
                h: SCRATCH,
                rot: 0,
                di: (component.side - 2) as isize,
                dj: 0,
            });
            ops.push(Op::Copy {
                k: SCRATCH,
                h: SCRATCH,
                rot: 1,
                di: component.start as isize,
                dj: component.start as isize - (N as isize - 2),
            });
            ops.push(Op::Copy {
                k: SCRATCH,
                h: SCRATCH,
                rot: 1,
                di: component.start as isize,
                dj: -(component.start as isize),
            });
        }
        ComponentKind::Solid => {
            ops.extend(build_rectangle_at(
                SCRATCH,
                component.color,
                component.side,
                component.side,
                component.start,
                component.start,
            ));
        }
    }
}

fn build_rectangle_at(
    layer: usize,
    color: u8,
    height: usize,
    width: usize,
    top: usize,
    left: usize,
) -> Vec<Op> {
    let mut ops = vec![Op::Paint {
        k: layer,
        i: top,
        j: left,
        color,
    }];
    for dj in interval_growth(width) {
        ops.push(Op::Copy {
            k: layer,
            h: layer,
            rot: 0,
            di: 0,
            dj: dj as isize,
        });
    }
    for di in interval_growth(height) {
        ops.push(Op::Copy {
            k: layer,
            h: layer,
            rot: 0,
            di: di as isize,
            dj: 0,
        });
    }
    ops
}

fn interval_growth(target: usize) -> Vec<usize> {
    let mut cur = 1;
    let mut deltas = Vec::new();
    while cur < target {
        let delta = cur.min(target - cur);
        deltas.push(delta);
        cur += delta;
    }
    deltas
}

fn simulate_goal(input: &Input, ops: &[Op]) -> bool {
    let mut layers = [[[0_u8; N]; N]; 2];
    for &op in ops {
        match op {
            Op::Paint { k, i, j, color } => {
                layers[k][i][j] = color;
            }
            Op::Copy { k, h, rot, di, dj } => {
                let src = layers[h];
                for (i, row) in src.iter().enumerate() {
                    for (j, &color) in row.iter().enumerate() {
                        if color == 0 {
                            continue;
                        }
                        let (ri, rj) = rotate(i as isize, j as isize, rot);
                        let ni = ri + di;
                        let nj = rj + dj;
                        if ni < 0 || nj < 0 || ni >= N as isize || nj >= N as isize {
                            return false;
                        }
                        layers[k][ni as usize][nj as usize] = color;
                    }
                }
            }
            Op::Clear { k } => {
                layers[k] = [[0_u8; N]; N];
            }
        }
    }
    layers[0] == input.goal
}

fn rotate(i: isize, j: isize, rot: usize) -> (isize, isize) {
    match rot & 3 {
        0 => (i, j),
        1 => (j, N as isize - 1 - i),
        2 => (N as isize - 1 - i, N as isize - 1 - j),
        _ => (N as isize - 1 - j, i),
    }
}

fn enumerate_ordered_batches(n: usize) -> Vec<Vec<Vec<usize>>> {
    fn permutations(cur: &mut Vec<usize>, used: &mut [bool], out: &mut Vec<Vec<usize>>) {
        if cur.len() == used.len() {
            out.push(cur.clone());
            return;
        }
        for i in 0..used.len() {
            if used[i] {
                continue;
            }
            used[i] = true;
            cur.push(i);
            permutations(cur, used, out);
            cur.pop();
            used[i] = false;
        }
    }

    let mut perms = Vec::new();
    permutations(&mut Vec::new(), &mut vec![false; n], &mut perms);

    let mut all = Vec::new();
    for perm in perms {
        let split_masks = 1_usize << (n - 1);
        for mask in 0..split_masks {
            let mut batches = Vec::new();
            let mut cur = vec![perm[0]];
            for i in 0..n - 1 {
                if ((mask >> i) & 1) != 0 {
                    batches.push(cur);
                    cur = Vec::new();
                }
                cur.push(perm[i + 1]);
            }
            batches.push(cur);
            all.push(batches);
        }
    }
    all
}
