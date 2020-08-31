import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9


class SegmentTree:
    def __init__(self, n: int, segfunc=max, identity_element=0):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = segfunc
        self.n_origin = n
        self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        # for i, l in enumerate(ls):  # 木の葉に代入
        #     self.tree[i + self.num - 1] = l
        # for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
        #     self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def __getitem__(self, idx):  # オリジナル要素にアクセスするためのもの
        if isinstance(idx, slice):
            start = idx.start if idx.start else 0
            stop = idx.stop if idx.stop else self.n_origin
            l = start + self.num - 1
            r = l + stop - start
            return self.tree[l:r:idx.step]
        elif isinstance(idx, int):
            i = idx + self.num - 1
            return self.tree[i]

    def update(self, i, x):
        '''
        i番目の要素をxに変更する(木の中間ノードも更新する) O(logN)
        '''
        i += self.num - 1
        self.tree[i] = x
        while i:  # 木を更新
            i = (i - 1) // 2
            self.tree[i] = self.func(self.tree[i * 2 + 1],
                                     self.tree[i * 2 + 2])

    def query(self, l, r):
        '''区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど'''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res_right = []
        res_left = []
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res_left.append(self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res_right.append(self.tree[r - 1])
            l >>= 1
            r >>= 1
        res = self.ide
        # 左右の順序を保って結合
        for x in res_left:
            res = self.func(x, res)
        for x in reversed(res_right):
            res = self.func(res, x)
        return res


'''
dp[i,h] ... 花の美しさの最大値、:iまで考慮したとき、最後の高さhであるようなとり方の場合の

if H[i]<=h: #付け足すことはできない
    dp[i+1, h] = dp[i,h]
else: #付け足すことができるとき h<H[i]のすべてのhについて
    dp[i+1, H[i]] = max(dp[i+1,h])+A[i] #最後の高さがH[i]になるので

だけど、O(N^2)を処理することはできない！

i->i+1は配列更新にしておいて,
dp[i,:]をsegment treeで持てばok!


もしH[i]足せって問題でも区間更新遅延セグ木で持てば大丈夫そうって示唆がされるね
'''

N = a_int()
H = ints()
A = ints()

dpi = SegmentTree(2 * 10**5 + 1)
for i in range(N):
    ma = dpi.query(0, H[i])
    dpi.update(H[i], ma + A[i])
print(max(dpi[0:2 * 10**5 + 1]))
