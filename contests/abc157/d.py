import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = [-1] * N  # idxが各ノードに対応。

    def root(self, A):
        # print(A)
        # ノード番号を受け取って一番上の親ノードの番号を帰す
        if (self.parent[A] < 0):
            return A
        self.parent[A] = self.root(self.parent[A])  # 経由したノードすべての親を上書き
        return self.parent[A]

    def size(self, A):
        # ノード番号を受け取って、そのノードが含まれている集合のサイズを返す。
        return -self.parent[self.root(A)]

    def unite(self, A, B):
        # ノード番号を2つ受け取って、そのノード同士をつなげる処理を行う。
        # 引数のノードを直接つなぐ代わりに、親同士を連結する処理にする。
        A = self.root(A)
        B = self.root(B)

        # すでにくっついている場合
        if (A == B):
            return False

        # 大きい方に小さい方をくっつけたほうが処理が軽いので大小比較
        if (self.size(A) < self.size(B)):
            A, B = B, A

        # くっつける
        self.parent[A] += self.parent[B]  # sizeの更新
        self.parent[B] = A  # self.rootが呼び出されればBにくっついてるノードもすべて親がAだと上書きされる

        return True

    def is_in_same(self, A, B):
        return self.root(A) == self.root(B)


# aとbの友人候補
# aとbはブロックされてないけど友達でもない
# かつ直接友人じゃないけど友人同士でつながっているのは友人候補である。


# まず友達関係のdisjoint setを求めておく
# あるdisjoint setにおいて、全員が友達候補であると仮定しておく。すでに友達である人数を引く。さらにブロック関係の人数も引く。()

from collections import defaultdict
N, M, K = read_ints()
uf = UnionFind(N)
direct_friends = defaultdict(lambda: 0)
for m in range(M):
    a, b = read_ints()
    direct_friends[a - 1] += 1
    direct_friends[b - 1] += 1
    uf.unite(a - 1, b - 1)

# まって、ブロックだけど、同じunionにあるブロック数だけカウントしたい
block = defaultdict(lambda: 0)
for m in range(K):
    a, b = read_ints()
    if uf.is_in_same(a - 1, b - 1):
        block[a - 1] += 1
        block[b - 1] += 1


ans = []
for i in range(N):
    katei = uf.size(i) - 1
    katei -= direct_friends[i]
    katei -= block[i]
    ans.append(katei)
print(*ans)
