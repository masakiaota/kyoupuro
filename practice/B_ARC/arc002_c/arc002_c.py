# https://atcoder.jp/contests/arc002/tasks/arc002_3

# 全パターンのカウントを取れば良い

N = int(input())
C = input()
candi = ['A', 'B', 'X', 'Y']

from itertools import product, combinations

pat = []
for a, b in product(candi, candi):  # 16通り
    pat.append(a + b)
# 16通りを重複なしで2つ選ぶのは8*15 =120通り
# Nは10^3程度なので全パターン試しても間に合う
ans = N
for L, R in combinations(pat, r=2):
    ans = min(ans, len(C.replace(L, "L").replace(R, "R")))
print(ans)
