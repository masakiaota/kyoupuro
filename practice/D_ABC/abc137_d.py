# https://atcoder.jp/contests/abc137/tasks/abc137_d
# 全然わからん ソートしてgreedyに報酬高いやつから突っ込めるだけ突っ込んで見る←だめでした

# 解説AC
# 後ろから問題を見ていくと良いかも？
# 前から貪欲ではなぜ行けないのか？→残り日付が遠い中で最大の報酬 よりも 残り日付が近い中で最大の報酬を選んだほうが最適になる可能性があるから
# では後ろから貪欲は？→残り日付が近いなかで最大の報酬はその時点で最適な選択肢となることが確定できる。→OK
# だけどi日後までの候補に絞ってソートし直すと O(MNlogN)の計算量になる
# →priority queueで高速化
# つまり i 日後を考えるときに、それが可能となる候補をqueに追加 (最大logNがM回必要)
# その中で最大の報酬をもたらすものをpop (最大logNがM回必要)
# 日付のソートでNlogNかかる。
# この問題の計算量はO((M+N)logN)である


from heapq import heapify, heappop, heappush, heappushpop
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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
AB = read_tuple(N)
AB.sort()

q = PriorityQueue([])
l = 0  # ABのindex
ans = 0

for i in range(1, M + 1):
    for j in range(l, N):
        a, b = AB[j]
        if j == N - 1:
            l = N
        if a > i:  # i日以内に支払われない # こういう書き方はbugの原因になるから控えよう
            l = j
            break
        q.push(-b)  # pythonでは最小ヒープのため
    if q:  # qが空のときはしょうがないのでskip
        ans -= q.pop()
print(ans)
exit()

# 前処理しておくと実装が楽という例 (しかもソートの計算量がなくなるので更に早くなる)
N, M = read_ints()
AB = read_tuple(N)

q = PriorityQueue([])
l = 0  # ABのindex
ans = 0

from collections import defaultdict
# iに関する要素はソートしてカウントしてくんじゃなくて,defaultdict使ったほうが実装が楽だよねという話
ilimb = defaultdict(lambda: [])
for a, b in AB:
    ilimb[a].append(b)

for i in range(1, M + 1):
    for b in ilimb[i]:
        q.push(-b)  # pythonでは最小ヒープのため
    if q:  # qが空のときはしょうがないのでskip
        ans -= q.pop()

print(ans)
