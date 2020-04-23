# https://atcoder.jp/contests/arc033/tasks/arc033_3
# セグ木実装
import sys
read = sys.stdin.readline


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
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
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res = self.ide
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res = self.func(res, self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res = self.func(res, self.tree[r - 1])
            l >>= 1
            r >>= 1
        return res


Q = int(input())
segtree = SegmentTree([0] * 200001, segfunc=lambda x,
                      y: x + y, identity_element=0)


def is_ok(idx, segtree, x):
    # 条件を満たすかどうか？問題ごとに定義
    return segtree.query(0, idx + 1) >= x


def meguru_bisect(ng, ok, segtree, x):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid, segtree, x):
            ok = mid
        else:
            ng = mid
    return ok


for _ in range(Q):
    t, x = map(int, read().split())
    if t == 1:
        segtree.update(x, segtree[x] + 1)
    else:
        # 二分探索で初めてX以上になるidxを探す
        # →和がX以上になる(ok)状態の最小値
        idx = meguru_bisect(0, 200002, segtree, x)
        print(idx)
        # print(segtree[idx:idx + 5])
        # print(len(segtree[:]))
        segtree.update(idx, segtree[idx] - 1)
