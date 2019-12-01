# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_6_A

# load data
N = int(input())
lines = []
for _ in range(N):
    x1, y1, x2, y2 = list(map(int, input().split()))
    # 前処理として、x1,y1を必ず下端点or左端点にする
    if y1 == y2:  # 平行線の場合
        if x1 > x2:
            x1, x2 = x2, x1
    else:  # 垂直線の場合
        if y1 > y2:
            y1, y2 = y2, y1
    lines.append((x1, y1, x2, y2))

import matplotlib.pyplot as plt
plt_lin_ls = []
for x1, y1, x2, y2 in lines:
    plt_lin_ls.append((x1, x2))
    plt_lin_ls.append((y1, y2))
plt.plot(*plt_lin_ls)
plt.show()
