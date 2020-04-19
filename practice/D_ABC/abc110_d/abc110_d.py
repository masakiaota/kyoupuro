# https://atcoder.jp/contests/abc110/tasks/abc110_d
# 誤読をしないように気をつけよう
# 発想は惜しいところまで行っていたしもう一回解いてみよう

N, M = map(int, input().split())


def factorization(n: int):
    if n == 1:
        return []  # 1は素数ではない
    # 素因数分解
    arr = []
    temp = n
    for i in range(2, int(n**0.5) + 1):  # ここにバグがないか心配
        if temp % i == 0:
            cnt = 0
            while temp % i == 0:
                cnt += 1
                temp //= i
            arr.append((i, cnt))

    if temp != 1:
        arr.append((temp, 1))

    if arr == []:
        arr.append((n, 1))

    return arr


def combination_mod(n, r, mod):
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


MOD = 10**9 + 7
facts = factorization(M)
ans = 1
for i, n in facts:
    ans *= combination_mod(n + N - 1, n, MOD)  # 取り出したところがたまたま壁になるっていう発想でもいいかも
    ans %= MOD
print(ans)
