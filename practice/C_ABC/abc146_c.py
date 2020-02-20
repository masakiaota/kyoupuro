# https://atcoder.jp/contests/abc146/tasks/abc146_c?lang=ja
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


A, B, X = read_ints()


def is_ok(arg):
    # 整数を買えればTrueを返す
    return A * arg + B * len(str(arg)) <= X


def meguru_bisect():
    ng = 10**9 + 1  # 店で買える最大の整数+1
    ok = 0
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect())
