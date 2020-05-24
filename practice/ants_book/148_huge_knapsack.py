# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_H
# ここでverifyできる がpythonが遅いためTLEになる...

from itertools import product
from collections import defaultdict
from typing import Dict
from bisect import bisect_left, bisect_right

N, W = map(int, input().split())

VW = []
for _ in range(N):  # 読み込み
    v, w = map(int, input().split())
    VW.append((v, w))

# # 入力例
# N, W = 4, 5
# VW = [(3, 2),
#       (2, 1),
#       (4, 2),
#       (2, 3)]

WV1 = defaultdict(lambda: -1)  # N//2だけ半分全列挙する(w_sumが同じ時はv_sumの大きい方の値を採用する)
N_half = N // 2
for bit in product(range(2), repeat=N_half):
    v_sum, w_sum = 0, 0
    for idx, to_use in enumerate(bit):
        if to_use:
            v, w = VW[idx]
            v_sum += v
            w_sum += w
    if w_sum <= W:
        WV1[w_sum] = max(WV1[w_sum], v_sum)

WV2 = defaultdict(lambda: -1)  # N-N//wの半分全列挙
for bit in product(range(2), repeat=(N - N_half)):
    v_sum, w_sum = 0, 0
    for idx, to_use in enumerate(bit, start=N_half):
        if to_use:
            v, w = VW[idx]
            v_sum += v
            w_sum += w
    if w_sum <= W:
        WV2[w_sum] = max(WV2[w_sum], v_sum)


def to_increase(WV: Dict[int, int]):
    '''WV1,WV2双方をw,vについて真に単調増加にする
    ∵あるW'以下の最大値が知りたい。逆に言うとwが増えて価値が減少するような詰め込み方はいらない'''
    ret = []
    ma = -1
    for w, v in sorted(WV.items()):
        if v <= ma:
            continue
        ma = v
        ret.append((w, v))
    return ret


WV1 = to_increase(WV1)
WV2 = to_increase(WV2)
# 二分探索ですばやくW以下となるVの最大を取得
# print(WV1, WV2)
W2, V2 = zip(*WV2)
ans = -1
for w, v in WV1:
    idx = bisect_right(W2, W - w) - 1  # W-w以下も含めるためのright
    # print(w, v, W2[idx], V2[idx])
    ans = max(ans, V2[idx] + v)


print(ans)
