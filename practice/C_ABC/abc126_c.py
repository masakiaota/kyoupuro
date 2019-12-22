# https://atcoder.jp/contests/abc126/tasks/abc126_c
N, K = list(map(int, input().split()))

probs = []
keisuu = 1 / N
for dice in range(1, N + 1):
    score = dice
    cnt = 0
    while score < K:
        score *= 2
        cnt += 1
    probs.append(keisuu * (1 / 2)**cnt)


print(sum(probs))
