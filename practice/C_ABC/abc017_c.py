# https://atcoder.jp/contests/abc017/tasks/abc017_3
# 難しかった imos法 発想の転換(引き算に)

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

# 例えば得点がすべて1点だったとする。(宝石の数と一致)
# このとき、一番宝石の数が少ないところにかぶる区間を抜けばいいのだから
# imosして宝石の全体数-minが正解

# じゃ得点が任意の数だったら？
# 得点は宝石につきまとうと考える。そうすると、得点で範囲をimosして同様の操作をすれば、各宝石が何点をカバーしているかがわかる。

# 大事な概念はその宝石が何点得点を担当しているかだ。


N, M = read_ints()
imos = Imos1d(M)
total = 0
for _ in range(N):
    l, r, s = read_ints()
    imos.add(l - 1, r, s)
    total += s
print(total - min(imos.get_result()))
