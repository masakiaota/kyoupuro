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


def run_length_encoding(s):
    '''連長圧縮を行う
    s ... iterable object e.g. list, str 
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx'''
    s_composed, s_sum = [], []
    s_idx = [0]
    pre = s[0]
    cnt = 1
    for i, ss in enumerate(s[1:], start=1):
        if pre == ss:
            cnt += 1
        else:
            s_sum.append(cnt)
            s_composed.append(pre)
            s_idx.append(i)
            cnt = 1
            pre = ss
    s_sum.append(cnt)
    s_composed.append(pre)
    # assert len(s_sum) == len(s_composed)
    return s_composed, s_sum, s_idx


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

N = a_int()
A = ints()

# 原則安いときに買って高いときに売る
# 複利の計算が入るのがめんどいな
# 極大極小みつけて売り買いかな
# 同じ金額の日は無いも等しいので消しちゃうか
A, _, _ = run_length_encoding(A)
A = [400] + A + [0]
# print(A)

torihiki = []
for i in range(len(A) - 2):
    if A[i] > A[i + 1] and A[i + 1] < A[i + 2]:
        torihiki.append(A[i + 1])
    elif A[i] < A[i + 1] and A[i + 1] > A[i + 2]:
        torihiki.append(A[i + 1])

# 愚直シミュレーション
en = 1000
kabu = 0

for i, a in enu(torihiki):
    if i & 1:  # 売る
        en += kabu * a
        kabu = 0
    else:  # 買う
        kabu = en // a
        en %= a
print(en)
