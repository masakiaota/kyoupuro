# https://atcoder.jp/contests/abc015/tasks/abc015_4
# ナップサック問題 枚数も伝播する #枚数を超えないうちで一番価値のあるものを出力すれば良い

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


from itertools import product
W = read_a_int()
N, K = read_ints()
AB = read_tuple(N)

# 枚数も伝播するdp
# dp[i][j]...i-1番目までの商品まで考慮したとき、重さj以下で達成できる(スクショ枚数、価値の最大値)
# scs[i][j] ... 上記に対応するスクショ枚数
dp = [[0] * (W + 1) for _ in range(N + 1)]  # 初期化も同時に
scs = [[0] * (W + 1) for _ in range(N + 1)]  # 初期化も同時に #これだと貪欲的に選んでる気がするな

# dpしながら答えを取得してもいい
ans = 0
for i, j in product(range(N), range(1, W + 1)):
    w, v = AB[i]

    notselect = dp[i][j]
    select = dp[i][j - w] + v if j - w >= 0 else -1
    if select > notselect:
        s = scs[i][j - w] + 1
    else:
        s = scs[i][j]
    tmp = max(select, notselect)
    dp[i + 1][j] = tmp
    scs[i + 1][j] = s
    if s <= K:
        ans = max(ans, tmp)
print(ans)
from pprint import pprint
pprint(dp)
pprint(scs)

'''
6
4 1
1 2
2 2
3 2
3 3

みたいな入力のときにこれは2を出力してしまう。正確には、取ったほうがスコアが高くなるが、取らないほうが枚数は少なく済むみたいなときに死ぬ。
'''
