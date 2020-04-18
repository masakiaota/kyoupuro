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


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)

# Nはたかだか10人
# グループもたかだか3つ
# if グループが1つ→1通り
# 2つ→2**10 通りでも良い
# 3つ→3**10
# 全探索しても60074程度なので全然間に合う


N = read_a_int()
A = {}
for i in range(1, N):
    tmp = read_ints()
    for j, t in enu(tmp, start=i + 1):
        A[i - 1, j - 1] = t


ans = -1000001*100
# # 全員が同じグループ
# ans = max(ans, sum(A.values()))


def f(grp):
    ret = 0
    for g in grp.values():
        if len(g) > 1:
            for a, b in combinations(g, r=2):
                if a > b:
                    a, b = b, a
                ret += A[a, b]
    return ret


# 3グループに分かれる
for p in iter_p_adic(3, N):
    grp = {}
    for i in ra(3):
        grp[i] = []
    for i, pp in enu(p):
        grp[pp].append(i)
    ans = max(ans, f(grp))
print(ans)
