# 始点と終点のベクトルを更新していくイメージ


from math import sin, cos, pi


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''抽象化セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは[単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = segfunc
        self.n_origin = len(ls)
        self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def __getitem__(self, idx):  # オリジナル要素にアクセスするためのもの
        if isinstance(idx, slice):
            start = idx.start if idx.start else 0
            stop = idx.stop if idx.stop else self.n_origin
            l = start + self.num - 1
            r = l + stop - start
            return self.tree[l:r:idx.step]
        elif isinstance(idx, int):
            i = idx + self.num - 1
            return self.tree[i]

    def update(self, i, x):
        '''
        i番目の要素をxに変更する(木の中間ノードも更新する) O(logN)
        '''
        i += self.num - 1
        self.tree[i] = x
        while i:  # 木を更新
            i = (i - 1) // 2
            self.tree[i] = self.func(self.tree[i * 2 + 1],
                                     self.tree[i * 2 + 2])

    def query(self, l, r):
        '''区間[l,r)に対するクエリをO(logN)で処理する'''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res_right = []
        res_left = []
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res_left.append(self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res_right.append(self.tree[r - 1])
            l >>= 1
            r >>= 1
        res = self.ide
        # 左右の順序を保って結合
        for x in res_left:
            res = self.func(x, res)
        for x in reversed(res_right):
            res = self.func(res, x)
        return res


# セグ木の各要素は(vx,vy,ang)を持つことにする。angはそのベクトルの右側の辺が垂直から何度傾いているかを示す

def segfunc(l, r):
    c = cos(l[2])
    s = sin(l[2])
    return (l[0] + (c * r[0] - s * r[1]),
            l[1] + (s * r[0] + c * r[1]),
            l[2] + r[2])


def solve(N, C, L, S, A):
    tmp = [(0, y, 0) for y in L]
    # print(tmp)
    segtree = SegmentTree(tmp, segfunc, identity_element=(0, 0, 0))
    S = [s - 1 for s in S]
    A = [(a - 180) * (pi / 180) for a in A]  # ラジアンに直しておく
    # print(segtree.tree)
    for i, a in zip(S, A):
        x, y, _ = segtree[i]
        segtree.update(i, (x, y, a))
        ansx, ansy, _ = segtree.query(0, N)
        print(ansx, ansy)


# 入力例1
N = 2
C = 1
L = [10, 5]
S = [1]
A = [90]
solve(N, C, L, S, A)

print()

# 入力例2
N = 3
C = 2
L = [5, 5, 5]
S = [1, 2]
A = [270, 90]
solve(N, C, L, S, A)
