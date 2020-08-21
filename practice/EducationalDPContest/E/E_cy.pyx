import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ctypedef long long LL


cdef ints(): return list(map(int, read().split()))

cdef read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))

cdef chmin(LL *a, LL b): #andをつければ動く
    '''使用例 chmin(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if b<a[0]:
        a[0]=b

cdef:
    LL N,W_max,_,i,w,v
import numpy as np

N, W_max = ints()
_W, _V= read_col(N)
# for i in range(N):
#     w, v=ints()
#     W[i]=w
#     V[i]=v

cdef LL[:] W=np.array(_W)
cdef LL[:] V=np.array(_V)



'''
dp[i,j] ... 重さの最小 (:iまで考慮したとき、価値がちょうどjのとき)
答えはW以下の数字が入ってるマスの一番左側

更新則
chmin(dp[i+1,j+V[i]] , dp[i, j]+W[i]) #ナップサックに入れた場合
chmin(dp[i+1,j] , dp[i, j]) # ナップサックに入れなかった場合
'''


cdef solve(LL N, LL W_max,LL[:] W, LL[:] V):
    cdef LL i,j,V_max,jv
    V_max = sum(V) + 1
    cdef LL[:,:] dp = np.full((N + 1, V_max), 10**12, dtype=np.int64)

    # 初期化
    dp[0,0] = 0  # 一個も選ばず価値が0なら必ず重さも0

    # 更新
    for i in range(N):
        for j in range(V_max):
            jv = j + V[i]
            if jv < V_max:
                chmin(&dp[i + 1,jv], dp[i,j] +W[i])
                # dp[i + 1,jv] = min(dp[i + 1,jv], dp[i,j] +W[i])
            chmin(&dp[i + 1,j], dp[i,j])
            # dp[i + 1, j] = min(dp[i + 1,j], dp[i,j])

    # 左から見てく
    for j in range(V_max - 1, -1, -1):
        if any([dp[i][j] <= W_max for i in range(N+1)]):
            print(j)
            return

solve(N, W_max, W, V)
