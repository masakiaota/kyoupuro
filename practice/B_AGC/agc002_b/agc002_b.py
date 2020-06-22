# https://atcoder.jp/contests/agc002/tasks/agc002_b
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


N, M = read_ints()
X, Y = read_col(M)
X = mina(*X)
Y = mina(*Y)
# 有向グラフでつないで1からbfsで到達するノードの数を記録する？#順序情報が消えてしまう
# ただ単純に赤の可能性のある配列を用意しておけばいいんじゃない？
# ボールの個数も管理しないと
prob = [False] * N
n_balls = [1] * N
prob[0] = True

for x, y in zip(X, Y):
    prob[y] = prob[y] or prob[x]
    n_balls[x] -= 1
    n_balls[y] += 1
    if n_balls[x] == 0:
        prob[x] = False


print(sum(prob))
