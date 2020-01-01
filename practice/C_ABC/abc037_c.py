# https://atcoder.jp/contests/abc037/tasks/abc037_c
# 親の顔より見た累積和

from itertools import accumulate
N, K = list(map(int, input().split()))
A = list(map(int, input().split()))
A_acc = [0] + list(accumulate(A))

ans = 0
for i in range(K, N + 1):
    ans += A_acc[i] - A_acc[i - K]

print(ans)
