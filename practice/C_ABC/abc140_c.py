# https://atcoder.jp/contests/abc140/tasks/abc140_c

# A 未知
# B 既知

# 例
# B: 2 5
# A: 2 2 5

# B: 3
# A: 3 3

# B: 0 153 10 10 23
# A: 0   0 10 10 10 23

# B: 0 153 10 10 23
# C: 0  0 153 10 10 23
# AはBCの小さい方の要素である
# A: 0   0 10 10 10 23

# なぜならばA[i+1]について見てみると
# 少なくとも B[i]>=A[i+1]かつB[i+1]>=A[i+1]が保証されるので最も大きいA[i+1]はmin(B[i],B[i+1])となる

input()
B = list(map(int, input().split()))
print(sum([min(x, y) for x, y in zip(B + [10**5 + 1], [B[0]] + B)]))
