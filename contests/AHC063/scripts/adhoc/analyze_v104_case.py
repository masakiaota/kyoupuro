#!/usr/bin/env python3
import argparse
import collections
import dataclasses
import pathlib
import subprocess
import sys
from typing import Dict, List, Optional, Tuple

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
    pos: Tuple[int, ...]      # head -> tail
    colors: Tuple[int, ...]   # head -> tail
    food: Tuple[int, ...]      # flattened board


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
    progress_points: List[Tuple[int, int, int]]  # (turn, lcp, len)


@dataclasses.dataclass
class BfsResult:
    found: bool
    depth: Optional[int]
    expansions: int
    terminated_by_cap: bool
    path: List[StepTrace]


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


def lcp(colors: Tuple[int, ...], d: Tuple[int, ...]) -> int:
    i = 0
    lim = min(len(colors), len(d))
    while i < lim and colors[i] == d[i]:
        i += 1
    return i


def rc_of(cell: int, n: int) -> Tuple[int, int]:
    return divmod(cell, n)


def step(state: State, n: int, dir_idx: int) -> Optional[Tuple[State, int, Optional[int]]]:
    dr, dc, _ = DIRS[dir_idx]
    head = state.pos[0]
    hr, hc = rc_of(head, n)
    nr = hr + dr
    nc = hc + dc
    if nr < 0 or nr >= n or nc < 0 or nc >= n:
        return None
    nh = nr * n + nc
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


def replay(inp: GameInput, moves: List[str]) -> ReplayResult:
    n = inp.n
    state = State(
        pos=tuple(i * n for i in range(4, -1, -1)),
        colors=(1, 1, 1, 1, 1),
        food=inp.food,
    )
    progress_points: List[Tuple[int, int, int]] = []
    current_lcp = lcp(state.colors, inp.d)
    progress_points.append((0, current_lcp, len(state.colors)))

    for turn, mv in enumerate(moves):
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
        turn=len(moves),
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
        nh = nr * n + nc
        if nh == neck:
            continue
        out.append((ch, nh, state.food[nh]))
    return out


def remaining_by_color(state: State) -> Dict[int, int]:
    ctr: Dict[int, int] = collections.Counter(c for c in state.food if c != 0)
    return dict(sorted(ctr.items()))


def bfs_next_target(
    inp: GameInput,
    start: State,
    ell: int,
    depth_limit: int,
    expansion_cap: int,
) -> BfsResult:
    if ell >= inp.m:
        return BfsResult(True, 0, 0, False, [])

    target_prefix = inp.d[: ell + 1]

    q = collections.deque()
    q.append(start)
    depth: Dict[State, int] = {start: 0}
    parent: Dict[State, Tuple[Optional[State], Optional[StepTrace]]] = {start: (None, None)}

    expansions = 0

    while q:
        cur = q.popleft()
        cur_depth = depth[cur]
        expansions += 1
        if expansions > expansion_cap:
            return BfsResult(False, None, expansions - 1, True, [])

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
            return BfsResult(True, cur_depth, expansions, False, path)

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

            old_d = depth.get(ns)
            if old_d is not None and old_d <= cur_depth + 1:
                continue
            depth[ns] = cur_depth + 1
            parent[ns] = (cur, StepTrace(move=ch, ate=ate, bite_idx=bite_idx, length=len(ns.colors)))
            q.append(ns)

    return BfsResult(False, None, expansions, False, [])


def format_progress(points: List[Tuple[int, int, int]], limit: int = 12) -> str:
    if len(points) <= limit:
        return ";".join(f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in points)
    head = points[: limit // 2]
    tail = points[-(limit // 2) :]
    return ";".join(f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in head) + ";...;" + ";".join(
        f"t{t}:lcp{lc}:len{ln}" for t, lc, ln in tail
    )


def maybe_run_solver(solver_bin: pathlib.Path, input_path: pathlib.Path, output_path: pathlib.Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with input_path.open("rb") as fin, output_path.open("wb") as fout:
        cp = subprocess.run([str(solver_bin)], stdin=fin, stdout=fout, stderr=subprocess.PIPE)
    if cp.returncode != 0:
        err = cp.stderr.decode(errors="replace")
        raise RuntimeError(f"solver failed: code={cp.returncode}\n{err}")


def case_to_file(case_arg: str) -> str:
    return case_arg if case_arg.endswith(".txt") else f"{case_arg}.txt"


def main() -> int:
    ap = argparse.ArgumentParser(description="Analyze v104 case output and stop reason")
    ap.add_argument("case", help="case id or case file (e.g. 0062 or 0062.txt)")
    ap.add_argument("--input-dir", default=str(ROOT / "tools" / "in"))
    ap.add_argument("--output", default="")
    ap.add_argument("--out-dir", default=str(ROOT / "results" / "out" / "v104_pro_incremental_opt"))
    ap.add_argument("--rerun", action="store_true", help="rerun current v104 solver and overwrite output")
    ap.add_argument("--solver-bin", default=str(ROOT / "target" / "release" / "v104_pro_incremental_opt"))
    ap.add_argument("--depth-limit", type=int, default=24)
    ap.add_argument("--cap-small", type=int, default=25000)
    ap.add_argument("--cap-large", type=int, default=300000)
    args = ap.parse_args()

    case_file = case_to_file(args.case)
    input_path = pathlib.Path(args.input_dir) / case_file
    if args.output:
        output_path = pathlib.Path(args.output)
    else:
        output_path = pathlib.Path(args.out_dir) / case_file

    if not input_path.exists():
        print(f"error: input not found: {input_path}", file=sys.stderr)
        return 1

    if args.rerun:
        solver_bin = pathlib.Path(args.solver_bin)
        if not solver_bin.exists():
            print(f"error: solver bin not found: {solver_bin}", file=sys.stderr)
            return 1
        maybe_run_solver(solver_bin, input_path, output_path)

    if not output_path.exists():
        print(f"error: output not found: {output_path}", file=sys.stderr)
        return 1

    inp = parse_input(input_path)
    moves = parse_output(output_path)
    rep = replay(inp, moves)

    st = rep.state
    final_lcp = lcp(st.colors, inp.d)
    next_color = inp.d[final_lcp] if final_lcp < inp.m else -1
    rem = remaining_by_color(st)
    legal = legal_moves(st, inp.n)
    score = score_of(inp, st, rep.turn)

    head_r, head_c = rc_of(st.pos[0], inp.n)
    neck_r, neck_c = rc_of(st.pos[1], inp.n)

    print(f"case={case_file}")
    print(f"input={input_path}")
    print(f"output={output_path}")
    print(f"turns={rep.turn}")
    print(f"replay_valid={str(rep.valid).lower()}")
    if not rep.valid:
        print(f"invalid_reason={rep.invalid_reason}")
        return 0

    print(f"length={len(st.colors)}")
    print(f"lcp={final_lcp}")
    print(f"next_target_color={next_color}")
    print(f"score_recomputed={score}")
    print(f"remaining_food_count={sum(rem.values())}")
    print("remaining_by_color=" + ",".join(f"{k}:{v}" for k, v in rem.items()))
    print(f"head=({head_r},{head_c}) neck=({neck_r},{neck_c})")
    print(
        "legal_moves="
        + ",".join(
            f"{ch}:{val}@({rc_of(cell, inp.n)[0]},{rc_of(cell, inp.n)[1]})" for ch, cell, val in legal
        )
    )
    print(f"physically_stuck={str(len(legal) == 0).lower()}")
    print(f"lcp_progress={format_progress(rep.progress_points)}")

    if final_lcp < inp.m:
        small = bfs_next_target(inp, st, final_lcp, args.depth_limit, args.cap_small)
        print(
            f"bfs_small(found={str(small.found).lower()},depth={small.depth},expansions={small.expansions},cap_hit={str(small.terminated_by_cap).lower()})"
        )
        large = bfs_next_target(inp, st, final_lcp, args.depth_limit, args.cap_large)
        print(
            f"bfs_large(found={str(large.found).lower()},depth={large.depth},expansions={large.expansions},cap_hit={str(large.terminated_by_cap).lower()})"
        )
        if large.found:
            path = "".join(t.move for t in large.path)
            print(f"bfs_path={path}")
            events = ";".join(
                f"{i+1}:{t.move}:ate{t.ate}:bite{t.bite_idx}:len{t.length}" for i, t in enumerate(large.path)
            )
            print(f"bfs_events={events}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
