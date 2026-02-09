import sys


def main() -> None:
    from collections import deque

    it = iter(sys.stdin.buffer.read().split())
    n = int(next(it))
    k = int(next(it))
    h = [int(next(it)) for _ in range(n)]

    # maxdq: indices with non-increasing values (front is max)
    # mindq: indices with non-decreasing values (front is min)
    maxdq: deque[int] = deque()
    mindq: deque[int] = deque()

    ans = 0
    for i, x in enumerate(h):
        while maxdq and h[maxdq[-1]] <= x:
            maxdq.pop()
        maxdq.append(i)

        while mindq and h[mindq[-1]] >= x:
            mindq.pop()
        mindq.append(i)

        left = i - k + 1
        if left < 0:
            continue

        while maxdq[0] < left:
            maxdq.popleft()
        while mindq[0] < left:
            mindq.popleft()

        diff = h[maxdq[0]] - h[mindq[0]]
        if diff > ans:
            ans = diff

    sys.stdout.write(f"{ans}\n")


if __name__ == "__main__":
    main()
