# https://atcoder.jp/contests/abc067/tasks/arc078_b
# 1とNの最短経路について注目したときに、それぞれの最適戦略を考えてみる
# 相手を負かせたい→相手側に攻めて道を塞ぐのが有利(相手のマスが少なくなるから)
# 自分が負けそう→退避して行きたい(でも相手のマスのほうが多かったらもう逆転は不可能)
# 勝者は→先にたどり着けるノードの中で多い方が勝者(同数の場合はFennecが敗者)

import sys
read = sys.stdin.readline
from collections import defaultdict, deque


def read_ints():
    return list(map(int, read().split()))


class Tree:
    def __init__(self, N):
        '''重み付き無向木構造をクラスにしてみる(余り意味はなさそう)'''
        from collections import defaultdict, deque
        self.N = N
        self.tree = defaultdict(lambda: [])
        self.dists = None
        self.parents = None
        self.root = None

    def _link(self, a, b, d):
        '''
        d is a distance between a and b
        '''
        self.tree[a].append((b, d))
        self.tree[b].append((a, d))

    def _init_dists(self, root=0):
        self.dists = [-1] * self.N
        self.parents = [-1] * self.N
        que = deque([(root, -1, 0)])  # 現在のノード、前のノード、距離
        self.dists[root] = 0
        while que:
            u, p, d = que.popleft()
            self.parents[u] = p
            for nu, dadd in self.tree[u]:
                if nu == p:  # 親はもう探索しない
                    continue
                nd = d + dadd
                self.dists[nu] = nd
                que.append((nu, u, nd))

    def ret_dist(self, a, b):
        '''
        a,b間の距離を高速に取得する
        '''
        raise NotImplementedError()
        if self.parents == None:
            self._init_dists()
        # aとbの共通祖先を見つける必要がある
        # 共通祖先ってどうやって持ってくるんだろう...

    def ret_dists(self, u):
        '''
        uを始点とした各点への距離を返す (うまくやれば二度目は高速化できそう？)
        '''
        ret = [-1] * self.N
        que = deque([(u, -1, 0)])  # 現在のノード、前のノード、距離
        ret[u] = 0
        while que:
            u, p, d = que.popleft()
            for nu, dadd in self.tree[u]:
                if nu == p:  # 親はもう探索しない
                    continue
                nd = d + dadd
                ret[nu] = nd
                que.append((nu, u, nd))
        return ret


N = int(input())
graph = Tree(N)
for _ in range(N - 1):
    a, b = read_ints()
    a -= 1
    b -= 1
    graph._link(a, b, 1)

# def ret_dists(u):
#     ret = [-1] * N
#     que = deque([(u, -1, 0)])  # 現在のノード、前のノード、距離
#     ret[u] = 0
#     while que:
#         u, p, d = que.popleft()
#         nd = 1 + d
#         for nu in graph[u]:
#             if nu == p:  # 親はもう探索しない
#                 continue
#             ret[nu] = nd
#             que.append((nu, u, nd))

#     return ret


fennec_dist = graph.ret_dists(0)
snuke_dist = graph.ret_dists(N - 1)

n_snuke = 0  # snukeに近いノードの数
n_fennec = 0  # 0を含んでfennecに近い #fennecが塗れるので
for f, s in zip(fennec_dist, snuke_dist):
    if f > s:  # すぬけにちかい
        n_snuke += 1
    else:
        n_fennec += 1

# print(n_fennec, n_snuke)
if n_fennec > n_snuke:
    print('Fennec')
else:
    print('Snuke')
