# https://atcoder.jp/contests/abc052/tasks/arc067_a
# N!の約数の個数
# N, N-1, N-2 について素因数分解を個々に行い、素因数とその個数をカウントしていく
# 各素因数の出現回数について (a1+1)(a2+1)...が答え。


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


MOD = 10**9 + 7
from collections import defaultdict
N = int(input())
factor_num = defaultdict(lambda: 0)
for n in range(1, N + 1):
    tmp = factorization(n)
    for k, v in tmp:
        factor_num[k] += v

# 約数の個数
ans = 1
for v in factor_num.values():
    ans *= (v + 1)
    if ans >= MOD:
        ans %= MOD
print(ans)
