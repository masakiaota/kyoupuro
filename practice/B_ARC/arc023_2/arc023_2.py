# https://atcoder.jp/contests/arc023/tasks/arc023_2
# ちょうどD回で行ける範囲と偶数奇数で分ける
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


def ints(): return list(map(int, read().split()))


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret


R, C, D = ints()
A = read_matrix(R)
ans = -1
for r in range(R):
    if r > D:
        break
    for c in range(C):
        if r + c > D:  # ピッタリは許容可能
            break
        if D % 2 == (r + c) % 2:
            ans = max(ans, A[r][c])
print(ans)
