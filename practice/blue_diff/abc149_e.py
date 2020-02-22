# https://atcoder.jp/contests/abc149/tasks/abc149_e

# すべての握手の組み合わせN**2を列挙しソートしM番目までを足し合わせればOK
# だけど制約からこれを行うことは困難
# すべてを列挙しなくともM番目の値を知ることは二分探索で可能(参考:億マス計算)
# Aの累積和を保持しておけば、M番目の値の探索中にMまでの値の合計もついでに計算できる
# 以下reverseでソート済みだと仮定
# XがM番目の数→X以上である数はM個以上(cntとする)→cntがM個以上の条件を満たすうちの最大となるXがM番目の値
# そのあと余分な分を引く処理とか必要

from bisect import bisect_right, bisect_left
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


class cumsum1d:
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


N, M = read_ints()
A = read_ints()
A.sort()  # bisectを使う都合上 reverseは抜き
A_reversed = list(reversed(A))
A_rev_acc = cumsum1d(A_reversed)


def is_ok(X):
    # M番目の数はXである→X以上の個数>=M となるうちで最大のX(もっとも左の方のX)
    # X以上の個数>=Mを返す
    # X以下の個数はai+aj>=Xを満たす個数
    cnt = 0
    ans = 0
    for a in A:
        aa = X - a
        idx_reverse = N - bisect_left(A, aa)  # 大きい方からだと何番目か
        # これはbisect_right(A_reversed,aa)に等しい
        cnt += idx_reverse
        ans += A_rev_acc.total(0, idx_reverse) + idx_reverse * a
    return cnt >= M, ans, cnt


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        flg, ans, cnt = is_ok(mid)
        if flg:
            ok = mid
            ans_true = ans  # さいごにokとなる状態がans
            cnt_true = cnt
        else:
            ng = mid
    return ans_true, ok, cnt_true


ans_tmp, M_th_num, M_plus_alpha_th = \
    meguru_bisect(2 * 10 ** 5 + 1, 0)
# print(ans_tmp, M_th_num, M_plus_alpha_th)
print(ans_tmp - (M_plus_alpha_th - M) * M_th_num)
