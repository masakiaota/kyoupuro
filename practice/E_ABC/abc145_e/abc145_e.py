# https://atcoder.jp/contests/abc145/tasks/abc145_e
# なんとなくナップサック問題っぽい
# A分以内に食べきる事のできる最大の美味しさは？ならすぐにできる。
# 問題は最後のA分は必要ないこと

# →i番目の料理を最後に食べたときに、それ以外の料理でT分以内に食べることのできる美味しさの最大は？
# O(n^3) ∵ 各iについて(3000) × dp(3000*3000)
# →TLEしてしまいそう...


# 普通にナップサックして解いてから。もしiを最後に食べてたらの処理をする？
# ナップサック復元して、まだ食べてないもののなかから美味しさが最大のものを食べればよい！

'''
dp[i][j] ... 最大の美味しさの総和。[,i)の料理まで考慮したとき。j分以内に食べ終わることができる。
'''

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


# default import
from itertools import product, permutations, combinations


# ナップサック復元して、まだ食べてないもののなかから美味しさが最大のものを食べればよい！
# ただし注意として、

N, T = read_ints()
A, B = read_col(N)
tmp = list(zip(A, B))
tmp.sort()
A, B = map(list, zip(*tmp))
# print(A, B)

'''
dp[i][j] ... 最大の美味しさの総和。[,i)の料理まで考慮したとき。j分以内に食べ終わることができる。
'''

dp = [[0] * T for _ in range(N + 1)]

for i, j in product(ra(N), ra(T)):
    dp[i + 1][j] = max(dp[i + 1][j], dp[i][j])
    if j - A[i] >= 0:
        dp[i + 1][j] = max(dp[i + 1][j], dp[i][j - A[i]] + B[i])

ans = 0
for i in ra(N):
    ans = max(ans, dp[i][-1] + B[i])
print(ans)
# 仮に食べる料理がすべて決定していたらAの昇順で食べていったほうが必ず良い(Aが長いのを先に食べると閉店時間になってしまう場合がある)
# ということはB[i]+dp[i][-1]でi番目を最後に食べた場合の満足度が計算できる。(もし全然余裕がある場合はdp[i][j]が小さいので自然と答えにはならない)
# (ソートしておいたABでナップザックは事前にしておく)


# 以下だと何故かaftercontestでWAが出る
# もしかすると、なるべく細かいのを使うっていうのがうまくできないのかも
# # dp復元しながら食べた料理の美味しさを0に書き換える(あとでmaxを取るときに無視するため)
# i = N
# j = T - 1
# while i > 0:
#     if dp[i][j] == dp[i - 1][j]:  # 上から来たものだったらi-1番目のものは使ってない
#         pass
#     else:  # 斜めから来てたらi-1番目のものを使っている
#         B[i - 1] = 0  # 使ったものは美味しさを0にセットしておく
#         j -= A[i - 1]
#     i -= 1

# print(dp[-1][-1] + max(B))
