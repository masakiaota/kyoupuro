# https://atcoder.jp/contests/abc153/tasks/abc153_f
# 座標圧縮、貪欲法、imos法


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


# まず、何回攻撃すればいいのかを計算する。これがとにかく必要だ(なくてもいいけど)。
#
# 素直な戦略として、左から倒せるギリギリの爆弾を投下して倒すのが最適
# だけどナイーブに実装すると、O(N^2)。だから体力を管理するのが重要。
# どこにどれだけダメージが蓄積したのかはimos法(デルタ関数をおいてから累積和)で管理できる。

from bisect import bisect_left
N, D, A = read_ints()
XH = read_tuple(N)
XH.sort()  # 座標でソート

n_atk = []  # 何回攻撃するのが必要か
X = []  # 座標アクセス用配列
for x, h in XH:
    n_atk.append((h - 1) // A + 1)
    X.append(x)

damege = [0] * (N + 10)  # ダメージ管理用、配列外アクセスがめんどくさいので長めに取る
ans = 0
# j = 0  # 次のxが、今のx+2d以下でもっとも大きなidx
for i, (x, n) in enumerate(zip(X, n_atk)):
    damege[i] += damege[i - 1]  # 積分して、現時点の蓄積回数を表示
    atk = max(0, n - damege[i])  # iには何回攻撃したか
    ans += atk
    # 攻撃下回数を記録
    damege[i] += atk
    # 効果が切れる点を予約 # 尺取ならO(1)で次に行けるけど二分探索でも間に合うか
    damege[bisect_left(X, x + 2 * D + 1, lo=i)] -= atk  # 効果切れを予約

print(ans)
