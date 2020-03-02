# 下書き
# https://juppy.hatenablog.com/entry/2019/05/02/%E8%9F%BB%E6%9C%AC_python_%E3%82%BB%E3%82%B0%E3%83%A1%E3%83%B3%E3%83%88%E6%9C%A8_%E7%AB%B6%E6%8A%80%E3%83%97%E3%83%AD%E3%82%B0%E3%83%A9%E3%83%9F%E3%83%B3%E3%82%B0_Atcoder

# ここではMRQ(minimum range query)を想定する


class SegmentTree:
    def __init__(self, ls: list, segfunc, outer):
        '''
        セグ木 (下書き)
        一次元のリストlsを受け取り初期化する。
        区間のルールはsegfuncによって定義される
        outerは区間外に設定する値。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        '''
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [outer] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):
            # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def update(self, i, x):
        return NotImplementedError()

    def query(self, l, r):
        return NotImplementedError()


def segfunc():
    # 処理したい内容
    return
