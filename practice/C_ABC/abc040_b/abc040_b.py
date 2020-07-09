# https://atcoder.jp/contests/abc040/tasks/abc040_b
# min |W-H| + n - WH s.t WH<=n を求めたい
# 書くHについてW=floor(n/H)を計算し上式を調べれば良い
n = int(input())
ans = 2**31
for H in range(1, n + 1):
    W = n // H
    ans = min(ans, abs(W - H) + n - W * H)
print(ans)
