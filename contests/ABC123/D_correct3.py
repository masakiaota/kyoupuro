# ヒープに要素を追加、一番でかいのを取り出すという操作を最大3000回やるやり方
# これもなかなか早い
# ヒープは追加も要素の取り出しもO(log n)で住むので、
# 計算オーダーはO(K log n)で済む(nはヒープの要素数)らしいが
# not in があるのでO(n K log n)では？
# pythonのヒープは使い方に癖があるのでこの機会に習得しよう
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


X, Y, Z, K = read_ints()
A = read_ints()
B = read_ints()
C = read_ints()

A.sort(reverse=True)
B.sort(reverse=True)
C.sort(reverse=True)

heap = []  # ヒープといっても順序を工夫したただのリスト

from heapq import heapify, heappop, heappush, heappushpop
heappush(heap, (-(A[0] + B[0] + C[0]), 0, 0, 0))

ans = []
for k_th in range(1, K+1):
    heap_max, i, j, k = heappop(heap)
    # print(heap_max)
    ans.append(-heap_max)
    for di, dj, dk in zip([1, 0, 0], [0, 1, 0], [0, 0, 1]):
        i_new, j_new, k_new = i + di, j + dj, k + dk
        if i_new >= X or j_new >= Y or k_new >= Z:
            continue
        tmp = (-(A[i_new] + B[j_new] + C[k_new]), i_new, j_new, k_new)
        if tmp in heap:  # ここが計算量がおおい
            continue
        heappush(heap, tmp)
        # print(heap)


print(*ans, sep='\n')
