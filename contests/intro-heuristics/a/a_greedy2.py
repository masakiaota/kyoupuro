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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce


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


def maximizer(newT, bestT, bestscore):
    tmpscore = score(D, C, S, newT)
    if tmpscore > bestscore:
        return newT, tmpscore
    else:
        return bestT, bestscore

# editorialよりd日目の改善は、改善せずにd+n_days経過したときの関数にしたほうが
# 最終的なスコアと相関があるんじゃない？


def make_T(n_days):
    T = []
    last = [-1] * 26
    for d in range(D):
        ma = -INF
        for i in range(26):
            tmp = S[d][i]
            for j in range(n_days):
                tmp += C[i] * (d + j - last[i])
            if tmp > ma:
                t = i
                ma = tmp
        last[t] = d  # Tを選んだあとで決める
        T.append(t)
    return T


T = make_T(0)
sco = score(D, C, S, T)
for i in range(1, 26):
    T, sco = maximizer(make_T(i), T, sco)

# print(score(D, C, S, T))
print(*mina(*T, sub=-1), sep='\n')
