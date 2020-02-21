# https://atcoder.jp/contests/arc037/tasks/arc037_c
# 当時は黄色パフォっぽかったけど今は青ぐらいな気がする

# 二分探索をうまく使う。

# K番目の値をXと仮定する。
# X以下の数はK個以上は存在する。(この条件を満たす最小のXが答え)
# Xが指定されたときにX以下がK個以上か判別する関数を二分探索をすれば良い
# どうやってXが指定されたときにX以下がK個以上か判別する関数をつくるか？
# a_i * b_j <= X を満たしたいのだから、b_j <= X/a_i を満たすb_jの個数をa_iだけ集計すれば良い これも二分探索で実装可能

from bisect import bisect_right
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
A = read_ints()
B = read_ints()
A.sort()
B.sort()


def is_ok(X):
    # a_i * b_j <= Xを満たす個数がKよりも多いか？
    cnt = 0
    for a in A:
        # aa = X / a #注意ここは小数点の関係で以下のbisect_rightが取れなくなったりするので整数にする必要がある
        aa = X // a
        cnt += bisect_right(B, aa)
    return cnt >= K


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(-1, 10 ** 18 + 1))
