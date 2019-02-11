from itertools import product

H, W = list(map(int, input().split()))
A = [list(map(int, input().split())) for _ in range(H)]

move = [(-1, 0), (0, -1), (0, 1), (1, 0)]

cord = []
# cord_prime = []
N = 0

# print(A)

for i, j in product(range(H), range(W)):
    # print(A[i][j])
    if A[i][j] % 2 == 0:
        continue
    else:
        # for di, dj in move:
        #     if j + dj < 0 or i + di < 0 or j + dj >= W or i + di >= H:
        #         continue

        #     if A[i + di][j + dj] % 2:  # 探索が奇数だったら
        #         N += 1
        #         cord.append((i + 1, j + 1, i + di + 1, j + dj + 1))
        #         A[i + di][j + dj] += 1
        #         A[i][j] -= 1
        #         continue

        # なかったら押し付ける
        A[i][j] -= 1
        if i == H - 1:
            # 最終行では隣に押し付ける
            if j != W - 1:
                A[i][j + 1] += 1
                N += 1
                cord.append((i + 1, j + 1, i + 1, j + 2))
            else:
                # 最終マスではなにもしない
                A[i][j] += 1
        else:
            # 基本的に下に押し付ける
            A[i + 1][j] += 1
            N += 1
            cord.append((i + 1, j + 1, i + 2, j + 1))

print(N)
for c in cord:
    print(*c)

# print(A)
# print(A)
