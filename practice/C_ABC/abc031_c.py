# https://atcoder.jp/contests/abc031/tasks/abc031_c
# たかだか50なのでO(N^3)でもええか

# 高橋を固定して順番にまわして言ってみる
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


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


# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from fractions import gcd


def lcm(a, b):
    # 最小公約数
    g = gcd(a, b)
    return a * b // g


N = read_a_int()
A = read_ints()


def ret_points(ls, i, j):  # 閉会区間
    if i > j:
        i, j = j, i
    tmp = ls[i:j + 1]
    p_taka = sum(tmp[::2])
    p_aoki = sum(tmp[1::2])
    return p_taka, p_aoki


ans = -50 * 100
for taka in range(N):  # 高橋が選んだときに
    p_ao_tmp = -50 * 100
    for aoki in range(N):  # 青木は最も特典が多く得られる要素に丸をつける
        if taka == aoki:
            continue
        p_taka, p_aoki = ret_points(A, taka, aoki)
        if p_ao_tmp < p_aoki:
            p_ao_tmp = p_aoki
            # print(taka, aoki)
            p_taka2 = p_taka
    ans = max(ans, p_taka2)
print(ans)
