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
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


class Compress:
    def __init__(self, ls):
        # 座標圧縮クラス(仮) #どうしたら使いやすくなるのか知らんけど
        self.i_to_orig = sorted(set(ls))
        self.orig_to_i = {}
        for i, zahyou in enumerate(self.i_to_orig):
            self.orig_to_i[zahyou] = i
        self.len = len(self.i_to_orig)

    def __len__(self):
        return len(self.i_to_orig)


N = read_a_int()
A, B = read_col(N)
A.sort()
B.sort()
if N & 1:
    print(B[N // 2] - A[N // 2] + 1)
else:
    l = (A[N // 2 - 1] + A[N // 2])
    r = (B[N // 2] + B[N // 2 - 1])
    print(abs(r - l) + 1)


# 偶数奇数でちょっとわけて考えよう
# Nが奇数なら→X[N//2]の候補の数

# 2つの領域の重なり方は3種類
# 1. 重なっていない
# 2. 一部重なっている
# 3. 完全に内包関係にある(ちょうどの場合も含む)

# 座標圧縮と前後ろから累積和で行けない？
# 一番lをXにしたときと一番rをXにしたとき,中央値の始まる端と終わる端がわかればよい

# 奇数の場合は
