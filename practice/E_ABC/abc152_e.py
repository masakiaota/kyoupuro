# https://atcoder.jp/contests/abc152/tasks/abc152_e
# 見た瞬間にわかってしまった天才だ

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


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
# default import
from collections import defaultdict, Counter, deque
from fractions import gcd


def ret_inv(a, p):
    return pow(a, p - 2, p)


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g


N = read_a_int()
A = read_ints()

flat = defaultdict(lambda: 0)  # 素数がkey
for a in A:  # modで演算できるようにするか掛け算を実際には行わずに保持しておく必要がある。
    tmp = factorization(a)
    for k, v in tmp:
        flat[k] = max(flat[k], v)

flat_mod = 1
for k, v in flat.items():
    flat_mod *= pow(k, v, MOD)
    if flat_mod >= MOD:
        flat_mod %= MOD

ans = 0
for a in A:
    ans += flat_mod * ret_inv(a, MOD)
    if ans >= MOD:
        ans %= MOD
print(ans)
