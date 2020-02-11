# https://atcoder.jp/contests/abc113/tasks/abc113_c
# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
data = []
for i in range(M):
    data.append(list(map(int, read().split())) + [i])

data.sort()
ans = []
cnt = [0] * (N + 1)  # それぞれの出現回数をカウントする →x番目を取得するため
for p, y, i in data:
    cnt[p] += 1
    x = cnt[p]
    ans.append((i, str(p).zfill(6) + str(x).zfill(6)))

ans.sort()
for _, a in ans:
    print(a)
