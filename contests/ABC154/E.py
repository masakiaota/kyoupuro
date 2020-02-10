import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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


# 桁DP！！！
# 1以上N以下であって10進数で表したときに0でない数字がちょうどK個あるようなものの数
# 0がちょうどlen-K個ある
# dp[i][j][k] ... i桁目、j未満フラグ、k今まで0以外が何回出てきたか

# 条件
# N[i]!='0'ならk+1となる、==0 ならkのまま遷移
# 場合分け
# 未満フラグで未満のとき
# 未満フラグがないとき、
    # n==0のときとそれ以外


# dp[0,0,0]=1
# dp[0,0,1]=0
# dp[0,1,0]=0
# dp[0,1,1]=0

# dp[1,0,0]=1
# dp[1,0,0]=1
# dp[1,0,0]=1
# dp[1,0,0]=1


N = input()
K = read_a_int()

dp = [[[0] * 4 for _ in range(2)] for _ in range(len(N) + 1)]
dp[0][0][0] = 1

# for i in range(len(N)):
#     for n in range(10):
#         if int(N[i]) == n:
#             if N[i] == '0':
#                 for k in range(4):
#                     dp[i + 1][0][k] += dp[i][0][k]
#             else:
#                 for k in range(3):
#                     dp[i + 1][0][k + 1] += dp[i][0][k]
#                     dp[i + 1][1][k + 1] += dp[i][1][k] + dp[i][0][k]
#         else:
#             if n == 0:
#                 for k in range(4):
#                     dp[i + 1][1][k] += dp[i][1][k] + dp[i][0][k]
#             elif n != 0:
#                 for k in range(3):
#                     dp[i + 1][1][k + 1] += dp[i][1][k] + dp[i][0][k]

# for i in range(len(N)):
#     for j in range(2):
#         for n in range(10 if j else int(N[i]) + 1):
#             if j == 0 and int(N[i]) == n:
#                 if N[i] == '0':
#                     for k in range(4):
#                         dp[i + 1][0][k] += dp[i][0][k]
#                 else:
#                     for k in range(3):
#                         dp[i + 1][0][k + 1] += dp[i][0][k]
#                 break  # んー
#             elif j == 0:
#                 # ただ未満のとき
#                 dp[]

#             if j == 1:
#                 if n == 0:
#                     for k in range(4):
#                         dp[i + 1][1][k] += dp[i][1][k] + dp[i][0][k]
#                 elif n != 0:
#                     for k in range(3):
#                         dp[i + 1][1][k + 1] += dp[i][1][k] + dp[i][0][k]

print(dp)
