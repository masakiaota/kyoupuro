# https://onlinejudge.u-aizu.ac.jp/courses/library/6/NTL/1/NTL_1_B
# pythonのpowが優秀でMODまで取ることができる

MOD = 10**9 + 7
a, b = list(map(int, input().split()))
print(pow(a, b, MOD))
