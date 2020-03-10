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
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

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
        l += self.num - 1
        r += self.num - 2  # ここで半開区間でなく[l,r]で以下を処理する
        res = self.ide
        while r - l > 1:  # 右から寄りながら結果を結合していくイメージ
            if l & 1 == 0:
                res = self.func(res, self.tree[l])
            if r & 1 == 1:
                res = self.func(res, self.tree[r])
                r -= 1
            l = l // 2  # 親の一つ右に移動
            r = (r - 1) // 2  # 親の一つ左に移動
        if l == r:
            res = self.func(res, self.tree[l])
        else:
            res = self.func(res, self.tree[l])
            res = self.func(res, self.tree[r])
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


# http://tsutaj.hatenablog.com/entry/2017/03/30/224339
# 遅延評価セグメント木
# 区間和クエリを扱うのを例に
import operator


class SegmentTreeForSum:
    def __init__(self, ls: list, identity_element):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = operator.add
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)
        self.lazy = [self.ide] * (2 * self.num - 1)  # 遅延配列
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def _lazyeval(self, i, l, r):
        '''
        遅延評価用の関数
        - self.tree[i] に self.lazy[i]の値を伝播させて遅延更新する
        - 子ノードにself.lazyの値を伝播させる **ここは問題ごとに書き換える必要がある**
        - self.tree[i]をリセットする
        '''
        lazyi = self.lazy[i]
        if lazyi == self.ide:
            return  # 値を伝播する必要がない場合は即時終了
        self.tree[i] = self.func(self.tree[i], lazyi)
        if r - l > 1:  # r-lが1より離れていれば最下段ではない(半開区間)
            # 子へ遅延値を伝播させていく
            # ここのルールは問題によって異なる クエリが区間和の場合はこう
            self.lazy[2 * i + 1] += lazyi // 2
            self.lazy[2 * i + 2] += lazyi // 2
        self.lazy[i] = self.ide  # 遅延配列を空に戻す

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

    def update_kukan(self, l, r, x):
        '''
        区間[l,r)に対してxをself.funcする(今回なら足し込む)
        '''
        eval()

    def query(self, l, r):
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num - 1
        r += self.num - 2  # ここで半開区間でなく[l,r]で以下を処理する
        res = self.ide
        while r - l > 1:  # 右から寄りながら結果を結合していくイメージ
            if l & 1 == 0:
                res = self.func(res, self.tree[l])
            if r & 1 == 1:
                res = self.func(res, self.tree[r])
                r -= 1
            l = l // 2  # 親の一つ右に移動
            r = (r - 1) // 2  # 親の一つ左に移動
        if l == r:
            res = self.func(res, self.tree[l])
        else:
            res = self.func(res, self.tree[l])
            res = self.func(res, self.tree[r])
        return res


# 参考
# https://smijake3.hatenablog.com/entry/2018/11/03/100133
# こっちのほうがわかりやすいかもね
