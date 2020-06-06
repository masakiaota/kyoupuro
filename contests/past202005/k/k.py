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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 素直にシミュレーションするとO(n2)でおわりそう
# どうやってすばやくコンテナを見つけるか、そして移動するかがキーポイント

# f,t,xで登場したxについては、最後にどの机に移動したのかわかる
# 有向グラフとして捉えてみれば、最後に矢印の指す方向をたどれば良い
# linked listに近いか


N, Q = read_ints()
# 机の番号をi、コンテナの番号をN+iに対応させたlinkedlistてきなものを処理する
linked = [[-1, N + i] for i in range(N)]  # i番目の要素は[下、上]と連結している。連結していないものは-1
linked.extend([[i, -1] for i in range(N)])
assert len(linked) == 2 * N
desk = list(range(N, 2 * N))  # 各机の最上の番号

# print(linked)
# print(desk)
for _ in range(Q):
    f, t, x = read_ints()
    f -= 1
    t -= 1
    x += N - 1  # +N済み

    topt = desk[t]
    # 各机の上の最大の番号更新
    desk[t] = desk[f]  # 机の最上段の番号に上書き
    desk[f] = linked[x][0]  # fromはxの下の番号になる
    # リンク更新
    linked[topt][1] = x  # xを上に乗っける
    linked[linked[x][0]][1] = -1  # xの下は連結成分を外す
    linked[x][0] = topt  # 最上段のものにxを連結

#     print(linked, x, topt)
#     print(desk)

# print(linked)
# コンテナiが置かれてる机の番号か
ans = {}
for i in range(N):
    d = i
    while d != -1:
        ans[d] = i + 1
        d = linked[d][1]

# print(ans)
ans2 = []
for i in range(N, 2 * N):
    ans2.append(ans[i])
print(*ans2, sep='\n')
