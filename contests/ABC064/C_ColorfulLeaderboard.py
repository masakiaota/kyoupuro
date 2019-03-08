# WA
import numpy as np
N = int(input())
A = np.array(list(map(int, input().split())))


col = []  # boolean
for rate in range(0, 3200, 400):
    if len(A[(rate <= A) & (A < (rate + 400))]) is not 0:
        col.append(True)
    else:
        col.append(False)

jiyu = len(A[(3200 <= A)])  # 自由枠

assert len(col) == 8

# min
mi = sum(col)

# 自由に色を変えられるって任意のRGBにできるって意味なんかい！！！！！
# # max
# if jiyu < (8 - mi):
#     ma = mi+jiyu
# else:
#     ma = 8

ma = mi+jiyu

print("{} {}".format(mi, ma))
