# https://atcoder.jp/contests/abc142/tasks/abc142_e
# このナップサックdpは面白い！
# 配るdpをイメージするとわかりやすい
# dp[i][bit] ... [0,i)の鍵を考慮したときに状態bitであるための最小の費用
# bitは開けられる宝箱を2進数で示している。5(101)だったら、0,2の宝箱が開けられる。
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def as_bit(ls):
    # 開けられる宝箱をbitで示す
    ret = 0
    for l in ls:
        ret ^= 1 << (l - 1)
    return ret


from itertools import product
INF = 10**9
N, M = read_ints()
C = []
A = []
for _ in range(M):
    a, b = read_ints()
    A.append(a)
    c = read_ints()
    C.append(as_bit(c))

'''
dp[i][bit] ... [0,i)の鍵を考慮したときに状態bitであるための最小の費用
bitは開けられる宝箱を2進数で示している。5(101)だったら、0,2の宝箱が開けられる。

更新則
dp[i+1][bit|C[i]] = min(dp[i+1][bit|C[i]], dp[i][bit] + A[i]) # i番目を取る
dp[i+1][bit] = min(dp[i+1][bit], dp[i][bit]) #i番目を取らない
∵配るDP。ナンプサックDPの重さの更新がbit演算に置き換わっただけ

初期条件&境界条件
dp[i][bit]=INF
dp[i][0]=0
'''

dp = [[INF] * (2 ** N) for _ in range(M + 1)]

for i in range(M + 1):
    dp[i][0] = 0

for i, bit in product(range(M), range(2 ** N)):
    dp[i + 1][bit | C[i]] = min(dp[i + 1][bit | C[i]], dp[i][bit] + A[i])
    dp[i + 1][bit] = min(dp[i + 1][bit], dp[i][bit])

ans = dp[-1][-1]
print(ans if ans != INF else -1)
