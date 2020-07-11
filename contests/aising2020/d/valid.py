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
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

# 各クエリO(1)
# 求めたいのは0になるまでの回数
# 事前に1 - 2*10**5まで、になった回数を記録しておくとか？ (小さい方の数は必ずわかっている) (あまりがbit超で10**5を超えないのはわかっている)

# これがクリアされても 2**(2*10**5)の数字であまりが取れない問題がある
# 一回割ってしまえばこっちのもんよ

N = a_int()
X = read()[:-1]


px0 = X.count('1')
# 上から、px0-1, px0, px0+1で割ったもの
x0n = 0
x0 = 0
x0p = 0
for i, bit in enu(reversed(X)):
    if bit == '1':
        x0n += pow(2, i, px0 - 1)
        x0n %= px0 - 1
        x0 += pow(2, i, px0)
        x0 %= px0
        x0p += pow(2, i, px0 + 1)
        x0p %= px0 + 1


x0 = [x0n, x0, x0p]  # px0-1, px0, px0+1で割ったもの

first_mod = []  # 後ろから順番に1回だけ操作を行ったときの数字(Xi % popcount(Xi))
for i, bit in enumerate(reversed(X)):
    if bit == '0':
        # 1に反転する
        first_mod.append(x0[2] + pow(2, i, px0 + 1))
    elif bit == '1':
        first_mod.append(x0[0] + pow(2, i, px0 - 1))


print()
# これは必ずpx0+-1よりも小さいので、事前に対応表を作っておけばすぐに求められる(それとも求めなくてもいいか？)(logがつくからしなくても良さそう)

ans = []
for x in reversed(first_mod):
    cnt = 1  # すでに一回操作しているので
    while x != 0:
        x = x % bin(x).count('1')
        cnt += 1
    ans.append(cnt)


print(*ans, sep='\n')
