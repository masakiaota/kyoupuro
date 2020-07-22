# https://atcoder.jp/contests/arc054/tasks/arc054_b
# scipyに放り投げたら終わりそう
# 導関数がめちゃくちゃだから微分でやりたくない
from scipy.optimize import minimize_scalar
P = float(input())


def f(x):
    return x + P * pow((2**(-2 / 3)), x) if x >= 0 else 1e15


res = minimize_scalar(f, tol=0.000001)
print(res['fun'])
