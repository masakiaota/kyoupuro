# https://atcoder.jp/contests/abc077/tasks/arc084_a
# bisectで行けるけどめぐる式の練習

# Bを固定すると、

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
B = read_ints()
C = read_ints()


def is_ok(arg):
    # 条件を満たすかどうか？問題ごとに定義
    pass


MIN = -1
MAX = 10**9


def meguru_bisect():
    ng = MIN  # とり得る最小の値-1
    ok = MAX  # とり得る最大の値+1
    # 最大最小が逆の場合はよしなにひっくり返す
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok
