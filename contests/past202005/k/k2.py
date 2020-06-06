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


class linkedlist:
    def __init__(self, N):
        '''要素がN個あるlinkedlistをつくる'''
        self.ls = [[-1, -1]
                   for _ in range(N)]  # 0はleft,1はrightへのリンク #-1は接続されていない

    def link(self, i, j):
        '''i番目の要素の右にj番目の要素をつける
        iの右、jの左へのリンクは破棄する'''
        if self.ls[i][1] != -1:  # もし接続されていればカットしておく
            self.cut(i, 1)
        self.ls[i][1] = j
        if self.ls[j][0] != -1:
            self.cut(j, 0)
        self.ls[j][0] = i

    def cut(self, i, is_right):
        '''i番目のis_right側のリンクを切る'''
        adj = self.ls[i][is_right]
        if adj == -1:  # 無いほうがいいかも？
            ValueError('unlinked element')
        self.ls[adj][1 - is_right] = -1
        self.ls[i][is_right] = -1


# 素直にシミュレーションするとO(n2)でおわりそう
# どうやってすばやくコンテナを見つけるか、そして移動するかがキーポイント

# f,t,xで登場したxについては、最後にどの机に移動したのかわかる
# 有向グラフとして捉えてみれば、最後に矢印の指す方向をたどれば良い
# linked listに近いか


N, Q = read_ints()


# 机の番号をi、コンテナの番号をN+iに対応させたlinkedlistてきなものを処理する
linked = linkedlist(2 * N)
for i in ra(N):
    linked.link(i, N + i)
desk = list(range(N, 2 * N))  # 各机の最上の番号

for _ in range(Q):
    f, t, x = read_ints()
    f -= 1
    t -= 1
    x += N - 1  # +N済み

    topt = desk[t]
    # 各机の上の最大の番号更新
    desk[t] = desk[f]  # 机の最上段の番号に上書き
    desk[f] = linked.ls[x][0]  # fromはxの下の番号になる

    # リンク更新
    linked.link(topt, x)


ans = {}
for i in range(N):
    d = i
    while d != -1:
        ans[d] = i + 1
        d = linked.ls[d][1]

# print(ans)
ans2 = []
for i in range(N, 2 * N):
    ans2.append(ans[i])
print(*ans2, sep='\n')
