N, A, B, C = list(map(int, input().split()))
L = [int(input()) for _ in range(N)]

from itertools import permutations


def min_cost(g_list):
    ret = []
    for g1, g2, g3 in permutations(g_list):
        ret.append(abs(g1 - A) + abs(g2 - B) + abs(g3 - C))
    return min(ret)


ans = []
for LL in permutations(L):
    for p1 in range(1, N - 1):
        for p2 in range(p1+1, N):
            for p3 in range(p2 + 1, N+1):
                g1, g2, g3 = sum(L[:p1]), sum(L[p1:p2]), sum(L[p2:p3])
                syn = [len(
                    L[:p1])-1, len(L[p1:p2])-1, len(L[p2:p3])-1]
                # print(g1, g2, g3)
                # print(p1, p2, p3)
                ans.append(min_cost([g1, g2, g3]) + sum(syn)*10)

print(min(ans))
# print(ans)
