# https: // onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/all/ALDS1_3_B
# queueの練習
# 単純にqueueで実装しようとするとO(N**2)になる。
# リングバッファを用いるとpopとappnedの操作のコストがO(1)に抑えられるの
# そのため全体としてはO(N)までオーダーが落ちる
# なお、pythonのdequeは最適化されていて、popのコストはO(1)に抑えられたままである

n, q = list(map(int, input().split()))

# pythonに甘えたサボり実装
from collections import deque
que = deque([])
for _ in range(n):
    que.append((int(x) if x.isdigit() else x for x in input().split()))

tottime = 0
while que:
    name, lefttime = que.popleft()
    if lefttime <= q:
        tottime += lefttime
        print(name, tottime)
    else:
        lefttime -= q
        tottime += q
        que.append((name, lefttime))

# リングバッファによる実装
# リングサイズはnにしておけば良いのかな？
# なんかTLEになる
# process_ls = []
# lefttime_ls = []
# for _ in range(n):
#     p, t = input().split()
#     process_ls.append(p)
#     lefttime_ls.append(int(t))

# n_process_left = n
# i = 0
# tottime = 0
# while n_process_left:
#     # idx = i % n
#     lefttime = lefttime_ls[i]
#     if lefttime != 0:
#         if lefttime <= q:
#             tottime += lefttime
#             lefttime_ls[i] = 0
#             n_process_left -= 1
#             print(process_ls[i], tottime)
#         else:
#             tottime += q
#             lefttime_ls[i] = lefttime-q
#     i += 1
#     if i == n:
#         i = 0
