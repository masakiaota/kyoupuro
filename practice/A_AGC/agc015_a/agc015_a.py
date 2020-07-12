# https://atcoder.jp/contests/agc015/tasks/agc015_a
# 最小と最大は必ず一つは存在するのか
n, a, b = map(int, input().split())
if b < a or (n == 1 and a != b):  # コーナーケース
    print(0)
    exit()
ma = b * (n - 1) + a
mi = a * (n - 1) + b
print((ma - mi + 1))
