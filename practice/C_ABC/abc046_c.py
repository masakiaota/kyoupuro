# https://atcoder.jp/contests/abc046/tasks/arc062_a
# 条件を考えよう
# iのときの割合の人数n[i],m[i]を考えると
# n[i]=p*A[i], m[i]=p*B[i] かつ n[i]>=n[i-1], m[i]>=m[i-1] を満たす最小のn[i], m[i]
# このような条件を満たすpを探すのは容易だ
# n[i-1]//A[i] がn[i]をギリギリ超えないpなのだから、ギリギリ超えるpはn[i-1]//A[i] + 1 (ぴったり割り切れるときは注意)
# nについてもmについてもギリギリ越したいのだから大きい方を採用する。(小さい方では片方が超えなくなる)


N = int(input())
ans = 1


def ret_nm(t, a, n, m):
    times_t = (n - 1) // t + 1
    times_a = (m - 1) // a + 1
    times = max(times_t, times_a)
    return times * t, times * a


n = m = 1
for _ in range(N):
    t, a = map(int, input().split())
    n, m = ret_nm(t, a, n, m)


print(n + m)
