# https://atcoder.jp/contests/arc059/tasks/arc059_a
# 全探索してもいいし、二乗誤差の最小化から平均値を求めても良い

N = int(input())
A = list(map(int, input().split()))
candi1 = sum(A) // N
candi2 = candi1 + 1

ans1 = ans2 = 0
for a in A:
    ans1 += (a - candi1)**2
    ans2 += (a - candi2)**2
print(min(ans1, ans2))
