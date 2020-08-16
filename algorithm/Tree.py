# 木でよく使うテクを管理する
# オイラーツアー した結果をセグ木に乗っけるみたいなこともしたいけどね
# 辺と頂点の2version実装したいね
# 直径
# LCA(ダブリング)
# 全方位木DP?(無理)


class Tree:
    def __init__(self, N: int):
        """N頂点の重みなし木クラス
        宣言後 link()->set_root()してからいろいろする

        Args:
            N (int): ノード数
        """
        from collections import defaultdict
        self.N = N
        self.tree = defaultdict(lambda: [])
        self.dists = None
        self.parents = None
        self.children = None
        self.root = None
        self.up = None  # doubling用

    def link(self, a, b):  # 木を作るとき用
        self.tree[a].append((b))
        self.tree[b].append((a))

    def set_root(self, root: int):
        # ルート決定時に子方向と親方向とdistが決まるし
        from collections import deque, defaultdict
        self.up = None  # ダブリングのやつは初期化
        self.root = root
        self.dists = [-1] * self.N
        self.parents = [-1] * self.N
        self.children = defaultdict(lambda: [])
        que = deque([(root, -1, 0)])  # 現在のノード、前のノード、距離
        self.dists[root] = 0
        while que:
            u, p, d = que.popleft()
            self.parents[u] = p
            for nx in self.tree[u]:
                if nx == p:  # 親はもう探索しない
                    continue
                nd = d + 1
                self.children[u].append(nx)
                self.dists[nx] = nd
                que.append((nx, u, nd))

    def node_euler_tour(self):
        """ノード上のオイラーツアーをする

        Returns:
            tour: tour[i] ... i回目に訪問したノード
            inn: inn[u] ... 初めてuを訪問したときの回数(tourのidxに対応) (u以下の部分木に入るとき)
            out: out[u] ... 最後にuを訪問したときの回数(tourのidxに対応) (u以下の部分木から抜けるとき)
        """

        assert self.children != None, 'set_rootしてないかも'
        tour = []  # tour[i] ... i回目に訪問したノード
        inn = [-1] * self.N  # inn[u] ... 初めてuを訪問したときの回数(tourのidxに対応)
        out = [-1] * self.N  # out[u] ... 最後にuを訪問したときの回数(tourのidxに対応)
        cnt = 0

        def dfs(u):
            nonlocal cnt
            tour.append(u)
            inn[u] = cnt
            cnt += 1
            for nx in self.children[u]:
                dfs(nx)
            tour.append(u)
            out[u] = cnt
            cnt += 1
        dfs(self.root)
        return tour, inn, out

    def lca(self, x: int, y: int):
        '''x,yのlowest common ancestor'''
        if self.up == None:
            self._doubling()

        dx = self.dists[x]  # depthと同じ
        dy = self.dists[y]
        if dx > dy:  # 必ずdxのほうが小さくなるように
            dx, dy = dy, dx
            x, y = y, x
        # 同じ高さまで移動させる
        y = self._up(y, dy - dx)  # xと同じ高さになるまでyを上昇させる
        if x == y:  # 早期終了
            return x

        # 二分探索でlcaを求める →upがおなじになるdepthの最小
        ng = dx
        ok = 0
        while (abs(ok - ng) > 1):
            mid = (ok + ng) // 2
            # isokを求める
            dd = dx - mid
            xx = self._up(x, dd)
            yy = self._up(y, dd)
            if xx == yy:
                ok = mid
            else:
                ng = mid
        # okは条件を満たすdepth
        return self._up(x, dx - ok)

    def _up(self, x: int, n: int):
        # ノードxからn個上のノードを返す
        for j in range(n.bit_length()):
            if (n >> j) & 1:
                x = self.up[j][x]
        return x

    def _doubling(self):
        max_depth = max(self.dists)
        K = max_depth.bit_length()
        up = [[0] * (self.N + 1)
              for _ in range(K)]  # up[k][u] はuの2^k個親のノードを指す
        up[0] = self.parents + [-1]  # 自己参照できるようにさいごに-1をつけておく
        for k in range(K - 1):
            for u in range(self.N + 1):
                up[k + 1][u] = up[k][up[k][u]]
        self.up = up


# # test的な
# tree = Tree(6)
# tree.link(0, 1)
# tree.link(0, 5)
# tree.link(1, 2)
# tree.link(1, 4)
# tree.link(2, 3)
# tree.set_root(0)  # ok
# print(tree.children)
# print(tree.parents)
# tour, inn, out = tree.node_euler_tour() #ok
# print(tour)
# print(inn)
# print(out)
# tree._doubling()  # ok
# print(*tree.up, sep='\n')
# print(tree.lca(2, 4))  # 1 #ok
# print(tree.lca(3, 5))  # 0
# print(tree.lca(4, 3))  # 1
# print(tree.lca(3, 2))  # 2
# print(tree.lca(2, 2))  # 2


# TODO
class Tree:  # 重み付きにしたい
    def __init__(self, N: int):
        """N頂点の木クラス

        Args:
            N (int): ノード数
        """
        from collections import defaultdict
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
        from collections import deque
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
