N, A, B, C = list(map(int, input().split()))
L = [int(input()) for _ in range(N)]

from itertools import permutations, combinations


def dellist(items, contents):
    items = items.copy()
    for rm in contents:
        # print(rm)
        items.remove(rm)
    return items


ans = []
L_temp = L.copy()

for n_g1 in range(1, N):
    for n_g2 in range(1, N - n_g1+1):
        for n_g3 in range(1, N - n_g1 - n_g2 + 1):
            # print(n_g1, n_g2, n_g3)
            L_temp = L.copy()
            for G1 in combinations(L_temp, n_g1):
                L_temp = L.copy()
                L_temp = dellist(L_temp, G1)
                # print('G1', L_temp)
                for G2 in combinations(L_temp, n_g2):
                    L_temp = L.copy()
                    L_temp = dellist(L_temp, G1)
                    L_temp = dellist(L_temp, G2)
                    # print('G2', L_temp)
                    for G3 in combinations(L_temp, n_g3):
                        # print('G3', L_temp)
                        s1, s2, s3 = sum(G1), sum(G2), sum(G3)
                        syn = len(G1) + len(G2) + len(G3) - 3
                        ans.append(abs(s1 - A) + abs(s2 - B) +
                                   abs(s3 - C) + syn * 10)

# print(ans)
print(min(ans))
