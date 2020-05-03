import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


from bisect import bisect_left, bisect_right, insort_left


class BinarySearchTree:
    def __init__(self, ls: list = []):
        '''
        C++でいうsetを実装する。二分探索木をガチで実装しようとすると大変なので、ここでは配列二分法を用いる。
        pythonの標準ライブラリがヨイショに抱っこしてくれるおかげで楽に実装できる。
        https://docs.python.org/ja/3/library/bisect.html
        ls ... 渡す初期配列
        '''
        self.bst = ls  # insertをO(1)にするためにlistの代わりにdequeを用います

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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd

# 各ノードに今までの最長増加部分列と現在の増加部分列長の記録があれば、たかだた分岐地点から探索できるのでは？
N = read_a_int()
A = read_ints()
Tree = [[] for _ in range(N)]
for _ in range(N - 1):
    u, v = read_ints()
    u -= 1
    v -= 1
    Tree[u].append(v)
    Tree[v].append(u)

LIS = BinarySearchTree([-1])

ans = [-1] * N


def dfs(u, p, LIS):  # これって参照渡しに成るんすかねすると厄介かもしれない
    '''現在のノード、親、最長増加部分列の状態'''
    LIS.insert_replace_right(A[u])
    ans[u] = len(LIS) - 1
    # print(u, LIS.bst)
    if len(Tree[u]) > 2 or u == 0:  # 配列のコピーを渡す
        for nu in Tree[u]:
            if nu == p:
                continue
            dfs(nu, u, BinarySearchTree(LIS.bst.copy()))  # 多分ここでTLE(1ケースだけ)
    elif len(Tree[u]) == 2:  # 参照渡しでok
        for nu in Tree[u]:
            if nu == p:
                continue
            dfs(nu, u, LIS)


dfs(0, -1, LIS)
print(*ans, sep='\n')
