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


from heapq import heapify, heappop, heappush, heappushpop


class PriorityQueue:
    def __init__(self, heap):
        '''
        heap ... list
        '''
        self.heap = heap
        heapify(self.heap)

    def push(self, item):
        heappush(self.heap, item)

    def pop(self):
        return heappop(self.heap)

    def pushpop(self, item):
        return heappushpop(self.heap, item)

    def __call__(self):
        return self.heap


heap = []  # ヒープといっても順序を工夫したただのリスト

q = PriorityQueue(heap)
q.push((-(A[0] + B[0] + C[0]), 0, 0, 0))

considered = set()
ans = []
for k_th in range(1, K+1):
    heap_max, i, j, k = q.pop()
    ans.append(-heap_max)
    for di, dj, dk in zip([1, 0, 0], [0, 1, 0], [0, 0, 1]):
        i_new, j_new, k_new = i + di, j + dj, k + dk
        if i_new >= X or j_new >= Y or k_new >= Z:
            continue
        if (i_new, j_new, k_new) in considered:
            continue
        considered.add((i_new, j_new, k_new))
        q.push((-(A[i_new] + B[j_new] + C[k_new]), i_new, j_new, k_new))


print(*ans, sep='\n')
