# https://atcoder.jp/contests/abc100/tasks/abc100_d

# もし正の数だけだったら簡単→それぞれの合計値でソートすれば良い
# Nはたかだか1000→N^2でも間に合う

# 突っ込んだときに良くなるものを貪欲に選べばよいのでは？
# だめだ。-3 1 1 1 1とかのときに1が選ばれなくなる
# 正負の仮定を8パターン列挙しておいて、それぞれに貪欲にやる←行けそう

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


N, M = read_ints()
XYZ = read_tuple(N)


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


def sort_and_ret_score(p):
    a = [-1, 1]
    XYZ.sort(reverse=True, key=lambda x: (
        a[p[0]] * x[0] + a[p[1]] * x[1] + a[p[2]] * x[2]))  # すべて正だとして仮定した場合の最大値
    tmp = 0
    for x, y, z in XYZ[:M]:
        tmp += a[p[0]] * x + a[p[1]] * y + a[p[2]] * z
    return tmp


ans = 0
for p in iter_p_adic(2, 3):
    ans = max(ans, sort_and_ret_score(p))


print(ans)
