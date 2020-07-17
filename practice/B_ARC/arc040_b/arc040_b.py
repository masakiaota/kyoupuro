# https://atcoder.jp/contests/arc040/tasks/arc040_b
# greedyに塗ればいいんじゃない？
N, R = map(int, input().split())
S = list(map(int, input().replace('.', '0').replace('o', '1')))  # 塗られていれば1
if 0 not in S:
    print(0)
    exit()
for end in range(N - 1, -1, -1):
    if S[end] == 0:
        break
now = 0
cnt = 0
while now < end - (R - 1):
    if S[now] == 1:
        now += 1
    else:
        S[now:now + R] = [1] * R
    cnt += 1
print(cnt + 1)
