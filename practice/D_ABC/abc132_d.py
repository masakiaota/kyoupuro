# https://atcoder.jp/contests/abc132/tasks/abc132_d
# K個を区別なくi個のグループに分ける通りの数？
# 制約はたかだか2000
# 赤の壁のパターンを考えるとか？[WIP]
# mod取りながらcombination


MOD = 10**9 + 7


def combination_mod(n, r, mod):
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


N, K = list(map(int, input().split()))
left1 = N - K + 1
left2 = K - 1
for i in range(1, K + 1):
    ans = combination_mod(left1, i, MOD) * combination_mod(left2, i - 1, MOD)
    # print(ans)
    print(ans % MOD)
