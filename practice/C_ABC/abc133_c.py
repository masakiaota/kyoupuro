# https://atcoder.jp/contests/abc133/tasks/abc133_c
# O(n)だと2*10**9なのでpythonだと間に合わない！
# mod 2019 の計算結果は2019ごとに繰り返すことに気がつくと一気に計算が少なくなる

MOD = 2019  # これなにげに3の倍数だからいつも見たいなMODの演算が通用しない笑
L, R = list(map(int, input().split()))

# このコーナーケースに注意
if (R - L) >= MOD:
    print(0)
    exit()

# Lをなるべく0に近づける
L %= MOD
# RをなるべくLに近づけてみる
R %= MOD
if R <= L:
    R += MOD

# print(L, R)
ans = 2**16
for i in range(L, R):  # i<jが成り立つ範囲に注意
    for j in range(i + 1, R + 1):
        ans = min(ans, (i * j) % MOD)
print(ans)
