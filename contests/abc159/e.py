import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


class cumsum2d:  # 二次元累積和クラス pypyでも使えるよ
    def __init__(self, ls: list):
        '''
        2次元のリストを受け取る
        '''
        from itertools import product
        H = len(ls)
        W = len(ls[0])
        self.ls_accum = [[0] * (W + 1)]
        for l in ls:
            self.ls_accum.append([0] + l)

        # 縦に累積
        for i, j in product(range(1, H + 1), range(1, W + 1)):
            self.ls_accum[i][j] += self.ls_accum[i - 1][j]
        # 横に累積
        for i, j in product(range(1, H + 1), range(1, W + 1)):
            self.ls_accum[i][j] += self.ls_accum[i][j - 1]

    def total(self, i1, j1, i2, j2):
        '''
        点(i1,j1),(i1,j2),(i2,j1),(i2,j2)の4点が成す四角の中の合計を取り出す
        ただし i1<=i2, j1<=j2
        ただし、i軸に関しては[i1,i2),j軸に関しては[j1,j2)の半開区間である
        '''
        return self.ls_accum[i2][j2] - self.ls_accum[i1][j2] \
            - self.ls_accum[i2][j1] + self.ls_accum[i1][j1]


class cumsum1d:  # 一次元累積和クラス
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


MOD = 10**9 + 7
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


# やっぱり二次元累積和でホワイトチョコレートの個数を管理して良さそう。
# Kが10以上なら簡単で縦に着れば良い
# 縦は全探索
# 横は累積和で管理

H, W, K = read_ints()
S = read_map(H)
S_cums = []
for s in S:
    S_cums.append(cumsum1d(map(int, list(s))))
# いや累積和を10こ作って於けばいいんじゃね


def ret_renketusum(p, i, j):
    # pの分割で[i,j)まで最大いくつの1があるか
    ma = S_cums[0].total(i, j)
    tmp = S_cums[0].total(i, j)
    for k, pp in enumerate(p, start=1):
        if pp == 1:  # cutする
            ma = max(ma, tmp)
            tmp = S_cums[k].total(i, j)
        else:
            tmp += S_cums[k].total(i, j)
    ma = max(ma, tmp)
    return ma


ans = 1000000000
for p in iter_p_adic(2, H - 1):
    # どこを分割するのか全探索
    l = 0
    r = 1
    n_cut = 0
    while r <= W:
        # print(l, r, ret_renketusum(p, l, r))
        if ret_renketusum(p, l, r) > K:
            if r - l == 1:
                n_cut = 1000000000000
                break
            n_cut += 1
            l = r - 1
            r = l
        r += 1
    ans = min(n_cut + sum(p), ans)

print(ans)
