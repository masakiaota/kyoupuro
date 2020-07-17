# https://atcoder.jp/contests/arc036/tasks/arc036_b
# これも極大極小を調べる問題ぽいっすね
# 極小点を記録していく
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


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


N = a_int()
H, = read_col(N)
ids_lmin = [0]
for i in range(1, N - 1):
    if H[i - 1] > H[i] and H[i] < H[i + 1]:
        ids_lmin.append(i)
ids_lmin.append(N - 1)

ans = 0
for s, u in zip(ids_lmin, ids_lmin[1:]):
    ans = max(ans, u - s + 1)
print(ans)
