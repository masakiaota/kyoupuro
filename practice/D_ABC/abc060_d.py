# https://atcoder.jp/contests/abc060/tasks/arc073_b
# 典型的なナップサック。だけど配列が大きいので素直に実装するとTLEになる
# 成約により、w1以上は必ず前のjを見ることに注意するとテーブルのサイズがぐっと減ることに気がつくがこれを実装するのはなかなかめんどくさそう。
# defaltdictを利用した再帰メモ化なら比較的実装可能では？

# 他にも全探索でゴリ通す方法
# w1を予め引いておく方法(ボトムアップdpするならこれかな)がある。

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


N, W = read_ints()
w, v = read_col(N, 2)

from collections import defaultdict
dp = defaultdict(lambda: -1)


def f(i, j):  # i番目を含んで考慮したとき重さjまでで達成できる価値の最大値
    if dp[i, j] != -1:
        return dp[i, j]
    if i == -1:
        return 0
    if j - w[i] < 0:
        return f(i - 1, j)
    ret = max(f(i - 1, j - w[i]) + v[i], f(i - 1, j))
    dp[i, j] = ret
    return ret


print(f(N - 1, W))
