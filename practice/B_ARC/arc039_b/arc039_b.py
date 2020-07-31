# https://atcoder.jp/contests/arc039/tasks/arc039_b
# N C K%N じゃないのか(困惑)
# どうならべてもいい時はn個の箱(仕切りn-1個)にk個ものを入れていく感じ


def combination_mod(n, r, mod=10**9 + 7):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


n, k = map(int, input().split())
print(combination_mod(n + k - 1, k) if n > k else combination_mod(n, k % n))
