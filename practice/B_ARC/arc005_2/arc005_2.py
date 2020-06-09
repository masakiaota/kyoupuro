# https://atcoder.jp/contests/arc005/tasks/arc005_2
* xy, W = input().split()

x, y = map(lambda x: int(x) - 1, xy)
import numpy as np
M = []
for i in range(9):
    M.append(list(map(int, list(input()))))
M = np.array(M)
# M_extended = np.block([[M[::-1, ::-1], M[::-1, 1:-1], M[::-1, ::-1]],
#                        [M[1:-1, ::-1], M[1:-1, 1:-1], M[1:-1, ::-1]],
#                        [M[::-1, ::-1], M[::-1, 1:-1], M[::-1, ::-1]]])
M_extended = np.vstack(
    [np.hstack([M[::-1, ::-1], M[::-1, 1:-1], M[::-1, ::-1]]),
     np.hstack([M[1:-1, ::-1], M[1:-1, 1:-1], M[1:-1, ::-1]]),
     np.hstack([M[::-1, ::-1], M[::-1, 1:-1], M[::-1, ::-1]])])

shift = 8
x, y = x + shift, y + shift
ans = []
for i in range(4):
    if W == 'R':
        yy, xx = y, x + i
    elif W == 'L':
        yy, xx = y, x - i
    elif W == 'U':
        yy, xx = y - i, x
    elif W == 'D':
        yy, xx = y + i, x
    elif W == 'RU':
        yy, xx = y - i, x + i
    elif W == 'RD':
        yy, xx = y + i, x + i
    elif W == 'LU':
        yy, xx = y - i, x - i
    elif W == 'LD':
        yy, xx = y + i, x - i

    ans.append(M_extended[yy, xx])
print(*ans, sep='')
