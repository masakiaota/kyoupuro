# https://atcoder.jp/contests/agc013/tasks/agc013_a
# 単調性を愚直に調べれば良い #と思ったけどこれはめんどいぞ
# めちゃくちゃ苦手なやつ
from itertools import groupby
N = int(input())
A = list(map(int, input().split()))
B = []
for key, v in groupby(A):
    B.append(key)

# /\か\/となる個数を見つければ良い？違う
diff = []
for i in range(len(B) - 1):
    diff.append(1 if B[i] < B[i + 1] else -1)

# 初めて符号が変わった瞬間のものは考慮しない
nums = []
for d, v in groupby(diff):
    nums.append(len(list(v)))

# print(nums)
is_can_del = 0
ans = 0
for n in nums:
    if n == 1 and is_can_del == 1:
        is_can_del = 0
    else:
        ans += 1
        is_can_del = 1
if is_can_del == 0:
    ans += 1
print(ans)
