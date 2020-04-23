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
