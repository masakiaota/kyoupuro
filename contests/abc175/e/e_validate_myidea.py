import sys

sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque


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

R, C, K = ints()
RCV = read_tuple(K)
V = [[0] * C for _ in ra(R)]
for r, c, v in RCV:
    V[r - 1][c - 1] = v

# dp = [[[0] * 4 for _ in ra(C)] for _ in ra(R)]
dp0 = [[0] * C for _ in ra(R)]
dp1 = [[0] * C for _ in ra(R)]
dp2 = [[0] * C for _ in ra(R)]
dp3 = [[0] * C for _ in ra(R)]

# dp[0][0][0] = 0
dp1[0][0] = V[0][0]
dp2[0][0] = V[0][0]
dp3[0][0] = V[0][0]

for j in ra(1, C):
    dp1[0][j] = max(dp1[0][j - 1], dp0[0][j - 1] + V[0][j])
    dp2[0][j] = max(dp2[0][j - 1], dp1[0][j - 1] + V[0][j])
    dp3[0][j] = max(dp3[0][j - 1], dp2[0][j - 1] + V[0][j])

for i in ra(1, R):
    dp0[i][0] = dp1[i - 1][0]
    dp1[i][0] = dp0[i][0] + V[i][0]
    dp2[i][0] = dp0[i][0] + V[i][0]
    dp3[i][0] = dp0[i][0] + V[i][0]


for i in ra(1, R):
    for j in ra(1, C):
        ma = max(dp0[i - 1][j],
                 dp1[i - 1][j],
                 dp2[i - 1][j],
                 dp3[i - 1][j])

        dp0[i][j] = max(dp0[i][j - 1],  # 左
                        ma)  # 上

        if ma > max(dp3[i][j - 1], dp2[i][j - 1], dp1[i][j - 1]):  # 上から採用
            dp1[i][j] = ma + V[i][j]  # どうやらすべて
            dp2[i][j] = ma + V[i][j]
            dp3[i][j] = ma + V[i][j]
        else:  # 横から採用
            dp1[i][j] = max(dp0[i][j - 1] + V[i][j],  # 左から来て拾う
                            dp1[i][j - 1],  # 拾わないほうが大きい場合もある
                            ma + V[i][j])  # これがないとWAになる。どうやら上から微妙な大きさになった場合に最終的に最適になる場合があるらしい？
            dp2[i][j] = max(dp1[i][j - 1] + V[i][j], dp2[i][j - 1])
            dp3[i][j] = max(dp2[i][j - 1] + V[i][j], dp3[i][j - 1])


print(max(dp0[R - 1][C - 1],
          dp1[R - 1][C - 1],
          dp2[R - 1][C - 1],
          dp3[R - 1][C - 1],
          ))

# print(*values, sep='\n')
# print(*dp3, sep='\n')

# dp2 = [[-1] * C for _ in ra(R)]
# for i in ra(R):
#     for j in ra(C):
#         dp2[i][j] = max(dp[i][j])
# print(*dp2, sep='\n')
