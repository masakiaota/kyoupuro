# https://atcoder.jp/contests/abc157/tasks/abc157_e
# セグメント木
# 各文字をビットに対応させる(セグ木を26本持っても良い)
import sys
read = sys.stdin.readline
from functools import reduce


def read_a_int():
    return int(read())


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
st = SqrtDecomposedList(S_bit, segfunc, 0)

Q = read_a_int()
for q in range(Q):
    com, a, b = read().split()
    if int(com) == 1:
        i, c = int(a) - 1, b
        st.update(i, moji_to_bit(c))
    else:
        l, r = int(a) - 1, int(b)
        print(bit_to_sum(st.query(l, r)))
