# https://atcoder.jp/contests/abc153/tasks/abc153_f
# 座標圧縮、貪欲法？


import sys
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

# まず、何回攻撃すればいいのかを計算する。これがとにかく必要だ(なくてもいいけど)。
#
# 素直な戦略として、左から倒せるギリギリの爆弾を投下して倒すのが最適
# だけどナイーブに実装すると、O(N^2)。だから体力を管理するのが重要。
# 累積和で体力料を効率的に管理できるらしいが...?


N, D, A = read_ints()
X, H = read_col(N, 2)
