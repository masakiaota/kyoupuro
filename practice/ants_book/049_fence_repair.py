# http://poj.org/problem?id=3253
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


N = 3
L = [8, 5, 8]

# 短いものからgreedyにマージしていくのが最適
# 一番目に短いものと次に短いものを高速に(わかりやすく)取得するためにpriority queueを用いる

pq = PriorityQueue(L)
ans = 0
while len(pq) > 1:
    mi1 = pq.pop()
    mi2 = pq.pop()
    new = mi1 + mi2
    ans += new
    pq.push(new)
print(ans)
