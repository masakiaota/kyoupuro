# https://atcoder.jp/contests/abc114/tasks/abc114_d

# 75数...約数をちょうど75個持つ整数
# 約数がちょうど75個→ その数を素因数分解すると
# A**74 or A**2 * B**4 * C**4 or A**2 * B**24 or A**16 * B**4 (必要条件？)

# N！の約数である→N!を割り切れる→素因数分解の肩の数が大きい

# N!の素因数分解の結果に対して、指数が4以上→2つ必要、2以上→1つ必要
# 一般化する。指数が4以上である底の個数をB,指数が2以上4未満である底の個数をAと定義した場合
# BC2 * A + BC3 が答えとなる


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


def make_divisors(n: int, sort=False):
    # 約数列挙
    divisors = []
    for i in range(1, int(n**0.5) + 1):
        if n % i == 0:
            divisors.append(i)
            if i != n // i:
                divisors.append(n // i)
    if sort:
        divisors.sort()
    return divisors


def combination_mod(n, r, mod):
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


from collections import defaultdict

N = int(input())
fact = defaultdict(lambda: 0)
for n in range(1, N + 1):
    tmp = factorization(n)
    for k, v in tmp:
        fact[k] += v

# A**74 or B**2 * C**4 * D**4 or E**2 * F**24 or G**14 * H**4 (必要条件？)
A, B, C, D, E, F, G, H = 0, 0, 0, 0, 0, 0, 0, 0
for k, v in fact.items():
    if v >= 74:
        A += 1

    if v >= 4:
        C += 1
    if 2 <= v < 4:
        B += 1

    if v >= 24:
        F += 1
    if 24 > v >= 2:
        E += 1

    if v >= 14:
        G += 1
    if 14 > v >= 4:
        H += 1


# print(fact)
MOD = 10**9 + 7
ans = A
ans += combination_mod(C, 2, MOD) * B + combination_mod(C, 3, MOD) * 3
ans += E * F + combination_mod(F, 2, MOD) * 2
ans += G * H + combination_mod(G, 2, MOD) * 2

print(ans)
