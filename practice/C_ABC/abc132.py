# https://atcoder.jp/contests/abc132/tasks/abc132_c
# ソートして真ん中の差を求めるだけでは?

N = int(input())
D = sorted(map(int, input().split()))
print(D[N // 2] - D[(N // 2) - 1])
