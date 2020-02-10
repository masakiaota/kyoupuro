N = int(input())
K = int(input())

# F(N,K) ... N以下の整数で0でない数字がちょうどK個あるものの個数

from functools import lru_cache
import sys
sys.setrecursionlimit(1 << 25)


@lru_cache(None)
def F(N, K):
    '''N以下でちょうど0以外がK回出てくる (0以外を特にカウントする)'''
    # 終了条件
    assert N > -1
    if K < 0:
        return 0
    if N < 10:
        if K == 0:
            return 1  # '0'でないものが0個の状況、一桁ならば0は必ず0から始まる
        if K == 1:
            return N  # 3だったら1,2,3と'0'でないものが存在する。
        return 0  # それ以外はありえない。

    # 桁dp
    q, r = divmod(N, 10)
    ret = 0
    for d in range(10):
        # d=0のとき、これは必ずF(q,K)の通りの数足される
        if d == 0:
            ret += F(q, K)
        else:  # d=0のとき以外はKは一つ消費される
            # d>rのときF(q-1,K-1)から寄与が来る
            if d > r:
                ret += F(q - 1, K - 1)
            else:
                ret += F(q, K - 1)
    return ret


print(F(N, K))
