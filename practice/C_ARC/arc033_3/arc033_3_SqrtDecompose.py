# REになる...なぜ
import sys
read = sys.stdin.readline
from functools import reduce


class SqrtDecomposedList:
    def __init__(self, ls, segfunc, identity_element):
        from math import sqrt, ceil
        from copy import copy
        self.f = segfunc  # queryのfunction
        self.ide = identity_element
        self.n = len(ls)
        self.ls = copy(ls)
        self.b = ceil(sqrt(self.n))
        self.bucket = [self.ide] * (self.b + 1)
        self._build()

    def _build(self):
        for d in range(self.b):
            self.bucket[d] = reduce(
                self.f, self.ls[(d * self.b):((d + 1) * self.b)], self.ide)
        # for i, v in enumerate(self.ls):
        #     self.bucket[i // self.b] = \
        #         self.f(v, self.bucket[i // self.b])

    def __getitem__(self, idx):
        return self.ls[idx]

    def update(self, i, x):
        '''i番目の要素をxに更新する O(√n)'''
        self.ls[i] = x
        s = i // self.b * self.b
        t = s + self.b
        # print(i, self.b, i // self.b, s, t)
        self.bucket[i // self.b] = reduce(self.f,
                                          self.ls[s:t], self.ide)

    def query(self, l, r):
        '''半開区間[l,r)で指定すること O(√n)'''
        if r - l < 2 * self.b:
            return reduce(self.f, self.ls[l:r])
        l_bucket = (l - 1) // self.b + 1
        r_bucket = r // self.b  # 半開区間なのでこれでいい
        ret = reduce(self.f, self.bucket[l_bucket:r_bucket], self.ide)
        ret = self.f(
            reduce(self.f, self.ls[l:(l_bucket * self.b)], self.ide),
            ret)
        ret = self.f(
            reduce(self.f, self.ls[(r_bucket * self.b):r], self.ide),
            ret)
        return ret


Q = int(input())
sqls = SqrtDecomposedList([0] * 200001, segfunc=lambda x,
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
        sqls.update(x, 1)
    else:
        # 二分探索で初めてX以上になるidxを探す
        # →和がX以上になる(ok)状態の最小値
        idx = meguru_bisect(0, 200002, sqls, x)
        print(idx)
        # print(segtree[idx:idx + 5])
        # print(len(segtree[:]))
        sqls.update(idx, 0)
