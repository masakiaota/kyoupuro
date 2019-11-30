# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_3_C
# 本の説明を図に書き出してみるとよく理解できる。
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


def contains(G, p):
    '''
    Gは多角形を表すリストで、今回ならばvectorが中に並んでいることとする
    pは内容しているか判別したい点で、Vectorで表す

    pが多角形Gの辺上にあれば1
    pが多角形Gに内包されていれば2
    それ以外は0をreturnする
    '''
    is_contain = False  # 内包してるか
    for i in range(len(G)):
        a = G[i].sub(p)
        b = G[(i + 1) % len(G)].sub(p)  # i+1が0に戻るようにこのような形式にしている。
        # もしpがG[i]とG[i+1]の線分上にある場合は即座に1をreturnします
        if abs(cross(a, b)) < EPS and dot(a, b) < EPS:
            # 外積が0→a,bが同一直線上
            # かつ 内積が負→a,bは逆を向いている
            # ならばpは線分上に存在する
            return 1
        # 内包を判定する。前処理として、yの座標によってa,bを入れ替える
        if a[1] > b[1]:  # aの方のy座標を小さくしたい
            a, b = b, a
        if a[1] < EPS and b[1] > EPS and cross(a, b) > EPS:  # 実際に判別する
            is_contain = (not is_contain)
    return (0, 2)[is_contain]  # 書き方キモいけど三項演算子の短い書き方


# load data
N = int(input())
G = []
for _ in range(N):
    g = Vector(list(map(int, input().split())))
    G.append(g)

# answer query
Q = int(input())
for _ in range(Q):
    p = Vector(list(map(int, input().split())))
    print(contains(G, p))
