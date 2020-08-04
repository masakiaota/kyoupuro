from bisect import bisect_left, bisect_right, insort_left

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


############# Mo's algorithm ##################
# offline queryをlについて平方分割、rについてしゃくとり法したもの
from operator import itemgetter


class Mo:
    def __init__(self, ls):
        from math import sqrt, ceil
        self.ls = ls.copy()
        self.n = len(ls)
        self.b = ceil(sqrt(self.n))  # bukectのサイズ及び個数

    def _init_states(self):
        ########################################
        # self.states = None  # その時点における状態(自分で定義しろ) #いくつでもいい

        ########################################

        # [l,r)の半開区間で考える
        self.l = 0
        self.r = 0

        # queryを格納する用
        self.bucket = [list() for _ in range((self.b + 1))]

    def _add(self, i):
        # i番目の要素を含めて考えるときへstatesを更新
        ########################################
        pass
        ########################################

    def _delete(self, i):
        # i番目の要素を除いて考えるときへstatesを更新
        ########################################
        pass
        ########################################

    def _one_process(self, l, r):
        # クエリ[l,r)に対してstatesを更新する
        for i in range(self.r, r):  # rまで伸長
            self._add(i)
        for i in range(self.r - 1, r - 1, -1):  # rまで短縮
            self._delete(i)
        for i in range(self.l, l):  # lまで短縮
            self._delete(i)
        for i in range(self.l - 1, l - 1, -1):  # lまで伸長
            self._add(i)

        self.l = l
        self.r = r

    def process(self, queries):
        self._init_states()

        for i, (l, r) in enumerate(queries):  # queryをbucketに格納
            self.bucket[l // self.b].append((l, r, i))

        for i in range(len(self.bucket)):
            self.bucket[i].sort(key=itemgetter(1))

        ret = [-1] * len(queries)
        for b in self.bucket:
            for l, r, i in b:  # クエリに答えていく
                self._one_process(l, r)
                ########################################
                # クエリに答える作業をここで書く
                # ans=
                ret[i] = ans
                ########################################
        return ret
