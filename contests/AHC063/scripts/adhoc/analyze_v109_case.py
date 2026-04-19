#!/usr/bin/env python3
import argparse
import collections
import dataclasses
import heapq
import pathlib
import sys
from typing import Deque, Dict, List, Optional, Tuple

DIRS: List[Tuple[int, int, str]] = [(-1, 0, "U"), (1, 0, "D"), (0, -1, "L"), (0, 1, "R")]
DIR_TO_IDX = {d: i for i, (_, _, d) in enumerate(DIRS)}
ROOT = pathlib.Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class GameInput:
    n: int
    m: int
    c: int
    d: Tuple[int, ...]
    food: Tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class State:
    pos: Tuple[int, ...]
    colors: Tuple[int, ...]
    food: Tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class NoBiteState:
    pos: Tuple[int, ...]
    food: Tuple[int, ...]


@dataclasses.dataclass
class StepTrace:
    move: str
    ate: int
    bite_idx: Optional[int]
    length: int


@dataclasses.dataclass
class ReplayResult:
    state: State
    turn: int
    valid: bool
    invalid_reason: Optional[str]
    progress_points: List[Tuple[int, int, int]]


@dataclasses.dataclass
class SearchResult:
    found: bool
    depth: Optional[int]
    expansions: int
    terminated_by_cap: bool
    path: List[StepTrace]


@dataclasses.dataclass
class StaticTargetInfo:
    cell: int
    color: int
    reachable: bool
    reachable_goal_neighbors: int
    goal_neighbors: List[int]
    component: int


def parse_input(path: pathlib.Path) -> GameInput:
    tokens = path.read_text().split()
    it = iter(tokens)
    n = int(next(it))
    m = int(next(it))
    c = int(next(it))
    d = tuple(int(next(it)) for _ in range(m))
    food = tuple(int(next(it)) for _ in range(n * n))
    return GameInput(n=n, m=m, c=c, d=d, food=food)


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


def step(state: State, n: int, dir_idx: int) -> Optional[Tuple[State, int, Optional[int]]]:
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
            cell = new_pos[p]
            new_food[cell] = new_colors[p]
        new_pos = new_pos[: bite_idx + 1]
        new_colors = new_colors[: bite_idx + 1]

    return State(pos=new_pos, colors=tuple(new_colors), food=tuple(new_food)), ate, bite_idx


def step_no_bite(state: NoBiteState, n: int, dir_idx: int) -> Optional[Tuple[NoBiteState, int]]:
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

    ate = new_food[nh]
    if ate != 0:
        new_food[nh] = 0
        new_pos = new_pos + (state.pos[old_len - 1],)

    for idx in range(1, len(new_pos) - 1):
        if new_pos[idx] == nh:
            return None

    return NoBiteState(pos=new_pos, food=tuple(new_food)), ate


def replay(inp: GameInput, moves: List[str], turn_limit: Optional[int]) -> ReplayResult:
    n = inp.n
    state = State(
        pos=tuple(i * n for i in range(4, -1, -1)),
        colors=(1, 1, 1, 1, 1),
        food=inp.food,
    )
    progress_points: List[Tuple[int, int, int]] = []
    current_lcp = lcp(state.colors, inp.d)
    progress_points.append((0, current_lcp, len(state.colors)))

    replay_len = len(moves) if turn_limit is None else min(turn_limit, len(moves))
    for turn, mv in enumerate(moves[:replay_len]):
        nxt = step(state, n, DIR_TO_IDX[mv])
        if nxt is None:
            return ReplayResult(
                state=state,
                turn=turn,
                valid=False,
                invalid_reason=f"invalid move at turn={turn} dir={mv}",
                progress_points=progress_points,
            )
        state, _, _ = nxt
        new_lcp = lcp(state.colors, inp.d)
        if new_lcp != current_lcp:
            current_lcp = new_lcp
            progress_points.append((turn + 1, current_lcp, len(state.colors)))

    return ReplayResult(
        state=state,
        turn=replay_len,
        valid=True,
        invalid_reason=None,
        progress_points=progress_points,
    )


def score_of(inp: GameInput, state: State, turn: int) -> int:
    e = 0
    for p in range(len(state.colors)):
        if state.colors[p] != inp.d[p]:
            e += 1
    return turn + 10000 * (e + 2 * (inp.m - len(state.colors)))


def remaining_by_color(state: State) -> Dict[int, int]:
    ctr: Dict[int, int] = collections.Counter(c for c in state.food if c != 0)
    return dict(sorted(ctr.items()))


def legal_moves(state: State, n: int) -> List[Tuple[str, int, int]]:
    out: List[Tuple[str, int, int]] = []
    head = state.pos[0]
    hr, hc = rc_of(head, n)
    neck = state.pos[1] if len(state.pos) >= 2 else -1
    for dr, dc, ch in DIRS:
        nr = hr + dr
        nc = hc + dc
        if nr < 0 or nr >= n or nc < 0 or nc >= n:
            continue
        nh = cell_of(nr, nc, n)
        if nh == neck:
            continue
        out.append((ch, nh, state.food[nh]))
    return out


def neighbors(cell: int, n: int) -> List[int]:
    r, c = rc_of(cell, n)
    out: List[int] = []
    for dr, dc, _ in DIRS:
        nr = r + dr
        nc = c + dc
        if 0 <= nr < n and 0 <= nc < n:
            out.append(cell_of(nr, nc, n))
    return out


def build_static_components(state: State, n: int) -> Tuple[List[int], List[int]]:
    blocked = set(state.pos[1:-1])
    comp = [-1] * (n * n)
    sizes: List[int] = []
    cid = 0
    for start in range(n * n):
        if start in blocked or comp[start] != -1:
            continue
        q: Deque[int] = collections.deque([start])
        comp[start] = cid
        size = 0
        while q:
            cur = q.popleft()
            size += 1
            for nxt in neighbors(cur, n):
                if nxt in blocked or comp[nxt] != -1:
                    continue
                comp[nxt] = cid
                q.append(nxt)
        sizes.append(size)
        cid += 1
    return comp, sizes


def analyze_static_targets(
    inp: GameInput,
    state: State,
    target_color: int,
) -> Tuple[int, List[StaticTargetInfo], List[Tuple[int, int, int]]]:
    comp, sizes = build_static_components(state, inp.n)
    head_comp = comp[state.pos[0]]

    target_infos: List[StaticTargetInfo] = []
    target_comps = set()
    for cell, color in enumerate(state.food):
        if color != target_color:
            continue
        goal_neighbors = [x for x in neighbors(cell, inp.n) if x != state.pos[1]]
        reachable_goal_neighbors = sum(1 for x in goal_neighbors if comp[x] == head_comp)
        component = comp[cell]
        target_infos.append(
            StaticTargetInfo(
                cell=cell,
                color=color,
                reachable=component == head_comp,
                reachable_goal_neighbors=reachable_goal_neighbors,
                goal_neighbors=goal_neighbors,
                component=component,
            )
        )
        if component != -1:
            target_comps.add(component)

    barriers: List[Tuple[int, int, int]] = []
    if head_comp != -1 and target_comps:
        for idx in range(1, len(state.pos) - 1):
            cell = state.pos[idx]
            adj_comps = {comp[nxt] for nxt in neighbors(cell, inp.n) if comp[nxt] != -1}
            if head_comp in adj_comps and any(tc in adj_comps for tc in target_comps if tc != head_comp):
                barriers.append((idx, cell, state.colors[idx]))

    return sizes[head_comp], target_infos, barriers


def bfs_next_target(
    inp: GameInput,
    start: State,
    ell: int,
    depth_limit: int,
    expansion_cap: int,
) -> SearchResult:
    if ell >= inp.m:
        return SearchResult(True, 0, 0, False, [])

    target_prefix = inp.d[: ell + 1]
    q: Deque[State] = collections.deque([start])
    depth: Dict[State, int] = {start: 0}
    parent: Dict[State, Tuple[Optional[State], Optional[StepTrace]]] = {start: (None, None)}

    expansions = 0
    while q:
        cur = q.popleft()
        cur_depth = depth[cur]
        expansions += 1
        if expansions > expansion_cap:
            return SearchResult(False, None, expansions - 1, True, [])

        if len(cur.colors) >= ell + 1 and cur.colors[: ell + 1] == target_prefix:
            path: List[StepTrace] = []
            x = cur
            while True:
                prev, tr = parent[x]
                if prev is None:
                    break
                assert tr is not None
                path.append(tr)
                x = prev
            path.reverse()
            return SearchResult(True, cur_depth, expansions, False, path)

        if cur_depth >= depth_limit:
            continue

        keep = min(len(cur.colors), ell)
        if cur.colors[:keep] != inp.d[:keep]:
            continue

        for dir_idx, (_, _, ch) in enumerate(DIRS):
            nxt = step(cur, inp.n, dir_idx)
            if nxt is None:
                continue
            ns, ate, bite_idx = nxt

            keep2 = min(len(ns.colors), ell)
            if ns.colors[:keep2] != inp.d[:keep2]:
                continue

            nd = cur_depth + 1
            if depth.get(ns, sys.maxsize) <= nd:
                continue
            depth[ns] = nd
            parent[ns] = (cur, StepTrace(move=ch, ate=ate, bite_idx=bite_idx, length=len(ns.colors)))
            q.append(ns)

    return SearchResult(False, None, expansions, False, [])


def count_color(food: Tuple[int, ...], color: int) -> int:
    return sum(1 for x in food if x == color)


def nearest_target_dist(pos: Tuple[int, ...], food: Tuple[int, ...], n: int, target_color: int) -> int:
    head = pos[0]
    hr, hc = rc_of(head, n)
    best = 10**9
    for cell, color in enumerate(food):
        if color != target_color:
            continue
        tr, tc = rc_of(cell, n)
        best = min(best, abs(hr - tr) + abs(hc - tc))
    return best


def search_without_bite(
    inp: GameInput,
    start: State,
    target_color: int,
    depth_limit: int,
    expansion_cap: int,
) -> SearchResult:
    init = NoBiteState(pos=start.pos, food=start.food)
    start_target_count = count_color(init.food, target_color)
    if start_target_count == 0:
        return SearchResult(True, 0, 0, False, [])

    pq: List[Tuple[int, int, int, NoBiteState]] = []
    heapq.heappush(pq, (nearest_target_dist(init.pos, init.food, inp.n, target_color), 0, 0, init))
    dist: Dict[NoBiteState, int] = {init: 0}
    parent: Dict[NoBiteState, Tuple[Optional[NoBiteState], Optional[StepTrace]]] = {init: (None, None)}
    uid = 1
    expansions = 0

    while pq:
        _, cur_depth, _, cur = heapq.heappop(pq)
        if dist.get(cur) != cur_depth:
            continue

        expansions += 1
        if expansions > expansion_cap:
            return SearchResult(False, None, expansions - 1, True, [])

        if count_color(cur.food, target_color) < start_target_count:
            path: List[StepTrace] = []
            x = cur
            while True:
                prev, tr = parent[x]
                if prev is None:
                    break
                assert tr is not None
                path.append(tr)
                x = prev
            path.reverse()
            return SearchResult(True, cur_depth, expansions, False, path)

        if cur_depth >= depth_limit:
            continue

        for dir_idx, (_, _, ch) in enumerate(DIRS):
            nxt = step_no_bite(cur, inp.n, dir_idx)
            if nxt is None:
                continue
            ns, ate = nxt
            nd = cur_depth + 1
            if dist.get(ns, sys.maxsize) <= nd:
                continue
            dist[ns] = nd
            parent[ns] = (cur, StepTrace(move=ch, ate=ate, bite_idx=None, length=len(ns.pos)))
            h = nearest_target_dist(ns.pos, ns.food, inp.n, target_color)
            heapq.heappush(pq, (nd + h, nd, uid, ns))
            uid += 1

    return SearchResult(False, None, expansions, False, [])


def format_progress(points: List[Tuple[int, int, int]], limit: int = 12) -> str:
    if len(points) <= limit:
        return ";".join(f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in points)
    head = points[: limit // 2]
    tail = points[-(limit // 2) :]
    return ";".join(f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in head) + ";...;" + ";".join(
        f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in tail
    )


def format_path(path: List[StepTrace], limit: int = 80) -> str:
    s = "".join(x.move for x in path)
    if len(s) <= limit:
        return s
    head = s[: limit // 2]
    tail = s[-(limit // 2) :]
    return f"{head}...{tail}"


def format_events(path: List[StepTrace], limit: int = 16) -> str:
    if not path:
        return ""
    items = [f"{i+1}:{t.move}:ate{t.ate}:bite{t.bite_idx}:len{t.length}" for i, t in enumerate(path)]
    if len(items) <= limit:
        return ";".join(items)
    return ";".join(items[: limit // 2]) + ";...;" + ";".join(items[-(limit // 2) :])


def render_grid(state: State, inp: GameInput, target_color: int) -> str:
    comp, _ = build_static_components(state, inp.n)
    head_comp = comp[state.pos[0]]
    body = set(state.pos[1:-1])
    target_cells = {idx for idx, color in enumerate(state.food) if color == target_color}

    rows: List[str] = []
    for r in range(inp.n):
        row: List[str] = []
        for c in range(inp.n):
            cell = cell_of(r, c, inp.n)
            if cell == state.pos[0]:
                row.append("H")
            elif cell == state.pos[-1]:
                row.append("T")
            elif cell in body:
                row.append("#")
            elif state.food[cell] != 0:
                ch = "*" if cell in target_cells else str(state.food[cell])
                row.append(ch)
            elif comp[cell] == head_comp:
                row.append(" ")
            else:
                row.append(".")
        rows.append("".join(row))
    return "\n".join(rows)


def main() -> int:
    ap = argparse.ArgumentParser(description="Analyze why v109 stops on a specific case/turn")
    ap.add_argument("case", help="case id or case file (e.g. 0068 or 0068.txt)")
    ap.add_argument("--input-dir", default=str(ROOT / "tools" / "in"))
    ap.add_argument("--output", default="")
    ap.add_argument("--out-dir", default=str(ROOT / "results" / "out" / "v109_pro_suffix_opt"))
    ap.add_argument("--turn", type=int, default=-1, help="1-origin turn count to replay; -1 means full output")
    ap.add_argument("--depth-limit", type=int, default=28, help="prefix-preserving BFS depth limit")
    ap.add_argument("--cap-small", type=int, default=50000)
    ap.add_argument("--cap-large", type=int, default=250000)
    ap.add_argument("--no-bite-depth", type=int, default=80)
    ap.add_argument("--no-bite-cap", type=int, default=300000)
    args = ap.parse_args()

    case_file = case_to_file(args.case)
    input_path = pathlib.Path(args.input_dir) / case_file
    output_path = pathlib.Path(args.output) if args.output else pathlib.Path(args.out_dir) / case_file

    if not input_path.exists():
        print(f"error: input not found: {input_path}", file=sys.stderr)
        return 1
    if not output_path.exists():
        print(f"error: output not found: {output_path}", file=sys.stderr)
        return 1

    inp = parse_input(input_path)
    moves = parse_output(output_path)
    turn_limit = None if args.turn < 0 else args.turn
    rep = replay(inp, moves, turn_limit)

    print(f"case={case_file}")
    print(f"input={input_path}")
    print(f"output={output_path}")
    print(f"requested_turn={'full' if turn_limit is None else turn_limit}")
    print(f"replayed_turn={rep.turn}")
    print(f"replay_valid={str(rep.valid).lower()}")
    if not rep.valid:
        print(f"invalid_reason={rep.invalid_reason}")
        return 0

    st = rep.state
    final_lcp = lcp(st.colors, inp.d)
    next_color = inp.d[final_lcp] if final_lcp < inp.m else -1
    rem = remaining_by_color(st)
    legal = legal_moves(st, inp.n)
    score = score_of(inp, st, rep.turn)
    head_r, head_c = rc_of(st.pos[0], inp.n)
    neck_r, neck_c = rc_of(st.pos[1], inp.n)

    print(f"length={len(st.colors)}")
    print(f"lcp={final_lcp}")
    print(f"next_target_color={next_color}")
    if final_lcp < inp.m:
        print("target_suffix=" + ",".join(map(str, inp.d[final_lcp:])))
    print(f"score_recomputed={score}")
    print(f"remaining_food_count={sum(rem.values())}")
    print("remaining_by_color=" + ",".join(f"{k}:{v}" for k, v in rem.items()))
    print(f"head=({head_r},{head_c}) neck=({neck_r},{neck_c})")
    print(
        "legal_moves="
        + ",".join(f"{ch}:{val}@({rc_of(cell, inp.n)[0]},{rc_of(cell, inp.n)[1]})" for ch, cell, val in legal)
    )
    print(f"physically_stuck={str(len(legal) == 0).lower()}")
    print(f"lcp_progress={format_progress(rep.progress_points)}")

    if final_lcp < inp.m:
        comp_size, target_infos, barriers = analyze_static_targets(inp, st, next_color)
        print(f"static_head_component_size={comp_size}")
        print(
            "static_targets="
            + ";".join(
                f"({rc_of(x.cell, inp.n)[0]},{rc_of(x.cell, inp.n)[1]}):reachable={str(x.reachable).lower()}:goal_neighbors_reachable={x.reachable_goal_neighbors}/{len(x.goal_neighbors)}:component={x.component}"
                for x in target_infos
            )
        )
        if barriers:
            print(
                "static_barrier_segments="
                + ";".join(
                    f"idx{idx}@({rc_of(cell, inp.n)[0]},{rc_of(cell, inp.n)[1]}):color{color}"
                    for idx, cell, color in barriers[:12]
                )
            )
        else:
            print("static_barrier_segments=")

        no_bite = search_without_bite(inp, st, next_color, args.no_bite_depth, args.no_bite_cap)
        print(
            f"no_bite_search(found={str(no_bite.found).lower()},depth={no_bite.depth},expansions={no_bite.expansions},cap_hit={str(no_bite.terminated_by_cap).lower()})"
        )
        if no_bite.found:
            print(f"no_bite_path={format_path(no_bite.path)}")
            print(f"no_bite_events={format_events(no_bite.path)}")

        small = bfs_next_target(inp, st, final_lcp, args.depth_limit, args.cap_small)
        print(
            f"prefix_bfs_small(found={str(small.found).lower()},depth={small.depth},expansions={small.expansions},cap_hit={str(small.terminated_by_cap).lower()})"
        )
        large = bfs_next_target(inp, st, final_lcp, args.depth_limit, args.cap_large)
        print(
            f"prefix_bfs_large(found={str(large.found).lower()},depth={large.depth},expansions={large.expansions},cap_hit={str(large.terminated_by_cap).lower()})"
        )
        if large.found:
            print(f"prefix_bfs_path={format_path(large.path)}")
            print(f"prefix_bfs_events={format_events(large.path)}")

    print("grid_legend=H:head,T:tail,#:body,*:next-target-color food,x:other food,space:static head component,.:other component")
    print("grid_begin")
    print(render_grid(st, inp, next_color if final_lcp < inp.m else -1))
    print("grid_end")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
