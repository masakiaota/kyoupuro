# https://atcoder.jp/contests/abc142/tasks/abc142_d
# これは,A,Bの最大公約数を見つけてからそれを素因数分解し、累乗の底の個数＋1が答えとなる。

from fractions import gcd


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


A, B = list(map(int, input().split()))


G = gcd(A, B)
ans = len(factorization(G)) + 1
print(ans)
