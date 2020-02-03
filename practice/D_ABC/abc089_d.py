# https://atcoder.jp/contests/abc089/tasks/abc089_d

# queryがたくさんある系
# query一つにつきO(1)ぐらいで応答しないと行けない→前処理の必要あり

# あるマスについてはD倍したマスだけ把握しておけばいいんじゃない？
'''
D=2
1 4 3
2 5 7
8 9 6
'''
# だったらstart=1,2において、移動のスコアの累積和を入れていく
# Qに回答時は、L%D番目のstartの累積和において、L//Dにおいて累積和の差を求めれば良い
# ピッタリDだった場合に面倒なので-0.5して置くのは良いかもしれない

from collections import defaultdict
from itertools import accumulate
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def get_cost(i, j, x, y):
    return abs(x - i) + abs(y - j)


H, W, D = read_ints()
A = read_matrix(H)
ruiseki = defaultdict(lambda: [])
num_to_co = {}

for i, a in enumerate(A):
    for j, aa in enumerate(a):
        num_to_co[aa] = (i, j)

for num in range(1, D + 1):
    pre_i, pre_j = num_to_co[num]
    pre_num = num
    ruiseki[num].append(0)
    while W * H >= pre_num + D:
        now_num = pre_num + D
        i, j = num_to_co[now_num]
        cost = get_cost(pre_i, pre_j, i, j)
        ruiseki[num].append(cost)
        # 必要なもの、前のi,j今のi,j 今の番号
        # 更新するもの 前のi,j←今のi,j 今の番号を次の番号に
        pre_i, pre_j = i, j
        pre_num = now_num

    ruiseki[num] = list(accumulate(ruiseki[num]))
# print(ruiseki)

# Qに答える番だ
Q = int(input())
for _ in range(Q):
    L, R = read_ints()
    tmp = ruiseki[L % D if L % D != 0 else D]
    # print(tmp)
    # print((R - 1) // D, (L - 1) // D)
    print(tmp[(R - 1) // D] - tmp[(L - 1) // D])
