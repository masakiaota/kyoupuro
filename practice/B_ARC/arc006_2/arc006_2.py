N, L = map(int, input().split())
S = []
for _ in range(L):
    S.append(' ' + input() + ' ')
S = S[::-1]
s = list(input()).index('o') + 1
i = 0
j = s
while i != L:
    assert S[i][j] == '|'
    # print(S[i], j)
    if S[i][j - 1] == '-':
        j -= 2
    elif S[i][j + 1] == '-':
        j += 2
    i += 1
# print(j)
print((j - 1) // 2 + 1)
