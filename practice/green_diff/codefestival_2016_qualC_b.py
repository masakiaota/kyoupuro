# https://atcoder.jp/contests/code-festival-2016-qualc/tasks/codefestival_2016_qualC_b

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
rr = range


def read_ints():
    return list(map(int, read().split()))


K, T = read_ints()
A = read_ints()
if T == 1:
    # コーナーケース
    print(K - 1)
    exit()

# 差が最も少なくなるように二分割すれば良い
# 二分探索でもいいが線形探索で愚直に合計値を計算しても十分間に合う
A.sort()
ans = 2**31
for i in rr(1, T):
    r = sum(A[:i])
    l = sum(A[i:])
    ans = min(ans, abs(r - l))
print(max(ans - 1, 0))
