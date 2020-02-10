# https://atcoder.jp/contests/abc007/tasks/abc007_4

# B以下の個数 - A-1以下の個数が答え
# X以下の個数を高速に求める方法を考える
# 少なくとも1つの4or9が含まれる通りの数 = X-4と9を含まない通りの数
# X以下の4と9を含まない通りの数を桁dpで得る


# def f(X):
#     cnt = 0
#     for x in range(X + 1):
#         xx = str(x)
#         if xx.count('4') or xx.count('9'):
#             cnt += 1
#             print(xx)
#     return cnt


# def get_non49(X):
#     X = '0' + str(X)
#     dp = [[0] * 2 for _ in range(len(X))]
#     # dp[i][j] ... i桁目まで考慮したときに、49が出てこない数字のパターン,ただしjは未満フラグ
#     # こう定義しちゃったけどどうやら、dp[i][j][k]と定義したほうが実装は楽っぽい (こっちだと条件分岐がめんどい)
#     dp[0][0] = 1
#     for i in range(len(X) - 1):
#         D = int(X[i + 1])
#         for d in range(10):
#             if d == 4 or d == 9:
#                 continue
#             dp[i + 1][1] += dp[i][1]
#             if d < D:
#                 dp[i + 1][1] += dp[i][0]
#         dp[i + 1][0] = dp[i][0] if D not in [4, 9] else 0

#     return sum(dp[-1])
#     # そして実装が違うっぽいという


# A, B = list(map(int, input().split()))

# A = A - get_non49(A - 1)
# B = B + 1 - get_non49(B)
# print(B - A)


# def ketadp(X):
#     X = '0' + str(X)
#     dp = [[[0] * 2 for _ in range(2)] for _ in range(len(X))]
#     dp[0][0][0] = 1
#     for i in range(len(X) - 1):
#         for j in range(2):
#             for k in range(2):
#                 for d in range(10 if j else int(X[i + 1]) + 1):
#                     dp[i + 1][j or d < int(X[i + 1])][k or d == 4 or d == 9] \
#                         += dp[i][j][k]

#     # print(dp)
#     return dp[-1][0][1] + dp[-1][1][1]


# A, B = list(map(int, input().split()))
# A = ketadp(A - 1)
# B = ketadp(B)
# print(B - A)

from functools import lru_cache
import sys
sys.setrecursionlimit(1 << 25)


@lru_cache(None)
def f(X: int):
    '''
    X以下で4,9が一つも含まれていない数の個数
    '''
    # 終了条件
    assert X >= 0
    if X < 10:  # もし一桁なら
        return X - ((X >= 4) + (X >= 9)) + 1

    # 再帰桁dp
    q, r = divmod(X, 10)
    ret = 0
    for d in range(10):
        if d == 4 or d == 9:
            continue
        elif d > r:
            ret += f(q - 1)
        else:
            ret += f(q)
    return ret


A, B = list(map(int, input().split()))
A = A - f(A - 1)
B = B + 1 - f(B)
print(B - A)
