# https://atcoder.jp/contests/abc128/tasks/abc128_d


# 単純なアイデアとしては、ほぼ愚直に操作を行う。
# K個のViを取って一番合計の大きいもの
# K-1のViを取って価値の引くものを1つ詰めるとしたときに一番合計の大きくなるもの
# K-iのViを取って価値の低いものをiつ詰めるとしたときに一番合計が大きくなるもの

# 最大N個とるのを最大N回ずらして、最大N個の手持ちのソートするので、オーダーはO(N^3 logN)
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
V = read_ints()
V = V + V

ans = 0
for i in range(K):
    n_pick = K - i
    n_back = i
    if n_pick > N:
        n_back = n_pick - N
        n_pick = N

    for j in range(n_pick + 1):  # スライドよう
        v = V[N - j:N + n_pick - j]
        v.sort()
        # print(v)
        if n_back < len(v) and v[n_back] < 0:
            ans = max(ans, sum(v[n_back:]))  # ここでは最大n_back個まで捨てることができる
        else:
            ans = max(ans, sum([max(0, x) for x in v]))
print(ans)
