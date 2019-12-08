# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_D
# P422にはDPによる解法が乗っているが、実際解くにはP423に乗っているようなある種貪欲な方法のほうが計算量が少なく済む。
from bisect import bisect_left, bisect_right, insort_left
from collections import deque


class BinarySearchTree:
    def __init__(self, ls: list = []):
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

    def insert_replace_left(self, x):
        '''
        xを挿入して、xの左の数字(次に小さい)を削除する。idxがはみ出す場合は挿入だけ
        '''
        idx_del = self.bisect_left(x) - 1
        if idx_del + 1 == 0:  # xがどの要素よりも小さい
            self.insert(x)
        if idx_del < len(self.bst):
            self.insert(x)
            del self.bst[idx_del]

    def insert_replace_right(self, x):
        '''
        xを挿入して、xの右の数字(次に大きい)を削除する。idxがはみ出す場合は挿入だけ
        '''
        idx_del = self.bisect_left(x) + 1
        if idx_del - 1 == len(self.bst):  # xがどの要素よりも大きい
            self.insert(x)
        else:
            self.insert(x)
            del self.bst[idx_del]


# load data
N = int(input())
A = []
for _ in range(N):
    A.append(int(input()))


L = BinarySearchTree()
L.insert(A[0])

for a in A[1:]:
    L.insert_replace_right(a)

print(len(L))
