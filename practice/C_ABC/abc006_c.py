# https://atcoder.jp/contests/abc006/tasks/abc006_3

# 答えをa,b,cと仮定。M=2a+3b+4c , N=a+b+cを満たす組を見つけたい。(しかし3重forはTLEになりそう)
# 連立方程式でbを削除すると、M-3N=c-aである。これならば1重forで済む。
# aを仮定してやれば(forで全探索)、c=M-3N+aだからである。
# するとbも求まる。一方bは両方の式で成り立っていないと行けない。

N, M = map(int, input().split())


def retc(a): return M - 3 * N + a


def retb(a, c): return N - a - c


def check(a, b, c): return M == 2 * a + 3 * b + 4 * c


for a in range(M // 2 + 1):
    c = retc(a)
    b = retb(a, c)
    if check(a, b, c) and a >= 0 and b >= 0 and c >= 0:
        print(a, b, c)
        exit()
print(*[-1] * 3)
