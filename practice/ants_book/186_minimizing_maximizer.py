# https://www.slideshare.net/hcpc_hokudai/advanced-dp-2016
# これもスライド図示がめっちゃわかりやすい
# 要はDPが区間クエリの処理を含むのでそこをセグ木で高速化できるという話


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        self.ide = identity_element
        self.func = segfunc
        self.n_origin = len(ls)
        self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

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
        '''i番目の要素をxに変更する(木の中間ノードも更新する) O(logN)'''
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


# 入力
n = 40
m = 6
s = [20, 1, 10, 20, 15, 30]
t = [30, 10, 20, 30, 25, 40]

# 0basedindexに
# tは半開区間のためそのまま
s = [ss - 1 for ss in s]
INF = 10**6
dp = SegmentTree([0] + [INF] * (n - 1), min,
                 identity_element=INF)  # DP配列をセグ木に乗っける(初期化済み)

for ss, tt in zip(s, t):
    mi = dp.query(ss, tt)
    dp.update(tt - 1, mi + 1)

print(dp[n - 1])  # ok
