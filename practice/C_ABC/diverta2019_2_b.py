# https://atcoder.jp/contests/diverta2019-2/tasks/diverta2019_2_b


from itertools import combinations
from collections import defaultdict

N = int(input())
points = [tuple(map(int, input().split())) for _ in range(N)]

if N == 1:
    print(1)
    exit()

# points.sort()  # 先にソートしておくと大小関係の処理が楽

cnts = defaultdict(lambda: 0)
for pa, pb in combinations(points, 2):
    dx = pa[0] - pb[0]
    dy = pa[1] - pb[1]
    if dx < 0:  # 事前にソートしておくとココらへんの処理がいらなくなるよ
        dx *= -1
        dy *= -1
    if dx == 0:
        dy = max(dy, -dy)
    if dy == 0:
        dx = max(dx, -dx)
    cnts[(dx, dy)] += 1

print(N - max(cnts.values()))
