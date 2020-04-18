import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 1クエリをO(1)O(log N)ぐらいにしたい

# 1→ただ引けば良い (答えにちゃんと加算する)
# 2→min(奇数番)を覚えておいて,セット販売がそれを超えたらなにもしない→O(1)
# 3→min(すべて)を覚えておいて,こえたら何もしない

# 1で大小関係が変わると厄介,2,3で関係性が変わる可能性あり
# min(奇数番)をどうやったらO(1 or log N)で取得できるか？
# min(奇数番)の更新を更新するたびに行えばよい

# 1→引いたところが奇数番かしらべてminを更新するか決める
# 2→min(奇数番)←min(奇数番)-売る数a,ついでにmin(すべて)も更新するか決める
# 3→min(奇数番)←min(奇数番)-売る数a

# 奇数番を何枚売ったか
# すべてを何枚売ったかも記録

N = read_a_int()
C = read_ints()
min_even = min(C[::2])
N_even = len(C[::2])
min_all = min(C)
n_even = 0
n_all = 0

Q = read_a_int()
ans = 0


def check():
    ret = []
    for i, c in enu(C):
        c -= n_all
        if i % 2 == 0:
            c -= n_even
        ret.append(c)
    print(*ret)


for q in range(Q):
    com, *tmp = read_ints()
    if com == 1:
        x, a = tmp
        n_card = C[x - 1] - n_all - (n_even if x & 1 else 0)
        if n_card >= a:
            # 売る
            C[x - 1] -= a
            ans += a
            # 最小値の更新
            min_all = min(min_all, C[x - 1])
            if x & 1:
                min_even = min(min_even, C[x - 1])

    elif com == 2:
        a = tmp[0]
        if min_even >= a:
            ans += a * N_even
            # 統合値の更新
            min_all = min(min_all, min_even - a)
            min_even = min(min_even, min_even - a)
            n_even += a
    else:
        a = tmp[0]
        # print(min_all,min_even)
        if min_all >= a:
            ans += a * N
            # 統合値の更新
            min_even = min(min_even, min_even - a)
            min_all = min(min_all, min_all - a)
            n_all += a
    # check()


print(ans)
