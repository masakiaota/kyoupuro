# https://atcoder.jp/contests/abc150/tasks/abc150_d
# 半公倍数Xとは、X=a_k (p_k + 0.5) for any k を満たす数字である(p_kは負はない整数)
# 2を掛ければ、2X=a_k P_k where P_k=2p_k+1 となる。2Xは数列Aの公倍数と解釈できる。
# よってこの問題は、a_k (k=1...N)の公倍数2X は 0<=X<=Mを満たす範囲にいくつ存在するか？という風に読み替えられる。
# 数列Aの最小公倍数を2lcmと定義すると、答えの候補はlcmの倍数であるところまで絞り込めた。
# これで答えの候補を最小公倍数の半分、lcmの倍数であるところまで絞り込めた。

# ところでP_kは必ず奇数でなければ行けないのだから,Xとa_kは同じ回数2で割り切れなければ行けない。
# これを満たすXは,lcmがP_kと同じ回数2で割り切れる かつ lcmの奇数倍の数である。

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from fractions import gcd


def lcm(a, b):
    # 最小公倍数 #単位元は1
    g = gcd(a, b)
    return a // g * b


N, M = read_ints()
A = read_ints()
X = 1
for a in A:
    X = lcm(X, a)
X = X // 2

# すべてのa//2 はX(lcmと同じ回数2で割り切れないければ行けない)
lcm = X
n_div = 0
while lcm & 1 == 0:
    lcm //= 2
    n_div += 1

for a in A:
    if n_div != 0 and (a // 2) % (pow(2, n_div)):
        print(0)  # 割り切れなかった時点でそのような半公倍数は存在しない
        exit()


print((M // X + 1) // 2)  # 奇数倍のlcmの個数 つまりすべて-偶数の分
