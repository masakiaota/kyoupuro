H, W = list(map(int, input().split()))
S = ['#' * (W + 2)]
for _ in range(H):
    S.append('#' + input() + '#')
S.append('#' * (W + 2))

rinsetu = {}
rinsetu2 = {}
for i in range(1, H + 1):
    for j in range(1, W + 1):
        if S[i][j] == '#':
            tmp = []
            if S[i + 1][j] == '.':
                tmp.append((i + 1, j))
            if S[i - 1][j] == '.':
                tmp.append((i - 1, j))
            if S[i][j + 1] == '.':
                tmp.append((i, j + 1))
            if S[i][j - 1] == '.':
                tmp.append((i, j - 1))
            rinsetu[(i, j)] = tmp

        if S[i][j] == '.':
            tmp = []
            if S[i + 1][j] == '#':
                tmp.append((i + 1, j))
            if S[i - 1][j] == '#':
                tmp.append((i - 1, j))
            if S[i][j + 1] == '#':
                tmp.append((i, j + 1))
            if S[i][j - 1] == '#':
                tmp.append((i, j - 1))
            rinsetu2[(i, j)] = tmp
print(rinsetu)
print(rinsetu2)


# 再帰処理がかけない
