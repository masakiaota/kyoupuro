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


N, K = read_ints()
A, = read_col(N)
# 減少に転じるidxだけ取得しておけばOKでは？
pre = -1
decreidx = [0]  # 初めて減少したidx
for i, a in enu(A):
    if pre >= a:
        decreidx.append(i)
    pre = a
decreidx.append(N)
ans = 0
for i in range(len(decreidx) - 1):
    ans += max(0, decreidx[i + 1] - decreidx[i] - (K - 1))
print(ans)
