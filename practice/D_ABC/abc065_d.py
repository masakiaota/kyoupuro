# https://atcoder.jp/contests/abc065/tasks/arc076_b
# 全結合グラフを作ってから最小全域木を作るとメモリも時間も足りない。
# 座標(a,b)にある町に隣接している街はx,yでソートしたときに一番近い街なので、そことのリンクさえ貼っておけば良いのでは


import sys
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


from operator import itemgetter
from scipy.sparse import lil_matrix, csr_matrix
N = read_a_int()
XY = set(read_tuple(N))
num_to_zahyou = {}
zahyou_to_num = {}
for i, (x, y) in enumerate(XY):
    num_to_zahyou[i] = (x, y)
    zahyou_to_num[x, y] = i
XY_sortedX = sorted(XY)
XY_sortedY = sorted(XY, key=itemgetter(1, 0))

graph = lil_matrix((i, i), dtype='int32')
for x, y in XY_sortedX:
     # なんか違うな
