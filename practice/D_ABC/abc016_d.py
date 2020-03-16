# https://atcoder.jp/contests/abc016/tasks/abc016_4
# 交差している線分の数//2 -1が答え
from math import sqrt


class Vector:
    def __init__(self, ls):
        '''
        ls ... list
        '''
        self.vec = ls

    def __len__(self):
        return len(self.vec)

    def __getitem__(self, idx):
        return self.vec[idx]

    def add(self, vec):
        '''
        vec ... vector class
        '''
        assert len(self) == len(vec)
        ret = [a + b for a, b in zip(self.vec, vec.vec)]
        return Vector(ret)

    def sub(self, vec):
        '''
        vec ... vector class
        '''
        assert len(self) == len(vec)
        ret = [a - b for a, b in zip(self.vec, vec.vec)]
        return Vector(ret)

    def mul(self, vec):
        '''
        vec ... vector class
        '''
        assert len(self) == len(vec)
        ret = [a * b for a, b in zip(self.vec, vec.vec)]
        return Vector(ret)

    def norm(self):
        tmp = sum([x * x for x in self.vec])
        return sqrt(tmp)


def norm(vec):
    '''
    vec ... Vector class
    '''
    return vec.norm()


def cross(a, b):
    '''
    Outer product for 2d
    a,b ... Vector class
    '''
    assert len(a) == 2 and len(b) == 2
    first = a[0] * b[1]
    second = a[1] * b[0]
    return first - second


def dot(a, b):
    return sum(a.mul(b))


def ccw(p0, p1, p2):
    '''
    問題を解くための関数
    引数はすべてVector
    '''
    a = p1.sub(p0)
    b = p2.sub(p0)
    if cross(a, b) > 0:
        # 'COUNTER_CLOCKWISE'
        return 1
    elif cross(a, b) < 0:
        # 'CLOCKWISE'
        return -1
    elif dot(a, b) < 0:  # 同一直線状でa,bが逆を向いている
        # 'ONLINE_BACK'
        return 2
    elif a.norm() < b.norm():  # a,bが同じ方向を向いて かつ bがaよりも長い
        # 'ONLINE_FRONT'
        return -2
    else:
        # 'ON_SEGMENT'
        return 0


def is_intersect(args: list):
    x0, y0, x1, y1, x2, y2, x3, y3 = args
    p0 = Vector([x0, y0])
    p1 = Vector([x1, y1])
    p2 = Vector([x2, y2])
    p3 = Vector([x3, y3])
    return (ccw(p0, p1, p2) * ccw(p0, p1, p3) <= 0) and (ccw(p2, p3, p0) * ccw(p2, p3, p1) <= 0)


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


MOD = 10**9 + 7
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations

ax, ay, bx, by = read_ints()
a = Vector([ax, ay])
b = Vector([bx, by])
N = read_a_int()
XY = read_tuple(N)
ans = 0
for i in range(N):
    s = Vector(list(XY[i - 1]))
    t = Vector(list(XY[i]))
    ans += is_intersect([a[0], a[1], b[0], b[1], s[0], s[1], t[0], t[1]])

print(ans // 2 + 1)
