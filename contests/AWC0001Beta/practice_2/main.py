import itertools
import sys
from functools import lru_cache


def main() -> None:
    line = sys.stdin.readline().strip()
    if not line:
        return
    n, q = map(int, line.split())
    _ = q  # upper bound; we just ensure our strategy stays within it.

    def is_lighter(a: str, b: str) -> bool:
        # Returns True iff weight(a) < weight(b).
        sys.stdout.write(f"? {a} {b}\n")
        sys.stdout.flush()
        ans = sys.stdin.readline().strip()
        if ans == "<":
            return True
        if ans == ">":
            return False
        # If the judge returns something unexpected (e.g. on protocol error), stop.
        raise SystemExit(0)

    if n == 5:
        ans = _solve_n5(is_lighter)
    else:
        labels = [chr(ord("A") + i) for i in range(n)]
        ans = "".join(_merge_sort(labels, is_lighter))

    sys.stdout.write(f"! {ans}\n")
    sys.stdout.flush()


def _merge_sort(items: list[str], is_lighter) -> list[str]:
    if len(items) <= 1:
        return items
    mid = len(items) // 2
    left = _merge_sort(items[:mid], is_lighter)
    right = _merge_sort(items[mid:], is_lighter)
    return _merge(left, right, is_lighter)


def _merge(left: list[str], right: list[str], is_lighter) -> list[str]:
    res: list[str] = []
    i = 0
    j = 0
    while i < len(left) and j < len(right):
        if is_lighter(left[i], right[j]):
            res.append(left[i])
            i += 1
        else:
            res.append(right[j])
            j += 1
    if i < len(left):
        res.extend(left[i:])
    if j < len(right):
        res.extend(right[j:])
    return res


def _solve_n5(is_lighter) -> str:
    # For N=5, Q=7. Build an optimal adaptive decision tree by DP over
    # the remaining possible permutations (bitmask over 120 permutations).
    n = 5
    perms = list(itertools.permutations(range(n)))
    full_mask = (1 << len(perms)) - 1

    # before[i][j] is a bitmask of permutations where i appears before j (i is lighter).
    before = [[0] * n for _ in range(n)]
    for idx, perm in enumerate(perms):
        pos = [0] * n
        for p, x in enumerate(perm):
            pos[x] = p
        bit = 1 << idx
        for i in range(n):
            pi = pos[i]
            for j in range(n):
                if i == j:
                    continue
                if pi < pos[j]:
                    before[i][j] |= bit

    @lru_cache(maxsize=None)
    def best(mask: int) -> tuple[int, tuple[int, int]]:
        # Returns (min_worst_depth, best_pair) for the given candidate set.
        if mask & (mask - 1) == 0:
            return 0, (-1, -1)

        best_depth = 10**9
        best_pair = (-1, -1)
        for i in range(n):
            for j in range(i + 1, n):
                m_ij = mask & before[i][j]
                m_ji = mask & before[j][i]
                if m_ij == 0 or m_ji == 0:
                    # Already implied by constraints; asking wastes a query.
                    continue
                d1, _ = best(m_ij)
                d2, _ = best(m_ji)
                d = 1 + max(d1, d2)
                if d < best_depth:
                    best_depth = d
                    best_pair = (i, j)
        return best_depth, best_pair

    mask = full_mask
    while mask & (mask - 1):
        _, (i, j) = best(mask)
        a = chr(ord("A") + i)
        b = chr(ord("A") + j)
        if is_lighter(a, b):
            mask &= before[i][j]
        else:
            mask &= before[j][i]

    idx = mask.bit_length() - 1
    perm = perms[idx]
    return "".join(chr(ord("A") + x) for x in perm)


if __name__ == "__main__":
    main()

