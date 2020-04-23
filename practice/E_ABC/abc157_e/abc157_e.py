# https://atcoder.jp/contests/abc157/tasks/abc157_e
# セグメント木
# 各文字をビットに対応させる(セグ木を26本持っても良い)
import sys
read = sys.stdin.readline


def read_a_int():
    return int(read())


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''
        セグ木 (下書き)
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = segfunc
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

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
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num - 1
        r += self.num - 2  # ここで半開区間でなく[l,r]で以下を処理する
        res = self.ide
        while r - l > 1:  # 右から寄りながら結果を結合していくイメージ
            if l & 1 == 0:
                res = self.func(res, self.tree[l])
            if r & 1 == 1:
                res = self.func(res, self.tree[r])
                r -= 1
            l = l // 2  # 親の一つ右に移動
            r = (r - 1) // 2  # 親の一つ左に移動
        if l == r:
            res = self.func(res, self.tree[l])
        else:
            res = self.func(res, self.tree[l])
            res = self.func(res, self.tree[r])
        return res


def segfunc(x, y):
    # 処理したい内容
    return x | y


def moji_to_bit(a):
    return 1 << (ord(a) - ord('a'))


def bit_to_sum(n):
    return sum([(n >> i) & 1 for i in range(n.bit_length())])


N = read_a_int()
S = read()[:-1]
S_bit = [moji_to_bit(s) for s in S]

# build segment tree
st = SegmentTree(S_bit, segfunc, 0)

Q = read_a_int()
for q in range(Q):
    com, a, b = read().split()
    if int(com) == 1:
        i, c = int(a) - 1, b
        st.update(i, moji_to_bit(c))
    else:
        l, r = int(a) - 1, int(b)
        print(bit_to_sum(st.query(l, r)))
