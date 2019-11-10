# https://onlinejudge.u-aizu.ac.jp/courses/library/3/DSL/2/DSL_2_C
# むずい、普通にむずい


class OneDTree:
    def __init__(self, P):
        N = len(P)
        self.P = sorted(P)
        # 配列[ノード]でそのノードの内容を返す。
        self.location = [None] * N  # 整列した配列Pにおける位置
        self.left = [None] * N  # そのノードの左子
        self.right = [None] * N  # そのノードの右子
        self.np = 0  # ノード番号の初期化
        self.make1DTree(0, N)  # 1dtreeを作る

    def make1DTree(self, l, r):
        # 再帰関数なので終了条件
        if not (l < r):
            # 左<右の関係性が崩れたら終わり
            return None
        # P = sorted(self.P)  # ぶっちゃけ意味ないの省略

        mid = (l + r) // 2  # Pのidx 真ん中が二分探索木(もしくはその部分木)で根になる。

        t = self.np  # 二分木におけるノード番号を割り当てる
        self.np += 1  # ノード番号の更新

        self.location[t] = mid
        self.left[t] = self.make1DTree(l, mid)
        # 左部分木の中央値は親の左子となる。(右部分木のrootになるのは確定なので)
        self.right[t] = self.make1DTree(mid + 1, r)

        return t  # 現在のノードを返す

    def find(self, sx, tx):
        '''
        sx ... 範囲の最初
        tx ... 範囲の終わり 閉区間に注意
        '''
        ret = []  # 範囲に含まれる点を格納しておく

        def dfs(v, sx, tx):
            x = self.P[self.location[v]]
            if sx <= x <= tx:  # もし今のノードの指す値が範囲に入っていればok
                ret.append(x)

            # 続いて右と左の子が領域に含まれているかも探索する。
            if self.left[v] is not None and sx <= x:
                dfs(self.left[v], sx, tx)  # 左部分木について再帰的に探索
            if self.right[v] is not None and x <= tx:
                dfs(self.right[v], sx, tx)
        dfs(0, sx, tx)  # rootのノード番号が0なのは確定
        return ret


from random import shuffle
# test
P = [0, 2, 4, 5, 9, 12, 13, 15, 18, 20]
shuffle(P)
print(P)

kdtree = OneDTree(P)
print(kdtree.find(6, 15))
