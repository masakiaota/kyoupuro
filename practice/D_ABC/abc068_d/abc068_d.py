# https://atcoder.jp/contests/abc068/tasks/arc079_b
# 重要な考察
# すべての要素がNを超えているとき、N回操作するすべての要素は-1される。
# つまりN*(K//N)回操作されたときに、すべての要素はN+K//N-1(=base)になってほしい
# 残りのK%N回の操作でこの数列がどう変化するかは愚直にシミュレーションしてもいいし、計算しても求まる

ra = range
enu = enumerate
K = int(input())
N = 50

base = N + K // N - 1
n_operate = K % N

ans = []
# はじめのK%N要素はbase-K%N-1となる
for _ in ra(K % N):
    ans.append(base + N - K % N + 1)

for _ in ra(N - K % N):
    ans.append(base - K % N)
print(N)
print(*ans)
