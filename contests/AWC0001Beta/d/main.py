import sys


def main() -> None:
    it = iter(sys.stdin.buffer.read().split())
    n = int(next(it))
    m = int(next(it))
    k = int(next(it))

    a = [0] * n
    b = [0] * n
    for i in range(n):
        a[i] = int(next(it))
        b[i] = int(next(it))

    # dp[i][c] = max profit for a valid visiting sequence that ends at town i
    #            with total accommodation cost exactly c (and includes town i).
    neg = -10**30
    dp = [[neg] * (m + 1) for _ in range(n)]
    rng_cost = range(m + 1)

    ans = 0
    for i in range(n):
        ai = a[i]
        bi = b[i]
        if bi <= m:
            if ai > dp[i][bi]:
                dp[i][bi] = ai
            if ai > ans:
                ans = ai

        l = i - k
        if l < 0:
            l = 0
        if l == i:
            continue

        # best[c] = max dp[j][c] over previous towns j in [i-K, i-1]
        best = [neg] * (m + 1)
        for j in range(l, i):
            row = dp[j]
            for c in rng_cost:
                v = row[c]
                if v > best[c]:
                    best[c] = v

        for cost in range(bi, m + 1):
            prev = best[cost - bi]
            if prev == neg:
                continue
            val = prev + ai
            if val > dp[i][cost]:
                dp[i][cost] = val
            if val > ans:
                ans = val

    print(ans)


if __name__ == "__main__":
    main()
