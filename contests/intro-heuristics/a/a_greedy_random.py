# greedy is all you need
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


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


def score(D, C, S, T):
    last = [-1] * 26
    # scores = [0]
    score = 0
    for d in range(D):
        # scores.append(scores[-1] + S[d][T[d]])
        score += S[d][T[d]]
        last[T[d]] = d
        for i in range(26):
            # scores[-1] -= C[i] * (d - last[i])
            score -= C[i] * (d - last[i])  # この場で一番罰則が大きいやつを使うとか？
    return score


D = a_int()
C = ints()
S = read_tuple(D)

T = []

last = [-1] * 26
for d in range(D):
    ma = -INF
    for i in range(26):
        tmp = S[d][i] + C[i] * (d - last[i])
        if tmp > ma:
            t = i
            ma = tmp
    last[t] = d  # Tを選んだあとで決める
    T.append(t)

# このTを一つずつ変えてもっと良くできないか試してみる
tmpT = T.copy()
for i in range(D):
    for j in range(26):
        tmpT[i] = j
        tmp_score = score(D, C, S, tmpT)


print(*mina(*T, sub=-1), sep='\n')
