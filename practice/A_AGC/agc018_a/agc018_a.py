# https://atcoder.jp/contests/agc018/tasks/agc018_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


from fractions import gcd
N, K = read_ints()
A = read_ints()
# 結論からいうとKがminとmaxの間でかつ、gcd(A)の倍数であるならば、必ずKを構成することができる。
# ∵ある2要素a,bからはgcd(a,b)の倍数しか作ることができない。これをすべての要素に適応するだけ。

g = 0  # gcdの単位元
for a in A:
    g = gcd(g, a)

if min(A) <= K <= max(A) and K % g == 0:
    print('POSSIBLE')
else:
    print('IMPOSSIBLE')
