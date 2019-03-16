from collections import defaultdict
import sys
from math import factorial
read = sys.stdin.readline


def readln():
    return int(read())


N = readln()
C = [readln() for _ in range(N)]

d = defaultdict(lambda: 0)
s_sect = defaultdict(lambda: -1)
e_sect = defaultdict(lambda: -1)
mod = 10**9 + 7
old = None


# ブロックに切り分けてから数え上げだと思うがブロックにどう切り分ければいいのかわからん


for i, c in enumerate(C):
    e_sect[c] = i
    if c not in s_sect.keys():
        s_sect = i
    if c != old:
        d[c] += 1
        old = c

for sec
for s, e in zip(s_sect, e_sect):
    if
    # 全然わからん

ans = 0
for cnt in d.values():
    ans += factorial(cnt - 1) % mod

print(ans+1)
