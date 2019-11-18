# https://onlinejudge.u-aizu.ac.jp/courses/library/4/CGL/all/CGL_2_A
# 内積で判別すれば良い
# 丸め込み誤差の判別が糞すぎてもうやりたくない


from math import sqrt, isclose


def euclid_norm(x, y):
    return sqrt(pow(x, 2) + pow(y, 2))


def is_parallel(x0, y0, x1, y1, x2, y2, x3, y3):
    x_v1 = x1 - x0
    y_v1 = y1 - y0
    x_v2 = x3 - x2
    y_v2 = y3 - y2
    dot = (x_v1 * x_v2 + y_v1 * y_v2) / \
        (euclid_norm(x_v1, y_v1) * euclid_norm(x_v2, y_v2))
    # print(dot)
    if isclose(dot, 1, rel_tol=1e-10) or isclose(dot, -1, rel_tol=1e-10):
        return True
    return False


def is_orhogonal(x0, y0, x1, y1, x2, y2, x3, y3):
    x_v1 = x1 - x0
    y_v1 = y1 - y0
    x_v2 = x3 - x2
    y_v2 = y3 - y2
    dot = (x_v1 * x_v2 + y_v1 * y_v2) / \
        (euclid_norm(x_v1, y_v1) * euclid_norm(x_v2, y_v2))
    # print(dot)
    if isclose(dot, 0, rel_tol=1e-10):
        return True
    return False


# load data
N = int(input())
for i in range(N):
    tmp = list(map(int, input().split()))
    if is_parallel(*tmp):
        print(2)
    elif is_orhogonal(*tmp):
        print(1)
    else:
        print(0)
