import itertools
import random
import subprocess
import sys
from pathlib import Path


def run_one(n: int, q: int, hidden: str) -> int:
    # hidden: light -> heavy order
    rank = {c: i for i, c in enumerate(hidden)}

    here = Path(__file__).resolve().parent
    proc = subprocess.Popen(
        [sys.executable, "main.py"],
        cwd=here,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    assert proc.stdin is not None
    assert proc.stdout is not None

    proc.stdin.write(f"{n} {q}\n")
    proc.stdin.flush()

    queries = 0
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("program terminated unexpectedly")
        line = line.strip()
        if not line:
            continue
        if line[0] == "?":
            parts = line.split()
            if len(parts) != 3:
                raise RuntimeError(f"invalid query line: {line!r}")
            a, b = parts[1], parts[2]
            if len(a) != 1 or len(b) != 1 or a == b:
                raise RuntimeError(f"invalid query args: {line!r}")
            if a not in rank or b not in rank:
                raise RuntimeError(f"out of range label: {line!r}")
            queries += 1
            if queries > q:
                raise RuntimeError(f"too many queries: {queries} > {q}")
            ans = "<" if rank[a] < rank[b] else ">"
            proc.stdin.write(ans + "\n")
            proc.stdin.flush()
            continue

        if line[0] == "!":
            parts = line.split()
            if len(parts) != 2:
                raise RuntimeError(f"invalid answer line: {line!r}")
            got = parts[1]
            if got != hidden:
                raise RuntimeError(f"wrong answer: got={got} expected={hidden}")
            break

        raise RuntimeError(f"unexpected output: {line!r}")

    proc.stdin.close()
    proc.stdout.close()
    proc.wait(timeout=2)
    return queries


def main() -> None:
    # Exhaustive check for N=5 (Q=7)
    n, q = 5, 7
    mx = 0
    for perm in itertools.permutations([chr(ord("A") + i) for i in range(n)]):
        hidden = "".join(perm)
        used = run_one(n, q, hidden)
        mx = max(mx, used)
    print(f"N=5 ok. max_queries={mx}")

    # Random check for N=26 (Q=100)
    n, q = 26, 100
    letters = [chr(ord("A") + i) for i in range(n)]
    mx = 0
    for _ in range(200):
        random.shuffle(letters)
        hidden = "".join(letters)
        used = run_one(n, q, hidden)
        mx = max(mx, used)
    print(f"N=26 ok. max_queries={mx}")


if __name__ == "__main__":
    main()

