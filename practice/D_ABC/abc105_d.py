# https://atcoder.jp/contests/abc105/tasks/abc105_d


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


class cumsum1d:  # 一次元累積和クラス
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


from collections import Counter
N, M = read_ints()
A = read_ints()

# 素直にl,rを全探索はできない
# 累積和を割れば0sumrangesの亜種
A_cum = cumsum1d(A)
tmp = []
for a in A_cum.ls_accum:
    tmp.append(a % M)
cnts = Counter(tmp)

ans = 0
for c in cnts.values():
    ans += c * (c - 1) // 2

print(ans)
