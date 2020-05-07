import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


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


B = read_ints()
N = read_a_int()
A, = read_col(N)

# まず文字数が小さければ必ず小さくなるので、それを第一優先でkeyにする
# 次に同じ文字数の場合greedyに比較すれば良い
# 具体的には与えられたBを本来の順番に対応させて(以下のように)
# 0,8,1,...,2
# 0,1,2,...,9
# その文字に置換してしまえばこの世の数字に対応する

replace = {}
for i, b in enu(B):
    replace[str(b)] = str(i)


def f(x):
    x = str(x)
    ret = 0
    for xx in x:
        ret = ret * 10 + int(replace[xx])
    return ret


A.sort(key=f)
print(*A, sep='\n')
