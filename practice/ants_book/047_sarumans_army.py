# http://poj.org/problem?id=3069
from bisect import bisect_left, bisect_right
N = 6
R = 10
X = [1, 7, 15, 20, 30, 50]
# 端からギリギリRになるところに印をつけて更新していく
# whileで愚直にシミュレーションをする
i = 0
ans = 0
while i < N:
    x_left = X[i]
    # ここでは二分探索で印をつける点と、次の左端の点のidxを見つける
    x_right = X[bisect_right(X, x_left + R, lo=i) - 1] + R
    i = bisect_right(X, x_right, lo=i)
    ans += 1
print(ans)
