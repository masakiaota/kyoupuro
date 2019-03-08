import sys
read = sys.stdin.readline


def readln():
    return list(map(int, read().split()))


N, M = readln()
A = []
B = []
for _ in range(M):
    a, b = readln()
    A.append(a-1)
    B.append(b-1)
# AB = [readln() for _ in range(M)]
# A = [ab[0]-1 for ab in AB[::-1]]
# B = [ab[1]-1 for ab in AB[::-1]]
A.reverse()
B.reverse()


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

    def concat(self, A, B):
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


ans = [N * (N - 1) // 2]
uni = UnionFind(N)
for a, b in zip(A, B):
    if uni.root(a) == uni.root(b):
        ans.append(ans[-1])
    else:
        ans.append(ans[-1] - uni.size(a) * uni.size(b))
        uni.concat(a, b)

for a in ans[-2::-1]:
    print(a)
