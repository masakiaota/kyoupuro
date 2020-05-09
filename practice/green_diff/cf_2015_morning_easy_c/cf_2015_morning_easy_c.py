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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import

N, K, M, R = read_ints()
# コーナーケース
if N == 1:
    print(R)
    exit()

S, = read_col(N - 1)
S.sort(reverse=True)

if sum(S[:K]) >= R * K:
    print(0)
    exit()

x = R * K - sum(S[:K - 1])
if x > M:
    print(-1)
else:
    print(x)
