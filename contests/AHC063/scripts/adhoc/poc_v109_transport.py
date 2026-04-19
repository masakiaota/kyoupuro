#!/usr/bin/env python3
import argparse
import collections
import dataclasses
import heapq
import pathlib
import sys
from typing import Deque, Dict, FrozenSet, List, Optional, Tuple

DIRS: List[Tuple[int, int, str]] = [(-1, 0, "U"), (1, 0, "D"), (0, -1, "L"), (0, 1, "R")]
DIR_TO_IDX = {d: i for i, (_, _, d) in enumerate(DIRS)}
ROOT = pathlib.Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class InputData:
    n: int
    m: int
    d: Tuple[int, ...]
    food: Tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class FullState:
    pos: Tuple[int, ...]
    colors: Tuple[int, ...]
    food: Tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class ReducedState:
    pos: Tuple[int, ...]
    generic_food: FrozenSet[int]
    target_present: bool


@dataclasses.dataclass
class SearchNode:
    state: ReducedState
    depth: int
    parent: Optional[int]
    move: str
    ate_generic: bool
    ate_target: bool
    bite_idx: Optional[int]


def parse_input(path: pathlib.Path) -> InputData:
    tokens = path.read_text().split()
    it = iter(tokens)
    n = int(next(it))
    m = int(next(it))
    _c = int(next(it))
    d = tuple(int(next(it)) for _ in range(m))
    food = tuple(int(next(it)) for _ in range(n * n))
    return InputData(n=n, m=m, d=d, food=food)


def parse_output(path: pathlib.Path) -> List[str]:
    out: List[str] = []
    for line in path.read_text().splitlines():
        s = line.strip()
        if not s:
            continue
        if s not in DIR_TO_IDX:
            raise ValueError(f"invalid direction token: {s}")
        out.append(s)
    return out


def case_to_file(case_arg: str) -> str:
    return case_arg if case_arg.endswith(".txt") else f"{case_arg}.txt"


def rc_of(cell: int, n: int) -> Tuple[int, int]:
    return divmod(cell, n)


def cell_of(r: int, c: int, n: int) -> int:
    return r * n + c


def lcp(colors: Tuple[int, ...], d: Tuple[int, ...]) -> int:
    i = 0
    lim = min(len(colors), len(d))
    while i < lim and colors[i] == d[i]:
        i += 1
    return i


def step_full(state: FullState, n: int, dir_idx: int) -> Optional[Tuple[FullState, int, Optional[int]]]:
    dr, dc, _ = DIRS[dir_idx]
    head = state.pos[0]
    hr, hc = rc_of(head, n)
    nr = hr + dr
    nc = hc + dc
    if nr < 0 or nr >= n or nc < 0 or nc >= n:
        return None
    nh = cell_of(nr, nc, n)
    if len(state.pos) >= 2 and nh == state.pos[1]:
        return None

    old_len = len(state.pos)
    new_food = list(state.food)
    new_pos = (nh,) + state.pos[: old_len - 1]
    new_colors = list(state.colors)

    ate = new_food[nh]
    if ate != 0:
        new_food[nh] = 0
        new_pos = new_pos + (state.pos[old_len - 1],)
        new_colors.append(ate)

    bite_idx: Optional[int] = None
    for idx in range(1, len(new_pos) - 1):
        if new_pos[idx] == nh:
            bite_idx = idx
            break

    if bite_idx is not None:
        for p in range(bite_idx + 1, len(new_pos)):
            new_food[new_pos[p]] = new_colors[p]
        new_pos = new_pos[: bite_idx + 1]
        new_colors = new_colors[: bite_idx + 1]

    return FullState(pos=new_pos, colors=tuple(new_colors), food=tuple(new_food)), ate, bite_idx


def replay_to_turn(inp: InputData, moves: List[str], turn: int) -> FullState:
    state = FullState(
        pos=tuple(i * inp.n for i in range(4, -1, -1)),
        colors=(1, 1, 1, 1, 1),
        food=inp.food,
    )
    for mv in moves[:turn]:
        nxt = step_full(state, inp.n, DIR_TO_IDX[mv])
        if nxt is None:
            raise ValueError(f"invalid replay at turn={mv}")
        state, _, _ = nxt
    return state


def find_target_cell(state: FullState, target_color: int) -> int:
    cells = [idx for idx, col in enumerate(state.food) if col == target_color]
    if len(cells) != 1:
        raise ValueError(f"expected exactly one target-color cell, got {len(cells)}")
    return cells[0]


def to_reduced(state: FullState, target_cell: int) -> ReducedState:
    generic = frozenset(idx for idx, col in enumerate(state.food) if col != 0 and idx != target_cell)
    return ReducedState(pos=state.pos, generic_food=generic, target_present=state.food[target_cell] != 0)


def reachable_goal_neighbors_count(state: ReducedState, n: int, target_cell: int) -> int:
    blocked = set(state.pos[1:-1])
    head = state.pos[0]
    q: Deque[int] = collections.deque([head])
    seen = {head}
    while q:
        cur = q.popleft()
        r, c = rc_of(cur, n)
        for dr, dc, _ in DIRS:
            nr = r + dr
            nc = c + dc
            if nr < 0 or nr >= n or nc < 0 or nc >= n:
                continue
            nxt = cell_of(nr, nc, n)
            if nxt in blocked or nxt in seen:
                continue
            seen.add(nxt)
            q.append(nxt)

    cnt = 0
    neck = state.pos[1] if len(state.pos) >= 2 else -1
    tr, tc = rc_of(target_cell, n)
    for dr, dc, _ in DIRS:
        nr = tr + dr
        nc = tc + dc
        if nr < 0 or nr >= n or nc < 0 or nc >= n:
            continue
        nxt = cell_of(nr, nc, n)
        if nxt == neck:
            continue
        if nxt in seen:
            cnt += 1
    return cnt


def heuristic(state: ReducedState, n: int, target_cell: int, ell: int) -> Tuple[int, int, int, int]:
    head = state.pos[0]
    hr, hc = rc_of(head, n)
    tr, tc = rc_of(target_cell, n)
    manhattan = abs(hr - tr) + abs(hc - tc)
    gate = reachable_goal_neighbors_count(state, n, target_cell)
    extra = len(state.pos) - ell
    return (0 if gate > 0 else 1, abs(extra), manhattan, 0 if extra == 0 else 1)


def legal_goal_now(state: ReducedState, n: int, target_cell: int, ell: int) -> bool:
    if not state.target_present or len(state.pos) != ell:
        return False
    head = state.pos[0]
    neck = state.pos[1] if len(state.pos) >= 2 else -1
    hr, hc = rc_of(head, n)
    tr, tc = rc_of(target_cell, n)
    if abs(hr - tr) + abs(hc - tc) != 1:
        return False
    return target_cell != neck


def step_reduced(
    state: ReducedState,
    n: int,
    dir_idx: int,
    target_cell: int,
    ell: int,
) -> Optional[Tuple[ReducedState, bool, bool, Optional[int]]]:
    dr, dc, _ = DIRS[dir_idx]
    head = state.pos[0]
    hr, hc = rc_of(head, n)
    nr = hr + dr
    nc = hc + dc
    if nr < 0 or nr >= n or nc < 0 or nc >= n:
        return None
    nh = cell_of(nr, nc, n)
    if len(state.pos) >= 2 and nh == state.pos[1]:
        return None

    ate_target = state.target_present and nh == target_cell
    if ate_target and len(state.pos) != ell:
        return None
    ate_generic = nh in state.generic_food

    new_food = set(state.generic_food)
    target_present = state.target_present
    old_len = len(state.pos)
    new_pos = (nh,) + state.pos[: old_len - 1]

    if ate_target:
        target_present = False
        new_pos = new_pos + (state.pos[old_len - 1],)
    elif ate_generic:
        new_food.remove(nh)
        new_pos = new_pos + (state.pos[old_len - 1],)

    bite_idx: Optional[int] = None
    for idx in range(1, len(new_pos) - 1):
        if new_pos[idx] == nh:
            bite_idx = idx
            break

    if bite_idx is not None:
        if bite_idx + 1 < ell:
            return None
        for p in range(bite_idx + 1, len(new_pos)):
            cell = new_pos[p]
            if cell == target_cell and target_present:
                return None
            new_food.add(cell)
        new_pos = new_pos[: bite_idx + 1]

    return (
        ReducedState(pos=new_pos, generic_food=frozenset(new_food), target_present=target_present),
        ate_generic,
        ate_target,
        bite_idx,
    )


def reconstruct(nodes: List[SearchNode], idx: int) -> str:
    rev: List[str] = []
    while nodes[idx].parent is not None:
        rev.append(nodes[idx].move)
        idx = nodes[idx].parent
    rev.reverse()
    return "".join(rev)


def search_empty_finish(
    start: ReducedState,
    n: int,
    target_cell: int,
    ell: int,
    depth_limit: int,
    expansion_cap: int,
) -> Tuple[Optional[str], int]:
    q: Deque[Tuple[int, ...]] = collections.deque([start.pos])
    depth: Dict[Tuple[int, ...], int] = {start.pos: 0}
    parent: Dict[Tuple[int, ...], Tuple[Optional[Tuple[int, ...]], str]] = {start.pos: (None, "")}
    expansions = 0

    while q:
        cur = q.popleft()
        cur_depth = depth[cur]
        expansions += 1
        if expansions > expansion_cap:
            return None, expansions - 1
        if cur_depth >= depth_limit:
            continue

        for dir_idx, (_, _, ch) in enumerate(DIRS):
            dr, dc, _ = DIRS[dir_idx]
            hr, hc = rc_of(cur[0], n)
            nr = hr + dr
            nc = hc + dc
            if nr < 0 or nr >= n or nc < 0 or nc >= n:
                continue
            nh = cell_of(nr, nc, n)
            if len(cur) >= 2 and nh == cur[1]:
                continue
            if nh == target_cell:
                if len(cur) != ell or not start.target_present:
                    continue
                path = [ch]
                x = cur
                while parent[x][0] is not None:
                    px, mv = parent[x]
                    path.append(mv)
                    assert px is not None
                    x = px
                path.reverse()
                return "".join(path), expansions
            if nh in start.generic_food:
                continue
            nxt = (nh,) + cur[:-1]
            collide = False
            for idx in range(1, len(nxt) - 1):
                if nxt[idx] == nh:
                    collide = True
                    break
            if collide:
                continue
            if nxt in depth:
                continue
            depth[nxt] = cur_depth + 1
            parent[nxt] = (cur, ch)
            q.append(nxt)

    return None, expansions


def search_transport(
    start: ReducedState,
    n: int,
    target_cell: int,
    ell: int,
    depth_limit: int,
    max_extra_len: int,
    expansion_cap: int,
) -> Tuple[Optional[str], int, int, Optional[str], Tuple[int, int, int, int], Optional[ReducedState]]:
    nodes = [SearchNode(state=start, depth=0, parent=None, move="", ate_generic=False, ate_target=False, bite_idx=None)]
    pq: List[Tuple[Tuple[int, int, int, int], int, int, int]] = []
    heapq.heappush(pq, (heuristic(start, n, target_cell, ell), 0, 0, 0))
    best_depth: Dict[ReducedState, int] = {start: 0}
    uid = 1
    expansions = 0
    best_open_idx: Optional[int] = None
    best_open_key = (-1, -10**9, -10**9, -10**9)

    while pq:
        _, depth, _, idx = heapq.heappop(pq)
        node = nodes[idx]
        if best_depth.get(node.state) != depth:
            continue

        gate = reachable_goal_neighbors_count(node.state, n, target_cell)
        hr, hc = rc_of(node.state.pos[0], n)
        tr, tc = rc_of(target_cell, n)
        open_key = (gate, -abs(len(node.state.pos) - ell), -(abs(hr - tr) + abs(hc - tc)), -depth)
        if best_open_idx is None or open_key > best_open_key:
            best_open_idx = idx
            best_open_key = open_key

        if legal_goal_now(node.state, n, target_cell, ell):
            for dir_idx, (_, _, ch) in enumerate(DIRS):
                res = step_reduced(node.state, n, dir_idx, target_cell, ell)
                if res is None:
                    continue
                ns, ate_generic, ate_target, bite_idx = res
                if ate_target and len(ns.pos) == ell + 1:
                    child = len(nodes)
                    nodes.append(
                        SearchNode(
                            state=ns,
                            depth=depth + 1,
                            parent=idx,
                            move=ch,
                            ate_generic=ate_generic,
                            ate_target=ate_target,
                            bite_idx=bite_idx,
                        )
                    )
                    best_open_seq = reconstruct(nodes, best_open_idx) if best_open_idx is not None else None
                    best_open_state = nodes[best_open_idx].state if best_open_idx is not None else None
                    return reconstruct(nodes, child), expansions, len(best_depth), best_open_seq, best_open_key, best_open_state

        if depth >= depth_limit:
            continue

        expansions += 1
        if expansions > expansion_cap:
            break

        for dir_idx, (_, _, ch) in enumerate(DIRS):
            res = step_reduced(node.state, n, dir_idx, target_cell, ell)
            if res is None:
                continue
            ns, ate_generic, ate_target, bite_idx = res
            if len(ns.pos) > ell + max_extra_len:
                continue
            if ate_target:
                continue
            nd = depth + 1
            if best_depth.get(ns, sys.maxsize) <= nd:
                continue
            best_depth[ns] = nd
            child = len(nodes)
            nodes.append(
                SearchNode(
                    state=ns,
                    depth=nd,
                    parent=idx,
                    move=ch,
                    ate_generic=ate_generic,
                    ate_target=ate_target,
                    bite_idx=bite_idx,
                )
            )
            h = heuristic(ns, n, target_cell, ell)
            heapq.heappush(pq, (h, nd, uid, child))
            uid += 1

    best_open_seq = reconstruct(nodes, best_open_idx) if best_open_idx is not None else None
    best_open_state = nodes[best_open_idx].state if best_open_idx is not None else None
    return None, expansions, len(best_depth), best_open_seq, best_open_key, best_open_state


def main() -> int:
    ap = argparse.ArgumentParser(description="PoC search for transport-and-cut maneuver on v109 terminal states")
    ap.add_argument("case", help="case id or case file")
    ap.add_argument("--turn", type=int, default=681)
    ap.add_argument("--input-dir", default=str(ROOT / "tools" / "in"))
    ap.add_argument("--out-dir", default=str(ROOT / "results" / "out" / "v109_pro_suffix_opt"))
    ap.add_argument("--depth-limit", type=int, default=28)
    ap.add_argument("--max-extra-len", type=int, default=8)
    ap.add_argument("--expansion-cap", type=int, default=120000)
    ap.add_argument("--empty-finish-depth", type=int, default=64)
    ap.add_argument("--empty-finish-cap", type=int, default=200000)
    args = ap.parse_args()

    case_file = case_to_file(args.case)
    input_path = pathlib.Path(args.input_dir) / case_file
    output_path = pathlib.Path(args.out_dir) / case_file
    if not input_path.exists():
        print(f"error: input not found: {input_path}", file=sys.stderr)
        return 1
    if not output_path.exists():
        print(f"error: output not found: {output_path}", file=sys.stderr)
        return 1

    inp = parse_input(input_path)
    moves = parse_output(output_path)
    full = replay_to_turn(inp, moves, args.turn)
    ell = lcp(full.colors, inp.d)
    if ell != len(full.colors):
        print(f"error: terminal state is not exact prefix, ell={ell}, len={len(full.colors)}", file=sys.stderr)
        return 1
    if ell >= inp.m:
        print("already complete")
        return 0

    target_color = inp.d[ell]
    target_cell = find_target_cell(full, target_color)
    start = to_reduced(full, target_cell)
    seq, expansions, visited, best_open_seq, best_open_key, best_open_state = search_transport(
        start,
        inp.n,
        target_cell,
        ell,
        args.depth_limit,
        args.max_extra_len,
        args.expansion_cap,
    )

    print(f"case={case_file}")
    print(f"turn={args.turn}")
    print(f"ell={ell}")
    print(f"target_color={target_color}")
    print(f"target_cell=({rc_of(target_cell, inp.n)[0]},{rc_of(target_cell, inp.n)[1]})")
    print(f"depth_limit={args.depth_limit}")
    print(f"max_extra_len={args.max_extra_len}")
    print(f"expansions={expansions}")
    print(f"visited={visited}")
    if seq is None:
        print("found=false")
    else:
        print("found=true")
        print(f"sequence={seq}")
        print(f"sequence_len={len(seq)}")
    gate, neg_len_fit, neg_dist, neg_depth = best_open_key
    print(f"best_gate_open_neighbors={gate}")
    print(f"best_gate_len_gap={-neg_len_fit}")
    print(f"best_gate_head_target_manhattan={-neg_dist}")
    print(f"best_gate_depth={-neg_depth}")
    if best_open_seq is not None:
        print(f"best_gate_sequence={best_open_seq}")
        print(f"best_gate_sequence_len={len(best_open_seq)}")
    if best_open_state is not None:
        finish_seq, finish_exp = search_empty_finish(
            best_open_state,
            inp.n,
            target_cell,
            ell,
            args.empty_finish_depth,
            args.empty_finish_cap,
        )
        print(f"empty_finish_expansions={finish_exp}")
        if finish_seq is None:
            print("empty_finish_found=false")
        else:
            print("empty_finish_found=true")
            print(f"empty_finish_sequence={finish_seq}")
            print(f"empty_finish_sequence_len={len(finish_seq)}")
            if best_open_seq is not None:
                full_seq = best_open_seq + finish_seq
                print(f"combined_sequence={full_seq}")
                print(f"combined_sequence_len={len(full_seq)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
