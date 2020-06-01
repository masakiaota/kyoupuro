# めちゃくちゃ頭いいな
# k番目の数は？→x以下の数がk個以上存在する最小のx を探索
# 区間クエリに答えるには→ √Nごとに区切ったバゲットをソートしておけば都合が良い
# 良い都合→ある区間の処理ははたかだかO(√N logn)でx以下の数の個数を取得できる ∵バゲット内では二分探索で、要素はそのまま見て、x以下の個数を得られる

from typing import List
from math import sqrt
from bisect import bisect_right

n = 7
m = 3
A = [1, 5, 2, 6, 3, 7, 4]
query = [(2, 5, 3), (4, 4, 1), (1, 7, 3)]


B = int(sqrt(n)) + 1  # bucketのサイズ

bucket: List[List[int]] = [[] for _ in range(B)]  # Aのbucket. 各要素はソートされた数列
for i in range(n):
    # print(i // B, i)
    bucket[i // B].append(A[i])

for i in range(len(bucket)):
    bucket[i].sort()


def solve_query(i, j, k):
    # 二分探索でk番目の数字を探索する

    l, r = i - 1, j  # 0basedindex そして 半開区間にする
    l_bucket = (l - 1) // B + 1  # bucketのidx
    r_bucket = r // B  # 半開区間なのでこれでいい

    def is_ok(x):
        # x以下の数がk個以上ならok
        num_el_x = 0
        num_el_x += sum([xx <= x for xx in A[l:l_bucket * B]])
        num_el_x += sum([xx <= x for xx in A[r_bucket * B:r]])
        for i in range(l_bucket, r_bucket):
            num_el_x += bisect_right(bucket[i], x)
        return num_el_x >= k

    def meguru_bisect(ng, ok):
        while (abs(ok - ng) > 1):
            mid = (ok + ng) // 2
            if is_ok(mid):
                ok = mid
            else:
                ng = mid
        return ok

    print(meguru_bisect(-10**9 - 1, 10**9 + 1))


for i, j, k in query:
    solve_query(i, j, k)
