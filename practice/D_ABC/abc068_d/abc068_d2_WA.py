# https://atcoder.jp/contests/abc068/tasks/arc079_b
# 重要な考察
# すべての要素がNを超えているとき、N回操作するすべての要素は-1される。
# つまりN*(K//N)回操作されたときに、すべての要素はN+K//N-1(=base)になってほしい
# 残りのK%N回の操作では、一つの要素に値を押し付けてしまっても問題ない 制約より(最大でも50*50で2500こえるから無理やんけ！)

ra = range
enu = enumerate
K = int(input())
N = 50

base = N + K // N - 1
n_operate = K % N

if base - n_operate == 0:  # コーナーケース対策
    N = 49
    base = N + K // N - 1
    n_operate = K % N


ans = []
# はじめのK%N要素はbase-K%N-1となる
for _ in ra(1):
    ans.append(base + N * (n_operate))  # 最大10**16+2500だから制約超えちゃう

for _ in ra(N - 1):
    ans.append(base - n_operate)  # コーナーケースとしてこれが0になってしまう可能性がある
print(N)
print(*ans)
