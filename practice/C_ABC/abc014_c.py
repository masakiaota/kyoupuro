# https://atcoder.jp/contests/abc014/tasks/abc014_3
# imosするだけ


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


def read_a_int():
    return int(read())


n = read_a_int()
imos = Imos1d(1000001)
for _ in range(n):
    a, b = read_ints()
    imos.add(a, b + 1, 1)
print(max(imos.get_result()))
