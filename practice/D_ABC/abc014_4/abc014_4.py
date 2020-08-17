# https://atcoder.jp/contests/abc014/tasks/abc014_4
# 任意のノードをルートとしてよい。クエリa,bに対してdepth[a]-depth[lca(a,b)]とdepth[b]-depth[lca(a,b)]の合計+1が答え(閉路長)になる

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce


class Tree:
    def __init__(self, N: int):
        """N頂点の重みなし木クラス
        宣言後 link()->set_root()してからいろいろする

        Args:
            N (int): ノード数
        """
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


N = a_int()
tree = Tree(N)
for _ in ra(N - 1):
    x, y = mina(*ints())
    tree.link(x, y)
tree.set_root(0)
Q = a_int()
ans = []
for _ in ra(Q):
    a, b = mina(*ints())
    l = tree.lca(a, b)
    ans.append(tree.dists[a] + tree.dists[b] - 2 * tree.dists[l] + 1)
print(*ans, sep='\n')
