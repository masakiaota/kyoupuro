# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/all/ALDS1_1_B
# pythonでは組み込み関数を使えば良い
from fractions import gcd


a, b = list(map(int, input().split()))
print(gcd(a, b))
