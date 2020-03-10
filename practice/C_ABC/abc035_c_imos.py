# https://atcoder.jp/contests/abc035/tasks/abc035_c
# イモス法で解き直してみる

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


class Imos1d:
    def __init__(self, N: int):
        '''
        [0]*N の配列に対して、区間の加算を管理する。
        '''
        self.ls = [0] * (N + 1)  # 配列外参照を防ぐため多くとっておく

    def add(self, l, r, x):
        '''
        [l,r)の区間にxを足し込む O(1)
        '''
        self.ls[l] += x
        self.ls[min(r, N)] -= x  # 配列外参照は余分に作ったところにまとめておく(どうせ使わない)

    def get_result(self):
        '''
        O(N) かけて、区間の加算結果を取得する
        '''
        from itertools import accumulate
        return list(accumulate(self.ls[:-1]))


N, Q = read_ints()

imos = Imos1d(N)

for q in range(Q):
    l, r = read_ints()
    imos.add(l - 1, r, 1)

result = [str(a & 1) for a in imos.get_result()]
print(''.join(result))
