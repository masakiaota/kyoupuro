# https://atcoder.jp/contests/arc081/tasks/arc081_a
N = int(input())
A = list(map(int, input().split()))

# まずは使える辺の長さを抽出
from collections import Counter
A_cnt = Counter(A)
candi = []
for k, v in A_cnt.items():
    if v >= 4:
        candi.extend([k, k])
    if v >= 2:
        candi.append(k)
if len(candi) < 2:
    print(0)
    exit()
A_ls = sorted(candi, reverse=True)
print(A_ls[0] * A_ls[1])
