# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/9/ALDS1_9_C
# これもライブラリを使えば一瞬
from heapq import heapify, heappop, heappush, heappushpop

heap = []
# データの入力
while True:
    tmp = input()
    if tmp.startswith('end'):
        break
    elif tmp.startswith('insert'):
        k = int(tmp.split()[1])
        heappush(heap, -k)
    elif tmp.startswith('extract'):
        print(-heappop(heap))
