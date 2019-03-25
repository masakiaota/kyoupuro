def readln():
    return list(map(int, input().split()))


S = input()
tmp = 0
ans = 0
for s in S:
    if s == 'A' or s == 'T' or s == 'C' or s == 'G':
        tmp += 1
        ans = max(tmp, ans)
    else:
        tmp = 0

print(ans)
