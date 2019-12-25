# https://atcoder.jp/contests/abc139/tasks/abc139_c
# めちゃ簡単では

N = int(input())
H = list(map(int, input().split()))

ans = 0
cnt = 0
pre = -1
for h in H:
    if h <= pre:
        cnt += 1
    else:
        cnt = 0
    pre = h
    ans = max(ans, cnt)

print(ans)
