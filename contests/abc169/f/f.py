import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


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


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 998244353
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, S = read_ints()
A = read_ints()

# 包除？
# もしピッタリSを作ることのできる部分集合をかぶりなく複数構成できるとすると、その組み合わせを計算すればよい
# 求めたいものは→Sの部分和問題。なるべく細かく。背反に構成。
# 部分和復元してまた部分和？
# 3000*3000だから複数回DPはTLE

# 一回部分和を解いて、復元するときに分岐の数のを掛ければいいのでは？ #むしろ掛け算をDPするときにしちゃえばいいのでは？


# 1回部分和DPする
'''
dp[i][j]...[,i)の数の中の部分集合で、和をjにすることのできる通りの総数

使う可能性のあるものを全列挙できたらいいなぁ
'''

dp = [[0] * (S + 1) for _ in ra(N + 1)]
dp[0][0] = 1
for i, j in product(ra(N), ra(S + 1)):
    dp[i + 1][j] = dp[i][j]
    if j - A[i] >= 0:
        dp[i + 1][j] += dp[i][j - A[i]]

        dp[i + 1][j] %= MOD

print(*dp, sep='\n')
print(dp[-1][-1])
