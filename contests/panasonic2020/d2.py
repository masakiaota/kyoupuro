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
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 同型→文字数カウントのパターンが同じ
# 標準形→文字数辞書順の最も速いもの


N = read_a_int()


moji = 'abcdefghij'
candi = []
moji = moji[:N]


def dfs(now, i, j):
    # j....いままでで最大の文字のidx
    # 文字を作って返す
    if i == N:
        candi.append(now)
        return
        # return now
    for jj in range(j + 2):
        dfs(now + moji[jj], i + 1, max(j, jj))


dfs('', 0, -1)

print(*sorted(candi), sep='\n')


# for i in range(1, N + 1):  # 何番目まで使うか
#     mojisyu = moji[:i]  # ここで使う文字種類
#     # 文字種類でaを先頭にできるだけ標準形を列挙したい
#     for j in range(1, i):
