# https://atcoder.jp/contests/abc080/tasks/abc080_d
# 縦時系列の番組表の四角を横にスライドさせてmaxを求めれば良い
# つまりimos法
# ただし予約タイムが存在するので、開始時刻を-1してあげる
# また同じチャンネルで連続するところは連結してしまおう


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
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


N, C = read_ints()
# 前処理が大変なパターン
from collections import defaultdict, Counter
STbyC = defaultdict(lambda: [])
for n in range(N):
    s, t, c = read_ints()
    STbyC[c].append(s)
    STbyC[c].append(t)

kukan = []
for c in STbyC:
    tmp = Counter(STbyC[c])
    tmp = [k for k, v in tmp.items() if v == 1]  # 二回出現したやつというのは連続録画
    kukan.extend(sorted(tmp))


imos = Imos1d(10 ** 5 + 1)
for i in range(0, len(kukan), 2):
    j = i + 1
    imos.add(kukan[i] - 1, kukan[j], 1)


print(max(imos.get_result()))
