# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_4_A
# 可視化用


N = int(input())
points = []
for _ in range(N):
    points.append(tuple(map(int, input().split())))


import matplotlib.pyplot as plt
x = [x[0] for x in points]
y = [y[1] for y in points]
plt.scatter(x, y)
plt.show()
