# https://atcoder.jp/contests/dwacon5th-prelims/tasks/dwacon5th_prelims_b
# 解説見てしまった
# N≤1000の性質をうまく使う必要はない。O(N^2)で全部書き出せばいい。完全に引っ掛け。
# 答えの仮定、bitごとに見る、上位桁からのgreedyらへんがキーワード

from itertools import combinations
N, K = map(int, input().split())
A = list(map(int, input().split()))
B = [sum(A[s:t]) for s, t in combinations(range(N + 1), 2)]

l = max(B).bit_length()  # 何桁扱えば十分か？
x = 0  # 初期値
for i in range(l, -1, -1):  # 上位桁からgreedyに決定
    # 候補のうち i bit目=1がK個以上あれば答えに計上できる
    x_tmp = x + (1 << i)
    if sum((x_tmp & b) == x_tmp for b in B) >= K:  # 確定した桁+i桁を1としたときに答えになり得るか？
        x = x_tmp
print(x)
