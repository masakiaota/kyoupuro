# https://atcoder.jp/contests/dwango2016-prelims/tasks/dwango2016qual_b
# K_j = max(L_j, L_{j+1}) ⇔ L_j = min(K_{j-1},K_{j})
N = int(input())
K = list(map(int, input().split()))
ans = [K[0]]
for i in range(N - 2):
    ans.append(min(K[i], K[i + 1]))
ans.append(K[-1])
print(*ans)
