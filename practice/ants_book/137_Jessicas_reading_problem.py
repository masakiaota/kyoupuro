# 面倒なのでセグ木で殴るか迷ったけど 素直にしゃくとり法で実装

P = 5
A = [1, 8, 8, 1, 1]  # ちょっと 蟻本のサンプルと違うけど

n = len(set(A))  # ユニークの種類数

from collections import Counter
cnt = Counter([])  # default dict代わり

idxs = []
r = 0
num = 0  # 種類数
for l in range(P):
    while r < P and num + (cnt[A[r]] == 0) < n:  # 初めて条件を満たす一歩手前をr)にする。
        if cnt[A[r]] == 0:
            num += 1
        cnt[A[r]] += 1
        r += 1
    idxs.append((l, r))
    if r == P:
        break  # これ以上短くしても条件を満たすことはない
    # 抜けるときの更新
    if cnt[A[l]] == 1:
        num -= 1
    cnt[A[l]] -= 1


print(min([r - l + 1 for l, r in idxs]))
