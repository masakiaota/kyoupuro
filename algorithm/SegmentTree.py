# Segment Tree
# https://juppy.hatenablog.com/entry/2019/05/02/%E8%9F%BB%E6%9C%AC_python_%E3%82%BB%E3%82%B0%E3%83%A1%E3%83%B3%E3%83%88%E6%9C%A8_%E7%AB%B6%E6%8A%80%E3%83%97%E3%83%AD%E3%82%B0%E3%83%A9%E3%83%9F%E3%83%B3%E3%82%B0_Atcoder


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
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
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res = self.ide
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res = self.func(res, self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res = self.func(res, self.tree[r - 1])
            l >>= 1
            r >>= 1
        return res


def segfunc(x, y):
    # 処理したい内容
    return min(x, y)  # 例えばね


# test
test = [1, 3, 1, 3, 7, 1, 2, 5, 3, 7, 9, 1, 3, 6, 2]
st = SegmentTree(test, min, 10 ** 10)
print(test)
print(st.query(0, len(test)))
print(st.query(0, 7))
print(st.query(7, 10))

st.update(2, 100)
st.update(10, 1)
print(st.query(0, len(test)))
print(st.query(2, 3))
print(st.query(7, 11))
# 大丈夫そう


# 遅延評価セグメント木
# http://tsutaj.hatenablog.com/entry/2017/03/30/224339 アイデアはわかりやすいけど再帰関数なので避けた
# 区間和クエリを扱うのを例に (あとであとで実装)

# 参考
# https://smijake3.hatenablog.com/entry/2018/11/03/100133
# こっちのほうがわかりやすいかもね

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


class LazySegmentTree:  # 一般に自分でいじるセグ木
    def __init__(self, ls: list, segfunc, identity_element, lazy_ide=None):
        '''
        セグ木 pypyじゃないとTLEなるかも
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
            i -= 1  # to 0basedindex
            v = self.lazy[i]
            if v == self.lide:
                continue
            #########################################################
            # この4つの配列をどう更新するかは問題によって異なる
            self.tree[2 * i + 1]
            self.tree[2 * i + 2]
            self.lazy[2 * i + 1]
            self.lazy[2 * i + 2]
            #########################################################

            self.lazy[i] = self.lide  # 遅延配列を空に戻す

    def update(self, l, r, x):
        '''
        [l,r)番目の要素をxを適応するクエリを行う O(logN)
        '''
        # 1, 根から区間内においてlazyの値を伝播しておく(self.treeの値が有効になる)
        ids = tuple(_gidx(l, r, self.num))
        #########################################################
        # 区間加算queryのような操作の順序が入れ替え可能な場合これをする必要なないが多くの場合でしたほうがバグが少なく(若干遅くなる)
        self._lazyprop(*ids)
        #########################################################
        # 2, 区間に対してtree,lazyの値を更新 (treeは根方向に更新するため、lazyはpropで葉方向に更新するため)
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        while l < r:
            #########################################################
            # ** 問題によって値のセットの仕方も変えるべし**
            if r & 1:
                r -= 1
                self.tree[r - 1]
                self.lazy[r - 1]
            if l & 1:
                self.tree[l - 1]
                self.lazy[l - 1]
                l += 1
            #########################################################
            l >>= 1
            r >>= 1
        # 3, 伝播させた区間について下からdataの値を伝播する
        for i in ids:
            i -= 1  # to 0based
            #########################################################
            # 関数の先頭でlazy propを省略した場合は、現在のノードにlazyが反映されていないことがある
            # lazyを省略するならここを慎重に書き換えなければならない
            self.tree[i] = self.func(
                self.tree[2 * i + 1], self.tree[2 * i + 2])  # +self.lazy[i] 的なね(区間加算クエリだったらだったら)
            #########################################################

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
            r >>= 1
        return res

######################################以下具体例##########################################


class SegmentTreeForRMQandRUQ:  # range minimum query and range update query
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
            # **問題によって価のセットの仕方も変わる**
            if r & 1:
                r -= 1
                self.tree[r - 1] = x
                self.lazy[r - 1] = x
            if l & 1:
                self.tree[l - 1] = x
                self.lazy[l - 1] = x
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


class SegmentTreeForRMQandRAQ:  # range minimum query and range add query
    def __init__(self, ls: list, segfunc=min, identity_element=2**63, lazy_ide=0):
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
            # 今回なら範囲(最小値を持つ)に加算なので、そのままvを加算すればよい
            self.tree[2 * i + 1] += v
            self.tree[2 * i + 2] += v
            self.lazy[2 * i + 1] += v
            self.lazy[2 * i + 2] += v
            self.lazy[i] = self.lide  # 遅延配列を空に戻す

    def update(self, l, r, x):
        '''
        [l,r)番目の要素をxを加算する(木の中間ノードも更新する) O(logN)
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
            # ** 問題によって値のセットの仕方も変えるべし**
            if r & 1:
                r -= 1
                self.tree[r - 1] += x
                self.lazy[r - 1] += x
            if l & 1:
                self.tree[l - 1] += x
                self.lazy[l - 1] += x  # lazyの区間に値をセット
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


import operator


class SegmentTreeForRSQandRAQ:  # 区間合計(ホントは何でも良い)クエリ と 区間加算クエリを扱うことにする
    def __init__(self, ls: list, segfunc=operator.add, identity_element=0, lazy_ide=0):
        '''
        セグ木 もしかしたらバグがあるかも
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
            i -= 1  # to 0basedindex
            v = self.lazy[i]
            if v == self.lide:
                continue
            #########################################################
            # この4つの配列をどう更新するかは問題によって異なる
            self.tree[2 * i + 1] += v >> 1  # 区間加算クエリなので
            self.tree[2 * i + 2] += v >> 1
            self.lazy[2 * i + 1] += v >> 1
            self.lazy[2 * i + 2] += v >> 1
            #########################################################

            self.lazy[i] = self.lide  # 遅延配列を空に戻す

    def update(self, l, r, x):
        '''
        [l,r)番目の要素をxを適応するクエリを行う O(logN)
        '''
        # 1, 根から区間内においてlazyの値を伝播しておく(self.treeの値が有効になる)
        ids = tuple(_gidx(l, r, self.num))
        #########################################################
        # 区間加算queryのような操作の順序が入れ替え可能な場合これをする必要なないが多くの場合でしたほうがバグが少なく(若干遅くなる)
        # self._lazyprop(*ids)
        #########################################################
        # 2, 区間に対してtree,lazyの値を更新 (treeは根方向に更新するため、lazyはpropで葉方向に更新するため)
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        while l < r:
            #########################################################
            # ** 問題によって値のセットの仕方も変えるべし**
            if r & 1:
                r -= 1
                self.tree[r - 1] += x
                self.lazy[r - 1] += x
            if l & 1:
                self.tree[l - 1] += x
                self.lazy[l - 1] += x
                l += 1
            #########################################################
            x <<= 1  # 区間加算クエリでは上段では倍倍になるはずだよね
            l >>= 1
            r >>= 1
        # 3, 伝播させた区間について下からdataの値を伝播する
        for i in ids:
            i -= 1  # to 0based
            #########################################################
            # 関数の先頭でlazy propを省略した場合は、現在のノードにlazyが反映されていないことがある
            # lazyを省略するならここを慎重に書き換えなければならない
            self.tree[i] = self.func(
                self.tree[2 * i + 1], self.tree[2 * i + 2]) + self.lazy[i]
            #########################################################

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
            r >>= 1
        return res
