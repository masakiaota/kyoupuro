# mo's algorithm
# offline queryをlについて平方分割、rについてしゃくとり法したもの
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


from collections import defaultdict
from operator import itemgetter


class Mo:
    def __init__(self, ls):
        # データは突っ込んで置きたい
        from math import sqrt, ceil
        self.ls = ls
        self.n = len(ls)
        self.b = ceil(sqrt(self.n))  # bukectのサイズ及び個数

    def _init_states(self):
        ########################################
        # self.states = None  # その時点における状態(自分で定義しろ) #2つでもいい
        self.n_unique = 0
        self.cnt = defaultdict(lambda: 0)
        ########################################

        # [l,r)の半開区間で考える
        self.l = 0
        self.r = 0

        # queryを格納する用
        self.bucket = [list() for _ in range((self.b + 1))]

    def _add(self, i):
        # i番目の要素を含めて考えるときへstatesを更新
        if self.cnt[self.ls[i]] == 0:
            self.n_unique += 1
        self.cnt[self.ls[i]] += 1

    def _delete(self, i):
        # i番目の要素を削除して考えるときへstatesを更新
        self.cnt[self.ls[i]] -= 1
        if self.cnt[self.ls[i]] == 0:
            self.n_unique -= 1

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

        idx = defaultdict(lambda: [])  # queryの順番を記録しておく
        for i, (l, r) in enumerate(queries):  # queryをbucketに格納
            idx[l, r].append(i)
            self.bucket[l // self.b].append((l, r))

        for i in range(len(self.bucket)):
            self.bucket[i].sort(key=itemgetter(1))

        ret = [-1] * len(queries)
        for b in self.bucket:
            for l, r in b:  # クエリに答えていく
                self._one_process(l, r)
                ########################################
                # クエリに答える作業をここで書く
                ret[idx[l, r].pop()] = self.n_unique
                ########################################
        return ret


def ints(): return list(map(int, read().split()))


N, Q = ints()
C = ints()
queries = []
for _ in range(Q):
    l, r = ints()
    queries.append((l - 1, r))


mo = Mo(C)
ans = mo.process(queries)
print(*ans, sep='\n')

# まあTLEなんですけどね
