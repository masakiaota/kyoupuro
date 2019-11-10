# https://onlinejudge.u-aizu.ac.jp/courses/library/3/DSL/2/DSL_2_C
# むずい、普通にむずい
# また定数倍が遅いせいかpythonだとTLEが取れない
# ACしている人のコードを見ると二分探索をもっとうまく使っているようだった(でもこれも二分探索木のはずでは？)

from operator import itemgetter
from sys import stdin


class TwoDTree:
    def __init__(self, P):
        '''
        Pの形式として[(pointID, x, y),...,()]となっていることを想定する
        '''
        N = len(P)
        self.P = P.copy()  # Pをコピーしておく
        # 配列[ノード]でそのノードの内容を返す。
        self.location = [None] * N  # 整列した配列Pにおける位置
        self.left = [None] * N  # そのノードの左子
        self.right = [None] * N  # そのノードの右子
        self.np = 0  # ノード番号の初期化
        self.make2DTree(0, N, 0)  # 1dtreeを作る

    def make2DTree(self, l, r, depth):
        # 再帰関数なので終了条件
        if not (l < r):
            # 左<右の関係性が崩れたら終わり
            return None

        mid = (l + r) // 2  # Pのidx 真ん中が二分探索木(もしくはその部分木)で根になる。
        t = self.np  # 二分木におけるノード番号を割り当てる
        self.np += 1  # ノード番号の更新

        # ここからx,y軸の分岐
        if depth % 2 == 0:
            # はじめの要素がx軸だと仮定して
            self.P[l:r] = sorted(self.P[l:r], key=itemgetter(1))
        else:
            self.P[l:r] = sorted(
                self.P[l:r], key=itemgetter(2))  # depthが奇数のときはy軸

        self.location[t] = mid
        self.left[t] = self.make2DTree(l, mid, depth + 1)
        self.right[t] = self.make2DTree(mid + 1, r, depth + 1)
        return t  # 現在のノードを返す

    def find(self, sx, tx, sy, ty):
        '''
        sx ... xの範囲の最初
        tx ... xの範囲の終わり 閉区間に注意
        sy ... yの範囲の最初
        ty ... yの範囲の終わり 閉区間に注意
        '''
        ret = []  # 範囲に含まれる点を格納しておく

        def dfs(v, sx, tx, sy, ty, depth):
            id, x, y = self.P[self.location[v]]

            if (sx <= x <= tx) and (sy <= y <= ty):  # もし今のノードの指す値が範囲に入っていればok
                ret.append((id, x, y))

            # 続いて右と左の子が領域に含まれているかも探索する。
            # ここでdepthが偶数奇数で場合分けをする
            if depth % 2 == 0:
                if self.left[v] is not None and sx <= x:
                    dfs(self.left[v], sx, tx, sy, ty, depth + 1)
                if self.right[v] is not None and x <= tx:
                    dfs(self.right[v], sx, tx, sy, ty, depth + 1)
            else:
                if self.left[v] is not None and sy <= y:
                    dfs(self.left[v], sx, tx, sy, ty, depth + 1)
                if self.right[v] is not None and y <= ty:
                    dfs(self.right[v], sx, tx, sy, ty, depth + 1)

        dfs(0, sx, tx, sy, ty, 0)
        return ret


# load data
N = int(input())
Points = []
lines = stdin.readlines()  # ctrlDまでの文字列を読み込む #入力高速化のための魔改造
for id in range(N):
    x, y = list(map(int, lines[id].split()))
    Points.append((id, x, y))


kdtree = TwoDTree(Points)

# Q = int(input())
Q = int(lines[N])
# print(Q)
for q in range(Q):
    sx, tx, sy, ty = list(map(int, lines[N + q + 1].split()))
    # ID_ls = list(map(itemgetter(0), kdtree.find(sx, tx, sy, ty)))
    ID_ls = [x[0] for x
             in kdtree.find(sx, tx, sy, ty)]  # itemgetterよりも内包表記のほうが早い
    if len(ID_ls) == 0:
        print()
    else:
        print(*sorted(ID_ls), sep='\n')
        print()
