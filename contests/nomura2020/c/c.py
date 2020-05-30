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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations, accumulate
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


# 根からなるべく早く分岐していたほうが有利 (だけど作りすぎるとダメ)
# 分岐できる最大値まで分岐させるのが良いのでは？
# 分岐できる最大値は？
#
# 累積和じゃね？

# 数は求められそうになった→存在しない場合はどういうとき？
# →その場で作ることのできる葉の数を越したとき

# 二倍二倍でシミュレーション化(必要な数を超えては行けない)


N = read_a_int()
A = read_ints()

# i番目以下に必要な葉の数
n_leaves_under = list(reversed(list(accumulate(reversed(A))))) + [0]

# 頂点数をシミュレーションしていく
# 頂点数はn_leaves_underを超えては行けない
# なるべく葉の数を引いてからなるべく2倍にするほうが良い
# A[i]が現在作れる葉の数よりも大きかったら強制終了

now = 1  # 頂点の数
ans = 0
for i in range(N + 1):
    # print(now, n_leaves_under[i + 1])
    if A[i] > now:
        print(-1)
        exit()
    ans += now
    now -= A[i]
    now = min(now * 2, n_leaves_under[i + 1])

print(ans)
