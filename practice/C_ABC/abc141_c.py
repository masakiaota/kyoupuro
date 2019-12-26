# https://atcoder.jp/contests/abc141/tasks/abc141_c
# やるだけ

N, K, Q = list(map(int, input().split()))
scores = [0] * N
for _ in range(Q):
    a = int(input())
    scores[a - 1] += 1

for s in scores:
    # 本来のスコアへの変換は
    # K - (Q-s) #もともと-他の人の正解数
    if K - (Q - s) <= 0:
        print('No')
    else:
        print('Yes')
