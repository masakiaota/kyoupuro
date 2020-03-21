
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


MOD = 10**9 + 7
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from fractions import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g


# 白い通路を作りたい

# 連続する黒領域は一気にひっくり返したい。
# bfsで一番黒の少ない経路を選ぶ？ 反例は？
'''
.....
#####
#####
#####
.....
これだったら1回で済む
'''

# 下か右に進むときに#だったら連続して進んでよくね？(状態を持っておいて)

'''
.....
#####
##.##
#####
.....
これだったら1回で済む
'''

H, W = read_ints()
S = read_map_as_int(H)  # が1 .が0

from collections import deque
# bfs
cost = 1 if S[0][0] == 1 else 0
que = deque([(0, 0, cost)])
ans = 100000
mv = [(1, 0), (0, 1)]
cnt = 0
while que:
    cnt += 1
    i, j, cost = que.popleft()
    print(i, j, cost)
    if i == H - 1 and j == W - 1:
        ans = min(cost, ans)
    for di, dj in mv:
        ni = i + di
        nj = j + dj
        if 0 <= ni < H and 0 <= nj < W:
            if S[ni][nj] == 1 and S[i][j] == 0:
                que.append((ni, nj, cost + 1))
            else:
                que.append((ni, nj, cost))
print(ans, cnt)
