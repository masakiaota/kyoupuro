# https://atcoder.jp/contests/arc024/tasks/arc024_2
# 最初っからすべて同じ色だったら-1は確定
# 連続区間については2つずつ減ってく
# 一番ながい連続区間長に対して-2できるか回数が答えでは？
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


N, *C = map(int, open(0).read().split())
l_max = -1
pre = -1
i_pre = 0
for i, c in enu(C + C):
    if pre != c:
        l_max = max(l_max, i - i_pre)
        i_pre = i
    pre = c
if i_pre == 0:
    exit(-1)

ans = 0
while l_max > 0:
    l_max -= 2
    ans += 1
print(ans)

# print((l_max - 1) // 2 + 1)
# 実はO(1)で計算可能
