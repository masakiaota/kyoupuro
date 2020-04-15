# https://atcoder.jp/contests/abc063/tasks/arc075_b

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


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


N, A, B = read_ints()
H, = read_col(N)
H.sort(reverse=True)

# 操作回数をcとして決め打ち二分探索をする
# 満たす条件→すべての魔物を消し去る。その中で最小のcを探したい
# cが与えられたときにすべての魔物を消しされるかはどうやって求める？
# すべての魔物にc*Bダメージしたあとに、(A-B)ダメージをc回使うことで全滅箚せられればok


def is_ok(c):
    # 魔物を全滅させられればok
    bias = c * B
    addit = (A - B)
    c_cnt = c  # こいつは0になるまで使える
    for h in H:
        h -= bias
        if h <= 0:
            return True  # 全滅(早期終了)
        else:
            c_cnt -= (h - 1) // addit + 1
            if c_cnt < 0:  # 全滅させられない
                return False
    return True  # 全滅


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


print(meguru_bisect(-1, 10**9 + 1))
