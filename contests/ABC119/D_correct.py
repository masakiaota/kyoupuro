# from sys import stdin #入力の高速化(0.2秒ぐらい早くなる)
A, B, Q = list(map(int, input().split()))
# もし神社(寺)が1つしかなかった場合に両端に仮想の神社を建築して処理の一貫性を保つ。かりにこれが選ばれても最大のコストなので最終的に最適になることはない。
INF = 10 ** 18

# S = [-INF] + list(map(int, (stdin.readline() for _ in [0] * A))) + [INF]
# T = [-INF] + list(map(int, [stdin.readline() for _ in [0] * B])) + [INF]
S = [-INF] + [int(input()) for _ in [0]*A]+[INF]
T = [-INF] + [int(input()) for _ in [0]*B]+[INF]

from bisect import bisect_right
# from bisect import bisect_left as bisect_right #これでも通ったりする。
from itertools import product

for q in range(Q):  # 各問に答える
    x = int(input())  # 位置の指定
    s_idx, t_idx = bisect_right(S, x), bisect_right(T, x)
    ans = INF
    # 左右の神社と寺のの組み合わせ
    for s, t in product(S[s_idx - 1:s_idx + 1], T[t_idx - 1:t_idx + 1]):
        t_root = abs(t - x) + abs(s - t)  # 寺から最初に回るルート
        s_root = abs(s - x) + abs(s - t)
        ans = min(ans, t_root, s_root)
    # 最小をとって終わり
    print(ans)

'''
bisect_right

ソートされた順序を保ったまま x を a に挿入できる点を探し当てます。リストの中から検索する部分集合を指定するには、パラメータの lo と hi を使います。デフォルトでは、リスト全体が使われます。x がすでに a に含まれている場合、挿入点は既存のどのエントリーよりも後(右)になります。戻り値は、list.insert() の第一引数として使うのに適しています。a はすでにソートされているものとします。


In[3]: A
Out[3]: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

In[4]: bisect.bisect_right(A, 2.5)
Out[4]: 3

In[5]: bisect.bisect_right(A, 3)
Out[5]: 4
'''
