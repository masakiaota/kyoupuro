# 木でよく使うテクを管理する


class Tree:
    def __init__(self, N):
        '''重み付き無向木構造をクラスにしてみる(余り意味はなさそう)
        !!!未完成!!!
        '''
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
