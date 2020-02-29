# https://atcoder.jp/contests/abc108/tasks/arc102_a
# ちょっと難しい
# 情報を整理すると以下の条件が整理される
# aに関する必要条件 2aはKの倍数。
# aが決定したとき(Aとする)のbとc必要十分条件、b=xK-Aを満たす数字 where xは整数、bは[1,N]
# 以上の条件を満たすものを数えればいいのか？


N, K = map(int, input().split())


def f(A):
    # Aが与えられたとき、必要十分条件を満たすbはいくつあるのか
    # 1+A<=xK<=N+A を満たす xの個数を知りたい
    # N+A以下の整数でKの倍数はいくつある？ - AまでにKの倍数はいくつある？
    nb = (N + A) // K - A // K
    return nb * nb


ans = 0
for i in range(1, N + 1):
    if (2 * i) % K == 0:
        ans += f(i)
print(ans)
