# https://atcoder.jp/contests/abc134/tasks/abc134_e
import sys
read = sys.stdin.readline


def read_a_int():
    return int(read())


from bisect import bisect_left, bisect_right, insort_left
from array import array


class BinarySearchTree:
    def __init__(self, ls: list = []):
        '''
        C++でいうsetを実装する。二分探索木をガチで実装しようとすると大変なので、ここでは配列二分法を用いる。
        pythonの標準ライブラリがヨイショに抱っこしてくれるおかげで楽に実装できる。
        https://docs.python.org/ja/3/library/bisect.html


        ls ... 渡す初期配列
        '''
        self.bst = array('q', sorted(
            ls))  # insertを爆速にするためにarray型にします。signed long long 前提です

    # def __repr__(self):
    #     return f'BST:{self.bst}'

    def __len__(self):
        return len(self.bst)

    def __getitem__(self, idx):
        return self.bst[idx]

    def size(self):
        return len(self.bst)

    def insert(self, x):
        # idx = self.bisect_left(x)
        # self.bst.insert(idx,x)
        insort_left(self.bst, x)

    def pop_left(self):
        x = self.bst[0]
        del self.bst[0]
        return x

    def pop_right(self):
        x = self.bst[-1]
        del self.bst[-1]
        return x

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
        idx_del = self.bisect_right(x) + 1  # xと同じ大きさも削除したいならbisect_left
        if idx_del - 1 == len(self.bst):  # xがどの要素よりも大きい
            self.insert(x)
        else:
            self.insert(x)
            del self.bst[idx_del]


N = read_a_int()
bst = BinarySearchTree([-1])
for n in range(N):
    a = read_a_int()
    # すべてより小さかったら、新しく追加
    if bst.bst[0] >= a:
        bst.insert(a)
    else:
        # bstのなかで、aより小さく且つ一番大きい要素をaに書き換えていく
        idx = bst.bisect_left(a) - 1
        bst.bst[idx] = a

print(len(bst))
