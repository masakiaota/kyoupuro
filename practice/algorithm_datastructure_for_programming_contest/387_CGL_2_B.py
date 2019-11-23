# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_2_B
# この問題は後の点の内包に繋がります
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

    def __repr__(self):
        return f'Vector({self.vec})'

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


EPS = 1e-10


def ccw(p0, p1, p2):
    '''
    問題を解くための関数
    引数はすべてVector
    '''
    a = p1.sub(p0)
    b = p2.sub(p0)
    if cross(a, b) > EPS:
        # 'COUNTER_CLOCKWISE'
        return 1
    elif cross(a, b) < -EPS:
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


# load data
N = int(input())
for _ in range(N):
    if is_intersect(list(map(int, input().split()))):
        print(1)
    else:
        print(0)
