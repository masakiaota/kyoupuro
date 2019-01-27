N = int(input())
idx_to_AB = {}
for i in range(N):
    idx_to_AB[i] = list(map(int, input().split()))

A_to_idx = {value[0]: key for key, value in idx_to_AB.items()}
B_to_idx = {value[1]: key for key, value in idx_to_AB.items()}

A = sorted(A_to_idx, reverse=True)
B = sorted(B_to_idx, reverse=True)

cntA, cntB = 0, 0
while (len(idx_to_AB)):
    # 先行

    if A[0] > B[0]:
        cntA += A[0]
        B.remove(idx_to_AB[A_to_idx[A[0]]][1])
        del idx_to_AB[A_to_idx[A[0]]]
        A = A[1:]

    else:
        cntA += idx_to_AB[B_to_idx[B[0]]][0]
        A.remove(idx_to_AB[B_to_idx[B[0]]][0])
        del idx_to_AB[B_to_idx[B[0]]]
        B = B[1:]
    # print(idx_to_AB)
    # print(A, B)

    if len(idx_to_AB) == 0:
        break
    # 後攻
    if A[0] < B[0]:
        cntB += B[0]
        A.remove(idx_to_AB[B_to_idx[B[0]]][0])
        del idx_to_AB[B_to_idx[B[0]]]
        B = B[1:]
    else:
        cntB += idx_to_AB[A_to_idx[A[0]]][1]
        B.remove(idx_to_AB[A_to_idx[A[0]]][1])
        del idx_to_AB[A_to_idx[A[0]]]
        A = A[1:]

print(cntA-cntB)
