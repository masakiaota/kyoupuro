import sys


def main() -> None:
    it = iter(map(int, sys.stdin.buffer.read().split()))
    n = next(it)
    k = next(it)
    d = [next(it) for _ in range(n)]

    total = sum(d)
    if k == 0:
        print(total)
        return

    d.sort(reverse=True)
    print(total - sum(d[:k]))


if __name__ == "__main__":
    main()
