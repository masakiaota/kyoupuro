# https://atcoder.jp/contests/code-festival-2014-final/tasks/code_festival_final_e
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
R = read_ints()

if N < 3:
    print(0)
    exit()

# 愚直に極大点極小点を列挙すれば良いのでは...
# また端点は必ず含まれるのだから+2しておけば良い
# フラットになっている部分は注意→事前に取り除いておけば良い
RR = []
pre = 10**9
for r in R:
    if pre != r:
        RR.append(r)
    pre = r


ans = 0
for i in ra(1, len(RR) - 1):
    pre = RR[i - 1]
    now = RR[i]
    nex = RR[i + 1]
    if pre > now < nex or pre < now > nex:
        ans += 1

print(ans + 2 if ans != 0 else 0)
