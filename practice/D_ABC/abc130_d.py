# https://atcoder.jp/contests/abc130/tasks/abc130_d
# 連続部分列→累積和を疑え
# 累積和でもすべてのi,jについてK以上か調べるのは辛い
# a0 ... aNについての累積和 b0 ... b(N+1)について
# sum(A[i:j])>=Kは b[j]-b[i]>=K であり、 iを固定したときに, b[j]>=K+b[i]となるB[j]の個数をカウントする。
# よって、Bに対して、K+b[i]を挿入する一番小さいidx(bisect_left)でidxを取得して、それより上の要素の数をカウントする
# 各iについてこれを行えばO(nlogn)
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


class cumsum1d:
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


from bisect import bisect_left

N, K = read_ints()
A = read_ints()
A_cum = cumsum1d(A).ls_accum

ans = 0
for a in A_cum:
    x = K + a
    idx = bisect_left(A_cum, x)
    ans += N - idx + 1
print(ans)
