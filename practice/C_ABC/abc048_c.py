# https://atcoder.jp/contests/abc048/tasks/arc064_a
# 問題文により、操作後の数列は以下を満たす
# x>=a[i]+a[i+1]
# よって、x-a[i] >= a[i+1]を満たすようにa[i+1]を決定していけば良い
# 実際に与えられたaと満たすように作ったaの差が答え


N, x = list(map(int, input().split()))
A = list(map(int, input().split()))

ans = 0
# 端の処理には気をつけよう
if A[0] > x:
    ans = A[0] - x
    A[0] = x

for i in range(N - 1):
    new = x - A[i]
    if new >= A[i + 1]:
        # 条件を満たしているのでok
        continue
    ans += A[i + 1] - new
    A[i + 1] = new

print(ans)
