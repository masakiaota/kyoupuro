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


N, T = ints()
A, B = read_col(N)
# より高速化する宿題順にソートしてその順にやるのが最適
t_fast = [b - a for a, b in zip(A, B)]
t_fast.sort()
time = sum(A)
ans = 0
while ans < N and time > T:
    time += t_fast[ans]
    ans += 1
print(ans if time <= T else -1)
