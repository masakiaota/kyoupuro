import sys
read = sys.stdin.buffer.readline

ctypedef long long cint


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


cdef a_int(): return int(read())


cdef ints(): return list(map(int, read().split()))


cdef read_tuple(int H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret




'''
dp[i][j]...(0,0)からスタートして(i,j)に到達する経路のなかでの最大の価値
dp[i][j]=max(dp[i][j-1],dp[i-1][j]) + v[i][j]


dp[i][j][k] ... (0,0)からスタートして(i,j)に到達する経路のなかでi行目ですでにk(以下)個選んでいるときの最大の価値

遷移は、i,jのvを拾うときと拾わないときのmax
dp[i][j][k] = max(dp[i][j-1][k-1]+V[i,j], dp[i][j-1][k]
                  ) #k個のままのほうが価値が高いかk-1から拾ったほうが価値が高いか

上からの遷移 # 拾うか拾わないかでこうかな
dp[i][j][0]=max(dp[i][j-1][0],max_k(dp[i-1][j]))
dp[i][j][1]=max(dp[i][j-1][0],max_k(dp[i-1][j][k])+V[i][j])
'''

cdef:
    cint R,C,K,_,r,c,v,i,j,k,ma

R, C, K = ints()

cdef cint[3005][3005] V 
for _ in range(K):
    r,c,v=map(int, input().split())
    V[r - 1][c - 1] = v

cdef cint[3005][3005][4] dp

# dp[0][0][0] = 0
for k in range(1, 4):
    dp[0][0][k] = V[0][0]
for j in range(1, C):
    for k in range(1, 4):
        dp[0][j][k] = max(dp[0][j - 1][k], dp[0][j - 1][k - 1] + V[0][j])

for i in range(1, R):
    dp[i][0][0] = max(dp[i - 1][0])
    dp[i][0][1] = dp[i][0][0] + V[i][0]
    dp[i][0][2] = dp[i][0][0] + V[i][0]
    dp[i][0][3] = dp[i][0][0] + V[i][0]


cdef cint up,left
for i in range(1, R):
    for j in range(1, C):
        up=i-1
        left=j-1
        ma = max(*dp[up][j])
        dp[i][j][0] = max(dp[i][left][0],  # 左
                          ma)  # 上

        dp[i][j][1] = max(dp[i][left][0] + V[i][j],  # 左から来て拾う
                          dp[i][left][1],  # 拾わないほうが大きい場合もある
                          ma + V[i][j])  # 上から来て拾う

        dp[i][j][2] = max(dp[i][left][1] + V[i][j], dp[i][left][2])
        dp[i][j][3] = max(dp[i][left][2] + V[i][j], dp[i][left][3])


print(max(dp[R - 1][C - 1]))

