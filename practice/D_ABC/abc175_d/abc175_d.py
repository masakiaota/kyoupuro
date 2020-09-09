# https://atcoder.jp/contests/abc175/tasks/abc175_d
# 各ループの1周分の累積和を作ればおけ
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
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations, accumulate
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


N, K = ints()
P = mina(*ints())
C = ints()

scores = [[] for _ in range(N)]  # 各頂点からスタートするscore
for i in range(N):
    # 頂点iからスタートする累積和を作る
    C_i = []
    now = i
    visited = set()
    while P[now] not in visited:
        now = P[now]
        C_i.append(C[now])
        visited.add(now)
    scores[i] = list(accumulate(C_i))


def get_ans(score: list, K: int)->int:
    n = (K - 1) // len(score)  # +n周回ったときにとまる
    pad = score[-1] * (n - 1)
    sco_ = []
    for s in score:
        sco_.append(s + pad)
    pad = score[-1] * (n)
    k = n * len(score)
    for s in score:
        if k == K:
            break
        sco_.append(s + pad)
        k += 1
    # print(sco_)
    return max(sco_)


print(scores)
ans = -10**10 - 114514
for i in range(N):
    score = scores[i]
    if len(score) > K or score[-1] <= 0:
        # if len(score) > + K:
        ans = max(ans, max(score))
    else:  # 何周もしたほうが良い
        # print(i)
        ans = max(ans, get_ans(score, K))

# あー最初に戻ってこない場合の閉路があるわ
# ダブリングでもありだわ
print(ans)
