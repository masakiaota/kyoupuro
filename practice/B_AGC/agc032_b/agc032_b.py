# https://atcoder.jp/contests/agc032/tasks/agc032_b
# 完全グラフからどの辺を減らそうかって考えるとわかる

N = int(input())
ans = set()
for i in range(1, N + 1):
    for j in range(i + 1, N + 1):
        ans.add((i, j))

if N & 1:  # 奇数
    N -= 1

for k in range(N // 2):
    i = k + 1
    j = N - k
    ans.remove((i, j))
print(len(ans))
for i, j in ans:
    print(i, j)
