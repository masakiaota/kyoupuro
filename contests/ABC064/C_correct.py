# 要は全員自由色のパターンを見落としていた
# もし自由色がいて、かつ、固定色が0人の場合、最小のパターンは1
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
# max
ma = mi + jiyu

# 以下を見落としていた
if (mi is 0) and (jiyu is not 0):
    mi = 1

print("{} {}".format(mi, ma))
