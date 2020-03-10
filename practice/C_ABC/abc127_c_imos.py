# https://atcoder.jp/contests/abc127/tasks/abc127_c
# 脳死でimosで殴っても良い


class Imos1d:
    def __init__(self, N: int):
        '''
        [0]*N の配列に対して、区間の加算を管理する。
        '''
        self.ls = [0] * (N + 1)  # 配列外参照を防ぐため多くとっておく
        self.N = N

    def add(self, l, r, x):
        '''
        [l,r)の区間にxを足し込む O(1)
        '''
        self.ls[l] += x
        self.ls[min(r, self.N)] -= x  # 配列外参照は余分に作ったところにまとめておく(どうせ使わない)

    def get_result(self):
        '''
        O(N) かけて、区間の加算結果を取得する
        '''
        from itertools import accumulate
        return list(accumulate(self.ls[:-1]))


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
imos = Imos1d(N)  # 各idカードはいくつのゲートを通ることができるのか
for _ in range(M):
    l, r = read_ints()
    imos.add(l - 1, r, 1)
print(sum([M == x for x in imos.get_result()]))
