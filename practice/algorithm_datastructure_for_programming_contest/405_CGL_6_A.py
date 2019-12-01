# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_6_A


from bisect import bisect_left, bisect_right, insort_left
from collections import deque


class BinarySearchTree:
    def __init__(self, ls: list=[]):
        '''
        C++でいうsetを実装する。二分探索木をガチで実装しようとすると大変なので、ここでは配列二分法を用いる。
        pythonの標準ライブラリがヨイショに抱っこしてくれるおかげで楽に実装できる。
        https://docs.python.org/ja/3/library/bisect.html


        ls ... 渡す初期配列
        '''
        self.bst = deque(sorted(ls))  # insertをO(1)にするためにlistの代わりにdequeを用います

    def __repr__(self):
        return f'BST:{self.bst}'

    def __len__(self):
        return len(self.bst)

    def __getitem__(self, idx):
        return self.bst[idx]

    def size(self):
        return len(self.bst)

    def insert(self, x):
        insort_left(self.bst, x)

    def remove(self, x):
        '''
        xを取り除く。xがself.bstに存在することを保証してください。
        同一のものが存在した場合は左から消していく
        '''
        del self.bst[self.find(x)]

    def bisect_left(self, x):
        '''
        ソートされた順序を保ったまま x を self.bst に挿入できる点を探し当てます。
        lower_bound in C++
        '''
        return bisect_left(self.bst, x)

    def bisect_right(self, x):
        '''
        bisect_left() と似ていますが、 self.bst に含まれる x のうち、どのエントリーよりも後ろ(右)にくるような挿入点を返します。
        upper_bound in C++
        '''
        return bisect_right(self.bst, x)

    def find(self, x):
        '''
        xのidxを探索
        '''
        idx = bisect_left(self.bst, x)
        if idx != len(self.bst) and self.bst[idx] == x:
            return idx
        raise ValueError



    # load data
N = int(input())
lines = []
for _ in range(N):
    x1, y1, x2, y2 = list(map(int, input().split()))
    # 前処理として、x1,y1を必ず下端点or左端点にする
    if y1 == y2:  # 平行線の場合
        if x1 > x2:
            x1, x2 = x2, x1
    else:  # 垂直線の場合
        if y1 > y2:
            y1, y2 = y2, y1
    lines.append((x1, y1, x2, y2))

# P409の下の方に書いてあるが、交差を判定する前に削除してしまったり、追加するまえに判定してしまったりすることを防ぐために
# うまい感じにソートするためにendpointsを導入する
# 各要素は(y,端点の種類,x,左端点のx座標)で定義される)
BOTTOM = 0
LEFT = 1
RIGHT = 2
TOP = 3
# 線分の端点を必ず左下始まりにする
endpoints = []
for x1, y1, x2, y2 in lines:
    if y1 == y2:  # 平行線の場合
        endpoints.append((y1, LEFT, x1, x2))  # 左端点の追加
        endpoints.append((y2, RIGHT, x2, -1))  # 右端点の追加
    else:  # 垂直線の場合
        endpoints.append((y1, BOTTOM, x1, -1))  # 下端点の追加
        endpoints.append((y2, TOP, x2, -1))  # 下端点の追加

# yを下から走査するためにソート
endpoints.sort()

bst = BinarySearchTree()
ans = 0
for y, p_type, x, x_t in endpoints:
    if p_type == RIGHT:
        continue  # 後述しますが、右端点は左端点とセットで処理するためスキップしても問題ないです

    # 以下端点の種類ごとに操作を実装
    if p_type == TOP:
        bst.remove(x)  # 上端点の場合はbstに登録してあるxを削除
    elif p_type == BOTTOM:
        bst.insert(x)  # 下端点の場合はbstにxを登録
    elif p_type == LEFT:
        s = bst.bisect_left(x)  # bstにおいて、水平線の左は何番目に大きいか
        t = bst.bisect_right(x_t)  # bstにおいて、水平線の右は何番目に大きいか(同じ値も含めて)
        ans += t - s

print(ans)
