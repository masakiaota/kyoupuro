# https://atcoder.jp/contests/abc175/tasks/abc175_d
# ダブリングで解く
# K>ループ長の場合、累積の最後の値が負ならmaxを取ればいいし、正ならば 累積の最後の値*周数をかけてから計算
import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import

from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


N, K = ints()
P = mina(*ints())
C = ints()

next = [[-1] * N for _ in range(32)]  # next[k][i] ノードiから2^k回移動するときのノード
score = [[0] * N for _ in range(32)]  # score[k][i] ノードiから2^k回移動するときに加算されるスコア
# nextの構築
next[0] = P
for k in range(31):
    for i in range(N):
        next[k + 1][i] = next[k][next[k][i]]
# scoreの構築
for i in range(N):
    score[0][i] = C[P[i]]
for k in range(31):
    for i in range(N):
        score[k + 1][i] = score[k][i] + score[k][next[k][i]]


# これじゃ道中がわからない...
# 一点しか求まらない
def get_score(i, k):  # 途中の1点のscoreを算出
    now = i
    ret = 0
    for j in range(k.bit_length()):
        if (k >> j) & 1:
            ret += score[j][now]
            now = next[j][now]
    return ret, now


# 各ノードからK回移動したときのscoreを計算する
ans = -10**10
for i in range(N):
    # print(i, 'について')
    if K <= N:
        # 素直に最初からK回シミュレーションする
        tmp = 0
        now = i
        for _ in range(K):
            tmp += score[0][now]
            now = next[0][now]
            ans = max(tmp, ans)
            # print(tmp)
    else:
        # 最初からN回シミュレーション + K-NからN回シミュレーション
        tmp = 0
        now = i
        for j in range(N):
            tmp += score[0][now]
            now = next[0][now]
            ans = max(tmp, ans)
            # print(tmp)

        tmp, now = get_score(i, K - N)
        for j in range(N):
            tmp += score[0][now]
            now = next[0][now]
            ans = max(tmp, ans)
            # print(tmp)


print(ans)
