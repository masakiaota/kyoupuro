import sys


def main() -> None:
    it = iter(sys.stdin.buffer.read().split())
    n = int(next(it))
    l = int(next(it))
    r = int(next(it))

    best_score = -1
    best_id = -1
    for i in range(1, n + 1):
        p = int(next(it))
        if l <= p <= r and p > best_score:
            best_score = p
            best_id = i

    sys.stdout.write(f"{best_id}\n")


if __name__ == "__main__":
    main()
