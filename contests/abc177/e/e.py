import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


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
            if temp == 1:
                break

    if temp != 1:
        arr.append((temp, 1))

    if arr == []:
        arr.append((n, 1))

    return arr


def can_factorization(n: int):
    if n == 1:
        return []  # 1は素数ではない
    # 素因数分解
    arr = []
    temp = n
    for i in range(2, int(n**0.5) + 1):  # ここにバグがないか心配
        if temp % i == 0:
            arr.append(i)

    if temp != 1:
        arr.append((temp, 1))

    # if arr == []:
    #     arr.append((n, 1))

    return arr


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
import random
from math import gcd


N = a_int()
A = ints()
random.shuffle(A)

# setかはすぐわかる
# setでなければ not coprime
# pairは互いに素かをみればいいのか
# つまり因数分解して足してったときにすべての素数のべき数が1以下であれば良い

g_set = 0
cnt = defaultdict(lambda: 0)
flg = 1  # pairwiseであるフラグ
for a in A:
    g_set = gcd(g_set, a)
    if flg:
        for p, n in factorization(a):
            if cnt[p] != 0:
                flg = 0
            cnt[p] += n


# print(cnt)
# for v in cnt.values():
#     if v > 1:
#         flg = 0
#         break

if g_set > 1:
    print('not coprime')
elif flg:
    print('pairwise coprime')
else:
    print('setwise coprime')
