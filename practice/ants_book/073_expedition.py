# http://poj.org/problem?id=2431

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

    def __len__(self):
        return len(self.heap)


N = 4
L = 25
P = 10
A = [10, 14, 20, 21]  # 座標
B = [10, 5, 2, 4]  # 補給量

A.reverse()
B.reverse()

# すでに通過したガソリンスタンドを使う権利があると解釈する
pq = PriorityQueue([])
# 距離Lはたかだか10**6なので距離を1づつforで回しても間に合う
ans = 0
for x in range(1, L):  # Lの手前までに1リットルあれば良い
    P -= 1
    if A and A[-1] == x:
        pq.push(-B[-1])
        del B[-1]
        del A[-1]
    if P == 0:
        if pq:
            P -= pq.pop()
            ans += 1
        else:
            print(-1)
            exit()
print(ans)
