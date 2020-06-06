import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret


    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため
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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# priority queue2つじゃない？
# pqに突っ込む情報(-消費期限, 列数, 商品id)

N = read_a_int()
T = []
cnt = 0
for i in range(N):
    K, *t = read_ints()
    tmp = []
    for tt in t + [-1, -1]:  # -1は番兵
        tmp.append((-tt, i, cnt))
        cnt += 1
    T.append(list(reversed(tmp)))

# print(*T, sep='\n')
M = read_a_int()
A = read_ints()

pq1 = PriorityQueue([])  # 1番目の商品の大きさを管理
pq2 = PriorityQueue([])  # 2番目の商品の大きさ管理
items2 = []  # 2番目の商品順番を管理
for i in range(N):  # データを適切に格納
    new = T[i].pop()
    pq1.push(new)
    new = T[i].pop()
    items2.append(new)
    pq2.push(new)
    # これでT[i]には3番目からの商品しかなくなった
    # 2番目の商品はitems2にある


ans = []
used = set()
for a in A:
    if a == 1:
        while pq1.heap[0][2] in used:  # 多分いらないけど #使われていないなかで最大のtまで捨てる
            print('おかしい！')
            pq1.pop()
        t, i, tid = pq1.pop()
        used.add(tid)
        ans.append(-t)
        pq1.push(items2[i])
        items2[i] = T[i].pop()
        pq2.push(items2[i])

    else:
        # 使ったものを捨てる処理
        while pq1.heap[0][2] in used:
            print(ans)
            print(used, pq1.heap[0])
            print('おかしい！')
            pq1.pop()
        while pq2.heap[0][2] in used:
            pq2.pop()

        # なぜか等号を外すとWA #→まだ使ってないけど、1番目に存在する商品は1,2の両方に登場しうる(商品自体は1にある
        if pq1.heap[0][0] <= pq2.heap[0][0]:
            # 1番目から取る→上の処理と同じ
            t, i, tid = pq1.pop()
            used.add(tid)
            ans.append(-t)
            pq1.push(items2[i])
            items2[i] = T[i].pop()
            pq2.push(items2[i])
        else:  # 2番目から取る
            t, i, tid = pq2.pop()
            ans.append(-t)
            used.add(tid)
            items2[i] = T[i].pop()
            pq2.push(items2[i])

print(*ans, sep='\n')
