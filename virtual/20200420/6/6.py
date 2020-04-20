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
from array import array


class BinarySearchTree:
    def __init__(self, ls: list = []):
        '''
        C++でいうsetを実装する。二分探索木をガチで実装しようとすると大変なので、ここでは配列二分法を用いる。
        pythonの標準ライブラリがヨイショに抱っこしてくれるおかげで楽に実装できる。
        https://docs.python.org/ja/3/library/bisect.html


        ls ... 渡す初期配列
        '''
        self.bst = array('l',
                         sorted(ls))  # insertを爆速にするためにarray型にします。signed long long 前提です

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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc127/tasks/abc127_f
# bとaを別で考えれば簡単、いままで出てきたAの中央値がxだしね

# とりあえずf(x)のxを出力するプログラムを作成してからどうやって計算量を削減するか考える

Q = read_a_int()
# 中央値

com, a, b = read_ints()
b_sum = b
A = BinarySearchTree([a])


def f(x, A, b_sum):
    ret = b_sum
    for a in A:
        ret += abs(a - x)
    return ret


for _ in range(Q - 1):
    com, *tmp = read_ints()
    if com == 1:
        a, b = tmp
        b_sum += b
        A.insert(a)
    else:
        x = A[(len(A) - 1) // 2]
        print(x, f(x, A, b_sum))
