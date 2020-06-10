# https://atcoder.jp/contests/arc004/tasks/arc004_2
# maxは合計だから簡単
# minは二回折ったときに三角不等式を満たす場合に0
# Nはたかだか500なので折れる点を全探索しても間に合う
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_a_int(): return int(read())


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


from itertools import combinations

N = read_a_int()
D, = read_col(N)

# コーナーケース
if N == 1:
    print(sum(D))
    print(sum(D))
    exit()


def ret_result(*argv):
    # 三角になるか？
    argv = sorted(argv)
    a, b, c = argv
    return max(c - a - b, 0)


ans = 30000 * 500 + 1
for i, j in combinations(range(N), r=2):
    x = sum(D[:i])
    y = sum(D[i:j])
    z = sum(D[j:])
    ans = min(ans, ret_result(x, y, z))

print(sum(D))
print(ans)
