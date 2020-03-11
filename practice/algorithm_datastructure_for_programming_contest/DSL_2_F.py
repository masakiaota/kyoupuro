# https://onlinejudge.u-aizu.ac.jp/courses/library/3/DSL/all/DSL_2_F
# 遅延セグ木


def _gidx(l, r, treesize):
    '''
    lazy propagation用idx生成器 木の下から生成される。1based-indexなので注意.(使うときは-1するとか)
    もとの配列において[l,r)を指定したときに更新すべきidxをyieldする
    treesizeは多くの場合self.num
    '''
    L, R = l + treesize, r + treesize
    lm = (L // (L & -L)) >> 1  # これで成り立つの天才か？
    rm = (R // (R & -R)) >> 1
    while L < R:
        if R <= rm:
            yield R
        if L <= lm:
            yield L
        L >>= 1
        R >>= 1
    while L:  # Rでもいいけどね
        yield L
        L >>= 1


class SegmentTreeForRMQ:  # range minimum query
    def __init__(self, ls: list, segfunc=min, identity_element=2**63, lazy_ide=None):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.lide = lazy_ide  # lazy用単位元
        self.func = segfunc
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num)  # 関係ない値を-1においてアクセスを許すと都合が良い
        self.lazy = [self.lide] * (2 * self.num)  # 遅延配列
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def _lazyprop(self, *ids):
        '''
        遅延評価用の関数
        - self.tree[i] に self.lazy[i]の値を伝播させて遅延更新する
        - 子ノードにself.lazyの値を伝播させる **ここは問題ごとに書き換える必要がある**
        - self.lazy[i]をリセットする
        '''
        for i in reversed(ids):
            i -= 1  # 0basedindexに修正
            v = self.lazy[i]
            if v == self.lide:
                continue  # 単位元ならする必要のNASA
            # どうやって遅延更新するかは問題によってことなる
            # 今回なら数字書き換えなので数字をそのまま子ノードに伝播
            # lazyもtreeも書き換える必要あり
            self.tree[2 * i + 1] = v
            self.tree[2 * i + 2] = v
            self.lazy[2 * i + 1] = v
            self.lazy[2 * i + 2] = v
            self.lazy[i] = self.lide  # 遅延配列を空に戻す

    def update(self, l, r, x):
        '''
        [l,r)番目の要素をxに変更する(木の中間ノードも更新する) O(logN)
        '''
        # 1, 根から区間内においてlazyの値を伝播しておく(self.treeの値が有効になる)
        ids = tuple(_gidx(l, r, self.num))
        self._lazyprop(*ids)
        # 2, 区間に対してtree,lazyの値を更新 (treeは根方向に更新するため、lazyはpropで葉方向に更新するため)
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        while l < r:
            if r & 1:
                r -= 1  # 一旦半開ではなくするために
                self.tree[r - 1] = x
                self.lazy[r - 1] = x
            if l & 1:
                self.tree[l - 1] = x
                self.tree[l - 1] = x  # ここのセットの仕方も問題によって変えるべし
                self.lazy[l - 1] = x  # lazyの区間に値をセット
                l += 1
            l >>= 1
            r >>= 1
        # 3, 伝播させた区間について下からdataの値を伝播する
        for i in ids:
            i -= 1  # to 0based
            self.tree[i] = self.func(
                self.tree[2 * i + 1], self.tree[2 * i + 2])

    def query(self, l, r):
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        # 1, 根からにlazyの値を伝播させる
        self._lazyprop(*_gidx(l, r, self.num))
        # 2, 区間[l,r)の最小値を求める
        l += self.num
        r += self.num
        res = self.ide
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if r & 1:
                r -= 1
                res = self.func(res, self.tree[r - 1])
            if l & 1:
                res = self.func(res, self.tree[l - 1])
                l += 1
            l >>= 1
            r >>= 1  # 親の一つ左に移動
        return res


n, q = map(int, input().split())
ls = [2 ** 31 - 1] * n
seg = SegmentTreeForRMQ(ls, identity_element=2**31 - 1)

for _ in range(q):
    cmd, *tmp = map(int, input().split())
    if cmd == 0:
        s, t, x = tmp
        seg.update(s, t + 1, x)
    else:
        s, t = tmp
        print(seg.query(s, t + 1))
