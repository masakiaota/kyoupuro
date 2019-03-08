N = list(map(str, input().split()))

flgs = [False] * 4
# 1974と対応
for n in N:
    for i, m in enumerate('1974'):
        if n == m:
            flgs[i] = True

if False in flgs:
    print('NO')
else:
    print('YES')
