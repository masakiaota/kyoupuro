# https://atcoder.jp/contests/mujin-pc-2016/tasks/mujin_pc_2016_b
l1, l2, l3 = map(int, input().split())
outer = l1 + l2 + l3
# 三角不等式が成り立つときはinnerは0か
inner = min(abs(l3 - l1 - l2), abs(l1 - l2 - l3), abs(l2 - l1 - l3))
if l1 + l2 >= l3 and l2 + l3 >= l1 and l3 + l1 >= l2:
    inner = 0

from math import pi
print((outer**2 - inner**2) * pi)
