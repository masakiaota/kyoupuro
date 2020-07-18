# https://atcoder.jp/contests/arc042/tasks/arc042_b
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


# 点と線の距離の公式のminを求めればok
# 直線ax+by+c=0と点x0,y0の距離の公式はd=abs(ax0+by0+c)/sqrt(a**2 + b**2)
# 点sx,sy→tx,tyの直線の公式は a=ty-sy, b=-(tx-sx), c=sy*tx-sx*tyとなる

from math import sqrt
x, y = ints()
N = a_int()
XY = read_tuple(N)
XY.append(XY[0])

ans = 2**31
for (sx, sy), (tx, ty) in zip(XY, XY[1:]):
    a = ty - sy
    b = -(tx - sx)
    c = sy * tx - sx * ty
    ans = min(ans,
              abs(a * x + b * y + c) / sqrt(a**2 + b**2))
print(ans)
