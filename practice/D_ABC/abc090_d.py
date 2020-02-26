# https://atcoder.jp/contests/abc090/tasks/arc091_b
N, K = map(int, input().split())
# aで1~Nまでの数を割っていったときにK以下はいくつあるか？


def f(a, n, k):
    syou, amari = divmod(n, a)
    one_syou = max(a - k, 0)
    plus = max(amari + 1 - k, 0) if k != 0 else amari
    # print(syou, amari, one_syou, plus)
    return syou * one_syou + plus


ans = 0
for a in range(1, N + 1):
    ans += f(a, N, K)
    # print(a, f(a, N, K))

print(ans)
