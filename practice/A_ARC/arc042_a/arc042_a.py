# https://atcoder.jp/contests/arc042/tasks/arc042_a
# データ構造で殴るか、逆順から読むか
# 呼ばれたものは必ず先頭に行くのだからaiを逆から表示していけば良い

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


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


N, M = map(int, input().split())

A, = read_col(M)
used = set()
ans = []
for a in reversed(A):
    if a in used:
        continue
    ans.append(a)
    used.add(a)

for i in range(1, N + 1):
    if i in used:
        continue
    ans.append(i)
print(*ans, sep='\n')
