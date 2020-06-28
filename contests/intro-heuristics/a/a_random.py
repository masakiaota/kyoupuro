import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


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

# random variable is all you need


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
            score -= C[i] * (d - last[i])
    return score


from random import choices, seed

D = a_int()
C = ints()
S = read_tuple(D)

candi = list(range(26))

bestscore = -INF
for i in range(3 * 10000):
    seed(i)
    T = choices(candi, k=D)
    tmpscore = score(D, C, S, T)
    if tmpscore > bestscore:
        bestscore = tmpscore
        ansT = T
# print(score(D, C, S, ansT))
# print(bestscore)

print(*mina(*ansT, sub=-1), sep='\n')
