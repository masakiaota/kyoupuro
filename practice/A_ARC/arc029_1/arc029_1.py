# https://atcoder.jp/contests/arc029/tasks/arc029_1
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


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


from itertools import product
N = a_int()
T, = read_col(N)
# N<=4なので全通り書き出せる
# 各肉をどの焼き器に割り当てるかをbit全探索
ans = 2 ** 31
for assigns in product(range(2), repeat=N):
    t1, t2 = 0, 0
    for i, tf in enumerate(assigns):
        if tf:
            t1 += T[i]
        else:
            t2 += T[i]
    ans = min(ans,
              max(t1, t2))
print(ans)
