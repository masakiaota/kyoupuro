# https://atcoder.jp/contests/abc144/tasks/abc144_e


# 修行を行わないときに最適なのは両方を逆順でソートしておくこと

# 求めたいのは最大値！
# 最小はxにできると仮定してみる？
# 答えはxより小さくなる→greedyに(Ai-a)Fi<xして判定すれば良い

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
A = read_ints()
F = read_ints()
A.sort()  # 前からFの大きい順に対処していく人になっている
F.sort(reverse=True)


def is_ok(x, K):
    for i in range(N):
        if A[i] * F[i] <= x:
            continue
        # ceil(A[i] - (x / F[i]))なのでA[i]-floor((x / F[i]))に変形できる
        a = A[i] - (x // F[i])  # Kから使うべき個数
        K -= a
        if K < 0:
            return False
    return K > -1


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid, K):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(-1, 10**12 + 1))
