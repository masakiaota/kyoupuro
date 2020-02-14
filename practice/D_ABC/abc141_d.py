# https://atcoder.jp/contests/abc141/tasks/abc141_d
# 割引券は半額にする操作
# 値段が大きいものに貪欲に使っていきたい
# 割って逐一挿入して最大のものをまた割る？
# 配列平衡二分木で実装←atcoderの環境が古くて使えなかった！


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


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


N, M = read_ints()
A = read_ints()
que = PriorityQueue([-a for a in A])
for _ in range(M):
    x = -que.pop()
    x //= 2
    que.push(-x)

print(-sum(que.heap))
