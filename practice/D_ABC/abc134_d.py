# https://atcoder.jp/contests/abc134/tasks/abc134_d
# よくわからん本当に緑diff?
# とりあえずエラトステネスの篩のように倍数を列挙する感じでほぼ愚直にやる
# N/2以上の整数に関しては即座に答えが決まる。
# 答えが決まった部分より下を引いていけば良いのでは？

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = [0] + read_ints()

B = [0] * (N + 1)
B[(N // 2) + 1:] = A[(N // 2) + 1:]


for i in range(N // 2, 0, -1):
    a = A[i]
    j = i + i
    while j < N + 1:
        a -= B[j]
        j += i
    B[i] = a % 2
# print(B)


print(sum(B))
for i, b in enumerate(B):
    if b:
        print(i, end=' ')
