# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/1/GRL_1_C
# 螺旋本の説明わかりづらすぎへんか？
# ここでは隣接行列を更新して、i→jに行くときの最短経路としたい(なのでshapeは隣接行列と同じ)

# 操作を理解した後に、螺旋本の証明を読むとわかった気になれる。
# https://qiita.com/okaryo/items/8e6cd73f8a676b7a5d75 このサイトは読んでおくとざっくり操作の概要と仕組みがわかる
# https://triple-four.hatenablog.com/entry/2019/04/02/143939 このサイトでは超丁寧に解説している。お金取れるレベル

from copy import deepcopy

INF = 2**36 - 1


def warshall_floyd(adj_mat: list):
    '''
    隣接行列を受け取る(隣接していないノード間のコストはINFを前提)
    全点間の最短距離を示す行列を返す。
    '''
    n = len(adj_mat)
    ret = deepcopy(adj_mat)
    for k in range(n):  # kは経由を示す。
        for i in range(n):  # iは出発を示す
            if ret[i][k] == INF:
                # ノード同士がつながってなければ更新しない
                continue
            for j in range(n):  # jは終点を示す。
                if ret[k][j] == INF:
                    continue
                ret[i][j] = min(ret[i][j], ret[i][k] + ret[k][j])
                # 直接行ったほうが近いか、kを経由したほうが近いか
    return ret  # 閉路が存在するかどうかは対角成分を見れば良い(自己に戻ってくるのに距離が負ならば負の閉路があるということ)


n_V, n_E = list(map(int, input().split()))
adj_mat = [[INF] * n_V for _ in range(n_V)]  # INFで初期化
# 対角成分だけは0で初期化
for ij in range(n_V):
    adj_mat[ij][ij] = 0
for _ in range(n_E):
    s, t, d = list(map(int, input().split()))
    adj_mat[s][t] = d

ans = warshall_floyd(adj_mat)

# 負のループの調査
for ij in range(n_V):
    if ans[ij][ij] < 0:
        print('NEGATIVE CYCLE')
        exit()

for i in range(n_V):
    for j in range(n_V):
        print(ans[i][j] if ans[i][j] != INF else "INF", end='')
        if j == n_V - 1:
            print()
        else:
            print(' ', end='')
