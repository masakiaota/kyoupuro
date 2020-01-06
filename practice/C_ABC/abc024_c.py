# https://atcoder.jp/contests/abc024/tasks/abc024_c
# 区間をマージ → ユニオンファインド? →流石に10*9のノードを2秒で処理するのは厳しい
# 区間をマージしていって、その都度s,tがつながるかを検証 →誤答です
# 確かに貪欲法で解けるけど区間を管理したい。


from collections import defaultdict
N, D, K = list(map(int, input().split()))
LR = []
for _ in range(D):
    LR.append(tuple(map(int, input().split())))

now = []
T = []
for _ in range(K):
    s, t = list(map(int, input().split()))
    now.append(s)
    T.append(t)


ans = defaultdict(lambda: 10 ** 9)
for i in range(D):
    l, r = LR[i]
    for k in range(K):
        if now[k] == T[k]:
            continue
        if l <= now[k] <= r:
            # ゴール
            if l <= T[k] <= r:
                now[k] = T[k]
                ans[k] = i + 1
                continue
            now[k] = l if now[k] > T[k] else r

for i in range(K):
    print(ans[i])


# 以下がんばったけど誤答
# from collections import deque, defaultdict
# from bisect import bisect_left, bisect_right

# N, D, K = list(map(int, input().split()))
# LR = []
# for _ in range(D):
#     LR.append(tuple(map(int, input().split())))

# ST = []
# for _ in range(K):
#     ST.append(tuple(map(int, input().split())))


# L = deque([-10 ** 10, 10 ** 10])
# R = deque([-10 ** 10, 10 ** 10])
# ans = defaultdict(lambda: 10**9)
# for i, (l, r) in enumerate(LR):
#     # 区間マージの実装
#     print(l, r)
#     l_idx = bisect_left(L, l) - 1
#     r_idx = bisect_left(R, r)
#     if L[l_idx] < l and R[l_idx] >= l:
#         # l_new = L[l_idx]
#         l = L[l_idx]
#     if R[r_idx] > r and L[r_idx] <= r:
#         # r_new = R[r_idx]
#         r = R[r_idx]
#     # delete until no overwrap
#     idx_del = []
#     for i in range(r_idx, l_idx - 1, -1):
#         if l <= L[i] and R[i] <= r:
#             idx_del.append(i)
#     for i in idx_del:
#         del L[i]
#         del R[i]

#     idx_insert = bisect_left(L, l)
#     assert idx_insert == bisect_left(R, r)
#     L.insert(idx_insert, l)
#     R.insert(idx_insert, r)
#     print(L)
#     print(R)

#     # 問題固有の判定
#     for j, (s, t) in enumerate(ST):
#         for l, r in zip(L, R):
#             if l <= s and t <= r:
#                 ans[j] = min(ans[j], i)

# for i in range(K):
#     print(ans[i])
