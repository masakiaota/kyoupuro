# https://atcoder.jp/contests/abc048/tasks/abc048_b
# B問題だけど緑パフォ

a, b, x = map(int, input().split())
print(b // x - (a - 1) // x)
