# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/1/GRL_1_C
# 当然のようにAtCoderではscipyが使える
# https://note.nkmk.me/python-scipy-shortest-path/ ワーシャルフロイド法: floyd_warshall()の章
# あとここで言われているように0の重みを自動で消してしまうという発狂案件が存在します。 https://juppy.hatenablog.com/entry/2019/06/04/scipyのFloyd-WarshallとDijkstraのすすめ_Python_競技プログラミング_Atcoder_1#floyd_warshall


from scipy.sparse import csr_matrix, lil_matrix
from scipy.sparse.csgraph import floyd_warshall, csgraph_from_dense, NegativeCycleError
from numpy import isinf

n_V, n_E = list(map(int, input().split()))
adj_mat = lil_matrix((n_V, n_V), dtype='int')  # INFで初期化
# 対角成分だけは0で初期化
for ij in range(n_V):
    adj_mat[ij, ij] = 0
for _ in range(n_E):
    s, t, d = list(map(int, input().split()))
    adj_mat[s, t] = d
try:
    ans = floyd_warshall(adj_mat)
except NegativeCycleError:
    # 負のループ
    print("NEGATIVE CYCLE")
    exit()

for i in range(n_V):
    for j in range(n_V):
        print(int(ans[i, j]) if not isinf(ans[i, j]) else "INF", end='')
        if j == n_V - 1:
            print()
        else:
            print(' ', end='')
